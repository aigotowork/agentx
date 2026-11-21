//! Claude Stop waits run only in the originating hook process. The daemon
//! admits finite endpoint/claim operations and returns frozen retirement effects.
use super::client::{self, HookOutput};
use crate::agents::claude_service as native;
use crate::integrations::hooks::{HookPayload, client::HookClient};
use crate::runtime::notify::NotifyServer;
use anyhow::{Result, bail};
use serde_json::Value;
use std::time::{Duration, Instant};

#[expect(clippy::large_enum_variant, reason = "A single hook wait returns its frozen native Output by value; keep the direct finite result without an extra allocation.")]
enum WaitOutcome {
    Delivered(native::Output),
    TimedOut,
    ParentClosed,
}

fn endpoint(hook: &HookClient, actor: &native::Actor, port: Option<u16>, claim: Option<&native::StopClaim>) -> Result<()> {
    match client::call(hook, native::Request::PollEndpoint { actor: actor.clone(), port, stop_claim: claim.cloned() })? {
        native::Response::Changed => Ok(()),
        _ => bail!("unexpected Claude poll endpoint response"),
    }
}

fn wait(hook: &HookClient, actor: &native::Actor, timeout: u64, bootstrap: bool, claim: Option<&native::StopClaim>, background: bool) -> Result<WaitOutcome> {
    let server = match NotifyServer::new() {
        Ok(server) => Some(server),
        Err(error) => {
            crate::log::log_warn("hooks", "claude.poll_listener_failed", &error.to_string());
            None
        }
    };
    endpoint(hook, actor, server.as_ref().map(NotifyServer::port), claim)?;
    let result = wait_loop(hook, actor, Duration::from_secs(timeout), bootstrap, claim, background, server.as_ref());
    let cleanup = endpoint(hook, actor, None, claim);
    match result {
        Ok(output) => {
            cleanup?;
            Ok(output)
        }
        Err(error) => {
            if let Err(cleanup) = cleanup {
                crate::log::log_warn("hooks", "claude.poll_cleanup_failed", &cleanup.to_string());
            }
            Err(error)
        }
    }
}

fn wait_loop(hook: &HookClient, actor: &native::Actor, timeout: Duration, bootstrap: bool, claim: Option<&native::StopClaim>, background: bool, server: Option<&NotifyServer>) -> Result<WaitOutcome> {
    let started = Instant::now();
    let mut waited = false;
    while started.elapsed() < timeout {
        // A drained input pipe is normal; only ERR/NVAL proves broken input.
        // Check before freezing output once we have actually waited.
        if waited && !background && crate::sys::io::stdin_appears_broken() {
            return Ok(WaitOutcome::ParentClosed);
        }
        let output = client::prepare(hook, actor, true, bootstrap, claim)?;
        if client::mailbox_text(&output.mailbox).is_some() {
            return Ok(WaitOutcome::Delivered(output));
        }
        let remaining = timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            break;
        }
        if let Some(server) = server {
            server.wait(remaining.min(Duration::from_secs(30)));
        } else {
            std::thread::sleep(remaining.min(Duration::from_millis(100)));
        }
        waited = true;
        // Exact endpoint ownership also renews last_seen/tcp_mode, preserving
        // a later poller or namesake successor on registration/cleanup races.
        endpoint(hook, actor, server.map(NotifyServer::port), claim)?;
    }
    Ok(WaitOutcome::TimedOut)
}

pub(super) fn root(hook: &HookClient, actor: &native::Actor, payload: &HookPayload) -> Result<HookOutput> {
    let background_tasks = payload.raw.get("background_tasks").and_then(Value::as_array).map(|tasks| {
        tasks
            .iter()
            .map(|task| match task.get("type").and_then(Value::as_str) {
                Some("shell") => native::BackgroundTask::Shell,
                Some("monitor") => native::BackgroundTask::Monitor,
                Some("MCP task") => native::BackgroundTask::Mcp,
                _ => native::BackgroundTask::MayRunAgent,
            })
            .collect()
    });
    let (timeout, pty) = match client::call(hook, native::Request::BeginRootStop { actor: actor.clone(), background_tasks })? {
        native::Response::RootStop { timeout_secs, pty, retired_children } => {
            client::apply_stop(retired_children);
            (timeout_secs, pty)
        }
        _ => bail!("unexpected Claude root stop response"),
    };
    if pty {
        return Ok(HookOutput::default());
    }
    match wait(hook, actor, timeout, false, None, hook.context.is_background)? {
        WaitOutcome::Delivered(output) => client::stop_output(hook, actor, output, None),
        WaitOutcome::TimedOut => {
            client::activity(hook, actor, native::Activity::PollTimedOut)?;
            Ok(HookOutput::default())
        }
        WaitOutcome::ParentClosed => Ok(HookOutput::default()),
    }
}

pub(super) fn child(hook: &HookClient, root: &native::Actor, payload: &HookPayload) -> Result<HookOutput> {
    let Some(id) = payload.raw.get("agent_id").and_then(Value::as_str).filter(|id| !id.is_empty()) else {
        return Ok(HookOutput::default());
    };
    let mut raw = payload.raw.clone();
    if let Some(object) = raw.as_object_mut() {
        object.insert("session_id".into(), Value::String(root.session_id.clone()));
    }
    let request = native::Request::BeginChildStop {
        root: root.clone(),
        agent_id: id.to_owned(),
        prompt_id: raw.get("prompt_id").and_then(Value::as_str).unwrap_or("").to_owned(),
        fingerprint: super::hash_raw_payload(&raw),
        transcript_path: raw.get("agent_transcript_path").and_then(Value::as_str).filter(|path| !path.is_empty()).map(str::to_owned),
    };
    let (claim, actor, timeout, dormant) = match client::call(hook, request)? {
        native::Response::ChildStop(native::ChildStopResult::Acquired { claim, actor, timeout_secs, dormant, .. }) => (claim, actor, timeout_secs, dormant),
        native::Response::ChildStop(native::ChildStopResult::Retired(outcome)) => {
            client::apply_stop(outcome);
            return Ok(HookOutput::default());
        }
        native::Response::ChildStop(native::ChildStopResult::Missing | native::ChildStopResult::Duplicate) => return Ok(HookOutput::default()),
        _ => bail!("unexpected Claude child stop response"),
    };
    client::activity(hook, &actor, native::Activity::Idle)?;
    let decision = match wait(hook, &actor, timeout, dormant, Some(&claim), false)? {
        WaitOutcome::Delivered(output) => return client::stop_output(hook, &actor, output, Some(claim)),
        WaitOutcome::TimedOut => native::FinishChildStop::TimedOut,
        WaitOutcome::ParentClosed => native::FinishChildStop::TaskCompleted,
    };
    match client::call(hook, native::Request::FinishChildStop { claim, decision })? {
        native::Response::Stopped(outcome) => {
            client::apply_stop(outcome);
            Ok(HookOutput::default())
        }
        _ => bail!("unexpected Claude child retirement response"),
    }
}
