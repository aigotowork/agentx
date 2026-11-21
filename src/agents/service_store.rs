//! Daemon-owned participant use cases. Caller configuration is explicit;
//! process authority and row incarnation are checked separately.

#[cfg(test)]
mod acceptance_tests;

use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

use super::attachments::*;
use super::service::*;
use crate::db::{AgentxDb, InstanceRow};
use crate::service::{CallerProcess, RequestContext, ServiceError, ServiceResult};

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

fn actor(row: InstanceRow) -> ResolvedActor {
    ResolvedActor { participant: ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }, record: row }
}

fn row(db: &AgentxDb, name: &str) -> ServiceResult<ResolvedActor> {
    db.get_instance_full(name).map_err(internal)?.map(actor).ok_or_else(|| ServiceError::NotFound(format!("participant '{name}'")))
}

/// Resolve claims against retained bindings without consulting daemon environment.
/// Explicit CLI names select local user authority; managed targets still carry
/// their full incarnation/execution/binding, rather than inheriting a name.
pub(crate) fn resolve_actor(db: &AgentxDb, context: &RequestContext) -> ServiceResult<ResolvedActor> {
    let invocation = &context.caller.invocation;
    // Verified shell actor wins shared process/session defaults. Legacy
    // injected environment tokens remain optional enhancements: invalid or
    // expired legacy tokens retain the established manual-resolution fallback.
    let token = invocation.actor_capability.as_deref().or_else(|| invocation.environment.get("AGENTX_CLAUDE_ACTOR").map(String::as_str));
    let session = invocation.environment.get("AGENTX_CLAUDE_ACTOR_SESSION").map(String::as_str).or_else(|| invocation.provider_session.as_ref().map(|session| session.session_id.as_str())).unwrap_or("");
    if let Some(token) = token {
        if let Some(name) = db.resolve_claude_actor_capability(token, session).map_err(internal)? {
            let resolved = row(db, &name)?;
            if let Some(explicit) = invocation.explicit_name.as_deref() {
                let selected = super::identity::resolve_from_name(db, explicit).map_err(|error| ServiceError::InvalidInput(error.to_string()))?;
                if selected.name != resolved.record.name {
                    return Err(ServiceError::Forbidden("explicit name conflicts with verified shell actor".into()));
                }
            }
            if invocation.activation_target.as_ref().is_some_and(|target| target.participant != resolved.participant) {
                return Err(ServiceError::Forbidden("attachment conflicts with verified shell actor".into()));
            }
            return Ok(resolved);
        }
        if invocation.actor_capability.is_some() {
            return Err(ServiceError::Forbidden("invalid shell actor capability".into()));
        }
    }
    if let Some(target) = &invocation.activation_target {
        if !SqliteAttachmentStore::new(db).validate(target).map_err(internal)? {
            return Err(ServiceError::StaleFence);
        }
        let resolved = row(db, &target.participant.canonical_name)?;
        if resolved.participant != target.participant {
            return Err(ServiceError::StaleFence);
        }
        return Ok(resolved);
    }
    if let Some(name) = invocation.explicit_name.as_deref().filter(|name| !name.is_empty()) {
        let name = db.get_instance_full(name).ok().flatten().map(|row| row.name).or_else(|| db.get_instance_by_agent_id(name).ok().flatten()).ok_or_else(|| ServiceError::NotFound(format!("participant '{name}'")))?;
        return row(db, &name);
    }
    if let Some(session) = &invocation.provider_session {
        let store = SqliteAttachmentStore::new(db);
        // Canonical lookup can restore a stopped row, so resolution uses the
        // read-only current attachment/session bindings instead.
        if let Some(name) = db.get_session_binding(&session.session_id).map_err(internal)? {
            let resolved = row(db, &name)?;
            if resolved.record.tool != session.provider {
                return Err(ServiceError::StaleFence);
            }
            return Ok(resolved);
        }
        let target: Option<String> = db
            .conn()
            .query_row(
                "SELECT a.target FROM participant_attachments a JOIN participant_sessions s ON s.id=a.lineage WHERE a.active=1 AND s.provider=? AND s.scope=? AND s.session_id=?",
                params![session.provider, session.scope, session.session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(internal)?;
        if let Some(target) = target {
            let target: ActivationTarget = serde_json::from_str(&target).map_err(internal)?;
            if !store.validate(&target).map_err(internal)? {
                return Err(ServiceError::StaleFence);
            }
            return row(db, &target.participant.canonical_name);
        }
    }
    if let Some(binding) = invocation.process_binding_id.as_deref().filter(|binding| !binding.is_empty()) {
        if let Some(name) = db.get_process_binding(binding).map_err(internal)? {
            return row(db, &name);
        }
        return Err(ServiceError::IdentityRequired("session expired; run 'agentx start' to reconnect".into()));
    }
    Err(ServiceError::IdentityRequired("no AgentX participant in caller context".into()))
}

fn check_participant(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<ResolvedActor> {
    let current = row(db, &participant.canonical_name).map_err(|error| match error {
        ServiceError::NotFound(_) => ServiceError::StaleFence,
        other => other,
    })?;
    if current.participant != *participant {
        return Err(ServiceError::StaleFence);
    }
    Ok(current)
}

/// A visible owner token is correlation, never proof of process ownership.
pub(crate) fn attachment_owned_by_process(db: &AgentxDb, target: &ActivationTarget, owner: &AttachmentOwnerId, peer: &CallerProcess) -> ServiceResult<bool> {
    let Some(incarnation) = peer.incarnation.as_deref().filter(|value| !value.is_empty()) else {
        return Ok(false);
    };
    if peer.pid == 0 {
        return Ok(false);
    }
    let stored: Option<(String, String, u32, Option<String>)> = db
        .conn()
        .query_row("SELECT target, owner, owner_pid, owner_identity FROM participant_attachments WHERE binding_id=? AND active=1", [&target.binding.0], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .optional()
        .map_err(internal)?;
    let Some((stored_target, stored_owner, pid, identity)) = stored else {
        return Ok(false);
    };
    let stored_target: ActivationTarget = serde_json::from_str(&stored_target).map_err(internal)?;
    Ok(stored_target == *target && stored_owner == owner.0 && pid == peer.pid && identity.as_deref() == Some(incarnation) && SqliteAttachmentStore::new(db).validate(target).map_err(internal)?)
}

fn verified_process(context: &RequestContext) -> ServiceResult<AttachmentOwnerProcess> {
    let identity = context.process.incarnation.clone().filter(|value| !value.is_empty()).ok_or_else(|| ServiceError::Forbidden("process incarnation unavailable".into()))?;
    if context.process.pid == 0 {
        return Err(ServiceError::Forbidden("authenticated caller PID required".into()));
    }
    Ok(AttachmentOwnerProcess { pid: context.process.pid, process_identity: Some(identity) })
}

/// Persist replayable participant results atomically with their state changes.
/// This is used only by operations whose effects fit the SQLite transaction.
pub(super) fn retained<T: serde::Serialize + serde::de::DeserializeOwned>(db: &AgentxDb, context: &RequestContext, input: &Value, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    if context.caller.request_id.0.is_empty() {
        return Err(ServiceError::InvalidInput("request ID required".into()));
    }
    let key = format!("participant_request:{}:{}", context.database_generation, context.caller.request_id.0);
    // Compare complete intent without retaining arbitrary invocation secrets.
    use sha2::{Digest, Sha256};
    let intent_bytes = serde_json::to_vec(&json!({"input":input,"invocation":context.caller.invocation,"pid":context.process.pid,"incarnation":context.process.incarnation})).map_err(internal)?;
    let intent = Value::String(Sha256::digest(intent_bytes).iter().map(|byte| format!("{byte:02x}")).collect());
    transaction(db, || {
        if let Some(stored) = db.kv_get(&key).map_err(internal)? {
            let stored: Value = serde_json::from_str(&stored).map_err(internal)?;
            if stored["intent"] != intent {
                return Err(ServiceError::Conflict("request ID reused for different participant operation".into()));
            }
            return serde_json::from_value(stored["outcome"].clone()).map_err(internal);
        }
        let outcome = action()?;
        db.kv_set(&key, Some(&serde_json::to_string(&json!({"intent":intent,"outcome":outcome})).map_err(internal)?)).map_err(internal)?;
        Ok(outcome)
    })
}

fn subscribe_defaults(db: &AgentxDb, name: &str, tool: &str, presets: &str) -> ServiceResult<()> {
    if !tool.parse::<crate::integrations::tool::Tool>().is_ok_and(|tool| tool.spec().released) {
        return Ok(());
    }
    db.cleanup_subscriptions(name).map_err(internal)?;
    db.cleanup_thread_memberships_for_name_reuse(name).map_err(internal)?;
    for preset in presets.split(',').map(str::trim).filter(|value| !value.is_empty()) {
        let (field, value) = match preset {
            "collision" => ("collision", "1"),
            "created" => ("action", "created"),
            "stopped" => ("action", "stopped"),
            "blocked" => ("status", "blocked"),
            _ => return Err(ServiceError::InvalidInput(format!("unknown subscription preset '{preset}'"))),
        };
        let filters = std::collections::HashMap::from([(field.to_owned(), vec![value.to_owned()])]);
        crate::db::subscriptions::create_filter_subscription(db, &filters, &[], name, false, None).map_err(ServiceError::InvalidInput)?;
    }
    Ok(())
}

fn register(db: &AgentxDb, context: &RequestContext, input: &Registration) -> ServiceResult<ResolvedActor> {
    if input.tool.is_empty() || input.wait_timeout < 0 {
        return Err(ServiceError::InvalidInput("tool and nonnegative timeout required".into()));
    }
    let name = match &input.name {
        Some(name) if super::identity::is_valid_name(name) => name.clone(),
        Some(_) => return Err(ServiceError::InvalidInput("invalid participant name".into())),
        None => super::names::allocate_unreserved_name(db).map_err(internal)?,
    };
    if db.get_instance_full(&name).map_err(internal)?.is_some() {
        return Err(ServiceError::Conflict(format!("participant '{name}' exists")));
    }
    if let Some(session) = input.session_id.as_deref().filter(|value| !value.is_empty())
        && db.get_session_binding(session).map_err(internal)?.is_some()
    {
        return Err(ServiceError::Conflict("session already bound".into()));
    }
    if let Some(binding) = input.process_binding_id.as_deref().filter(|value| !value.is_empty())
        && db.get_process_binding(binding).map_err(internal)?.is_some()
    {
        return Err(ServiceError::Conflict("process already bound".into()));
    }
    let invocation = &context.caller.invocation;
    let now = crate::shared::time::now_epoch_f64();
    let current_max = db.get_last_event_id();
    let cursor = invocation.environment.get("AGENTX_LAUNCH_EVENT_ID").and_then(|value| value.parse::<i64>().ok()).filter(|value| *value >= 0 && *value <= current_max).unwrap_or(current_max);
    let launch_context = json!({"env": launch_environment(&invocation.environment), "process_id":input.process_binding_id});
    let data = json!({"created_at":now,"last_event_id":cursor,"directory":invocation.cwd.to_string_lossy(),"tool":input.tool,
        "session_id":input.session_id,"transcript_path":input.transcript_path.as_deref().unwrap_or(""),"wait_timeout":input.wait_timeout,
        "subagent_timeout":input.subagent_timeout,"hints":input.hints,"status":"inactive","status_context":"new","status_time":crate::shared::time::now_epoch_i64(),"launch_context":launch_context.to_string()});
    db.save_instance_reservation(&name, data.as_object().unwrap()).map_err(internal)?;
    if let Some(session) = input.session_id.as_deref().filter(|value| !value.is_empty()) {
        db.set_session_binding(session, &name).map_err(internal)?;
    }
    if let Some(binding) = input.process_binding_id.as_deref().filter(|value| !value.is_empty()) {
        db.set_process_binding(binding, input.session_id.as_deref().unwrap_or(""), &name).map_err(internal)?;
    }
    subscribe_defaults(db, &name, &input.tool, &input.auto_subscribe)?;
    db.insert_event_row(
        "life",
        &name,
        &json!({"action":"created","by":invocation.environment.get("AGENTX_LAUNCHED_BY").map(String::as_str).unwrap_or("unknown"),"is_agentx_launched":invocation.environment.get("AGENTX_LAUNCHED").is_some_and(|value| value=="1")}),
        None,
    )
    .map_err(internal)?;
    row(db, &name)
}

fn launch_environment(environment: &std::collections::HashMap<String, String>) -> std::collections::BTreeMap<&str, &str> {
    const KEYS: &[&str] = &[
        "AGENTX_PROCESS_ID",
        "AGENTX_INSTANCE_NAME",
        "AGENTX_LAUNCHED",
        "AGENTX_LAUNCHED_BY",
        "AGENTX_LAUNCH_BATCH_ID",
        "AGENTX_LAUNCH_EVENT_ID",
        "AGENTX_LAUNCHED_PRESET",
        "AGENTX_TIMEOUT",
        "AGENTX_SUBAGENT_TIMEOUT",
        "AGENTX_HINTS",
        "AGENTX_AUTO_SUBSCRIBE",
        "CODEX_HOME",
    ];
    environment.iter().filter(|(key, _)| KEYS.contains(&key.as_str())).map(|(key, value)| (key.as_str(), value.as_str())).collect()
}

pub(crate) fn reserve_native_root(db: &AgentxDb, context: &RequestContext, input: crate::runtime::hosts::NativeRootReservation) -> ServiceResult<ParticipantRef> {
    reserve_native_root_with_collaboration(db, context, input, None)
}

pub(crate) fn reserve_product_root(db: &AgentxDb, context: &RequestContext, input: crate::runtime::hosts::NativeRootReservation, role: String, workspace: std::path::PathBuf) -> ServiceResult<ParticipantRef> {
    let scope = db.path().parent().ok_or(ServiceError::StaleFence)?;
    if !scope.join("product-execution.enabled").is_file() {
        return Err(ServiceError::Forbidden("scope has not enabled dedicated product execution".into()));
    }
    let workspace = workspace.canonicalize().map_err(internal)?;
    if workspace != input.cwd || !workspace.is_dir() {
        return Err(ServiceError::InvalidInput("product root requires its canonical workspace as cwd".into()));
    }
    let role = crate::config::roles::resolve_role(scope, Some(&workspace), &role).map_err(internal)?;
    let collaboration = super::workspaces::LaunchCollaboration { workspace: Some(workspace.to_string_lossy().into_owned()), role: Some(role), endpoint: false, instruction_additions: None };
    super::workspaces::validate(&collaboration)?;
    reserve_native_root_with_collaboration(db, context, input, Some(collaboration))
}

fn reserve_native_root_with_collaboration(db: &AgentxDb, context: &RequestContext, input: crate::runtime::hosts::NativeRootReservation, collaboration: Option<super::workspaces::LaunchCollaboration>) -> ServiceResult<ParticipantRef> {
    verified_process(context)?;
    if input.execution.0.is_empty() || input.owner.0.is_empty() || !input.cwd.is_absolute() || !input.canonical_home.is_absolute() {
        return Err(ServiceError::InvalidInput("qualified native execution, owner, cwd and home required".into()));
    }
    let intent = if let Some(collaboration) = &collaboration { json!({"reserve_product_root":input,"collaboration":collaboration}) } else { json!({"reserve_native_root":input}) };
    let participant = retained(db, context, &intent, || {
        if collaboration.is_some() && input.explicit_name.as_ref().is_none_or(|name| db.get_instance_full(name).map_or(true, |row| row.is_some())) {
            return Err(ServiceError::Conflict("dedicated product execution requires a new exact participant name".into()));
        }
        let mut effective = context.clone();
        effective.caller.invocation.cwd = input.cwd.clone();
        effective.caller.invocation.environment = input.environment.clone();
        let config = crate::config::AgentxConfig::load_for_scope(db.path().parent().unwrap_or(std::path::Path::new(".")), &input.environment);
        let timeout = config.timeout;
        let presets = config.auto_subscribe.as_str();
        let registered = register(
            db,
            &effective,
            &Registration {
                name: input.explicit_name.clone(),
                tool: "codex".into(),
                session_id: None,
                process_binding_id: None,
                transcript_path: None,
                wait_timeout: timeout,
                subagent_timeout: None,
                hints: Some(config.hints.clone()),
                auto_subscribe: presets.into(),
            },
        )?;
        let max = db.get_last_event_id();
        let cursor = input.initial_cursor.filter(|cursor| *cursor >= 0 && *cursor <= max).unwrap_or(registered.record.last_event_id);
        let mut launch = json!({"env":launch_environment(&input.environment),"native_home":input.canonical_home,"execution":input.execution,"owner":input.owner,"pid_identity":context.process.incarnation});
        if let Some(collaboration) = &collaboration {
            launch["collaboration"] = serde_json::to_value(collaboration).map_err(internal)?;
            launch["product_execution"] = json!(input.execution);
        }
        db.update_instance_fields(&registered.record.name, json!({"last_event_id":cursor,"launch_args":serde_json::to_string(&input.launch_args).map_err(internal)?,"launch_profile":input.launch_profile.as_ref().map(serde_json::to_string).transpose().map_err(internal)?.unwrap_or_default(),"launch_context":launch.to_string()}).as_object().unwrap()).map_err(internal)?;
        super::launch_relations::record(db, context, &registered.participant, true)?;
        Ok(registered.participant)
    })?;
    check_participant(db, &participant)?;
    Ok(participant)
}

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    match request {
        Request::Lifecycle(request) => super::lifecycle_store::handle(db, context, request).map(Response::Lifecycle),
        Request::Launch(request) => super::launch_store::handle(db, context, request).map(Response::Launch),
        Request::Labels(request) => super::labels_store::handle(db, context, request).map(Response::Labels),
        Request::Hermes(request) => super::hermes_store::handle(db, context, request).map(Response::Hermes),
        Request::Copilot(request) => super::copilot_store::handle(db, context, request).map(Response::Copilot),
        Request::Claude(request) => super::claude_store::handle(db, context, request).map(Response::Claude),
        Request::HookSession(input) => super::hook_store::observe_session(db, context, input).map(Response::Registered),
        Request::ObserveModelRequest(input) => super::model_request_store::observe(db, context, input),
        Request::Recovery(request) => super::recovery_store::handle(db, context, request).map(Response::Recovery),
        Request::ToolActivity(input) => super::hook_store::tool_activity(db, context, input).map(|()| Response::Changed),
        Request::ObserveHookPosition(input) => super::hook_store::observe_position(db, context, input).map(Response::Registered),
        Request::StopSession(input) => super::hook_store::stop_session(db, context, input).map(Response::SessionStopped),
        Request::BindLaunchedSession { participant, session_id } => {
            verified_process(context)?;
            let invocation = &context.caller.invocation;
            if session_id.is_empty() || invocation.environment.get("AGENTX_LAUNCHED").map(String::as_str) != Some("1") || invocation.activation_target.is_some() {
                return Err(ServiceError::InvalidInput("launched Codex process session required".into()));
            }
            let binding = invocation.process_binding_id.as_deref().filter(|value| !value.is_empty()).ok_or_else(|| ServiceError::IdentityRequired("launched process binding required".into()))?;
            if invocation.provider_session.as_ref().is_some_and(|session| session.provider != "codex" || session.session_id != session_id) {
                return Err(ServiceError::StaleFence);
            }
            let result = retained(db, context, &json!({"bind_launched_session":{"participant":participant,"session_id":session_id}}), || {
                let current = check_participant(db, &participant)?;
                if current.record.tool != "codex" || db.get_process_binding(binding).map_err(internal)?.as_deref() != Some(participant.canonical_name.as_str()) {
                    return Err(ServiceError::StaleFence);
                }
                if let Some(existing) = current.record.session_id.as_deref().filter(|value| !value.is_empty()) {
                    if existing != session_id {
                        return Err(ServiceError::StaleFence);
                    }
                    return Ok(Response::Registered(current));
                }
                let actor = super::invocation_binding::bind_session(db, context, &session_id, "codex", false)?.ok_or_else(|| ServiceError::IdentityRequired("no participant bound to this launched process".into()))?;
                Ok(Response::Registered(actor))
            })?;
            validate_registered(db, result)
        }
        Request::PluginStart(input) => {
            if input.session_id.is_empty() || input.tool.is_empty() {
                return Err(ServiceError::InvalidInput("plugin tool and session required".into()));
            }
            if context.caller.invocation.process_binding_id.as_deref().is_none_or(|binding| binding.is_empty()) {
                return Err(ServiceError::IdentityRequired("plugin process binding required".into()));
            }
            let mut effective = context.clone();
            if let Some(cwd) = &input.cwd {
                effective.caller.invocation.cwd = cwd.clone();
            }
            let result = retained(db, context, &json!({"plugin_start":input}), || {
                let actor = super::invocation_binding::bind_session(db, &effective, &input.session_id, &input.tool, input.recover_soft_stopped)?.ok_or_else(|| ServiceError::IdentityRequired("no participant bound to this plugin process".into()))?;
                let name = &actor.participant.canonical_name;
                let max = db.get_last_event_id();
                let cursor = if actor.record.last_event_id == 0 {
                    effective.caller.invocation.environment.get("AGENTX_LAUNCH_EVENT_ID").and_then(|value| value.parse::<i64>().ok()).filter(|cursor| *cursor >= 0 && *cursor <= max).unwrap_or(max)
                } else {
                    actor.record.last_event_id
                };
                let mut updates = json!({"tool":input.tool,"session_id":input.session_id,"directory":effective.caller.invocation.cwd.to_string_lossy(),"last_event_id":cursor,"status":"listening","status_context":"start","status_time":crate::shared::time::now_epoch_i64()});
                if let Some(path) = &input.transcript_path {
                    updates["transcript_path"] = json!(path);
                }
                db.update_instance_fields(name, updates.as_object().unwrap()).map_err(internal)?;
                super::invocation_binding::capture_launch_context(db, &effective, name)?;
                row(db, name).map(Response::Registered)
            })?;
            validate_registered(db, result)
        }
        Request::PluginStop { participant, reason, soft } => {
            if resolve_actor(db, context)?.participant != participant {
                return Err(ServiceError::StaleFence);
            }
            if !soft {
                return handle(db, context, Request::Stop { participant, reason });
            }
            let event = transaction(db, || {
                let actor = check_participant(db, &participant)?;
                let name = &participant.canonical_name;
                if actor.record.status == "inactive" && actor.record.status_context == format!("exit:{reason}") {
                    return Ok(None);
                }
                let snapshot = db.get_instance_snapshot(name).map_err(internal)?;
                db.set_status(name, "inactive", &format!("exit:{reason}")).map_err(internal)?;
                // ProcessContinues keeps subscriptions and process binding.
                if let Some(session) = actor.record.session_id {
                    db.delete_session_binding(&session).map_err(internal)?;
                }
                db.delete_notify_endpoints(name).map_err(internal)?;
                let data = json!({"action":"stopped","by":"session","reason":format!("exit:{reason}"),"snapshot":snapshot});
                let id = db.insert_event_row("life", name, &data, None).map_err(internal)?;
                Ok(Some((id, data)))
            })?;
            if let Some((id, data)) = event {
                db.after_event_logged(id, "life", &participant.canonical_name, &data);
            }
            Ok(Response::Stopped(true))
        }
        Request::Lookup { name } => Ok(Response::Participant(db.get_instance_full(&name).map_err(internal)?.map(actor).map(|actor| actor.participant))),
        Request::Resolve => {
            let actor = resolve_actor(db, context)?;
            if db.conn().is_autocommit() {
                crate::service::context::participant_configuration(&actor.record);
            }
            Ok(Response::Actor(actor))
        }
        Request::Record { name } => {
            let record = db.get_instance_full(&name).map_err(internal)?;
            if db.conn().is_autocommit()
                && let Some(record) = &record
            {
                crate::service::context::participant_configuration(record);
            }
            Ok(Response::Record(record))
        }
        Request::List => db.iter_instances_full().map(Response::Records).map_err(internal),
        Request::Register(input) => {
            let result = retained(db, context, &json!({"register":input}), || register(db, context, &input).map(Response::Registered))?;
            validate_registered(db, result)
        }
        Request::Status { participant, status, context: status_context, detail } => {
            transaction(db, || {
                if resolve_actor(db, context)?.participant != participant {
                    return Err(ServiceError::StaleFence);
                }
                let current = check_participant(db, &participant)?;
                // A callback already in flight when the operator begins a
                // kill cannot clear the marker needed to complete retirement.
                if current.record.status_context == "exit:killed" {
                    return Err(ServiceError::StaleFence);
                }
                let tool_name = status_context.strip_prefix("approved:").unwrap_or("");
                super::lifecycle::try_set_status(db, &participant.canonical_name, &status, &status_context, super::lifecycle::StatusUpdate { detail: &detail, tool_name, ..Default::default() }).map_err(internal)?;
                Ok(())
            })?;
            Ok(Response::Changed)
        }
        Request::Heartbeat { participant } => transaction(db, || {
            if resolve_actor(db, context)?.participant != participant {
                return Err(ServiceError::StaleFence);
            }
            check_participant(db, &participant)?;
            db.update_heartbeat(&participant.canonical_name).map_err(internal)?;
            Ok(Response::Changed)
        }),
        Request::Stop { participant, reason } => transaction(db, || {
            let current = check_participant(db, &participant)?;
            if current.record.status_context == "exit:killed" {
                if resolve_actor(db, context)?.participant != participant {
                    return Err(ServiceError::StaleFence);
                }
                return super::hook_store::stop_killed(db, context, &participant).map(|outcome| Response::Stopped(outcome.stopped));
            }
            let snapshot = db.get_instance(&participant.canonical_name).map_err(internal)?;
            db.finalize_instance_stop(&participant.canonical_name, current.record.created_at, current.record.session_id.as_deref(), current.record.agent_id.as_deref(), &json!({"action":"stopped","reason":reason,"snapshot":snapshot}))
                .map(Response::Stopped)
                .map_err(internal)
        }),
        Request::Attachments(operation) => attachments(db, context, operation).map(Response::Attachment),
    }
}

fn attachments(db: &AgentxDb, context: &RequestContext, operation: AttachmentOperation) -> ServiceResult<AttachmentOutcome> {
    let mut store = SqliteAttachmentStore::for_owner_process(db, AttachmentOwnerProcess { pid: context.process.pid, process_identity: context.process.incarnation.clone() });
    Ok(match operation {
        AttachmentOperation::Participant { name } => AttachmentOutcome::Participant(store.participant(&name)),
        AttachmentOperation::CanonicalParticipant { session } => AttachmentOutcome::CanonicalParticipant(store.canonical_participant(&session)),
        AttachmentOperation::Attach(request) => {
            verified_process(context)?;
            let outcome = retained(db, context, &json!({"attach":request}), || match store.attach(request) {
                Err(AttachmentFailure::StoreUnavailable) => Err(ServiceError::Unavailable("attachment store unavailable".into())),
                result => Ok(AttachmentOutcome::Attached(result)),
            })?;
            if let AttachmentOutcome::Attached(Ok(attachment)) = &outcome
                && !store.validate(&attachment.target).map_err(internal)?
            {
                return Err(ServiceError::StaleFence);
            }
            return Ok(outcome);
        }
        AttachmentOperation::Lookup { execution, session } => AttachmentOutcome::Attachment(store.lookup(&execution, &session)),
        AttachmentOperation::Current { participant } => AttachmentOutcome::Attachment(store.current_for_participant(&participant)),
        AttachmentOperation::Owned { owner } => AttachmentOutcome::Attachments(store.owned(&owner)),
        AttachmentOperation::Validate { target } => AttachmentOutcome::Valid(store.validate(&target)),
        AttachmentOperation::Retire { target } => {
            let current = store.current_for_participant(&target.participant).map_err(internal)?;
            if let Some(current) = current
                && !attachment_owned_by_process(db, &target, &current.owner, &context.process)?
            {
                return Err(ServiceError::Forbidden("attachment belongs to another process".into()));
            }
            AttachmentOutcome::Done(store.retire(&target))
        }
        AttachmentOperation::RetireOwner { owner } => {
            for attachment in store.owned(&owner).map_err(internal)? {
                if !attachment_owned_by_process(db, &attachment.target, &owner, &context.process)? {
                    return Err(ServiceError::Forbidden("attachment belongs to another process".into()));
                }
            }
            AttachmentOutcome::Done(store.retire_owner(&owner))
        }
    })
}

fn validate_registered(db: &AgentxDb, result: Response) -> ServiceResult<Response> {
    if let Response::Registered(actor) = &result {
        check_participant(db, &actor.participant)?;
    }
    Ok(result)
}
