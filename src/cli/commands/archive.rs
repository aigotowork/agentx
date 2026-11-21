//! `agentx archive` command — list and query archived sessions.
//!
//!
//! Archives are stored in `~/.agentx/archive/session-{timestamp}/`, each containing
//! a copy of agentx.db from that session. This command lists and queries them.

use crate::service::{Request, Response, client::Client, maintenance};

use crate::shared::CommandContext;

/// Parsed arguments for `agentx archive`.
#[derive(clap::Parser, Debug)]
#[command(name = "archive", about = "List and query archived sessions")]
pub struct ArchiveArgs {
    /// Archive selector (index or name prefix)
    pub selector: Option<String>,
    /// Subcommand (e.g. "agents")
    pub subcmd: Option<String>,
    /// JSON output
    #[arg(long)]
    pub json: bool,
    /// Filter to current directory
    #[arg(long)]
    pub here: bool,
    /// SQL WHERE clause for filtering
    #[arg(long)]
    pub sql: Option<String>,
    /// Limit archives listed (default: all), or events queried from a
    /// selected archive (default: 20)
    #[arg(long)]
    pub last: Option<usize>,
}

fn limit_archives(archives: &[serde_json::Value], last: Option<usize>) -> &[serde_json::Value] {
    &archives[..archives.len().min(last.unwrap_or(usize::MAX))]
}

/// Resolve archive by index (1-based) or name prefix.
fn resolve_archive<'a>(selector: &str, archives: &'a [serde_json::Value]) -> Option<&'a serde_json::Value> {
    // Try as index
    if let Ok(idx) = selector.parse::<usize>()
        && idx >= 1
        && idx <= archives.len()
    {
        return Some(&archives[idx - 1]);
    }

    // Try as name or prefix match
    for archive in archives {
        let name = archive["name"].as_str().unwrap_or("");
        if name == selector || name.contains(selector) {
            return Some(archive);
        }
    }

    None
}

fn maintenance_call(client: &Client, request: maintenance::Request) -> Result<maintenance::Response, String> {
    match client.call(Request::Maintenance(request)).map_err(|e| e.to_string())? {
        Response::Maintenance(response) => Ok(response),
        _ => Err("unexpected archive response".into()),
    }
}

use crate::shared::shorten_path;

pub(crate) fn cmd_archive(client: &Client, args: &ArchiveArgs, _ctx: Option<&CommandContext>) -> i32 {
    let json_output = args.json;
    let here_filter = args.here;
    let last_count = args.last;
    let sql_filter = args.sql.clone();

    // Get archives list
    let archives = match maintenance_call(client, maintenance::Request::Archives { here: here_filter }) {
        Ok(maintenance::Response::Archives(archives)) => archives,
        Ok(_) => {
            eprintln!("Error: unexpected archive response");
            return 1;
        }
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };

    // No selector = list mode
    if args.selector.is_none() {
        if archives.is_empty() {
            if json_output {
                println!("{}", serde_json::json!({"archives": [], "count": 0, "total": 0}));
            } else {
                println!("No archives found");
            }
            return 0;
        }

        let visible_archives = limit_archives(&archives, last_count);

        if json_output {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "archives": visible_archives,
                    "count": visible_archives.len(),
                    "total": archives.len(),
                }))
                .unwrap_or_default()
            );
            return 0;
        }

        // Human-readable list
        println!("Archives:");
        for archive in visible_archives {
            let idx = archive["index"].as_i64().unwrap_or(0);
            let name = archive["name"].as_str().unwrap_or("?");
            let events = archive["events"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| "?".into());
            let instances = archive["instances"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| "?".into());
            println!("  {idx:>2}. {name}  {events} events  {instances} agents");
        }
        if visible_archives.len() < archives.len() {
            println!("  ... showing {} of {} (--last N for more)", visible_archives.len(), archives.len());
        }
        return 0;
    }

    // Selector provided — resolve archive
    let selector = args.selector.as_deref().unwrap();
    let resolved = match resolve_archive(selector, &archives) {
        Some(a) => a,
        None => {
            eprintln!("Error: Archive not found: {selector}");
            eprintln!("Run 'agentx archive' to list available archives");
            return 1;
        }
    };

    // Subcommand: agents
    if args.subcmd.as_deref() == Some("agents") {
        match maintenance_call(client, maintenance::Request::ArchivedParticipants { selector: resolved["name"].as_str().unwrap_or(selector).into(), here: here_filter, predicate: sql_filter.clone() }) {
            Ok(maintenance::Response::ArchivedParticipants(instances)) => {
                if json_output {
                    println!("{}", serde_json::to_string(&instances).unwrap_or_default());
                } else if instances.is_empty() {
                    println!("Archive is empty");
                } else {
                    println!("{:<8} {:<8} {:<40} transcript", "name", "status", "directory");
                    for inst in &instances {
                        let name = inst["name"].as_str().unwrap_or("?");
                        let status = inst["status"].as_str().unwrap_or("?");
                        let dir = shorten_path(inst["directory"].as_str().unwrap_or(""));
                        let transcript = shorten_path(inst["transcript_path"].as_str().unwrap_or(""));
                        // Truncate for display
                        let name = &name[..name.len().min(8)];
                        let status = &status[..status.len().min(8)];
                        let dir = if dir.len() > 40 { &dir[..40] } else { &dir };
                        println!("{name:<8} {status:<8} {dir:<40} {transcript}");
                    }
                }
                0
            }
            Ok(_) => {
                eprintln!("Error: unexpected archive response");
                1
            }
            Err(e) => {
                eprintln!("Error: Query failed: {e}");
                1
            }
        }
    } else {
        // Default: query events
        match maintenance_call(client, maintenance::Request::ArchivedEvents { selector: resolved["name"].as_str().unwrap_or(selector).into(), here: here_filter, predicate: sql_filter, limit: last_count.unwrap_or(20) }) {
            Ok(maintenance::Response::ArchivedEvents(events)) => {
                if json_output {
                    println!("{}", serde_json::to_string(&events).unwrap_or_default());
                } else if events.is_empty() {
                    println!("No events in archive");
                } else {
                    for event in &events {
                        let eid = event["id"].as_i64().unwrap_or(0);
                        let ts_raw = event["timestamp"].as_str().unwrap_or("");
                        let ts = if let Some(t_idx) = ts_raw.find('T') { &ts_raw[t_idx + 1..ts_raw.len().min(t_idx + 9)] } else { ts_raw };
                        let etype = event["type"].as_str().unwrap_or("");
                        let inst = event["instance"].as_str().unwrap_or("");
                        let data = &event["data"];

                        match etype {
                            "message" => {
                                let text = data["text"].as_str().unwrap_or("");
                                let truncated = if text.len() > 60 {
                                    let end = (0..=60).rev().find(|&i| text.is_char_boundary(i)).unwrap_or(0);
                                    format!("{}...", &text[..end])
                                } else {
                                    text.to_string()
                                };
                                println!("#{eid} {ts} {etype:<8} {inst:<8} \"{truncated}\"");
                            }
                            "status" => {
                                let status = data["status"].as_str().unwrap_or("?");
                                let context = data["context"].as_str().unwrap_or("");
                                println!("#{eid} {ts} {etype:<8} {inst:<8} {status} {context}");
                            }
                            "life" => {
                                let action = data["action"].as_str().unwrap_or("?");
                                let by = data["by"].as_str().unwrap_or("");
                                if by.is_empty() {
                                    println!("#{eid} {ts} {etype:<8} {inst:<8} {action}");
                                } else {
                                    println!("#{eid} {ts} {etype:<8} {inst:<8} {action} by:{by}");
                                }
                            }
                            _ => {
                                println!("#{eid} {ts} {etype:<8} {inst:<8}");
                            }
                        }
                    }
                }
                0
            }
            Ok(_) => {
                eprintln!("Error: unexpected archive response");
                1
            }
            Err(e) => {
                eprintln!("Error: Query failed: {e}");
                1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_args_last_flag() {
        use clap::Parser;
        let args = ArchiveArgs::try_parse_from(["archive", "--last", "50", "1"]).unwrap();
        assert_eq!(args.last, Some(50));
        assert_eq!(args.selector.as_deref(), Some("1"));
    }

    #[test]
    fn test_archive_args_last_default() {
        use clap::Parser;
        let args = ArchiveArgs::try_parse_from(["archive", "1"]).unwrap();
        assert_eq!(args.last, None);
        assert_eq!(args.selector.as_deref(), Some("1"));
    }

    #[test]
    fn test_archive_args_sql_flag() {
        use clap::Parser;
        let args = ArchiveArgs::try_parse_from(["archive", "--sql", "type='message'", "1"]).unwrap();
        assert_eq!(args.sql.as_deref(), Some("type='message'"));
        assert_eq!(args.selector.as_deref(), Some("1"));
    }

    #[test]
    fn test_archive_args_json_flag() {
        use clap::Parser;
        let args = ArchiveArgs::try_parse_from(["archive", "--json", "1"]).unwrap();
        assert!(args.json);
        assert_eq!(args.selector.as_deref(), Some("1"));
    }

    #[test]
    fn test_archive_list_limit() {
        let archives = vec![serde_json::json!({"index": 1}), serde_json::json!({"index": 2}), serde_json::json!({"index": 3})];
        assert_eq!(limit_archives(&archives, Some(2)).len(), 2);
        assert_eq!(limit_archives(&archives, Some(20)).len(), 3);
        assert!(limit_archives(&archives, Some(0)).is_empty());
        assert_eq!(limit_archives(&archives, None).len(), 3);
    }

    #[test]
    fn test_resolve_archive_by_index() {
        let archives = vec![serde_json::json!({"index": 1, "name": "session-2025-01-01_120000"}), serde_json::json!({"index": 2, "name": "session-2025-01-02_120000"})];
        let result = resolve_archive("1", &archives);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["name"].as_str().unwrap(), "session-2025-01-01_120000");
    }

    #[test]
    fn test_resolve_archive_by_name() {
        let archives = vec![serde_json::json!({"index": 1, "name": "session-2025-01-01_120000"})];
        let result = resolve_archive("2025-01-01", &archives);
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_archive_not_found() {
        let archives = vec![serde_json::json!({"index": 1, "name": "session-2025-01-01_120000"})];
        let result = resolve_archive("nonexistent", &archives);
        assert!(result.is_none());
    }

    #[test]
    fn test_shorten_path() {
        // Test with a path that doesn't start with HOME
        assert_eq!(shorten_path("/usr/local/bin"), "/usr/local/bin");
    }
}
