//! Codex hook host adapter. Qualified shared roots keep their native attachment;
//! ordinary hooks use their explicit process binding. Only flushed output
//! confirms mailbox/bootstrap receipts, and Provider discovery stays local.
use crate::agents::service::{ObserveHookPosition, Request, ResolvedActor, Response, ToolActivity};
use crate::integrations::hooks::{HookPayload, client::HookClient, tool_status};
use crate::observability::trace::{self, Phase};
use crate::runtime::context::AgentxContext;
use crate::shared::{ST_ACTIVE, ST_LISTENING};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::path::PathBuf;
use crate::service::codex_hook::{Activity, HookEvent, HookOutcome};

fn directory(hook: &HookClient, payload: &HookPayload) -> PathBuf {
    payload.raw.get("cwd").and_then(Value::as_str).filter(|cwd| !cwd.is_empty()).map(PathBuf::from).unwrap_or_else(|| hook.context.cwd.clone())
}

/// Resolve before updating anything: internal work cannot consume the root's
/// one-shot terminal handoff or alter its process/session identity.
fn resolve(hook: &mut HookClient, event: &str, payload: &HookPayload) -> Result<Option<ResolvedActor>> {
    let shared = crate::integrations::codex::roots::is_shared_context(&hook.context);
    if payload.raw.get("agent_id").and_then(Value::as_str).is_some_and(|id| !id.is_empty()) {
        return Ok(None);
    }
    if shared {
        let store = crate::agents::service::RemoteAttachments::new(hook.client.clone());
        let Some(attachment) = crate::integrations::codex::roots::resolve_shared_attachment(&store, &hook.context, payload.session_id.as_deref().unwrap_or(""))? else {
            return Ok(None);
        };
        let mut invocation = hook.client.invocation().clone();
        invocation.activation_target = Some(attachment.target);
        invocation.provider_session = Some(attachment.session);
        hook.client = hook.client.with_invocation(invocation);
    }
    let actor = hook.resolve("codex")?;
    if event == "codex-sessionstart" && payload.raw.get("source").and_then(Value::as_str) == Some("compact") {
        let Some(session) = payload.session_id.as_deref().filter(|session| !session.is_empty()) else {
            return Ok(None);
        };
        if actor.record.session_id.as_deref() != Some(session) {
            return Ok(None);
        }
    }
    if !shared && super::is_internal_thread(&actor.record, payload) {
        return Ok(None);
    }
    let directory = directory(hook, payload);
    let transcript = payload
        .transcript_path
        .as_deref()
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            trace::time(Phase::TranscriptLookup, || {
                let session = payload.session_id.as_deref()?;
                // Reuse a still-existing path only for the exact bound session.
                if actor.record.session_id.as_deref() == Some(session) && std::path::Path::new(&actor.record.transcript_path).is_file() {
                    return Some(actor.record.transcript_path.clone());
                }
                super::derive_codex_transcript_path_for_context(session, &hook.context)
            })
        })
        .map(|path| super::normalize_codex_transcript_path(&path));
    if event == "codex-sessionstart" && !shared && payload.raw.get("source").and_then(Value::as_str) != Some("compact") {
        let Some(session) = payload.session_id.as_deref().filter(|session| !session.is_empty()) else {
            return Ok(None);
        };
        let mut invocation = hook.client.invocation().clone();
        invocation.launch_context = Some(Value::Object(trace::time(Phase::LaunchContext, || crate::agents::binding::capture_context_snapshot(&hook.context.agentx_dir, &directory, &hook.context.raw_env))));
        hook.client = hook.client.with_invocation(invocation);
        return match hook.participant(Request::HookSession(crate::agents::service::HookSession { tool: "codex".into(), session_id: session.to_owned(), transcript_path: transcript, directory: Some(directory), create_if_unbound: false }))? {
            Response::Registered(actor) => Ok(Some(actor)),
            _ => bail!("unexpected Codex session response"),
        };
    }
    match hook.participant(Request::ObserveHookPosition(ObserveHookPosition { participant: actor.participant, session_id: payload.session_id.clone().filter(|session| !session.is_empty()), transcript_path: transcript, directory: Some(directory) }))? {
        Response::Registered(actor) => Ok(Some(actor)),
        _ => bail!("unexpected Codex hook position response"),
    }
}

fn context_output(event: &str, text: &str, visible: bool) -> Value {
    let mut output = json!({"hookSpecificOutput":{"hookEventName":super::codex_event_name(event),"additionalContext":text}});
    if visible {
        output["systemMessage"] = json!(text);
    }
    output
}

fn observe_recovery_turn(hook: &HookClient, actor: &ResolvedActor, payload: &HookPayload, stopped: bool) {
    if hook.context.process_id.is_none() || actor.record.model_request.as_ref().is_none_or(|state| state.error.is_none()) {
        return;
    }
    let Some(session) = payload.session_id.as_ref() else { return };
    let turn = payload.raw.get("turn_id").and_then(Value::as_str).filter(|turn| !turn.is_empty()).map(str::to_owned);
    let request = crate::agents::recovery::Request::ObserveTurn { participant: actor.participant.clone(), session: session.clone(), turn, stopped };
    if let Err(error) = hook.participant(Request::Recovery(request)) {
        crate::log::log_warn("hooks", "codex.recovery_observation", &error.to_string());
    }
}

pub(super) fn route(hook: &mut HookClient, event: &str, payload: &HookPayload) -> Result<()> {
    let Some(actor) = resolve(hook, event, payload)? else {
        if event == "codex-stop" {
            use std::io::Write;
            let mut stdout = std::io::stdout().lock();
            serde_json::to_writer(&mut stdout, &json!({}))?;
            stdout.flush()?;
        }
        return Ok(());
    };
    let shared = crate::integrations::codex::roots::is_shared_context(&hook.context);
    let mut mailbox = None;
    let mut bootstrap = None;
    let mut output = None;
    match event {
        "codex-sessionstart" => {
            let compact = payload.raw.get("source").and_then(Value::as_str) == Some("compact");
            if !compact {
                bootstrap = trace::time(Phase::Bootstrap, || hook.initial_context(&actor))?;
                if let Some((text, _)) = &bootstrap {
                    output = Some(context_output(event, text, false));
                }
            }
            if compact {
                bootstrap = trace::time(Phase::Bootstrap, || hook.context_for(&actor, true, crate::runtime::agent_context::ContextReason::Compact))?;
                if let Some((text, _)) = &bootstrap {
                    output = Some(context_output(event, text, false));
                }
            }
            if !shared && !compact {
                hook.status(&actor, ST_LISTENING, "start", "")?;
                crate::runtime::env::set_terminal_title(&actor.participant.canonical_name);
                crate::relay::worker::try_spawn_worker();
            }
            if !compact {
                hook.wake(&actor)?;
            }
        }
        "codex-userpromptsubmit" => {
            if !shared || is_direct_terminal(&hook.context) {
                observe_recovery_turn(hook, &actor, payload, false);
            }
            let triggered = payload.raw.get("prompt").and_then(Value::as_str).is_some_and(|prompt| prompt.trim() == super::AGENTX_TRIGGER);
            if shared {
                hook.status(&actor, ST_ACTIVE, "prompt", "")?;
                bootstrap = trace::time(Phase::Bootstrap, || hook.bootstrap(&actor, false))?;
                if let Some((text, _)) = &bootstrap {
                    output = Some(context_output(event, text, false));
                }
            } else if triggered {
                mailbox = hook.pending(&actor)?;
                if let Some(pending) = &mailbox {
                    output = Some(context_output(event, &pending.formatted, true));
                } else {
                    hook.status(&actor, ST_ACTIVE, "prompt", "")?;
                }
            } else {
                hook.status(&actor, ST_ACTIVE, "prompt", "")?;
            }
        }
        "codex-premodelrequest" | "codex-postmodelrequest" => {
            use crate::agents::model_request::{ObserveModelRequest, RequestPhase};
            let phase = if event == "codex-premodelrequest" {
                RequestPhase::Started
            } else {
                match payload.raw.get("status").and_then(Value::as_str) {
                    Some("completed") => RequestPhase::Completed,
                    Some("failed") => RequestPhase::Failed,
                    Some("cancelled") => RequestPhase::Cancelled,
                    _ => bail!("unrecognized model request status"),
                }
            };
            let turn_id = payload.raw.get("turn_id").and_then(Value::as_str).filter(|value| !value.is_empty()).ok_or_else(|| anyhow::anyhow!("model request turn_id required"))?;
            let session_id = payload.session_id.clone().filter(|value| !value.is_empty()).ok_or_else(|| anyhow::anyhow!("model request session_id required"))?;
            let error = payload.raw.get("error").and_then(Value::as_str).map(|value| value.chars().take(4096).collect());
            hook.participant(Request::ObserveModelRequest(ObserveModelRequest {
                participant: actor.participant.clone(),
                session_id,
                turn_id: turn_id.to_owned(),
                phase,
                model: payload.raw.get("model").and_then(Value::as_str).unwrap_or("").to_owned(),
                error,
            }))?;
        }
        "codex-pretooluse" => {
            let written_files = if payload.tool_name == "apply_patch" { tool_status::patch_files(&payload.tool_input).into_iter().map(str::to_owned).collect() } else { Vec::new() };
            hook.participant(Request::ToolActivity(ToolActivity {
                participant: actor.participant.clone(),
                tool_name: payload.tool_name.clone(),
                detail: tool_status::extract_tool_detail("codex", &payload.tool_name, &payload.tool_input),
                written_files,
            }))?;
        }
        "codex-posttooluse" => {
            if super::is_approval_block(&actor.record) {
                hook.status(&actor, ST_ACTIVE, &format!("approved:{}", payload.tool_name), "")?;
            }
            if !shared {
                mailbox = hook.pending(&actor)?;
                if let Some(pending) = &mailbox {
                    output = Some(context_output(event, &pending.formatted, true));
                }
            }
        }
        "codex-stop" | "codex-interrupt" => {
            if !shared || is_direct_terminal(&hook.context) {
                observe_recovery_turn(hook, &actor, payload, event == "codex-stop");
            }
            hook.status(&actor, ST_LISTENING, if event == "codex-interrupt" { "interrupted" } else { "" }, "")?;
            hook.wake(&actor)?;
            if event == "codex-stop" {
                output = Some(json!({}));
            }
        }
        _ => {}
    }
    if let Some(output) = output {
        trace::time(Phase::Output, || hook.flush_json(&actor, &output, mailbox.as_ref(), bootstrap.as_ref().map(|(_, receipt)| receipt.as_str()))).inspect_err(|_| trace::fail(Phase::Output))?;
    }
    Ok(())
}

fn is_direct_terminal(context: &AgentxContext) -> bool {
    context.raw_env.get("AGENTX_CODEX_DIRECT_TERMINAL").is_some_and(|mode| mode == "1")
}

pub(super) fn dispatch(event: &str) -> i32 {
    let context = trace::time(Phase::HookContext, AgentxContext::from_os);
    if !crate::integrations::hooks::common::hook_gate_check(&context) {
        return 0;
    }
    trace::claim_scope(&context.agentx_dir);
    let raw = match trace::time(Phase::ReadInput, || serde_json::from_reader(std::io::stdin().lock())) {
        Ok(raw) => raw,
        Err(_) => {
            trace::fail(Phase::ReadInput);
            return 0;
        }
    };
    let payload = HookPayload::from_codex_native(super::codex_event_name(event), raw);
    let result = crate::integrations::hooks::common::dispatch_with_panic_guard("codex", event, Err(anyhow::anyhow!("Codex hook panicked")), || {
        let mut hook = trace::time(Phase::DaemonAcquire, || {
            if crate::integrations::codex::roots::is_shared_context(&context) && is_direct_terminal(&context) {
                let invocation = crate::runtime::host_client::invocation(&context);
                let client = crate::service::client::Client::connect_existing(&context.agentx_dir, invocation)?;
                Ok(HookClient { client, context: context.clone() })
            } else {
                HookClient::connect(&context, false)
            }
        }).inspect_err(|_| trace::fail(Phase::DaemonAcquire))?;
        trace::time(Phase::Route, || route(&mut hook, event, &payload))
    });
    if result.is_err() {
        trace::fail(Phase::Route);
    }
    0
}

pub(super) fn managed_event(event: &str) -> bool {
    matches!(event, "codex-pretooluse" | "codex-posttooluse" | "codex-premodelrequest" | "codex-postmodelrequest" | "codex-userpromptsubmit" | "codex-stop" | "codex-interrupt")
}

/// Serde skips unknown payload fields without building the general JSON tree.
/// Borrow tool input without decoding it unless this is a PreToolUse callback.
#[derive(serde::Deserialize)]
struct ManagedInput<'a> {
    session_id: Option<String>,
    transcript_path: Option<String>,
    session_path: Option<String>,
    cwd: Option<PathBuf>,
    agent_id: Option<String>,
    tool_name: Option<String>,
    #[serde(borrow)]
    tool_input: Option<&'a serde_json::value::RawValue>,
    prompt: Option<String>,
    turn_id: Option<String>,
    status: Option<String>,
    model: Option<String>,
    error: Option<String>,
}

fn decode_managed(event: &str, bytes: &[u8], cwd: &std::path::Path) -> Result<Option<HookEvent>> {
    use crate::agents::model_request::RequestPhase;
    let ManagedInput { session_id, transcript_path, session_path, cwd: directory, agent_id, tool_name, tool_input, prompt, turn_id, status, model, error } = serde_json::from_slice(bytes)?;
    if agent_id.is_some_and(|id| !id.is_empty()) {
        return Ok(None);
    }
    let session_id = session_id.filter(|id| !id.is_empty());
    let transcript_path = transcript_path.or(session_path).filter(|path| !path.is_empty()).map(|path| super::normalize_codex_transcript_path(&path));
    let directory = directory.filter(|path| !path.as_os_str().is_empty()).unwrap_or_else(|| cwd.into());
    let turn_id = turn_id.filter(|id| !id.is_empty());
    let activity = match event {
        "codex-pretooluse" => {
            let tool_name = tool_name.filter(|name| !name.is_empty()).ok_or_else(|| anyhow::anyhow!("tool name required"))?;
            let input = tool_input.map(|raw| serde_json::from_str(raw.get())).transpose()?.unwrap_or_default();
            let written_files = if tool_name == "apply_patch" { tool_status::patch_files(&input).into_iter().map(str::to_owned).collect() } else { Vec::new() };
            let detail = if let Some(first) = written_files.first() { first.clone() } else { tool_status::extract_tool_detail("codex", &tool_name, &input) };
            Activity::PreTool { tool_name, detail, written_files }
        }
        "codex-posttooluse" => Activity::PostTool { tool_name: tool_name.unwrap_or_default() },
        "codex-premodelrequest" | "codex-postmodelrequest" => {
            let phase = if event == "codex-premodelrequest" { RequestPhase::Started } else {
                match status.as_deref() {
                    Some("completed") => RequestPhase::Completed,
                    Some("failed") => RequestPhase::Failed,
                    Some("cancelled") => RequestPhase::Cancelled,
                    _ => bail!("unrecognized model request status"),
                }
            };
            if session_id.is_none() {
                bail!("model request session required");
            }
            let turn_id = turn_id.ok_or_else(|| anyhow::anyhow!("model request turn required"))?;
            Activity::Model { phase, turn_id, model: model.unwrap_or_default(), error: error.map(|value| value.chars().take(4096).collect()) }
        }
        "codex-userpromptsubmit" => Activity::Prompt { triggered: prompt.is_some_and(|prompt| prompt.trim() == super::AGENTX_TRIGGER), turn_id },
        "codex-stop" | "codex-interrupt" => Activity::Stop { interrupted: event == "codex-interrupt", turn_id },
        _ => return Ok(None),
    };
    Ok(Some(HookEvent { session_id, transcript_path, directory, activity }))
}

pub(super) fn dispatch_managed(event: &str) {
    use crate::integrations::hooks::{common, output};
    use crate::service::client::Client;
    use std::io::{Read, Write};

    let context = trace::time(Phase::HookContext, || {
        crate::integrations::nested_env::scrub_inherited_identity(crate::integrations::tool::Tool::Codex);
        AgentxContext::from_os()
    });
    if !context.is_launched || !context.agentx_dir_override || !common::hook_gate_check(&context) {
        return;
    }
    trace::claim_scope(&context.agentx_dir);
    let result = common::dispatch_with_panic_guard("codex", event, Err(anyhow::anyhow!("managed Codex hook panicked")), || {
        let input = trace::time(Phase::ReadInput, || -> Result<_> {
            // Retain bytes only to keep the legacy route available if the peer
            // does not advertise the operation. Do not deserialize tool_response.
            const MAX_INPUT: u64 = 64 * 1024 * 1024;
            let mut bytes = Vec::new();
            std::io::stdin().lock().take(MAX_INPUT + 1).read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_INPUT { bail!("Codex hook input too large"); }
            Ok((decode_managed(event, &bytes, &context.cwd)?, bytes))
        }).inspect_err(|_| trace::fail(Phase::ReadInput))?;
        let (input, bytes) = input;
        let mut prepared = None;
        if let Some(input) = input {
            let invocation = crate::runtime::host_client::invocation(&context);
            match trace::time(Phase::Route, || Client::call_codex_hook(&context.agentx_dir, invocation, input))? {
                Some((client, HookOutcome::Output(output))) => {
                    prepared = Some((client, output));
                }
                Some(_) => {}
                None => {
                    let payload = HookPayload::from_codex_native(super::codex_event_name(event), serde_json::from_slice(&bytes)?);
                    let mut hook = HookClient { client: Client::connect_existing(&context.agentx_dir, crate::runtime::host_client::invocation(&context))?, context };
                    return route(&mut hook, event, &payload);
                }
            }
        }
        if let Some((client, output)) = prepared {
            trace::time(Phase::Output, || {
                let mut stdout = std::io::stdout().lock();
                serde_json::to_writer(&mut stdout, &context_output(event, &output.formatted_context, true))?;
                stdout.flush()?;
                output::confirm_hook_output(&client, output.confirmation)
            }).inspect_err(|_| trace::fail(Phase::Output))?;
        } else if event == "codex-stop" {
            let mut stdout = std::io::stdout().lock();
            serde_json::to_writer(&mut stdout, &json!({}))?;
            stdout.flush()?;
        }
        Ok(())
    });
    if result.is_err() {
        trace::fail(Phase::Route);
    }
}
