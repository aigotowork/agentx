//! Shell workflow regressions. Real-agent coverage is exercised separately.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn script(name: &str) -> String {
    format!("{}/src/workflows/bundled/{name}.sh", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn missing_option_values_are_reported() {
    for (name, options) in [
        ("confess", vec!["--target", "--iam", "--task", "--tool"]),
        ("debate", vec!["--workers", "--tool", "--rounds", "--timeout", "--context", "--iam"]),
        ("fatcow", vec!["--path", "--focus", "--tool", "--ask", "--timeout", "--iam"]),
        ("onidle", vec!["--iam", "--timeout"]),
    ] {
        for option in options {
            let output = Command::new("bash").args([script(name), option.into()]).output().unwrap();
            assert_eq!(output.status.code(), Some(1), "{name} {option}");
            assert!(String::from_utf8_lossy(&output.stderr).contains("requires"), "{name} {option}: {:?}", output.stderr);
        }
    }
}

fn stub() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agentx");
    std::fs::write(
        &path,
        r#"#!/usr/bin/env bash
printf '%s\n' "$*" >> "$CALL_LOG"
case "$1" in
  list)
    if [[ "$2" == worker && -n "$ORACLE_STOPPED" ]]; then exit 1; fi
    case "$2" in
      self) if [[ "$*" == *--json* ]]; then printf '{"name":"tester","session_id":"test","directory":"/tmp"}\n'; else echo tester; fi ;;
      --format) echo 'watch|codex|listening||0|' ;;
      *) echo listening ;;
    esac ;;
  events) echo '{"data":{"text":"answer"}}' ;;
  r) echo '{"names":["worker"]}' ;;
  launch)
    if [[ "$2" == --help ]]; then echo 'Supported tools: claude, codex'; exit 0; fi
    while [[ $# -gt 0 ]]; do
      if [[ "$1" == --title && "$2" == Judge* ]]; then echo 'deliberate launch failure' >&2; exit 1; fi
      shift
    done
    echo '{"names":["worker"]}' ;;
esac
"#,
    )
    .unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

fn command(name: &str, dir: &tempfile::TempDir) -> Command {
    let mut command = Command::new("bash");
    command.arg(script(name));
    command.env("PATH", format!("{}:{}", dir.path().display(), std::env::var("PATH").unwrap()));
    command.env("CALL_LOG", dir.path().join("calls"));
    command
}

#[test]
fn failed_launch_cleans_up_previously_started_agents() {
    for name in ["confess", "debate"] {
        let dir = stub();
        let mut command = command(name, &dir);
        command.args(["--tool", "codex"]);
        if name == "debate" {
            command.args(["topic", "--spawn"]);
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("deliberate launch failure"));
        let calls = std::fs::read_to_string(dir.path().join("calls")).unwrap();
        assert!(calls.lines().any(|line| line.starts_with("kill worker")), "{calls}");
        // Scripts read launched names from the JSON result, not the human text.
        assert!(calls.lines().filter(|line| line.starts_with("launch ") && !line.contains("--help")).all(|line| line.contains("--json")), "{calls}");
    }
}

#[test]
fn onidle_launch_proceeds_and_quiet_injection_fires() {
    for target in ["codex", "target"] {
        let dir = stub();
        let output = command("onidle", &dir).args(["watch", target, "hello", "--quiet"]).output().unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        let calls = std::fs::read_to_string(dir.path().join("calls")).unwrap();
        if target == "codex" {
            assert!(calls.contains("codex --go --agentx-prompt hello"), "{calls}");
        } else {
            assert!(calls.contains("term inject target hello --enter"), "{calls}");
        }
    }
}

#[test]
fn fatcow_resolves_ingestion_paths_and_resumes_stopped_agents() {
    let dir = stub();
    let output = command("fatcow", &dir).current_dir(env!("CARGO_MANIFEST_DIR")).args(["--path", "src/workflows/mod.rs", "--tool", "codex", "--interactive"]).output().unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    let calls = std::fs::read_to_string(dir.path().join("calls")).unwrap();
    assert!(calls.contains(&format!("You are a fat cow for: `{}/src/workflows/mod.rs`", env!("CARGO_MANIFEST_DIR"))), "{calls}");
    std::fs::write(dir.path().join("calls"), "").unwrap();
    let output = command("fatcow", &dir).env("ORACLE_STOPPED", "1").args(["--ask", "worker", "question"]).output().unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    let calls = std::fs::read_to_string(dir.path().join("calls")).unwrap();
    assert!(calls.contains("instance='worker' AND life_action='stopped'"), "{calls}");
    assert!(!calls.lines().any(|line| line.starts_with("events ") && line.contains("--json")), "{calls}");
    assert!(calls.contains("kill worker --go"), "{calls}");
    assert!(calls.contains("r worker --go"), "{calls}");
}
