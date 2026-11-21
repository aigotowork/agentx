//! Select exact local Agents and submit one continuation per model error.
use crate::agents::{
    recovery::{Attempt, Candidate, Phase, Request, Response, Selection},
    service,
};
use crate::cli::{client::Session, router::GlobalFlags, workspace::AgentFilterArgs};
use crate::service::{Request as NativeRequest, Response as NativeResponse, ServiceError, ServiceResult};
use crate::tui::select::{self, Choice};
use anyhow::{Result, bail};
use clap::{CommandFactory, Parser};
use std::{
    collections::{BTreeMap, HashSet},
    io::IsTerminal,
};

#[derive(Debug, Parser)]
#[command(
    name = "recover",
    about = "Continue live Agents stalled by model request errors",
    after_help = "Example: agentx recover --dir . --label project=atlas\nWithout names or --all, choose and confirm eligible Agents interactively. --all submits across workspaces; it cannot be combined with a directory or workspace filter. A queued request is not confirmation of recovery."
)]
pub(crate) struct RecoverArgs {
    /// Exact local Agent names, optionally prefixed with @.
    #[arg(value_name = "@NAME", value_parser = exact_name)]
    targets: Vec<String>,
    /// Queue continuation for all eligible local Agents in this AgentX scope.
    #[arg(long, conflicts_with_all = ["targets", "dir", "workspace"])]
    all: bool,
    #[command(flatten)]
    filters: AgentFilterArgs,
    /// Print an array of per-Agent results; requires names or --all.
    #[arg(long)]
    json: bool,
}

#[derive(serde::Serialize)]
struct Outcome {
    agent: String,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

fn exact_name(raw: &str) -> Result<String, String> {
    let name = raw.strip_prefix('@').unwrap_or(raw);
    if name.len() > 128 || !crate::agents::identity::is_valid_name(name) {
        return Err("expected an exact local Agent name, optionally prefixed with @".into());
    }
    Ok(name.to_owned())
}

pub(crate) fn help_command() -> clap::Command {
    RecoverArgs::command()
}

fn call(session: &Session, request: Request) -> ServiceResult<Response> {
    match session.client.call(NativeRequest::Participants(service::Request::Recovery(request)))? {
        NativeResponse::Participants(service::Response::Recovery(response)) => Ok(response),
        _ => Err(ServiceError::Internal("unexpected recovery response".into())),
    }
}

fn choice(name: &str, candidate: &Candidate) -> Choice {
    let age = crate::shared::time::format_age(crate::shared::time::now_epoch_i64().saturating_sub(candidate.error_since));
    let detail = candidate.reason.lines().next().unwrap_or("request failed").chars().filter(|c| !c.is_control()).take(96).collect::<String>();
    Choice { value: name.into(), label: Some(format!("@{name} [Codex]")), description: format!("{age} · {detail}") }
}

fn outcome(agent: String, result: ServiceResult<Response>) -> Outcome {
    match result {
        Ok(Response::Accepted(Attempt { id, phase, replayed })) => {
            let (status, reason) = match phase {
                Phase::Queued if replayed => ("already_requested", "continuation already queued; recovery not confirmed"),
                Phase::Queued => ("queued", "waiting for the session to accept continuation; recovery not confirmed"),
                Phase::Issuing => ("in_progress", "continuation being submitted; recovery not confirmed"),
                Phase::Submitted => ("submitted", "continuation submitted; model recovery not confirmed"),
                Phase::Unknown => ("unknown", "continuation unconfirmed; this episode will not be automatically retried"),
            };
            Outcome { agent, status, request_id: Some(id), reason: Some(reason.into()) }
        }
        Err(error) => {
            let status = match error {
                ServiceError::OutcomeUnknown(_) => "unknown",
                ServiceError::StaleFence | ServiceError::Conflict(_) => "skipped",
                _ => "failed",
            };
            Outcome { agent, status, request_id: None, reason: Some(error.to_string()) }
        }
        _ => Outcome { agent, status: "failed", request_id: None, reason: Some("unexpected recovery result".into()) },
    }
}

pub(crate) fn run(args: RecoverArgs, flags: &GlobalFlags) -> Result<i32> {
    let interactive = args.targets.is_empty() && !args.all;
    if interactive && (args.json || !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal()) {
        bail!("agentx recover needs explicit @targets or --all in noninteractive and --json mode");
    }
    let labels = args.filters.label_predicates()?;
    let mut seen = HashSet::new();
    let names = args.targets.into_iter().filter(|name| seen.insert(name.clone())).collect::<Vec<_>>();
    let session = Session::connect(flags.name.as_deref(), flags.go)?;
    let workspace = args.filters.workspace(&session, interactive)?;
    let selection = Selection { labels, workspace: workspace.as_selector().map(str::to_owned), names };
    let Response::Candidates(candidates) = call(&session, Request::Select(selection.clone()))? else { bail!("unexpected recovery selection response") };
    let candidates = candidates.into_iter().map(|item| (item.target.participant.canonical_name.clone(), item)).collect::<BTreeMap<_, _>>();
    let selected = if interactive && !candidates.is_empty() {
        let choices = candidates.iter().map(|(name, candidate)| choice(name, candidate)).collect::<Vec<_>>();
        let Some(selected) = select::select_multiple(&format!("Select Agents to recover · {}", workspace.display()), &choices, "recover")? else { return Ok(130) };
        selected
    } else if selection.names.is_empty() {
        candidates.keys().cloned().collect()
    } else {
        selection.names.clone()
    };

    let results = selected
        .into_iter()
        .map(|name| {
            let result = match candidates.get(&name) {
                Some(candidate) => call(&session, Request::Submit { target: candidate.target.clone(), selection: Selection { names: vec![name.clone()], ..selection.clone() } }),
                None => Err(ServiceError::Conflict("not found, not eligible, or does not match the workspace and label filters".into())),
            };
            outcome(name, result)
        })
        .collect::<Vec<_>>();
    if args.json {
        println!("{}", serde_json::to_string(&results)?);
    } else if results.is_empty() {
        println!("No eligible Agents to recover in {}.", workspace.display());
    } else {
        for result in &results {
            println!("@{}: {} — {}", result.agent, result.status, result.reason.as_deref().unwrap_or_default());
        }
    }
    Ok(i32::from(results.iter().any(|result| matches!(result.status, "skipped" | "failed" | "unknown"))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::labels::LabelPredicate;

    #[test]
    fn recover_parses_exact_names_and_combines_label_filters() {
        let args = RecoverArgs::try_parse_from(["recover", "@luna", "recover", "--dir", ".", "--label", "project=atlas", "--label", "!blocked"]).unwrap();
        assert_eq!(args.targets, ["luna", "recover"]);
        assert_eq!(args.filters.dir.as_deref(), Some("."));
        assert_eq!(args.filters.label_predicates().unwrap(), [LabelPredicate::Eq { key: "project".into(), value: "atlas".into() }, LabelPredicate::Missing { key: "blocked".into() }]);
        for operands in [["recover", "@luna", "--all"], ["recover", "@luna:REMOTE", "--json"], ["recover", "@", "--json"]] {
            assert!(RecoverArgs::try_parse_from(operands).is_err());
        }
        assert!(RecoverArgs::try_parse_from(["recover", "--all", "--dir", "."]).is_err());
        assert!(RecoverArgs::try_parse_from(["recover", "--all", "--workspace", "global"]).is_err());
        assert!(RecoverArgs::try_parse_from(["recover", "--all", "--label", "project=atlas"]).is_ok());
    }
}
