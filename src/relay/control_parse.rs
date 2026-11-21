//! Validate the established peer action grammar into finite native intentions.
use super::control::rpc_action;
use super::control_service::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const REMOTE_EVENTS_HARD_CAP: usize = 2000;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(super) enum Action {
    Host(HostAction),
    InstanceGet { target: String, field: Option<String> },
    InstanceSet { target: String, field: String, value: String },
    Events { filters: crate::events::filters::FilterMap, predicate: Option<String>, last: usize, max_bytes: Option<usize> },
    Subscribe { caller: String, external: bool, filters: crate::events::filters::FilterMap, predicates: Vec<String>, once: bool, on_hit: Option<String> },
    Subscriptions,
    Unsubscribe { id: String },
}

pub(super) fn required<'a>(params: &'a Value, key: &str) -> Result<&'a str, String> {
    optional(params, key).ok_or_else(|| format!("missing {key}"))
}
pub(super) fn optional<'a>(params: &'a Value, key: &str) -> Option<&'a str> {
    params.get(key).and_then(Value::as_str)
}
fn flag(params: &Value, key: &str) -> bool {
    params.get(key).and_then(Value::as_bool).unwrap_or(false)
}
fn strings(params: &Value, key: &str) -> Vec<String> {
    params.get(key).and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect()
}
fn count(params: &Value, key: &str, default: usize) -> usize {
    params.get(key).and_then(Value::as_u64).and_then(|value| usize::try_from(value).ok()).unwrap_or(default)
}
pub(super) fn normalize_field(field: &str) -> String {
    let field = field.to_uppercase();
    if field.starts_with("AGENTX_") { field } else { format!("AGENTX_{field}") }
}
pub(super) fn reject_secret(field: &str) -> Result<(), String> {
    let normalized = normalize_field(field);
    let field = match normalized.as_str() {
        "AGENTX_RELAY_PSK" => "relay_psk",
        "AGENTX_RELAY_TOKEN" => "relay_token",
        "AGENTX_RELAY_ID" => "relay_id",
        "AGENTX_RELAY" => "relay",
        _ => return Ok(()),
    };
    Err(format!("{field} is not remotely queryable"))
}
fn filters(params: &Value) -> Result<crate::events::filters::FilterMap, String> {
    params.get("filters").filter(|value| !value.is_null()).map(|value| serde_json::from_value(value.clone()).map_err(|error| format!("invalid filters param: {error}"))).unwrap_or_else(|| Ok(Default::default()))
}

impl RemoteLaunch {
    pub(super) fn from_params(params: &Value) -> Result<Self, String> {
        let fields = params.as_object().ok_or("launch parameters must be an object")?;
        const ALLOWED: &[&str] = &["tool", "selection", "count", "args", "launcher", "system_prompt", "initial_prompt", "background", "terminal", "cwd"];
        if let Some(key) = fields.keys().find(|key| !ALLOWED.contains(&key.as_str())) {
            return Err(format!("unknown launch parameter '{key}'"));
        }
        let count = count(params, "count", 1);
        if count == 0 {
            return Err("count must be at least 1".into());
        }
        let tool = optional(params, "tool").map(crate::config::profiles::canonical_tool).transpose().map_err(|error| error.to_string())?;
        let selection = params.get("selection").map(|value| serde_json::from_value(value.clone())).transpose().map_err(|error| format!("invalid launch selection: {error}"))?.unwrap_or_default();
        Ok(Self {
            tool,
            selection,
            count,
            args: strings(params, "args"),
            launcher: optional(params, "launcher").map(str::to_owned),
            system_prompt: optional(params, "system_prompt").map(str::to_owned),
            initial_prompt: optional(params, "initial_prompt").map(str::to_owned),
            background: flag(params, "background"),
            terminal: optional(params, "terminal").map(str::to_owned),
            cwd: optional(params, "cwd").map(str::to_owned),
        })
    }
}
impl RemoteResume {
    pub(super) fn from_params(params: &Value) -> Result<Self, String> {
        Ok(Self { target: required(params, "target")?.into(), fork: flag(params, "fork"), extra_args: strings(params, "extra_args"), launcher: optional(params, "launcher").map(str::to_owned) })
    }
}

pub(super) fn parse(action: &str, params: &Value) -> Result<Action, String> {
    use HostAction as H;
    let target = || required(params, "target").map(str::to_owned);
    let host = match action {
        rpc_action::LAUNCH => H::Launch(RemoteLaunch::from_params(params)?),
        rpc_action::LAUNCH_PROFILE => H::Launch(RemoteLaunch::from_params(params)?),
        rpc_action::LAUNCH_PREVIEW => H::PrepareLaunch(RemoteLaunch::from_params(params)?),
        rpc_action::KILL => H::Kill { target: target()? },
        rpc_action::RESUME => H::Resume(RemoteResume::from_params(params)?),
        rpc_action::CONFIG_GET => {
            if let Some(instance) = optional(params, "instance") {
                let field = optional(params, "field").map(str::to_owned);
                if let Some(field) = &field {
                    reject_secret(field)?;
                }
                return Ok(Action::InstanceGet { target: instance.into(), field });
            }
            let fields = strings(params, "fields");
            for field in &fields {
                reject_secret(field)?;
            }
            H::ConfigGet { fields }
        }
        rpc_action::CONFIG_SET => {
            let field = required(params, "field")?;
            reject_secret(field)?;
            let value = required(params, "value")?.to_owned();
            if let Some(instance) = optional(params, "instance") {
                return if field == "timeout" && (value.is_empty() || value.eq_ignore_ascii_case("default")) {
                    Ok(Action::Host(H::InstanceTimeoutDefault { target: instance.into(), requested: value }))
                } else {
                    Ok(Action::InstanceSet { target: instance.into(), field: field.into(), value })
                };
            }
            let normalized = normalize_field(field);
            if normalized == "AGENTX_RELAY_ENABLED" {
                return Err("relay_enabled is not remotely settable; use relay off".into());
            }
            crate::config::user_field(&normalized).ok_or_else(|| format!("Unknown config key '{field}'"))?;
            H::ConfigSet { field: field.into(), value }
        }
        rpc_action::RELAY_OFF => H::RelayOff,
        rpc_action::TERM_SCREEN => H::TermScreen { target: target()?, json: flag(params, "json"), clean: flag(params, "clean") },
        rpc_action::TERM_INJECT => H::TermInject { target: target()?, text: optional(params, "text").unwrap_or("").into(), enter: flag(params, "enter") },
        rpc_action::TRANSCRIPT => H::Transcript(RemoteTranscript {
            target: target()?,
            last: count(params, "last", 10),
            range: optional(params, "range").map(str::to_owned),
            json: flag(params, "json"),
            full: flag(params, "full"),
            detailed: flag(params, "detailed"),
            display_target: optional(params, "display_target").map(str::to_owned),
            origin_device: optional(params, "origin_device").map(str::to_owned),
        }),
        rpc_action::EVENTS => {
            let last = count(params, "last", 20);
            return Ok(Action::Events {
                filters: filters(params)?,
                predicate: optional(params, "sql").map(str::to_owned),
                last: if last == 0 { 20 } else { last.min(REMOTE_EVENTS_HARD_CAP) },
                max_bytes: params.get("max_bytes").and_then(Value::as_u64).and_then(|value| usize::try_from(value).ok()),
            });
        }
        rpc_action::SUB_CREATE => {
            return Ok(Action::Subscribe {
                caller: required(params, "caller")?.into(),
                external: flag(params, "caller_is_external"),
                filters: filters(params)?,
                predicates: strings(params, "sql_parts"),
                once: flag(params, "once"),
                on_hit: optional(params, "on_hit").map(str::to_owned),
            });
        }
        rpc_action::SUB_LIST => return Ok(Action::Subscriptions),
        rpc_action::SUB_UNSUB => {
            let id = required(params, "id")?;
            return Ok(Action::Unsubscribe { id: if id.starts_with("sub-") { id.into() } else { format!("sub-{id}") } });
        }
        _ => return Err(format!("unsupported remote action '{action}'")),
    };
    Ok(Action::Host(host))
}
