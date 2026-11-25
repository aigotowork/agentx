//! Durable control state. All reads and writes use the communication DB's Namespace generation fence.
use super::types::*;
use crate::cloud::communication::{
    store::{self, Error, Result},
    types::{AgentRegistration, AgentView, AgentWorkspace, HostRegistration},
};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::HashSet;

const LEASE_SECONDS: i64 = 30;
const MAX_PENDING: i64 = 64;

#[derive(Default)]
pub(super) struct LeaseFeatures {
    pub directory_launch: bool,
    pub workspace_management: bool,
}

struct Lease {
    id: String,
    expires_at: i64,
    workspaces: Vec<WorkspaceCapability>,
    project_roots: Vec<String>,
    directory_launch: bool,
    workspace_management: bool,
}

fn current_lease(conn: &Connection, host: &str) -> Result<Option<Lease>> {
    let row = conn
        .query_row("SELECT lease_id,expires_at,workspaces,project_roots,directory_launch,workspace_management FROM control_leases WHERE host=?1 AND is_current=1", [host], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get(4)?, row.get(5)?))
        })
        .optional()?;
    row.map(|(id, expires_at, workspaces, project_roots, directory_launch, workspace_management)| {
        Ok(Lease { id, expires_at, workspaces: serde_json::from_str(&workspaces)?, project_roots: serde_json::from_str(&project_roots)?, directory_launch, workspace_management })
    })
    .transpose()
}

fn installation(conn: &Connection, host: &str) -> Result<String> {
    store::host_name(host)?;
    conn.query_row("SELECT installation_id FROM hosts WHERE name=?1", [host], |row| row.get(0)).optional()?.ok_or(Error::NotFound("host is not registered"))
}

fn live_lease(conn: &Connection, now: i64, host: &str, installation_id: &str, lease_id: Option<&str>) -> Result<Lease> {
    store::opaque(installation_id)?;
    if let Some(id) = lease_id {
        store::opaque(id)?;
    }
    if installation(conn, host)? != installation_id {
        return Err(Error::Installation);
    }
    let value = current_lease(conn, host)?.filter(|lease| lease.expires_at > now);
    match (value, lease_id) {
        (Some(lease), expected) if expected.is_none_or(|id| id == lease.id) => Ok(lease),
        (_, Some(_)) => Err(Error::Lease),
        (_, None) => Err(Error::Offline),
    }
}

fn capability_view(generation: String, host: String, installation_id: String, now: i64, lease: Option<Lease>) -> HostCapabilities {
    let online = lease.as_ref().is_some_and(|lease| lease.expires_at > now);
    let observed_at = lease.as_ref().map(|lease| lease.expires_at - LEASE_SECONDS);
    let directory_launch = lease.as_ref().is_some_and(|lease| lease.directory_launch).then_some(true);
    let workspace_management = (online && lease.as_ref().is_some_and(|lease| lease.workspace_management)).then_some(true);
    let (workspaces, project_roots) = lease.map(|lease| (lease.workspaces, lease.project_roots)).unwrap_or_default();
    HostCapabilities { generation, host, installation_id, online, observed_at, workspaces, project_roots, directory_launch, workspace_management }
}

pub(super) fn capabilities(conn: &Connection, now: i64, generation: String, host: String) -> Result<HostCapabilities> {
    let installation_id = installation(conn, &host)?;
    let lease = current_lease(conn, &host)?;
    Ok(capability_view(generation, host, installation_id, now, lease))
}

fn validate_availability(available: bool, reason: Option<&str>) -> Result<()> {
    match (available, reason) {
        (true, None) => Ok(()),
        (false, Some(reason)) => store::opaque(reason),
        _ => Err(Error::Invalid("an unavailable capability must have a reason code and an available one must not")),
    }
}

fn validate_catalogue(workspaces: &[WorkspaceCapability]) -> Result<()> {
    if workspaces.len() > MAX_WORKSPACES || workspaces.iter().map(|workspace| workspace.roles.len()).sum::<usize>() > MAX_ROLES {
        return Err(Error::Invalid("catalogue exceeds 64 workspaces or 512 roles"));
    }
    let mut ids = HashSet::new();
    for workspace in workspaces {
        store::workspace_id(&workspace.workspace_id)?;
        validate_workspace_name(&workspace.name)?;
        validate_availability(workspace.default_available, workspace.default_reason.as_deref())?;
        if !ids.insert(&workspace.workspace_id) {
            return Err(Error::Invalid("duplicate workspace ID"));
        }
        let mut roles = HashSet::new();
        for role in &workspace.roles {
            store::opaque(&role.name)?;
            validate_availability(role.available, role.reason.as_deref())?;
            if !roles.insert(role.name.as_str()) {
                return Err(Error::Invalid("duplicate workspace role"));
            }
        }
    }
    Ok(())
}

fn validate_workspace_name(name: &str) -> Result<()> {
    if name.trim() != name || name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
        return Err(Error::Invalid("workspace name must be nonblank, at most 128 bytes and contain no control characters"));
    }
    Ok(())
}

fn validate_project_roots(roots: &[String]) -> Result<()> {
    if roots.len() > MAX_WORKSPACES {
        return Err(Error::Invalid("too many project root aliases"));
    }
    let mut seen = HashSet::new();
    for root in roots {
        crate::cloud::validate_id(root).map_err(|_| Error::Invalid("invalid project root alias"))?;
        if !seen.insert(root) {
            return Err(Error::Invalid("duplicate project root alias"));
        }
    }
    Ok(())
}

fn validate_request_id(id: &str) -> Result<()> {
    store::opaque(id)?;
    if matches!(id, "." | "..") {
        return Err(Error::Invalid("request_id cannot be a URL dot segment"));
    }
    Ok(())
}

/// A timestamp-only renewal never advances the Workspace roster.
pub(super) fn renew(conn: &Connection, now: i64, host: String, input: &RenewControlLease, features: &LeaseFeatures) -> Result<(HostCapabilities, bool)> {
    let RenewControlLease { generation, installation_id, lease_id, previous_lease_id, workspaces, project_roots } = input;
    store::opaque(lease_id)?;
    if let Some(previous) = previous_lease_id {
        store::opaque(previous)?;
    }
    validate_catalogue(workspaces)?;
    validate_project_roots(project_roots)?;
    let LeaseFeatures { directory_launch, workspace_management } = *features;
    let mut workspaces = workspaces.clone();
    workspaces.sort_unstable_by(|a, b| a.workspace_id.cmp(&b.workspace_id));
    for workspace in &mut workspaces {
        workspace.roles.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    }
    let mut project_roots = project_roots.clone();
    project_roots.sort_unstable();

    store::renew_host(conn, now, host.clone(), HostRegistration { generation: generation.clone(), installation_id: installation_id.clone() })?;
    let current = current_lease(conn, &host)?;
    let changed =
        current.as_ref().is_none_or(|lease| lease.expires_at <= now || lease.workspaces != workspaces || lease.project_roots != project_roots || lease.directory_launch != directory_launch || lease.workspace_management != workspace_management);
    let expires_at = now + LEASE_SECONDS;
    match current {
        Some(lease) if lease.id == *lease_id && lease.expires_at > now => {
            if changed {
                conn.execute(
                    "UPDATE control_leases SET expires_at=?3,workspaces=?4,project_roots=?5,directory_launch=?6,workspace_management=?7 WHERE host=?1 AND lease_id=?2",
                    params![host, lease_id, expires_at, serde_json::to_string(&workspaces)?, serde_json::to_string(&project_roots)?, directory_launch, workspace_management],
                )?;
            } else {
                conn.execute("UPDATE control_leases SET expires_at=?3 WHERE host=?1 AND lease_id=?2", params![host, lease_id, expires_at])?;
            }
        }
        active => {
            if active.as_ref().is_some_and(|lease| lease.expires_at > now && previous_lease_id.as_deref() != Some(&lease.id)) {
                return Err(Error::Lease);
            }
            let used: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM control_leases WHERE host=?1 AND lease_id=?2)", params![host, lease_id], |row| row.get(0))?;
            if used {
                return Err(Error::Lease);
            }
            conn.execute("UPDATE control_leases SET is_current=0 WHERE host=?1 AND is_current=1", [&host])?;
            conn.execute(
                "INSERT INTO control_leases(host,lease_id,expires_at,is_current,workspaces,project_roots,directory_launch,workspace_management) VALUES(?1,?2,?3,1,?4,?5,?6,?7)",
                params![host, lease_id, expires_at, serde_json::to_string(&workspaces)?, serde_json::to_string(&project_roots)?, directory_launch, workspace_management],
            )?;
        }
    }
    let lease = Lease { id: lease_id.clone(), expires_at, workspaces, project_roots, directory_launch, workspace_management };
    Ok((capability_view(generation.clone(), host, installation_id.clone(), now, Some(lease)), changed))
}

fn validate_command(command: &ControlCommand) -> Result<()> {
    match command {
        ControlCommand::Launch { workspace_id, role, name, title, initial_prompt, working_directory, create_directory } => {
            store::workspace_id(workspace_id)?;
            if let Some(role) = role {
                store::opaque(role)?;
            }
            if let Some(name) = name {
                store::name(name)?;
            }
            crate::agents::launch_title::normalize(title.as_deref()).map_err(Error::InvalidMessage)?;
            if initial_prompt.as_ref().is_some_and(|text| text.len() > 16 * 1024 || text.trim().is_empty() || text.chars().any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))) {
                return Err(Error::Invalid("initial_prompt must be non-empty plain text of at most 16 KiB"));
            }
            let path = working_directory.as_deref().unwrap_or_default();
            if !valid_working_directory(path) || (*create_directory == Some(true) && path.is_empty()) {
                return Err(Error::Invalid("working_directory must be a workspace-relative path; creation requires a nonempty path"));
            }
        }
        ControlCommand::Stop { name, incarnation } => {
            store::name(name)?;
            store::incarnation(incarnation)?;
        }
        ControlCommand::AddWorkspace { parent, directory, name, .. } => {
            crate::cloud::validate_id(parent).map_err(|_| Error::Invalid("invalid project root alias"))?;
            validate_workspace_name(name)?;
            if directory.is_empty() || !valid_working_directory(directory) {
                return Err(Error::Invalid("directory must be a nonempty workspace-relative path"));
            }
        }
    }
    Ok(())
}

fn check_command(conn: &Connection, host: &str, lease: &Lease, command: &ControlCommand) -> Result<()> {
    match command {
        ControlCommand::Launch { workspace_id, role, name, working_directory, create_directory, .. } => {
            let supported = lease.workspaces.iter().find(|item| &item.workspace_id == workspace_id).is_some_and(|item| match role {
                Some(selector) => item.roles.iter().any(|entry| entry.name == *selector && entry.available),
                None => item.default_available,
            });
            if !supported || ((working_directory.is_some() || create_directory.is_some()) && !lease.directory_launch) {
                return Err(Error::Capability);
            }
            if let Some(name) = name
                && store::active_agent(conn, name)?.is_some()
            {
                return Err(Error::Conflict("requested agent name is already registered"));
            }
        }
        ControlCommand::Stop { name, incarnation } => {
            if store::active_agent(conn, name)?.is_none_or(|agent| agent.host != host || agent.incarnation != *incarnation) {
                return Err(Error::Agent);
            }
        }
        ControlCommand::AddWorkspace { parent, .. } => {
            if !lease.workspace_management || !lease.project_roots.contains(parent) {
                return Err(Error::Capability);
            }
        }
    }
    Ok(())
}

struct Operation {
    id: String,
    host: String,
    installation_id: String,
    lease_id: String,
    command: String,
    claim: Option<String>,
    outcome: Option<String>,
}

impl Operation {
    fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self { id: row.get(0)?, host: row.get(1)?, installation_id: row.get(2)?, lease_id: row.get(3)?, command: row.get(4)?, claim: row.get(5)?, outcome: row.get(6)? })
    }

    fn receipt(self, generation: &str, execution_lease_live: bool) -> Result<ControlReceipt> {
        let outcome = if let Some(encoded) = self.outcome {
            Some(serde_json::from_str(&encoded)?)
        } else if execution_lease_live {
            None
        } else if self.claim.is_some() {
            Some(ControlOutcome::Uncertain { code: "host_lease_lost".into() })
        } else {
            Some(ControlOutcome::Failed { code: "host_lease_lost".into() })
        };
        let status = match &outcome {
            Some(ControlOutcome::Succeeded { .. } | ControlOutcome::WorkspaceAdded { .. }) => ControlStatus::Succeeded,
            Some(ControlOutcome::Failed { .. }) => ControlStatus::Failed,
            Some(ControlOutcome::Uncertain { .. }) => ControlStatus::Uncertain,
            None if self.claim.is_some() => ControlStatus::Running,
            None => ControlStatus::Queued,
        };
        Ok(ControlReceipt { generation: generation.into(), request_id: self.id, host: self.host, installation_id: self.installation_id, command: serde_json::from_str(&self.command)?, status, claim_id: self.claim, outcome })
    }
}

fn lookup(conn: &Connection, id: &str) -> Result<Option<Operation>> {
    Ok(conn.query_row("SELECT request_id,host,installation_id,execution_lease_id,command,claim_id,outcome FROM control_operations WHERE request_id=?1", [id], Operation::read).optional()?)
}

fn receipt(conn: &Connection, now: i64, generation: &str, op: Operation) -> Result<ControlReceipt> {
    let live: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM control_leases WHERE host=?1 AND lease_id=?2 AND is_current=1 AND expires_at>?3)", params![op.host, op.lease_id, now], |row| row.get(0))?;
    op.receipt(generation, live)
}

pub(super) fn get(conn: &Connection, now: i64, generation: &str, id: &str) -> Result<ControlReceipt> {
    validate_request_id(id)?;
    receipt(conn, now, generation, lookup(conn, id)?.ok_or(Error::NotFound("control operation was not found"))?)
}

pub(super) fn create(conn: &Connection, now: i64, input: &CreateControlOperation) -> Result<ControlReceipt> {
    let CreateControlOperation { generation, request_id, host, installation_id, command } = input;
    validate_request_id(request_id)?;
    validate_command(command)?;
    let encoded = serde_json::to_string(command)?;
    if let Some(op) = lookup(conn, request_id)? {
        if op.host != *host || op.installation_id != *installation_id || op.command != encoded {
            return Err(Error::Conflict("request_id already identifies a different control operation"));
        }
        return receipt(conn, now, generation, op);
    }
    let lease = live_lease(conn, now, host, installation_id, None)?;
    check_command(conn, host, &lease, command)?;
    let pending: i64 = conn.query_row("SELECT count(*) FROM control_operations WHERE host=?1 AND execution_lease_id=?2 AND outcome IS NULL", params![host, lease.id], |row| row.get(0))?;
    if pending >= MAX_PENDING {
        return Err(Error::Conflict("host control queue is full"));
    }
    conn.execute("INSERT INTO control_operations(request_id,host,installation_id,execution_lease_id,command) VALUES(?1,?2,?3,?4,?5)", params![request_id, host, installation_id, lease.id, encoded])?;
    Ok(ControlReceipt { generation: generation.clone(), request_id: request_id.clone(), host: host.clone(), installation_id: installation_id.clone(), command: command.clone(), status: ControlStatus::Queued, claim_id: None, outcome: None })
}

pub(super) fn poll(conn: &Connection, now: i64, host: &str, input: &PollControlOperation) -> Result<ControlPoll> {
    let lease = live_lease(conn, now, host, &input.installation_id, Some(&input.lease_id))?;
    loop {
        let op = conn
            .query_row(
                "SELECT request_id,host,installation_id,execution_lease_id,command,claim_id,outcome FROM control_operations
                 WHERE host=?1 AND execution_lease_id=?2 AND outcome IS NULL ORDER BY claim_id IS NULL,ordinal LIMIT 1",
                params![host, lease.id],
                Operation::read,
            )
            .optional()?;
        let Some(mut op) = op else { return Ok(ControlPoll { operation: None }) };
        // A retry must surface the same claim even if its role/target changed after dispatch.
        if op.claim.is_none() {
            let command = serde_json::from_str(&op.command)?;
            let failure = match check_command(conn, host, &lease, &command) {
                Ok(()) => None,
                Err(Error::Capability) => Some("capability_unavailable"),
                Err(Error::Agent) => Some("stale_agent"),
                Err(Error::Conflict(_)) => Some("agent_name_taken"),
                Err(error) => return Err(error),
            };
            if let Some(code) = failure {
                persist_outcome(conn, &op.id, &ControlOutcome::Failed { code: code.into() })?;
                continue;
            }
            let claim = uuid::Uuid::new_v4().to_string();
            conn.execute("UPDATE control_operations SET claim_id=?2 WHERE request_id=?1", params![op.id, claim])?;
            op.claim = Some(claim);
        }
        return Ok(ControlPoll { operation: Some(op.receipt(&input.generation, true)?) });
    }
}

fn persist_outcome(conn: &Connection, request_id: &str, outcome: &ControlOutcome) -> Result<()> {
    conn.execute("UPDATE control_operations SET outcome=?2 WHERE request_id=?1", params![request_id, serde_json::to_string(outcome)?])?;
    Ok(())
}

fn confirm_agent_success(conn: &Connection, now: i64, generation: &str, op: &Operation, command: &ControlCommand, agent: &AgentView) -> Result<AgentView> {
    store::name(&agent.name)?;
    if agent.host != op.host {
        return Err(Error::Agent);
    }
    match command {
        ControlCommand::Launch { workspace_id, name, .. } => {
            if name.as_ref().is_some_and(|name| *name != agent.name) {
                return Err(Error::Agent);
            }
            // The host report supplies an exact address. Only the registration owns its membership.
            let registered = store::exact_agent(conn, &agent.host, &agent.name, &agent.incarnation)?.ok_or(Error::Agent)?;
            if !matches!(&registered.workspace, AgentWorkspace::Project { workspace_id: actual } if actual == workspace_id) {
                return Err(Error::AgentWorkspace);
            }
            Ok(registered)
        }
        ControlCommand::Stop { name, incarnation } if *name == agent.name && *incarnation == agent.incarnation => {
            // Keep retirement and the exact successful stop receipt in the same transaction.
            store::retire_agent(
                conn,
                now,
                agent.name.clone(),
                AgentRegistration { generation: generation.into(), installation_id: op.installation_id.clone(), host: agent.host.clone(), incarnation: agent.incarnation.clone(), workspace: agent.workspace.clone() },
            )
        }
        ControlCommand::Stop { .. } => Err(Error::Agent),
        ControlCommand::AddWorkspace { .. } => Err(Error::Invalid("workspace registration requires a workspace_added outcome")),
    }
}

fn same_report(saved: &ControlOutcome, incoming: &ControlOutcome) -> bool {
    match (saved, incoming) {
        (ControlOutcome::Succeeded { agent: a }, ControlOutcome::Succeeded { agent: b }) => a.host == b.host && a.name == b.name && a.incarnation == b.incarnation,
        _ => saved == incoming,
    }
}

pub(super) fn report(conn: &Connection, now: i64, host: &str, id: &str, input: &ReportControlOperation) -> Result<ControlReceipt> {
    validate_request_id(id)?;
    store::opaque(&input.claim_id)?;
    let lease = live_lease(conn, now, host, &input.installation_id, Some(&input.lease_id))?;
    let mut op = lookup(conn, id)?.filter(|op| op.host == host && op.installation_id == input.installation_id).ok_or(Error::NotFound("control operation was not found on this installation"))?;
    if op.claim.as_deref() != Some(&input.claim_id) {
        return Err(Error::ControlClaim);
    }
    match &input.outcome {
        ControlOutcome::Succeeded { agent } => {
            store::incarnation(&agent.incarnation)?;
            store::workspace(&agent.workspace)?;
        }
        ControlOutcome::WorkspaceAdded { workspace_id } => store::workspace_id(workspace_id)?,
        _ => (),
    }
    if let Some(saved) = &op.outcome {
        if !same_report(&serde_json::from_str(saved)?, &input.outcome) {
            return Err(Error::ControlOutcome);
        }
        return op.receipt(&input.generation, true);
    }
    let command = serde_json::from_str(&op.command)?;
    let outcome = match &input.outcome {
        ControlOutcome::Succeeded { agent } => ControlOutcome::Succeeded { agent: confirm_agent_success(conn, now, &input.generation, &op, &command, agent)? },
        ControlOutcome::WorkspaceAdded { workspace_id } => {
            let ControlCommand::AddWorkspace { name, .. } = command else {
                return Err(Error::Invalid("workspace_added requires an add_workspace command"));
            };
            if !lease.workspaces.iter().any(|workspace| &workspace.workspace_id == workspace_id && workspace.name == name) {
                return Err(Error::Capability);
            }
            input.outcome.clone()
        }
        ControlOutcome::Failed { code } | ControlOutcome::Uncertain { code } => {
            store::opaque(code)?;
            input.outcome.clone()
        }
    };
    persist_outcome(conn, id, &outcome)?;
    op.outcome = Some(serde_json::to_string(&outcome)?);
    op.receipt(&input.generation, true)
}

#[cfg(test)]
mod tests;
