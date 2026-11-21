use serde_json::{Value, json};

#[cfg(test)]
use crate::agents::binding as instance_binding;
#[cfg(test)]
use crate::agents::lifecycle;
#[cfg(test)]
use crate::agents::records as instances;
#[cfg(test)]
use crate::db::{AgentxDb, InstanceRow};
#[cfg(test)]
use crate::integrations::hooks::HookResult;
use crate::integrations::hooks::{HookPayload, common};
use crate::log;
use crate::runtime::context::AgentxContext;
use crate::shared::{ST_ACTIVE, ST_BLOCKED, ST_LISTENING};

use crate::integrations::hooks::kimi::config::kimi_config_dir;

#[cfg(test)]
fn resolve_instance(db: &AgentxDb, ctx: &AgentxContext, payload: &HookPayload) -> Option<InstanceRow> {
    instance_binding::resolve_instance_from_binding(db, payload.session_id.as_deref(), ctx.process_id.as_deref())
}

/// Resolve a Kimi session's transcript file.
///
/// Kimi stores each session's wire log at:
/// ```text
///   $KIMI_CODE_HOME/sessions/wd_<dir>_<hash>/<session_id>/agents/main/wire.jsonl
/// ```
/// The working-directory bucket (`wd_*`) is unknown here, so scan the buckets
/// for the one containing this session. `session_id` already carries the
/// `session_` prefix (matching the on-disk directory name).
pub fn derive_kimi_transcript_path(session_id: &str) -> Option<String> {
    let base = kimi_config_dir().join("sessions");
    if !base.exists() {
        return None;
    }
    let entries = std::fs::read_dir(&base).ok()?;
    for entry in entries.flatten() {
        let wd = entry.path();
        if !wd.is_dir() {
            continue;
        }
        let candidate = wd.join(session_id).join("agents").join("main").join("wire.jsonl");
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

#[cfg(test)]
fn update_position(db: &AgentxDb, ctx: &AgentxContext, payload: &HookPayload, instance_name: &str) {
    let mut updates = serde_json::Map::new();
    if let Some(session_id) = payload.session_id.as_ref().filter(|s| !s.is_empty()) {
        updates.insert("session_id".into(), Value::String(session_id.clone()));
        if let Some(tp) = derive_kimi_transcript_path(session_id) {
            updates.insert("transcript_path".into(), Value::String(tp));
        }
    }
    let cwd = payload.raw.get("cwd").and_then(Value::as_str).unwrap_or_else(|| ctx.cwd.to_str().unwrap_or(""));
    if !cwd.is_empty() {
        updates.insert("directory".into(), Value::String(cwd.to_string()));
    }
    if !updates.is_empty() {
        instances::update_instance_position(db, instance_name, &updates);
    }
}

#[cfg(test)]
pub(crate) fn handle_sessionstart(db: &AgentxDb, ctx: &AgentxContext, payload: &HookPayload) -> HookResult {
    // Persistent hooks also fire in plain runs; stay out of those.
    if ctx.process_id.is_none() {
        return hook_noop();
    }

    let session_id = match payload.session_id.as_deref() {
        Some(sid) => sid,
        None => return hook_noop(),
    };

    let env_tool = ctx.raw_env.get("AGENTX_TOOL").cloned();
    if env_tool.as_deref().is_some_and(|tool| tool != "kimi") {
        log::log_warn("hooks", "kimi.sessionstart.tool_env_refused", &format!("session_id={} process_id={:?} env_tool={:?}", session_id, ctx.process_id, env_tool));
        return hook_noop();
    }

    let bind = instance_binding::bind_session_to_process_for_tool(db, session_id, ctx.process_id.as_deref(), "kimi", true);

    log::log_info("hooks", "kimi.sessionstart.bind", &format!("instance={:?} session_id={} process_id={:?}", bind, session_id, ctx.process_id,));

    let instance_name = match bind {
        instance_binding::ToolCheckedBind::Bound(name) => name,
        instance_binding::ToolCheckedBind::Rejected => return hook_noop(),
        instance_binding::ToolCheckedBind::Unbound => return hook_noop(),
    };

    instance_binding::capture_and_store_launch_context(db, &instance_name);
    update_position(db, ctx, payload, &instance_name);

    lifecycle::set_status(db, &instance_name, ST_LISTENING, "start", Default::default());

    crate::runtime::env::set_terminal_title(&instance_name);
    crate::relay::worker::try_spawn_worker();

    // NOTE: bootstrap is intentionally NOT injected here. Kimi does not add
    // SessionStart hook output to model context (it is an observation-only
    // event), so bootstrapping happens on the first UserPromptSubmit instead
    // (see handle_userpromptsubmit).
    hook_noop()
}

#[cfg(test)]
fn handle_userpromptsubmit(db: &AgentxDb, ctx: &AgentxContext, payload: &HookPayload) -> HookResult {
    let instance = match resolve_instance(db, ctx, payload) {
        Some(inst) => inst,
        None => return hook_noop(),
    };
    let instance_name = &instance.name;
    update_position(db, ctx, payload, instance_name);

    // Bootstrap is delivered here, NOT at SessionStart: kimi only injects
    // UserPromptSubmit hook output into model context — SessionStart output is
    // not added to context (see kimi hooks docs). Prepend it to the first
    // delivery so a launched agent learns it's on agentx.
    //
    // KNOWN LIMITATION (kimi 0.9.0): this makes the bootstrap *visible* — kimi
    // wraps UserPromptSubmit output as a `<hook_result>` block in the turn,
    // unlike codex/claude which inject it invisibly into the system prompt.
    // Kimi has no per-instance invisible channel today: no `--system-prompt`
    // flag, no `-c` config override, no `systemPrompt` config key, and AGENTS.md
    // (its only invisible system-prompt source) loads from shared paths so it
    // can't carry a per-instance name without polluting the workspace. Revisit
    // and switch to an invisible launch-time injection (mirroring codex's
    // developer_instructions path) if a future kimi release adds a system-prompt
    // append flag or env var.
    let bootstrap = common::inject_bootstrap_once(db, ctx, instance_name, &instance, &instance.tool);
    let pending = common::prepare_pending_messages(db, instance_name);

    let additional_context = match (&bootstrap, &pending) {
        (Some(boot), Some(p)) => Some(format!("{boot}\n\n{}", p.formatted)),
        (Some(boot), None) => Some(boot.clone()),
        (None, Some(p)) => Some(p.formatted.clone()),
        (None, None) => None,
    };

    if let Some(additional_context) = additional_context {
        return HookResult::Allow { additional_context: Some(additional_context), system_message: None, delivery_ack: pending.map(|p| p.ack) };
    }

    hook_noop()
}

#[cfg(test)]
fn handle_pretooluse(db: &AgentxDb, _ctx: &AgentxContext, payload: &HookPayload) -> HookResult {
    let instance = match resolve_instance(db, _ctx, payload) {
        Some(inst) => inst,
        None => return hook_noop(),
    };
    let instance_name = &instance.name;

    crate::integrations::hooks::common::update_tool_status(db, instance_name, "kimi", &payload.tool_name, &payload.tool_input);

    hook_noop()
}

#[cfg(test)]
fn handle_posttooluse(_db: &AgentxDb, _ctx: &AgentxContext, _payload: &HookPayload) -> HookResult {
    // Kimi treats PostToolUse as observation-only; hook output is ignored and
    // must not advance the delivery cursor — silently acking here caused vanish.
    // Delivery happens at UserPromptSubmit and Stop only.
    hook_noop()
}

/// PermissionRequest (observation-only): kimi fires this just before it blocks
/// waiting for the user to approve/reject a tool call. Mark the agent `blocked`
/// so `agentx list` reflects the stall and the delivery gate (require_idle) holds
/// off injecting until the user responds. `PermissionResult` clears it.
///
/// agentx's own `[[permission.rules]]` allow-rules mean an agent's `agentx send`
/// etc. are auto-approved and never reach an `ask` — so this only fires for
/// tool calls that genuinely need a human (mirrors claude's handle_permission_request).
#[cfg(test)]
fn handle_permissionrequest(db: &AgentxDb, ctx: &AgentxContext, payload: &HookPayload) -> HookResult {
    let instance = match resolve_instance(db, ctx, payload) {
        Some(inst) => inst,
        None => return hook_noop(),
    };
    let instance_name = &instance.name;

    let detail = crate::integrations::hooks::tool_status::extract_tool_detail("kimi", &payload.tool_name, &payload.tool_input);
    lifecycle::set_status(db, instance_name, ST_BLOCKED, "approval", lifecycle::StatusUpdate { detail: &detail, ..Default::default() });

    hook_noop()
}

/// PermissionResult (observation-only): kimi fires this once approval resolves.
/// In every case the agent is still mid-turn — an approved tool now runs, a
/// declined one feeds rejection back to the model — so flip out of `blocked`
/// back to `active`. The `Stop` hook sets `listening` when the turn actually
/// ends. (PermissionResult carries no tool_input, so the tool name is the best
/// available detail; mirrors claude's PostToolUse "approved:" restore.)
///
/// The payload's `decision` (`approved`/`rejected`/`cancelled`/`error`) drives a
/// decision-aware context so a declined call isn't mislabeled as approved:
/// `approved:<tool>` only when actually approved, `denied:<tool>` otherwise.
#[cfg(test)]
fn handle_permissionresult(db: &AgentxDb, ctx: &AgentxContext, payload: &HookPayload) -> HookResult {
    let instance = match resolve_instance(db, ctx, payload) {
        Some(inst) => inst,
        None => return hook_noop(),
    };
    let instance_name = &instance.name;

    let decision = payload.raw.get("decision").and_then(Value::as_str);
    let verb = if decision == Some("approved") { "approved" } else { "denied" };
    lifecycle::set_status(db, instance_name, ST_ACTIVE, &format!("{verb}:{}", payload.tool_name), Default::default());

    hook_noop()
}

#[cfg(test)]
pub(crate) fn handle_stop(db: &AgentxDb, ctx: &AgentxContext, payload: &HookPayload) -> HookResult {
    let instance = match resolve_instance(db, ctx, payload) {
        Some(inst) => inst,
        None => return hook_noop(),
    };
    let instance_name = &instance.name;

    if let Some(prepared) = common::prepare_pending_messages(db, instance_name) {
        // Defer ack to dispatch after Block stdout succeeds. Leave status
        // active — listening only on empty Stop (or PTY recovery). Stop Block
        // model visibility is the same delivery class as before; UPS remains
        // the wire-proven inject path.
        return HookResult::Block { reason: prepared.formatted, delivery_ack: Some(prepared.ack) };
    }

    lifecycle::set_status(db, instance_name, ST_LISTENING, "", Default::default());
    common::notify_hook_instance_with_db(db, instance_name);
    hook_noop()
}

#[cfg(test)]
pub(crate) fn handle_sessionend(db: &AgentxDb, ctx: &AgentxContext, payload: &HookPayload) -> HookResult {
    let instance = match resolve_instance(db, ctx, payload) {
        Some(inst) => inst,
        None => return hook_noop(),
    };
    let instance_name = &instance.name;

    if instance.tool != "kimi" {
        log::log_warn("hooks", "kimi.sessionend.tool_mismatch_ignored", &format!("instance={} tool={} payload_session_id={:?}", instance_name, instance.tool, payload.session_id));
        return hook_noop();
    }

    if let Some(incoming) = payload.session_id.as_deref()
        && let Some(primary) = instance.session_id.as_deref()
        && incoming != primary
    {
        log::log_warn("hooks", "kimi.sessionend.historical_ignored", &format!("instance={} incoming={} primary={}", instance_name, incoming, primary));
        return hook_noop();
    }

    // Current Kimi emits SessionEnd for a real session close (`exit` or
    // `archive`). The process is necessarily still alive while this synchronous
    // hook runs, so PID liveness cannot identify a resumable transition. A new
    // in-process session starts before the old SessionEnd and is protected by
    // the historical-session guard above.
    common::finalize_session(db, instance_name, "sessionend", None);

    hook_noop()
}

#[cfg(test)]
fn handle_subagentstart(_db: &AgentxDb, _ctx: &AgentxContext, _payload: &HookPayload) -> HookResult {
    hook_noop()
}

#[cfg(test)]
fn handle_subagentstop(_db: &AgentxDb, _ctx: &AgentxContext, _payload: &HookPayload) -> HookResult {
    hook_noop()
}

#[cfg(test)]
fn handle_notification(_db: &AgentxDb, _ctx: &AgentxContext, _payload: &HookPayload) -> HookResult {
    // Kimi treats Notification as observation-only; hook output is ignored and
    // must not advance the delivery cursor — silently acking here caused vanish.
    // Delivery happens at UserPromptSubmit and Stop only.
    hook_noop()
}

#[cfg(test)]
fn hook_noop() -> HookResult {
    HookResult::Allow { additional_context: None, system_message: None, delivery_ack: None }
}

#[cfg(test)]
pub(crate) fn get_handler(hook_name: &str) -> Option<fn(&AgentxDb, &AgentxContext, &HookPayload) -> HookResult> {
    match hook_name {
        "kimi-sessionstart" => Some(handle_sessionstart),
        "kimi-userpromptsubmit" => Some(handle_userpromptsubmit),
        "kimi-pretooluse" => Some(handle_pretooluse),
        "kimi-posttooluse" => Some(handle_posttooluse),
        "kimi-permissionrequest" => Some(handle_permissionrequest),
        "kimi-permissionresult" => Some(handle_permissionresult),
        "kimi-stop" => Some(handle_stop),
        "kimi-sessionend" => Some(handle_sessionend),
        "kimi-subagentstart" => Some(handle_subagentstart),
        "kimi-subagentstop" => Some(handle_subagentstop),
        "kimi-notification" => Some(handle_notification),
        _ => None,
    }
}

/// Production Kimi hooks use explicit caller identity and post-flush receipts.
pub fn dispatch_kimi_hook(hook_name: &str) -> i32 {
    let context = AgentxContext::from_os();
    if !common::hook_gate_check(&context) {
        return 0;
    }
    // These native events cannot inject model input. Do not prepare a claim.
    if matches!(hook_name, "kimi-posttooluse" | "kimi-notification" | "kimi-subagentstart" | "kimi-subagentstop") {
        return 0;
    }
    let raw: Value = match serde_json::from_reader(std::io::stdin().lock()) {
        Ok(raw) => raw,
        Err(error) => {
            log::log_warn("hooks", "kimi.parse_error", &error.to_string());
            return 0;
        }
    };
    let payload = HookPayload::from_kimi(hook_name, raw);
    let result = common::dispatch_with_panic_guard("kimi", hook_name, Ok(0), || {
        let hook = super::super::client::HookClient::connect(&context, hook_name == "kimi-sessionstart")?;
        route_client(&hook, hook_name, payload)
    });
    match result {
        Ok(code) => code,
        Err(error) => {
            log::log_warn("hooks", "kimi.dispatch_error", &error.to_string());
            0
        }
    }
}

fn route_client(hook: &super::super::client::HookClient, hook_name: &str, mut payload: HookPayload) -> anyhow::Result<i32> {
    if payload.transcript_path.as_deref().is_none_or(str::is_empty) {
        payload.transcript_path = payload.session_id.as_deref().and_then(derive_kimi_transcript_path);
    }
    let directory = payload.raw.get("cwd").and_then(Value::as_str).filter(|cwd| !cwd.is_empty()).map(std::path::PathBuf::from).or_else(|| Some(hook.context.cwd.clone()));
    let actor = if matches!(hook_name, "kimi-sessionstart" | "kimi-userpromptsubmit") { hook.session("kimi", &payload, directory, hook_name == "kimi-sessionstart")? } else { hook.resolve("kimi")? };
    let mut mailbox = None;
    let mut bootstrap = None;
    let mut output = None;
    let mut exit_code = 0;
    match hook_name {
        "kimi-sessionstart" => {
            hook.status(&actor, ST_LISTENING, "start", "")?;
            crate::runtime::env::set_terminal_title(&actor.participant.canonical_name);
            crate::relay::worker::try_spawn_worker();
            // Kimi ignores SessionStart output; first prompt carries bootstrap.
        }
        "kimi-userpromptsubmit" => {
            bootstrap = hook.bootstrap(&actor, false)?;
            mailbox = hook.pending(&actor)?;
            let refreshed = if bootstrap.is_none() { hook.runtime_context(&actor)? } else { String::new() };
            let text = bootstrap.iter().map(|(text, _)| text.as_str()).chain((!refreshed.is_empty()).then_some(refreshed.as_str())).chain(mailbox.iter().map(|pending| pending.formatted.as_str())).collect::<Vec<_>>().join("\n\n");
            if !text.is_empty() {
                output = Some(json!({"hookSpecificOutput":{"message":text}}));
            }
        }
        "kimi-pretooluse" => hook.tool_status(&actor, &payload)?,
        "kimi-permissionrequest" => {
            let detail = super::super::tool_status::extract_tool_detail("kimi", &payload.tool_name, &payload.tool_input);
            hook.status(&actor, ST_BLOCKED, "approval", &detail)?;
        }
        "kimi-permissionresult" => {
            let verb = if payload.raw.get("decision").and_then(Value::as_str) == Some("approved") { "approved" } else { "denied" };
            hook.status(&actor, ST_ACTIVE, &format!("{verb}:{}", payload.tool_name), "")?;
        }
        "kimi-stop" => {
            mailbox = hook.pending(&actor)?;
            if let Some(pending) = &mailbox {
                eprintln!("{}", pending.formatted);
                output = Some(json!({"hookSpecificOutput":{"permissionDecision":"deny","permissionDecisionReason":pending.formatted}}));
                exit_code = 2;
            } else {
                hook.status(&actor, ST_LISTENING, "", "")?;
                hook.wake(&actor)?;
            }
        }
        "kimi-sessionend" => hook.stop(&actor, payload.session_id.as_deref(), "sessionend", false, false)?,
        _ => {}
    }
    if let Some(output) = output
        && let Err(error) = hook.flush_json(&actor, &output, mailbox.as_ref(), bootstrap.as_ref().map(|(_, receipt)| receipt.as_str()))
    {
        log::log_warn("hooks", "kimi.output_error", &error.to_string());
    }
    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn pretooluse_records_tool_context_even_without_detail() {
        let (_dir, agentx_dir, home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let db = AgentxDb::open_at(&agentx_dir.join("agentx.db")).unwrap();
        db.save_instance_named("luna", json!({"tool": "kimi", "created_at": 1.0, "status": "listening"}).as_object().unwrap()).unwrap();
        db.rebind_session("ses-kimi", "luna").unwrap();
        db.set_process_binding("pid-kimi", "ses-kimi", "luna").unwrap();
        let ctx = AgentxContext::from_env(&std::collections::HashMap::from([("AGENTX_PROCESS_ID".into(), "pid-kimi".into())]), home);
        for (tool, input, expected_detail) in [("Bash", json!({"command": "echo hello"}), "echo hello"), ("Unknown", json!({}), "")] {
            let payload = HookPayload::from_kimi("kimi-pretooluse", json!({"session_id": "ses-kimi", "tool_name": tool, "tool_input": input}));
            handle_pretooluse(&db, &ctx, &payload);
            let row = db.get_instance_full("luna").unwrap().unwrap();
            assert_eq!(row.status, ST_ACTIVE);
            assert_eq!(row.status_context, format!("tool:{tool}"));
            assert_eq!(row.status_detail, expected_detail);
        }
    }
}
