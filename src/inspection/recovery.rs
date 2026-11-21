//! Causal guard for an atomic continuation mailbox request, never a Provider restart.
use super::contract::{Episode, RecoveryPolicy};
use crate::agents::model_request::{ERROR_CONTEXT, RequestPhase};
use crate::db::{AgentxDb, InstanceRow};
use crate::management::contract::AgentRef;
use crate::service::{ServiceError, ServiceResult};

pub(crate) fn episode(row: &InstanceRow) -> Option<Episode> {
    let state = row.model_request.as_ref()?;
    let error = state.error.as_ref()?;
    Some(Episode {
        agent: AgentRef { name: row.name.clone(), incarnation: Some(row.created_at.to_bits().to_string()), stopped_event: None },
        session_id: state.session_id.clone(),
        source: error.source.clone(),
        failing_turn: error.turn_id.clone(),
        error_since: error.error_since,
    })
}

/// Repeated failing turns share one unresolved episode barrier. JSON encoding avoids delimiter aliases.
pub(crate) fn barrier(episode: &Episode) -> String {
    serde_json::to_string(&(&episode.agent, &episode.session_id, &episode.source, episode.error_since)).expect("episode tuple serializes")
}

pub(crate) fn eligible(db: &AgentxDb, row: &InstanceRow, policy: &RecoveryPolicy, expected: &Episode, now: i64) -> ServiceResult<()> {
    let refuse = |reason: &str| ServiceError::Conflict(reason.into());
    if !policy.enabled {
        return Err(refuse("continuation recovery policy is disabled"));
    }
    let current = episode(row).ok_or_else(|| refuse("no unresolved native request error"))?;
    if current.agent != expected.agent || current.session_id != expected.session_id || current.source != expected.source || current.error_since != expected.error_since || current.failing_turn != expected.failing_turn {
        return Err(ServiceError::StaleFence);
    }
    if row.origin_device_id.is_some() || row.session_id.as_deref() != Some(expected.session_id.as_str()) || expected.source != "model_request" {
        return Err(ServiceError::StaleFence);
    }
    let state = row.model_request.as_ref().ok_or(ServiceError::StaleFence)?;
    let status = crate::agents::lifecycle::get_instance_status_stored(row, db);
    if row.status != "error" || row.status_context != ERROR_CONTEXT || status.status != "error" || row.status_context.starts_with("exit:") {
        return Err(refuse("instance is not currently in native request error"));
    }
    if state.phase != RequestPhase::Failed || state.turn_id != expected.failing_turn {
        return Err(refuse("request is retrying, cancelled, completed or has changed turn"));
    }
    let alive = row.pid.and_then(|pid| u32::try_from(pid).ok()).filter(|pid| *pid > 0).is_some_and(|pid| !crate::sys::process::has_exited(pid) && !row.pid_reused(pid));
    if !alive {
        return Err(refuse("current instance process is not alive"));
    }
    if expected.error_since < 0 || now < expected.error_since || now.saturating_sub(expected.error_since) < i64::from(policy.error_duration_seconds) {
        return Err(refuse("native unresolved error has not reached the configured duration"));
    }
    Ok(())
}
