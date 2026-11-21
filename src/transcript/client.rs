//! Host transcript reads from one daemon-selected source; no managed SQLite.
use super::service::{Selection, Source};
use super::{self as transcript, Exchange, ReadOptions, format_transcript_exchanges};
use crate::agents::attachments::ParticipantRef;
use crate::service::{Request, Response, client::Client, queries};
use serde_json::{Value, json};
use std::path::Path;

pub(crate) fn selection(client: &Client, name: &str, expected: Option<&ParticipantRef>) -> Result<Selection, String> {
    match client.call(Request::Queries(queries::Request::TranscriptSource { name: name.into(), expected: expected.cloned() })).map_err(|error| error.to_string())? {
        Response::Queries(queries::Response::TranscriptSource(selection)) => Ok(selection),
        _ => Err("daemon returned a different transcript source".into()),
    }
}
pub(crate) fn render(client: &Client, name: &str, opts: &RenderOptions<'_>) -> Result<String, String> {
    render_origin(client, name, name, None, opts)
}
pub(crate) fn render_participant(client: &Client, participant: &ParticipantRef, opts: &RenderOptions<'_>) -> Result<String, String> {
    let selected = selection(client, &participant.canonical_name, Some(participant))?;
    render_selection(selected, &participant.canonical_name, None, opts)
}
pub(crate) fn render_origin(client: &Client, name: &str, display_name: &str, device: Option<&str>, opts: &RenderOptions<'_>) -> Result<String, String> {
    render_selection(selection(client, name, None)?, display_name, device, opts)
}
pub(crate) fn render_selection(mut selected: Selection, display_name: &str, device: Option<&str>, opts: &RenderOptions<'_>) -> Result<String, String> {
    if device.is_none()
        && let Some(lookup) = selected.codex_lookup.take()
        && selected.source.as_ref().is_none_or(|source| !Path::new(&source.path).is_file())
        && let Some(path) = crate::integrations::hooks::codex::find_transcript_in_home(&lookup.home, &lookup.session)
    {
        selected.source = Some(Source { name: selected.canonical_name.clone(), path, tool: "codex".into(), session_id: Some(lookup.session) });
    }
    let Some(source) = selected.source else {
        if let Some(missing) = selected.missing {
            return Err(missing);
        }
        let name = selected.canonical_name;
        let display_name = if display_name.is_empty() { &name } else { display_name };
        let command = match device {
            Some(device) => format!("agentx events --remote-fetch --device {device} --participant {name} --type message"),
            None => format!("agentx events --participant {name} --type message"),
        };
        return Err(format!("No model transcript is registered for {display_name}.\nView transport messages with: {command}"));
    };
    render_source(&source, opts)
}

pub(crate) fn inventory(client: &Client, live_only: bool) -> Result<Vec<super::service::TrackedSource>, String> {
    match client.call(Request::Queries(queries::Request::TranscriptInventory { live_only })).map_err(|error| error.to_string())? {
        Response::Queries(queries::Response::TranscriptInventory(sources)) => Ok(sources),
        _ => Err("daemon returned a different transcript inventory".into()),
    }
}

pub(crate) fn correlate(client: &Client, targets: &[(String, Option<String>)]) -> Result<std::collections::HashMap<String, String>, String> {
    match client.call(Request::Queries(queries::Request::TranscriptCorrelations { targets: targets.to_vec() })).map_err(|error| error.to_string())? {
        Response::Queries(queries::Response::TranscriptCorrelations(sources)) => Ok(sources),
        _ => Err("daemon returned a different transcript correlation".into()),
    }
}
pub(crate) fn read_source(source: &Source, last: usize, detailed: bool, retry_codex: bool) -> Result<transcript::Transcript, String> {
    let backend = transcript::backend_from_agent_or_path(&source.tool, &source.path)?;
    transcript::read(Path::new(&source.path), backend, &ReadOptions { last, detailed, session_id: source.session_id.clone(), allow_codex_retry: retry_codex })
}
pub struct RenderOptions<'a> {
    pub range: Option<&'a str>,
    pub last_n: usize,
    pub json_mode: bool,
    pub full_mode: bool,
    pub detailed: bool,
    pub retry_codex: bool,
}

impl Default for RenderOptions<'_> {
    fn default() -> Self {
        Self { range: None, last_n: 10, json_mode: false, full_mode: false, detailed: false, retry_codex: true }
    }
}

fn conversation_header(shown: &[&Exchange], loaded: &[Exchange], instance_name: &str) -> String {
    let first = shown.first().map_or(0, |e| e.position);
    let last = shown.last().map_or(0, |e| e.position);
    let total = loaded.iter().map(|e| e.position).max().unwrap_or(0);
    let count = shown.len();
    let noun = if count == 1 { "exchange" } else { "exchanges" };
    let span = if first == last { first.to_string() } else { format!("{first}-{last}") };
    format!("Recent conversation ({count} {noun}, {span} of {total}) - @{instance_name}:")
}

/// Explain an empty range result against the exchanges that do exist.
fn range_miss_message(exchanges: &[Exchange], range: Option<ExchangeRange>) -> Option<String> {
    let (start, end) = range?;
    let last = exchanges.iter().map(|e| e.position).max()?;
    let asked = match end {
        Some(end) if end == start => start.to_string(),
        Some(end) => format!("{start}-{end}"),
        None => format!("{start}-"),
    };
    Some(format!("No exchanges in range {asked}; this transcript has 1-{last} (e.g. --last 5 or {}-{last})", last.saturating_sub(4).max(1)))
}

type ExchangeRange = (usize, Option<usize>);

/// Parse "N", "N-M" or "N-" (N to the end). Positions start at 1.
fn parse_range(s: &str) -> Result<ExchangeRange, String> {
    let invalid = || format!("invalid range '{s}' (expected N, N-M or N-)");
    let position = |v: &str| v.parse::<usize>().ok().filter(|&v| v >= 1);
    match s.split_once('-') {
        None => position(s).map(|n| (n, Some(n))).ok_or_else(invalid),
        Some((start, "")) => position(start).map(|n| (n, None)).ok_or_else(invalid),
        Some((start, end)) => match (position(start), position(end)) {
            (Some(start), Some(end)) if start <= end => Ok((start, Some(end))),
            _ => Err(invalid()),
        },
    }
}

fn render_source(source: &Source, opts: &RenderOptions<'_>) -> Result<String, String> {
    let range = opts.range.map(parse_range).transpose()?;
    let effective_last = if range.is_some() { usize::MAX } else { opts.last_n };
    let exchanges = read_source(source, effective_last, opts.detailed, opts.retry_codex)?.exchanges;

    let filtered: Vec<&Exchange> = match range {
        Some((start, end)) => exchanges.iter().filter(|e| e.position >= start && end.is_none_or(|end| e.position <= end)).collect(),
        None => exchanges.iter().collect(),
    };

    if filtered.is_empty()
        && let Some(msg) = range_miss_message(&exchanges, range)
    {
        return Err(msg);
    }

    if opts.json_mode {
        let json_output: Vec<Value> = filtered
            .iter()
            .map(|ex| {
                let mut obj = json!({
                    "position": ex.position,
                    "user": ex.user,
                    "action": ex.action,
                    "files": ex.files,
                    "timestamp": ex.timestamp,
                });
                if opts.detailed {
                    obj["usage"] = json!(ex.usage);
                    obj["tools"] = json!(
                        ex.tools
                            .iter()
                            .map(|t| {
                                let mut tool = json!({
                                    "name": t.name,
                                    "is_error": t.is_error,
                                });
                                if let Some(ref f) = t.file {
                                    tool["file"] = json!(f);
                                }
                                if let Some(ref c) = t.command {
                                    tool["command"] = json!(c);
                                }
                                tool
                            })
                            .collect::<Vec<_>>()
                    );
                    obj["edits"] = json!(ex.edits);
                    obj["errors"] = json!(ex.errors);
                    obj["ended_on_error"] = json!(ex.ended_on_error);
                }
                obj
            })
            .collect();
        return serde_json::to_string(&json_output).map_err(|e| e.to_string());
    }

    if filtered.is_empty() {
        return Ok("No exchanges found".to_string());
    }

    let header = conversation_header(&filtered, &exchanges, &source.name);
    let owned: Vec<Exchange> = filtered.into_iter().cloned().collect();
    let mut formatted = format_transcript_exchanges(&owned, opts.full_mode, opts.detailed);
    if let Some(hint) = transcript::flag_hint(opts.full_mode, opts.detailed) {
        formatted.push('\n');
        formatted.push_str(hint);
    }
    Ok(format!("{header}\n\n{formatted}"))
}
