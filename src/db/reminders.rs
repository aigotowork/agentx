//! SQLite persistence for one-shot reminders. The Events use case owns all
//! state decisions and composes completion with the message write scope.

use anyhow::Result;
use rusqlite::{OptionalExtension, params};

use super::AgentxDb;
use crate::agents::attachments::ParticipantRef;
use crate::events::service::{ReminderCursor, ReminderPage, ReminderState, ReminderView};
use crate::service::RequestId;

pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS reminders (
        id TEXT PRIMARY KEY NOT NULL,
        request_id TEXT UNIQUE NOT NULL,
        target_name TEXT NOT NULL,
        target_incarnation_bits TEXT NOT NULL,
        due_at_ms INTEGER NOT NULL,
        message TEXT NOT NULL,
        state TEXT NOT NULL CHECK(state IN ('pending','enqueued','failed','canceled')),
        created_at_ms INTEGER NOT NULL,
        completed_at_ms INTEGER,
        delivery_event_id INTEGER,
        failure_reason TEXT,
        created_by TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_reminders_due ON reminders(state, due_at_ms, id);
    CREATE INDEX IF NOT EXISTS idx_reminders_history ON reminders(due_at_ms, id);
    CREATE INDEX IF NOT EXISTS idx_reminders_recovery ON reminders(completed_at_ms, id) WHERE state='enqueued';
";

const VIEW: &str = "id,target_name,target_incarnation_bits,due_at_ms,message,state,completed_at_ms,delivery_event_id,failure_reason";

pub(crate) struct TimeReminder {
    pub view: ReminderView,
    pub request_id: RequestId,
    pub created_by: Option<ParticipantRef>,
    pub created_at_ms: i64,
}

fn view(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReminderView> {
    let bits = row.get::<_, String>(2)?.parse().map_err(|error| decode_error(2, error))?;
    let state = match row.get::<_, String>(5)?.as_str() {
        "pending" => ReminderState::Pending,
        "enqueued" => ReminderState::Enqueued,
        "failed" => ReminderState::Failed,
        "canceled" => ReminderState::Canceled,
        _ => return Err(decode_error(5, std::io::Error::other("invalid reminder state"))),
    };
    Ok(ReminderView {
        id: row.get(0)?,
        target: ParticipantRef { canonical_name: row.get(1)?, incarnation_bits: bits },
        due_at_ms: row.get(3)?,
        message: row.get(4)?,
        state,
        completed_at_ms: row.get(6)?,
        delivery_event_id: row.get(7)?,
        failure_reason: row.get(8)?,
    })
}

fn decode_error(column: usize, error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(column, rusqlite::types::Type::Text, Box::new(error))
}

fn record(row: &rusqlite::Row<'_>) -> rusqlite::Result<TimeReminder> {
    let created_by = row.get::<_, Option<String>>(11)?.map(|json| serde_json::from_str(&json)).transpose().map_err(|error| decode_error(11, error))?;
    Ok(TimeReminder { view: view(row)?, request_id: RequestId(row.get(9)?), created_at_ms: row.get(10)?, created_by })
}

pub(crate) type ReminderKey = (i64, String);

impl AgentxDb {
    pub(crate) fn reminder_by_request(&self, id: &RequestId) -> Result<Option<TimeReminder>> {
        Ok(self.conn().prepare_cached(&format!("SELECT {VIEW},request_id,created_at_ms,created_by FROM reminders WHERE request_id=?"))?.query_row([&id.0], record).optional()?)
    }

    pub(crate) fn reminder(&self, id: &str) -> Result<Option<TimeReminder>> {
        Ok(self.conn().prepare_cached(&format!("SELECT {VIEW},request_id,created_at_ms,created_by FROM reminders WHERE id=?"))?.query_row([id], record).optional()?)
    }

    pub(crate) fn reminders(&self, include_history: bool, after: Option<&ReminderCursor>) -> Result<ReminderPage> {
        const LIMIT: usize = 100;
        let lower = after.map_or((i64::MIN, ""), |cursor| (cursor.due_at_ms, cursor.id.as_str()));
        let pending = if include_history { "" } else { "AND state='pending'" };
        let mut stmt = self.conn().prepare_cached(&format!("SELECT {VIEW} FROM reminders WHERE (due_at_ms,id)>(?1,?2) {pending} ORDER BY due_at_ms,id LIMIT ?3"))?;
        let mut reminders = Vec::new();
        let mut next = None;
        let mut bytes = 0usize;
        for row in stmt.query_map(params![lower.0, lower.1, LIMIT as i64 + 1], view)? {
            let reminder = row?;
            let size = serde_json::to_vec(&reminder)?.len() + 1;
            if reminders.len() == LIMIT || bytes + size > crate::events::service::MAX_PAGE_BYTES {
                let last: &ReminderView = reminders.last().ok_or_else(|| anyhow::anyhow!("reminder exceeds the native page limit"))?;
                next = Some(ReminderCursor { due_at_ms: last.due_at_ms, id: last.id.clone() });
                break;
            }
            bytes += size;
            reminders.push(reminder);
        }
        Ok(ReminderPage { reminders, next })
    }

    pub(crate) fn insert_reminder(&self, record: &TimeReminder) -> Result<()> {
        anyhow::ensure!(!self.conn().is_autocommit(), "reminder creation requires a write scope");
        let view = &record.view;
        let creator = record.created_by.as_ref().map(serde_json::to_string).transpose()?;
        self.conn().execute(
            "INSERT INTO reminders(id,request_id,target_name,target_incarnation_bits,due_at_ms,message,state,created_at_ms,created_by) VALUES (?1,?2,?3,?4,?5,?6,'pending',?7,?8)",
            params![view.id, record.request_id.0, view.target.canonical_name, view.target.incarnation_bits.to_string(), view.due_at_ms, view.message, record.created_at_ms, creator],
        )?;
        Ok(())
    }

    pub(crate) fn complete_reminder(&self, id: &str, now: i64, state: ReminderState, event: Option<i64>, reason: Option<&str>) -> Result<bool> {
        anyhow::ensure!(!self.conn().is_autocommit(), "reminder completion requires a write scope");
        let state = match state {
            ReminderState::Enqueued => "enqueued",
            ReminderState::Failed => "failed",
            ReminderState::Canceled => "canceled",
            ReminderState::Pending => anyhow::bail!("reminder completion requires a terminal state"),
        };
        Ok(self.conn().execute("UPDATE reminders SET state=?2,completed_at_ms=?3,delivery_event_id=?4,failure_reason=?5 WHERE id=?1 AND state='pending'", params![id, state, now, event, reason])? == 1)
    }

    pub(crate) fn due_reminder_keys(&self, now: i64, after: &ReminderKey, limit: i64) -> Result<Vec<ReminderKey>> {
        let mut stmt = self.conn().prepare_cached("SELECT due_at_ms,id FROM reminders WHERE state='pending' AND (due_at_ms,id)>(?1,?2) AND due_at_ms<=?3 ORDER BY due_at_ms,id LIMIT ?4")?;
        Ok(stmt.query_map(params![after.0, after.1, now, limit], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?)
    }

    pub(crate) fn has_pending_reminders(&self) -> Result<bool> {
        Ok(self.conn().query_row("SELECT EXISTS(SELECT 1 FROM reminders WHERE state='pending')", [], |row| row.get(0))?)
    }

    pub(crate) fn enqueued_reminder_keys(&self, after: &ReminderKey, through: i64, limit: i64) -> Result<Vec<ReminderKey>> {
        let mut stmt = self.conn().prepare_cached("SELECT completed_at_ms,id FROM reminders WHERE state='enqueued' AND (completed_at_ms,id)>(?1,?2) AND delivery_event_id<=?3 ORDER BY completed_at_ms,id LIMIT ?4")?;
        Ok(stmt.query_map(params![after.0, after.1, through, limit], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?)
    }
}
