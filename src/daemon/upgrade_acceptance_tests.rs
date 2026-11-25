//! Process acceptance with original release executables. Every daemon and DB
//! belongs to a disposable scope. Windows uses the same native stop request.
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::{DaemonScope, lifecycle};
use serde_json::Value;

struct Disposable {
    scope: DaemonScope,
    old: Child,
}

impl Drop for Disposable {
    fn drop(&mut self) {
        let _ = super::shutdown(&self.scope);
        if self.old.try_wait().ok().flatten().is_none() {
            let _ = self.old.kill();
        }
        let _ = self.old.wait();
    }
}

fn command(binary: &Path, root: &Path, scope: &Path) -> Command {
    let mut command = Command::new(binary);
    command.env_clear();
    for name in ["PATH", "SystemRoot", "WINDIR", "COMSPEC", "PATHEXT"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
        .current_dir(root.join("workspace"))
        .env("AGENTX_DIR", scope)
        .env("HOME", root.join("home"))
        .env("USERPROFILE", root.join("home"))
        .env("APPDATA", root.join("home"))
        .env("LOCALAPPDATA", root.join("home"))
        .env("TMPDIR", root.join("tmp"))
        .env("TEMP", root.join("tmp"))
        .env("TMP", root.join("tmp"))
        .env("CI", "1")
        .env("NO_COLOR", "1")
        .env("AGENTX_UPDATE_VERIFY", "1")
        .stdin(Stdio::null());
    command
}

fn cli(binary: &Path, root: &Path, scope: &Path, args: &[&str]) -> String {
    let output = command(binary, root, scope).args(args).output().unwrap();
    assert!(output.status.success(), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

fn start(binary: &Path, root: &Path, name: &str) -> Disposable {
    let scope = DaemonScope::prepare(root.join(name)).unwrap();
    let old = command(binary, root, scope.root()).args(["daemon", "serve", "--scope"]).arg(scope.root()).stdout(Stdio::null()).stderr(std::fs::File::create(root.join(format!("{name}.stderr"))).unwrap()).spawn().unwrap();
    let mut fixture = Disposable { scope, old };
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        let status = command(binary, root, fixture.scope.root()).args(["daemon", "status"]).output().unwrap();
        if status.status.success() && String::from_utf8_lossy(&status.stdout).lines().any(|line| line == "Daemon: running") {
            return fixture;
        }
        assert!(fixture.old.try_wait().unwrap().is_none(), "old release daemon exited");
        assert!(Instant::now() < end, "old release daemon never became ready");
        std::thread::park_timeout(Duration::from_millis(30));
    }
}

fn binary(variable: &str) -> PathBuf {
    std::fs::canonicalize(std::env::var_os(variable).unwrap_or_else(|| panic!("set {variable} to the exact release executable"))).unwrap()
}

#[test]
#[ignore = "requires source-built AGENTX_ACCEPTANCE_BINARY and real AGENTX_UPGRADE_V0731 and AGENTX_UPGRADE_V0733 binaries for this OS"]
fn published_daemons_drain_without_changing_the_neighbor_or_old_receipt() {
    let current = binary("AGENTX_ACCEPTANCE_BINARY");
    for (variable, version) in [("AGENTX_UPGRADE_V0731", "0.7.31"), ("AGENTX_UPGRADE_V0733", "0.7.33")] {
        let old = binary(variable);
        let root = tempfile::Builder::new().prefix("ax-upgrade-").tempdir().unwrap();
        let root = root.path();
        for directory in ["home", "tmp", "workspace"] {
            std::fs::create_dir(root.join(directory)).unwrap();
        }
        assert_eq!(cli(&old, root, root, &["--version"]).trim(), format!("agentx {version}"));
        let mut target = start(&old, root, "target");
        let mut neighbor = start(&old, root, "neighbor");
        let scope = target.scope.root();
        let other = neighbor.scope.root();
        let before = cli(&old, root, scope, &["daemon", "status"]);
        let neighbor_before = cli(&old, root, other, &["daemon", "status"]);
        let registered = command(&old, root, scope).args(["start", "--name", "target"]).env("AGENTX_PROCESS_ID", "upgrade-fixture-binding").output().unwrap();
        assert!(registered.status.success(), "old registration: {}", String::from_utf8_lossy(&registered.stderr));
        let marker = format!("retained across {version}");
        let sent: Value = serde_json::from_str(&cli(&old, root, scope, &["send", "--from", "upgrade-fixture", "--json", "@target", "--", &marker])).unwrap();
        let event_id = sent["event_id"].as_i64().unwrap();
        // The published CLI omits the request ID from its JSON. Read back only
        // the test-owned receipt key so the candidate can reconcile it via IPC.
        let request_id: String = {
            let db = rusqlite::Connection::open_with_flags(scope.join("agentx.db"), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            db.query_row("SELECT request_id FROM message_acceptances WHERE event_id = ?", [event_id], |row| row.get(0)).unwrap()
        };
        let upgrade = cli(&current, root, scope, &["daemon", "restart"]);
        assert!(upgrade.contains("Native IPC upgraded.") && upgrade.contains("Reopen Agents"), "{upgrade}");
        assert!(target.old.wait().unwrap().success(), "old release did not drain successfully");
        let status = lifecycle::status(scope, scope).unwrap();
        let info = status.info.unwrap();
        assert!(before.contains(&format!("generation={}", info.database_generation)));
        assert!(!before.contains(&format!("instance={}", info.daemon_instance)));
        assert_eq!(neighbor_before, cli(&old, root, other, &["daemon", "status"]));
        assert!(neighbor.old.try_wait().unwrap().is_none());
        let receipt: Value = serde_json::from_str(&cli(&current, root, scope, &["send", "--receipt", &request_id, "--json"])).unwrap();
        assert_eq!(receipt["request_id"], request_id);
        assert_eq!(receipt["event_id"], event_id);
        assert_eq!(receipt["delivered_to"], sent["delivered_to"]);
        let events = cli(&current, root, scope, &["events", "--last", "30", "--full"]);
        assert_eq!(events.lines().filter(|line| line.contains(&marker)).count(), 1, "{events}");
        let second = cli(&current, root, scope, &["daemon", "restart"]);
        assert!(!second.contains("Native IPC upgraded."), "{second}");
        println!("{version}: exact old process drained, neighbor unchanged, generation and original event/receipt retained");
    }
}
