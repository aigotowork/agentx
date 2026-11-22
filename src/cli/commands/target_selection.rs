//! Omitted-target CLI policy. Catalogs are read-only; native lifecycle operations
//! retain effect ownership and reject changed participants or stopped snapshots.

use crate::agents::attachments::ParticipantRef;
use crate::cli::client::Session;
use crate::db::InstanceRow;
use crate::service::{Request, Response, queries};
use crate::tui::select::{self, Choice};
use anyhow::{Result, bail};
use std::collections::HashSet;
use std::io::IsTerminal;

pub(super) fn require_terminal(command: &str, machine_output: bool) -> Result<()> {
    if machine_output || !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        bail!("agentx {command} needs an explicit target in noninteractive or --json mode; omit it in a terminal to select");
    }
    Ok(())
}

fn describe(row: &InstanceRow) -> String {
    let title = crate::agents::launch_title::from_context(row.launch_context.as_deref());
    format!("{} · {} · {}{}", row.status, row.tool, crate::shared::platform::shorten_path(&row.directory), title.map(|title| format!(" · {title}")).unwrap_or_default())
}

fn agent_choice(name: &str, context: Option<&str>, description: String) -> Choice {
    use crate::agents::workspaces::{CheckedCollaboration, checked_collaboration};
    let role = match checked_collaboration(context) {
        CheckedCollaboration::Valid(snapshot) => snapshot.role.map(|role| role.name),
        _ => None,
    };
    Choice { value: name.into(), label: Some(format!("{name} [{}]", role.as_deref().unwrap_or("-"))), description }
}

/// Read the exact integer fence and its matching current row in the same native
/// query path used by terminal consumers. Float JSON timestamps are not fences.
fn exact_record(session: &Session, name: &str) -> Result<Option<queries::TerminalEndpoint>> {
    use crate::agents::service::{Request as P, Response as PR};
    let Response::Participants(PR::Participant(participant)) = session.client.call(Request::Participants(P::Lookup { name: name.into() }))? else {
        bail!("unexpected participant lookup response");
    };
    let Some(participant) = participant else {
        return Ok(None);
    };
    match session.client.call(Request::Queries(queries::Request::Terminal { participant })) {
        Ok(Response::Queries(queries::Response::Terminal(mut endpoint))) => {
            endpoint.record.created_at = f64::from_bits(endpoint.participant.incarnation_bits);
            Ok(Some(endpoint))
        }
        Err(crate::service::ServiceError::StaleFence) => Ok(None),
        Err(error) => Err(error.into()),
        _ => bail!("unexpected terminal projection response"),
    }
}

pub(super) fn attach_endpoints(session: &Session, name: Option<&str>, filter: &queries::AgentFilter) -> Result<Vec<queries::TerminalEndpoint>> {
    let request = if name.is_none() && filter.is_unfiltered() { queries::Request::Terminals } else { queries::Request::MatchingTerminals { name: name.map(str::to_owned), filter: filter.clone() } };
    let Response::Queries(queries::Response::Terminals(endpoints)) = session.client.call(Request::Queries(request))? else {
        bail!("unexpected terminal inventory response");
    };
    Ok(endpoints)
}

pub(super) fn attach(session: &Session, filter: &queries::AgentFilter) -> Result<Option<ParticipantRef>> {
    require_terminal("attach", false)?;
    let endpoints = attach_endpoints(session, None, filter)?
        .into_iter()
        .filter(|endpoint| {
            let row = &endpoint.record;
            if crate::agents::records::is_remote_instance(row) || row.pid.is_none_or(|pid| pid <= 0 || row.pid_reused(pid as u32) || !crate::sys::process::is_alive(pid as u32)) {
                return false;
            }
            let info = crate::runtime::terminal::resolve_terminal_info(row.terminal_preset_effective.as_deref(), row.launch_context.as_deref());
            matches!(info.preset_name.as_str(), "tmux" | "tmux-split") && crate::runtime::tmux::attach_command(&info).is_ok()
        })
        .collect::<Vec<_>>();
    let scope = filter.workspace.display();
    if endpoints.is_empty() && filter.is_unfiltered() {
        bail!("No attachable local tmux Agents; launch with --headless --terminal tmux");
    }
    if endpoints.is_empty() {
        bail!("No attachable local tmux Agents match the filters in {scope}");
    }
    let choices = endpoints.iter().map(|endpoint| agent_choice(&endpoint.record.name, endpoint.record.launch_context.as_deref(), describe(&endpoint.record))).collect::<Vec<_>>();
    let Some(name) = select::select(&format!("Select Agent to attach · {scope}"), &choices)? else {
        return Ok(None);
    };
    Ok(endpoints.into_iter().find(|endpoint| endpoint.record.name == name).map(|endpoint| endpoint.participant))
}

fn local_tracked(row: &InstanceRow) -> bool {
    row.pid.is_some_and(|pid| pid > 0) && !crate::agents::records::is_remote_instance(row)
}

fn filtered_kill_candidates(session: &Session, filter: &queries::AgentFilter) -> Result<Vec<InstanceRow>> {
    use crate::agents::{labels, service::Request as Participants, service::Response as ParticipantResult};
    let mut candidates = Vec::new();
    let mut after = None;
    loop {
        let query = labels::LabelQuery::visible_agents(&filter.labels, filter.workspace.as_selector(), after);
        let Response::Participants(ParticipantResult::Labels(labels::Response::Page(page))) = session.client.call(Request::Participants(Participants::Labels(labels::Request::Select(query))))? else {
            bail!("unexpected filtered Agent inventory response");
        };
        for mut row in page.records.into_iter().filter(local_tracked) {
            // Rebuild the row's exact fence from the native integer, not the wire float.
            let snapshot = page.snapshots.get(&row.name).ok_or_else(|| anyhow::anyhow!("missing exact identity for filtered Agent '{}'", row.name))?;
            row.created_at = f64::from_bits(snapshot.participant.incarnation_bits);
            candidates.push(row);
        }
        after = page.next;
        if after.is_none() {
            return Ok(candidates);
        }
    }
}

pub(super) fn kill(session: &Session, filter: &queries::AgentFilter, records: &mut Vec<InstanceRow>) -> Result<Option<Vec<String>>> {
    let candidates = if filter.is_unfiltered() {
        let mut candidates = Vec::new();
        for row in session.records()?.iter().filter(|row| local_tracked(row)) {
            if let Some(endpoint) = exact_record(session, &row.name)?
                && local_tracked(&endpoint.record)
            {
                candidates.push(endpoint.record);
            }
        }
        candidates
    } else {
        filtered_kill_candidates(session, filter)?
    };
    if candidates.is_empty() {
        bail!("No local Agents with tracked processes match in {}", filter.workspace.display());
    }
    let choices = candidates.iter().map(|row| agent_choice(&row.name, row.launch_context.as_deref(), describe(row))).collect::<Vec<_>>();
    let selected = select::select_multiple(&format!("Select Agents to kill · {}", filter.workspace.display()), &choices, "kill")?;
    // The existing kill path consumes these frozen rows and their exact fences.
    // It never re-resolves the selected names after the picker returns.
    *records = candidates;
    Ok(selected)
}

pub(super) struct ResumeTarget {
    pub name: String,
    pub expected: Option<ParticipantRef>,
    stopped_event: Option<i64>,
    session_id: String,
}

impl ResumeTarget {
    pub(super) fn validate(&self, source: Option<&crate::agents::lifecycle_service::ResumeFence>) -> Result<()> {
        let Some(source) = source else { bail!("selected resume source is missing") };
        if source.session_id != self.session_id {
            bail!("Selected session changed; run the command again to choose its current session");
        }
        if let Some(event) = self.stopped_event
            && (source.stopped_event != Some(event) || source.name.as_deref() != Some(self.name.as_str()))
        {
            bail!("Selected stopped session changed; run the command again to choose its current session");
        }
        Ok(())
    }
}

pub(super) fn resume(session: &Session, fork: bool) -> Result<Option<ResumeTarget>> {
    let records = session.records()?;
    let mut targets = Vec::new();
    let mut choices = Vec::new();
    if fork {
        for row in records.iter().filter(|row| !crate::agents::records::is_remote_instance(row) && row.session_id.as_deref().is_some_and(|session| !session.is_empty()) && supports(&row.tool, true)) {
            if let Some(endpoint) = exact_record(session, &row.name)? {
                let current = endpoint.record;
                if current.session_id.as_deref().is_none_or(str::is_empty) || !supports(&current.tool, true) {
                    continue;
                }
                choices.push(agent_choice(&current.name, current.launch_context.as_deref(), describe(&current)));
                targets.push(ResumeTarget { name: current.name, expected: Some(endpoint.participant), stopped_event: None, session_id: current.session_id.unwrap_or_default() });
            }
        }
    }
    // The bounded native history query includes the stop event ID. It supports
    // every Provider session format and lets us reject a newer same-name stop.
    let query = crate::events::service::EventQuery { filters: [("type".into(), vec!["life".into()]), ("action".into(), vec!["stopped".into()])].into(), limit: crate::events::service::MAX_PAGE_EVENTS, ..Default::default() };
    let Response::Events(crate::events::service::Response::Page(page)) = session.client.call(Request::Events(crate::events::service::Request::Query(query)))? else {
        bail!("unexpected stopped history response");
    };
    append_stopped(&records, &page.events, fork, &mut targets, &mut choices);
    if choices.is_empty() {
        bail!("No eligible {} sessions; inspect agentx list --stopped or pass an explicit target", if fork { "fork" } else { "resume" });
    }
    let Some(name) = select::select(if fork { "Select session to fork" } else { "Select session to resume" }, &choices)? else {
        return Ok(None);
    };
    Ok(targets.into_iter().find(|target| target.name == name))
}

fn supports(tool: &str, fork: bool) -> bool {
    crate::runtime::launcher::LaunchTool::from_str(tool).is_ok_and(|tool| tool.spec().resume.is_some_and(|resume| !fork || resume.fork.is_some()))
}

fn append_stopped(records: &[InstanceRow], events: &[serde_json::Value], fork: bool, targets: &mut Vec<ResumeTarget>, choices: &mut Vec<Choice>) {
    let mut seen: HashSet<_> = records.iter().map(|row| row.name.clone()).collect();
    for event in events {
        let Some(name) = event["instance"].as_str() else {
            continue;
        };
        if !seen.insert(name.to_owned()) {
            continue;
        }
        let snapshot = &event["data"]["snapshot"];
        let Some(id) = event["id"].as_i64() else {
            continue;
        };
        let tool = snapshot["tool"].as_str().unwrap_or_default();
        if snapshot["session_id"].as_str().is_none_or(str::is_empty) || !supports(tool, fork) {
            continue;
        }
        choices.push(agent_choice(
            name,
            snapshot["launch_context"].as_str(),
            format!("stopped · {tool} · {} · {}", event["ts"].as_str().or_else(|| event["timestamp"].as_str()).unwrap_or_default(), crate::shared::platform::shorten_path(snapshot["directory"].as_str().unwrap_or_default())),
        ));
        targets.push(ResumeTarget { name: name.into(), expected: None, stopped_event: Some(id), session_id: snapshot["session_id"].as_str().unwrap_or_default().into() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn selection_stopped_candidates_keep_latest_event_and_tool_capabilities() {
        let event = |id, name, tool, session| json!({"id":id,"instance":name,"data":{"snapshot":{"tool":tool,"session_id":session,"directory":"/tmp"}}});
        let events = vec![event(8, "neko", "codex", "provider-specific-id"), event(7, "neko", "codex", "older-id"), event(6, "luna", "gemini", "session"), event(5, "mira", "adhoc", "session"), event(4, "taro", "codex", "")];
        let (mut targets, mut choices) = (Vec::new(), Vec::new());
        append_stopped(&[], &events, true, &mut targets, &mut choices);
        assert_eq!(choices.iter().map(|choice| choice.value.as_str()).collect::<Vec<_>>(), vec!["neko"]);
        assert_eq!(targets[0].stopped_event, Some(8));
        let (mut resume_targets, mut resume_choices) = (Vec::new(), Vec::new());
        append_stopped(&[], &events, false, &mut resume_targets, &mut resume_choices);
        assert_eq!(resume_choices.iter().map(|choice| choice.value.as_str()).collect::<Vec<_>>(), vec!["neko", "luna"]);
    }

    #[test]
    fn selection_stopped_session_rejects_a_newer_same_name_snapshot() {
        let target = ResumeTarget { name: "neko".into(), expected: None, stopped_event: Some(8), session_id: "session".into() };
        let mut source =
            crate::agents::lifecycle_service::ResumeFence { name: Some("neko".into()), participant: None, stopped_event: Some(8), latest_stopped_event: Some(8), snapshot_digest: None, session_id: "session".into(), fork: false, child: None };
        target.validate(Some(&source)).unwrap();
        source.stopped_event = Some(9);
        assert!(target.validate(Some(&source)).is_err());
        source.stopped_event = Some(8);
        source.session_id = "new-provider-session".into();
        assert!(target.validate(Some(&source)).is_err());
    }
}
