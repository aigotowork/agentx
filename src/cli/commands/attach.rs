//! Attach to an existing local tmux-backed Agent without changing its execution.
use crate::cli::workspace::AgentFilterArgs;
use clap::{CommandFactory, Parser};

#[derive(Parser, Debug)]
#[command(name = "attach", about = "Attach to a running Agent's tmux terminal", after_help = "Example: agentx attach --dir .\nShortcut: agentx a [target].\nDetach with Ctrl+B then D (tmux's default binding); the Agent keeps running.")]
pub(crate) struct AttachArgs {
    /// Exact Agent name; omit to choose a local tmux Agent in your workspace.
    #[arg(conflicts_with = "all")]
    pub target: Option<String>,
    #[command(flatten)]
    filters: AgentFilterArgs,
    /// Choose a terminal from all workspaces in this AgentX scope.
    #[arg(long, conflicts_with_all = ["dir", "workspace"])]
    all: bool,
}

pub(crate) fn help_command() -> clap::Command {
    AttachArgs::command()
}

fn participant(client: &crate::service::client::Client, name: &str) -> Result<crate::agents::attachments::ParticipantRef, String> {
    use crate::agents::service::{Request as Participants, Response as ParticipantResult};
    use crate::service::{Request, Response};
    let response = client.call(Request::Participants(Participants::Lookup { name: name.into() })).map_err(|error| error.to_string())?;
    let Response::Participants(ParticipantResult::Participant(participant)) = response else {
        return Err("daemon returned a different participant lookup response".into());
    };
    participant.ok_or_else(|| format!("Agent '{name}' has stopped"))
}

pub(crate) fn run(session: &crate::cli::client::Session, args: &AttachArgs) -> i32 {
    let result = (|| -> Result<i32, String> {
        let filter = args.filters.resolve(session, args.target.is_none() && !args.all).map_err(|error| error.to_string())?;
        let Some(target) = args.target.as_deref() else {
            let Some(selected) = super::target_selection::attach(session, &filter).map_err(|error| error.to_string())? else {
                return Ok(130);
            };
            if !filter.is_unfiltered() && !super::target_selection::attach_endpoints(session, Some(&selected.canonical_name), &filter).map_err(|error| error.to_string())?.iter().any(|endpoint| endpoint.participant == selected) {
                return Err("Selected Agent no longer matches the filters; select again".into());
            }
            return crate::runtime::terminal_client::attach(&session.client, &selected);
        };
        let target = target.strip_prefix('@').unwrap_or(target);
        let endpoint = if filter.is_unfiltered() {
            let participant = participant(&session.client, target)?;
            crate::runtime::terminal_client::endpoint(&session.client, &participant)?
        } else {
            super::target_selection::attach_endpoints(session, Some(target), &filter).map_err(|error| error.to_string())?.pop().ok_or_else(|| format!("Agent '{target}' not found or does not match the filters in {}", filter.workspace.display()))?
        };
        if crate::agents::records::is_remote_instance(&endpoint.record) {
            return Err("attach requires a local Agent; connect from its owning device".into());
        }
        crate::runtime::terminal_client::attach(&session.client, &endpoint.participant)
    })();
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn attach_uses_exact_daemon_fence_for_fractional_timestamp() {
        use crate::agents::service::{Request as P, Response as PR};
        use crate::service::{InvocationContext, Request, Response, client::Client};
        let timestamp = f64::from_bits(1_793_000_000.0f64.to_bits() + 7);
        let scope = tempfile::tempdir().unwrap();
        let path = scope.path().join("agentx.db");
        let db = crate::db::AgentxDb::open_at(&path).unwrap();
        db.conn().execute("INSERT INTO instances(name, tool, created_at) VALUES('neko', 'codex', ?1)", [timestamp]).unwrap();
        drop(db);
        let _daemon = crate::daemon::test_support::TestDaemon::start(scope.path(), &path);
        let client = Client::connect_existing(scope.path(), InvocationContext::default()).unwrap();
        let Response::Participants(PR::Records(records)) = client.call(Request::Participants(P::List)).unwrap() else { panic!("records") };
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "neko");
        let participant = super::participant(&client, "neko").unwrap();
        assert_eq!(participant.canonical_name, "neko");
        assert_eq!(participant.incarnation_bits, timestamp.to_bits());
        crate::runtime::terminal_client::endpoint(&client, &participant).unwrap();
    }
}
