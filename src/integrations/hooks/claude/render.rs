//! Claude-owned JSON and shell command rendering. Inputs are already admitted
//! by the participant domain; these helpers neither resolve actors nor write state.
use super::HookPayload;
use serde_json::{Value, json};

pub(super) fn context(event: &str, text: &str, visible: Option<&str>) -> Value {
    let mut output = json!({"hookSpecificOutput":{"hookEventName":event,"additionalContext":text}});
    if let Some(visible) = visible {
        output["systemMessage"] = json!(visible);
    }
    output
}

/// Preserve the existing dual display/model shape without reformatting the
/// domain's frozen mailbox or reading another participant snapshot.
pub(super) fn mailbox_display(formatted: &str) -> &str {
    formatted.strip_prefix("<agentx>").and_then(|text| text.strip_suffix("</agentx>")).unwrap_or(formatted)
}

pub(super) fn combine_post(outputs: &[Value]) -> Value {
    if outputs.len() == 1 {
        return outputs[0].clone();
    }
    let system: Vec<_> = outputs.iter().filter_map(|output| output.get("systemMessage").and_then(Value::as_str)).collect();
    let contexts: Vec<_> = outputs.iter().filter_map(|output| output.pointer("/hookSpecificOutput/additionalContext").and_then(Value::as_str)).collect();
    let mut output = context("PostToolUse", &contexts.join("\n\n---\n\n"), None);
    if !system.is_empty() {
        output["systemMessage"] = json!(system.join(" + "));
    }
    output
}

pub(super) fn shell_actor(payload: &HookPayload, token: &str, session: &str) -> Option<Value> {
    let command = payload.tool_input.get("command")?.as_str()?;
    let mut input = payload.tool_input.clone();
    let object = input.as_object_mut()?;
    let actor_var = super::actor::ENV_VAR;
    let session_var = super::actor::SESSION_ENV_VAR;
    let command = if payload.tool_name == "PowerShell" {
        format!("$env:{actor_var} = {}; $env:{session_var} = {}\n{command}", super::powershell_single_quote(token), super::powershell_single_quote(session))
    } else {
        let quote = crate::integrations::launch_args::shell::shell_quote;
        format!("export {actor_var}={} {session_var}={}\n{command}", quote(token), quote(session))
    };
    object.insert("command".into(), json!(command));
    Some(json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","updatedInput":input}}))
}

pub(super) fn subagent_start(agent_id: &str, root: &str) -> Value {
    let command = crate::runtime::env::build_agentx_command();
    context("SubagentStart", &format!("Your agent ID: {agent_id}\nYour agentx parent: {root}\nTo use agentx, first run: {command} start --name {agent_id}"), None)
}

pub(super) fn child_denied(reason: &str) -> Value {
    json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":reason}})
}

pub(super) fn task(output: &crate::agents::claude_service::TaskOutput) -> Option<Value> {
    if output.messages.is_empty() {
        return None;
    }
    let children = if output.children.is_empty() {
        "none".to_owned()
    } else {
        output
            .children
            .iter()
            .map(|child| match &child.agent_id {
                Some(id) => format!("{} (agent_id: {id})", child.participant.canonical_name),
                None => child.participant.canonical_name.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let messages = output.messages.iter().map(|message| format!("{}: {}", message.from, message.text)).collect::<Vec<_>>().join("\n");
    let summary = format!(
        "[Task tool completed - Message history during Task tool]\nSubagents: {children}\nThe following {} message(s) occurred:\n\n{messages}\n\n[End of message history. Subagents have finished and are no longer active.]",
        output.messages.len()
    );
    Some(context("PostToolUse", &summary, Some("[Task subagent messages shown to instance]")))
}
