//! Batch launch tracking and notification-driven launch confirmation.
//!
//! batch is ready, times out, or errors. Used by `agentx events --wait` and
//! the launcher to wait for readiness after `agentx launch --tool claude --count N`.

#[cfg(test)]
use std::{
    thread,
    time::{Duration, Instant},
};

use crate::agents::lifecycle as instance_lifecycle;
use crate::db::AgentxDb;
use crate::service::{ServiceError, ServiceResult};
use rusqlite::{OptionalExtension, params};
use std::collections::HashSet;

#[path = "launch_status_client.rs"]
mod client;
pub(crate) use client::wait_for_launch_client;

/// Result of a launch wait operation.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct LaunchResult {
    pub status: LaunchStatus,
    pub expected: Option<i64>,
    pub ready: Option<i64>,
    pub failed: Option<i64>,
    pub blocked: Option<i64>,
    pub instances: Vec<String>,
    pub failures: Vec<String>,
    pub blockers: Vec<String>,
    pub launcher: Option<String>,
    pub timestamp: Option<String>,
    pub batch_id: Option<String>,
    pub batches: Option<Vec<String>>,
    pub hint: Option<String>,
    pub message: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum LaunchStatus {
    Ready,
    Blocked,
    Timeout,
    Error,
    NoLaunches,
}

impl LaunchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LaunchStatus::Ready => "ready",
            LaunchStatus::Blocked => "blocked",
            LaunchStatus::Timeout => "timeout",
            LaunchStatus::Error => "error",
            LaunchStatus::NoLaunches => "no_launches",
        }
    }
}

/// Internal launch status data from DB queries.
struct LaunchData {
    expected: i64,
    ready: i64,
    failed: i64,
    blocked: i64,
    instances: Vec<String>,
    failures: Vec<String>,
    blockers: Vec<String>,
    launcher: String,
    timestamp: String,
    batch_id: Option<String>,
    batches: Option<Vec<String>>,
}

/// Count ready instances for a batch via 'ready' life events.
fn get_ready_for_batch(db: &AgentxDb, batch_id: &str) -> (i64, Vec<String>) {
    let conn = db.conn();
    let mut stmt = match conn.prepare(
        "SELECT instance FROM events \
         WHERE type = 'life' \
         AND json_extract(data, '$.action') = 'ready' \
         AND json_extract(data, '$.batch_id') = ?",
    ) {
        Ok(s) => s,
        Err(_) => return (0, vec![]),
    };
    let instances: Vec<String> = match stmt.query_map(rusqlite::params![batch_id], |row| row.get::<_, String>(0)) {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(_) => vec![],
    };
    let count = instances.len() as i64;
    (count, instances)
}

fn get_failed_for_batch(db: &AgentxDb, batch_id: &str, launcher: &str, batch_event_id: i64, ready_instances: &[String]) -> ServiceResult<(i64, Vec<String>)> {
    let mut seen = HashSet::new();
    let mut failures = launch_failed_events_for_batch(db, batch_id, &mut seen);
    let ready: HashSet<&str> = ready_instances.iter().map(String::as_str).collect();

    for name in get_batch_instance_names(db, batch_id) {
        if seen.contains(&name) {
            continue;
        }

        if let Some(inst) = db.get_instance_full(&name).map_err(internal)?
            && let Some(detail) = instance_lifecycle::try_finalize_launch_failure_detail_stored(db, &inst, None)?
        {
            // Keep batch-scoped failure observation atomic with required
            // stored diagnosis. Host evidence only enriches presentation.
            emit_row_finalized_event(db, &name, launcher, batch_id, &detail)?;
            seen.insert(name.clone());
            failures.push(format!("{name}: {detail}"));
            continue;
        }

        if ready.contains(name.as_str()) {
            continue;
        }

        if let Some(detail) = stopped_detail_for_instance(db, &name, batch_event_id) {
            seen.insert(name.clone());
            failures.push(format!("{name}: {detail}"));
        }
    }

    let count = failures.len() as i64;
    Ok((count, failures))
}

/// Record a batch failure inside ObserveLaunch's daemon write scope. Launcher
/// and batch are explicit observations, never selected from daemon environment.
fn emit_row_finalized_event(db: &AgentxDb, name: &str, launcher: &str, batch_id: &str, detail: &str) -> ServiceResult<()> {
    let event_data = serde_json::json!({
        "action": "launch_failed",
        "by": launcher,
        "status": "inactive",
        "context": "launch_failed",
        "reason": "row_finalized",
        "detail": detail,
        "batch_id": batch_id,
    });
    db.log_event("life", name, &event_data).map_err(internal)?;
    Ok(())
}

/// Publish once after the exact native terminal host has been validated.
/// Readiness does not overwrite the thread's independent turn status.
pub(super) fn record_native_ready(db: &AgentxDb, participant: &crate::agents::attachments::ParticipantRef, environment: &std::collections::HashMap<String, String>) -> ServiceResult<Option<(String, String)>> {
    let (Some(launcher), Some(batch)) = (environment.get("AGENTX_LAUNCHED_BY"), environment.get("AGENTX_LAUNCH_BATCH_ID")) else { return Ok(None) };
    let name = &participant.canonical_name;
    let phase = format!("delivery_launch:{name}:{}", participant.incarnation_bits);
    let error = |error: anyhow::Error| ServiceError::Internal(error.to_string());
    if matches!(db.kv_get(&phase).map_err(error)?.as_deref(), Some("ready" | "failed")) {
        return Ok(None);
    }
    db.log_event("life", name, &serde_json::json!({"action":"ready","by":launcher,"batch_id":batch,"context":"native:terminal_ready"})).map_err(error)?;
    db.kv_set(&phase, Some("ready")).map_err(error)?;
    Ok(Some((launcher.clone(), batch.clone())))
}

fn get_blocked_for_batch(db: &AgentxDb, batch_id: &str) -> (i64, Vec<String>) {
    let conn = db.conn();
    let mut stmt = match conn.prepare(
        "SELECT instance, json_extract(data, '$.detail') FROM events \
         WHERE type = 'life' \
         AND json_extract(data, '$.action') = 'launch_blocked' \
         AND json_extract(data, '$.batch_id') = ?",
    ) {
        Ok(s) => s,
        Err(_) => return (0, vec![]),
    };
    let blockers: Vec<String> = match stmt.query_map(params![batch_id], |row| {
        let name: String = row.get(0)?;
        let detail: Option<String> = row.get(1).ok();
        Ok(format!("{}: {}", name, detail.unwrap_or_else(|| "launch blocked".to_string())))
    }) {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(_) => vec![],
    };
    let count = blockers.len() as i64;
    (count, blockers)
}

fn launch_failed_events_for_batch(db: &AgentxDb, batch_id: &str, seen: &mut HashSet<String>) -> Vec<String> {
    let conn = db.conn();
    let mut stmt = match conn.prepare(
        "SELECT instance, json_extract(data, '$.reason'), json_extract(data, '$.detail') FROM events \
         WHERE type = 'life' \
         AND json_extract(data, '$.action') = 'launch_failed' \
         AND json_extract(data, '$.batch_id') = ?",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    match stmt.query_map(rusqlite::params![batch_id], |row| {
        let name: String = row.get(0)?;
        let reason: Option<String> = row.get(1).ok();
        let detail: Option<String> = row.get(2).ok();
        let text = detail.filter(|s| !s.is_empty()).or(reason.filter(|s| !s.is_empty())).unwrap_or_else(|| "launch failed".to_string());
        Ok(format!("{name}: {text}"))
    }) {
        Ok(rows) => rows
            .filter_map(|r| r.ok())
            .filter(|failure| {
                let Some((name, _)) = failure.split_once(": ") else {
                    return true;
                };
                seen.insert(name.to_string())
            })
            .collect(),
        Err(_) => vec![],
    }
}

fn stopped_detail_for_instance(db: &AgentxDb, name: &str, batch_event_id: i64) -> Option<String> {
    // Filter by monotonic event id instead of a string timestamp — format-
    // agnostic and not coupled to RFC3339 remaining lexicographically
    // sortable.
    let mut stmt = db
        .conn()
        .prepare(
            "SELECT json_extract(data, '$.by'), json_extract(data, '$.reason') FROM events \
             WHERE type = 'life' \
               AND instance = ?1 \
               AND json_extract(data, '$.action') = 'stopped' \
               AND id >= ?2 \
             ORDER BY id ASC LIMIT 1",
        )
        .ok()?;

    stmt.query_row(params![name, batch_event_id], |row| {
        let by: Option<String> = row.get(0).ok();
        let reason: Option<String> = row.get(1).ok();
        let mut detail = "launch stopped before it remained ready".to_string();
        if let Some(reason) = reason.filter(|s| !s.is_empty()) {
            detail.push_str(": ");
            detail.push_str(&reason);
        }
        if let Some(by) = by.filter(|s| !s.is_empty()) {
            detail.push_str(" by ");
            detail.push_str(&by);
        }
        Ok(detail)
    })
    .ok()
}

fn get_batch_instance_names(db: &AgentxDb, batch_id: &str) -> Vec<String> {
    let conn = db.conn();
    let data_str: String = match conn.query_row(
        "SELECT data FROM events
         WHERE type = 'life'
           AND json_extract(data, '$.action') = 'batch_launched'
           AND json_extract(data, '$.batch_id') = ?
         ORDER BY id DESC
         LIMIT 1",
        params![batch_id],
        |row| row.get(0),
    ) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let data: serde_json::Value = match serde_json::from_str(&data_str) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    data.get("instances").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(ToString::to_string)).collect()).unwrap_or_default()
}

fn get_batch_failure_details_for_ids(db: &AgentxDb, batch_ids: &[String]) -> ServiceResult<Vec<String>> {
    let mut details = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for batch_id in batch_ids {
        for name in get_batch_instance_names(db, batch_id) {
            if !seen.insert(name.clone()) {
                continue;
            }
            let Some(inst) = db.get_instance_full(&name).map_err(internal)? else {
                continue;
            };
            if let Some(detail) = instance_lifecycle::try_finalize_launch_failure_detail_stored(db, &inst, None)?.or_else(|| instance_lifecycle::get_launch_blocker_detail_stored(&inst)) {
                details.push(format!("{}: {}", name, detail));
            }
        }
    }

    Ok(details)
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

/// Aggregate multiple batches into a single LaunchData.
fn aggregate_batches(batches: &[BatchInfo], launcher: &str) -> LaunchData {
    let total_expected: i64 = batches.iter().map(|b| b.expected).sum();
    let total_ready: i64 = batches.iter().map(|b| b.ready).sum();
    let total_failed: i64 = batches.iter().map(|b| b.failed).sum();
    let total_blocked: i64 = batches.iter().map(|b| b.blocked).sum();
    let mut all_instances = Vec::new();
    let mut all_failures = Vec::new();
    let mut all_blockers = Vec::new();
    for b in batches {
        all_instances.extend(b.instances.clone());
        all_failures.extend(b.failures.clone());
        all_blockers.extend(b.blockers.clone());
    }
    let batch_ids: Vec<String> = batches.iter().map(|b| b.batch_id.clone()).collect();
    LaunchData {
        expected: total_expected,
        ready: total_ready,
        failed: total_failed,
        blocked: total_blocked,
        instances: all_instances,
        failures: all_failures,
        blockers: all_blockers,
        launcher: launcher.to_string(),
        timestamp: batches.first().map(|b| b.timestamp.clone()).unwrap_or_default(),
        batch_id: None,
        batches: Some(batch_ids),
    }
}

/// Per-batch info used during aggregation.
struct BatchInfo {
    batch_id: String,
    launcher: String,
    expected: i64,
    ready: i64,
    failed: i64,
    blocked: i64,
    instances: Vec<String>,
    failures: Vec<String>,
    blockers: Vec<String>,
    timestamp: String,
}

/// Launch timeout in seconds — batches older than this are considered stale.
const LAUNCH_TIMEOUT_SECONDS: i64 = 60;

/// Query aggregated launch status across all pending/recent batches.
///
/// Queries batch_launched events, gets ready counts from 'ready' life events,
/// and aggregates.
fn get_launch_status(db: &AgentxDb, launcher: Option<&str>) -> ServiceResult<Option<LaunchData>> {
    let conn = db.conn();

    let (sql, params): (String, Vec<String>) = if let Some(name) = launcher {
        (
            "SELECT id, timestamp, instance as launcher, \
                    json_extract(data, '$.batch_id') as batch_id, \
                    json_extract(data, '$.launched') as expected \
             FROM events \
             WHERE type = 'life' \
               AND json_extract(data, '$.action') = 'batch_launched' \
               AND instance = ?1 \
             ORDER BY id DESC LIMIT 20"
                .to_string(),
            vec![name.to_string()],
        )
    } else {
        (
            "SELECT id, timestamp, instance as launcher, \
                    json_extract(data, '$.batch_id') as batch_id, \
                    json_extract(data, '$.launched') as expected \
             FROM events \
             WHERE type = 'life' \
               AND json_extract(data, '$.action') = 'batch_launched' \
             ORDER BY id DESC LIMIT 20"
                .to_string(),
            vec![],
        )
    };

    let mut stmt = conn.prepare(&sql).map_err(internal)?;
    let launches: Vec<(i64, String, String, String, i64)> = if params.is_empty() {
        stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, i64>(4).unwrap_or(0)))).map_err(internal)?.collect::<Result<_, _>>().map_err(internal)?
    } else {
        stmt.query_map(rusqlite::params![params[0]], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, i64>(4).unwrap_or(0))))
            .map_err(internal)?
            .collect::<Result<_, _>>()
            .map_err(internal)?
    };

    if launches.is_empty() {
        return Ok(None);
    }

    // Build batch info with ready counts
    let mut batches: Vec<BatchInfo> = Vec::new();
    for (batch_event_id, ts, lnchr, batch_id, expected) in &launches {
        let (ready_count, ready_instances) = get_ready_for_batch(db, batch_id);
        let (failed_count, failures) = get_failed_for_batch(db, batch_id, lnchr, *batch_event_id, &ready_instances)?;
        let (blocked_count, blockers) = get_blocked_for_batch(db, batch_id);
        batches.push(BatchInfo { batch_id: batch_id.clone(), launcher: lnchr.clone(), expected: *expected, ready: ready_count, failed: failed_count, blocked: blocked_count, instances: ready_instances, failures, blockers, timestamp: ts.clone() });
    }

    let effective_launcher = launcher.map(String::from).unwrap_or_else(|| batches[0].launcher.clone());

    // Cutoff for "recent" launches
    let now = chrono::Utc::now().timestamp();
    let cutoff = now - LAUNCH_TIMEOUT_SECONDS;

    // Parse timestamp to epoch for comparison
    let ts_epoch = |ts: &str| -> i64 { chrono::DateTime::parse_from_rfc3339(ts).or_else(|_| chrono::DateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S%.fZ")).map(|dt| dt.timestamp()).unwrap_or(0) };

    // Priority 1: pending batches (ready < expected) that are recent
    let pending: Vec<&BatchInfo> = batches.iter().filter(|b| b.ready + b.failed + b.blocked < b.expected && ts_epoch(&b.timestamp) > cutoff).collect();
    if !pending.is_empty() {
        let owned: Vec<BatchInfo> = pending
            .into_iter()
            .map(|b| BatchInfo {
                batch_id: b.batch_id.clone(),
                launcher: b.launcher.clone(),
                expected: b.expected,
                ready: b.ready,
                failed: b.failed,
                blocked: b.blocked,
                instances: b.instances.clone(),
                failures: b.failures.clone(),
                blockers: b.blockers.clone(),
                timestamp: b.timestamp.clone(),
            })
            .collect();
        return Ok(Some(aggregate_batches(&owned, &effective_launcher)));
    }

    // Priority 2: batches from last 60s
    let recent: Vec<&BatchInfo> = batches.iter().filter(|b| ts_epoch(&b.timestamp) > cutoff).collect();
    if !recent.is_empty() {
        let owned: Vec<BatchInfo> = recent
            .into_iter()
            .map(|b| BatchInfo {
                batch_id: b.batch_id.clone(),
                launcher: b.launcher.clone(),
                expected: b.expected,
                ready: b.ready,
                failed: b.failed,
                blocked: b.blocked,
                instances: b.instances.clone(),
                failures: b.failures.clone(),
                blockers: b.blockers.clone(),
                timestamp: b.timestamp.clone(),
            })
            .collect();
        return Ok(Some(aggregate_batches(&owned, &effective_launcher)));
    }

    // Priority 3: most recent batch
    let first = &batches[0];
    Ok(Some(LaunchData {
        expected: first.expected,
        ready: first.ready,
        failed: first.failed,
        blocked: first.blocked,
        instances: first.instances.clone(),
        failures: first.failures.clone(),
        blockers: first.blockers.clone(),
        launcher: effective_launcher,
        timestamp: first.timestamp.clone(),
        batch_id: Some(first.batch_id.clone()),
        batches: None,
    }))
}

/// Query batch status by ID prefix.
///
/// Sums expected across matching
/// batch_launched events, counts ready from 'ready' life events.
fn get_launch_batch(db: &AgentxDb, batch_id: &str) -> ServiceResult<Option<LaunchData>> {
    let conn = db.conn();

    // Get aggregated launch info for this batch_id prefix
    let mut stmt = conn
        .prepare(
            "SELECT MIN(id) as batch_event_id, \
                MIN(timestamp) as timestamp, \
                instance as launcher, \
                json_extract(data, '$.batch_id') as batch_id, \
                SUM(json_extract(data, '$.launched')) as expected \
         FROM events \
         WHERE type = 'life' \
           AND json_extract(data, '$.action') = 'batch_launched' \
           AND json_extract(data, '$.batch_id') LIKE ?1 \
         GROUP BY json_extract(data, '$.batch_id')",
        )
        .map_err(internal)?;

    let like_pattern = format!("{}%", batch_id);
    let row: Option<(i64, String, String, String, i64)> =
        stmt.query_row(rusqlite::params![like_pattern], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, i64>(4).unwrap_or(0)))).optional().map_err(internal)?;

    let Some((batch_event_id, timestamp, launcher, resolved_batch_id, expected)) = row else {
        return Ok(None);
    };

    let (ready_count, ready_instances) = get_ready_for_batch(db, &resolved_batch_id);
    let (failed_count, failures) = get_failed_for_batch(db, &resolved_batch_id, &launcher, batch_event_id, &ready_instances)?;
    let (blocked_count, blockers) = get_blocked_for_batch(db, &resolved_batch_id);

    Ok(Some(LaunchData { expected, ready: ready_count, failed: failed_count, blocked: blocked_count, instances: ready_instances, failures, blockers, launcher, timestamp, batch_id: Some(resolved_batch_id), batches: None }))
}

/// Each waiter gets its own endpoint, including anonymous and concurrent waits.
/// Drop removes it on every return path; missed notifications still use polling.
#[cfg(test)]
struct LaunchWaiter<'a> {
    db: &'a AgentxDb,
    name: String,
    server: crate::runtime::notify::NotifyServer,
}

#[cfg(test)]
impl<'a> LaunchWaiter<'a> {
    fn new(db: &'a AgentxDb, batch_id: Option<&str>) -> Option<Self> {
        let server = crate::runtime::notify::NotifyServer::new().ok()?;
        // LIKE wildcards are accepted by the existing batch query; keep those
        // waits aggregate rather than treating a wildcard as a literal prefix.
        let scope = batch_id.filter(|id| !id.contains(['%', '_'])).unwrap_or("");
        let pid = std::process::id();
        let owner = serde_json::json!({
            "token": uuid::Uuid::new_v4().to_string(),
            "pid": pid,
            "process_identity": crate::sys::process::identity(pid),
            "batch_prefix": scope,
        });
        let name = format!("launch-wait:{owner}");
        db.upsert_notify_endpoint(&name, "launch_wait", server.port()).ok()?;
        Some(Self { db, name, server })
    }
}

#[cfg(test)]
impl Drop for LaunchWaiter<'_> {
    fn drop(&mut self) {
        let _ = self.db.delete_notify_endpoint(&self.name, "launch_wait");
    }
}

/// Block until launch batch is ready, times out, or errors.
///
/// Args:
///   db: Database handle.
///   launcher: Instance name of the launcher (for aggregated lookup).
///   batch_id: Specific batch ID (takes priority over launcher).
///   timeout_secs: Max seconds to wait (default 30).
///
/// Returns LaunchResult with status and batch details.
#[cfg(test)]
pub fn wait_for_launch(db: &AgentxDb, launcher: Option<&str>, batch_id: Option<&str>, timeout_secs: u64) -> LaunchResult {
    // Legacy borrowed-DB waiter is retained only for existing domain fixtures.
    // Product waits use the typed client and execute returned cleanup locally.
    instance_lifecycle::cleanup_stale_placeholders(db);
    let waiter = LaunchWaiter::new(db, batch_id);
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let mut observed = false;
    loop {
        let result = match observe_launch(db, launcher, batch_id, timeout_secs) {
            Ok(result) => result,
            Err(error) => return client::error_result(format!("Launch status unavailable: {error}")),
        };
        if result.status == LaunchStatus::NoLaunches && observed {
            return client::error_result("Launch data disappeared (DB reset or pruned)".into());
        }
        if result.status != LaunchStatus::Timeout || start.elapsed() >= timeout {
            return result;
        }
        observed = true;
        let wait_time = timeout.saturating_sub(start.elapsed()).min(Duration::from_millis(500));
        if let Some(waiter) = &waiter {
            waiter.server.wait(wait_time);
        } else {
            thread::sleep(wait_time);
        }
    }
}

/// Instantaneous stored diagnosis. The caller's adapter owns cleanup admission,
/// notification registration and all log/terminal I/O after this returns.
pub(crate) fn observe_launch(db: &AgentxDb, launcher: Option<&str>, batch_id: Option<&str>, timeout_secs: u64) -> ServiceResult<LaunchResult> {
    let status_data = if let Some(id) = batch_id { get_launch_batch(db, id)? } else { get_launch_status(db, launcher)? };
    let Some(status_data) = status_data else {
        return Ok(LaunchResult {
            status: LaunchStatus::NoLaunches,
            expected: None,
            ready: None,
            failed: None,
            blocked: None,
            instances: vec![],
            failures: vec![],
            blockers: vec![],
            launcher: None,
            timestamp: None,
            batch_id: None,
            batches: None,
            hint: None,
            message: Some(if launcher.is_some() { "You haven't launched any instances" } else { "No launches found" }.into()),
        });
    };

    let has_failure = status_data.failed > 0;
    let has_blocked = status_data.blocked > 0;
    let is_timeout = status_data.ready + status_data.failed + status_data.blocked < status_data.expected;

    let hint = if has_failure {
        let mut hint = format!("Launch failed: {}/{} ready, {} failed (batch: {}).", status_data.ready, status_data.expected, status_data.failed, status_data.batch_id.as_deref().unwrap_or("?"));
        if !status_data.failures.is_empty() {
            hint.push_str(" Failed instances: ");
            hint.push_str(&status_data.failures.join("; "));
        }
        Some(hint)
    } else if has_blocked {
        let mut hint = format!("Launch blocked: {}/{} ready, {} blocked (batch: {}).", status_data.ready, status_data.expected, status_data.blocked, status_data.batch_id.as_deref().unwrap_or("?"));
        if !status_data.blockers.is_empty() {
            hint.push_str(" Blocked instances: ");
            hint.push_str(&status_data.blockers.join("; "));
        }
        Some(hint)
    } else if is_timeout {
        let batch_ids: Vec<String> = if let Some(ref batches) = status_data.batches {
            batches.clone()
        } else if let Some(ref batch_id) = status_data.batch_id {
            vec![batch_id.clone()]
        } else {
            vec![]
        };
        let batch_display = batch_ids.first().map(|s| s.as_str()).unwrap_or("?");
        let mut hint = format!(
            "Launch failed: {}/{} ready after {}s (batch: {}). \
             Check ~/.agentx/.tmp/logs/background_*.log or agentx list -v",
            status_data.ready, status_data.expected, timeout_secs, batch_display
        );
        let failures = get_batch_failure_details_for_ids(db, &batch_ids)?;
        if !failures.is_empty() {
            hint.push_str(" Failed instances: ");
            hint.push_str(&failures.join("; "));
        }
        Some(hint)
    } else {
        None
    };

    Ok(LaunchResult {
        status: if has_failure {
            LaunchStatus::Error
        } else if has_blocked {
            LaunchStatus::Blocked
        } else if is_timeout {
            LaunchStatus::Timeout
        } else {
            LaunchStatus::Ready
        },
        expected: Some(status_data.expected),
        ready: Some(status_data.ready),
        failed: Some(status_data.failed),
        blocked: Some(status_data.blocked),
        instances: status_data.instances,
        failures: status_data.failures,
        blockers: status_data.blockers,
        launcher: Some(status_data.launcher),
        timestamp: Some(status_data.timestamp),
        batch_id: status_data.batch_id,
        batches: status_data.batches,
        hint,
        message: None,
    })
}

/// Freeze only records from the observed batches. Deleted placeholders use the
/// retained cleanup record, never a fresh lookup of an old participant's name.
pub(crate) fn diagnostic_records(db: &AgentxDb, result: &LaunchResult, cleanup: &crate::agents::service::CleanupOutcome) -> ServiceResult<Vec<crate::db::InstanceRow>> {
    if matches!(result.status, LaunchStatus::Ready | LaunchStatus::NoLaunches) {
        return Ok(Vec::new());
    }
    let batches = result.batches.clone().unwrap_or_else(|| result.batch_id.clone().into_iter().collect());
    let mut seen = HashSet::new();
    let mut records = Vec::new();
    for batch in batches {
        for name in get_batch_instance_names(db, &batch) {
            if !seen.insert(name.clone()) {
                continue;
            }
            // Cleanup freezes the prior incarnation before deleting it. Do not
            // replace its evidence with a namesake created in the meantime.
            let record = if let Some(retired) = cleanup.effects.iter().find(|effect| effect.participant.canonical_name == name) { Some(retired.record.clone()) } else { db.get_instance_full(&name).map_err(internal)? };
            if let Some(record) = record {
                records.push(record);
            }
        }
    }
    Ok(records)
}

/// Serialize LaunchResult to JSON for CLI output.
impl LaunchResult {
    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert("status".into(), serde_json::Value::String(self.status.as_str().into()));

        if let Some(expected) = self.expected {
            obj.insert("expected".into(), serde_json::json!(expected));
        }
        if let Some(ready) = self.ready {
            obj.insert("ready".into(), serde_json::json!(ready));
        }
        if let Some(failed) = self.failed {
            obj.insert("failed".into(), serde_json::json!(failed));
        }
        if let Some(blocked) = self.blocked {
            obj.insert("blocked".into(), serde_json::json!(blocked));
        }
        if !self.instances.is_empty() {
            obj.insert("instances".into(), serde_json::json!(self.instances));
        }
        if !self.failures.is_empty() {
            obj.insert("failures".into(), serde_json::json!(self.failures));
        }
        if !self.blockers.is_empty() {
            obj.insert("blockers".into(), serde_json::json!(self.blockers));
        }
        if let Some(ref launcher) = self.launcher {
            obj.insert("launcher".into(), serde_json::json!(launcher));
        }
        if let Some(ref ts) = self.timestamp {
            obj.insert("timestamp".into(), serde_json::json!(ts));
        }
        if let Some(ref bid) = self.batch_id {
            obj.insert("batch_id".into(), serde_json::json!(bid));
        }
        if let Some(ref batches) = self.batches {
            obj.insert("batches".into(), serde_json::json!(batches));
        }
        if let Some(ref hint) = self.hint {
            obj.insert("hint".into(), serde_json::json!(hint));
        }
        if self.status == LaunchStatus::Timeout {
            obj.insert("timed_out".into(), serde_json::json!(true));
        }
        if let Some(ref msg) = self.message {
            obj.insert("message".into(), serde_json::json!(msg));
        }

        serde_json::Value::Object(obj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_test_db() -> (AgentxDb, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db = AgentxDb::open_raw(&dir.path().join("test.db")).unwrap();
        db.init_db().unwrap();
        (db, dir)
    }

    #[test]
    fn concurrent_launch_waiters_receive_queued_events_and_clean_up() {
        let (db, _dir) = make_test_db();
        {
            let first = LaunchWaiter::new(&db, None).unwrap();
            let second = LaunchWaiter::new(&db, None).unwrap();
            assert_ne!(first.name, second.name);
            for action in ["ready", "launch_failed", "launch_blocked", "launch_blocked_cleared", "stopped"] {
                db.log_event("life", "mari", &serde_json::json!({"action": action})).unwrap();
                assert!(first.server.wait(Duration::from_secs(2)));
                assert!(second.server.wait(Duration::from_secs(2)));
            }
            db.log_event("status", "mari", &serde_json::json!({"context": "launch_failed"})).unwrap();
            assert!(first.server.wait(Duration::from_secs(2)));
            assert!(second.server.wait(Duration::from_secs(2)));
            db.log_event("status", "mari", &serde_json::json!({"status": "active"})).unwrap();
            assert!(!first.server.wait(Duration::ZERO));
            assert!(!second.server.wait(Duration::ZERO));
        }
        let count: i64 = db.conn().query_row("SELECT COUNT(*) FROM notify_endpoints WHERE kind = 'launch_wait'", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn launch_notifications_route_to_batch_prefixes_and_aggregate_waiters() {
        let (db, _dir) = make_test_db();
        let aggregate = LaunchWaiter::new(&db, None).unwrap();
        let matching = LaunchWaiter::new(&db, Some("BATCH-a")).unwrap();
        let unrelated = LaunchWaiter::new(&db, Some("batch-b")).unwrap();
        let wildcard = LaunchWaiter::new(&db, Some("batch-%")).unwrap();
        for action in ["ready", "launch_failed", "launch_blocked"] {
            db.log_event("life", "mari", &serde_json::json!({"action": action, "batch_id": "batch-a123"})).unwrap();
            assert!(aggregate.server.wait(Duration::from_secs(2)));
            assert!(matching.server.wait(Duration::from_secs(2)));
            assert!(wildcard.server.wait(Duration::from_secs(2)));
            assert!(!unrelated.server.wait(Duration::ZERO));
        }
        // Stopped/failure fallback events may lack batch metadata. They must
        // still wake scoped waiters so launch failures are observed promptly.
        db.log_event("life", "mari", &serde_json::json!({"action": "stopped"})).unwrap();
        for waiter in [&aggregate, &matching, &unrelated, &wildcard] {
            assert!(waiter.server.wait(Duration::from_secs(2)));
        }
    }

    #[test]
    fn launch_wait_observes_ready_and_cleans_up() {
        let (db, _dir) = make_test_db();
        db.log_event(
            "life",
            "leku",
            &serde_json::json!({
                "action": "batch_launched", "batch_id": "wake-test", "launched": 1, "instances": ["mari"]
            }),
        )
        .unwrap();
        let path = db.path().to_owned();
        let writer = std::thread::spawn(move || {
            let db = AgentxDb::open_at(&path).unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let count: i64 = db.conn().query_row("SELECT COUNT(*) FROM notify_endpoints WHERE kind = 'launch_wait'", [], |r| r.get(0)).unwrap();
                if count > 0 {
                    break;
                }
                assert!(Instant::now() < deadline, "waiter never registered");
                std::thread::sleep(Duration::from_millis(5));
            }
            // Let the waiter enter its wait after registration.
            std::thread::sleep(Duration::from_millis(40));
            db.log_event("life", "mari", &serde_json::json!({"action": "ready", "batch_id": "wake-test"})).unwrap();
        });
        let result = wait_for_launch(&db, None, Some("wake-test"), 5);
        writer.join().unwrap();
        assert_eq!(result.status, LaunchStatus::Ready);
        let count: i64 = db.conn().query_row("SELECT COUNT(*) FROM notify_endpoints WHERE kind = 'launch_wait'", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn launch_wait_falls_back_when_writer_does_not_notify() {
        let (db, _dir) = make_test_db();
        db.log_event("life", "leku", &serde_json::json!({"action": "batch_launched", "batch_id": "fallback-test", "launched": 1})).unwrap();
        let path = db.path().to_owned();
        let writer = std::thread::spawn(move || {
            let db = AgentxDb::open_at(&path).unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let count: i64 = db.conn().query_row("SELECT COUNT(*) FROM notify_endpoints WHERE kind = 'launch_wait'", [], |r| r.get(0)).unwrap();
                if count > 0 {
                    break;
                }
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(5));
            }
            db.conn().execute("INSERT INTO events (timestamp, type, instance, data) VALUES (?, 'life', 'mari', ?)", rusqlite::params![crate::db::chrono_now_iso(), r#"{"action":"ready","batch_id":"fallback-test"}"#]).unwrap();
        });
        assert_eq!(wait_for_launch(&db, None, Some("fallback-test"), 3).status, LaunchStatus::Ready);
        writer.join().unwrap();
    }

    #[test]
    fn launch_wait_cleans_up_no_launches_and_zero_timeout() {
        let (db, _dir) = make_test_db();
        assert_eq!(wait_for_launch(&db, None, None, 0).status, LaunchStatus::NoLaunches);
        db.log_event("life", "leku", &serde_json::json!({"action": "batch_launched", "batch_id": "zero-test", "launched": 1})).unwrap();
        assert_eq!(wait_for_launch(&db, None, Some("zero-test"), 0).status, LaunchStatus::Timeout);
        let count: i64 = db.conn().query_row("SELECT COUNT(*) FROM notify_endpoints WHERE kind = 'launch_wait'", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_launch_status_as_str() {
        assert_eq!(LaunchStatus::Ready.as_str(), "ready");
        assert_eq!(LaunchStatus::Blocked.as_str(), "blocked");
        assert_eq!(LaunchStatus::Timeout.as_str(), "timeout");
        assert_eq!(LaunchStatus::Error.as_str(), "error");
        assert_eq!(LaunchStatus::NoLaunches.as_str(), "no_launches");
    }

    #[test]
    fn test_launch_result_to_json_no_launches() {
        let result = LaunchResult {
            status: LaunchStatus::NoLaunches,
            expected: None,
            ready: None,
            failed: None,
            blocked: None,
            instances: vec![],
            failures: vec![],
            blockers: vec![],
            launcher: None,
            timestamp: None,
            batch_id: None,
            batches: None,
            hint: None,
            message: Some("No launches found".into()),
        };
        let json = result.to_json();
        assert_eq!(json["status"], "no_launches");
        assert_eq!(json["message"], "No launches found");
    }

    #[test]
    fn test_launch_result_to_json_ready() {
        let result = LaunchResult {
            status: LaunchStatus::Ready,
            expected: Some(3),
            ready: Some(3),
            failed: Some(0),
            blocked: Some(0),
            instances: vec!["luna".into(), "nova".into(), "peso".into()],
            failures: vec![],
            blockers: vec![],
            launcher: Some("bigboss".into()),
            timestamp: Some("2024-01-01T00:00:00Z".into()),
            batch_id: Some("batch-123".into()),
            batches: None,
            hint: None,
            message: None,
        };
        let json = result.to_json();
        assert_eq!(json["status"], "ready");
        assert_eq!(json["expected"], 3);
        assert_eq!(json["ready"], 3);
        assert_eq!(json["instances"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_launch_result_to_json_timeout() {
        let result = LaunchResult {
            status: LaunchStatus::Timeout,
            expected: Some(3),
            ready: Some(1),
            failed: Some(0),
            blocked: Some(0),
            instances: vec!["luna".into()],
            failures: vec![],
            blockers: vec![],
            launcher: Some("bigboss".into()),
            timestamp: Some("2024-01-01T00:00:00Z".into()),
            batch_id: Some("batch-123".into()),
            batches: None,
            hint: Some("Launch failed: 1/3 ready after 30s (batch: batch-123). Check ~/.agentx/.tmp/logs/background_*.log or agentx list -v".into()),
            message: None,
        };
        let json = result.to_json();
        assert_eq!(json["status"], "timeout");
        assert_eq!(json["timed_out"], true);
        assert!(json["hint"].as_str().unwrap().contains("Launch failed"));
    }

    #[test]
    fn test_wait_for_launch_returns_error_on_launch_failed_event() {
        let (db, _dir) = make_test_db();

        db.log_event(
            "life",
            "leku",
            &serde_json::json!({
                "action": "batch_launched",
                "batch_id": "batch-fail",
                "launched": 1,
                "instances": ["mari"]
            }),
        )
        .unwrap();
        db.log_event(
            "life",
            "mari",
            &serde_json::json!({
                "action": "launch_failed",
                "batch_id": "batch-fail",
                "reason": "ready_never_observed",
                "detail": "readiness was never observed"
            }),
        )
        .unwrap();

        let result = wait_for_launch(&db, None, Some("batch-fail"), 1);
        assert_eq!(result.status, LaunchStatus::Error);
        assert_eq!(result.ready, Some(0));
        assert_eq!(result.failed, Some(1));
        assert_eq!(result.failures, vec!["mari: readiness was never observed".to_string()]);
        let json = result.to_json();
        assert_eq!(json["status"], "error");
        assert!(json.get("timed_out").is_none());
    }

    #[test]
    fn test_wait_for_launch_counts_status_context_launch_failed() {
        let (db, _dir) = make_test_db();

        let mut data = serde_json::Map::new();
        data.insert("status".into(), serde_json::json!("inactive"));
        data.insert("status_context".into(), serde_json::json!("launch_failed"));
        data.insert("status_detail".into(), serde_json::json!("tool startup failed"));
        data.insert("created_at".into(), serde_json::json!(crate::shared::time::now_epoch_i64()));
        db.save_instance_named("mari", &data).unwrap();

        db.log_event(
            "life",
            "leku",
            &serde_json::json!({
                "action": "batch_launched",
                "batch_id": "batch-row-fail",
                "launched": 1,
                "instances": ["mari"]
            }),
        )
        .unwrap();

        let result = wait_for_launch(&db, None, Some("batch-row-fail"), 1);
        assert_eq!(result.status, LaunchStatus::Error);
        assert_eq!(result.ready, Some(0));
        assert_eq!(result.failed, Some(1));
        assert_eq!(result.failures, vec!["mari: tool startup failed".to_string()]);
    }

    #[test]
    fn test_wait_for_launch_returns_blocked_on_launch_blocked_event() {
        let (db, _dir) = make_test_db();

        db.log_event(
            "life",
            "leku",
            &serde_json::json!({
                "action": "batch_launched",
                "batch_id": "batch-blocked",
                "launched": 1,
                "instances": ["mari"]
            }),
        )
        .unwrap();
        db.log_event(
            "life",
            "mari",
            &serde_json::json!({
                "action": "launch_blocked",
                "batch_id": "batch-blocked",
                "detail": "launch blocked: run agentx term mari"
            }),
        )
        .unwrap();

        let result = wait_for_launch(&db, None, Some("batch-blocked"), 1);
        assert_eq!(result.status, LaunchStatus::Blocked);
        assert_eq!(result.ready, Some(0));
        assert_eq!(result.failed, Some(0));
        assert_eq!(result.blocked, Some(1));
        assert_eq!(result.blockers, vec!["mari: launch blocked: run agentx term mari".to_string()]);
        let json = result.to_json();
        assert_eq!(json["status"], "blocked");
        assert_eq!(json["blocked"], 1);
    }

    #[test]
    fn test_wait_for_launch_ignores_stopped_after_ready() {
        let (db, _dir) = make_test_db();

        db.log_event(
            "life",
            "leku",
            &serde_json::json!({
                "action": "batch_launched",
                "batch_id": "batch-stopped",
                "launched": 1,
                "instances": ["mari"]
            }),
        )
        .unwrap();
        db.log_event(
            "life",
            "mari",
            &serde_json::json!({
                "action": "ready",
                "batch_id": "batch-stopped",
                "status": "listening",
                "context": "ready_observed"
            }),
        )
        .unwrap();
        db.log_event(
            "life",
            "mari",
            &serde_json::json!({
                "action": "stopped",
                "by": "pty",
                "reason": "closed"
            }),
        )
        .unwrap();

        let result = wait_for_launch(&db, None, Some("batch-stopped"), 1);
        assert_eq!(result.status, LaunchStatus::Ready);
        assert_eq!(result.ready, Some(1));
        assert_eq!(result.failed, Some(0));
        assert!(result.failures.is_empty());
    }

    #[test]
    fn test_wait_for_launch_counts_stopped_before_ready_as_failed() {
        let (db, _dir) = make_test_db();

        db.log_event(
            "life",
            "leku",
            &serde_json::json!({
                "action": "batch_launched",
                "batch_id": "batch-stopped-before-ready",
                "launched": 1,
                "instances": ["mari"]
            }),
        )
        .unwrap();
        db.log_event(
            "life",
            "mari",
            &serde_json::json!({
                "action": "stopped",
                "by": "pty",
                "reason": "closed"
            }),
        )
        .unwrap();

        let result = wait_for_launch(&db, None, Some("batch-stopped-before-ready"), 1);
        assert_eq!(result.status, LaunchStatus::Error);
        assert_eq!(result.ready, Some(0));
        assert_eq!(result.failed, Some(1));
        assert_eq!(result.failures, vec!["mari: launch stopped before it remained ready: closed by pty".to_string()]);
    }

    #[test]
    fn test_wait_for_launch_does_not_finalize_fresh_placeholder() {
        let (db, _dir) = make_test_db();

        let mut data = serde_json::Map::new();
        data.insert("status".into(), serde_json::json!("inactive"));
        data.insert("status_context".into(), serde_json::json!("new"));
        data.insert("created_at".into(), serde_json::json!(crate::shared::time::now_epoch_i64()));
        data.insert("status_time".into(), serde_json::json!(0));
        data.insert("tool".into(), serde_json::json!("claude"));
        db.save_instance_named("mari", &data).unwrap();

        db.log_event(
            "life",
            "leku",
            &serde_json::json!({
                "action": "batch_launched",
                "batch_id": "batch-fresh-placeholder",
                "launched": 1,
                "instances": ["mari"]
            }),
        )
        .unwrap();

        // This checks timeout finalization, not polling elapsed time. An
        // immediate deadline exercises the same finalization path.
        let result = wait_for_launch(&db, None, Some("batch-fresh-placeholder"), 0);
        assert_eq!(result.status, LaunchStatus::Timeout);
        assert_eq!(result.ready, Some(0));
        assert_eq!(result.failed, Some(0));
        assert!(result.failures.is_empty());

        let stored = db.get_instance_full("mari").unwrap().unwrap();
        assert_eq!(stored.status_context, "new");
    }

    #[test]
    fn test_get_batch_failure_details_uses_batch_instances_and_status_detail() {
        let (db, _dir) = make_test_db();

        let mut data = serde_json::Map::new();
        data.insert("status".into(), serde_json::json!("inactive"));
        data.insert("status_context".into(), serde_json::json!("launch_failed"));
        data.insert("created_at".into(), serde_json::json!(crate::shared::time::now_epoch_i64()));
        data.insert(
            "status_detail".into(),
            serde_json::json!("Error: Operation not permitted (os error 1) Fully reset tmux first (`tmux kill-server`), then start a fresh tmux server with approval/escalation (for example: `tmux new-session -d -s agentx-external`), then retry."),
        );
        db.save_instance_named("mari", &data).unwrap();

        db.log_event(
            "life",
            "leku",
            &serde_json::json!({
                "action": "batch_launched",
                "batch_id": "batch-123",
                "launched": 1,
                "instances": ["mari"]
            }),
        )
        .unwrap();

        let details = get_batch_failure_details_for_ids(&db, &["batch-123".to_string()]).unwrap();
        assert_eq!(
            details,
            vec![
                "mari: Error: Operation not permitted (os error 1) Fully reset tmux first (`tmux kill-server`), then start a fresh tmux server with approval/escalation (for example: `tmux new-session -d -s agentx-external`), then retry.".to_string()
            ]
        );
    }

    #[test]
    fn test_get_batch_failure_details_finalizes_new_instance() {
        let (db, _dir) = make_test_db();

        let mut data = serde_json::Map::new();
        data.insert("status".into(), serde_json::json!("inactive"));
        data.insert("status_context".into(), serde_json::json!("new"));
        data.insert("created_at".into(), serde_json::json!(crate::shared::time::now_epoch_i64() - crate::agents::lifecycle::LAUNCH_PLACEHOLDER_TIMEOUT - 1));
        data.insert("tool".into(), serde_json::json!("codex"));
        data.insert("launch_context".into(), serde_json::json!(r#"{"terminal_preset":"tmux","pane_id":""}"#));
        db.save_instance_named("mari", &data).unwrap();

        db.log_event(
            "life",
            "leku",
            &serde_json::json!({
                "action": "batch_launched",
                "batch_id": "batch-456",
                "launched": 1,
                "instances": ["mari"]
            }),
        )
        .unwrap();

        let details = get_batch_failure_details_for_ids(&db, &["batch-456".to_string()]).unwrap();
        assert_eq!(details.len(), 1);
        let detail = details[0].strip_prefix("mari: exited before binding (observed after ").and_then(|value| value.strip_suffix("s)")).and_then(|value| value.parse::<i64>().ok()).expect("failure detail should include the observed age in seconds");
        assert!(detail > crate::agents::lifecycle::LAUNCH_PLACEHOLDER_TIMEOUT);

        let stored = db.get_instance_full("mari").unwrap().unwrap();
        assert_eq!(stored.status_context, "launch_failed");
        assert_eq!(stored.status_detail, details[0].strip_prefix("mari: ").unwrap());
    }
}
