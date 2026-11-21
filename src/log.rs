//! Structured JSONL logging with size-based rotation (~8MB, 3 backups).

use crate::config::Config;
use chrono::Utc;
use std::fs::{self, OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender};

/// Max log file size before rotation (8MB).
const MAX_BYTES: u64 = 8_000_000;

/// Default number of backup files to keep (.log.1, .log.2, .log.3).
const DEFAULT_BACKUPS: u32 = 3;

/// Read backup count from AGENTX_LOG_BACKUPS env, falling back to default.
fn log_backups() -> u32 {
    std::env::var("AGENTX_LOG_BACKUPS").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(DEFAULT_BACKUPS).min(32)
}

const LOG_FILE: &str = "agentx.log";

/// ISO 8601 timestamp for log entries.
fn timestamp_now() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Rotate log file if over size limit.
/// agentx.log -> agentx.log.1 -> agentx.log.2 -> agentx.log.3 (oldest deleted).
fn rotate_if_needed(path: &std::path::Path) {
    let size = match fs::metadata(path) {
        Ok(m) => m.len(),
        Err(_) => return,
    };
    if size <= MAX_BYTES {
        return;
    }

    if log_backups() == 0 {
        let _ = fs::remove_file(path);
        return;
    }
    // Shift older backups: .3 deleted, .2->.3, .1->.2
    let backups = log_backups();
    for i in (1..=backups).rev() {
        let older = path.with_file_name(format!("{}.{}", LOG_FILE, i));
        if i == backups {
            let _ = fs::remove_file(&older);
        } else if older.exists() {
            let _ = fs::rename(&older, path.with_file_name(format!("{}.{}", LOG_FILE, i + 1)));
        }
    }
    // Current -> .1
    let _ = fs::rename(path, path.with_file_name(format!("{}.1", LOG_FILE)));
}

/// Log a structured event to the agentx log file.
///
/// Uses manual serde_json writes for full control over field ordering and
/// optional fields.
/// plus optional structured fields.
pub fn log(level: &str, subsystem: &str, event: &str, message: &str) {
    log_with_fields(level, subsystem, event, message, &[]);
}

/// Log with additional structured key-value fields.
///
/// Fields are appended after the standard fields (ts, level, subsystem, event, instance, msg).
/// Empty values are omitted.
pub fn log_with_fields(level: &str, subsystem: &str, event: &str, message: &str, fields: &[(&str, &str)]) {
    let scope = crate::paths::agentx_dir();
    log_in_scope(&scope, level, subsystem, event, message, fields);
}

/// Write to the host's explicit scope, never a listener thread's ambient cwd/env.
/// Rotation and append share a stable cross-process lock; failures do not recurse.
/// Callers supply safe diagnostic fields, never prompts, tickets, bodies or environments.
pub(crate) fn log_in_scope(scope: &std::path::Path, level: &str, subsystem: &str, event: &str, message: &str, fields: &[(&str, &str)]) {
    append(scope, record(scope, level, subsystem, event, message, fields), false);
}

type LogRecord = serde_json::Map<String, serde_json::Value>;

/// Best-effort diagnostics for the daemon. The worker cannot delay dispatch
/// or shutdown when the filesystem is unresponsive.
#[derive(Clone)]
pub(crate) struct DispatchLog {
    scope: PathBuf,
    sender: SyncSender<LogRecord>,
}

impl DispatchLog {
    pub(crate) fn start(scope: &Path) -> Option<Self> {
        let (sender, receiver) = mpsc::sync_channel(128);
        let path = scope.to_owned();
        std::thread::Builder::new().name("agentx-dispatch-log".into()).spawn(move || {
            for entry in receiver {
                append(&path, entry, true);
            }
        }).ok()?;
        Some(Self { scope: scope.to_owned(), sender })
    }

    pub(crate) fn offer(&self, failed: bool, fields: &[(&str, &str)]) {
        let (level, event) = if failed { ("ERROR", "dispatch.failed") } else { ("WARN", "dispatch.slow") };
        self.offer_event(level, "daemon", event, "Native dispatch diagnostic", fields);
    }

    pub(crate) fn offer_event(&self, level: &str, subsystem: &str, event: &str, message: &str, fields: &[(&str, &str)]) {
        let _ = self.sender.try_send(record(&self.scope, level, subsystem, event, message, fields));
    }
}

/// One optional summary, no wait for another process's rotation or append.
pub(crate) fn hook_trace(scope: &std::path::Path, selection: &str, summary: serde_json::Value) {
    let level = if selection == "sampled" { "INFO" } else { "WARN" };
    let message = match selection {
        "failed" => "Hook processing failed; inspect the trace phases and native attempts",
        "slow" => "Hook exceeded its diagnostic latency threshold",
        _ => "Sampled hook timing",
    };
    let mut entry = record(scope, level, "hooks", "hook.trace", message, &[]);
    entry.insert("trace".into(), summary);
    append(scope, entry, true);
}

fn record(scope: &std::path::Path, level: &str, subsystem: &str, event: &str, message: &str, fields: &[(&str, &str)]) -> serde_json::Map<String, serde_json::Value> {
    let ts = timestamp_now();
    let instance = Config::get().instance_name.unwrap_or_default();

    // Build JSON manually for control over field ordering and optionals
    let mut obj = serde_json::Map::new();
    obj.insert("ts".into(), serde_json::Value::String(ts));
    obj.insert("level".into(), serde_json::Value::String(level.to_uppercase()));
    obj.insert("subsystem".into(), serde_json::Value::String(subsystem.into()));
    obj.insert("event".into(), serde_json::Value::String(event.into()));
    obj.insert("pid".into(), serde_json::json!(std::process::id()));
    obj.insert("scope".into(), serde_json::json!(scope));
    obj.insert("build".into(), serde_json::json!(crate::daemon::build_identity()));

    if !instance.is_empty() {
        obj.insert("instance".into(), serde_json::Value::String(instance));
    }

    if !message.is_empty() {
        obj.insert("msg".into(), serde_json::Value::String(message.into()));
    }

    // Additional structured fields
    for (key, value) in fields {
        if !value.is_empty() {
            obj.insert((*key).to_string(), serde_json::Value::String((*value).to_string()));
        }
    }

    obj
}

fn append(scope: &std::path::Path, record: serde_json::Map<String, serde_json::Value>, nonblocking: bool) {
    let Ok(mut log_line) = serde_json::to_vec(&record) else { return };
    log_line.push(b'\n');
    let directory = scope.join(".tmp/logs");
    if create_dir_all(&directory).is_err() || crate::sys::fs::set_private_dir(&directory).is_err() {
        return;
    }
    let lock_path = directory.join("agentx.log.lock");
    let mut options = OpenOptions::new();
    options.create(true).read(true).write(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let Ok(lock) = options.open(&lock_path) else { return };
    if crate::sys::fs::set_private(&lock_path).is_err() {
        return;
    }
    let locked = if nonblocking { lock.try_lock().is_ok() } else { crate::sys::fs::lock_exclusive(&lock).is_ok() };
    if !locked {
        return;
    }
    let path = directory.join(LOG_FILE);
    rotate_if_needed(&path);

    // One write per line: O_APPEND keeps a single small write whole, but
    // `writeln!` issues the line and its newline separately, which
    // interleaves with other processes (`…}{…` lines).
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    if let Ok(mut file) = options.open(&path) {
        let _ = crate::sys::fs::set_private(&path);
        let _ = file.write_all(&log_line);
    }
}

/// Log info message.
pub fn log_info(subsystem: &str, event: &str, message: &str) {
    log("INFO", subsystem, event, message);
}

/// Log warning message.
pub fn log_warn(subsystem: &str, event: &str, message: &str) {
    log("WARN", subsystem, event, message);
}

/// Log error message with structured error_type/error_msg fields.
/// Splits "TypeName: detail" into error_type + error_msg.
pub fn log_error(subsystem: &str, event: &str, message: &str) {
    // Split "ErrorType: message" into structured fields
    if let Some((error_type, error_msg)) = message.split_once(": ")
        && !error_type.contains(' ')
    {
        log_with_fields("ERROR", subsystem, event, message, &[("error_type", error_type), ("error_msg", error_msg)]);
        return;
    }
    log("ERROR", subsystem, event, message);
}

/// Get recent log entries filtered by level and time.
///
/// Returns entries newest-first, up to `limit`.
pub fn get_recent_logs(hours: f64, levels: &[&str], limit: usize) -> Vec<serde_json::Value> {
    get_recent_logs_in_scope(&crate::paths::agentx_dir(), hours, levels, limit)
}

/// Include retained rotations so a recent failure does not disappear after rollover.
pub(crate) fn get_recent_logs_in_scope(scope: &std::path::Path, hours: f64, levels: &[&str], limit: usize) -> Vec<serde_json::Value> {
    find_recent(scope, hours, limit, |entry| entry.get("level").and_then(|value| value.as_str()).is_some_and(|level| levels.contains(&level)))
}

pub(crate) fn recent_hook_traces(scope: &std::path::Path, trace_id: Option<&str>, limit: usize) -> Vec<serde_json::Value> {
    find_recent(scope, 24.0 * 365.0, limit, |entry| {
        let id = entry.pointer("/trace/trace_id").and_then(serde_json::Value::as_str);
        let event = entry.get("event").and_then(serde_json::Value::as_str);
        if event == Some("hook.trace") { id.is_some() && trace_id.is_none_or(|wanted| id == Some(wanted)) } else { trace_id.is_some() && entry.get("trace_id").and_then(serde_json::Value::as_str) == trace_id }
    })
}

fn find_recent(scope: &std::path::Path, hours: f64, limit: usize, matches: impl Fn(&serde_json::Value) -> bool) -> Vec<serde_json::Value> {
    use std::io::{BufRead, BufReader};
    if !hours.is_finite() || hours < 0.0 || limit == 0 {
        return vec![];
    }
    let hours = hours.min(24.0 * 365.0);
    let cutoff = Utc::now() - chrono::Duration::milliseconds((hours * 3_600_000.0) as i64);
    let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let path = scope.join(".tmp/logs").join(LOG_FILE);
    // Keep only requested results in memory while scanning retained rotations.
    let mut entries = std::collections::BTreeMap::new();
    let mut sequence = 0u64;
    for index in 0..=log_backups() {
        let selected = if index == 0 { path.clone() } else { path.with_file_name(format!("{LOG_FILE}.{index}")) };
        let Ok(file) = fs::File::open(selected) else { continue };
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
            let ts = entry.get("ts").and_then(|v| v.as_str()).unwrap_or("");
            if ts >= cutoff_str.as_str() && matches(&entry) {
                sequence += 1;
                entries.insert((ts.to_owned(), std::cmp::Reverse(sequence)), entry);
                if entries.len() > limit {
                    entries.pop_first();
                }
            }
        }
    }
    entries.into_iter().rev().map(|(_, entry)| entry).collect()
}

/// Get summary of recent log activity.
///
/// Returns JSON with error_count, warn_count, last_error, last_warn.
pub fn get_log_summary(hours: f64) -> serde_json::Value {
    let entries = get_recent_logs(hours, &["ERROR", "WARN"], 100);

    let errors: Vec<&serde_json::Value> = entries.iter().filter(|e| e.get("level").and_then(|v| v.as_str()) == Some("ERROR")).collect();
    let warns: Vec<&serde_json::Value> = entries.iter().filter(|e| e.get("level").and_then(|v| v.as_str()) == Some("WARN")).collect();

    let last_error = errors.first().map(|e| {
        serde_json::json!({
            "event": format!("{}.{}",
                e.get("subsystem").and_then(|v| v.as_str()).unwrap_or(""),
                e.get("event").and_then(|v| v.as_str()).unwrap_or("")
            ),
            "ts": e.get("ts"),
            "instance": e.get("instance"),
        })
    });

    let last_warn = warns.first().map(|e| {
        serde_json::json!({
            "event": format!("{}.{}",
                e.get("subsystem").and_then(|v| v.as_str()).unwrap_or(""),
                e.get("event").and_then(|v| v.as_str()).unwrap_or("")
            ),
            "ts": e.get("ts"),
            "instance": e.get("instance"),
        })
    });

    serde_json::json!({
        "error_count": errors.len(),
        "warn_count": warns.len(),
        "last_error": last_error,
        "last_warn": last_warn,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;

    #[test]
    fn test_rotate_if_needed_small_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOG_FILE);
        fs::write(&path, "small content").unwrap();
        rotate_if_needed(&path);
        assert!(path.exists()); // Should not rotate
    }

    #[test]
    fn test_rotate_if_needed_large_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOG_FILE);

        // Create a file just over MAX_BYTES
        let mut f = fs::File::create(&path).unwrap();
        let chunk = vec![b'x'; 1024];
        for _ in 0..(MAX_BYTES / 1024 + 1) {
            f.write_all(&chunk).unwrap();
        }
        f.flush().unwrap();
        drop(f);

        rotate_if_needed(&path);

        // Original should be gone, .1 should exist
        assert!(!path.exists());
        assert!(dir.path().join(format!("{}.1", LOG_FILE)).exists());
    }

    #[test]
    fn test_rotate_chain() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOG_FILE);

        // Create existing backups
        fs::write(dir.path().join(format!("{}.1", LOG_FILE)), "backup1").unwrap();
        fs::write(dir.path().join(format!("{}.2", LOG_FILE)), "backup2").unwrap();

        // Create oversized main log
        let mut f = fs::File::create(&path).unwrap();
        let chunk = vec![b'x'; 1024];
        for _ in 0..(MAX_BYTES / 1024 + 1) {
            f.write_all(&chunk).unwrap();
        }
        f.flush().unwrap();
        drop(f);

        rotate_if_needed(&path);

        // .1 was old backup1, now should be .2
        // .2 was old backup2, now should be .3
        // New .1 should be the old main log
        assert!(!path.exists());
        assert!(dir.path().join(format!("{}.1", LOG_FILE)).exists());
        assert!(dir.path().join(format!("{}.2", LOG_FILE)).exists());
        assert!(dir.path().join(format!("{}.3", LOG_FILE)).exists());

        // .2 should contain old backup1 content
        let content = fs::read_to_string(dir.path().join(format!("{}.2", LOG_FILE))).unwrap();
        assert_eq!(content, "backup1");

        // .3 should contain old backup2 content
        let content = fs::read_to_string(dir.path().join(format!("{}.3", LOG_FILE))).unwrap();
        assert_eq!(content, "backup2");
    }

    #[test]
    fn test_rotate_deletes_oldest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOG_FILE);

        // Create all 3 backups
        fs::write(dir.path().join(format!("{}.1", LOG_FILE)), "b1").unwrap();
        fs::write(dir.path().join(format!("{}.2", LOG_FILE)), "b2").unwrap();
        fs::write(dir.path().join(format!("{}.3", LOG_FILE)), "b3").unwrap();

        // Create oversized main log
        let mut f = fs::File::create(&path).unwrap();
        let chunk = vec![b'x'; 1024];
        for _ in 0..(MAX_BYTES / 1024 + 1) {
            f.write_all(&chunk).unwrap();
        }
        f.flush().unwrap();
        drop(f);

        rotate_if_needed(&path);

        // .3 (old b3) should be deleted, replaced by old .2 (b2)
        let content = fs::read_to_string(dir.path().join(format!("{}.3", LOG_FILE))).unwrap();
        assert_eq!(content, "b2");
    }

    #[test]
    fn test_get_recent_logs_empty() {
        let entries = get_recent_logs(1.0, &["ERROR", "WARN"], 20);
        // May or may not be empty depending on whether log file exists
        let _ = entries;
    }

    #[test]
    fn test_get_log_summary_structure() {
        let summary = get_log_summary(1.0);
        assert!(summary.get("error_count").is_some());
        assert!(summary.get("warn_count").is_some());
    }

    #[test]
    fn test_get_recent_logs_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOG_FILE);

        // Write some test log entries
        let ts = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let entries = [
            format!(r#"{{"ts":"{}","level":"ERROR","subsystem":"test","event":"e1","msg":"err"}}"#, ts),
            format!(r#"{{"ts":"{}","level":"WARN","subsystem":"test","event":"e2","msg":"warn"}}"#, ts),
            format!(r#"{{"ts":"{}","level":"INFO","subsystem":"test","event":"e3","msg":"info"}}"#, ts),
        ];
        fs::write(&path, entries.join("\n") + "\n").unwrap();

        // Read the file directly to test parsing
        let content = fs::read_to_string(&path).unwrap();
        let mut results: Vec<serde_json::Value> = Vec::new();
        for line in content.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line)
                && v.get("level").and_then(|l| l.as_str()) == Some("ERROR")
            {
                results.push(v);
            }
        }
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["event"], "e1");
    }
}

#[cfg(test)]
#[path = "log/tests.rs"]
mod retention_tests;
