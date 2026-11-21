//! Scoped configuration file reads and validated edits shared by CLI and Relay.
//! The caller supplies scope/environment; this module never derives a daemon
//! caller from process-global configuration. Provider effects belong to runtime.
use crate::runtime::launcher::{LaunchTool, validate_tool_args};
use std::collections::HashMap;
use std::path::Path;

pub(crate) fn set_for_scope(scope: &Path, key: &str, value: &str) -> Result<(), String> {
    config_set_at_path(&scope.join("config.toml"), key, Some(value))
}
pub(crate) fn unset_for_scope(scope: &Path, key: &str) -> Result<(), String> {
    config_set_at_path(&scope.join("config.toml"), key, None)
}
pub(crate) fn drop_old_delivery_keys(scope: &Path) {
    drop_old_delivery_keys_at(&scope.join("config.toml"));
}

fn launch_tool_for_args_field(field_name: &str) -> Option<LaunchTool> {
    // Derive the field → tool map from the spec's `args_env` (the single source
    // of truth, e.g. `AGENTX_CLAUDE_ARGS` → `claude_args`) so a new tool's config
    // args are validated as soon as its spec declares `args_env` — no parallel
    // match to keep in sync. See `drift_released_tools_with_args_env_merge_...`.
    let spec = crate::integrations::registry::ALL.iter().find(|s| s.launch.args_env.is_some_and(|env| env.strip_prefix("AGENTX_").map(str::to_ascii_lowercase).as_deref() == Some(field_name)))?;
    LaunchTool::from_str(spec.name).ok()
}

pub(crate) fn validate_config_args(field_name: &str, value: &str) -> Result<(), String> {
    let Some(tool) = launch_tool_for_args_field(field_name) else {
        return Ok(());
    };
    if value.is_empty() {
        return Ok(());
    }

    let tokens = crate::integrations::launch_args::shell::shell_split(value, cfg!(windows)).map_err(|e| format!("invalid {field_name} from config: {e}"))?;
    let errors = validate_tool_args(&tool, &tokens);
    if errors.is_empty() { Ok(()) } else { Err(format!("invalid {field_name} from config: {}", errors.join("; "))) }
}

pub(crate) fn config_set_at_path(path: &Path, key: &str, value: Option<&str>) -> Result<(), String> {
    if let Some(path_key) = environment_key(key) {
        let new_value = match (path_key, value) {
            (_, None) => None,
            ("environment.files", Some(raw)) => {
                // CLI editing uses one quoted JSON string array. It preserves
                // path commas/spaces and maps directly to TOML's string array.
                let files: Vec<String> = serde_json::from_str(raw).map_err(|_| "env_files must be a JSON string array, for example '[\"shared.env\",\".env\"]'".to_string())?;
                if files.iter().any(String::is_empty) {
                    return Err("env_files paths cannot be empty; use [] to disable".into());
                }
                let mut array = toml_edit::Array::new();
                for file in files {
                    array.push(file);
                }
                Some(toml_edit::Value::Array(array))
            }
            (_, Some(raw)) => Some(crate::runtime::environment::parse_bool(raw).map_err(|error| error.to_string())?.into()),
        };
        return edit_config_at_path(path, |doc| {
            crate::config::set_doc_path(doc, path_key, new_value);
            Ok(true)
        });
    }
    let field = crate::config::user_field(key).ok_or_else(|| format!("Unknown config key '{key}'"))?;
    if let Some(value) = value {
        validate_config_args(field.name, value)?;
    }

    let new_value = value.map(|v| crate::config::field_toml_value(field, v)).transpose()?;
    edit_config_at_path(path, |doc| {
        crate::config::set_doc_path(doc, field.toml, new_value);
        // Refuse what the loader would reject and replace with the default.
        match crate::config::file_field_error(&doc.to_string(), field.name) {
            Some(err) => Err(err),
            None => Ok(true),
        }
    })
}

/// Read-modify-write config.toml under its lock. `edit` returns whether to
/// write the document back.
pub(crate) fn edit_config_at_path(path: &Path, edit: impl FnOnce(&mut toml_edit::DocumentMut) -> Result<bool, String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config directory: {e}"))?;
    }
    // Lock a stable sibling: the config itself is replaced by atomic rename.
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    let lock_file = std::fs::OpenOptions::new().write(true).create(true).truncate(false).open(&lock_path).map_err(|e| format!("Failed to open config lock: {e}"))?;
    crate::sys::fs::lock_exclusive(&lock_file).map_err(|e| format!("Failed to lock config: {e}"))?;

    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("Failed to read config.toml: {error}")),
    };
    let mut doc: toml_edit::DocumentMut = content.parse().map_err(|e| format!("Failed to parse config.toml: {e}"))?;
    if !edit(&mut doc)? {
        return Ok(());
    }
    crate::config::write_config_toml_path(path, &doc.to_string()).map_err(|e| format!("Failed to write config.toml: {e}"))
}

pub(crate) fn drop_old_delivery_keys_at(path: &Path) {
    const OLD_KEYS: [&str; 2] = ["launch.claude.delivery", "launch.copilot.delivery"];
    // Unlocked check on the text: almost always nothing to do. Also skips a
    // missing file (never created) and `idle_delivery`/`rpc_delivery`.
    let stale = std::fs::read_to_string(path).is_ok_and(|content| content.lines().any(|line| line.trim_start().starts_with("delivery")));
    if !stale {
        return;
    }
    let result = edit_config_at_path(path, |doc| {
        let before = doc.to_string();
        for key in OLD_KEYS {
            crate::config::set_doc_path(doc, key, None);
        }
        Ok(doc.to_string() != before)
    });
    if let Err(e) = result {
        crate::log::log_warn("config", "config.old_delivery_keys_failed", &e);
    }
}

pub(crate) fn environment_key(key: &str) -> Option<&'static str> {
    match key.to_ascii_uppercase().as_str() {
        "ENV_FILES" | "AGENTX_ENV_FILES" | "ENVIRONMENT.FILES" => Some("environment.files"),
        "ENV_OVERRIDE" | "AGENTX_ENV_OVERRIDE" | "ENVIRONMENT.OVERRIDE" => Some("environment.override"),
        _ => None,
    }
}

pub(crate) fn get_for_scope(scope: &Path, environment: &HashMap<String, String>, key: &str) -> (String, &'static str) {
    if let Some(path_key) = environment_key(key) {
        let env_key = if path_key == "environment.files" { "AGENTX_ENV_FILES" } else { "AGENTX_ENV_OVERRIDE" };
        if let Some(value) = environment.get(env_key) {
            return (value.clone(), "env");
        }
        if let Ok(settings) = crate::config::read_environment_settings(&scope.join("config.toml"), path_key == "environment.files", path_key == "environment.override") {
            if path_key == "environment.files" {
                if let Some(files) = settings.files {
                    return (serde_json::to_string(&files).unwrap_or_default(), "toml");
                }
            } else if let Some(value) = settings.override_values {
                return (value.to_string(), "toml");
            }
        }
        return ((if path_key == "environment.files" { "[]" } else { "false" }).into(), "default");
    }
    let Some(field) = crate::config::user_field(key) else {
        return match environment.get(key) {
            Some(val) => (val.clone(), "env"),
            None => (String::new(), "default"),
        };
    };
    if !field.is_relay()
        && let Some(val) = environment.get(field.env)
    {
        return (val.clone(), "env");
    }
    let (values, _) = crate::config::read_toml_values(&scope.join("config.toml"));
    if let Some(val) = values.get(field.name) {
        return (val.as_string(), "toml");
    }
    let default = crate::config::AgentxConfig::default().get_field(field.name).unwrap_or_default();
    let default = match field.kind {
        crate::config::FieldKind::Bool => (default == "1").to_string(),
        _ => default,
    };
    (default, "default")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scopes_and_invocation_overrides_remain_distinct_without_process_env_changes() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        set_for_scope(first.path(), "timeout", "111").unwrap();
        set_for_scope(second.path(), "timeout", "222").unwrap();
        let empty = HashMap::new();
        assert_eq!(get_for_scope(first.path(), &empty, "AGENTX_TIMEOUT"), ("111".into(), "toml"));
        assert_eq!(get_for_scope(second.path(), &empty, "AGENTX_TIMEOUT"), ("222".into(), "toml"));
        let environment = HashMap::from([("AGENTX_TIMEOUT".into(), "333".into()), ("AGENTX_RELAY_ID".into(), "must-be-ignored".into())]);
        assert_eq!(get_for_scope(first.path(), &environment, "AGENTX_TIMEOUT"), ("333".into(), "env"));
        set_for_scope(first.path(), "relay_id", "scoped-relay").unwrap();
        assert_eq!(get_for_scope(first.path(), &environment, "AGENTX_RELAY_ID"), ("scoped-relay".into(), "toml"));
    }
    #[test]
    fn invalid_edits_preserve_the_previous_document_and_unset_restores_default_source() {
        let scope = tempfile::tempdir().unwrap();
        set_for_scope(scope.path(), "timeout", "600").unwrap();
        let before = std::fs::read(scope.path().join("config.toml")).unwrap();
        assert!(set_for_scope(scope.path(), "timeout", "not-a-number").is_err());
        assert_eq!(std::fs::read(scope.path().join("config.toml")).unwrap(), before);
        unset_for_scope(scope.path(), "timeout").unwrap();
        assert_eq!(get_for_scope(scope.path(), &HashMap::new(), "AGENTX_TIMEOUT").1, "default");
    }
}

#[cfg(test)]
mod environment_tests {
    use super::*;

    #[test]
    fn environment_edits_preserve_order_empty_lists_and_other_fields() {
        let scope = tempfile::tempdir().unwrap();
        set_for_scope(scope.path(), "timeout", "321").unwrap();
        set_for_scope(scope.path(), "env_files", r#"["shared,one.env","project local.env"]"#).unwrap();
        set_for_scope(scope.path(), "env_override", "true").unwrap();
        let settings = crate::config::read_environment_settings(&scope.path().join("config.toml"), true, true).unwrap();
        assert_eq!(settings.files, Some(vec!["shared,one.env".into(), "project local.env".into()]));
        assert_eq!(settings.override_values, Some(true));
        let before = std::fs::read(scope.path().join("config.toml")).unwrap();
        assert!(set_for_scope(scope.path(), "env_files", "concealed-invalid-sentinel").is_err());
        assert_eq!(std::fs::read(scope.path().join("config.toml")).unwrap(), before);
        set_for_scope(scope.path(), "env_files", "[]").unwrap();
        assert_eq!(get_for_scope(scope.path(), &HashMap::new(), "env_files"), ("[]".into(), "toml"));
        unset_for_scope(scope.path(), "env_files").unwrap();
        assert_eq!(get_for_scope(scope.path(), &HashMap::new(), "env_files"), ("[]".into(), "default"));
        assert_eq!(get_for_scope(scope.path(), &HashMap::new(), "timeout"), ("321".into(), "toml"));
    }

    #[test]
    fn shadowed_environment_declarations_are_not_parsed_or_selected() {
        let scope = tempfile::tempdir().unwrap();
        std::fs::write(scope.path().join("config.toml"), "[environment]\nfiles=42\noverride=true\n").unwrap();
        assert!(crate::config::read_environment_settings(&scope.path().join("config.toml"), true, true).is_err());
        let selected = crate::config::read_environment_settings(&scope.path().join("config.toml"), false, true).unwrap();
        assert_eq!(selected.files, None);
        assert_eq!(selected.override_values, Some(true));
    }
}
