//! Start participation through typed participant lifecycle operations.
use crate::agents::lifecycle_service::{Bootstrap, Request as LifecycleRequest, Response as LifecycleResponse, StartMode};
use crate::cli::router::GlobalFlags;
use crate::service::{InvocationContext, Request, Response};
use anyhow::{Result, bail};
/// Parsed arguments for `agentx start`.
#[derive(clap::Parser, Debug)]
#[command(name = "start", about = "Start agentx participation")]
pub struct StartArgs {
    /// Register using an instance name or a subagent ID
    #[arg(long, conflicts_with = "orphan")]
    pub name: Option<String>,
    /// Rebind to a different instance name
    #[arg(long = "as")]
    pub as_name: Option<String>,
    /// Recover orphaned PTY process by name or PID
    #[arg(long)]
    pub orphan: Option<String>,
}

/// Run the start command.
pub fn run(argv: &[String], flags: &GlobalFlags) -> Result<i32> {
    // Registration --name belongs to start; --iam is the separate caller claim.
    let (filtered, _, _) = crate::cli::router::extract_global_flags_full(argv);

    use clap::Parser;
    let start_args = match StartArgs::try_parse_from(&filtered) {
        Ok(a) => a,
        Err(e) => return Ok(crate::cli::commands::help::clap_usage_error("start", &e)),
    };

    let mode = match (start_args.as_name, start_args.orphan) {
        (Some(_), Some(_)) => bail!("--as and --orphan cannot be used together"),
        (Some(name), None) => StartMode::Reclaim { name },
        (None, Some(target)) => StartMode::RecoverOrphan { target },
        (None, None) => StartMode::Participate,
    };
    let mut invocation = InvocationContext::capture_for_launch()?;
    invocation.explicit_name = flags.name.clone();
    let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
    let runtime = crate::runtime::context::AgentxContext::from_env(&invocation.environment, invocation.cwd.clone()).with_tty(invocation.stdin_is_tty, invocation.stdout_is_tty);
    let client = crate::service::client::Client::connect(scope, invocation)?;
    let output = match client.call(Request::Participants(crate::agents::service::Request::Lifecycle(LifecycleRequest::Start { mode, requested_name: start_args.name })))? {
        Response::Participants(crate::agents::service::Response::Lifecycle(LifecycleResponse::Started(output))) => output,
        _ => bail!("daemon returned a different lifecycle response"),
    };
    for warning in &output.warnings {
        eprintln!("{warning}");
    }
    if let Some(title) = &output.terminal_title {
        crate::runtime::env::set_terminal_title(title);
    }
    let mut text = output.stdout.clone();
    if let (Some(participant), Some(bootstrap)) = (&output.participant, &output.bootstrap) {
        let name = &participant.canonical_name;
        match bootstrap {
            Bootstrap::Root { tool } => {
                let bootstrap = crate::runtime::bootstrap::get_start_bootstrap_client(&client, &runtime, participant, tool)?;
                text.push_str(&format!("[agentx:{name}]\n{bootstrap}\n[agentx:{name}]\n"));
            }
            Bootstrap::Subagent { parent } => {
                let bootstrap = crate::runtime::bootstrap::get_subagent_bootstrap_for_scope(name, parent, &runtime.agentx_dir, &runtime.raw_env);
                if !bootstrap.is_empty() {
                    text.push_str(&bootstrap);
                    text.push('\n');
                }
            }
        }
    }
    if let Some(target) = &output.activation_target {
        use crate::agents::service::{AttachmentOperation, AttachmentOutcome};
        match client.call(Request::Participants(crate::agents::service::Request::Attachments(AttachmentOperation::Validate { target: target.clone() })))? {
            Response::Participants(crate::agents::service::Response::Attachment(AttachmentOutcome::Valid(Ok(true)))) => {}
            _ => bail!("Provider session attachment changed before bootstrap output."),
        }
    }
    {
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(text.as_bytes())?;
        stdout.flush()?;
    }
    if let (Some(participant), Some(receipt)) = (output.participant, output.bootstrap_receipt) {
        match client.call(Request::Participants(crate::agents::service::Request::Lifecycle(LifecycleRequest::BootstrapFlushed { participant, receipt })))? {
            Response::Participants(crate::agents::service::Response::Lifecycle(LifecycleResponse::Changed)) => {}
            _ => bail!("daemon returned a different bootstrap confirmation"),
        }
    }
    Ok(output.exit_code)
}
