//! `agentx config` command — view and edit configuration.
//!
//!
//! Supports: show all, get/set single key, --json, --edit, --reset,
//! per-instance config (-i), terminal preset management.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

#[cfg(test)]
use crate::agents::identity;
#[cfg(test)]
use crate::agents::records as instances;
#[cfg(test)]
use crate::db::AgentxDb;
#[cfg(test)]
use crate::shared::CommandContext;

/// Originating file context. Only durable participant/dev-root projections need
/// a Session; pure configuration actions never acquire a database daemon.
struct ConfigHost<'a> {
    scope: PathBuf,
    invocation: &'a crate::service::InvocationContext,
    session: Option<&'a crate::cli::client::Session>,
}
impl ConfigHost<'_> {
    fn path(&self) -> PathBuf {
        self.scope.join("config.toml")
    }
    fn get(&self, key: &str) -> (String, &'static str) {
        crate::config::file::get_for_scope(&self.scope, &self.invocation.environment, key)
    }
    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        crate::config::file::set_for_scope(&self.scope, key, value)
    }
    fn unset(&self, key: &str) -> Result<(), String> {
        crate::config::file::unset_for_scope(&self.scope, key)
    }
    fn config(&self) -> crate::config::AgentxConfig {
        crate::config::AgentxConfig::load_for_scope(&self.scope, &self.invocation.environment)
    }
    fn session(&self) -> Result<&crate::cli::client::Session, String> {
        self.session.ok_or_else(|| "this configuration operation needs a native session".into())
    }
}

pub(crate) fn try_local(args: &ConfigArgs) -> Option<i32> {
    let needs_session = args.instance.is_some()
        || (!args.edit && !args.reset && args.key.is_none())
        || (args.key.as_deref().is_some_and(|key| normalize_key(key) == "AGENTX_DEV_ROOT") && !args.info)
        || (args.key.as_deref() == Some("bootstrap") && args.value.as_deref() == Some("render"));
    if needs_session {
        return None;
    }
    let invocation = match crate::service::InvocationContext::capture() {
        Ok(invocation) => invocation,
        Err(error) => {
            eprintln!("Error: {error}");
            return Some(1);
        }
    };
    let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
    Some(cmd_config_impl(&ConfigHost { scope, invocation: &invocation, session: None }, args))
}
pub(crate) fn cmd_config_client(session: &crate::cli::client::Session, args: &ConfigArgs) -> i32 {
    cmd_config_impl(&ConfigHost { scope: session.client.scope_root().into(), invocation: &session.invocation, session: Some(session) }, args)
}

fn config_dev_root_client(host: &ConfigHost, value: Option<&str>, unset: bool, json_mode: bool, info: bool) -> i32 {
    if info {
        println!(
            "dev_root - Persistent dev-worktree fallback for binary re-exec\n\nStored under the active AGENTX_DIR. AGENTX_DEV_ROOT environment takes precedence.\n\n  agentx config dev_root /path/to/worktree\n  agentx config dev_root\n  agentx config dev_root --unset"
        );
        return 0;
    }
    let run = || -> Result<(), String> {
        use crate::service::{Request, Response, maintenance as M};
        let client = &host.session()?.client;
        if unset || value.is_some() {
            let request = M::Request::SetDevRoot { value: if unset { None } else { value.map(str::to_owned) } };
            match client.call(Request::Maintenance(request)).map_err(|error| error.to_string())? {
                Response::Maintenance(M::Response::Updated) => {}
                _ => return Err("daemon returned a different config mutation".into()),
            }
            if unset {
                println!("Unset dev_root");
            } else {
                println!("Set dev_root = {}", value.unwrap_or(""));
            }
        } else {
            let Response::Maintenance(M::Response::DevRoot(value)) = client.call(Request::Maintenance(M::Request::DevRoot)).map_err(|error| error.to_string())? else {
                return Err("daemon returned a different config projection".into());
            };
            if json_mode {
                println!("{}", json!({"dev_root":value}));
            } else if let Some(value) = value {
                println!("{value}");
            } else {
                println!("dev_root: (not set)");
            }
        }
        Ok(())
    };
    match run() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("Error: {error}");
            1
        }
    }
}

fn effective_dev_root(host: &ConfigHost) -> Result<Option<(PathBuf, &'static str)>, String> {
    if let Some(value) = host.invocation.environment.get("AGENTX_DEV_ROOT").filter(|value| !value.is_empty()) {
        return Ok(Some((value.into(), "env")));
    }
    use crate::service::{Request, Response, maintenance as M};
    match host.session()?.client.call(Request::Maintenance(M::Request::DevRoot)).map_err(|error| error.to_string())? {
        Response::Maintenance(M::Response::DevRoot(value)) => Ok(value.filter(|value| !value.is_empty()).map(|value| (value.into(), "kv"))),
        _ => Err("daemon returned a different dev-root projection".into()),
    }
}

fn config_instance_client(host: &ConfigHost, target: &str, args: &[String], json_mode: bool) -> Result<i32, String> {
    let session = host.session()?;
    let records = session.records().map_err(|error| error.to_string())?;
    let name = if target == "self" { session.actor.as_ref().ok_or("Cannot resolve 'self' — no active identity")?.record.name.as_str() } else { target };
    let exact = records.iter().find(|row| row.name == name);
    let row = if let Some(row) = exact {
        row
    } else {
        let mut matches = records.iter().filter(|row| row.name.starts_with(name)).collect::<Vec<_>>();
        matches.sort_by(|left, right| left.name.cmp(&right.name));
        match matches.as_slice() {
            [row] => *row,
            [] => return Err(format!("No agent named '{name}'")),
            _ => return Err(format!("Ambiguous agent prefix '{name}': {}", matches.iter().map(|row| row.name.as_str()).collect::<Vec<_>>().join(", "))),
        }
    };
    let participant = crate::agents::attachments::ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() };
    if target == "self" && session.actor.as_ref().is_none_or(|actor| actor.participant != participant) {
        return Err("self participant was replaced".into());
    }
    let key = args.first().map(String::as_str);
    let value = (args.len() > 1).then(|| args[1..].join(" "));
    if let Some((base, device)) = crate::relay::control::split_device_suffix(&row.name) {
        let action = if value.is_some() { crate::relay::control::rpc_action::CONFIG_SET } else { crate::relay::control::rpc_action::CONFIG_GET };
        let result = crate::relay::control::dispatch_remote(&session.client, device, Some(&row.name), action, &json!({"instance":base,"field":key,"value":value}), crate::relay::control::RPC_DEFAULT_TIMEOUT)?;
        if let Some(value) = value {
            println!("{}", render_config_instance_set_feedback(result["instance"].as_str().unwrap_or(base), key.unwrap_or(""), &value));
        } else {
            println!("{}", render_config_instance_get(&result, key, json_mode));
        }
        return Ok(0);
    }
    if let Some(value) = value {
        use crate::agents::lifecycle_service::{Request as L, Response as LR};
        let seconds = || value.parse::<i64>().ok().filter(|seconds| *seconds > 0).ok_or_else(|| format!("{} must be a positive integer (seconds)", key.unwrap_or("timeout")));
        let request = match key {
            Some("timeout") => L::SetTimeout { participant, timeout: if value.is_empty() || value.eq_ignore_ascii_case("default") { host.config().timeout } else { seconds()? } },
            Some("hints") => L::SetHints { participant, hints: (!value.is_empty()).then(|| value.clone()) },
            Some("subagent_timeout") => L::SetSubagentTimeout { participant, timeout: if value.is_empty() || value.eq_ignore_ascii_case("default") { None } else { Some(seconds()?) } },
            _ => return Err(format!("Unknown instance config key '{}'", key.unwrap_or(""))),
        };
        match session.client.call(crate::service::Request::Participants(crate::agents::service::Request::Lifecycle(request))).map_err(|error| error.to_string())? {
            crate::service::Response::Participants(crate::agents::service::Response::Lifecycle(LR::Changed)) => {}
            _ => return Err("daemon returned a different instance config mutation".into()),
        }
        println!("{}", render_config_instance_set_feedback(&row.name, key.unwrap_or(""), &value));
        crate::relay::trigger_push_client(&session.client);
    } else {
        let config = match key {
            None => build_instance_config_json(row),
            Some("timeout") => json!({"value":row.wait_timeout.unwrap_or(86400)}),
            Some("hints") => json!({"value":row.hints.as_deref().unwrap_or("")}),
            Some("subagent_timeout") => json!({"value":row.subagent_timeout}),
            Some(other) => return Err(format!("Unknown instance config key '{other}'")),
        };
        println!("{}", render_config_instance_get(&config, key, json_mode));
    }
    Ok(0)
}

/// Parsed arguments for `agentx config`.
///
/// Note: multi-word or dash-prefixed values should be quoted:
///   `agentx config codex_args "--model o3"`
#[derive(clap::Parser, Debug)]
#[command(name = "config", about = "View and edit configuration")]
pub struct ConfigArgs {
    /// Config key (e.g., "terminal", "AGENTX_TIMEOUT")
    pub key: Option<String>,
    /// Value to set (quote multi-word or dash-prefixed values)
    #[arg(allow_hyphen_values = true)]
    pub value: Option<String>,
    /// Additional arguments for `config bootstrap show/render`.
    pub extra: Vec<String>,
    /// JSON output
    #[arg(long)]
    pub json: bool,
    /// Open config in editor
    #[arg(long)]
    pub edit: bool,
    /// Unset value (back to the default)
    #[arg(long)]
    pub unset: bool,
    /// Reset config
    #[arg(long)]
    pub reset: bool,
    /// Show key description
    #[arg(long)]
    pub info: bool,
    /// Terminal setup mode
    #[arg(long)]
    pub setup: bool,
    /// Per-instance config
    #[arg(short = 'i')]
    pub instance: Option<String>,
}

/// Instance-level config keys.
#[cfg(test)]
const INSTANCE_KEYS: &[(&str, &str)] = &[("timeout", "Instance-specific idle timeout in seconds"), ("hints", "Instance-specific hints (injected with messages)"), ("subagent_timeout", "Instance-specific subagent timeout in seconds")];

// ── Config File Operations ───────────────────────────────────────────────

pub fn config_set(key: &str, value: &str) -> Result<(), String> {
    crate::config::file::set_for_scope(&crate::paths::agentx_dir(), key, value)
}
pub fn config_unset(key: &str) -> Result<(), String> {
    crate::config::file::unset_for_scope(&crate::paths::agentx_dir(), key)
}
pub fn config_set_and_apply(key: &str, value: &str) -> Result<(), String> {
    config_set(key, value)?;
    if crate::config::user_field(key).is_some_and(|field| field.name == "auto_approve") && !update_auto_approve_permissions() {
        return Err("failed to update auto-approve permissions".into());
    }
    Ok(())
}
pub fn config_get(key: &str) -> (String, &'static str) {
    crate::config::file::get_for_scope(&crate::paths::agentx_dir(), &std::env::vars().collect(), key)
}
#[cfg(test)]
use crate::config::file::config_set_at_path;

// ── Instance Config ──────────────────────────────────────────────────────

#[cfg(test)]
fn unique_instance_prefix(db: &AgentxDb, name: &str) -> Result<Option<String>, String> {
    let escaped = name.replace('!', "!!").replace('%', "!%").replace('_', "!_");
    let mut stmt = db.conn().prepare("SELECT name FROM instances WHERE name LIKE ? ESCAPE '!' ORDER BY name").map_err(|e| e.to_string())?;
    let candidates = stmt.query_map([format!("{escaped}%")], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())?;
    match candidates.as_slice() {
        [] => Ok(None),
        [only] => Ok(Some(only.clone())),
        _ => Err(format!("Ambiguous agent prefix '{name}': {}", candidates.join(", "))),
    }
}

pub fn render_config_instance_get(value: &Value, key: Option<&str>, json_mode: bool) -> String {
    if json_mode {
        return serde_json::to_string(value).unwrap_or_default();
    }

    match key {
        None => {
            let display = value.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let timeout = value.get("timeout").and_then(|v| v.as_i64()).unwrap_or(86400);
            let hints = value.get("hints").and_then(|v| v.as_str()).unwrap_or("");
            let subagent_timeout = value.get("subagent_timeout").and_then(|v| v.as_i64()).map(|t| format!("{t}s")).unwrap_or_else(|| "(default)".to_string());
            format!("Agent: {display}\n  timeout: {timeout}s\n  hints: {}\n  subagent_timeout: {subagent_timeout}", if hints.is_empty() { "(none)" } else { hints },)
        }
        Some(_) => value
            .get("value")
            .map(|v| match v {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string(),
            })
            .unwrap_or_default(),
    }
}

pub fn render_config_instance_set_feedback(instance_name: &str, key: &str, value: &str) -> String {
    match key {
        "timeout" => {
            if value.is_empty() || value.eq_ignore_ascii_case("default") {
                format!("Reset timeout for {instance_name}")
            } else {
                format!("Set timeout for {instance_name}: {value}s")
            }
        }
        "hints" => {
            if value.is_empty() {
                format!("Cleared hints for {instance_name}")
            } else {
                format!("Set hints for {instance_name}")
            }
        }
        "subagent_timeout" => {
            if value.is_empty() || value.eq_ignore_ascii_case("default") {
                format!("Cleared subagent_timeout for {instance_name}")
            } else {
                format!("Set subagent_timeout for {instance_name}: {value}s")
            }
        }
        _ => format!("Updated {key} for {instance_name}"),
    }
}

/// Build the canonical JSON shape returned by both local and remote no-key
/// instance config reads. Stable contract for scripts consuming
/// `agentx config -i <name> --json`:
///   `name`            — base name
///   `hints`           — null when unset (not "")
///   `timeout`         — null when unset (server-side default applies elsewhere)
///   `subagent_timeout`— null when unset
fn build_instance_config_json(instance: &crate::db::InstanceRow) -> serde_json::Value {
    serde_json::json!({
        "name": instance.name,
        "timeout": instance.wait_timeout,
        "hints": instance.hints.as_deref().filter(|s| !s.is_empty()),
        "subagent_timeout": instance.subagent_timeout,
    })
}

#[cfg(test)]
pub fn config_instance_get(db: &AgentxDb, instance_arg: &str, key: Option<&str>) -> Result<Value, String> {
    let name = instance_arg.to_string();
    let instance = db.get_instance_full(&name).map_err(|e| e.to_string())?.ok_or_else(|| identity::describe_missing_agent(db, instance_arg))?;

    Ok(match key {
        None => build_instance_config_json(&instance),
        Some("timeout") => serde_json::json!({"value": instance.wait_timeout.unwrap_or(86400)}),
        Some("hints") => serde_json::json!({"value": instance.hints.as_deref().unwrap_or("")}),
        Some("subagent_timeout") => match instance.subagent_timeout {
            Some(value) => serde_json::json!({"value": value}),
            None => serde_json::json!({"value": serde_json::Value::Null}),
        },
        Some(other) => return Err(format!("Unknown instance config key '{}'", other)),
    })
}

#[cfg(test)]
pub fn config_instance_set(db: &AgentxDb, instance_arg: &str, key: &str, value: &str) -> Result<Value, String> {
    let name = instance_arg.to_string();
    let instance = db.get_instance_full(&name).map_err(|e| e.to_string())?.ok_or_else(|| identity::describe_missing_agent(db, instance_arg))?;
    let inst_name = &instance.name;

    match key {
        "timeout" => {
            let secs = if value.is_empty() || value.eq_ignore_ascii_case("default") {
                crate::config::AgentxConfig::load(None).timeout
            } else {
                match value.parse::<i64>() {
                    Ok(secs) if secs > 0 => secs,
                    _ => return Err("timeout must be a positive integer (seconds)".to_string()),
                }
            };
            db.conn().execute("UPDATE instances SET wait_timeout = ? WHERE name = ?", rusqlite::params![secs, inst_name]).map_err(|e| e.to_string())?;
        }
        "hints" => {
            let mut updates = serde_json::Map::new();
            if value.is_empty() {
                updates.insert("hints".into(), serde_json::Value::Null);
            } else {
                updates.insert("hints".into(), serde_json::json!(value));
            }
            instances::update_instance_position(db, inst_name, &updates);
        }
        "subagent_timeout" => {
            let mut updates = serde_json::Map::new();
            if value.is_empty() || value.eq_ignore_ascii_case("default") {
                updates.insert("subagent_timeout".into(), serde_json::Value::Null);
            } else {
                let secs = value.parse::<i64>().map_err(|_| "subagent_timeout must be a positive integer (seconds)".to_string())?;
                if secs <= 0 {
                    return Err("subagent_timeout must be a positive integer (seconds)".to_string());
                }
                updates.insert("subagent_timeout".into(), serde_json::json!(secs));
            }
            instances::update_instance_position(db, inst_name, &updates);
        }
        other => return Err(format!("Unknown instance config key '{}'", other)),
    }

    crate::runtime::notify::wake_all(db);
    Ok(serde_json::json!({"instance": inst_name, "field": key, "value": value}))
}

// ── Main Entry Point ─────────────────────────────────────────────────────

fn note_bootstrap_exclusion_source(host: &ConfigHost) {
    let file_config = crate::config::AgentxConfig::load_for_scope(&host.scope, &std::collections::HashMap::new());
    let effective = host.config();
    if effective.bootstrap_exclude != file_config.bootstrap_exclude && host.invocation.environment.contains_key("AGENTX_BOOTSTRAP_EXCLUDE") {
        eprintln!("Note: bootstrap_exclude={} from the current environment overrides config.toml ({}).", effective.bootstrap_exclude, file_config.bootstrap_exclude);
    }
}

fn config_bootstrap(host: &ConfigHost, args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("show") => {
            if args.len() > 2 {
                return crate::cli::commands::help::usage_error("config bootstrap", &format!("Unexpected argument for agentx config bootstrap show: '{}'", args[2]));
            }
            note_bootstrap_exclusion_source(host);
            if let Some(part) = args.get(1) {
                match crate::runtime::bootstrap::part_source(part) {
                    Some(source) => println!("{source}"),
                    None => {
                        let hint = crate::shared::suggest::did_you_mean(part, crate::runtime::bootstrap::EXCLUDE_PARTS.iter().copied());
                        return crate::cli::commands::help::usage_error("config bootstrap", &format!("Unknown bootstrap part '{part}'{hint}\nParts: {}", crate::runtime::bootstrap::EXCLUDE_PARTS.join(", ")));
                    }
                }
            } else {
                let config = host.config();
                let excluded = crate::runtime::bootstrap::parse_excluded_parts(&config.bootstrap_exclude).unwrap_or_default();
                for part in crate::runtime::bootstrap::EXCLUDE_PARTS {
                    let state = if excluded.contains(part) { "excluded" } else { "included" };
                    println!("{part} ({state})\n{}\n", crate::runtime::bootstrap::part_source(part).unwrap_or_default());
                }
            }
            0
        }
        Some("render") => {
            if !(3..=4).contains(&args.len()) || (args.len() == 4 && args[3] != "launched") {
                return crate::cli::commands::help::usage_error("config bootstrap", "agentx config bootstrap render needs <name> <tool> [launched]");
            }
            if args[2].parse::<crate::integrations::tool::Tool>().is_err() {
                return crate::cli::commands::help::usage_error("config bootstrap", &format!("Unknown tool '{}'", args[2]));
            }
            note_bootstrap_exclusion_source(host);
            let mut agentx_ctx = crate::shared::AgentxContext::from_env(&host.invocation.environment, host.invocation.cwd.clone());
            agentx_ctx.is_launched = args.len() == 4;
            let session = match host.session() {
                Ok(session) => session,
                Err(error) => {
                    eprintln!("Error: {error}");
                    return 1;
                }
            };
            match crate::runtime::bootstrap::preview_context_client(&session.client, &agentx_ctx, &args[1], &args[2]) {
                Ok(output) => println!("{output}"),
                Err(error) => {
                    eprintln!("Error: {error}");
                    return 1;
                }
            }
            0
        }
        None => crate::cli::commands::help::usage_error("config bootstrap", "agentx config bootstrap needs show or render"),
        Some(other) => crate::cli::commands::help::unknown_subcommand("config bootstrap", other, &["show", "render"]),
    }
}

/// Main entry point for `agentx config` command.
fn cmd_config_impl(host: &ConfigHost, args: &ConfigArgs) -> i32 {
    if args.key.as_deref() != Some("bootstrap") && !args.extra.is_empty() {
        eprintln!("Error: quote multi-word config values");
        return 1;
    }
    if args.key.as_deref() == Some("environment") && args.instance.is_none() {
        if args.value.is_some() || !args.extra.is_empty() || args.edit || args.unset || args.reset || args.setup {
            eprintln!("Error: 'agentx config environment' is a read-only diagnostic");
            return 1;
        }
        if args.json {
            println!("{}", crate::runtime::environment::diagnostic_json());
        } else {
            println!("{}", crate::runtime::environment::diagnostic_text());
        }
        return 0;
    }
    let json_mode = args.json;
    let edit_mode = args.edit;
    let unset_mode = args.unset;
    let reset_mode = args.reset;
    let info_mode = args.info;
    let setup_mode = args.setup;
    let instance_name = args.instance.clone();
    // Reconstruct argv from key + value for backward compat with existing handlers
    let argv: Vec<String> = args.key.iter().cloned().chain(args.value.iter().cloned()).chain(args.extra.iter().cloned()).collect();

    // --setup: only valid with 'terminal kitty' (C4 fix)
    if setup_mode {
        let positional: Vec<&String> = argv.iter().filter(|a| !a.starts_with('-')).collect();
        let is_kitty_terminal = positional.len() >= 2 && positional[0] == "terminal" && positional[1].starts_with("kitty");
        if !is_kitty_terminal {
            eprintln!("Error: --setup is only valid with kitty: agentx config terminal kitty --setup");
            return 1;
        }
    }

    if let Some(instance) = &instance_name {
        return match config_instance_client(host, instance, &argv, json_mode) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("Error: {error}");
                1
            }
        };
    }

    // Edit mode
    if edit_mode {
        let path = host.path();
        // Ensure file exists
        let prepared = if path.exists() { crate::config::lock_down_config_permissions(&path) } else { crate::config::write_config_toml_path(&path, "# agentx configuration\n") };
        if let Err(e) = prepared {
            eprintln!("Error: Failed to prepare config.toml: {e}");
            return 1;
        }
        let editor = host.invocation.environment.get("EDITOR").or_else(|| host.invocation.environment.get("VISUAL")).cloned().unwrap_or_else(|| "vim".into());
        let status = std::process::Command::new(&editor).arg(&path).status();
        match status {
            Ok(s) if s.success() => return 0,
            Ok(s) => {
                eprintln!("Editor exited with {s}");
                return 1;
            }
            Err(e) => {
                eprintln!("Error: Failed to launch editor '{editor}': {e}");
                return 1;
            }
        }
    }

    // Reset mode — delegate to reset.rs for proper timestamped archiving + env file
    if reset_mode {
        return crate::cli::commands::reset::operations::reset_config_for_scope(&host.scope);
    }

    // No args: show all config
    if argv.is_empty() {
        return show_all_config(host, json_mode);
    }

    // File-level failures affect every get path, including special keys.
    if argv.len() == 1 && !unset_mode && !info_mode {
        let (_, errors) = crate::config::AgentxConfig::load_for_scope_with_errors(&host.scope, &host.invocation.environment);
        if let Some(msg) = errors.get("config.toml") {
            eprintln!("Invalid config.toml: {msg}");
        }
    }

    let key_arg = &argv[0];

    if key_arg == "bootstrap" {
        if instance_name.is_some() || edit_mode || unset_mode || reset_mode || setup_mode || info_mode || json_mode {
            eprintln!("Error: unsupported option for 'agentx config bootstrap'");
            return 1;
        }
        return config_bootstrap(host, &argv[1..]);
    }

    if normalize_key(key_arg) == "AGENTX_DEV_ROOT" {
        return config_dev_root_client(host, argv.get(1).map(|s| s.as_str()), unset_mode, json_mode, info_mode);
    }

    // Terminal subcommand
    if key_arg == "terminal" {
        if info_mode {
            print_terminal_info(host);
            return 0;
        }
        let rc = if unset_mode { config_terminal(host, &["default".to_string()], setup_mode) } else { config_terminal(host, &argv[1..], setup_mode) };
        if rc == 0 && (unset_mode || argv.len() > 1) {
            note_env_override(host, "AGENTX_TERMINAL");
        }
        return rc;
    }

    // Check for info request: `config key --info` or `config key info` or `config key ?`
    let wants_info = info_mode || argv.get(1).map(|s| s.as_str()) == Some("info") || argv.get(1).map(|s| s.as_str()) == Some("?");

    // Normalize key to AGENTX_ prefix
    let key = if crate::config::file::environment_key(key_arg) == Some("environment.files") {
        "AGENTX_ENV_FILES".into()
    } else if crate::config::file::environment_key(key_arg).is_some() {
        "AGENTX_ENV_OVERRIDE".into()
    } else {
        normalize_key(key_arg)
    };

    if wants_info && crate::config::file::environment_key(key_arg).is_some() {
        println!(
            "env_files: ordered JSON string array stored as [environment].files; [] disables.\nenv_override: true or false stored as [environment].override; default false.\nCLI --env-file / --env-override / --no-env-override overrides ENV, which overrides TOML.\nInspect selected sources and variable provenance: agentx config environment --json"
        );
        return 0;
    }
    if wants_info {
        return show_key_info(host, &key);
    }

    // Unknown keys would silently read as "(not set)" or be written to
    // config.toml where nothing reads them. Reading an env-only setting that
    // is actually set (e.g. AGENTX_DIR) stays allowed.
    let is_set_mode = argv.len() >= 2 || unset_mode;
    let env_readable = !is_set_mode && host.invocation.environment.contains_key(&key);
    if crate::config::user_field(&key).is_none() && crate::config::file::environment_key(key_arg).is_none() && !env_readable {
        let typed = key_arg.to_lowercase();
        let short: Vec<String> = crate::config::user_fields().map(|f| f.name.to_string()).chain(["dev_root".to_string()]).collect();
        eprintln!("Error: Unknown config key '{key_arg}'{}\nValid keys: {}", crate::shared::suggest::did_you_mean(typed.trim_start_matches("agentx_"), short.iter().map(String::as_str)), short.join(", "));
        return 1;
    }

    // Set mode: config KEY VALUE, or config KEY --unset (back to the default)
    if is_set_mode {
        if unset_mode && argv.len() >= 2 {
            eprintln!("Error: pass a value or --unset, not both");
            return 1;
        }
        // Relay settings change what the relay worker does and must go through
        // `agentx relay`, which validates them and starts/stops the worker to match.
        if key.starts_with("AGENTX_RELAY") {
            eprintln!("Error: {key} is managed by 'agentx relay' (new, connect, on, off).\nRun: agentx relay --help");
            return 1;
        }
        // Join all remaining args with spaces
        let value = argv[1..].join(" ");

        // Validate and write first; side effects run after the confirmation.
        let result = if unset_mode { host.unset(&key) } else { host.set(&key, &value) };
        if let Err(e) = result {
            eprintln!("Error: {e}");
            return 1;
        }
        if unset_mode {
            println!("Unset {key}");
        } else {
            println!("Set {key} = {value}");
        }
        note_env_override(host, &key);
        if key == "AGENTX_AUTO_APPROVE" && !update_auto_approve_permissions_for(host) {
            return 1;
        }
        return 0;
    }

    // Get mode: config KEY
    let (value, _source) = host.get(&key);
    if json_mode {
        let mut m = serde_json::Map::new();
        m.insert(key.clone(), serde_json::Value::String(value.clone()));
        println!("{}", serde_json::Value::Object(m));
    } else if value.is_empty() {
        println!("{key}: (not set)");
    } else {
        println!("{value}");
    }
    // The raw value is shown as set; say when the loader replaces it.
    if let Some(field) = crate::config::user_field(&key)
        && let (config, errors) = crate::config::AgentxConfig::load_for_scope_with_errors(&host.scope, &host.invocation.environment)
        && let Some(msg) = errors.get(field.name)
    {
        let effective = config.get_field(field.name).unwrap_or_default();
        eprintln!("Invalid, using the default ({effective}): {msg}");
    }

    0
}

/// An env var wins over config.toml. Agents launched by agentx carry every
/// setting in env as of their launch, so this is common there.
fn note_env_override(host: &ConfigHost, key: &str) {
    if let Ok(env_val) = host.invocation.environment.get(key).cloned().ok_or(()) {
        eprintln!("Note: {key}={env_val} in this environment overrides config.toml here and for agents launched from here");
    }
}

/// Normalize a key argument to AGENTX_* format.
fn normalize_key(input: &str) -> String {
    let upper = input.to_uppercase();
    if upper.starts_with("AGENTX_") { upper } else { format!("AGENTX_{upper}") }
}

/// Build runtime overrides map from instance DB data.
///
/// Reads per-instance DB values
/// (wait_timeout, hints, subagent_timeout) and returns overrides
/// where the instance value differs from the global config value.
fn get_runtime_overrides(host: &ConfigHost) -> std::collections::HashMap<&'static str, String> {
    let mut values = std::collections::HashMap::new();
    let Some(actor) = host.session.and_then(|session| session.actor.as_ref()) else {
        return values;
    };
    for (key, value) in [("AGENTX_TIMEOUT", actor.record.wait_timeout.map(|value| value.to_string())), ("AGENTX_HINTS", actor.record.hints.clone()), ("AGENTX_SUBAGENT_TIMEOUT", actor.record.subagent_timeout.map(|value| value.to_string()))] {
        if let Some(value) = value
            && value != host.get(key).0
        {
            values.insert(key, value);
        }
    }
    values
}

/// Show all config keys with values and sources.
fn show_all_config(host: &ConfigHost, json_mode: bool) -> i32 {
    let runtime_overrides = get_runtime_overrides(host);
    let dev_root = match effective_dev_root(host) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    let (_, config_errors) = crate::config::AgentxConfig::load_for_scope_with_errors(&host.scope, &host.invocation.environment);
    if let Some(msg) = config_errors.get("config.toml") {
        eprintln!("Invalid config.toml: {msg}");
    }

    if json_mode {
        let mut result = serde_json::Map::new();
        for key in crate::config::user_fields().map(|f| f.env) {
            let (value, _source) = host.get(key);
            // {KEY: value} — mask relay token
            let display = if key == "AGENTX_RELAY_TOKEN" && value.len() > 4 { format!("{}***", &value[..4]) } else { value };
            result.insert(key.to_string(), serde_json::Value::String(display));
        }
        if let Some((path, source)) = dev_root {
            result.insert("dev_root".to_string(), serde_json::Value::String(path.to_string_lossy().into_owned()));
            result.insert("dev_root_source".to_string(), serde_json::Value::String(source.to_string()));
        }
        if !config_errors.is_empty() {
            result.insert("config_errors".to_string(), json!(config_errors));
        }
        println!("{}", serde_json::to_string(&Value::Object(result)).unwrap_or_default());
    } else {
        println!("agentx configuration ({})\n", host.path().display());
        println!("agentx Settings:");
        for key in crate::config::user_fields().map(|f| f.env) {
            // Check runtime override first
            let (display, source) = if let Some(val) = runtime_overrides.get(key) {
                (val.clone(), "runtime")
            } else {
                let (value, source) = host.get(key);
                let display = if value.is_empty() {
                    "(not set)".to_string()
                } else if key == "AGENTX_RELAY_TOKEN" && !value.is_empty() {
                    let visible = value.len().min(4);
                    format!("{}...", &value[..visible])
                } else {
                    value
                };
                (display, source)
            };
            println!("  {key:<28} {display:<30} [{source}]");
        }
        if let Some((path, source)) = dev_root {
            println!("  {:<28} {:<30} [{}]", "dev_root", path.display(), source);
        }
        if !config_errors.is_empty() {
            println!("\nInvalid, using the default instead:");
            for msg in config_errors.values() {
                println!("  {msg}");
            }
        }
        println!("\n[env] = environment, [toml] = config.toml, [runtime] = agent override, [kv] = agentx.db key-value, [default] = default");
        println!("\nEdit: agentx config --edit");
    }
    0
}

/// Show detailed info for a config key.
/// Rich per-key help text.
pub fn config_help(key: &str) -> Option<String> {
    if let Some(spec) = crate::integrations::registry::ALL.iter().find(|s| s.launch.args_env == Some(key)) {
        return Some(format!(
            "{key} - Default args passed to {} on launch\n\nSet:   agentx config {}_args \"<args>\"\nClear: agentx config {}_args \"\"\n\nExplicit CLI arguments replace known singleton defaults (including aliases). Other arguments pass through in order.",
            spec.cli_binary, spec.name, spec.name
        ));
    }
    match key {
        "AGENTX_INSTRUCTION_SOURCES" => Some("Frozen instruction sources (JSON array; TOML instructions.sources). Builtin, text, or file entries are rendered once for the Agent and retained across compact/resume. [] disables fixed instructions. See .docs/agent-context.md."),
        "AGENTX_CONTEXT_SOURCES" => Some("Ordered AgentX prompt sources: a JSON array of objects with unique id and exactly one builtin, text or file. The builtin name is agentx. With instructions.sources configured, these are dynamic sources refreshed at initial/compact boundaries; [] disables dynamic content. Without it, legacy builtin entries become frozen instructions and text/file entries are dynamic. Relative files use the caller working directory. TOML key: context.sources. See .docs/agent-context.md."),
        "AGENTX_HEADLESS_TERMINAL" => Some("Background host for --headless: pty (default) or tmux.\nSet: agentx config headless_terminal tmux\nOverride one launch: agentx launch --tool codex --headless --terminal pty\nNew-window selection uses the separate terminal setting."),
        "AGENTX_TMUX_SOCKET" => Some("Tmux server socket. Relative paths resolve under AGENTX_DIR.\nDefault: $AGENTX_DIR/.tmp/tmux/socket\nSet: agentx config tmux_socket /short/private/agentx.sock\nExisting Agents retain their recorded server; this affects new launches."),
        "AGENTX_TMUX_CONFIG" => Some("Native tmux configuration file. Relative paths resolve under AGENTX_DIR.\nDefault: managed .tmp/tmux/default.conf with history-limit 100000 and mouse on.\nSet: agentx config tmux_config tmux.conf\nTmux reads this file when its server starts; editing it does not reload a running server."),

        "AGENTX_HINTS" => Some(
            "\
AGENTX_HINTS - Text injected with all messages

Purpose:
  Appends text to every message received by launched agents.
  Useful for persistent instructions or context.

Usage:
  agentx config hints \"Always respond in JSON format\"
  agentx config hints \"\"   # Clear hints

Example:
  agentx config hints \"Coordinate with @luna and @nova.\"

Notes:
  - Hints are appended to message content, not system prompt
  - Each agent can have different hints (set via agentx config -i <name> hints)
  - Global hints apply to all new launches",
        ),

        "AGENTX_NOTES" => Some(
            "\
AGENTX_NOTES - Notes appended to the built-in context guide

  Custom text added to agent system context at startup.
  Unlike AGENTX_HINTS (per-message), this is injected once and does not repeat.

Usage:
  agentx config notes \"Always check agentx list before spawning new agents\"
  agentx config notes \"\"                            # Clear
  AGENTX_NOTES=\"tips\" agentx launch --tool claude --count 1                 # Per-launch override

  Notes are frozen with the built-in instructions when a new Agent is created; changing notes does not rewrite existing Agents.",
        ),

        "AGENTX_BOOTSTRAP_EXCLUDE" => Some(
            "\
AGENTX_BOOTSTRAP_EXCLUDE - Comma-separated bootstrap parts to leave out

Parts: identity, messages, capabilities, rules, handoff
Default: none
Use all to exclude every built-in part.
Excluded parts may remove instructions agents need to use agentx correctly.
Individual parts affect the main bootstrap; all also suppresses the Claude subagent bootstrap.

  agentx config bootstrap_exclude rules
  agentx config bootstrap_exclude rules,handoff
  agentx config bootstrap_exclude all
  agentx config bootstrap_exclude none
  AGENTX_BOOTSTRAP_EXCLUDE=rules agentx launch --tool claude --count 1

Inspect the exact parts and rendered result:
  agentx config bootstrap show [part]
  agentx config bootstrap render <name> <tool> [launched]

Notes remain appended and take precedence over other agentx instructions.",
        ),

        "AGENTX_TIMEOUT" => Some(
            "\
AGENTX_TIMEOUT - Advanced: idle timeout for headless Claude (seconds)

Default: 86400 (24 hours)

This setting only applies to:
  - Headless Claude: agentx launch --tool claude --count N -- -p

Does NOT apply to:
  - Interactive PTY mode: agentx launch --tool claude --count N (main path)
  - Gemini or Codex

How it works:
  - Claude's Stop hook runs when Claude goes idle
  - Hook waits up to TIMEOUT seconds for a message
  - If no message within timeout, instance is unregistered

Usage (if needed):
  agentx config AGENTX_TIMEOUT 3600   # 1 hour
  export AGENTX_TIMEOUT=3600        # via environment",
        ),

        "AGENTX_SUBAGENT_TIMEOUT" => Some(
            "\
AGENTX_SUBAGENT_TIMEOUT - Timeout for Claude subagents (seconds)

Default: 30

Purpose:
  How long Claude waits for a subagent (Task tool) to complete.
  Shorter than main timeout since subagents should be quick.

Usage:
  agentx config subagent_timeout 60    # 1 minute
  agentx config subagent_timeout 30    # 30 seconds (default)

Notes:
  - Only applies to Claude Code's Task tool spawned agents
  - Parent agent blocks until subagent completes or times out
  - Increase for complex subagent tasks",
        ),

        "AGENTX_CLAUDE_DELIVERY" => Some(
            "\
AGENTX_CLAUDE_DELIVERY - How agentx wakes idle Claude agents

  agentx config claude_delivery inject     # type into the terminal
  agentx config claude_delivery --unset    # back to the default (native)
  AGENTX_CLAUDE_DELIVERY=inject agentx launch --tool claude  # one launch

native  bundled Claude mod (loaded with --plugin-dir) submits the wake (default)
inject  PTY wrapper types the wake into Claude's prompt

Native needs Claude Code 2.1.287+ (mod support). Managed disableSideloadFlags
rejects --plugin-dir and the launch fails; an older Claude, allowManagedModsOnly
or a disabled mod rollout leave the mod unloaded, so the launch never reports
ready and the agent shows as unavailable in list/TUI. Use inject in both cases.
Applies to new launches; kill and resume to change an agent.
Print (-p) sessions use hook delivery either way.",
        ),

        "AGENTX_RELAY" => Some(
            "\
AGENTX_RELAY - MQTT broker URL

Empty = use public brokers (broker.emqx.io, broker.hivemq.com, test.mosquitto.org).
Set automatically by 'agentx relay new' (pins first working broker).
Stored in [relay] in config.toml. Environment overrides are ignored for relay fields.

Private broker: agentx relay new --broker mqtts://host:port",
        ),

        "AGENTX_RELAY_ID" => Some(
            "\
AGENTX_RELAY_ID - Shared UUID for relay group

Generated by 'agentx relay new'. Other devices join with 'agentx relay connect <token>'.
Stored in [relay] in config.toml. Environment overrides are ignored for relay fields.
All devices with the same relay_id sync state via MQTT pub/sub.",
        ),

        "AGENTX_RELAY_TOKEN" => Some(
            "\
AGENTX_RELAY_TOKEN - Auth token for MQTT broker

Optional. Set via 'agentx relay new|connect --password <secret>'.
Stored in [relay] in config.toml. Environment overrides are ignored for relay fields.
Only needed if your broker requires authentication.",
        ),

        "AGENTX_AUTO_APPROVE" => Some(
            "\
AGENTX_AUTO_APPROVE - Auto-approve safe agentx commands

Purpose:
  When enabled, agentx agents auto-approve \"safe\" agentx commands without
  requiring user confirmation.

Usage:
  agentx config auto_approve 1    # Enable auto-approve
  agentx config auto_approve 0    # Disable (require approval)

Safe commands (auto-approved when enabled):
  send, start, list, events, listen, relay, config,
  transcript, archive, status, help, --help, --version

Always require approval:
  - agentx reset          (archives and clears database)
  - agentx stop           (stops instances)
  - agentx launch --tool claude --count <N>     (launches new instances)

Values: 1, true, yes, on (enabled) | 0, false, no, off, \"\" (disabled)",
        ),

        "AGENTX_AUTO_SUBSCRIBE" => Some(
            "\
AGENTX_AUTO_SUBSCRIBE - Auto-subscribe event presets for new instances

Default: collision

Purpose:
  Comma-separated list of event subscriptions automatically added
  when an instance registers with 'agentx start'.

Usage:
  agentx config auto_subscribe \"collision,created\"
  agentx config auto_subscribe \"\"   # No auto-subscribe

Available presets:
  collision    - Alert when agents edit same file (within 30s window)
  created      - Notify when new instances join
  stopped      - Notify when instances leave
  blocked      - Notify when any instance is blocked (needs approval)

Notes:
  - Instances can add/remove subscriptions at runtime
  - See 'agentx events --help' for subscription management",
        ),

        "AGENTX_TITLE_MODE" => Some(
            "\
AGENTX_TITLE_MODE - What appears in the terminal/tab title for agentx-launched agents

Default: combined

Purpose:
  Controls whether agentx replaces, combines, or leaves alone the wrapped tool's
  terminal title. In combined mode, agentx keeps its live status and appends the
  tool's own live title (for example, a Codex spinner).

Values:
  combined - Show '{icon} name - {tool title}' and update it live.
  label    - Show agentx's status label only: '{icon} name [tool]'.
  off      - Pass the tool's own terminal title through unchanged; agentx writes none.

Usage:
  agentx config title_mode combined   # agentx status + live tool title (default)
  agentx config title_mode label      # agentx status label only
  agentx config title_mode off        # use the tool's title
  agentx config title_mode <empty>    # reset to the default

Notes:
  - This affects terminal/tab titles, not the visible PTY output.
  - Tools that do not emit terminal titles have no live child title to append.
  - The same setting can be provided with AGENTX_TITLE_MODE in the environment.",
        ),

        "AGENTX_NAME_EXPORT" => Some(
            "\
AGENTX_NAME_EXPORT - Export instance name to custom env var

Purpose:
  When set, launched instances will have their name exported to
  the specified environment variable. Useful for scripts that need
  to reference the current instance name.

Usage:
  agentx config name_export \"MY_AGENT_NAME\"   # Export to MY_AGENT_NAME
  agentx config name_export \"\"                 # Disable export

Example:
  # Set export variable
  agentx config name_export \"AGENTX_NAME\"

  # Now launched instances have:
  # AGENTX_NAME=luna (or whatever name was generated)

  # Scripts can use it:
  # agentx send \"@$AGENTX_NAME completed task\"

Notes:
  - Only affects agentx-launched instances (agentx launch --tool TOOL [--count N])
  - Variable name must be a valid shell identifier
  - Works alongside AGENTX_PROCESS_ID (always set) for identity",
        ),

        "AGENTX_COPILOT_DELIVERY" => Some(
            "\
AGENTX_COPILOT_DELIVERY - How idle Copilot agents get messages

  agentx config copilot_delivery inject   # type into the terminal
  agentx config copilot_delivery --unset  # back to the default (native)
  AGENTX_COPILOT_DELIVERY=inject agentx launch --tool copilot   # per-launch override

native (default): `agentx launch --tool copilot` ships an extension in its per-run plugin that hands
messages to an idle session with the Copilot SDK (`session.send`) instead of
typing into the terminal: no waiting for an empty prompt, no extra wake turn.
Copilot CLI extensions are experimental: if Copilot's settings.json leaves
them unset, launch turns on enabledFeatureFlags.EXTENSIONS and says so on
stderr; if they were turned off (flag false, /extensions mode disabled, agentx
disabled), launch fails. Mid-turn delivery uses Copilot hooks either way.
Applies to new launches.

Diagnose: `agentx list <name>` shows the extension's wake port once bound;
agentx.log has plugin.copilot_* (extension), copilot.ext_* (agentx side) and
delivery.copilot_extension_stalled when idle messages are not delivered.",
        ),

        "AGENTX_RELAY_ENABLED" => Some(
            "\
AGENTX_RELAY_ENABLED - Whether relay sync is on

Default: true (when relay is configured)
Stored in [relay] in config.toml. Environment overrides are ignored for relay fields.

Usage:
  agentx relay off    Disable relay sync (keeps relay configuration)
  agentx relay on     Re-enable relay sync",
        ),

        _ => None,
    }
    .map(str::to_string)
}

/// Build terminal help text (shared between config --info and docs --config).
pub fn terminal_help_text(show_current: bool) -> String {
    terminal_help_text_for(None, show_current)
}
fn terminal_help_text_for(host: Option<&ConfigHost>, show_current: bool) -> String {
    use crate::runtime::terminal::presets::TERMINAL_PRESETS;

    let platform = match std::env::consts::OS {
        "macos" => "Darwin",
        "linux" => "Linux",
        "windows" => "Windows",
        other => other,
    };

    // Managed parents and their variants. A preset is "managed" when it
    // defines `close` — agentx can shut its agent window down on kill. The
    // built-in list is hand-curated here for ordering and human-readable
    // descriptions, but `is_managed_preset` below is the source of truth for
    // section bucketing so a new managed preset never gets mis-listed under
    // "Other (opens window only)".
    const MANAGED_PARENTS: &[(&str, &str)] = &[("kitty", "auto split/tab/window"), ("wezterm", "auto tab/split/window"), ("tmux", "detached sessions"), ("cmux", "workspaces"), ("zellij", "panes"), ("waveterm", "blocks"), ("herdr", "panes")];
    const MANAGED_VARIANTS: &[(&str, &[&str])] = &[("kitty", &["kitty-window", "kitty-tab", "kitty-split"]), ("wezterm", &["wezterm-window", "wezterm-tab", "wezterm-split"]), ("tmux", &["tmux-split"])];

    let all_managed: std::collections::HashSet<&str> = {
        let mut s = std::collections::HashSet::new();
        for (parent, _) in MANAGED_PARENTS {
            s.insert(*parent);
        }
        for (_, variants) in MANAGED_VARIANTS {
            for v in *variants {
                s.insert(v);
            }
        }
        s
    };

    // Check binary availability
    let is_available = |preset_name: &str| -> bool {
        let preset = TERMINAL_PRESETS.iter().find(|(n, _)| *n == preset_name);
        let Some((_name, p)) = preset else {
            return false;
        };
        if p.binary.is_some_and(|bin| crate::runtime::terminal::which_bin(bin).is_some()) {
            return true;
        }
        #[cfg(target_os = "macos")]
        {
            let app = p.app_name.unwrap_or(_name);
            // Handle preset names that already end in .app
            let bundle = if app.ends_with(".app") { app.to_string() } else { format!("{app}.app") };
            for dir in ["/Applications", "/Applications/Utilities", "/System/Applications", "/System/Applications/Utilities"] {
                let path = format!("{dir}/{bundle}");
                if std::path::Path::new(&path).exists() {
                    return true;
                }
            }
        }
        false
    };

    let mut lines = Vec::new();
    lines.push("AGENTX_TERMINAL — where agentx opens new agent windows".to_string());
    lines.push(String::new());

    if show_current {
        let (current, source) = host.map_or_else(|| config_get("AGENTX_TERMINAL"), |host| host.get("AGENTX_TERMINAL"));
        if current.is_empty() || current == "default" {
            // "default" is a sentinel meaning auto-detect; it isn't a preset,
            // so don't run it through close-presence inference.
            let suffix = if source == "default" { String::new() } else { format!(" [{source}]") };
            lines.push(format!("Current: default (auto-detect){suffix}"));
        } else {
            let kind = if crate::config::get_merged_preset(&current).is_some_and(|p| p.has_close(cfg!(windows))) { "managed" } else { "open only" };
            lines.push(format!("Current: {current} ({kind}) [{source}]"));
        }
        lines.push(String::new());
    }

    // Managed section
    lines.push("Managed (open + close on kill):".to_string());
    for (parent, desc) in MANAGED_PARENTS {
        // Skip if not on this platform
        let on_platform = TERMINAL_PRESETS.iter().find(|(n, _)| n == parent).is_some_and(|(_n, p)| p.platforms.is_empty() || p.platforms.contains(&platform));
        if !on_platform {
            continue;
        }
        let mark = if is_available(parent) { "[+]" } else { "[-]" };
        lines.push(format!("  {mark} {:<14} {desc}", parent));
    }
    lines.push(String::new());
    lines.push("  Variants:".to_string());
    for (parent, variants) in MANAGED_VARIANTS {
        let on_platform = TERMINAL_PRESETS.iter().find(|(n, _)| n == parent).is_some_and(|(_n, p)| p.platforms.is_empty() || p.platforms.contains(&platform));
        if !on_platform {
            continue;
        }
        lines.push(format!("    {parent}: {}", variants.join(", ")));
    }

    // Other (open-only, platform-filtered). Bucket by `close` presence rather
    // than the hand-curated MANAGED_PARENTS list, so any built-in preset that
    // grows a close command later doesn't silently land in the wrong section.
    lines.push(String::new());
    lines.push("Other (opens window only):".to_string());
    for (name, preset) in TERMINAL_PRESETS.iter() {
        let has_close = preset.close.default.is_some() || preset.close.windows.is_some();
        if all_managed.contains(name) || has_close {
            continue;
        }
        if !preset.platforms.is_empty() && !preset.platforms.contains(&platform) {
            continue;
        }
        let mark = if is_available(name) { "[+]" } else { "[-]" };
        lines.push(format!("  {mark} {name}"));
    }

    // TOML-defined presets not in built-ins
    let toml_path = host.map_or_else(crate::paths::config_toml_path, ConfigHost::path);
    let toml_presets: Vec<(String, bool)> = crate::config::load_toml_presets(&toml_path)
        .and_then(|p| {
            p.as_table().map(|t| {
                t.iter()
                    .filter(|(name, _)| !TERMINAL_PRESETS.iter().any(|(n, _)| *n == name.as_str()))
                    .map(|(name, val)| {
                        // close may be a legacy string or an argv array.
                        let has_close = val.get("close").is_some_and(|v| v.as_str().is_some_and(|s| !s.is_empty()) || v.as_array().is_some_and(|a| !a.is_empty()));
                        (name.clone(), has_close)
                    })
                    .collect()
            })
        })
        .unwrap_or_default();
    if !toml_presets.is_empty() {
        lines.push(String::new());
        lines.push("Custom presets (config.toml):".to_string());
        for (name, has_close) in &toml_presets {
            let kind = if *has_close { "open + close" } else { "open only" };
            let mark = if crate::config::get_merged_preset(name).and_then(|p| p.binary).map(|b| crate::runtime::terminal::which_bin(&b).is_some()).unwrap_or(true) { "[+]" } else { "[-]" };
            lines.push(format!("  {mark} {:<14} ({kind})", name));
        }
    }

    lines.push(String::new());
    lines.push("Custom command (open only):".to_string());
    lines.push("  agentx config terminal \"my-terminal -e bash {script}\"".to_string());
    lines.push(String::new());
    lines.push("Custom preset with close (~/.agentx/config.toml):".to_string());
    lines.push("  [terminal.presets.myterm]".to_string());
    lines.push("  open = \"myterm spawn -- bash {script}\"".to_string());
    lines.push("  close = \"myterm kill --id {pane_id}\"".to_string());
    lines.push("  binary = \"myterm\"".to_string());
    lines.push("  pane_id_env = \"MYTERM_PANE_ID\"".to_string());
    lines.push(String::new());
    lines.push("Placeholders:".to_string());
    lines.push("  {script}     = agentx-generated launch wrapper script path".to_string());
    lines.push("  {pane_id}    = pane/window/workspace ID from pane_id_env; falls back to {id}".to_string());
    lines.push("  {process_id} = AGENTX_PROCESS_ID for the launched agent".to_string());
    lines.push("  {pid}        = launched terminal process ID".to_string());
    lines.push("  {id}         = first line of stdout captured from the open command".to_string());
    lines.push("                 (herdr `agent start` JSON is parsed for result.agent.pane_id)".to_string());
    lines.push("  {cwd}        = working directory the agent will start in".to_string());
    lines.push("  {instance_name} = agentx instance name (e.g. \"luna\")".to_string());
    lines.push("  {tool}          = tool label (e.g. \"claude\", \"codex\")".to_string());
    lines.push("  {pane_title}    = pre-formatted label (\"\u{25c9} luna [claude]\"); falls back to".to_string());
    lines.push("                 {instance_name} when not set".to_string());
    lines.push(String::new());
    lines.push("Set:    agentx config terminal kitty".to_string());
    lines.push("Reset:  agentx config terminal default".to_string());

    lines.join("\n")
}

fn print_terminal_info(host: &ConfigHost) {
    println!("{}", terminal_help_text_for(Some(host), true));
}

fn show_key_info(host: &ConfigHost, key: &str) -> i32 {
    let Some(field) = crate::config::user_field(key) else {
        eprintln!("Unknown config key: {key}");
        eprintln!("Valid keys: {}", crate::config::user_fields().map(|f| f.name).collect::<Vec<_>>().join(", "));
        return 1;
    };
    // AGENTX_TERMINAL: dynamic help from TERMINAL_PRESETS
    if field.name == "terminal" {
        print_terminal_info(host);
        return 0;
    }
    let (value, source) = host.get(field.env);
    // If rich help text exists, show it with current value appended
    if let Some(help) = config_help(field.env) {
        println!("{help}");
        println!();
        if value.is_empty() {
            println!("Current value: (not set)");
        } else {
            println!("Current value: {value} [{source}]");
        }
        return 0;
    }
    println!("{}", field.env);
    println!("  Description: {}", field.desc);
    println!("  Type: {}", field.kind.as_str());
    if value.is_empty() {
        println!("  Value: (not set)");
    } else {
        println!("  Value: {value} [{source}]");
    }
    println!("  Set via: agentx config {} <value>", field.name);
    println!("  Or env: export {}=<value>", field.env);
    0
}

/// Handle terminal preset configuration.
fn config_terminal(host: &ConfigHost, argv: &[String], setup_mode: bool) -> i32 {
    use crate::runtime::terminal::presets::TERMINAL_PRESETS;

    if argv.is_empty() {
        // Show terminal status
        let (current, source) = host.get("AGENTX_TERMINAL");
        if current.is_empty() {
            println!("Terminal: (auto-detect)");
        } else {
            println!("Terminal: {current} [{source}]");
        }
        println!("\nAvailable presets:");
        let platform = crate::shared::platform::platform_name();
        for (name, preset) in TERMINAL_PRESETS.iter() {
            let marker = if *name == current { " ← current" } else { "" };
            let availability = if preset.platforms.is_empty() || preset.platforms.contains(&platform) { "" } else { " (unavailable on this platform)" };
            println!("  {name}{availability}{marker}");
        }
        // Include TOML-defined presets not in built-ins
        let toml_path = host.path();
        if let Some(toml_presets) = crate::config::load_toml_presets(&toml_path)
            && let Some(table) = toml_presets.as_table()
        {
            for name in table.keys() {
                if !TERMINAL_PRESETS.iter().any(|(n, _)| *n == name.as_str()) {
                    let marker = if *name == current { " ← current" } else { "" };
                    println!("  {}{}", name, marker);
                }
            }
        }
        println!("\nSet: agentx config terminal <preset>");
        return 0;
    }

    let preset_name = &argv[0];

    if preset_name == "--info" || preset_name == "info" || preset_name == "?" {
        print_terminal_info(host);
        return 0;
    }

    if preset_name == "default" || preset_name == "auto" {
        match host.unset("AGENTX_TERMINAL") {
            Ok(()) => {
                println!("Terminal reset to auto-detect");
                return 0;
            }
            Err(e) => {
                eprintln!("Error: {e}");
                return 1;
            }
        }
    }

    if argv.len() > 1 || preset_name.contains("{script}") {
        let command = argv.join(" ");
        if !command.contains("{script}") {
            eprintln!("Error: Custom terminal command must contain {{script}}");
            return 1;
        }
        return match host.set("AGENTX_TERMINAL", &command) {
            Ok(()) => {
                println!("Terminal set to custom command");
                0
            }
            Err(e) => {
                eprintln!("Error: {e}");
                1
            }
        };
    }

    // Validate preset exists (built-in or user-defined in config.toml)
    let builtin = TERMINAL_PRESETS.iter().find(|(name, _)| *name == preset_name.as_str());
    let valid = builtin.is_some() || matches!(preset_name.as_str(), "here" | "print") || crate::config::is_user_defined_preset_for_scope(preset_name, &host.scope);

    if !valid {
        let mut available: Vec<&str> = TERMINAL_PRESETS.iter().map(|(name, _)| *name).collect();
        // Include user-defined presets
        let toml_path = host.path();
        let user_names: Vec<String> = crate::config::load_toml_presets(&toml_path).and_then(|p| p.as_table().map(|t| t.keys().cloned().collect())).unwrap_or_default();
        eprintln!("Error: Unknown terminal preset '{preset_name}'");
        let user_refs: Vec<&str> = user_names.iter().map(|s| s.as_str()).collect();
        available.extend(user_refs);
        eprintln!("Available: {}", available.join(", "));
        return 1;
    }
    // config_set refuses a built-in preset for another platform, unless a
    // user preset of the same name overrides it.
    match host.set("AGENTX_TERMINAL", preset_name) {
        Ok(()) => {
            println!("Terminal set to: {preset_name}");
            if preset_name.starts_with("kitty") {
                if setup_mode {
                    kitty_setup();
                } else {
                    show_kitty_status(preset_name);
                }
            }
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

/// Show kitty remote control socket status after setting a kitty preset.
/// Diagnoses missing socket and hints at --setup if needed.
fn show_kitty_status(preset_name: &str) {
    if let Some(socket) = find_kitty_socket() {
        if preset_name == "kitty" {
            println!("  Socket found ({}) — splits/tabs available", socket.display());
        }
        return;
    }

    // No socket — diagnose why
    let conf = match find_kitty_conf() {
        Some(c) => c,
        None => {
            println!("  No kitty.conf found");
            println!("  Run: agentx config terminal kitty --setup");
            return;
        }
    };

    let has_rc = kitty_conf_has(&conf, "allow_remote_control");
    let has_listen = kitty_conf_has(&conf, "listen_on");

    match (has_rc.as_deref(), &has_listen) {
        (Some("yes" | "socket"), Some(_)) => {
            println!("  Config OK but no socket — restart kitty");
        }
        (Some(val), _) if val != "yes" && val != "socket" => {
            println!("  allow_remote_control is '{val}' — needs 'yes' or 'socket'");
            println!("  Edit {} manually, then restart kitty", conf.display());
        }
        _ => {
            println!("  Remote control not configured");
            println!("  Run: agentx config terminal kitty --setup");
        }
    }
}

// ── Kitty Setup (C4) ─────────────────────────────────────────────────────

/// Find kitty.conf path.
fn find_kitty_conf() -> Option<PathBuf> {
    let candidates = [dirs::config_dir().map(|d| d.join("kitty/kitty.conf")), dirs::home_dir().map(|h| h.join(".config/kitty/kitty.conf"))];
    candidates.into_iter().flatten().find(|p| p.exists())
}

/// Find kitty remote control socket.
fn find_kitty_socket() -> Option<PathBuf> {
    let candidates = [PathBuf::from("/tmp/kitty"), PathBuf::from("/tmp/mykitty")];
    candidates.into_iter().find(|p| p.exists())
}

/// Check if kitty.conf contains a specific key (exact match via whitespace split).
fn kitty_conf_has(path: &Path, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, |c: char| c.is_whitespace());
        if let (Some(k), Some(v)) = (parts.next(), parts.next())
            && k == key
        {
            return Some(v.trim().to_string());
        }
    }
    None
}

/// Configure kitty for remote control (splits/tabs).
fn kitty_setup() -> i32 {
    if let Some(socket) = find_kitty_socket() {
        println!("Kitty remote control already working ({})", socket.display());
        return 0;
    }

    let conf = match find_kitty_conf() {
        Some(c) => c,
        None => {
            eprintln!("Error: Could not find kitty.conf");
            return 1;
        }
    };

    let has_rc = kitty_conf_has(&conf, "allow_remote_control");
    let has_listen = kitty_conf_has(&conf, "listen_on");

    if matches!(has_rc.as_deref(), Some("yes" | "socket")) && has_listen.is_some() {
        println!("Config OK ({}) but no socket — restart kitty", conf.display());
        return 0;
    }

    if let Some(ref rc_val) = has_rc
        && rc_val != "yes"
        && rc_val != "socket"
    {
        eprintln!("Error: allow_remote_control is '{rc_val}' in {}", conf.display());
        eprintln!("  Change to 'yes' or 'socket', then restart kitty");
        return 1;
    }

    let mut lines_to_add = Vec::new();
    if has_rc.is_none() {
        lines_to_add.push("allow_remote_control yes");
    }
    if has_listen.is_none() {
        lines_to_add.push("listen_on unix:/tmp/kitty");
    }

    match std::fs::OpenOptions::new().append(true).open(&conf) {
        Ok(mut file) => {
            use std::io::Write;
            let _ = writeln!(file, "\n# Added by agentx for remote control (splits/tabs)");
            for line in &lines_to_add {
                let _ = writeln!(file, "{line}");
            }
        }
        Err(e) => {
            eprintln!("Error: Failed to write {}: {e}", conf.display());
            return 1;
        }
    }

    println!("Added to {}:", conf.display());
    for line in &lines_to_add {
        println!("  {line}");
    }
    println!("\nRestart kitty to apply changes");
    0
}

/// Update tool permissions when auto_approve changes.
/// Sync installed hook permissions with `auto_approve` in config.toml. Those
/// permission files are shared like config.toml, so env overrides don't apply.
fn update_auto_approve_permissions() -> bool {
    let invocation = match crate::service::InvocationContext::capture() {
        Ok(invocation) => invocation,
        Err(error) => {
            eprintln!("Error: {error}");
            return false;
        }
    };
    let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
    update_auto_approve_permissions_for(&ConfigHost { scope, invocation: &invocation, session: None })
}
fn update_auto_approve_permissions_for(host: &ConfigHost) -> bool {
    let enabled = crate::config::AgentxConfig::load_for_scope(&host.scope, &std::collections::HashMap::new()).auto_approve;
    let failures = crate::runtime::config_effects::refresh_installed_hook_permissions(host.invocation, enabled);
    if enabled {
        println!("Auto-approve enabled for safe agentx commands");
    } else {
        println!("Auto-approve disabled - safe agentx commands will require approval");
    }
    for (tool, error) in &failures {
        eprintln!("Failed to update {tool} auto-approve permissions: {error}");
    }
    failures.is_empty()
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn native_config_session(scope: &Path) -> (crate::daemon::test_support::TestDaemon, crate::cli::client::Session) {
        use crate::agents::service::{Request as P, Response as PR};
        use crate::service::{InvocationContext, Request, Response, client::Client};
        let db = AgentxDb::open_at(&scope.join("agentx.db")).unwrap();
        for name in ["luna", "lune", "a_one", "a%one"] {
            db.save_instance_named(name, json!({"tool":"adhoc","created_at":100.0,"wait_timeout":15,"hints":"old hints","subagent_timeout":10}).as_object().unwrap()).unwrap();
        }
        drop(db);
        let daemon = crate::daemon::test_support::TestDaemon::start(scope, &scope.join("agentx.db"));
        let invocation = InvocationContext { cwd: scope.into(), explicit_name: Some("luna".into()), environment: std::collections::HashMap::from([("AGENTX_TIMEOUT".into(), "4321".into())]), ..Default::default() };
        let client = Client::connect_existing(scope, invocation.clone()).unwrap();
        let Response::Participants(PR::Actor(actor)) = client.call(Request::Participants(P::Resolve)).unwrap() else {
            panic!("actor");
        };
        let session = crate::cli::client::Session { client, invocation, actor: Some(actor), command: CommandContext { explicit_name: Some("luna".into()), identity: None, go: false, identity_warning: None } };
        (daemon, session)
    }

    #[test]
    fn native_config_instance_mutations_use_scoped_default_and_literal_prefix() {
        use crate::agents::service::{Request as P, Response as PR};
        use crate::service::{Request, Response};
        use clap::Parser;
        let directory = tempfile::Builder::new().prefix("agentx-config-instance-").tempdir().unwrap();
        let (_daemon, session) = native_config_session(directory.path());
        for arguments in [vec!["config", "-i", "luna", "timeout", "default"], vec!["config", "-i", "luna", "hints", ""], vec!["config", "-i", "luna", "subagent_timeout", "default"]] {
            assert_eq!(cmd_config_client(&session, &ConfigArgs::try_parse_from(arguments).unwrap()), 0);
        }
        let Response::Participants(PR::Record(Some(row))) = session.client.call(Request::Participants(P::Record { name: "luna".into() })).unwrap() else {
            panic!("row");
        };
        assert_eq!(row.wait_timeout, Some(4321));
        assert_eq!(row.hints, None);
        assert_eq!(row.subagent_timeout, None);
    }

    #[test]
    fn local_config_edits_only_the_originating_file_scope() {
        use clap::Parser;
        let directory = tempfile::Builder::new().prefix("agentx-config-file-only-").tempdir().unwrap();
        let invocation = crate::service::InvocationContext { cwd: directory.path().into(), ..Default::default() };
        let host = ConfigHost { scope: directory.path().into(), invocation: &invocation, session: None };
        std::fs::write(host.path(), "[launch.claude]\ndelivery = \"inject\"\n").unwrap();
        assert_eq!(cmd_config_impl(&host, &ConfigArgs::try_parse_from(["config", "timeout", "6789"]).unwrap()), 0);
        let saved = std::fs::read_to_string(host.path()).unwrap();
        for arguments in [["config", "timeout", "--json"], ["config", "timeout", "--info"]] {
            assert_eq!(cmd_config_impl(&host, &ConfigArgs::try_parse_from(arguments).unwrap()), 0);
        }
        assert!(saved.contains("delivery = \"inject\""));
        assert_eq!(std::fs::read_to_string(host.path()).unwrap(), saved);
        assert_eq!(host.get("AGENTX_TIMEOUT"), ("6789".into(), "toml"));
        assert!(!directory.path().join("agentx.db").exists());
    }

    #[test]
    fn native_config_transport_failure_cannot_recreate_storage() {
        use clap::Parser;
        let directory = tempfile::Builder::new().prefix("agentx-config-unavailable-").tempdir().unwrap();
        let (daemon, session) = native_config_session(directory.path());
        drop(daemon);
        let database = directory.path().join("agentx.db");
        std::fs::remove_file(&database).unwrap();
        assert_eq!(cmd_config_client(&session, &ConfigArgs::try_parse_from(["config", "-i", "luna", "timeout", "wrong"]).unwrap()), 1);
        assert!(!database.exists());
    }

    #[test]
    fn test_unique_instance_prefix_requires_one_literal_match() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        for name in ["luna", "lune", "a_one", "axone", "a%one", "a!one"] {
            db.conn().execute("INSERT INTO instances (name, created_at) VALUES (?, 1)", [name]).unwrap();
        }
        assert_eq!(unique_instance_prefix(&db, "luna").unwrap().as_deref(), Some("luna"));
        let error = unique_instance_prefix(&db, "lun").unwrap_err();
        assert!(error.contains("luna, lune"));
        assert_eq!(unique_instance_prefix(&db, "a_").unwrap().as_deref(), Some("a_one"));
        assert_eq!(unique_instance_prefix(&db, "a%").unwrap().as_deref(), Some("a%one"));
        assert_eq!(unique_instance_prefix(&db, "a!").unwrap().as_deref(), Some("a!one"));
        assert_eq!(unique_instance_prefix(&db, "absent").unwrap(), None);
    }

    #[test]
    fn config_set_reads_after_acquiring_sibling_lock() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        config_set_at_path(&path, "hints", Some("before")).unwrap();
        let lock = std::fs::OpenOptions::new().write(true).open(path.with_extension("toml.lock")).unwrap();
        crate::sys::fs::lock_exclusive(&lock).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            let path = &path;
            scope.spawn(move || {
                tx.send(config_set_at_path(path, "timeout", Some("600"))).unwrap();
            });
            let blocked = rx.recv_timeout(std::time::Duration::from_millis(100));
            // Simulate another writer committing while it owns the lock.
            crate::config::write_config_toml_path(path, "[launch]\nhints = 'under-lock'\n").unwrap();
            drop(lock);
            assert!(matches!(blocked, Err(std::sync::mpsc::RecvTimeoutError::Timeout)));
            rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap().unwrap();
        });
        let content = std::fs::read_to_string(&path).unwrap();
        let doc: toml_edit::DocumentMut = content.parse().unwrap();
        assert_eq!(doc["launch"]["hints"].as_str(), Some("under-lock"));
        assert_eq!(doc["preferences"]["timeout"].as_integer(), Some(600));
    }

    #[test]
    fn test_normalize_key() {
        assert_eq!(normalize_key("hints"), "AGENTX_HINTS");
        assert_eq!(normalize_key("AGENTX_HINTS"), "AGENTX_HINTS");
        assert_eq!(normalize_key("terminal"), "AGENTX_TERMINAL");
        assert_eq!(normalize_key("agentx_timeout"), "AGENTX_TIMEOUT");
    }

    #[test]
    fn test_config_args_json_flag() {
        use clap::Parser;
        let args = ConfigArgs::try_parse_from(["config", "--json", "key"]).unwrap();
        assert!(args.json);
        assert_eq!(args.key, Some("key".to_string()));
    }

    #[test]
    fn test_config_args_info_not_swallowed() {
        use clap::Parser;
        // A config key followed by --info should not treat the flag as a value.
        let args = ConfigArgs::try_parse_from(["config", "hints", "--info"]).unwrap();
        assert!(args.info);
        assert_eq!(args.key, Some("hints".to_string()));
        assert!(args.value.is_none());
    }

    #[test]
    fn test_config_args_set_value() {
        use clap::Parser;
        let args = ConfigArgs::try_parse_from(["config", "hints", "myvalue"]).unwrap();
        assert_eq!(args.key, Some("hints".to_string()));
        assert_eq!(args.value.as_deref(), Some("myvalue"));
    }

    #[test]
    fn test_config_args_instance() {
        use clap::Parser;
        let args = ConfigArgs::try_parse_from(["config", "-i", "self", "hints", "text"]).unwrap();
        assert_eq!(args.instance, Some("self".to_string()));
        assert_eq!(args.key, Some("hints".to_string()));
        assert_eq!(args.value.as_deref(), Some("text"));
    }

    #[test]
    fn test_config_args_dev_root_unset() {
        use clap::Parser;
        let args = ConfigArgs::try_parse_from(["config", "dev_root", "--unset"]).unwrap();
        assert_eq!(args.key, Some("dev_root".to_string()));
        assert!(args.unset);
        assert!(args.value.is_none());
    }

    #[test]
    fn test_config_args_hyphen_value() {
        use clap::Parser;
        // "agentx config codex_args '--model o3'" — quoted so shell passes as one token
        let args = ConfigArgs::try_parse_from(["config", "codex_args", "--model o3"]).unwrap();
        assert_eq!(args.key, Some("codex_args".to_string()));
        assert_eq!(args.value.as_deref(), Some("--model o3"));
    }

    #[test]
    fn test_config_args_flags_after_value_not_swallowed() {
        use clap::Parser;
        // A value followed by --json should not swallow the output flag.
        let args = ConfigArgs::try_parse_from(["config", "hints", "myval", "--json"]).unwrap();
        assert_eq!(args.key, Some("hints".to_string()));
        assert_eq!(args.value.as_deref(), Some("myval"));
        assert!(args.json);
    }

    #[test]
    fn test_instance_key_validation() {
        assert!(INSTANCE_KEYS.iter().any(|(k, _)| *k == "timeout"));
        assert!(INSTANCE_KEYS.iter().any(|(k, _)| *k == "hints"));
        assert!(INSTANCE_KEYS.iter().any(|(k, _)| *k == "subagent_timeout"));
        assert!(!INSTANCE_KEYS.iter().any(|(k, _)| *k == "invalid"));
    }

    #[test]
    fn test_config_key_info_exists() {
        // All keys should have descriptions
        for field in crate::config::user_fields() {
            assert!(!field.desc.is_empty(), "{}", field.name);
        }
        assert!(crate::config::user_field("relay_psk").is_none());
        assert!(crate::config::user_field("AGENTX_GROK_ARGS").is_some());
    }

    #[test]
    fn test_config_dev_root_set_get_unset() {
        use clap::Parser;
        let directory = tempfile::Builder::new().prefix("agentx-config-native-").tempdir().unwrap();
        let (_daemon, session) = native_config_session(directory.path());
        for arguments in [vec!["config", "dev_root", "/tmp/worktree"], vec!["config", "dev_root"], vec!["config", "dev_root", "--unset"]] {
            assert_eq!(cmd_config_client(&session, &ConfigArgs::try_parse_from(arguments.clone()).unwrap()), 0);
            let crate::service::Response::Maintenance(crate::service::maintenance::Response::DevRoot(value)) = session.client.call(crate::service::Request::Maintenance(crate::service::maintenance::Request::DevRoot)).unwrap() else {
                panic!("dev root");
            };
            assert_eq!(value, if arguments.contains(&"--unset") { None } else { Some("/tmp/worktree".into()) });
            assert_eq!(crate::config::dev_root::read(directory.path()).unwrap(), value.map(PathBuf::from));
        }
    }

    #[test]
    fn test_config_set_accepts_unknown_upstream_args() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[launch.claude]\nargs = \"--model keep\"\n").unwrap();

        config_set_at_path(&path, "AGENTX_CLAUDE_ARGS", Some("--future-upstream-flag")).unwrap();
        assert!(std::fs::read_to_string(&path).unwrap().contains("--future-upstream-flag"));
    }

    #[test]
    fn test_config_set_saves_valid_args() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        config_set_at_path(&path, "AGENTX_PI_ARGS", Some("--model safe-model")).unwrap();

        let parsed: toml::Table = std::fs::read_to_string(path).unwrap().parse().unwrap();
        assert_eq!(parsed["launch"]["pi"]["args"].as_str(), Some("--model safe-model"));
        assert!(parsed.get("pi").is_none(), "no stray top-level [pi]");
    }

    #[test]
    fn test_config_set_copilot_delivery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let read = || -> toml::Table { std::fs::read_to_string(&path).unwrap().parse().unwrap() };

        config_set_at_path(&path, "AGENTX_COPILOT_DELIVERY", Some("inject")).unwrap();
        assert_eq!(read()["launch"]["copilot"]["idle_delivery"].as_str(), Some("inject"));
        assert!(config_set_at_path(&path, "AGENTX_COPILOT_DELIVERY", Some("pty")).is_err());

        // --unset writes "", which removes the key (back to the default).
        config_set_at_path(&path, "AGENTX_COPILOT_DELIVERY", None).unwrap();
        assert!(read()["launch"]["copilot"].get("idle_delivery").is_none());
    }

    #[test]
    fn config_set_refuses_what_the_loader_would_reject() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        for (key, value) in [
            ("AGENTX_TIMEOUT", "999999"),
            ("AGENTX_TIMEOUT", "soon"),
            ("AGENTX_SUBAGENT_TIMEOUT", "0"),
            ("AGENTX_AUTO_APPROVE", "maybe"),
            ("AGENTX_AUTO_SUBSCRIBE", "a b"),
            ("AGENTX_TITLE_MODE", "loud"),
            ("AGENTX_TERMINAL", "kitty; rm -rf ~"),
            ("AGENTX_RELAY_PSK", "secret"),
            ("AGENTX_NOT_A_KEY", "1"),
        ] {
            assert!(config_set_at_path(&path, key, Some(value)).is_err(), "{key}={value} must be refused");
        }
        assert!(!path.exists() || std::fs::read_to_string(&path).unwrap().trim().is_empty());
    }

    #[test]
    fn config_set_writes_the_field_type() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let read = || -> toml::Table { std::fs::read_to_string(&path).unwrap().parse().unwrap() };

        config_set_at_path(&path, "hints", Some("true")).unwrap();
        config_set_at_path(&path, "timeout", Some("600")).unwrap();
        config_set_at_path(&path, "auto_approve", Some("no")).unwrap();
        config_set_at_path(&path, "grok_args", Some("--model x")).unwrap();
        let t = read();
        assert_eq!(t["launch"]["hints"].as_str(), Some("true"));
        assert_eq!(t["preferences"]["timeout"].as_integer(), Some(600));
        assert_eq!(t["preferences"]["auto_approve"].as_bool(), Some(false));
        assert_eq!(t["launch"]["grok"]["args"].as_str(), Some("--model x"));

        // Explicit "" is a value (no auto-subscribe presets); unset removes the key.
        config_set_at_path(&path, "auto_subscribe", Some("")).unwrap();
        assert_eq!(read()["launch"]["auto_subscribe"].as_str(), Some(""));
        config_set_at_path(&path, "auto_subscribe", None).unwrap();
        assert!(read()["launch"].get("auto_subscribe").is_none());
        assert!(config_set_at_path(&path, "timeout", Some("")).is_err());
    }

    #[test]
    #[serial_test::serial] // resets the process-wide Config other tests read
    #[cfg(target_os = "macos")]
    fn test_terminal_help_text_lists_cmux_as_managed() {
        crate::config::Config::reset();
        crate::config::Config::init();
        let help = terminal_help_text(false);
        let managed = help.split("Other (opens window only):").next().expect("managed section should exist");
        let other = help.split("Other (opens window only):").nth(1).expect("other section should exist");

        assert!(managed.contains("cmux"));
        assert!(!other.contains("cmux"));
    }

    #[test]
    fn test_terminal_help_text_documents_new_placeholders() {
        // If you add a placeholder to substitute_open_argv, document it.
        let help = terminal_help_text(false);
        for placeholder in ["{instance_name}", "{tool}", "{cwd}", "{pane_title}"] {
            assert!(help.contains(placeholder), "missing {placeholder} placeholder docs",);
        }
    }

    #[test]
    #[serial_test::serial] // "default" reads AGENTX_TIMEOUT, which serial tests set
    fn test_config_instance_set_timeout_default_resets() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn().execute("INSERT INTO instances (name, created_at, wait_timeout) VALUES (?1, ?2, ?3)", rusqlite::params!["luna", crate::shared::time::now_epoch_f64(), 15]).unwrap();

        config_instance_set(&db, "luna", "timeout", "default").unwrap();
        let timeout: i64 = db.conn().query_row("SELECT wait_timeout FROM instances WHERE name = 'luna'", [], |row| row.get(0)).unwrap();
        // The configured default, not a literal: a developer's shell may export AGENTX_TIMEOUT.
        let expected = crate::config::AgentxConfig::load(None).timeout;
        assert_ne!(expected, 15);
        assert_eq!(timeout, expected);
    }

    #[test]
    fn test_config_instance_set_empty_hints_clears() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn().execute("INSERT INTO instances (name, created_at, hints) VALUES (?1, ?2, ?3)", rusqlite::params!["luna", crate::shared::time::now_epoch_f64(), "keep me"]).unwrap();

        config_instance_set(&db, "luna", "hints", "").unwrap();
        let hints: Option<String> = db.conn().query_row("SELECT hints FROM instances WHERE name = 'luna'", [], |row| row.get(0)).unwrap();
        assert_eq!(hints, None);
    }

    #[test]
    fn test_config_instance_set_subagent_timeout_default_clears() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn().execute("INSERT INTO instances (name, created_at, subagent_timeout) VALUES (?1, ?2, ?3)", rusqlite::params!["luna", crate::shared::time::now_epoch_f64(), 20]).unwrap();

        config_instance_set(&db, "luna", "subagent_timeout", "default").unwrap();
        let timeout: Option<i64> = db.conn().query_row("SELECT subagent_timeout FROM instances WHERE name = 'luna'", [], |row| row.get(0)).unwrap();
        assert_eq!(timeout, None);
    }

    #[test]
    fn test_render_config_instance_get_scalar_matches_local_style() {
        let rendered = render_config_instance_get(&serde_json::json!({"value": 42}), Some("timeout"), false);
        assert_eq!(rendered, "42");
    }

    #[test]
    fn test_render_config_instance_get_null_renders_empty_string() {
        let rendered = render_config_instance_get(&serde_json::json!({"value": serde_json::Value::Null}), Some("timeout"), false);
        assert_eq!(rendered, "");
    }

    #[test]
    fn test_render_config_instance_get_full_output_contract() {
        let rendered = render_config_instance_get(
            &serde_json::json!({
                "name": "luna",
                "timeout": 120,
                "hints": "ship it",
                "subagent_timeout": 45,
            }),
            None,
            false,
        );
        assert_eq!(rendered, "Agent: luna\n  timeout: 120s\n  hints: ship it\n  subagent_timeout: 45s");
    }

    #[test]
    fn test_render_config_instance_get_full_output_uses_empty_state_labels() {
        let rendered = render_config_instance_get(
            &serde_json::json!({
                "name": "luna",
                "timeout": 86400,
                "hints": "",
                "subagent_timeout": serde_json::Value::Null,
            }),
            None,
            false,
        );
        assert_eq!(rendered, "Agent: luna\n  timeout: 86400s\n  hints: (none)\n  subagent_timeout: (default)");
    }

    #[test]
    fn test_render_config_instance_get_json_mode_passthrough() {
        let rendered = render_config_instance_get(&serde_json::json!({"value": 42, "name": "luna"}), Some("timeout"), true);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["value"], 42);
        assert_eq!(parsed["name"], "luna");
    }

    #[test]
    fn test_config_instance_get_full_output_uses_exact_name() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES (?1, ?2)", rusqlite::params!["luna", crate::shared::time::now_epoch_f64()]).unwrap();

        let config = config_instance_get(&db, "luna", None).unwrap();
        assert_eq!(config["name"], "luna");
    }

    #[test]
    fn test_config_instance_get_timeout_uses_default_value() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES (?1, ?2)", rusqlite::params!["luna", crate::shared::time::now_epoch_f64()]).unwrap();

        let config = config_instance_get(&db, "luna", Some("timeout")).unwrap();
        assert_eq!(config["value"], 86400);
    }

    #[test]
    fn test_config_instance_get_full_output_schema() {
        // Stable JSON contract for `agentx config -i <name> --json`. Keep the
        // local and remote (RPC) paths emitting identical shapes:
        //   - name: exact name
        //   - hints: null when empty or unset (not "")
        //   - timeout: null when the row has no explicit value (registration
        //     paths write the resolved AGENTX_TIMEOUT explicitly; a bare INSERT
        //     that skips the column, as here, has nothing to fall back on)
        //   - subagent_timeout: null when unset
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES (?1, ?2)", rusqlite::params!["luna", crate::shared::time::now_epoch_f64()]).unwrap();

        let config = config_instance_get(&db, "luna", None).unwrap();
        assert_eq!(config["name"], "luna");
        assert!(config["timeout"].is_null(), "unset timeout must serialize as null (no schema default; issue #71)");
        assert!(config["hints"].is_null(), "unset hints must serialize as null");
        assert!(config["subagent_timeout"].is_null(), "unset subagent_timeout must serialize as null");
    }

    #[test]
    fn test_config_instance_get_full_output_nullifies_empty_strings_and_null_timeout() {
        // Empty hints must be normalized to null. Also covers the legacy case
        // where wait_timeout was explicitly NULL before the schema default
        // existed.
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, created_at, hints, wait_timeout) \
                 VALUES (?1, ?2, ?3, NULL)",
                rusqlite::params!["luna", crate::shared::time::now_epoch_f64(), ""],
            )
            .unwrap();

        let config = config_instance_get(&db, "luna", None).unwrap();
        assert!(config["hints"].is_null(), "empty hints must serialize as null");
        assert!(config["timeout"].is_null(), "explicit NULL wait_timeout must serialize as null");
    }

    #[test]
    fn test_config_instance_get_subagent_timeout_unset_returns_null_value() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES (?1, ?2)", rusqlite::params!["luna", crate::shared::time::now_epoch_f64()]).unwrap();

        let config = config_instance_get(&db, "luna", Some("subagent_timeout")).unwrap();
        assert_eq!(config["value"], serde_json::Value::Null);
    }

    #[test]
    fn test_render_config_instance_set_feedback_timeout_reset() {
        let rendered = render_config_instance_set_feedback("luna", "timeout", "default");
        assert_eq!(rendered, "Reset timeout for luna");
    }

    #[test]
    fn test_render_config_instance_set_feedback_subagent_timeout_contract() {
        let rendered = render_config_instance_set_feedback("luna", "subagent_timeout", "30");
        assert_eq!(rendered, "Set subagent_timeout for luna: 30s");
    }
}
