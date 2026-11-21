//! File-backed observations over typed native queries; independent from work.
use crate::service::{InvocationContext, Request, Response, client::Client, queries};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "agentx bindings", about = "Read file-owned Task/Delivery assignments and status (JSON)")]
struct Arguments {
    /// Bounded project .controller root; defaults to nearest project from caller cwd.
    #[arg(long)]
    project: Option<PathBuf>,
    /// Explicit file to observe, relative to caller cwd; repeatable, including outside .controller.
    #[arg(long = "file")]
    paths: Vec<PathBuf>,
    /// Rediscover and reread, including current errors and last successful snapshots.
    #[arg(long)]
    refresh: bool,
    /// Filter by this current Agent's exact participant, scope and data generation.
    #[arg(long)]
    agent: Option<String>,
    /// Explicit local data scope; defaults to the normal AGENTX_DIR selection.
    #[arg(long)]
    scope: Option<PathBuf>,
}
pub(crate) fn help_command() -> clap::Command {
    use clap::CommandFactory;
    Arguments::command()
}
pub(crate) fn dispatch(argv: &[String]) -> Option<i32> {
    let args = match argv.first().map(String::as_str) {
        Some("bindings") => argv[1..].to_vec(),
        Some("help") if argv.get(1).map(String::as_str) == Some("bindings") => vec!["--help".into()],
        _ => return None,
    };
    Some(match Arguments::try_parse_from(std::iter::once("agentx bindings".to_owned()).chain(args)) {
        Ok(args) => match execute(args) {
            Ok(projection) => {
                println!("{}", serde_json::to_string(&projection).expect("serializable file projection"));
                0
            }
            Err(error) => {
                eprintln!("agentx bindings: {error:#}");
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
fn execute(args: Arguments) -> anyhow::Result<crate::file_bindings::index::Projection> {
    let invocation = InvocationContext::capture()?;
    let scope = args.scope.unwrap_or_else(|| crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd).0);
    let query = crate::file_bindings::Query { project: args.project.unwrap_or_else(|| crate::file_bindings::index::project_root(&invocation.cwd)), paths: args.paths, refresh: args.refresh, agent: args.agent };
    let client = Client::connect(&scope, invocation)?;
    match client.call(Request::Queries(queries::Request::FileBindings(query)))? {
        Response::Queries(queries::Response::FileBindings(projection)) => Ok(projection),
        _ => anyhow::bail!("unexpected binding query response; use a daemon supporting file bindings"),
    }
}
