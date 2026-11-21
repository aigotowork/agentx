//! Translate the selected app-server CLI configuration into native per-thread
//! overrides. Managed daemon start does not honor process `-c` overrides.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::agents::attachments::ParticipantRef;
use crate::integrations::hooks::runtime::LaunchCtx;
use crate::integrations::tool::Tool;

use super::protocol::{NativeFrame, RawObject};
use super::service::{ServiceIdentity, environment_input_is_normalized};

#[cfg(test)]
#[path = "configuration/acceptance_tests.rs"]
mod acceptance_tests;

/// Classify the command position, skipping option values. Management, schema,
/// help and non-stdio listener invocations retain direct native execution.
pub(crate) fn is_shared_app_server(args: &[String]) -> bool {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if matches!(arg, "--help" | "-h" | "--version" | "-V" | "--") {
            return false;
        }
        if matches!(
            arg,
            "-c" | "--config" | "--enable" | "--disable" | "--profile" | "-p" | "--model" | "-m" | "--cd" | "-C" | "--sandbox" | "-s" | "--ask-for-approval" | "-a" | "--image" | "-i" | "--local-provider" | "--remote" | "--remote-auth-token-env"
        ) {
            index += 2;
            continue;
        }
        if arg.starts_with('-') {
            index += 1;
            continue;
        }
        if arg != "app-server" {
            return false;
        }
        let tail = &args[index + 1..];
        let mut next = 0;
        while next < tail.len() {
            let arg = tail[next].as_str();
            if matches!(arg, "daemon" | "proxy" | "generate-ts" | "generate-json-schema" | "--help" | "-h") {
                return false;
            }
            if arg == "--listen" {
                if tail.get(next + 1).is_none_or(|value| value != "stdio://") {
                    return false;
                }
                next += 2;
            } else if let Some(value) = arg.strip_prefix("--listen=") {
                if value != "stdio://" {
                    return false;
                }
                next += 1;
            } else if matches!(arg, "-c" | "--config" | "--enable" | "--disable") {
                next += 2;
            } else {
                next += 1;
            }
        }
        return true;
    }
    false
}

/// Complete caller argv becomes only a bare app-server acquisition plus
/// thread config. Unknown flags are rejected explicitly rather than ignored.
pub(crate) fn thread_overrides(args: &[String]) -> Result<RawObject> {
    let mut overrides = Value::Object(Default::default());
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "app-server" || arg == "--listen=stdio://" {
            index += 1;
            continue;
        }
        if arg == "--listen" && args.get(index + 1).is_some_and(|value| value == "stdio://") {
            index += 2;
            continue;
        }
        if matches!(arg, "-c" | "--config") {
            let raw = args.get(index + 1).context("Codex config override is missing its value")?;
            apply_override(&mut overrides, raw)?;
            index += 2;
        } else if let Some(raw) = arg.strip_prefix("-c=").or_else(|| arg.strip_prefix("--config=")) {
            apply_override(&mut overrides, raw)?;
            index += 1;
        } else if matches!(arg, "--enable" | "--disable") {
            let feature = args.get(index + 1).context("Codex feature flag is missing its value")?;
            let key = format!("features.{}={}", serde_json::to_string(feature)?, arg == "--enable");
            apply_override(&mut overrides, &key)?;
            index += 2;
        } else {
            bail!("Shared Codex app-server cannot translate native option {arg:?}; use supported -c config overrides");
        }
    }
    Ok(RawObject::parse(&serde_json::to_string(&overrides)?)?)
}

fn apply_override(root: &mut Value, raw: &str) -> Result<()> {
    let (key, literal) = raw.split_once('=').context("Codex config override must be key=value")?;
    let key = key.trim();
    let key_document: toml::Value = toml::from_str(&format!("{key} = 0"))?;
    let mut current = &key_document;
    let mut segments = Vec::new();
    while let Some(table) = current.as_table() {
        if table.len() != 1 {
            bail!("Invalid Codex config key {key}");
        }
        let (segment, next) = table.iter().next().unwrap();
        segments.push(segment.clone());
        current = next;
    }
    let document: toml::Value = toml::from_str(&format!("value = {literal}")).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::from_iter([("value".to_owned(), toml::Value::String(literal.to_owned()))])));
    let value = serde_json::to_value(&document["value"])?;
    let mut destination = root;
    for segment in segments.iter().take(segments.len().saturating_sub(1)) {
        if !destination.is_object() {
            *destination = Value::Object(Default::default());
        }
        destination = destination.as_object_mut().unwrap().entry(segment.clone()).or_insert_with(|| Value::Object(Default::default()));
    }
    let last = segments.last().context("Empty Codex config key")?;
    if !destination.is_object() {
        *destination = Value::Object(Default::default());
    }
    destination.as_object_mut().unwrap().insert(last.clone(), value);
    Ok(())
}

/// Only known native provider/MCP dependency fields are inspected. Opaque
/// unrelated config remains raw, including numbers outside Value's range.
fn validate_environment_dependencies(config: &RawObject) -> Result<()> {
    let check = |name: &str| -> Result<()> {
        if environment_input_is_normalized(name) {
            bail!("Shared Codex thread configuration depends on normalized environment input {name}; its backend value cannot be verified for this thread");
        }
        Ok(())
    };
    for section in ["model_providers", "mcp_servers"] {
        let Some(raw) = config.get(section).filter(|raw| raw.get() != "null") else {
            continue;
        };
        let entries: HashMap<String, Box<serde_json::value::RawValue>> = serde_json::from_str(raw.get())?;
        for entry in entries.values() {
            let entry = RawObject::parse(entry.get())?;
            for key in ["env_key", "bearer_token_env_var"] {
                if let Some(raw) = entry.get(key).filter(|raw| raw.get() != "null") {
                    check(&serde_json::from_str::<String>(raw.get())?)?;
                }
            }
            if let Some(raw) = entry.get("env_vars").filter(|raw| raw.get() != "null") {
                for name in serde_json::from_str::<Vec<String>>(raw.get())? {
                    check(&name)?;
                }
            }
            if let Some(raw) = entry.get("env_http_headers").filter(|raw| raw.get() != "null") {
                for name in serde_json::from_str::<HashMap<String, String>>(raw.get())?.values() {
                    check(name)?;
                }
            }
            if let Some(raw) = entry.get("env").filter(|raw| raw.get() != "null") {
                for name in serde_json::from_str::<HashMap<String, Box<serde_json::value::RawValue>>>(raw.get())?.keys() {
                    check(name)?;
                }
            }
        }
    }
    // Native profiles can declare the same provider/MCP inputs. Inspect those
    // declarations without interpreting arbitrary plugin or tool payloads.
    if let Some(raw) = config.get("profiles").filter(|raw| raw.get() != "null") {
        let profiles: HashMap<String, Box<serde_json::value::RawValue>> = serde_json::from_str(raw.get())?;
        for profile in profiles.values() {
            validate_environment_dependencies(&RawObject::parse(profile.get())?)?;
        }
    }
    Ok(())
}

pub(crate) struct SessionConfiguration {
    pub(crate) env: HashMap<String, String>,
    defaults: RawObject,
    pub(crate) cwd: String,
    auto_approve: bool,
}

impl SessionConfiguration {
    pub(crate) fn new(args: &[String], mut env: HashMap<String, String>, cwd: &Path, service: &ServiceIdentity, auto_approve: bool) -> Result<Self> {
        env.insert("AGENTX_CODEX_SHARED".to_owned(), "1".to_owned());
        env.insert("AGENTX_LAUNCHED".to_owned(), "1".to_owned());
        env.insert("AGENTX_TOOL".to_owned(), "codex".to_owned());
        env.insert("AGENTX_PROCESS_ID".to_owned(), service.execution.0.clone());
        env.insert("CODEX_HOME".to_owned(), service.canonical_home.to_string_lossy().into_owned());
        env.remove("AGENTX_INSTANCE_NAME");
        env.remove("AGENTX_PTY_MODE");
        Ok(Self { env, defaults: thread_overrides(args)?, cwd: cwd.to_string_lossy().into_owned(), auto_approve })
    }

    /// Apply defaults below caller request config, then append trusted AgentX
    /// hooks and reserved identity exports. Unrelated raw config survives.
    pub(crate) fn prepare_root(&self, raw: &str, participant: &ParticipantRef) -> Result<String> {
        let mut message = NativeFrame::parse(raw)?;
        let mut params = message.params()?.unwrap_or_else(RawObject::empty);
        let native_cwd: String = params.get("cwd").filter(|value| value.get() != "null").map(|value| serde_json::from_str(value.get())).transpose()?.unwrap_or_else(|| self.cwd.clone());
        let root_cwd = if Path::new(&native_cwd).is_absolute() { Path::new(&native_cwd).to_path_buf() } else { Path::new(&self.cwd).join(native_cwd) };
        let mut config = self.defaults.clone();
        if let Some(existing) = params.get("config").filter(|value| value.get() != "null") {
            config.overlay(&RawObject::parse(existing.get())?)?;
        }
        validate_environment_dependencies(&config)?;
        let mut hook_args = Vec::new();
        if let Some(hooks) = config.get("hooks") {
            let hooks: Value = serde_json::from_str(hooks.get())?;
            let hooks = toml::Value::try_from(hooks)?;
            hook_args.extend(["-c".to_owned(), format!("hooks={}", crate::integrations::hooks::codex::toml_literal(&hooks))]);
        }
        let hook_context = LaunchCtx { tool: Tool::Codex, env: self.env.clone(), cwd: root_cwd, args: hook_args, auto_approve: self.auto_approve };
        let hooks = crate::integrations::hooks::codex::prepare_shared(&hook_context)?;
        config.overlay(&thread_overrides(&hooks.args)?)?;

        let mut environment = self.env.clone();
        let mut shell = config.get("shell_environment_policy").filter(|value| value.get() != "null").map(|value| RawObject::parse(value.get())).transpose()?.unwrap_or_else(RawObject::empty);
        if let Some(existing) = shell.get("set").filter(|value| value.get() != "null") {
            let caller: HashMap<String, String> = serde_json::from_str(existing.get())?;
            environment.extend(caller);
        }
        for key in ["AGENTX_CODEX_SHARED", "AGENTX_LAUNCHED", "AGENTX_PROCESS_ID", "AGENTX_DIR", "AGENTX_TOOL", "CODEX_HOME"] {
            if let Some(value) = self.env.get(key) {
                environment.insert(key.to_owned(), value.clone());
            }
        }
        environment.insert("AGENTX_INSTANCE_NAME".to_owned(), participant.canonical_name.clone());
        let agentx_config = crate::config::AgentxConfig::load(Some(&self.env));
        let export = self.env.get("AGENTX_NAME_EXPORT").map(String::as_str).unwrap_or(&agentx_config.name_export);
        if !export.is_empty() {
            environment.insert(export.to_owned(), participant.canonical_name.clone());
        }
        if let Some(export) = self.env.get("AGENTX_PROCESS_ID_EXPORT").filter(|value| !value.is_empty()) {
            environment.insert(export.clone(), self.env["AGENTX_PROCESS_ID"].clone());
        }
        shell.set("set", &environment)?;
        config.set("shell_environment_policy", &shell)?;
        let bootstrap = crate::runtime::bootstrap::SHARED_CODEX_BOOTSTRAP;
        if let Some(existing) = params.get("developerInstructions").filter(|value| value.get() != "null") {
            let existing: String = serde_json::from_str(existing.get())?;
            params.set("developerInstructions", &format!("{bootstrap}\n{existing}"))?;
        } else {
            let existing = config.get("developer_instructions").filter(|value| value.get() != "null").map(|value| serde_json::from_str::<String>(value.get())).transpose()?.unwrap_or_default();
            config.set("developer_instructions", &format!("{bootstrap}\n{existing}"))?;
        }
        params.set("config", &config)?;
        if params.get("cwd").is_none_or(|value| value.get() == "null") {
            params.set("cwd", &self.cwd)?;
        }
        message.object.set("params", &params)?;
        Ok(message.object.render()?)
    }
}
