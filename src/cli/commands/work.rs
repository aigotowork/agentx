//! Finite work/report CLI. This adapter resolves no domain state locally and
//! uses the daemon's existing participant authority and generation fences.
use crate::{
    collaboration::service as work,
    service::{InvocationContext, client::Client},
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "agentx work", about = "Read project work and report exact assigned collaboration facts", after_help = work::REPORT_GUIDANCE)]
struct Arguments {
    #[command(subcommand)]
    operation: Operation,
}
#[derive(clap::Args)]
struct Selection {
    #[arg(long)]
    scope: PathBuf,
    #[arg(long)]
    project: PathBuf,
    /// Existing participant claim, checked by the native identity resolver
    #[arg(long = "iam")]
    name: Option<String>,
}
#[derive(Subcommand)]
enum Operation {
    /// Read the current work, assignments, brief, evidence and eligibility
    Show {
        #[command(flatten)]
        selection: Selection,
        #[arg(long)]
        work: String,
        #[arg(long)]
        generation: Option<String>,
    },
    /// Read a finite page of work summaries
    List {
        #[command(flatten)]
        selection: Selection,
        #[arg(long, default_value_t = 32)]
        limit: u32,
        #[arg(long)]
        after: Option<String>,
    },
    /// Read exact current project participants without credential-bearing fields
    Candidates {
        #[command(flatten)]
        selection: Selection,
        #[arg(long, default_value_t = 32)]
        limit: u32,
        #[arg(long)]
        after: Option<String>,
    },
    /// Reconcile a retained command without resubmitting it
    Command {
        #[command(flatten)]
        selection: Selection,
        #[arg(long)]
        generation: String,
        #[arg(long)]
        command_id: String,
    },
    /// Report a native AgentReport JSON value for your exact dispatched assignment
    #[command(after_help = work::REPORT_GUIDANCE)]
    Report {
        #[command(flatten)]
        selection: Selection,
        #[arg(long)]
        work: String,
        #[arg(long)]
        generation: String,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        assignment: String,
        #[arg(long)]
        brief: u64,
        #[arg(long)]
        command_id: String,
        #[arg(long)]
        payload: String,
    },
    /// Apply one finite operator command JSON value with durable command identity
    Apply {
        #[command(flatten)]
        selection: Selection,
        #[arg(long)]
        work: Option<String>,
        #[arg(long)]
        generation: String,
        #[arg(long)]
        revision: Option<u64>,
        #[arg(long)]
        command_id: String,
        #[arg(long)]
        payload: String,
    },
}
pub(crate) fn help_text() -> String {
    help_command().render_long_help().to_string()
}
pub(crate) fn help_command() -> clap::Command {
    use clap::CommandFactory;
    Arguments::command()
}
pub(crate) fn dispatch(argv: &[String]) -> Option<i32> {
    let args = match argv.first().map(String::as_str) {
        Some("work") => argv[1..].to_vec(),
        Some("help") if argv.get(1).map(String::as_str) == Some("work") => vec!["--help".into()],
        _ => return None,
    };
    Some(match Arguments::try_parse_from(std::iter::once("agentx work".to_owned()).chain(args)) {
        Ok(arguments) => match execute(arguments.operation) {
            Ok(response) => {
                let rejected = matches!(response, work::Response::Rejected(_));
                match serde_json::to_string(&response) {
                    Ok(output) => {
                        println!("{output}");
                        if rejected { 2 } else { 0 }
                    }
                    Err(error) => {
                        eprintln!("agentx work: {error}");
                        1
                    }
                }
            }
            Err(error) => {
                eprintln!("agentx work: {error}");
                1
            }
        },
        Err(error) => {
            let code = error.exit_code();
            let _ = crate::cli::commands::help::print_clap_error(&error);
            code
        }
    })
}
fn execute(operation: Operation) -> anyhow::Result<work::Response> {
    let selection = match &operation {
        Operation::Show { selection, .. } | Operation::List { selection, .. } | Operation::Candidates { selection, .. } | Operation::Command { selection, .. } | Operation::Report { selection, .. } | Operation::Apply { selection, .. } => selection,
    };
    let scope = selection.scope.canonicalize()?;
    let project = selection.project.canonicalize()?;
    anyhow::ensure!(scope.is_dir() && project.is_dir(), "scope and project must be directories");
    let scope_text = scope.to_str().ok_or_else(|| anyhow::anyhow!("scope must be UTF-8"))?.to_owned();
    let project_text = project.to_str().ok_or_else(|| anyhow::anyhow!("project must be UTF-8"))?.to_owned();
    let mut invocation = InvocationContext::capture()?;
    invocation.cwd = project;
    invocation.explicit_name = selection.name.clone();
    let client = Client::connect(&scope, invocation)?;
    let generation = client.info().database_generation.clone();
    let reference = |id: String, generation: String| work::WorkRef { scope: scope_text.clone(), generation, project: project_text.clone(), id };
    let read = |generation, read| work::Request::Read { generation, project: project_text.clone(), read };
    let request = match operation {
        Operation::Show { work, generation: expected, .. } => {
            let generation = expected.unwrap_or(generation);
            read(generation.clone(), work::Read::Detail(reference(work, generation)))
        }
        Operation::List { limit, after, .. } => read(generation, work::Read::List(work::Page { limit, after })),
        Operation::Candidates { limit, after, .. } => read(generation, work::Read::Candidates(work::Page { limit, after })),
        Operation::Command { generation, command_id, .. } => read(generation, work::Read::Command { command_id }),
        Operation::Report { work, generation, revision, assignment, brief, command_id, payload, .. } => {
            anyhow::ensure!(payload.len() <= work::MAX_RECORD_BYTES, "payload too large");
            let report = serde_json::from_str::<work::AgentReport>(&payload)?;
            work::Request::Apply(work::CommandCall {
                command_id,
                generation: generation.clone(),
                project: project_text.clone(),
                work: Some(reference(work, generation)),
                expected_revision: Some(revision),
                command: work::Command::Report { assignment_id: assignment, brief_revision: brief, report },
            })
        }
        Operation::Apply { work, generation, revision, command_id, payload, .. } => {
            anyhow::ensure!(payload.len() <= work::MAX_RECORD_BYTES, "payload too large");
            let command = serde_json::from_str::<work::OperatorCommand>(&payload)?;
            work::Request::Apply(work::CommandCall { command_id, generation: generation.clone(), project: project_text.clone(), work: work.map(|id| reference(id, generation)), expected_revision: revision, command: work::Command::Operator(command) })
        }
    };
    match client.call(crate::service::Request::Collaboration(request))? {
        crate::service::Response::Collaboration(response) => Ok(response),
        _ => anyhow::bail!("unexpected native response"),
    }
}
