//! Daemon-only Hermes chat identity and registry transactions. No Provider
//! files, host subprocesses, signal delivery or recursive client calls occur.
#[cfg(test)]
mod acceptance_tests;

use super::attachments::ParticipantRef;
use super::hermes_service::{ActiveBridge, Bridge, GrantOperation, RegistryEntry, Request, Response, Visibility};
use crate::db::{AgentxDb, InstanceRow};
use crate::service::{RequestContext, ServiceError, ServiceResult};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
use std::path::{Component, Path};

const PREFIX: &str = "hermes_bridge:";
fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn scope_home(home: &Path) -> ServiceResult<String> {
    if !home.is_absolute() || home.components().any(|part| matches!(part, Component::ParentDir)) {
        return Err(ServiceError::InvalidInput("canonical absolute Hermes profile required".into()));
    }
    home.to_str().filter(|home| !home.is_empty()).map(str::to_owned).ok_or_else(|| ServiceError::InvalidInput("invalid Hermes profile path".into()))
}
fn fence(row: &InstanceRow) -> ParticipantRef {
    ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }
}
fn row_key(bridge: &Bridge) -> String {
    format!("hermes_bridge_row:{}", bridge.token)
}
fn load(db: &AgentxDb, name: &str) -> ServiceResult<Option<Bridge>> {
    db.kv_get(&format!("{PREFIX}{name}")).map_err(internal)?.map(|value| serde_json::from_str(&value).map_err(internal)).transpose()
}
pub(crate) fn stored_bridges(db: &AgentxDb) -> ServiceResult<Vec<Bridge>> {
    db.kv_prefix(PREFIX).map_err(internal)?.into_iter().map(|(_, value)| serde_json::from_str(&value).map_err(internal)).collect()
}
/// A silent gateway is omitted after the existing 90-second observation lease.
/// Profile files and the note's restart/install advice belong to the host.
pub(crate) fn stored_stale_gateways(db: &AgentxDb) -> ServiceResult<Vec<String>> {
    const PREFIX: &str = "hermes_gateway_stale:";
    let now = crate::shared::time::now_epoch_i64();
    Ok(db.kv_prefix(PREFIX).map_err(internal)?.into_iter().filter(|(_, seen)| seen.parse::<i64>().is_ok_and(|seen| now.saturating_sub(seen) < 90)).map(|(key, _)| key[PREFIX.len()..].to_owned()).collect())
}
fn save(db: &AgentxDb, bridge: &Bridge) -> ServiceResult<()> {
    db.kv_set(&format!("{PREFIX}{}", bridge.name), Some(&serde_json::to_string(bridge).map_err(internal)?)).map_err(internal)
}
fn participant(db: &AgentxDb, bridge: &Bridge) -> ServiceResult<Option<ParticipantRef>> {
    let Some(row) = db.get_instance_full(&bridge.name).map_err(internal)? else {
        return Ok(None);
    };
    if row.origin_device_id.is_some() || row.pid.is_some() {
        return Err(ServiceError::StaleFence);
    }
    let owner = db.get_process_binding(&bridge.token).map_err(internal)?;
    if owner.as_deref().is_some_and(|name| name != bridge.name) {
        return Err(ServiceError::StaleFence);
    }
    let bound = owner.as_deref() == Some(bridge.name.as_str());
    let current = fence(&row);
    if let Some(saved) = db.kv_get(&row_key(bridge)).map_err(internal)? {
        let saved: ParticipantRef = serde_json::from_str(&saved).map_err(internal)?;
        if saved != current || row.tool != "hermes" {
            return Err(ServiceError::StaleFence);
        }
    } else if !bound {
        // Legacy registries gain an incarnation fence only from their existing
        // exact token binding, never from a visible namesake or chat label.
        return Err(ServiceError::StaleFence);
    }
    Ok(Some(current))
}
fn last_stop(db: &AgentxDb, name: &str) -> ServiceResult<Option<Value>> {
    let raw: Option<String> = db.conn().query_row("SELECT data FROM events WHERE type='life' AND instance=?1 AND json_extract(data,'$.action')='stopped' ORDER BY id DESC LIMIT 1", [name], |row| row.get(0)).optional().map_err(internal)?;
    raw.map(|raw| serde_json::from_str(&raw).map_err(internal)).transpose()
}
fn bind(db: &AgentxDb, bridge: &Bridge, session: &str) -> ServiceResult<()> {
    if session.is_empty() {
        return Err(ServiceError::InvalidInput("Hermes session required".into()));
    }
    if db.get_instance_full(&bridge.name).map_err(internal)?.is_none() {
        return Err(ServiceError::StaleFence);
    }
    // A route may follow a stopped CLI/bridge session, but cannot silently
    // steal the active session of another current participant.
    if let Some(other) = db.get_session_binding(session).map_err(internal)?
        && other != bridge.name
        && db.get_instance_full(&other).map_err(internal)?.is_some()
    {
        return Err(ServiceError::Conflict("Hermes session belongs to another participant".into()));
    }
    db.delete_session_bindings_for_instance(&bridge.name).map_err(internal)?;
    db.rebind_session(session, &bridge.name).map_err(internal)?;
    db.update_instance_fields(&bridge.name, json!({"session_id":session}).as_object().unwrap()).map_err(internal)?;
    db.set_process_binding(&bridge.token, session, &bridge.name).map_err(internal)
}
fn activate(db: &AgentxDb, context: &RequestContext, bridge: &Bridge) -> ServiceResult<ActiveBridge> {
    let existing = participant(db, bridge)?;
    let recreated = existing.is_none();
    if recreated {
        let stopped = last_stop(db, &bridge.name)?;
        if let Some(saved) = db.kv_get(&row_key(bridge)).map_err(internal)? {
            let saved: ParticipantRef = serde_json::from_str(&saved).map_err(internal)?;
            if stopped.as_ref().is_some_and(|event| event.pointer("/snapshot/created_at").and_then(Value::as_f64).is_none_or(|created| created.to_bits() != saved.incarnation_bits)) {
                return Err(ServiceError::StaleFence);
            }
        }
        let maximum = db.get_last_event_id();
        let cursor = stopped.as_ref().and_then(|event| event.pointer("/snapshot/last_event_id")).and_then(Value::as_i64).filter(|cursor| (0..=maximum).contains(cursor)).unwrap_or(maximum);
        let mut invocation = context.caller.invocation.clone();
        invocation.environment.insert("AGENTX_LAUNCH_EVENT_ID".into(), cursor.to_string());
        let binding = super::binding::BindingContext::explicit(db, &invocation);
        let transcript = Path::new(&bridge.home).join("state.db");
        if !super::binding::initialize_instance_with_context(db, &binding, &bridge.name, None, None, None, None, transcript.to_str(), Some("hermes"), false, None, None, None, Some(&bridge.home)) {
            return Err(ServiceError::Internal("cannot initialize Hermes bridge".into()));
        }
        db.update_instance_fields(&bridge.name, json!({"last_event_id":cursor}).as_object().unwrap()).map_err(internal)?;
        db.delete_process_bindings_for_instance(&bridge.name).map_err(internal)?;
        db.set_process_binding(&bridge.token, "", &bridge.name).map_err(internal)?;
        if let Some(session) = stopped.as_ref().and_then(|event| event.pointer("/snapshot/session_id")).and_then(Value::as_str).filter(|session| !session.is_empty()) {
            bind(db, bridge, session)?;
        }
        super::lifecycle::try_set_status(db, &bridge.name, "listening", "start", Default::default()).map_err(internal)?;
        db.log_event("life", &bridge.name, &json!({"action":"started","tool":"hermes","name":bridge.name})).map_err(internal)?;
    } else {
        let row = db.get_instance_full(&bridge.name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        if db.get_process_binding(&bridge.token).map_err(internal)?.as_deref() != Some(bridge.name.as_str()) {
            db.set_process_binding(&bridge.token, row.session_id.as_deref().unwrap_or(""), &bridge.name).map_err(internal)?;
        }
    }
    db.update_instance_fields(
        &bridge.name,
        json!({"tool":"hermes","directory":bridge.home,"transcript_path":Path::new(&bridge.home).join("state.db").to_string_lossy(),"launch_context":json!({"hermes_bridge":{"platform":bridge.platform,"chat":bridge.chat}}).to_string()})
            .as_object()
            .unwrap(),
    )
    .map_err(internal)?;
    db.update_heartbeat(&bridge.name).map_err(internal)?;
    let row = db.get_instance_full(&bridge.name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    let participant = fence(&row);
    db.kv_set(&row_key(bridge), Some(&serde_json::to_string(&participant).map_err(internal)?)).map_err(internal)?;
    Ok(ActiveBridge { bridge: bridge.clone(), participant, recreated })
}
fn current_actor(db: &AgentxDb, context: &RequestContext, expected: &ParticipantRef) -> ServiceResult<Bridge> {
    let bridge = load(db, &expected.canonical_name)?.ok_or(ServiceError::StaleFence)?;
    if participant(db, &bridge)?.as_ref() != Some(expected) || context.caller.invocation.process_binding_id.as_deref() != Some(bridge.token.as_str()) || super::service_store::resolve_actor(db, context)?.participant != *expected {
        return Err(ServiceError::StaleFence);
    }
    Ok(bridge)
}
fn validate_active(db: &AgentxDb, entry: &ActiveBridge) -> ServiceResult<()> {
    if load(db, &entry.bridge.name)?.as_ref() != Some(&entry.bridge) || participant(db, &entry.bridge)?.as_ref() != Some(&entry.participant) {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}
fn forget_inner(db: &AgentxDb, context: &RequestContext, bridge: &Bridge, expected: Option<&ParticipantRef>, by: &str, reason: &str) -> ServiceResult<Vec<super::hook_service::RetiredParticipant>> {
    if load(db, &bridge.name)?.as_ref() != Some(bridge) || participant(db, bridge)?.as_ref() != expected {
        return Err(ServiceError::StaleFence);
    }
    let effects = if let Some(row) = db.get_instance_full(&bridge.name).map_err(internal)? {
        // Registry admission is the authority; reuse the canonical child-first
        // retirement and return its frozen host effects after commit.
        let outcome = if row.status_context == "exit:killed" { super::hook_store::stop_killed(db, context, &fence(&row))? } else { super::hook_store::hard_stop(db, context, &row, by, reason)? };
        outcome.effects
    } else {
        Vec::new()
    };
    db.kv_set(&format!("{PREFIX}{}", bridge.name), None).map_err(internal)?;
    db.kv_set(&row_key(bridge), None).map_err(internal)?;
    Ok(effects)
}
fn forget_profile(db: &AgentxDb, context: &RequestContext, home: &str) -> ServiceResult<(Vec<Bridge>, Vec<super::hook_service::RetiredParticipant>)> {
    let bridges = stored_bridges(db)?.into_iter().filter(|bridge| bridge.home == home).collect::<Vec<_>>();
    let mut effects = Vec::new();
    for bridge in &bridges {
        let expected = participant(db, bridge)?;
        effects.extend(forget_inner(db, context, bridge, expected.as_ref(), "system", "uninstalled")?);
    }
    Ok((bridges, effects))
}
fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}
pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    if context.process.pid == 0 || context.process.incarnation.as_deref().is_none_or(str::is_empty) {
        return Err(ServiceError::Forbidden("authenticated Hermes caller required".into()));
    }
    transaction(db, || match request {
        Request::StaleGateways => Ok(Response::StaleGateways(stored_stale_gateways(db)?)),
        Request::List { home } => {
            let home = home.as_deref().map(scope_home).transpose()?;
            let registry =
                stored_bridges(db)?.into_iter().filter(|bridge| home.as_ref().is_none_or(|home| *home == bridge.home)).map(|bridge| Ok(RegistryEntry { participant: participant(db, &bridge)?, bridge })).collect::<ServiceResult<Vec<_>>>()?;
            Ok(Response::Registry(registry))
        }
        Request::Add(input) => {
            let home = scope_home(&input.home)?;
            if input.session_key.is_empty() || input.platform.is_empty() || input.name.as_deref().is_some_and(|name| !super::identity::is_valid_name(name)) {
                return Err(ServiceError::InvalidInput("valid Hermes chat, platform and name required".into()));
            }
            let intent = json!({"hermes_add":input});
            let entry = super::service_store::retained(db, context, &intent, || {
                if let Some(bridge) = stored_bridges(db)?.into_iter().find(|bridge| bridge.home == home && bridge.session_key == input.session_key) {
                    return activate(db, context, &bridge);
                }
                let name = match &input.name {
                    Some(name) => name.clone(),
                    None => super::names::allocate_unreserved_name(db).map_err(internal)?,
                };
                if load(db, &name)?.is_some() || db.get_instance_full(&name).map_err(internal)?.is_some() {
                    return Err(ServiceError::Conflict(format!("The name {name} is taken")));
                }
                let bridge = Bridge {
                    name,
                    token: format!("hermes-bridge-{}", uuid::Uuid::new_v4()),
                    home: home.clone(),
                    session_key: input.session_key.clone(),
                    platform: input.platform.clone(),
                    chat: input.chat.clone(),
                    origin: input.origin.clone(),
                    visibility: Visibility::default(),
                };
                save(db, &bridge)?;
                let entry = activate(db, context, &bridge)?;
                if let Some(session) = input.session_id.as_deref().filter(|session| !session.is_empty()) {
                    bind(db, &bridge, session)?;
                }
                Ok(entry)
            })?;
            validate_active(db, &entry)?;
            Ok(Response::Active(entry))
        }
        Request::Sync { home } => {
            let home = scope_home(&home)?;
            let mut entries = Vec::new();
            for bridge in stored_bridges(db)?.into_iter().filter(|bridge| bridge.home == home) {
                if db.get_instance_full(&bridge.name).map_err(internal)?.is_none() && last_stop(db, &bridge.name)?.is_some_and(|event| event["reason"] == "killed") {
                    forget_inner(db, context, &bridge, None, "system", "killed")?;
                } else {
                    entries.push(activate(db, context, &bridge)?);
                }
            }
            Ok(Response::Synced(entries))
        }
        Request::BindSession { participant, session_id } => {
            let bridge = current_actor(db, context, &participant)?;
            super::service_store::retained(db, context, &json!({"hermes_bind_session":{"participant":participant,"session_id":session_id}}), || bind(db, &bridge, &session_id))?;
            Ok(Response::Changed)
        }
        Request::Visibility { participant, visibility } => {
            let bridge = current_actor(db, context, &participant)?;
            super::service_store::retained(db, context, &json!({"hermes_visibility":{"participant":participant,"visibility":visibility}}), || {
                let mut bridge = bridge.clone();
                bridge.visibility = visibility;
                save(db, &bridge)
            })?;
            let bridge = load(db, &bridge.name)?.ok_or(ServiceError::StaleFence)?;
            Ok(Response::Active(ActiveBridge { bridge, participant, recreated: false }))
        }
        Request::Remove { expected } => {
            let intent = json!({"hermes_remove":expected});
            let removed = super::service_store::retained(db, context, &intent, || {
                let effects = forget_inner(db, context, &expected.bridge, expected.participant.as_ref(), "system", "disconnected")?;
                Ok((expected.bridge.clone(), effects))
            })?;
            if load(db, &expected.bridge.name)?.is_some() || db.get_instance_full(&expected.bridge.name).map_err(internal)?.is_some() {
                return Err(ServiceError::StaleFence);
            }
            let outcome = super::hook_service::SessionStopOutcome { stopped: true, retained: false, effects: removed.1 };
            super::hook_store::validate_stop_outcome(db, &outcome)?;
            Ok(Response::Removed { bridge: removed.0, effects: outcome.effects })
        }
        Request::ForgetHome { home } => {
            let intent = json!({"hermes_forget_home":home});
            let (bridges, effects) = super::service_store::retained(db, context, &intent, || forget_profile(db, context, &scope_home(&home)?))?;
            let outcome = super::hook_service::SessionStopOutcome { stopped: true, retained: false, effects };
            super::hook_store::validate_stop_outcome(db, &outcome)?;
            for bridge in &bridges {
                if load(db, &bridge.name)?.is_some() || db.get_instance_full(&bridge.name).map_err(internal)?.is_some() {
                    return Err(ServiceError::StaleFence);
                }
            }
            Ok(Response::Forgotten { bridges, effects: outcome.effects })
        }
        Request::Grant { home, operation } => {
            let key = format!("hermes_grant:{}", scope_home(&home)?);
            match operation {
                GrantOperation::Read => {}
                GrantOperation::RecordOwned => db.kv_set(&key, Some("1")).map_err(internal)?,
                GrantOperation::ClearOwned => db.kv_set(&key, None).map_err(internal)?,
            }
            Ok(Response::GrantOwned(db.kv_get(&key).map_err(internal)?.is_some()))
        }
        Request::GatewayVersion { home, current } => {
            let key = format!("hermes_gateway_stale:{}", scope_home(&home)?);
            let stamp = crate::shared::time::now_epoch_i64().to_string();
            db.kv_set(&key, if current { None } else { Some(stamp.as_str()) }).map_err(internal)?;
            Ok(Response::Changed)
        }
    })
}
