//! Fenced, transactional request-state projection. Never performs provider I/O.
use super::model_request::{ERROR_CONTEXT, ModelRequestState, ObserveModelRequest, RequestError, RequestPhase};
use super::service::Response;
use crate::db::AgentxDb;
use crate::service::{RequestContext, ServiceError, ServiceResult};
use crate::shared::{ST_ACTIVE, ST_BLOCKED, ST_ERROR, ST_INACTIVE};
use serde_json::json;

pub(crate) fn observe(db: &AgentxDb, context: &RequestContext, input: ObserveModelRequest) -> ServiceResult<Response> {
    store(db, context, input, None)
}

pub(super) fn observe_verified(db: &AgentxDb, context: &RequestContext, input: ObserveModelRequest, row: &crate::db::InstanceRow) -> ServiceResult<Response> {
    store(db, context, input, Some(row))
}

fn store(db: &AgentxDb, context: &RequestContext, input: ObserveModelRequest, verified: Option<&crate::db::InstanceRow>) -> ServiceResult<Response> {
    if input.session_id.is_empty() || input.session_id.len() > 512 || input.turn_id.is_empty() || input.turn_id.len() > 512 || input.model.len() > 1024 || input.error.as_ref().is_some_and(|error| error.len() > 16_384) {
        return Err(ServiceError::InvalidInput("model request requires session/turn and at most 16 KiB error".into()));
    }
    db.with_write_scope(|| {
        let observed;
        let row = match verified {
            Some(row) => row,
            None => {
                observed = super::hook_store::observation_actor(db, context, &input.participant, Some(&input.session_id)).map_err(anyhow::Error::new)?;
                &observed
            }
        };
        super::service_store::retained(db, context, &json!({"model_request":input}), || {
            if row.status == ST_INACTIVE {
                return Ok(Response::Changed);
            }
            let now = crate::shared::time::now_epoch_i64();
            let mut state = row.model_request.as_deref().cloned().filter(|state| state.session_id == input.session_id).unwrap_or_else(|| ModelRequestState {
                session_id: input.session_id.clone(),
                turn_id: input.turn_id.clone(),
                model: input.model.clone(),
                phase: input.phase,
                last_progress_at: None,
                error: None,
                retired_turns: Vec::new(),
            });
            if state.retired_turns.contains(&input.turn_id) || (state.turn_id != input.turn_id && input.phase != RequestPhase::Started) {
                return Ok(Response::Changed);
            }
            if state.turn_id != input.turn_id {
                state.retired_turns.push(std::mem::replace(&mut state.turn_id, input.turn_id.clone()));
                if state.retired_turns.len() > 32 {
                    state.retired_turns.remove(0);
                }
            }
            state.model = input.model.clone();
            state.phase = input.phase;
            match input.phase {
                RequestPhase::Started => state.last_progress_at = Some(now),
                RequestPhase::Completed => {
                    state.last_progress_at = Some(now);
                    state.error = None;
                }
                RequestPhase::Failed => {
                    let previous = state.error.as_ref();
                    state.error = Some(RequestError {
                        turn_id: input.turn_id.clone(),
                        source: "model_request".into(),
                        reason: input.error.clone().filter(|error| !error.is_empty()).unwrap_or_else(|| "Model request failed (no error detail reported)".into()),
                        error_since: previous.map_or(now, |error| error.error_since),
                        last_error_at: now,
                        consecutive_failures: previous.map_or(1, |error| error.consecutive_failures.saturating_add(1)),
                    });
                }
                RequestPhase::Cancelled => {}
            }
            db.update_instance_fields(&row.name, json!({"model_request":serde_json::to_string(&state).map_err(internal)?}).as_object().unwrap()).map_err(internal)?;
            if context.caller.invocation.process_binding_id.is_some() {
                super::recovery_store::observe_request(db, row, &state)?;
            }
            let (status, status_context, detail) = if row.status == ST_BLOCKED || (row.status == ST_ERROR && row.status_context != ERROR_CONTEXT) {
                (row.status.as_str(), row.status_context.as_str(), row.status_detail.as_str())
            } else if let Some(error) = &state.error {
                (ST_ERROR, ERROR_CONTEXT, error.reason.as_str())
            } else if input.phase == RequestPhase::Cancelled || (input.phase == RequestPhase::Completed && row.status == crate::shared::ST_LISTENING) {
                (row.status.as_str(), row.status_context.as_str(), row.status_detail.as_str())
            } else {
                (ST_ACTIVE, "model:request", "")
            };
            super::lifecycle::try_set_status(db, &row.name, status, status_context, super::lifecycle::StatusUpdate { detail, ..Default::default() }).map_err(internal)?;
            Ok(Response::Changed)
        })
        .map_err(anyhow::Error::new)
    })
    .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

#[cfg(test)]
mod tests;
