//! Daemon-only session lifecycle policy. One write scope owns binding, exact
//! descendant retirement and retained outcomes; hosts execute frozen effects.
#[cfg(test)]
mod operator_tests;
#[cfg(test)]
mod retirement_tests;
#[cfg(test)]
mod session_tests;

use super::attachments::{AttachmentStore, ParticipantRef, SqliteAttachmentStore};
use super::hook_service::*;
use super::service::{ResolvedActor, resolve_actor};
use crate::db::{AgentxDb, InstanceRow};
use crate::service::{RequestContext, ServiceError, ServiceResult};
use rusqlite::params;
use serde_json::json;
use std::collections::HashSet;

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn participant(row: &InstanceRow) -> ParticipantRef {
    ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }
}
fn current(db: &AgentxDb, fence: &ParticipantRef) -> ServiceResult<InstanceRow> {
    let row = db.get_instance_full(&fence.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    if participant(&row) != *fence {
        return Err(ServiceError::StaleFence);
    }
    Ok(row)
}
fn peer(context: &RequestContext) -> ServiceResult<()> {
    if context.process.pid == 0 || context.process.incarnation.as_deref().is_none_or(str::is_empty) {
        return Err(ServiceError::Forbidden("authenticated hook process required".into()));
    }
    Ok(())
}

pub(crate) fn observe_session(db: &AgentxDb, context: &RequestContext, input: HookSession) -> ServiceResult<ResolvedActor> {
    peer(context)?;
    if input.session_id.is_empty() || input.tool.parse::<crate::integrations::tool::Tool>().is_err() {
        return Err(ServiceError::InvalidInput("hook tool and session required".into()));
    }
    if context.caller.invocation.process_binding_id.as_deref().is_none_or(str::is_empty) {
        return Err(ServiceError::IdentityRequired("hook process binding required".into()));
    }
    if input.create_if_unbound && !matches!(input.tool.as_str(), "qoder" | "gemini" | "antigravity" | "kimi") {
        return Err(ServiceError::Forbidden("tool cannot create an unbound hook identity".into()));
    }
    if context.caller.invocation.environment.get("AGENTX_TOOL").is_some_and(|tool| !tool.is_empty() && tool != &input.tool) {
        return Err(ServiceError::StaleFence);
    }
    let mut effective = context.clone();
    if let Some(directory) = &input.directory {
        effective.caller.invocation.cwd = directory.clone();
    }
    let actor = super::service_store::retained(db, context, &json!({"hook_session":input}), || {
        let actor = super::invocation_binding::bind_session_with_creation(db, &effective, &input.session_id, &input.tool, input.create_if_unbound)?.ok_or_else(|| ServiceError::IdentityRequired("no participant bound to this hook process".into()))?;
        let name = &actor.record.name;
        db.rebind_instance_session(name, &input.session_id).map_err(internal)?;
        let maximum = db.get_last_event_id();
        let cursor = if actor.record.last_event_id == 0 {
            effective.caller.invocation.environment.get("AGENTX_LAUNCH_EVENT_ID").and_then(|value| value.parse::<i64>().ok()).filter(|cursor| (0..=maximum).contains(cursor)).unwrap_or(maximum)
        } else {
            actor.record.last_event_id
        };
        let mut updates = json!({"tool":input.tool,"session_id":input.session_id,"last_event_id":cursor});
        if let Some(directory) = &input.directory {
            updates["directory"] = json!(directory.to_string_lossy());
        }
        if let Some(path) = &input.transcript_path {
            updates["transcript_path"] = json!(path);
        }
        db.update_instance_fields(name, updates.as_object().unwrap()).map_err(internal)?;
        super::invocation_binding::capture_launch_context(db, &effective, name)?;
        let record = db.get_instance_full(name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        Ok(ResolvedActor { participant: participant(&record), record })
    })?;
    if current(db, &actor.participant)?.status_context == "exit:killed" {
        return Err(ServiceError::StaleFence);
    }
    Ok(actor)
}

/// Observation authority is narrower than session creation: exact row and
/// actor, plus the complete live attachment when this is a managed root.
pub(super) fn observation_actor(db: &AgentxDb, context: &RequestContext, fence: &ParticipantRef, session: Option<&str>) -> ServiceResult<InstanceRow> {
    let row = current(db, fence)?;
    if resolve_actor(db, context)?.participant != *fence {
        return Err(ServiceError::StaleFence);
    }
    validate_observation(db, context, fence, &row, session)?;
    Ok(row)
}

fn validate_observation(db: &AgentxDb, context: &RequestContext, fence: &ParticipantRef, row: &InstanceRow, session: Option<&str>) -> ServiceResult<()> {
    let invocation = &context.caller.invocation;
    if invocation.environment.get("AGENTX_TOOL").is_some_and(|tool| !tool.is_empty() && tool != &row.tool) {
        return Err(ServiceError::StaleFence);
    }
    if session.is_some_and(|session| row.session_id.as_deref().filter(|value| !value.is_empty()).is_some_and(|bound| bound != session)) {
        return Err(ServiceError::StaleFence);
    }
    match SqliteAttachmentStore::new(db).current_for_participant(fence).map_err(internal)? {
        Some(attachment) => {
            if invocation.activation_target.as_ref() != Some(&attachment.target)
                || row.tool != attachment.session.provider
                || session.is_some_and(|session| session != attachment.session.session_id)
                || row.session_id.as_deref().filter(|value| !value.is_empty()).is_some_and(|session| session != attachment.session.session_id)
                || invocation.provider_session.as_ref().is_some_and(|session| session != &attachment.session)
            {
                return Err(ServiceError::StaleFence);
            }
        }
        None => {
            if invocation.activation_target.is_some() {
                return Err(ServiceError::StaleFence);
            }
            if let Some(claimed) = &invocation.provider_session
                && (claimed.provider != row.tool
                    || claimed.session_id.is_empty()
                    || session.is_some_and(|session| session != claimed.session_id)
                    || row.session_id.as_deref().filter(|value| !value.is_empty()).is_some_and(|session| session != claimed.session_id))
            {
                return Err(ServiceError::StaleFence);
            }
            if let Some(session) = session
                && db.get_session_binding(session).map_err(internal)?.is_some_and(|name| name != row.name)
            {
                return Err(ServiceError::StaleFence);
            }
        }
    }
    Ok(())
}

pub(crate) fn tool_activity(db: &AgentxDb, context: &RequestContext, input: ToolActivity) -> ServiceResult<()> {
    store_tool_activity(db, context, input, false)
}

fn store_tool_activity(db: &AgentxDb, context: &RequestContext, input: ToolActivity, verified: bool) -> ServiceResult<()> {
    peer(context)?;
    if input.tool_name.is_empty() || input.written_files.iter().any(String::is_empty) || (input.tool_name != "apply_patch" && !input.written_files.is_empty()) {
        return Err(ServiceError::InvalidInput("tool name and ordered apply_patch file paths required".into()));
    }
    db.with_write_scope(|| {
        if !verified {
            observation_actor(db, context, &input.participant, context.caller.invocation.provider_session.as_ref().map(|session| session.session_id.as_str()))?;
        }
        super::service_store::retained(db, context, &json!({"tool_activity":input}), || {
            let name = &input.participant.canonical_name;
            let status_context = format!("tool:{}", input.tool_name);
            let detail = input.written_files.first().map(String::as_str).unwrap_or(&input.detail);
            super::lifecycle::try_set_status(db, name, "active", &status_context, super::lifecycle::StatusUpdate { detail, tool_name: &input.tool_name, ..Default::default() }).map_err(internal)?;
            // Keep the row on its first file. Extra status events preserve the
            // existing collision-detection contract for every later patch path.
            if input.written_files.len() > 1 {
                let current = db.get_instance_full(name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
                for file in input.written_files.iter().skip(1) {
                    let mut event = json!({"status":current.status,"context":current.status_context,"detail":file});
                    if let Some(state) = &current.model_request {
                        event["model_request"] = json!(state);
                        if let Some(error) = &state.error {
                            event["detail"] = json!(error.reason);
                            event["file"] = json!(file);
                        }
                    }
                    db.log_event("status", name, &event).map_err(internal)?;
                }
            }
            Ok(())
        })
        .map_err(anyhow::Error::new)
    })
    .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

/// Valid only inside the current daemon dispatch. Derived from the actual
/// process binding and never constructed from a participant name on the wire.
pub(crate) struct CodexPtyActor(ResolvedActor);

pub(crate) fn resolve_codex_pty(db: &AgentxDb, context: &RequestContext, session: Option<&str>, transcript: Option<&str>, directory: &std::path::Path) -> ServiceResult<Option<CodexPtyActor>> {
    peer(context)?;
    let invocation = &context.caller.invocation;
    if invocation.explicit_name.is_some()
        || invocation.activation_target.is_some()
        || invocation.provider_session.is_some()
        || invocation.actor_capability.is_some()
        || invocation.environment.get("AGENTX_CODEX_SHARED").is_some_and(|value| value == "1")
        || invocation.process_binding_id.as_deref().is_none_or(str::is_empty)
    {
        return Err(ServiceError::Forbidden("managed Codex PTY binding required".into()));
    }
    if directory.as_os_str().is_empty() || session.is_some_and(str::is_empty) || transcript.is_some_and(str::is_empty) {
        return Err(ServiceError::InvalidInput("supplied Codex hook position values must be nonempty".into()));
    }
    // Resolve the opaque process binding directly, without the generic CLI's
    // name, shell capability or provider-session fallback precedence.
    let binding = invocation.process_binding_id.as_deref().expect("checked above");
    let name = db.get_process_binding(binding).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    let mut row = db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    if row.tool != "codex" || row.status_context == "exit:killed" {
        return Err(ServiceError::StaleFence);
    }
    if transcript.is_none() && (directory.file_name().is_some_and(|name| name == "memories" || name == "memories_v2") || session.is_some_and(|session| row.session_id.as_deref().filter(|bound| !bound.is_empty()).is_some_and(|bound| bound != session)))
    {
        return Ok(None);
    }
    let participant = participant(&row);
    // The current binding (not the launch reservation's old name) owns legal
    // SessionStart transitions into a restored or other canonical participant.
    validate_observation(db, context, &participant, &row, session)?;
    let mut updates = serde_json::Map::new();
    if let Some(session) = session.filter(|session| row.session_id.as_deref() != Some(*session)) {
        updates.insert("session_id".into(), json!(session));
        row.session_id = Some(session.to_owned());
    }
    if let Some(path) = transcript.filter(|path| row.transcript_path != *path) {
        updates.insert("transcript_path".into(), json!(path));
        row.transcript_path = path.to_owned();
    }
    if row.directory != directory.to_string_lossy() {
        updates.insert("directory".into(), json!(directory.to_string_lossy()));
        row.directory = directory.to_string_lossy().into_owned();
    }
    if !updates.is_empty() {
        // The daemon serializes the entire dispatch; this single UPDATE is
        // atomic and does not need a retained result or separate transaction.
        db.update_instance_fields(&name, &updates).map_err(internal)?;
    }
    Ok(Some(CodexPtyActor(ResolvedActor { participant, record: row })))
}

impl CodexPtyActor {
    pub(crate) fn participant(&self) -> &ParticipantRef {
        &self.0.participant
    }

    pub(crate) fn approval_blocked(&self) -> bool {
        self.0.record.status == "blocked" && self.0.record.status_context == "pty:approval"
    }

    pub(crate) fn status(&self, db: &AgentxDb, status: &str, context: &str) -> ServiceResult<()> {
        let tool_name = context.strip_prefix("approved:").unwrap_or("");
        super::lifecycle::try_set_status(db, &self.0.participant.canonical_name, status, context, super::lifecycle::StatusUpdate { tool_name, ..Default::default() }).map_err(internal)
    }

    pub(crate) fn tool_activity(&self, db: &AgentxDb, context: &RequestContext, tool_name: String, detail: String, written_files: Vec<String>) -> ServiceResult<()> {
        store_tool_activity(db, context, ToolActivity { participant: self.0.participant.clone(), tool_name, detail, written_files }, true)
    }

    pub(crate) fn model_request(&self, db: &AgentxDb, context: &RequestContext, input: super::model_request::ObserveModelRequest) -> ServiceResult<()> {
        super::model_request_store::observe_verified(db, context, input, &self.0.record).map(|_| ())
    }

    pub(crate) fn observe_recovery_turn(&self, db: &AgentxDb, context: &RequestContext, session: Option<String>, turn: Option<String>, stopped: bool) {
        if self.0.record.model_request.as_ref().is_none_or(|state| state.error.is_none()) {
            return;
        }
        if let Some(session) = session {
            let request = super::recovery::Request::ObserveTurn { participant: self.0.participant.clone(), session, turn, stopped };
            if let Err(error) = super::recovery_store::handle(db, context, request) {
                crate::log::log_warn("hooks", "codex.recovery_observation", &error.to_string());
            }
        }
    }
}

pub(crate) fn observe_position(db: &AgentxDb, context: &RequestContext, input: ObserveHookPosition) -> ServiceResult<ResolvedActor> {
    peer(context)?;
    if input.session_id.as_deref().is_some_and(str::is_empty) || input.transcript_path.as_deref().is_some_and(str::is_empty) || input.directory.as_ref().is_some_and(|path| path.as_os_str().is_empty()) {
        return Err(ServiceError::InvalidInput("supplied hook position values must be nonempty".into()));
    }
    if context.caller.invocation.activation_target.is_some() && input.session_id.is_none() {
        return Err(ServiceError::InvalidInput("attached hook position requires its incoming session".into()));
    }
    db.with_write_scope(|| {
        observation_actor(db, context, &input.participant, input.session_id.as_deref())?;
        super::service_store::retained(db, context, &json!({"hook_position":input}), || {
            let mut updates = serde_json::Map::new();
            if let Some(session) = &input.session_id {
                updates.insert("session_id".into(), json!(session));
            }
            if let Some(path) = &input.transcript_path {
                updates.insert("transcript_path".into(), json!(path));
            }
            if let Some(directory) = &input.directory {
                updates.insert("directory".into(), json!(directory.to_string_lossy()));
            }
            if !updates.is_empty() {
                db.update_instance_fields(&input.participant.canonical_name, &updates).map_err(internal)?;
            }
            Ok(())
        })?;
        let record = current(db, &input.participant)?;
        Ok(ResolvedActor { participant: input.participant.clone(), record })
    })
    .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

fn status(db: &AgentxDb, row: &InstanceRow, value: &str, context: &str) -> ServiceResult<()> {
    let mut updates = json!({"status":value,"status_context":context,"status_detail":"","status_time":crate::shared::time::now_epoch_i64()});
    if value == "listening" {
        updates["last_stop"] = json!(crate::shared::time::now_epoch_i64());
    }
    db.update_instance_fields(&row.name, updates.as_object().unwrap()).map_err(internal)?;
    db.log_event("status", &row.name, &json!({"status":value,"context":context,"position":row.last_event_id,"session":row.session_id})).map_err(internal)?;
    if row.status != value {
        db.wake_after_commit(&row.name, crate::runtime::notify::WakeKind::DELIVERY_LOOPS);
    }
    Ok(())
}

fn freeze(db: &AgentxDb, row: &InstanceRow, process_action: StopProcessAction) -> ServiceResult<RetiredParticipant> {
    let mut query = db.conn().prepare("SELECT process_id FROM process_bindings WHERE instance_name=? ORDER BY process_id").map_err(internal)?;
    let process_bindings = query.query_map([&row.name], |row| row.get(0)).map_err(internal)?.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)?;
    let mut query = db.conn().prepare("SELECT kind,port FROM notify_endpoints WHERE instance=? ORDER BY kind").map_err(internal)?;
    let endpoints = query.query_map([&row.name], |row| Ok((row.get(0)?, row.get(1)?))).map_err(internal)?.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)?;
    Ok(RetiredParticipant {
        participant: participant(row),
        record: row.clone(),
        process_identity: db.get_instance_pid_identity(&row.name).map_err(internal)?,
        scope: db.path().parent().unwrap_or(std::path::Path::new(".")).to_path_buf(),
        process_bindings,
        endpoints,
        process_action,
    })
}

/// Enumerate the complete child graph before any row is deleted. Both lineage
/// relationships matter; repeated links are deduplicated, cycles fail closed.
fn descendants(db: &AgentxDb, row: InstanceRow, depth: usize, visiting: &mut HashSet<String>, seen: &mut HashSet<String>, rows: &mut Vec<InstanceRow>) -> ServiceResult<()> {
    if depth >= 10 || visiting.contains(&row.name) {
        return Err(ServiceError::Conflict("session descendant cycle or depth limit".into()));
    }
    if seen.contains(&row.name) {
        return Ok(());
    }
    visiting.insert(row.name.clone());
    let mut query = db.conn().prepare("SELECT name FROM instances WHERE parent_name=?1 OR (?2 IS NOT NULL AND parent_session_id=?2) ORDER BY name").map_err(internal)?;
    // Native children can share their root's session id. Such an alias owns
    // only its named children, not every sibling in the root session.
    let child_session = row.session_id.as_deref().filter(|session| !session.is_empty() && row.parent_session_id.as_deref() != Some(*session));
    let names = query.query_map(params![row.name, child_session], |row| row.get::<_, String>(0)).map_err(internal)?.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)?;
    for name in names {
        let child = db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        descendants(db, child, depth + 1, visiting, seen, rows)?;
    }
    visiting.remove(&row.name);
    seen.insert(row.name.clone());
    rows.push(row);
    Ok(())
}

fn cleanup_actor(db: &AgentxDb, row: &InstanceRow) -> ServiceResult<()> {
    db.revoke_claude_actor_capabilities_for_instance(&row.name).map_err(internal)?;
    if row.parent_name.as_deref().is_none_or(str::is_empty)
        && let Some(session) = &row.session_id
    {
        db.revoke_claude_actor_capabilities_for_session(session).map_err(internal)?;
        db.clear_claude_session_validation(session).map_err(internal)?;
        db.kv_delete_prefix(&format!("subagent_stop_inflight:{session}:")).map_err(internal)?;
        db.kv_delete_prefix(&crate::integrations::hooks::claude::native::session_prefix(session)).map_err(internal)?;
    }
    Ok(())
}

/// The caller owns admission and the outer transaction. This core freezes all
/// effects before child-first retirement; an error leaves no partial stop plan.
pub(super) fn hard_stop(db: &AgentxDb, context: &RequestContext, row: &InstanceRow, by: &str, reason: &str) -> ServiceResult<SessionStopOutcome> {
    retire_tree(db, context, row, by, reason, false, None)?.ok_or_else(|| ServiceError::Internal("unguarded retirement was deferred".into()))
}

/// Daemon composition only. Lifecycle owns retained operator intent/result in
/// this transaction; ordinary hook self authority and kill tickets stay intact.
pub(crate) fn stop_operator(db: &AgentxDb, context: &RequestContext, fence: &ParticipantRef, initiator: &str, decision: &OperatorStop) -> ServiceResult<SessionStopOutcome> {
    peer(context)?;
    if db.conn().is_autocommit() {
        return Err(ServiceError::Internal("operator stop requires its lifecycle authority transaction".into()));
    }
    if initiator.is_empty() || initiator.len() > 255 {
        return Err(ServiceError::InvalidInput("bounded operator initiator required".into()));
    }
    let row = current(db, fence)?;
    if row.origin_device_id.is_some() {
        return Err(ServiceError::Forbidden("operator stop requires a local participant".into()));
    }
    let reason = match decision {
        OperatorStop::Participation { reason } => {
            if reason.is_empty() || reason.len() > 128 || reason == "killed" {
                return Err(ServiceError::InvalidInput("participation stop reason required; tracked kills use tickets".into()));
            }
            reason.as_str()
        }
        OperatorStop::KilledWithoutProcess => {
            if row.pid.is_some() {
                return Err(ServiceError::InvalidInput("tracked process kill requires its native kill ticket".into()));
            }
            "killed"
        }
    };
    // Keep a nested rollback scope even when an internal caller catches an
    // error; no partial child retirement/marker may escape this core.
    db.with_write_scope(|| {
        if matches!(decision, OperatorStop::KilledWithoutProcess) && row.status_context != "exit:killed" {
            db.mark_killed(&row.name, initiator)?;
        }
        let actual = current(db, fence)?;
        if actual.pid != row.pid || actual.pid_identity() != row.pid_identity() || actual.session_id != row.session_id || actual.agent_id != row.agent_id {
            return Err(ServiceError::StaleFence.into());
        }
        let killed = actual.status_context == "exit:killed";
        let by = if killed && !actual.status_detail.is_empty() { actual.status_detail.as_str() } else { initiator };
        hard_stop(db, context, &actual, by, if killed { "killed" } else { reason }).map_err(anyhow::Error::new)
    })
    .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

/// Separate system admission, never an impersonated hook or operator call.
/// The inspected row/process fence must still match under the write lock.
pub(super) fn cleanup_stop(db: &AgentxDb, context: &RequestContext, row: &InstanceRow, reason: &str, placeholder: bool, guarded: bool) -> ServiceResult<Option<SessionStopOutcome>> {
    if db.conn().is_autocommit() {
        return Err(ServiceError::Internal("cleanup requires its system authority transaction".into()));
    }
    let actual = current(db, &participant(row))?;
    if actual.session_id != row.session_id || actual.agent_id != row.agent_id || actual.pid != row.pid || actual.pid_identity() != row.pid_identity() {
        return Err(ServiceError::StaleFence);
    }
    let identity = if guarded {
        if super::cleanup_store::tracked_process_state(&actual) != super::service::TrackedProcessState::Gone {
            return Err(ServiceError::StaleFence);
        }
        Some((actual.pid.and_then(|pid| u32::try_from(pid).ok()).ok_or(ServiceError::StaleFence)?, actual.pid_identity().ok_or(ServiceError::StaleFence)?))
    } else {
        None
    };
    let by = if actual.status_context == "exit:killed" && !actual.status_detail.is_empty() { actual.status_detail.as_str() } else { "system" };
    retire_tree(db, context, &actual, by, reason, placeholder, identity.as_ref().map(|(pid, identity)| (*pid, identity.as_str())))
}

fn retire_tree(db: &AgentxDb, context: &RequestContext, row: &InstanceRow, by: &str, reason: &str, placeholder: bool, pid_guard: Option<(u32, &str)>) -> ServiceResult<Option<SessionStopOutcome>> {
    let mut rows = Vec::new();
    descendants(db, row.clone(), 0, &mut HashSet::new(), &mut HashSet::new(), &mut rows)?;
    // Observed parent death cannot prove the death of an independently tracked
    // child. Leave the whole tree for that child's own admitted cleanup.
    if pid_guard.is_some() && rows.iter().any(|child| child.name != row.name && child.pid.is_some()) {
        return Ok(None);
    }
    let mut effects = Vec::with_capacity(rows.len());
    for stopped in &rows {
        let action = if pid_guard.is_some() {
            StopProcessAction::KeepRunning
        } else if stopped.background != 0 {
            StopProcessAction::TerminateGroup
        } else {
            StopProcessAction::TrackSurvivor
        };
        effects.push(freeze(db, stopped, action)?);
    }
    if row.status_context != "exit:killed" {
        status(db, row, "inactive", reason)?;
    }
    for stopped in &rows {
        let reason = if stopped.name == row.name { reason } else { "parent_stopped" };
        let mut snapshot = db.get_instance_snapshot(&stopped.name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        snapshot["parent_session_id"] = json!(stopped.parent_session_id);
        let mut event = json!({"action":"stopped","by":by,"reason":reason,"snapshot":snapshot});
        let won = if stopped.name == row.name {
            if placeholder {
                event["placeholder"] = json!(true);
            }
            if let Some(guard) = pid_guard {
                db.finalize_instance_stop_if_pid_identity(&stopped.name, stopped.created_at, stopped.session_id.as_deref(), stopped.agent_id.as_deref(), guard, &event)
            } else {
                db.finalize_instance_stop(&stopped.name, stopped.created_at, stopped.session_id.as_deref(), stopped.agent_id.as_deref(), &event)
            }
        } else {
            db.finalize_instance_stop(&stopped.name, stopped.created_at, stopped.session_id.as_deref(), stopped.agent_id.as_deref(), &event)
        }
        .map_err(internal)?;
        if !won {
            return Err(ServiceError::StaleFence);
        }
        cleanup_actor(db, stopped)?;
    }
    let outcome = SessionStopOutcome { stopped: true, retained: false, effects };
    if row.status_context == "exit:killed" {
        let retired = KilledRetirement { generation: context.database_generation.clone(), participant: participant(row), outcome: outcome.clone() };
        db.kv_set(&killed_retirement_key(&retired.generation, &retired.participant), Some(&serde_json::to_string(&retired).map_err(internal)?)).map_err(internal)?;
    }
    Ok(Some(outcome))
}

/// Canonical binding-switch soft retirement. Binding owns admission and has
/// already moved PID/terminal state and endpoints to the destination identity.
/// The old row keeps its cursor/history; the continuing process is untouched.
pub(super) fn soft_retire_switched(db: &AgentxDb, inspected: &InstanceRow) -> ServiceResult<()> {
    if db.conn().is_autocommit() {
        return Err(ServiceError::Internal("session switch requires its binding transaction".into()));
    }
    let row = current(db, &participant(inspected))?;
    if row.session_id != inspected.session_id || row.agent_id != inspected.agent_id || row.pid.is_some() {
        return Err(ServiceError::StaleFence);
    }
    super::lifecycle::try_set_status(db, &row.name, "inactive", "exit:session_switch", Default::default()).map_err(internal)?;
    let mut snapshot = db.get_instance_snapshot(&row.name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    snapshot["parent_session_id"] = json!(row.parent_session_id);
    snapshot["name_announced"] = json!(row.name_announced);
    snapshot["last_seen"] = json!(row.last_seen);
    snapshot["subscriptions"] = json!(crate::db::subscriptions::subscriptions_to_save(db, &row.name, row.created_at).map_err(internal)?);
    db.delete_session_bindings_for_instance(&row.name).map_err(internal)?;
    db.delete_process_bindings_for_instance(&row.name).map_err(internal)?;
    db.delete_notify_endpoints(&row.name).map_err(internal)?;
    db.cleanup_subscriptions(&row.name).map_err(internal)?;
    db.log_life_event(&row.name, "stopped", "session", "exit:session_switch", Some(snapshot)).map_err(internal)?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct KilledRetirement {
    generation: String,
    participant: ParticipantRef,
    outcome: SessionStopOutcome,
}

fn killed_retirement_key(generation: &str, fence: &ParticipantRef) -> String {
    format!("participant_killed_retirement:{generation}:{}:{:016x}", fence.canonical_name, fence.incarnation_bits)
}

/// Internal killed retirement, never a hook RPC. Lifecycle validates its exact
/// operator ticket; registered-host exit validates its retained host/child lease.
/// Both callers hold that authority in the same outer write scope. Retirement
/// retains its frozen plan so either actor can win the live-row deletion race.
pub(crate) fn stop_killed(db: &AgentxDb, context: &RequestContext, fence: &ParticipantRef) -> ServiceResult<SessionStopOutcome> {
    peer(context)?;
    if db.conn().is_autocommit() {
        return Err(ServiceError::Internal("operator stop requires the lifecycle authority transaction".into()));
    }
    db.with_write_scope(|| {
        let row = current(db, fence)?;
        if row.status_context != "exit:killed" {
            return Err(ServiceError::StaleFence.into());
        }
        let by = if row.status_detail.is_empty() { "session" } else { row.status_detail.as_str() };
        hard_stop(db, context, &row, by, "killed").map_err(anyhow::Error::new)
    })
    .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

/// Recover target-owned retirement after lifecycle has validated ticket peer,
/// Issuing state and the original kill-owner token. Absence alone yields None;
/// a recorded plan must match the ticket and the exact original killed marker.
pub(super) fn recover_killed(db: &AgentxDb, context: &RequestContext, ticket: &super::lifecycle_service::KillTicket, initiator: &str, marker_time: i64) -> ServiceResult<Option<SessionStopOutcome>> {
    peer(context)?;
    if db.conn().is_autocommit() {
        return Err(ServiceError::Internal("kill recovery requires the lifecycle authority transaction".into()));
    }
    if ticket.generation != context.database_generation {
        return Err(ServiceError::StaleFence);
    }
    if let Some(row) = db.get_instance_full(&ticket.participant.canonical_name).map_err(internal)? {
        if participant(&row) != ticket.participant {
            return Err(ServiceError::StaleFence);
        }
        return Ok(None);
    }
    let Some(stored) = db.kv_get(&killed_retirement_key(&ticket.generation, &ticket.participant)).map_err(internal)? else {
        return Ok(None);
    };
    let retired: KilledRetirement = serde_json::from_str(&stored).map_err(internal)?;
    if retired.generation != ticket.generation || retired.participant != ticket.participant || !retired.outcome.stopped || retired.outcome.retained {
        return Err(ServiceError::StaleFence);
    }
    let root = retired.outcome.effects.iter().find(|effect| effect.participant == ticket.participant).ok_or(ServiceError::StaleFence)?;
    if participant(&root.record) != ticket.participant
        || root.record.pid != Some(i64::from(ticket.pid))
        || root.process_identity != ticket.process_identity
        || root.record.pid_identity() != ticket.process_identity
        || root.record.status != "inactive"
        || root.record.status_context != "exit:killed"
        || root.record.status_detail != initiator
        || root.record.status_time != marker_time
    {
        return Err(ServiceError::StaleFence);
    }
    validate_stop_outcome(db, &retired.outcome)?;
    Ok(Some(retired.outcome))
}

/// Apply the same namesake-successor fence to retained lifecycle kill results
/// and hook stop results. The plan still requires host OS-incarnation checks.
pub(super) fn validate_stop_outcome(db: &AgentxDb, outcome: &SessionStopOutcome) -> ServiceResult<()> {
    for effect in &outcome.effects {
        if db.get_instance_full(&effect.participant.canonical_name).map_err(internal)?.is_some_and(|row| participant(&row) != effect.participant) {
            return Err(ServiceError::StaleFence);
        }
    }
    Ok(())
}

pub(crate) fn stop_session(db: &AgentxDb, context: &RequestContext, input: StopSession) -> ServiceResult<SessionStopOutcome> {
    peer(context)?;
    let outcome = super::service_store::retained(db, context, &json!({"stop_session":input}), || {
        let row = current(db, &input.participant)?;
        if input.session_id.as_deref().filter(|session| !session.is_empty()).is_some_and(|session| row.session_id.as_deref() != Some(session)) {
            return Ok(SessionStopOutcome { retained: true, ..Default::default() });
        }
        if resolve_actor(db, context)?.participant != input.participant {
            return Err(ServiceError::StaleFence);
        }
        let killed = row.status_context == "exit:killed";
        if !killed && row.tool == "qoder" && matches!(input.reason.as_str(), "clear" | "resume") {
            return Ok(SessionStopOutcome { retained: true, ..Default::default() });
        }
        if !killed
            && row.tool == "copilot"
            && input.reason == "user_exit"
            && row.tcp_mode != 0
            && let Some(session) = input.session_id.as_deref().filter(|session| !session.is_empty())
        {
            db.release_instance_session(&row.name, session).map_err(internal)?;
            db.update_instance_fields(&row.name, json!({"name_announced":false}).as_object().unwrap()).map_err(internal)?;
            status(db, &row, "listening", "clear")?;
            return Ok(SessionStopOutcome { retained: true, ..Default::default() });
        }
        if !killed && row.tool == "claude" {
            let env = &context.caller.invocation.environment;
            if env.get("AGENTX_PTY_MODE").map(String::as_str) == Some("1")
                && env.get("AGENTX_LAUNCHED").map(String::as_str) == Some("1")
                && let Some(batch) = env.get("AGENTX_LAUNCH_BATCH_ID")
            {
                let ready: bool = db
                    .conn()
                    .query_row("SELECT EXISTS(SELECT 1 FROM events WHERE type='life' AND instance=?1 AND json_extract(data,'$.action')='ready' AND json_extract(data,'$.batch_id')=?2)", params![row.name, batch], |row| row.get(0))
                    .map_err(internal)?;
                if !ready {
                    return Ok(SessionStopOutcome { retained: true, ..Default::default() });
                }
            }
        }
        if input.preserve_live_pty && !matches!(row.tool.as_str(), "claude" | "copilot") {
            return Err(ServiceError::InvalidInput("tool has no session-only PTY preservation policy".into()));
        }
        if !killed && (input.soft || row.tool == "antigravity") {
            let effect = freeze(db, &row, StopProcessAction::KeepRunning)?;
            let exit = format!("exit:{}", input.reason);
            status(db, &row, "inactive", &exit)?;
            if let Some(session) = &row.session_id {
                db.conn().execute("DELETE FROM session_bindings WHERE session_id=? AND instance_name=?", params![session, row.name]).map_err(internal)?;
            }
            if row.tool == "antigravity" {
                db.delete_process_bindings_for_instance(&row.name).map_err(internal)?;
            }
            db.delete_notify_endpoints(&row.name).map_err(internal)?;
            db.log_event("life", &row.name, &json!({"action":"stopped","by":"session","reason":exit,"snapshot":db.get_instance_snapshot(&row.name).map_err(internal)?})).map_err(internal)?;
            return Ok(SessionStopOutcome { stopped: true, retained: true, effects: vec![effect] });
        }
        let by = if killed && !row.status_detail.is_empty() { row.status_detail.clone() } else { "session".into() };
        let reason = if killed { "killed".into() } else { format!("exit:{}", input.reason) };
        hard_stop(db, context, &row, &by, &reason)
    })?;
    // Recovered effects remain about the frozen incarnation. A later namesake
    // is never a valid subject for those old process/terminal effects.
    validate_stop_outcome(db, &outcome)?;
    Ok(outcome)
}
