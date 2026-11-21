//! Identity resolution — 3-tier binding (process → session → ad-hoc).

use regex::Regex;
use std::sync::LazyLock;

use crate::db::AgentxDb;
use crate::shared::{AgentxError, SenderIdentity, SenderKind};

/// UUID pattern for agent_id detection.
static UUID_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap());

/// Valid native instance name: lowercase letters, digits, underscore.
static NAME_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z0-9_]+$").unwrap());

/// Dangerous characters for user-provided names (injection prevention).
static DANGEROUS_CHARS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[|&;$`<>]").unwrap());

/// Dangerous characters including @ (for --from validation).
static DANGEROUS_CHARS_WITH_AT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[|&;$`<>@]").unwrap());

/// Commands that require a resolved identity to operate.
const REQUIRE_IDENTITY: &[&str] = &["send", "listen"];

/// Check if value looks like a UUID (agent_id format).
pub fn looks_like_uuid(value: &str) -> bool {
    UUID_PATTERN.is_match(value)
}

/// Check if name looks like a Claude Task agent_id (7-char hex).
pub fn looks_like_agent_id(name: &str) -> bool {
    name.len() == 7 && name.chars().all(|c| c.is_ascii_hexdigit())
}

/// Check if name is a valid native instance name (lowercase letters, digits, underscore).
pub fn is_valid_name(name: &str) -> bool {
    NAME_RE.is_match(name)
}

/// Build error message for invalid native instance names.
pub fn name_error(name: &str) -> String {
    format!("Invalid instance name '{name}'. Use lowercase letters, numbers, and underscore.")
}

/// Generate actionable error message for instance not found.
///
/// For subagent agent_ids, don't suggest `--as` (causes process binding conflicts).
pub fn instance_not_found_error(name: &str) -> String {
    if looks_like_agent_id(name) {
        return format!("Instance '{name}' not found. Your session may have ended. Stop working and end your turn.");
    }
    format!("Instance '{name}' not found. Run 'agentx start --as {name}' to reclaim your identity.")
}

/// Like `instance_not_found_error`, but suppresses the `--as` prescription when
/// the missing name corresponds to a subagent slot. Subagents share their
/// parent's session_id, so `agentx start --as <subagent_name>` from inside a
/// subagent bash context rebinds the parent's identity — the prescription
/// itself is the bug trigger. This variant consults live+historical state via
/// `AgentxDb::was_subagent_name` and returns the same "session may have ended"
/// text used for raw agent_ids instead.
pub fn instance_not_found_error_for(db: &AgentxDb, name: &str) -> String {
    if looks_like_agent_id(name) || looks_like_uuid(name) || db.was_subagent_name(name) {
        return format!("Instance '{name}' not found. Your session may have ended. Stop working and end your turn.");
    }
    format!("Instance '{name}' not found. Run 'agentx start --as {name}' to reclaim your identity.")
}

/// Validate user-provided name input for length and dangerous characters.
///
/// Used for `--iam` and `--from` flag validation in CLI commands.
pub fn validate_name_input(name: &str, max_length: usize, allow_at: bool) -> Result<(), String> {
    if name.len() > max_length {
        return Err(format!("Name too long ({} chars, max {max_length})", name.len()));
    }

    let pattern = if allow_at { &*DANGEROUS_CHARS } else { &*DANGEROUS_CHARS_WITH_AT };

    let bad_chars: Vec<&str> = pattern.find_iter(name).map(|m| m.as_str()).collect();
    if !bad_chars.is_empty() {
        let unique: std::collections::HashSet<&str> = bad_chars.into_iter().collect();
        let chars_str: Vec<&str> = unique.into_iter().collect();
        return Err(format!("Name contains invalid characters: {}", chars_str.join(" ")));
    }

    Ok(())
}

/// Latest stop of an agent that no longer has a live row.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct StoppedAgent {
    pub name: String,
    pub stopped_at: Option<i64>,
    pub by: String,
    pub reason: String,
    pub tool: String,
    pub directory: String,
    pub session_id: String,
    /// Child exit code when the tool exited on its own; output is set only for a failure.
    pub exit_code: Option<i64>,
    pub output: String,
}

impl StoppedAgent {
    /// `stopped 23m ago (killed by muse) · claude · ~/Dev/x`
    pub fn summary(&self) -> String {
        let when = match self.stopped_at {
            Some(ts) => {
                let age = crate::shared::now_epoch_i64() - ts;
                match crate::shared::format_age(age).as_str() {
                    "now" => "just now".to_string(),
                    a => format!("{a} ago"),
                }
            }
            None => "earlier".to_string(),
        };
        let mut out = format!("stopped {when}");
        match (self.reason.as_str(), self.by.as_str()) {
            ("", "") => {}
            (reason, "") => out.push_str(&format!(" ({reason})")),
            ("", by) => out.push_str(&format!(" (by {by})")),
            (reason, by) => out.push_str(&format!(" ({reason} by {by})")),
        }
        if let Some(code) = self.exit_code.filter(|&code| code != 0) {
            out.push_str(&format!(" · exit {code}"));
        }
        if !self.tool.is_empty() {
            out.push_str(&format!(" · {}", self.tool));
        }
        if !self.directory.is_empty() {
            out.push_str(&format!(" · {}", crate::shared::platform::shorten_path(&self.directory)));
        }
        out
    }
}

/// Load the latest stop for the exact native `input_name` when it has no
/// live row. `None` means it is live or agentx has never seen it.
pub fn last_stopped(db: &AgentxDb, input_name: &str) -> Option<StoppedAgent> {
    if db.get_instance_full(input_name).ok().flatten().is_some() {
        return None;
    }
    let (timestamp, data): (String, String) = db
        .conn()
        .query_row(
            "SELECT timestamp, data FROM events
             WHERE type = 'life' AND instance = ?1
               AND json_extract(data, '$.action') = 'stopped'
             ORDER BY id DESC LIMIT 1",
            rusqlite::params![input_name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok()?;
    let data: serde_json::Value = serde_json::from_str(&data).unwrap_or_default();
    let field = |v: &serde_json::Value| v.as_str().unwrap_or("").to_string();
    Some(StoppedAgent {
        stopped_at: parse_event_timestamp(&timestamp),
        by: field(&data["by"]),
        reason: field(&data["reason"]),
        tool: field(&data["snapshot"]["tool"]),
        directory: field(&data["snapshot"]["directory"]),
        session_id: field(&data["snapshot"]["session_id"]),
        exit_code: data["exit_code"].as_i64(),
        output: field(&data["output"]),
        name: input_name.to_owned(),
    })
}

/// Indented `Last output:` block for a crashed tool, empty when there is none.
pub fn last_output_block(output: &str) -> String {
    if output.is_empty() {
        return String::new();
    }
    let mut block = String::from("\n  Last output:");
    for line in output.lines() {
        block.push_str("\n    ");
        block.push_str(line);
    }
    block
}

fn parse_event_timestamp(ts: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(ts).map(|dt| dt.timestamp()).ok().or_else(|| chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S%.f").ok().map(|dt| dt.and_utc().timestamp()))
}

/// Live native names and recently stopped native names for typo suggestions.
pub fn known_agent_names(db: &AgentxDb) -> Vec<String> {
    let mut names: Vec<String> = db.iter_instances_full().unwrap_or_default().into_iter().map(|row| row.name).collect();
    if let Ok(mut stmt) = db.conn().prepare(
        "SELECT DISTINCT instance FROM events
         WHERE type = 'life' AND json_extract(data, '$.action') = 'stopped'
         ORDER BY id DESC LIMIT 200",
    ) && let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0))
    {
        names.extend(rows.flatten());
    }
    names
}

/// Explain why `input_name` is not a live agent, with the next useful command.
///
/// Stopped: `'tuna' stopped 23m ago (killed by muse) · claude …`
/// plus resume/transcript hints. Never seen: close-name suggestions.
pub fn describe_missing_agent(db: &AgentxDb, input_name: &str) -> String {
    if let Some(stopped) = last_stopped(db, input_name) {
        return format!("'{input_name}' {}{}\n  Resume: agentx r {name}  |  History: agentx transcript {name}", stopped.summary(), last_output_block(&stopped.output), name = stopped.name);
    }
    let known = known_agent_names(db);
    let mut msg = format!("No agent named '{input_name}'");
    msg.push_str(&crate::shared::suggest::did_you_mean(input_name, known.iter().map(String::as_str)));
    msg.push_str("\n  Active: agentx list  |  Stopped: agentx list --stopped");
    msg
}

/// Resolve `--iam NAME` with strict instance lookup.
///
/// Resolution order:
/// 1. Instance name lookup (exact) -> kind=Instance if found
/// 2. Agent ID (UUID) lookup -> kind=Instance if found
/// 3. Error if not found
pub fn resolve_from_name(db: &AgentxDb, name: &str) -> Result<SenderIdentity, AgentxError> {
    if !looks_like_uuid(name) && !is_valid_name(name) {
        return Err(AgentxError::InvalidInput(name_error(name)));
    }
    let resolved_name = name.to_string();

    // 1. Instance name lookup (exact match)
    if let Ok(Some(data)) = db.get_instance(&resolved_name) {
        crate::log::log_info("identity", "resolve_from_name", &format!("name={}, method=instance_name", resolved_name));
        return Ok(SenderIdentity { kind: SenderKind::Instance, name: resolved_name, session_id: data.get("session_id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string()), instance_data: Some(data) });
    }

    // 2. Agent ID lookup (Claude Code sends short IDs like 'a6d9caf')
    if let Ok(Some(instance_name)) = db.get_instance_by_agent_id(&resolved_name)
        && let Ok(Some(data)) = db.get_instance(&instance_name)
    {
        crate::log::log_info("identity", "resolve_from_name", &format!("name={}, method=agent_id, resolved={}", resolved_name, instance_name));
        return Ok(SenderIdentity { kind: SenderKind::Instance, name: instance_name, session_id: data.get("session_id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string()), instance_data: Some(data) });
    }

    // 3. Not found
    crate::log::log_info("identity", "resolve_from_name.not_found", &format!("name={}", resolved_name));
    Err(AgentxError::NotFound(instance_not_found_error_for(db, &resolved_name)))
}

/// Resolve sender identity for CLI commands and hook handlers.
///
/// # Arguments
///
/// * `db` - Database handle
/// * `name` - Instance name from `--iam` flag (strict lookup)
/// * `system_sender` - System notification sender name (e.g., 'agentx-launcher')
/// * `session_id` - Explicit session_id (for hook context, bypasses env detection)
/// * `process_id` - AGENTX_PROCESS_ID (for launched instances)
/// * `codex_thread_id` - Codex thread ID for opportunistic session binding
///
/// # Priority
///
/// 1. `system_sender` - system notifications
/// 2. `session_id` - explicit session (internal use)
/// 3. `name` (--iam) - strict instance lookup
/// 4. Auto-detect from `process_id` (AGENTX_PROCESS_ID)
/// 5. Error if no identity
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn resolve_identity_with_expectation(db: &AgentxDb, name: Option<&str>, system_sender: Option<&str>, session_id: Option<&str>, process_id: Option<&str>, codex_thread_id: Option<&str>, identity_expected: bool) -> Result<SenderIdentity, AgentxError> {
    // 1. System sender (internal use)
    if let Some(sender) = system_sender {
        return Ok(SenderIdentity { kind: SenderKind::System, name: sender.to_string(), instance_data: None, session_id: None });
    }

    // 2. Explicit session_id (internal use)
    if let Some(sid) = session_id
        && !sid.is_empty()
    {
        let resolved_name = db.get_session_binding(sid).map_err(|e| AgentxError::DatabaseError(e.to_string()))?;

        match resolved_name {
            Some(inst_name) => {
                let data = db.get_instance(&inst_name).map_err(|e| AgentxError::DatabaseError(e.to_string()))?;

                match data {
                    Some(d) => {
                        crate::log::log_info("identity", "resolve", &format!("method=session_id, name={}", inst_name));
                        return Ok(SenderIdentity { kind: SenderKind::Instance, name: inst_name, session_id: Some(sid.to_string()), instance_data: Some(d) });
                    }
                    None => {
                        return Err(AgentxError::NotFound("Instance not found for session_id".to_string()));
                    }
                }
            }
            None => {
                crate::log::log_warn("identity", "resolve.session_id_not_found", &format!("session_id={}", &sid[..sid.len().min(8)]));
                return Err(AgentxError::NotFound("Instance not found for session_id".to_string()));
            }
        }
    }

    // 3. Strict instance lookup (--iam NAME)
    if let Some(n) = name
        && !n.is_empty()
    {
        return resolve_from_name(db, n);
    }

    // 4. Auto-detect from process binding (agentx-launched instances)
    if let Some(pid) = process_id
        && !pid.is_empty()
    {
        let bound_name = db.get_process_binding(pid).map_err(|e| AgentxError::DatabaseError(e.to_string()))?;

        match bound_name {
            Some(inst_name) => {
                let data = db.get_instance(&inst_name).map_err(|e| AgentxError::DatabaseError(e.to_string()))?;

                match data {
                    Some(d) => {
                        let has_session = d.get("session_id").and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty());

                        // Opportunistic Codex session binding for command-time recovery.
                        // Native SessionStart is the primary binding path; this keeps
                        // resume/orphan flows tolerant if a command arrives first.
                        // Uses bind_session_to_process for proper resume/placeholder handling.
                        let mut final_name = inst_name.clone();
                        if !has_session
                            && let Some(thread_id) = codex_thread_id
                            && !thread_id.is_empty()
                            && let Some(resolved) = crate::agents::binding::bind_session_to_process(db, thread_id, process_id)
                        {
                            final_name = resolved;
                        }

                        // Re-read instance data — session_id may have been set during binding
                        let final_data = db.get_instance(&final_name).map_err(|e| AgentxError::DatabaseError(e.to_string()))?.unwrap_or(d);

                        let sid = final_data.get("session_id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string());

                        crate::log::log_info("identity", "resolve", &format!("method=process_binding, name={}, process_id={}", final_name, pid));
                        return Ok(SenderIdentity { kind: SenderKind::Instance, name: final_name, session_id: sid, instance_data: Some(final_data) });
                    }
                    None => {
                        crate::log::log_warn("identity", "resolve.process_instance_missing", &format!("process_id={}, bound_name={}", pid, inst_name));
                        return Err(AgentxError::NotFound(instance_not_found_error_for(db, &inst_name)));
                    }
                }
            }
            None => {
                if identity_expected {
                    crate::log::log_warn("identity", "resolve.process_binding_expired", &format!("process_id={}", pid));
                }
                return Err(AgentxError::IdentityRequired("Session expired. Run 'agentx start' to reconnect.".to_string()));
            }
        }
    }

    // 5. No identity
    if identity_expected {
        crate::log::log_warn("identity", "resolve.no_identity", &format!("has_process_id={}, has_name={}", process_id.is_some_and(|s| !s.is_empty()), name.is_some_and(|s| !s.is_empty()),));
    }
    Err(AgentxError::IdentityRequired("No agentx identity. Run 'agentx start' to connect, or use --iam <yourname> for an existing participant.".to_string()))
}

#[cfg(test)]
pub fn resolve_identity(db: &AgentxDb, name: Option<&str>, system_sender: Option<&str>, session_id: Option<&str>, process_id: Option<&str>, codex_thread_id: Option<&str>) -> Result<SenderIdentity, AgentxError> {
    resolve_identity_with_expectation(db, name, system_sender, session_id, process_id, codex_thread_id, crate::shared::is_inside_ai_tool())
}

/// Check if a command requires identity gating.
pub fn requires_identity(cmd: &str) -> bool {
    REQUIRE_IDENTITY.contains(&cmd)
}

/// Identity gate check for CLI commands.
///
/// Returns `Ok(())` if the identity requirement is satisfied, or `Err` with
/// an actionable error message. The `send` command bypasses the gate when
/// external sender flags (`--from`, `-b`) are present.
pub fn require_identity_gate(cmd: &str, explicit_name: Option<&str>, has_from_flag: bool) -> Result<(), AgentxError> {
    if !requires_identity(cmd) {
        return Ok(());
    }

    // --iam provided: identity will be resolved later
    if explicit_name.is_some() {
        return Ok(());
    }

    // send command: --from or -b bypasses identity requirement
    if cmd == "send" && has_from_flag {
        return Ok(());
    }

    Err(AgentxError::IdentityRequired(format!("'{cmd}' requires identity. Use --iam <yourname> or run inside an agentx-launched session.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_db() -> (AgentxDb, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();
        (db, dir)
    }

    fn insert_instance(db: &AgentxDb, name: &str, session_id: Option<&str>) {
        let now = chrono::Utc::now().timestamp() as f64;
        db.conn()
            .execute(
                "INSERT INTO instances (name, session_id, status, created_at, tool)
             VALUES (?1, ?2, 'active', ?3, 'claude')",
                rusqlite::params![name, session_id, now],
            )
            .unwrap();
    }

    fn insert_process_binding(db: &AgentxDb, process_id: &str, instance_name: &str) {
        let now = chrono::Utc::now().timestamp() as f64;
        db.conn()
            .execute(
                "INSERT INTO process_bindings (process_id, instance_name, updated_at)
             VALUES (?1, ?2, ?3)",
                rusqlite::params![process_id, instance_name, now],
            )
            .unwrap();
    }

    fn insert_session_binding(db: &AgentxDb, session_id: &str, instance_name: &str) {
        let now = chrono::Utc::now().timestamp() as f64;
        db.conn()
            .execute(
                "INSERT INTO session_bindings (session_id, instance_name, created_at)
             VALUES (?1, ?2, ?3)",
                rusqlite::params![session_id, instance_name, now],
            )
            .unwrap();
    }

    // ── Name validation tests ──────────────────────────────────────────

    #[test]
    fn test_looks_like_uuid() {
        assert!(looks_like_uuid("12345678-1234-1234-1234-123456789abc"));
        assert!(looks_like_uuid("ABCDEF01-2345-6789-ABCD-EF0123456789"));
        assert!(!looks_like_uuid("not-a-uuid"));
        assert!(!looks_like_uuid("12345678-1234-1234-1234-12345678"));
    }

    #[test]
    fn test_looks_like_agent_id() {
        assert!(looks_like_agent_id("a6d9caf"));
        assert!(looks_like_agent_id("1234567"));
        assert!(!looks_like_agent_id("a6d9ca")); // too short
        assert!(!looks_like_agent_id("a6d9cafg")); // too long
        assert!(!looks_like_agent_id("a6d9caz")); // non-hex
    }

    #[test]
    fn test_is_valid_name() {
        assert!(is_valid_name("luna"));
        assert!(is_valid_name("test_name_123"));
        assert!(!is_valid_name("Luna")); // uppercase
        assert!(!is_valid_name("my-name")); // dash
        assert!(!is_valid_name("")); // empty
        assert!(!is_valid_name("name with space"));
    }

    #[test]
    fn test_instance_not_found_error() {
        // Normal name: suggest --as
        let err = instance_not_found_error("luna");
        assert!(err.contains("--as luna"));

        // Agent ID: don't suggest --as
        let err = instance_not_found_error("a6d9caf");
        assert!(err.contains("Stop working"));
        assert!(!err.contains("--as"));
    }

    #[test]
    fn test_validate_name_input() {
        // Valid
        assert!(validate_name_input("luna", 50, true).is_ok());
        assert!(validate_name_input("test_name", 50, true).is_ok());

        // Too long
        let long_name = "a".repeat(51);
        let err = validate_name_input(&long_name, 50, true).unwrap_err();
        assert!(err.contains("too long"));

        // Dangerous chars
        let err = validate_name_input("name;evil", 50, true).unwrap_err();
        assert!(err.contains("invalid characters"));

        // @ allowed by default
        assert!(validate_name_input("@luna", 50, true).is_ok());

        // @ rejected when allow_at=false
        assert!(validate_name_input("@luna", 50, false).is_err());
    }

    // ── resolve_from_name tests ────────────────────────────────────────

    #[test]
    fn test_resolve_from_name_exact() {
        let (db, _dir) = make_test_db();
        insert_instance(&db, "luna", Some("sess-1"));

        let identity = resolve_from_name(&db, "luna").unwrap();
        assert_eq!(identity.name, "luna");
        assert!(matches!(identity.kind, SenderKind::Instance));
        assert_eq!(identity.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn test_resolve_from_name_agent_id() {
        let (db, _dir) = make_test_db();
        insert_instance(&db, "luna", None);
        // Set agent_id
        db.conn().execute("UPDATE instances SET agent_id = '12345678-1234-1234-1234-123456789abc' WHERE name = 'luna'", []).unwrap();

        let identity = resolve_from_name(&db, "12345678-1234-1234-1234-123456789abc").unwrap();
        assert_eq!(identity.name, "luna");
    }

    #[test]
    fn test_resolve_from_name_not_found() {
        let (db, _dir) = make_test_db();

        let err = resolve_from_name(&db, "nonexistent").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_resolve_from_name_invalid() {
        let (db, _dir) = make_test_db();

        let err = resolve_from_name(&db, "Invalid-Name!").unwrap_err();
        assert!(err.to_string().contains("Invalid instance name"));
    }

    // ── resolve_identity tests ─────────────────────────────────────────

    #[test]
    fn test_resolve_identity_system_sender() {
        let (db, _dir) = make_test_db();

        let identity = resolve_identity(&db, None, Some("agentx-launcher"), None, None, None).unwrap();
        assert!(matches!(identity.kind, SenderKind::System));
        assert_eq!(identity.name, "agentx-launcher");
    }

    #[test]
    fn test_resolve_identity_session_id() {
        let (db, _dir) = make_test_db();
        insert_instance(&db, "luna", Some("sess-1"));
        insert_session_binding(&db, "sess-1", "luna");

        let identity = resolve_identity(&db, None, None, Some("sess-1"), None, None).unwrap();
        assert!(matches!(identity.kind, SenderKind::Instance));
        assert_eq!(identity.name, "luna");
        assert_eq!(identity.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn test_resolve_identity_name() {
        let (db, _dir) = make_test_db();
        insert_instance(&db, "luna", None);

        let identity = resolve_identity(&db, Some("luna"), None, None, None, None).unwrap();
        assert_eq!(identity.name, "luna");
    }

    #[test]
    fn test_resolve_identity_process_binding() {
        let (db, _dir) = make_test_db();
        insert_instance(&db, "luna", Some("sess-1"));
        insert_process_binding(&db, "pid-123", "luna");

        let identity = resolve_identity(&db, None, None, None, Some("pid-123"), None).unwrap();
        assert_eq!(identity.name, "luna");
        assert_eq!(identity.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn test_resolve_identity_process_binding_codex_session_bind() {
        let (db, _dir) = make_test_db();
        // Instance without session_id
        insert_instance(&db, "luna", None);
        insert_process_binding(&db, "pid-123", "luna");

        let identity = resolve_identity(&db, None, None, None, Some("pid-123"), Some("thread-abc")).unwrap();
        assert_eq!(identity.name, "luna");
        // Session should now be bound
        assert_eq!(identity.session_id.as_deref(), Some("thread-abc"));

        // Verify binding was persisted
        let bound = db.get_session_binding("thread-abc").unwrap();
        assert_eq!(bound, Some("luna".to_string()));
    }

    #[test]
    fn test_resolve_identity_process_binding_expired() {
        let (db, _dir) = make_test_db();
        // No process binding exists

        let err = resolve_identity(&db, None, None, None, Some("pid-123"), None).unwrap_err();
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn test_resolve_identity_no_identity() {
        let (db, _dir) = make_test_db();

        let err = resolve_identity(&db, None, None, None, None, None).unwrap_err();
        assert!(err.to_string().contains("No agentx identity"));
    }

    #[test]
    fn test_resolve_identity_priority_system_over_name() {
        let (db, _dir) = make_test_db();
        insert_instance(&db, "luna", None);

        // system_sender takes priority over name
        let identity = resolve_identity(&db, Some("luna"), Some("agentx-launcher"), None, None, None).unwrap();
        assert!(matches!(identity.kind, SenderKind::System));
        assert_eq!(identity.name, "agentx-launcher");
    }

    #[test]
    fn test_resolve_identity_priority_session_over_name() {
        let (db, _dir) = make_test_db();
        insert_instance(&db, "luna", Some("sess-1"));
        insert_instance(&db, "nova", Some("sess-2"));
        insert_session_binding(&db, "sess-1", "luna");

        // session_id takes priority over name
        let identity = resolve_identity(&db, Some("nova"), None, Some("sess-1"), None, None).unwrap();
        assert_eq!(identity.name, "luna");
    }

    // ── Identity gating tests ──────────────────────────────────────────

    #[test]
    fn test_requires_identity() {
        assert!(requires_identity("send"));
        assert!(requires_identity("listen"));
        assert!(!requires_identity("list"));
        assert!(!requires_identity("status"));
        assert!(!requires_identity("events"));
    }

    #[test]
    fn test_identity_gate_non_gated_command() {
        assert!(require_identity_gate("list", None, false).is_ok());
    }

    #[test]
    fn test_identity_gate_with_name() {
        assert!(require_identity_gate("send", Some("luna"), false).is_ok());
    }

    #[test]
    fn test_identity_gate_send_with_from() {
        assert!(require_identity_gate("send", None, true).is_ok());
    }

    #[test]
    fn test_identity_gate_send_no_identity() {
        let err = require_identity_gate("send", None, false).unwrap_err();
        assert!(err.to_string().contains("requires identity"));
    }

    #[test]
    fn test_identity_gate_listen_no_identity() {
        let err = require_identity_gate("listen", None, false).unwrap_err();
        assert!(err.to_string().contains("requires identity"));
    }

    // ── Codex resume regression tests ──────────────────────────────────

    #[test]
    fn test_codex_session_bind_resume_switches_to_canonical() {
        // Regression: Codex thread_id matches an existing session binding (resume scenario).
        // Must switch identity to canonical instance, not stay on placeholder.
        let (db, _dir) = make_test_db();
        insert_instance(&db, "canonical", Some("thread-resume"));
        insert_session_binding(&db, "thread-resume", "canonical");
        insert_instance(&db, "placeholder", None);
        // Real launch placeholders are pending/new (PLACEHOLDER_STATUS/CONTEXT); set that
        // here so deletion is gated on genuine placeholder-ness, not merely a null session.
        db.conn().execute("UPDATE instances SET status = 'pending', status_context = 'new' WHERE name = 'placeholder'", []).unwrap();
        insert_process_binding(&db, "pid-codex", "placeholder");

        let identity = resolve_identity(&db, None, None, None, Some("pid-codex"), Some("thread-resume")).unwrap();

        // Must resolve to canonical, not placeholder
        assert_eq!(identity.name, "canonical");
        assert_eq!(identity.session_id.as_deref(), Some("thread-resume"));

        // Placeholder should be deleted (was true placeholder)
        assert!(db.get_instance("placeholder").unwrap().is_none());
    }

    #[test]
    fn test_codex_session_bind_already_bound_same_instance() {
        // Session already bound to the same instance we're on → no-op, no crash
        let (db, _dir) = make_test_db();
        insert_instance(&db, "luna", Some("thread-same"));
        insert_session_binding(&db, "thread-same", "luna");
        insert_process_binding(&db, "pid-codex", "luna");

        let identity = resolve_identity(&db, None, None, None, Some("pid-codex"), Some("thread-same")).unwrap();

        assert_eq!(identity.name, "luna");
        assert_eq!(identity.session_id.as_deref(), Some("thread-same"));
    }

    // ── Missing-agent descriptions ─────────────────────────────────────

    fn log_stop(db: &AgentxDb, name: &str, by: &str, reason: &str) {
        let snapshot = serde_json::json!({"tool": "codex", "directory": "/tmp/work"});
        db.log_life_event(name, "stopped", by, reason, Some(snapshot)).unwrap();
    }

    #[test]
    fn last_stopped_reports_latest_stop_and_ignores_live_agents() {
        let (db, _dir) = make_test_db();
        log_stop(&db, "tuna", "pty", "closed");
        log_stop(&db, "tuna", "muse", "killed");

        let stopped = last_stopped(&db, "tuna").expect("stopped agent");
        assert_eq!(stopped.by, "muse");
        assert_eq!(stopped.reason, "killed");
        let summary = stopped.summary();
        assert!(summary.starts_with("stopped "), "{summary}");
        assert!(summary.contains("(killed by muse) · codex · /tmp/work"), "{summary}");

        insert_instance(&db, "tuna", None);
        assert!(last_stopped(&db, "tuna").is_none(), "live agent is not stopped");
    }

    #[test]
    fn describe_missing_agent_shows_crash_exit_code_and_output() {
        let (db, _dir) = make_test_db();
        let data = serde_json::json!({
            "action": "stopped", "by": "pty", "reason": "closed", "exit_code": 1,
            "output": "TypeError: boom\n    at onResponseStart",
            "snapshot": {"tool": "pi"},
        });
        db.log_event("life", "tuna", &data).unwrap();

        let msg = describe_missing_agent(&db, "tuna");
        assert!(msg.contains("(closed by pty) · exit 1 · pi"), "{msg}");
        assert!(msg.contains("\n  Last output:\n    TypeError: boom\n        at onResponseStart\n  Resume:"), "{msg}");
    }

    #[test]
    fn describe_missing_agent_explains_stopped_and_suggests_typos() {
        let (db, _dir) = make_test_db();
        log_stop(&db, "tuna", "muse", "killed");
        insert_instance(&db, "nazu", None);

        let stopped = describe_missing_agent(&db, "tuna");
        assert!(stopped.starts_with("'tuna' stopped "), "{stopped}");
        assert!(stopped.contains("agentx r tuna"), "{stopped}");

        let typo = describe_missing_agent(&db, "tnua");
        assert!(typo.starts_with("No agent named 'tnua'"), "{typo}");
        assert!(typo.contains("Did you mean: tuna?"), "{typo}");

        let unknown = describe_missing_agent(&db, "zzzz");
        assert!(!unknown.contains("Did you mean"), "{unknown}");
        assert!(unknown.contains("agentx list --stopped"), "{unknown}");
    }
}
