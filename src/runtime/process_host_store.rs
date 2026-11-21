//! Daemon-only process-host persistence. The caller owns terminal I/O and the
//! child handle; all writes retain participant, peer and process fences.

use crate::agents::attachments::ParticipantRef;
use crate::agents::service::resolve_actor;
use crate::db::AgentxDb;
use crate::service::{RequestContext, ServiceError, ServiceResult};
use crate::shared::{ST_ACTIVE, ST_BLOCKED};

use super::hosts::{ProcessRegistration, Request, Response};

#[derive(serde::Serialize, serde::Deserialize)]
struct RegisteredProcess {
    registration: ProcessRegistration,
    pid: u32,
    incarnation: String,
    #[serde(default)]
    last_poll: Option<i64>,
    #[serde(default)]
    protocol: Option<u32>,
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

fn key(participant: &ParticipantRef) -> String {
    format!("runtime_process:{}", participant.canonical_name)
}

fn verified_peer(context: &RequestContext) -> ServiceResult<&str> {
    if context.process.pid == 0 {
        return Err(ServiceError::Forbidden("host process is unavailable".into()));
    }
    context.process.incarnation.as_deref().filter(|identity| !identity.is_empty()).ok_or_else(|| ServiceError::Forbidden("host process incarnation is unavailable".into()))
}

fn load(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<Option<RegisteredProcess>> {
    db.kv_get(&key(participant)).map_err(internal)?.map(|raw| serde_json::from_str(&raw).map_err(internal)).transpose()
}

pub(crate) fn delivery_progress(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<Option<super::hosts::DeliveryHostProgress>> {
    Ok(load(db, participant)?.filter(|stored| stored.registration.participant == *participant).map(|stored| {
        let observed = db.poll_progress.last(&stored.registration.token);
        super::hosts::DeliveryHostProgress {
            pid: stored.pid,
            incarnation: stored.incarnation.clone(),
            child: stored.registration.child,
            protocol: observed.map(|_| crate::daemon::PROTOCOL_VERSION).or(stored.protocol),
            last_poll: observed.or(stored.last_poll),
            process_alive: if crate::sys::process::has_exited(stored.pid) { Some(false) } else { crate::sys::process::identity(stored.pid).map(|identity| identity == stored.incarnation) },
        }
    }))
}

fn polling_process(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef) -> ServiceResult<Option<RegisteredProcess>> {
    Ok(load(db, participant)?.filter(|stored| stored.registration.participant == *participant && stored.pid == context.process.pid && context.process.incarnation.as_deref() == Some(stored.incarnation.as_str())))
}

pub(crate) fn poll_registration_token(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef) -> ServiceResult<Option<String>> {
    Ok(polling_process(db, context, participant)?.map(|stored| stored.registration.token))
}

/// Called within the snapshot transaction. Only the exact registered host may
/// publish progress; ordinary hooks and operators cannot make it look healthy.
pub(crate) fn observe_delivery_poll(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef) -> ServiceResult<()> {
    let Some(mut stored) = polling_process(db, context, participant)? else { return Ok(()) };
    let now = crate::shared::time::now_epoch_i64();
    if stored.last_poll != Some(now) || stored.protocol != Some(crate::daemon::PROTOCOL_VERSION) {
        stored.last_poll = Some(now);
        stored.protocol = Some(crate::daemon::PROTOCOL_VERSION);
        db.kv_set(&key(participant), Some(&serde_json::to_string(&stored).map_err(internal)?)).map_err(internal)?;
    }
    Ok(())
}

/// The normal, already fenced heartbeat also checkpoints recent read progress.
pub(crate) fn checkpoint_delivery_poll(db: &AgentxDb, registration: &ProcessRegistration) -> ServiceResult<()> {
    let Some(last) = db.poll_progress.last(&registration.token) else { return Ok(()) };
    let Some(mut stored) = load(db, &registration.participant)?.filter(|stored| stored.registration == *registration) else { return Ok(()) };
    if stored.last_poll.is_some_and(|old| (0..30).contains(&last.saturating_sub(old))) && stored.protocol == Some(crate::daemon::PROTOCOL_VERSION) {
        return Ok(());
    }
    stored.last_poll = Some(last);
    stored.protocol = Some(crate::daemon::PROTOCOL_VERSION);
    db.kv_set(&key(&registration.participant), Some(&serde_json::to_string(&stored).map_err(internal)?)).map_err(internal)
}

pub(crate) fn validate(db: &AgentxDb, context: &RequestContext, registration: &ProcessRegistration) -> ServiceResult<crate::agents::service::ResolvedActor> {
    let incarnation = verified_peer(context)?;
    let actor = resolve_actor(db, context)?;
    let stored = load(db, &registration.participant)?.ok_or(ServiceError::StaleFence)?;
    let identity = db.get_instance_pid_identity(&registration.participant.canonical_name).map_err(internal)?;
    if actor.participant != registration.participant
        || stored.registration != *registration
        || stored.pid != context.process.pid
        || stored.incarnation != incarnation
        || actor.record.pid != Some(i64::from(registration.child.pid))
        || identity != registration.child.incarnation
    {
        return Err(ServiceError::StaleFence);
    }
    Ok(actor)
}

/// Exit observations can outlive SessionEnd's row/binding removal. Retain the
/// original authenticated host and child token, and reject any successor row.
/// This does not authorize endpoint, status or other live process mutations.
pub(crate) fn validate_exit(db: &AgentxDb, context: &RequestContext, registration: &ProcessRegistration) -> ServiceResult<()> {
    let incarnation = verified_peer(context)?;
    let stored = load(db, &registration.participant)?.ok_or(ServiceError::StaleFence)?;
    if stored.registration != *registration || stored.pid != context.process.pid || stored.incarnation != incarnation {
        return Err(ServiceError::StaleFence);
    }
    let name = &registration.participant.canonical_name;
    if let Some(row) = db.get_instance_full(name).map_err(internal)? {
        let identity = db.get_instance_pid_identity(name).map_err(internal)?;
        if row.created_at.to_bits() != registration.participant.incarnation_bits || row.pid != Some(i64::from(registration.child.pid)) || identity != registration.child.incarnation {
            return Err(ServiceError::StaleFence);
        }
    }
    Ok(())
}

/// Called inside the daemon's write scope. No external writes, waits or managed
/// database opens occur here. Child identity is read before storing its PID.
pub(super) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    let incarnation = verified_peer(context)?;
    match request {
        Request::RegisterProcess { participant, child, launch_context } => {
            let actor = resolve_actor(db, context)?;
            // PrepareKill freezes the current PID before granting host effects.
            // A delayed PTY registration must not replace that process while
            // the operator completes or recovers its exact kill ticket.
            if actor.participant != participant || actor.record.status_context == "exit:killed" {
                return Err(ServiceError::StaleFence);
            }
            if child.pid == 0 || !crate::sys::process::is_alive(child.pid) {
                return Err(ServiceError::InvalidInput("child process is unavailable".into()));
            }
            let observed = crate::sys::process::identity(child.pid);
            if child.incarnation != observed {
                return Err(ServiceError::StaleFence);
            }
            let current = load(db, &participant)?;
            if let Some(current) = &current
                && current.registration.participant == participant
                && (current.pid != context.process.pid || current.incarnation != incarnation)
                && !crate::sys::process::has_exited(current.pid)
                && crate::sys::process::identity(current.pid).as_deref().is_none_or(|identity| identity == current.incarnation)
            {
                return Err(ServiceError::Conflict("participant already has a live process host".into()));
            }
            let registration = match current {
                Some(current) if current.pid == context.process.pid && current.incarnation == incarnation && current.registration.participant == participant && current.registration.child == child => current.registration,
                _ => ProcessRegistration { token: uuid::Uuid::new_v4().to_string(), participant, child },
            };
            let name = &registration.participant.canonical_name;
            db.update_instance_pid_with_identity(name, registration.child.pid, registration.child.incarnation.as_deref()).map_err(internal)?;
            if let Some(launch_context) = launch_context {
                db.store_launch_context(name, &launch_context).map_err(internal)?;
            }
            let stored = RegisteredProcess { registration: registration.clone(), pid: context.process.pid, incarnation: incarnation.to_owned(), last_poll: None, protocol: None };
            db.kv_set(&key(&registration.participant), Some(&serde_json::to_string(&stored).map_err(internal)?)).map_err(internal)?;
            let display_name = crate::agents::launch_title::pane_display_from_record(&actor.record);
            Ok(Response::ProcessRegistered { registration, display_name })
        }
        Request::CaptureProcessContext { registration, launch_context } => {
            validate(db, context, &registration)?;
            db.store_launch_context(&registration.participant.canonical_name, &launch_context).map_err(internal)?;
            Ok(Response::Renewed)
        }
        Request::SetProcessEndpoint { registration, kind, port } => {
            validate(db, context, &registration)?;
            if port == 0 {
                return Err(ServiceError::InvalidInput("process endpoint port must be nonzero".into()));
            }
            db.upsert_notify_endpoint(&registration.participant.canonical_name, kind.as_str(), port).map_err(internal)?;
            Ok(Response::Renewed)
        }
        Request::ReleaseProcessEndpoint { registration, kind, port } => {
            validate(db, context, &registration)?;
            db.conn().execute("DELETE FROM notify_endpoints WHERE instance = ?1 AND kind = ?2 AND port = ?3", rusqlite::params![registration.participant.canonical_name, kind.as_str(), port]).map_err(internal)?;
            Ok(Response::Released)
        }
        Request::ObserveApproval { participant, approval } => {
            let actor = resolve_actor(db, context)?;
            if actor.participant != participant {
                return Err(ServiceError::StaleFence);
            }
            let already_blocked = actor.record.status == ST_BLOCKED && actor.record.status_context == "pty:approval";
            let edge = if approval { (!already_blocked).then_some((ST_BLOCKED, "pty:approval")) } else { already_blocked.then_some((ST_ACTIVE, "pty:approval_cleared")) };
            let Some((status, status_context)) = edge else {
                return Ok(Response::ProcessStatus(actor.record.status));
            };
            // Keep the gated-command detail and publish the paired status event
            // in the same transaction. Never clear a newer hook-owned state.
            db.set_status(&participant.canonical_name, status, status_context).map_err(internal)?;
            let mut data = serde_json::json!({"status": status, "context": status_context, "position": actor.record.last_event_id});
            if !actor.record.status_detail.is_empty() {
                data["detail"] = serde_json::json!(actor.record.status_detail);
            }
            db.log_event("status", &participant.canonical_name, &data).map_err(internal)?;
            Ok(Response::ProcessStatus(status.to_owned()))
        }
        _ => Err(ServiceError::Internal("unexpected process host operation".into())),
    }
}
