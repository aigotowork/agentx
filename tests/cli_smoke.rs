//! Hermetic CLI smoke tests: invoke the `agentx` binary in a temp AGENTX_DIR and
//! assert exit codes + stdout shape.

mod support;

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::{Duration, Instant};
use support::{Agentx, parse_agentx_marker};

#[cfg(unix)]
#[test]
fn fixture_pins_cargo_target_before_dev_root_reexec() {
    use std::os::unix::fs::PermissionsExt;

    let h = Agentx::new();
    let foreign_root = h.root_path().join("foreign-checkout");
    let foreign_bin = foreign_root.join("target/debug/agentx");
    std::fs::create_dir_all(foreign_bin.parent().unwrap()).unwrap();
    std::fs::write(&foreign_bin, "#!/bin/sh\nprintf 'foreign binary\\n'\n").unwrap();
    std::fs::set_permissions(&foreign_bin, std::fs::Permissions::from_mode(0o755)).unwrap();

    let output = h.cmd().env("AGENTX_DEV_ROOT", &foreign_root).args(["config", "AGENTX_DIR"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), h.agentx_dir.to_string_lossy(),);
    let log = std::fs::read_to_string(h.agentx_dir.join(".tmp/logs/agentx.log")).unwrap_or_default();
    assert!(!log.contains("\"event\":\"dev_root_reexec\""), "{log}");
}

#[cfg(unix)]
#[test]
fn pty_defaults_unusable_term_and_preserves_valid_term() {
    let h = Agentx::new();
    for term in [None, Some("xterm-kitty"), Some("dumb"), Some("")] {
        let mut command = h.cmd();
        command.env_remove("TERM");
        if let Some(term) = term {
            command.env("TERM", term);
        }
        let output = command.args(["pty", "sh", "-c", "printf 'CHILD_TERM=<%s>' \"${TERM-unset}\""]).output().unwrap();
        assert!(output.status.success(), "TERM={term:?}: {}", String::from_utf8_lossy(&output.stderr));
        let expected_term = match term {
            Some(value) if !value.is_empty() && value != "dumb" => value,
            _ => "xterm-256color",
        };
        let expected = format!("CHILD_TERM=<{expected_term}>");
        assert!(String::from_utf8_lossy(&output.stdout).contains(&expected), "TERM={term:?}: stdout={:?}", String::from_utf8_lossy(&output.stdout));
    }
}

#[cfg(windows)]
#[test]
fn windows_pty_answers_startup_queries_while_waiting_for_terminal_metadata() {
    let h = Agentx::new();
    let process_id = "cpr-startup-metadata";
    // The managed proxy must register against the same identity that `start`
    // created. It will reject an unrelated process binding before reading CPR.
    // Keep the manual identity alive while the managed PTY initializes. A
    // one-shot `agentx start` process exits immediately, so the daemon's
    // process-binding cleanup correctly expires it before delivery startup.
    let name = h.start_listening_with_process_id(process_id);
    let id_dir = h.agentx_dir.join(".tmp/terminal_ids");
    std::fs::create_dir_all(&id_dir).unwrap();
    let id_file = id_dir.join(process_id);
    let log_file = h.agentx_dir.join(".tmp/logs/agentx.log");
    let release_file = h.agentx_dir.join(".tmp/cpr-release");

    // The terminal ID arrives only once the proxy has answered ConPTY's
    // cursor query. Before the fix, metadata collection finished (and missed
    // this ID) before the reader could answer; slow launches also leaked the
    // expired cursor reply into the child's prompt.
    let startup_log = log_file.clone();
    let terminal = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if std::fs::read_to_string(&startup_log).unwrap_or_default().contains("\"event\":\"startup.dsr_answered\"") {
                std::fs::write(id_file, "terminal-after-cpr").unwrap();
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    });
    // Keep the ConPTY child alive after its startup marker. The host persists
    // terminal metadata asynchronously after answering CPR; exiting at the
    // marker lets delivery teardown retire the identity before that commit.
    let mut command = h.cmd();
    command
        .env("AGENTX_INSTANCE_NAME", &name)
        .env("AGENTX_PROCESS_ID", process_id)
        .env("AGENTX_TEST_CPR_RELEASE", &release_file)
        // Ignore pane IDs inherited from the terminal running this test.
        .env("AGENTX_LAUNCHED_PRESET", "cpr-test-without-pane")
        .args([
            "pty",
            "codex",
            "--agentx-tool-path",
            "powershell.exe",
            "-NoProfile",
            "-Command",
            "Write-Output CPR_CHILD_READY; $deadline = (Get-Date).AddSeconds(45); while (-not (Test-Path -LiteralPath $env:AGENTX_TEST_CPR_RELEASE)) { if ((Get-Date) -gt $deadline) { exit 1 }; Start-Sleep -Milliseconds 20 }",
        ]);
    let pty = std::thread::spawn(move || command.output().unwrap());
    let answered = terminal.join().unwrap();
    assert!(answered, "proxy did not answer startup CPR");

    use rusqlite::OptionalExtension;
    h.eventually("live terminal metadata commit and PTY delivery initialization", Duration::from_secs(30), || {
        let conn = rusqlite::Connection::open(h.agentx_dir.join("agentx.db")).map_err(|error| error.to_string())?;
        let raw: Option<String> = conn.query_row("SELECT launch_context FROM instances WHERE name = ?1", [&name], |row| row.get(0)).optional().map_err(|error| error.to_string())?;
        let Some(raw) = raw else { return Ok(None) };
        let context: serde_json::Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
        let initialized = std::fs::read_to_string(&log_file).unwrap_or_default().contains("\"event\":\"delivery.init.success\"");
        Ok((context["terminal_id"] == "terminal-after-cpr" && initialized).then_some(()))
    });
    std::fs::write(&release_file, "release").unwrap();
    let output = pty.join().unwrap();
    assert!(output.status.success(), "stdout={} stderr={}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("CPR_CHILD_READY"));
}

#[test]
fn windows_hook_cleanup_preserves_foreign_hooks_and_stays_in_fixture() {
    let h = Agentx::new();
    for (tool, event, suffix, filename) in [("claude", "SessionStart", "claude-sessionstart", "settings.json"), ("cursor", "sessionStart", "cursor-sessionstart", "hooks.json"), ("codex", "SessionStart", "codex-sessionstart", "hooks.json")] {
        let dir = h.home.join(format!(".{tool}"));
        std::fs::create_dir_all(&dir).unwrap();
        let handlers = serde_json::json!([
            {"type": "command", "command": format!("C:/dev/agentx.exe {suffix}")},
            {"type": "command", "command": "user-hook"},
            {"type": "command", "command": "echo \"agentx cursor-stop\""}
        ]);
        let hooks = if tool == "cursor" { serde_json::json!({"hooks": {event: handlers}}) } else { serde_json::json!({"hooks": {event: [{"hooks": handlers}]}}) };
        let path = dir.join(filename);
        std::fs::write(&path, hooks.to_string()).unwrap();
        let mut command = h.cmd();
        if tool == "codex" {
            command.env("CODEX_HOME", &dir);
        }
        let output = command.args(["hooks", "remove", tool]).output().unwrap();
        assert!(output.status.success(), "{tool}: {}", String::from_utf8_lossy(&output.stderr));
        let remaining = std::fs::read_to_string(path).unwrap();
        assert!(!remaining.contains("agentx.exe"), "{tool}: {remaining}");
        assert!(remaining.contains("user-hook"), "{tool}: {remaining}");
        assert!(remaining.contains("echo"), "{tool}: {remaining}");
    }

    let outside = tempfile::tempdir().unwrap();
    let sentinel = outside.path().join("hooks.json");
    let original = r#"{"hooks":{"SessionStart":[{"hooks":[{"command":"agentx codex-sessionstart"}]}]}}"#;
    std::fs::write(&sentinel, original).unwrap();
    let output = h.cmd().env("CODEX_HOME", outside.path()).args(["hooks", "remove", "codex"]).output().unwrap();
    assert!(output.status.success());
    assert_eq!(std::fs::read_to_string(sentinel).unwrap(), original);
}

#[test]
fn fixture_drop_terminates_registered_process_group() {
    #[cfg(unix)]
    let mut child = Command::new("sh").args(["-c", "exec sleep 60"]).process_group(0).spawn().expect("spawn cleanup test process group");
    #[cfg(windows)]
    let mut child = Command::new("powershell").args(["-NoProfile", "-Command", "Start-Sleep -Seconds 60"]).spawn().expect("spawn cleanup test process group");
    let pid = i64::from(child.id());

    let h = Agentx::new();
    h.track_cleanup_pid(pid);
    assert!(h.process_group_alive(pid), "cleanup test process group did not start");

    let reaper = std::thread::spawn(move || child.wait().expect("reap cleanup test process"));
    drop(h);
    let status = reaper.join().expect("cleanup reaper thread");
    assert!(!status.success(), "fixture cleanup should terminate the registered process");

    let deadline = Instant::now() + Duration::from_secs(7);
    while Instant::now() < deadline && support::process_group_alive(pid) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!support::process_group_alive(pid), "fixture drop left process group {pid} alive");
}

#[cfg(unix)]
#[test]
fn fixture_drop_terminates_orphan_without_instance_row() {
    let h = Agentx::new();
    // Only the pidfile owns this process; neither instance rows nor explicit
    // fixture cleanup registration can discover it.
    // Reap the descendant on group termination so zombie lifetime does not
    // depend on the host init process. Killing only the shell still leaves it
    // waiting for its live child, which the bounded exit check detects.
    let ready_path = h.root_path().join("orphan-ready");
    let mut child = Command::new("sh")
        .args(["-c", r#"sleep 60 & descendant=$!; trap 'wait "$descendant"; exit 143' TERM; printf ready > "$1"; wait "$descendant""#, "orphan-fixture"])
        .arg(&ready_path)
        .process_group(0)
        .spawn()
        .expect("spawn orphan process group");
    let pid = i64::from(child.id());
    let tmp = h.agentx_dir.join(".tmp");
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(
        tmp.join("launched_pids.json"),
        serde_json::json!({pid.to_string(): {
            "tool": "claude", "names": ["orphan"], "launched_at": 1.0
        }})
        .to_string(),
    )
    .unwrap();
    // Confirm the descendant exists without an unbounded pipe read. Register
    // its group first so fixture cleanup also runs if readiness times out.
    h.eventually("orphan descendant started", Duration::from_secs(3), || Ok(ready_path.exists().then_some(())));
    drop(h);
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().expect("poll orphan") {
            assert!(!status.success(), "orphan must be terminated by fixture teardown");
            break;
        }
        if Instant::now() >= deadline {
            support::terminate_process_group(pid);
            let _ = child.kill();
            let _ = child.wait();
            panic!("fixture teardown left orphan {pid} alive");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while support::process_group_alive(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    if support::process_group_alive(pid) {
        support::terminate_process_group(pid);
        panic!("fixture teardown left orphan descendants in group {pid}");
    }
}

#[test]
fn help_prints_and_exits_zero() {
    let h = Agentx::new();
    let (code, stdout, _stderr) = h.run(["--help"]);
    assert_eq!(code, 0, "stdout={stdout}");
    assert!(stdout.contains("agentx"), "stdout={stdout}");
    assert!(stdout.contains("Commands:") || stdout.contains("Launch:"), "stdout={stdout}");
}

#[test]
fn version_prints_and_exits_zero() {
    let h = Agentx::new();
    let (code, stdout, stderr) = h.run(["--version"]);
    assert_eq!(code, 0, "stderr={stderr}");
    assert_eq!(stdout.trim(), concat!("agentx ", env!("CARGO_PKG_VERSION")), "stderr={stderr}");
}

#[test]
fn status_json_in_fresh_dir() {
    let h = Agentx::new();
    let (code, stdout, _stderr) = h.run(["status", "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("status json: {e}\n{stdout}"));
    let scope = h.path().canonicalize().expect("canonical fixture scope");
    assert_eq!(v["agentx_dir"].as_str(), scope.to_str());
    assert_eq!(v["instances"]["total"], 0);
}

#[test]
fn status_clean_logs_displays_path() {
    let h = Agentx::new();
    let (code, stdout, _stderr) = h.run(["status"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("logs:      ✓ ok"));
    let log_path = h.path().canonicalize().expect("canonical fixture scope").join(".tmp").join("logs").join("agentx.log");
    assert!(stdout.contains(&log_path.display().to_string()), "stdout should contain log path: {stdout}");
}

#[test]
fn list_json_empty() {
    let h = Agentx::new();
    let (code, stdout, _stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    let arr = v.as_array().expect("list returns array");
    assert!(arr.is_empty(), "expected empty list, got {stdout}");
}

#[test]
fn kill_json_reports_every_target() {
    let h = Agentx::new();
    let agent = h.start();
    let (code, _, stderr) = h.run(["stop", &agent]);
    assert_eq!(code, 0, "stop failed: {stderr}");

    let (code, stdout, stderr) = h.run(["kill", &agent, "nosuchagent", "--json"]);
    assert_eq!(code, 1, "a missing target fails the command: stderr={stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("kill --json object");
    assert_eq!(report["ok"], false);
    let results = report["results"].as_array().expect("results array");
    assert_eq!(results.len(), 2, "one outcome per target: {stdout}");
    assert_eq!(results[0]["target"], agent.as_str());
    assert_eq!(results[0]["kind"], "stopped");
    assert_eq!(results[0]["result"], "already_stopped");
    assert_eq!(results[0]["resume"], format!("agentx r {agent}"));
    assert_eq!(results[1]["target"], "nosuchagent");
    assert_eq!(results[1]["result"], "error");
    assert!(results[1]["error"].is_string(), "{stdout}");
}

#[cfg(unix)]
#[test]
fn launch_json_reports_names_for_kill_json() {
    use std::os::unix::fs::PermissionsExt;

    let h = Agentx::new();
    // A stand-in `claude` that answers --version and then idles: enough for
    // agentx to launch, track, and kill it without a real tool.
    let fake_bin = h.root_path().join("fake-bin");
    std::fs::create_dir_all(&fake_bin).expect("create fake bin");
    let fake_claude = fake_bin.join("claude");
    std::fs::write(&fake_claude, "#!/bin/sh\ncase \"$1\" in --version) echo '2.1.200 (Claude Code)'; exit 0;; esac\nsleep 300\n").expect("write fake claude");
    std::fs::set_permissions(&fake_claude, std::fs::Permissions::from_mode(0o755)).expect("chmod fake claude");
    let agentx_bin_dir = std::path::Path::new(env!("CARGO_BIN_EXE_agentx")).parent().expect("agentx bin dir").to_path_buf();
    let path = std::env::join_paths([agentx_bin_dir, fake_bin].into_iter().chain(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()))).expect("join PATH");

    let output = h.cmd().args(["launch", "--tool", "claude", "--count", "1", "--headless", "--json"]).env("PATH", &path).output().expect("run launch");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    let launch: serde_json::Value = serde_json::from_str(stdout.trim()).expect("stdout is exactly one JSON object");
    assert_eq!(launch["action"], "launch");
    assert_eq!(launch["tool"], "claude");
    assert_eq!(launch["launched"], 1);
    assert_eq!(launch["failed"], 0);
    assert!(launch["batch_id"].as_str().is_some_and(|b| !b.is_empty()), "{launch}");
    let names = launch["names"].as_array().expect("names array");
    assert_eq!(names.len(), 1, "{launch}");
    let name = names[0].as_str().expect("name string");

    let (_, stdout, stderr) = h.run(["kill", name, "--json"]);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}: {stdout} / {stderr}"));
    let outcome = &report["results"][0];
    assert_eq!(outcome["target"], name, "{report}");
    // Usually still running; a slow host may have already given up waiting
    // for the fake tool to become ready, which stops it first.
    assert!(outcome["kind"] == "agent" || outcome["kind"] == "stopped", "{report}");
    assert_eq!(outcome["resume"], format!("agentx r {name}"));
}

#[test]
fn events_empty_in_fresh_dir() {
    let h = Agentx::new();
    let (code, stdout, _stderr) = h.run(["events", "--last", "5"]);
    assert_eq!(code, 0);
    assert!(stdout.trim().is_empty(), "expected no events, got {stdout}");
}

#[test]
fn send_without_identity_errors_with_hint() {
    let h = Agentx::new();
    let (code, _stdout, stderr) = h.run(["send", "@nobody", "--", "hi"]);
    assert_ne!(code, 0, "send without identity must fail: stderr={stderr}");
    assert!(stderr.contains("identity not found"), "expected stable hint, got: {stderr}");
}

#[test]
fn iam_defaults_send_and_listen_to_the_injected_process_identity() {
    let h = Agentx::new();
    let sender = h.start_with_process_id("iam-sender");
    let recipient = h.start_with_process_id("iam-recipient");
    let (code, output, error) = h.run_as_process("iam-sender", ["send", &format!("@{recipient}"), "--intent", "inform", "--json", "--", "implicit identity"]);
    assert_eq!(code, 0, "output={output} error={error}");
    let (code, output, error) = h.run_as_process("iam-recipient", ["listen", "--timeout", "1", "--json"]);
    assert_eq!(code, 0, "output={output} error={error}");
    let message: serde_json::Value = output.lines().filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok()).find(|message| message["from"] == sender).expect("message attributed to the injected sender");
    assert_eq!(message["text"], "implicit identity");
}

#[test]
fn iam_explicit_caller_and_start_registration_have_separate_parameters() {
    let h = Agentx::new();
    let (code, output, error) = h.run(["start", "--name", "caller"]);
    assert_eq!(code, 0, "output={output} error={error}");
    assert_eq!(parse_agentx_marker(&output).as_deref(), Some("caller"));
    let (code, output, error) = h.run(["--iam", "caller", "start", "--name", "recipient"]);
    assert_eq!(code, 0, "output={output} error={error}");
    assert_eq!(parse_agentx_marker(&output).as_deref(), Some("recipient"));
    for argv in [vec!["--iam", "caller", "send", "@recipient", "--json", "--", "prefix"], vec!["send", "--iam", "caller", "@recipient", "--json", "--", "suffix"]] {
        let (code, output, error) = h.run(argv);
        assert_eq!(code, 0, "output={output} error={error}");
    }
    let (code, output, error) = h.run(["listen", "--iam", "recipient", "--timeout", "1", "--json"]);
    assert_eq!(code, 0, "output={output} error={error}");
    let messages: Vec<serde_json::Value> = output.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();
    assert_eq!(messages.len(), 2, "{output}");
    assert!(messages.iter().all(|message| message["from"] == "caller"));
    for argv in [vec!["send", "--name", "caller", "@recipient", "--", "obsolete option"], vec!["listen", "--name", "recipient", "--timeout", "1"], vec!["launch", "--tool", "codex", "--name", "caller", "--dry-run"]] {
        let (code, output, error) = h.run(argv);
        assert_ne!(code, 0, "old caller option must fail: {output}");
        assert!(error.contains("--name"), "{error}");
    }
    let (code, output, error) = h.run(["--iam", "missing", "start"]);
    assert_ne!(code, 0, "--iam must not register a new identity: {output}");
    assert!(error.contains("missing"), "{error}");
}

#[test]
fn iam_expired_binding_errors_for_identity_operations_without_blocking_queries() {
    let h = Agentx::new();
    let recipient = h.start();
    for argv in [vec!["send", &format!("@{recipient}"), "--", "stale sender"], vec!["listen", "--timeout", "1"]] {
        let (code, output, error) = h.run_as_process("iam-expired", argv);
        assert_ne!(code, 0, "expired identity must fail: {output}");
        assert!(error.contains("session expired") && error.contains("agentx start"), "{error}");
    }
    let (code, output, error) = h.run_as_process("iam-expired", ["list", "--json"]);
    assert_eq!(code, 0, "output={output} error={error}");
}

#[test]
fn send_to_missing_agent_lists_available() {
    let h = Agentx::new();
    let me = h.start();

    let (code, _stdout, stderr) = h.run(["send", "@nope", "--iam", &me, "--", "hi"]);
    assert_ne!(code, 0, "send to nonexistent must fail");
    assert!(stderr.contains("@nope") && stderr.contains("Available:"), "stderr={stderr}");
}

#[test]
fn send_preserves_literal_identity_tokens_with_auto_resolved_sender() {
    let h = Agentx::new();
    let recipient = h.start();
    let process_id = "send-trailing-name-process";

    let mut start = h.cmd();
    start.env("AGENTX_PROCESS_ID", process_id).arg("start");
    let start_out = start.output().expect("spawn agentx start");
    let start_stdout = String::from_utf8_lossy(&start_out.stdout);
    let start_stderr = String::from_utf8_lossy(&start_out.stderr);
    assert!(start_out.status.success(), "stdout={start_stdout} stderr={start_stderr}");
    let sender = parse_agentx_marker(&start_stdout).expect("sender marker");

    let mut send = h.cmd();
    send.env("AGENTX_PROCESS_ID", process_id).args(["send", &format!("@{recipient}"), "--", "ack", "--iam", &sender]);
    let send_out = send.output().expect("spawn agentx send");
    let send_stdout = String::from_utf8_lossy(&send_out.stdout);
    let send_stderr = String::from_utf8_lossy(&send_out.stderr);
    assert!(send_out.status.success(), "stdout={send_stdout} stderr={send_stderr}");

    let (_, events, _) = h.run(["events", "--type", "message", "--last", "1"]);
    assert!(events.contains(&format!(r#""text":"ack --iam {sender}""#)), "events={events}");
}

#[test]
fn ai_tool_broadcast_to_many_requires_go_preview() {
    let h = Agentx::new();
    let sender = h.start();
    for _ in 0..4 {
        h.start();
    }

    let mut cmd = h.cmd();
    cmd.env("CODEX_SANDBOX", "1").args(["send", "--iam", &sender, "--", "probably meant one person"]);
    let out = cmd.output().expect("spawn agentx send");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_ne!(code, 0, "send should preview first: stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("BROADCAST SEND PREVIEW") && stdout.contains("broadcast to 4 agents") && stdout.contains("Did you mean to send this to everyone?") && stdout.contains("agentx send --go"), "stdout={stdout}");

    let (_, events_out, _) = h.run(["events", "--type", "message", "--last", "5"]);
    assert!(events_out.trim().is_empty(), "preview must not send a message: events={events_out}");

    let mut go_cmd = h.cmd();
    go_cmd.env("CODEX_SANDBOX", "1").args(["--go", "send", "--iam", &sender, "--", "confirmed broadcast"]);
    let go_out = go_cmd.output().expect("spawn agentx --go send");
    let go_code = go_out.status.code().unwrap_or(-1);
    let go_stdout = String::from_utf8_lossy(&go_out.stdout);
    let go_stderr = String::from_utf8_lossy(&go_out.stderr);
    assert_eq!(go_code, 0, "stdout={go_stdout} stderr={go_stderr}");
    assert!(go_stdout.contains("Sent to:") || go_stdout.contains("Sent to 4 agents"), "stdout={go_stdout}");
}

#[test]
fn reset_preview_delivers_mail_and_hook_reset_preserves_it() {
    let h = Agentx::new();
    let sender = h.start();
    let process_id = "reset-pending-delivery-process";
    let recipient = h.start_with_process_id(process_id);

    let send_message = |text: &str| {
        let (code, stdout, stderr) = h.run(["send", "--iam", &sender, &format!("@{recipient}"), "--", text]);
        assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    };

    send_message("pending during reset preview");
    let mut preview = h.cmd();
    preview.env("AGENTX_PROCESS_ID", process_id).env("CODEX_SANDBOX", "1").arg("reset");
    let output = preview.output().expect("run reset preview");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("RESET PREVIEW"), "stdout={stdout}");
    assert!(stdout.contains("pending during reset preview"), "stdout={stdout}");

    send_message("pending during reset hooks");
    let mut hooks = h.cmd();
    hooks.env("AGENTX_PROCESS_ID", process_id).env("CODEX_SANDBOX", "1").args(["--go", "reset", "hooks"]);
    let output = hooks.output().expect("run reset hooks");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    // Hook reset stays host-local; the next explicit listen owns delivery.
    assert!(!stdout.contains("pending during reset hooks"), "stdout={stdout}");
    let (code, stdout, stderr) = h.run(["listen", "--iam", &recipient, "--timeout", "1", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("pending during reset hooks"), "stdout={stdout}");
}

#[test]
fn start_send_events_roundtrip() {
    let h = Agentx::new();
    let sender = h.start();
    let recipient = h.start();
    assert_ne!(sender, recipient, "two starts must assign distinct names");

    let (c, stdout, stderr) = h.run(["send", &format!("@{recipient}"), "--iam", &sender, "--", "hello there"]);
    assert_eq!(c, 0, "stderr={stderr} stdout={stdout}");

    let (c4, events_out, _) = h.run(["events", "--last", "10"]);
    assert_eq!(c4, 0);
    let message_lines: Vec<_> = events_out.lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok()).filter(|v| v["type"] == "message").collect();
    assert_eq!(message_lines.len(), 1, "events={events_out}");
    let msg = &message_lines[0];
    assert_eq!(msg["instance"], sender.as_str(), "attribution = sender");
    assert_eq!(msg["data"]["from"], sender.as_str());
    assert_eq!(msg["data"]["text"], "hello there");

    // Recipient/scope contract: the message event only carries from/text, so
    // we check routing via per-instance unread on `list --json`. Recipient
    // must show unread=1, sender unread=0.
    let (c5, list_out, _) = h.run(["list", "--json"]);
    assert_eq!(c5, 0);
    let list: serde_json::Value = serde_json::from_str(&list_out).expect("list json");
    let by_name: std::collections::HashMap<_, _> = list.as_array().expect("array").iter().map(|v| (v["name"].as_str().unwrap().to_string(), v["unread_count"].as_u64().unwrap_or(0))).collect();
    assert_eq!(by_name.get(&recipient).copied(), Some(1), "recipient unread; list={list_out}");
    assert_eq!(by_name.get(&sender).copied(), Some(0), "sender unread; list={list_out}");

    let (c6, listen_out, listen_err) = h.run(["listen", "--iam", &recipient, "--timeout", "1", "--json"]);
    assert_eq!(c6, 0, "listen failed: stderr={listen_err}");
    let delivered: Vec<serde_json::Value> = listen_out.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    assert_eq!(delivered.len(), 1, "listen output={listen_out}");
    assert_eq!(delivered[0]["from"], sender.as_str());
    assert_eq!(delivered[0]["text"], "hello there");
    let event_id = delivered[0]["event_id"].as_i64().expect("event_id");
    assert_eq!(delivered[0]["reply_id"], event_id.to_string());

    let (c7, list_after_listen_out, _) = h.run(["list", "--json"]);
    assert_eq!(c7, 0);
    let list_after_listen: serde_json::Value = serde_json::from_str(&list_after_listen_out).expect("list json after listen");
    let after_by_name: std::collections::HashMap<_, _> = list_after_listen.as_array().expect("array").iter().map(|v| (v["name"].as_str().unwrap().to_string(), v["unread_count"].as_u64().unwrap_or(0))).collect();
    assert_eq!(after_by_name.get(&recipient).copied(), Some(0), "listen should advance recipient cursor; list={list_after_listen_out}");
}

#[test]
fn intent_and_reply_to_roundtrip() {
    // Wiki contract (messaging.md §Intent + event-model.md `msg_intent`/`reply_to_local`):
    // request → ack with --reply-to flattens through `events_v` so threads/replies
    // can be traced. Locks: data.intent on send, and data.reply_to_local resolved
    // from the parent event id.
    let h = Agentx::new();
    let a = h.start();
    let b = h.start();

    let (c, _, e) = h.run(["send", &format!("@{b}"), "--iam", &a, "--intent", "request", "--", "ping"]);
    assert_eq!(c, 0, "request send failed: stderr={e}");

    let (_, req_out, _) = h.run(["events", "--type", "message", "--from", &a, "--last", "5"]);
    let req: serde_json::Value = req_out.lines().find_map(|l| serde_json::from_str(l).ok()).expect("request event present");
    assert_eq!(req["data"]["intent"], "request");
    let req_id = req["id"].as_i64().expect("event id is i64");

    let (c2, _, e2) = h.run(["send", &format!("@{a}"), "--iam", &b, "--intent", "ack", "--reply-to", &req_id.to_string(), "--", "pong"]);
    assert_eq!(c2, 0, "ack send failed: stderr={e2}");

    let (_, ack_out, _) = h.run(["events", "--intent", "ack", "--last", "5"]);
    let ack: serde_json::Value = ack_out.lines().find_map(|l| serde_json::from_str(l).ok()).expect("ack event present");
    assert_eq!(ack["data"]["intent"], "ack");
    assert_eq!(ack["data"]["from"], b.as_str());
    assert_eq!(ack["data"]["reply_to_local"].as_i64(), Some(req_id), "reply_to_local must resolve to request event id; ack={ack}");
}

#[test]
fn remote_reply_to_origin_resolves_imported_event_row() {
    let h = Agentx::new();
    let a = h.start();
    let b = h.start();

    // Direct SQLite access to seed colliding local event and imported remote event
    let conn = rusqlite::Connection::open(h.agentx_dir.join("agentx.db")).expect("open agentx db");

    // Colliding local message at id = 42
    let local_data = serde_json::json!({
        "from": a,
        "text": "colliding local message",
        "intent": "request"
    });
    conn.execute("INSERT INTO events (id, type, instance, timestamp, data) VALUES (42, 'message', ?1, '2026-09-04T00:00:00Z', ?2)", rusqlite::params![a, local_data.to_string()]).expect("insert colliding local event");

    // Imported relay message at id = 100 with origin 42 and device short BOXE
    let remote_data = serde_json::json!({
        "from": "remote:BOXE",
        "text": "remote request",
        "intent": "request",
        "_relay": {
            "id": 42,
            "short": "BOXE",
            "device": "dev-uuid-boxe"
        }
    });
    conn.execute("INSERT INTO events (id, type, instance, timestamp, data) VALUES (100, 'message', 'remote:BOXE', '2026-09-04T00:00:01Z', ?1)", rusqlite::params![remote_data.to_string()]).expect("insert remote imported event");

    // Reply using remote token 42:BOXE
    let (c, _, e) = h.run(["send", &format!("@{a}"), "--iam", &b, "--intent", "ack", "--reply-to", "42:BOXE", "--", "pong"]);
    assert_eq!(c, 0, "ack send with remote reply-to failed: stderr={e}");

    let (_, ack_out, _) = h.run(["events", "--intent", "ack", "--last", "5", "--full"]);
    let ack: serde_json::Value = ack_out.lines().find_map(|l| serde_json::from_str(l).ok()).expect("ack event present");

    assert_eq!(ack["data"]["intent"], "ack");
    assert_eq!(ack["data"]["reply_to"], "42:BOXE");
    assert_eq!(ack["data"]["reply_to_local"].as_i64(), Some(100), "reply_to_local must resolve to imported event id 100, not colliding local id 42; ack={ack}");
}

#[test]
fn lifecycle_events_emitted_for_start_and_stop() {
    // Wiki contract (agent-lifecycle.md + event-model.md): start emits
    // life.started, stop emits life.stopped — filterable via --action.
    // events table is the lifecycle source of truth (see
    // feedback_events_are_source_of_truth memory).
    let h = Agentx::new();
    let a = h.start();

    let (c, started_out, _) = h.run(["events", "--action", "started", "--agent", &a, "--last", "5"]);
    assert_eq!(c, 0);
    let started: Vec<serde_json::Value> = started_out.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    assert_eq!(started.len(), 1, "expected 1 life.started for {a}, got: {started_out}");
    assert_eq!(started[0]["instance"], a.as_str());
    assert_eq!(started[0]["data"]["action"], "started");

    let (cs, _, es) = h.run(["stop", &a]);
    assert_eq!(cs, 0, "stop failed: {es}");

    let (_, stopped_out, _) = h.run(["events", "--action", "stopped", "--agent", &a, "--last", "5"]);
    let stopped: Vec<serde_json::Value> = stopped_out.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    assert_eq!(stopped.len(), 1, "expected 1 life.stopped for {a}, got: {stopped_out}");
    assert_eq!(stopped[0]["data"]["action"], "stopped");
    // Snapshot lives on the event but is streamlined out by default;
    // --full surfaces it. Rebind relies on it (see start_as_reclaims_stopped_identity).
    let (_, full_out, _) = h.run(["events", "--action", "stopped", "--agent", &a, "--last", "5", "--full"]);
    let full: serde_json::Value = full_out.lines().find_map(|l| serde_json::from_str(l).ok()).expect("stopped event under --full");
    assert!(full["data"]["snapshot"].is_object(), "stop must preserve snapshot for rebind; full={full_out}");
}

#[test]
fn start_as_reclaims_stopped_identity() {
    // Wiki contract (identity.md §--as + agentx-start.md Path B): after stop,
    // `start --as <name>` rebinds the same name (no random reallocation).
    // Distinct from bare `start`, which would draw a fresh name.
    let h = Agentx::new();
    let a = h.start();

    let (cs, _, es) = h.run(["stop", &a]);
    assert_eq!(cs, 0, "stop failed: {es}");

    let (cr, stdout, stderr) = h.run(["start", "--as", &a]);
    assert_eq!(cr, 0, "start --as failed: stderr={stderr}");
    assert!(stdout.contains(&format!("[agentx:{a}]")), "reclaim marker missing; stdout={stdout}");

    // Reclaimed instance is alive again under the same name.
    // (Reclaim is a quiet rebind: no new life.started event, just a logged
    // rebind.complete. The marker + a re-populated instances row is the
    // observable contract.)
    let (_, names_out, _) = h.run(["list", "--names"]);
    assert!(names_out.lines().any(|l| l.trim() == a), "list --names missing {a} after reclaim: {names_out}");

    // And the stopped snapshot must still be on record — that's what made
    // the cursor-preserving rebind possible.
    let (_, full_out, _) = h.run(["events", "--action", "stopped", "--agent", &a, "--last", "5", "--full"]);
    let snap_present = full_out.lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok()).any(|v| v["data"]["snapshot"].is_object());
    assert!(snap_present, "stopped snapshot missing; full={full_out}");
}

#[test]
fn bigboss_send_bypasses_identity_gate() {
    // Wiki contract (messaging.md §@bigboss + reference_send_bigboss_flag memory):
    // `send -b` is sender-as-bigboss and bypasses the identity gate that
    // normally requires `--iam` / a bound session. Sender_kind=external
    // distinguishes the message from instance-to-instance traffic.
    let h = Agentx::new();
    let recipient = h.start();

    // Note: no --iam. -b is the sole identity signal.
    let (c, _, stderr) = h.run(["send", "-b", &format!("@{recipient}"), "--", "from above"]);
    assert_eq!(c, 0, "send -b must bypass identity gate; stderr={stderr}");
    assert!(!stderr.contains("identity not found"), "gate should not fire under -b; stderr={stderr}");

    // --full bypasses streamlining so sender_kind is visible.
    let (_, events_out, _) = h.run(["events", "--type", "message", "--from", "bigboss", "--last", "5", "--full"]);
    let msg: serde_json::Value = events_out.lines().find_map(|l| serde_json::from_str(l).ok()).expect("bigboss message event");
    assert_eq!(msg["data"]["from"], "bigboss");
    assert_eq!(msg["data"]["text"], "from above");
    assert_eq!(msg["data"]["sender_kind"], "external", "bigboss must record as external sender; msg={msg}");
}

#[test]
fn config_unknown_key_is_rejected() {
    let h = Agentx::new();
    let (code, _stdout, stderr) = h.run(["config", "no_such_key"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("Unknown config key 'no_such_key'"), "stderr={stderr}");

    // Setting a typo must not write it to config.toml either.
    let (code, _stdout, stderr) = h.run(["config", "timout", "60"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("Did you mean: timeout?"), "stderr={stderr}");
}

#[test]
fn config_known_unset_key_reports_not_set() {
    let h = Agentx::new();
    let (code, stdout, _stderr) = h.run(["config", "hints"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("(not set)"), "stdout={stdout}");
}

#[test]
fn unknown_command_errors() {
    let h = Agentx::new();
    let (code, _stdout, stderr) = h.run(["nonsense-not-a-command"]);
    assert_ne!(code, 0);
    assert!(!stderr.is_empty(), "expected error message on stderr");
}

#[test]
fn antigravity_e2e_hook_dispatch() {
    let h = Agentx::new();
    let transcript = tempfile::NamedTempFile::new().expect("temp transcript");
    let transcript_path = transcript.path().to_string_lossy().to_string();

    // Spawn agentx start with AGENTX_PROCESS_ID to register a process binding
    let mut start_cmd = h.cmd();
    start_cmd.arg("start");
    start_cmd.env("AGENTX_PROCESS_ID", "pid-agy-123");
    let start_out = start_cmd.output().expect("failed to run agentx start");
    let me = support::parse_agentx_marker(&String::from_utf8_lossy(&start_out.stdout)).expect("no [agentx:NAME] marker");
    let conn = rusqlite::Connection::open(h.agentx_dir.join("agentx.db")).expect("open agentx db");
    conn.execute("UPDATE instances SET tool = 'antigravity' WHERE name = ?1", [&me]).expect("mark fixture as Antigravity");

    // 1. Pipe PreInvocation (session start) to gemini-sessionstart.
    // This will bind the session_id "sess-agy-1" to the active instance.
    let session_start_payload = serde_json::json!({
        "conversationId": "sess-agy-1",
        "transcriptPath": transcript_path,
    });

    use std::io::Write;
    use std::process::Stdio;

    let mut cmd = h.cmd();
    cmd.args(["gemini-sessionstart"]);
    cmd.env("ANTIGRAVITY_AGENT", "1");
    cmd.env("AGENTX_PROCESS_ID", "pid-agy-123");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn agentx sessionstart");
    {
        let mut stdin = child.stdin.take().expect("failed to open stdin");
        stdin.write_all(serde_json::to_string(&session_start_payload).unwrap().as_bytes()).unwrap();
    }
    let out = child.wait_with_output().expect("failed to wait sessionstart");
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let first_stdout = String::from_utf8_lossy(&out.stdout);
    let first: serde_json::Value = serde_json::from_str(first_stdout.trim()).expect("first sessionstart json");
    let first_context = first["injectSteps"][0]["ephemeralMessage"].as_str().expect("initial Antigravity bootstrap");
    assert!(first_context.contains("[AGENTX SESSION]"));
    assert!(first_context.contains(&format!("--iam {me}")));

    // Verify session_id binding matches in the DB via agentx list --json
    let (code, stdout, stderr) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("failed to parse list json");
    assert_eq!(v["session_id"].as_str(), Some("sess-agy-1"));

    // Antigravity fires this hook before every model invocation. Its ephemeral
    // bootstrap must be present after the one-shot name announcement too.
    let mut cmd = h.cmd();
    cmd.args(["gemini-sessionstart"]);
    cmd.env("ANTIGRAVITY_AGENT", "1");
    cmd.env("AGENTX_PROCESS_ID", "pid-agy-123");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn repeated sessionstart");
    {
        let mut stdin = child.stdin.take().expect("failed to open stdin");
        stdin.write_all(serde_json::to_string(&session_start_payload).unwrap().as_bytes()).unwrap();
    }
    let out = child.wait_with_output().expect("failed to wait repeated sessionstart");
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let repeated_stdout = String::from_utf8_lossy(&out.stdout);
    let repeated: serde_json::Value = serde_json::from_str(repeated_stdout.trim()).expect("repeated sessionstart json");
    let repeated_context = repeated["injectSteps"][0]["ephemeralMessage"].as_str().expect("recurring Antigravity bootstrap");
    assert!(repeated_context.contains("[AGENTX SESSION]"));
    assert!(repeated_context.contains(&format!("--iam {me}")));

    // 2. Now pipe PreToolUse to gemini-beforetool.
    // Since the session is bound, it should resolve the instance and execute successfully.
    let before_tool_payload = serde_json::json!({
        "conversationId": "sess-agy-1",
        "transcriptPath": transcript_path,
        "toolCall": {
            "name": "run_command",
            "args": { "CommandLine": "echo hello", "Cwd": "/tmp" }
        }
    });

    let mut cmd = h.cmd();
    cmd.args(["gemini-beforetool"]);
    cmd.env("ANTIGRAVITY_AGENT", "1");
    cmd.env("AGENTX_PROCESS_ID", "pid-agy-123");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn agentx beforetool");
    {
        let mut stdin = child.stdin.take().expect("failed to open stdin");
        stdin.write_all(serde_json::to_string(&before_tool_payload).unwrap().as_bytes()).unwrap();
    }
    let out = child.wait_with_output().expect("failed to wait beforetool");
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("beforetool json");
    assert_eq!(parsed, serde_json::json!({ "decision": "allow" }));

    // 3. AfterTool cannot inject context for Antigravity, so it must not ack delivery.
    // Send from bigboss: self-sends are never delivered, so they'd leave nothing pending.
    let (send_code, _, send_stderr) = h.run(["send", &format!("@{me}"), "-b", "--intent", "request", "--", "ping"]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");

    let after_tool_payload = serde_json::json!({
        "conversationId": "sess-agy-1",
        "transcriptPath": transcript_path,
        "toolCall": {
            "name": "run_command",
            "args": { "CommandLine": "echo done", "Cwd": "/tmp" }
        }
    });

    let mut cmd = h.cmd();
    cmd.args(["gemini-aftertool"]);
    cmd.env("ANTIGRAVITY_AGENT", "1");
    cmd.env("AGENTX_PROCESS_ID", "pid-agy-123");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn agentx aftertool");
    {
        let mut stdin = child.stdin.take().expect("failed to open stdin");
        stdin.write_all(serde_json::to_string(&after_tool_payload).unwrap().as_bytes()).unwrap();
    }
    let out = child.wait_with_output().expect("failed to wait aftertool");
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let after_stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(after_stdout.trim()).expect("aftertool json");
    assert_eq!(parsed, serde_json::json!({}));
}

/// Pipe a JSON payload to a native cursor hook and return its parsed stdout.
///
/// Cursor hook command names route directly to `Tool::Cursor` (no shared-prefix
/// disambiguation like Antigravity's `ANTIGRAVITY_AGENT`), so the only env the
/// gate check needs is `AGENTX_PROCESS_ID` to resolve the bound instance.
fn run_cursor_hook(h: &Agentx, hook: &str, process_id: &str, payload: &serde_json::Value) -> serde_json::Value {
    use std::io::Write;
    use std::process::Stdio;

    let mut cmd = h.cmd();
    cmd.args([hook]);
    cmd.env("AGENTX_PROCESS_ID", process_id);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {hook}: {e}"));
    {
        let mut stdin = child.stdin.take().expect("open stdin");
        stdin.write_all(serde_json::to_string(payload).unwrap().as_bytes()).unwrap();
    }
    let out = child.wait_with_output().unwrap_or_else(|e| panic!("wait {hook}: {e}"));
    assert_eq!(out.status.code().unwrap_or(-1), 0, "{hook} stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("{hook} json: {e}\nstdout={stdout}"))
}

/// Simulate the provider-owned registration prerequisite of native hooks.
fn start_provider_process(h: &Agentx, tool: &str, process_id: &str) -> String {
    let output = h.cmd().arg("start").env("AGENTX_PROCESS_ID", process_id).env("AGENTX_TOOL", tool).output().expect("failed to run agentx start");
    assert!(output.status.success(), "{tool} registration failed: {}", String::from_utf8_lossy(&output.stderr));
    parse_agentx_marker(&String::from_utf8_lossy(&output.stdout)).expect("no [agentx:NAME] marker")
}

/// End-to-end cursor-agent native hook lifecycle over JSON-on-stdin.
///
/// Mirrors `antigravity_e2e_hook_dispatch`, but exercises cursor's real payload
/// shape (`conversation_id`/`tool_input`/`tool_output`) and its distinct
/// delivery contract: unlike Antigravity (whose aftertool cannot inject and
/// must return `{}`), cursor's `postToolUse` injects pending messages via
/// `additional_context` and acks delivery.
#[test]
fn cursor_e2e_hook_dispatch() {
    let h = Agentx::new();
    let transcript = tempfile::NamedTempFile::new().expect("temp transcript");
    let transcript_path = transcript.path().to_string_lossy().to_string();
    let pid = "pid-cur-123";
    let session_id = "sess-cur-1";

    let me = start_provider_process(&h, "cursor", pid);

    // 1. sessionStart binds the conversation to the active instance. Cursor reads
    //    the id from `conversation_id` (snake_case, per the docs' common schema)
    //    and the handler always returns an `env` object.
    let session_start = run_cursor_hook(
        &h,
        "cursor-sessionstart",
        pid,
        &serde_json::json!({
            "conversation_id": session_id,
            "transcript_path": transcript_path,
            "workspace_roots": ["/tmp"],
            "is_background_agent": false,
            "composer_mode": "agent",
        }),
    );
    assert!(session_start.get("env").is_some(), "sessionStart should emit env block: {session_start}");

    // Binding is visible via list --json.
    let (code, stdout, stderr) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["session_id"].as_str(), Some(session_id));

    // 2. beforeSubmitPrompt marks the instance active and must not block the
    //    prompt (`continue: true`).
    let before_submit = run_cursor_hook(
        &h,
        "cursor-beforesubmitprompt",
        pid,
        &serde_json::json!({
            "conversation_id": session_id,
            "transcript_path": transcript_path,
            "prompt": "do a thing",
        }),
    );
    assert_eq!(before_submit["continue"], true);
    assert!(before_submit["additional_context"].is_string(), "{before_submit}");

    let (code, stdout, _) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["status"].as_str(), Some("active"));

    // 3. preToolUse records tool status and returns an empty object.
    let pre_tool = run_cursor_hook(
        &h,
        "cursor-pretooluse",
        pid,
        &serde_json::json!({
            "conversation_id": session_id,
            "transcript_path": transcript_path,
            "tool_name": "Shell",
            "tool_input": { "command": "echo hello", "working_directory": "/tmp" },
        }),
    );
    assert_eq!(pre_tool, serde_json::json!({}));

    // 4. Queue a message, then postToolUse delivers it via additional_context.
    //    Send from an external sender (bigboss), not `me`: the DB delivery
    //    filter (`should_deliver_to`) drops any message whose `from` equals the
    //    receiver, so a self-addressed send would never be pending and the
    //    postToolUse assertion below would pass vacuously.
    let (send_code, _, send_stderr) = h.run(["send", "--from", "bigboss", &format!("@{me}"), "--intent", "request", "--", "ping"]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");

    let post_tool = run_cursor_hook(
        &h,
        "cursor-posttooluse",
        pid,
        &serde_json::json!({
            "conversation_id": session_id,
            "transcript_path": transcript_path,
            "tool_name": "Shell",
            "tool_input": { "command": "echo hello" },
            "tool_output": "{\"exitCode\":0,\"stdout\":\"hello\"}",
        }),
    );
    let injected = post_tool.get("additional_context").and_then(serde_json::Value::as_str).unwrap_or_else(|| panic!("postToolUse should inject additional_context: {post_tool}"));
    assert!(injected.contains("ping"), "delivered context should carry the message text: {injected:?}");
}

/// End-to-end GitHub Copilot CLI native hook lifecycle over JSON-on-stdin.
///
/// Mirrors `cursor_e2e_hook_dispatch` but exercises copilot's real payload shape
/// (`session_id`/`tool_name`/`tool_input`/`tool_result`, Claude-style `command`
/// hooks). Copilot's `SessionStart` returns `additionalContext`/`{}` (no `env`
/// block), `PostToolUse` injects pending messages via `additionalContext` and
/// acks delivery. Reuses `run_cursor_hook` — it is a generic "pipe JSON to a
/// native hook" runner, not cursor-specific.
#[test]
fn copilot_e2e_hook_dispatch() {
    let h = Agentx::new();
    let transcript = tempfile::NamedTempFile::new().expect("temp transcript");
    let transcript_path = transcript.path().to_string_lossy().to_string();
    let pid = "pid-cop-123";
    let session_id = "sess-cop-1";

    let me = start_provider_process(&h, "copilot", pid);

    // 1. SessionStart binds the session to the active instance.
    let _ = run_cursor_hook(
        &h,
        "copilot-sessionstart",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "transcript_path": transcript_path,
            "cwd": "/tmp",
        }),
    );

    // Binding is visible via list --json.
    let (code, stdout, stderr) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["session_id"].as_str(), Some(session_id));

    // 2. UserPromptSubmit marks the instance active and refreshes context.
    let prompt_submit = run_cursor_hook(
        &h,
        "copilot-userpromptsubmit",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "transcript_path": transcript_path,
            "prompt": "do a thing",
        }),
    );
    assert!(prompt_submit["additionalContext"].is_string(), "{prompt_submit}");

    let (code, stdout, _) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["status"].as_str(), Some("active"));

    // 3. PreToolUse records tool status and returns an empty object.
    let pre_tool = run_cursor_hook(
        &h,
        "copilot-pretooluse",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "transcript_path": transcript_path,
            "tool_name": "bash",
            "tool_input": { "command": "echo hello" },
        }),
    );
    assert_eq!(pre_tool, serde_json::json!({}));

    // 4. Queue a message from an external sender, then PostToolUse delivers it
    //    via additionalContext. (Self-addressed sends are dropped by the DB
    //    delivery filter, so the assertion below would pass vacuously.)
    let (send_code, _, send_stderr) = h.run(["send", "--from", "bigboss", &format!("@{me}"), "--intent", "request", "--", "ping"]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");

    let post_tool = run_cursor_hook(
        &h,
        "copilot-posttooluse",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "transcript_path": transcript_path,
            "tool_name": "bash",
            "tool_input": { "command": "echo hello" },
            "tool_result": { "text_result_for_llm": "hello" },
        }),
    );
    let injected = post_tool.get("additionalContext").and_then(serde_json::Value::as_str).unwrap_or_else(|| panic!("PostToolUse should inject additionalContext: {post_tool}"));
    assert!(injected.contains("ping"), "delivered context should carry the message text: {injected:?}");
}

/// Pipe a Qoder hook payload to `qoder-*` and return its stdout as JSON
/// (`{}` when the hook wrote nothing, as no-op Qoder hooks do).
fn run_qoder_hook(h: &Agentx, hook: &str, process_id: &str, payload: &serde_json::Value) -> serde_json::Value {
    use std::io::Write;
    use std::process::Stdio;

    let mut cmd = h.cmd();
    cmd.arg(hook);
    cmd.env("AGENTX_PROCESS_ID", process_id);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {hook}: {e}"));
    {
        let mut stdin = child.stdin.take().expect("open stdin");
        stdin.write_all(serde_json::to_string(payload).unwrap().as_bytes()).unwrap();
    }
    let out = child.wait_with_output().unwrap_or_else(|e| panic!("wait {hook}: {e}"));
    assert_eq!(out.status.code().unwrap_or(-1), 0, "{hook} stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.trim().is_empty() {
        return serde_json::json!({});
    }
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("{hook} json: {e}\nstdout={stdout}"))
}

/// End-to-end Qoder CLI hook lifecycle over JSON-on-stdin, with Qoder's
/// Claude-shaped payloads and `hookSpecificOutput` replies.
#[test]
fn qoder_e2e_hook_dispatch() {
    let h = Agentx::new();
    let pid = "pid-qod-123";
    let session_id = "sess-qod-1";
    let transcript_path = "/home/u/.qoder/projects/p/sess-qod-1.jsonl";

    // 1. UserPromptSubmit with no prior SessionStart (untrusted folder): binds
    //    lazily and carries the bootstrap.
    let first = run_qoder_hook(
        &h,
        "qoder-userpromptsubmit",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "transcript_path": transcript_path,
            "cwd": "/tmp",
            "hook_event_name": "UserPromptSubmit",
            "prompt": "do a thing",
        }),
    );
    assert_eq!(first["hookSpecificOutput"]["hookEventName"], "UserPromptSubmit", "{first}");
    let (code, stdout, stderr) = h.run(["list", "--all", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let rows: Vec<serde_json::Value> = serde_json::from_str(&stdout).expect("list json");
    let [v] = rows.as_slice() else { panic!("expected one lazily registered Qoder participant: {stdout}") };
    let me = v["name"].as_str().expect("Qoder participant name");
    assert_eq!(v["session_id"].as_str(), Some(session_id));
    assert_eq!(v["status"].as_str(), Some("active"));

    // 2. PreToolUse records tool status and writes nothing.
    let pre = run_qoder_hook(
        &h,
        "qoder-pretooluse",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "tool_name": "Bash",
            "tool_input": { "command": "echo hello" },
        }),
    );
    assert_eq!(pre, serde_json::json!({}));

    // 3. A queued message is delivered by PostToolUse via additionalContext.
    let (send_code, _, send_stderr) = h.run(["send", "--from", "bigboss", &format!("@{me}"), "--intent", "request", "--", "ping"]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");
    let post = run_qoder_hook(
        &h,
        "qoder-posttooluse",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "tool_name": "Bash",
            "tool_input": { "command": "echo hello" },
            "tool_response": "hello",
        }),
    );
    let injected = post["hookSpecificOutput"]["additionalContext"].as_str().unwrap_or_else(|| panic!("PostToolUse should inject additionalContext: {post}"));
    assert!(injected.contains("ping"), "{injected:?}");
    assert_eq!(post["hookSpecificOutput"]["hookEventName"], "PostToolUse");

    // 3b. A failed tool call delivers too, answering under its own event name.
    let (send_code, _, send_stderr) = h.run(["send", "--from", "bigboss", &format!("@{me}"), "--intent", "inform", "--", "after-failure"]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");
    let failed = run_qoder_hook(
        &h,
        "qoder-posttoolusefailure",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "tool_name": "Bash",
            "tool_input": { "command": "false" },
            "error": "Command exited with non-zero status code 1",
        }),
    );
    assert_eq!(failed["hookSpecificOutput"]["hookEventName"], "PostToolUseFailure", "{failed}");
    assert!(failed["hookSpecificOutput"]["additionalContext"].as_str().unwrap_or("").contains("after-failure"), "{failed}");

    // 4. A message queued during the turn blocks Stop with it as the reason.
    let (send_code, _, send_stderr) = h.run(["send", "--from", "bigboss", &format!("@{me}"), "--intent", "request", "--", "pong"]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");
    let stop = run_qoder_hook(&h, "qoder-stop", pid, &serde_json::json!({ "session_id": session_id, "stop_hook_active": false }));
    assert_eq!(stop["decision"], "block", "{stop}");
    assert!(stop["reason"].as_str().unwrap_or("").contains("pong"));

    // 5. With nothing pending, Stop goes listening and writes nothing.
    let stop = run_qoder_hook(&h, "qoder-stop", pid, &serde_json::json!({ "session_id": session_id, "stop_hook_active": true }));
    assert_eq!(stop, serde_json::json!({}));
    let (_, stdout, _) = h.run(["list", me, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["status"].as_str(), Some("listening"));

    // 6. SessionEnd for the bound session removes the instance; `clear` does not.
    let _ = run_qoder_hook(&h, "qoder-sessionend", pid, &serde_json::json!({ "session_id": session_id, "reason": "clear" }));
    let (code, _, _) = h.run(["list", me, "--json"]);
    assert_eq!(code, 0, "clear must not end the instance");
    let _ = run_qoder_hook(&h, "qoder-sessionend", pid, &serde_json::json!({ "session_id": session_id, "reason": "other" }));
    let (code, _, _) = h.run(["list", me, "--json"]);
    assert_ne!(code, 0, "SessionEnd for the bound session ends the instance");
}

/// Pipe argv to a native argv-style hook and return its parsed stdout.
fn run_argv_hook(h: &Agentx, hook: &str, process_id: Option<&str>, args: &[&str]) -> serde_json::Value {
    let mut cmd = h.cmd();
    cmd.arg(hook);
    cmd.args(args);
    if let Some(process_id) = process_id {
        cmd.env("AGENTX_PROCESS_ID", process_id);
    }

    let out = cmd.output().unwrap_or_else(|e| panic!("spawn {hook}: {e}"));
    assert_eq!(out.status.code().unwrap_or(-1), 0, "{hook} stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("{hook} json: {e}\nstdout={stdout}"))
}

/// End-to-end Pi argv hook lifecycle.
///
/// Pi's extension invokes agentx with argv, not JSON stdin. This test mirrors the
/// native hook smoke tests above while staying hermetic: no real Pi process is
/// launched, only a fake process binding plus the `pi-*` hook commands.
#[test]
fn pi_e2e_hook_dispatch() {
    let h = Agentx::new();
    let transcript = tempfile::NamedTempFile::new().expect("temp transcript");
    let transcript_path = transcript.path().to_string_lossy().to_string();
    let pid = "pid-pi-123";
    let session_id = "sess-pi-1";

    let me = start_provider_process(&h, "pi", pid);

    // 1. pi-start binds the session and returns bootstrap context to the plugin.
    let cwd = h.root.path().to_string_lossy().to_string();
    let start = run_argv_hook(&h, "pi-start", Some(pid), &["--session-id", session_id, "--transcript-path", &transcript_path, "--cwd", &cwd]);
    assert_eq!(start["name"].as_str(), Some(me.as_str()));
    assert_eq!(start["session_id"].as_str(), Some(session_id));
    assert!(start["bootstrap"].as_str().is_some_and(|text| text.contains(&format!("--iam {me}"))), "pi-start should return bootstrap with the agent identity: {start}");

    let (code, stdout, stderr) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["tool"].as_str(), Some("pi"));
    assert_eq!(v["session_id"].as_str(), Some(session_id));
    assert_eq!(v["transcript_path"].as_str(), Some(transcript_path.as_str()));
    assert_eq!(v["directory"].as_str(), Some(cwd.as_str()));

    // 2. pi-status marks active/listening transitions.
    let status = run_argv_hook(&h, "pi-status", Some(pid), &["--name", &me, "--status", "active", "--context", "prompt", "--detail", "working"]);
    assert_eq!(status, serde_json::json!({ "ok": true }));
    let (code, stdout, _) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["status"].as_str(), Some("active"));

    // 3. pi-beforetool records tool status (observation only, never blocks).
    let before_tool = run_argv_hook(&h, "pi-beforetool", Some(pid), &["--name", &me, "--tool", "bash", "--input-json", r#"{"command":"echo hello"}"#]);
    assert_eq!(before_tool, serde_json::json!({ "ok": true }));
    let (code, stdout, _) = h.run(["events", "--agent", &me, "--type", "status", "--last", "5"]);
    assert_eq!(code, 0);
    let tool_status: serde_json::Value =
        stdout.lines().filter_map(|line| serde_json::from_str(line).ok()).find(|event: &serde_json::Value| event["data"]["context"] == "tool:bash").unwrap_or_else(|| panic!("tool:bash status event missing: {stdout}"));
    assert!(tool_status["data"]["detail"].as_str().is_some_and(|detail| detail.contains("echo hello")), "tool detail should include bash command: {tool_status}");

    // 4. pi-read exposes pending messages and can ack the cursor.
    let (send_code, _, send_stderr) = h.run(["send", "--from", "bigboss", &format!("@{me}"), "--intent", "request", "--", "ping"]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");

    let check = run_argv_hook(&h, "pi-read", Some(pid), &["--name", &me, "--check"]);
    assert_eq!(check, serde_json::json!(true));

    // Another launch (e.g. this one after stop + resume elsewhere) cannot read the inbox.
    let stale = run_argv_hook(&h, "pi-read", Some("pid-stale"), &["--name", &me]);
    assert!(stale.get("error").is_some(), "stale read: {stale}");

    let batch = run_argv_hook(&h, "pi-read", Some(pid), &["--name", &me]);
    assert_eq!(batch["has_more"], false, "pi-read batch: {batch}");
    let ping = batch["messages"].as_array().and_then(|items| items.iter().find(|m| m["message"] == "ping")).unwrap_or_else(|| panic!("pi-read should return pending ping: {batch}"));
    let ping_id = ping["event_id"].as_i64().expect("event_id").to_string();

    let receipt = batch["receipt"].to_string();
    let no_receipt = run_argv_hook(&h, "pi-read", Some(pid), &["--name", &me, "--ack", "--up-to", &ping_id]);
    assert!(no_receipt["error"].as_str().is_some_and(|error| error.contains("--receipt")), "{no_receipt}");
    let ack = run_argv_hook(&h, "pi-read", Some(pid), &["--name", &me, "--ack", "--up-to", &ping_id, "--receipt", &receipt]);
    assert_eq!(ack["acked_to"].to_string(), ping_id);
    let check = run_argv_hook(&h, "pi-read", Some(pid), &["--name", &me, "--check"]);
    assert_eq!(check, serde_json::json!(false));

    // 5. pi-stop finalizes the session.
    let stop = run_argv_hook(&h, "pi-stop", Some(pid), &["--name", &me, "--reason", "done"]);
    assert_eq!(stop, serde_json::json!({ "ok": true }));
    let (code, stdout, _) = h.run(["events", "--agent", &me, "--action", "stopped", "--last", "5"]);
    assert_eq!(code, 0);
    let stopped: serde_json::Value = stdout.lines().find_map(|line| serde_json::from_str(line).ok()).unwrap_or_else(|| panic!("stopped event missing: {stdout}"));
    assert_eq!(stopped["data"]["action"].as_str(), Some("stopped"));
}

enum UnreadTiming {
    Preexisting,
    ArrivingAfterReadiness,
}

struct ChildGuard {
    child: Option<std::process::Child>,
}

impl ChildGuard {
    fn new(child: std::process::Child) -> Self {
        Self { child: Some(child) }
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.as_mut().unwrap().try_wait()
    }

    /// Wait for the child to exit up to `timeout`. Kills and reaps if the deadline is exceeded.
    /// Captures all stdout and stderr.
    fn wait_bounded(mut self, timeout: Duration) -> (i32, String, String) {
        let deadline = Instant::now() + timeout;
        let mut child = self.child.take().unwrap();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let mut stdout = Vec::new();
                    let mut stderr = Vec::new();
                    if let Some(mut out) = child.stdout.take() {
                        use std::io::Read;
                        let _ = out.read_to_end(&mut stdout);
                    }
                    if let Some(mut err) = child.stderr.take() {
                        use std::io::Read;
                        let _ = err.read_to_end(&mut stderr);
                    }
                    return (status.code().unwrap_or(-1), String::from_utf8_lossy(&stdout).into_owned(), String::from_utf8_lossy(&stderr).into_owned());
                }
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("events --wait CLI child process exceeded bounded deadline of {timeout:?}");
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("events --wait CLI child try_wait error: {e}");
                }
            }
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn run_events_wait_cli_oracle(timing: UnreadTiming, wait_secs: u64, expected_code: i32) {
    let h = Agentx::new();
    let me = h.start();
    let other = h.start();
    let db_path = h.agentx_dir.join("agentx.db");
    let conn = rusqlite::Connection::open(&db_path).expect("open test db");

    conn.execute("DELETE FROM events", []).unwrap();
    conn.execute("UPDATE instances SET last_event_id = 0 WHERE name IN (?1, ?2)", rusqlite::params![me, other]).unwrap();

    let mut send_code = None;
    if matches!(timing, UnreadTiming::Preexisting) {
        let (sc, _, _) = h.run(["send", &format!("@{me}"), "--iam", &other, "--", "pre"]);
        send_code = Some(sc);
    }

    let initial_cursor: i64 = conn.query_row("SELECT last_event_id FROM instances WHERE name = ?1", rusqlite::params![me], |r| r.get(0)).unwrap_or(0);

    let child = h.cmd().args(["events", "--wait", &wait_secs.to_string(), "--sql", "data LIKE '%sol002_target%'", "--iam", &me]).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().expect("spawn events wait");
    let mut child = ChildGuard::new(child);

    let read_observer_port = |conn: &rusqlite::Connection| conn.query_row("SELECT port FROM notify_endpoints WHERE instance LIKE 'event-observer:observe-%' AND kind = 'events_wait' LIMIT 1", [], |row| row.get::<_, u16>(0)).ok();
    let mut observer_port = None;
    if matches!(timing, UnreadTiming::ArrivingAfterReadiness) {
        for _ in 0..100 {
            observer_port = read_observer_port(&conn);
            if observer_port.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(observer_port.is_some(), "waiter notify endpoint must be registered before sending arriving message");

        let (sc, _, _) = h.run(["send", &format!("@{me}"), "--iam", &other, "--", "arr"]);
        send_code = Some(sc);
    }

    std::thread::sleep(Duration::from_millis(300));
    let mid_wait_cursor: i64 = conn.query_row("SELECT last_event_id FROM instances WHERE name = ?1", rusqlite::params![me], |r| r.get(0)).unwrap_or(0);
    let premature_exit = child.try_wait().unwrap_or(None);

    if expected_code == 0 && premature_exit.is_none() {
        let data = serde_json::json!({"status": "active", "detail": "sol002_target"});
        conn.execute("INSERT INTO events (timestamp, type, instance, data) VALUES (datetime('now'), 'status', ?1, ?2)", rusqlite::params![me, data.to_string()]).expect("insert matching status event");
        // Raw fixture inserts bypass daemon post-commit notifications.
        let port = observer_port.expect("ready observer endpoint");
        std::net::TcpStream::connect(("127.0.0.1", port)).expect("wake observer for matching status event");
    }

    let deadline_secs = wait_secs + 2;
    let (code, stdout, stderr) = child.wait_bounded(Duration::from_secs(deadline_secs));

    assert!(read_observer_port(&conn).is_none(), "observer endpoint leaked after command exit");
    let endpoint_ready = observer_port.is_some();
    let preview_pattern = format!("<agentx>{other} → {me}</agentx>");
    let preview_count = stdout.matches(&preview_pattern).count();

    let send_ok = send_code.is_none_or(|c| c == 0);
    let pending = premature_exit.is_none();
    // A timeout may legitimately finish before the mid-wait probe on a busy
    // runner. Its final exit code and unchanged cursor still reject a false
    // match. Success scenarios must remain pending until we insert the match.
    let wait_state_ok = pending || (expected_code == 1 && premature_exit.is_some_and(|status| status.code() == Some(1)));
    let oracle_passed = send_ok && wait_state_ok && mid_wait_cursor == initial_cursor && preview_count <= 1 && code == expected_code;

    assert!(
        oracle_passed,
        "events --wait oracle failed (pending={pending}, premature_exit={premature_exit:?}, mid_cursor={mid_wait_cursor}, initial_cursor={initial_cursor}, preview_count={preview_count}, code={code}, expected_code={expected_code}, endpoint_ready={endpoint_ready}): stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn events_wait_cli_preexisting_unread_times_out_with_one() {
    run_events_wait_cli_oracle(UnreadTiming::Preexisting, 2, 1);
}

#[test]
fn events_wait_cli_arriving_unread_then_matching_status_exits_zero() {
    run_events_wait_cli_oracle(UnreadTiming::ArrivingAfterReadiness, 4, 0);
}

#[test]
fn commands_on_stopped_agent_explain_when_and_how_to_resume() {
    let h = Agentx::new();
    let me = h.start();
    let gone = h.start();
    let (cs, _, es) = h.run(["stop", &gone]);
    assert_eq!(cs, 0, "stop failed: {es}");

    // kill/stop of an already-stopped agent is a no-op success, not "not found".
    for cmd in ["kill", "stop"] {
        let (code, stdout, stderr) = h.run([cmd, &gone]);
        assert_eq!(code, 0, "{cmd}: stdout={stdout} stderr={stderr}");
        assert!(stdout.contains(&format!("'{gone}' stopped ")), "{cmd}: stdout={stdout}");
        assert!(stdout.contains(&format!("agentx r {gone}")), "{cmd}: stdout={stdout}");
    }

    for args in [vec!["list", gone.as_str()], vec!["term", gone.as_str()]] {
        let (code, _stdout, stderr) = h.run(args.clone());
        assert_eq!(code, 1, "{args:?}");
        assert!(stderr.contains(&format!("'{gone}' stopped ")), "{args:?}: stderr={stderr}");
    }

    let (code, _stdout, stderr) = h.run(["send", &format!("@{gone}"), "--iam", &me, "--", "hi"]);
    assert_eq!(code, 1);
    assert!(stderr.contains(&format!("@{gone} stopped")) && stderr.contains(&format!("agentx r {gone}")), "stderr={stderr}");
}

#[test]
fn unknown_agent_gets_typo_suggestion() {
    let h = Agentx::new();
    let me = h.start();
    let typo: String = {
        let mut c: Vec<char> = me.chars().collect();
        c.swap(1, 2);
        c.into_iter().collect()
    };
    if typo == me {
        return; // name with repeated letters; swap is a no-op
    }
    let (code, _stdout, stderr) = h.run(["kill", &typo]);
    assert_eq!(code, 1);
    assert!(stderr.contains(&format!("Agent '{typo}' not found")) && stderr.contains(&format!("Did you mean: {me}?")), "stderr={stderr}");
}

#[test]
fn send_rejects_ambiguous_bare_word_and_self_target() {
    let h = Agentx::new();
    let me = h.start();
    let other = h.start();

    // A lone word without '@' or '--' must not silently broadcast.
    let (code, _stdout, stderr) = h.run(["send", &other, "--iam", &me]);
    assert_eq!(code, 1);
    assert!(stderr.contains(&format!("Did you mean @{other}?")), "stderr={stderr}");

    let (code, _stdout, stderr) = h.run(["send", "hello", "--iam", &me]);
    assert_eq!(code, 1);
    assert!(stderr.contains("agentx send -- hello"), "stderr={stderr}");

    let (code, _stdout, stderr) = h.run(["send", &format!("@{me}"), "--iam", &me, "--", "x"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("do not receive their own messages"), "stderr={stderr}");

    // bigboss:DEVICE routes to bigboss, so it is also a self-only target.
    let (code, _stdout, stderr) = h.run(["send", "-b", "@bigboss:ABCD", "--", "x"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("do not receive their own messages"), "stderr={stderr}");
}

#[test]
fn unknown_command_and_tool_suggest_corrections() {
    let h = Agentx::new();
    let (code, _stdout, stderr) = h.run(["lst"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("Did you mean: list?"), "stderr={stderr}");

    let (code, _stdout, stderr) = h.run(["launch", "--tool", "claud"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("Unknown tool 'claud'"), "stderr={stderr}");
}

#[test]
fn send_warns_when_name_disagrees_with_the_shell_identity() {
    let h = Agentx::new();
    let sender = h.start_with_process_id("pid-sender");
    let other = h.start_with_process_id("pid-other");

    // pid-sender's shell is bound to `sender`, but it sends under `other` —
    // exactly the shape of an identity that drifted after a rebind or recovery.
    let (code, stdout, stderr) = h.run_as_process("pid-sender", ["send", "--iam", &other, "--", "hi"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stderr.contains(&sender) && stderr.contains(&other), "expected a drift warning naming both identities, got stderr={stderr}");
}

#[test]
fn send_is_quiet_when_name_matches_the_shell_identity() {
    let h = Agentx::new();
    let sender = h.start_with_process_id("pid-sender");

    let (code, stdout, stderr) = h.run_as_process("pid-sender", ["send", "--iam", &sender, "--", "hi"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(!stderr.contains("warning"), "a matching --iam must not warn, got stderr={stderr}");
}

#[test]
fn send_with_from_skips_the_name_drift_warning() {
    let h = Agentx::new();
    let _sender = h.start_with_process_id("pid-sender");
    let other = h.start_with_process_id("pid-other");

    // With --from the sender is the external name, so --iam disagreeing with
    // the shell's binding says nothing about who is sending.
    let (code, stdout, stderr) = h.run_as_process("pid-sender", ["send", "--iam", &other, "--from", "bigboss", "--", "hi"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(!stderr.contains("warning"), "--from must suppress the drift warning, got stderr={stderr}");
}

#[test]
fn from_in_message_text_does_not_suppress_the_name_drift_warning() {
    let h = Agentx::new();
    let sender = h.start_with_process_id("pid-sender");
    let other = h.start_with_process_id("pid-other");

    let (code, stdout, stderr) = h.run_as_process("pid-sender", ["send", "--iam", &other, "--", "use", "--from", "x"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stderr.contains(&sender) && stderr.contains(&other), "--from after -- is message text, got stderr={stderr}");
}

#[test]
fn send_with_attached_from_skips_the_name_drift_warning() {
    let h = Agentx::new();
    let _sender = h.start_with_process_id("pid-sender");
    let other = h.start_with_process_id("pid-other");

    let (code, stdout, stderr) = h.run_as_process("pid-sender", ["send", "--iam", &other, "--from=bigboss", "--", "hi"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(!stderr.contains("warning"), "--from=NAME must suppress the drift warning, got stderr={stderr}");
}

#[test]
fn run_help_and_docs_flags_preserve_script_help() {
    let h = Agentx::new();
    let caller = h.start();
    for args in [vec!["run", "--help"], vec!["run", "-h", "--iam", &caller], vec!["run", "docs", "--help", "--iam", &caller]] {
        let (code, stdout, stderr) = h.run(&args);
        assert_eq!(code, 0, "{args:?}: {stderr}");
        assert!(stdout.contains("agentx run <name>"), "{stdout}");
        assert!(!stdout.contains("# Creating Custom Scripts"), "{stdout}");
    }
    let (code, stdout, stderr) = h.run(["run", "debate", "--help", "--iam", &caller]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("Usage: agentx run debate [OPTIONS] TOPIC"), "{stdout}");
    let (code, stdout, stderr) = h.run(["run", "docs", "--scripts", "--iam", &caller]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("# Creating Custom Scripts"), "{stdout}");
    assert!(!stdout.contains("# CLI Reference"), "{stdout}");
    let (code, stdout, stderr) = h.run(["run", "docs", "--scrpits", "--iam", &caller]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("Unknown docs option: --scrpits"), "{stderr}");
}

#[test]
fn config_reports_file_parse_errors_in_get_and_list_modes() {
    let h = Agentx::new();
    std::fs::write(h.agentx_dir.join("config.toml"), "timeout = [broken\n").unwrap();
    for args in [vec!["config", "timeout"], vec!["config", "terminal"], vec!["config", "dev_root"], vec!["config"], vec!["config", "--json"]] {
        let (code, stdout, stderr) = h.run(args);
        assert_eq!(code, 1, "{stderr}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.contains("environment settings contain invalid TOML") && stderr.contains("config.toml"), "{stderr}");
        assert!(!stderr.contains("[broken"), "parser diagnostic leaked the config value: {stderr}");
    }
}

/// Spawn `agentx events --follow <args>` and forward its stdout lines over a
/// channel, so tests can wait for each line with a timeout.
fn spawn_follow(h: &Agentx, args: &[&str]) -> (std::process::Child, std::sync::mpsc::Receiver<String>) {
    use std::io::BufRead;
    let mut child = h.cmd().args(["events", "--follow"]).args(args).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn().expect("spawn events --follow");
    let stdout = child.stdout.take().expect("follow stdout");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    (child, rx)
}

fn next_follow_line(rx: &std::sync::mpsc::Receiver<String>) -> serde_json::Value {
    let line = rx.recv_timeout(Duration::from_secs(10)).expect("events --follow produced no line within 10s");
    serde_json::from_str(&line).unwrap_or_else(|e| panic!("follow line is not JSON ({e}): {line}"))
}

#[test]
fn events_follow_streams_new_matches_and_resumes_from_cursor() {
    let h = Agentx::new();
    let agent = h.start();
    let send = |text: &str| {
        let (code, _, stderr) = h.run(["send", "-b", &format!("@{agent}"), "--", text]);
        assert_eq!(code, 0, "send failed: {stderr}");
    };

    send("first");
    // --last 1 replays the existing message; receiving it proves the follower
    // has fixed its cursor, so the next send is guaranteed to stream.
    let (mut follower, rx) = spawn_follow(&h, &["--type", "message", "--last", "1"]);
    let first = next_follow_line(&rx);
    assert_eq!(first["data"]["text"], "first", "backlog line: {first}");
    send("second");
    let second = next_follow_line(&rx);
    assert_eq!(second["data"]["text"], "second", "streamed line: {second}");
    assert!(second["id"].as_i64() > first["id"].as_i64());
    let _ = follower.kill();
    let _ = follower.wait();

    // A saved cursor replays everything after it, in id order.
    let cursor = first["id"].as_i64().expect("event id").to_string();
    let (mut resumed, rx) = spawn_follow(&h, &["--type", "message", "--after-id", &cursor]);
    let replayed = next_follow_line(&rx);
    assert_eq!(replayed["id"], second["id"], "resumed line: {replayed}");
    let _ = resumed.kill();
    let _ = resumed.wait();
}

#[cfg(unix)]
#[test]
fn events_follow_exits_when_reader_closes() {
    let h = Agentx::new();
    let mut child = h.cmd().args(["events", "--follow"]).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn().expect("spawn events --follow");
    // Close the read end without any event arriving: an idle follower must
    // notice on its own instead of waiting for a write to fail.
    drop(child.stdout.take());
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("events --follow kept running after its reader closed");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(status.success(), "follow exit status: {status}");
}

#[test]
fn events_follow_rejects_conflicting_flags() {
    let h = Agentx::new();
    for (args, expected) in [
        (vec!["events", "--after-id", "1"], "--after-id requires --follow"),
        (vec!["events", "--follow", "--wait", "5"], "--follow cannot be combined with --wait"),
        (vec!["events", "--follow", "--after-id", "1", "--last", "2"], "--last and --after-id are mutually exclusive"),
        (vec!["events", "--follow", "--after-id", "999999"], "is past the newest event"),
    ] {
        let (code, _, stderr) = h.run(&args);
        assert_eq!(code, 1, "{args:?}: stderr={stderr}");
        assert!(stderr.contains(expected), "{args:?}: stderr={stderr}");
    }
}

#[test]
fn events_follow_exits_on_query_error() {
    let h = Agentx::new();
    // The native predicate guard rejects unbound parameters before following.
    let (code, stdout, stderr) = h.run(["events", "--follow", "--sql", "type = ?"]);
    assert_eq!(code, 2, "stderr={stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("cannot contain unbound parameters"), "stderr={stderr}");
}

#[test]
fn kill_without_target_rejects_nonterminal_and_json_without_effects() {
    let h = Agentx::new();
    // Startup's context/bootstrap read resolves the caller after registration.
    // An owned process binding lets that same invocation resolve its new row.
    let agent = h.start_with_process_id("kill-no-target-owner");
    let db = rusqlite::Connection::open_with_flags(h.agentx_dir.join("agentx.db"), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let snapshot = || {
        let row = db.query_row("SELECT name, created_at, status FROM instances WHERE name=?1", [&agent], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?.to_bits(), row.get::<_, String>(2)?))).unwrap();
        let events = db.query_row("SELECT count(*) FROM events", [], |row| row.get::<_, i64>(0)).unwrap();
        (row, events)
    };
    let before = snapshot();
    for args in [vec!["kill"], vec!["kill", "--json"]] {
        // The shared bounded runner supplies null stdin and owns child cleanup.
        let (code, stdout, error) = h.run(&args);
        assert_eq!(code, 1, "{args:?}: {error}");
        assert!(stdout.is_empty(), "omitted target must not report a kill result");
        assert!(error.contains("needs an explicit target in noninteractive or --json mode"), "{error}");
        assert!(!error.contains("\x1b[?1049h"), "noninteractive rejection must not enter the picker");
        assert_eq!(snapshot(), before, "omitted target changed the exact participant or events");
    }
}
