//! Cursor Agent native hook handlers and hooks.json management.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::integrations::hooks::{HookPayload, common};
use crate::log;
use crate::paths;
use crate::runtime::context::AgentxContext;
use crate::shared::{ST_ACTIVE, ST_LISTENING};

const AGENTX_TRIGGER: &str = "<agentx>";
const HOOK_TIMEOUT_SECS: u64 = 15;
const CURSOR_HOOK_COMMANDS: &[(&str, &str)] =
    &[("sessionStart", "cursor-sessionstart"), ("beforeSubmitPrompt", "cursor-beforesubmitprompt"), ("preToolUse", "cursor-pretooluse"), ("postToolUse", "cursor-posttooluse"), ("stop", "cursor-stop"), ("sessionEnd", "cursor-sessionend")];

#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("existing Cursor config at {} could not be read: {source}", path.display())]
    ExistingReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("existing Cursor config at {} is not valid JSON: {source}", path.display())]
    ExistingParseFailed {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("existing Cursor config at {} must be a JSON object", path.display())]
    ExistingRootNotObject { path: PathBuf },
    #[error("failed to create Cursor config directory {}: {source}", path.display())]
    DirCreateFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("JSON serialization failed: {0}")]
    SerializationFailed(#[from] serde_json::Error),
    #[error("atomic write to {} failed: {source}", path.display())]
    AtomicWriteFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("post-write Cursor hook verification failed for {}", .0.display())]
    PostWriteVerifyFailed(PathBuf),
    #[error("Cursor permissions setup failed for {}", .0.display())]
    PermissionsSetupFailed(PathBuf),
}

fn default_cursor_config_dir() -> PathBuf {
    crate::runtime::env::tool_home().join(".cursor")
}

pub fn get_cursor_hooks_path() -> PathBuf {
    default_cursor_config_dir().join("hooks.json")
}

pub fn get_cursor_permissions_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CURSOR_CONFIG_DIR")
        && !dir.is_empty()
    {
        return PathBuf::from(dir).join("cli-config.json");
    }
    if cfg!(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly"))
        && let Ok(dir) = std::env::var("XDG_CONFIG_HOME")
        && !dir.is_empty()
    {
        return PathBuf::from(dir).join("cursor").join("cli-config.json");
    }
    default_cursor_config_dir().join("cli-config.json")
}

fn build_cursor_hook_command(command: &str) -> String {
    let mut parts = crate::runtime::env::get_agentx_prefix();
    parts.push(command.to_string());
    parts.join(" ")
}

fn is_agentx_cursor_command(command: &str) -> bool {
    crate::integrations::hooks::runtime::is_agentx_command(command, &CURSOR_HOOK_COMMANDS.iter().map(|(_, suffix)| *suffix).collect::<Vec<_>>())
}

fn expected_hook(event: &str, command: &str) -> Value {
    let mut obj = serde_json::Map::from_iter([("command".to_string(), Value::String(build_cursor_hook_command(command))), ("timeout".to_string(), json!(HOOK_TIMEOUT_SECS))]);
    if event == "stop" {
        obj.insert("loop_limit".to_string(), Value::Null);
    }
    Value::Object(obj)
}

fn merge_agentx_hooks(root: &mut Value) {
    if !root.is_object() {
        *root = json!({});
    }
    let obj = root.as_object_mut().unwrap();
    obj.entry("version".to_string()).or_insert_with(|| json!(1));
    let hooks = obj.entry("hooks".to_string()).or_insert_with(|| json!({}));
    if !hooks.is_object() {
        *hooks = json!({});
    }
    let hooks = hooks.as_object_mut().unwrap();

    for (event, command) in CURSOR_HOOK_COMMANDS {
        let entries = hooks.entry((*event).to_string()).or_insert_with(|| json!([]));
        if !entries.is_array() {
            *entries = json!([]);
        }
        let entries = entries.as_array_mut().unwrap();
        entries.retain(|entry| !entry.get("command").and_then(Value::as_str).is_some_and(is_agentx_cursor_command));
        entries.push(expected_hook(event, command));
    }
}

fn remove_agentx_hooks(root: &mut Value) {
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return;
    };
    for entries in hooks.values_mut() {
        let Some(entries) = entries.as_array_mut() else {
            continue;
        };
        entries.retain(|entry| !entry.get("command").and_then(Value::as_str).is_some_and(is_agentx_cursor_command));
    }
    hooks.retain(|_, entries| entries.as_array().is_some_and(|entries| !entries.is_empty()));
}

fn read_json_object(path: &Path) -> Result<serde_json::Map<String, Value>, SetupError> {
    if !path.exists() {
        return Ok(serde_json::Map::new());
    }
    let content = std::fs::read_to_string(path).map_err(|source| SetupError::ExistingReadFailed { path: path.to_path_buf(), source })?;
    let value = serde_json::from_str::<Value>(&content).map_err(|source| SetupError::ExistingParseFailed { path: path.to_path_buf(), source })?;
    value.as_object().cloned().ok_or_else(|| SetupError::ExistingRootNotObject { path: path.to_path_buf() })
}

fn write_json(path: &Path, value: &Value) -> Result<(), SetupError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| SetupError::DirCreateFailed { path: parent.to_path_buf(), source })?;
    }
    let content = serde_json::to_string_pretty(value)?;
    paths::atomic_write_io(path, &content).map_err(|source| SetupError::AtomicWriteFailed { path: path.to_path_buf(), source })
}

fn verify_hooks_at(path: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(root) = serde_json::from_str::<Value>(&content) else {
        return false;
    };
    let Some(hooks) = root.get("hooks").and_then(Value::as_object) else {
        return false;
    };
    CURSOR_HOOK_COMMANDS.iter().all(|(event, command)| {
        hooks.get(*event).and_then(Value::as_array).is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry.get("command").and_then(Value::as_str) == Some(build_cursor_hook_command(command).as_str()) && entry.get("timeout").and_then(Value::as_u64).is_some() && (*event != "stop" || entry.get("loop_limit").is_some_and(Value::is_null))
            })
        })
    })
}

fn cursor_permission_rules() -> Vec<String> {
    let prefix = crate::runtime::env::build_agentx_command();
    common::SAFE_AGENTX_COMMANDS.iter().map(|command| format!("Shell({prefix} {command})")).collect()
}

fn all_cursor_permission_rules() -> Vec<String> {
    let mut rules = Vec::new();
    for prefix in ["agentx", "uvx agentx"] {
        rules.push(format!("Shell({prefix})"));
        for command in common::SAFE_AGENTX_COMMANDS {
            rules.push(format!("Shell({prefix} {command})"));
        }
    }
    rules
}

fn update_cursor_permissions_at(path: &Path, add: bool) -> Result<(), SetupError> {
    if !add && !path.exists() {
        return Ok(());
    }
    let mut root = read_json_object(path)?;
    if let Some(permissions) = root.get_mut("permissions").and_then(Value::as_object_mut)
        && let Some(allow) = permissions.get_mut("allow").and_then(Value::as_array_mut)
    {
        let managed = all_cursor_permission_rules();
        allow.retain(|entry| !entry.as_str().is_some_and(|entry| managed.iter().any(|rule| rule == entry)));
        if allow.is_empty() {
            permissions.remove("allow");
        }
    }
    if !add {
        if root.get("permissions").and_then(Value::as_object).is_some_and(|permissions| permissions.is_empty()) {
            root.remove("permissions");
        }
        return write_json(path, &Value::Object(root));
    }
    if path.file_name().and_then(|name| name.to_str()) == Some("cli-config.json") {
        root.entry("version".to_string()).or_insert_with(|| json!(1));
        root.entry("editor".to_string()).or_insert_with(|| json!({ "vimMode": false }));
    }
    let permissions = root.entry("permissions".to_string()).or_insert_with(|| json!({ "allow": [], "deny": [] }));
    if !permissions.is_object() {
        *permissions = json!({ "allow": [], "deny": [] });
    }
    let permissions = permissions.as_object_mut().unwrap();
    let allow = permissions.entry("allow".to_string()).or_insert_with(|| json!([]));
    if !allow.is_array() {
        *allow = json!([]);
    }
    let allow = allow.as_array_mut().unwrap();
    for rule in cursor_permission_rules() {
        if !allow.iter().any(|entry| entry.as_str() == Some(&rule)) {
            allow.push(Value::String(rule));
        }
    }
    permissions.entry("deny".to_string()).or_insert_with(|| json!([]));
    write_json(path, &Value::Object(root))
}

fn update_cursor_permissions(add: bool) -> Result<(), SetupError> {
    update_cursor_permissions_at(&get_cursor_permissions_path(), add)
}

fn verify_cursor_permissions() -> bool {
    let Ok(content) = std::fs::read_to_string(get_cursor_permissions_path()) else {
        return false;
    };
    let Ok(root) = serde_json::from_str::<Value>(&content) else {
        return false;
    };
    let expected = cursor_permission_rules();
    let managed = all_cursor_permission_rules();
    root.pointer("/permissions/allow").and_then(Value::as_array).is_some_and(|allow| {
        expected.iter().all(|rule| allow.iter().any(|entry| entry.as_str() == Some(rule))) && allow.iter().all(|entry| entry.as_str().is_none_or(|entry| !managed.iter().any(|rule| rule == entry) || expected.iter().any(|rule| rule == entry)))
    })
}

fn remove_cursor_hooks_at(path: &Path) -> bool {
    if !path.exists() {
        return true;
    }
    match read_json_object(path) {
        Ok(root) => {
            let mut value = Value::Object(root);
            remove_agentx_hooks(&mut value);
            write_json(path, &value).is_ok()
        }
        Err(_) => false,
    }
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_absolute() && !paths.contains(&path) {
        paths.push(path);
    }
}

fn cursor_hooks_cleanup_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    push_unique(&mut paths, default_cursor_config_dir().join("hooks.json"));
    push_unique(&mut paths, get_cursor_hooks_path());
    if let Some(root) = crate::runtime::env::legacy_tool_config_root() {
        push_unique(&mut paths, root.join(".cursor").join("hooks.json"));
    }
    paths
}

fn cursor_permissions_cleanup_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    push_unique(&mut paths, default_cursor_config_dir().join("cli-config.json"));
    if let Some(root) = crate::runtime::env::legacy_tool_config_root() {
        push_unique(&mut paths, root.join(".cursor").join("cli.json"));
    }
    if let Ok(dir) = std::env::var("CURSOR_CONFIG_DIR")
        && !dir.is_empty()
    {
        push_unique(&mut paths, PathBuf::from(dir).join("cli-config.json"));
    }
    if cfg!(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly"))
        && let Ok(dir) = std::env::var("XDG_CONFIG_HOME")
        && !dir.is_empty()
    {
        push_unique(&mut paths, PathBuf::from(dir).join("cursor").join("cli-config.json"));
    }
    push_unique(&mut paths, get_cursor_permissions_path());
    paths
}

pub fn remove_cursor_hooks() -> bool {
    let hooks_ok = cursor_hooks_cleanup_paths().iter().filter(|path| crate::runtime::env::hook_cleanup_allowed(path)).all(|path| remove_cursor_hooks_at(path));
    let permissions_ok = cursor_permissions_cleanup_paths().iter().filter(|path| crate::runtime::env::hook_cleanup_allowed(path)).all(|path| update_cursor_permissions_at(path, false).is_ok());
    hooks_ok && permissions_ok
}

pub fn try_setup_cursor_hooks(include_permissions: bool) -> Result<(), SetupError> {
    let hooks_path = get_cursor_hooks_path();
    let mut hooks = Value::Object(read_json_object(&hooks_path)?);
    merge_agentx_hooks(&mut hooks);
    write_json(&hooks_path, &hooks)?;
    if !verify_hooks_at(&hooks_path) {
        return Err(SetupError::PostWriteVerifyFailed(hooks_path));
    }
    if include_permissions {
        update_cursor_permissions(true)?;
        if !verify_cursor_permissions() {
            return Err(SetupError::PermissionsSetupFailed(get_cursor_permissions_path()));
        }
    } else {
        update_cursor_permissions(false)?;
    }
    Ok(())
}

pub fn verify_cursor_hooks_installed(check_permissions: bool) -> bool {
    verify_hooks_at(&get_cursor_hooks_path()) && (!check_permissions || verify_cursor_permissions())
}

fn cursor_session_env(ctx: &AgentxContext) -> Value {
    const KEYS: &[&str] = &["AGENTX_PROCESS_ID", "AGENTX_INSTANCE_NAME", "AGENTX_TOOL", "AGENTX_DIR", "AGENTX_LAUNCHED", "AGENTX_PTY_MODE", "AGENTX_BACKGROUND", "AGENTX_LAUNCHED_BY", "AGENTX_LAUNCH_BATCH_ID", "AGENTX_LAUNCH_EVENT_ID"];
    Value::Object(KEYS.iter().filter_map(|key| ctx.raw_env.get(*key).map(|value| ((*key).to_string(), Value::String(value.clone())))).collect())
}

fn route_client(hook: &super::client::HookClient, hook_name: &str, payload: &HookPayload) -> anyhow::Result<()> {
    let directory =
        payload.raw.get("cwd").and_then(Value::as_str).or_else(|| payload.raw.get("workspace_roots").and_then(Value::as_array).and_then(|roots| roots.first()).and_then(Value::as_str)).map(PathBuf::from).or_else(|| Some(hook.context.cwd.clone()));
    let actor = if hook_name == "cursor-sessionend" { hook.resolve("cursor")? } else { hook.session("cursor", payload, directory, false)? };
    let mut output = json!({});
    let mut mailbox = None;
    let mut bootstrap = None;
    match hook_name {
        "cursor-sessionstart" => {
            hook.status(&actor, ST_LISTENING, "start", "")?;
            hook.wake(&actor)?;
            crate::runtime::env::set_terminal_title(&actor.participant.canonical_name);
            crate::relay::worker::try_spawn_worker();
            bootstrap = hook.bootstrap(&actor, false)?;
            output = json!({"env":cursor_session_env(&hook.context)});
            if let Some((text, _)) = &bootstrap {
                output["additional_context"] = json!(text);
            }
        }
        "cursor-beforesubmitprompt" => {
            let prompt = payload.raw.get("prompt").and_then(Value::as_str).unwrap_or("");
            hook.status(&actor, ST_ACTIVE, if prompt.trim() == AGENTX_TRIGGER { "trigger" } else { "prompt" }, "")?;
            output = json!({"continue":true,"additional_context":hook.runtime_context(&actor)?});
        }
        "cursor-pretooluse" => hook.tool_status(&actor, payload)?,
        "cursor-posttooluse" => {
            mailbox = hook.pending(&actor)?;
            if let Some(pending) = &mailbox {
                output = json!({"additional_context":pending.formatted});
            }
        }
        "cursor-stop" => {
            hook.status(&actor, ST_LISTENING, "", "")?;
            hook.wake(&actor)?;
            if payload.raw.get("status").and_then(Value::as_str) == Some("completed") {
                mailbox = hook.pending(&actor)?;
                if let Some(pending) = &mailbox {
                    output = json!({"followup_message":pending.formatted});
                }
            }
        }
        "cursor-sessionend" => hook.stop(&actor, payload.session_id.as_deref(), payload.raw.get("reason").and_then(Value::as_str).unwrap_or("unknown"), false, false)?,
        _ => {}
    }
    if let Err(error) = hook.flush_json(&actor, &output, mailbox.as_ref(), bootstrap.as_ref().map(|(_, receipt)| receipt.as_str())) {
        log::log_warn("hooks", "cursor.output_error", &error.to_string());
    }
    Ok(())
}

/// Dispatch one Cursor JSON-on-stdin hook through daemon-owned capabilities.
pub fn dispatch_cursor_hook_native(hook_name: &str) -> i32 {
    let context = AgentxContext::from_os();
    if !common::hook_gate_check(&context) {
        return 0;
    }
    let raw: Value = match serde_json::from_reader(std::io::stdin().lock()) {
        Ok(raw) => raw,
        Err(error) => {
            log::log_warn("hooks", "cursor.parse_error", &error.to_string());
            return 0;
        }
    };
    let payload = HookPayload::from_cursor_native(hook_name, raw);
    let result = common::dispatch_with_panic_guard("cursor", hook_name, Ok(()), || {
        let hook = super::client::HookClient::connect(&context, hook_name == "cursor-sessionstart")?;
        route_client(&hook, hook_name, &payload)
    });
    if let Err(error) = result {
        log::log_warn("hooks", "cursor.dispatch_error", &error.to_string());
        use std::io::Write;
        // Cursor requires an explicit allow even when attribution or IPC fails.
        let output = if hook_name == "cursor-sessionstart" { json!({"env":cursor_session_env(&context),"continue":true}) } else { json!({"continue":true}) };
        let mut stdout = std::io::stdout().lock();
        if serde_json::to_writer(&mut stdout, &output).is_ok() {
            let _ = stdout.flush();
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::hooks::test_helpers::EnvGuard;
    use serial_test::serial;

    #[test]
    fn windows_executable_hooks_are_owned() {
        assert!(is_agentx_cursor_command(r#"& 'C:\Program Files\agentx.exe' cursor-stop"#));
        assert!(is_agentx_cursor_command("C:/dev/agentx.exe cursor-sessionstart"));
        assert!(!is_agentx_cursor_command("other-agentx.exe cursor-stop"));
        assert!(!is_agentx_cursor_command("user cursor-stop"));
        assert!(!is_agentx_cursor_command(r#"echo "agentx cursor-stop""#));
        assert!(!is_agentx_cursor_command("agentx cursor-stop-extra"));
    }

    fn cursor_test_env() -> (tempfile::TempDir, PathBuf, EnvGuard) {
        let guard = EnvGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        let home = dir.path().join("home");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("AGENTX_DIR", workspace.join(".agentx"));
            std::env::remove_var("CURSOR_CONFIG_DIR");
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        (dir, workspace, guard)
    }

    #[test]
    #[serial]
    fn setup_is_idempotent_and_preserves_existing_hooks() {
        let (dir, _workspace, _guard) = cursor_test_env();
        let hooks_path = dir.path().join("home/.cursor/hooks.json");
        std::fs::create_dir_all(hooks_path.parent().unwrap()).unwrap();
        std::fs::write(
            &hooks_path,
            serde_json::to_string_pretty(&json!({
                "version": 1,
                "hooks": {
                    "sessionStart": [{ "command": "./custom-start.sh" }]
                }
            }))
            .unwrap(),
        )
        .unwrap();

        try_setup_cursor_hooks(false).unwrap();
        let first = std::fs::read_to_string(&hooks_path).unwrap();
        try_setup_cursor_hooks(false).unwrap();
        let second = std::fs::read_to_string(&hooks_path).unwrap();

        assert_eq!(first, second);
        assert!(verify_cursor_hooks_installed(false));
        let root: Value = serde_json::from_str(&second).unwrap();
        assert!(root["hooks"]["sessionStart"].as_array().unwrap().iter().any(|hook| hook["command"] == "./custom-start.sh"));
        assert!(!dir.path().join("home/.cursor/cli-config.json").exists());
    }

    #[test]
    #[serial]
    fn permissions_cleanup_preserves_other_rules() {
        let (dir, _workspace, _guard) = cursor_test_env();
        let permissions_path = dir.path().join("home/.cursor/cli-config.json");
        std::fs::create_dir_all(permissions_path.parent().unwrap()).unwrap();
        std::fs::write(
            &permissions_path,
            serde_json::to_string_pretty(&json!({
                "permissions": {
                    "allow": ["Shell(custom)"]
                }
            }))
            .unwrap(),
        )
        .unwrap();

        try_setup_cursor_hooks(true).unwrap();
        assert!(verify_cursor_hooks_installed(true));
        try_setup_cursor_hooks(false).unwrap();

        let root: Value = serde_json::from_str(&std::fs::read_to_string(permissions_path).unwrap()).unwrap();
        assert_eq!(root["permissions"]["allow"], json!(["Shell(custom)"]));
    }

    #[test]
    #[serial]
    fn setup_replaces_legacy_prefixes_with_scoped_permissions() {
        let (dir, _workspace, _guard) = cursor_test_env();
        let permissions_path = dir.path().join("home/.cursor/cli-config.json");
        std::fs::create_dir_all(permissions_path.parent().unwrap()).unwrap();
        std::fs::write(
            &permissions_path,
            serde_json::to_string_pretty(&json!({
                "permissions": {
                    "allow": [
                        "Shell(custom)",
                        "Shell(agentx)",
                        "Shell(uvx agentx)",
                        "Shell(uvx agentx send)"
                    ]
                }
            }))
            .unwrap(),
        )
        .unwrap();

        try_setup_cursor_hooks(true).unwrap();

        let root: Value = serde_json::from_str(&std::fs::read_to_string(permissions_path).unwrap()).unwrap();
        let allow = root["permissions"]["allow"].as_array().unwrap();
        assert!(allow.iter().any(|rule| rule == "Shell(custom)"));
        assert!(!allow.iter().any(|rule| rule == "Shell(agentx)"));
        assert!(!allow.iter().any(|rule| rule == "Shell(uvx agentx)"));
        for rule in cursor_permission_rules() {
            assert!(allow.iter().any(|entry| entry == &rule), "missing {rule}");
        }
        assert!(!allow.iter().any(|rule| rule == "Shell(agentx kill)"));
        assert!(!allow.iter().any(|rule| rule == "Shell(agentx reset)"));
    }

    #[test]
    #[serial]
    fn setup_removes_stale_hook_prefixes() {
        let (dir, _workspace, _guard) = cursor_test_env();
        let hooks_path = dir.path().join("home/.cursor/hooks.json");
        std::fs::create_dir_all(hooks_path.parent().unwrap()).unwrap();
        std::fs::write(
            &hooks_path,
            serde_json::to_string_pretty(&json!({
                "hooks": {
                    "stop": [
                        { "command": "agentx cursor-stop" },
                        { "command": "uvx agentx cursor-stop" },
                        { "command": "./custom-stop.sh" }
                    ]
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let legacy = std::fs::read_to_string(&hooks_path).unwrap();
        try_setup_cursor_hooks(false).unwrap();

        let root: Value = serde_json::from_str(&std::fs::read_to_string(&hooks_path).unwrap()).unwrap();
        let stop = root["hooks"]["stop"].as_array().unwrap();
        assert_eq!(stop.iter().filter(|hook| hook["command"] == build_cursor_hook_command("cursor-stop")).count(), 1);
        assert!(stop.iter().any(|hook| hook["command"] == "./custom-stop.sh"));
        assert_eq!(stop.iter().filter(|hook| hook["command"].as_str().is_some_and(is_agentx_cursor_command)).count(), 1);
        std::fs::write(&hooks_path, legacy).unwrap();
        assert!(remove_cursor_hooks());
        let remaining = std::fs::read_to_string(&hooks_path).unwrap();
        assert!(!remaining.contains("uvx"));
        assert!(remaining.contains("./custom-stop.sh"));
    }

    #[test]
    #[serial]
    fn normal_mode_permissions_honor_cursor_config_dir() {
        let (dir, _workspace, _guard) = cursor_test_env();
        let home = dir.path().join("home");
        let override_dir = dir.path().join("cursor-override");
        unsafe {
            std::env::set_var("AGENTX_DIR", home.join(".agentx"));
            std::env::set_var("CURSOR_CONFIG_DIR", &override_dir);
        }

        assert_eq!(get_cursor_permissions_path(), override_dir.join("cli-config.json"));
    }

    #[test]
    #[serial]
    fn project_local_agentx_dir_does_not_move_cursor_config() {
        let (dir, _workspace, _guard) = cursor_test_env();
        let home = dir.path().join("home");

        assert_eq!(get_cursor_hooks_path(), home.join(".cursor/hooks.json"));
        assert_eq!(get_cursor_permissions_path(), home.join(".cursor/cli-config.json"));
    }

    #[test]
    #[serial]
    fn normal_mode_permissions_honor_xdg_on_supported_platforms() {
        if !cfg!(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly")) {
            return;
        }
        let (dir, _workspace, _guard) = cursor_test_env();
        let home = dir.path().join("home");
        let xdg = dir.path().join("xdg");
        unsafe {
            std::env::set_var("AGENTX_DIR", home.join(".agentx"));
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }

        assert_eq!(get_cursor_permissions_path(), xdg.join("cursor/cli-config.json"));
    }

    #[test]
    #[serial]
    fn remove_preserves_unrelated_hooks() {
        let (dir, _workspace, _guard) = cursor_test_env();
        let hooks_path = dir.path().join("home/.cursor/hooks.json");
        std::fs::create_dir_all(hooks_path.parent().unwrap()).unwrap();
        std::fs::write(
            &hooks_path,
            serde_json::to_string_pretty(&json!({
                "version": 1,
                "hooks": {
                    "sessionEnd": [{ "command": "./custom-end.sh" }]
                }
            }))
            .unwrap(),
        )
        .unwrap();

        try_setup_cursor_hooks(false).unwrap();
        assert!(remove_cursor_hooks());

        let root: Value = serde_json::from_str(&std::fs::read_to_string(hooks_path).unwrap()).unwrap();
        assert_eq!(root["hooks"]["sessionEnd"], json!([{ "command": "./custom-end.sh" }]));
        assert!(root["hooks"].as_object().unwrap().get("sessionStart").is_none());
    }

    // Unix-only: same $HOME-redirection limitation as
    // `normal_mode_permissions_honor_cursor_config_dir`.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn remove_cleans_default_and_isolated_paths() {
        let (dir, workspace, _guard) = cursor_test_env();
        let home = dir.path().join("home");
        let hooks_paths = [home.join(".cursor/hooks.json"), workspace.join(".cursor/hooks.json")];
        let permissions_paths = [home.join(".cursor/cli-config.json"), workspace.join(".cursor/cli.json")];
        for path in &hooks_paths {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                path,
                serde_json::to_string_pretty(&json!({
                    "hooks": {
                        "stop": [
                            { "command": "agentx cursor-stop" },
                            { "command": "./custom-stop.sh" }
                        ]
                    }
                }))
                .unwrap(),
            )
            .unwrap();
        }
        for path in &permissions_paths {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                path,
                serde_json::to_string_pretty(&json!({
                    "permissions": {
                        "allow": ["Shell(agentx)", "Shell(custom)"]
                    }
                }))
                .unwrap(),
            )
            .unwrap();
        }

        assert!(remove_cursor_hooks());

        for path in hooks_paths {
            let root: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert_eq!(root["hooks"]["stop"], json!([{ "command": "./custom-stop.sh" }]));
        }
        for path in permissions_paths {
            let root: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert_eq!(root["permissions"]["allow"], json!(["Shell(custom)"]));
        }
    }
}
