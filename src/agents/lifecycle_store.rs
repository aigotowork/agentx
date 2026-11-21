//! Participant lifecycle persistence adapter. Start orchestration is added here
//! after explicit invocation binding; every named mutation is incarnation fenced.
use super::attachments::ParticipantRef;
use super::lifecycle_service::*;
use crate::agents::binding as instance_binding;
use crate::agents::{identity, lifecycle, names as instance_names, records as instances};
use crate::db::{AgentxDb, InstanceRow};
use crate::integrations::hooks::claude::actor as claude_actor;
use crate::log::log_info;
use crate::runtime::context::AgentxContext;
use crate::runtime::process_tracking as pidtrack;
use crate::service::{RequestContext, ServiceError, ServiceResult};
use crate::shared::constants::ST_ACTIVE;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use sha2::Digest;
use std::collections::HashMap;
use std::path::PathBuf;

#[path = "lifecycle_resume_store.rs"]
mod resume_store;

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    let (participant, column, value) = match request {
        Request::Context(request) => return super::context_store::handle(db, context, request).map(Response::Context),
        Request::SetTimeout { participant, timeout } => {
            if timeout <= 0 {
                return Err(ServiceError::InvalidInput("timeout must be a positive integer (seconds)".into()));
            }
            (participant, "wait_timeout", timeout.into())
        }
        Request::SetSubagentTimeout { participant, timeout } => {
            if timeout.is_some_and(|value| value <= 0) {
                return Err(ServiceError::InvalidInput("subagent_timeout must be positive".into()));
            }
            (participant, "subagent_timeout", timeout.map(serde_json::Value::from).unwrap_or_default())
        }
        Request::SetHints { participant, hints } => (participant, "hints", hints.map(serde_json::Value::String).unwrap_or_default()),
        Request::PrepareKill { participant, initiator } => return prepare_kill(db, context, &participant, &initiator).map(Response::KillPrepared),
        Request::BeginKill { ticket } => return begin_kill(db, context, &ticket),
        Request::CompleteKill { ticket } => return complete_kill(db, context, &ticket).map(Response::KillCompleted),
        Request::KillDenied { ticket } => return deny_kill(db, context, &ticket),
        Request::ClaimOrphans { candidates } => return claim_orphans(db, context, &candidates).map(Response::OrphansClaimed),
        Request::StopOperator { participant, initiator, decision } => return stop_operator(db, context, &participant, &initiator, &decision).map(Response::OperatorStopped),
        Request::ResolveResume { target, expected, fork } => return resume_store::resolve(db, context, &target, expected.as_ref(), fork).map(Response::ResumeSelected),
        Request::PreviewResume { target, fork } => return resume_store::preview(db, context, &target, fork).map(Response::ResumeSelected),
        Request::ReserveResumeFork { source } => return resume_store::reserve_fork(db, context, source).map(Response::ResumeFork),
        Request::ReserveResume { source, launch } => return resume_store::reserve(db, context, &source, launch).map(Response::ResumeReserved),
        Request::Start { mode, requested_name } => return retained_start(db, context, mode, requested_name).map(Response::Started),
        Request::PrepareBootstrap { participant, force } => {
            return fenced(db, &participant, || {
                if super::service::resolve_actor(db, context)?.participant != participant {
                    return Err(ServiceError::StaleFence);
                }
                let row = db.get_instance_full(&participant.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
                let receipt = if row.name_announced != 0 && !force { None } else { Some(issue_bootstrap_receipt(db, context, &participant, context.caller.invocation.activation_target.as_ref())?) };
                Ok(Response::BootstrapPrepared { receipt })
            });
        }
        Request::BootstrapFlushed { participant, receipt } => return confirm_bootstrap(db, context, &participant, &receipt),
    };
    fenced(db, &participant, || {
        let values = serde_json::Map::from_iter([(column.to_string(), value)]);
        db.update_instance_fields(&participant.canonical_name, &values).map_err(internal)?;
        db.wake_after_commit(&participant.canonical_name, crate::runtime::notify::WakeKind::ALL);
        Ok(Response::Changed)
    })
}
fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

fn claim_orphans(db: &AgentxDb, context: &RequestContext, candidates: &[pidtrack::OrphanCandidate]) -> ServiceResult<Vec<AdoptedOrphan>> {
    kill_peer(context)?;
    if context.caller.request_id.0.is_empty() {
        return Err(ServiceError::InvalidInput("orphan adoption request ID required".into()));
    }
    let input = json!({"candidates":candidates,"invocation":context.caller.invocation,"peer":{"pid":context.process.pid,"incarnation":context.process.incarnation}});
    let intent: String = sha2::Sha256::digest(serde_json::to_vec(&input).map_err(internal)?).iter().map(|byte| format!("{byte:02x}")).collect();
    let key = format!("participant_orphan_adoption:{}:{}", context.database_generation, context.caller.request_id.0);
    kill_transaction(db, || {
        if let Some(stored) = db.kv_get(&key).map_err(internal)? {
            let stored: Value = serde_json::from_str(&stored).map_err(internal)?;
            if stored["intent"].as_str() != Some(&intent) {
                return Err(ServiceError::Conflict("orphan adoption request reused for another input or caller".into()));
            }
            let outcome: Vec<AdoptedOrphan> = serde_json::from_value(stored["outcome"].clone()).map_err(internal)?;
            for adopted in &outcome {
                let row = db.get_instance_full(&adopted.record.name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
                if participant(&row) != participant(&adopted.record) || row.pid != adopted.record.pid || row.pid_identity().as_deref() != Some(adopted.process_identity.as_str()) {
                    return Err(ServiceError::StaleFence);
                }
            }
            return Ok(outcome);
        }
        let adopted = pidtrack::try_adopt_orphans(db, candidates).map_err(internal)?;
        let mut outcome = Vec::new();
        for (_, name) in adopted {
            let record = db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
            let process_identity = record.pid_identity().ok_or(ServiceError::StaleFence)?;
            outcome.push(AdoptedOrphan { record, process_identity });
        }
        db.kv_set(&key, Some(&serde_json::to_string(&json!({"intent":intent,"outcome":outcome})).map_err(internal)?)).map_err(internal)?;
        Ok(outcome)
    })
}

/// Operator retirement retains the complete frozen plan before the live row
/// disappears. Peer/intent and namesake validation also apply to lost replies.
fn stop_operator(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef, initiator: &str, decision: &super::hook_service::OperatorStop) -> ServiceResult<super::hook_service::SessionStopOutcome> {
    kill_peer(context)?;
    if context.caller.request_id.0.is_empty() {
        return Err(ServiceError::InvalidInput("operator stop request ID required".into()));
    }
    let input = json!({"participant":participant,"initiator":initiator,"decision":decision,"invocation":context.caller.invocation,"peer":{"pid":context.process.pid,"incarnation":context.process.incarnation}});
    let intent: String = sha2::Sha256::digest(serde_json::to_vec(&input).map_err(internal)?).iter().map(|byte| format!("{byte:02x}")).collect();
    let key = format!("participant_operator_stop:{}:{}", context.database_generation, context.caller.request_id.0);
    kill_transaction(db, || {
        if let Some(stored) = db.kv_get(&key).map_err(internal)? {
            let stored: Value = serde_json::from_str(&stored).map_err(internal)?;
            if stored["intent"].as_str() != Some(&intent) {
                return Err(ServiceError::Conflict("operator stop request reused for another caller or intent".into()));
            }
            let outcome = serde_json::from_value(stored["outcome"].clone()).map_err(internal)?;
            super::hook_store::validate_stop_outcome(db, &outcome)?;
            return Ok(outcome);
        }
        // Registry removal is the canonical bridge disconnect; an ordinary
        // retirement alone would let the gateway recreate its registration.
        if super::hermes_service::stored_bridges(db)?.iter().any(|bridge| bridge.name == participant.canonical_name) {
            return Err(ServiceError::Conflict("Hermes chat must be disconnected through its exact registry entry".into()));
        }
        let outcome = super::service::stop_operator(db, context, participant, initiator, decision)?;
        db.kv_set(&key, Some(&serde_json::to_string(&json!({"intent":intent,"outcome":outcome})).map_err(internal)?)).map_err(internal)?;
        Ok(outcome)
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
enum KillState {
    Prepared,
    Issuing,
    Denied,
    Completed(super::hook_service::SessionStopOutcome),
}

#[derive(serde::Serialize, serde::Deserialize)]
struct KillAuthority {
    intent: String,
    plan: KillPlan,
    peer_pid: u32,
    peer_incarnation: String,
    initiator: String,
    marker_time: i64,
    state: KillState,
}

fn kill_peer(context: &RequestContext) -> ServiceResult<&str> {
    if context.process.pid == 0 {
        return Err(ServiceError::Forbidden("authenticated operator process required".into()));
    }
    context.process.incarnation.as_deref().filter(|value| !value.is_empty()).ok_or_else(|| ServiceError::Forbidden("authenticated operator incarnation required".into()))
}

fn kill_key(ticket: &KillTicket) -> String {
    format!("participant_kill:{}:{}", ticket.generation, ticket.token)
}
fn kill_owner(participant: &ParticipantRef) -> String {
    format!("participant_kill_owner:{}:{:016x}", participant.canonical_name, participant.incarnation_bits)
}
fn kill_transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}
fn save_kill(db: &AgentxDb, authority: &KillAuthority) -> ServiceResult<()> {
    db.kv_set(&kill_key(&authority.plan.ticket), Some(&serde_json::to_string(authority).map_err(internal)?)).map_err(internal)
}
fn load_kill(db: &AgentxDb, context: &RequestContext, ticket: &KillTicket) -> ServiceResult<KillAuthority> {
    let peer = kill_peer(context)?;
    if ticket.generation != context.database_generation {
        return Err(ServiceError::StaleFence);
    }
    let stored = db.kv_get(&kill_key(ticket)).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    let authority: KillAuthority = serde_json::from_str(&stored).map_err(internal)?;
    if authority.plan.ticket != *ticket || authority.peer_pid != context.process.pid || authority.peer_incarnation != peer {
        return Err(ServiceError::StaleFence);
    }
    Ok(authority)
}
fn kill_row(db: &AgentxDb, authority: &KillAuthority, require_marker: bool) -> ServiceResult<InstanceRow> {
    let ticket = &authority.plan.ticket;
    let row = db.get_instance_full(&ticket.participant.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    if participant(&row) != ticket.participant || row.pid != Some(i64::from(ticket.pid)) || row.pid_identity() != ticket.process_identity {
        return Err(ServiceError::StaleFence);
    }
    if require_marker
        && (db.kv_get(&kill_owner(&ticket.participant)).map_err(internal)?.as_deref() != Some(ticket.token.as_str())
            || row.status != "inactive"
            || row.status_context != "exit:killed"
            || row.status_detail != authority.initiator
            || row.status_time != authority.marker_time)
    {
        return Err(ServiceError::StaleFence);
    }
    Ok(row)
}

fn prepare_kill(db: &AgentxDb, context: &RequestContext, fence: &ParticipantRef, initiator: &str) -> ServiceResult<KillPlan> {
    let peer = kill_peer(context)?;
    if initiator.is_empty() || initiator.len() > 1024 || context.caller.request_id.0.is_empty() {
        return Err(ServiceError::InvalidInput("kill initiator and request ID required".into()));
    }
    let normalized = json!({"participant":fence,"initiator":initiator,"invocation":context.caller.invocation,"pid":context.process.pid,"incarnation":peer});
    let intent: String = sha2::Sha256::digest(serde_json::to_vec(&normalized).map_err(internal)?).iter().map(|byte| format!("{byte:02x}")).collect();
    let request_key = format!("participant_kill_request:{}:{}", context.database_generation, context.caller.request_id.0);
    kill_transaction(db, || {
        if let Some(stored) = db.kv_get(&request_key).map_err(internal)? {
            let ticket: KillTicket = serde_json::from_str(&stored).map_err(internal)?;
            let authority = load_kill(db, context, &ticket)?;
            if authority.intent != intent {
                return Err(ServiceError::Conflict("kill request ID reused for another actor or intent".into()));
            }
            kill_row(db, &authority, true)?;
            return match authority.state {
                KillState::Prepared => Ok(authority.plan),
                _ => Err(ServiceError::OutcomeUnknown("kill ticket has already been issued; reconcile its completion before new host effects".into())),
            };
        }
        let record = db.get_instance_full(&fence.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        if participant(&record) != *fence || record.origin_device_id.is_some() {
            return Err(ServiceError::StaleFence);
        }
        let pid = record.pid.and_then(|pid| u32::try_from(pid).ok()).filter(|pid| *pid > 0 && i32::try_from(*pid).is_ok()).ok_or_else(|| ServiceError::InvalidInput(format!("No tracked PID for '{}'", fence.canonical_name)))?;
        if db.kv_get(&kill_owner(fence)).map_err(internal)?.is_some() {
            return Err(ServiceError::Conflict("another operator kill already owns this participant".into()));
        }
        let ticket = KillTicket { token: uuid::Uuid::new_v4().to_string(), generation: context.database_generation.clone(), participant: fence.clone(), pid, process_identity: record.pid_identity() };
        db.mark_killed(&fence.canonical_name, initiator).map_err(internal)?;
        let marker_time = db.get_instance_full(&fence.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?.status_time;
        let authority = KillAuthority { intent, plan: KillPlan { ticket: ticket.clone(), record }, peer_pid: context.process.pid, peer_incarnation: peer.into(), initiator: initiator.into(), marker_time, state: KillState::Prepared };
        db.kv_set(&kill_owner(fence), Some(&ticket.token)).map_err(internal)?;
        save_kill(db, &authority)?;
        db.kv_set(&request_key, Some(&serde_json::to_string(&ticket).map_err(internal)?)).map_err(internal)?;
        Ok(authority.plan)
    })
}

fn begin_kill(db: &AgentxDb, context: &RequestContext, ticket: &KillTicket) -> ServiceResult<Response> {
    kill_transaction(db, || {
        let mut authority = load_kill(db, context, ticket)?;
        kill_row(db, &authority, true)?;
        if !matches!(authority.state, KillState::Prepared) {
            return Err(ServiceError::OutcomeUnknown("kill issuance has already been granted; do not repeat the signal or pane close".into()));
        }
        authority.state = KillState::Issuing;
        save_kill(db, &authority)?;
        Ok(Response::KillGranted)
    })
}

fn complete_kill(db: &AgentxDb, context: &RequestContext, ticket: &KillTicket) -> ServiceResult<super::hook_service::SessionStopOutcome> {
    kill_transaction(db, || {
        let mut authority = load_kill(db, context, ticket)?;
        // The live row is normally absent after commit. Recover the complete
        // frozen result first, then reject any namesake successor.
        if let KillState::Completed(outcome) = &authority.state {
            super::hook_store::validate_stop_outcome(db, outcome)?;
            return Ok(outcome.clone());
        }
        if !matches!(authority.state, KillState::Issuing) {
            return Err(ServiceError::Conflict("kill ticket was not issued or has been denied".into()));
        }
        if db.kv_get(&kill_owner(&ticket.participant)).map_err(internal)?.as_deref() != Some(ticket.token.as_str()) {
            return Err(ServiceError::StaleFence);
        }
        if let Some(outcome) = super::hook_store::recover_killed(db, context, ticket, &authority.initiator, authority.marker_time)? {
            authority.state = KillState::Completed(outcome.clone());
            save_kill(db, &authority)?;
            return Ok(outcome);
        }
        if db.get_instance_full(&ticket.participant.canonical_name).map_err(internal)?.is_none() {
            return Err(ServiceError::OutcomeUnknown("participant disappeared without canonical killed-retirement evidence".into()));
        }
        kill_row(db, &authority, true)?;
        let outcome = super::hook_store::stop_killed(db, context, &ticket.participant)?;
        authority.state = KillState::Completed(outcome.clone());
        save_kill(db, &authority)?;
        Ok(outcome)
    })
}

fn deny_kill(db: &AgentxDb, context: &RequestContext, ticket: &KillTicket) -> ServiceResult<Response> {
    kill_transaction(db, || {
        let mut authority = load_kill(db, context, ticket)?;
        let row = kill_row(db, &authority, false)?;
        if matches!(authority.state, KillState::Denied) {
            return Ok(Response::Changed);
        }
        if matches!(authority.state, KillState::Completed(_)) {
            return Err(ServiceError::Conflict("completed kill cannot be denied".into()));
        }
        if db.kv_get(&kill_owner(&ticket.participant)).map_err(internal)?.as_deref() != Some(ticket.token.as_str()) {
            return Err(ServiceError::StaleFence);
        }
        if row.status == "inactive" && row.status_context == "exit:killed" && row.status_detail == authority.initiator && row.status_time == authority.marker_time {
            let original = &authority.plan.record;
            db.conn()
                .execute(
                    "UPDATE instances SET status=?1,status_context=?2,status_detail=?3,status_time=?4 WHERE name=?5 AND created_at=?6 AND pid=?7 AND status='inactive' AND status_context='exit:killed' AND status_detail=?8 AND status_time=?9",
                    rusqlite::params![original.status, original.status_context, original.status_detail, original.status_time, original.name, original.created_at, original.pid, authority.initiator, authority.marker_time],
                )
                .map_err(internal)?;
        }
        db.kv_set(&kill_owner(&ticket.participant), None).map_err(internal)?;
        authority.state = KillState::Denied;
        save_kill(db, &authority)?;
        Ok(Response::Changed)
    })
}

fn retained_start(db: &AgentxDb, context: &RequestContext, mode: StartMode, requested_name: Option<String>) -> ServiceResult<StartOutput> {
    if context.caller.request_id.0.is_empty() {
        return Err(ServiceError::InvalidInput("start request ID required".into()));
    }
    let intent = json!({"mode":mode,"requested_name":requested_name,"invocation":context.caller.invocation,"process": {"pid":context.process.pid,"incarnation":context.process.incarnation}});
    let digest = sha2::Sha256::digest(serde_json::to_vec(&intent).map_err(internal)?);
    let intent: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let key = format!("participant_start:{}:{}", context.database_generation, context.caller.request_id.0);
    let mut cleanup = Vec::new();
    let output = db
        .with_write_scope(|| {
            if let Some(retained) = db.kv_get(&key)? {
                let retained: Value = serde_json::from_str(&retained)?;
                if retained["intent"].as_str() != Some(&intent) {
                    return Err(ServiceError::Conflict("start request ID reused for another actor or intent".into()).into());
                }
                let output: StartOutput = serde_json::from_value(retained["output"].clone())?;
                if let Some(participant) = &output.participant {
                    check_current(db, participant)?;
                }
                if let Some(target) = &output.activation_target {
                    use super::attachments::AttachmentStore;
                    if !super::attachments::SqliteAttachmentStore::new(db).validate(target)? {
                        return Err(ServiceError::StaleFence.into());
                    }
                }
                return Ok(output);
            }
            let mut output = start_intent(db, context, &mode, requested_name.as_deref(), &mut cleanup)?;
            if output.bootstrap.is_some()
                && let Some(participant) = &output.participant
            {
                let receipt = issue_bootstrap_receipt(db, context, participant, output.activation_target.as_ref())?;
                output.bootstrap_receipt = Some(receipt);
            }
            db.kv_set(&key, Some(&json!({"intent":intent,"output":output}).to_string()))?;
            Ok(output)
        })
        .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))?;
    // The tracking file may outlive a failed response; it must never disappear
    // while the row adoption/recovery can still roll back.
    let scope = db.path().parent().unwrap_or(std::path::Path::new("."));
    for pid in cleanup {
        pidtrack::remove_pid(scope, pid);
    }
    Ok(output)
}

pub(super) fn issue_bootstrap_receipt(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef, target: Option<&super::attachments::ActivationTarget>) -> ServiceResult<String> {
    let receipt = uuid::Uuid::new_v4().to_string();
    let key = format!("participant_bootstrap:{}:{receipt}", context.database_generation);
    let authority = json!({"participant":participant,"target":target,"pid":context.process.pid,"incarnation":context.process.incarnation});
    db.kv_set(&key, Some(&authority.to_string())).map_err(internal)?;
    Ok(receipt)
}

fn check_current(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<()> {
    let row = db.get_instance_full(&participant.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    if row.created_at.to_bits() != participant.incarnation_bits {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}

fn confirm_bootstrap(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef, receipt: &str) -> ServiceResult<Response> {
    fenced(db, participant, || {
        let key = format!("participant_bootstrap:{}:{receipt}", context.database_generation);
        let retained = db.kv_get(&key).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        let authority: Value = serde_json::from_str(&retained).map_err(internal)?;
        let expected: ParticipantRef = serde_json::from_value(authority["participant"].clone()).map_err(internal)?;
        if expected != *participant || authority["pid"].as_u64() != Some(context.process.pid as u64) || authority["incarnation"] != json!(context.process.incarnation) {
            return Err(ServiceError::StaleFence);
        }
        if !authority["target"].is_null() {
            use super::attachments::AttachmentStore;
            let target = serde_json::from_value(authority["target"].clone()).map_err(internal)?;
            if !super::attachments::SqliteAttachmentStore::new(db).validate(&target).map_err(internal)? {
                return Err(ServiceError::StaleFence);
            }
        }
        if !db.ack_hook_delivery(&participant.canonical_name, 0, true, Some(f64::from_bits(participant.incarnation_bits))).map_err(internal)? {
            return Err(ServiceError::StaleFence);
        }
        Ok(Response::Changed)
    })
}
fn fenced<T>(db: &AgentxDb, participant: &ParticipantRef, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| {
        let row = db.get_instance_full(&participant.canonical_name)?.ok_or(ServiceError::StaleFence)?;
        if row.created_at.to_bits() != participant.incarnation_bits {
            return Err(ServiceError::StaleFence.into());
        }
        action().map_err(Into::into)
    })
    .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

fn participant(record: &InstanceRow) -> ParticipantRef {
    ParticipantRef { canonical_name: record.name.clone(), incarnation_bits: record.created_at.to_bits() }
}
fn current_participant(db: &AgentxDb, name: &str) -> Result<ParticipantRef> {
    Ok(participant(&db.get_instance_full(name)?.ok_or(ServiceError::StaleFence)?))
}
fn runtime_context(db: &AgentxDb, request: &RequestContext) -> AgentxContext {
    let invocation = &request.caller.invocation;
    let mut ctx = AgentxContext::from_env(&invocation.environment, invocation.cwd.clone()).with_tty(invocation.stdin_is_tty, invocation.stdout_is_tty);
    ctx.process_id = invocation.process_binding_id.clone();
    ctx.agentx_dir = db.path().parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
    ctx
}
fn prepare_bootstrap(db: &AgentxDb, output: &mut StartOutput, name: &str, tool: &str) -> Result<()> {
    output.participant = Some(current_participant(db, name)?);
    output.bootstrap = Some(Bootstrap::Root { tool: tool.to_string() });
    Ok(())
}

/// All allocator/reclaim/adoption decisions run after durable-intent lookup in
/// the same write scope. No host output or terminal mutation runs in this scope.
fn start_intent(db: &AgentxDb, request: &RequestContext, mode: &StartMode, requested_name: Option<&str>, cleanup: &mut Vec<u32>) -> Result<StartOutput> {
    if request.caller.invocation.explicit_name.is_some() {
        super::service::resolve_actor(db, request)?;
    }
    let ctx = runtime_context(db, request);
    let agentx_dir = ctx.agentx_dir.clone();
    let rebind_target = match mode {
        StartMode::Reclaim { name } => Some(name.clone()),
        _ => None,
    };
    let orphan_target = match mode {
        StartMode::RecoverOrphan { target } => Some(target.clone()),
        _ => None,
    };
    if crate::integrations::codex::roots::is_shared_context(&ctx) {
        use super::attachments::{AttachmentStore, SqliteAttachmentStore};
        if rebind_target.is_some() || orphan_target.is_some() {
            bail!("Shared provider session identity is owned by its attachment; --as and --orphan do not apply.");
        }
        let store = SqliteAttachmentStore::new(db);
        let attachment = crate::integrations::codex::roots::resolve_shared_attachment(&store, &ctx, ctx.codex_thread_id.as_deref().unwrap_or(""))?.ok_or_else(|| anyhow::anyhow!("No current attachment for this provider session."))?;
        let name = &attachment.target.participant.canonical_name;
        if let Some(requested) = requested_name
            && requested != name.as_str()
        {
            bail!("Explicit identity disagrees with this provider session attachment.");
        }
        if let Some(caller) = request.caller.invocation.explicit_name.as_deref()
            && caller != name.as_str()
        {
            bail!("Explicit caller identity disagrees with this provider session attachment.");
        }
        if !store.validate(&attachment.target)? {
            return Err(ServiceError::StaleFence.into());
        }
        return Ok(StartOutput { participant: Some(attachment.target.participant.clone()), bootstrap: Some(Bootstrap::Root { tool: "codex".into() }), activation_target: Some(attachment.target), ..Default::default() });
    }
    let verified_actor = claude_actor::resolve_actor_from_env(db, &request.caller.invocation.environment).map_err(anyhow::Error::new)?;
    if let (Some(actor), Some(name)) = (verified_actor.as_ref(), requested_name) {
        claude_actor::ensure_explicit_matches(db, actor, name).map_err(anyhow::Error::new)?;
    }
    if let (Some(actor), Some(name)) = (verified_actor.as_ref(), request.caller.invocation.explicit_name.as_deref()) {
        claude_actor::ensure_explicit_matches(db, actor, name).map_err(anyhow::Error::new)?;
    }
    let requested_name = requested_name.map(|name| name.to_string());
    if let Some(actor) = verified_actor.as_ref()
        && let Some(row) = db.get_instance_full(&actor.name)?
        && instances::is_subagent_instance(&row)
    {
        if rebind_target.is_some() {
            return Ok(StartOutput { stdout: "[AGENTX] Subagents cannot use --as. End your turn.\n".into(), exit_code: 1, ..Default::default() });
        }
        if orphan_target.is_some() {
            return Ok(StartOutput { stdout: "[AGENTX] Subagents cannot use --orphan. End your turn.\n".into(), exit_code: 1, ..Default::default() });
        }
        return start_subagent_effects(db, request, &row);
    }
    let subagent_via_name = if verified_actor.is_none() { requested_name.as_deref().map(|id| detect_subagent(db, id)).transpose()?.flatten() } else { None };
    let subagent_via_as = if verified_actor.is_none() { rebind_target.as_deref().map(|id| detect_subagent(db, id)).transpose()?.flatten() } else { None };
    if subagent_via_as.is_some() || (subagent_via_name.is_some() && rebind_target.is_some()) {
        return Ok(StartOutput { stdout: "[AGENTX] Subagents cannot change identity. End your turn.\n".into(), exit_code: 1, ..Default::default() });
    }
    if let Some(orphan) = orphan_target {
        return start_from_orphan_effects(db, &agentx_dir, &orphan, cleanup);
    }
    let caller_name = verified_actor.as_ref().map(|actor| actor.name.as_str()).or(request.caller.invocation.explicit_name.as_deref()).or(requested_name.as_deref());
    if let Some(rebind) = rebind_target {
        return start_rebind_effects(db, request, &rebind, &ctx, caller_name);
    }
    if let Some(subagent) = subagent_via_name {
        return start_subagent_effects(db, request, &subagent);
    }
    start_bare_effects(db, request, &ctx, requested_name.as_deref().or(caller_name))
}

/// Resolve a live child row directly by agent_id (or by its exact row name).
fn detect_subagent(db: &AgentxDb, check_id: &str) -> Result<Option<InstanceRow>> {
    let name = db.get_instance_by_agent_id(check_id)?.unwrap_or_else(|| check_id.to_string());
    let row = db.get_instance_full(&name)?;
    Ok(row.filter(|row| row.parent_name.as_ref().is_some_and(|name| !name.is_empty())))
}

/// Promote an existing dormant child row into active agentx participation.
fn start_subagent_effects(db: &AgentxDb, request: &RequestContext, info: &InstanceRow) -> Result<StartOutput> {
    let mut output = StartOutput { participant: Some(participant(info)), ..Default::default() };
    let parent_name = info.parent_name.as_deref().unwrap_or("");
    if parent_name.is_empty() || info.agent_id.as_deref().unwrap_or("").is_empty() {
        bail!("Subagent row '{}' is missing parent/agent identity", info.name);
    }

    let was_announced = info.name_announced != 0;
    lifecycle::try_set_status(db, &info.name, ST_ACTIVE, "tool:start", Default::default())?;
    super::invocation_binding::capture_launch_context(db, request, &info.name)?;

    log_info("lifecycle", "start.subagent", &format!("name={} parent={} agent_id={} announced={}", info.name, parent_name, info.agent_id.as_deref().unwrap_or(""), was_announced));

    if was_announced {
        output.stdout.push_str(&format!("agentx already started for {}", info.name));
        output.stdout.push('\n');
        return Ok(output);
    }

    output.bootstrap = Some(Bootstrap::Subagent { parent: parent_name.to_string() });

    Ok(output)
}

/// Recover orphaned PTY process by PID or name.
fn start_from_orphan_effects(db: &AgentxDb, agentx_dir: &std::path::Path, target: &str, cleanup: &mut Vec<u32>) -> Result<StartOutput> {
    let mut output = StartOutput::default();
    let (orphans, adopted) = pidtrack::try_claim_orphans_deferred(db, agentx_dir).map_err(anyhow::Error::msg)?;
    cleanup.extend(adopted.iter().map(|(orphan, _)| orphan.pid));

    let target_pid = target.parse::<u32>().ok();
    let is_target = |o: &pidtrack::OrphanProcess| target_pid == Some(o.pid) || (target_pid.is_none() && o.names.iter().any(|n| n == target));

    // The PTY already rejoined under a live row, which now owns it again. A
    // real orphan with the same historical name still takes precedence.
    if !orphans.iter().any(is_target)
        && let Some((orphan, name)) = adopted.iter().find(|(o, _)| is_target(o))
    {
        output.participant = Some(current_participant(db, name)?);
        output.stdout.push_str(&format!("PID {} is already live as '{}'; nothing to recover.", orphan.pid, name));
        output.stdout.push('\n');
        return Ok(output);
    }

    if orphans.is_empty() {
        bail!("No orphan processes found.");
    }

    // Match by PID or name
    let orphan = if let Some(pid) = target_pid {
        match orphans.iter().find(|o| o.pid == pid) {
            Some(o) => o,
            None => bail!("Orphan PID {} not found.", pid),
        }
    } else {
        let matches: Vec<_> = orphans.iter().filter(|o| o.names.contains(&target.to_string())).collect();
        match matches.len() {
            0 => bail!("Orphan '{}' not found.", target),
            1 => matches[0],
            _ => {
                let pids: Vec<String> = matches.iter().map(|m| m.pid.to_string()).collect();
                bail!("Multiple orphans match '{}' (PIDs: {}). Use --orphan <pid>.", target, pids.join(", "));
            }
        }
    };

    let pid = orphan.pid;

    if orphan.process_id.is_empty() {
        bail!("Orphan PID {} has no process_id and cannot be recovered.", pid);
    }

    let preferred_name = orphan.names.last().cloned().unwrap_or_default();
    let can_reuse = !preferred_name.is_empty() && identity::is_valid_name(&preferred_name) && db.get_instance_full(&preferred_name)?.is_none();
    let name = if can_reuse { preferred_name } else { instance_names::allocate_unreserved_name(db)? };

    // Core DB registration
    if let Err(e) = pidtrack::recover_single_orphan_to_db(db, orphan, &name) {
        // Release a generated name's reservation row so retries don't strand
        // one pending placeholder per attempt.
        if !can_reuse {
            let _ = db.delete_instance(&name);
        }
        bail!("Failed to recover orphan PID {pid}: {e}");
    }

    db.log_event(
        "life",
        &name,
        &json!({
            "action": "started",
            "by": "cli",
            "reason": "orphan_recover",
            "orphan_pid": pid,
        }),
    )?;

    cleanup.push(pid);
    output.participant = Some(current_participant(db, &name)?);

    output.stdout.push_str(&format!("[agentx:{}]", name));
    output.stdout.push('\n');
    if can_reuse {
        output.stdout.push_str(&format!("Recovered orphan PID {} as '{}'.", pid, name));
        output.stdout.push('\n');
    } else {
        output.stdout.push_str(&format!("Recovered orphan PID {} as new identity '{}' (name conflict/unavailable).", pid, name));
        output.stdout.push('\n');
    }

    log_info("start", "orphan.recovered", &format!("name={} pid={} tool={}", name, pid, orphan.tool));

    Ok(output)
}

#[derive(Debug, Clone)]
struct ChildLink {
    name: String,
    parent_name: Option<String>,
}

fn snapshot_child_links(db: &AgentxDb, session_id: Option<&str>) -> Result<Vec<ChildLink>> {
    let Some(session_id) = session_id.filter(|value| !value.is_empty()) else {
        return Ok(Vec::new());
    };
    let mut stmt = db.conn().prepare("SELECT name, parent_name FROM instances WHERE parent_session_id = ?")?;
    let rows = stmt.query_map(rusqlite::params![session_id], |row| Ok(ChildLink { name: row.get(0)?, parent_name: row.get(1)? }))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn restore_child_links_after_root_rebind(db: &AgentxDb, links: &[ChildLink], session_id: &str, old_root: &str, new_root: &str) -> Result<()> {
    db.with_write_scope(|| {
        for link in links {
            let parent_name = match link.parent_name.as_deref() {
                Some(parent) if parent == old_root => Some(new_root),
                other => other,
            };
            db.conn().execute("UPDATE instances SET parent_session_id = ?, parent_name = ? WHERE name = ?", rusqlite::params![session_id, parent_name, &link.name])?;
        }
        Ok(())
    })
}

/// Rebind session identity (`--as <name>`), preserving last_event_id and any
/// live Claude child hierarchy owned by the current root actor.
fn start_rebind_effects(db: &AgentxDb, request: &RequestContext, rebind_target: &str, ctx: &AgentxContext, explicit_name: Option<&str>) -> Result<StartOutput> {
    let mut output = StartOutput::default();
    // Resolve the target name
    let target_name = rebind_target.to_string();

    // Guard: refuse to reclaim a subagent slot. Subagents share their parent's
    // session_id, so `agentx start --as <subagent_name>` from inside a subagent
    // bash would rebind session_bindings[parent_sid] to the subagent name,
    // clobbering the parent's identity. `--as` is documented for top-level
    // restartable identities (compaction/resume/clear), not for subagent
    // lifecycle — which has its own SubagentStart bootstrap path.
    if db.was_subagent_name(&target_name) {
        output.warnings.push(format!(
            "Error: '{target_name}' is a subagent slot; cannot be reclaimed with --as.\n\
             Subagents register via 'agentx start --name <agent-id>' in the SubagentStart context. If your session ended, stop working and end your turn."
        ));
        output.exit_code = 1;
        return Ok(output);
    }

    let explicit_current_name = explicit_name.unwrap_or("");

    // Resolve session_id from process binding or existing instance
    let mut session_id: Option<String> = None;
    if let Some(ref process_id) = ctx.process_id
        && let Some((sid, _)) = db.get_process_binding_full(process_id)?
    {
        session_id = sid.filter(|s| !s.is_empty());
    }
    if session_id.is_none()
        && !explicit_current_name.is_empty()
        && let Some(current_data) = db.get_instance_full(explicit_current_name)?
    {
        session_id = current_data.session_id.filter(|s| !s.is_empty());
    }
    if session_id.is_none() {
        session_id = resolve_launched_session_id(ctx);
    }
    // A direct per-run tool (plain `claude`/`codex`) has no hooks but still
    // exposes its native session id; bare start binds its adhoc identity to
    // it, so rebind must use it too or the old identity stays bound.
    // Classified from the context alone: an id found via `--name`'s row must
    // not turn a hookless direct session into a `claude`/`codex` row.
    let adhoc_session = !ctx.is_launched && ctx.process_id.is_none() && crate::integrations::hooks::runtime::is_per_run(ctx.tool);
    if adhoc_session && session_id.is_none() {
        session_id = resolve_native_session_id(ctx);
    }
    let tool = if !adhoc_session && (ctx.process_id.is_some() || session_id.is_some()) { ctx.tool.as_str() } else { "adhoc" };
    let current_name = if !explicit_current_name.is_empty() {
        explicit_current_name.to_string()
    } else if let Some(ref sid) = session_id {
        db.get_session_binding(sid)?.unwrap_or_default()
    } else {
        String::new()
    };
    let child_links = snapshot_child_links(db, session_id.as_deref())?;

    let target_meta = load_rebind_target_metadata(db, &target_name)?;
    if let Some(ref meta) = target_meta {
        ensure_rebind_compatible(&target_name, meta, ctx, tool)?;
    }

    // Preserve last_event_id from target (cursor preservation)
    let mut last_event_id = target_meta.as_ref().map(|m| m.last_event_id);
    let target_data = db.get_instance_full(&target_name)?;

    // Final fallback: use current max to avoid re-delivering old messages
    if last_event_id.is_none() {
        last_event_id = Some(db.get_last_event_id());
    }

    if target_data.as_ref().is_some_and(instances::is_remote_instance) {
        bail!("Remote start is not supported for '{target_name}'. Start it on the owning device instead.");
    }
    if target_data.is_some() {
        db.delete_instance(&target_name)?;
    }
    db.delete_process_bindings_for_instance(&target_name)?;
    db.delete_session_bindings_for_instance(&target_name)?;
    if !current_name.is_empty() && current_name != target_name {
        db.delete_instance(&current_name)?;
    }

    // Create fresh instance with the target name.
    let cwd_override = ctx.cwd.to_string_lossy().to_string();
    if !instance_binding::initialize_instance_with_context(
        db,
        &instance_binding::BindingContext::explicit(db, &request.caller.invocation),
        &target_name,
        session_id.as_deref(),
        None, // parent_session_id
        None, // parent_name
        None, // agent_id
        None, // transcript_path
        Some(tool),
        false, // background
        None,  // wait_timeout
        None,  // subagent_timeout
        None,  // hints
        Some(&cwd_override),
    ) {
        bail!("Failed to initialize participant identity");
    }

    if let Some(ref sid) = session_id {
        let old_root = if current_name.is_empty() { target_name.as_str() } else { current_name.as_str() };
        restore_child_links_after_root_rebind(db, &child_links, sid, old_root, &target_name)?;
        if old_root != target_name {
            db.rebind_claude_root_actor_state(sid, old_root, &target_name)?;
        }
    }

    // Preserve cursor; announcement is confirmed after client output flush.
    {
        let mut updates = serde_json::Map::new();
        if let Some(eid) = last_event_id {
            updates.insert("last_event_id".into(), serde_json::json!(eid));
        }
        db.update_instance_fields(&target_name, &updates)?;
    }

    if let Some(ref sid) = session_id {
        db.set_session_binding(sid, &target_name)?;
        if !adhoc_session && ctx.tool == crate::integrations::tool::Tool::Claude {
            db.mark_claude_session_validated(sid, &target_name)?;
        }
    }
    if let Some(ref process_id) = ctx.process_id {
        db.set_process_binding(process_id, session_id.as_deref().unwrap_or(""), &target_name)?;
        if !current_name.is_empty() && current_name != target_name {
            db.migrate_notify_endpoints(&current_name, &target_name)?;
        }
        db.wake_after_commit(&target_name, crate::runtime::notify::WakeKind::DELIVERY_LOOPS);
    }

    // The hook bind paths rename the pane on every bind; a rebind must too, or
    // the pane keeps advertising the identity this rebind just replaced.
    output.terminal_title = Some(target_name.clone());

    // Print bootstrap
    prepare_bootstrap(db, &mut output, &target_name, tool)?;

    log_info("start", "rebind.complete", &format!("from={} to={}", current_name, target_name));

    Ok(output)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RebindTargetMetadata {
    tool: String,
    directory: String,
    last_event_id: i64,
}

/// `row_tool` is the tool the reclaimed row will have: `adhoc` for a direct
/// per-run tool, which may reclaim either its own earlier adhoc identity or one
/// from an `agentx launch --tool <tool>` launch.
fn ensure_rebind_compatible(target_name: &str, meta: &RebindTargetMetadata, ctx: &AgentxContext, row_tool: &str) -> Result<()> {
    let current_tool = ctx.tool.as_str();
    if !meta.tool.is_empty() && meta.tool != current_tool && meta.tool != row_tool {
        bail!("Refusing to reclaim '{target_name}': latest identity used tool '{}' but current session is '{}'", meta.tool, current_tool);
    }

    let current_dir = ctx.cwd.to_string_lossy();
    if !meta.directory.is_empty() && !same_path(&meta.directory, &current_dir) {
        bail!("Refusing to reclaim '{target_name}': latest identity used directory '{}' but current session is '{}'", meta.directory, current_dir);
    }

    Ok(())
}

fn same_path(left: &str, right: &str) -> bool {
    normalize_path_for_compare(left) == normalize_path_for_compare(right)
}

fn normalize_path_for_compare(path: &str) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
}

/// Load rebind metadata from the live row first, then the latest stopped snapshot.
fn load_rebind_target_metadata(db: &AgentxDb, name: &str) -> Result<Option<RebindTargetMetadata>> {
    if let Some(inst) = db.get_instance_full(name)? {
        return Ok(Some(RebindTargetMetadata { tool: inst.tool, directory: inst.directory, last_event_id: inst.last_event_id }));
    }

    let mut stmt = db.conn().prepare("SELECT data FROM events WHERE type='life' AND instance=? ORDER BY id DESC LIMIT 10")?;

    let rows: Vec<String> = stmt.query_map(rusqlite::params![name], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;

    for data_str in &rows {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(data_str)
            && data.get("action").and_then(|v| v.as_str()) == Some("stopped")
            && let Some(snapshot) = data.get("snapshot")
        {
            return Ok(Some(RebindTargetMetadata {
                tool: snapshot.get("tool").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                directory: snapshot.get("directory").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                last_event_id: snapshot.get("last_event_id").and_then(|v| v.as_i64()).unwrap_or(0),
            }));
        }
    }

    Ok(None)
}

/// Resolve the Claude session id visible to a CLI invocation.
///
/// Claude sets `CLAUDE_CODE_SESSION_ID` in Bash and PowerShell subprocesses,
/// and it matches the `session_id` passed to hooks.
fn resolve_claude_session_id(env: &HashMap<String, String>) -> Option<String> {
    env.get("CLAUDE_CODE_SESSION_ID").filter(|value| !value.is_empty()).cloned()
}

/// Native session id of an `agentx launch --tool <tool>` launch (how a launched session binds
/// late). A direct run has no agentx hooks, so its id never identifies a hooked
/// session: it can only key an adhoc identity (see `start_bare`).
fn resolve_launched_session_id(ctx: &AgentxContext) -> Option<String> {
    ctx.process_id.is_some().then(|| resolve_native_session_id(ctx)).flatten()
}

fn resolve_native_session_id(ctx: &AgentxContext) -> Option<String> {
    match ctx.tool {
        crate::integrations::tool::Tool::Claude => resolve_claude_session_id(&ctx.raw_env),
        crate::integrations::tool::Tool::Codex => ctx.codex_thread_id.clone(),
        _ => None,
    }
}

/// Path C: Bare start — detect tool or create adhoc instance.
fn start_bare_effects(db: &AgentxDb, request: &RequestContext, ctx: &AgentxContext, explicit_name: Option<&str>) -> Result<StartOutput> {
    let mut output = StartOutput::default();
    let explicit_name = explicit_name.map(|name| name.to_string());
    let explicit_name = explicit_name.as_deref();

    let launched_session_id = resolve_launched_session_id(ctx);
    // A direct per-run tool has no hooks, but its native ID can still make a
    // repeated manual `agentx start` return the same adhoc identity.
    let adhoc_session_id = (!ctx.is_launched && ctx.process_id.is_none() && crate::integrations::hooks::runtime::is_per_run(ctx.tool)).then(|| resolve_native_session_id(ctx)).flatten();
    let session_id = launched_session_id.as_ref().or(adhoc_session_id.as_ref());
    let tool = if ctx.process_id.is_some() { ctx.tool.as_str() } else { "adhoc" };

    if explicit_name.is_none()
        && let Some(session_id) = session_id
        && let Some(bound_name) = db.get_session_binding(session_id)?
    {
        // Only agentx writes session bindings, so a row keyed by this session's
        // own id is trusted identity evidence. Heal bindings created by older
        // versions before returning the existing row.
        if launched_session_id.is_some() && ctx.tool == crate::integrations::tool::Tool::Claude {
            db.mark_claude_session_validated(session_id, &bound_name)?;
        }
        output.participant = Some(current_participant(db, &bound_name)?);
        output.stdout.push_str(&format!("agentx already started for {bound_name}"));
        output.stdout.push('\n');
        return Ok(output);
    }

    // An AGENTX_PROCESS_ID that still belongs to a live instance is that agent's
    // own shell: never mint a second identity or repoint its binding.
    let process_owner = match ctx.process_id.as_deref() {
        Some(pid) => match db.get_process_binding(pid)? {
            Some(owner) => db.get_instance_full(&owner)?,
            None => None,
        },
        None => None,
    };
    if explicit_name.is_none()
        && let Some(ref owner) = process_owner
    {
        // Launched but its session not bound yet (SessionStart pending or
        // failed): connect it here instead of leaving it unregistered.
        if owner.session_id.as_deref().is_none_or(str::is_empty)
            && let Some(ref session_id) = launched_session_id
            && let Some(bound) = instance_binding::bind_session_with_context(db, &instance_binding::BindingContext::explicit(db, &request.caller.invocation), session_id, ctx.process_id.as_deref())?
        {
            if ctx.tool == crate::integrations::tool::Tool::Claude {
                db.mark_claude_session_validated(session_id, &bound)?;
            }
            prepare_bootstrap(db, &mut output, &bound, &owner.tool)?;
            db.log_event(
                "life",
                &bound,
                &json!({
                    "action": "started",
                    "tool": owner.tool,
                    "name": bound,
                }),
            )?;
            return Ok(output);
        }
        output.participant = Some(participant(owner));
        output.stdout.push_str(&format!("agentx already started for {}", owner.name));
        output.stdout.push('\n');
        return Ok(output);
    }

    // Resolve or generate name
    let name = if let Some(n) = explicit_name { n.to_string() } else { instance_names::allocate_unreserved_name(db)? };

    // Remote instances are relay mirrors. Starting them remotely is intentionally
    // unsupported because the useful remote lifecycle operations are launch/resume/kill.
    if let Some(ref existing) = db.get_instance_full(&name)?
        && crate::agents::records::is_remote_instance(existing)
    {
        bail!("Remote start is not supported for '{name}'. Start it on the owning device instead.");
    }

    // Check if already exists and active (only for explicit names —
    // generate_unique_name creates a placeholder row we must skip past)
    if explicit_name.is_some()
        && let Some(existing) = db.get_instance_full(&name)?
        && existing.status != "stopped"
    {
        output.participant = Some(participant(&existing));
        output.stdout.push_str(&format!("agentx already started for {}", name));
        output.stdout.push('\n');
        return Ok(output);
    }

    if !instance_binding::initialize_instance_with_context(
        db,
        &instance_binding::BindingContext::explicit(db, &request.caller.invocation),
        &name,
        session_id.map(String::as_str),
        None, // parent_session_id
        None, // parent_name
        None, // agent_id
        None, // transcript_path
        Some(tool),
        false, // background
        None,  // wait_timeout
        None,  // subagent_timeout
        None,  // hints
        None,  // cwd_override
    ) {
        bail!("Failed to initialize participant identity");
    }

    if let Some(session_id) = session_id {
        db.set_session_binding(session_id, &name)?;
        if launched_session_id.is_some() && ctx.tool == crate::integrations::tool::Tool::Claude {
            db.mark_claude_session_validated(session_id, &name)?;
        }
    }

    if let Some(ref process_id) = ctx.process_id
        && process_owner.as_ref().is_none_or(|owner| owner.name == name)
    {
        db.set_process_binding(process_id, "", &name)?;
    }

    prepare_bootstrap(db, &mut output, &name, tool)?;

    // Log
    db.log_event(
        "life",
        &name,
        &json!({
            "action": "started",
            "tool": tool,
            "name": name,
        }),
    )?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::start::StartArgs;
    use clap::Parser;
    use rusqlite::params;
    use serde_json::json;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn native_start_replay_conflict_and_replacement_fence() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let ctx = make_ctx(&[], "/tmp/project");
        let request = request(&ctx);
        let first = retained_start(&db, &request, StartMode::Participate, None).unwrap();
        let replay = retained_start(&db, &request, StartMode::Participate, None).unwrap();
        assert_eq!(first.participant, replay.participant);
        assert_eq!(first.bootstrap_receipt, replay.bootstrap_receipt);
        assert_eq!(db.iter_instances_full().unwrap().len(), 1);
        assert!(matches!(retained_start(&db, &request, StartMode::Participate, Some("changed".into())), Err(ServiceError::Conflict(_))));
        let participant = first.participant.unwrap();
        db.conn().execute("UPDATE instances SET created_at=created_at+1 WHERE name=?", [&participant.canonical_name]).unwrap();
        assert_eq!(retained_start(&db, &request, StartMode::Participate, None).unwrap_err(), ServiceError::StaleFence);
    }

    #[test]
    fn native_bootstrap_receipt_requires_flush_confirmation_and_exact_caller() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let request = request(&make_ctx(&[], "/tmp/project"));
        let started = retained_start(&db, &request, StartMode::Participate, None).unwrap();
        let participant = started.participant.unwrap();
        let receipt = started.bootstrap_receipt.unwrap();
        assert_eq!(db.get_instance_full(&participant.canonical_name).unwrap().unwrap().name_announced, 0, "preparing or losing stdout must not announce");
        let mut other = request.clone();
        other.process.pid += 1;
        assert_eq!(confirm_bootstrap(&db, &other, &participant, &receipt).unwrap_err(), ServiceError::StaleFence);
        confirm_bootstrap(&db, &request, &participant, &receipt).unwrap();
        assert_eq!(db.get_instance_full(&participant.canonical_name).unwrap().unwrap().name_announced, 1);
        db.conn().execute("UPDATE instances SET created_at=created_at+1 WHERE name=?", [&participant.canonical_name]).unwrap();
        assert_eq!(confirm_bootstrap(&db, &request, &participant, &receipt).unwrap_err(), ServiceError::StaleFence);
    }

    #[test]
    fn native_reclaim_required_write_failure_rolls_back_identity_and_acceptance() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn().execute_batch("INSERT INTO instances(name,tool,directory,session_id,status,created_at,last_event_id) VALUES('old','adhoc','/tmp/project','session','active',1,3),('reclaimed','adhoc','/tmp/project',NULL,'inactive',2,7); CREATE TRIGGER fail_reclaim_cursor BEFORE UPDATE OF last_event_id ON instances WHEN NEW.name='reclaimed' BEGIN SELECT RAISE(ABORT,'injected cursor failure'); END;").unwrap();
        db.set_session_binding("session", "old").unwrap();
        let request = request(&make_ctx(&[], "/tmp/project"));
        assert!(retained_start(&db, &request, StartMode::Reclaim { name: "reclaimed".into() }, Some("old".into())).is_err());
        assert_eq!(db.get_instance_full("old").unwrap().unwrap().created_at, 1.0);
        assert_eq!(db.get_instance_full("reclaimed").unwrap().unwrap().created_at, 2.0);
        assert_eq!(db.get_instance_full("reclaimed").unwrap().unwrap().last_event_id, 7);
        assert_eq!(db.get_session_binding("session").unwrap().as_deref(), Some("old"));
        assert!(db.kv_get(&format!("participant_start:{}:{}", request.database_generation, request.caller.request_id.0)).unwrap().is_none());
    }

    fn request(ctx: &AgentxContext) -> RequestContext {
        RequestContext {
            caller: crate::service::CallerContext { request_id: crate::service::RequestId(uuid::Uuid::new_v4().to_string()), invocation: crate::runtime::host_client::invocation(ctx), deadline_unix_ms: None, expected_database_generation: None },
            process: crate::service::CallerProcess { pid: std::process::id(), incarnation: Some("fixture".into()) },
            daemon_instance: "fixture".into(),
            database_generation: "fixture".into(),
        }
    }

    fn kill_fixture(db: &AgentxDb) -> ParticipantRef {
        db.conn().execute_batch("INSERT INTO instances(name,tool,directory,created_at,pid,status,status_context,status_detail,status_time,launch_context) VALUES('victim','adhoc','/fixture',1,4242,'listening','ready','original',123,'{\"pid_identity\":\"original-process\"}'); INSERT INTO instances(name,tool,directory,created_at,pid,parent_name,launch_context) VALUES('child','adhoc','/fixture',2,4243,'victim','{\"pid_identity\":\"child-process\"}');").unwrap();
        ParticipantRef { canonical_name: "victim".into(), incarnation_bits: 1f64.to_bits() }
    }

    #[test]
    fn native_operator_kill_retains_child_first_outcome_and_rejects_successor_replay() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let fence = kill_fixture(&db);
        let context = request(&make_ctx(&[], "/operator"));
        let plan = prepare_kill(&db, &context, &fence, "operator").unwrap();
        assert_eq!(prepare_kill(&db, &context, &fence, "operator").unwrap().ticket, plan.ticket);
        assert!(matches!(prepare_kill(&db, &context, &fence, "changed"), Err(ServiceError::Conflict(_))));
        begin_kill(&db, &context, &plan.ticket).unwrap();
        assert!(matches!(begin_kill(&db, &context, &plan.ticket), Err(ServiceError::OutcomeUnknown(_))));
        let outcome = complete_kill(&db, &context, &plan.ticket).unwrap();
        assert!(outcome.stopped);
        assert_eq!(outcome.effects.iter().map(|effect| effect.record.name.as_str()).collect::<Vec<_>>(), vec!["child", "victim"]);
        assert_eq!(outcome.effects[1].process_identity.as_deref(), Some("original-process"));
        let events = db.get_last_event_id();
        let recovered = complete_kill(&db, &context, &plan.ticket).unwrap();
        assert_eq!(serde_json::to_value(&outcome).unwrap(), serde_json::to_value(&recovered).unwrap());
        assert_eq!(db.get_last_event_id(), events, "recovery cannot emit another lifecycle event");
        db.conn().execute("INSERT INTO instances(name,tool,created_at,pid) VALUES('victim','adhoc',3,4242)", []).unwrap();
        assert_eq!(complete_kill(&db, &context, &plan.ticket).unwrap_err(), ServiceError::StaleFence);
    }

    #[test]
    fn native_operator_kill_peer_generation_and_pid_are_fenced() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let fence = kill_fixture(&db);
        let context = request(&make_ctx(&[], "/operator"));
        let plan = prepare_kill(&db, &context, &fence, "operator").unwrap();
        let mut other = context.clone();
        other.process.incarnation = Some("another-peer".into());
        assert_eq!(begin_kill(&db, &other, &plan.ticket).unwrap_err(), ServiceError::StaleFence);
        other = context.clone();
        other.database_generation = "replacement-database".into();
        assert_eq!(begin_kill(&db, &other, &plan.ticket).unwrap_err(), ServiceError::StaleFence);
        db.conn().execute("UPDATE instances SET pid=4244 WHERE name='victim'", []).unwrap();
        assert_eq!(begin_kill(&db, &context, &plan.ticket).unwrap_err(), ServiceError::StaleFence);
        assert_eq!(deny_kill(&db, &context, &plan.ticket).unwrap_err(), ServiceError::StaleFence);
    }

    #[test]
    fn native_operator_kill_recovers_exact_hook_retirement_without_signalling_again() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let fence = kill_fixture(&db);
        let context = request(&make_ctx(&[], "/operator"));
        let plan = prepare_kill(&db, &context, &fence, "operator").unwrap();
        begin_kill(&db, &context, &plan.ticket).unwrap();
        let retired = db.with_write_scope(|| super::super::hook_store::stop_killed(&db, &context, &fence).map_err(anyhow::Error::new)).unwrap();
        let before = db.get_last_event_id();
        let completed = complete_kill(&db, &context, &plan.ticket).unwrap();
        assert_eq!(serde_json::to_value(&completed).unwrap(), serde_json::to_value(&retired).unwrap());
        assert_eq!(db.get_last_event_id(), before);
    }

    #[test]
    fn native_operator_kill_absence_without_retirement_is_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let fence = kill_fixture(&db);
        let context = request(&make_ctx(&[], "/operator"));
        let plan = prepare_kill(&db, &context, &fence, "operator").unwrap();
        begin_kill(&db, &context, &plan.ticket).unwrap();
        db.delete_instance("victim").unwrap();
        assert!(matches!(complete_kill(&db, &context, &plan.ticket), Err(ServiceError::OutcomeUnknown(_))));
        assert!(matches!(load_kill(&db, &context, &plan.ticket).unwrap().state, KillState::Issuing));
    }

    #[test]
    fn native_operator_denial_restores_exact_status_and_preserves_hook_progress() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let fence = kill_fixture(&db);
        let mut context = request(&make_ctx(&[], "/operator"));
        let first = prepare_kill(&db, &context, &fence, "operator").unwrap();
        begin_kill(&db, &context, &first.ticket).unwrap();
        deny_kill(&db, &context, &first.ticket).unwrap();
        let restored = db.get_instance_full("victim").unwrap().unwrap();
        assert_eq!((restored.status.as_str(), restored.status_context.as_str(), restored.status_detail.as_str(), restored.status_time), ("listening", "ready", "original", 123));
        context.caller.request_id.0 = "second-attempt".into();
        let next = prepare_kill(&db, &context, &fence, "operator").unwrap();
        db.conn().execute("UPDATE instances SET status='active',status_context='new-hook',status_detail='new work' WHERE name='victim'", []).unwrap();
        deny_kill(&db, &context, &next.ticket).unwrap();
        assert_eq!(db.get_instance_full("victim").unwrap().unwrap().status_context, "new-hook");
        assert_eq!(db.iter_instances_full().unwrap().len(), 2);
    }

    #[test]
    fn native_operator_completion_retention_failure_rolls_back_whole_stop() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let fence = kill_fixture(&db);
        let context = request(&make_ctx(&[], "/operator"));
        let plan = prepare_kill(&db, &context, &fence, "operator").unwrap();
        begin_kill(&db, &context, &plan.ticket).unwrap();
        let before = db.get_last_event_id();
        db.conn().execute_batch("CREATE TRIGGER fail_kill_retention BEFORE INSERT ON kv WHEN NEW.key LIKE 'participant_kill:%' BEGIN SELECT RAISE(ABORT,'injected kill result failure'); END;").unwrap();
        assert!(complete_kill(&db, &context, &plan.ticket).is_err());
        assert_eq!(db.iter_instances_full().unwrap().len(), 2);
        assert_eq!(db.get_last_event_id(), before);
        assert!(matches!(load_kill(&db, &context, &plan.ticket).unwrap().state, KillState::Issuing));
        db.conn().execute_batch("DROP TRIGGER fail_kill_retention;").unwrap();
        assert!(complete_kill(&db, &context, &plan.ticket).unwrap().stopped);
    }

    #[test]
    fn native_operator_parent_delete_failure_restores_children_and_bindings() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let fence = kill_fixture(&db);
        db.set_process_binding("child-binding", "child-session", "child").unwrap();
        let context = request(&make_ctx(&[], "/operator"));
        let plan = prepare_kill(&db, &context, &fence, "operator").unwrap();
        begin_kill(&db, &context, &plan.ticket).unwrap();
        let before = db.get_last_event_id();
        db.conn().execute_batch("CREATE TRIGGER fail_kill_parent BEFORE DELETE ON instances WHEN OLD.name='victim' BEGIN SELECT RAISE(ABORT,'injected parent deletion failure'); END;").unwrap();
        assert!(complete_kill(&db, &context, &plan.ticket).is_err());
        assert_eq!(db.iter_instances_full().unwrap().len(), 2);
        assert_eq!(db.get_process_binding("child-binding").unwrap().as_deref(), Some("child"));
        assert_eq!(db.get_last_event_id(), before);
    }
    fn start_bare(db: &AgentxDb, ctx: &AgentxContext, name: Option<&str>) -> Result<i32> {
        start_bare_effects(db, &request(ctx), ctx, name).map(|output| output.exit_code)
    }
    fn start_rebind(db: &AgentxDb, name: &str, ctx: &AgentxContext, current: Option<&str>) -> Result<i32> {
        let output = start_rebind_effects(db, &request(ctx), name, ctx, current)?;
        if let Some(title) = output.terminal_title {
            crate::runtime::env::set_terminal_title(&title);
        }
        Ok(output.exit_code)
    }
    fn start_from_orphan(db: &AgentxDb, scope: &std::path::Path, target: &str, _ctx: &AgentxContext) -> Result<i32> {
        let mut cleanup = Vec::new();
        let result = db.with_write_scope(|| start_from_orphan_effects(db, scope, target, &mut cleanup))?;
        for pid in cleanup {
            pidtrack::remove_pid(scope, pid);
        }
        Ok(result.exit_code)
    }

    fn make_ctx(tool_env: &[(&str, &str)], cwd: &str) -> AgentxContext {
        let mut env = HashMap::new();
        for (k, v) in tool_env {
            env.insert((*k).to_string(), (*v).to_string());
        }
        AgentxContext::from_env(&env, PathBuf::from(cwd))
    }

    /// Claude context carrying exactly one session-id source, so an ambient
    /// value from the shell running the tests cannot decide the outcome.
    fn make_claude_ctx(session: Option<(&str, &str)>, cwd: &str) -> AgentxContext {
        let mut env = HashMap::new();
        env.insert("CLAUDECODE".to_string(), "1".to_string());
        if let Some((key, value)) = session {
            env.insert(key.to_string(), value.to_string());
        }
        AgentxContext::from_env(&env, PathBuf::from(cwd))
    }

    fn log_stopped_snapshot(db: &AgentxDb, name: &str, tool: &str, directory: &str, session_id: &str, last_event_id: i64) {
        db.log_event(
            "life",
            name,
            &json!({
                "action": "stopped",
                "snapshot": {
                    "tool": tool,
                    "directory": directory,
                    "session_id": session_id,
                    "last_event_id": last_event_id
                }
            }),
        )
        .unwrap();
    }
    #[test]
    fn test_start_args_bare() {
        let args = StartArgs::try_parse_from(["start"]).unwrap();
        assert!(args.orphan.is_none());
        assert!(args.as_name.is_none());
    }

    #[test]
    fn test_start_args_orphan() {
        let args = StartArgs::try_parse_from(["start", "--orphan", "1234"]).unwrap();
        assert_eq!(args.orphan, Some("1234".to_string()));
        assert!(args.as_name.is_none());
    }

    #[test]
    fn test_start_args_rebind() {
        let args = StartArgs::try_parse_from(["start", "--as", "luna"]).unwrap();
        assert!(args.orphan.is_none());
        assert_eq!(args.as_name, Some("luna".to_string()));
    }

    #[test]
    fn test_start_args_bare_as_errors() {
        let err = StartArgs::try_parse_from(["start", "--as"]);
        assert!(err.is_err());
    }

    #[test]
    fn test_start_args_bare_orphan_errors() {
        let err = StartArgs::try_parse_from(["start", "--orphan"]);
        assert!(err.is_err());
    }

    #[test]
    fn test_start_args_unknown_flag_errors() {
        let err = StartArgs::try_parse_from(["start", "--bogus"]);
        assert!(err.is_err());
    }

    #[test]
    #[serial]
    fn test_start_rejects_remote_instances() {
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        db.conn().execute("INSERT INTO instances (name, origin_device_id, created_at) VALUES (?1, ?2, ?3)", params!["luna:ABCD", "remote-device", crate::shared::time::now_epoch_f64()]).unwrap();

        let ctx = make_ctx(&[], "/tmp/project");
        let err = start_bare(&db, &ctx, Some("luna:ABCD")).unwrap_err();
        assert!(err.to_string().contains("Remote start is not supported"), "unexpected error: {err}");
    }

    #[test]
    fn test_resolve_claude_session_id() {
        let env = |value: Option<&str>| -> HashMap<String, String> { value.map(|value| HashMap::from([("CLAUDE_CODE_SESSION_ID".to_string(), value.to_string())])).unwrap_or_default() };

        assert_eq!(resolve_claude_session_id(&env(Some("claude-sess"))), Some("claude-sess".to_string()));
        assert_eq!(resolve_claude_session_id(&env(Some(""))), None);
        assert_eq!(resolve_claude_session_id(&env(None)), None);
    }

    #[test]
    fn codex_native_identity_prefers_session_and_supports_older_builds() {
        // Only launched Codex binds its native id; a plain run joins as adhoc.
        for (pairs, expected) in [(vec![("CODEX_THREAD_ID", "thread")], "thread"), (vec![("CODEX_THREAD_ID", "thread"), ("CODEX_SESSION_ID", "root")], "root")] {
            let env = pairs.into_iter().chain([("AGENTX_PROCESS_ID", "pid-codex")]).map(|(k, v)| (k.to_string(), v.to_string())).collect();
            let ctx = AgentxContext::from_env(&env, PathBuf::from("/tmp"));
            assert_eq!(resolve_launched_session_id(&ctx).as_deref(), Some(expected));
        }
    }

    #[test]
    #[serial]
    fn manual_tools_without_native_identity_start_as_adhoc() {
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        for tool in ["gemini", "antigravity", "claude", "codex", "pi"] {
            let env = HashMap::from([("AGENTX_TOOL".to_string(), tool.to_string())]);
            let mut ctx = AgentxContext::from_env(&env, PathBuf::from("/tmp"));
            ctx.tool = tool.parse().unwrap();
            assert_eq!(start_bare(&db, &ctx, None).unwrap(), 0);
        }
        let rows = db.iter_instances_full().unwrap();
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().all(|row| row.tool == "adhoc" && row.session_id.is_none()));
    }

    #[test]
    #[serial]
    fn test_plain_claude_start_joins_adhoc_without_installing_hooks() {
        let (_dir, _agentx_dir, home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();

        // Claude is per-run: a direct `claude` run joins as adhoc without
        // installing hooks, and its native ID deduplicates manual starts.
        let ctx = make_claude_ctx(Some(("CLAUDE_CODE_SESSION_ID", "sess-plain")), "/tmp/project");
        assert_eq!(start_bare(&db, &ctx, None).unwrap(), 0);

        assert_eq!(start_bare(&db, &ctx, None).unwrap(), 0);
        let rows = db.iter_instances_full().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].tool, "adhoc");
        assert_eq!(rows[0].session_id.as_deref(), Some("sess-plain"));
        assert_eq!(db.get_session_binding("sess-plain").unwrap(), Some(rows[0].name.clone()));
        assert!(!home.join(".claude").join("settings.json").exists());
    }

    #[test]
    #[serial]
    fn test_plain_codex_start_reuses_adhoc_identity() {
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        let env = HashMap::from([("CODEX_THREAD_ID".to_string(), "thread-plain".to_string())]);
        let ctx = AgentxContext::from_env(&env, PathBuf::from("/tmp"));
        assert_eq!(start_bare(&db, &ctx, None).unwrap(), 0);
        assert_eq!(start_bare(&db, &ctx, None).unwrap(), 0);
        let rows = db.iter_instances_full().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].tool, "adhoc");
        assert_eq!(rows[0].session_id.as_deref(), Some("thread-plain"));
    }

    #[test]
    #[serial]
    fn test_start_never_takes_over_a_live_process_binding() {
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        let launched_ctx = |session: Option<&str>| {
            let mut env = HashMap::from([("CLAUDECODE".to_string(), "1".to_string()), ("AGENTX_LAUNCHED".to_string(), "1".to_string()), ("AGENTX_PROCESS_ID".to_string(), "pid-parent".to_string())]);
            if let Some(sid) = session {
                env.insert("CLAUDE_CODE_SESSION_ID".to_string(), sid.to_string());
            }
            AgentxContext::from_env(&env, PathBuf::from("/tmp/project"))
        };
        assert_eq!(start_bare(&db, &launched_ctx(Some("sess-parent")), None).unwrap(), 0);
        let parent = db.get_process_binding("pid-parent").unwrap().unwrap();

        // The parent's own shell without a session var: still the parent.
        assert_eq!(start_bare(&db, &launched_ctx(None), None).unwrap(), 0);
        assert_eq!(db.iter_instances_full().unwrap().len(), 1);

        // Another session carrying the same process id never repoints it.
        assert_eq!(start_bare(&db, &launched_ctx(Some("sess-other")), None).unwrap(), 0);
        assert_eq!(db.iter_instances_full().unwrap().len(), 1);
        assert_eq!(db.get_process_binding("pid-parent").unwrap().as_deref(), Some(parent.as_str()));
    }

    #[test]
    #[serial]
    fn test_start_binds_session_to_launched_placeholder() {
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        // Launcher pre-registered the process; SessionStart has not bound it.
        instance_binding::initialize_instance_in_position_file(&db, "luna", None, None, None, None, None, Some("claude"), false, None, None, None, None);
        db.set_process_binding("pid-launch", "", "luna").unwrap();

        let env =
            HashMap::from([("CLAUDECODE".to_string(), "1".to_string()), ("AGENTX_LAUNCHED".to_string(), "1".to_string()), ("AGENTX_PROCESS_ID".to_string(), "pid-launch".to_string()), ("CLAUDE_CODE_SESSION_ID".to_string(), "sess-late".to_string())]);
        let ctx = AgentxContext::from_env(&env, PathBuf::from("/tmp/project"));
        assert_eq!(start_bare(&db, &ctx, None).unwrap(), 0);

        assert_eq!(db.get_session_binding("sess-late").unwrap().as_deref(), Some("luna"));
        assert_eq!(db.iter_instances_full().unwrap().len(), 1);
    }

    #[test]
    #[serial]
    fn test_root_rebind_preserves_child_hierarchy_and_actor_state() {
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();

        db.conn()
            .execute(
                "INSERT INTO instances
                 (name, session_id, tool, status, status_time, last_seen, created_at)
                 VALUES ('nova', 'sess-1', 'claude', 'active', 0, 0, 0)",
                [],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances
                 (name, parent_session_id, parent_name, agent_id, tool, status,
                  status_time, last_seen, created_at)
                 VALUES ('nova_task_1', 'sess-1', 'nova', 'agent-1', 'claude',
                         'active', 0, 0, 0)",
                [],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances
                 (name, parent_session_id, parent_name, agent_id, tool, status,
                  status_time, last_seen, created_at)
                 VALUES ('nova_task_2', 'sess-1', 'nova_task_1', 'agent-2', 'claude',
                         'active', 0, 0, 0)",
                [],
            )
            .unwrap();

        let token = db.issue_claude_actor_capability("sess-1", "tool-root", None, "nova").unwrap();

        let links = snapshot_child_links(&db, Some("sess-1")).unwrap();
        assert_eq!(links.len(), 2);
        db.delete_instance("nova").unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances
                 (name, session_id, tool, status, status_time, last_seen, created_at)
                 VALUES ('sol', 'sess-1', 'claude', 'active', 0, 0, 0)",
                [],
            )
            .unwrap();

        restore_child_links_after_root_rebind(&db, &links, "sess-1", "nova", "sol").unwrap();
        db.rebind_claude_root_actor_state("sess-1", "nova", "sol").unwrap();

        let direct = db.get_instance_full("nova_task_1").unwrap().unwrap();
        assert_eq!(direct.parent_session_id.as_deref(), Some("sess-1"));
        assert_eq!(direct.parent_name.as_deref(), Some("sol"));
        let nested = db.get_instance_full("nova_task_2").unwrap().unwrap();
        assert_eq!(nested.parent_session_id.as_deref(), Some("sess-1"));
        assert_eq!(nested.parent_name.as_deref(), Some("nova_task_1"));
        assert_eq!(db.resolve_claude_actor_capability(&token, "sess-1").unwrap(), Some("sol".to_string()));
    }

    #[test]
    #[serial]
    fn test_same_name_root_rebind_restores_child_session_links() {
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances
                 (name, session_id, tool, directory, status, status_time, last_seen, created_at)
                 VALUES ('nova', 'sess-1', 'claude', '/tmp/project', 'active', 0, 0, 1)",
                [],
            )
            .unwrap();
        db.set_session_binding("sess-1", "nova").unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances
                 (name, parent_session_id, parent_name, agent_id, tool, status,
                  status_time, last_seen, created_at)
                 VALUES ('nova_task_1', 'sess-1', 'nova', 'agent-1', 'claude',
                         'active', 0, 0, 2)",
                [],
            )
            .unwrap();
        let token = db.issue_claude_actor_capability("sess-1", "tool-child", Some("agent-1"), "nova_task_1").unwrap();

        let ctx = make_ctx(&[("CLAUDECODE", "1")], "/tmp/project");
        assert_eq!(start_rebind(&db, "nova", &ctx, Some("nova")).unwrap(), 0);

        let child = db.get_instance_full("nova_task_1").unwrap().unwrap();
        assert_eq!(child.parent_session_id.as_deref(), Some("sess-1"));
        assert_eq!(child.parent_name.as_deref(), Some("nova"));
        assert_eq!(db.resolve_claude_actor_capability(&token, "sess-1").unwrap(), Some("nova_task_1".to_string()));
    }

    #[test]
    #[serial]
    fn test_start_rebind_rejects_cross_tool_stopped_snapshot_hijack() {
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();

        log_stopped_snapshot(&db, "fama", "codex", "/tmp/dasha-code/.worktrees/layer1-basic-conversation-fixes", "sid-fama", 42);

        let ctx = make_ctx(&[("CLAUDECODE", "1")], "/tmp/agentx-gan-harness/.worktrees/bench-infra");

        let err = start_rebind(&db, "fama", &ctx, None).unwrap_err();
        assert!(err.to_string().contains("Refusing to reclaim 'fama'"), "unexpected error: {err}");

        assert!(db.get_instance_full("fama").unwrap().is_none());
        assert_eq!(db.get_session_binding("sid-fama").unwrap(), None);
    }

    #[test]
    #[serial]
    fn test_start_rebind_allows_matching_stopped_snapshot_reclaim() {
        for session_id in [None, Some("sid-current")] {
            let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
            let db = AgentxDb::open().unwrap();

            log_stopped_snapshot(&db, "nova", "claude", "/tmp/dasha-code/.worktrees/layer1-basic-conversation-fixes", "sid-nova", 77);

            let ctx = make_claude_ctx(session_id.map(|sid| ("CLAUDE_CODE_SESSION_ID", sid)), "/tmp/dasha-code/.worktrees/layer1-basic-conversation-fixes");

            let exit_code = start_rebind(&db, "nova", &ctx, None).unwrap();
            assert_eq!(exit_code, 0);

            let inst = db.get_instance_full("nova").unwrap().unwrap();
            // A direct (per-run) Claude has no hooks: the reclaim is adhoc,
            // bound to Claude's session id when it exposed one.
            assert_eq!(inst.tool, "adhoc");
            assert_eq!(inst.session_id.as_deref(), session_id);
            assert_eq!(inst.directory, "/tmp/dasha-code/.worktrees/layer1-basic-conversation-fixes");
            assert_eq!(inst.last_event_id, 77);
        }
    }

    #[test]
    #[serial]
    fn test_plain_claude_rebind_moves_session_binding_to_target() {
        // Targets aren't consonant/vowel-alternating like generated names,
        // so the bare start's random identity can never collide with them.
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        let ctx = make_claude_ctx(Some(("CLAUDE_CODE_SESSION_ID", "sess-plain")), "/tmp/project");

        assert_eq!(start_bare(&db, &ctx, None).unwrap(), 0);
        let first = db.get_session_binding("sess-plain").unwrap().unwrap();

        assert_eq!(start_rebind(&db, "rebound", &ctx, None).unwrap(), 0);
        assert_eq!(db.get_session_binding("sess-plain").unwrap().as_deref(), Some("rebound"));
        assert!(db.get_instance_full(&first).unwrap().is_none());
        let rebound = db.get_instance_full("rebound").unwrap().unwrap();
        assert_eq!(rebound.tool, "adhoc");
        assert_eq!(db.get_validated_claude_session_owner("sess-plain").unwrap(), None, "an adhoc identity must not enter Claude's hook validation cache");

        // `--name` resolving the session through the current row keeps the
        // target adhoc too.
        assert_eq!(start_rebind(&db, "renamed", &ctx, Some("rebound")).unwrap(), 0);
        assert_eq!(db.get_instance_full("renamed").unwrap().unwrap().tool, "adhoc");
        assert_eq!(db.get_validated_claude_session_owner("sess-plain").unwrap(), None);
        assert_eq!(start_rebind(&db, "rebound", &ctx, None).unwrap(), 0);

        // A later bare start returns the rebound identity, and the adhoc
        // identity can be reclaimed again from the same plain Claude.
        assert_eq!(start_bare(&db, &ctx, None).unwrap(), 0);
        assert_eq!(db.iter_instances_full().unwrap().len(), 1);
        assert_eq!(start_rebind(&db, "rebound", &ctx, None).unwrap(), 0);
        assert_eq!(db.iter_instances_full().unwrap().len(), 1);
    }

    #[test]
    #[serial]
    fn test_start_rebind_rejects_cross_directory_stopped_snapshot_hijack() {
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();

        log_stopped_snapshot(&db, "mira", "claude", "/tmp/dasha-code/.worktrees/layer1-basic-conversation-fixes", "sid-mira", 18);

        let ctx = make_ctx(&[("CLAUDECODE", "1")], "/tmp/agentx-gan-harness/.worktrees/bench-infra");

        let err = start_rebind(&db, "mira", &ctx, None).unwrap_err();
        assert!(err.to_string().contains("Refusing to reclaim 'mira'"), "unexpected error: {err}");

        assert!(db.get_instance_full("mira").unwrap().is_none());
    }

    #[test]
    #[cfg(unix)]
    fn test_same_path_resolves_symlink_aliases() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        let alias = dir.path().join("alias");
        std::fs::create_dir_all(&real).unwrap();
        std::os::unix::fs::symlink(&real, &alias).unwrap();

        assert!(same_path(real.to_string_lossy().as_ref(), alias.to_string_lossy().as_ref()));
    }

    #[test]
    #[serial]
    fn rebind_renames_the_pane_to_the_reclaimed_name() {
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();

        let ctx = make_claude_ctx(Some(("CLAUDE_CODE_SESSION_ID", "sess-title")), "/tmp/project");
        assert_eq!(start_bare(&db, &ctx, None).unwrap(), 0);

        // Longer than generated names (4-letter CVCV), so the bare start's
        // random identity can never collide with the target.
        let _ = crate::runtime::env::take_last_terminal_title();
        assert_eq!(start_rebind(&db, "rebound", &ctx, None).unwrap(), 0);
        assert_eq!(crate::runtime::env::take_last_terminal_title().as_deref(), Some("rebound"), "a rebind must rename the pane, or the pane keeps advertising the old name");
    }

    /// Track this test process as a stopped PTY last named `riko`, so
    /// `start_from_orphan` sees a live orphan.
    fn track_riko_pty(agentx_dir: &std::path::Path) -> u32 {
        let pid = std::process::id();
        let tmp = agentx_dir.join(".tmp");
        std::fs::create_dir_all(&tmp).unwrap();
        let entry = json!({
            "tool": "claude",
            "names": ["riko"],
            "launched_at": crate::shared::time::now_epoch_f64(),
            "directory": "/tmp/project",
            "process_id": "proc-riko",
            "session_id": "sess-riko",
        });
        std::fs::write(tmp.join("launched_pids.json"), serde_json::to_string(&json!({ pid.to_string(): entry })).unwrap()).unwrap();
        pid
    }

    fn insert_live_row(db: &AgentxDb, name: &str, session_id: Option<&str>) {
        db.conn()
            .execute(
                "INSERT INTO instances (name, session_id, tool, status, created_at)
                 VALUES (?1, ?2, 'claude', 'active', ?3)",
                params![name, session_id, crate::shared::time::now_epoch_f64()],
            )
            .unwrap();
    }

    fn instance_names(db: &AgentxDb) -> Vec<String> {
        let mut names: Vec<String> = db.iter_instances_full().unwrap().into_iter().map(|row| row.name).collect();
        names.sort();
        names
    }

    fn still_tracked(agentx_dir: &std::path::Path, pid: u32) -> bool {
        pidtrack::get_orphan_processes(agentx_dir, None).iter().any(|o| o.pid == pid)
    }

    #[test]
    #[serial]
    fn orphan_recovery_reuses_a_free_name() {
        let (_dir, agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        let pid = track_riko_pty(&agentx_dir);

        let ctx = make_ctx(&[], "/tmp/project");
        assert_eq!(start_from_orphan(&db, &agentx_dir, &pid.to_string(), &ctx).unwrap(), 0);
        assert_eq!(instance_names(&db), vec!["riko"]);
        assert_eq!(db.get_instance_full("riko").unwrap().unwrap().pid, Some(pid as i64));
        assert!(!still_tracked(&agentx_dir, pid));
    }

    #[test]
    #[serial]
    fn orphan_rejoined_under_another_name_is_adopted_not_recovered() {
        // Stopped as riko, then `agentx start` in its own shell minted melo. The
        // PTY is live again; recovering it as riko would split it in two.
        let (_dir, agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        insert_live_row(&db, "melo", Some("sess-riko"));
        db.conn().execute("UPDATE instances SET launch_context = '{\"tty\":\"/dev/ttys009\"}' WHERE name = 'melo'", []).unwrap();
        db.set_process_binding("proc-riko", "sess-riko", "melo").unwrap();
        let pid = track_riko_pty(&agentx_dir);

        let ctx = make_ctx(&[], "/tmp/project");
        assert_eq!(start_from_orphan(&db, &agentx_dir, &pid.to_string(), &ctx).unwrap(), 0);
        assert_eq!(instance_names(&db), vec!["melo"]);
        assert_eq!(db.get_instance_full("melo").unwrap().unwrap().pid, Some(pid as i64), "the owning row gets the pid back, so `agentx kill melo` reaches the process");
        let launch_context: serde_json::Value = serde_json::from_str(&db.get_instance_full("melo").unwrap().unwrap().launch_context.unwrap()).unwrap();
        assert_eq!(launch_context["tty"], "/dev/ttys009", "hook context is kept");
        assert_eq!(launch_context["process_id"], "proc-riko");
        assert!(!still_tracked(&agentx_dir, pid));
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn orphan_recovery_by_name_prefers_a_real_orphan_over_an_adopted_one() {
        // Two PTYs once named riko: one rejoined as melo, the other is orphaned.
        let (_dir, agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        insert_live_row(&db, "melo", Some("sess-riko"));
        db.set_process_binding("proc-riko", "sess-riko", "melo").unwrap();
        let adopted_pid = track_riko_pty(&agentx_dir);
        let orphan_pid = std::os::unix::process::parent_id();
        let pidfile = agentx_dir.join(".tmp").join("launched_pids.json");
        let mut entries: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&pidfile).unwrap()).unwrap();
        entries[orphan_pid.to_string()] = json!({
            "tool": "claude",
            "names": ["riko"],
            "launched_at": crate::shared::time::now_epoch_f64(),
            "process_id": "proc-other",
            "session_id": "sess-other",
        });
        std::fs::write(&pidfile, entries.to_string()).unwrap();

        let ctx = make_ctx(&[], "/tmp/project");
        assert_eq!(start_from_orphan(&db, &agentx_dir, "riko", &ctx).unwrap(), 0);
        assert_eq!(db.get_instance_full("melo").unwrap().unwrap().pid, Some(adopted_pid as i64));
        assert_eq!(db.get_instance_full("riko").unwrap().unwrap().pid, Some(orphan_pid as i64), "the real orphan is recovered, not skipped");
    }

    #[test]
    #[serial]
    fn orphan_rejoined_by_session_is_adopted() {
        // The row carries the PTY's session but lost its process binding.
        let (_dir, agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        insert_live_row(&db, "riko", Some("sess-riko"));
        let pid = track_riko_pty(&agentx_dir);

        let ctx = make_ctx(&[], "/tmp/project");
        assert_eq!(start_from_orphan(&db, &agentx_dir, "riko", &ctx).unwrap(), 0);
        assert_eq!(instance_names(&db), vec!["riko"]);
        assert_eq!(db.get_instance_full("riko").unwrap().unwrap().pid, Some(pid as i64));
        assert_eq!(db.get_process_binding("proc-riko").unwrap().as_deref(), Some("riko"), "adoption by session repairs the missing process binding");
        assert!(!still_tracked(&agentx_dir, pid));
    }

    #[test]
    #[serial]
    fn orphan_recovery_mints_when_name_belongs_to_another_pty() {
        let (_dir, agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open().unwrap();
        // Same session and name, but bound to a different PTY: not ours.
        insert_live_row(&db, "riko", Some("sess-riko"));
        db.set_process_binding("proc-other", "sess-riko", "riko").unwrap();
        let pid = track_riko_pty(&agentx_dir);

        let ctx = make_ctx(&[], "/tmp/project");
        assert_eq!(start_from_orphan(&db, &agentx_dir, &pid.to_string(), &ctx).unwrap(), 0);
        let riko = db.get_instance_full("riko").unwrap().unwrap();
        assert_eq!(riko.pid, None, "another PTY's row must not be taken over");
        assert_eq!(db.get_process_binding("proc-other").unwrap().as_deref(), Some("riko"));
        let recovered = db.get_process_binding("proc-riko").unwrap().unwrap();
        assert_ne!(recovered, "riko");
        assert_eq!(instance_names(&db).len(), 2);
    }
}

#[cfg(test)]
#[path = "lifecycle_operator_tests.rs"]
mod operator_tests;
