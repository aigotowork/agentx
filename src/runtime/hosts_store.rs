//! Daemon-only host registration implementation. It borrows the owning daemon's
//! connection and never opens SQLite or performs Provider/terminal I/O.

use crate::agents::attachments::{AttachmentStore, SqliteAttachmentStore};
use crate::agents::lifecycle;
use crate::db::AgentxDb;
use crate::service::{RequestContext, ServiceError, ServiceResult};
use crate::shared::{ST_ACTIVE, ST_INACTIVE, ST_LISTENING};

use super::hosts::{HostRegistration, LaunchWaitRegistration, NativeRootState, Request, Response};

#[cfg(test)]
mod acceptance_tests;

pub(crate) use super::process_host_store::validate as validate_process_registration;
pub(crate) use super::process_host_store::validate_exit as validate_process_exit_registration;
pub(crate) use super::process_host_store::{checkpoint_delivery_poll, delivery_progress, observe_delivery_poll, poll_registration_token};

#[derive(serde::Serialize, serde::Deserialize)]
struct RegisteredHost {
    registration: HostRegistration,
    pid: u32,
    incarnation: String,
    #[serde(default)]
    direct_terminal: bool,
}

fn key(name: &str) -> String {
    format!("runtime_host:{name}")
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

/// Dispatch durable host effects, delegating participant invariants to their
/// domain while retaining one daemon-owned database connection.
pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    match request {
        Request::DirectTerminalSupport => Ok(Response::Renewed),
        Request::WakeHook { participant } => transaction(db, || {
            if crate::agents::service::resolve_actor(db, context)?.participant != participant {
                return Err(ServiceError::StaleFence);
            }
            db.wake_after_commit(&participant.canonical_name, &[super::notify::WakeKind::Hook]);
            Ok(Response::Renewed)
        }),
        request @ (Request::ReportClaudeMod { .. } | Request::BeginClaudeModPoll { .. } | Request::FinishClaudeModPoll { .. }) => transaction(db, || crate::integrations::hooks::claude::native::handle_host(db, context, request)),
        Request::SetCopilotEndpoint { participant, session_id, port } => transaction(db, || {
            crate::agents::copilot_service::validate_endpoint(db, context, &participant, &session_id)?;
            if port == 0 {
                return Err(ServiceError::InvalidInput("Copilot notify port must be nonzero".into()));
            }
            db.upsert_notify_endpoint(&participant.canonical_name, "plugin", port).map_err(internal)?;
            Ok(Response::Renewed)
        }),
        Request::SetPluginEndpoint { participant, port, standby } => transaction(db, || {
            let actor = crate::agents::service::resolve_actor(db, context)?;
            if actor.participant != participant || context.caller.invocation.process_binding_id.is_none() {
                return Err(ServiceError::StaleFence);
            }
            if port == 0 {
                return Err(ServiceError::InvalidInput("plugin endpoint port must be nonzero".into()));
            }
            let name = &participant.canonical_name;
            db.upsert_notify_endpoint(name, if standby { "session_start" } else { "plugin" }, port).map_err(internal)?;
            if !standby {
                db.delete_notify_endpoint(name, "session_start").map_err(internal)?;
                db.wake_after_commit(name, super::notify::WakeKind::DELIVERY_LOOPS);
            }
            Ok(Response::Renewed)
        }),
        Request::BootstrapSnapshot { name } => Ok(Response::BootstrapSnapshot(super::bootstrap::snapshot(db, &name))),
        Request::ObserveClaudeModHealth { registration } => transaction(db, || {
            validate_process_registration(db, context, &registration)?;
            let binding = context.caller.invocation.process_binding_id.as_deref().ok_or_else(|| ServiceError::IdentityRequired("native mod host has no process binding".into()))?;
            crate::integrations::hooks::claude::native::report_health(db, binding, &registration.participant.canonical_name).map_err(internal)?;
            Ok(Response::Renewed)
        }),
        request @ (Request::RegisterProcess { .. } | Request::CaptureProcessContext { .. } | Request::SetProcessEndpoint { .. } | Request::ReleaseProcessEndpoint { .. } | Request::ObserveApproval { .. }) => {
            let result = transaction(db, || super::process_host_store::handle(db, context, request));
            if let Ok(Response::ProcessRegistered { registration, .. }) = &result {
                use crate::observability::runtime::{HostAssociationState, HostObservationKind, ObservedProcess};
                crate::service::context::host_validated(
                    HostObservationKind::ProcessHost,
                    registration.participant.clone(),
                    None,
                    Some(ObservedProcess { pid: registration.child.pid, incarnation: registration.child.incarnation.clone() }),
                    HostAssociationState::Registered,
                );
                crate::service::context::host_configuration(&context.caller.invocation, registration.participant.clone(), None);
            }
            result
        }
        Request::ObserveLaunch { launcher, batch_id, timeout_secs } => transaction(db, || {
            let cleanup = crate::agents::service::cleanup_system(db, context, crate::agents::service::SystemCleanup::LaunchPlaceholders)?;
            let result = super::launch_status::observe_launch(db, launcher.as_deref(), batch_id.as_deref(), timeout_secs)?;
            let diagnostics = super::launch_status::diagnostic_records(db, &result, &cleanup)?;
            Ok(Response::LaunchStatus { result, cleanup, diagnostics })
        }),
        Request::RegisterLaunchWaiter { batch_prefix, port } => {
            let incarnation = verified_host(context)?;
            if port == 0 {
                return Err(ServiceError::InvalidInput("launch waiter port must be nonzero".into()));
            }
            let scope = batch_prefix.as_deref().filter(|id| !id.contains(['%', '_'])).unwrap_or("");
            let owner = serde_json::json!({
                "token": uuid::Uuid::new_v4().to_string(),
                "pid": context.process.pid,
                "process_identity": incarnation,
                "batch_prefix": scope,
            });
            let registration = LaunchWaitRegistration { name: format!("launch-wait:{owner}"), port };
            db.upsert_notify_endpoint(&registration.name, "launch_wait", port).map_err(internal)?;
            Ok(Response::LaunchWaiter(registration))
        }
        Request::ReleaseLaunchWaiter(registration) => {
            let incarnation = verified_host(context)?;
            let owner: serde_json::Value = registration.name.strip_prefix("launch-wait:").and_then(|name| serde_json::from_str(name).ok()).ok_or(ServiceError::StaleFence)?;
            if owner.get("pid").and_then(serde_json::Value::as_u64) != Some(u64::from(context.process.pid)) || owner.get("process_identity").and_then(serde_json::Value::as_str) != Some(incarnation) {
                return Err(ServiceError::StaleFence);
            }
            db.conn().execute("DELETE FROM notify_endpoints WHERE instance = ?1 AND kind = 'launch_wait' AND port = ?2", rusqlite::params![registration.name, registration.port]).map_err(internal)?;
            Ok(Response::Released)
        }
        Request::ReserveNativeRoot(input) => crate::agents::service::reserve_native_root(db, context, input).map(Response::Reserved),
        Request::ReserveProductRoot { root, role, workspace } => crate::agents::service::reserve_product_root(db, context, root, role, workspace).map(Response::Reserved),
        Request::RegisterNativeRoot { target, owner, wake_port, state } => {
            if wake_port == 0 {
                return Err(ServiceError::InvalidInput("native root wake port must be nonzero".into()));
            }
            let incarnation = verified_host(context)?;
            let result = transaction(db, || {
                let current = SqliteAttachmentStore::new(db).current_for_participant(&target.participant).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
                if current.target != target || current.owner != owner {
                    return Err(ServiceError::StaleFence);
                }
                if !crate::agents::service::attachment_owned_by_process(db, &target, &owner, &context.process)? {
                    return Err(ServiceError::StaleFence);
                }
                if db.get_instance_full(&target.participant.canonical_name).map_err(internal)?.is_none_or(|row| row.status_context == "exit:killed") {
                    return Err(ServiceError::StaleFence);
                }
                let stored = load(db, &target.participant.canonical_name)?;
                let (registration, direct_terminal) = match stored {
                    Some(stored) if stored.pid == context.process.pid && stored.incarnation == incarnation && stored.registration.target == target && stored.registration.owner == owner && stored.registration.wake_port == wake_port => {
                        (stored.registration, stored.direct_terminal)
                    }
                    _ => (HostRegistration { token: uuid::Uuid::new_v4().to_string(), target, owner, wake_port }, false),
                };
                let name = &registration.target.participant.canonical_name;
                let stored = RegisteredHost { registration: registration.clone(), pid: context.process.pid, incarnation: incarnation.to_owned(), direct_terminal };
                db.kv_set(&key(name), Some(&serde_json::to_string(&stored).map_err(internal)?)).map_err(internal)?;
                db.update_instance_pid_with_identity(name, context.process.pid, Some(incarnation)).map_err(internal)?;
                if let Some(launch_context) = context.caller.invocation.launch_context.as_ref() {
                    db.store_launch_context(name, &launch_context.to_string()).map_err(internal)?;
                }
                db.update_instance_fields(name, &serde_json::Map::from_iter([("name_announced".to_owned(), serde_json::json!(false))])).map_err(internal)?;
                db.upsert_notify_endpoint(name, "hook", wake_port).map_err(internal)?;
                db.update_heartbeat(name).map_err(internal)?;
                lifecycle::try_set_status(db, name, if state == NativeRootState::Idle { ST_LISTENING } else { ST_ACTIVE }, "native:attached", Default::default()).map_err(internal)?;
                Ok(Response::Registered(registration))
            });
            if let Ok(Response::Registered(registration)) = &result {
                crate::service::context::host_validated(
                    crate::observability::runtime::HostObservationKind::NativeRoot,
                    registration.target.participant.clone(),
                    Some(registration.target.clone()),
                    None,
                    crate::observability::runtime::HostAssociationState::Registered,
                );
                crate::service::context::host_configuration(&context.caller.invocation, registration.target.participant.clone(), Some(registration.target.execution.clone()));
            }
            result
        }
        Request::NativeRootReady(registration) => {
            let recorded = transaction(db, || {
                validate_registration(db, context, &registration, true)?;
                let participant = &registration.target.participant;
                let name = &participant.canonical_name;
                let attachment = SqliteAttachmentStore::new(db).current_for_participant(participant).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
                let row = db.get_instance_full(name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
                if row.tool != "codex" || attachment.session.provider != "codex" || row.status_context == "exit:killed" {
                    return Err(ServiceError::StaleFence);
                }
                match row.session_id.as_deref().filter(|session| !session.is_empty()) {
                    None => db.update_instance_fields(name, &serde_json::Map::from_iter([("session_id".into(), serde_json::json!(attachment.session.session_id))])).map_err(internal)?,
                    Some(session) if session == attachment.session.session_id => {}
                    _ => return Err(ServiceError::StaleFence),
                }
                let mut stored = load(db, name)?.ok_or(ServiceError::StaleFence)?;
                stored.direct_terminal = true;
                db.kv_set(&key(name), Some(&serde_json::to_string(&stored).map_err(internal)?)).map_err(internal)?;
                super::launch_status::record_native_ready(db, &registration.target.participant, &context.caller.invocation.environment)
            })?;
            if let Some((launcher, batch)) = recorded
                && let Err(error) = db.check_batch_completion(&launcher, &batch)
            {
                crate::log::log_error("launch", "batch_completion", &error.to_string());
            }
            Ok(Response::Renewed)
        }
        Request::NativeRecovery { registration, action } => transaction(db, || {
            validate_registration(db, context, &registration, true)?;
            let participant = &registration.target.participant;
            let row = db.get_instance_full(&participant.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
            let session = direct_recovery_session(db, &row, participant)?.ok_or(ServiceError::StaleFence)?;
            crate::agents::recovery_store::native_host(db, &row, &session, action).map(Response::NativeRecovery)
        }),
        Request::RenewNativeRoot(registration) => {
            let result = transaction(db, || {
                validate_registration(db, context, &registration, true)?;
                let name = &registration.target.participant.canonical_name;
                db.upsert_notify_endpoint(name, "hook", registration.wake_port).map_err(internal)?;
                db.update_heartbeat(name).map_err(internal)?;
                Ok(Response::Renewed)
            });
            if result.is_ok() {
                crate::service::context::host_validated(
                    crate::observability::runtime::HostObservationKind::NativeRoot,
                    registration.target.participant.clone(),
                    Some(registration.target.clone()),
                    None,
                    crate::observability::runtime::HostAssociationState::Renewed,
                );
            }
            result
        }
        Request::ReleaseNativeRoot(registration) => {
            let mut validated = false;
            let result = transaction(db, || {
                // Repeating the exact release is harmless. A changed token/row or
                // attachment must never remove a successor's endpoint or status.
                let name = &registration.target.participant.canonical_name;
                if load(db, name)?.is_none() {
                    return Ok(Response::Released);
                }
                validate_registration(db, context, &registration, false)?;
                if db.get_notify_endpoint_port(name, "hook") == Some(registration.wake_port) {
                    db.delete_notify_endpoint(name, "hook").map_err(internal)?;
                }
                db.clear_instance_pid(name).map_err(internal)?;
                db.update_tcp_mode(name, false).map_err(internal)?;
                lifecycle::try_set_status(db, name, ST_INACTIVE, "native:disconnected", Default::default()).map_err(internal)?;
                db.kv_set(&key(name), None).map_err(internal)?;
                validated = true;
                Ok(Response::Released)
            });
            if result.is_ok() && validated {
                crate::service::context::host_validated(
                    crate::observability::runtime::HostObservationKind::NativeRoot,
                    registration.target.participant.clone(),
                    Some(registration.target.clone()),
                    None,
                    crate::observability::runtime::HostAssociationState::Released,
                );
            }
            result
        }
    }
}

fn verified_host(context: &RequestContext) -> ServiceResult<&str> {
    if context.process.pid == 0 {
        return Err(ServiceError::Forbidden("runtime host has no authenticated process".into()));
    }
    context.process.incarnation.as_deref().filter(|identity| !identity.is_empty()).ok_or_else(|| ServiceError::Forbidden("runtime host process incarnation is unavailable".into()))
}

fn load(db: &AgentxDb, name: &str) -> ServiceResult<Option<RegisteredHost>> {
    db.kv_get(&key(name)).map_err(internal)?.map(|value| serde_json::from_str(&value).map_err(internal)).transpose()
}

pub(crate) fn direct_recovery_session(db: &AgentxDb, row: &crate::db::InstanceRow, participant: &crate::agents::attachments::ParticipantRef) -> ServiceResult<Option<String>> {
    let Some(stored) = load(db, &row.name)? else { return Ok(None) };
    if !stored.direct_terminal
        || stored.registration.target.participant != *participant
        || row.pid != Some(i64::from(stored.pid))
        || db.get_instance_pid_identity(&row.name).map_err(internal)?.as_deref() != Some(&stored.incarnation)
        || db.get_notify_endpoint_port(&row.name, "hook") != Some(stored.registration.wake_port)
        || crate::sys::process::has_exited(stored.pid)
        || crate::sys::process::identity(stored.pid).as_deref() != Some(&stored.incarnation)
    {
        return Ok(None);
    }
    let current = SqliteAttachmentStore::new(db).current_for_participant(participant).map_err(internal)?;
    Ok(current
        .filter(|attachment| attachment.target == stored.registration.target && attachment.owner == stored.registration.owner && attachment.session.provider == "codex" && row.session_id.as_deref() == Some(&attachment.session.session_id))
        .map(|attachment| attachment.session.session_id))
}

fn validate_registration(db: &AgentxDb, context: &RequestContext, registration: &HostRegistration, require_attachment: bool) -> ServiceResult<()> {
    let incarnation = verified_host(context)?;
    let name = &registration.target.participant.canonical_name;
    let stored = load(db, name)?.ok_or(ServiceError::StaleFence)?;
    if stored.registration != *registration || stored.pid != context.process.pid || stored.incarnation != incarnation {
        return Err(ServiceError::StaleFence);
    }
    let row = db.get_instance_full(name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    if row.created_at.to_bits() != registration.target.participant.incarnation_bits || row.pid != Some(stored.pid as i64) || db.get_instance_pid_identity(name).map_err(internal)?.as_deref() != Some(incarnation) {
        return Err(ServiceError::StaleFence);
    }
    let current = SqliteAttachmentStore::new(db).current_for_participant(&registration.target.participant).map_err(internal)?;
    match current {
        Some(current) if current.target == registration.target && current.owner == registration.owner => {
            if crate::agents::service::attachment_owned_by_process(db, &registration.target, &registration.owner, &context.process)? {
                Ok(())
            } else {
                Err(ServiceError::StaleFence)
            }
        }
        None if !require_attachment => Ok(()),
        _ => Err(ServiceError::StaleFence),
    }
}
