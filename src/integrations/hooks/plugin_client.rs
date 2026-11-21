//! Provider plugin command adaptation. Parsing and output stay in the caller;
//! canonical lifecycle/mail effects use typed native capabilities over IPC.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::agents::service::{PluginStart, Request as ParticipantRequest, ResolvedActor, Response as ParticipantResponse};
use crate::messaging::inbox::DeliveryEvidence;
use crate::runtime::context::AgentxContext;
use crate::runtime::hosts::{HostService, Request as HostRequest};
use crate::service::client::Client;
use crate::service::{Request, Response};

use super::output::{self, OutputReceipt};
use super::plugin_rpc::{self, has_flag, parse_flag};

fn participant(client: &Client, request: ParticipantRequest) -> Result<ParticipantResponse> {
    match client.call(Request::Participants(request))? {
        Response::Participants(response) => Ok(response),
        _ => bail!("unexpected plugin participant response"),
    }
}

fn resolve(client: &Client, argv: &[String]) -> Result<ResolvedActor> {
    let actor = match participant(client, ParticipantRequest::Resolve)? {
        ParticipantResponse::Actor(actor) => actor,
        _ => bail!("unexpected plugin identity response"),
    };
    if let Some(name) = parse_flag(argv, "--name")
        && name != actor.participant.canonical_name
    {
        bail!("{name} is not attached to this process");
    }
    Ok(actor)
}

fn endpoint(client: &Client, actor: &ResolvedActor, port: u16, standby: bool) -> Result<()> {
    HostService::call(client, HostRequest::SetPluginEndpoint { participant: actor.participant.clone(), port, standby })?;
    Ok(())
}

fn start(client: &Client, context: &AgentxContext, argv: &[String], requested_tool: &str, recover_soft_stopped: bool) -> Result<Value> {
    let session_id = parse_flag(argv, "--session-id").context("Missing --session-id")?;
    let existing = if requested_tool == "opencode" { Some(resolve(client, &[])?) } else { None };
    let tool = if existing.as_ref().is_some_and(|actor| actor.record.tool == "kilo") { "kilo" } else { requested_tool };
    let transcript_path =
        parse_flag(argv, "--transcript-path").or_else(|| matches!(tool, "opencode" | "kilo").then(|| crate::runtime::env::opencode_family_db_path(tool)).flatten().filter(|path| path.exists()).map(|path| path.to_string_lossy().into_owned()));
    let input = PluginStart { tool: tool.to_owned(), session_id: session_id.clone(), transcript_path, cwd: parse_flag(argv, "--cwd").map(Into::into), recover_soft_stopped };
    let actor = match participant(client, ParticipantRequest::PluginStart(input))? {
        ParticipantResponse::Registered(actor) => actor,
        _ => bail!("unexpected plugin start response"),
    };
    if let Some(port) = parse_flag(argv, "--notify-port").and_then(|port| port.parse().ok()) {
        endpoint(client, &actor, port, false)?;
    }
    let name = &actor.participant.canonical_name;
    let instructions = crate::runtime::bootstrap::get_instructions_client(client, context, name, tool)?;
    let runtime_context = crate::runtime::bootstrap::get_initial_context_client(client, context, name, tool)?;
    let bootstrap = [instructions.as_str(), runtime_context.as_str()].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join("\n\n");
    let mut result = json!({"name": name, "session_id": session_id, "bootstrap": bootstrap, "instructions": instructions, "context": runtime_context});
    if matches!(tool, "opencode" | "kilo") {
        let (agent, model) = super::opencode::launch_agent_and_model_from_args(actor.record.launch_args.as_deref());
        if let Some(agent) = agent {
            result["agent"] = json!(agent);
        }
        if let Some(model) = model {
            result["model"] = model;
        }
    }
    crate::relay::worker::try_spawn_worker();
    Ok(result)
}

fn read(client: &Client, actor: &ResolvedActor, argv: &[String]) -> Result<Value> {
    if has_flag(argv, "--ack") {
        let through: i64 = parse_flag(argv, "--up-to").context("Missing --up-to")?.parse().context("Invalid --up-to")?;
        let receipt: OutputReceipt = serde_json::from_str(&parse_flag(argv, "--receipt").context("Missing exact --receipt")?)?;
        let (participant, expected) = match &receipt {
            OutputReceipt::Unmanaged(token) => (&token.participant, token.through.0),
            OutputReceipt::Claimed(receipt) => (&receipt.target.participant, receipt.through.0),
        };
        if participant != &actor.participant || expected != through || through <= 0 {
            bail!("stale plugin output receipt");
        }
        output::confirm(client, &receipt, false, DeliveryEvidence::ToolOutputWritten)?;
        return Ok(json!({"acked_to": through}));
    }
    if has_flag(argv, "--check") {
        let inbox = crate::messaging::service::RemoteInbox::new(client.clone());
        use crate::messaging::inbox::PendingInbox;
        return Ok(json!(inbox.snapshot(&actor.participant)?.is_some()));
    }
    let Some(prepared) = output::prepare(client, &actor.participant)? else {
        return Ok(json!({"messages": [], "has_more": false}));
    };
    let after = crate::messaging::inbox::EventId(prepared.ack.last_event_id);
    let has_more = match client.call(Request::Messages(crate::messaging::service::Request::Inbox(crate::messaging::service::InboxOperation::HasPendingAfter { participant: actor.participant.clone(), after })))? {
        Response::Messages(crate::messaging::service::Response::Inbox(crate::messaging::service::InboxOutcome::HasPendingAfter(result))) => result?,
        _ => bail!("unexpected pending-after response"),
    };
    Ok(json!({"messages": prepared.messages, "has_more": has_more, "receipt": prepared.receipt}))
}

fn call(client: &Client, context: &AgentxContext, tool: &str, command: &str, argv: &[String], recover_soft_stopped: bool) -> Result<Value> {
    if command == "start" {
        return start(client, context, argv, tool, recover_soft_stopped);
    }
    let actor = resolve(client, argv)?;
    match command {
        "standby" => {
            let port = parse_flag(argv, "--port").context("Missing --port")?.parse()?;
            endpoint(client, &actor, port, true)?;
            Ok(json!({"name": actor.participant.canonical_name}))
        }
        "context" => {
            let session = parse_flag(argv, "--session-id").context("Missing --session-id")?;
            if actor.record.session_id.as_deref() != Some(session.as_str()) {
                bail!("context refresh requires the current provider session");
            }
            let instructions = crate::runtime::bootstrap::get_instructions_client(client, context, &actor.participant.canonical_name, &actor.record.tool)?;
            let text = crate::runtime::bootstrap::get_compact_context_client(client, context, &actor.participant.canonical_name, &actor.record.tool)?;
            let current = resolve(client, argv)?;
            if current.participant != actor.participant || current.record.session_id.as_deref() != Some(session.as_str()) {
                bail!("context attachment changed before output");
            }
            Ok(json!({"instructions": instructions, "context": text}))
        }
        "read" => read(client, &actor, argv),
        "status" | "beforetool" => {
            let observed_tool = parse_flag(argv, "--tool");
            let (status, status_context, detail) = if let Some(observed_tool) = observed_tool {
                let input = parse_flag(argv, "--input-json").and_then(|value| serde_json::from_str(&value).ok()).unwrap_or_else(|| json!({}));
                ("active".to_owned(), format!("tool:{observed_tool}"), super::tool_status::extract_tool_detail(&actor.record.tool, &observed_tool, &input))
            } else {
                (parse_flag(argv, "--status").context("Missing --status")?, parse_flag(argv, "--context").unwrap_or_default(), parse_flag(argv, "--detail").unwrap_or_default())
            };
            participant(client, ParticipantRequest::Status { participant: actor.participant, status, context: status_context, detail })?;
            Ok(json!({"ok": true}))
        }
        "stop" => {
            let soft = has_flag(argv, "--soft");
            let reason = parse_flag(argv, "--reason").unwrap_or_else(|| "unknown".to_owned());
            participant(client, ParticipantRequest::PluginStop { participant: actor.participant, reason, soft })?;
            Ok(if soft { json!({"ok": true, "soft": true}) } else { json!({"ok": true}) })
        }
        _ => bail!("Unknown {tool} hook: {command}"),
    }
}

pub(super) fn dispatch(tool: &str, hook_name: &str, argv: &[String], recover_soft_stopped: bool) -> (i32, String) {
    let context = AgentxContext::from_os();
    if !super::common::hook_gate_check(&context) {
        return (0, String::new());
    }
    let argv = argv.strip_prefix(&[hook_name.to_owned()]).unwrap_or(argv);
    let command = hook_name.strip_prefix(tool).and_then(|name| name.strip_prefix('-')).unwrap_or("");
    let mut invocation = crate::runtime::host_client::invocation(&context);
    if command == "start" {
        invocation.launch_context = Some(Value::Object(crate::agents::binding::capture_context_snapshot(&context.agentx_dir, &context.cwd, &context.raw_env)));
    }
    let result = Client::connect(&context.agentx_dir, invocation).map_err(anyhow::Error::from).and_then(|client| call(&client, &context, tool, command, argv, recover_soft_stopped));
    match result {
        Ok(output) => (0, output.to_string()),
        Err(error) => plugin_rpc::error(error),
    }
}
