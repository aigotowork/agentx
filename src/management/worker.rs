//! Isolated management host. Only a successful journal claim permits runtime effects.
use super::contract::*;
use crate::agents::attachments::ParticipantRef;
use crate::runtime::{launch_profile, launcher, resume_client};
use crate::service::{InvocationContext, Request as NativeRequest, Response as NativeResponse, client::Client};
use std::io::Read;

const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_RESULT_BYTES: usize = 1024 * 1024;

/// Internal same-binary entry; run before ordinary CLI/environment initialization.
pub(crate) fn dispatch(argv: &[String]) -> Option<i32> {
    if argv.first().map(String::as_str) != Some("__management-worker") {
        return None;
    }
    Some(match run(argv) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("management worker: {}", bounded(error, 4096));
            1
        }
    })
}

fn run(argv: &[String]) -> Result<(), String> {
    if argv.len() != 1 {
        return Err("management worker takes input only through stdin".into());
    }
    let mut bytes = Vec::new();
    std::io::stdin().take((MAX_INPUT_BYTES + 1) as u64).read_to_end(&mut bytes).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err("management worker input exceeds limit".into());
    }
    let input: WorkerInput = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    establish_host(&input.invocation)?;
    // The operation is an originating host with an explicit scope and cwd.
    // Apply the ordinary dotenv contract only in this isolated process.
    crate::runtime::environment::prepare(&["daemon".into(), "web".into(), "--scope".into(), input.scope.clone()]).map_err(|error| error.to_string())?;
    crate::config::Config::init();
    let mut invocation = InvocationContext::capture().map_err(|error| error.to_string())?;
    invocation.environment_file_keys.extend(input.invocation.environment_file_keys);
    invocation.launch_context = input.invocation.launch_context;
    let scope = std::path::Path::new(&input.scope).canonicalize().map_err(|error| error.to_string())?;
    let (resolved, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
    if resolved.canonicalize().map_err(|error| error.to_string())? != scope {
        return Err("management worker invocation scope mismatch".into());
    }
    let client = Client::connect_existing_fenced(&scope, invocation, &input.generation).map_err(|error| error.to_string())?;
    if client.scope_root() != scope {
        return Err("management worker scope changed".into());
    }
    // Never retry a claim or execute following a failed/lost claim response.
    let command = match call(&client, Request::Claim { request_id: input.request_id.clone(), ticket: input.ticket.clone() })? {
        Response::Claimed(command) => command,
        _ => return Err("unexpected management claim response".into()),
    };
    let result = execute_with_id(&client, &command, &input.request_id)
        .and_then(|result| if serde_json::to_vec(&result).map_err(|error| error.to_string())?.len() > MAX_RESULT_BYTES { Err("management result exceeds limit".into()) } else { Ok(result) })
        .map_err(|error| bounded(error, 4096));
    crate::log::log_in_scope(
        &scope,
        if result.is_err() { "ERROR" } else { "INFO" },
        "management",
        if result.is_err() { "operation.failed" } else { "operation.finished" },
        if result.is_err() { "Execution failed; inspect the original operation receipt for the domain error" } else { "Execution completed" },
        &[("generation", &input.generation), ("operation_id", &input.request_id)],
    );
    match call(&client, Request::Finish { request_id: input.request_id, ticket: input.ticket, result })? {
        Response::Operation(_) => Ok(()),
        _ => Err("unexpected management finish response".into()),
    }
}

/// This process has no other threads yet. Validate before changing its frozen host inputs.
fn establish_host(invocation: &InvocationContext) -> Result<(), String> {
    if !invocation.cwd.is_absolute() || !invocation.cwd.is_dir() {
        return Err("management worker needs an existing absolute working directory".into());
    }
    if invocation.environment.iter().any(|(key, value)| key.is_empty() || key.contains(['=', '\0']) || value.contains('\0')) {
        return Err("invalid management worker environment".into());
    }
    std::env::set_current_dir(&invocation.cwd).map_err(|error| error.to_string())?;
    // SAFETY: dispatch is an early internal entry in a dedicated worker process,
    // before Client capture, daemon connection or any runtime thread is created.
    unsafe {
        for (key, _) in std::env::vars_os() {
            std::env::remove_var(key);
        }
        for (key, value) in &invocation.environment {
            std::env::set_var(key, value);
        }
    }
    Ok(())
}

fn call(client: &Client, request: Request) -> Result<Response, String> {
    match client.call(NativeRequest::Management(request)).map_err(native_error)? {
        NativeResponse::Management(response) => Ok(response),
        _ => Err("unexpected management response".into()),
    }
}

fn execute_with_id(client: &Client, command: &Command, request_id: &str) -> Result<CommandResult, String> {
    match command {
        Command::Launch(input) => launch(client, input),
        Command::PatchLabels(_) => Err("labels patches execute in the native transaction without a worker".into()),
        Command::Send { agent, message } => send(client, agent, message, request_id).map(CommandResult::Sent),
        Command::DirectorySend { directory, message } => directory_send(client, directory, message, request_id),
        Command::Stop(agent) => {
            let participant = participant(agent)?;
            let result = crate::runtime::kill_client::kill(client, &participant, "bigboss").map_err(unknown)?;
            use crate::runtime::terminal::KillResult;
            let outcome = match result.kill_result {
                KillResult::Sent => "sent",
                KillResult::AlreadyDead => "already_dead",
                KillResult::PermissionDenied => return Err("permission denied stopping Agent".into()),
            };
            Ok(CommandResult::Stopped(StopOutcome { name: result.target, outcome: outcome.into() }))
        }
        Command::StopParticipation(agent) => {
            let outcome = crate::runtime::stop_effects::retire(client, &participant(agent)?, "bigboss", &crate::agents::hook_service::OperatorStop::Participation { reason: "management".into() }).map_err(native_error)?;
            Ok(CommandResult::Stopped(StopOutcome { name: agent.name.clone(), outcome: if outcome.retained { "participation_retained" } else { "participation_stopped" }.into() }))
        }
        Command::Fork(agent) => resume(client, agent, true),
        Command::Resume(agent) => resume(client, agent, false),
        Command::Transcript(agent) => transcript(client, agent),
    }
}

fn participant(agent: &AgentRef) -> Result<ParticipantRef, String> {
    if agent.stopped_event.is_some() {
        return Err("operation requires a live Agent reference".into());
    }
    let incarnation_bits = agent.incarnation.as_deref().ok_or("Agent incarnation required")?.parse::<u64>().map_err(|_| "invalid Agent incarnation")?;
    Ok(ParticipantRef { canonical_name: agent.name.clone(), incarnation_bits })
}

/// Share the durable native Send fence between direct and directory commands.
fn send(client: &Client, agent: &AgentRef, message: &MessageInput, request_id: &str) -> Result<SendOutcome, String> {
    for id in &message.attachments {
        let Response::Attachment(stored) = call(client, Request::Attachment(id.clone()))? else {
            return Err("unexpected attachment response".into());
        };
        super::uploads::check(&stored).map_err(|error| error.to_string())?;
    }
    let caller = crate::service::CallerContext {
        request_id: crate::service::RequestId(request_id.into()),
        invocation: client.invocation().clone(),
        deadline_unix_ms: None,
        expected_database_generation: Some(client.generation().map_err(native_error)?.into()),
    };
    let request = NativeRequest::Management(Request::Send { agent: agent.clone(), message: message.clone() });
    match client.call_with_context(caller, request).map_err(native_error)? {
        NativeResponse::Management(Response::Sent(result)) => Ok(result),
        _ => Err(unknown("unexpected fenced message response")),
    }
}

/// The launch host must load dotenv from the admitted operation cwd before Claim.
fn check_directory(client: &Client, directory: &str) -> Result<(), String> {
    let requested = std::path::Path::new(directory);
    if !requested.is_absolute() || !requested.is_dir() {
        return Err("operation directory must be an existing absolute directory".into());
    }
    let requested = requested.canonicalize().map_err(|error| error.to_string())?;
    let frozen = client.invocation().cwd.canonicalize().map_err(|error| error.to_string())?;
    if requested != frozen {
        return Err("operation directory differs from the frozen worker environment cwd".into());
    }
    Ok(())
}

fn launch(client: &Client, input: &LaunchInput) -> Result<CommandResult, String> {
    let invocation = client.invocation();
    if input.role.is_some() && (input.tool.is_some() || input.profile.is_some()) {
        return Err("Role conflicts with explicit tool or profile".into());
    }
    // The HTTP adapter already resolved LaunchInput.directory against its project.
    // establish_host validated this frozen absolute cwd; resolving the input again
    // here would reject relative paths or apply their relative segments twice.
    let cwd = invocation.cwd.to_string_lossy();
    let flags = crate::runtime::launch_options::AgentxLaunchFlags { dir: Some(cwd.to_string()), workspace: input.workspace.clone(), role: input.role.clone(), ..Default::default() };
    let selection = launch_profile::LaunchSelection { profile: input.profile.clone(), ..Default::default() };
    let (collaboration, resolved) = launch_profile::prepare_with_role(client.scope_root(), &invocation.cwd, &invocation.environment, &flags, input.tool.as_deref(), &selection, &[]).map_err(|error| format!("{error:#}"))?;
    let tool = launcher::LaunchTool::from_str(&resolved.tool).map_err(|error| error.to_string())?;
    let (args, background) = launcher::prepare_resolved_execution(&tool, &resolved.args, true);
    let mut context = crate::runtime::context::AgentxContext::from_env(&invocation.environment, invocation.cwd.clone()).with_tty(invocation.stdin_is_tty, invocation.stdout_is_tty);
    context.agentx_dir = client.scope_root().to_path_buf();
    context.environment_file_keys = invocation.environment_file_keys.clone();
    let params = launcher::LaunchParams {
        tool: resolved.tool,
        count: 1,
        args,
        persisted_args: Some(resolved.args),
        launch_profile: resolved.profile,
        collaboration: Some(collaboration),
        background,
        cwd: Some(cwd.into_owned()),
        title: input.title.clone(),
        labels: input.labels.clone(),
        initial_prompt: input.prompt.clone(),
        launcher: Some("bigboss".into()),
        run_here: Some(false),
        ..Default::default()
    };
    launcher::launch_with_client(params, &context, client).map(launch_result).map_err(unknown)
}

/// Resolve configuration before endpoint effects, then retain exact native identity
/// through the same fenced Send used by a directly selected management recipient.
fn directory_send(client: &Client, directory: &str, message: &MessageInput, request_id: &str) -> Result<CommandResult, String> {
    check_directory(client, directory)?;
    let mut entry = super::offices::entry(client, directory).map_err(native_error)?;
    let selected = crate::config::roles::select_workspace(&client.invocation().cwd, &client.invocation().environment, Some(directory), None).map_err(|error| error.to_string())?;
    let (mut address, _) = crate::runtime::directory::resolve(client, std::path::Path::new(directory), selected.as_deref()).map_err(directory_error)?;
    // Preview is observational. Refuse a changed target before lookup/adoption or
    // launch, and retain the shared runtime's frozen full Role for this execution.
    let role = &address.role;
    if entry.directory != address.requested_directory
        || entry.requested_workspace != address.requested_workspace
        || entry.resolved_workspace != address.resolved_workspace
        || entry.role.path != role.path.to_string_lossy()
        || entry.role.name != role.name
        || entry.role.namespace != role.namespace
        || entry.role.profile != role.profile
        || entry.role.description != role.description
        || entry.role.default != role.default
    {
        return Err("directory configuration changed during resolution; inspect the route again".into());
    }
    let exact = crate::runtime::directory::recipient(client, &mut address, "bigboss").map_err(directory_error)?;
    let recipient = AgentRef { name: exact.canonical_name, incarnation: Some(exact.incarnation_bits.to_string()), stopped_event: None };
    // EndpointLookup/Reserve established this exact binding; do not re-resolve
    // mutable config after side effects merely to create a presentation receipt.
    entry.endpoint = EndpointView { status: "bound".into(), participant: Some(recipient.clone()), candidates: 1, reason: None };
    let sent = send(client, &recipient, message, request_id).map_err(|error| if address.created && !error.starts_with("outcome_unknown:") { unknown(error) } else { error })?;
    Ok(CommandResult::DirectorySent(DirectorySendOutcome { entry, recipient, sent, created: address.created }))
}

fn directory_error(error: anyhow::Error) -> String {
    match error.downcast_ref::<crate::service::ServiceError>() {
        Some(crate::service::ServiceError::OutcomeUnknown(_)) => unknown(error),
        _ => error.to_string(),
    }
}

fn resume(client: &Client, agent: &AgentRef, fork: bool) -> Result<CommandResult, String> {
    let Response::Agent(view) = call(client, Request::Agent(agent.clone()))? else {
        return Err("unexpected resume Agent response".into());
    };
    if if fork { !view.can_fork } else { !view.can_resume } {
        return Err(if fork { "Agent cannot fork its session" } else { "Agent cannot be resumed" }.into());
    }
    let expected = agent.incarnation.as_ref().map(|_| participant(agent)).transpose()?;
    // Resolve stopped history by its session, never by a possibly reused name.
    let target = if agent.stopped_event.is_some() { view.session_id.as_deref().ok_or("stopped Agent has no session")? } else { &agent.name };
    let plan = resume_client::prepare(client, target, expected.as_ref(), fork, &["--headless".into()], Some("bigboss")).map_err(unknown)?;
    if let Some(event) = &agent.stopped_event {
        let event = event.parse::<i64>().map_err(|_| "invalid stopped event")?;
        if plan.source.as_ref().and_then(|source| source.stopped_event) != Some(event) {
            return Err("stopped resume source changed".into());
        }
    }
    resume_client::execute(client, &plan).map(launch_result).map_err(unknown)
}

pub(super) fn launch_result(result: launcher::LaunchResult) -> CommandResult {
    let agents: Vec<_> = result.handles.iter().filter_map(|handle| handle.get("instance_name").and_then(serde_json::Value::as_str).map(str::to_owned)).collect();
    let participants = result.launched_participants;
    let exact = !participants.is_empty() && participants.len() == result.launched && participants.len() == agents.len() && participants.iter().zip(&agents).all(|(participant, name)| &participant.canonical_name == name);
    CommandResult::Launched(LaunchOutcome {
        agents,
        failed: result.failed.min(u32::MAX as usize) as u32,
        errors: result.errors.iter().take(MAX_PAGE as usize).map(|error| bounded(error.get("error").and_then(serde_json::Value::as_str).unwrap_or("Agent launch failed").into(), 4096)).collect(),
        agent_refs: exact.then(|| participants.into_iter().map(|participant| AgentRef { name: participant.canonical_name, incarnation: Some(participant.incarnation_bits.to_string()), stopped_event: None }).collect()),
    })
}

fn transcript(client: &Client, agent: &AgentRef) -> Result<CommandResult, String> {
    let Response::TranscriptSource(selected) = call(client, Request::TranscriptSource(agent.clone()))? else {
        return Err("unexpected transcript source response".into());
    };
    let Some(source) = selected.source else {
        return Ok(CommandResult::Transcript(TranscriptPage { agent: agent.clone(), available: false, reason: Some(bounded(selected.missing.unwrap_or_else(|| "No model transcript is registered".into()), 4096)), exchanges: Vec::new() }));
    };
    let loaded = crate::transcript::client::read_source(&source, MAX_PAGE as usize, false, false)?;
    let mut exchanges = Vec::new();
    let mut bytes = 0;
    let mut truncated = false;
    for exchange in loaded.exchanges.into_iter().take(MAX_PAGE as usize) {
        let view = ExchangeView {
            position: exchange.position.to_string(),
            at: bounded(exchange.timestamp, 128),
            user: bounded(exchange.user, MAX_TEXT),
            assistant: bounded(exchange.final_text, MAX_TEXT),
            action: bounded(exchange.action, MAX_TEXT),
            files: exchange.files.into_iter().take(MAX_PAGE as usize).map(|file| bounded(file, 4096)).collect(),
        };
        bytes += serde_json::to_vec(&view).map_err(|error| error.to_string())?.len();
        if bytes > MAX_RESULT_BYTES - MAX_TEXT {
            truncated = true;
            break;
        }
        exchanges.push(view);
    }
    let reason = if truncated {
        Some("Transcript result reached management size limit".into())
    } else if exchanges.is_empty() {
        Some("No exchanges found".into())
    } else {
        None
    };
    Ok(CommandResult::Transcript(TranscriptPage { agent: agent.clone(), available: true, reason, exchanges }))
}

fn bounded(mut text: String, limit: usize) -> String {
    if text.len() > limit {
        let mut end = limit;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

fn unknown(error: impl std::fmt::Display) -> String {
    format!("outcome_unknown: {error}")
}

fn native_error(error: crate::service::ServiceError) -> String {
    match error {
        crate::service::ServiceError::OutcomeUnknown(_) => unknown(error),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::directory_error;
    use crate::service::ServiceError;

    #[test]
    fn directory_errors_preserve_uncertain_outcomes_and_definite_refusals() {
        let lost = anyhow::Error::new(ServiceError::OutcomeUnknown("endpoint reply lost".into())).context("directory lookup");
        assert!(directory_error(lost).starts_with("outcome_unknown:"));
        let changed = ServiceError::Conflict("workspace endpoint role changed".into());
        assert!(!directory_error(changed.into()).starts_with("outcome_unknown:"));
    }
}
