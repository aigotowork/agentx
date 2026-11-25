//! Originating CLI invocation, typed service acquisition and participant views.
//! No managed database is opened here; query failure never selects a fallback.

use crate::agents::service::{Request as Participants, ResolvedActor, Response as ParticipantResult};
use crate::db::InstanceRow;
use crate::service::{InvocationContext, Request, Response, ServiceError, ServiceResult, client::Client};
use crate::shared::{CommandContext, SenderIdentity, SenderKind};

/// Capture one originating invocation for callers migrating a remote-only branch.
pub(crate) fn connect() -> Result<Client, String> {
    let invocation = InvocationContext::capture().map_err(|error| error.to_string())?;
    let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
    Client::connect(scope, invocation).map_err(|error| error.to_string())
}

pub(crate) struct Session {
    pub client: Client,
    pub invocation: InvocationContext,
    pub command: CommandContext,
    pub actor: Option<ResolvedActor>,
}

impl Session {
    pub(crate) fn connect(name: Option<&str>, go: bool) -> ServiceResult<Self> {
        let mut invocation = InvocationContext::capture()?;
        invocation.explicit_name = name.map(str::to_owned);
        Self::from_invocation(invocation, go, false, false)
    }

    pub(crate) fn connect_required(name: Option<&str>, go: bool) -> ServiceResult<Self> {
        let mut invocation = InvocationContext::capture()?;
        invocation.explicit_name = name.map(str::to_owned);
        Self::from_invocation(invocation, go, false, true)
    }

    /// Build the submission caller without resolving presentation-only actor
    /// data. Shared Codex still needs its exact thread attachment before a write.
    pub(crate) fn for_direct_send(name: Option<&str>, go: bool) -> ServiceResult<Option<Self>> {
        let mut invocation = InvocationContext::capture()?;
        invocation.explicit_name = name.map(str::to_owned);
        apply_environment_identity_fallback(&mut invocation);
        let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
        let runtime = crate::runtime::context::AgentxContext::from_env(&invocation.environment, invocation.cwd.clone());
        if crate::integrations::codex::roots::is_shared_context(&runtime) {
            return Ok(None);
        }
        let client = Client::connect(scope, invocation.clone())?;
        let command = CommandContext { explicit_name: invocation.explicit_name.clone(), identity: None, go, identity_warning: None };
        Ok(Some(Self { client, invocation, command, actor: None }))
    }

    /// Launch accepts a human attribution label without inventing a durable
    /// participant. Managed/shared identities still resolve through the domain.
    pub(crate) fn for_launch(invocation: InvocationContext, go: bool) -> ServiceResult<Self> {
        Self::from_invocation(invocation, go, true, false)
    }

    fn from_invocation(mut invocation: InvocationContext, go: bool, allow_label: bool, require_identity: bool) -> ServiceResult<Self> {
        apply_environment_identity_fallback(&mut invocation);
        let explicit = invocation.explicit_name.clone();
        let name = explicit.as_deref();
        let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
        let mut client = Client::connect(scope, invocation.clone())?;
        let runtime = crate::runtime::context::AgentxContext::from_env(&invocation.environment, invocation.cwd.clone()).with_environment_file_keys(invocation.environment_file_keys.clone()).with_tty(invocation.stdin_is_tty, invocation.stdout_is_tty);
        if crate::integrations::codex::roots::is_shared_context(&runtime) {
            let store = crate::agents::service::RemoteAttachments::new(client.clone());
            let attachment = crate::integrations::codex::roots::resolve_shared_attachment(&store, &runtime, runtime.codex_thread_id.as_deref().unwrap_or(""))
                .map_err(|error| ServiceError::IdentityRequired(error.to_string()))?
                .ok_or_else(|| ServiceError::IdentityRequired("No current attachment for this provider session.".into()))?;
            invocation.activation_target = Some(attachment.target);
            client = client.with_invocation(invocation.clone());
        }
        let has_claim = name.is_some() || invocation.process_binding_id.is_some() || invocation.activation_target.is_some() || invocation.environment.contains_key("AGENTX_CLAUDE_ACTOR_SESSION");
        let mut actor = if has_claim {
            match client.call(Request::Participants(Participants::Resolve)) {
                Ok(Response::Participants(ParticipantResult::Actor(actor))) => Some(actor),
                Err(ServiceError::IdentityRequired(_) | ServiceError::NotFound(_)) if !require_identity && name.is_none() && invocation.activation_target.is_none() => None,
                Err(ServiceError::NotFound(_)) if allow_label && invocation.activation_target.is_none() => None,
                Err(error) => return Err(error),
                _ => return Err(ServiceError::Internal("daemon returned a different participant response".into())),
            }
        } else {
            None
        };
        if invocation.activation_target.is_some()
            && let (Some(name), Some(actor)) = (name, actor.as_ref())
            && name != actor.record.name
        {
            return Err(ServiceError::Forbidden(format!("Shared Codex caller --iam must match its attached participant {}", actor.record.name)));
        }
        if let Some(current) = &actor
            && current.record.tool == "codex"
            && current.record.session_id.as_deref().is_none_or(str::is_empty)
            && runtime.is_launched
            && invocation.activation_target.is_none()
            && invocation.process_binding_id.is_some()
            && let Some(session_id) = runtime.codex_thread_id.as_deref().filter(|value| !value.is_empty())
        {
            actor = match client.call(Request::Participants(Participants::BindLaunchedSession { participant: current.participant.clone(), session_id: session_id.to_owned() }))? {
                Response::Participants(ParticipantResult::Registered(actor)) => Some(actor),
                _ => return Err(ServiceError::Internal("daemon returned a different session binding response".into())),
            };
        }
        let identity = actor.as_ref().map(|actor| SenderIdentity { kind: SenderKind::Instance, name: actor.record.name.clone(), session_id: actor.record.session_id.clone(), instance_data: serde_json::to_value(&actor.record).ok() });
        let mut command = CommandContext { explicit_name: name.map(str::to_owned), identity, go, identity_warning: None };
        // A verified shell actor may be a child sharing its parent's process.
        // Only an ordinary explicit selection needs the legacy drift warning.
        if name.is_some() && invocation.process_binding_id.is_some() && !invocation.environment.contains_key("AGENTX_CLAUDE_ACTOR_SESSION") && invocation.activation_target.is_none() {
            let mut bound_invocation = invocation.clone();
            bound_invocation.explicit_name = None;
            match client.with_invocation(bound_invocation).call(Request::Participants(Participants::Resolve)) {
                Ok(Response::Participants(ParticipantResult::Actor(bound))) => {
                    if let Some(actor) = &actor
                        && actor.participant != bound.participant
                        && actor.record.parent_name.as_deref() != Some(&bound.record.name)
                    {
                        let name = &actor.record.name;
                        command.identity_warning = Some(format!("[agentx] warning: --iam '{name}' but this shell is bound to '{}'. If you are '{name}', run 'agentx start --as {name}'.", bound.record.name));
                    }
                }
                Err(ServiceError::IdentityRequired(_) | ServiceError::NotFound(_)) => {}
                Err(error) => return Err(error),
                _ => return Err(ServiceError::Internal("daemon returned a different participant response".into())),
            }
        }
        Ok(Self { client, invocation, command, actor })
    }

    pub(crate) fn records(&self) -> ServiceResult<Vec<InstanceRow>> {
        match self.client.call(Request::Participants(Participants::List))? {
            Response::Participants(ParticipantResult::Records(records)) => Ok(records),
            _ => Err(ServiceError::Internal("daemon returned a different participant response".into())),
        }
    }

    /// Listing may retire dead rows. Recover a lost reply with the same ID so
    /// its frozen host effects survive deletion of the original participants.
    pub(crate) fn listing(&self, target: Option<String>) -> ServiceResult<crate::service::queries::Listing> {
        let crate::service::queries::Response::Listing(mut listing) = self.cleanup_query(crate::service::queries::Request::Listing { target })? else {
            return Err(ServiceError::Internal("daemon returned a different listing projection".into()));
        };
        for effect in std::mem::take(&mut listing.cleanup.effects) {
            crate::runtime::stop_effects::apply(effect);
        }
        // Diagnostic files and terminal capture belong to this host. The daemon
        // supplies original rows so presentation can enrich its stored status.
        for record in &listing.records {
            if let Some(status) = listing.statuses.get_mut(&record.name)
                && status.context == "launch_failed"
                && let Some(detail) = crate::agents::lifecycle::capture_launch_failure_evidence(record)
                && !status.description.contains(&detail)
            {
                status.description.push('\n');
                status.description.push_str(&detail);
            }
        }
        Ok(listing)
    }

    pub(crate) fn diagnostics(&self) -> ServiceResult<crate::service::queries::Diagnostics> {
        let crate::service::queries::Response::Diagnostics(mut diagnostics) = self.cleanup_query(crate::service::queries::Request::Diagnostics)? else {
            return Err(ServiceError::Internal("daemon returned a different diagnostics projection".into()));
        };
        for effect in std::mem::take(&mut diagnostics.cleanup.effects) {
            crate::runtime::stop_effects::apply(effect);
        }
        Ok(diagnostics)
    }

    // Listing, filtered ListingLabels and Diagnostics retain their
    // cleanup plan atomically. This never reacquires a daemon or generation.
    pub(crate) fn cleanup_query(&self, query: crate::service::queries::Request) -> ServiceResult<crate::service::queries::Response> {
        let request = Request::Queries(query);
        let caller = crate::service::CallerContext { request_id: crate::service::RequestId(uuid::Uuid::new_v4().to_string()), invocation: self.invocation.clone(), deadline_unix_ms: None, expected_database_generation: None };
        let result = match self.client.call_with_context(caller.clone(), request.clone()) {
            Err(ServiceError::OutcomeUnknown(_)) => self.client.call_with_context(caller, request),
            result => result,
        }?;
        match result {
            Response::Queries(result) => Ok(result),
            _ => Err(ServiceError::Internal("daemon returned a different query response".into())),
        }
    }

    /// Ordinary command output is the adhoc participant's mailbox boundary.
    pub(crate) fn deliver_pending(&self, json: bool) -> ServiceResult<bool> {
        if json || crate::cli::context::command_owns_inline_delivery() {
            return Ok(false);
        }
        let Some(actor) = self.actor.as_ref().filter(|actor| actor.record.tool == "adhoc") else {
            return Ok(false);
        };
        let Some(batch) = crate::cli::output::Batch::prepare(&self.client, &actor.participant)? else {
            return Ok(false);
        };
        let records = self.records()?;
        let messages: Vec<_> = batch.messages.iter().collect();
        let formatted = crate::cli::output::format_router_messages(&messages, &actor.record.name, &records);
        let output = format!("\n{}\n[agentx]\n{}\n{formatted}\n{}", "─".repeat(40), "─".repeat(40), batch.remaining_note(&actor.record.name));
        batch.emit(&self.client, &output, true)?;
        Ok(true)
    }

    pub(crate) fn set_command_status(&self, command: &str) -> ServiceResult<()> {
        if matches!(command, "listen" | "start" | "stop" | "kill" | "reset" | "status") {
            return Ok(());
        }
        let Some(actor) = &self.actor else {
            return Ok(());
        };
        if actor.record.tool != "adhoc" && actor.record.parent_name.as_deref().is_none_or(str::is_empty) {
            return Ok(());
        }
        let status = if actor.record.tool == "adhoc" { "inactive" } else { "active" };
        match self.client.call(Request::Participants(Participants::Status { participant: actor.participant.clone(), status: status.into(), context: format!("tool:{command}"), detail: String::new() }))? {
            Response::Participants(ParticipantResult::Changed) => Ok(()),
            _ => Err(ServiceError::Internal("daemon returned a different participant response".into())),
        }
    }
}

fn apply_environment_identity_fallback(invocation: &mut InvocationContext) {
    if invocation.explicit_name.is_none()
        && invocation.process_binding_id.as_deref().is_none_or(str::is_empty)
        && let Some(name) = invocation.environment.get("AGENTX_INSTANCE_NAME").filter(|name| !name.is_empty())
    {
        invocation.explicit_name = Some(name.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_instance_name_fills_only_an_unbound_implicit_identity() {
        let mut invocation = InvocationContext { environment: std::collections::HashMap::from([("AGENTX_INSTANCE_NAME".into(), "rezu".into())]), ..Default::default() };
        apply_environment_identity_fallback(&mut invocation);
        assert_eq!(invocation.explicit_name.as_deref(), Some("rezu"));

        invocation.process_binding_id = Some("process-binding".into());
        invocation.explicit_name = None;
        apply_environment_identity_fallback(&mut invocation);
        assert_eq!(invocation.explicit_name, None, "a process binding remains the identity source when present");

        invocation.explicit_name = Some("selected".into());
        apply_environment_identity_fallback(&mut invocation);
        assert_eq!(invocation.explicit_name.as_deref(), Some("selected"), "explicit selection takes precedence");
    }
}
