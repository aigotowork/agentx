//! Authenticated control admission and exact worker completion. This daemon
//! adapter never connects to a Client or performs Provider/file/network effects.
use super::control_parse::{self, Action};
use super::control_service::*;
use crate::db::AgentxDb;
use crate::service::{RequestContext, ServiceError, ServiceResult};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) const MAX_ACTIVE: usize = 32;
pub(super) const MAX_CONTROLS: usize = 32;
const MAX_INPUT_BYTES: usize = 64 * 1024;
const MAX_RESULT_BYTES: usize = 2 * 1024 * 1024;
const RESULT_RETENTION_SECS: f64 = 300.0;
const MAX_RETAINED_RESULTS: usize = 64;
const FRESHNESS_SECS: f64 = 300.0;
const RECORD: &str = "relay_control_receipt:";
const ACTIVE: &str = "relay_control_active:";
const RESULT: &str = "relay_control_result:";

#[derive(Serialize, Deserialize)]
#[serde(tag = "stage")]
enum Stage {
    Accepted { effect: ControlEffect },
    Issuing { effect: ControlEffect },
    Completed,
}
#[derive(Serialize, Deserialize)]
struct Record {
    generation: String,
    relay_id: String,
    source: String,
    request_id: Option<String>,
    action: String,
    intent_digest: String,
    owner_pid: u32,
    owner_incarnation: Option<String>,
    token: String,
    completion_digest: Option<String>,
    stage: Stage,
}
#[derive(Serialize, Deserialize)]
struct RetainedResult {
    completed_at: f64,
    reply: ControlReply,
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}
fn canonical(value: Value) -> Value {
    match value {
        Value::Object(values) => {
            let mut entries = values.into_iter().collect::<Vec<_>>();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(entries.into_iter().map(|(key, value)| (key, canonical(value))).collect())
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonical).collect()),
        value => value,
    }
}
fn reply_digest(reply: &ControlReply) -> ServiceResult<String> {
    Ok(digest(&serde_json::to_vec(&canonical(serde_json::to_value(reply).map_err(internal)?)).map_err(internal)?))
}
fn load(db: &AgentxDb, id: &str) -> ServiceResult<Record> {
    let value = db.kv_get(&format!("{RECORD}{id}")).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    serde_json::from_str(&value).map_err(internal)
}
fn save(db: &AgentxDb, id: &str, record: &Record) -> anyhow::Result<()> {
    db.kv_set(&format!("{RECORD}{id}"), Some(&serde_json::to_string(record)?))
}
fn check_owner(record: &Record, context: &RequestContext, token: &str) -> ServiceResult<()> {
    let Some(incarnation) = context.process.incarnation.as_deref().filter(|value| !value.is_empty()) else {
        return Err(ServiceError::StaleFence);
    };
    if record.generation != context.database_generation || record.token != token || record.owner_pid != context.process.pid || record.owner_incarnation.as_deref() != Some(incarnation) {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}

/// Retire proven-dead hosts to an unknown completed receipt. This frees live
/// admission capacity without transferring or replaying their execution grant.
pub(super) fn reconcile_abandoned(db: &AgentxDb) -> ServiceResult<bool> {
    reconcile_abandoned_with(db, |pid| (crate::sys::process::has_exited(pid), crate::sys::process::identity(pid)))
}

/// Observe before opening a write scope. An unavailable observation is not
/// proof of exit, and legacy unknown-incarnation owners cannot prove PID reuse.
fn reconcile_abandoned_with(db: &AgentxDb, mut observe: impl FnMut(u32) -> (bool, Option<String>)) -> ServiceResult<bool> {
    let mut changed = false;
    for (key, _) in db.kv_prefix(ACTIVE).map_err(internal)? {
        let Some(id) = key.strip_prefix(ACTIVE) else {
            continue;
        };
        let mut record = load(db, id)?;
        let (exited, incarnation) = observe(record.owner_pid);
        let reused = record.owner_incarnation.as_deref().filter(|value| !value.is_empty()).zip(incarnation.as_deref().filter(|value| !value.is_empty())).is_some_and(|(prior, current)| prior != current);
        if !exited && !reused {
            continue;
        }
        let reply = ControlReply { ok: false, result: json!({"error": "remote action owner exited; its outcome is unknown and it will not be reissued", "outcome": "unknown_remote_effect"}) };
        record.stage = Stage::Completed;
        record.completion_digest = Some(reply_digest(&reply)?);
        let event = db
            .with_write_scope(|| {
                save(db, id, &record)?;
                retain_result(db, id, &reply)?;
                db.kv_set(&key, None)?;
                emit(db, result_data(&record, &reply))
            })
            .map_err(internal)?;
        changed |= event.is_some();
        wake_result(db, event);
    }
    Ok(changed)
}

/// Persists the unique grant before replying. A lost grant reply never permits
/// execution on a retry; Issuing has no timeout/takeover transition.
pub(super) fn begin(db: &AgentxDb, context: &RequestContext, id: &str, token: &str) -> ServiceResult<bool> {
    let mut record = load(db, id)?;
    check_owner(&record, context, token)?;
    let Stage::Accepted { effect } = record.stage else {
        return Ok(false);
    };
    record.stage = Stage::Issuing { effect };
    db.with_write_scope(|| save(db, id, &record)).map_err(internal)?;
    Ok(true)
}

fn result_data(record: &Record, reply: &ControlReply) -> Option<Value> {
    record.request_id.as_ref().map(|request_id| json!({"request_id": request_id, "action": record.action, "ok": reply.ok, "result": reply.result}))
}
fn emit(db: &AgentxDb, data: Option<Value>) -> anyhow::Result<Option<(i64, Value)>> {
    data.map(|data| db.insert_event_row("rpc_result", "_rpc", &data, None).map(|id| (id, data))).transpose()
}
fn wake_result(db: &AgentxDb, event: Option<(i64, Value)>) {
    if let Some((id, data)) = event {
        db.after_event_logged(id, "rpc_result", "_rpc", &data);
    }
}
fn retained(db: &AgentxDb, id: &str) -> ServiceResult<ControlReply> {
    let result = db.kv_get(&format!("{RESULT}{id}")).map_err(internal)?;
    if let Some(value) = result {
        let result: RetainedResult = serde_json::from_str(&value).map_err(internal)?;
        if crate::shared::time::now_epoch_f64() - result.completed_at < RESULT_RETENTION_SECS {
            return Ok(result.reply);
        }
        db.kv_set(&format!("{RESULT}{id}"), None).map_err(internal)?;
    }
    Ok(ControlReply { ok: false, result: json!({"error": "remote action completed; retained result is no longer available", "completed": true, "outcome": "completed_result_unavailable"}) })
}
fn retain_result(db: &AgentxDb, id: &str, reply: &ControlReply) -> anyhow::Result<()> {
    let now = crate::shared::time::now_epoch_f64();
    let mut values = db.kv_prefix(RESULT)?.into_iter().map(|(key, value)| serde_json::from_str::<RetainedResult>(&value).map(|value| (key, value.completed_at))).collect::<Result<Vec<_>, _>>()?;
    values.sort_by(|a, b| a.1.total_cmp(&b.1));
    let excess = values.len().saturating_sub(MAX_RETAINED_RESULTS - 1);
    for (index, (key, at)) in values.into_iter().enumerate() {
        if index < excess || now - at >= RESULT_RETENTION_SECS {
            db.kv_set(&key, None)?;
        }
    }
    db.kv_set(&format!("{RESULT}{id}"), Some(&serde_json::to_string(&RetainedResult { completed_at: now, reply: reply.clone() })?))
}
pub(super) fn complete(db: &AgentxDb, context: &RequestContext, id: &str, token: &str, reply: ControlReply) -> ServiceResult<bool> {
    if serde_json::to_vec(&reply).map_err(internal)?.len() > MAX_RESULT_BYTES {
        return Err(ServiceError::InvalidInput("relay result exceeds the 2 MiB completion budget".into()));
    }
    let mut record = load(db, id)?;
    check_owner(&record, context, token)?;
    match &record.stage {
        Stage::Completed => {
            let incoming = reply_digest(&reply)?;
            if record.completion_digest.as_deref() != Some(&incoming) {
                return Err(ServiceError::Conflict("control completion differs from its retained result".into()));
            }
            return Ok(false);
        }
        Stage::Accepted { .. } => return Err(ServiceError::Conflict("control effect has not been granted for execution".into())),
        Stage::Issuing { .. } => {}
    }
    record.stage = Stage::Completed;
    record.completion_digest = Some(reply_digest(&reply)?);
    let event = db
        .with_write_scope(|| {
            retain_result(db, id, &reply)?;
            save(db, id, &record)?;
            db.kv_set(&format!("{ACTIVE}{id}"), None)?;
            emit(db, result_data(&record, &reply))
        })
        .map_err(internal)?;
    wake_result(db, event);
    Ok(true)
}

/// Each wire request identity is located independently of action/input. Compact
/// completed receipts survive result consumption, freshness expiry and restart.
pub(super) fn accept(db: &AgentxDb, context: &RequestContext, relay_id: &str, events: &[Value], own_short: &str, source: &str, own_device: &str) -> ServiceResult<ReceiveResult> {
    let reconciled = reconcile_abandoned(db)?;
    let mut received =
        db.with_write_scope(|| accept_inner(db, context, relay_id, events, own_short, source, own_device).map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))?;
    received.changed |= reconciled;
    Ok(received)
}
fn accept_inner(db: &AgentxDb, context: &RequestContext, relay_id: &str, events: &[Value], own_short: &str, source: &str, own_device: &str) -> ServiceResult<ReceiveResult> {
    if source.is_empty() || source == "unknown" {
        return Ok(ReceiveResult::default());
    }
    if source.len() > 256 {
        return Err(ServiceError::InvalidInput("relay source identity exceeds 256 bytes".into()));
    }
    let controls = events.iter().filter(|event| event["type"] == "control").collect::<Vec<_>>();
    if controls.len() > MAX_CONTROLS {
        return Err(ServiceError::InvalidInput("too many control events in one relay envelope".into()));
    }
    let now = crate::shared::time::now_epoch_f64();
    let seen_key = format!("relay_ctrl_{source}");
    let mut seen: serde_json::Map<String, Value> = db.kv_get(&seen_key).map_err(internal)?.and_then(|value| serde_json::from_str(&value).ok()).unwrap_or_default();
    seen.retain(|_, value| value.as_f64().is_some_and(|at| now - at < FRESHNESS_SECS));
    let mut out = ReceiveResult::default();
    for event in controls {
        let Some(data) = event.get("data").filter(|value| value.is_object()) else {
            continue;
        };
        if !data["target_device"].as_str().is_some_and(|target| target.eq_ignore_ascii_case(own_short)) {
            continue;
        }
        if data.get("from_device").and_then(Value::as_str).is_some_and(|sender| sender != source) {
            continue;
        }
        let Some(ts) = event.get("ts").filter(|value| value.is_number()) else {
            continue;
        };
        let Some(at) = ts.as_f64().filter(|at| (now - at).abs() < FRESHNESS_SECS) else {
            continue;
        };
        let _ = at;
        let action = data["action"].as_str().unwrap_or("");
        if !super::control::advertised_remote_capabilities().contains(&action) {
            continue;
        }
        let request_id = data["request_id"].as_str().filter(|value| !value.is_empty());
        if request_id.is_none() && action != super::control::rpc_action::RELAY_OFF {
            continue;
        }
        if request_id.is_some_and(|id| id.len() > 256) {
            return Err(ServiceError::InvalidInput("relay request ID exceeds 256 bytes".into()));
        }
        let params = data.get("params").cloned().unwrap_or_else(|| json!({"target": data["target"].as_str().unwrap_or("")}));
        let initiated_by = data["from"].as_str().unwrap_or("remote");
        let parsed = control_parse::parse(action, &params);
        let normalized = match &parsed {
            Ok(parsed) => serde_json::to_value(parsed).map_err(internal)?,
            Err(_) => params.clone(),
        };
        let input = serde_json::to_vec(&canonical(json!([action, normalized, initiated_by]))).map_err(internal)?;
        if input.len() > MAX_INPUT_BYTES {
            return Err(ServiceError::InvalidInput("relay control input exceeds 64 KiB".into()));
        }
        let intent_digest = digest(&input);
        let ts_key = ts.to_string();
        let identity = match request_id {
            Some(request_id) => json!(["rpc", request_id]),
            None => json!(["one-way", ts_key]),
        };
        let id = digest(&serde_json::to_vec(&json!([context.database_generation, relay_id, source, identity])).map_err(internal)?);
        if db.kv_get(&format!("{RECORD}{id}")).map_err(internal)?.is_some() {
            let record = load(db, &id)?;
            if record.intent_digest != intent_digest {
                return Err(ServiceError::Conflict("relay request ID is already bound to different action/input".into()));
            }
            if seen.contains_key(&ts_key) && !matches!(&record.stage, Stage::Accepted { .. }) {
                continue;
            }
            seen.insert(ts_key.clone(), ts.clone());
            db.kv_set(&seen_key, Some(&Value::Object(seen.clone()).to_string())).map_err(internal)?;
            match &record.stage {
                Stage::Completed => {
                    let reply = retained(db, &id)?;
                    let event = db.with_write_scope(|| emit(db, result_data(&record, &reply))).map_err(internal)?;
                    out.changed |= event.is_some();
                    wake_result(db, event);
                }
                Stage::Accepted { effect } if check_owner(&record, context, &record.token).is_ok() => out.effects.push(effect.clone()),
                _ => {
                    let reply = ControlReply { ok: false, result: json!({"error": "remote action outcome is unknown or still in progress; it will not be reissued", "outcome": "unknown_remote_effect"}) };
                    let event = db.with_write_scope(|| emit(db, result_data(&record, &reply))).map_err(internal)?;
                    out.changed |= event.is_some();
                    wake_result(db, event);
                }
            }
            continue;
        }
        if seen.contains_key(&ts_key) {
            continue;
        }
        let token = uuid::Uuid::new_v4().to_string();
        let mut record = Record {
            generation: context.database_generation.clone(),
            relay_id: relay_id.into(),
            source: source.into(),
            request_id: request_id.map(str::to_owned),
            action: action.into(),
            intent_digest,
            owner_pid: context.process.pid,
            owner_incarnation: context.process.incarnation.clone(),
            token: token.clone(),
            completion_digest: None,
            stage: Stage::Completed,
        };
        let mut effect = None;
        let mut reply = None;
        match parsed {
            Err(error) => reply = Some(ControlReply::error(error)),
            Ok(Action::Host(mut action)) => match prepare_host(db, context, &mut action) {
                Err(error) => reply = Some(ControlReply::error(error)),
                Ok(participant) => {
                    if db.kv_prefix(ACTIVE).map_err(internal)?.len() >= MAX_ACTIVE {
                        reply = Some(ControlReply::error("relay remote-effect capacity is full; no new effect was admitted"));
                    } else {
                        let plan = ControlEffect { effect_id: id.clone(), token, source_device: source.into(), request_id: record.request_id.clone(), initiated_by: initiated_by.into(), participant, action };
                        record.stage = Stage::Accepted { effect: plan.clone() };
                        effect = Some(plan);
                    }
                }
            },
            Ok(action) => {
                // The operation and its retained receipt share the outer scope.
                let event = db
                    .with_write_scope(|| {
                        let applied = db.with_write_scope(|| execute_local(db, context, action, own_device).map_err(anyhow::Error::msg));
                        let reply = match applied {
                            Ok(result) => ControlReply { ok: true, result },
                            Err(error) => ControlReply::error(error),
                        };
                        record.completion_digest = Some(reply_digest(&reply)?);
                        save(db, &id, &record)?;
                        retain_result(db, &id, &reply)?;
                        seen.insert(ts_key.clone(), ts.clone());
                        db.kv_set(&seen_key, Some(&Value::Object(seen.clone()).to_string()))?;
                        emit(db, result_data(&record, &reply))
                    })
                    .map_err(internal)?;
                out.changed |= event.is_some();
                wake_result(db, event);
                continue;
            }
        }
        let event = db
            .with_write_scope(|| {
                if let Some(reply) = &reply {
                    record.completion_digest = Some(reply_digest(reply)?);
                }
                save(db, &id, &record)?;
                if effect.is_some() {
                    db.kv_set(&format!("{ACTIVE}{id}"), Some("1"))?;
                }
                if let Some(reply) = &reply {
                    retain_result(db, &id, reply)?;
                }
                seen.insert(ts_key.clone(), ts.clone());
                db.kv_set(&seen_key, Some(&Value::Object(seen.clone()).to_string()))?;
                emit(db, reply.as_ref().and_then(|reply| result_data(&record, reply)))
            })
            .map_err(internal)?;
        out.changed |= event.is_some();
        wake_result(db, event);
        if let Some(effect) = effect {
            out.effects.push(effect);
        }
    }
    Ok(out)
}

#[derive(Serialize, Deserialize)]
struct ExpectedRpc {
    generation: String,
    source: String,
    action: String,
    owner_pid: u32,
    owner_incarnation: Option<String>,
    at: f64,
}
const EXPECTED: &str = "relay_rpc_expected:";
pub(super) fn expect_rpc(db: &AgentxDb, context: &RequestContext, request_id: &str, short: &str, action: &str) -> ServiceResult<()> {
    if request_id.is_empty() || request_id.len() > 256 || !super::control::advertised_remote_capabilities().contains(&action) {
        return Err(ServiceError::InvalidInput("invalid expected relay RPC identity/action".into()));
    }
    let source = db.kv_get(&format!("relay_short_{short}")).map_err(internal)?.ok_or_else(|| ServiceError::NotFound(format!("relay device {short}")))?;
    let key = format!("{EXPECTED}{}", digest(request_id.as_bytes()));
    let at = crate::shared::time::now_epoch_f64();
    let expected = ExpectedRpc { generation: context.database_generation.clone(), source, action: action.into(), owner_pid: context.process.pid, owner_incarnation: context.process.incarnation.clone(), at };
    if let Some(value) = db.kv_get(&key).map_err(internal)? {
        let previous: ExpectedRpc = serde_json::from_str(&value).map_err(internal)?;
        if previous.generation != expected.generation || previous.source != expected.source || previous.action != expected.action || previous.owner_pid != expected.owner_pid || previous.owner_incarnation != expected.owner_incarnation {
            return Err(ServiceError::Conflict("outgoing RPC request ID is already bound to another caller/peer/action".into()));
        }
        return Ok(());
    }
    db.with_write_scope(|| {
        let mut entries = db.kv_prefix(EXPECTED)?;
        for (key, value) in &entries {
            let value: ExpectedRpc = serde_json::from_str(value)?;
            if at - value.at >= FRESHNESS_SECS {
                db.kv_set(key, None)?;
            }
        }
        entries = db.kv_prefix(EXPECTED)?;
        anyhow::ensure!(entries.len() < 1024, "too many outstanding relay RPC requests");
        db.kv_set(&key, Some(&serde_json::to_string(&expected)?))
    })
    .map_err(internal)
}

pub(super) fn rpc_result(db: &AgentxDb, context: &RequestContext, request_id: &str, consume: bool) -> ServiceResult<Option<Value>> {
    use rusqlite::OptionalExtension;
    let key = format!("{EXPECTED}{}", digest(request_id.as_bytes()));
    let Some(expected) = db.kv_get(&key).map_err(internal)? else {
        return Ok(None);
    };
    let expected: ExpectedRpc = serde_json::from_str(&expected).map_err(internal)?;
    if expected.generation != context.database_generation || expected.owner_pid != context.process.pid || expected.owner_incarnation != context.process.incarnation {
        return Err(ServiceError::StaleFence);
    }
    let row = db
        .conn()
        .query_row(
            "SELECT id,data FROM events WHERE type='rpc_result' AND json_extract(data,'$.request_id')=? AND json_extract(data,'$.action')=? AND json_extract(data,'$._relay.device')=? ORDER BY id DESC LIMIT 1",
            rusqlite::params![request_id, expected.action, expected.source],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(internal)?;
    let Some((id, data)) = row else {
        return Ok(None);
    };
    let data = serde_json::from_str(&data).map_err(internal)?;
    if consume {
        db.with_write_scope(|| {
            db.conn().execute("DELETE FROM events WHERE id=?", [id])?;
            db.kv_set(&key, None)
        })
        .map_err(internal)?;
    }
    Ok(Some(data))
}

fn local_name(db: &AgentxDb, target: &str) -> Result<String, String> {
    let name = target.to_owned();
    let row = db.get_instance_full(&name).map_err(|error| error.to_string())?.ok_or_else(|| crate::agents::identity::describe_missing_agent(db, target))?;
    if row.origin_device_id.as_deref().is_some_and(|device| !device.is_empty()) {
        return Err("remote control cannot forward to another device".into());
    }
    Ok(name)
}
fn prepare_host(db: &AgentxDb, context: &RequestContext, action: &mut HostAction) -> Result<Option<crate::agents::attachments::ParticipantRef>, String> {
    if context.process.incarnation.as_deref().is_none_or(|value| value.is_empty()) {
        return Err("remote host action requires an observed process incarnation; no effect was admitted".into());
    }
    let target = match action {
        HostAction::Kill { target } | HostAction::TermScreen { target, .. } | HostAction::TermInject { target, .. } | HostAction::InstanceTimeoutDefault { target, .. } => target,
        HostAction::Resume(request) => {
            let name = request.target.clone();
            if db.get_instance_full(&name).map_err(|error| error.to_string())?.is_none() {
                return Ok(None);
            }
            request.target = name;
            &mut request.target
        }
        HostAction::Transcript(request) if request.origin_device.is_none() => {
            let name = request.target.clone();
            if db.get_instance_full(&name).map_err(|error| error.to_string())?.is_none() {
                return Ok(None);
            }
            request.target = name;
            &mut request.target
        }
        _ => return Ok(None),
    };
    *target = local_name(db, target)?;
    match crate::agents::service::handle(db, context, crate::agents::service::Request::Lookup { name: target.clone() }).map_err(|error| error.to_string())? {
        crate::agents::service::Response::Participant(Some(participant)) => Ok(Some(participant)),
        _ => Err("local participant disappeared before remote effect admission".into()),
    }
}

fn execute_local(db: &AgentxDb, context: &RequestContext, action: Action, own_device: &str) -> Result<Value, String> {
    use crate::events::service::{Request as E, Response as R};
    let events = |request| crate::events::store::handle(db, context, request).map_err(|error| error.to_string());
    match action {
        Action::Host(_) => unreachable!("host action never executes inside daemon dispatch"),
        Action::InstanceGet { target, field } => instance_config(db, context, &target, field.as_deref(), None),
        Action::InstanceSet { target, field, value } => instance_config(db, context, &target, Some(&field), Some(&value)),
        Action::Events { filters, predicate, last, max_bytes } => remote_events(db, context, filters, predicate, last, max_bytes, own_device),
        Action::Subscribe { caller, external, filters, predicates, once, on_hit } => {
            let caller = if external { caller } else { db.get_instance_full(&caller).ok().flatten().map(|row| row.name).ok_or_else(|| format!("caller '{caller}' not found on this device"))? };
            if filters.is_empty() && predicates.is_empty() {
                return Err("provide at least one filter or SQL WHERE clause".into());
            }
            let predicate = (!predicates.is_empty()).then(|| predicates.iter().map(|part| format!("({part})")).collect::<Vec<_>>().join(" AND "));
            let R::Subscribed(created) = events(E::Subscribe { filters, predicate, owner: caller.clone(), once, on_hit })? else {
                return Err("unexpected subscription outcome".into());
            };
            Ok(json!({"id": created.id, "caller": caller, "already_existed": created.already_exists}))
        }
        Action::Subscriptions => {
            let R::Subscriptions(subs) = events(E::Subscriptions)? else {
                return Err("unexpected subscription outcome".into());
            };
            Ok(json!({"subs": subs}))
        }
        Action::Unsubscribe { id } => {
            let R::Unsubscribed(removed) = events(E::Unsubscribe { id: id.clone() })? else {
                return Err("unexpected subscription outcome".into());
            };
            Ok(json!({"id": id, "removed": removed}))
        }
    }
}

fn instance_config(db: &AgentxDb, context: &RequestContext, target: &str, field: Option<&str>, value: Option<&str>) -> Result<Value, String> {
    use crate::agents::{lifecycle_service::Request as L, service as P};
    let name = local_name(db, target)?;
    let P::Response::Record(Some(row)) = P::handle(db, context, P::Request::Record { name: name.clone() }).map_err(|error| error.to_string())? else {
        return Err("participant disappeared".into());
    };
    if let Some(value) = value {
        let P::Response::Participant(Some(participant)) = P::handle(db, context, P::Request::Lookup { name: name.clone() }).map_err(|error| error.to_string())? else {
            return Err("participant disappeared".into());
        };
        let default = value.is_empty() || value.eq_ignore_ascii_case("default");
        let seconds = || value.parse::<i64>().ok().filter(|value| *value > 0).ok_or_else(|| "timeout must be a positive integer (seconds)".to_string());
        let request = match field.unwrap_or("") {
            "hints" => L::SetHints { participant, hints: (!value.is_empty()).then(|| value.into()) },
            "timeout" => L::SetTimeout { participant, timeout: seconds()? },
            "subagent_timeout" => L::SetSubagentTimeout { participant, timeout: if default { None } else { Some(seconds()?) } },
            field => return Err(format!("Unknown instance config key '{field}'")),
        };
        P::handle(db, context, P::Request::Lifecycle(request)).map_err(|error| error.to_string())?;
        return Ok(json!({"instance": name, "field": field, "value": value}));
    }
    Ok(match field {
        None => {
            json!({"name": row.name, "timeout": row.wait_timeout, "hints": row.hints.as_deref().filter(|value| !value.is_empty()), "subagent_timeout": row.subagent_timeout})
        }
        Some("timeout") => json!({"value": row.wait_timeout.unwrap_or(86400)}),
        Some("hints") => json!({"value": row.hints.unwrap_or_default()}),
        Some("subagent_timeout") => json!({"value": row.subagent_timeout}),
        Some(field) => return Err(format!("Unknown instance config key '{field}'")),
    })
}

pub(super) fn remote_events_byte_cap(db: &AgentxDb, own_device: &str) -> usize {
    let state_bytes = serde_json::to_vec(&super::push::build_state(db, own_device)).map_or(0, |bytes| bytes.len());
    super::push::sealed_payload_budget(db).saturating_sub(state_bytes).saturating_sub(super::crypto::HEADER_LEN + super::crypto::TAG_LEN + 2 * 1024).max(1024)
}
fn remote_events(db: &AgentxDb, context: &RequestContext, filters: crate::events::filters::FilterMap, predicate: Option<String>, last: usize, max_bytes: Option<usize>, own_device: &str) -> Result<Value, String> {
    use crate::events::service::{EventQuery, Request as E, Response as R};
    let widest = remote_events_byte_cap(db, own_device);
    let byte_cap = max_bytes.unwrap_or(widest).clamp(1024, widest);
    let mut events = Vec::new();
    let mut through = None;
    while events.len() < last {
        let query = EventQuery { filters: filters.clone(), predicate: predicate.clone(), through, limit: (last - events.len()).min(crate::events::service::MAX_PAGE_EVENTS), ..Default::default() };
        let R::Page(page) = crate::events::store::handle(db, context, E::Query(query)).map_err(|error| format!("sql error: {error}"))? else {
            return Err("unexpected event page".into());
        };
        let next = page.events.last().and_then(|event| event["id"].as_i64()).map(|id| id - 1);
        let more = page.has_more && next.is_some() && next != through;
        events.extend(page.events);
        through = next;
        if !more {
            break;
        }
    }
    let mut truncated = false;
    let mut omitted = 0;
    loop {
        let mut out = json!({"events": events, "count": events.len()});
        if truncated {
            out["truncated"] = json!(true);
            out["omitted"] = json!(omitted);
            out["max_bytes"] = json!(byte_cap);
        }
        if serde_json::to_vec(&out).map_err(|error| error.to_string())?.len() <= byte_cap || events.is_empty() {
            return Ok(out);
        }
        if events.len() == 1 && byte_cap >= widest && super::push::shrink_level(&events[0]) < super::push::SHRINK_LAST_LEVEL {
            super::push::shrink_event(&mut events[0]);
        } else {
            events.pop();
            omitted += 1;
        }
        truncated = true;
    }
}

#[cfg(test)]
pub(super) fn test_context() -> RequestContext {
    RequestContext {
        caller: crate::service::CallerContext { request_id: crate::service::RequestId("relay-test".into()), invocation: Default::default(), deadline_unix_ms: None, expected_database_generation: None },
        process: crate::service::CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
        daemon_instance: "fixture-daemon".into(),
        database_generation: "fixture-generation".into(),
    }
}

#[cfg(test)]
pub(super) fn test_action(db: &AgentxDb, params: &Value, action: &str) -> Result<Value, String> {
    execute_local(db, &test_context(), control_parse::parse(action, params)?, "fixture-device")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, AgentxDb, RequestContext) {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_raw(&dir.path().join("agentx.db")).unwrap();
        db.init_db().unwrap();
        (dir, db, test_context())
    }
    fn event(id: &str, offset: f64, action: &str, params: Value) -> Value {
        json!({"type":"control","ts":crate::shared::time::now_epoch_f64().floor() + offset,"data":{"action":action,"request_id":id,"target_device":"WXYZ","from":"_:ABCD","from_device":"peer-a","params":params}})
    }
    fn admit(db: &AgentxDb, context: &RequestContext, events: &[Value]) -> ServiceResult<ReceiveResult> {
        accept(db, context, "fixture-relay", events, "WXYZ", "peer-a", "fixture-device")
    }
    fn launch(id: &str, offset: f64) -> Value {
        event(id, offset, "launch", json!({"tool":"codex","count":1,"cwd":"/does-not-exist"}))
    }
    fn reply() -> ControlReply {
        ControlReply { ok: true, result: json!({"observed":"host completed"}) }
    }
    fn last_result(db: &AgentxDb) -> Value {
        serde_json::from_str(&db.conn().query_row("SELECT data FROM events WHERE type='rpc_result' ORDER BY id DESC LIMIT 1", [], |row| row.get::<_, String>(0)).unwrap()).unwrap()
    }

    #[test]
    fn receive_admits_a_plan_without_running_launch_or_reading_its_cwd() {
        let (_dir, db, context) = fixture();
        let received = admit(&db, &context, &[launch("launch-a", 0.01)]).unwrap();
        assert!(!received.changed);
        assert_eq!(received.effects.len(), 1);
        assert!(matches!(received.effects[0].action, HostAction::Launch(_)));
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM instances", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
        assert_eq!(db.kv_prefix(ACTIVE).unwrap().len(), 1);
    }

    #[test]
    fn grant_is_unique_and_completed_receipt_survives_result_consumption() {
        let (_dir, db, context) = fixture();
        let effect = admit(&db, &context, &[launch("launch-a", 0.01)]).unwrap().effects.remove(0);
        assert!(begin(&db, &context, &effect.effect_id, &effect.token).unwrap());
        assert!(!begin(&db, &context, &effect.effect_id, &effect.token).unwrap());
        assert!(complete(&db, &context, &effect.effect_id, &effect.token, reply()).unwrap());
        assert!(!complete(&db, &context, &effect.effect_id, &effect.token, reply()).unwrap());
        db.conn().execute("DELETE FROM events WHERE type='rpc_result'", []).unwrap();
        let repeated = admit(&db, &context, &[launch("launch-a", 0.02)]).unwrap();
        assert!(repeated.effects.is_empty());
        assert!(repeated.changed);
        assert_eq!(last_result(&db)["result"], reply().result);
        assert!(db.kv_prefix(ACTIVE).unwrap().is_empty());
    }

    #[test]
    fn a_changed_action_under_one_request_identity_conflicts_before_a_second_effect() {
        let (_dir, db, context) = fixture();
        admit(&db, &context, &[launch("same-id", 0.01)]).unwrap();
        let error = admit(&db, &context, &[event("same-id", 0.02, "relay_off", json!({}))]).unwrap_err();
        assert!(matches!(error, ServiceError::Conflict(_)));
        assert_eq!(db.kv_prefix(ACTIVE).unwrap().len(), 1);
    }

    #[test]
    fn lost_grant_reply_and_worker_replacement_do_not_reissue_a_mutation() {
        let (_dir, db, context) = fixture();
        let effect = admit(&db, &context, &[launch("launch-a", 0.01)]).unwrap().effects.remove(0);
        begin(&db, &context, &effect.effect_id, &effect.token).unwrap();
        let mut replacement = context.clone();
        replacement.process.incarnation = Some("successor-host".into());
        let duplicate = admit(&db, &replacement, &[launch("launch-a", 0.02)]).unwrap();
        assert!(duplicate.effects.is_empty());
        assert_eq!(last_result(&db)["result"]["outcome"], "unknown_remote_effect");
        assert_eq!(begin(&db, &replacement, &effect.effect_id, &effect.token), Err(ServiceError::StaleFence));
        assert_eq!(complete(&db, &replacement, &effect.effect_id, &effect.token, reply()), Err(ServiceError::StaleFence));
    }

    #[test]
    fn reset_generation_and_forged_tokens_cannot_begin_or_complete_old_effects() {
        let (_dir, db, context) = fixture();
        let effect = admit(&db, &context, &[launch("launch-a", 0.01)]).unwrap().effects.remove(0);
        assert_eq!(begin(&db, &context, &effect.effect_id, "wrong-token"), Err(ServiceError::StaleFence));
        let mut reset = context.clone();
        reset.database_generation = "new-generation".into();
        assert_eq!(begin(&db, &reset, &effect.effect_id, &effect.token), Err(ServiceError::StaleFence));
        begin(&db, &context, &effect.effect_id, &effect.token).unwrap();
        assert_eq!(complete(&db, &reset, &effect.effect_id, &effect.token, reply()), Err(ServiceError::StaleFence));
    }

    #[test]
    fn failed_acceptance_rolls_back_entire_control_batch_and_returns_no_plan() {
        let (_dir, db, context) = fixture();
        db.conn().execute_batch("CREATE TRIGGER fail_control_receipt BEFORE INSERT ON kv WHEN NEW.key LIKE 'relay_control_receipt:%' BEGIN SELECT RAISE(ABORT,'fixture refusal'); END;").unwrap();
        assert!(admit(&db, &context, &[launch("launch-a", 0.01)]).is_err());
        assert!(db.kv_prefix(ACTIVE).unwrap().is_empty());
        assert!(db.kv_prefix(RECORD).unwrap().is_empty());
        assert!(db.kv_get("relay_ctrl_peer-a").unwrap().is_none());
    }

    #[test]
    fn completion_write_failure_preserves_issuing_and_never_grants_again() {
        let (_dir, db, context) = fixture();
        let effect = admit(&db, &context, &[launch("launch-a", 0.01)]).unwrap().effects.remove(0);
        begin(&db, &context, &effect.effect_id, &effect.token).unwrap();
        db.conn().execute_batch("CREATE TRIGGER fail_rpc_result BEFORE INSERT ON events WHEN NEW.type='rpc_result' BEGIN SELECT RAISE(ABORT,'fixture refusal'); END;").unwrap();
        assert!(complete(&db, &context, &effect.effect_id, &effect.token, reply()).is_err());
        assert!(matches!(load(&db, &effect.effect_id).unwrap().stage, Stage::Issuing { .. }));
        assert!(!begin(&db, &context, &effect.effect_id, &effect.token).unwrap());
    }

    #[test]
    fn active_limit_rejects_new_host_work_but_preserves_reads_and_completion() {
        let (_dir, db, context) = fixture();
        let mut first = None;
        for n in 0..MAX_ACTIVE {
            let mut received = admit(&db, &context, &[launch(&format!("launch-{n}"), 0.01 + n as f64 / 1000.0)]).unwrap();
            if n == 0 {
                first = Some(received.effects.remove(0));
            }
        }
        let overflow = admit(&db, &context, &[launch("overflow", 0.2)]).unwrap();
        assert!(overflow.effects.is_empty());
        assert!(last_result(&db)["result"]["error"].as_str().unwrap().contains("capacity"));
        let local = admit(&db, &context, &[event("read-events", 0.3, "events", json!({"last":1}))]).unwrap();
        assert!(local.effects.is_empty());
        assert!(last_result(&db)["ok"].as_bool().unwrap());
        let effect = first.unwrap();
        begin(&db, &context, &effect.effect_id, &effect.token).unwrap();
        complete(&db, &context, &effect.effect_id, &effect.token, reply()).unwrap();
        assert_eq!(db.kv_prefix(ACTIVE).unwrap().len(), MAX_ACTIVE - 1);
    }

    #[test]
    fn pruned_answer_keeps_completed_tombstone_and_never_reexecutes() {
        let (_dir, db, context) = fixture();
        let effect = admit(&db, &context, &[launch("launch-a", 0.01)]).unwrap().effects.remove(0);
        begin(&db, &context, &effect.effect_id, &effect.token).unwrap();
        complete(&db, &context, &effect.effect_id, &effect.token, reply()).unwrap();
        db.kv_set(&format!("{RESULT}{}", effect.effect_id), None).unwrap();
        assert!(!complete(&db, &context, &effect.effect_id, &effect.token, reply()).unwrap());
        let duplicate = admit(&db, &context, &[launch("launch-a", 0.02)]).unwrap();
        assert!(duplicate.effects.is_empty());
        assert_eq!(last_result(&db)["result"]["outcome"], "completed_result_unavailable");
    }

    #[test]
    fn inner_sender_disagreement_cannot_admit_an_authenticated_control_plan() {
        let (_dir, db, context) = fixture();
        let mut control = launch("launch-a", 0.01);
        control["data"]["from_device"] = json!("different-peer");
        assert!(admit(&db, &context, &[control]).unwrap().effects.is_empty());
        assert!(db.kv_prefix(RECORD).unwrap().is_empty());
    }

    #[test]
    fn remote_event_predicate_cannot_read_unrelated_managed_tables() {
        let (_dir, db, _) = fixture();
        let result = test_action(&db, &json!({"sql":"id=(SELECT COUNT(*) FROM instances)"}), "events");
        assert!(result.is_err());
    }

    #[test]
    fn outgoing_rpc_only_consumes_the_expected_peer_action_and_host() {
        let (_dir, db, context) = fixture();
        db.kv_set("relay_short_ABCD", Some("peer-a")).unwrap();
        expect_rpc(&db, &context, "outgoing-a", "ABCD", "term_screen").unwrap();
        for (peer, action) in [("peer-b", "term_screen"), ("peer-a", "kill")] {
            db.log_event("rpc_result", "_rpc", &json!({"request_id":"outgoing-a","action":action,"ok":true,"_relay":{"device":peer}})).unwrap();
        }
        assert!(rpc_result(&db, &context, "outgoing-a", true).unwrap().is_none());
        db.log_event("rpc_result", "_rpc", &json!({"request_id":"outgoing-a","action":"term_screen","ok":true,"result":{"content":"correct"},"_relay":{"device":"peer-a"}})).unwrap();
        let mut other = context.clone();
        other.process.pid += 1;
        assert_eq!(rpc_result(&db, &other, "outgoing-a", true), Err(ServiceError::StaleFence));
        assert_eq!(rpc_result(&db, &context, "outgoing-a", true).unwrap().unwrap()["result"]["content"], "correct");
        assert!(rpc_result(&db, &context, "outgoing-a", true).unwrap().is_none());
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM events WHERE type='rpc_result'", [], |row| row.get::<_, i64>(0)).unwrap(), 2);
    }

    #[test]
    fn accepted_plan_can_be_reoffered_before_begin_but_never_granted_twice() {
        let (_dir, db, context) = fixture();
        let control = launch("launch-a", 0.01);
        let first = admit(&db, &context, std::slice::from_ref(&control)).unwrap().effects.remove(0);
        let repeated = admit(&db, &context, &[control]).unwrap().effects.remove(0);
        assert_eq!(first.effect_id, repeated.effect_id);
        assert_eq!(first.token, repeated.token);
        assert!(begin(&db, &context, &first.effect_id, &first.token).unwrap());
        assert!(!begin(&db, &context, &repeated.effect_id, &repeated.token).unwrap());
    }

    #[test]
    fn dead_host_reconciliation_frees_capacity_without_replaying_its_action() {
        let (_dir, db, context) = fixture();
        let effect = admit(&db, &context, &[launch("launch-a", 0.01)]).unwrap().effects.remove(0);
        begin(&db, &context, &effect.effect_id, &effect.token).unwrap();
        let mut record = load(&db, &effect.effect_id).unwrap();
        record.owner_incarnation = Some("definitely-not-current-process-incarnation".into());
        save(&db, &effect.effect_id, &record).unwrap();
        if crate::sys::process::identity(context.process.pid).is_some() {
            assert!(reconcile_abandoned(&db).unwrap());
            assert!(db.kv_prefix(ACTIVE).unwrap().is_empty());
            let repeated = admit(&db, &context, &[launch("launch-a", 0.02)]).unwrap();
            assert!(repeated.effects.is_empty());
            assert_eq!(last_result(&db)["result"]["outcome"], "unknown_remote_effect");
        }
    }

    #[test]
    fn one_way_relay_off_keeps_wire_timestamp_identity_without_inventing_request_ids() {
        let (_dir, db, context) = fixture();
        let mut control = event("discarded", 0.01, "relay_off", json!({}));
        control["data"].as_object_mut().unwrap().remove("request_id");
        let effect = admit(&db, &context, &[control.clone()]).unwrap().effects.remove(0);
        assert!(effect.request_id.is_none());
        begin(&db, &context, &effect.effect_id, &effect.token).unwrap();
        complete(&db, &context, &effect.effect_id, &effect.token, reply()).unwrap();
        assert!(admit(&db, &context, &[control]).unwrap().effects.is_empty());
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM events WHERE type='rpc_result'", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
    }

    #[test]
    fn unknown_or_empty_host_incarnation_rejects_effects_but_keeps_native_reads_usable() {
        for incarnation in [None, Some(String::new())] {
            let (_dir, db, mut context) = fixture();
            context.process.incarnation = incarnation;
            let received = admit(&db, &context, &[launch("unverified-host", 0.01)]).unwrap();
            assert!(received.effects.is_empty());
            assert!(db.kv_prefix(ACTIVE).unwrap().is_empty());
            assert_eq!(last_result(&db)["ok"], false);
            assert!(last_result(&db)["result"]["error"].as_str().unwrap().contains("observed process incarnation"));
            let read = admit(&db, &context, &[event("native-read", 0.02, "events", json!({"last":1}))]).unwrap();
            assert!(read.effects.is_empty());
            assert_eq!(last_result(&db)["ok"], true);
        }
    }

    #[test]
    fn timeout_default_defers_file_resolution_and_freezes_the_exact_participant() {
        let (_dir, db, context) = fixture();
        db.conn().execute("INSERT INTO instances(name,created_at,wait_timeout) VALUES ('luna',1.0,33)", []).unwrap();
        let received = admit(&db, &context, &[event("default-timeout", 0.01, "config_set", json!({"instance":"luna","field":"timeout","value":"DeFaUlT"}))]).unwrap();
        assert_eq!(received.effects.len(), 1);
        let effect = &received.effects[0];
        assert!(matches!(&effect.action,HostAction::InstanceTimeoutDefault {target,requested} if target=="luna" && requested=="DeFaUlT"));
        assert_eq!(effect.participant.as_ref().unwrap().canonical_name, "luna");
        assert_eq!(effect.participant.as_ref().unwrap().incarnation_bits, 1.0f64.to_bits());
        assert_eq!(db.get_instance_full("luna").unwrap().unwrap().wait_timeout, Some(33));
        let explicit = admit(&db, &context, &[event("explicit-timeout", 0.02, "config_set", json!({"instance":"luna","field":"timeout","value":"120"}))]).unwrap();
        assert!(explicit.effects.is_empty());
        assert_eq!(last_result(&db)["ok"], true);
        assert_eq!(db.get_instance_full("luna").unwrap().unwrap().wait_timeout, Some(120));
    }

    #[test]
    fn admitted_resume_or_fork_cannot_select_a_namesake_successor() {
        use crate::agents::{lifecycle_service as L, service as P};
        for fork in [false, true] {
            let (_dir, db, context) = fixture();
            db.conn().execute("INSERT INTO instances(name,created_at,tool,session_id,status) VALUES ('luna',1.0,'codex','original-session','inactive')", []).unwrap();
            let effect = admit(&db, &context, &[event("resume-source", 0.01, "resume", json!({"target":"luna","fork":fork}))]).unwrap().effects.remove(0);
            assert!(matches!(&effect.action,HostAction::Resume(request) if request.target=="luna" && request.fork==fork));
            assert_eq!(effect.participant.as_ref().unwrap().incarnation_bits, 1f64.to_bits());
            db.conn().execute("UPDATE instances SET created_at=2.0,session_id='successor-session',status='listening' WHERE name='luna'", []).unwrap();
            let result = P::handle(&db, &context, P::Request::Lifecycle(L::Request::ResolveResume { target: "luna".into(), expected: effect.participant, fork }));
            assert!(matches!(result, Err(ServiceError::StaleFence)));
            assert_eq!(db.get_instance_full("luna").unwrap().unwrap().session_id.as_deref(), Some("successor-session"));
            assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM instances", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
        }
    }

    #[test]
    fn legacy_unknown_owner_cannot_reoffer_begin_or_complete_through_option_equality() {
        for incarnation in [None, Some(String::new())] {
            let (_dir, db, context) = fixture();
            let effect = admit(&db, &context, &[launch("legacy-owner", 0.01)]).unwrap().effects.remove(0);
            let mut record = load(&db, &effect.effect_id).unwrap();
            record.owner_incarnation = incarnation.clone();
            save(&db, &effect.effect_id, &record).unwrap();
            let mut unknown = context.clone();
            unknown.process.incarnation = incarnation;
            assert!(admit(&db, &unknown, &[launch("legacy-owner", 0.02)]).unwrap().effects.is_empty());
            assert_eq!(last_result(&db)["result"]["outcome"], "unknown_remote_effect");
            assert_eq!(begin(&db, &unknown, &effect.effect_id, &effect.token), Err(ServiceError::StaleFence));
            assert_eq!(complete(&db, &unknown, &effect.effect_id, &effect.token, reply()), Err(ServiceError::StaleFence));
            assert!(matches!(load(&db, &effect.effect_id).unwrap().stage, Stage::Accepted { .. }));
        }
    }

    #[test]
    fn retirement_requires_positive_exit_or_two_known_different_incarnations() {
        for (prior, current, exited, retired) in [
            (None, None, false, false),
            (None, Some("observed"), false, false),
            (Some(""), Some("observed"), false, false),
            (Some("prior"), None, false, false),
            (Some("prior"), Some(""), false, false),
            (Some("prior"), Some("prior"), false, false),
            (Some("prior"), Some("successor"), false, true),
            (None, None, true, true),
            (Some(""), None, true, true),
            (Some("prior"), None, true, true),
        ] {
            let (_dir, db, context) = fixture();
            let effect = admit(&db, &context, &[launch("retirement", 0.01)]).unwrap().effects.remove(0);
            begin(&db, &context, &effect.effect_id, &effect.token).unwrap();
            let mut record = load(&db, &effect.effect_id).unwrap();
            record.owner_incarnation = prior.map(str::to_owned);
            save(&db, &effect.effect_id, &record).unwrap();
            let changed = reconcile_abandoned_with(&db, |pid| {
                assert_eq!(pid, context.process.pid);
                assert!(db.conn().is_autocommit());
                (exited, current.map(str::to_owned))
            })
            .unwrap();
            assert_eq!(changed, retired, "prior={prior:?} current={current:?} exited={exited}");
            assert_eq!(db.kv_prefix(ACTIVE).unwrap().is_empty(), retired);
            if retired {
                assert!(admit(&db, &context, &[launch("retirement", 0.02)]).unwrap().effects.is_empty());
                assert_eq!(last_result(&db)["result"]["outcome"], "unknown_remote_effect");
            } else {
                assert!(matches!(load(&db, &effect.effect_id).unwrap().stage, Stage::Issuing { .. }));
            }
        }
    }
}
