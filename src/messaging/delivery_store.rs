//! Daemon-owned PTY policy. Validate runtime process authority before touching
//! participant state; output/wake effects follow the outer commit.
use super::delivery_service::*;
use crate::agents::service::resolve_actor;
use crate::db::AgentxDb;
use crate::service::{RequestContext, ServiceError, ServiceResult};
use serde_json::json;

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

pub(crate) fn snapshot(db: &AgentxDb, context: &RequestContext) -> ServiceResult<DeliverySnapshot> {
    let actor = resolve_actor(db, context)?;
    let name = &actor.record.name;
    let mut statement = db.conn().prepare_cached("SELECT kind,port FROM notify_endpoints WHERE instance=? ORDER BY kind").map_err(internal)?;
    let endpoints = statement.query_map([name], |row| Ok((row.get(0)?, row.get(1)?))).map_err(internal)?.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)?;
    let pending = db.unread_messages_after(name, actor.participant.incarnation_bits, actor.record.last_event_id, crate::shared::MAX_MESSAGES_PER_DELIVERY).map_err(internal)?;
    let has_session = actor.record.session_id.as_deref().is_some_and(|session| !session.is_empty());
    let recovery = crate::agents::recovery_store::pending(db, &actor.record)?;
    Ok(DeliverySnapshot { actor, pending, endpoints, has_session, recovery })
}

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    if let Request::FinalizeLaunchExit { registration, never_ready, elapsed_secs, exit_code, output } = &request {
        return finalize_launch_exit(db, context, registration, *never_ready, *elapsed_secs, *exit_code, output.as_deref());
    }
    if matches!(request, Request::Poll) {
        return transaction(db, || {
            let snapshot = snapshot(db, context)?;
            crate::runtime::hosts_store::observe_delivery_poll(db, context, &snapshot.actor.participant)?;
            Ok(Response::Snapshot(snapshot))
        });
    }
    let registration = match &request {
        Request::ObserveLaunch { registration, .. }
        | Request::Recovery { registration, .. }
        | Request::ObserveGate { registration, .. }
        | Request::Heartbeat { registration }
        | Request::Finish { registration, .. }
        | Request::BindSession { registration, .. }
        | Request::Activity { registration, .. } => registration,
        Request::Poll | Request::FinalizeLaunchExit { .. } => unreachable!(),
    }
    .clone();
    // Exit cleanup may follow a soft stop that cleared the process binding.
    // Retained host/child authority still cannot cross a participant incarnation.
    if let Request::Finish { registration, killed, exit_code, output } = request {
        return transaction(db, || {
            crate::runtime::hosts_store::validate_process_exit_registration(db, context, &registration)?;
            let name = &registration.participant.canonical_name;
            let Some(row) = db.get_instance_full(name).map_err(internal)? else {
                return Ok(Response::Changed(false));
            };
            let recorded_kill = row.status_context == "exit:killed";
            if recorded_kill {
                let outcome = crate::agents::service::stop_killed(db, context, &registration.participant)?;
                return Ok(Response::Changed(outcome.stopped));
            }
            let killed = killed || recorded_kill;
            if !killed && row.status == "inactive" && matches!(row.tool.as_str(), "antigravity" | "omp") {
                db.delete_notify_endpoints(name).map_err(internal)?;
                if let Some(binding) = &context.caller.invocation.process_binding_id {
                    db.conn().execute("DELETE FROM process_bindings WHERE process_id=? AND instance_name=?", rusqlite::params![binding, name]).map_err(internal)?;
                }
                return Ok(Response::Changed(false));
            }
            let mut data = json!({"action":"stopped","by":if recorded_kill && !row.status_detail.is_empty() { row.status_detail.as_str() } else { "pty" },"reason":if killed { "killed" } else { "closed" },"snapshot":db.get_instance_snapshot(name).map_err(internal)?});
            if !killed {
                if let Some(code) = exit_code {
                    data["exit_code"] = json!(code);
                }
                if let Some(output) = output {
                    data["output"] = json!(output);
                }
            }
            let won = db.finalize_instance_stop(name, row.created_at, row.session_id.as_deref(), row.agent_id.as_deref(), &data).map_err(internal)?;
            Ok(Response::Changed(won))
        });
    }
    let mut effects = Vec::new();
    let changed = transaction(db, || {
        let actor = crate::runtime::hosts_store::validate_process_registration(db, context, &registration)?;
        let name = &registration.participant.canonical_name;
        match request {
            Request::Recovery { ticket, action, .. } => crate::agents::recovery_store::host(db, &actor.record, ticket, action),
            Request::Heartbeat { .. } => {
                if heartbeat_due(&actor.record, crate::shared::time::now_epoch_i64()) {
                    db.update_heartbeat(name).map_err(internal)?;
                }
                crate::runtime::hosts_store::checkpoint_delivery_poll(db, &registration)?;
                Ok(true)
            }
            Request::BindSession { session, cwd, .. } => {
                if session.is_empty() {
                    return Err(ServiceError::InvalidInput("session required".into()));
                }
                if let Some(owner) = db.get_session_binding(&session).map_err(internal)?
                    && owner != *name
                {
                    return Err(ServiceError::Conflict("session belongs to another participant".into()));
                }
                db.rebind_instance_session(name, &session).map_err(internal)?;
                if let Some(binding) = &context.caller.invocation.process_binding_id {
                    db.set_process_binding(binding, &session, name).map_err(internal)?;
                }
                db.update_instance_fields(name, json!({"directory":cwd.to_string_lossy()}).as_object().unwrap()).map_err(internal)?;
                Ok(true)
            }
            Request::Activity { status, context: activity_context, detail, .. } => {
                if !matches!(status.as_str(), "active" | "listening" | "blocked" | "inactive") {
                    return Err(ServiceError::InvalidInput("invalid activity status".into()));
                }
                db.update_instance_fields(name, json!({"status":status,"status_context":activity_context,"status_detail":detail,"status_time":crate::shared::time::now_epoch_i64()}).as_object().unwrap()).map_err(internal)?;
                let data = json!({"status":status,"context":activity_context,"detail":detail,"position":actor.record.last_event_id});
                let id = db.insert_event_row("status", name, &data, None).map_err(internal)?;
                effects.push((id, "status", data));
                Ok(true)
            }
            Request::ObserveLaunch { observation, .. } => {
                let phase_key = format!("delivery_launch:{}:{}", name, registration.participant.incarnation_bits);
                let phase = db.kv_get(&phase_key).map_err(internal)?;
                if matches!(phase.as_deref(), Some("ready" | "failed")) {
                    return Ok(false);
                }
                let (next, status, status_context, event_action, reason, detail) = match observation {
                    LaunchObservation::Ready { cleared_block } => ("ready", "listening", if cleared_block { "launch_blocked_cleared" } else { "ready_observed" }, "ready", "", "".to_owned()),
                    LaunchObservation::Blocked { detail } if phase.is_none() => ("blocked", "blocked", "launch_blocked", "launch_blocked", "screen_settled_not_ready", detail),
                    LaunchObservation::Failed { reason, detail } if phase.as_deref() != Some("blocked") => {
                        let mut data = json!({"action":"launch_failed","status":"inactive","context":"launch_failed","reason":reason,"detail":detail});
                        launch_origin(context, &mut data);
                        let id = db.insert_event_row("life", name, &data, None).map_err(internal)?;
                        effects.push((id, "life", data));
                        db.kv_set(&phase_key, Some("failed")).map_err(internal)?;
                        return Ok(true);
                    }
                    _ => return Ok(false),
                };
                db.set_status(name, status, status_context).map_err(internal)?;
                let mut data = json!({"action":event_action,"status":status,"context":status_context,"reason":reason,"detail":detail});
                launch_origin(context, &mut data);
                let id = db.insert_event_row("life", name, &data, None).map_err(internal)?;
                effects.push((id, "life", data));
                db.kv_set(&phase_key, Some(next)).map_err(internal)?;
                Ok(true)
            }
            Request::ObserveGate { observation, .. } => match observation {
                GateObservation::Blocked { cursor, context, detail } => {
                    if actor.record.status != crate::shared::ST_LISTENING || cursor != actor.record.last_event_id || db.unread_messages_after(name, actor.participant.incarnation_bits, cursor, 1).map_err(internal)?.is_empty() {
                        return Ok(false);
                    }
                    db.set_gate_status(name, &context, &detail).map_err(internal)
                }
                GateObservation::Cleared { cursor, context } => {
                    if cursor != actor.record.last_event_id || actor.record.status_context != context {
                        return Ok(false);
                    }
                    db.clear_gate_status(name, &context).map_err(internal)?;
                    Ok(true)
                }
                GateObservation::RecoveredIdle { previous_context } => {
                    if actor.record.tool == "codex" || actor.record.status != "active" || actor.record.status_context != previous_context {
                        return Ok(false);
                    }
                    db.set_status(name, "listening", "pty:recovered").map_err(internal)?;
                    Ok(true)
                }
                GateObservation::CodexTurnCompleted { session, turn, phase, progress_at, status_time, cursor, completed_at_ms } => {
                    use crate::agents::model_request::{ERROR_CONTEXT, RequestPhase};
                    let row = &actor.record;
                    if row.tool != "codex"
                        || row.origin_device_id.is_some()
                        || row.parent_name.is_some()
                        || row.status != "active"
                        || row.status_context != ERROR_CONTEXT
                        || row.status_time != status_time
                        || row.session_id.as_deref() != Some(&session)
                        || row.last_event_id != cursor
                        || !matches!(phase, RequestPhase::Started | RequestPhase::Completed)
                        || row.model_request.as_ref().is_none_or(|model| model.error.is_some() || model.session_id != session || model.turn_id != turn || model.phase != phase || model.last_progress_at != Some(progress_at))
                        || completed_at_ms < progress_at.max(status_time).saturating_mul(1000)
                        || completed_at_ms > chrono::Utc::now().timestamp_millis()
                        || db.unread_messages_after(name, actor.participant.incarnation_bits, cursor, 1).map_err(internal)?.is_empty()
                    {
                        return Ok(false);
                    }
                    const CONTEXT: &str = "pty:codex-turn-complete";
                    db.set_status(name, "listening", CONTEXT).map_err(internal)?;
                    db.conn().execute("UPDATE instances SET status_detail='' WHERE name=?", [name]).map_err(internal)?;
                    let data = json!({"status":"listening","context":CONTEXT,"position":cursor,"session":session});
                    let id = db.insert_event_row("status", name, &data, None).map_err(internal)?;
                    effects.push((id, "status", data));
                    db.wake_after_commit(name, crate::runtime::notify::WakeKind::DELIVERY_LOOPS);
                    Ok(true)
                }
            },
            _ => unreachable!(),
        }
    })?;
    for (id, kind, data) in effects {
        db.after_event_logged(id, kind, &registration.participant.canonical_name, &data);
        if data["action"] == "ready"
            && let (Some(launcher), Some(batch)) = (data["by"].as_str().filter(|value| *value != "unknown"), data["batch_id"].as_str())
            && let Err(error) = db.check_batch_completion(launcher, batch)
        {
            crate::log::log_error("delivery", "batch_completion", &error.to_string());
        }
    }
    Ok(Response::Changed(changed))
}

fn launch_origin(context: &RequestContext, data: &mut serde_json::Value) {
    let environment = &context.caller.invocation.environment;
    data["by"] = json!(environment.get("AGENTX_LAUNCHED_BY").map(String::as_str).unwrap_or("unknown"));
    if let Some(batch) = environment.get("AGENTX_LAUNCH_BATCH_ID") {
        data["batch_id"] = json!(batch);
    }
}

/// Exit evidence remains tied to a retained host/child lease even after a
/// SessionEnd removed the row. A successor row always rejects this observation.
fn finalize_launch_exit(db: &AgentxDb, context: &RequestContext, registration: &crate::runtime::hosts::ProcessRegistration, never_ready: bool, elapsed_secs: u64, exit_code: i32, output: Option<&str>) -> ServiceResult<Response> {
    transaction(db, || {
        crate::runtime::hosts_store::validate_process_exit_registration(db, context, registration)?;
        let name = &registration.participant.canonical_name;
        let key = format!("delivery_launch_exit:{}:{}", name, registration.participant.incarnation_bits);
        if db.kv_get(&key).map_err(internal)?.is_some() {
            return Ok(Response::Changed(false));
        }
        let row = db.get_instance_full(name).map_err(internal)?;
        let environment = &context.caller.invocation.environment;
        let launch_cursor = ["AGENTX_LAUNCH_START_EVENT_ID", "AGENTX_LAUNCH_EVENT_ID"].iter().find_map(|key| environment.get(*key)?.parse::<i64>().ok());
        let recorded_kill: bool = match launch_cursor {
            Some(cursor) => db
                .conn()
                .query_row("SELECT EXISTS(SELECT 1 FROM events WHERE type='life' AND instance=?1 AND id>?2 AND json_extract(data,'$.action')='stopped' AND json_extract(data,'$.reason')='killed')", rusqlite::params![name, cursor], |row| row.get(0))
                .map_err(internal)?,
            None => false,
        };
        if recorded_kill || row.as_ref().is_some_and(|row| row.status_context == "exit:killed") {
            db.kv_set(&key, Some("killed")).map_err(internal)?;
            return Ok(Response::Changed(false));
        }
        let unbound = row.as_ref().is_some_and(|row| row.session_id.is_none() && row.status_context == "new" && matches!(row.status.as_str(), "inactive" | "pending"));
        let launched = environment.get("AGENTX_LAUNCHED").is_some_and(|value| value == "1");
        let never_ready = never_ready && launched;
        let crashed = launched && !never_ready && elapsed_secs < 60 && !matches!(exit_code, 0 | 129 | 130 | 143);
        if !unbound && !never_ready && !crashed {
            return Ok(Response::Changed(false));
        }
        let (reason, mut detail) = if unbound {
            ("exited_before_bind", format!("exited {elapsed_secs}s after spawn before binding (exit code {exit_code})"))
        } else if never_ready {
            ("exited_before_ready", format!("exited {elapsed_secs}s after spawn before ready (exit code {exit_code})"))
        } else {
            ("exited_after_ready", format!("crashed {elapsed_secs}s after spawn, after ready (exit code {exit_code})"))
        };
        if let Some(output) = output {
            detail.push_str("\nPTY output:\n");
            detail.push_str(output);
        }
        if unbound {
            // The host supplied its output tail. Never run tmux or read a
            // terminal/log file from inside the daemon's write transaction.
            db.update_instance_fields(name, json!({"status":"inactive","status_context":"launch_failed","status_detail":detail,"status_time":crate::shared::time::now_epoch_i64()}).as_object().unwrap()).map_err(internal)?;
            db.log_event("status", name, &json!({"status":"inactive","context":"launch_failed","detail":detail,"position":row.as_ref().unwrap().last_event_id})).map_err(internal)?;
        }
        let mut data = json!({"action":"launch_failed","status":"inactive","context":"launch_failed","reason":reason,"detail":detail,"participant":registration.participant});
        launch_origin(context, &mut data);
        db.log_event("life", name, &data).map_err(internal)?;
        if !unbound && let Some(row) = &row {
            let stopped = json!({"action":"stopped","by":"pty","reason":"closed","exit_code":exit_code,"snapshot":db.get_instance_snapshot(name).map_err(internal)?,"output":output});
            if !db.finalize_instance_stop(name, row.created_at, row.session_id.as_deref(), row.agent_id.as_deref(), &stopped).map_err(internal)? {
                return Err(ServiceError::StaleFence);
            }
        }
        if let Some(binding) = &context.caller.invocation.process_binding_id {
            db.conn().execute("DELETE FROM process_bindings WHERE process_id=? AND instance_name=?", rusqlite::params![binding, name]).map_err(internal)?;
        }
        db.kv_set(&key, Some("failed")).map_err(internal)?;
        Ok(Response::Changed(true))
    })
}
