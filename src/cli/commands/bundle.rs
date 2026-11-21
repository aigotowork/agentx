//! `agentx bundle` command — structured context sharing.
//!
//!
//! Subcommands: list, show, cat, chain, prepare/preview, create.

use std::path::Path;
use std::time::SystemTime;

use serde_json::{Value, json};

use crate::cli::client::Session;
#[cfg(test)]
use crate::db::AgentxDb;
use crate::messaging::bundle_service::{Category, Page, Request as BundleRequest, Response as BundleResponse};
use crate::messaging::bundles;
use crate::service::{Request, Response, ServiceError};

#[cfg(test)]
use crate::cli::commands::transcript::exact_instance_transcript;
use crate::cli::commands::transcript::get_exchanges;
use crate::transcript::{Exchange, format_exchanges};

/// Parsed arguments for `agentx bundle`.
///
/// Uses manual subcommand routing to support:
/// - `agentx bundle` → list (default)
/// - `agentx bundle --json --last 5` → list with flags
/// - `agentx bundle <id>` → implicit show
/// - `agentx bundle <subcmd> ...` → explicit subcommand
#[derive(clap::Parser, Debug)]
#[command(name = "bundle", about = "Manage context bundles")]
pub struct BundleArgs {
    /// All arguments (subcommand routing is manual due to implicit show/list)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(clap::Parser, Debug)]
pub struct BundleListArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub last: Option<usize>,
}

#[derive(clap::Parser, Debug)]
pub struct BundleShowArgs {
    /// Bundle ID (event ID or bundle: prefix)
    pub id: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Parser, Debug)]
pub struct BundleCatArgs {
    /// Bundle ID
    pub id: String,
}

#[derive(clap::Parser, Debug)]
pub struct BundleChainArgs {
    /// Starting bundle ID
    pub id: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Parser, Debug)]
pub struct BundlePrepareArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub compact: bool,
    /// Target agent (default: self)
    #[arg(long = "for")]
    pub for_agent: Option<String>,
    /// Last N transcript exchanges (default: 40)
    #[arg(long = "last-transcript", default_value = "40")]
    pub last_transcript: usize,
    /// Last N events (default: 10)
    #[arg(long = "last-events", default_value = "10")]
    pub last_events: usize,
}

#[derive(clap::Parser, Debug)]
pub struct BundleCreateArgs {
    /// Bundle title (positional)
    pub title_positional: Option<String>,
    /// Bundle title (flag form, overrides positional)
    #[arg(long = "title")]
    pub title_flag: Option<String>,
    #[arg(long)]
    pub json: bool,
    /// Raw bundle JSON
    #[arg(long = "bundle")]
    pub bundle_json: Option<String>,
    /// Path to bundle JSON file
    #[arg(long = "bundle-file")]
    pub bundle_file: Option<String>,
    /// Bundle description
    #[arg(long)]
    pub description: Option<String>,
    /// Comma-separated event IDs
    #[arg(long)]
    pub events: Option<String>,
    /// Comma-separated file paths
    #[arg(long)]
    pub files: Option<String>,
    /// Transcript ranges (e.g., "3-14:normal,6:full")
    #[arg(long)]
    pub transcript: Option<String>,
    /// Parent bundle ID to extend
    #[arg(long)]
    pub extends: Option<String>,
}

// ── Bundle Lookup ────────────────────────────────────────────────────────

fn call(session: &Session, request: BundleRequest) -> Result<BundleResponse, String> {
    match session.client.call(Request::Messages(crate::messaging::service::Request::Bundle(request))).map_err(|error| error.to_string())? {
        Response::Messages(crate::messaging::service::Response::Bundle(response)) => Ok(response),
        _ => Err("daemon returned a different bundle response".into()),
    }
}
fn get_bundle_by_id(session: &Session, id: &str) -> Result<Option<Value>, String> {
    match call(session, BundleRequest::Lookup { id: id.into() })? {
        BundleResponse::Lookup(bundle) => Ok(bundle),
        _ => Err("daemon returned a different bundle lookup".into()),
    }
}

/// Drain bounded pages without changing the requested count or event snapshot.
fn pages(session: &Session, limit: usize, request: impl FnMut(Option<i64>, Option<i64>, usize) -> BundleRequest) -> Result<Vec<Value>, String> {
    let mut result = Vec::new();
    visit_pages(session, limit, request, |page| {
        result.extend(page);
        Ok(())
    })?;
    Ok(result)
}
fn visit_pages(session: &Session, limit: usize, mut request: impl FnMut(Option<i64>, Option<i64>, usize) -> BundleRequest, mut visit: impl FnMut(Vec<Value>) -> Result<(), String>) -> Result<(), String> {
    let (mut cursor, mut through) = (None, None);
    let mut count = 0;
    while count < limit {
        let BundleResponse::Page(Page { events: page, through: watermark, next, has_more }) = call(session, request(cursor, through, limit - count))? else {
            return Err("daemon returned a different bundle page".into());
        };
        if has_more && (page.is_empty() || next == cursor) {
            return Err("bundle page did not advance".into());
        }
        through = Some(watermark);
        cursor = next;
        count += page.len();
        visit(page)?;
        if !has_more {
            break;
        }
    }
    Ok(())
}

// ── Subcommands ──────────────────────────────────────────────────────────

/// List bundles: `agentx bundle [list] [--last N] [--json]`
fn cmd_bundle_list(session: &Session, args: &BundleListArgs) -> Result<i32, String> {
    let json_mode = args.json;
    let last_n = args.last.unwrap_or(20);

    let rows: Vec<(i64, String, String, String)> = pages(session, last_n, |before, through, limit| BundleRequest::List { before, through, limit })?
        .into_iter()
        .map(|event| (event["id"].as_i64().unwrap_or_default(), event["timestamp"].as_str().unwrap_or_default().to_owned(), event["instance"].as_str().unwrap_or_default().to_owned(), event["data"].to_string()))
        .collect();

    // JSON mode outputs even when empty (prints "[]")
    if json_mode {
        let mut bundles: Vec<Value> = Vec::new();
        for (id, ts, _inst, data_str) in &rows {
            let data: Value = serde_json::from_str(data_str).unwrap_or(json!({}));
            let events_val = data.get("refs").and_then(|r| r.get("events")).cloned().unwrap_or(json!([]));
            bundles.push(json!({
                "id": id,
                "timestamp": ts,
                "bundle_id": data.get("bundle_id").and_then(|v| v.as_str()).unwrap_or(""),
                "title": data.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                "description": data.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                "created_by": data.get("created_by").and_then(|v| v.as_str()).unwrap_or(""),
                "events": events_val,
            }));
        }
        println!("{}", serde_json::to_string(&bundles).unwrap_or_default());
        return Ok(0);
    }

    if rows.is_empty() {
        println!("No bundles found. Create one with: agentx bundle prepare");
        return Ok(0);
    }

    // Human-readable table
    let now_secs = crate::shared::time::now_epoch_i64();

    println!("{:<12} {:<30} {:<12} AGE", "BUNDLE_ID", "TITLE", "BY");
    for (_id, ts, _inst, data_str) in &rows {
        let data: Value = serde_json::from_str(data_str).unwrap_or(json!({}));
        let bundle_id = data.get("bundle_id").and_then(|v| v.as_str()).unwrap_or("");
        let title = data.get("title").and_then(|v| v.as_str()).unwrap_or("(untitled)");
        let created_by = data.get("created_by").and_then(|v| v.as_str()).unwrap_or("");

        // Parse timestamp for age
        let age = if let Ok(created) = chrono::DateTime::parse_from_rfc3339(ts).or_else(|_| chrono::DateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S%.f%z")) {
            let age_secs = now_secs - created.timestamp();
            format_age(age_secs)
        } else {
            "?".to_string()
        };

        // Truncate display
        let short_id = if bundle_id.len() > 12 { &bundle_id[..12] } else { bundle_id };
        let short_title = if title.len() > 28 {
            let mut end = 25;
            while end > 0 && !title.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &title[..end])
        } else {
            title.to_string()
        };

        println!("{short_id:<12} {short_title:<30} {created_by:<12} {age}");
    }

    Ok(0)
}

/// Show bundle: `agentx bundle show <id> [--json]`
fn cmd_bundle_show(session: &Session, args: &BundleShowArgs) -> Result<i32, String> {
    let json_mode = args.json;

    let bundle = match get_bundle_by_id(session, &args.id)? {
        Some(b) => b,
        None => {
            eprintln!("Error: Bundle not found: {}", args.id);
            return Ok(1);
        }
    };

    if json_mode {
        println!("{}", serde_json::to_string(&bundle).unwrap_or_default());
    } else {
        let title = bundle.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let desc = bundle.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let bundle_id = bundle.get("bundle_id").and_then(|v| v.as_str()).unwrap_or("");
        let created_by = bundle.get("created_by").and_then(|v| v.as_str()).unwrap_or("");

        println!("Bundle: {bundle_id}");
        println!("Title: {title}");
        println!("By: {created_by}");
        println!("Description: {desc}");

        if let Some(refs) = bundle.get("refs") {
            if let Some(events) = refs.get("events").and_then(|v| v.as_array()) {
                println!("\nEvents: {} referenced", events.len());
            }
            if let Some(files) = refs.get("files").and_then(|v| v.as_array()) {
                println!("Files: {} referenced", files.len());
            }
            if let Some(transcript) = refs.get("transcript").and_then(|v| v.as_array()) {
                println!("Transcript: {} ranges", transcript.len());
            }
        }

        if let Some(extends) = bundle.get("extends").and_then(|v| v.as_str()) {
            println!("\nExtends: {extends}");
        }
    }

    Ok(0)
}

/// Cat bundle (expand full content): `agentx bundle cat <id>`
fn cmd_bundle_cat(session: &Session, args: &BundleCatArgs) -> Result<i32, String> {
    let bundle = match get_bundle_by_id(session, &args.id)? {
        Some(b) => b,
        None => {
            eprintln!("Error: Bundle not found: {}", args.id);
            return Ok(1);
        }
    };

    let title = bundle.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let desc = bundle.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let bundle_id = bundle.get("bundle_id").and_then(|v| v.as_str()).unwrap_or("");
    let created_by = bundle.get("created_by").and_then(|v| v.as_str()).unwrap_or("");

    println!("=== Bundle: {bundle_id} ===");
    println!("Title: {title}");
    println!("By: {created_by}");
    println!("Description: {desc}");

    if let Some(refs) = bundle.get("refs") {
        // Files section (KB/MB size + "modified X ago")
        if let Some(files) = refs.get("files").and_then(|v| v.as_array()) {
            let sep = "━".repeat(80);
            println!("\n{sep}");
            println!("FILES ({})", files.len());
            println!("{sep}\n");
            for file_val in files {
                let path = file_val.as_str().unwrap_or("");
                if path.is_empty() {
                    continue;
                }
                let p = session.invocation.cwd.join(path);
                if p.exists() {
                    let meta = std::fs::metadata(&p).ok();
                    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                    let lines = std::fs::read_to_string(&p).map(|c| c.lines().count()).unwrap_or(0);
                    let size_str = if size < 1024 {
                        format!("{size} B")
                    } else if size < 1024 * 1024 {
                        format!("{:.1} KB", size as f64 / 1024.0)
                    } else {
                        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                    };
                    let modified_str = meta
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .and_then(|mt| SystemTime::now().duration_since(mt).ok())
                        .map(|d| {
                            let secs = d.as_secs() as i64;
                            if secs < 3600 {
                                format!("{}m ago", secs / 60)
                            } else if secs < 86400 {
                                format!("{}h ago", secs / 3600)
                            } else {
                                format!("{}d ago", secs / 86400)
                            }
                        })
                        .unwrap_or_default();
                    println!("{path} ({lines} lines, {size_str}, modified {modified_str})");
                } else {
                    println!("{path} (not found)");
                }
            }
            println!();
        }

        // Events section — full JSON output, supports ID ranges like "100-105"
        if let Some(events) = refs.get("events").and_then(|v| v.as_array()) {
            let sep = "━".repeat(80);
            println!("\n{sep}");
            println!("EVENTS ({})", events.len());
            println!("{sep}\n");

            visit_pages(
                session,
                usize::MAX,
                |after, through, limit| BundleRequest::ReferencedEvents { refs: events.clone(), after, through, limit },
                |page| {
                    for event in page {
                        println!("{}\n", serde_json::to_string_pretty(&event).map_err(|error| error.to_string())?);
                    }
                    Ok(())
                },
            )?;
        }

        if let Some(transcript) = refs.get("transcript").and_then(|v| v.as_array()) {
            let sep = "━".repeat(80);
            println!("\n{sep}");
            println!("TRANSCRIPT ({} entries)", transcript.len());
            println!("{sep}\n");
            print_bundle_transcript(session, created_by, transcript)?;
        }
    }

    Ok(0)
}

/// Render each `range:detail` transcript ref against the creator's transcript.
fn print_bundle_transcript(session: &Session, created_by: &str, refs: &[Value]) -> Result<(), String> {
    let Some(source) = crate::transcript::client::selection(&session.client, created_by, None)?.source else {
        println!("Transcript unavailable: agent '{created_by}' not found or has no transcript");
        return Ok(());
    };
    let (path, tool, session_id) = (source.path, source.tool, source.session_id);
    if !Path::new(&path).exists() {
        println!("Transcript unavailable: {path} not found");
        return Ok(());
    }

    // Parsed once per detail mode: [normal/full, detailed].
    let mut cache: [Option<Result<Vec<Exchange>, String>>; 2] = [None, None];
    for tref in refs {
        let ref_data = match bundles::parse_transcript_ref(tref) {
            Ok(r) => r,
            Err(e) => {
                println!("  (invalid ref: {e})");
                continue;
            }
        };
        let range = ref_data.get("range").and_then(|v| v.as_str()).unwrap_or("");
        let detail = ref_data.get("detail").and_then(|v| v.as_str()).unwrap_or("normal");
        let full = detail == "full" || detail == "detailed";
        let detailed = detail == "detailed";

        println!("--- Transcript [{range}] ({detail}) ---\n");

        let Some((start, end)) = parse_exchange_range(range) else {
            println!("  (invalid range: {range})\n");
            continue;
        };
        let exchanges = cache[detailed as usize].get_or_insert_with(|| get_exchanges(&path, &tool, usize::MAX, detailed, session_id.as_deref(), true));
        match exchanges {
            Ok(all) => {
                let selected: Vec<Exchange> = all.iter().filter(|e| e.position >= start && e.position <= end).cloned().collect();
                if selected.is_empty() {
                    println!("  (no exchanges in range; transcript has {} exchanges)\n", all.len());
                } else {
                    println!("{}", format_exchanges(&selected, full, detailed));
                }
            }
            Err(e) => println!("Error reading transcript: {e}\n"),
        }
    }
    Ok(())
}

/// Parse `N` or `N-M` (1-based, inclusive).
fn parse_exchange_range(range: &str) -> Option<(usize, usize)> {
    let (start, end) = match range.split_once('-') {
        Some((s, e)) => (s.trim().parse().ok()?, e.trim().parse().ok()?),
        None => {
            let pos = range.trim().parse().ok()?;
            (pos, pos)
        }
    };
    (start >= 1 && start <= end).then_some((start, end))
}

/// Chain: `agentx bundle chain <id> [--json]`
fn cmd_bundle_chain(session: &Session, args: &BundleChainArgs) -> Result<i32, String> {
    let json_mode = args.json;

    let mut chain = Vec::new();
    let mut current_id = args.id.clone();
    let mut seen = std::collections::HashSet::new();

    // First bundle must exist
    let first = match get_bundle_by_id(session, &current_id)? {
        Some(b) => b,
        None => {
            eprintln!("Error: Bundle not found: {}", args.id);
            return Ok(1);
        }
    };

    let bid = first.get("bundle_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    seen.insert(bid.clone());
    chain.push(first.clone());

    // Walk up the chain
    if let Some(parent_id) = first.get("extends").and_then(|v| v.as_str()) {
        current_id = parent_id.to_string();
        loop {
            let bundle = match get_bundle_by_id(session, &current_id)? {
                Some(b) => b,
                None => {
                    // Warn about missing ancestor
                    eprintln!("Warning: missing ancestor bundle {current_id}");
                    break;
                }
            };

            let bid = bundle.get("bundle_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if seen.contains(&bid) {
                break; // Cycle detection
            }
            seen.insert(bid.clone());
            chain.push(bundle.clone());

            match bundle.get("extends").and_then(|v| v.as_str()) {
                Some(parent) => current_id = parent.to_string(),
                None => break,
            }
        }
    }

    if json_mode {
        println!("{}", serde_json::to_string(&chain).unwrap_or_default());
        return Ok(0);
    }

    println!("Bundle chain ({} levels):", chain.len());
    for (i, bundle) in chain.iter().enumerate() {
        let bid = bundle.get("bundle_id").and_then(|v| v.as_str()).unwrap_or("");
        let title = bundle.get("title").and_then(|v| v.as_str()).unwrap_or("(untitled)");
        let indent = "  ".repeat(i);
        let marker = if i == 0 { "→" } else { "↳" };
        println!("{indent}{marker} {bid}: {title}");
    }

    Ok(0)
}

/// One prepare event category: (display label, JSON key, rows `(id, timestamp, instance, data)`).
type EventCategory = (&'static str, &'static str, Vec<(i64, String, String, Value)>);

/// Recent events touching `agent`, newest first, `limit` per category.
fn bundle_event_categories(session: &Session, agent: &str, limit: usize) -> Result<Vec<EventCategory>, String> {
    [("Messages to", "messages_to", Category::MessagesTo), ("Messages from", "messages_from", Category::MessagesFrom), ("File operations", "file_operations", Category::FileOperations), ("Lifecycle", "lifecycle", Category::Lifecycle)]
        .into_iter()
        .map(|(label, key, category)| {
            let rows = pages(session, limit, |before, through, limit| BundleRequest::Context { agent: agent.into(), category, before, through, limit })?
                .into_iter()
                .map(|event| (event["id"].as_i64().unwrap_or_default(), event["timestamp"].as_str().unwrap_or_default().into(), event["instance"].as_str().unwrap_or_default().into(), event["data"].clone()))
                .collect();
            Ok((label, key, rows))
        })
        .collect()
}

/// Prepare: `agentx bundle prepare [--for AGENT] [--last-transcript N] [--last-events N] [--compact] [--json]`
fn cmd_bundle_prepare(session: &Session, args: &BundlePrepareArgs) -> Result<i32, String> {
    let last_transcript = args.last_transcript;
    let last_events = args.last_events;

    // The caller creates the bundle, whichever agent's context it covers.
    let caller = session.actor.as_ref().map(|actor| actor.record.name.clone());
    let agent = match args.for_agent.clone().or_else(|| caller.clone()) {
        Some(name) => name,
        None => {
            eprintln!("Error: No agent context. Use --for <agent> to specify.");
            return Ok(1);
        }
    };

    let selected = crate::transcript::client::selection(&session.client, &agent, None)?;
    let agent = selected.canonical_name;
    let mut transcript_text: Option<String> = None;
    let mut transcript_range: Option<String> = None;
    if let Some(source) = selected.source {
        let (path, tool, session_id) = (source.path, source.tool, source.session_id);
        match get_exchanges(&path, &tool, last_transcript, false, session_id.as_deref(), true) {
            Ok(exchanges) => {
                if let (Some(first), Some(last)) = (exchanges.first(), exchanges.last()) {
                    transcript_range = Some(if first.position == last.position { first.position.to_string() } else { format!("{}-{}", first.position, last.position) });
                    transcript_text = Some(format_exchanges(&exchanges, false, false));
                }
            }
            Err(e) => transcript_text = Some(format!("Error reading transcript: {e}")),
        }
    }

    let categories = bundle_event_categories(session, &agent, last_events)?;

    let mut event_ids: Vec<i64> = categories.iter().flat_map(|(_, _, rows)| rows.iter().map(|(id, ..)| *id)).collect();
    event_ids.sort_unstable_by(|a, b| b.cmp(a));
    event_ids.dedup();

    let mut files: Vec<String> = categories
        .iter()
        .flat_map(|(_, _, rows)| rows.iter())
        .filter_map(|(.., data)| data.get("detail").and_then(|v| v.as_str()))
        .filter(|detail| detail.contains('/') || [".py", ".ts", ".js", ".md", ".json", ".rs"].iter().any(|ext| detail.ends_with(ext)))
        .map(str::to_string)
        .collect();
    files.sort();
    files.dedup();

    // Template command (events capped at 20, files at 10)
    let mut template_parts = vec![match caller {
        Some(ref name) => format!("agentx bundle create \"Bundle Title Here\" --iam {name}"),
        None => "agentx bundle create \"Bundle Title Here\"".to_string(),
    }];
    template_parts.push("--description \"detailed description text here\"".to_string());
    if let Some(ref range) = transcript_range {
        template_parts.push(format!("--transcript \"{range}:normal\""));
    }
    if !event_ids.is_empty() {
        let ids: Vec<String> = event_ids.iter().take(20).map(i64::to_string).collect();
        template_parts.push(format!("--events \"{}\"", ids.join(",")));
    }
    if !files.is_empty() {
        let sample: Vec<&str> = files.iter().take(10).map(String::as_str).collect();
        template_parts.push(format!("--files \"{}\"", sample.join(",")));
    }
    let template_command = template_parts.join(" \\\n  ");

    if args.json {
        let events_json: serde_json::Map<String, Value> = categories
            .iter()
            .map(|(_, key, rows)| {
                let events: Vec<Value> = rows.iter().map(|(id, ts, inst, data)| json!({"id": id, "timestamp": ts, "instance": inst, "data": data})).collect();
                (key.to_string(), json!(events))
            })
            .collect();
        let result = json!({
            "agent": agent,
            "transcript": {
                "text": transcript_text,
                "range": transcript_range,
            },
            "events": events_json,
            "files": files,
            "template_command": template_command,
            "note": format!("Last {last_transcript} transcript exchanges, {last_events} events per category"),
        });
        println!("{}", serde_json::to_string(&result).unwrap_or_default());
        return Ok(0);
    }

    let sep = "─".repeat(40);
    println!("[Bundle Context: {agent}]\n");

    if !args.compact {
        println!("{sep}");
        println!("HOW TO USE THIS CONTEXT:\n");
        println!(
            "Edit the CREATE command below into your bundle, or pass the same flags to \
'agentx send' (with --title) to create and send in one step."
        );
        println!("Transcript refs: range:detail, comma-separated (e.g. 3-14:normal,6:full,22-30:detailed)");
        println!("  normal = truncated | full = complete text | detailed = full + tool I/O, edits, errors\n");
        println!("- Pick the relevant transcript ranges, events, and files from the context below");
        println!("- Use agentx transcript and agentx events to find anything else relevant");
        println!(
            "- Give each transcript range the detail it needs \
(full when the whole text matters, normal only when the summary below is enough)"
        );
        println!(
            "- Description: be comprehensive and precise. Explain what the bundle contains, \
summarise the referenced transcript ranges and events, and give enough insight for another agent \
to understand everything you know: what happened, decisions, current state, issues, plans.\n"
        );
        println!("A good bundle includes everything relevant and nothing irrelevant.\n");
        println!("View: agentx transcript {agent} [N-M] [--full|--detailed]");
        println!("View: agentx events --agent {agent} [--last N]\n");
        println!("Use agentx bundle prepare --compact to hide this section\n");
    }

    println!("{sep}");
    println!("TRANSCRIPT");
    println!("{}\n", transcript_text.as_deref().unwrap_or("(none)"));

    for (label, _, rows) in &categories {
        if rows.is_empty() {
            continue;
        }
        println!("--- {label} ({} events) ---", rows.len());
        for (id, ts, inst, data) in rows {
            let ts_short = ts.get(..19).unwrap_or(ts);
            println!("  #{id} | {inst} @ {ts_short} | {}", format_event_summary(data));
        }
        println!();
    }

    if !files.is_empty() {
        println!("{sep}");
        println!("FILES ({})", files.len());
        for f in files.iter().take(20) {
            // Show relative-ish path (last 3 components)
            let parts: Vec<&str> = f.split('/').collect();
            let short = if parts.len() > 3 { parts[parts.len() - 3..].join("/") } else { f.clone() };
            println!("  {short}");
        }
        if files.len() > 20 {
            println!("  +{} more", files.len() - 20);
        }
        println!();
    }

    println!("{sep}");
    println!("CREATE:");
    println!("{template_command}");

    Ok(0)
}

/// Create: `agentx bundle create [TITLE] --description DESC [--events LIST] [--files LIST] [--transcript RANGES] [--extends ID] [--json]`
fn cmd_bundle_create(session: &Session, args: &BundleCreateArgs) -> Result<i32, String> {
    let json_mode = args.json;

    // Mutual exclusion: --bundle and --bundle-file
    if args.bundle_json.is_some() && args.bundle_file.is_some() {
        eprintln!("Error: --bundle and --bundle-file are mutually exclusive");
        return Ok(1);
    }

    // Raw bundle mode
    let raw = match (&args.bundle_json, &args.bundle_file) {
        (Some(raw), _) => Some(raw.clone()),
        (_, Some(path)) => Some(std::fs::read_to_string(session.invocation.cwd.join(path)).map_err(|error| format!("Cannot read bundle file {path}: {error}"))?),
        _ => None,
    };
    if let Some(raw) = raw {
        let mut bundle: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error: Invalid JSON: {e}");
                return Ok(1);
            }
        };
        return create_and_log_bundle(session, &mut bundle, json_mode);
    }

    // Inline bundle creation mode
    let title = match args.title_flag.as_ref().or(args.title_positional.as_ref()) {
        Some(t) => t.clone(),
        None => {
            return Ok(crate::cli::commands::help::usage_error("bundle create", "agentx bundle create needs a title"));
        }
    };

    let description = match &args.description {
        Some(d) => d.clone(),
        None => {
            eprintln!("Error: --description is required");
            return Ok(1);
        }
    };

    // Build bundle data
    let events_list = bundles::parse_csv_list(args.events.as_deref());
    let files_list = bundles::parse_csv_list(args.files.as_deref());

    let transcript_refs: Vec<Value> = if let Some(ref t) = args.transcript {
        t.split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    return None;
                }
                bundles::parse_transcript_ref(&json!(s)).ok()
            })
            .collect()
    } else {
        vec![]
    };

    let mut bundle = json!({
        "title": title,
        "description": description,
        "refs": {
            "events": events_list.iter().map(|e| json!(e)).collect::<Vec<_>>(),
            "files": files_list.iter().map(|f| json!(f)).collect::<Vec<_>>(),
            "transcript": transcript_refs,
        },
    });

    if let Some(ref ext) = args.extends {
        bundle["extends"] = json!(ext);
    }

    create_and_log_bundle(session, &mut bundle, json_mode)
}

/// Validate, create, and log a bundle event.
fn create_and_log_bundle(session: &Session, bundle: &mut Value, json_mode: bool) -> Result<i32, String> {
    bundles::validate_bundle(bundle)?;
    bundles::warn_missing_files_for_cwd(bundle, &session.invocation.cwd);
    let request = Request::Messages(crate::messaging::service::Request::Bundle(BundleRequest::Create { bundle: bundle.clone(), actor: session.actor.as_ref().map(|actor| actor.participant.clone()) }));
    let caller = crate::service::CallerContext {
        request_id: crate::service::RequestId(uuid::Uuid::new_v4().to_string()),
        invocation: session.invocation.clone(),
        deadline_unix_ms: None,
        expected_database_generation: Some(session.client.info().database_generation.clone()),
    };
    let response = match session.client.call_with_context(caller.clone(), request.clone()) {
        Err(ServiceError::OutcomeUnknown(_)) => session.client.call_with_context(caller, request),
        result => result,
    }
    .map_err(|error| error.to_string())?;
    let Response::Messages(crate::messaging::service::Response::Bundle(BundleResponse::Created(created))) = response else {
        return Err("daemon returned a different bundle creation".into());
    };
    for warning in created.warnings {
        eprintln!("Warning: {warning}");
    }
    crate::relay::trigger_push_client(&session.client);
    if json_mode {
        println!("{}", json!({"bundle_id":created.bundle_id}));
    } else {
        println!("{}", created.bundle_id);
    }
    Ok(0)
}

/// Format a short summary for an event data object.
fn format_event_summary(data: &Value) -> String {
    if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
        let short = if text.len() > 50 {
            let mut end = 47;
            while end > 0 && !text.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &text[..end])
        } else {
            text.to_string()
        };
        return format!("\"{}\"", short.replace('\n', " "));
    }
    if let Some(action) = data.get("action").and_then(|v| v.as_str()) {
        return action.to_string();
    }
    if let Some(status) = data.get("status").and_then(|v| v.as_str()) {
        let ctx = data.get("context").and_then(|v| v.as_str()).unwrap_or("");
        return if ctx.is_empty() { status.to_string() } else { format!("{status}:{ctx}") };
    }
    "(no summary)".to_string()
}

use crate::shared::time::format_age;

// ── Main Entry Point ─────────────────────────────────────────────────────

/// Main entry point for `agentx bundle` command.
/// Manual subcommand routing to support implicit list/show patterns.
pub(crate) fn cmd_bundle(session: &Session, args: &BundleArgs) -> i32 {
    match bundle_client(session, args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error}");
            1
        }
    }
}

fn bundle_client(session: &Session, args: &BundleArgs) -> Result<i32, String> {
    let argv = &args.args;
    let subcmd = argv.first().map(|s| s.as_str()).unwrap_or("list");
    let sub_argv: Vec<String> = if argv.is_empty() { vec![] } else { argv[1..].to_vec() };

    /// Try parsing a clap Args struct from sub-argv. Returns exit code on error.
    fn try_parse<T: clap::Parser>(name: &str, argv: &[String]) -> Result<T, i32> {
        T::try_parse_from(std::iter::once(name.to_string()).chain(argv.iter().cloned())).map_err(|e| crate::cli::commands::help::clap_usage_error(name, &e))
    }

    match subcmd {
        "list" => match try_parse::<BundleListArgs>("bundle list", &sub_argv) {
            Ok(a) => cmd_bundle_list(session, &a),
            Err(code) => Ok(code),
        },
        "show" => match try_parse::<BundleShowArgs>("bundle show", &sub_argv) {
            Ok(a) => cmd_bundle_show(session, &a),
            Err(code) => Ok(code),
        },
        "cat" => match try_parse::<BundleCatArgs>("bundle cat", &sub_argv) {
            Ok(a) => cmd_bundle_cat(session, &a),
            Err(code) => Ok(code),
        },
        "chain" => match try_parse::<BundleChainArgs>("bundle chain", &sub_argv) {
            Ok(a) => cmd_bundle_chain(session, &a),
            Err(code) => Ok(code),
        },
        "prepare" | "preview" => match try_parse::<BundlePrepareArgs>("bundle prepare", &sub_argv) {
            Ok(a) => cmd_bundle_prepare(session, &a),
            Err(code) => Ok(code),
        },
        "create" => match try_parse::<BundleCreateArgs>("bundle create", &sub_argv) {
            Ok(a) => cmd_bundle_create(session, &a),
            Err(code) => Ok(code),
        },
        _ => {
            if subcmd.starts_with('-') {
                // Flags without subcommand → list mode: `agentx bundle --json --last 5`
                match try_parse::<BundleListArgs>("bundle list", argv) {
                    Ok(a) => cmd_bundle_list(session, &a),
                    Err(code) => Ok(code),
                }
            } else {
                // Any non-flag, non-subcommand token → implicit show (bundle ID or prefix)
                // get_bundle_by_id() handles numeric IDs, bundle: prefix, and bare prefix lookup
                match try_parse::<BundleShowArgs>("bundle show", argv) {
                    Ok(a) => cmd_bundle_show(session, &a),
                    Err(code) => Ok(code),
                }
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::time::now_epoch_f64;
    use std::fs;

    #[test]
    fn test_format_age() {
        assert_eq!(format_age(30), "30s");
        assert_eq!(format_age(90), "1m");
        assert_eq!(format_age(3700), "1h");
        assert_eq!(format_age(90000), "1d");
    }

    #[test]
    fn test_format_event_summary_message() {
        let data = json!({"text": "hello world"});
        let summary = format_event_summary(&data);
        assert!(summary.contains("hello world"));
    }

    #[test]
    fn test_format_event_summary_status() {
        let data = json!({"status": "active", "context": "tool:Write"});
        let summary = format_event_summary(&data);
        assert_eq!(summary, "active:tool:Write");
    }

    #[test]
    fn test_format_event_summary_life() {
        let data = json!({"action": "stopped"});
        let summary = format_event_summary(&data);
        assert_eq!(summary, "stopped");
    }

    #[test]
    fn test_format_event_summary_truncation() {
        let long_text = "x".repeat(100);
        let data = json!({"text": long_text});
        let summary = format_event_summary(&data);
        assert!(summary.len() < 60);
        assert!(summary.ends_with("...\""));
    }

    // ── Clap sub-struct parse tests ────────────────────────────────

    use clap::Parser;

    #[test]
    fn test_bundle_top_level_passthrough() {
        // Top-level BundleArgs uses trailing_var_arg — accepts anything
        let args = BundleArgs::try_parse_from(["bundle"]).unwrap();
        assert!(args.args.is_empty());

        let args = BundleArgs::try_parse_from(["bundle", "--json", "--last", "5"]).unwrap();
        assert_eq!(args.args, vec!["--json", "--last", "5"]);

        let args = BundleArgs::try_parse_from(["bundle", "bundle:abc123"]).unwrap();
        assert_eq!(args.args, vec!["bundle:abc123"]);
    }

    #[test]
    fn test_bundle_list_parse() {
        let a = BundleListArgs::try_parse_from(["list", "--json", "--last", "5"]).unwrap();
        assert!(a.json);
        assert_eq!(a.last, Some(5));
    }

    #[test]
    fn test_bundle_show_parse() {
        let a = BundleShowArgs::try_parse_from(["show", "bundle:abc123"]).unwrap();
        assert_eq!(a.id, "bundle:abc123");
    }

    #[test]
    fn test_bundle_prepare_parse() {
        let a = BundlePrepareArgs::try_parse_from(["prepare", "--for", "peso", "--compact", "--last-transcript", "10"]).unwrap();
        assert_eq!(a.for_agent.as_deref(), Some("peso"));
        assert!(a.compact);
        assert_eq!(a.last_transcript, 10);
    }

    #[test]
    fn test_bundle_create_positional_title() {
        let a = BundleCreateArgs::try_parse_from(["create", "My Bundle", "--description", "A test"]).unwrap();
        assert_eq!(a.title_positional.as_deref(), Some("My Bundle"));
        assert_eq!(a.description.as_deref(), Some("A test"));
    }

    #[test]
    fn test_bundle_create_flag_title() {
        let a = BundleCreateArgs::try_parse_from(["create", "--title", "My Bundle", "--description", "A test"]).unwrap();
        assert_eq!(a.title_flag.as_deref(), Some("My Bundle"));
        assert_eq!(a.description.as_deref(), Some("A test"));
    }

    #[test]
    fn test_bundle_list_rejects_bogus() {
        assert!(BundleListArgs::try_parse_from(["list", "--bogus"]).is_err());
    }

    fn test_db() -> AgentxDb {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();
        std::mem::forget(dir);
        db
    }

    #[test]
    fn test_bundle_transcript_source_falls_back_to_stopped_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let transcript_path = dir.path().join("claude.jsonl");
        let db = test_db();

        let lines = [
            json!({
                "type": "user",
                "timestamp": "2026-04-13T12:00:00Z",
                "message": {
                    "content": [{"type": "text", "text": "remember marker"}]
                }
            }),
            json!({
                "type": "assistant",
                "timestamp": "2026-04-13T12:00:01Z",
                "message": {
                    "content": [{"type": "text", "text": "ack marker"}]
                }
            }),
        ];
        fs::write(&transcript_path, lines.iter().map(serde_json::Value::to_string).collect::<Vec<_>>().join("\n")).unwrap();

        let snapshot = json!({
            "name": "huno",
            "tool": "claude",
            "session_id": "sess-123",
            "transcript_path": transcript_path.to_string_lossy().to_string(),
            "created_at": now_epoch_f64(),
        });
        db.log_life_event("huno", "stopped", "cli", "killed", Some(snapshot)).unwrap();

        let (path, tool, sid) = exact_instance_transcript(&db, "huno").unwrap();
        assert_eq!(path, transcript_path.to_string_lossy());
        assert_eq!(tool, "claude");
        assert_eq!(sid.as_deref(), Some("sess-123"));

        let exchanges = get_exchanges(&path, &tool, 10, false, sid.as_deref(), true).unwrap();
        assert_eq!(exchanges.len(), 1);
        assert_eq!(exchanges[0].position, 1);
    }

    #[test]
    fn test_bundle_transcript_source_does_not_invent_claude_metadata() {
        let db = test_db();
        assert_eq!(exact_instance_transcript(&db, "missing"), None);
    }

    #[test]
    fn test_parse_exchange_range() {
        assert_eq!(parse_exchange_range("7"), Some((7, 7)));
        assert_eq!(parse_exchange_range("3-14"), Some((3, 14)));
        assert_eq!(parse_exchange_range("0"), None);
        assert_eq!(parse_exchange_range("9-3"), None);
        assert_eq!(parse_exchange_range("x-3"), None);
    }
}

#[cfg(test)]
mod native_bundle_tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, crate::daemon::test_support::TestDaemon, Session) {
        let root = tempfile::tempdir().unwrap();
        let daemon = crate::daemon::test_support::TestDaemon::start(root.path(), &root.path().join("agentx.db"));
        let invocation = crate::service::InvocationContext { cwd: root.path().into(), ..Default::default() };
        let client = crate::service::client::Client::connect_existing(root.path(), invocation.clone()).unwrap();
        let session = Session { client, invocation, actor: None, command: crate::shared::CommandContext { explicit_name: None, identity: None, go: false, identity_warning: None } };
        (root, daemon, session)
    }
    #[test]
    fn native_bundle_reader_never_reopens_managed_storage_after_transport_loss() {
        let (root, daemon, session) = fixture();
        assert_eq!(cmd_bundle(&session, &BundleArgs { args: vec!["list".into(), "--json".into()] }), 0);
        drop(daemon);
        std::fs::remove_file(root.path().join("agentx.db")).unwrap();
        assert_eq!(cmd_bundle(&session, &BundleArgs { args: vec!["list".into(), "--json".into()] }), 1);
        assert!(!root.path().join("agentx.db").exists());
    }
    #[test]
    fn explicit_missing_bundle_file_cannot_fall_back_to_inline_creation() {
        let (_root, _daemon, session) = fixture();
        assert_eq!(cmd_bundle(&session, &BundleArgs { args: vec!["create".into(), "--bundle-file".into(), "missing.json".into(), "--title".into(), "fallback".into(), "--description".into(), "must-not-create".into()] }), 1);
        assert!(pages(&session, 10, |before, through, limit| BundleRequest::List { before, through, limit }).unwrap().is_empty());
    }
}
