//! Shared CLI/management directory addressing resolves configuration locally, then uses the existing
//! native launch reservation and durable mailbox. No filesystem lock is a lease.
use crate::agents::attachments::ParticipantRef;
use crate::agents::launch_service::{Request as LaunchRequest, Response as LaunchResponse};
use crate::agents::service::{Request as ParticipantRequest, Response as ParticipantResponse};
use crate::config::roles::DefaultRoute;
use crate::runtime::launcher::{self, LaunchParams, LaunchTool};
use crate::service::client::Client;
use crate::service::{Request, Response};
use anyhow::{Context, Result, bail};
use std::path::Path;

#[derive(serde::Serialize)]
pub(crate) struct Address {
    pub requested_directory: String,
    pub requested_workspace: Option<String>,
    pub resolved_workspace: Option<String>,
    pub role: crate::config::roles::ResolvedRole,
    pub recipient: Option<String>,
    pub created: bool,
}

pub(crate) fn resolve(client: &Client, requested: &Path, selected: Option<&Path>) -> Result<(Address, DefaultRoute)> {
    let requested = requested.canonicalize()?;
    if !requested.is_dir() {
        bail!("requested directory is not a directory");
    }
    let route = crate::config::roles::resolve_default_with_environment(client.scope_root(), selected, client.invocation().environment.get("AGENTX_PROFILE").map(String::as_str))?;
    let address = Address {
        requested_directory: requested.to_string_lossy().into_owned(),
        requested_workspace: selected.map(|path| path.to_string_lossy().into_owned()),
        resolved_workspace: route.workspace.as_ref().map(|path| path.to_string_lossy().into_owned()),
        role: route.role.clone(),
        recipient: None,
        created: false,
    };
    Ok((address, route))
}

fn lookup(client: &Client, address: &Address) -> Result<Option<ParticipantRef>> {
    let operation = LaunchRequest::EndpointLookup { workspace: address.resolved_workspace.clone(), role_path: address.role.path.to_string_lossy().into_owned() };
    match client.call(Request::Participants(ParticipantRequest::Launch(operation)))? {
        Response::Participants(ParticipantResponse::Launch(LaunchResponse::Endpoint(participant))) => Ok(participant),
        _ => bail!("daemon returned a different workspace endpoint response"),
    }
}

fn usable_recipient(client: &Client, participant: ParticipantRef) -> Result<ParticipantRef> {
    let records = match client.call(Request::Participants(ParticipantRequest::List))? {
        Response::Participants(ParticipantResponse::Records(records)) => records,
        _ => bail!("daemon returned a different participant response"),
    };
    let record = records.iter().find(|record| record.name == participant.canonical_name && record.created_at.to_bits() == participant.incarnation_bits).context("workspace endpoint changed before message submission; inspect its current state")?;
    // A reserved placeholder already owns its durable mailbox, even before
    // the winning launcher has started its Provider process.
    if record.status == "error" || (record.status == "inactive" && !crate::agents::records::is_launching_placeholder(record)) || record.status_context == "launch_failed" {
        bail!("workspace endpoint @{} is {} ({}); recover or explicitly retire it before sending", record.name, record.status, record.status_context);
    }
    Ok(participant)
}

fn reuse_winner(client: &Client, address: &mut Address) -> Result<ParticipantRef> {
    let participant = lookup(client, address)?.context("workspace reservation changed after a conflict; inspect before retrying")?;
    let name = usable_recipient(client, participant)?;
    address.recipient = Some(name.canonical_name.clone());
    Ok(name)
}

fn reservation_conflict(detail: &str) -> bool {
    detail.contains("workspace endpoint reservation conflict:")
}

/// Native Reserve arbitrates concurrent creators before any Provider spawn.
pub(crate) fn recipient(client: &Client, address: &mut Address, launcher: &str) -> Result<ParticipantRef> {
    if let Some(participant) = lookup(client, address)? {
        let name = usable_recipient(client, participant)?;
        address.recipient = Some(name.canonical_name.clone());
        return Ok(name);
    }
    let selection = crate::runtime::launch_profile::LaunchSelection { profile: Some(address.role.profile.clone()), ..Default::default() };
    let resolved = crate::runtime::launch_profile::prepare(client.scope_root(), &client.invocation().environment, None, &selection, &[])?;
    let tool = LaunchTool::from_str(&resolved.tool)?;
    let (args, background) = launcher::prepare_resolved_execution(&tool, &resolved.args, true);
    let mut runtime = crate::runtime::context::AgentxContext::from_env(&client.invocation().environment, client.invocation().cwd.clone())
        .with_environment_file_keys(client.invocation().environment_file_keys.clone())
        .with_tty(client.invocation().stdin_is_tty, client.invocation().stdout_is_tty);
    runtime.agentx_dir = client.scope_root().to_path_buf();
    let launched = launcher::launch_with_client(
        LaunchParams {
            tool: resolved.tool,
            count: 1,
            args,
            persisted_args: Some(resolved.args),
            launch_profile: resolved.profile,
            collaboration: Some(crate::agents::workspaces::LaunchCollaboration { workspace: address.resolved_workspace.clone(), role: Some(address.role.clone()), endpoint: true, instruction_additions: None }),
            background,
            cwd: Some(address.resolved_workspace.clone().unwrap_or_else(|| address.requested_directory.clone())),
            launcher: Some(launcher.into()),
            run_here: Some(false),
            ..Default::default()
        },
        &runtime,
        client,
    );
    let result = match launched {
        Ok(result) => result,
        Err(error) if reservation_conflict(&format!("{error:#}")) => return reuse_winner(client, address),
        Err(error) => return Err(uncertain(error)),
    };
    if result.launched > 0 {
        let participant = lookup(client, address).map_err(uncertain)?.context("launch succeeded but the workspace endpoint is unavailable; inspect before retrying").map_err(uncertain)?;
        let name = usable_recipient(client, participant).map_err(uncertain)?;
        address.recipient = Some(name.canonical_name.clone());
        address.created = true;
        return Ok(name);
    }
    // Only a known reservation collision may reuse the winner. Other failures
    // may have created a failed/uncertain process and must stay visible.
    let detail = serde_json::to_string(&result.errors)?;
    if reservation_conflict(&detail) {
        return reuse_winner(client, address);
    }
    Err(uncertain(anyhow::anyhow!("directory recipient launch failed: {detail}; inspect the workspace endpoint before retrying")))
}

/// After a launch attempt, failure cannot authorize another operation to replay it.
fn uncertain(error: impl std::fmt::Display) -> anyhow::Error {
    crate::service::ServiceError::OutcomeUnknown(error.to_string()).into()
}

#[cfg(test)]
mod tests {
    use super::reservation_conflict;

    #[test]
    fn only_native_reservation_collisions_allow_winner_reuse() {
        assert!(reservation_conflict("No instances launched (0/1): conflict: workspace endpoint reservation conflict: already reserved by 'romi'"));
        assert!(!reservation_conflict("workspace endpoint role changed; retire existing participant"));
        assert!(!reservation_conflict("Provider launch failed: connection lost"));
    }
}
