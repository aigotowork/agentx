//! Relay worker host: the dispatcher has returned before any operation here.
//! Every domain call keeps the receive client's generation and caller context.
use super::control_service::*;
use crate::service::{Request as AppRequest, Response as AppResponse, client::Client};
use serde_json::{Value, json};

pub(super) struct Job {
    pub storage: Client,
    pub effect: ControlEffect,
}
pub(super) struct Notice {
    pub changed: bool,
    pub disabled: bool,
}

pub(super) fn run(job: Job) -> Notice {
    let Job { storage, effect } = job;
    let granted = super::persistence::call(&storage, super::service::Request::BeginControl { effect_id: effect.effect_id.clone(), token: effect.token.clone() });
    if !matches!(granted, Ok(super::service::Response::ControlGranted(true))) {
        if let Err(error) = granted {
            crate::log::log_warn("relay", "relay.control_grant_unknown", &error.to_string());
        }
        return Notice { changed: false, disabled: false };
    }
    let disabled = matches!(&effect.action, HostAction::RelayOff);
    let observed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(&storage, &effect)));
    let mut reply = match observed {
        Ok(Ok(result)) => ControlReply { ok: result.get("ok").and_then(Value::as_bool).unwrap_or(true), result },
        Ok(Err(error)) => ControlReply { ok: false, result: json!({"error": error, "outcome": "unknown_remote_effect"}) },
        Err(_) => ControlReply { ok: false, result: json!({"error": "remote action host panicked; its effect may already exist", "outcome": "unknown_remote_effect"}) },
    };
    if serde_json::to_vec(&reply).map_or(true, |bytes| bytes.len() > 2 * 1024 * 1024) {
        reply = ControlReply { ok: false, result: json!({"error": "remote action completed; its answer exceeds the completion budget", "completed": true, "outcome": "completed_result_unavailable", "observed_ok": reply.ok}) };
    }
    let succeeded = reply.ok;
    match super::persistence::call(&storage, super::service::Request::CompleteControl { effect_id: effect.effect_id, token: effect.token, reply }) {
        Ok(super::service::Response::Changed(changed)) => Notice { changed, disabled: disabled && succeeded },
        Ok(_) => Notice { changed: false, disabled: false },
        Err(error) => {
            crate::log::log_warn("relay", "relay.control_completion_unknown", &error.to_string());
            Notice { changed: false, disabled: false }
        }
    }
}

/// Queue admission failed before any host operation. Claim exactly once, then
/// report an explicit rejection; a concurrent Issuing grant is never replaced.
pub(super) fn reject_unstarted(job: Job) {
    if matches!(super::persistence::call(&job.storage, super::service::Request::BeginControl { effect_id: job.effect.effect_id.clone(), token: job.effect.token.clone() }), Ok(super::service::Response::ControlGranted(true))) {
        let _ = super::persistence::call(
            &job.storage,
            super::service::Request::CompleteControl { effect_id: job.effect.effect_id, token: job.effect.token, reply: ControlReply::error("relay worker effect admission is unavailable; no host action was started") },
        );
    }
}

pub(super) fn resolve_remote_cwd(requested: Option<&str>) -> Result<String, String> {
    let requested = requested.filter(|value| !value.is_empty()).ok_or("remote launch requires a working directory (--dir) but none was provided")?;
    if std::path::Path::new(requested).is_dir() { Ok(requested.into()) } else { Err(format!("requested cwd does not exist or is not a directory: {requested}")) }
}

fn prepare_launch(storage: &Client, request: &RemoteLaunch) -> Result<crate::runtime::launch_profile::ResolvedLaunchConfiguration, String> {
    prepare_launch_at(storage.scope_root(), &storage.invocation().environment, request)
}

fn prepare_launch_at(scope: &std::path::Path, inherited: &std::collections::HashMap<String, String>, request: &RemoteLaunch) -> Result<crate::runtime::launch_profile::ResolvedLaunchConfiguration, String> {
    let mut environment = inherited.clone();
    environment.remove("AGENTX_PROFILE");
    let resolved = crate::runtime::launch_profile::prepare(scope, &environment, request.tool.as_deref(), &request.selection, &request.args).map_err(|error| format!("{error:#}"))?;
    let tool = crate::runtime::launcher::LaunchTool::from_str(&resolved.tool).map_err(|error| error.to_string())?;
    if request.count == 0 || request.count > tool.spec().launch.max_launch_count {
        return Err("Launch count is outside this tool's supported range".into());
    }
    Ok(resolved)
}

fn execute(storage: &Client, effect: &ControlEffect) -> Result<Value, String> {
    use crate::runtime::launcher::{self, LaunchParams, LaunchTool};
    let config = crate::config::AgentxConfig::load_for_scope(storage.scope_root(), &storage.invocation().environment);
    match &effect.action {
        HostAction::Launch(request) => {
            let resolved = prepare_launch(storage, request)?;
            let tool = LaunchTool::from_str(&resolved.tool).map_err(|error| error.to_string())?;
            let (args, background) = launcher::prepare_resolved_execution(&tool, &resolved.args, request.background);
            let cwd = resolve_remote_cwd(request.cwd.as_deref())?;
            let mut environment = storage.invocation().environment.clone();
            environment.insert("AGENTX_DIR".into(), storage.scope_root().to_string_lossy().into_owned());
            let context = crate::runtime::context::AgentxContext::from_env(&environment, std::path::PathBuf::from(&cwd)).with_tty(false, false);
            // Relay has no cross-device exact actor authority. Never reuse the
            // long-lived worker's ambient participant as the remote initiator.
            let mut invocation = storage.invocation().clone();
            invocation.explicit_name = None;
            invocation.process_binding_id = None;
            invocation.provider_session = None;
            invocation.activation_target = None;
            invocation.actor_capability = None;
            invocation.environment.remove("AGENTX_CLAUDE_ACTOR");
            invocation.environment.insert("AGENTX_LAUNCH_ORIGIN_UNAVAILABLE".into(), "1".into());
            let launch_storage = storage.with_invocation(invocation);
            let result = launcher::launch_with_client(
                LaunchParams {
                    labels: Default::default(),
                    bindings: Vec::new(),
                    tool: resolved.tool,
                    count: request.count,
                    args,
                    persisted_args: Some(resolved.args),
                    launch_profile: resolved.profile,
                    collaboration: None,
                    title: None,
                    prior_session_id: None,
                    resume_cursor: None,
                    system_prompt: request.system_prompt.clone(),
                    initial_prompt: request.initial_prompt.clone(),
                    background,
                    stdio: false,
                    cwd: Some(cwd),
                    env: None,
                    launcher: request.launcher.clone().or_else(|| Some("user".into())),
                    run_here: Some(false),
                    batch_id: None,
                    name: None,
                    skip_validation: false,
                    terminal: request.terminal.clone(),
                    append_reply_handoff: false,
                },
                &context,
                &launch_storage,
            )
            .map_err(|error| error.to_string())?;
            launch_result(storage, &result)
        }
        HostAction::PrepareLaunch(request) => {
            let resolved = prepare_launch(storage, request)?;
            let cwd = resolve_remote_cwd(request.cwd.as_deref())?;
            let flags = crate::runtime::launch_options::AgentxLaunchFlags { dir: Some(cwd), headless: request.background, terminal: request.terminal.clone(), ..Default::default() };
            Ok(crate::runtime::launch_profile::preview(&resolved, request.count, &flags))
        }
        HostAction::Kill { .. } => {
            let participant = effect.participant.as_ref().ok_or("kill effect has no admitted participant")?;
            let result = crate::runtime::kill_client::kill(storage, participant, &effect.initiated_by)?;
            Ok(json!({
                "target": result.target, "pid": result.pid, "kill_result": result.kill_result.as_str(),
                "pane_closed": result.pane_closed, "pane_retry_command": result.pane_retry_command,
                "preset_name": result.preset_name, "pane_id": result.pane_id,
                "ok": !matches!(result.kill_result, crate::runtime::terminal::KillResult::PermissionDenied),
            }))
        }
        HostAction::Resume(request) => {
            let result = crate::runtime::resume_client::resume_fenced(storage, &request.target, effect.participant.as_ref(), request.fork, &request.extra_args, request.launcher.as_deref()).map_err(|error| error.to_string())?;
            launch_result(storage, &result)
        }
        HostAction::TermScreen { target, json, clean } => {
            let participant = effect.participant.as_ref().ok_or("terminal effect has no admitted participant")?;
            let content = crate::runtime::terminal_client::screen(storage, participant, *json, *clean)?;
            Ok(json!({"target": target, "content": content}))
        }
        HostAction::TermInject { target, text, enter } => {
            let participant = effect.participant.as_ref().ok_or("terminal effect has no admitted participant")?;
            let message = crate::runtime::terminal_client::inject(storage, participant, text, *enter)?;
            Ok(json!({"target": target, "message": message}))
        }
        HostAction::ConfigGet { fields } => {
            let mut values = serde_json::Map::new();
            for field in fields {
                super::control_parse::reject_secret(field)?;
                let normalized = super::control_parse::normalize_field(field);
                let (value, source) = crate::config::file::get_for_scope(storage.scope_root(), &storage.invocation().environment, &normalized);
                values.insert(field.clone(), json!({"value": value, "source": source}));
            }
            Ok(Value::Object(values))
        }
        HostAction::ConfigSet { field, value } => {
            super::control_parse::reject_secret(field)?;
            crate::config::file::set_for_scope(storage.scope_root(), field, value)?;
            crate::runtime::config_effects::apply_for_scope(storage.scope_root(), storage.invocation(), field)?;
            Ok(json!({"field": field, "value": value}))
        }
        HostAction::InstanceTimeoutDefault { target, requested } => {
            use crate::agents::{lifecycle_service as L, service as P};
            let participant = effect.participant.as_ref().ok_or("timeout reset has no admitted participant")?;
            match storage.call(AppRequest::Participants(P::Request::Lifecycle(L::Request::SetTimeout { participant: participant.clone(), timeout: config.timeout }))).map_err(|error| error.to_string())? {
                AppResponse::Participants(P::Response::Lifecycle(L::Response::Changed)) => Ok(json!({"instance": target, "field": "timeout", "value": requested})),
                _ => Err("unexpected timeout reset response".into()),
            }
        }
        HostAction::RelayOff => {
            let cleared = super::control::disable_local_relay(&config, storage)?;
            Ok(json!({"disabled": true, "cleared_remote_state": cleared}))
        }
        HostAction::Transcript(request) => {
            let options = crate::transcript::client::RenderOptions { range: request.range.as_deref(), last_n: request.last, json_mode: request.json, full_mode: request.full, detailed: request.detailed, retry_codex: false };
            let content = if let Some(origin) = request.origin_device.as_deref() {
                crate::transcript::client::render_origin(storage, &request.target, request.display_target.as_deref().unwrap_or(&request.target), Some(origin), &options)?
            } else if let Some(participant) = effect.participant.as_ref() {
                crate::transcript::client::render_participant(storage, participant, &options)?
            } else {
                crate::transcript::client::render(storage, &request.target, &options)?
            };
            Ok(json!({"target": request.target, "content": content}))
        }
    }
}

fn launch_result(storage: &Client, result: &crate::runtime::launcher::LaunchResult) -> Result<Value, String> {
    let mut value = serde_json::to_value(result).map_err(|error| error.to_string())?;
    for name in result.handles.iter().filter_map(|handle| handle.get("instance_name").and_then(Value::as_str)) {
        let record = storage.call(AppRequest::Participants(crate::agents::service::Request::Record { name: name.into() })).map_err(|error| error.to_string())?;
        if let AppResponse::Participants(crate::agents::service::Response::Record(Some(row))) = record
            && let Some(preset) = row.terminal_preset_effective
        {
            value["terminal_preset"] = json!(preset);
            break;
        }
    }
    Ok(value)
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn remote_profile_uses_target_scope_without_reselecting_target_env() {
        let target = tempfile::tempdir().unwrap();
        std::fs::write(target.path().join("config.toml"), "[launch]\ndefault_profile='deep'\n[launch.codex]\nargs='--model legacy'\n[launch.profiles.deep]\ntool='codex'\nargs=['--model','target','--add-dir','target-dir']\n").unwrap();
        let request = RemoteLaunch::from_params(&json!({"selection":{"profile":"deep"},"args":["--model","override"],"count":2})).unwrap();
        let environment = std::collections::HashMap::from([("AGENTX_PROFILE".into(), "missing".into()), ("AGENTX_CODEX_ARGS".into(), "--model env-legacy".into())]);
        let resolved = prepare_launch_at(target.path(), &environment, &request).unwrap();
        assert_eq!(resolved.args, ["--add-dir", "target-dir", "--model", "override"]);
        assert_eq!(resolved.profile.unwrap().definition.args[1], "target");
        let empty = tempfile::tempdir().unwrap();
        assert!(prepare_launch_at(empty.path(), &environment, &request).is_err());
        let default_request = RemoteLaunch::from_params(&json!({})).unwrap();
        assert_eq!(prepare_launch_at(target.path(), &environment, &default_request).unwrap().args[1], "target");
    }
}
