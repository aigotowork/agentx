//! `agentx list` command — list active instances.
//!
//!
//! Supports: human-readable, --json, --names, --format, -v,
//! single instance query (self/named), full listing with unread counts.

#[cfg(test)]
use std::collections::HashMap;

use crate::agents::lifecycle::format_age;
use crate::agents::records::{is_remote_instance, is_subagent_instance};
#[cfg(test)]
use crate::db::AgentxDb;
use crate::db::InstanceRow;
use crate::shared::{ST_LISTENING, shorten_path, shorten_path_max, status_icon};

use crate::cli::workspace::AgentFilterArgs;
use crate::service::queries::{AgentFilter, AgentWorkspace, Bindings, Listing};
#[cfg(test)]
use crate::service::queries_store::listing::{get_bindings_batch, get_unread_count, get_unread_counts_batch};
#[cfg(test)]
impl Bindings {
    fn for_instance(db: &AgentxDb, name: &str) -> Self {
        Self { session: db.has_session_binding(name), process: db.has_process_binding_for_instance(name) }
    }
}

fn tool_label(bindings: Bindings, data: &InstanceRow) -> String {
    if data.tool == "adhoc" {
        return "AD-HOC".to_string();
    }
    if let Some(platform) = data.launch_context_value("hermes_bridge", "platform") {
        return format!("HERMES:{}", platform.to_uppercase());
    }
    let tool = data.tool.to_uppercase();
    if is_remote_instance(data) || is_subagent_instance(data) {
        return tool;
    }
    if bindings.session && bindings.process { tool } else { format!("{tool}*") }
}

/// Binding summary for verbose/detail views: "hooks, process", "hooks", ...
/// An ad-hoc row's session binding only keys its identity (no hooks run).
fn bindings_display(bindings: Bindings, data: &InstanceRow) -> &'static str {
    let session = bindings.session;
    if data.tool == "adhoc" {
        return if session { "session" } else { "none" };
    }
    match (session, bindings.process) {
        (true, true) => "hooks, process",
        (true, false) => "hooks",
        (false, true) => "process",
        (false, false) => "none",
    }
}

/// Parsed arguments for `agentx list`.
#[derive(clap::Parser, Debug)]
#[command(name = "list", about = "List active agents")]
pub struct ListArgs {
    #[command(flatten)]
    pub(crate) filters: AgentFilterArgs,
    /// Show every workspace (or every stopped record with --stopped).
    #[arg(long, conflicts_with_all = ["dir", "workspace"])]
    pub all: bool,
    /// Agent name or "self"
    pub name: Option<String>,
    /// Field to extract (used with name)
    pub field: Option<String>,
    /// Show recently stopped agents
    #[arg(long, conflicts_with = "labels")]
    pub stopped: bool,
    /// JSON output
    #[arg(long)]
    pub json: bool,
    /// Verbose output
    #[arg(short = 'v', long)]
    pub verbose: bool,
    /// Names-only output
    #[arg(long)]
    pub names: bool,
    /// Shell export format
    #[arg(long)]
    pub sh: bool,
    /// Custom format template with {field} placeholders
    #[arg(long)]
    pub format: Option<String>,
    /// Limit results (with --stopped)
    #[arg(long)]
    pub last: Option<usize>,
}

/// Usage is read only for an explicitly selected local agent.
fn read_usage(path: &str, tool: &str, session_id: Option<&str>) -> Option<crate::transcript::SessionUsage> {
    if path.is_empty() {
        return None;
    }
    crate::cli::commands::transcript::get_transcript(path, tool, 1, false, session_id, false).ok()?.usage
}

/// Main entry point for `agentx list` command.
///
/// Returns exit code (0 = success, 1 = error).
pub(crate) fn cmd_list(session: &crate::cli::client::Session, args: &ListArgs) -> i32 {
    let workspace = match args.filters.workspace(session, args.name.is_none() && !args.all) {
        Ok(workspace) => workspace,
        Err(error) => {
            eprintln!("Error: {error:#}");
            return 1;
        }
    };
    if args.stopped {
        return cmd_list_stopped(session, args, &workspace);
    }
    if args.names && args.name.is_none() {
        use crate::service::{Request, Response, ServiceError, queries};
        let result = args.filters.label_predicates().and_then(|labels| session.client.call(Request::Queries(queries::Request::AgentNames(AgentFilter { workspace: workspace.clone(), labels }))));
        match result {
            Ok(Response::Queries(queries::Response::AgentNames(names))) => {
                for name in names {
                    println!("{name}");
                }
                return 0;
            }
            // A daemon may implement the older listing operation only.
            Err(ServiceError::Unsupported(_)) => {}
            Err(error) => {
                eprintln!("Error: {error}");
                return 1;
            }
            Ok(_) => {
                eprintln!("Error: unexpected Agent names response");
                return 1;
            }
        }
    }
    let target = args.name.as_deref().filter(|name| *name != "self").map(str::to_owned);
    let selected = if args.filters.labels.is_empty() {
        session.listing(target)
    } else {
        let target = if args.name.as_deref() == Some("self") { session.actor.as_ref().map(|actor| actor.participant.canonical_name.clone()) } else { target };
        match session.cleanup_query(crate::service::queries::Request::ListingLabels { target, selectors: args.filters.labels.clone(), workspace: workspace.as_selector().map(str::to_owned) }) {
            Ok(crate::service::queries::Response::Listing(mut listing)) => {
                for effect in std::mem::take(&mut listing.cleanup.effects) {
                    crate::runtime::stop_effects::apply(effect);
                }
                Ok(listing)
            }
            Ok(_) => Err(crate::service::ServiceError::Internal("unexpected label listing response".into())),
            Err(error) => Err(error),
        }
    };
    let mut listing = match selected {
        Ok(listing) => listing,
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    if workspace != AgentWorkspace::All && args.filters.labels.is_empty() {
        listing.records.retain(|record| workspace.visible(crate::agents::workspaces::workspace(record).as_deref()));
    }
    let config = crate::config::AgentxConfig::load_for_scope(session.client.scope_root(), &session.invocation.environment);
    let sender_identity = session.command.identity.clone();
    let current_name = sender_identity.as_ref().map(|actor| actor.name.clone());
    let json_output = args.json;
    let verbose_output = args.verbose;
    let names_output = args.names;
    let sh_output = args.sh;
    let format_template = args.format.clone();
    let target_name = args.name.as_deref();
    let field_name = args.field.as_deref();

    // Single instance query: agentx list <name|self> [field] [--json]
    let generation = match session.client.generation() {
        Ok(generation) => generation,
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    if let Some(target) = target_name {
        let is_self = target == "self";

        if is_self && sender_identity.is_none() {
            eprintln!("Error: Cannot use 'self' without identity. Run 'agentx start' first.");
            return 1;
        }

        let lookup_name = if is_self { current_name.clone().unwrap_or_default() } else { listing.selected_name.clone().unwrap_or_else(|| target.to_string()) };

        if lookup_name.is_empty() {
            eprintln!("Error: No name to look up.");
            return 1;
        }

        match listing.records.iter().find(|record| record.name == lookup_name).cloned() {
            Some(data) => {
                let mut payload = serde_json::json!({
                    "name": lookup_name,
                    "generation": generation,
                    "incarnation": data.created_at.to_bits().to_string(),
                    "labels": listing.labels.get(&lookup_name).map(|snapshot| &snapshot.labels),
                    "labels_revision": listing.labels.get(&lookup_name).map(|snapshot| &snapshot.revision),
                    "title": crate::agents::launch_title::from_context(data.launch_context.as_deref()),
                    "session_id": data.session_id,
                    "status": data.status,
                    "status_context": data.status_context,
                    "status_detail": data.status_detail,
                    "model_request": data.model_request,
                    "directory": data.directory,
                    "transcript_path": data.transcript_path,
                    "parent_name": data.parent_name,
                    "agent_id": data.agent_id,
                    "tool": data.tool,
                    "workspace": crate::agents::workspaces::workspace(&data),
                    "role": crate::agents::workspaces::collaboration(&data).and_then(|value| value.role),
                });

                if is_self
                    && let Some(id) = &sender_identity
                    && let Some(sid) = &id.session_id
                {
                    payload["session_id"] = serde_json::json!(sid);
                }

                let usage = if is_remote_instance(&data) {
                    None
                } else if !sh_output && (field_name.is_none() || field_name == Some("usage")) {
                    read_usage(&data.transcript_path, &data.tool, payload["session_id"].as_str())
                } else {
                    None
                };
                payload["usage"] = serde_json::to_value(&usage).unwrap_or(serde_json::Value::Null);

                if let Some(field) = field_name {
                    println!("{}", extract_field_value(&payload, field));
                } else if sh_output {
                    print_sh_exports(&payload);
                } else if json_output {
                    println!("{}", serde_json::to_string(&payload).unwrap_or_default());
                } else {
                    print_instance_details(&listing, &config, &data, &lookup_name, usage.as_ref());
                }
                return 0;
            }
            _ => {
                if is_self && workspace == AgentWorkspace::All && args.filters.labels.is_empty() {
                    let payload = serde_json::json!({
                        "name": lookup_name,
                        "session_id": sender_identity.as_ref().and_then(|id| id.session_id.as_deref()).unwrap_or(""),
                    });
                    if let Some(field) = field_name {
                        println!("{}", extract_field_value(&payload, field));
                    } else if sh_output {
                        print_sh_exports(&payload);
                    } else if json_output {
                        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
                    } else {
                        println!("{lookup_name}");
                    }
                    return 0;
                } else {
                    eprintln!("Error: {}", listing.missing_target.as_deref().unwrap_or("Agent not found"));
                    return 1;
                }
            }
        }
    }

    // Full listing mode
    let sorted_instances = &listing.records;
    if names_output {
        for data in sorted_instances {
            println!("{}", data.name);
        }
        return 0;
    }

    let unread_counts = &listing.unread;
    let binding_counts = &listing.bindings;

    if json_output || format_template.is_some() {
        let mut result_list: Vec<serde_json::Value> = Vec::new();

        for data in sorted_instances {
            let cs = listing.statuses[&data.name].clone();
            let (status, description, age_seconds) = (cs.status, cs.description, cs.age_seconds);

            // Get binding status
            let bindings = binding_counts.get(&data.name).copied().unwrap_or_default();
            let hooks_bound = bindings.session;
            let process_bound = bindings.process;

            // Parse launch_context JSON
            let launch_context: serde_json::Value = data.launch_context.as_deref().filter(|s| !s.is_empty()).and_then(|s| serde_json::from_str(s).ok()).unwrap_or(serde_json::json!({}));

            let payload = serde_json::json!({
                "generation": generation,
                "incarnation": data.created_at.to_bits().to_string(),
                "labels": listing.labels.get(&data.name).map(|snapshot| &snapshot.labels),
                "labels_revision": listing.labels.get(&data.name).map(|snapshot| &snapshot.revision),
                "name": data.name,
                "title": crate::agents::launch_title::from_context(data.launch_context.as_deref()),
                "status": status,
                "status_context": data.status_context,
                "status_detail": data.status_detail,
                "model_request": data.model_request,
                "status_age_seconds": age_seconds,
                "description": description,
                "unread_count": unread_counts.get(&data.name).copied().unwrap_or(0),
                "headless": data.background != 0,
                "session_id": data.session_id.as_deref().unwrap_or(""),
                "directory": data.directory,
                "parent_name": data.parent_name,
                "agent_id": data.agent_id,
                "background_log_file": if data.background_log_file.is_empty() { None } else { Some(&data.background_log_file) },
                "transcript_path": if data.transcript_path.is_empty() { None } else { Some(&data.transcript_path) },
                "created_at": data.created_at,
                "tool": data.tool,
                "hooks_bound": hooks_bound,
                "process_bound": process_bound,
                "launch_context": launch_context,
                "workspace": crate::agents::workspaces::workspace(data),
                "role": crate::agents::workspaces::collaboration(data).and_then(|value| value.role),
            });
            result_list.push(payload);
        }

        if let Some(ref template) = format_template {
            // Validate template keys against first payload (error on unknown fields)
            if let Some(first) = result_list.first()
                && let Some(obj) = first.as_object()
            {
                // Find all {key} placeholders in template
                let mut i = 0;
                let bytes = template.as_bytes();
                while i < bytes.len() {
                    if bytes[i] == b'{'
                        && let Some(end) = template[i + 1..].find('}')
                    {
                        let key = &template[i + 1..i + 1 + end];
                        if !key.is_empty() && !obj.contains_key(key) {
                            eprintln!("Error: unknown field '{{{}}}' in --format template", key);
                            return 1;
                        }
                        i += end + 2;
                        continue;
                    }
                    i += 1;
                }
            }
            for payload in &result_list {
                let obj = payload.as_object().unwrap();
                let mut line = template.clone();
                for (key, val) in obj {
                    let replacement = match val {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Null => String::new(),
                        other => other.to_string(),
                    };
                    line = line.replace(&format!("{{{key}}}"), &replacement);
                }
                println!("{line}");
            }
        } else {
            println!("{}", serde_json::to_string(&result_list).unwrap_or_default());
        }
        return 0;
    }

    // Human-readable output
    let display_name = current_name.unwrap_or_default();

    if !display_name.is_empty() {
        println!("Your name: {display_name}");
    } else {
        println!("Your name: (not participating)");
    }
    println!();

    // Check if multiple tool types exist
    let mut tool_types = std::collections::HashSet::new();
    for data in sorted_instances {
        tool_types.insert(data.tool.clone());
    }
    let show_tool = tool_types.len() > 1;

    // Check if multiple directories
    let mut directories = std::collections::HashSet::new();
    for data in sorted_instances {
        if !data.directory.is_empty() {
            directories.insert(data.directory.clone());
        }
    }

    // Compute name column width
    let mut max_name_len = 0;
    for data in sorted_instances {
        let mut n = data.name.len();
        if data.background != 0 {
            n += 11; // " [headless]"
        }
        if is_remote_instance(data) {
            n += 9; // " [remote]"
        }
        let uc = unread_counts.get(&data.name).copied().unwrap_or(0);
        if uc > 0 {
            n += format!(" +{uc}").len();
        }
        max_name_len = max_name_len.max(n);
    }
    let name_col_width = (max_name_len + 2).max(14);

    for data in sorted_instances {
        let name = data.name.clone();
        let cs = listing.statuses[&data.name].clone();
        let (status, age_str, description) = (cs.status, cs.age_string, cs.description);
        let icon = status_icon(&status);

        let age_display = if age_str == "now" {
            age_str.clone()
        } else if !age_str.is_empty() {
            format!("{age_str} ago")
        } else {
            String::new()
        };

        let desc_sep = if !description.is_empty() { ": " } else { "" };

        let tool_prefix = if show_tool {
            let padded = format!("[{}]", tool_label(binding_counts.get(&data.name).copied().unwrap_or_default(), data));
            // Long labels (HERMES:TELEGRAM) still get a separating space.
            let width = padded.chars().count().max(9) + 1;
            format!("{padded:<width$}")
        } else {
            String::new()
        };

        // Badges
        let headless_badge = if data.background != 0 { " [headless]" } else { "" };
        let remote_badge = if is_remote_instance(data) { " [remote]" } else { "" };
        let workspace = crate::agents::workspaces::workspace(data);
        let collaboration = crate::agents::workspaces::collaboration(data);
        let role_badge = collaboration.as_ref().and_then(|value| value.role.as_ref()).map(|role| format!(" [{}]", role.name)).unwrap_or_default();
        let workspace_badge = workspace.as_ref().map(|path| if args.all { format!(" [{}]", shorten_path_max(path, 40)) } else { String::new() }).unwrap_or_else(|| " [global]".into());

        // Unread
        let unread = unread_counts.get(&data.name).copied().unwrap_or(0);
        let unread_str = if unread > 0 { format!(" +{unread}") } else { String::new() };

        // Listening-since suffix: show idle duration for listening agents idle >= 60s
        let listening_since = if status == ST_LISTENING && cs.age_seconds >= 60 { format!(" since {}", format_age(cs.age_seconds)) } else { String::new() };

        // Subagent timeout marker: show countdown when < 10s remaining
        let timeout_marker = if status == ST_LISTENING && data.parent_session_id.is_some() {
            let timeout = if let Some(ref parent_name) = data.parent_name { listing.records.iter().find(|record| &record.name == parent_name).and_then(|p| p.subagent_timeout) } else { None }.unwrap_or(config.subagent_timeout);
            let remaining = timeout.saturating_sub(cs.age_seconds);
            if remaining > 0 && remaining < 10 { format!(" \u{23f1} {remaining}s") } else { String::new() }
        } else {
            String::new()
        };

        let name_part = format!("{name}{role_badge}{workspace_badge}{headless_badge}{remote_badge}{unread_str}");
        let status_text = format!("{age_display}{desc_sep}{description}{listening_since}{timeout_marker}");

        println!("{tool_prefix}{icon} {name_part:<width$}{status_text}", width = name_col_width);

        if verbose_output {
            let session_id = data.session_id.as_deref().unwrap_or("(none)");
            let directory_display = if data.directory.is_empty() { "(none)".to_string() } else { shorten_path_max(&data.directory, 60) };
            let parent = data.parent_name.as_deref().unwrap_or("(none)");
            let tool_display = if data.tool == "adhoc" { "ad-hoc" } else { &data.tool };

            let created_str = if data.created_at > 0.0 {
                let now = crate::shared::time::now_epoch_f64();
                let age_f = now - data.created_at;
                // format_age takes i64 where 0 → "now". Use f64 threshold
                // so sub-second ages display as "0s" instead of "now".
                let age_str = if age_f <= 0.0 {
                    "now".to_string()
                } else {
                    let secs = age_f as i64;
                    if secs < 60 { format!("{secs}s") } else { format_age(secs) }
                };
                format!("{age_str} ago")
            } else {
                "(unknown)".to_string()
            };

            println!("    session_id:   {session_id}");
            println!("    tool:         {tool_display}");
            println!("    created:      {created_str}");
            println!("    directory:    {directory_display}");

            if parent != "(none)" {
                println!("    parent:       {parent}");
                let agent_id = data.agent_id.as_deref().unwrap_or("(none)");
                println!("    agent_id:     {agent_id}");
            }

            // Binding status
            println!("    bindings:     {}", bindings_display(binding_counts.get(&data.name).copied().unwrap_or_default(), data));

            let transcript = if data.transcript_path.is_empty() { "(none)".to_string() } else { shorten_path_max(&data.transcript_path, 60) };
            if data.background != 0 && !data.background_log_file.is_empty() {
                println!("    headless log: {}", shorten_path_max(&data.background_log_file, 60));
            }
            println!("    transcript:   {transcript}");

            if !data.status_detail.is_empty() {
                let detail = if data.status_detail.len() > 60 {
                    let end = (0..=60).rev().find(|&i| data.status_detail.is_char_boundary(i)).unwrap_or(0);
                    format!("{}...", &data.status_detail[..end])
                } else {
                    data.status_detail.clone()
                };
                println!("    detail:       {detail}");
                if let Some(cmd) = crate::relay::preview_fetch_command(&data.status_detail) {
                    println!("    full detail:  {cmd}");
                }
            }
            println!();
        }
    }

    if sorted_instances.is_empty() {
        println!("No active agents. Launch one with: agentx launch --tool claude");
    }

    // Recently stopped summary
    let recently_stopped = &listing.recently_stopped;
    if !recently_stopped.is_empty() {
        let names = if recently_stopped.len() <= 5 { recently_stopped.join(", ") } else { format!("{} +{}", recently_stopped[..5].join(", "), recently_stopped.len() - 5) };
        println!("\nRecently stopped (10m): {names}");
        println!("  -> agentx list --stopped [name]");
    }

    // Hint about archives if no instances
    if sorted_instances.is_empty() {
        let archive_dir = session.client.scope_root().join("archive");
        if archive_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&archive_dir)
        {
            let archive_count = entries.filter_map(|e| e.ok()).filter(|e| e.file_name().to_str().map(|s| s.starts_with("session-")).unwrap_or(false)).count();
            if archive_count > 0 {
                let plural = if archive_count != 1 { "s" } else { "" };
                println!("({archive_count} archived session{plural} - run: agentx archive)");
            }
        }
    }

    0
}

fn print_instance_details(listing: &Listing, config: &crate::config::AgentxConfig, data: &InstanceRow, display_name: &str, usage: Option<&crate::transcript::SessionUsage>) {
    let cs = listing.statuses[&data.name].clone();
    let status = cs.status;

    // Status line construction
    let status_line = if data.status_context.is_empty() { status.clone() } else { format!("{status} ({})", data.status_context) };

    println!("{display_name}:");

    // Core Identity
    let headless_str = if data.background != 0 { " (Headless)" } else { "" };
    let tool_display = if data.tool == "adhoc" { "ad-hoc" } else { &data.tool };
    println!("  Tool:        {tool_display}{headless_str}");
    let collaboration = crate::agents::workspaces::collaboration(data);
    println!("  Workspace:   {}", crate::agents::workspaces::workspace(data).as_deref().unwrap_or("global"));
    if let Some(role) = collaboration.and_then(|value| value.role) {
        println!("  Role:        {} ({})", role.name, role.path.display());
    }

    if let Some(ref term) = data.terminal_preset_effective.as_ref().or(data.terminal_preset_requested.as_ref())
        && !term.is_empty()
    {
        println!("  Terminal:    {term}");
    }

    let session_id = data.session_id.as_deref().unwrap_or("(none)");
    println!("  Session:     {session_id}");

    // Status & Connection
    println!("  Status:      {status_line}");
    if !data.status_detail.is_empty() {
        println!("  Detail:      {}", data.status_detail);
    }

    // Uptime and Age
    let now = crate::shared::time::now_epoch_f64();
    if data.status_time > 0 {
        let state_age = now - (data.status_time as f64);
        if state_age > 0.0 {
            println!("  State Age:   {}", format_age(state_age as i64));
        }
    }
    if data.created_at > 0.0 {
        let uptime = now - data.created_at;
        if uptime > 0.0 {
            println!("  Uptime:      {}", format_age(uptime as i64));
        }
    }

    // Unread Count
    let unread = listing.unread.get(&data.name).copied().unwrap_or(0);
    if unread > 0 {
        let s = if unread == 1 { "" } else { "s" };
        println!("  Unread:      {unread} message{s}");
    }

    // Bindings
    println!("  Bindings:    {}", bindings_display(listing.bindings.get(&data.name).copied().unwrap_or_default(), data));
    if let Some(port) = listing.plugin_ports.get(&data.name) {
        println!("  Plugin:      wake port {port}");
    }

    if let Some(pid) = data.pid {
        println!("  PID:         {pid}");
    }

    // Hierarchy
    if let Some(ref parent) = data.parent_name
        && !parent.is_empty()
    {
        println!("  Parent:      {parent}");
        if let Some(ref agent_id) = data.agent_id
            && !agent_id.is_empty()
        {
            println!("  Agent ID:    {agent_id}");
        }

        // Subagent Timeout
        let timeout = data.subagent_timeout.unwrap_or(config.subagent_timeout);
        let remaining = timeout.saturating_sub(cs.age_seconds);
        if status == ST_LISTENING && remaining > 0 {
            println!("  Timeout:     {}s remaining", remaining);
        }
    }

    // Paths
    println!("  Directory:   {}", shorten_path_max(&data.directory, 80));

    if !data.transcript_path.is_empty() {
        println!("  Transcript:  {}", shorten_path(&data.transcript_path));
    }
    if let Some(usage) = usage {
        println!("  Usage:       {}", crate::transcript::format_usage(usage));
    }

    if data.background != 0 && !data.background_log_file.is_empty() {
        println!("  Log File:    {}", shorten_path(&data.background_log_file));
    }
}

/// Extract a field value from a JSON payload, normalizing booleans to "1"/"0".
fn extract_field_value(payload: &serde_json::Value, field: &str) -> String {
    match payload.get(field) {
        Some(serde_json::Value::Bool(b)) => if *b { "1" } else { "0" }.to_string(),
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Null) => String::new(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Print shell-export format for `agentx list --sh`.
fn print_sh_exports(payload: &serde_json::Value) {
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let session_id = payload.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
    let status = payload.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
    let directory = payload.get("directory").and_then(|v| v.as_str()).unwrap_or("");

    println!("export AGENTX_INSTANCE_NAME={}", shell_quote(name));
    println!("export AGENTX_SID={}", shell_quote(session_id));
    println!("export AGENTX_STATUS={}", shell_quote(status));
    println!("export AGENTX_DIRECTORY={}", shell_quote(directory));
}

use crate::integrations::launch_args::shell::shell_quote;

/// `agentx list --stopped [name] [--all] [--last N]` — show stopped instances from life events.
/// Without a name: shows recent stopped (default last 20, use --all for unlimited).
/// With a name: shows details for that specific stopped instance.
/// Uses human-friendly formatting rather than raw JSON for readability.
fn cmd_list_stopped(session: &crate::cli::client::Session, args: &ListArgs, workspace: &AgentWorkspace) -> i32 {
    let show_all = args.all;
    let last_n: usize = args.last.unwrap_or(20);
    let filter_name = args.name.as_deref();

    let now = crate::shared::time::now_epoch_f64();

    let limit = if show_all { 10000 } else { last_n };

    let scoped = workspace != &AgentWorkspace::All;
    let mut entries = match session.client.call(crate::service::Request::Queries(crate::service::queries::Request::StoppedHistory { target: filter_name.map(str::to_owned), limit: if filter_name.is_some() || scoped { 10000 } else { limit } })) {
        Ok(crate::service::Response::Queries(crate::service::queries::Response::StoppedHistory(entries))) => entries,
        Ok(_) => {
            eprintln!("Error: daemon returned a different stopped history projection");
            return 1;
        }
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    if scoped {
        entries.retain(|entry| {
            let data: serde_json::Value = serde_json::from_str(&entry.data).unwrap_or_default();
            workspace.visible(crate::agents::workspaces::workspace_from_context(data["snapshot"]["launch_context"].as_str()).as_deref())
        });
        if filter_name.is_none() {
            entries.truncate(last_n);
        }
    }
    if args.json && filter_name.is_none() {
        let values: Vec<_> = entries
            .iter()
            .map(|entry| {
                let mut data: serde_json::Value = serde_json::from_str(&entry.data).unwrap_or_else(|_| serde_json::json!({}));
                data["name"] = serde_json::json!(entry.instance);
                data["timestamp"] = serde_json::json!(entry.timestamp);
                data
            })
            .collect();
        println!("{}", serde_json::json!(values));
        return 0;
    }

    if entries.is_empty() {
        if let Some(name) = filter_name {
            println!("No stopped events found for '{name}'");
        } else {
            println!("No recently stopped agents (last 60m)");
        }
        return 0;
    }

    if filter_name.is_some() {
        // Detailed view for a single instance (show all stop events, not just 1)
        let entry = &entries[0];
        let data: serde_json::Value = serde_json::from_str(&entry.data).unwrap_or_default();
        let snapshot = &data["snapshot"];
        let usage = read_usage(snapshot["transcript_path"].as_str().unwrap_or(""), snapshot["tool"].as_str().unwrap_or(""), snapshot["session_id"].as_str());
        if args.json || args.field.is_some() {
            let mut payload = snapshot.clone();
            if !payload.is_object() {
                payload = serde_json::json!({});
            }
            payload["name"] = serde_json::json!(entry.instance);
            payload["usage"] = serde_json::to_value(&usage).unwrap_or(serde_json::Value::Null);
            if let Some(field) = &args.field {
                println!("{}", extract_field_value(&payload, field));
            } else {
                println!("{}", payload);
            }
            return 0;
        }
        println!("Stopped: {}", entry.instance);
        println!("  Time:       {}", entry.timestamp);
        if let Some(by) = data["by"].as_str() {
            println!("  By:         {by}");
        }
        if let Some(reason) = data["reason"].as_str() {
            println!("  Reason:     {reason}");
        }
        if let Some(code) = data["exit_code"].as_i64() {
            println!("  Exit code:  {code}");
        }
        if let Some(tool) = snapshot["tool"].as_str() {
            println!("  Tool:       {tool}");
        }
        if let Some(dir) = snapshot["directory"].as_str() {
            println!("  Directory:  {dir}");
        }
        if let Some(sid) = snapshot["session_id"].as_str()
            && !sid.is_empty()
        {
            println!("  Session:    {sid}");
        }
        if let Some(tp) = snapshot["transcript_path"].as_str()
            && !tp.is_empty()
        {
            println!("  Transcript: {tp}");
        }
        if let Some(usage) = &usage {
            println!("  Usage:      {}", crate::transcript::format_usage(usage));
        }
        let output = data["output"].as_str().unwrap_or("");
        print!("{}", crate::agents::identity::last_output_block(output));
        println!("\n  Resume: agentx r {}", entry.instance);

        // Show history if multiple stop events
        if entries.len() > 1 {
            println!("\n  Stop history ({} events):", entries.len());
            for (i, e) in entries.iter().enumerate() {
                let d: serde_json::Value = serde_json::from_str(&e.data).unwrap_or_default();
                let reason = d["reason"].as_str().unwrap_or("");
                let by = d["by"].as_str().unwrap_or("");
                let age = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&e.timestamp) {
                    let event_epoch = dt.timestamp() as f64;
                    format_age((now - event_epoch) as i64)
                } else {
                    e.timestamp.clone()
                };
                let by_part = if by.is_empty() { String::new() } else { format!(" by:{by}") };
                let marker = if i == 0 { " (latest)" } else { "" };
                println!("    {age} ago  [{reason}{by_part}]{marker}");
            }
        }
    } else {
        // Summary table
        let header = if show_all { format!("Stopped agents (all, showing {}):", entries.len()) } else { format!("Stopped agents (last {last_n}):") };
        println!("{header}\n");
        for entry in &entries {
            let data: serde_json::Value = serde_json::from_str(&entry.data).unwrap_or_default();
            let snapshot = &data["snapshot"];
            let tool = snapshot["tool"].as_str().unwrap_or("?");
            let reason = data["reason"].as_str().unwrap_or("");
            let by = data["by"].as_str().unwrap_or("");
            // Parse timestamp for age
            let age = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&entry.timestamp) {
                let event_epoch = dt.timestamp() as f64;
                format_age((now - event_epoch) as i64)
            } else {
                entry.timestamp.clone()
            };
            let dir = snapshot["directory"].as_str().unwrap_or("");
            let by_part = if by.is_empty() { String::new() } else { format!(" by:{by}") };
            let dir_part = if dir.is_empty() { String::new() } else { format!("  {}", shorten_path_max(dir, 40)) };
            println!("  {} ({tool}) {age} ago  [{reason}{by_part}]{dir_part}", entry.instance);
        }
        if !show_all {
            println!("\n  --all: show all  |  --last N: show last N");
        }
        println!("  Details: agentx list --stopped <name>");
        println!("  Resume:  agentx r <name>");
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_usage_reads_whole_session_and_preserves_missing_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"type\":\"user\",\"message\":{\"content\":\"first\"}}\n",
                "{\"type\":\"assistant\",\"message\":{\"id\":\"one\",\"content\":[],\"usage\":{\"input_tokens\":10,\"output_tokens\":5}}}\n",
                "{\"type\":\"user\",\"message\":{\"content\":\"second\"}}\n",
                "{\"type\":\"assistant\",\"message\":{\"id\":\"two\",\"content\":[],\"usage\":{\"input_tokens\":20,\"output_tokens\":7}}}\n"
            ),
        )
        .unwrap();
        let usage = read_usage(path.to_str().unwrap(), "claude", None).unwrap();
        assert_eq!(usage.tokens.input, 30);
        assert_eq!(usage.tokens.output, 12);
        assert_eq!(usage.context_tokens, Some(20));
        assert_eq!(usage.context_window, None);
        assert_eq!(usage.cost_usd, None);
        assert!(read_usage("", "claude", None).is_none());
        std::fs::write(&path, "").unwrap();
        assert!(read_usage(path.to_str().unwrap(), "claude", None).is_none());
    }

    fn test_db() -> AgentxDb {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_raw(&dir.path().join("test.db")).unwrap();
        db.init_db().unwrap();
        std::mem::forget(dir);
        db
    }

    #[test]
    fn unread_batch_preserves_cursors_recipients_and_duplicate_semantics() {
        let db = test_db();
        for (name, origin) in [("full", None), ("pend", None), ("none", None), ("remo", Some("dev-1"))] {
            db.conn().execute("INSERT INTO instances (name, tool, origin_device_id, created_at) VALUES (?, 'codex', ?, 0)", rusqlite::params![name, origin]).unwrap();
        }
        let first = db.log_event("message", "sender", &serde_json::json!({"delivered_to": ["full", "pend", "remo"]})).unwrap();
        db.log_event("status", "sender", &serde_json::json!({"delivered_to": ["full"]})).unwrap();
        db.log_event("message", "sender", &serde_json::json!({"delivered_to": ["full", "full", "pend", "remo"]})).unwrap();
        db.log_event("message", "sender", &serde_json::json!({})).unwrap();
        db.log_event("message", "sender", &serde_json::json!({"delivered_to": []})).unwrap();
        db.conn().execute("UPDATE instances SET last_event_id = ? WHERE name = 'full'", [first]).unwrap();
        let instances = db.iter_instances_full().unwrap();
        let counts = get_unread_counts_batch(&db, &instances);
        assert_eq!(counts.len(), 2);
        assert_eq!(counts["full"], 1);
        assert_eq!(counts["pend"], 2);
        for inst in instances.iter().filter(|inst| !is_remote_instance(inst)) {
            assert_eq!(counts.get(&inst.name).copied().unwrap_or(0), get_unread_count(&db, &inst.name, inst.last_event_id));
        }
        assert!(get_unread_counts_batch(&db, &[]).is_empty());
        let remote: Vec<_> = instances.into_iter().filter(is_remote_instance).collect();
        assert!(get_unread_counts_batch(&db, &remote).is_empty());
    }

    #[test]
    fn binding_batch_matches_individual_checks_with_multiple_bindings() {
        let db = test_db();
        for name in ["full", "hook", "proc", "none"] {
            db.conn().execute("INSERT INTO instances (name, tool, created_at) VALUES (?, 'codex', 0)", [name]).unwrap();
        }
        db.set_session_binding("s1", "full").unwrap();
        db.set_session_binding("s2", "full").unwrap();
        db.set_process_binding("p1", "s1", "full").unwrap();
        db.set_process_binding("p2", "s2", "full").unwrap();
        db.set_session_binding("s3", "hook").unwrap();
        db.set_process_binding("p3", "", "proc").unwrap();
        let bindings = get_bindings_batch(&db);
        assert_eq!(bindings.len(), 4);
        for name in ["full", "hook", "proc", "none"] {
            assert_eq!(bindings[name], Bindings::for_instance(&db, name));
        }
    }

    #[test]
    #[ignore = "manual unread-count scaling benchmark"]
    fn benchmark_unread_counts_batch() {
        for (agents, base_cursor, body_size) in [(10, 9000, 16), (100, 9000, 16), (500, 9000, 16), (10, 0, 4096)] {
            let db = test_db();
            db.conn().execute_batch("BEGIN").unwrap();
            for i in 0..agents {
                db.conn().execute("INSERT INTO instances (name, tool, last_event_id, created_at) VALUES (?, 'codex', ?, 0)", rusqlite::params![format!("agent{i}"), base_cursor + if base_cursor > 0 { i % 500 } else { 0 }]).unwrap();
            }
            for id in 1..=10000 {
                let data = serde_json::json!({"delivered_to": [format!("agent{}", id % agents), format!("agent{}", (id + 1) % agents)], "text": "x".repeat(body_size)});
                db.conn().execute("INSERT INTO events (id, timestamp, type, instance, data) VALUES (?, '2026-01-01', 'message', 'sender', ?)", rusqlite::params![id, data.to_string()]).unwrap();
            }
            db.conn().execute_batch("COMMIT").unwrap();
            let instances = db.iter_instances_full().unwrap();
            let start = std::time::Instant::now();
            let old: HashMap<_, _> = instances
                .iter()
                .filter_map(|inst| {
                    let count = get_unread_count(&db, &inst.name, inst.last_event_id);
                    (count > 0).then(|| (inst.name.clone(), count))
                })
                .collect();
            let old_time = start.elapsed();
            let start = std::time::Instant::now();
            let new = get_unread_counts_batch(&db, &instances);
            let new_time = start.elapsed();
            assert_eq!(old, new);
            println!("{agents} agents / 10000 messages / cursor {base_cursor} / body {body_size} bytes: per-agent {old_time:?}, batch {new_time:?}");
        }
    }

    fn label(db: &AgentxDb, name: &str) -> String {
        tool_label(get_bindings_batch(db).get(name).copied().unwrap_or_default(), &db.get_instance_full(name).unwrap().unwrap())
    }

    #[test]
    fn tool_label_stars_only_local_roots_missing_a_binding() {
        let db = test_db();
        for (name, tool, parent, origin) in
            [("full", "claude", None, None), ("pend", "codex", None, None), ("hook", "gemini", None, None), ("subx", "claude", Some("full"), None), ("remo", "claude", None, Some("dev-1")), ("adho", "adhoc", None, None)]
        {
            db.conn()
                .execute(
                    "INSERT INTO instances (name, tool, parent_name, origin_device_id, created_at)
                     VALUES (?, ?, ?, ?, 1000.0)",
                    rusqlite::params![name, tool, parent, origin],
                )
                .unwrap();
        }
        db.set_session_binding("s-full", "full").unwrap();
        db.set_process_binding("p-full", "s-full", "full").unwrap();
        db.set_process_binding("p-pend", "", "pend").unwrap();
        db.set_session_binding("s-hook", "hook").unwrap();
        db.set_session_binding("s-adho", "adho").unwrap();

        assert_eq!(label(&db, "full"), "CLAUDE");
        assert_eq!(label(&db, "pend"), "CODEX*");
        assert_eq!(label(&db, "hook"), "GEMINI*");
        assert_eq!(label(&db, "subx"), "CLAUDE");
        assert_eq!(label(&db, "remo"), "CLAUDE");
        assert_eq!(label(&db, "adho"), "AD-HOC");

        let bindings = |name| bindings_display(get_bindings_batch(&db).get(name).copied().unwrap_or_default(), &db.get_instance_full(name).unwrap().unwrap());
        assert_eq!(bindings("full"), "hooks, process");
        assert_eq!(bindings("pend"), "process");
        assert_eq!(bindings("subx"), "none");
        assert_eq!(bindings("adho"), "session");
    }
}
