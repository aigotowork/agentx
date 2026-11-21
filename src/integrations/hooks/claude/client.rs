//! Originating Claude hook host. Native participant operations own actor and
//! receipt policy; this adapter owns Provider envelopes, rendering and stdout.
#[cfg(test)]
mod acceptance_tests;

use super::render;
use crate::agents::{
    claude_service as native,
    hook_service::SessionStopOutcome,
    lifecycle_service::Bootstrap,
    service::{Request as ParticipantRequest, Response as ParticipantResponse},
};
use crate::integrations::hooks::{HookPayload, client::HookClient};
use crate::messaging::inbox::{DeliveryEvidence, DeliveryReceipt};
use crate::messaging::service::{OutputConfirmation, OutputToken, PreparedOutput};
use crate::runtime::context::AgentxContext;
use crate::service::{CallerContext, Request, RequestId, Response, ServiceError};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;

#[derive(Default)]
pub(super) struct HookOutput {
    pub value: Option<Value>,
    actor: Option<native::Actor>,
    mailbox: Option<OutputConfirmation>,
    bootstrap: Option<String>,
    history: Option<OutputToken>,
    release: Option<native::StopClaim>,
    activate_prompt: bool,
}

/// Reconcile an absent response with the same intent, caller ID and retained
/// generation before exposing any output. This never reacquires a daemon.
pub(super) fn call(hook: &HookClient, input: native::Request) -> Result<native::Response> {
    let client = &hook.client;
    let caller = CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: client.invocation().clone(), deadline_unix_ms: None, expected_database_generation: Some(client.info().database_generation.clone()) };
    let request = Request::Participants(ParticipantRequest::Claude(input));
    let invoke = || client.call_with_context(caller.clone(), request.clone());
    let response = match invoke() {
        Err(ServiceError::OutcomeUnknown(_)) => invoke(),
        other => other,
    }?;
    match response {
        Response::Participants(ParticipantResponse::Claude(response)) => Ok(response),
        _ => bail!("unexpected Claude participant response"),
    }
}

fn directory(hook: &HookClient, payload: &HookPayload) -> PathBuf {
    let path = payload.raw.get("cwd").and_then(Value::as_str).filter(|path| !path.is_empty()).map(PathBuf::from).unwrap_or_else(|| hook.context.cwd.clone());
    if path.is_absolute() { path } else { hook.context.cwd.join(path) }
}

fn observe(hook: &HookClient, event: &str, payload: &HookPayload) -> Result<native::Resolution> {
    let kind = match event {
        super::HOOK_SESSIONSTART => native::ObservationKind::SessionStart {
            source: match payload.raw.get("source").and_then(Value::as_str).unwrap_or("") {
                "startup" => native::StartSource::Startup,
                "resume" => native::StartSource::Resume,
                "fork" => native::StartSource::Fork,
                "compact" => native::StartSource::Compact,
                _ => native::StartSource::Other,
            },
        },
        super::HOOK_SUBAGENT_START => native::ObservationKind::SubagentStart { agent_type: payload.raw.get("agent_type").and_then(Value::as_str).unwrap_or("task").to_owned() },
        _ => native::ObservationKind::Callback,
    };
    // SubagentStop acquires the native claim before its child row is read.
    let agent_id = if event == super::HOOK_SUBAGENT_STOP { None } else { payload.raw.get("agent_id").and_then(Value::as_str).filter(|id| !id.is_empty()).map(str::to_owned) };
    let terminal_shell_tool_use = if matches!(event, super::HOOK_POST | super::HOOK_POST_FAILURE | super::HOOK_PERMISSION_DENIED) && super::is_shell_tool(&payload.tool_name) { super::tool_use_id(payload).map(str::to_owned) } else { None };
    let transcript_path = payload
        .transcript_path
        .as_deref()
        .filter(|path| !path.is_empty())
        .map(|path| {
            let path = PathBuf::from(path);
            if path.is_absolute() { path } else { hook.context.cwd.join(path) }
        })
        .map(|path| path.to_string_lossy().into_owned());
    let mut input = native::Observation {
        kind,
        session_id: super::hook_payload_session_id(&payload.raw).to_owned(),
        env_session_candidate: super::session_id_from_claude_env_file(hook.context.claude_env_file.as_deref()),
        agent_id,
        transcript_path,
        directory: Some(directory(hook, payload)),
        evidence: native::TranscriptEvidence::NotRead,
        terminal_shell_tool_use,
    };
    let resolve = |input| match call(hook, native::Request::Observe(input))? {
        native::Response::Observed(resolution) => Ok(resolution),
        _ => bail!("unexpected Claude identity response"),
    };
    let first = resolve(input.clone())?;
    if !matches!(first, native::Resolution::NeedsLineage { .. }) {
        return Ok(first);
    }
    input.evidence = match input.transcript_path.as_deref() {
        None => native::TranscriptEvidence::Missing,
        Some(path) => match super::lineage::read_transcript_session_ids(path) {
            Ok(evidence) => native::TranscriptEvidence::Envelopes { top_level_sessions: evidence.top_level, message_sessions: evidence.message_envelope, bytes_read: evidence.bytes_read, records_read: evidence.records_read },
            Err(error) => {
                crate::log::log_warn("hooks", "claude.lineage_read_failed", &format!("path={path} err={error}"));
                native::TranscriptEvidence::ReadFailed
            }
        },
    };
    // Supplied evidence changes intent and therefore receives a fresh ID.
    resolve(input)
}

pub(super) fn activity(hook: &HookClient, actor: &native::Actor, activity: native::Activity) -> Result<Option<String>> {
    match call(hook, native::Request::Activity { actor: actor.clone(), activity })? {
        native::Response::Activity { actor_capability } => Ok(actor_capability),
        _ => bail!("unexpected Claude activity response"),
    }
}

pub(super) fn apply_stop(outcome: SessionStopOutcome) {
    for effect in outcome.effects {
        crate::runtime::stop_effects::apply(effect);
    }
}

pub(super) fn prepare(hook: &HookClient, actor: &native::Actor, mailbox: bool, bootstrap: bool, claim: Option<&native::StopClaim>) -> Result<native::Output> {
    match call(hook, native::Request::PrepareOutput { actor: actor.clone(), mailbox, bootstrap, stop_claim: claim.cloned() })? {
        native::Response::Output(output) => Ok(output),
        _ => bail!("unexpected Claude prepared output"),
    }
}

pub(super) fn mailbox_text(output: &PreparedOutput) -> Option<&str> {
    match output {
        PreparedOutput::Unmanaged { messages, formatted_context, .. } if !messages.is_empty() => Some(formatted_context),
        PreparedOutput::Claimed { messages, batch, .. } if !messages.is_empty() => Some(&batch.formatted_context),
        _ => None,
    }
}

fn confirmation(output: &PreparedOutput) -> Option<OutputConfirmation> {
    mailbox_text(output)?;
    match output {
        PreparedOutput::Unmanaged { receipt, .. } => Some(OutputConfirmation::Unmanaged(receipt.clone())),
        PreparedOutput::Claimed { target, batch, .. } => Some(OutputConfirmation::Claimed(DeliveryReceipt { target: target.clone(), batch: batch.id.clone(), through: batch.window.through, evidence: DeliveryEvidence::HookOutputWritten })),
        PreparedOutput::Deferred => None,
    }
}

fn bootstrap_text(hook: &HookClient, actor: &native::Actor, offer: &native::BootstrapOffer) -> Result<String> {
    Ok(match &offer.kind {
        Bootstrap::Root { tool } => crate::runtime::bootstrap::get_bootstrap_client(&hook.client, &hook.context, &actor.participant.canonical_name, tool)?,
        Bootstrap::Subagent { parent } => crate::runtime::bootstrap::get_subagent_bootstrap_for_scope(&actor.participant.canonical_name, parent, &hook.context.agentx_dir, &hook.context.raw_env),
    })
}

fn prepared_output(actor: &native::Actor, output: native::Output, value: Value) -> HookOutput {
    HookOutput { value: Some(value), actor: Some(actor.clone()), mailbox: confirmation(&output.mailbox), bootstrap: output.bootstrap.map(|offer| offer.receipt), ..Default::default() }
}

pub(super) fn stop_output(hook: &HookClient, actor: &native::Actor, output: native::Output, release: Option<native::StopClaim>) -> Result<HookOutput> {
    let mut reason = mailbox_text(&output.mailbox).unwrap_or("").to_owned();
    if let Some(offer) = &output.bootstrap {
        let text = bootstrap_text(hook, actor, offer)?;
        if !text.is_empty() {
            reason = if reason.is_empty() { text } else { format!("{text}\n\n{reason}") };
        }
    }
    let mut result = prepared_output(actor, output, json!({"decision":"block","reason":reason}));
    result.release = release;
    Ok(result)
}

fn context_output(hook: &HookClient, actor: &native::Actor, output: native::Output, event: &str, show_mail: bool) -> Result<HookOutput> {
    let mut parts = Vec::new();
    let mut post_outputs = Vec::new();
    if let Some(offer) = &output.bootstrap {
        let text = bootstrap_text(hook, actor, offer)?;
        if event == "PostToolUse" {
            post_outputs.push(render::context(event, &text, Some("[AGENTX info shown to instance]")));
        }
        parts.push(text);
    }
    let mail = mailbox_text(&output.mailbox);
    if let Some(text) = mail {
        if event == "PostToolUse" {
            post_outputs.push(render::context(event, text, if show_mail { Some(render::mailbox_display(text)) } else { None }));
        }
        parts.push(text.to_owned());
    }
    if parts.is_empty() {
        return Ok(HookOutput::default());
    }
    let display = if show_mail { mail.map(render::mailbox_display) } else { None };
    let value = if event == "PostToolUse" { render::combine_post(&post_outputs) } else { render::context(event, &parts.join("\n\n---\n\n"), display) };
    Ok(prepared_output(actor, output, value))
}

pub(super) fn route(hook: &HookClient, event: &str, payload: &HookPayload) -> Result<HookOutput> {
    let resolution = observe(hook, event, payload)?;
    let view = match resolution {
        native::Resolution::Ready(view) => view,
        native::Resolution::Historical { session_id, root } if event == super::HOOK_SESSIONEND => {
            let actor = native::Actor { root: root.clone(), participant: root, session_id, agent_id: None };
            session_end(hook, actor, payload)?;
            return Ok(HookOutput::default());
        }
        _ => return Ok(unrouted(event, payload)),
    };
    let actor = &view.actor;
    match event {
        super::HOOK_SESSIONSTART => {
            crate::runtime::env::set_terminal_title(&actor.participant.canonical_name);
            crate::relay::worker::try_spawn_worker();
            let Some(offer) = view.bootstrap else {
                return Ok(HookOutput::default());
            };
            let reason = match payload.raw.get("source").and_then(Value::as_str) {
                Some("compact") => crate::runtime::agent_context::ContextReason::Compact,
                Some("resume") => crate::runtime::agent_context::ContextReason::Resume,
                _ => crate::runtime::agent_context::ContextReason::Startup,
            };
            let text = crate::runtime::bootstrap::get_context_client(&hook.client, &hook.context, &actor.participant.canonical_name, &view.record.tool, reason)?;
            Ok(HookOutput { value: Some(render::context("SessionStart", &text, None)), actor: Some(actor.clone()), bootstrap: Some(offer.receipt), ..Default::default() })
        }
        super::HOOK_SUBAGENT_START => Ok(HookOutput { value: actor.agent_id.as_deref().map(|id| render::subagent_start(id, &actor.root.canonical_name)), ..Default::default() }),
        super::HOOK_SUBAGENT_STOP => super::poll::child(hook, actor, payload),
        super::HOOK_PRE => {
            let detail = crate::integrations::hooks::tool_status::extract_tool_detail("claude", &payload.tool_name, &payload.tool_input);
            if matches!(payload.tool_name.as_str(), "Edit" | "Write") && detail.contains("session-memory/") {
                return Ok(HookOutput::default());
            }
            let visible = super::is_shell_tool(&payload.tool_name) && super::tool_use_id(payload).is_some() && payload.tool_input.get("command").and_then(Value::as_str).is_some_and(super::visibly_invokes_agentx);
            let token = activity(hook, actor, native::Activity::ToolStarted { tool_name: payload.tool_name.clone(), detail, tool_use_id: super::tool_use_id(payload).map(str::to_owned), visible_agentx_shell: visible })?;
            Ok(HookOutput { value: token.as_deref().and_then(|token| render::shell_actor(payload, token, &actor.session_id)), ..Default::default() })
        }
        super::HOOK_POST => {
            activity(hook, actor, native::Activity::ToolSucceeded { tool_name: payload.tool_name.clone(), tool_use_id: super::tool_use_id(payload).map(str::to_owned) })?;
            if super::is_subagent_tool(&payload.tool_name) {
                return task_completed(hook, actor, payload);
            }
            if actor.agent_id.is_some() && !super::is_shell_tool(&payload.tool_name) {
                return Ok(HookOutput::default());
            }
            let output = prepare(hook, actor, true, actor.agent_id.is_none() && super::is_shell_tool(&payload.tool_name) && hook.context.is_launched, None)?;
            context_output(hook, actor, output, "PostToolUse", actor.agent_id.is_none())
        }
        super::HOOK_POST_FAILURE | super::HOOK_PERMISSION_DENIED => {
            let denied = event == super::HOOK_PERMISSION_DENIED;
            let detail = payload.raw.get(if denied { "reason" } else { "error" }).and_then(Value::as_str).unwrap_or(if denied { "permission denied" } else { "tool failed" }).to_owned();
            let tool_name = payload.tool_name.clone();
            let tool_use_id = super::tool_use_id(payload).map(str::to_owned);
            let activity_value = if denied { native::Activity::PermissionDenied { tool_name, detail, tool_use_id } } else { native::Activity::ToolFailed { tool_name, detail, tool_use_id } };
            activity(hook, actor, activity_value)?;
            Ok(HookOutput::default())
        }
        super::HOOK_PERMISSION_REQUEST => {
            let detail = crate::integrations::hooks::tool_status::extract_tool_detail("claude", &payload.tool_name, &payload.tool_input);
            activity(hook, actor, native::Activity::PermissionRequested { tool_name: payload.tool_name.clone(), detail, tool_use_id: super::tool_use_id(payload).map(str::to_owned) })?;
            Ok(HookOutput::default())
        }
        super::HOOK_STOP_FAILURE => {
            let error = payload.raw.get("error").and_then(Value::as_str).unwrap_or("unknown");
            let detail = payload.raw.get("error_details").and_then(Value::as_str).unwrap_or(error);
            activity(hook, actor, native::Activity::StopFailed { error: error.to_owned(), detail: detail.to_owned() })?;
            Ok(HookOutput::default())
        }
        super::HOOK_USERPROMPTSUBMIT if actor.agent_id.is_some() => Ok(unrouted(event, payload)),
        super::HOOK_USERPROMPTSUBMIT => {
            let bare_wake = super::is_bare_agentx_wake(payload);
            activity(hook, actor, native::Activity::Prompt { bare_wake })?;
            let output = prepare(hook, actor, hook.context.is_pty_mode, hook.context.is_launched, None)?;
            let mut result = context_output(hook, actor, output, "UserPromptSubmit", true)?;
            if result.value.is_none() && bare_wake {
                return Ok(HookOutput {
                    value: Some(json!({"decision":"block","reason":"This agentx wake arrived after another hook handled the queued delivery. Nothing is pending; no action is needed.","suppressOriginalPrompt":true})),
                    ..Default::default()
                });
            }
            result.activate_prompt = bare_wake && result.bootstrap.is_some() && result.mailbox.is_none();
            Ok(result)
        }
        super::HOOK_POLL if actor.agent_id.is_none() => super::poll::root(hook, actor, payload),
        super::HOOK_NOTIFY if actor.agent_id.is_none() => {
            let message = payload.raw.get("message").and_then(Value::as_str).unwrap_or("");
            let notification = match payload.notification_type.as_deref() {
                Some("idle_prompt") => Some(native::Activity::Idle),
                Some("permission_prompt" | "auth_success") => None,
                Some("elicitation_dialog") => Some(native::Activity::Approval),
                _ if message == "Claude is waiting for your input" => Some(native::Activity::Idle),
                _ if message.starts_with("Claude needs your permission") => Some(native::Activity::Approval),
                _ => None,
            };
            if let Some(notification) = notification {
                activity(hook, actor, notification)?;
            }
            Ok(HookOutput::default())
        }
        super::HOOK_SESSIONEND if actor.agent_id.is_none() => {
            session_end(hook, actor.clone(), payload)?;
            Ok(HookOutput::default())
        }
        _ => Ok(HookOutput::default()),
    }
}

fn task_completed(hook: &HookClient, actor: &native::Actor, payload: &HookPayload) -> Result<HookOutput> {
    let async_launched = payload.raw.pointer("/tool_response/status").and_then(Value::as_str) == Some("async_launched");
    let output = match call(hook, native::Request::TaskCompleted { actor: actor.clone(), async_launched })? {
        native::Response::TaskOutput(output) => output,
        _ => bail!("unexpected Claude task output"),
    };
    let Some(output) = output else {
        return Ok(HookOutput::default());
    };
    let Some(value) = render::task(&output) else {
        return Ok(HookOutput::default());
    };
    Ok(HookOutput { value: Some(value), actor: Some(actor.clone()), mailbox: confirmation(&output.mailbox), history: output.history_receipt, ..Default::default() })
}

fn session_end(hook: &HookClient, actor: native::Actor, payload: &HookPayload) -> Result<()> {
    let reason = payload.raw.get("reason").and_then(Value::as_str).unwrap_or("unknown").to_owned();
    match call(hook, native::Request::SessionEnd { root: actor, reason })? {
        native::Response::Stopped(outcome) => {
            apply_stop(outcome);
            Ok(())
        }
        _ => bail!("unexpected Claude session end response"),
    }
}

fn unrouted(event: &str, payload: &HookPayload) -> HookOutput {
    let value = if event == super::HOOK_USERPROMPTSUBMIT && super::is_bare_agentx_wake(payload) {
        let (_, text, _) = super::block_unroutable_bare_agentx_wake();
        serde_json::from_str(&text).ok()
    } else if event == super::HOOK_PRE && super::child_visibly_invokes_agentx(payload) {
        Some(render::child_denied("This Claude subagent has no verified agentx identity. Run `agentx start` in its parent Claude session first, then spawn a new subagent."))
    } else {
        None
    };
    HookOutput { value, ..Default::default() }
}

/// The complete JSON must flush before the combined canonical confirmation.
/// On write failure neither mailbox nor bootstrap/history is acknowledged.
pub(super) fn flush(hook: &HookClient, output: HookOutput, writer: &mut impl Write) -> Result<()> {
    let Some(value) = output.value else {
        return Ok(());
    };
    serde_json::to_writer(&mut *writer, &value)?;
    writer.flush()?;
    if let Some(actor) = output.actor {
        if output.mailbox.is_some() || output.bootstrap.is_some() || output.history.is_some() {
            call(hook, native::Request::ConfirmFlushed { actor: actor.clone(), mailbox: output.mailbox, bootstrap: output.bootstrap, history: output.history })?;
        }
        if output.activate_prompt {
            activity(hook, &actor, native::Activity::Prompt { bare_wake: false })?;
        }
    }
    if let Some(claim) = output.release {
        match call(hook, native::Request::FinishChildStop { claim, decision: native::FinishChildStop::Released })? {
            native::Response::Stopped(outcome) => apply_stop(outcome),
            _ => bail!("unexpected Claude stop release response"),
        }
    }
    Ok(())
}

pub(super) fn dispatch(event: &str) -> i32 {
    use crate::observability::trace::{self, Phase};
    let context = trace::time(Phase::HookContext, AgentxContext::from_os);
    if context.process_id.is_none() {
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
    let payload = HookPayload::from_claude(raw);
    let mut hook = match trace::time(Phase::DaemonAcquire, || HookClient::connect(&context, false)) {
        Ok(hook) => hook,
        Err(error) => {
            trace::fail(Phase::DaemonAcquire);
            let output = error_output(event, &payload, &error);
            if let Some(value) = output.value {
                let mut writer = std::io::stdout().lock();
                let _ = serde_json::to_writer(&mut writer, &value).and_then(|_| writer.flush().map_err(serde_json::Error::io));
            }
            return 0;
        }
    };
    if event == super::HOOK_SESSIONSTART {
        let cwd = directory(&hook, &payload);
        let mut invocation = hook.client.invocation().clone();
        invocation.launch_context = Some(Value::Object(trace::time(Phase::LaunchContext, || crate::agents::binding::capture_context_snapshot(&context.agentx_dir, &cwd, &context.raw_env))));
        hook.client = hook.client.with_invocation(invocation);
    }
    let result = trace::time(Phase::Route, || super::common::dispatch_with_panic_guard("claude", event, Err(anyhow::anyhow!("Claude hook panicked")), || route(&hook, event, &payload)));
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            trace::fail(Phase::Route);
            error_output(event, &payload, &error)
        }
    };
    if trace::time(Phase::Output, || flush(&hook, output, &mut std::io::stdout().lock())).is_err() {
        trace::fail(Phase::Output);
    }
    // Stop/SubagentStop consume decision:block only on exit 0.
    0
}

fn error_output(event: &str, payload: &HookPayload, error: &anyhow::Error) -> HookOutput {
    if event == super::HOOK_SUBAGENT_STOP {
        return HookOutput { value: Some(json!({"decision":"block","reason":format!("agentx could not finish SubagentStop: {error}. Please stop again.")})), ..Default::default() };
    }
    unrouted(event, payload)
}
