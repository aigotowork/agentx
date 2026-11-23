//! Scope-owned SQLite transactions. Reach callers only through the typed scope operations.
mod inbox;
pub(super) mod transcript;

use super::types::*;
use crate::messaging::routing::validate_message;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Row, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DB_NAME: &str = "communication.sqlite3";
const TRANSCRIPT_SCHEMA: &str = include_str!("store/transcript.sql");
const MESSAGE_HISTORY_INDEXES: &str = include_str!("store/message-history-indexes.sql");
const LEASE_SECONDS: i64 = 60;
const MESSAGE_COLUMNS: &str = "id,from_name,from_host,from_incarnation,to_name,to_host,to_incarnation,body,intent,reply_to,thread,created_at";
fn operator() -> AgentView {
    AgentView { host: OPERATOR_HOST.into(), name: OPERATOR_NAME.into(), incarnation: OPERATOR_INCARNATION.into() }
}

#[derive(Debug, thiserror::Error)]
pub(in crate::cloud) enum Error {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("{0}")]
    InvalidMessage(String),
    #[error("{0}")]
    NotFound(&'static str),
    #[error("{0}")]
    Conflict(&'static str),
    #[error("agent already has an outstanding inbox claim")]
    ClaimOutstanding,
    #[error("scope generation has changed")]
    Generation,
    #[error("scope credential has changed")]
    Credential,
    #[error("sender host is offline; renew its heartbeat")]
    Offline,
    #[error("host name belongs to another installation")]
    Installation,
    #[error("exact agent registration is no longer current")]
    Agent,
    #[error("exact daemon lease is no longer current")]
    Lease,
    #[error("requested workspace or role is unavailable")]
    Capability,
    #[error("report does not match the exact control claim")]
    ControlClaim,
    #[error("control claim already has a different terminal outcome")]
    ControlOutcome,
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("scope operation worker failed")]
    Worker,
    #[error("request body exceeds the scope size limit")]
    PayloadTooLarge,
    #[error("all scope wait slots are occupied")]
    WaitCapacity,
}

pub(in crate::cloud) type Result<T> = std::result::Result<T, Error>;

pub(super) fn initialize(root: &Path, generation: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(root)?;
    let mut conn = open(root, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute_batch(include_str!("store/schema.sql"))?;
    tx.execute_batch(MESSAGE_HISTORY_INDEXES)?;
    tx.execute_batch(TRANSCRIPT_SCHEMA)?;
    tx.execute("INSERT OR IGNORE INTO scope_metadata(singleton,generation) VALUES(1,?1)", [generation])?;
    check_generation(&tx, generation)?;
    tx.commit()?;
    Ok(())
}

/// Install additive read schema on scope databases created by older deployments.
pub(in crate::cloud) fn upgrade_message_history(root: &Path, generation: &str) -> Result<()> {
    transaction(
        root,
        generation,
        true,
        || true,
        |conn, _| {
            conn.execute_batch(MESSAGE_HISTORY_INDEXES)?;
            Ok(conn.execute_batch(TRANSCRIPT_SCHEMA)?)
        },
    )
}

pub(in crate::cloud) fn transaction<T>(root: &Path, generation: &str, write: bool, credential_valid: impl Fn() -> bool, action: impl FnOnce(&Connection, i64) -> Result<T>) -> Result<T> {
    if !credential_valid() {
        return Err(Error::Credential);
    }
    let (flags, behavior) = if write { (OpenFlags::SQLITE_OPEN_READ_WRITE, TransactionBehavior::Immediate) } else { (OpenFlags::SQLITE_OPEN_READ_ONLY, TransactionBehavior::Deferred) };
    let mut conn = open(root, flags)?;
    let tx = conn.transaction_with_behavior(behavior)?;
    if !credential_valid() {
        return Err(Error::Credential);
    }
    check_generation(&tx, generation)?;
    let result = action(&tx, now())?;
    if !credential_valid() {
        return Err(Error::Credential);
    }
    tx.commit()?;
    Ok(result)
}

fn open(root: &Path, flags: OpenFlags) -> rusqlite::Result<Connection> {
    let conn = Connection::open_with_flags(root.join(DB_NAME), flags)?;
    conn.busy_timeout(Duration::from_secs(3))?;
    if flags.contains(OpenFlags::SQLITE_OPEN_READ_WRITE) {
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
    }
    Ok(conn)
}

fn check_generation(conn: &Connection, expected: &str) -> Result<()> {
    let actual: String = conn.query_row("SELECT generation FROM scope_metadata WHERE singleton=1", [], |row| row.get(0))?;
    if actual != expected {
        return Err(Error::Generation);
    }
    Ok(())
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs().try_into().unwrap_or(i64::MAX)
}

fn online(last_seen: i64, now: i64) -> bool {
    now.saturating_sub(last_seen) < LEASE_SECONDS
}

pub(super) fn counts(conn: &Connection, now: i64) -> Result<(u32, u32, u32)> {
    Ok(conn.query_row(
        "SELECT (SELECT count(*) FROM hosts WHERE name<>?2), (SELECT count(*) FROM hosts WHERE last_seen>?1 AND name<>?2), (SELECT count(*) FROM agents WHERE retired=0)",
        params![now.saturating_sub(LEASE_SECONDS), OPERATOR_HOST],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?)
}

pub(in crate::cloud) fn name(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 64 || !value.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')) {
        return Err(Error::Invalid("agent names use 1–64 lowercase ASCII letters, digits, hyphens or underscores"));
    }
    Ok(())
}

pub(in crate::cloud) fn host_name(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 253 || !value.as_bytes()[0].is_ascii_alphanumeric() || !value.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')) {
        return Err(Error::Invalid("host names use lowercase ASCII letters, digits, dots, hyphens or underscores and start with a letter or digit"));
    }
    Ok(())
}

pub(in crate::cloud) fn opaque(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 || !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')) {
        return Err(Error::Invalid("identifier uses 1–128 ASCII letters, digits, hyphens, underscores, dots or colons"));
    }
    Ok(())
}

fn message_id(value: &str) -> Result<i64> {
    if value.len() > 19 || value.starts_with('0') || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::Invalid("message IDs are positive decimal strings"));
    }
    value.parse().ok().filter(|id| *id > 0).ok_or(Error::Invalid("message IDs are positive decimal strings"))
}

fn page_limit(limit: Option<u32>, default: u32, max: u32) -> Result<usize> {
    let limit = limit.unwrap_or(default);
    if !(1..=max).contains(&limit) {
        return Err(Error::Invalid("limit is outside the permitted range"));
    }
    Ok(limit as usize)
}

pub(super) fn hosts(conn: &Connection, now: i64, input: HostQuery) -> Result<HostPage> {
    if let Some(after) = &input.after {
        host_name(after)?;
    }
    let limit = page_limit(input.limit, 50, DIRECTORY_LIMIT)?;
    let mut items = query_hosts(conn, now, input.after.as_deref().unwrap_or(""), limit as i64 + 1)?;
    let next = if items.len() > limit {
        items.pop();
        items.last().map(|item| item.name.clone())
    } else {
        None
    };
    Ok(HostPage { items, next })
}

pub(super) fn agents(conn: &Connection, now: i64, input: AgentQuery) -> Result<AgentPage> {
    if let Some(after) = &input.after {
        name(after)?;
    }
    if let Some(host) = &input.host {
        host_name(host)?;
    }
    let limit = page_limit(input.limit, 50, DIRECTORY_LIMIT)?;
    let mut items = query_agents(conn, now, input.after.as_deref().unwrap_or(""), input.host.as_deref(), limit as i64 + 1, input.facts.unwrap_or(false))?;
    let next = if items.len() > limit {
        items.pop();
        items.last().map(|item| item.name.clone())
    } else {
        None
    };
    Ok(AgentPage { items, next })
}

fn query_hosts(conn: &Connection, now: i64, after: &str, limit: i64) -> rusqlite::Result<Vec<HostView>> {
    let mut stmt = conn.prepare("SELECT name,last_seen FROM hosts WHERE name>?1 AND name<>?3 ORDER BY name LIMIT ?2")?;
    stmt.query_map(params![after, limit, OPERATOR_HOST], |row| {
        let last_seen = row.get(1)?;
        Ok(HostView { name: row.get(0)?, last_seen, online: online(last_seen, now) })
    })?
    .collect()
}

fn read_agent(row: &Row<'_>, start: usize) -> rusqlite::Result<AgentView> {
    Ok(AgentView { name: row.get(start)?, host: row.get(start + 1)?, incarnation: row.get(start + 2)? })
}

fn query_agents(conn: &Connection, now: i64, after: &str, host: Option<&str>, limit: i64, facts: bool) -> Result<Vec<CloudAgentSummary>> {
    let mut stmt = conn.prepare(
        "SELECT a.name,a.host,a.incarnation,f.facts,f.reported_at FROM agents a
         LEFT JOIN agent_facts f ON ?4 AND f.name=a.name AND f.incarnation=a.incarnation
         WHERE a.retired=0 AND a.name>?1 AND (?2 IS NULL OR a.host=?2) ORDER BY a.name LIMIT ?3",
    )?;
    let mut rows = stmt.query(params![after, host, limit, facts])?;
    let mut result = Vec::new();
    while let Some(row) = rows.next()? {
        let AgentView { name, host, incarnation } = read_agent(row, 0)?;
        let value: Option<String> = row.get(3)?;
        let reported: Option<i64> = row.get(4)?;
        let facts = transcript::decode_facts(value.zip(reported), now, false)?;
        result.push(CloudAgentSummary { name, host, incarnation, facts });
    }
    Ok(result)
}

pub(in crate::cloud) fn renew_host(conn: &Connection, now: i64, host: String, input: HostRegistration) -> Result<HostView> {
    host_name(&host)?;
    opaque(&input.installation_id)?;
    let changed = conn.execute(
        "INSERT INTO hosts(name,installation_id,last_seen) VALUES(?1,?2,?3)
         ON CONFLICT(name) DO UPDATE SET last_seen=excluded.last_seen WHERE hosts.installation_id=excluded.installation_id",
        params![host, input.installation_id, now],
    )?;
    if changed == 0 {
        return Err(Error::Installation);
    }
    Ok(HostView { name: host, last_seen: now, online: true })
}

fn current_host(conn: &Connection, now: i64, host: &str, installation_id: &str, require_online: bool) -> Result<()> {
    host_name(host)?;
    opaque(installation_id)?;
    let registration: Option<(String, i64)> = conn.query_row("SELECT installation_id,last_seen FROM hosts WHERE name=?1", [host], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
    match registration {
        None => Err(Error::NotFound("host is not registered")),
        Some((registered, _)) if registered != installation_id => Err(Error::Installation),
        Some((_, last_seen)) if require_online && !online(last_seen, now) => Err(Error::Offline),
        Some(_) => Ok(()),
    }
}

pub(in crate::cloud) fn active_agent(conn: &Connection, key: &str) -> Result<Option<AgentView>> {
    Ok(conn.query_row("SELECT name,host,incarnation FROM agents WHERE name=?1 AND retired=0", [key], |row| read_agent(row, 0)).optional()?)
}

fn current_agent(conn: &Connection, now: i64, agent: &AgentView, installation_id: &str) -> Result<()> {
    name(&agent.name)?;
    opaque(&agent.incarnation)?;
    current_host(conn, now, &agent.host, installation_id, true)?;
    if active_agent(conn, &agent.name)?.as_ref() != Some(agent) {
        return Err(Error::Agent);
    }
    Ok(())
}

pub(in crate::cloud) fn register_agent(conn: &Connection, now: i64, agent_name: String, input: AgentRegistration) -> Result<AgentView> {
    name(&agent_name)?;
    opaque(&input.incarnation)?;
    current_host(conn, now, &input.host, &input.installation_id, true)?;
    let agent = AgentView { name: agent_name, host: input.host, incarnation: input.incarnation };
    if let Some(active) = active_agent(conn, &agent.name)? {
        return if active == agent { Ok(active) } else { Err(Error::Conflict("agent name already has a registered instance")) };
    }
    let count = conn.execute("INSERT INTO agents(name,host,incarnation) VALUES(?1,?2,?3) ON CONFLICT(name,incarnation) DO NOTHING", params![agent.name, agent.host, agent.incarnation])?;
    if count == 0 {
        return Err(Error::Conflict("retired agent incarnation cannot be registered again"));
    }
    Ok(agent)
}

pub(in crate::cloud) fn retire_agent(conn: &Connection, now: i64, agent_name: String, input: AgentRegistration) -> Result<AgentView> {
    name(&agent_name)?;
    opaque(&input.incarnation)?;
    current_host(conn, now, &input.host, &input.installation_id, false)?;
    let changed = conn.execute("UPDATE agents SET retired=1 WHERE name=?1 AND host=?2 AND incarnation=?3", params![agent_name, input.host, input.incarnation])?;
    if changed == 0 {
        return Err(Error::Agent);
    }
    Ok(AgentView { name: agent_name, host: input.host, incarnation: input.incarnation })
}

fn recipient(conn: &Connection, to: &str, reply_to: Option<i64>) -> Result<AgentView> {
    let target = to.strip_prefix('@').ok_or(Error::Invalid("recipient must be @name or @host/name"))?;
    let (host, agent) = if let Some((host, agent)) = target.split_once('/') {
        if host == OPERATOR_HOST && agent == OPERATOR_NAME {
            return if reply_to.is_some() { Ok(operator()) } else { Err(Error::Invalid("operator messages require a reply ID")) };
        }
        host_name(host)?;
        (Some(host), agent)
    } else {
        (None, target)
    };
    name(agent)?;
    let resolved = active_agent(conn, agent)?.ok_or(Error::NotFound("recipient is not registered"))?;
    if host.is_some_and(|host| host != resolved.host) {
        return Err(Error::NotFound("recipient is not registered on the requested host"));
    }
    Ok(resolved)
}

fn read_message(row: &Row<'_>) -> rusqlite::Result<MessageView> {
    let id: i64 = row.get(0)?;
    let intent = match row.get::<_, String>(8)?.as_str() {
        "request" => Intent::Request,
        "inform" => Intent::Inform,
        "ack" => Intent::Ack,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(MessageView { id: id.to_string(), from: read_agent(row, 1)?, to: read_agent(row, 4)?, body: row.get(7)?, intent, reply_to: row.get::<_, Option<i64>>(9)?.map(|id| id.to_string()), thread: row.get(10)?, created_at: row.get(11)? })
}

/// Page by durable message ID without reading the inbox or resolving current short names.
pub(super) fn list_messages(conn: &Connection, input: MessageHistoryQuery) -> Result<MessageHistoryPage> {
    name(&input.name)?;
    host_name(&input.host)?;
    opaque(&input.incarnation)?;
    let limit = page_limit(input.limit, 20, BATCH_LIMIT)?;
    if input.after.is_some() && input.before.is_some() {
        return Err(Error::Invalid("choose either before or after"));
    }
    let start = match input.after.as_deref() {
        None | Some("0") => 0,
        Some(value) => message_id(value)?,
    };
    let order = if input.after.is_some() { "ASC" } else { "DESC" };
    let end = input.before.as_deref().map(message_id).transpose()?.map_or(i64::MAX, |before| before - 1);
    let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM agents WHERE name=?1 AND host=?2 AND incarnation=?3)", params![input.name, input.host, input.incarnation], |row| row.get(0))?;
    if !exists {
        return Err(Error::NotFound("exact agent registration was not found"));
    }

    // Each arm can walk its exact identity's index backwards. Exclude a self-send
    // from the recipient arm so it appears once without a deduplication sort.
    let mut stmt = conn.prepare(&format!(
        "SELECT {MESSAGE_COLUMNS} FROM messages INDEXED BY messages_from_history
         WHERE from_name=?1 AND from_host=?2 AND from_incarnation=?3 AND id<=?4 AND id>?6
         UNION ALL
         SELECT {MESSAGE_COLUMNS} FROM messages INDEXED BY messages_to_history
         WHERE to_name=?1 AND to_host=?2 AND to_incarnation=?3 AND id<=?4 AND id>?6
           AND (from_name,from_host,from_incarnation)<>(to_name,to_host,to_incarnation)
         ORDER BY id {order} LIMIT ?5"
    ))?;
    let rows = stmt.query_map(params![input.name, input.host, input.incarnation, end, limit as i64 + 1, start], read_message)?;
    let mut items = Vec::new();
    let mut bytes = 0;
    let mut next = None;
    for row in rows {
        let message = row?;
        bytes += message.body.len();
        if items.len() == limit || bytes > MAX_BATCH_BODY_BYTES {
            next = items.last().map(|previous: &MessageView| previous.id.clone());
            break;
        }
        items.push(message);
    }
    Ok(MessageHistoryPage { items, next })
}

fn validate_body(body: &str) -> Result<()> {
    if body.len() > MAX_BODY_BYTES {
        return Err(Error::Invalid("message body must be at most 64 KiB"));
    }
    validate_message(body).map_err(Error::InvalidMessage)
}

fn replay(conn: &Connection, request_id: &str, fingerprint: &[u8]) -> Result<Option<MessageView>> {
    let existing = conn.query_row(&format!("SELECT {MESSAGE_COLUMNS},fingerprint FROM messages WHERE request_id=?1"), [request_id], |row| Ok((read_message(row)?, row.get::<_, Vec<u8>>(12)?))).optional()?;
    match existing {
        Some((message, original)) if original == fingerprint => Ok(Some(message)),
        Some(_) => Err(Error::Conflict("request ID already belongs to another send")),
        None => Ok(None),
    }
}

struct NewMessage {
    from: AgentView,
    to: AgentView,
    body: String,
    intent: Intent,
    reply_to: Option<i64>,
    thread: Option<String>,
}

fn insert_message(conn: &Connection, now: i64, request_id: &str, fingerprint: &[u8], input: NewMessage) -> Result<MessageView> {
    let NewMessage { from, to, body, intent, reply_to, thread } = input;
    // A reply to Space is delivered to durable history immediately; it needs no host claim.
    let confirmed = to.host == OPERATOR_HOST;
    let id: i64 = conn.query_row(
        "INSERT INTO messages(request_id,fingerprint,from_name,from_host,from_incarnation,to_name,to_host,to_incarnation,body,intent,reply_to,thread,created_at,confirmed)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14) RETURNING id",
        params![request_id, fingerprint, from.name, from.host, from.incarnation, to.name, to.host, to.incarnation, body, intent.as_str(), reply_to, thread, now, confirmed],
        |row| row.get(0),
    )?;
    Ok(MessageView { id: id.to_string(), from, to, body, intent, reply_to: reply_to.map(|id| id.to_string()), thread, created_at: now })
}

pub(super) fn send_human_message(conn: &Connection, now: i64, input: SendHumanMessage) -> Result<MessageView> {
    opaque(&input.request_id)?;
    validate_body(&input.body)?;
    name(&input.to.name)?;
    host_name(&input.to.host)?;
    opaque(&input.to.incarnation)?;

    // Scope members cannot reserve IDs in the administrator's replay namespace.
    let request_id = format!("~space:{}", input.request_id);
    let fingerprint = Sha256::digest(serde_json::to_vec(&input)?);
    if let Some(message) = replay(conn, &request_id, &fingerprint)? {
        return Ok(message);
    }
    if active_agent(conn, &input.to.name)?.as_ref() != Some(&input.to) {
        return Err(Error::Agent);
    }

    let from = operator();
    // Keep both message foreign keys intact without exposing a host or active Agent.
    conn.execute("INSERT OR IGNORE INTO hosts(name,installation_id,last_seen) VALUES(?1,?1,0)", [&from.host])?;
    conn.execute("INSERT OR IGNORE INTO agents(name,host,incarnation,retired) VALUES(?1,?2,?3,1)", params![from.name, from.host, from.incarnation])?;
    insert_message(conn, now, &request_id, &fingerprint, NewMessage { from, to: input.to, body: input.body, intent: Intent::Request, reply_to: None, thread: None })
}

pub(super) fn send(conn: &Connection, now: i64, input: SendMessage) -> Result<MessageView> {
    opaque(&input.request_id)?;
    validate_body(&input.body)?;
    if input.thread.as_ref().is_some_and(|thread| thread.trim().is_empty() || thread.len() > 128 || thread.chars().any(char::is_control)) {
        return Err(Error::Invalid("thread must be nonblank, at most 128 bytes and contain no control characters"));
    }
    let reply_to = input.reply_to.as_deref().map(message_id).transpose()?;
    let fingerprint = Sha256::digest(serde_json::to_vec(&input)?);
    if let Some(message) = replay(conn, &input.request_id, &fingerprint)? {
        return Ok(message);
    }
    current_agent(conn, now, &input.from, &input.installation_id)?;
    let to = recipient(conn, &input.to, reply_to)?;
    let natural = input.request_id.starts_with(NATURAL_REPLY_PREFIX);
    if natural && (to.host != OPERATOR_HOST || input.intent != Intent::Ack || reply_to.is_none()) {
        return Err(Error::Invalid("automatic reply requires an operator request"));
    }
    if let Some(reply_id) = reply_to {
        let reverse: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE id=?1 AND from_name=?2 AND from_incarnation=?3 AND to_name=?4 AND to_incarnation=?5)",
            params![reply_id, to.name, to.incarnation, input.from.name, input.from.incarnation],
            |row| row.get(0),
        )?;
        if !reverse {
            return Err(Error::NotFound("reply target does not belong to these exact participants"));
        }
        // A host may recover after an explicit Agent reply committed but before
        // its local turn journal advanced. Treat the exact relation as the
        // durable idempotency key so automatic natural forwarding cannot add a
        // second answer when the tool's response was lost.
        if natural {
            let prior = conn
                .query_row(
                    &format!("SELECT {MESSAGE_COLUMNS} FROM messages WHERE reply_to=?1 AND to_host='~space' AND from_name=?2 AND from_host=?3 AND from_incarnation=?4 AND to_name=?5 AND to_incarnation=?6 ORDER BY id LIMIT 1"),
                    params![reply_id, input.from.name, input.from.host, input.from.incarnation, to.name, to.incarnation],
                    read_message,
                )
                .optional()?;
            if let Some(message) = prior {
                return Ok(message);
            }
        }
    }
    insert_message(conn, now, &input.request_id, &fingerprint, NewMessage { from: input.from, to, body: input.body, intent: input.intent, reply_to, thread: input.thread })
}

pub(super) fn operation(conn: &Connection, request_id: String) -> Result<MessageOperation> {
    opaque(&request_id)?;
    let (message, confirmed, claimed) = conn
        .query_row(&format!("SELECT {MESSAGE_COLUMNS},confirmed,claim_id IS NOT NULL FROM messages WHERE request_id=?1"), [&request_id], |row| Ok((read_message(row)?, row.get::<_, bool>(12)?, row.get::<_, bool>(13)?)))
        .optional()?
        .ok_or(Error::NotFound("send request ID was not found"))?;
    let status = if confirmed {
        OperationStatus::Confirmed
    } else if active_agent(conn, &message.to.name)?.as_ref() != Some(&message.to) {
        OperationStatus::Undeliverable
    } else if claimed {
        OperationStatus::Claimed
    } else {
        OperationStatus::Accepted
    };
    Ok(MessageOperation { request_id, status, message })
}

pub(super) fn claim(conn: &Connection, now: i64, input: &ClaimInbox) -> Result<(InboxBatch, bool)> {
    inbox::claim(conn, now, input)
}

pub(super) fn confirm(conn: &Connection, now: i64, input: ConfirmInbox) -> Result<InboxConfirmation> {
    inbox::confirm(conn, now, input)
}

pub(super) fn recover(conn: &Connection, now: i64, input: RecoverInbox) -> Result<InboxBatch> {
    inbox::recover(conn, now, input)
}

#[cfg(test)]
mod tests;
