//! Daemon-owned storage for instruction snapshots and initial-context receipts.
//! Content generation and file reads stay in the caller. All state is scoped to
//! authenticated participant incarnations; native-session aliases permit resume.
use super::context_service::{Request, Response};
use crate::agents::attachments::ParticipantRef;
use crate::db::AgentxDb;
use crate::runtime::agent_context::{FrozenInstructions, MAX_CONTEXT_BYTES};
use crate::service::{RequestContext, ServiceError, ServiceResult};
use serde::{Deserialize, Serialize};
use sha2::Digest;

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn digest(value: &impl Serialize) -> ServiceResult<String> {
    Ok(sha2::Sha256::digest(serde_json::to_vec(value).map_err(internal)?).iter().map(|byte| format!("{byte:02x}")).collect())
}
fn key(context: &RequestContext, kind: &str, value: &impl Serialize) -> ServiceResult<String> {
    Ok(format!("agent_context:{kind}:{}:{}", context.database_generation, digest(value)?))
}
#[derive(Serialize, Deserialize)]
struct SavedInstructions {
    name: String,
    instructions: FrozenInstructions,
}
/// View only the snapshot retained for this exact incarnation. Unlike the
/// Provider's context operation, this never resolves or persists session aliases.
pub(crate) fn read_frozen(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef) -> ServiceResult<Option<FrozenInstructions>> {
    db.kv_get(&key(context, "instructions", participant)?).map_err(internal)?.map(|raw| serde_json::from_str::<SavedInstructions>(&raw).map(|saved| saved.instructions).map_err(internal)).transpose()
}

#[derive(Serialize, Deserialize)]
struct InitialReceipt {
    participant: ParticipantRef,
    session_id: String,
    pid: u32,
    incarnation: Option<String>,
}

pub(super) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    db.with_write_scope(|| handle_inner(db, context, request).map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}
fn handle_inner(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    let actor = super::service::resolve_actor(db, context)?;
    let expected = match &request {
        Request::ReadInstructions { participant } | Request::FreezeInstructions { participant, .. } | Request::PrepareInitial { participant, .. } | Request::ConfirmInitial { participant, .. } | Request::ReleaseInitial { participant, .. } => {
            participant
        }
    };
    if actor.participant != *expected {
        return Err(ServiceError::StaleFence);
    }
    let participant = &actor.participant;
    let instruction_key = key(context, "instructions", participant)?;
    let session = actor.record.session_id.as_deref().filter(|s| !s.is_empty());
    let session_key = session.map(|session| key(context, "session", &(&actor.record.tool, session))).transpose()?;
    match request {
        Request::ReadInstructions { .. } | Request::FreezeInstructions { .. } => {
            let mut saved = db.kv_get(&instruction_key).map_err(internal)?;
            if saved.is_none()
                && let Some(session_key) = &session_key
            {
                saved = db.kv_get(session_key).map_err(internal)?.filter(|value| serde_json::from_str::<SavedInstructions>(value).is_ok_and(|saved| saved.name == participant.canonical_name));
            }
            if saved.is_none()
                && let Request::FreezeInstructions { instructions, .. } = request
            {
                if instructions.text.len() > MAX_CONTEXT_BYTES {
                    return Err(ServiceError::InvalidInput("instructions exceed context limit".into()));
                }
                saved = Some(serde_json::to_string(&SavedInstructions { name: participant.canonical_name.clone(), instructions }).map_err(internal)?);
            }
            if let Some(saved) = saved {
                let parsed: SavedInstructions = serde_json::from_str(&saved).map_err(internal)?;
                db.kv_set(&instruction_key, Some(&saved)).map_err(internal)?;
                if let Some(session_key) = session_key {
                    db.kv_set(&session_key, Some(&saved)).map_err(internal)?;
                }
                Ok(Response::Instructions(Some(parsed.instructions)))
            } else {
                Ok(Response::Instructions(None))
            }
        }
        Request::PrepareInitial { session_id, .. } => {
            if session != Some(session_id.as_str()) {
                return Err(ServiceError::StaleFence);
            }
            let delivered_key = key(context, "initial", &(participant, &session_id))?;
            if db.kv_get(&delivered_key).map_err(internal)?.is_some() {
                return Ok(Response::InitialPrepared { receipt: None });
            }
            let pending_key = key(context, "initial_pending", &(participant, &session_id))?;
            if let Some(raw) = db.kv_get(&pending_key).map_err(internal)? {
                let pending: InitialReceipt = serde_json::from_str(&raw).map_err(internal)?;
                let alive = pending
                    .incarnation
                    .as_deref()
                    .map(|identity| crate::sys::process::identity(pending.pid).map(|current| current == identity).unwrap_or_else(|| crate::sys::process::is_alive(pending.pid)))
                    .unwrap_or_else(|| crate::sys::process::is_alive(pending.pid));
                if alive {
                    return Ok(Response::InitialPrepared { receipt: None });
                }
            }
            let receipt = format!("context:{}", uuid::Uuid::new_v4());
            let saved = InitialReceipt { participant: participant.clone(), session_id, pid: context.process.pid, incarnation: context.process.incarnation.clone() };
            let raw = serde_json::to_string(&saved).map_err(internal)?;
            db.kv_set(&key(context, "receipt", &receipt)?, Some(&raw)).map_err(internal)?;
            db.kv_set(&pending_key, Some(&raw)).map_err(internal)?;
            db.kv_set(&key(context, "pending_receipt", &(participant, &saved.session_id))?, Some(&receipt)).map_err(internal)?;
            Ok(Response::InitialPrepared { receipt: Some(receipt) })
        }
        operation @ (Request::ConfirmInitial { .. } | Request::ReleaseInitial { .. }) => {
            let (receipt, release) = match operation {
                Request::ConfirmInitial { receipt, .. } => (receipt, false),
                Request::ReleaseInitial { receipt, .. } => (receipt, true),
                _ => unreachable!(),
            };
            let raw = db.kv_get(&key(context, "receipt", &receipt)?).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
            let saved: InitialReceipt = serde_json::from_str(&raw).map_err(internal)?;
            if saved.participant != *participant || session != Some(saved.session_id.as_str()) || saved.pid != context.process.pid || saved.incarnation != context.process.incarnation {
                return Err(ServiceError::StaleFence);
            }
            let pending_receipt = key(context, "pending_receipt", &(participant, &saved.session_id))?;
            if db.kv_get(&pending_receipt).map_err(internal)?.as_deref() != Some(receipt.as_str()) {
                return Err(ServiceError::StaleFence);
            }
            if !release {
                db.kv_set(&key(context, "initial", &(participant, &saved.session_id))?, Some("delivered")).map_err(internal)?;
            }
            db.kv_set(&key(context, "initial_pending", &(participant, &saved.session_id))?, None).map_err(internal)?;
            if release {
                db.kv_set(&pending_receipt, None).map_err(internal)?;
            }
            Ok(Response::Confirmed)
        }
    }
}
