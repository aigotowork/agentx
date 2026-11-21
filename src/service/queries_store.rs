//! Daemon-owned snapshot and search adapter. No client opens the managed database.
use super::queries::status;
use super::queries::*;
use crate::log::log_warn;
use crate::service::{RequestContext, ServiceResult};
use crate::shared::ST_ACTIVE;
use crate::{db::AgentxDb, paths};
use rusqlite::{Connection, params};

#[path = "listing_store.rs"]
pub(crate) mod listing;

#[path = "queries_info_store.rs"]
pub(crate) mod info_store;

fn epoch_now() -> f64 {
    crate::shared::time::now_epoch_f64()
}
fn format_duration_short(secs: u64) -> String {
    crate::shared::time::format_age(secs as i64)
}
fn read_device_uuid(conn: &Connection) -> String {
    kv_get(conn, "device_uuid").unwrap_or_default()
}

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    Ok(match request {
        Request::AgentInfo { target, section } => Response::AgentInfo(info_store::read(db, context, &target, section)?),
        Request::FileBindings(_) => return Err(crate::service::ServiceError::Internal("file bindings require application-owned index".into())),
        Request::Diagnostics => Response::Diagnostics(diagnostics(db, context)?),
        Request::TranscriptSource { name, expected } => Response::TranscriptSource(crate::transcript::store::select(db, &name, expected.as_ref())?),
        Request::TranscriptInventory { live_only } => Response::TranscriptInventory(crate::transcript::store::inventory(db, live_only)?),
        Request::TranscriptCorrelations { targets } => Response::TranscriptCorrelations(crate::transcript::store::correlate(db, &targets)?),
        Request::ListingLabels { target, selectors, workspace } => Response::Listing(listing::load_labels(db, context, target.as_deref(), selectors, workspace)?),
        Request::Listing { target } => Response::Listing(listing::load(db, context, target.as_deref())?),
        Request::AgentNames(filter) => Response::AgentNames(listing::names(db, filter)?),
        Request::StoppedHistory { target, limit } => Response::StoppedHistory(listing::stopped_history(db, target.as_deref(), limit)?),
        Request::Snapshot(options) => Response::Snapshot(load_all(db, context, options, &AgentFilter::default())?),
        Request::FilteredSnapshot { options, filter } => Response::Snapshot(load_all(db, context, options, &filter)?),
        Request::Stopped => Response::Stopped(with_info_references(db, context, load_stopped(db.conn(), epoch_now(), None, &AgentWorkspace::All))),
        Request::FilteredStopped(workspace) => Response::Stopped(with_info_references(db, context, load_stopped(db.conn(), epoch_now(), None, &workspace))),
        Request::StoppedTargets { targets } => Response::StoppedTargets(targets.iter().filter_map(|target| crate::agents::identity::last_stopped(db, target)).collect()),
        Request::Terminal { participant } => {
            let record = db.get_instance_full(&participant.canonical_name).map_err(query_error)?.ok_or(crate::service::ServiceError::StaleFence)?;
            if record.created_at.to_bits() != participant.incarnation_bits {
                return Err(crate::service::ServiceError::StaleFence);
            }
            Response::Terminal(terminal_endpoint(db, record)?)
        }
        Request::Terminals => Response::Terminals(matching_terminals(db, context, None, &AgentFilter::default())?),
        Request::MatchingTerminals { name, filter } => Response::Terminals(matching_terminals(db, context, name.as_deref(), &filter)?),
        Request::UnreadPreview { participant } => {
            let record = db.get_instance_full(&participant.canonical_name).map_err(query_error)?.ok_or(crate::service::ServiceError::StaleFence)?;
            if record.created_at.to_bits() != participant.incarnation_bits {
                return Err(crate::service::ServiceError::StaleFence);
            }
            let messages = db.try_get_unread_messages(&record.name).map_err(query_error)?;
            let mut seen = std::collections::HashSet::new();
            let senders: Vec<_> = messages.iter().map(|message| message.from.clone()).filter(|name| seen.insert(name.clone())).collect();
            let preview = if senders.is_empty() {
                None
            } else {
                let receiver = &record.name;
                let mut text = format!("{} → {receiver}", senders.join(", "));
                if text.len() > 200 {
                    let end = (0..=197).rev().find(|&index| text.is_char_boundary(index)).unwrap_or(0);
                    text.truncate(end);
                    text.push_str("...");
                }
                Some(format!("<agentx>{text}</agentx>"))
            };
            Response::UnreadPreview(preview)
        }
        Request::SearchTimeline { query, limit } => {
            let (messages, events) = load_timeline_search(db.conn(), &query, limit);
            Response::Timeline { messages, events }
        }
    })
}

fn query_error(error: impl std::fmt::Display) -> crate::service::ServiceError {
    crate::service::ServiceError::Internal(error.to_string())
}

/// Status retains the old sweep/finalization policy, without doing a full list
/// cleanup or reading log files/terminal state on the daemon dispatcher.
fn diagnostics(db: &AgentxDb, context: &RequestContext) -> ServiceResult<Diagnostics> {
    db.with_write_scope(|| {
        let cleanup = crate::agents::service::cleanup_system(db, context, crate::agents::service::SystemCleanup::DeadProcesses).map_err(anyhow::Error::new)?;
        for record in db.iter_instances_full()? {
            if !crate::agents::records::is_remote_instance(&record) && crate::agents::records::is_launching_placeholder(&record) {
                crate::agents::lifecycle::try_finalize_launch_failure_detail_stored(db, &record, None).map_err(anyhow::Error::new)?;
            }
        }
        let mut counts = ParticipantCounts::default();
        let mut statement = db.conn().prepare("SELECT status,COUNT(*) FROM instances GROUP BY status")?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        for row in rows {
            let (status, count) = row?;
            match status.as_str() {
                status if status.starts_with("active") => counts.active += count,
                "listening" => counts.listening += count,
                status if status.starts_with("blocked") => counts.blocked += count,
                "error" => counts.error += count,
                "launching" => counts.launching += count,
                _ => counts.inactive += count,
            }
        }
        counts.total = counts.active + counts.listening + counts.blocked + counts.error + counts.launching + counts.inactive;
        let mut statement = db.conn().prepare("SELECT name FROM instances WHERE status_context='launch_failed' ORDER BY status_time DESC LIMIT 5")?;
        let names = statement.query_map([], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut launch_failures = Vec::new();
        for name in names {
            launch_failures.push(db.get_instance_full(&name)?.ok_or_else(|| anyhow::anyhow!("diagnostic participant vanished within snapshot"))?);
        }
        let dev_root = db.kv_get(crate::db::DEV_ROOT_KV_KEY)?.filter(|value| !value.is_empty());
        Ok(Diagnostics { counts, launch_failures, dev_root, cleanup })
    })
    .map_err(|error| error.downcast_ref::<crate::service::ServiceError>().cloned().unwrap_or_else(|| query_error(error)))
}

fn terminal_endpoint(db: &AgentxDb, record: crate::db::InstanceRow) -> ServiceResult<TerminalEndpoint> {
    use rusqlite::OptionalExtension;
    let port: Option<i64> = db.conn().query_row("SELECT port FROM notify_endpoints WHERE instance = ?1 AND kind = 'inject'", [&record.name], |row| row.get(0)).optional().map_err(query_error)?;
    let port = port.map(|value| u16::try_from(value).ok().filter(|value| *value > 0).ok_or_else(|| query_error("invalid terminal endpoint port"))).transpose()?;
    Ok(TerminalEndpoint { participant: crate::agents::attachments::ParticipantRef { canonical_name: record.name.clone(), incarnation_bits: record.created_at.to_bits() }, record, port })
}

type Labeled = std::collections::BTreeMap<String, crate::agents::labels::LabelSnapshot>;

fn selected_labels(db: &AgentxDb, context: &RequestContext, filter: &AgentFilter) -> ServiceResult<Option<Labeled>> {
    if filter.labels.is_empty() {
        return Ok(None);
    }
    let (_, snapshots) = listing::select_labeled(db, context, &filter.labels, filter.workspace.as_selector())?;
    Ok(Some(snapshots))
}

fn matches_labels(labels: Option<&Labeled>, name: &str, incarnation: f64) -> bool {
    labels.is_none_or(|labels| labels.get(name).is_some_and(|snapshot| snapshot.participant.incarnation_bits == incarnation.to_bits()))
}

fn matching_terminals(db: &AgentxDb, context: &RequestContext, name: Option<&str>, filter: &AgentFilter) -> ServiceResult<Vec<TerminalEndpoint>> {
    let labels = selected_labels(db, context, filter)?;
    let names = if let Some(name) = name {
        vec![name.to_owned()]
    } else {
        let mut statement = db.conn().prepare("SELECT instance FROM notify_endpoints WHERE kind = 'inject' ORDER BY instance").map_err(query_error)?;
        statement.query_map([], |row| row.get::<_, String>(0)).map_err(query_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(query_error)?
    };
    let mut endpoints = Vec::new();
    for name in names {
        if let Some(record) = db.get_instance_full(&name).map_err(query_error)?
            && filter.matches_workspace(record.launch_context.as_deref())
            && matches_labels(labels.as_ref(), &name, record.created_at)
        {
            endpoints.push(terminal_endpoint(db, record)?);
        }
    }
    Ok(endpoints)
}

/// Run all snapshot queries against the connection.
fn load_all(db: &AgentxDb, context: &RequestContext, options: SnapshotOptions, filter: &AgentFilter) -> ServiceResult<DataState> {
    let labels = selected_labels(db, context, filter)?;
    let conn = db.conn();
    let device_uuid = read_device_uuid(conn);
    let now = epoch_now();

    let (mut agents, mut remote_agents) = load_instances(conn, &device_uuid, now, filter, labels.as_ref());

    // Compute unread counts
    compute_unread_batch(conn, &mut agents);

    // Load recently stopped (last 10 minutes from events)
    let stopped_agents = if labels.is_none() { load_recently_stopped(conn, now, &filter.workspace) } else { Vec::new() };

    let mut agents = with_info_references(db, context, agents);
    let stopped_agents = with_info_references(db, context, stopped_agents);

    // Load orphan processes from pidtrack file
    let orphans = if filter.is_unfiltered() { load_orphans(conn) } else { Vec::new() };

    // Load one shared timeline window, then split into message vs status/life
    // so both panes stay aligned to the same event-id/time range.
    let timeline_limit = options.timeline_limit;
    let (mut messages, mut events) = load_timeline(conn, timeline_limit);
    messages.sort_by(|a, b| a.time.total_cmp(&b.time));
    events.sort_by(|a, b| a.time.total_cmp(&b.time));

    // Read relay config flags once per snapshot — both relay_enabled and
    // load_relay_state need them, so reading config.toml twice would be
    // pointless I/O on the TUI's hot path.
    let (configured, enabled) = (options.relay_configured, options.relay_enabled);
    let relay_enabled = configured && enabled;

    // Sort agents by created_at descending (newest first)
    agents.sort_by(|a, b| b.created_at.total_cmp(&a.created_at));
    remote_agents.sort_by(|a, b| b.created_at.total_cmp(&a.created_at));

    let relay_health = load_relay_state(conn, configured, enabled);

    Ok(DataState { agents, remote_agents, stopped_agents, orphans, messages, events, relay_enabled, relay_health, search_results: None })
}

fn unscoped_info(target: info::Target) -> info::Reference {
    info::Reference { scope: Default::default(), generation: String::new(), target, legacy: None }
}

fn with_info_references(db: &AgentxDb, context: &RequestContext, mut agents: Vec<Agent>) -> Vec<Agent> {
    let scope = db.path().parent().expect("scoped database path");
    for reference in agents.iter_mut().filter_map(|agent| agent.info_reference.as_mut()) {
        reference.scope = scope.to_owned();
        reference.generation.clone_from(&context.database_generation);
    }
    agents
}

// ── Instance loading ────────────────────────────────────────────

/// Extract a string field from a JSON object, returning `default` if missing or non-string.
fn json_str<'a>(v: &'a serde_json::Value, key: &str, default: &'a str) -> &'a str {
    v.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}

fn parse_status(s: &str) -> Option<AgentStatus> {
    match s {
        "error" => Some(AgentStatus::Error),
        "active" => Some(AgentStatus::Active),
        "listening" => Some(AgentStatus::Listening),
        "blocked" => Some(AgentStatus::Blocked),
        "launching" => Some(AgentStatus::Launching),
        "inactive" => Some(AgentStatus::Inactive),
        _ => None,
    }
}

fn load_instances(conn: &Connection, device_uuid: &str, now: f64, filter: &AgentFilter, labels: Option<&Labeled>) -> (Vec<Agent>, Vec<Agent>) {
    let mut local = Vec::new();
    let mut remote = Vec::new();

    let sql = format!(
        "SELECT name, tool, status, status_context, status_detail,
                created_at, status_time, last_stop, tcp_mode,
                directory, last_event_id, origin_device_id,
                pid, session_id, background, terminal_preset_effective, transcript_path,
                CASE WHEN ?1 THEN {} END
         FROM instances i ORDER BY created_at DESC",
        crate::db::labels::WORKSPACE_MEMBER_SQL,
    );
    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return (local, remote),
    };

    let rows = match stmt.query_map([filter.workspace != AgentWorkspace::All], |row| {
        Ok((
            row.get::<_, String>(0)?,          // name (PK, never NULL)
            row.get::<_, String>(1)?,          // tool (never NULL)
            row.get::<_, String>(2)?,          // status (never NULL)
            row.get::<_, Option<String>>(3)?,  // status_context
            row.get::<_, Option<String>>(4)?,  // status_detail
            row.get::<_, f64>(5)?,             // created_at (never NULL)
            row.get::<_, Option<i64>>(6)?,     // status_time
            row.get::<_, Option<f64>>(7)?,     // last_stop (heartbeat, may be real or int)
            row.get::<_, Option<i64>>(8)?,     // tcp_mode
            row.get::<_, Option<String>>(9)?,  // directory
            row.get::<_, Option<i64>>(10)?,    // last_event_id
            row.get::<_, Option<String>>(11)?, // origin_device_id
            row.get::<_, Option<i64>>(12)?,    // pid
            row.get::<_, Option<String>>(13)?, // session_id
            row.get::<_, Option<i64>>(14)?,    // background (headless)
            row.get::<_, Option<String>>(15)?, // terminal_preset_effective
            row.get::<_, Option<String>>(16)?, // transcript_path
            row.get::<_, Option<String>>(17)?, // adopted workspace
        ))
    }) {
        Ok(r) => r,
        Err(_) => return (local, remote),
    };

    for row in rows {
        let row = match row {
            Ok(r) => r,
            Err(e) => {
                log_warn("tui", "db.instance_parse_error", &format!("{}", e));
                continue;
            }
        };
        let (name, tool_s, status_s, ctx, detail, created_at, status_time, last_stop, tcp_mode, directory, last_event_id, origin_device_id, pid, session_id, background, terminal_preset, transcript_path, workspace) = row;
        if !filter.workspace.visible(workspace.as_deref()) || !matches_labels(labels, &name, created_at) {
            continue;
        }

        let ctx = ctx.unwrap_or_default();
        let detail = detail.unwrap_or_default();
        let status_time = status_time.unwrap_or(0);
        let last_stop = last_stop.unwrap_or(0.0);
        let tcp_mode = tcp_mode.unwrap_or(0);
        let directory = directory.unwrap_or_default();
        let origin_device_id = origin_device_id.unwrap_or_default();
        let background = background.unwrap_or(0);

        let is_remote = !origin_device_id.is_empty() && origin_device_id != device_uuid;

        // Compute heartbeat: last_stop, fallback to status_time, fallback to created_at
        let heartbeat_epoch = if last_stop > 0.0 {
            last_stop
        } else if status_time > 0 {
            status_time as f64
        } else {
            created_at
        };

        let has_tcp = tcp_mode != 0;

        // Remote rows are stored namespaced as "base:SHORT" (SHORT = the origin
        // device's relay short-id). Split it back so display_name() re-appends
        // the real short-id rather than a UUID prefix that doesn't match.
        let (name, device_name) = if is_remote {
            match name.rsplit_once(':') {
                Some((base, short)) if !base.is_empty() && !short.is_empty() => (base.to_string(), Some(short.to_string())),
                // Fallback for an unexpectedly un-namespaced remote row.
                _ => (name, Some(origin_device_id.chars().take(4).collect::<String>().to_uppercase())),
            }
        } else {
            (name, None)
        };

        // Sync age for remote agents from KV
        let sync_age = if is_remote { get_device_sync_age(conn, &origin_device_id, now) } else { None };

        let terminal_preset = terminal_preset.filter(|s| !s.is_empty() && s != "default");

        // Build agent with raw DB values first.
        // Unknown statuses are treated as inactive to avoid falsely showing them as live.
        let parsed_status = parse_status(&status_s);
        let mut agent = Agent {
            info_reference: (!is_remote).then(|| unscoped_info(info::Target::Incarnation { name: name.clone(), bits: created_at.to_bits() })),
            name,
            tool: AgentTool::parse(&tool_s),
            status: parsed_status.unwrap_or(AgentStatus::Inactive),
            status_context: ctx,
            status_detail: detail,
            created_at,
            status_time: status_time as f64,
            last_heartbeat: heartbeat_epoch,
            has_tcp,
            directory,
            unread: 0, // computed separately
            last_event_id: last_event_id.map(|id| id as u64),
            device_name,
            sync_age,
            pid: pid.map(|p| p as u32),
            session_id: session_id.filter(|s| !s.is_empty()),
            transcript_path: transcript_path.filter(|s| !s.is_empty()),
            headless: background != 0,
            terminal_preset,
        };

        if parsed_status.is_none() {
            if agent.status_detail.is_empty() {
                agent.status_detail = format!("unknown status '{}'", status_s);
            }
            if agent.status_context.is_empty() {
                agent.status_context = "unknown_status".into();
            }
        }

        // Apply stale detection for local agents
        if !is_remote {
            let computed = status::compute_status(&agent, now);
            agent.status = computed.status;
            agent.status_context = computed.status_context;
            agent.status_detail = computed.status_detail;
        }

        if is_remote {
            remote.push(agent);
        } else {
            local.push(agent);
        }
    }

    (local, remote)
}

fn get_device_sync_age(conn: &Connection, device_id: &str, now: f64) -> Option<String> {
    let key = format!("relay_sync_time_{}", device_id);
    let ts: f64 = conn.query_row("SELECT value FROM kv WHERE key = ?", params![key], |row| row.get::<_, String>(0)).ok().and_then(|s| s.parse::<f64>().ok())?;

    let age = (now - ts).max(0.0) as u64;
    Some(format!("{} ago", format_duration_short(age)))
}

// ── Unread counts ───────────────────────────────────────────────

fn compute_unread_batch(conn: &Connection, agents: &mut [Agent]) {
    if agents.is_empty() {
        return;
    }

    // Find minimum waterline
    let min_waterline = agents.iter().filter_map(|a| a.last_event_id).min().unwrap_or(0);

    // Fetch all messages since minimum waterline
    let mut stmt = match conn.prepare("SELECT id, data FROM events WHERE id > ? AND type = 'message' ORDER BY id") {
        Ok(s) => s,
        Err(_) => return,
    };

    let msgs: Vec<(u64, String)> = stmt.query_map(params![min_waterline as i64], |row| Ok((row.get::<_, i64>(0)? as u64, row.get::<_, String>(1)?))).ok().map(|rows| rows.flatten().collect()).unwrap_or_default();

    let mut broadcast_ids: Vec<u64> = Vec::new();
    let mut broadcast_by_sender: std::collections::HashMap<String, Vec<u64>> = std::collections::HashMap::new();
    let mut mentions_by_recipient: std::collections::HashMap<String, Vec<u64>> = std::collections::HashMap::new();

    for (event_id, data) in &msgs {
        let json = match serde_json::from_str::<serde_json::Value>(data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let sender_kind = json_str(&json, "sender_kind", "agent");
        if sender_kind == "system" || sender_kind == "sys" {
            continue;
        }

        let from = json_str(&json, "from", "");
        let scope = json_str(&json, "scope", "broadcast");

        match scope {
            "broadcast" => {
                broadcast_ids.push(*event_id);
                broadcast_by_sender.entry(from.to_string()).or_default().push(*event_id);
            }
            "mentions" => {
                if let Some(arr) = json.get("mentions").and_then(|m| m.as_array()) {
                    for recipient in arr.iter().filter_map(|v| v.as_str()) {
                        if recipient == from {
                            continue;
                        }
                        mentions_by_recipient.entry(recipient.to_string()).or_default().push(*event_id);
                    }
                }
            }
            _ => {}
        }
    }

    for agent in agents.iter_mut() {
        let waterline = agent.last_event_id.unwrap_or(0);
        let mut count = count_gt(&broadcast_ids, waterline);
        if let Some(self_sent) = broadcast_by_sender.get(&agent.name) {
            count = count.saturating_sub(count_gt(self_sent, waterline));
        }
        if let Some(mentions) = mentions_by_recipient.get(&agent.name) {
            count += count_gt(mentions, waterline);
        }
        agent.unread = count;
    }
}

fn count_gt(sorted_ids: &[u64], waterline: u64) -> usize {
    let idx = sorted_ids.partition_point(|id| *id <= waterline);
    sorted_ids.len().saturating_sub(idx)
}

// ── Recently stopped ────────────────────────────────────────────

fn load_recently_stopped(conn: &Connection, now: f64, workspace: &AgentWorkspace) -> Vec<Agent> {
    load_stopped(conn, now, Some(crate::agents::lifecycle::RECENTLY_STOPPED_WINDOW), workspace)
}

/// Load stopped agents. `max_age_secs`: None = all time, Some(n) = last n seconds.
fn load_stopped(conn: &Connection, now: f64, max_age_secs: Option<f64>, workspace: &AgentWorkspace) -> Vec<Agent> {
    let cutoff = max_age_secs.map(|age| (now - age).max(0.0) as i64).unwrap_or(0);
    let row_limit = 512i64;
    let mut stmt = match conn.prepare(
        "SELECT instance, data, timestamp, id FROM events
         WHERE type = 'life'
           AND json_extract(data, '$.action') IN ('stopped', 'killed')
           AND unixepoch(timestamp) >= ?
         ORDER BY id DESC
         LIMIT ?",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows: Vec<(String, String, String, i64)> = stmt.query_map(params![cutoff, row_limit], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).ok().map(|r| r.flatten().collect()).unwrap_or_default();

    // Fetch all active instance names in one query
    let active_names: std::collections::HashSet<String> = conn.prepare("SELECT name FROM instances").ok().and_then(|mut stmt| stmt.query_map([], |row| row.get::<_, String>(0)).ok().map(|r| r.flatten().collect())).unwrap_or_default();

    // Deduplicate by name (keep most recent)
    let mut seen = std::collections::HashSet::new();
    let mut stopped = Vec::new();

    for (name, data, ts, event) in &rows {
        let stopped_at = parse_iso_to_epoch(ts);
        if stopped_at < cutoff as f64 {
            continue;
        }
        if !seen.insert(name.clone()) {
            continue;
        }
        if active_names.contains(name) {
            continue;
        }

        let json: serde_json::Value = serde_json::from_str(data).unwrap_or_default();
        let action = json_str(&json, "action", "stopped");
        let snapshot = json.get("snapshot").cloned().unwrap_or_default();
        if !workspace.visible(crate::agents::workspaces::workspace_from_context(snapshot["launch_context"].as_str()).as_deref()) {
            continue;
        }

        let tool_s = json_str(&snapshot, "tool", "claude");
        let directory = json_str(&snapshot, "directory", "");
        let recorded_creation = info::snapshot_creation(&snapshot);
        let created_at = recorded_creation.unwrap_or(now);

        stopped.push(Agent {
            info_reference: Some(unscoped_info(info::Target::Stopped { name: name.clone(), event: *event, bits: recorded_creation.map(f64::to_bits) })),
            name: name.clone(),
            tool: AgentTool::parse(tool_s),
            status: AgentStatus::Inactive,
            status_context: action.to_string(),
            status_detail: String::new(),
            created_at,
            status_time: stopped_at,
            last_heartbeat: stopped_at,
            has_tcp: false,
            directory: directory.to_string(),
            unread: 0,
            last_event_id: None,
            device_name: None,
            sync_age: None,
            headless: false,
            session_id: snapshot.get("session_id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(str::to_owned),
            transcript_path: snapshot.get("transcript_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(str::to_owned),
            pid: None,
            terminal_preset: None,
        });
    }

    stopped
}

// ── Orphan processes ────────────────────────────────────────────

/// Alive pidtrack entry with the fields needed to decide whether a live row owns it.
#[derive(Clone)]
struct TrackedPty {
    orphan: OrphanProcess,
    process_id: String,
    session_id: String,
}

/// 5-second TTL cache for pidtrack data to avoid excessive I/O in TUI polling.
static ORPHAN_CACHE: std::sync::Mutex<Option<(std::time::Instant, Vec<TrackedPty>)>> = std::sync::Mutex::new(None);
const ORPHAN_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(5);

fn load_orphans(conn: &Connection) -> Vec<OrphanProcess> {
    let cached = ORPHAN_CACHE.lock().ok().and_then(|guard| match *guard {
        Some((ts, ref cached)) if ts.elapsed() < ORPHAN_CACHE_TTL => Some(cached.clone()),
        _ => None,
    });
    let tracked = match cached {
        Some(tracked) => tracked,
        None => {
            // A missing or unreadable pidfile is retried on the next load.
            let Some(tracked) = read_tracked_ptys() else {
                return vec![];
            };
            if let Ok(mut guard) = ORPHAN_CACHE.lock() {
                *guard = Some((std::time::Instant::now(), tracked.clone()));
            }
            tracked
        }
    };

    // Ownership is checked on every call: the DB changes faster than the cache.
    let active_db_pids: Vec<u32> =
        conn.prepare("SELECT pid FROM instances WHERE pid IS NOT NULL").ok().and_then(|mut stmt| stmt.query_map([], |row| row.get::<_, i64>(0)).ok().map(|rows| rows.flatten().map(|p| p as u32).collect())).unwrap_or_default();
    tracked.into_iter().filter(|t| !active_db_pids.contains(&t.orphan.pid)).filter(|t| crate::runtime::process_tracking::owning_instance(conn, &t.process_id, &t.session_id).is_none()).map(|t| t.orphan).collect()
}

fn read_tracked_ptys() -> Option<Vec<TrackedPty>> {
    let content = std::fs::read_to_string(paths::pidtrack_path()).ok()?;
    let pidmap: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(&content).ok()?;

    let mut tracked = Vec::new();
    for (pid_str, info) in &pidmap {
        let pid: u32 = match pid_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        if !crate::runtime::process_tracking::is_alive(pid) {
            continue;
        }

        let tool_s = json_str(info, "tool", "claude");
        let names: Vec<String> = info.get("names").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        let launched_at = info.get("launched_at").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let directory = json_str(info, "directory", "").to_string();

        tracked.push(TrackedPty { orphan: OrphanProcess { pid, tool: AgentTool::parse(tool_s), names, launched_at, directory }, process_id: json_str(info, "process_id", "").to_string(), session_id: json_str(info, "session_id", "").to_string() });
    }
    Some(tracked)
}

// ── Timeline ────────────────────────────────────────────────────

/// Parse timeline rows into messages and events.
fn parse_timeline_rows(rows: Vec<(i64, String, String, String, String)>) -> (Vec<Message>, Vec<Event>) {
    let mut messages = Vec::new();
    let mut events = Vec::new();
    for (id, timestamp, instance, event_type, data) in rows {
        match event_type.as_str() {
            "message" => {
                if let Some(msg) = parse_message_row(id, &timestamp, &data) {
                    messages.push(msg);
                }
            }
            "status" | "life" => {
                if let Some(event) = parse_status_or_life_row(id, &timestamp, &instance, &event_type, &data) {
                    events.push(event);
                }
            }
            _ => {}
        }
    }
    (messages, events)
}

/// Query 5-column timeline rows from a prepared statement.
fn query_timeline_rows(stmt: &mut rusqlite::Statement, params: &[&dyn rusqlite::types::ToSql]) -> Vec<(i64, String, String, String, String)> {
    stmt.query_map(params, |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))).ok().map(|r| r.flatten().collect()).unwrap_or_default()
}

fn load_timeline(conn: &Connection, limit: usize) -> (Vec<Message>, Vec<Event>) {
    let mut stmt = match conn.prepare(
        "SELECT id, timestamp, instance, type, data FROM events
         WHERE type IN ('message', 'status', 'life')
         ORDER BY id DESC LIMIT ?",
    ) {
        Ok(s) => s,
        Err(_) => return (vec![], vec![]),
    };
    let limit_param = limit as i64;
    parse_timeline_rows(query_timeline_rows(&mut stmt, &[&limit_param]))
}

fn load_timeline_search(conn: &Connection, query: &str, limit: usize) -> (Vec<Message>, Vec<Event>) {
    // Strip double-quotes and wrap in quotes for FTS5 phrase search.
    // Empty after stripping (e.g. query was all quotes) → return empty; `""` is invalid FTS5.
    let stripped = query.replace('"', "");
    if stripped.is_empty() {
        return (vec![], vec![]);
    }
    let fts_query = format!("\"{}\"", stripped);
    let mut stmt = match conn.prepare(
        "SELECT e.id, e.timestamp, e.instance, e.type, e.data
         FROM events e
         JOIN events_fts ON events_fts.rowid = e.id
         WHERE events_fts MATCH ?
           AND e.type IN ('message', 'status', 'life')
         ORDER BY e.id DESC LIMIT ?",
    ) {
        Ok(s) => s,
        Err(_) => return (vec![], vec![]),
    };
    let limit_param = limit as i64;
    let (mut messages, mut events) = parse_timeline_rows(query_timeline_rows(&mut stmt, &[&fts_query as &dyn rusqlite::types::ToSql, &limit_param]));

    // Sort ascending by time (same order as load_timeline results in load_all)
    messages.sort_by(|a, b| a.time.total_cmp(&b.time));
    events.sort_by(|a, b| a.time.total_cmp(&b.time));

    (messages, events)
}

fn parse_message_row(id: i64, timestamp: &str, data: &str) -> Option<Message> {
    let json: serde_json::Value = serde_json::from_str(data).ok()?;

    let from = json_str(&json, "from", "?").to_string();
    let text = json_str(&json, "text", "").to_string();
    let sender_kind_s = json_str(&json, "sender_kind", "agent");

    let sender_kind = match sender_kind_s {
        "system" | "sys" => SenderKind::System,
        "user" | "human" | "external" => SenderKind::External,
        _ => SenderKind::Instance,
    };

    let recipients: Vec<String> = json.get("mentions").and_then(|m| m.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();

    let intent = json.get("intent").and_then(|v| v.as_str()).map(String::from);
    // Imported event IDs belong to the origin device, not this database.
    let reply_to = json.get("reply_to_local").filter(|_| json.get("_relay").is_none()).and_then(|v| v.as_u64());

    Some(Message { event_id: id as u64, sender: from, recipients, body: text, time: parse_iso_to_epoch(timestamp), sender_kind, intent, reply_to })
}

fn parse_status_or_life_row(id: i64, timestamp: &str, instance: &str, event_type: &str, data: &str) -> Option<Event> {
    let json: serde_json::Value = serde_json::from_str(data).ok()?;

    if event_type == "life" {
        let action = json_str(&json, "action", "unknown");
        let kind = match action {
            "ready" | "started" => ActivityKind::Started,
            "stopped" | "killed" => ActivityKind::Stopped,
            _ => ActivityKind::StateChange,
        };
        let reason = json_str(&json, "reason", "");
        let by = json_str(&json, "by", "");
        let mut detail_text = action.to_string();
        let mut sub_lines = Vec::new();
        match action {
            "stopped" | "killed" => {
                if !reason.is_empty() || !by.is_empty() {
                    let mut parts = Vec::new();
                    if !by.is_empty() {
                        parts.push(format!("by {}", by));
                    }
                    if !reason.is_empty() {
                        parts.push(reason.to_string());
                    }
                    detail_text = format!("{} ({})", action, parts.join(", "));
                }
                if let Some(snap) = json.get("snapshot") {
                    let tool = json_str(snap, "tool", "");
                    let dir = json_str(snap, "directory", "");
                    let session = json_str(snap, "session_id", "");
                    let pid = snap.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
                    let created_at = snap.get("created_at").and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0.0);
                    let event_time = parse_iso_to_epoch(timestamp);

                    let mut line1 = format!("tool: {}  pid: {}", tool, pid);
                    if created_at > 0.0 && event_time > created_at {
                        let dur = (event_time - created_at) as u64;
                        let dur_str = if dur < 60 {
                            format!("{}s", dur)
                        } else if dur < 3600 {
                            format!("{}m {}s", dur / 60, dur % 60)
                        } else {
                            format!("{}h {}m", dur / 3600, (dur % 3600) / 60)
                        };
                        line1.push_str(&format!("  ran: {}", dur_str));
                    }
                    sub_lines.push(line1);

                    if !dir.is_empty() {
                        sub_lines.push(format!("dir: {}", truncate_path(dir, 60)));
                    }
                    if !session.is_empty() {
                        sub_lines.push(format!("session: {}", session));
                    }
                    let mut extras = Vec::new();
                    let parent = json_str(snap, "parent_name", "");
                    if !parent.is_empty() {
                        extras.push(format!("parent: {}", parent));
                    }
                    if !extras.is_empty() {
                        sub_lines.push(extras.join(" | "));
                    }
                    sub_lines.push(format!("resume: agentx r {}", instance));
                }
            }
            "batch_launched" => {
                let tool_raw = json_str(&json, "tool", "");
                let tool = tool_raw.strip_suffix("-pty").unwrap_or(tool_raw);
                let count = json.get("launched").and_then(|v| v.as_u64()).unwrap_or(0);
                let mut text = format!("launched {}× {}", count, tool);
                if !by.is_empty() {
                    text.push_str(&format!(" (by {})", by));
                }
                detail_text = text;
            }
            "created" | "started" => {
                let mut parts = Vec::new();
                if !by.is_empty() {
                    parts.push(format!("by {}", by));
                }
                if !reason.is_empty() {
                    parts.push(reason.to_string());
                }
                if !parts.is_empty() {
                    detail_text = format!("{} ({})", action, parts.join(", "));
                }
            }
            "relay_device_join" | "relay_device_leave" => {
                // relay::ACTION_DEVICE_*
                let text = json_str(&json, "text", action);
                detail_text = text.to_string();
            }
            _ => {}
        }
        return Some(Event { row_id: id as u64, agent: instance.to_string(), time: parse_iso_to_epoch(timestamp), kind: EventKind::Activity(kind), tool: String::new(), detail: detail_text, sub_lines });
    }

    // status events
    let status = json_str(&json, "status", "active");
    let context = json_str(&json, "context", "");
    let detail = json_str(&json, "detail", "");

    let (kind, tool_name, detail_text) = if status == ST_ACTIVE && context.starts_with("tool:") {
        let t = context.strip_prefix("tool:").unwrap_or(context);
        // Suppress agentx send commands — the resulting message is already shown
        if detail.starts_with("agentx send") {
            return None;
        }
        (EventKind::Tool, t.to_string(), detail.to_string())
    } else {
        let activity = match status {
            "stopped" | "inactive" => ActivityKind::Stopped,
            "blocked" => ActivityKind::Blocked,
            "listening" => ActivityKind::Listening,
            "active" => ActivityKind::Active,
            _ => ActivityKind::StateChange,
        };
        let text = match (context.is_empty(), detail.is_empty()) {
            (true, true) => status.to_string(),
            (false, true) => context.to_string(),
            (true, false) => detail.to_string(),
            (false, false) => format!("{}: {}", context, detail),
        };
        (EventKind::Activity(activity), String::new(), text)
    };

    Some(Event { row_id: id as u64, agent: instance.to_string(), time: parse_iso_to_epoch(timestamp), kind, tool: tool_name, detail: detail_text, sub_lines: vec![] })
}

/// Truncate a path to max_len characters, replacing the middle with "...".
fn truncate_path(path: &str, max_len: usize) -> String {
    let chars: Vec<char> = path.chars().collect();
    if chars.len() <= max_len {
        return path.to_string();
    }
    let keep = max_len.saturating_sub(3) / 2;
    let head: String = chars[..keep].iter().collect();
    let tail: String = chars[chars.len() - keep..].iter().collect();
    format!("{}...{}", head, tail)
}

// ── Relay ───────────────────────────────────────────────────────

/// Read (configured, enabled) flags from config.toml. `configured` is true
/// when `relay.id` is a non-empty string; `enabled` is the `relay.enabled`
/// flag (defaults true when absent — matches AgentxConfig::default).
/// Read a non-empty string value from the kv table. Returns None if missing or empty.
fn kv_get(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM kv WHERE key = ?", params![key], |row| row.get(0)).ok().filter(|s: &String| !s.is_empty())
}

/// Build a RelayObservation from the TUI's raw connection + pidfile + caller-
/// supplied config flags, then derive RelayHealth. The TUI snapshot path
/// doesn't carry an AgentxConfig/AgentxDb pair, so we reconstruct the observation
/// locally rather than threading those types through every snapshot call.
/// `configured` and `enabled` are passed in so the caller can read config.toml
/// once per snapshot rather than us re-reading it here.
fn load_relay_state(conn: &Connection, configured: bool, enabled: bool) -> crate::relay::RelayHealth {
    let raw_status = kv_get(conn, "relay_status");
    let raw_error = kv_get(conn, "relay_last_error");
    let heartbeat_age_s = kv_get(conn, crate::relay::HEARTBEAT_KEY).and_then(|s| s.parse::<f64>().ok()).map(|ts| (crate::shared::time::now_epoch_f64() - ts).max(0.0));
    let pidfile = crate::relay::worker::observe_pid_file();

    let obs = crate::relay::RelayObservation { configured, enabled, raw_status, raw_error, heartbeat_age_s, pidfile, last_push: 0.0, broker: None };
    crate::relay::derive_relay_health(&obs)
}

#[cfg(test)]
mod tests {
    use super::{AgentFilter, AgentWorkspace, Request, Response, SnapshotOptions, compute_unread_batch, count_gt, handle, load_instances, load_recently_stopped, parse_message_row, parse_status_or_life_row};
    use crate::integrations::tool::Tool;
    use crate::tui::model::{ActivityKind, Agent, AgentStatus, AgentTool, EventKind, SenderKind};
    use rusqlite::Connection;

    fn diagnostics_context() -> crate::service::RequestContext {
        crate::service::RequestContext {
            caller: crate::service::CallerContext {
                request_id: crate::service::RequestId(uuid::Uuid::new_v4().to_string()),
                invocation: crate::service::InvocationContext::default(),
                deadline_unix_ms: None,
                expected_database_generation: Some("diagnostic-fixture".into()),
            },
            process: crate::service::CallerProcess { pid: std::process::id(), incarnation: Some("diagnostic-caller".into()) },
            daemon_instance: "fixture".into(),
            database_generation: "diagnostic-fixture".into(),
        }
    }

    #[test]
    fn terminal_and_dashboard_filters_match_saved_workspace_and_current_labels() {
        use crate::agents::{attachments::ParticipantRef, labels};
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let context = diagnostics_context();
        let stamp = f64::from_bits(1_793_000_000.0_f64.to_bits() + 7);
        for (name, member) in [("team", Some("/project")), ("other", Some("/other")), ("unassigned", None)] {
            let launch = serde_json::json!({"collaboration":{"workspace":member,"role":null,"endpoint":false}}).to_string();
            db.conn().execute("INSERT INTO instances(name,tool,created_at,directory,launch_context) VALUES(?1,'codex',?2,'/same-cwd',?3)", rusqlite::params![name, stamp, launch]).unwrap();
        }
        let participant = ParticipantRef { canonical_name: "team".into(), incarnation_bits: stamp.to_bits() };
        labels::handle(&db, &context, labels::Request::Patch(labels::PatchLabels { participant: participant.clone(), set: [("task".into(), "build".into())].into(), remove: vec![], if_revision: None })).unwrap();
        let options = SnapshotOptions { timeline_limit: 20, relay_configured: false, relay_enabled: false };
        let mut filter = AgentFilter { workspace: AgentWorkspace::Team("/project".into()), labels: vec![] };
        let snapshot = |filter| {
            let Response::Snapshot(snapshot) = handle(&db, &context, Request::FilteredSnapshot { options: options.clone(), filter }).unwrap() else { panic!("dashboard snapshot") };
            snapshot
        };
        let mut names = snapshot(filter.clone()).agents.into_iter().map(|agent| agent.name).collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, ["team", "unassigned"]);
        filter.labels.push(labels::parse_selector("task=build").unwrap());
        assert_eq!(snapshot(filter.clone()).agents.iter().map(|agent| agent.name.as_str()).collect::<Vec<_>>(), ["team"]);
        let terminals = |name: &str, filter: AgentFilter| {
            let Response::Terminals(endpoints) = handle(&db, &context, Request::MatchingTerminals { name: Some(name.into()), filter }).unwrap() else { panic!("terminal candidates") };
            endpoints
        };
        assert!(terminals("other", filter.clone()).is_empty());
        assert!(terminals("unassigned", filter.clone()).is_empty());
        assert_eq!(terminals("team", filter)[0].participant, participant);
        assert_eq!(snapshot(AgentFilter { workspace: AgentWorkspace::Global, labels: vec![] }).agents[0].name, "unassigned");
    }

    #[test]
    fn diagnostic_projection_keeps_status_counts_and_defers_log_evidence_to_host() {
        let directory = tempfile::tempdir().unwrap();
        let db = crate::db::AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
        let log = directory.path().join("launch.log");
        std::fs::write(&log, "originating-host-only-diagnostic").unwrap();
        let created = crate::shared::time::now_epoch_f64() - 120.0;
        db.conn().execute("INSERT INTO instances(name,tool,created_at,status,status_context,background_log_file) VALUES('pending','codex',?1,'inactive','new',?2)", rusqlite::params![created, log.to_string_lossy()]).unwrap();
        db.conn().execute("INSERT INTO instances(name,tool,created_at,status) VALUES('active','adhoc',?1,'active:work'),('blocked','adhoc',?1,'blocked'),('unknown','adhoc',?1,'future-status')", [crate::shared::time::now_epoch_f64()]).unwrap();
        let result = super::diagnostics(&db, &diagnostics_context()).unwrap();
        assert_eq!(result.counts.active, 1);
        assert_eq!(result.counts.blocked, 1);
        assert_eq!(result.counts.inactive, 2);
        assert_eq!(result.counts.total, 4);
        assert_eq!(result.launch_failures.len(), 1);
        let failure = &result.launch_failures[0];
        assert_eq!(failure.status_context, "launch_failed");
        assert!(!failure.status_detail.contains("originating-host-only-diagnostic"));
        assert!(crate::agents::lifecycle::capture_launch_failure_evidence(failure).unwrap().contains("originating-host-only-diagnostic"));
    }

    #[test]
    fn diagnostic_required_status_event_failure_rolls_back_cleanup_retention() {
        let directory = tempfile::tempdir().unwrap();
        let db = crate::db::AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
        let created = crate::shared::time::now_epoch_f64() - 120.0;
        db.conn().execute("INSERT INTO instances(name,tool,created_at,status,status_context) VALUES('pending','codex',?1,'inactive','new')", [created]).unwrap();
        let context = diagnostics_context();
        db.conn().execute_batch("CREATE TRIGGER reject_diagnostic_status BEFORE INSERT ON events WHEN NEW.type='status' BEGIN SELECT RAISE(ABORT,'status storage unavailable'); END;").unwrap();
        assert!(super::diagnostics(&db, &context).is_err());
        assert_eq!(db.get_instance_full("pending").unwrap().unwrap().status_context, "new");
        assert!(db.kv_prefix("participant_request:").unwrap().is_empty());
        db.conn().execute_batch("DROP TRIGGER reject_diagnostic_status;").unwrap();
        assert_eq!(super::diagnostics(&db, &context).unwrap().launch_failures.len(), 1);
    }

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE events (
                id INTEGER PRIMARY KEY,
                type TEXT NOT NULL,
                instance TEXT NOT NULL,
                data TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );
            CREATE TABLE instances (
                name TEXT PRIMARY KEY
            );
            ",
        )
        .unwrap();
        conn
    }

    #[test]
    fn parse_tool_preserves_unknown_persisted_value() {
        assert_eq!(AgentTool::parse("future-tool"), AgentTool::Unknown("future-tool".to_string()));
    }

    fn make_agent(name: &str, last_event_id: u64) -> Agent {
        Agent {
            info_reference: None,
            name: name.into(),
            tool: Tool::Claude.into(),
            status: AgentStatus::Active,
            status_context: String::new(),
            status_detail: String::new(),
            created_at: 0.0,
            status_time: 0.0,
            last_heartbeat: 0.0,
            has_tcp: false,
            directory: String::new(),
            unread: 0,
            last_event_id: Some(last_event_id),
            device_name: None,
            sync_age: None,
            headless: false,
            session_id: None,
            transcript_path: None,
            pid: None,
            terminal_preset: None,
        }
    }

    #[test]
    fn load_recently_stopped_filters_older_than_ten_minutes() {
        let conn = setup_conn();
        let now = chrono::DateTime::parse_from_rfc3339("2026-02-18T00:10:00+00:00").unwrap().timestamp() as f64;

        conn.execute("INSERT INTO events (id, type, instance, data, timestamp) VALUES (?, ?, ?, ?, ?)", rusqlite::params![1i64, "life", "olda", r#"{"action":"stopped","snapshot":{"tool":"claude"}}"#, "2026-02-17T23:59:00+00:00"]).unwrap();
        conn.execute("INSERT INTO events (id, type, instance, data, timestamp) VALUES (?, ?, ?, ?, ?)", rusqlite::params![2i64, "life", "reca", r#"{"action":"killed","snapshot":{"tool":"gemini"}}"#, "2026-02-18T00:09:30+00:00"]).unwrap();

        let stopped = load_recently_stopped(&conn, now, &AgentWorkspace::All);
        assert_eq!(stopped.len(), 1);
        assert_eq!(stopped[0].name, "reca");
        assert_eq!(stopped[0].status_time, now - 30.0);
        assert_eq!(stopped[0].last_heartbeat, now - 30.0);
    }

    #[test]
    fn load_recently_stopped_excludes_currently_active_instances() {
        let conn = setup_conn();
        let now = chrono::DateTime::parse_from_rfc3339("2026-02-18T00:10:00+00:00").unwrap().timestamp() as f64;

        conn.execute("INSERT INTO instances (name) VALUES (?)", rusqlite::params!["live"]).unwrap();
        conn.execute("INSERT INTO events (id, type, instance, data, timestamp) VALUES (?, ?, ?, ?, ?)", rusqlite::params![1i64, "life", "live", r#"{"action":"stopped","snapshot":{"tool":"claude"}}"#, "2026-02-18T00:09:30+00:00"]).unwrap();

        let stopped = load_recently_stopped(&conn, now, &AgentWorkspace::All);
        assert!(stopped.is_empty());
    }

    #[test]
    fn unread_batch_counts_mentions_and_broadcasts_without_self_messages() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE events (
                id INTEGER PRIMARY KEY,
                type TEXT NOT NULL,
                data TEXT NOT NULL
            );
            ",
        )
        .unwrap();

        conn.execute("INSERT INTO events (id, type, data) VALUES (1, 'message', ?)", rusqlite::params![r#"{"from":"aone","scope":"broadcast","sender_kind":"agent"}"#]).unwrap();
        conn.execute("INSERT INTO events (id, type, data) VALUES (2, 'message', ?)", rusqlite::params![r#"{"from":"sys","scope":"broadcast","sender_kind":"system"}"#]).unwrap();
        conn.execute("INSERT INTO events (id, type, data) VALUES (3, 'message', ?)", rusqlite::params![r#"{"from":"cone","scope":"mentions","mentions":["aone","btwo"],"sender_kind":"agent"}"#]).unwrap();
        conn.execute("INSERT INTO events (id, type, data) VALUES (4, 'message', ?)", rusqlite::params![r#"{"from":"btwo","scope":"broadcast","sender_kind":"agent"}"#]).unwrap();

        let mut agents = vec![make_agent("aone", 0), make_agent("btwo", 2)];
        compute_unread_batch(&conn, &mut agents);

        assert_eq!(agents[0].unread, 2, "aone: mention(3) + broadcast(4)");
        assert_eq!(agents[1].unread, 1, "btwo: mention(3), self broadcast(4) ignored");
    }

    #[test]
    fn unknown_status_defaults_to_inactive_in_instance_load() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE instances (
                name TEXT,
                tool TEXT,
                status TEXT,
                status_context TEXT,
                status_detail TEXT,
                created_at REAL,
                status_time INTEGER,
                last_stop INTEGER,
                tcp_mode INTEGER,
                directory TEXT,
                last_event_id INTEGER,
                origin_device_id TEXT,
                pid INTEGER,
                session_id TEXT,
                background INTEGER,
                terminal_preset_effective TEXT,
                transcript_path TEXT,
                launch_context TEXT
            );
            CREATE TABLE kv (key TEXT, value TEXT);
            ",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO instances (name, tool, status, status_context, status_detail, created_at, status_time, last_stop, tcp_mode, directory, last_event_id, origin_device_id, pid, session_id, background, terminal_preset_effective)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params!["nazo", "claude", "teleporting", "", "", 10.0f64, 0i64, 0i64, 0i64, "/tmp", 0i64, "", Option::<i64>::None, Option::<String>::None, 0i64, Option::<String>::None],
        )
        .unwrap();

        let (local, remote) = load_instances(&conn, "", 100.0, &AgentFilter::default(), None);
        assert!(remote.is_empty());
        assert_eq!(local.len(), 1);
        assert_eq!(local[0].status, AgentStatus::Inactive);
        assert_eq!(local[0].status_context, "unknown_status");
        assert!(local[0].status_detail.contains("teleporting"));
    }

    #[test]
    fn parse_message_row_defaults_missing_fields() {
        let msg = parse_message_row(7, "2026-02-18T00:09:30+00:00", "{}").unwrap();
        assert_eq!(msg.event_id, 7);
        assert_eq!(msg.sender, "?");
        assert_eq!(msg.body, "");
        assert_eq!(msg.sender_kind, SenderKind::Instance);
        assert!(msg.recipients.is_empty());
        assert!(msg.intent.is_none());
        assert!(msg.reply_to.is_none());
    }

    #[test]
    fn parse_message_row_maps_mentions_and_external_sender_kind() {
        let msg =
            parse_message_row(8, "2026-02-18T00:09:30+00:00", r#"{"from":"sys","text":"hi","scope":"mentions","sender_kind":"human","mentions":["nova"],"delivered_to":["nova"],"intent":"request","reply_to":"7:BOXE","reply_to_local":42}"#).unwrap();

        assert_eq!(msg.sender_kind, SenderKind::External);
        assert_eq!(msg.recipients, vec!["nova"]);
        assert_eq!(msg.intent.as_deref(), Some("request"));
        assert_eq!(msg.reply_to, Some(42));
    }

    #[test]
    fn imported_reply_does_not_use_origin_local_id() {
        let msg = parse_message_row(100, "2026-02-18T00:09:30+00:00", r#"{"from":"nova:BOXE","text":"reply","reply_to":"7:BOXE","reply_to_local":42,"_relay":{"device":"remote","short":"BOXE","id":8}}"#).unwrap();
        assert!(msg.reply_to.is_none());
    }

    #[test]
    fn parse_status_row_with_tool_context_creates_tool_event() {
        let ev = parse_status_or_life_row(9, "2026-02-18T00:09:30+00:00", "nova", "status", r#"{"status":"active","context":"tool:Read","detail":"src/lib.rs"}"#).unwrap();

        assert_eq!(ev.row_id, 9);
        assert_eq!(ev.agent, "nova");
        assert_eq!(ev.kind, EventKind::Tool);
        assert_eq!(ev.tool, "Read");
        assert_eq!(ev.detail, "src/lib.rs");
    }

    #[test]
    fn parse_life_stopped_event_includes_resume_subline() {
        let ev = parse_status_or_life_row(10, "2026-02-18T00:10:00+00:00", "nova", "life", r#"{"action":"stopped","reason":"idle","by":"bigboss","snapshot":{"tool":"claude","pid":1234,"directory":"/tmp/demo","created_at":1739837340.0}}"#).unwrap();

        assert_eq!(ev.kind, EventKind::Activity(ActivityKind::Stopped));
        assert!(ev.detail.contains("stopped"));
        assert!(ev.detail.contains("bigboss"));
        assert!(ev.sub_lines.iter().any(|l| l.starts_with("resume: agentx r nova")), "expected resume hint in sub-lines, got {:?}", ev.sub_lines);
    }

    #[test]
    fn count_gt_works_with_sorted_ids() {
        let ids = vec![2, 4, 9, 12];
        assert_eq!(count_gt(&ids, 0), 4);
        assert_eq!(count_gt(&ids, 4), 2);
        assert_eq!(count_gt(&ids, 12), 0);
    }
}
