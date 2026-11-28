//! Durable control state. All reads and writes use the communication DB's scope generation fence.
use super::types::*;
use crate::cloud::communication::{
    store::{self, Error, Result},
    types::{AgentRegistration, AgentView, HostRegistration},
};
use rusqlite::{Connection, OptionalExtension, params};
use std::{collections::HashSet, path::Path};

const LEASE_SECONDS: i64 = 30;
const MAX_PENDING: i64 = 64;
const SCHEMA_VERSION: i64 = 2;

// Upgrade old scope databases on first control access. Normal control reads remain read-only.
pub(super) fn transaction<T>(root: &Path, generation: &str, write: bool, authorized: impl Fn() -> bool, action: impl Fn(&Connection, i64) -> Result<T>) -> Result<T> {
    let schema_version = |conn: &Connection| conn.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0));
    if !write {
        let value = store::transaction(root, generation, false, &authorized, |conn, now| match schema_version(conn)? {
            SCHEMA_VERSION => action(conn, now).map(Some),
            0 | 1 => Ok(None),
            _ => Err(Error::Conflict("unsupported scope storage schema")),
        })?;
        if let Some(value) = value {
            return Ok(value);
        }
    }
    store::transaction(root, generation, true, authorized, |conn, now| {
        match schema_version(conn)? {
            0 => conn.execute_batch(include_str!("schema.sql"))?,
            1 => conn.execute_batch("ALTER TABLE control_leases ADD COLUMN directory_launch INTEGER NOT NULL DEFAULT 0 CHECK(directory_launch IN (0,1)); PRAGMA user_version=2;")?,
            SCHEMA_VERSION => (),
            _ => return Err(Error::Conflict("unsupported scope storage schema")),
        }
        action(conn, now)
    })
}

struct Lease {
    id: String,
    expires_at: i64,
    workspaces: Vec<WorkspaceCapability>,
    directory_launch: bool,
}

fn current_lease(conn: &Connection, host: &str) -> Result<Option<Lease>> {
    let row = conn.query_row("SELECT lease_id,expires_at,workspaces,directory_launch FROM control_leases WHERE host=?1 AND is_current=1", [host], |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, String>(2)?, row.get(3)?))).optional()?;
    row.map(|(id, expires_at, workspaces, directory_launch)| Ok(Lease { id, expires_at, workspaces: serde_json::from_str(&workspaces)?, directory_launch })).transpose()
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
    let workspaces = lease.map(|lease| lease.workspaces).unwrap_or_default();
    HostCapabilities { generation, host, installation_id, online, observed_at, workspaces, directory_launch }
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
    let mut aliases = HashSet::new();
    for workspace in workspaces {
        validate_workspace(&workspace.alias)?;
        validate_availability(workspace.default_available, workspace.default_reason.as_deref())?;
        if !aliases.insert(workspace.alias.as_str()) {
            return Err(Error::Invalid("duplicate workspace alias"));
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

fn validate_workspace(alias: &str) -> Result<()> {
    crate::cloud::validate_id(alias).map_err(|_| Error::Invalid("invalid workspace alias"))
}

fn validate_request_id(id: &str) -> Result<()> {
    store::opaque(id)?;
    if matches!(id, "." | "..") {
        return Err(Error::Invalid("request_id cannot be a URL dot segment"));
    }
    Ok(())
}

pub(super) fn renew(conn: &Connection, now: i64, host: String, input: &RenewControlLease, directory_launch: bool) -> Result<HostCapabilities> {
    let RenewControlLease { generation, installation_id, lease_id, previous_lease_id, workspaces } = input;
    store::opaque(lease_id)?;
    if let Some(previous) = previous_lease_id {
        store::opaque(previous)?;
    }
    validate_catalogue(workspaces)?;
    store::renew_host(conn, now, host.clone(), HostRegistration { generation: generation.clone(), installation_id: installation_id.clone() })?;
    let current = current_lease(conn, &host)?;
    let expires_at = now + LEASE_SECONDS;
    let encoded = serde_json::to_string(workspaces)?;
    match current {
        Some(lease) if lease.id == *lease_id && lease.expires_at > now => {
            conn.execute("UPDATE control_leases SET expires_at=?3,workspaces=?4,directory_launch=?5 WHERE host=?1 AND lease_id=?2", params![host, lease_id, expires_at, encoded, directory_launch])?;
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
            conn.execute("INSERT INTO control_leases(host,lease_id,expires_at,is_current,workspaces,directory_launch) VALUES(?1,?2,?3,1,?4,?5)", params![host, lease_id, expires_at, encoded, directory_launch])?;
        }
    }
    Ok(capability_view(generation.clone(), host, installation_id.clone(), now, Some(Lease { id: lease_id.clone(), expires_at, workspaces: workspaces.clone(), directory_launch })))
}

fn validate_command(command: &ControlCommand) -> Result<()> {
    match command {
        ControlCommand::Launch { workspace, role, name, title, initial_prompt, working_directory, create_directory } => {
            validate_workspace(workspace)?;
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
            store::opaque(incarnation)?;
        }
    }
    Ok(())
}

fn check_command(conn: &Connection, host: &str, lease: &Lease, command: &ControlCommand) -> Result<()> {
    match command {
        ControlCommand::Launch { workspace, role, name, working_directory, create_directory, .. } => {
            let supported = lease.workspaces.iter().find(|item| item.alias == *workspace).is_some_and(|item| match role {
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
            Some(ControlOutcome::Succeeded { .. }) => ControlStatus::Succeeded,
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
    let encoded = serde_json::to_string(command)?;
    if let Some(op) = lookup(conn, request_id)? {
        if op.host != *host || op.installation_id != *installation_id || op.command != encoded {
            return Err(Error::Conflict("request_id already identifies a different control operation"));
        }
        return receipt(conn, now, generation, op);
    }
    validate_command(command)?;
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

fn confirm_success(conn: &Connection, now: i64, generation: &str, op: &Operation, agent: &AgentView) -> Result<()> {
    let command: ControlCommand = serde_json::from_str(&op.command)?;
    store::name(&agent.name)?;
    store::opaque(&agent.incarnation)?;
    let matches = match &command {
        ControlCommand::Launch { name, .. } => name.as_ref().is_none_or(|name| *name == agent.name),
        ControlCommand::Stop { name, incarnation } => *name == agent.name && *incarnation == agent.incarnation,
    };
    if !matches || agent.host != op.host {
        return Err(Error::Agent);
    }
    if matches!(command, ControlCommand::Stop { .. }) {
        // Keep cloud retirement and the successful exact stop receipt in the same transaction.
        store::retire_agent(conn, now, agent.name.clone(), AgentRegistration { generation: generation.into(), installation_id: op.installation_id.clone(), host: agent.host.clone(), incarnation: agent.incarnation.clone() })?;
        return Ok(());
    }
    // Launch evidence can be reconciled after the successfully started Agent has already exited.
    let registered: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM agents WHERE name=?1 AND host=?2 AND incarnation=?3)", params![agent.name, agent.host, agent.incarnation], |row| row.get(0))?;
    if !registered {
        return Err(Error::Agent);
    }
    Ok(())
}

pub(super) fn report(conn: &Connection, now: i64, host: &str, id: &str, input: &ReportControlOperation) -> Result<ControlReceipt> {
    validate_request_id(id)?;
    store::opaque(&input.claim_id)?;
    live_lease(conn, now, host, &input.installation_id, Some(&input.lease_id))?;
    let mut op = lookup(conn, id)?.filter(|op| op.host == host && op.installation_id == input.installation_id).ok_or(Error::NotFound("control operation was not found on this installation"))?;
    if op.claim.as_deref() != Some(&input.claim_id) {
        return Err(Error::ControlClaim);
    }
    let encoded = serde_json::to_string(&input.outcome)?;
    if let Some(saved) = &op.outcome {
        if *saved != encoded {
            return Err(Error::ControlOutcome);
        }
        return op.receipt(&input.generation, true);
    }
    match &input.outcome {
        ControlOutcome::Succeeded { agent } => confirm_success(conn, now, &input.generation, &op, agent)?,
        ControlOutcome::Failed { code } | ControlOutcome::Uncertain { code } => store::opaque(code)?,
    }
    persist_outcome(conn, id, &input.outcome)?;
    op.outcome = Some(encoded);
    op.receipt(&input.generation, true)
}

#[cfg(test)]
mod tests;
