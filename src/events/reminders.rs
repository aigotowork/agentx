//! One-shot reminder policy and bounded writer scheduling. A reminder is
//! consumed only by the transaction that records its final delivery outcome.

use std::time::{Duration, Instant};

use crate::agents::{attachments::ParticipantRef, records::is_remote_instance, service::resolve_actor};
use crate::db::{
    AgentxDb, InstanceRow,
    reminders::{ReminderKey, TimeReminder},
};
use crate::service::{RequestContext, ServiceError, ServiceResult};

use super::service::{MAX_REMINDER_DELAY_MS, ReminderCreated, ReminderCursor, ReminderPage, ReminderState, ReminderView};

const BATCH: i64 = 32;
const PENDING_INTERVAL: Duration = Duration::from_millis(100);
const IDLE_INTERVAL: Duration = Duration::from_secs(5);
const PASS_BUDGET: Duration = Duration::from_millis(75);
const RECOVERY_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

fn outcome<T>(result: anyhow::Result<T>) -> ServiceResult<T> {
    result.map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

fn local_actor(db: &AgentxDb, context: &RequestContext) -> ServiceResult<ParticipantRef> {
    let actor = resolve_actor(db, context)?;
    if actor.record.name == crate::shared::SENDER || is_remote_instance(&actor.record) {
        return Err(ServiceError::Forbidden("time reminders require a local Agent identity".into()));
    }
    if let Some(reason) = unavailable_reason(&actor.record) {
        return Err(ServiceError::Forbidden(reason.into()));
    }
    Ok(actor.participant)
}

fn unavailable_reason(row: &InstanceRow) -> Option<&'static str> {
    if row.status_context == "launch_failed" {
        Some("target launch failed")
    } else if row.status == "stopped" || (row.status == "inactive" && row.status_context.starts_with("exit:")) {
        Some("target stopped")
    } else {
        None
    }
}

pub(super) fn create(db: &AgentxDb, context: &RequestContext, target: ParticipantRef, due_at_ms: i64, message: String) -> ServiceResult<ReminderCreated> {
    crate::messaging::routing::validate_message(&message).map_err(ServiceError::InvalidInput)?;
    let request_id = &context.caller.request_id;
    outcome(db.with_write_scope(|| {
        let creator = local_actor(db, context)?;
        if let Some(previous) = db.reminder_by_request(request_id)? {
            if previous.created_by.as_ref() != Some(&creator) || previous.view.target != target || previous.view.due_at_ms != due_at_ms || previous.view.message != message {
                return Err(ServiceError::Conflict("request ID already used with a different reminder".into()).into());
            }
            return Ok(ReminderCreated { reminder: previous.view, already_exists: true });
        }
        let now = now_ms();
        if due_at_ms <= now || due_at_ms > now.saturating_add(MAX_REMINDER_DELAY_MS) {
            return Err(ServiceError::InvalidInput("reminder due time must be in the next 365 days".into()).into());
        }
        if target.canonical_name == crate::shared::SENDER {
            return Err(ServiceError::Unsupported("time reminders require a local Agent target".into()).into());
        }
        let Some(row) = db.get_instance_full(&target.canonical_name)? else {
            return Err(ServiceError::StaleFence.into());
        };
        if row.created_at.to_bits() != target.incarnation_bits {
            return Err(ServiceError::StaleFence.into());
        }
        if is_remote_instance(&row) {
            return Err(ServiceError::Unsupported("remote time reminders are unavailable".into()).into());
        }
        if let Some(reason) = unavailable_reason(&row) {
            return Err(ServiceError::Conflict(reason.into()).into());
        }
        let id = format!("rem-{}", uuid::Uuid::new_v4().simple());
        let view = ReminderView { id, target, due_at_ms, message, state: ReminderState::Pending, completed_at_ms: None, delivery_event_id: None, failure_reason: None };
        let record = TimeReminder { view, request_id: request_id.clone(), created_by: Some(creator), created_at_ms: now };
        db.insert_reminder(&record)?;
        Ok(ReminderCreated { reminder: record.view, already_exists: false })
    }))
}

pub(super) fn list(db: &AgentxDb, include_history: bool, after: Option<&ReminderCursor>) -> ServiceResult<ReminderPage> {
    db.reminders(include_history, after).map_err(internal)
}

pub(super) fn show(db: &AgentxDb, id: &str) -> ServiceResult<ReminderView> {
    db.reminder(id).map_err(internal)?.map(|row| row.view).ok_or_else(|| ServiceError::NotFound(format!("reminder '{id}'")))
}

pub(super) fn cancel(db: &AgentxDb, context: &RequestContext, id: &str) -> ServiceResult<bool> {
    outcome(db.with_write_scope(|| {
        let actor = local_actor(db, context)?;
        let Some(record) = db.reminder(id)? else { return Ok(false) };
        if record.created_by.as_ref() != Some(&actor) && record.view.target != actor {
            return Err(ServiceError::Forbidden("only the original reminder creator or target may cancel it".into()).into());
        }
        if !db.complete_reminder(id, now_ms(), ReminderState::Canceled, None, None)? {
            return Err(ServiceError::Conflict(format!("reminder {id} is no longer pending")).into());
        }
        Ok(true)
    }))
}

fn eligible_target(db: &AgentxDb, target: &ParticipantRef) -> anyhow::Result<Result<InstanceRow, &'static str>> {
    Ok(match db.get_instance_full(&target.canonical_name)? {
        None => Err("target is stopped or missing"),
        Some(row) if row.created_at.to_bits() != target.incarnation_bits => Err("target incarnation was replaced"),
        Some(row) if is_remote_instance(&row) => Err("target is no longer local"),
        Some(row) => match unavailable_reason(&row) {
            Some(reason) => Err(reason),
            None if crate::agents::records::is_launching_placeholder(&row) && crate::agents::lifecycle::get_instance_status_stored(&row, db).context == "launch_failed" => Err("target launch failed"),
            None => Ok(row),
        },
    })
}

fn deliver(db: &AgentxDb, id: &str, now: i64) -> anyhow::Result<Option<ReminderState>> {
    db.with_background_write_scope(|| {
        let Some(record) = db.reminder(id)? else { return Ok(None) };
        let reminder = record.view;
        if reminder.state != ReminderState::Pending || reminder.due_at_ms > now {
            return Ok(None);
        }
        let state = match eligible_target(db, &reminder.target)? {
            Err(reason) => {
                anyhow::ensure!(db.complete_reminder(id, now, ReminderState::Failed, None, Some(reason))?, "reminder changed inside its write transaction");
                ReminderState::Failed
            }
            Ok(_) => {
                let message = crate::messaging::submission::persist_reminder_message(db, &reminder.target, id, &reminder.message)?;
                anyhow::ensure!(db.complete_reminder(id, now, ReminderState::Enqueued, Some(message.event_id), None)?, "reminder changed inside its write transaction");
                db.after_event_logged(message.event_id, "message", &message.routing_instance, &message.data);
                db.wake_exact_after_commit(&reminder.target);
                ReminderState::Enqueued
            }
        };
        Ok(Some(state))
    })
}

struct Recovery {
    after: ReminderKey,
    through_event: i64,
}

pub(crate) struct Schedule {
    next_check: Instant,
    after: ReminderKey,
    through_due: Option<i64>,
    recovery: Option<Recovery>,
}

impl Schedule {
    pub(crate) fn new(db: &AgentxDb) -> Self {
        Self { next_check: Instant::now(), after: (i64::MIN, String::new()), through_due: None, recovery: Some(Recovery { after: (now_ms().saturating_sub(RECOVERY_WINDOW_MS), String::new()), through_event: db.get_last_event_id() }) }
    }

    pub(crate) fn invalidate(&mut self) {
        self.next_check = Instant::now();
    }

    pub(crate) fn after_reset(&mut self) {
        self.after = (i64::MIN, String::new());
        self.through_due = None;
        self.recovery = None;
        self.invalidate();
    }

    pub(crate) fn check(&mut self, db: &AgentxDb, available: impl FnOnce() -> bool) {
        if self.next_check > Instant::now() || !available() {
            return;
        }
        let now = now_ms();
        let pending = self.process_due(db, now).unwrap_or_else(|error| {
            crate::log::log_error("events", "reminder.schedule", &error.to_string());
            true
        });
        if let Some(recovery) = &mut self.recovery {
            match recovery_step(db, recovery) {
                Ok(false) => self.recovery = None,
                Ok(true) => {}
                Err(error) => crate::log::log_error("events", "reminder.recover", &error.to_string()),
            }
        }
        self.next_check = Instant::now() + if pending || self.recovery.is_some() { PENDING_INTERVAL } else { IDLE_INTERVAL };
    }

    fn process_due(&mut self, db: &AgentxDb, now: i64) -> anyhow::Result<bool> {
        let through = self.through_due.unwrap_or(now).min(now);
        let keys = db.due_reminder_keys(through, &self.after, BATCH)?;
        let mut more = keys.len() == BATCH as usize;
        let start = Instant::now();
        let (mut enqueued, mut failed) = (0, 0);
        for key in keys {
            let id = &key.1;
            match deliver(db, id, now) {
                Ok(Some(ReminderState::Enqueued)) => enqueued += 1,
                Ok(Some(ReminderState::Failed)) => {
                    failed += 1;
                    crate::log::log_warn("events", "reminder.target_unavailable", id);
                }
                Ok(_) => {}
                Err(error) => crate::log::log_error("events", "reminder.delivery_retry", &format!("{id}: {error}")),
            }
            // Continue past a failing row; restart from the front at the tail.
            self.after = key;
            if start.elapsed() >= PASS_BUDGET {
                more = true;
                break;
            }
        }
        self.through_due = more.then_some(through);
        if !more {
            self.after = (i64::MIN, String::new());
        }
        if enqueued + failed > 0 {
            crate::log::log_info("events", "reminder.processed", &format!("enqueued={enqueued} failed={failed}"));
        }
        Ok(more || db.has_pending_reminders()?)
    }
}

fn recovery_step(db: &AgentxDb, recovery: &mut Recovery) -> anyhow::Result<bool> {
    let keys = db.enqueued_reminder_keys(&recovery.after, recovery.through_event, BATCH)?;
    let more = keys.len() == BATCH as usize;
    let start = Instant::now();
    for key in keys {
        let wake = || -> anyhow::Result<()> {
            if let Some(record) = db.reminder(&key.1)?
                && let Some(event) = record.view.delivery_event_id
                && let Ok(row) = eligible_target(db, &record.view.target)?
                && row.last_event_id < event
            {
                db.wake_exact_after_commit(&record.view.target);
            }
            Ok(())
        };
        if let Err(error) = wake() {
            crate::log::log_error("events", "reminder.recover_target", &format!("{}: {error}", key.1));
        }
        recovery.after = key;
        if start.elapsed() >= PASS_BUDGET {
            return Ok(true);
        }
    }
    Ok(more)
}

#[cfg(test)]
mod tests;
