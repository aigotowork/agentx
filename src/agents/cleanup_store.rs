//! Internal participant cleanup policy used by actual read/launch adapters.
//! Only frozen plans leave this module; process signals, waits and pidtrack
//! writes belong to the response's originating host after commit.
#[cfg(test)]
mod tests;

use super::attachments::ParticipantRef;
use super::service::{CleanupOutcome, SystemCleanup, TrackedProcessState};
use crate::db::{AgentxDb, InstanceRow};
use crate::service::{RequestContext, ServiceError, ServiceResult};
use serde_json::json;

const SWEEP_INTERVAL: f64 = 30.0;
const SWEEP_KEY: &str = "_dead_process_sweep_at";

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn participant(row: &InstanceRow) -> ParticipantRef {
    ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }
}
fn peer(context: &RequestContext) -> ServiceResult<()> {
    if context.process.pid == 0 || context.process.incarnation.as_deref().is_none_or(str::is_empty) {
        return Err(ServiceError::Forbidden("authenticated cleanup caller required".into()));
    }
    Ok(())
}

/// Lack of access to process identity or liveness is not evidence of exit.
/// Clock policy separately protects any PID without positive exit evidence.
pub(crate) fn tracked_process_state(row: &InstanceRow) -> TrackedProcessState {
    if row.status == crate::shared::ST_LAUNCHING || super::records::is_launching_placeholder(row) || super::records::is_remote_instance(row) {
        return TrackedProcessState::Unverifiable;
    }
    let (Some(pid), Some(expected)) = (row.pid.and_then(|pid| u32::try_from(pid).ok()).filter(|pid| *pid > 0), row.pid_identity().filter(|identity| !identity.is_empty())) else {
        return TrackedProcessState::Unverifiable;
    };
    match crate::sys::process::identity(pid) {
        Some(actual) if !actual.is_empty() => {
            if actual == expected {
                TrackedProcessState::Alive
            } else {
                TrackedProcessState::Gone
            }
        }
        _ if crate::sys::process::has_exited(pid) => TrackedProcessState::Gone,
        _ => TrackedProcessState::Unverifiable,
    }
}

fn append(outcome: &mut CleanupOutcome, root: &InstanceRow, stopped: Option<super::hook_service::SessionStopOutcome>) {
    match stopped {
        Some(stopped) => {
            outcome.retired += usize::from(stopped.stopped);
            outcome.effects.extend(stopped.effects);
        }
        None => {
            let fence = participant(root);
            if !outcome.deferred.contains(&fence) {
                outcome.deferred.push(fence);
            }
        }
    }
}

fn reap(db: &AgentxDb, context: &RequestContext, row: &InstanceRow, outcome: &mut CleanupOutcome) -> ServiceResult<bool> {
    if tracked_process_state(row) != TrackedProcessState::Gone {
        return Ok(false);
    }
    let stopped = super::hook_store::cleanup_stop(db, context, row, "process_exit", false, true)?;
    append(outcome, row, stopped);
    Ok(true)
}

fn sweep_due(db: &AgentxDb, now: f64) -> ServiceResult<bool> {
    let last = db.kv_get(SWEEP_KEY).map_err(internal)?.and_then(|value| value.parse::<f64>().ok()).unwrap_or(0.0);
    Ok(!last.is_finite() || last > now || now - last >= SWEEP_INTERVAL)
}

fn cleanup_remote(db: &AgentxDb, now: f64) -> ServiceResult<()> {
    let times: std::collections::HashMap<_, _> = db.kv_prefix("relay_sync_time_").map_err(internal)?.into_iter().collect();
    for row in db.iter_instances_full().map_err(internal)? {
        let Some(device) = &row.origin_device_id else {
            continue;
        };
        let last = times.get(&format!("relay_sync_time_{device}")).and_then(|value| value.parse::<f64>().ok()).unwrap_or(0.0);
        if last > 0.0 && now - last <= 90.0 {
            continue;
        }
        // Remote presence expiry never grants local process authority.
        db.conn().execute("DELETE FROM instances WHERE name=?1 AND created_at=?2 AND origin_device_id=?3", rusqlite::params![row.name, row.created_at, device]).map_err(internal)?;
    }
    Ok(())
}

fn cleanup(db: &AgentxDb, context: &RequestContext, mode: &SystemCleanup) -> ServiceResult<CleanupOutcome> {
    let mut outcome = CleanupOutcome::default();
    if let SystemCleanup::Resume { participant: fence } = mode {
        let row = db.get_instance_full(&fence.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        if participant(&row) != *fence || super::records::is_remote_instance(&row) {
            return Err(ServiceError::StaleFence);
        }
        reap(db, context, &row, &mut outcome)?;
        return Ok(outcome);
    }
    let now = crate::shared::time::now_epoch_f64();
    let listing = matches!(mode, SystemCleanup::Listing);
    let dead = matches!(mode, SystemCleanup::DeadProcesses);
    let due = (listing || dead) && sweep_due(db, now)?;
    if dead && !due {
        return Ok(outcome);
    }
    let grace = listing && super::lifecycle::is_in_wake_grace_shared(db);
    if listing && !grace {
        cleanup_remote(db, now)?;
    }
    for observed in db.iter_instances_full().map_err(internal)? {
        let Some(row) = db.get_instance_full(&observed.name).map_err(internal)? else {
            continue;
        };
        if participant(&row) != participant(&observed) {
            return Err(ServiceError::StaleFence);
        }
        if super::records::is_remote_instance(&row) {
            continue;
        }
        if (listing || dead) && reap(db, context, &row, &mut outcome)? {
            continue;
        }
        if dead {
            continue;
        }
        if super::records::is_launching_placeholder(&row) && row.created_at > 0.0 && now - row.created_at > super::lifecycle::CLEANUP_PLACEHOLDER_THRESHOLD as f64 {
            append(&mut outcome, &row, super::hook_store::cleanup_stop(db, context, &row, "stale_cleanup", true, false)?);
            continue;
        }
        if !listing || grace {
            continue;
        }
        // This projection only uses persisted status/heartbeat, never a log
        // read or terminal capture while the cleanup write lock is held.
        let computed = super::lifecycle::get_instance_status_stored(&row, db);
        if computed.status != crate::shared::ST_INACTIVE {
            continue;
        }
        let reason = if matches!(computed.context.as_str(), "killed" | "closed" | "timeout" | "interrupted" | "session_switch") && computed.age_seconds > 60 {
            "exit_cleanup"
        } else if computed.context == "stale" && computed.age_seconds > 3600 {
            "stale_cleanup"
        } else if computed.age_seconds > 3600 {
            "inactive_cleanup"
        } else {
            continue;
        };
        if reason != "exit_cleanup"
            && let Some(pid) = row.pid
        {
            // Legacy/unknown identity must keep a possibly running process.
            let Some(pid) = u32::try_from(pid).ok().filter(|pid| *pid > 0) else {
                continue;
            };
            if !crate::sys::process::has_exited(pid) {
                continue;
            }
        }
        let guarded = reason != "exit_cleanup" && row.pid.is_some() && row.pid_identity().is_some_and(|identity| !identity.is_empty());
        append(&mut outcome, &row, super::hook_store::cleanup_stop(db, context, &row, reason, false, guarded)?);
    }
    if due {
        db.prune_orphan_endpoints(60.0).map_err(internal)?;
        db.kv_set(SWEEP_KEY, Some(&now.to_string())).map_err(internal)?;
    }
    Ok(outcome)
}

/// The request adapter supplies an authenticated observed peer but no self
/// actor. The system's finite policy, not an arbitrary caller target/reason,
/// decides admission. A retry recovers its original frozen cleanup plan.
pub(crate) fn cleanup_system(db: &AgentxDb, context: &RequestContext, mode: SystemCleanup) -> ServiceResult<CleanupOutcome> {
    peer(context)?;
    let outcome = super::service_store::retained(db, context, &json!({"system_cleanup":mode}), || cleanup_in_write_scope(db, context, &mode))?;
    super::hook_store::validate_stop_outcome(db, &super::hook_service::SessionStopOutcome { stopped: outcome.retired > 0, retained: false, effects: outcome.effects.clone() })?;
    Ok(outcome)
}

/// Compose with an already-retained native lifecycle operation. Its caller
/// owns the full intent/result acceptance, so this core creates no second key.
pub(super) fn cleanup_in_write_scope(db: &AgentxDb, context: &RequestContext, mode: &SystemCleanup) -> ServiceResult<CleanupOutcome> {
    peer(context)?;
    if db.conn().is_autocommit() {
        return Err(ServiceError::Internal("system cleanup requires its adapter transaction".into()));
    }
    cleanup(db, context, mode)
}
