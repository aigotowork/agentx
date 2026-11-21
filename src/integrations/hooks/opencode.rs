//! OpenCode/Kilo hook handlers. `opencode-start` binds the session; status,
//! read and stop are the shared plugin RPC handlers (see `plugin_rpc`).

use anyhow::{Context, Result, bail};
use serde_json::Value;
use serde_json::value::RawValue;

use crate::runtime::context::AgentxContext;
#[cfg(test)]
use crate::shared::ST_LISTENING;
#[cfg(test)]
use crate::{
    agents::{binding as instance_binding, lifecycle, records as instances},
    db::AgentxDb,
    log::{log_error, log_info},
    runtime::bootstrap,
};

use crate::integrations::hooks::common;
#[cfg(test)]
use crate::integrations::hooks::plugin_rpc;
use crate::integrations::hooks::plugin_rpc::parse_flag;
use crate::integrations::hooks::runtime::{self, LaunchCtx, PerRunAdapter, RuntimeInjection};

/// Extract `--flag value` or `--flag=value` from argv.
fn parse_value_arg(argv: &[String], flags: &[&str]) -> Option<String> {
    for (idx, token) in argv.iter().enumerate() {
        for flag in flags {
            if token == flag {
                return argv.get(idx + 1).cloned();
            }
            let prefix = format!("{flag}=");
            if let Some(value) = token.strip_prefix(&prefix)
                && !value.is_empty()
            {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// `provider/model` or OpenCode 2's `provider/model#variant` as the plugin's
/// model JSON; a separate OpenCode 1 `--variant` wins over an inline one.
fn parse_launch_model(raw: &str, variant: Option<&str>) -> Option<Value> {
    let (provider_id, rest) = raw.split_once('/')?;
    let (model_id, inline_variant) = match rest.split_once('#') {
        Some((model_id, variant)) => (model_id, Some(variant)),
        None => (rest, None),
    };
    if provider_id.is_empty() || model_id.is_empty() {
        return None;
    }
    let mut model = serde_json::json!({
        "providerID": provider_id,
        "modelID": model_id,
    });
    if let Some(variant) = variant.or(inline_variant).filter(|v| !v.is_empty()) {
        model["variant"] = Value::String(variant.to_string());
    }
    Some(model)
}

pub(super) fn launch_agent_and_model_from_args(launch_args: Option<&str>) -> (Option<String>, Option<Value>) {
    let Some(raw_args) = launch_args.filter(|value| !value.is_empty()) else {
        return (None, None);
    };
    let argv: Vec<String> = match serde_json::from_str(raw_args) {
        Ok(args) => args,
        Err(_) => return (None, None),
    };

    let agent = parse_value_arg(&argv, &["--agent"]);
    let variant = parse_value_arg(&argv, &["--variant"]);
    let model = parse_value_arg(&argv, &["--model", "-m"]).and_then(|value| parse_launch_model(&value, variant.as_deref()));
    (agent, model)
}

#[cfg(test)]
fn launch_agent_and_model(db: &AgentxDb, instance_name: &str) -> (Option<String>, Option<Value>) {
    db.get_instance_full(instance_name).ok().flatten().map(|instance| launch_agent_and_model_from_args(instance.launch_args.as_deref())).unwrap_or((None, None))
}

/// Upsert plugin notify endpoint in DB.
#[cfg(test)]
fn upsert_plugin_notify_endpoint(db: &AgentxDb, instance_name: &str, port: u16) {
    if let Err(e) = db.upsert_notify_endpoint(instance_name, "plugin", port) {
        log_error("native", "opencode.register_notify_fail", &format!("Failed to register plugin notify port for {}: {}", instance_name, e));
    }
}

#[cfg(test)]
fn instance_tool(db: &AgentxDb, instance_name: &str) -> String {
    db.get_instance_full(instance_name).ok().flatten().map(|instance| instance.tool).filter(|tool| tool == "opencode" || tool == "kilo").unwrap_or_else(|| "opencode".to_string())
}

/// Get the OpenCode-family SQLite database path for an instance tool.
///
/// Both apps use XDG_DATA_HOME. Kilo additionally supports `KILO_DB`, which
/// may be absolute or relative to Kilo's data directory.
#[cfg(test)]
fn get_family_db_path(tool: &str) -> Option<String> {
    crate::runtime::env::opencode_family_db_path(tool).filter(|p| p.exists()).map(|p| p.to_string_lossy().to_string())
}

#[cfg(test)]
fn get_opencode_db_path() -> Option<String> {
    get_family_db_path("opencode")
}

/// Handle opencode-start: bind session to process, set listening status.
///
/// Called by OpenCode plugin on session.created event.
/// Expects: agentx opencode-start --session-id <id> [--notify-port <port>]
///
/// Returns JSON: {"name": "<instance>", "session_id": "<id>", "bootstrap": "..."}
#[cfg(test)]
fn handle_start(ctx: &AgentxContext, db: &AgentxDb, argv: &[String]) -> (i32, String) {
    let session_id = match parse_flag(argv, "--session-id") {
        Some(sid) => sid,
        None => return (0, r#"{"error":"Missing --session-id"}"#.to_string()),
    };

    let notify_port: Option<u16> = parse_flag(argv, "--notify-port").and_then(|s| s.parse().ok());

    let process_id = match &ctx.process_id {
        Some(pid) => pid.clone(),
        None => return (0, r#"{"error":"AGENTX_PROCESS_ID not set"}"#.to_string()),
    };

    // Re-binding detection: session already bound (compaction or reconnect)
    if let Ok(Some(existing_name)) = db.get_session_binding(&session_id) {
        // Only the launch attached to the instance rebinds it: after `agentx stop`
        // and `agentx r`, the old process's reloaded plugin must not take back
        // the resumed agent's notify endpoint.
        let attached = db.get_process_binding(&process_id).ok().flatten();
        if attached.as_deref() != Some(existing_name.as_str()) {
            return plugin_rpc::error(format!("{existing_name} is not attached to this process"));
        }
        let tool = instance_tool(db, &existing_name);
        let mut rebind_updates = serde_json::Map::new();
        rebind_updates.insert("name_announced".into(), serde_json::json!(false));
        rebind_updates.insert("session_id".into(), serde_json::json!(&session_id));

        if let Some(db_path) = get_family_db_path(&tool) {
            rebind_updates.insert("transcript_path".into(), serde_json::json!(db_path));
        }

        instances::update_instance_position(db, &existing_name, &rebind_updates);
        lifecycle::set_status(db, &existing_name, ST_LISTENING, "start", Default::default());

        let bootstrap_text = bootstrap::get_bootstrap(db, ctx, &existing_name, &tool);

        if let Some(port) = notify_port {
            upsert_plugin_notify_endpoint(db, &existing_name, port);
        }

        log_info("hooks", "opencode-start.rebind", &format!("instance={} session_id={}", existing_name, session_id));

        let (launch_agent, launch_model) = launch_agent_and_model(db, &existing_name);
        let mut result = serde_json::json!({
            "name": existing_name,
            "session_id": session_id,
        });
        result["bootstrap"] = Value::String(bootstrap_text);
        if let Some(agent) = launch_agent {
            result["agent"] = Value::String(agent);
        }
        if let Some(model) = launch_model {
            result["model"] = model;
        }
        return (0, serde_json::to_string(&result).unwrap_or_default());
    }

    // Normal binding path
    let instance_name = match instance_binding::bind_session_to_process(db, &session_id, Some(&process_id)) {
        Some(name) => name,
        None => {
            return (0, r#"{"error":"No instance bound to this process"}"#.to_string());
        }
    };
    let tool = instance_tool(db, &instance_name);

    // Rebind session and initialize
    if let Err(e) = db.rebind_instance_session(&instance_name, &session_id) {
        log_error("hooks", "hook.error", &format!("hook=opencode-start op=rebind_session err={}", e));
    }

    // Initialize last_event_id BEFORE set_status(): set_status wakes the
    // plugin, which would otherwise deliver the whole backlog from 0.
    plugin_rpc::initialize_last_event_id(db, &instance_name);

    lifecycle::set_status(db, &instance_name, ST_LISTENING, "start", Default::default());

    // Capture launch context (preserves pane_id/terminal_preset from Rust PTY)
    instance_binding::capture_and_store_launch_context(db, &instance_name);

    // Update instance position
    let mut updates = serde_json::Map::new();
    updates.insert("session_id".into(), serde_json::json!(&session_id));
    if let Some(db_path) = get_family_db_path(&tool) {
        updates.insert("transcript_path".into(), serde_json::json!(db_path));
    }
    if !ctx.cwd.as_os_str().is_empty() {
        updates.insert("directory".into(), serde_json::json!(ctx.cwd.to_string_lossy()));
    }
    instances::update_instance_position(db, &instance_name, &updates);

    // Register TCP notify endpoint
    if let Some(port) = notify_port {
        upsert_plugin_notify_endpoint(db, &instance_name, port);
    }
    // The session exists now, so the plugin's standby port is spent.
    let _ = db.delete_notify_endpoint(&instance_name, crate::runtime::notify::WakeKind::SessionStart.as_str());

    let bootstrap_text = bootstrap::get_bootstrap(db, ctx, &instance_name, &tool);

    // Auto-spawn relay-worker now that an instance is active
    crate::relay::worker::try_spawn_worker();

    let (launch_agent, launch_model) = launch_agent_and_model(db, &instance_name);
    let mut response = serde_json::json!({
        "name": instance_name,
        "session_id": session_id,
    });
    response["bootstrap"] = Value::String(bootstrap_text);
    if let Some(agent) = launch_agent {
        response["agent"] = Value::String(agent);
    }
    if let Some(model) = launch_model {
        response["model"] = model;
    }
    (0, serde_json::to_string(&response).unwrap_or_default())
}

/// Handle opencode-standby: the plugin loaded with no session and can create
/// one on request. Registers its port as the instance's `session_start`
/// endpoint; the delivery thread wakes it once the TUI is ready and mail waits.
///
/// Expects: agentx opencode-standby --port <port>
#[cfg(test)]
fn handle_standby(ctx: &AgentxContext, db: &AgentxDb, argv: &[String]) -> (i32, String) {
    let Some(port) = parse_flag(argv, "--port").and_then(|s| s.parse::<u16>().ok()) else {
        return (0, r#"{"error":"Missing --port"}"#.to_string());
    };
    let Some(process_id) = &ctx.process_id else {
        return (0, r#"{"error":"AGENTX_PROCESS_ID not set"}"#.to_string());
    };
    let Ok(Some(name)) = db.get_process_binding(process_id) else {
        return (0, r#"{"error":"No instance bound to this process"}"#.to_string());
    };
    let kind = crate::runtime::notify::WakeKind::SessionStart.as_str();
    if let Err(e) = db.upsert_notify_endpoint(&name, kind, port) {
        return (0, serde_json::json!({"error": format!("register failed: {e}")}).to_string());
    }
    (0, serde_json::json!({"name": name}).to_string())
}

/// `opencode-status` carries both status changes and tool activity
/// (`--tool T --input-json J` from `tool.execute.before`).
#[cfg(test)]
fn handle_status(db: &AgentxDb, argv: &[String]) -> (i32, String) {
    if parse_flag(argv, "--tool").is_some() {
        let tool = parse_flag(argv, "--name").map(|name| instance_tool(db, &name)).unwrap_or_else(|| "opencode".to_string());
        return plugin_rpc::handle_tool(db, argv, &tool);
    }
    plugin_rpc::handle_status(db, argv)
}

/// Dispatch an OpenCode hook by name.
///
/// Returns (exit_code, stdout_output).
/// All OpenCode hooks return exit 0 (no blocking behavior).
pub fn dispatch_opencode_hook(hook_name: &str, argv: &[String]) -> (i32, String) {
    if hook_name == "opencode-approve" {
        let context = AgentxContext::from_os();
        let argv = argv.strip_prefix(&[hook_name.to_owned()]).unwrap_or(argv);
        return handle_approve(&context, argv);
    }
    super::plugin_client::dispatch("opencode", hook_name, argv, false)
}

/// Embedded agentx.ts plugin source (compiled into the binary).
pub const PLUGIN_SOURCE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/integrations/plugins/opencode/agentx.ts"));

const PLUGIN_FILENAME: &str = "agentx.ts";

pub static OPENCODE_PER_RUN: PerRunAdapter = PerRunAdapter { prepare: prepare_per_run, cleanup_legacy: cleanup_legacy_per_run, ensure_permissions: None, managed_value_flags: &[], strip_legacy_args: None };

pub static KILO_PER_RUN: PerRunAdapter = PerRunAdapter { prepare: prepare_per_run, cleanup_legacy: cleanup_legacy_per_run, ensure_permissions: None, managed_value_flags: &[], strip_legacy_args: None };

fn family_runtime(ctx: &LaunchCtx) -> (&'static str, &'static str) {
    match ctx.tool {
        crate::integrations::tool::Tool::OpenCode => ("opencode", "OPENCODE_CONFIG_CONTENT"),
        crate::integrations::tool::Tool::Kilo => ("kilo", "KILO_CONFIG_CONTENT"),
        _ => unreachable!("OpenCode runtime adapter used for {}", ctx.tool.as_str()),
    }
}

fn jsonc_to_json(input: &str) -> Result<String> {
    let bytes = input.as_bytes();
    let mut stripped = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escaped = false;
    while i < bytes.len() {
        let byte = bytes[i];
        if in_string {
            stripped.push(byte);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            stripped.push(byte);
            i += 1;
            continue;
        }
        if byte == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            if i < bytes.len() {
                stripped.push(b'\n');
                i += 1;
            }
            continue;
        }
        if byte == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            let mut closed = false;
            while i + 1 < bytes.len() {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    closed = true;
                    break;
                }
                if bytes[i] == b'\n' {
                    stripped.push(b'\n');
                }
                i += 1;
            }
            if !closed {
                bail!("unterminated block comment");
            }
            continue;
        }
        stripped.push(byte);
        i += 1;
    }

    let mut out = Vec::with_capacity(stripped.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escaped = false;
    while i < stripped.len() {
        let byte = stripped[i];
        if in_string {
            out.push(byte);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            out.push(byte);
            i += 1;
            continue;
        }
        if byte == b',' {
            let mut next = i + 1;
            while next < stripped.len() && stripped[next].is_ascii_whitespace() {
                next += 1;
            }
            if next < stripped.len() && matches!(stripped[next], b'}' | b']') {
                i += 1;
                continue;
            }
        }
        out.push(byte);
        i += 1;
    }
    String::from_utf8(out).context("JSONC input was not valid UTF-8")
}

/// A JSON object in its written order. serde_json's `Value` sorts object keys,
/// and OpenCode 1/Kilo resolve permission rules by position (last match wins),
/// so re-sorting a caller's config could flip their allow/deny rules.
#[derive(Debug)]
struct OrderedObject(Vec<(String, Box<RawValue>)>);

impl OrderedObject {
    fn parse(json: &str) -> Result<Self> {
        struct Entries;
        impl<'de> serde::de::Visitor<'de> for Entries {
            type Value = Vec<(String, Box<RawValue>)>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> std::result::Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    entries.push(entry);
                }
                Ok(entries)
            }
        }
        let mut de = serde_json::Deserializer::from_str(json);
        let entries = serde::Deserializer::deserialize_map(&mut de, Entries)?;
        de.end()?;
        Ok(Self(entries))
    }

    /// The effective value of `key`: JSON parsers keep the last duplicate.
    fn get(&self, key: &str) -> Option<&RawValue> {
        self.0.iter().rev().find(|(k, _)| k == key).map(|(_, v)| &**v)
    }

    /// Replace `key`'s value in place, or append it.
    fn set(&mut self, key: &str, value: Box<RawValue>) {
        match self.0.iter_mut().rev().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => self.0.push((key.to_string(), value)),
        }
    }

    fn to_raw(&self) -> Result<Box<RawValue>> {
        let mut out = String::from("{");
        for (i, (key, value)) in self.0.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&serde_json::to_string(key)?);
            out.push(':');
            out.push_str(value.get());
        }
        out.push('}');
        Ok(RawValue::from_string(out)?)
    }
}

fn raw_json<T: serde::Serialize + ?Sized>(value: &T) -> Result<Box<RawValue>> {
    Ok(RawValue::from_string(serde_json::to_string(value)?)?)
}

fn parse_caller_config(ctx: &LaunchCtx, env_var: &str) -> Result<OrderedObject> {
    let Some(raw) = ctx.var(env_var) else {
        return Ok(OrderedObject(Vec::new()));
    };
    let parsed = jsonc_to_json(raw).and_then(|json| OrderedObject::parse(&json).context("must contain a JSON/JSONC object"));
    match parsed {
        Ok(config) => Ok(config),
        Err(error) if ctx.tool == crate::integrations::tool::Tool::Kilo => {
            let message = format!("Ignoring invalid KILO_CONFIG_CONTENT: {error:#}");
            crate::log::log_warn("launcher", "kilo.config_content_invalid", &message);
            eprintln!("Warning: {message}");
            Ok(OrderedObject(Vec::new()))
        }
        Err(error) => Err(error).with_context(|| format!("Invalid {env_var}")),
    }
}

fn prepare_per_run(ctx: &LaunchCtx) -> Result<RuntimeInjection> {
    let (app, env_var) = family_runtime(ctx);
    // Published as a directory holding `index.ts`: OpenCode 2 rejects a plugin
    // path that is a file ("configured plugin path must be a directory"), and
    // OpenCode 1/Kilo resolve a directory without package.json to its index.
    let dir = runtime::publish_dir(app, &[("index.ts", PLUGIN_SOURCE.as_bytes())]).with_context(|| format!("Cannot publish {app} runtime plugin"))?;
    let plugin_url = runtime::file_url(&dir);
    let caller = parse_caller_config(ctx, env_var)?;
    // OpenCode 2 turns each config source's permissions into rules and appends
    // them, so agentx's rules add to the user's. OpenCode 1 and Kilo deep-merge
    // the sources instead, where agentx's `bash` object would replace a user's
    // plain `"bash": "allow"`; there the plugin approves agentx's own commands
    // at runtime (`opencode-approve`).
    let permission_rules = ctx.auto_approve && ctx.tool == crate::integrations::tool::Tool::OpenCode && crate::integrations::launch_args::opencode::is_opencode_2();
    let config = runtime_config(caller, &plugin_url, permission_rules).with_context(|| format!("Invalid {env_var}"))?;
    let mut env = vec![(env_var.to_string(), config)];
    if ctx.tool == crate::integrations::tool::Tool::Kilo {
        // Kilo's TUI attaches to a running background daemon when one exists,
        // and that daemon never sees this launch's KILO_CONFIG_CONTENT, so the
        // plugin would not load. Run the server in-process instead.
        env.push(("KILO_NO_DAEMON".to_string(), "1".to_string()));
    }
    Ok(RuntimeInjection { args: ctx.args.clone(), env })
}

/// The caller's config content plus agentx's plugin (and, for OpenCode 2, its
/// auto-approve rules), with everything the caller wrote left in its order.
fn runtime_config(mut config: OrderedObject, plugin_url: &str, permission_rules: bool) -> Result<String> {
    let mut plugins: Vec<Box<RawValue>> = match config.get("plugin") {
        Some(raw) => serde_json::from_str(raw.get()).context("plugin must be an array")?,
        None => Vec::new(),
    };
    // An inherited value (e.g. `agentx launch --tool opencode` run from an agentx agent's shell)
    // can carry another agentx version's plugin; two copies in one process would
    // both claim the host, so keep only the current one.
    plugins.retain(|plugin| !serde_json::from_str::<String>(plugin.get()).is_ok_and(|path| runtime::is_agentx_runtime_path(&path)));
    plugins.push(raw_json(plugin_url)?);
    config.set("plugin", raw_json(&plugins)?);
    if permission_rules {
        add_auto_approve_rules(&mut config).context("permission")?;
    }
    Ok(config.to_raw()?.get().to_string())
}

/// Append agentx's auto-approve rules (`permission.bash["<agentx> <cmd>*"] =
/// "allow"`) after the caller's own. OpenCode 2 migrates the legacy
/// `permission` key to rules (bash → shell), resolves a command with the last
/// matching rule, and loads config content last, so these allows win over a
/// broader `ask` from elsewhere. A caller string action (`"bash": "ask"`,
/// `"permission": "ask"`) becomes a `"*"` rule ahead of them.
fn add_auto_approve_rules(config: &mut OrderedObject) -> Result<()> {
    fn rules(raw: Option<&RawValue>) -> Result<OrderedObject> {
        let Some(raw) = raw else {
            return Ok(OrderedObject(Vec::new()));
        };
        if serde_json::from_str::<String>(raw.get()).is_ok() {
            return Ok(OrderedObject(vec![("*".to_string(), raw.to_owned())]));
        }
        OrderedObject::parse(raw.get()).context("must be a string or an object")
    }
    let mut permission = rules(config.get("permission"))?;
    let mut bash = rules(permission.get("bash")).context("bash")?;
    let prefix = crate::runtime::env::build_agentx_command();
    let allow = raw_json("allow")?;
    for command in common::SAFE_AGENTX_COMMANDS {
        bash.set(&format!("{prefix} {command}*"), allow.clone());
    }
    permission.set("bash", bash.to_raw()?);
    config.set("permission", permission.to_raw()?);
    Ok(())
}

/// `opencode-approve --command C` → `{"approve": bool}`: whether the plugin
/// may answer an OpenCode 1/Kilo bash permission request itself, because the
/// command is exactly one safe agentx command and auto-approve is on.
fn handle_approve(ctx: &AgentxContext, argv: &[String]) -> (i32, String) {
    let command = parse_flag(argv, "--command").unwrap_or_default();
    // POSIX shells only: the check parses POSIX quoting.
    let approve = common::auto_approve_enabled(ctx) && !cfg!(windows) && common::is_safe_agentx_command(&command);
    (0, serde_json::json!({ "approve": approve }).to_string())
}

fn discovery_roots(ctx: &LaunchCtx) -> Vec<std::path::PathBuf> {
    let (app, config_dir_env, disable_project_env, project_names): (&str, &str, &str, &[&str]) = match ctx.tool {
        crate::integrations::tool::Tool::OpenCode => ("opencode", "OPENCODE_CONFIG_DIR", "OPENCODE_DISABLE_PROJECT_CONFIG", &[".opencode"]),
        crate::integrations::tool::Tool::Kilo => ("kilo", "KILO_CONFIG_DIR", "KILO_DISABLE_PROJECT_CONFIG", &[".kilocode", ".kilo"]),
        _ => unreachable!("OpenCode discovery used for {}", ctx.tool.as_str()),
    };
    let home = ctx.home();
    let config_home = ctx.path_var("XDG_CONFIG_HOME").unwrap_or_else(|| home.join(".config"));

    let mut roots = vec![config_home.join(app)];
    if let Some(custom) = ctx.path_var(config_dir_env) {
        roots.push(custom);
    }

    for name in project_names {
        roots.push(home.join(name));
    }

    let project_disabled = matches!(
        ctx.var(disable_project_env)
            .map(|value| value.to_ascii_lowercase()),
        Some(value) if value == "true" || value == "1"
    );
    if !project_disabled {
        for ancestor in ctx.cwd.ancestors() {
            for name in project_names {
                roots.push(ancestor.join(name));
            }
            if ancestor.join(".git").exists() {
                break;
            }
        }
    }

    let mut deduped = Vec::with_capacity(roots.len());
    for root in roots {
        if !deduped.contains(&root) {
            deduped.push(root);
        }
    }
    deduped
}

fn cleanup_legacy_per_run(ctx: &LaunchCtx) -> Result<()> {
    let paths = discovery_roots(ctx).into_iter().flat_map(|root| ["plugin", "plugins"].map(|dir| root.join(dir).join(PLUGIN_FILENAME)));
    runtime::remove_owned_files(paths, is_agentx_owned).map(drop)
}

fn current_home_dir() -> std::path::PathBuf {
    crate::runtime::env::user_home().unwrap_or_default()
}

/// Resolve the user config home (`XDG_CONFIG_HOME`, else `~/.config` on
/// Unix/macOS or `%APPDATA%` on Windows), falling back to `~/.config` if
/// unresolvable.
fn xdg_config_home() -> String {
    crate::runtime::env::user_config_home().unwrap_or_else(|| current_home_dir().join(".config")).to_string_lossy().into_owned()
}

/// The XDG global plugin dir for an OpenCode-family app, where older agentx
/// installed its plugin.
fn plugin_dir_for_app(app: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(xdg_config_home()).join(app).join("plugins")
}

#[cfg(test)]
fn get_opencode_plugin_dir() -> std::path::PathBuf {
    plugin_dir_for_app("opencode")
}

/// Get the canonical install path for the agentx.ts plugin.
#[cfg(test)]
fn get_opencode_plugin_path() -> std::path::PathBuf {
    get_opencode_plugin_dir().join(PLUGIN_FILENAME)
}

#[cfg(test)]
fn get_kilo_plugin_path() -> std::path::PathBuf {
    plugin_dir_for_app("kilo").join(PLUGIN_FILENAME)
}

/// Remove agentx.ts from ALL plugin directories for an app.
///
/// Checks all candidate directories directly (without filtering by dir existence)
/// to avoid missing stale plugins when path resolution differs between install/remove.
fn remove_plugin(app: &str) -> std::io::Result<bool> {
    let mut paths = vec![plugin_dir_for_app(app).join(PLUGIN_FILENAME)];

    // Build candidate paths from all known locations (skip dir-exists filter
    // that scan_plugin_dirs uses — a dir might not show as existing due to
    // mount/symlink differences but the file inside might still be reachable).
    let xdg_base = std::path::PathBuf::from(xdg_config_home()).join(app);
    for sub in &["plugin", "plugins"] {
        let p = xdg_base.join(sub).join(PLUGIN_FILENAME);
        if !paths.contains(&p) {
            paths.push(p);
        }
    }
    let config_dir_env = if app == "kilo" { "KILO_CONFIG_DIR" } else { "OPENCODE_CONFIG_DIR" };
    if let Ok(custom_dir) = std::env::var(config_dir_env) {
        let custom_base = std::path::PathBuf::from(custom_dir);
        for sub in &["plugin", "plugins"] {
            let p = custom_base.join(sub).join(PLUGIN_FILENAME);
            if !paths.contains(&p) {
                paths.push(p);
            }
        }
    }
    if let Some(root) = crate::runtime::env::legacy_tool_config_root() {
        let tool_base = root.join(format!(".{app}"));
        for sub in &["plugin", "plugins"] {
            let p = tool_base.join(sub).join(PLUGIN_FILENAME);
            if !paths.contains(&p) {
                paths.push(p);
            }
        }
    }

    let mut removed = false;
    for p in paths {
        if is_agentx_owned(&p)? {
            std::fs::remove_file(&p)?;
            removed = true;
        }
    }
    Ok(removed)
}

/// Marker line in `src/integrations/plugins/opencode/agentx.ts` identifying agentx's plugin.
pub const PLUGIN_MARKER: &str = "// agentx-managed-plugin";

/// True when `path` holds an agentx OpenCode/Kilo plugin: the current source, the
/// marker, or a pre-marker version (every one exports `AgentxPlugin` and reads
/// `AGENTX_DIR`). A user's own `agentx.ts` is not matched.
pub fn is_agentx_owned(path: &std::path::Path) -> std::io::Result<bool> {
    crate::integrations::hooks::runtime::file_is_agentx_owned(path, |content| content == PLUGIN_SOURCE || content.contains(PLUGIN_MARKER) || (content.contains("AgentxPlugin") && content.contains("AGENTX_DIR")))
}

pub fn remove_opencode_plugin() -> std::io::Result<bool> {
    remove_plugin("opencode")
}

pub fn remove_kilo_plugin() -> std::io::Result<bool> {
    remove_plugin("kilo")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::hooks::test_helpers::EnvGuard;
    use serial_test::serial;

    fn sv(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    /// Create a fresh test DB in a temp directory with schema initialized.
    fn test_db() -> (tempfile::TempDir, AgentxDb) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();
        (dir, db)
    }

    // ── Argv parsing ──

    #[test]
    fn test_parse_flag_found() {
        let argv = sv(&["--session-id", "abc", "--notify-port", "12345"]);
        assert_eq!(parse_flag(&argv, "--session-id"), Some("abc".to_string()));
        assert_eq!(parse_flag(&argv, "--notify-port"), Some("12345".to_string()));
    }

    #[test]
    fn test_parse_flag_not_found() {
        let argv = sv(&["--session-id", "abc"]);
        assert_eq!(parse_flag(&argv, "--name"), None);
    }

    #[test]
    fn test_parse_flag_at_end() {
        // Flag at end with no value
        let argv = sv(&["--session-id"]);
        assert_eq!(parse_flag(&argv, "--session-id"), None);
    }

    #[test]
    fn test_parse_value_arg_supports_split_and_equals_forms() {
        let split = sv(&["--agent", "reviewer", "-m", "openai/gpt-5.4"]);
        assert_eq!(parse_value_arg(&split, &["--agent"]), Some("reviewer".to_string()));
        assert_eq!(parse_value_arg(&split, &["--model", "-m"]), Some("openai/gpt-5.4".to_string()));

        let equals = sv(&["--agent=planner", "--model=anthropic/claude-sonnet-4-6"]);
        assert_eq!(parse_value_arg(&equals, &["--agent"]), Some("planner".to_string()));
        assert_eq!(parse_value_arg(&equals, &["--model", "-m"]), Some("anthropic/claude-sonnet-4-6".to_string()));
    }

    #[test]
    fn test_parse_launch_model_validates_provider_and_model() {
        assert_eq!(
            parse_launch_model("openai/gpt-5.4", None),
            Some(serde_json::json!({
                "providerID": "openai",
                "modelID": "gpt-5.4",
            }))
        );
        assert_eq!(parse_launch_model("openai", None), None);
        assert_eq!(parse_launch_model("/gpt-5.4", None), None);
        assert_eq!(parse_launch_model("openai/", None), None);
        assert_eq!(parse_launch_model("openai/#high", None), None);
    }

    #[test]
    fn test_parse_launch_model_carries_variant() {
        let high = serde_json::json!({
            "providerID": "openai",
            "modelID": "gpt-5.4",
            "variant": "high",
        });
        // OpenCode 2 inline form.
        assert_eq!(parse_launch_model("openai/gpt-5.4#high", None), Some(high.clone()));
        // OpenCode 1 separate flag, which wins over an inline variant.
        assert_eq!(parse_launch_model("openai/gpt-5.4", Some("high")), Some(high.clone()));
        assert_eq!(parse_launch_model("openai/gpt-5.4#low", Some("high")), Some(high));
        // An empty inline variant is just the model.
        assert_eq!(parse_launch_model("openai/gpt-5.4#", None), Some(serde_json::json!({"providerID": "openai", "modelID": "gpt-5.4"})));

        let launch_args = serde_json::to_string(&sv(&["--model", "anthropic/claude-opus-4", "--variant", "max"])).unwrap();
        let (_, model) = launch_agent_and_model_from_args(Some(&launch_args));
        assert_eq!(model.unwrap()["variant"], "max");
    }

    #[test]
    fn test_launch_agent_and_model_from_args_parses_stored_launch_args() {
        let launch_args = serde_json::to_string(&sv(&["--agent=planner", "--model", "openai/gpt-5.4"])).unwrap();

        let (agent, model) = launch_agent_and_model_from_args(Some(&launch_args));
        assert_eq!(agent.as_deref(), Some("planner"));
        assert_eq!(
            model,
            Some(serde_json::json!({
                "providerID": "openai",
                "modelID": "gpt-5.4",
            }))
        );
    }

    #[test]
    fn test_launch_agent_and_model_from_args_supports_short_model_flag() {
        let launch_args = serde_json::to_string(&sv(&["--agent", "reviewer", "-m", "anthropic/claude-opus-4"])).unwrap();

        let (agent, model) = launch_agent_and_model_from_args(Some(&launch_args));
        assert_eq!(agent.as_deref(), Some("reviewer"));
        assert_eq!(
            model,
            Some(serde_json::json!({
                "providerID": "anthropic",
                "modelID": "claude-opus-4",
            }))
        );
    }

    #[test]
    fn test_launch_agent_and_model_reads_instance_launch_args() {
        let (_dir, db) = test_db();
        let launch_args = serde_json::to_string(&sv(&["--agent", "reviewer", "--model", "openai/gpt-5.4"])).unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, tool, status, created_at, launch_args)
                 VALUES (?1, 'opencode', 'active', 0, ?2)",
                rusqlite::params!["luna", launch_args],
            )
            .unwrap();

        let (agent, model) = launch_agent_and_model(&db, "luna");
        assert_eq!(agent.as_deref(), Some("reviewer"));
        assert_eq!(
            model,
            Some(serde_json::json!({
                "providerID": "openai",
                "modelID": "gpt-5.4",
            }))
        );
    }

    // ── Plugin management ──

    #[test]
    fn test_plugin_source_contains_entrypoint() {
        assert!(PLUGIN_SOURCE.contains("AgentxPlugin"));
        assert!(PLUGIN_SOURCE.contains("AGENTX_PLUGIN_HOST_PID"));
        assert!(PLUGIN_SOURCE.contains("if (!claimPluginHost()) return {}"));
        assert!(PLUGIN_SOURCE.contains("if (!claimPluginHost()) return async () => {}"));
    }

    #[test]
    #[serial]
    fn test_per_run_config_merges_jsonc_plugins_in_order() {
        let (_dir, agentx, home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let ctx = LaunchCtx {
            tool: crate::integrations::tool::Tool::OpenCode,
            env: [
                ("HOME".to_string(), home.to_string_lossy().into_owned()),
                (
                    "OPENCODE_CONFIG_CONTENT".to_string(),
                    r#"{
                      // caller plugin order must survive
                      "plugin": ["first", "second",],
                      "theme": "dark",
                    }"#
                    .to_string(),
                ),
            ]
            .into_iter()
            .collect(),
            cwd: home,
            args: vec!["--session".into(), "abc".into()],
            auto_approve: false,
        };
        let injection = (OPENCODE_PER_RUN.prepare)(&ctx).unwrap();
        assert_eq!(injection.args, ctx.args);
        let (_, raw) = injection.env.iter().find(|(key, _)| key == "OPENCODE_CONFIG_CONTENT").unwrap();
        let config: serde_json::Value = serde_json::from_str(raw).unwrap();
        assert_eq!(config["theme"], "dark");
        let plugins = config["plugin"].as_array().unwrap();
        assert_eq!(plugins[0], "first");
        assert_eq!(plugins[1], "second");
        let runtime = plugins[2].as_str().unwrap();
        assert!(runtime.starts_with("file://"));
        assert!(runtime.contains("/integrations/opencode/"));
        // A directory (OpenCode 2 rejects file plugin paths) with index.ts.
        #[cfg(unix)]
        {
            let dir = std::path::PathBuf::from(runtime.strip_prefix("file://").unwrap());
            assert!(dir.join("index.ts").is_file(), "{runtime}");
        }
        // Decodes the URL, so Windows `file:///C:/…` compares as a native path.
        assert!(runtime::is_agentx_runtime_path(runtime), "{runtime}");
        assert!(runtime::integrations_dir().starts_with(&agentx));
    }

    #[test]
    #[serial]
    fn test_per_run_config_replaces_inherited_agentx_plugin() {
        let (_dir, agentx, home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let stale = runtime::file_url(&agentx.join("integrations").join("opencode").join("0ld").join("agentx.ts"));
        let ctx =
            LaunchCtx { tool: crate::integrations::tool::Tool::Kilo, env: [("KILO_CONFIG_CONTENT".to_string(), serde_json::json!({"plugin": ["mine", stale]}).to_string())].into_iter().collect(), cwd: home, args: Vec::new(), auto_approve: false };
        let injection = (KILO_PER_RUN.prepare)(&ctx).unwrap();
        let config: serde_json::Value = serde_json::from_str(&injection.env[0].1).unwrap();
        let plugins = config["plugin"].as_array().unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0], "mine");
        assert_ne!(plugins[1].as_str(), Some(stale.as_str()));
        assert!(plugins[1].as_str().unwrap().contains("/integrations/kilo/"));
    }

    fn prepared_config(tool: crate::integrations::tool::Tool, caller: Option<Value>, auto_approve: bool) -> (Value, Vec<(String, String)>) {
        let dir = tempfile::tempdir().unwrap();
        let env_var = match tool {
            crate::integrations::tool::Tool::Kilo => "KILO_CONFIG_CONTENT",
            _ => "OPENCODE_CONFIG_CONTENT",
        };
        let ctx = LaunchCtx { tool, env: caller.map(|c| (env_var.to_string(), c.to_string())).into_iter().collect(), cwd: dir.path().to_path_buf(), args: Vec::new(), auto_approve };
        let injection = prepare_per_run(&ctx).unwrap();
        let (_, raw) = injection.env.iter().find(|(k, _)| k == env_var).unwrap();
        (serde_json::from_str(raw).unwrap(), injection.env.clone())
    }

    fn config_with(caller: &str, permission_rules: bool) -> String {
        runtime_config(OrderedObject::parse(caller).unwrap(), "file:///agentx", permission_rules).unwrap()
    }

    #[test]
    fn test_runtime_config_keeps_caller_key_order() {
        // Last match wins in OpenCode 1/Kilo: sorting these would deny `git push`.
        let caller = r#"{"theme":"x","permission":{"bash":{"git push*":"deny","git *":"allow"}},"plugin":["mine"]}"#;
        assert_eq!(config_with(caller, false), r#"{"theme":"x","permission":{"bash":{"git push*":"deny","git *":"allow"}},"plugin":["mine","file:///agentx"]}"#);
    }

    #[test]
    #[serial]
    fn test_runtime_config_auto_approve_rules_follow_caller_rules() {
        let (_dir, _agentx, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let prefix = crate::runtime::env::build_agentx_command();
        let send = format!("{prefix} send*");

        let config: Value = serde_json::from_str(&config_with("{}", true)).unwrap();
        assert_eq!(config["permission"]["bash"][&send], "allow");
        assert!(config["permission"]["bash"].get(format!("{prefix} kill*")).is_none());

        let config: Value = serde_json::from_str(&config_with("{}", false)).unwrap();
        assert!(config.get("permission").is_none());

        // Caller rules come first, in their order; a string action becomes
        // the "*" fallback.
        let config = config_with(r#"{"permission":{"edit":"deny","bash":{"z *":"ask","a *":"deny"}}}"#, true);
        assert!(config.starts_with(r#"{"permission":{"edit":"deny","bash":{"z *":"ask","a *":"deny","#));
        let config = config_with(r#"{"permission":{"bash":"deny"}}"#, true);
        assert!(config.starts_with(r#"{"permission":{"bash":{"*":"deny","#));
        let config: Value = serde_json::from_str(&config_with(r#"{"permission":"ask"}"#, true)).unwrap();
        assert_eq!(config["permission"]["*"], "ask");
        assert_eq!(config["permission"]["bash"][&send], "allow");
    }

    #[test]
    #[serial]
    fn test_per_run_kilo_never_writes_permission_rules() {
        let (_dir, _agentx, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let (config, env) = prepared_config(crate::integrations::tool::Tool::Kilo, None, true);
        assert!(config.get("permission").is_none());
        assert!(env.iter().all(|(k, _)| !k.ends_with("_PERMISSION")));
    }

    #[test]
    fn test_approve_only_single_safe_agentx_commands_with_auto_approve() {
        let ctx = |on: bool| {
            let mut env = std::collections::HashMap::new();
            if on {
                env.insert("AGENTX_AUTO_APPROVE".to_string(), "1".to_string());
            }
            AgentxContext::from_env(&env, std::path::PathBuf::from("/tmp"))
        };
        let approve = |on: bool, command: &str| {
            let (_, out) = handle_approve(&ctx(on), &["--command".to_string(), command.to_string()]);
            serde_json::from_str::<Value>(&out).unwrap()["approve"] == true
        };
        assert_eq!(approve(true, "agentx send @x -- hi"), !cfg!(windows));
        assert!(!approve(true, "agentx send @x -- hi; rm -rf ~"));
        assert!(!approve(true, "agentx kill x"));
        assert!(!approve(false, "agentx send @x -- hi"));
    }

    #[test]
    #[serial]
    fn test_per_run_kilo_bypasses_daemon() {
        let (_dir, _agentx, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let (_, env) = prepared_config(crate::integrations::tool::Tool::Kilo, None, false);
        assert!(env.contains(&("KILO_NO_DAEMON".to_string(), "1".to_string())));
        let (_, env) = prepared_config(crate::integrations::tool::Tool::OpenCode, None, false);
        assert!(env.iter().all(|(k, _)| k != "KILO_NO_DAEMON"));
    }

    #[test]
    fn test_per_run_opencode_rejects_invalid_config_content() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = LaunchCtx { tool: crate::integrations::tool::Tool::OpenCode, env: [("OPENCODE_CONFIG_CONTENT".to_string(), "{ broken".to_string())].into_iter().collect(), cwd: dir.path().to_path_buf(), args: Vec::new(), auto_approve: false };
        let err = parse_caller_config(&ctx, "OPENCODE_CONFIG_CONTENT").unwrap_err();
        assert!(err.to_string().contains("OPENCODE_CONFIG_CONTENT"));
    }

    #[test]
    fn test_per_run_cleanup_removes_effective_discovery_roots_only_when_owned() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let xdg = dir.path().join("xdg");
        let project = dir.path().join("repo");
        let cwd = project.join("nested");
        let custom = dir.path().join("custom");
        std::fs::create_dir_all(project.join(".git")).unwrap();
        std::fs::create_dir_all(&cwd).unwrap();

        let owned_paths = [xdg.join("opencode").join("plugin").join("agentx.ts"), home.join(".opencode").join("plugins").join("agentx.ts"), project.join(".opencode").join("plugin").join("agentx.ts"), custom.join("plugins").join("agentx.ts")];
        for path in &owned_paths {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, PLUGIN_SOURCE).unwrap();
        }
        let user = cwd.join(".opencode").join("plugins").join("agentx.ts");
        std::fs::create_dir_all(user.parent().unwrap()).unwrap();
        std::fs::write(&user, "export const Mine = async () => ({})").unwrap();

        let ctx = LaunchCtx {
            tool: crate::integrations::tool::Tool::OpenCode,
            env: [("HOME".to_string(), home.to_string_lossy().into_owned()), ("XDG_CONFIG_HOME".to_string(), xdg.to_string_lossy().into_owned()), ("OPENCODE_CONFIG_DIR".to_string(), custom.to_string_lossy().into_owned())].into_iter().collect(),
            cwd,
            args: Vec::new(),
            auto_approve: false,
        };
        (OPENCODE_PER_RUN.cleanup_legacy)(&ctx).unwrap();
        for path in owned_paths {
            assert!(!path.exists(), "{} should be removed", path.display());
        }
        assert!(user.exists(), "user plugin must survive");
    }

    #[test]
    #[serial]
    fn test_get_opencode_plugin_dir_defaults_to_xdg_global_path() {
        use crate::integrations::hooks::test_helpers::EnvVarGuard;
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let xdg = dir.path().join("xdg");
        std::fs::create_dir_all(home.join(".agentx")).unwrap();
        let _home = EnvVarGuard::set("HOME", &home);
        let _agentx = EnvVarGuard::set("AGENTX_DIR", home.join(".agentx"));
        let _xdg = EnvVarGuard::set("XDG_CONFIG_HOME", &xdg);

        assert_eq!(get_opencode_plugin_dir(), xdg.join("opencode").join("plugins"));
    }

    #[test]
    fn test_get_opencode_plugin_path() {
        let path = get_opencode_plugin_path();
        assert!(path.ends_with("agentx.ts"));
    }

    #[test]
    fn test_plugin_filename_constant() {
        assert_eq!(PLUGIN_FILENAME, "agentx.ts");
    }

    #[test]
    #[serial]
    fn test_project_local_agentx_dir_keeps_global_plugin_dir_and_cleans_legacy() {
        let _guard = EnvGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        let agentx_dir = workspace.join(".agentx");
        let home = dir.path().join("home");
        let xdg = dir.path().join("xdg");
        std::fs::create_dir_all(&agentx_dir).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        unsafe {
            std::env::set_var("AGENTX_DIR", &agentx_dir);
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
            std::env::remove_var("KILO_CONFIG_DIR");
        }

        assert_eq!(get_kilo_plugin_path(), xdg.join("kilo").join("plugins").join(PLUGIN_FILENAME));

        let legacy = workspace.join(".kilo").join("plugins").join(PLUGIN_FILENAME);
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, PLUGIN_SOURCE).unwrap();
        remove_kilo_plugin().unwrap();
        assert!(!legacy.exists());
    }

    #[test]
    #[serial]
    fn test_remove_supports_opencode_config_dir() {
        let _guard = EnvGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let xdg = dir.path().join("xdg");
        let custom = dir.path().join("custom-opencode");
        std::fs::create_dir_all(home.join(".agentx")).unwrap();
        std::fs::create_dir_all(custom.join("plugins")).unwrap();
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("AGENTX_DIR", home.join(".agentx"));
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
            std::env::set_var("OPENCODE_CONFIG_DIR", &custom);
        }

        let plugin_path = custom.join("plugins").join("agentx.ts");
        std::fs::write(&plugin_path, PLUGIN_SOURCE).unwrap();

        remove_opencode_plugin().unwrap();
        assert!(!plugin_path.exists());
    }

    #[test]
    #[serial]
    fn test_remove_supports_kilo_config_dir() {
        let _guard = EnvGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let xdg = dir.path().join("xdg");
        let custom = dir.path().join("custom-kilo");
        std::fs::create_dir_all(home.join(".agentx")).unwrap();
        std::fs::create_dir_all(custom.join("plugins")).unwrap();
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("AGENTX_DIR", home.join(".agentx"));
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
            std::env::set_var("KILO_CONFIG_DIR", &custom);
        }

        let plugin_path = custom.join("plugins").join("agentx.ts");
        std::fs::write(&plugin_path, PLUGIN_SOURCE).unwrap();

        remove_kilo_plugin().unwrap();
        assert!(!plugin_path.exists());
    }

    #[test]
    #[serial]
    fn test_remove_plugin_deletes_only_agentx_owned_files() {
        let _guard = EnvGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let xdg = dir.path().join("xdg");
        let custom = dir.path().join("custom-opencode");
        std::fs::create_dir_all(home.join(".agentx")).unwrap();
        std::fs::create_dir_all(xdg.join("opencode").join("plugins")).unwrap();
        std::fs::create_dir_all(custom.join("plugin")).unwrap();
        std::fs::create_dir_all(custom.join("plugins")).unwrap();
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("AGENTX_DIR", home.join(".agentx"));
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
            std::env::set_var("OPENCODE_CONFIG_DIR", &custom);
        }

        let user_file = xdg.join("opencode").join("plugins").join("agentx.ts");
        std::fs::write(&user_file, "export const Mine = async () => ({})").unwrap();
        // Pre-marker agentx version.
        let legacy = custom.join("plugin").join("agentx.ts");
        std::fs::write(&legacy, "const AGENTX_DIR = process.env.AGENTX_DIR\nexport const AgentxPlugin = async () => ({})").unwrap();
        let marked = custom.join("plugins").join("agentx.ts");
        std::fs::write(&marked, format!("{PLUGIN_MARKER}\nold body")).unwrap();

        assert!(PLUGIN_SOURCE.starts_with(PLUGIN_MARKER));
        remove_opencode_plugin().unwrap();
        assert!(user_file.exists(), "user's own agentx.ts must survive");
        assert!(!legacy.exists());
        assert!(!marked.exists());
    }

    #[test]
    #[serial]
    fn test_remove_plugin_reports_unreadable_plugin_path() {
        let _guard = EnvGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let custom = dir.path().join("custom-opencode");
        std::fs::create_dir_all(home.join(".agentx")).unwrap();
        // A directory where the plugin file should be can't be inspected.
        std::fs::create_dir_all(custom.join("plugins").join("agentx.ts")).unwrap();
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("AGENTX_DIR", home.join(".agentx"));
            std::env::set_var("XDG_CONFIG_HOME", dir.path().join("xdg"));
            std::env::set_var("OPENCODE_CONFIG_DIR", &custom);
        }
        let err = remove_opencode_plugin().unwrap_err();
        assert!(err.to_string().contains("agentx.ts"), "{err}");
    }

    // ── Transcript path ──

    #[test]
    fn test_get_opencode_db_path_missing() {
        // In test env, opencode db won't exist
        // This tests the "not found" path
        let result = get_opencode_db_path();
        // Could be Some or None depending on environment, just verify it doesn't panic
        let _ = result;
    }

    // ── Handler tests (unit-level, isolated DB) ──

    #[test]
    fn test_handle_start_missing_session_id() {
        crate::config::Config::init();
        let ctx = AgentxContext::from_os();
        let (_dir, db) = test_db();
        let argv = sv(&[]);
        let (code, output) = handle_start(&ctx, &db, &argv);
        assert_eq!(code, 0);
        assert!(output.contains("Missing --session-id"));
    }

    #[test]
    #[serial]
    fn test_handle_start_rebind_includes_launch_identity() {
        crate::config::Config::init();
        let (_env_dir, agentx_dir, test_home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let (_db_dir, db) = test_db();
        let launch_args = serde_json::to_string(&sv(&["--agent", "reviewer", "--model", "openai/gpt-5.4"])).unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, tool, status, created_at, launch_args)
                 VALUES (?1, 'opencode', 'active', 0, ?2)",
                rusqlite::params!["luna", launch_args],
            )
            .unwrap();
        db.set_session_binding("sess-1", "luna").unwrap();
        db.set_process_binding("pid-123", "sess-1", "luna").unwrap();

        let env = std::collections::HashMap::from([("AGENTX_DIR".to_string(), agentx_dir.to_string_lossy().to_string()), ("HOME".to_string(), test_home.to_string_lossy().to_string()), ("AGENTX_PROCESS_ID".to_string(), "pid-123".to_string())]);
        let ctx = AgentxContext::from_env(&env, std::path::PathBuf::from("/tmp"));

        let (code, output) = handle_start(&ctx, &db, &sv(&["--session-id", "sess-1"]));
        assert_eq!(code, 0);

        let payload: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(payload["name"], "luna");
        assert_eq!(payload["session_id"], "sess-1");
        assert_eq!(payload["agent"], "reviewer");
        assert_eq!(
            payload["model"],
            serde_json::json!({
                "providerID": "openai",
                "modelID": "gpt-5.4",
            })
        );
        assert!(payload["bootstrap"].as_str().is_some());
    }

    /// After `agentx stop` and `agentx r`, the old process's plugin may reload and
    /// call opencode-start for the session again. It must not rebind the
    /// resumed agent or take its notify endpoint.
    #[test]
    #[serial]
    fn test_handle_start_rebind_rejects_detached_process() {
        crate::config::Config::init();
        let (_env_dir, agentx_dir, test_home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let (_db_dir, db) = test_db();
        insert_instance(&db, "luna", "opencode");
        db.set_session_binding("sess-1", "luna").unwrap();
        db.set_process_binding("pid-new", "sess-1", "luna").unwrap();
        db.upsert_notify_endpoint("luna", "plugin", 4001).unwrap();

        let env = std::collections::HashMap::from([("AGENTX_DIR".to_string(), agentx_dir.to_string_lossy().to_string()), ("HOME".to_string(), test_home.to_string_lossy().to_string()), ("AGENTX_PROCESS_ID".to_string(), "pid-old".to_string())]);
        let ctx = AgentxContext::from_env(&env, std::path::PathBuf::from("/tmp"));
        let out = reply(handle_start(&ctx, &db, &sv(&["--session-id", "sess-1", "--notify-port", "4002"])));
        assert!(out.get("error").is_some(), "{out}");
        assert_eq!(db.get_notify_endpoint_port("luna", "plugin"), Some(4001));
        let inst = db.get_instance_full("luna").unwrap().unwrap();
        assert_eq!(inst.status_context, "idle");
    }

    fn insert_instance(db: &AgentxDb, name: &str, tool: &str) {
        db.conn()
            .execute(
                "INSERT INTO instances (name, tool, status, status_context, status_time, created_at)
                 VALUES (?1, ?2, 'listening', 'idle', 0, 0)",
                rusqlite::params![name, tool],
            )
            .unwrap();
    }

    fn reply(out: (i32, String)) -> Value {
        assert_eq!(out.0, 0);
        serde_json::from_str(&out.1).unwrap()
    }

    #[test]
    fn test_handle_status_reports_missing_args_and_unknown_instance() {
        let (_dir, db) = test_db();
        insert_instance(&db, "testinst", "opencode");
        for argv in [sv(&["--status", "listening"]), sv(&["--name", "testinst"]), sv(&["--name", "gone", "--status", "listening"]), sv(&["--name", "gone", "--tool", "bash"])] {
            let out = reply(handle_status(&db, &argv));
            assert!(out.get("error").is_some(), "{argv:?} -> {out}");
        }
    }

    #[test]
    fn test_handle_status_updates_status() {
        crate::config::Config::init();
        let (_dir, db) = test_db();
        insert_instance(&db, "testinst", "opencode");
        let argv = sv(&["--name", "testinst", "--status", "blocked", "--context", "approval", "--detail", "bash"]);
        assert_eq!(reply(handle_status(&db, &argv)), serde_json::json!({"ok": true}));
        let inst = db.get_instance_full("testinst").unwrap().unwrap();
        assert_eq!(inst.status, "blocked");
        assert_eq!(inst.status_context, "approval");
        assert_eq!(inst.status_detail, "bash");
    }

    /// Tool activity resolves the detail through the instance's own tool
    /// spec, so a Kilo instance maps like OpenCode.
    #[test]
    fn test_handle_status_tool_derives_detail_from_spec() {
        crate::config::Config::init();
        let (_dir, db) = test_db();
        insert_instance(&db, "kiki", "kilo");
        let input = serde_json::json!({"filePath": "/src/main.rs", "oldString": "a"});
        let argv = sv(&["--name", "kiki", "--tool", "edit", "--input-json", &input.to_string()]);
        assert_eq!(reply(handle_status(&db, &argv)), serde_json::json!({"ok": true}));
        let inst = db.get_instance_full("kiki").unwrap().unwrap();
        assert_eq!(inst.status, "active");
        assert_eq!(inst.status_context, "tool:edit");
        assert_eq!(inst.status_detail, "/src/main.rs");
    }

    #[test]
    #[serial]
    fn test_handle_start_initializes_cursor_to_launch_tail() {
        crate::config::Config::init();
        let (_env_dir, agentx_dir, test_home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let (_db_dir, db) = test_db();
        let old_id = db.log_event("message", "bigboss", &serde_json::json!({"from": "bigboss", "scope": "broadcast", "text": "old"})).unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, tool, status, created_at, last_event_id)
                 VALUES ('luna', 'opencode', 'pending', 0, 0)",
                [],
            )
            .unwrap();
        db.set_process_binding("pid-1", "", "luna").unwrap();
        let env = std::collections::HashMap::from([("AGENTX_DIR".to_string(), agentx_dir.to_string_lossy().to_string()), ("HOME".to_string(), test_home.to_string_lossy().to_string()), ("AGENTX_PROCESS_ID".to_string(), "pid-1".to_string())]);
        let ctx = AgentxContext::from_env(&env, std::path::PathBuf::from("/tmp"));
        let out = reply(handle_start(&ctx, &db, &sv(&["--session-id", "ses_new"])));
        assert_eq!(out["name"], "luna", "{out}");
        let inst = db.get_instance_full("luna").unwrap().unwrap();
        assert!(inst.last_event_id >= old_id, "cursor must skip the backlog");
        let unread = reply(plugin_rpc::handle_read(&db, &sv(&["--name", "luna"])));
        assert_eq!(unread["messages"], serde_json::json!([]));
    }

    #[test]
    #[serial]
    fn test_standby_port_registers_until_session_binds() {
        crate::config::Config::init();
        let (_env_dir, agentx_dir, test_home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let (_db_dir, db) = test_db();
        db.conn()
            .execute(
                "INSERT INTO instances (name, tool, status, created_at, last_event_id)
                 VALUES ('luna', 'opencode', 'pending', 0, 0)",
                [],
            )
            .unwrap();
        db.set_process_binding("pid-1", "", "luna").unwrap();
        let env = std::collections::HashMap::from([("AGENTX_DIR".to_string(), agentx_dir.to_string_lossy().to_string()), ("HOME".to_string(), test_home.to_string_lossy().to_string()), ("AGENTX_PROCESS_ID".to_string(), "pid-1".to_string())]);
        let ctx = AgentxContext::from_env(&env, std::path::PathBuf::from("/tmp"));
        let kind = crate::runtime::notify::WakeKind::SessionStart.as_str();

        let out = reply(handle_standby(&ctx, &db, &sv(&["--port", "4567"])));
        assert_eq!(out["name"], "luna", "{out}");
        assert!(db.has_notify_endpoint_kind("luna", kind));

        reply(handle_start(&ctx, &db, &sv(&["--session-id", "ses_new"])));
        assert!(!db.has_notify_endpoint_kind("luna", kind));
    }
}
