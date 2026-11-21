//! A11: real AgentX Claude launch/hook paths with a deliberately broken Codex.
//! The native Claude process is a fixture; no vendor runtime claim is made.
#![cfg(unix)]

mod support;

use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::Stdio;
use std::time::Duration;
use support::Agentx;

fn hook(h: &Agentx, command: &str, process: &str, print: bool, payload: &Value) -> String {
    let mut child = h
        .cmd()
        .arg(command)
        .env("AGENTX_PROCESS_ID", process)
        .env("AGENTX_LAUNCHED", "1")
        .env("AGENTX_TOOL", "claude")
        .env("AGENTX_PTY_MODE", if print { "0" } else { "1" })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(payload.to_string().as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

fn verify_claude_mode(print: bool) {
    let h = Agentx::new();
    let bin = h.root_path().join("provider-bin");
    let receipt = h.root_path().join("provider-receipt");
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&receipt).unwrap();
    let script = r#"#!/bin/sh
set -eu
if [ "${1-}" = --version ]; then printf '2.1.200 (Claude Code)\n'; exit 0; fi
printf '%s\0' "$@" > "$FIXTURE_RECEIPT/args"
printf '%s\0' "$AGENTX_TOOL" "$AGENTX_INSTANCE_NAME" "$AGENTX_PROCESS_ID" "${AGENTX_CODEX_SHARED-unset}" > "$FIXTURE_RECEIPT/env"
printf '%s' "$$" > "$FIXTURE_RECEIPT/pid"
exec /bin/sleep 300
"#;
    fs::write(bin.join("claude"), script).unwrap();
    fs::write(bin.join("codex"), "#!/bin/sh\nprintf invoked > \"$FIXTURE_RECEIPT/codex-invoked\"\nexit 91\n").unwrap();
    for tool in ["claude", "codex"] {
        fs::set_permissions(bin.join(tool), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::join_paths([bin, std::path::PathBuf::from(env!("CARGO_BIN_EXE_agentx")).parent().unwrap().to_path_buf()].into_iter().chain(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()))).unwrap();
    h.set_launch_env("PATH", path.to_str().unwrap());
    h.set_launch_env("FIXTURE_RECEIPT", receipt.to_str().unwrap());
    let mut args = vec!["launch", "--tool", "claude", "--count", "1", "--headless", "--json", "--go"];
    if print {
        args.extend(["--", "-p", "fixed fixture prompt"]);
    }
    let (code, stdout, stderr) = h.run(args);
    assert_eq!(code, 0, "{stdout} {stderr}");
    let launch: Value = serde_json::from_str(&stdout).unwrap();
    let name = launch["names"][0].as_str().unwrap();
    assert_eq!(launch["launched"], 1);
    h.eventually("fixture Claude started", Duration::from_secs(10), || Ok(receipt.join("env").exists().then_some(())));
    let pid = fs::read_to_string(receipt.join("pid")).unwrap().parse::<i64>().unwrap();
    h.track_cleanup_pid(pid);
    let bytes = fs::read(receipt.join("env")).unwrap();
    let env: Vec<_> = bytes.split(|b| *b == 0).filter(|s| !s.is_empty()).map(|s| String::from_utf8(s.to_vec()).unwrap()).collect();
    assert_eq!(env[0], "claude");
    assert_eq!(env[1], name);
    assert_eq!(env[3], "unset", "Codex shared mode leaked into Claude");
    let payload = json!({"session_id":"isolated-claude-session","cwd":h.workspace,"transcript_path":"","source":"startup","prompt":"fixture prompt","tool_name":"Bash","tool_input":{"command":"fixture"}});
    let bootstrap = hook(&h, "sessionstart", &env[2], print, &payload);
    assert!(bootstrap.contains("AGENTX SESSION") && bootstrap.contains(name), "{bootstrap}");
    let (code, _, error) = h.run(["send", "--iam", name, "--from", "fixture-sender", "--intent", "inform", &format!("@{name}"), "--", "A11_NON_CODEX_HOOK_MESSAGE"]);
    assert_eq!(code, 0, "{error}");
    let injected = hook(&h, if print { "post" } else { "userpromptsubmit" }, &env[2], print, &payload);
    assert!(injected.contains("A11_NON_CODEX_HOOK_MESSAGE"), "{injected}");
    let db = rusqlite::Connection::open(h.path().join("agentx.db")).unwrap();
    let cursor: i64 = db.query_row("SELECT last_event_id FROM instances WHERE name=?", [name], |row| row.get(0)).unwrap();
    let message: i64 = db.query_row("SELECT id FROM events WHERE type='message' AND json_extract(data,'$.text')='A11_NON_CODEX_HOOK_MESSAGE'", [], |row| row.get(0)).unwrap();
    assert!(cursor >= message, "successful hook flush did not confirm legacy delivery");
    assert!(!receipt.join("codex-invoked").exists(), "non-Codex launch/hook depended on Codex");
    assert!(!h.path().join("codex-services").exists());
    let (code, _, error) = h.run(["kill", name]);
    assert_eq!(code, 0, "{error}");
}

#[test]
fn acceptance_a11_claude_print_and_hook_delivery_work_with_poisoned_codex() {
    verify_claude_mode(true);
}

#[test]
fn acceptance_a11_claude_pty_and_hook_delivery_work_with_poisoned_codex() {
    verify_claude_mode(false);
}
