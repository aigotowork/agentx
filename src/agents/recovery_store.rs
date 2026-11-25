//! Orders model failure, turn-end evidence and one exclusive continuation.
use super::{
    attachments::ParticipantRef,
    labels,
    model_request::{ERROR_CONTEXT, ModelRequestState, RequestPhase},
    recovery::*,
};
use crate::{
    db::{AgentxDb, InstanceRow},
    service::{RequestContext, ServiceError, ServiceResult},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// A queued request expires rather than typing later after a user leaves the terminal.
const WINDOW_SECONDS: i64 = 10;

#[derive(Deserialize, Serialize)]
struct Episode {
    participant: ParticipantRef,
    session: String,
    unresolved: bool,
    failure: Option<String>,
    stopped: bool,
    attempt: Option<StoredAttempt>,
}

#[derive(Deserialize, Serialize)]
struct StoredAttempt {
    ticket: Ticket,
    until: i64,
    phase: Phase,
    enter_granted: bool,
}

fn key(name: &str) -> String {
    format!("model_recovery:{name}")
}
fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}
fn read(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<Option<Episode>> {
    let episode = db.kv_get(&key(&participant.canonical_name)).map_err(internal)?.map(|value| serde_json::from_str::<Episode>(&value).map_err(internal)).transpose()?;
    Ok(episode.filter(|episode| &episode.participant == participant))
}
fn save(db: &AgentxDb, episode: &Episode) -> ServiceResult<()> {
    db.kv_set(&key(&episode.participant.canonical_name), Some(&serde_json::to_string(episode).map_err(internal)?)).map_err(internal)
}
fn now() -> i64 {
    crate::shared::time::now_epoch_i64()
}
fn exact(row: &InstanceRow) -> ParticipantRef {
    ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }
}
fn supported(row: &InstanceRow) -> bool {
    row.tool == "codex" && row.origin_device_id.is_none()
}
fn failed(row: &InstanceRow, session: &str) -> bool {
    supported(row)
        && row.session_id.as_deref() == Some(session)
        && row.status == "error"
        && row.status_context == ERROR_CONTEXT
        && row.model_request.as_ref().is_some_and(|state| state.phase == RequestPhase::Failed && state.session_id == session && state.error.is_some())
}
fn eligible(row: &InstanceRow, episode: &Episode) -> bool {
    if episode.failure.is_none() || !failed(row, &episode.session) || super::cleanup_store::tracked_process_state(row) != super::service::TrackedProcessState::Alive {
        return false;
    }
    // A provider error can end the turn without delivering the Stop hook.
    episode.stopped
        || row.model_request.as_ref().is_some_and(|model| {
            let Some(error) = &model.error else { return false };
            error.turn_id == model.turn_id
                && crate::transcript::codex_completion::latest_turn_completion(std::path::Path::new(&row.transcript_path), &episode.session, &model.turn_id).is_some_and(|completed| completed >= error.last_error_at.saturating_mul(1000))
        })
}

fn pty_host(db: &AgentxDb, row: &InstanceRow) -> bool {
    row.parent_name.as_deref().is_none_or(str::is_empty) && db.has_notify_endpoint_kind(&row.name, "pty") && db.has_notify_endpoint_kind(&row.name, "inject")
}

fn live_host(db: &AgentxDb, row: &InstanceRow, participant: &ParticipantRef) -> ServiceResult<Option<crate::runtime::notify::WakeKind>> {
    use crate::runtime::notify::WakeKind;
    if crate::runtime::hosts_store::direct_recovery_session(db, row, participant)?.is_some() {
        return Ok(Some(WakeKind::Hook));
    }
    Ok((pty_host(db, row) && crate::runtime::hosts_store::delivery_progress(db, participant)?.is_some_and(|host| host.process_alive == Some(true) && host.last_poll.is_some() && row.pid == Some(i64::from(host.child.pid)))).then_some(WakeKind::Pty))
}

fn invalidate(episode: &mut Episode) {
    episode.stopped = false;
    episode.failure = None;
    if episode.attempt.as_ref().is_some_and(|attempt| attempt.phase == Phase::Queued) {
        episode.attempt = None;
    }
}

/// Record fresh failure identity in the same transaction as the model observation.
pub(super) fn observe_request(db: &AgentxDb, row: &InstanceRow, state: &ModelRequestState) -> ServiceResult<()> {
    if !supported(row) {
        return Ok(());
    }
    let participant = exact(row);
    let mut episode = match read(db, &participant)?.filter(|episode| episode.session == state.session_id) {
        Some(episode) => episode,
        None if state.phase == RequestPhase::Failed => Episode { participant, session: state.session_id.clone(), unresolved: false, failure: None, stopped: false, attempt: None },
        None => return Ok(()),
    };
    invalidate(&mut episode);
    if state.phase == RequestPhase::Failed {
        if !episode.unresolved {
            episode.attempt = None;
        }
        episode.failure = Some(Uuid::new_v4().to_string());
    }
    episode.unresolved = state.error.is_some();
    save(db, &episode)
}

fn observe_turn(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef, session: &str, turn: Option<&str>, stopped: bool) -> ServiceResult<()> {
    let invocation = &context.caller.invocation;
    if invocation.process_binding_id.is_none() {
        return Err(ServiceError::Forbidden("bound Codex hook process required".into()));
    }
    let row = super::hook_store::observation_actor(db, context, participant, Some(session))?;
    if !supported(&row) || row.session_id.as_deref() != Some(session) {
        return Ok(());
    }
    update_turn(db, &row, session, turn, stopped)
}

fn update_turn(db: &AgentxDb, row: &InstanceRow, session: &str, turn: Option<&str>, stopped: bool) -> ServiceResult<()> {
    let participant = exact(row);
    let Some(mut episode) = read(db, &participant)?.filter(|episode| episode.session == session) else { return Ok(()) };
    if stopped {
        if !failed(row, session) || episode.failure.is_none() || turn.is_some_and(|turn| row.model_request.as_ref().is_none_or(|state| state.turn_id != turn)) {
            return Ok(());
        }
        episode.stopped = true;
    } else {
        invalidate(&mut episode);
    }
    save(db, &episode)
}

fn matches_scope(row: &InstanceRow, selection: &Selection) -> bool {
    (selection.names.is_empty() || selection.names.contains(&row.name)) && crate::service::queries::AgentWorkspace::visible_for_selector(super::workspaces::workspace(row).as_deref(), selection.workspace.as_deref())
}

fn matches_labels(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef, selection: &Selection) -> ServiceResult<bool> {
    if selection.labels.is_empty() {
        return Ok(true);
    }
    let snapshot = labels::get(db, context, participant)?;
    Ok(selection.labels.iter().all(|predicate| predicate.matches(&snapshot.labels)))
}

fn normalize(mut selection: Selection) -> ServiceResult<Selection> {
    selection.labels = labels::normalize_selectors(&selection.labels)?;
    if selection.workspace.as_ref().is_some_and(|path| path.len() > 4096) || selection.names.iter().any(|name| name.len() > 128 || !super::identity::is_valid_name(name)) {
        return Err(ServiceError::InvalidInput("invalid recovery target or workspace".into()));
    }
    Ok(selection)
}

fn select(db: &AgentxDb, context: &RequestContext, selection: &Selection) -> ServiceResult<Vec<Candidate>> {
    let rows = if selection.names.is_empty() { db.iter_instances_full().map_err(internal)? } else { selection.names.iter().map(|name| db.get_instance_full(name).map_err(internal)).collect::<ServiceResult<Vec<_>>>()?.into_iter().flatten().collect() };
    let mut result = Vec::new();
    for row in rows {
        if !supported(&row) || row.status_context != ERROR_CONTEXT || !matches_scope(&row, selection) {
            continue;
        }
        let participant = exact(&row);
        let Some(episode) = read(db, &participant)? else { continue };
        if !eligible(&row, &episode) || live_host(db, &row, &participant)?.is_none() || !matches_labels(db, context, &participant, selection)? {
            continue;
        }
        let error = row.model_request.as_ref().and_then(|state| state.error.as_ref()).expect("eligible model error");
        result.push(Candidate { target: Target { participant, failure: episode.failure.expect("eligible failure") }, reason: error.reason.chars().take(160).collect(), error_since: error.error_since });
    }
    result.sort_by(|a, b| a.target.participant.canonical_name.cmp(&b.target.participant.canonical_name));
    Ok(result)
}

fn submit(db: &AgentxDb, context: &RequestContext, target: Target, selection: Selection) -> ServiceResult<Attempt> {
    let row = db.get_instance_full(&target.participant.canonical_name).map_err(internal)?.filter(|row| exact(row) == target.participant).ok_or(ServiceError::StaleFence)?;
    let mut episode = read(db, &target.participant)?.ok_or(ServiceError::StaleFence)?;
    if !matches_scope(&row, &selection) || !matches_labels(db, context, &target.participant, &selection)? {
        return Err(ServiceError::Conflict("target no longer matches the selection".into()));
    }
    let time = now();
    let wake = if episode.failure.as_ref() == Some(&target.failure) && eligible(&row, &episode) { live_host(db, &row, &target.participant)? } else { None };
    let valid = wake.is_some();
    if let Some(attempt) = &episode.attempt
        && (attempt.ticket.target == target || valid)
        && (attempt.phase != Phase::Queued || time < attempt.until)
    {
        let phase = if attempt.phase == Phase::Issuing && time >= attempt.until { Phase::Unknown } else { attempt.phase };
        return Ok(Attempt { id: attempt.ticket.id.clone(), phase, replayed: true });
    }
    let wake = wake.ok_or(ServiceError::StaleFence)?;
    let result = Attempt { id: Uuid::new_v4().to_string(), phase: Phase::Queued, replayed: false };
    episode.attempt = Some(StoredAttempt { ticket: Ticket { target, id: result.id.clone() }, until: time + WINDOW_SECONDS, phase: Phase::Queued, enter_granted: false });
    save(db, &episode)?;
    db.wake_after_commit(&row.name, &[wake]);
    Ok(result)
}

pub(super) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    match request {
        Request::Select(selection) => transaction(db, || select(db, context, &normalize(selection)?).map(Response::Candidates)),
        Request::Submit { target, selection } => transaction(db, || submit(db, context, target, normalize(selection)?).map(Response::Accepted)),
        Request::ObserveTurn { participant, session, turn, stopped } => transaction(db, || observe_turn(db, context, &participant, &session, turn.as_deref(), stopped).map(|()| Response::Observed)),
    }
}

pub(crate) fn pending(db: &AgentxDb, row: &InstanceRow) -> ServiceResult<Option<Ticket>> {
    if !supported(row) || row.status_context != ERROR_CONTEXT || !pty_host(db, row) {
        return Ok(None);
    }
    pending_for_row(db, row)
}

fn pending_for_row(db: &AgentxDb, row: &InstanceRow) -> ServiceResult<Option<Ticket>> {
    if !supported(row) || row.status_context != ERROR_CONTEXT {
        return Ok(None);
    }
    let Some(episode) = read(db, &exact(row))? else { return Ok(None) };
    Ok(episode.attempt.as_ref().filter(|attempt| attempt.phase == Phase::Queued && now() < attempt.until && episode.failure.as_ref() == Some(&attempt.ticket.target.failure) && eligible(row, &episode)).map(|attempt| attempt.ticket.clone()))
}

/// Caller must validate the exact live PTY registration in this transaction.
pub(crate) fn host(db: &AgentxDb, row: &InstanceRow, ticket: Ticket, action: HostAction) -> ServiceResult<bool> {
    if exact(row) != ticket.target.participant {
        return Err(ServiceError::StaleFence);
    }
    let Some(mut episode) = read(db, &ticket.target.participant)? else { return Ok(false) };
    let valid = pty_host(db, row) && eligible(row, &episode) && episode.failure.as_ref() == Some(&ticket.target.failure);
    let Some(attempt) = episode.attempt.as_mut().filter(|attempt| attempt.ticket == ticket) else { return Ok(false) };
    match action {
        HostAction::Claim if valid && now() < attempt.until && attempt.phase == Phase::Queued => attempt.phase = Phase::Issuing,
        HostAction::Enter if valid && now() < attempt.until && attempt.phase == Phase::Issuing && !attempt.enter_granted => attempt.enter_granted = true,
        HostAction::Finish { submitted } if attempt.phase == Phase::Issuing && (!submitted || attempt.enter_granted) => {
            attempt.phase = if submitted { Phase::Submitted } else { Phase::Unknown };
        }
        _ => return Ok(false),
    }
    save(db, &episode)?;
    Ok(true)
}

/// Already inside the transaction that validates the direct worker registration.
pub(crate) fn native_host(db: &AgentxDb, row: &InstanceRow, session: &str, action: NativeAction) -> ServiceResult<NativeOutcome> {
    let (ticket, recorded) = match action {
        NativeAction::Poll => return Ok(NativeOutcome::Pending { ticket: pending_for_row(db, row)?, model_error: row.model_request.as_ref().is_some_and(|state| state.session_id == session && state.error.is_some()) }),
        NativeAction::FailedTurnEnded { turn } => {
            update_turn(db, row, session, Some(&turn), true)?;
            return Ok(NativeOutcome::Changed(true));
        }
        NativeAction::Begin(ticket) => (ticket, None),
        NativeAction::Finish { ticket, recorded } => (ticket, Some(recorded)),
    };
    if exact(row) != ticket.target.participant {
        return Err(ServiceError::StaleFence);
    }
    let Some(mut episode) = read(db, &ticket.target.participant)?.filter(|episode| episode.session == session) else { return Ok(NativeOutcome::Changed(false)) };
    let valid = eligible(row, &episode) && episode.failure.as_ref() == Some(&ticket.target.failure);
    let Some(attempt) = episode.attempt.as_mut().filter(|attempt| attempt.ticket == ticket) else { return Ok(NativeOutcome::Changed(false)) };
    match recorded {
        None if valid && now() < attempt.until && attempt.phase == Phase::Queued => attempt.phase = Phase::Issuing,
        Some(recorded) if attempt.phase == Phase::Issuing => attempt.phase = if recorded { Phase::Submitted } else { Phase::Unknown },
        _ => return Ok(NativeOutcome::Changed(false)),
    }
    save(db, &episode)?;
    Ok(NativeOutcome::Changed(true))
}

#[cfg(test)]
#[test]
fn missing_stop_hook_requires_current_turn_completion() {
    use serde_json::json;
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let mut rollout = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let pid = std::process::id();
    let model = json!({"session_id":"session","turn_id":"turn","model":"fixture","phase":"failed","retired_turns":[],
        "error":{"turn_id":"turn","source":"hook","reason":"403","error_since":1,"last_error_at":1,"consecutive_failures":1}});
    let fields = json!({"tool":"codex","pid":pid,"created_at":5.0,"status":"error","status_context":ERROR_CONTEXT,
        "session_id":"session","transcript_path":rollout.path(),"model_request":model.to_string(),
        "launch_context":json!({"pid_identity":crate::sys::process::identity(pid).unwrap()}).to_string()});
    db.save_instance_named("kera", fields.as_object().unwrap()).unwrap();
    let row = db.get_instance_full("kera").unwrap().unwrap();
    let episode = Episode { participant: exact(&row), session: "session".into(), unresolved: true, failure: Some("failure".into()), stopped: false, attempt: None };
    writeln!(rollout, "{}", json!({"type":"session_meta","payload":{"id":"session"}})).unwrap();
    assert!(!eligible(&row, &episode));
    writeln!(rollout, "{}", json!({"timestamp":"2026-11-20T08:09:10Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"turn"}})).unwrap();
    assert!(eligible(&row, &episode));
    writeln!(rollout, "{}", json!({"type":"event_msg","payload":{"type":"task_started","turn_id":"next"}})).unwrap();
    assert!(!eligible(&row, &episode));
}
