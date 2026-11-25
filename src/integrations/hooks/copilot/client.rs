//! Actual Copilot hook/extension host. Native operations own frozen delivery
//! policy; this adapter owns payload parsing, SDK reports, endpoints and stdout.
use crate::agents::{
    attachments::AttachmentStore,
    copilot_service as native,
    service::{ObserveHookPosition, Request, ResolvedActor, Response},
};
use crate::integrations::hooks::{HookPayload, client::HookClient, output};
use crate::runtime::{
    context::AgentxContext,
    hosts::{HostService, Request as HostRequest},
};
use crate::service::{CallerContext, Request as ServiceRequest, RequestId, Response as ServiceResponse, ServiceError};
use crate::shared::{ST_ACTIVE, ST_LISTENING};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::PathBuf;

/// A lost native response is reconciled before any Provider output/SDK input.
/// Keep the original caller ID and Client generation through that retry.
fn call(hook: &HookClient, request: native::Request) -> Result<native::Response> {
    let client = &hook.client;
    let caller = CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: client.invocation().clone(), deadline_unix_ms: None, expected_database_generation: None };
    let request = ServiceRequest::Participants(Request::Copilot(request));
    let invoke = || client.call_with_context(caller.clone(), request.clone());
    let response = match invoke() {
        Err(ServiceError::OutcomeUnknown(_)) => invoke(),
        other => other,
    }?;
    match response {
        ServiceResponse::Participants(Response::Copilot(response)) => Ok(response),
        _ => bail!("unexpected Copilot native response"),
    }
}

fn resolve(hook: &mut HookClient) -> Result<ResolvedActor> {
    let actor = hook.resolve("copilot")?;
    let store = crate::agents::service::RemoteAttachments::new(hook.client.clone());
    if let Some(attachment) = store.current_for_participant(&actor.participant)? {
        let mut invocation = hook.client.invocation().clone();
        invocation.activation_target = Some(attachment.target);
        invocation.provider_session = Some(attachment.session);
        hook.client = hook.client.with_invocation(invocation);
    }
    Ok(actor)
}

fn directory(hook: &HookClient, payload: &HookPayload) -> PathBuf {
    payload.raw.get("cwd").and_then(Value::as_str).filter(|cwd| !cwd.is_empty()).map(PathBuf::from).unwrap_or_else(|| hook.context.cwd.clone())
}

fn position(hook: &HookClient, actor: ResolvedActor, payload: &HookPayload) -> Result<ResolvedActor> {
    if super::from_subagent_session(payload, &actor.record) {
        return Ok(actor);
    }
    match hook.participant(Request::ObserveHookPosition(ObserveHookPosition { participant: actor.participant, session_id: payload.session_id.clone(), transcript_path: payload.transcript_path.clone(), directory: Some(directory(hook, payload)) }))? {
        Response::Registered(actor) => Ok(actor),
        _ => bail!("unexpected Copilot hook position response"),
    }
}

fn pending(hook: &HookClient, actor: &ResolvedActor, payload: &HookPayload, kind: native::PendingHook) -> Result<Option<output::Prepared>> {
    match call(hook, native::Request::Pending { participant: actor.participant.clone(), session_id: payload.session_id.clone(), hook: kind })? {
        native::Response::Pending(prepared) => Ok(output::from_prepared(&actor.participant, prepared)),
        _ => bail!("unexpected Copilot pending-hook response"),
    }
}

pub(super) fn route(hook: &mut HookClient, event: &str, payload: &HookPayload) -> Result<()> {
    // Permission parsing and neutral SubagentStop require no participant I/O.
    if event == "copilot-permissionrequest" {
        return write_json(&super::permission_output(payload));
    }
    if event == "copilot-subagentstop" {
        return write_json(&json!({"decision":"allow"}));
    }
    let mut actor = resolve(hook)?;
    if super::from_subagent_session(payload, &actor.record) {
        return write_json(&if event == "copilot-agentstop" { json!({"decision":"allow"}) } else { json!({}) });
    }
    if event == "copilot-sessionstart" {
        let cwd = directory(hook, payload);
        let mut invocation = hook.client.invocation().clone();
        invocation.launch_context = Some(Value::Object(crate::agents::binding::capture_context_snapshot(&hook.context.agentx_dir, &cwd, &hook.context.raw_env)));
        hook.client = hook.client.with_invocation(invocation);
        actor = hook.session("copilot", payload, Some(cwd), false)?;
    } else if event != "copilot-sessionend" {
        actor = position(hook, actor, payload)?;
    }
    let mut mailbox = None;
    let mut bootstrap = None;
    let mut output = json!({});
    match event {
        "copilot-sessionstart" => {
            // Copilot emits this lazily inside its first already-active turn.
            if actor.record.status != ST_ACTIVE {
                hook.status(&actor, ST_LISTENING, "start", "")?;
            }
            crate::runtime::env::set_terminal_title(&actor.participant.canonical_name);
            crate::relay::worker::try_spawn_worker();
            hook.wake(&actor)?;
            bootstrap = hook.bootstrap(&actor, false)?;
        }
        "copilot-userpromptsubmit" => {
            let prompt = payload.raw.get("prompt").or_else(|| payload.raw.get("initial_prompt")).and_then(Value::as_str).unwrap_or("");
            call(hook, native::Request::ObservePrompt { participant: actor.participant.clone(), session_id: payload.session_id.clone(), prompt: prompt.to_owned() })?;
            output = json!({"additionalContext":hook.runtime_context(&actor)?});
        }
        "copilot-pretooluse" => hook.tool_status(&actor, payload)?,
        "copilot-posttooluse" | "copilot-posttoolusefailure" => {
            mailbox = pending(hook, &actor, payload, native::PendingHook::ToolResult)?;
            if let Some(prepared) = &mailbox {
                output = json!({"additionalContext":prepared.formatted});
            }
        }
        "copilot-agentstop" => {
            mailbox = pending(hook, &actor, payload, native::PendingHook::Stop)?;
            output = match &mailbox {
                Some(prepared) => json!({"decision":"block","reason":prepared.formatted}),
                None => json!({"decision":"allow"}),
            };
        }
        "copilot-notification" => {
            if payload.notification_type.as_deref() == Some("permission_prompt") {
                hook.status(&actor, "blocked", "approval", "")?;
            }
        }
        "copilot-erroroccurred" => {
            let context = payload.raw.get("error_context").or_else(|| payload.raw.get("errorContext")).and_then(Value::as_str).unwrap_or("unknown");
            let recoverable = payload.raw.get("recoverable").and_then(Value::as_bool);
            let error_name = payload.raw.pointer("/error/name").and_then(Value::as_str).unwrap_or("unknown");
            crate::log::log_warn("hooks", "copilot.error_occurred", &format!("instance={} context={context} error={error_name} recoverable={recoverable:?}", actor.record.name));
        }
        "copilot-subagentstart" => bootstrap = hook.bootstrap(&actor, false)?,
        "copilot-sessionend" => {
            let reason = payload.raw.get("reason").or_else(|| payload.raw.get("stop_reason")).and_then(Value::as_str).unwrap_or("unknown");
            hook.stop(&actor, payload.session_id.as_deref(), reason, false, false)?;
        }
        _ => {}
    }
    if let Some((text, _)) = &bootstrap {
        output = json!({"additionalContext":text});
    }
    hook.flush_json(&actor, &output, mailbox.as_ref(), bootstrap.as_ref().map(|(_, receipt)| receipt.as_str()))
}

fn write_json(value: &Value) -> Result<()> {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    stdout.flush()?;
    Ok(())
}

pub(super) fn dispatch(event: &str) -> i32 {
    let context = AgentxContext::from_os();
    if !crate::integrations::hooks::common::hook_gate_check(&context) {
        return 0;
    }
    let raw = match serde_json::from_reader(std::io::stdin().lock()) {
        Ok(raw) => raw,
        Err(error) => {
            crate::log::log_warn("hooks", "copilot.parse_error", &format!("hook={event} err={error}"));
            return 0;
        }
    };
    let payload = HookPayload::from_copilot_native(super::hook_type_for_command(event), raw);
    let result = HookClient::connect(&context, false).and_then(|mut hook| route(&mut hook, event, &payload));
    if let Err(error) = result {
        crate::log::log_warn("hooks", "copilot.native_error", &format!("hook={event} err={error:#}"));
    }
    0
}

fn extension(hook: &mut HookClient, event: &str, argv: &[String]) -> Result<Value> {
    let actor = resolve(hook)?;
    let session_id = super::flag(argv, "--session-id").filter(|session| !session.is_empty()).context("Copilot extension session required")?;
    match event {
        "copilot-ext-poll" => {
            if actor.record.session_id.as_deref().is_some_and(|bound| bound != session_id) {
                return Ok(json!({"name":actor.participant.canonical_name,"skipped":"session_mismatch"}));
            }
            // Admit the listener before freezing an Issuing output. A bad port
            // or rejected stale listener cannot strand a batch never exposed.
            if let Some(port) = super::flag(argv, "--notify-port") {
                let port = port.parse::<u16>().context("invalid Copilot notify port")?;
                if port != 0 {
                    match HostService::call(&hook.client, HostRequest::SetCopilotEndpoint { participant: actor.participant.clone(), session_id: session_id.to_owned(), port }) {
                        Ok(_) => {}
                        Err(ServiceError::StaleFence) => return Ok(json!({"name":actor.participant.canonical_name,"skipped":"session_mismatch"})),
                        Err(error) => return Err(error.into()),
                    }
                }
            }
            let result = match call(hook, native::Request::Poll { participant: actor.participant.clone(), session_id: session_id.to_owned(), idle: argv.iter().any(|arg| arg == "--idle") })? {
                native::Response::Poll(result) => result,
                _ => bail!("unexpected Copilot poll response"),
            };
            let name = &result.participant.canonical_name;
            if result.session_mismatch {
                return Ok(json!({"name":name,"skipped":"session_mismatch"}));
            }
            if let Some(offer) = result.offer {
                return Ok(json!({"name":name,"count":offer.count,"event_id":offer.receipt.through.0,"text":offer.text,"receipt":offer.receipt}));
            }
            Ok(match result.inflight {
                Some(through) => json!({"name":name,"inflight":through.0}),
                None => json!({"name":name}),
            })
        }
        "copilot-ext-sent" => {
            let receipt: native::OfferToken = serde_json::from_str(super::flag(argv, "--receipt").context("exact Copilot offer receipt required")?)?;
            if receipt.participant != actor.participant || receipt.session_id != session_id {
                bail!("stale Copilot offer receipt");
            }
            if let Some(event_id) = super::flag(argv, "--event-id")
                && event_id.parse::<i64>().ok() != Some(receipt.through.0)
            {
                bail!("Copilot receipt frontier mismatch");
            }
            let observation = if argv.iter().any(|arg| arg == "--unknown") {
                native::SendObservation::Unknown
            } else if argv.iter().any(|arg| arg == "--failed") {
                native::SendObservation::Failed
            } else {
                native::SendObservation::Accepted
            };
            call(hook, native::Request::Sent { receipt, observation })?;
            Ok(json!({"name":actor.participant.canonical_name}))
        }
        _ => bail!("unknown Copilot extension command: {event}"),
    }
}

pub(super) fn dispatch_ext(event: &str, argv: &[String]) -> (i32, String) {
    let context = AgentxContext::from_os();
    if !crate::integrations::hooks::common::hook_gate_check(&context) {
        return (0, "{}".into());
    }
    let argv = match argv.split_first() {
        Some((first, rest)) if first == event => rest,
        _ => argv,
    };
    let result = HookClient::connect(&context, false).and_then(|mut hook| extension(&mut hook, event, argv));
    let output = match result {
        Ok(output) => output,
        Err(error) => json!({"error":format!("{error:#}")}),
    };
    (0, output.to_string())
}
