//! Daemon-only Claude generation, actor and hook lifecycle policy. All host
//! I/O occurs before native observations or after committed frozen outcomes.
mod identity;
mod output;
mod stop;
#[cfg(test)]
mod tests;

use super::attachments::{AttachmentStore, ParticipantRef, SqliteAttachmentStore};
use super::claude_service::*;
use crate::db::{AgentxDb, InstanceRow};
use crate::service::{RequestContext, ServiceError, ServiceResult};
use serde_json::json;

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn participant(row: &InstanceRow) -> ParticipantRef {
    ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }
}
fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}
fn peer(context: &RequestContext) -> ServiceResult<()> {
    if context.process.pid == 0 || context.process.incarnation.as_deref().is_none_or(str::is_empty) {
        return Err(ServiceError::Forbidden("authenticated Claude hook process required".into()));
    }
    if context.caller.invocation.environment.get("AGENTX_TOOL").is_some_and(|tool| !tool.is_empty() && tool != "claude") {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}
fn current(db: &AgentxDb, fence: &ParticipantRef) -> ServiceResult<InstanceRow> {
    let row = db.get_instance_full(&fence.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    if participant(&row) != *fence || row.tool != "claude" || row.origin_device_id.is_some() {
        return Err(ServiceError::StaleFence);
    }
    Ok(row)
}
fn root_claims(db: &AgentxDb, context: &RequestContext, root: &InstanceRow) -> ServiceResult<()> {
    let invocation = &context.caller.invocation;
    if let Some(name) = &invocation.explicit_name
        && super::identity::resolve_from_name(db, name).map_err(internal)?.name != root.name
    {
        return Err(ServiceError::StaleFence);
    }
    if let Some(target) = &invocation.activation_target
        && (target.participant != participant(root) || !SqliteAttachmentStore::new(db).validate(target).map_err(internal)?)
    {
        return Err(ServiceError::StaleFence);
    }
    if let Some(attachment) = SqliteAttachmentStore::new(db).current_for_participant(&participant(root)).map_err(internal)? {
        if invocation.activation_target.as_ref() != Some(&attachment.target) || invocation.provider_session.as_ref().is_some_and(|session| *session != attachment.session) {
            return Err(ServiceError::StaleFence);
        }
    } else if invocation.activation_target.is_some() {
        return Err(ServiceError::StaleFence);
    }
    if invocation.provider_session.as_ref().is_some_and(|session| session.provider != "claude") {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}
fn root(db: &AgentxDb, context: &RequestContext, actor: &Actor, primary: bool) -> ServiceResult<InstanceRow> {
    let row = current(db, &actor.root)?;
    if row.parent_name.as_deref().is_some_and(|name| !name.is_empty()) || actor.session_id.is_empty() {
        return Err(ServiceError::StaleFence);
    }
    let binding = context.caller.invocation.process_binding_id.as_deref().filter(|binding| !binding.is_empty()).ok_or_else(|| ServiceError::IdentityRequired("Claude worker binding required".into()))?;
    if db.get_process_binding(binding).map_err(internal)?.as_deref() != Some(row.name.as_str()) || db.get_session_binding(&actor.session_id).map_err(internal)?.as_deref() != Some(row.name.as_str()) {
        return Err(ServiceError::StaleFence);
    }
    if primary && (row.session_id.as_deref() != Some(actor.session_id.as_str()) || db.get_validated_claude_session_owner(&actor.session_id).map_err(internal)?.as_deref() != Some(row.name.as_str())) {
        return Err(ServiceError::StaleFence);
    }
    root_claims(db, context, &row)?;
    Ok(row)
}
fn actor(db: &AgentxDb, context: &RequestContext, input: &Actor) -> ServiceResult<(InstanceRow, InstanceRow)> {
    let root = root(db, context, input, true)?;
    let row = current(db, &input.participant)?;
    match input.agent_id.as_deref() {
        Some(id) if !id.is_empty() && row.agent_id.as_deref() == Some(id) && row.parent_name.as_deref() == Some(root.name.as_str()) && row.parent_session_id.as_deref() == Some(input.session_id.as_str()) => {}
        None if input.participant == input.root => {}
        _ => return Err(ServiceError::StaleFence),
    }
    Ok((root, row))
}
fn status(db: &AgentxDb, row: &InstanceRow, value: &str, context: &str, detail: &str, tool: &str, tool_use: &str) -> ServiceResult<()> {
    // A late hook cannot erase the operator's killed marker/provenance.
    if row.status_context == "exit:killed" {
        return Ok(());
    }
    super::lifecycle::try_set_status(db, &row.name, value, context, super::lifecycle::StatusUpdate { detail, tool_name: tool, tool_use_id: tool_use, ..Default::default() }).map_err(internal)
}

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    peer(context)?;
    if let Request::Observe(input) = request {
        return identity::observe(db, context, input).map(Response::Observed);
    }
    let input = json!({"claude":request});
    transaction(db, || {
        let response = super::service_store::retained(db, context, &input, || match &request {
            Request::Observe(_) => unreachable!(),
            Request::Activity { actor: input, activity } => activity_update(db, context, input, activity),
            Request::PrepareOutput { actor: input, mailbox, bootstrap, stop_claim } => output::prepare(db, context, input, *mailbox, *bootstrap, stop_claim.as_ref()).map(Response::Output),
            Request::TaskCompleted { actor: input, async_launched } => output::task(db, context, input, *async_launched).map(Response::TaskOutput),
            Request::ConfirmFlushed { actor: input, mailbox, bootstrap, history } => {
                output::confirm(db, context, input, mailbox.as_ref(), bootstrap.as_deref(), history.as_ref())?;
                Ok(Response::Changed)
            }
            Request::BeginRootStop { actor: input, background_tasks } => stop::begin_root(db, context, input, background_tasks.as_deref()),
            Request::BeginChildStop { root, agent_id, prompt_id, fingerprint, transcript_path } => stop::begin_child(db, context, root, agent_id, prompt_id, fingerprint, transcript_path.as_deref()).map(Response::ChildStop),
            Request::FinishChildStop { claim, decision } => stop::finish_child(db, context, claim, *decision).map(Response::Stopped),
            Request::PollEndpoint { actor: input, port, stop_claim } => {
                stop::endpoint(db, context, input, *port, stop_claim.as_ref())?;
                Ok(Response::Changed)
            }
            Request::SessionEnd { root, reason } => stop::session_end(db, context, root, reason).map(Response::Stopped),
        })?;
        validate_response(db, context, &request, &response)?;
        Ok(response)
    })
}

fn validate_response(db: &AgentxDb, context: &RequestContext, request: &Request, response: &Response) -> ServiceResult<()> {
    match response {
        Response::Stopped(outcome) | Response::ChildStop(ChildStopResult::Retired(outcome)) => super::hook_store::validate_stop_outcome(db, outcome)?,
        Response::RootStop { retired_children, .. } => {
            super::hook_store::validate_stop_outcome(db, retired_children)?;
            if let Request::BeginRootStop { actor: input, .. } = request {
                actor(db, context, input)?;
            }
        }
        Response::ChildStop(ChildStopResult::Acquired { claim, .. }) => {
            stop::validate_claim(db, context, claim)?;
        }
        Response::Output(_) | Response::TaskOutput(_) | Response::Activity { .. } => {
            let input = match request {
                Request::PrepareOutput { actor, .. } | Request::TaskCompleted { actor, .. } | Request::Activity { actor, .. } => actor,
                _ => return Err(ServiceError::StaleFence),
            };
            actor(db, context, input)?;
        }
        Response::Changed => match request {
            Request::PollEndpoint { actor: input, .. } => {
                actor(db, context, input)?;
            }
            Request::ConfirmFlushed { actor: input, mailbox, history, .. } => {
                if mailbox.is_some() || history.is_some() || input.agent_id.is_some() {
                    actor(db, context, input)?;
                } else {
                    let row = current(db, &input.root)?;
                    if input.participant != input.root || row.session_id.as_deref() != Some(input.session_id.as_str()) {
                        return Err(ServiceError::StaleFence);
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

fn activity_update(db: &AgentxDb, context: &RequestContext, input: &Actor, activity: &Activity) -> ServiceResult<Response> {
    let (_, row) = actor(db, context, input)?;
    if row.status_context == "exit:killed" {
        return Ok(Response::Activity { actor_capability: None });
    }
    let mut capability = None;
    match activity {
        Activity::ToolStarted { tool_name, detail, tool_use_id, visible_agentx_shell } => {
            if tool_name.is_empty() {
                return Err(ServiceError::InvalidInput("Claude tool name required".into()));
            }
            status(db, &row, "active", &format!("tool:{tool_name}"), detail, tool_name, tool_use_id.as_deref().unwrap_or(""))?;
            if *visible_agentx_shell
                && matches!(tool_name.as_str(), "Bash" | "PowerShell")
                && let Some(agent) = input.agent_id.as_deref()
            {
                let id = tool_use_id.as_deref().filter(|id| !id.is_empty()).ok_or_else(|| ServiceError::InvalidInput("shell tool-use required".into()))?;
                capability = Some(db.issue_claude_actor_capability(&input.session_id, id, Some(agent), &row.name).map_err(internal)?);
            }
        }
        Activity::ToolSucceeded { tool_name, tool_use_id } => {
            if let Some(id) = tool_use_id {
                db.revoke_claude_actor_capability(&input.session_id, id, input.agent_id.as_deref()).map_err(internal)?;
            }
            if row.status == "blocked" {
                status(db, &row, "active", &format!("approved:{tool_name}"), "", tool_name, tool_use_id.as_deref().unwrap_or(""))?;
            }
        }
        Activity::ToolFailed { tool_name, detail, tool_use_id } | Activity::PermissionDenied { tool_name, detail, tool_use_id } => {
            if let Some(id) = tool_use_id {
                db.revoke_claude_actor_capability(&input.session_id, id, input.agent_id.as_deref()).map_err(internal)?;
            }
            let denied = matches!(activity, Activity::PermissionDenied { .. });
            status(db, &row, if denied { "blocked" } else { "active" }, &format!("{}:{tool_name}", if denied { "denied" } else { "tool_failed" }), detail, tool_name, tool_use_id.as_deref().unwrap_or(""))?;
        }
        Activity::PermissionRequested { tool_name, detail, tool_use_id } => status(db, &row, "blocked", "approval", detail, tool_name, tool_use_id.as_deref().unwrap_or(""))?,
        Activity::StopFailed { error, detail } => status(db, &row, "inactive", &format!("failure:{error}"), detail, "", "")?,
        Activity::Prompt { bare_wake } => {
            stop::mark_stale(db, context, input)?;
            if !bare_wake {
                status(db, &row, "active", "prompt", "", "", "")?;
            }
        }
        Activity::Idle => {
            status(db, &row, "listening", "", "", "", "")?;
            db.wake_after_commit(&row.name, &[crate::runtime::notify::WakeKind::Hook]);
        }
        Activity::Approval => status(db, &row, "blocked", "approval", "", "", "")?,
        Activity::PollTimedOut => status(db, &row, "inactive", "exit:timeout", "", "", "")?,
    }
    db.update_instance_fields(&row.name, json!({"last_seen":crate::shared::time::now_epoch_i64()}).as_object().unwrap()).map_err(internal)?;
    Ok(Response::Activity { actor_capability: capability })
}
