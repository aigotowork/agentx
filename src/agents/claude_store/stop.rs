//! Finite hook stop ownership and canonical retirement. The originating host
//! owns timers/listeners; no signal, wait, transcript read or Client call here.
use super::*;
use crate::agents::hook_service::SessionStopOutcome;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize)]
pub(super) struct StopOwner {
    pub claim: StopClaim,
    pub actor: Actor,
    pub pid: u32,
    pub incarnation: String,
}
fn claim_key(claim: &StopClaim) -> ServiceResult<String> {
    let bytes = serde_json::to_vec(&(&claim.generation, &claim.root, &claim.agent_id, &claim.prompt_id, &claim.fingerprint)).map_err(internal)?;
    let hash = Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    Ok(format!("subagent_stop_inflight:{}:{hash}", claim.session_id))
}
fn positively_gone(pid: u32, incarnation: &str) -> bool {
    crate::sys::process::has_exited(pid) || crate::sys::process::identity(pid).as_deref().filter(|identity| !identity.is_empty()).is_some_and(|actual| actual != incarnation)
}
pub(super) fn validate_claim(db: &AgentxDb, context: &RequestContext, claim: &StopClaim) -> ServiceResult<StopOwner> {
    if claim.generation != context.database_generation {
        return Err(ServiceError::StaleFence);
    }
    let raw = db.kv_get(&claim_key(claim)?).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    let owner: StopOwner = serde_json::from_str(&raw).map_err(internal)?;
    if owner.claim != *claim || owner.pid != context.process.pid || Some(owner.incarnation.as_str()) != context.process.incarnation.as_deref() {
        return Err(ServiceError::StaleFence);
    }
    actor(db, context, &owner.actor)?;
    Ok(owner)
}
fn retire(db: &AgentxDb, context: &RequestContext, row: &InstanceRow, reason: &str) -> ServiceResult<SessionStopOutcome> {
    let killed = row.status_context == "exit:killed";
    let by = if killed && !row.status_detail.is_empty() { row.status_detail.as_str() } else { "subagent" };
    crate::agents::hook_store::hard_stop(db, context, row, by, if killed { "killed" } else { reason })
}

pub(super) fn begin_child(db: &AgentxDb, context: &RequestContext, root_actor: &Actor, agent_id: &str, prompt_id: &str, fingerprint: &str, transcript: Option<&str>) -> ServiceResult<ChildStopResult> {
    if root_actor.agent_id.is_some() || root_actor.participant != root_actor.root {
        return Err(ServiceError::StaleFence);
    }
    let root = root(db, context, root_actor, true)?;
    if agent_id.is_empty() || prompt_id.len() > 1024 || !(16..=64).contains(&fingerprint.len()) || !fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ServiceError::InvalidInput("agent id and bounded hook payload fingerprint required".into()));
    }
    let claim = StopClaim {
        token: uuid::Uuid::new_v4().to_string(),
        generation: context.database_generation.clone(),
        root: root_actor.root.clone(),
        session_id: root_actor.session_id.clone(),
        agent_id: agent_id.to_owned(),
        prompt_id: prompt_id.to_owned(),
        fingerprint: fingerprint.to_owned(),
    };
    let key = claim_key(&claim)?;
    if let Some(raw) = db.kv_get(&key).map_err(internal)? {
        let previous: StopOwner = serde_json::from_str(&raw).map_err(internal)?;
        if previous.claim.generation != claim.generation || previous.claim.root != claim.root {
            return Err(ServiceError::StaleFence);
        }
        if previous.incarnation.is_empty() || !positively_gone(previous.pid, &previous.incarnation) {
            return Ok(ChildStopResult::Duplicate);
        }
    }
    // Claim ownership is durably established before reading the child row.
    let provisional = Actor { root: root_actor.root.clone(), participant: root_actor.root.clone(), session_id: root_actor.session_id.clone(), agent_id: Some(agent_id.to_owned()) };
    let mut owner = StopOwner { claim: claim.clone(), actor: provisional, pid: context.process.pid, incarnation: context.process.incarnation.clone().unwrap() };
    db.kv_set(&key, Some(&serde_json::to_string(&owner).map_err(internal)?)).map_err(internal)?;
    let Some(name) = db.get_instance_by_agent_id(agent_id).map_err(internal)? else {
        db.kv_set(&key, None).map_err(internal)?;
        return Ok(ChildStopResult::Missing);
    };
    let mut row = db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    owner.actor.participant = participant(&row);
    actor(db, context, &owner.actor)?;
    let mut updates = json!({"last_seen":crate::shared::time::now_epoch_i64()});
    if row.transcript_path.is_empty()
        && let Some(path) = transcript.filter(|path| !path.is_empty())
    {
        updates["transcript_path"] = json!(path);
    }
    db.update_instance_fields(&row.name, updates.as_object().unwrap()).map_err(internal)?;
    row = db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    let dormant = row.name_announced == 0;
    let direct: bool = db
        .conn()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM events e WHERE e.type='message' AND e.id>?1
         AND json_extract(e.data,'$.scope')='mentions' AND COALESCE(json_extract(e.data,'$.from'),'')!=?2
         AND EXISTS(SELECT 1 FROM json_each(e.data,'$.mentions') m WHERE m.value=?2))",
            params![row.last_event_id, row.name],
            |row| row.get(0),
        )
        .map_err(internal)?;
    if row.status_context == "exit:killed" || (dormant && !direct) {
        let result = retire(db, context, &row, "idle")?;
        db.kv_set(&key, None).map_err(internal)?;
        return Ok(ChildStopResult::Retired(result));
    }
    db.kv_set(&key, Some(&serde_json::to_string(&owner).map_err(internal)?)).map_err(internal)?;
    status(db, &row, "listening", "", "", "", "")?;
    let timeout = root.subagent_timeout.unwrap_or_else(|| crate::config::AgentxConfig::load_for_scope(db.path().parent().unwrap_or(std::path::Path::new(".")), &context.caller.invocation.environment).subagent_timeout).max(0) as u64;
    Ok(ChildStopResult::Acquired { claim, actor: owner.actor, record: row, timeout_secs: timeout, dormant })
}

pub(super) fn finish_child(db: &AgentxDb, context: &RequestContext, claim: &StopClaim, decision: FinishChildStop) -> ServiceResult<SessionStopOutcome> {
    let owner = validate_claim(db, context, claim)?;
    let (_, row) = actor(db, context, &owner.actor)?;
    let result = match decision {
        FinishChildStop::Released => SessionStopOutcome { retained: true, ..Default::default() },
        FinishChildStop::TimedOut => retire(db, context, &row, "timeout")?,
        FinishChildStop::TaskCompleted => retire(db, context, &row, "task_completed")?,
    };
    db.kv_set(&claim_key(claim)?, None).map_err(internal)?;
    Ok(result)
}

pub(super) fn mark_stale(db: &AgentxDb, context: &RequestContext, input: &Actor) -> ServiceResult<()> {
    if input.agent_id.is_some() {
        return Ok(());
    }
    let root = root(db, context, input, true)?;
    let timeout = root.subagent_timeout.unwrap_or_else(|| crate::config::AgentxConfig::load_for_scope(db.path().parent().unwrap_or(std::path::Path::new(".")), &context.caller.invocation.environment).subagent_timeout);
    let cutoff = crate::shared::time::now_epoch_i64().saturating_sub(timeout.max(0).saturating_mul(2));
    db.conn()
        .execute(
            "UPDATE instances SET status='inactive',status_context='subagent:stale',status_detail='No recent child hook; lifecycle uncertain'
         WHERE parent_session_id=?1 AND parent_name=?2 AND last_seen>0 AND last_seen<?3 AND status_context NOT IN ('subagent:stale','exit:killed')",
            params![input.session_id, root.name, cutoff],
        )
        .map_err(internal)?;
    Ok(())
}

pub(super) fn begin_root(db: &AgentxDb, context: &RequestContext, input: &Actor, registry: Option<&[BackgroundTask]>) -> ServiceResult<Response> {
    if input.agent_id.is_some() || input.participant != input.root {
        return Err(ServiceError::StaleFence);
    }
    let (_, row) = actor(db, context, input)?;
    let mut retired = SessionStopOutcome::default();
    if registry.is_some_and(|tasks| tasks.iter().all(|task| matches!(task, BackgroundTask::Shell | BackgroundTask::Monitor | BackgroundTask::Mcp))) {
        let mut query = db.conn().prepare("SELECT name FROM instances WHERE parent_session_id=? AND parent_name=? ORDER BY name").map_err(internal)?;
        let names = query.query_map(params![input.session_id, row.name], |row| row.get::<_, String>(0)).map_err(internal)?.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)?;
        for name in names {
            let Some(child) = db.get_instance_full(&name).map_err(internal)? else {
                continue;
            };
            let result = retire(db, context, &child, "orphaned")?;
            retired.stopped |= result.stopped;
            retired.effects.extend(result.effects);
        }
    }
    let timeout = row.wait_timeout.unwrap_or_else(|| crate::config::AgentxConfig::load_for_scope(db.path().parent().unwrap_or(std::path::Path::new(".")), &context.caller.invocation.environment).timeout).max(0) as u64;
    let pty = context.caller.invocation.environment.get("AGENTX_PTY_MODE").map(String::as_str) == Some("1");
    if !pty {
        db.update_instance_fields(&row.name, json!({"wait_timeout":timeout}).as_object().unwrap()).map_err(internal)?;
    }
    status(db, &row, "listening", "", "", "", "")?;
    if pty {
        db.wake_after_commit(&row.name, &[crate::runtime::notify::WakeKind::Hook]);
    }
    Ok(Response::RootStop { timeout_secs: timeout, pty, retired_children: retired })
}

#[derive(Serialize, Deserialize)]
struct PollOwner {
    actor: Actor,
    pid: u32,
    incarnation: String,
    port: u16,
}
pub(super) fn endpoint(db: &AgentxDb, context: &RequestContext, input: &Actor, port: Option<u16>, claim: Option<&StopClaim>) -> ServiceResult<()> {
    let (_, row) = actor(db, context, input)?;
    if let Some(claim) = claim
        && validate_claim(db, context, claim)?.actor != *input
    {
        return Err(ServiceError::StaleFence);
    }
    let key = format!("claude_poll:{}:{}:{}", context.database_generation, row.name, row.created_at.to_bits());
    let previous = db.kv_get(&key).map_err(internal)?.map(|raw| serde_json::from_str::<PollOwner>(&raw).map_err(internal)).transpose()?;
    let ours = |owner: &PollOwner| owner.pid == context.process.pid && Some(owner.incarnation.as_str()) == context.process.incarnation.as_deref() && owner.actor == *input;
    if let Some(port) = port {
        if port == 0 {
            return Err(ServiceError::InvalidInput("hook listener port must be positive".into()));
        }
        if previous.as_ref().is_some_and(|owner| !ours(owner) && !positively_gone(owner.pid, &owner.incarnation)) {
            return Err(ServiceError::Conflict("another Claude hook owns the poll endpoint".into()));
        }
        let owner = PollOwner { actor: input.clone(), pid: context.process.pid, incarnation: context.process.incarnation.clone().unwrap(), port };
        db.upsert_notify_endpoint(&row.name, "hook", port).map_err(internal)?;
        db.kv_set(&key, Some(&serde_json::to_string(&owner).map_err(internal)?)).map_err(internal)?;
    } else if let Some(previous) = previous {
        if !ours(&previous) {
            return Err(ServiceError::StaleFence);
        }
        db.conn().execute("DELETE FROM notify_endpoints WHERE instance=? AND kind='hook' AND port=?", params![row.name, previous.port]).map_err(internal)?;
        db.kv_set(&key, None).map_err(internal)?;
    }
    db.update_instance_fields(&row.name, json!({"tcp_mode":port.is_some(),"last_seen":crate::shared::time::now_epoch_i64()}).as_object().unwrap()).map_err(internal)?;
    Ok(())
}

fn cleanup_session(db: &AgentxDb, session: &str) -> ServiceResult<()> {
    db.kv_delete_prefix(&crate::integrations::hooks::claude::native::session_prefix(session)).map_err(internal)?;
    db.clear_claude_session_validation(session).map_err(internal)?;
    db.revoke_claude_actor_capabilities_for_session(session).map_err(internal)?;
    db.kv_delete_prefix(&format!("subagent_stop_inflight:{session}:")).map_err(internal)?;
    Ok(())
}
pub(super) fn session_end(db: &AgentxDb, context: &RequestContext, input: &Actor, reason: &str) -> ServiceResult<SessionStopOutcome> {
    if input.agent_id.is_some() || input.participant != input.root {
        return Err(ServiceError::StaleFence);
    }
    let row = root(db, context, input, false)?;
    if row.session_id.as_deref() != Some(input.session_id.as_str()) {
        cleanup_session(db, &input.session_id)?;
        return Ok(SessionStopOutcome { retained: true, ..Default::default() });
    }
    let env = &context.caller.invocation.environment;
    if row.status_context != "exit:killed"
        && env.get("AGENTX_PTY_MODE").map(String::as_str) == Some("1")
        && env.get("AGENTX_LAUNCHED").map(String::as_str) == Some("1")
        && let Some(batch) = env.get("AGENTX_LAUNCH_BATCH_ID")
    {
        let ready: bool = db
            .conn()
            .query_row("SELECT EXISTS(SELECT 1 FROM events WHERE type='life' AND instance=?1 AND json_extract(data,'$.action')='ready' AND json_extract(data,'$.batch_id')=?2)", params![row.name, batch], |row| row.get(0))
            .map_err(internal)?;
        if !ready {
            cleanup_session(db, &input.session_id)?;
            return Ok(SessionStopOutcome { retained: true, ..Default::default() });
        }
    }
    let killed = row.status_context == "exit:killed";
    let by = if killed && !row.status_detail.is_empty() { row.status_detail.as_str() } else { "session" };
    let exit = if killed { "killed".to_owned() } else { format!("exit:{reason}") };
    let result = crate::agents::hook_store::hard_stop(db, context, &row, by, &exit)?;
    cleanup_session(db, &input.session_id)?;
    Ok(result)
}
