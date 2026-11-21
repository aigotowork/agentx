//! Conservative evidence from the root rollout, without inherited history.
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use serde_json::Value;

/// Only a complete, unchanged regular file whose latest turn completed can
/// attest to idleness. A new user message or partial append invalidates it.
pub(crate) fn latest_turn_completion(path: &Path, session: &str, turn: &str) -> Option<i64> {
    const HEADER_LIMIT: u64 = 64 * 1024;
    const TAIL_LIMIT: u64 = 256 * 1024;
    if !path.is_absolute() || !path.metadata().ok()?.is_file() {
        return None;
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = options.open(path).ok()?;
    let before = file.metadata().ok()?;
    if !before.is_file() {
        return None;
    }
    let mut header = Vec::new();
    BufReader::new((&mut file).take(HEADER_LIMIT)).read_until(b'\n', &mut header).ok()?;
    if !header.ends_with(b"\n") {
        return None;
    }
    let meta: Value = serde_json::from_slice(&header).ok()?;
    if meta["type"] != "session_meta" || meta["payload"]["id"].as_str()? != session {
        return None;
    }
    let len = before.len();
    let start = len.saturating_sub(TAIL_LIMIT);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut tail = Vec::new();
    (&mut file).take(len - start).read_to_end(&mut tail).ok()?;
    let after = file.metadata().ok()?;
    if tail.len() as u64 != len - start || !tail.ends_with(b"\n") || after.len() != len || before.modified().ok()? != after.modified().ok()? {
        return None;
    }
    let body = if start > 0 { &tail[tail.iter().position(|byte| *byte == b'\n')? + 1..] } else { &tail };
    for line in body.rsplit(|byte| *byte == b'\n').filter(|line| !line.is_empty()) {
        let event: Value = serde_json::from_slice(line).ok()?;
        let payload = event.get("payload")?;
        match (event["type"].as_str()?, payload["type"].as_str()) {
            ("event_msg", Some("task_complete")) => {
                if payload["turn_id"].as_str()? != turn {
                    return None;
                }
                return chrono::DateTime::parse_from_rfc3339(event["timestamp"].as_str()?).ok().map(|stamp| stamp.timestamp_millis());
            }
            ("event_msg", Some("task_started" | "turn_aborted" | "user_message" | "error")) => return None,
            ("response_item", Some("message")) if payload["role"] == "user" => return None,
            _ => {}
        }
    }
    None
}

#[cfg(test)]
#[test]
fn codex_completion_requires_root_latest_complete_turn_and_complete_records() {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().unwrap();
    writeln!(file, "{}", serde_json::json!({"type":"session_meta","payload":{"id":"session"}})).unwrap();
    writeln!(file, "{}", serde_json::json!({"timestamp":"2026-11-20T08:09:10.528Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"turn"}})).unwrap();
    assert!(latest_turn_completion(file.path(), "session", "turn").is_some());
    assert!(latest_turn_completion(file.path(), "other", "turn").is_none());
    assert!(latest_turn_completion(file.path(), "session", "prior").is_none());
    write!(file, "{{").unwrap();
    assert!(latest_turn_completion(file.path(), "session", "turn").is_none());
    file.as_file().set_len(file.as_file().metadata().unwrap().len() - 1).unwrap();
    file.seek(SeekFrom::End(0)).unwrap();
    writeln!(file, "{}", serde_json::json!({"type":"event_msg","payload":{"type":"task_started","turn_id":"new"}})).unwrap();
    assert!(latest_turn_completion(file.path(), "session", "turn").is_none());
}
