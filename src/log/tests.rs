//! Independent persistence cases use explicit disposable scopes, never process-global logging paths.
use super::*;
use serde_json::{Value, json};

#[test]
fn runtime_read_logs_query_rotations_filter_sort_and_isolate_scope() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let logs_dir = root.path().join(".tmp/logs");
    let other_logs = other.path().join(".tmp/logs");
    fs::create_dir_all(&logs_dir).unwrap();
    fs::create_dir_all(&other_logs).unwrap();
    let now = Utc::now();
    let entry = |seconds: i64, level: &str, event: &str| json!({"ts":(now - chrono::Duration::seconds(seconds)).format("%Y-%m-%dT%H:%M:%SZ").to_string(),"level":level,"event":event}).to_string();
    fs::write(logs_dir.join("agentx.log.1"), format!("{}\nmalformed\n{}\n{}\n", entry(20, "ERROR", "older"), entry(7200, "ERROR", "expired"), entry(1, "INFO", "filtered"))).unwrap();
    fs::write(logs_dir.join("agentx.log.2"), format!("{}\n", entry(10, "WARN", "newer-backup"))).unwrap();
    fs::write(other_logs.join("agentx.log"), format!("{}\n", entry(0, "ERROR", "other-scope-secret"))).unwrap();
    // Rotation remains queryable even before a new current file exists.
    let logs = get_recent_logs_in_scope(root.path(), 1.0, &["ERROR", "WARN"], 10);
    assert_eq!(logs.iter().map(|value| value["event"].as_str().unwrap()).collect::<Vec<_>>(), ["newer-backup", "older"]);
    fs::write(logs_dir.join("agentx.log"), format!("{}\n", entry(0, "ERROR", "current"))).unwrap();
    let logs = get_recent_logs_in_scope(root.path(), 1.0, &["ERROR", "WARN"], 2);
    assert_eq!(logs.iter().map(|value| value["event"].as_str().unwrap()).collect::<Vec<_>>(), ["current", "newer-backup"]);
    assert!(get_recent_logs_in_scope(root.path(), 1.0, &["ERROR"], 0).is_empty());
    assert_eq!(get_recent_logs_in_scope(other.path(), 1.0, &["ERROR"], 10)[0]["event"], "other-scope-secret");
}

#[test]
fn runtime_read_logs_limited_query_preserves_equal_time_order() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join(".tmp/logs");
    fs::create_dir_all(&directory).unwrap();
    let timestamp = timestamp_now();
    let entry = |event: &str| json!({"ts":timestamp,"level":"ERROR","event":event}).to_string();
    fs::write(directory.join("agentx.log"), format!("{}\n{}\n", entry("current-first"), entry("current-second"))).unwrap();
    fs::write(directory.join("agentx.log.1"), format!("{}\n", entry("backup-first"))).unwrap();
    let limited = get_recent_logs_in_scope(root.path(), 1.0, &["ERROR"], 2);
    assert_eq!(limited.iter().map(|value| value["event"].as_str().unwrap()).collect::<Vec<_>>(), ["current-first", "current-second"]);
    let complete = get_recent_logs_in_scope(root.path(), 1.0, &["ERROR"], 3);
    assert_eq!(complete.iter().map(|value| value["event"].as_str().unwrap()).collect::<Vec<_>>(), ["current-first", "current-second", "backup-first"]);
}

#[test]
fn runtime_read_logs_explicit_scope_persists_private_process_diagnostics() {
    let root = tempfile::tempdir().unwrap();
    let scope = root.path().join("nested");
    log_in_scope(&scope, "error", "management", "worker_failure", "bounded diagnostic", &[("stage", "spawn"), ("generation", "fixture-generation"), ("empty", "")]);
    let path = scope.join(".tmp/logs/agentx.log");
    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 1);
    let value: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(value["level"], "ERROR");
    assert_eq!(value["stage"], "spawn");
    assert_eq!(value["generation"], "fixture-generation");
    assert!(value.get("empty").is_none());
    assert_eq!(value["pid"].as_u64().or_else(|| value["pid"].as_str().and_then(|v| v.parse().ok())), Some(u64::from(std::process::id())));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    }
}

#[test]
fn runtime_read_logs_rotate_with_bounded_retention() {
    let root = tempfile::tempdir().unwrap();
    let logs_dir = root.path().join(".tmp/logs");
    fs::create_dir_all(&logs_dir).unwrap();
    let path = logs_dir.join("agentx.log");
    let backups = log_backups();
    // Sparse oversized files test the boundary without generating megabytes of log events.
    for index in 0..=backups + 1 {
        fs::File::create(&path).unwrap().set_len(MAX_BYTES + 1).unwrap();
        log_in_scope(root.path(), "WARN", "verification", "rotate", &index.to_string(), &[]);
        let lines = fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(lines.trim()).unwrap();
        assert_eq!(value["event"], "rotate");
    }
    assert!(!logs_dir.join(format!("agentx.log.{}", backups + 1)).exists());
    if backups > 0 {
        assert!(logs_dir.join(format!("agentx.log.{backups}")).exists());
    }
}
