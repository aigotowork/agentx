//! Daemon-owned participant launch policy extracted from runtime::launcher.
//! A retained reservation binds every mutation to its originating peer and row.
use super::attachments::ParticipantRef;
use super::binding::{self, BindingContext};
use super::launch_service::*;
use crate::db::AgentxDb;
use crate::service::{RequestContext, ServiceError, ServiceResult};
use serde_json::json;

#[cfg(test)]
mod acceptance_tests;

#[derive(serde::Serialize, serde::Deserialize)]
struct Owner {
    reservation: Reservation,
    pid: u32,
    incarnation: String,
}
fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn key(token: &str) -> String {
    format!("participant_launch:{token}")
}
fn peer(context: &RequestContext) -> ServiceResult<&str> {
    if context.process.pid == 0 {
        return Err(ServiceError::Forbidden("launch caller PID unavailable".into()));
    }
    context.process.incarnation.as_deref().filter(|id| !id.is_empty()).ok_or_else(|| ServiceError::Forbidden("launch caller incarnation unavailable".into()))
}
fn owner(db: &AgentxDb, context: &RequestContext, reservation: &Reservation) -> ServiceResult<()> {
    let raw = db.kv_get(&key(&reservation.token)).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    let stored: Owner = serde_json::from_str(&raw).map_err(internal)?;
    if stored.reservation != *reservation || stored.pid != context.process.pid || stored.incarnation != peer(context)? {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}
fn current(db: &AgentxDb, context: &RequestContext, reservation: &Reservation) -> ServiceResult<crate::db::InstanceRow> {
    owner(db, context, reservation)?;
    let row = db.get_instance_full(&reservation.participant.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    if row.created_at.to_bits() != reservation.participant.incarnation_bits || db.get_process_binding(&reservation.process_binding).map_err(internal)?.as_deref() != Some(row.name.as_str()) {
        return Err(ServiceError::StaleFence);
    }
    Ok(row)
}
/// Caller owns the outer write scope and its complete retained intent.
pub(super) fn reserve(db: &AgentxDb, context: &RequestContext, input: Reserve) -> ServiceResult<Response> {
    reserve_with_creation(db, context, input, super::launch_relations::Creation::New)
}
/// Lifecycle supplies the frozen original creation before any new fact is published.
pub(super) fn reserve_with_creation(db: &AgentxDb, context: &RequestContext, input: Reserve, creation: super::launch_relations::Creation) -> ServiceResult<Response> {
    let incarnation = peer(context)?;
    super::labels::validate_set(&input.labels)?;
    let title = super::launch_title::normalize(input.title.as_deref()).map_err(ServiceError::InvalidInput)?;
    if input.tool.parse::<crate::integrations::tool::Tool>().is_err() || input.process_binding.is_empty() {
        return Err(ServiceError::InvalidInput("launch tool and process binding required".into()));
    }
    if let Some(snapshot) = &input.collaboration {
        super::workspaces::validate(snapshot)?;
        if snapshot.endpoint {
            let role = snapshot.role.as_ref().expect("validated endpoint role");
            if let Some(existing) = super::workspaces::endpoint_lookup(db, snapshot.workspace.as_deref(), &role.path.to_string_lossy())? {
                return Err(ServiceError::Conflict(format!("workspace endpoint reservation conflict: already reserved by '{}'", existing.canonical_name)));
            }
        }
    }
    if db.get_process_binding(&input.process_binding).map_err(internal)?.is_some() {
        return Err(ServiceError::Conflict("launch process binding already exists".into()));
    }
    let name = if let Some(name) = input.name {
        if !super::identity::is_valid_name(&name) {
            return Err(ServiceError::InvalidInput("launch name required".into()));
        }
        if let Some(row) = db.get_instance_full(&name).map_err(internal)? {
            if row.origin_device_id.as_deref().is_some_and(|id| !id.is_empty()) {
                return Err(ServiceError::Conflict("remote participant launch requires owning device".into()));
            }
            super::workspaces::protect_replacement(db, &row)?;
            if row.status == "inactive" {
                db.delete_instance(&name).map_err(internal)?;
            } else if !(row.status == super::names::PLACEHOLDER_STATUS && row.status_context == super::names::PLACEHOLDER_CONTEXT && row.session_id.as_deref().is_none_or(str::is_empty)) {
                return Err(ServiceError::Conflict(format!("Instance '{name}' already exists (stop it first or use a different name)")));
            }
        }
        name
    } else {
        super::names::allocate_unreserved_name(db).map_err(internal)?
    };
    let launch_cursor = db.get_last_event_id();
    let initial_cursor = input.resume_cursor.unwrap_or(launch_cursor);
    if !(0..=launch_cursor).contains(&initial_cursor) {
        return Err(ServiceError::InvalidInput("launch cursor exceeds database watermark".into()));
    }
    let mut invocation = context.caller.invocation.clone();
    // Keep the historical display-only event label aligned with the original
    // caller. Exact provenance is recorded separately below, never from this name.
    let creator_label = super::service_store::resolve_actor(db, context).ok().map(|actor| actor.participant.canonical_name).unwrap_or_else(|| "user".into());
    invocation.environment.insert("AGENTX_LAUNCHED_BY".into(), creator_label);
    invocation.environment.insert("AGENTX_LAUNCHED".into(), "1".into());
    invocation.environment.insert("AGENTX_LAUNCH_EVENT_ID".into(), initial_cursor.to_string());
    let effective = BindingContext::explicit(db, &invocation);
    if !binding::initialize_instance_with_context(db, &effective, &name, input.prior_session.as_deref(), None, None, None, None, Some(&input.tool), input.background, None, None, None, input.directory.to_str()) {
        return Err(ServiceError::Internal(format!("Failed to initialize instance '{name}'")));
    }
    super::invocation_binding::capture_launch_context(db, context, &name)?;
    // The resolved launch snapshot, never the caller's ambient environment,
    // owns membership. Persist it before bootstrap can observe this row.
    let row = db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    let mut launch_context: serde_json::Map<String, serde_json::Value> = row.launch_context.as_deref().map(serde_json::from_str).transpose().map_err(internal)?.unwrap_or_default();
    launch_context.insert("title".into(), serde_json::to_value(title).map_err(internal)?);
    launch_context.remove("collaboration");
    if let Some(snapshot) = &input.collaboration {
        launch_context.insert("collaboration".into(), serde_json::to_value(snapshot).map_err(internal)?);
    }
    db.update_instance_fields(&name, json!({"launch_context":serde_json::to_string(&launch_context).map_err(internal)?}).as_object().unwrap()).map_err(internal)?;
    db.update_instance_fields(&name, json!({"last_event_id":initial_cursor,"launch_args":input.launch_args,"launch_profile":input.launch_profile.as_ref().map(serde_json::to_string).transpose().map_err(internal)?.unwrap_or_default(),"name_announced":matches!(input.tool.as_str(),"codex"|"grok")}).as_object().unwrap()).map_err(internal)?;
    if input.resume_cursor.is_some() {
        crate::db::subscriptions::restore_stopped_subscriptions(db, &name).map_err(internal)?;
    }
    db.set_process_binding(&input.process_binding, "", &name).map_err(internal)?;
    let row = db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    let reservation = Reservation { token: uuid::Uuid::new_v4().to_string(), participant: ParticipantRef { canonical_name: name, incarnation_bits: row.created_at.to_bits() }, process_binding: input.process_binding, launch_cursor, initial_cursor };
    super::launch_relations::apply(db, context, &reservation.participant, creation)?;
    super::labels_store::initialize(db, context, &reservation.participant, &input.labels)?;
    if let Some(snapshot) = &input.collaboration
        && snapshot.endpoint
    {
        let role = snapshot.role.as_ref().expect("validated endpoint role");
        super::workspaces::bind_endpoint(db, snapshot.workspace.as_deref(), &role.path.to_string_lossy(), &reservation.participant)?;
    }
    db.kv_set(&key(&reservation.token), Some(&serde_json::to_string(&Owner { reservation: reservation.clone(), pid: context.process.pid, incarnation: incarnation.to_owned() }).map_err(internal)?)).map_err(internal)?;
    Ok(Response::Reserved(reservation))
}

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    // Capability discovery has no state or transaction effects. Older native
    // request decoders reject this variant instead of ignoring collaboration.
    if matches!(&request, Request::WorkspaceSupport | Request::CreationSupport | Request::TitleSupport | Request::LabelsSupport | Request::EndpointInspect { .. }) {
        return dispatch(db, context, request);
    }
    db.with_write_scope(|| dispatch(db, context, request).map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}
fn dispatch(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    match request {
        Request::WorkspaceSupport | Request::CreationSupport | Request::TitleSupport | Request::LabelsSupport => Ok(Response::Changed),
        Request::EndpointInspect { workspace, role_path } => {
            peer(context)?;
            Ok(Response::EndpointInspected(super::workspaces::endpoint_inspect(db, workspace.as_deref(), &role_path)?))
        }
        Request::EndpointLookup { workspace, role_path } => {
            peer(context)?;
            Ok(Response::Endpoint(super::workspaces::endpoint_lookup(db, workspace.as_deref(), &role_path)?))
        }
        Request::EndpointReset { workspace } => {
            peer(context)?;
            super::service_store::retained(db, context, &json!({"endpoint_reset":workspace}), || {
                super::workspaces::endpoint_reset(db, workspace.as_deref())?;
                Ok(Response::Changed)
            })
        }
        Request::Reserve(input) => {
            let intent = json!({"launch_reserve":input});
            let response = super::service_store::retained(db, context, &intent, || reserve(db, context, input))?;
            if let Response::Reserved(reservation) = &response {
                current(db, context, reservation)?;
            }
            Ok(response)
        }
        Request::TerminalLaunched { reservation, requested, effective, background } => {
            let row = current(db, context, &reservation)?;
            let name = &row.name;
            if let Some(background) = &background {
                let child = &background.child;
                // An unavailable observation is not an identity. The host keeps
                // its child alive until this observation is durably retained.
                if child.pid == 0 || child.incarnation.is_none() || crate::sys::process::identity(child.pid) != child.incarnation {
                    return Err(ServiceError::StaleFence);
                }
                // A PTY may already have registered its actual child while the
                // launcher was receiving the runner PID. Never replace it.
                if row.pid.is_none() || row.pid == Some(i64::from(child.pid)) {
                    db.update_instance_pid_with_identity(name, child.pid, child.incarnation.as_deref()).map_err(internal)?;
                }
                db.update_instance_fields(name, json!({"background_log_file":background.log_file}).as_object().unwrap()).map_err(internal)?;
            }
            binding::try_persist_terminal_launch_context(db, name, requested.as_deref(), &effective, Some(&reservation.process_binding)).map_err(internal)?;
            Ok(Response::Changed)
        }
        Request::RecordSupervisor(reservation) => {
            let row = current(db, context, &reservation)?;
            db.update_instance_pid_with_identity(&row.name, context.process.pid, Some(peer(context)?)).map_err(internal)?;
            Ok(Response::Changed)
        }
        Request::RefreshSupervisor { reservation, registration } => {
            owner(db, context, &reservation)?;
            crate::runtime::hosts_store::validate_process_registration(db, context, &registration)?;
            if registration.child.pid != context.process.pid || registration.child.incarnation.as_deref() != Some(peer(context)?) || context.caller.invocation.process_binding_id.as_deref() != Some(reservation.process_binding.as_str()) {
                return Err(ServiceError::StaleFence);
            }
            let refreshed = Reservation { token: uuid::Uuid::new_v4().to_string(), participant: registration.participant, ..reservation };
            db.kv_set(&key(&refreshed.token), Some(&serde_json::to_string(&Owner { reservation: refreshed.clone(), pid: context.process.pid, incarnation: peer(context)?.to_owned() }).map_err(internal)?)).map_err(internal)?;
            Ok(Response::Reserved(refreshed))
        }
        Request::FinishSupervisor(reservation) => {
            owner(db, context, &reservation)?;
            // A switched canonical identity needs an explicit refreshed lease.
            let name = reservation.participant.canonical_name.clone();
            if let Some(row) = db.get_instance_full(&name).map_err(internal)? {
                if row.created_at.to_bits() != reservation.participant.incarnation_bits
                    || db.get_process_binding(&reservation.process_binding).map_err(internal)?.as_deref() != Some(name.as_str())
                    || row.pid != Some(i64::from(context.process.pid))
                    || db.get_instance_pid_identity(&name).map_err(internal)?.as_deref() != Some(peer(context)?)
                {
                    return Err(ServiceError::StaleFence);
                }
                if row.status_context == "exit:killed" {
                    super::service::stop_killed(db, context, &reservation.participant)?;
                } else {
                    let stopped = json!({"action":"stopped","by":"stdio","reason":"closed","snapshot":db.get_instance_snapshot(&name).map_err(internal)?});
                    if !db.finalize_instance_stop(&name, row.created_at, row.session_id.as_deref(), row.agent_id.as_deref(), &stopped).map_err(internal)? {
                        return Err(ServiceError::StaleFence);
                    }
                }
            }
            db.conn().execute("DELETE FROM process_bindings WHERE process_id=? AND instance_name=?", rusqlite::params![reservation.process_binding, name]).map_err(internal)?;
            Ok(Response::Finished { name })
        }
        Request::CancelFailed(reservation) => {
            owner(db, context, &reservation)?;
            super::launch_relations::failed(db, &reservation.participant)?;
            if db.get_instance_full(&reservation.participant.canonical_name).map_err(internal)?.is_some() {
                let row = current(db, context, &reservation)?;
                if super::workspaces::is_bound_endpoint(db, &row)? {
                    // Failure to spawn/observe is not proof of retirement. Keep
                    // the endpoint visible for diagnosis and explicit cleanup.
                    super::lifecycle::try_set_status(db, &row.name, "inactive", "launch_failed", super::lifecycle::StatusUpdate { detail: "endpoint launch cancelled; confirm retirement before replacement", ..Default::default() })
                        .map_err(internal)?;
                } else {
                    db.delete_instance(&row.name).map_err(internal)?;
                }
            }
            db.conn().execute("DELETE FROM process_bindings WHERE process_id=? AND instance_name=?", rusqlite::params![reservation.process_binding, reservation.participant.canonical_name]).map_err(internal)?;
            db.kv_set(&key(&reservation.token), None).map_err(internal)?;
            Ok(Response::Changed)
        }
        Request::SpawnFailed { participant, launch_cursor, detail } => {
            peer(context)?;
            let binding = context.caller.invocation.process_binding_id.as_deref().filter(|value| !value.is_empty()).ok_or_else(|| ServiceError::IdentityRequired("launch process binding required".into()))?;
            let row = db.get_instance_full(&participant.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
            if row.created_at.to_bits() != participant.incarnation_bits || db.get_process_binding(binding).map_err(internal)?.as_deref() != Some(row.name.as_str()) || row.session_id.as_deref().is_some_and(|session| !session.is_empty()) {
                return Err(ServiceError::StaleFence);
            }
            // A runner may already be recorded by its launcher. A registered
            // different child belongs to its ProcessHost, never this callback.
            if let Some(pid) = row.pid
                && (pid != i64::from(context.process.pid) || db.get_instance_pid_identity(&row.name).map_err(internal)?.as_deref() != Some(peer(context)?))
            {
                return Err(ServiceError::StaleFence);
            }
            if !(0..=db.get_last_event_id()).contains(&launch_cursor) || detail.is_empty() || detail.len() > 64 * 1024 {
                return Err(ServiceError::InvalidInput("invalid launch failure observation".into()));
            }
            let killed: bool = db
                .conn()
                .query_row("SELECT EXISTS(SELECT 1 FROM events WHERE type='life' AND instance=?1 AND id>?2 AND json_extract(data,'$.action')='stopped' AND json_extract(data,'$.reason')='killed')", rusqlite::params![row.name, launch_cursor], |row| {
                    row.get(0)
                })
                .map_err(internal)?;
            let key = format!("spawn_failure:{}:{}:{launch_cursor}", row.name, participant.incarnation_bits);
            if row.status_context == "exit:killed" || killed || db.kv_get(&key).map_err(internal)?.is_some() {
                return Ok(Response::Changed);
            }
            super::lifecycle::try_set_status(db, &row.name, "inactive", "launch_failed", super::lifecycle::StatusUpdate { detail: &detail, ..Default::default() }).map_err(internal)?;
            db.emit_launch_failed_event(&row.name, "inactive", "launch_failed", "spawn_failed", &detail).map_err(internal)?;
            db.kv_set(&key, Some("observed")).map_err(internal)?;
            Ok(Response::Changed)
        }
        Request::BatchLaunched(batch) => {
            peer(context)?;
            db.log_event("life",&batch.launcher,&json!({"action":"batch_launched","by":batch.launcher,"batch_id":batch.batch_id,"tool":batch.tool,"count_requested":batch.requested,"launched":batch.launched,"failed":batch.failed,"background":batch.background,"instances":batch.instances})).map_err(internal)?;
            Ok(Response::Changed)
        }
    }
}
