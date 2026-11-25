//! Originating-host shutdown of one recorded Relay worker. Never stops the
//! data daemon, reconnects a Client or interprets unknown observation as exit.

use super::WorkerOwner;
use crate::service::client::Client;
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct CapturedWorker {
    pid_text: Option<String>,
    owner_text: Option<String>,
    owner: Option<WorkerOwner>,
}

#[derive(Clone, Debug)]
struct ProcessEvidence {
    exited: bool,
    incarnation: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Signal {
    Terminate,
    Kill,
}

/// Private host seam for exact observation/signal cases; no general effect RPC.
trait ProcessControl {
    fn observe(&mut self, pid: u32) -> ProcessEvidence;
    fn signal(&mut self, owner: &WorkerOwner, signal: Signal) -> Result<(), String>;
    fn pause(&mut self);
}

struct HostControl<'a> {
    scope: &'a Path,
    captured: &'a CapturedWorker,
}

/// Confirm shutdown of the original scoped worker before clearing native Relay
/// runtime. Any uncertain process/IPC result is an error; the Client is retained.
pub(crate) fn stop_relay_worker_client(client: &Client) -> Result<(), String> {
    let status = match crate::relay::persistence::call(client, crate::relay::service::Request::Status { configured: true, enabled: true, broker: None }).map_err(|error| error.to_string())? {
        crate::relay::service::Response::Status(status) => status,
        _ => return Err("unexpected Relay worker preflight response".into()),
    };
    let worker_port = match crate::relay::persistence::call(client, crate::relay::service::Request::WorkerPort).map_err(|error| error.to_string())? {
        crate::relay::service::Response::WorkerPort(port) => port,
        _ => return Err("unexpected Relay worker port response".into()),
    };
    let scope = client.scope_root();
    let _lock = super::lock_spawn_in(scope)?;
    let captured = CapturedWorker::capture(scope)?;
    if captured.owner.is_none() && (worker_port.is_some() || status.heartbeat_age.is_some() || matches!(status.status.as_deref(), Some("ok" | "connecting" | "error"))) {
        return Err("Relay runtime has worker evidence but no recorded owner; shutdown outcome unknown".into());
    }
    if let Some(owner) = &captured.owner {
        if owner.pid == std::process::id() || owner.pid == client.info().map_err(|error| error.to_string())?.pid {
            return Err("refusing to stop this process or the data daemon as a Relay worker".into());
        }
        stop_owned_process(owner, &mut HostControl { scope, captured: &captured })?;
    }
    finish_stop(&captured, scope, || {
        match crate::relay::persistence::call(client, crate::relay::service::Request::WorkerStopped).map_err(|error| format!("Relay worker exit was confirmed but WorkerStopped cleanup is unknown: {error}"))? {
            crate::relay::service::Response::Changed(true) => {}
            _ => return Err("Relay worker exit was confirmed but WorkerStopped cleanup was not confirmed".into()),
        }
        match crate::relay::persistence::call(client, crate::relay::service::Request::ClearWorkerRuntime).map_err(|error| format!("Relay worker exit was confirmed but runtime cleanup is unknown: {error}"))? {
            crate::relay::service::Response::Changed(true) => Ok(()),
            _ => Err("Relay worker exit was confirmed but runtime cleanup was not confirmed".into()),
        }
    })
}

fn finish_stop(captured: &CapturedWorker, scope: &Path, clear_runtime: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    captured.verify_current(scope, true)?;
    clear_runtime()?;
    captured.remove_files(scope)
}

fn read_optional(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

impl CapturedWorker {
    fn capture(scope: &Path) -> Result<Self, String> {
        let pid_text = read_optional(&scope.join(".tmp").join("relay.pid"))?;
        let owner_text = read_optional(&super::owner_file_path_in(scope))?;
        let pid = pid_text.as_deref().map(|text| text.trim().parse::<u32>().map_err(|_| "invalid Relay pidfile".to_string())).transpose()?;
        let owner: Option<WorkerOwner> = owner_text.as_deref().map(|text| serde_json::from_str(text).map_err(|error| format!("invalid Relay worker owner: {error}"))).transpose()?;
        if pid.zip(owner.as_ref()).is_some_and(|(pid, owner)| pid != owner.pid) {
            return Err("Relay PID and owner records disagree; no shutdown attempted".into());
        }
        let owner = owner.or_else(|| pid.map(|pid| WorkerOwner { pid, incarnation: None }));
        if let Some(owner) = &owner
            && (owner.pid <= 1 || cfg!(unix) && i32::try_from(owner.pid).is_err())
        {
            return Err("invalid Relay worker PID; no shutdown attempted".into());
        }
        Ok(Self { pid_text, owner_text, owner })
    }

    /// A missing pair is permitted only after the original exit was confirmed;
    /// any partial/replaced pair remains a fence failure.
    fn verify_current(&self, scope: &Path, allow_fully_removed_owner: bool) -> Result<(), String> {
        let current = Self::capture(scope)?;
        if current.pid_text == self.pid_text && current.owner_text == self.owner_text {
            return Ok(());
        }
        if allow_fully_removed_owner && current.pid_text.is_none() && current.owner_text.is_none() {
            return Ok(());
        }
        Err("Relay worker ownership changed; successor records retained".into())
    }

    fn remove_files(&self, scope: &Path) -> Result<(), String> {
        for (path, expected) in [(scope.join(".tmp").join("relay.pid"), &self.pid_text), (super::owner_file_path_in(scope), &self.owner_text)] {
            let Some(actual) = read_optional(&path)? else {
                continue;
            };
            if expected.as_ref() != Some(&actual) {
                return Err("Relay worker ownership changed; successor file retained".into());
            }
            std::fs::remove_file(&path).map_err(|error| format!("cannot remove {} after worker exit: {error}", path.display()))?;
        }
        Ok(())
    }
}

/// Known PID reuse proves the original exited, but never grants a signal to the
/// new occupant. Unknown identity and observation failures prove neither fact.
fn original_exited(owner: &WorkerOwner, evidence: ProcessEvidence) -> Result<bool, String> {
    if evidence.exited {
        return Ok(true);
    }
    let expected = owner.incarnation.as_deref().filter(|identity| !identity.is_empty());
    let observed = evidence.incarnation.as_deref().filter(|identity| !identity.is_empty());
    match (expected, observed) {
        (Some(expected), Some(observed)) => Ok(expected != observed),
        _ => Err(format!("cannot verify original Relay worker {} incarnation; shutdown outcome unknown", owner.pid)),
    }
}

impl ProcessControl for HostControl<'_> {
    fn observe(&mut self, pid: u32) -> ProcessEvidence {
        ProcessEvidence { exited: crate::sys::process::has_exited(pid), incarnation: crate::sys::process::identity(pid) }
    }

    fn signal(&mut self, owner: &WorkerOwner, signal: Signal) -> Result<(), String> {
        self.captured.verify_current(self.scope, false)?;
        if original_exited(owner, self.observe(owner.pid))? {
            return Ok(());
        }
        // Compare the retained nonempty incarnation immediately before each
        // PID signal. The signal result alone never confirms process exit.
        match signal {
            Signal::Terminate => {
                crate::sys::process::terminate(owner.pid);
            }
            Signal::Kill => {
                crate::sys::process::kill(owner.pid);
            }
        }
        Ok(())
    }

    fn pause(&mut self) {
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn stop_owned_process(owner: &WorkerOwner, host: &mut impl ProcessControl) -> Result<(), String> {
    if original_exited(owner, host.observe(owner.pid))? {
        return Ok(());
    }
    host.signal(owner, Signal::Terminate)?;
    for _ in 0..50 {
        if original_exited(owner, host.observe(owner.pid))? {
            return Ok(());
        }
        host.pause();
    }
    if original_exited(owner, host.observe(owner.pid))? {
        return Ok(());
    }
    host.signal(owner, Signal::Kill)?;
    for _ in 0..10 {
        if original_exited(owner, host.observe(owner.pid))? {
            return Ok(());
        }
        host.pause();
    }
    if original_exited(owner, host.observe(owner.pid))? {
        return Ok(());
    }
    Err(format!("Relay worker {} did not confirm exit after termination; outcome unknown", owner.pid))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner(identity: Option<&str>) -> WorkerOwner {
        WorkerOwner { pid: 424242, incarnation: identity.map(str::to_string) }
    }
    fn evidence(exited: bool, identity: Option<&str>) -> ProcessEvidence {
        ProcessEvidence { exited, incarnation: identity.map(str::to_string) }
    }
    fn write_owner(scope: &Path, owner: &WorkerOwner) {
        super::super::write_owner_in(scope, owner).unwrap();
        std::fs::write(scope.join(".tmp").join("relay.pid"), owner.pid.to_string()).unwrap();
    }

    struct ScriptedControl {
        state: ProcessEvidence,
        after_term: Option<ProcessEvidence>,
        after_kill: Option<ProcessEvidence>,
        replace_at_pause: Option<usize>,
        signals: Vec<Signal>,
        pauses: usize,
    }
    impl ScriptedControl {
        fn alive() -> Self {
            Self { state: evidence(false, Some("original")), after_term: None, after_kill: None, replace_at_pause: None, signals: Vec::new(), pauses: 0 }
        }
    }
    impl ProcessControl for ScriptedControl {
        fn observe(&mut self, _: u32) -> ProcessEvidence {
            self.state.clone()
        }
        fn signal(&mut self, _: &WorkerOwner, signal: Signal) -> Result<(), String> {
            self.signals.push(signal);
            if let Some(next) = match signal {
                Signal::Terminate => self.after_term.take(),
                Signal::Kill => self.after_kill.take(),
            } {
                self.state = next;
            }
            Ok(())
        }
        fn pause(&mut self) {
            self.pauses += 1;
            if self.replace_at_pause == Some(self.pauses) {
                self.state = evidence(false, Some("successor"));
            }
        }
    }

    #[test]
    fn unknown_legacy_or_empty_identity_never_grants_a_worker_signal() {
        for expected in [None, Some(""), Some("original")] {
            for observed in [None, Some("")] {
                let mut host = ScriptedControl::alive();
                host.state = evidence(false, observed);
                assert!(stop_owned_process(&owner(expected), &mut host).is_err());
                assert!(host.signals.is_empty());
            }
        }
        assert!(original_exited(&owner(None), evidence(false, Some("current"))).is_err());
        assert_eq!(original_exited(&owner(None), evidence(true, None)), Ok(true));
    }

    #[test]
    fn confirmed_exit_or_reuse_never_signals_the_current_occupant() {
        for state in [evidence(true, None), evidence(false, Some("successor"))] {
            let mut host = ScriptedControl::alive();
            host.state = state;
            stop_owned_process(&owner(Some("original")), &mut host).unwrap();
            assert!(host.signals.is_empty());
        }
        let mut host = ScriptedControl::alive();
        host.replace_at_pause = Some(50);
        stop_owned_process(&owner(Some("original")), &mut host).unwrap();
        assert_eq!(host.signals, vec![Signal::Terminate], "PID reuse before escalation must not reach KILL");
    }

    #[test]
    fn graceful_and_forced_stop_require_exit_while_unknown_never_reports_success() {
        let target = owner(Some("original"));
        let mut graceful = ScriptedControl::alive();
        graceful.after_term = Some(evidence(true, None));
        stop_owned_process(&target, &mut graceful).unwrap();
        assert_eq!(graceful.signals, vec![Signal::Terminate]);

        let mut forced = ScriptedControl::alive();
        forced.after_kill = Some(evidence(true, None));
        stop_owned_process(&target, &mut forced).unwrap();
        assert_eq!(forced.signals, vec![Signal::Terminate, Signal::Kill]);
        assert_eq!(forced.pauses, 50);

        let mut unknown = ScriptedControl::alive();
        unknown.after_term = Some(evidence(false, None));
        assert!(stop_owned_process(&target, &mut unknown).is_err());
        assert_eq!(unknown.signals, vec![Signal::Terminate]);

        let mut unconfirmed = ScriptedControl::alive();
        assert!(stop_owned_process(&target, &mut unconfirmed).is_err());
        assert_eq!(unconfirmed.signals, vec![Signal::Terminate, Signal::Kill]);
        assert_eq!(unconfirmed.pauses, 60);
    }

    #[test]
    fn changed_owner_blocks_signal_and_native_cleanup_and_preserves_successor_files() {
        let directory = tempfile::tempdir().unwrap();
        let target = owner(Some("original"));
        write_owner(directory.path(), &target);
        let captured = CapturedWorker::capture(directory.path()).unwrap();
        write_owner(directory.path(), &owner(Some("successor")));
        let mut host = HostControl { scope: directory.path(), captured: &captured };
        assert!(host.signal(&target, Signal::Kill).is_err(), "record mismatch rejects before any process syscall");
        let called = std::cell::Cell::new(false);
        assert!(
            finish_stop(&captured, directory.path(), || {
                called.set(true);
                Ok(())
            })
            .is_err()
        );
        assert!(!called.get());
        assert_eq!(CapturedWorker::capture(directory.path()).unwrap().owner.unwrap().incarnation.as_deref(), Some("successor"));
    }

    #[test]
    fn replaced_numeric_pidfile_blocks_signal_cleanup_and_preserves_both_records() {
        let directory = tempfile::tempdir().unwrap();
        let target = owner(Some("original"));
        write_owner(directory.path(), &target);
        let captured = CapturedWorker::capture(directory.path()).unwrap();
        std::fs::write(directory.path().join(".tmp").join("relay.pid"), "424243").unwrap();
        let current_pid = read_optional(&directory.path().join(".tmp").join("relay.pid")).unwrap();
        let current_owner = read_optional(&super::super::owner_file_path_in(directory.path())).unwrap();
        let mut host = HostControl { scope: directory.path(), captured: &captured };
        assert!(host.signal(&target, Signal::Kill).is_err(), "a changed numeric PID is rejected before process observation or signalling");
        let called = std::cell::Cell::new(false);
        assert!(
            finish_stop(&captured, directory.path(), || {
                called.set(true);
                Ok(())
            })
            .is_err()
        );
        assert!(!called.get());
        assert_eq!(read_optional(&directory.path().join(".tmp").join("relay.pid")).unwrap(), current_pid);
        assert_eq!(read_optional(&super::super::owner_file_path_in(directory.path())).unwrap(), current_owner);
    }

    #[test]
    fn unconfirmed_native_cleanup_retains_original_worker_evidence() {
        let directory = tempfile::tempdir().unwrap();
        write_owner(directory.path(), &owner(Some("original")));
        let captured = CapturedWorker::capture(directory.path()).unwrap();
        assert!(finish_stop(&captured, directory.path(), || Err("native cleanup outcome unknown".into())).is_err());
        assert_eq!(CapturedWorker::capture(directory.path()).unwrap(), captured);
        finish_stop(&captured, directory.path(), || Ok(())).unwrap();
        assert_eq!(CapturedWorker::capture(directory.path()).unwrap().owner, None);
    }

    fn fixture() -> (tempfile::TempDir, crate::daemon::test_support::TestDaemon, Client) {
        let directory = tempfile::Builder::new().prefix("agentx-relay-worker-stop-").tempdir().unwrap();
        let database = directory.path().join("agentx.db");
        let db = crate::db::AgentxDb::open_at(&database).unwrap();
        db.kv_set("relay_last_push", Some("7")).unwrap();
        drop(db);
        let daemon = crate::daemon::test_support::TestDaemon::start(directory.path(), &database);
        let client = Client::connect_existing(directory.path(), Default::default()).unwrap();
        (directory, daemon, client)
    }

    #[test]
    fn supplied_client_clears_only_its_scope_and_preserves_the_data_daemon() {
        let (directory, _daemon, client) = fixture();
        let other = tempfile::tempdir().unwrap();
        write_owner(other.path(), &owner(Some("unrelated-worker")));
        let other_record = CapturedWorker::capture(other.path()).unwrap();
        stop_relay_worker_client(&client).unwrap();
        assert_eq!(CapturedWorker::capture(other.path()).unwrap(), other_record);
        assert_eq!(client.scope_root(), directory.path().canonicalize().unwrap().as_path());
        let response = crate::relay::persistence::call(&client, crate::relay::service::Request::Status { configured: true, enabled: true, broker: None }).unwrap();
        let crate::relay::service::Response::Status(status) = response else {
            panic!("Status response");
        };
        assert_eq!(status.status, None);
        assert_eq!(status.heartbeat_age, None);
        assert_eq!(status.last_push, 7.0);
        assert!(matches!(status.health, crate::relay::RelayHealth::Waiting));
    }

    #[test]
    fn stop_refuses_its_data_daemon_and_transport_loss_never_reacquires_or_deletes() {
        let (directory, daemon, client) = fixture();
        write_owner(directory.path(), &WorkerOwner { pid: client.info().unwrap().pid, incarnation: crate::sys::process::identity(client.info().unwrap().pid) });
        let before = CapturedWorker::capture(directory.path()).unwrap();
        assert!(stop_relay_worker_client(&client).unwrap_err().contains("data daemon"));
        assert_eq!(CapturedWorker::capture(directory.path()).unwrap(), before);
        drop(daemon);
        let database = directory.path().join("agentx.db");
        std::fs::remove_file(&database).unwrap();
        assert!(stop_relay_worker_client(&client).is_err());
        assert!(!database.exists(), "stop must not reconnect or recreate managed storage after transport loss");
        assert_eq!(CapturedWorker::capture(directory.path()).unwrap(), before);
    }

    #[test]
    fn missing_owner_with_native_worker_evidence_is_unknown_and_never_cleared() {
        let (_directory, _daemon, client) = fixture();
        crate::relay::persistence::call(&client, crate::relay::service::Request::WorkerHeartbeat).unwrap();
        assert!(stop_relay_worker_client(&client).unwrap_err().contains("no recorded owner"));
        let response = crate::relay::persistence::call(&client, crate::relay::service::Request::Status { configured: true, enabled: true, broker: None }).unwrap();
        let crate::relay::service::Response::Status(status) = response else {
            panic!("Status response");
        };
        assert!(status.heartbeat_age.is_some(), "missing pidfile is not proof of worker exit");
    }

    #[test]
    fn stale_disconnected_status_without_owner_is_cleared_without_signal() {
        let (_directory, _daemon, client) = fixture();
        crate::relay::persistence::call(&client, crate::relay::service::Request::WorkerStopped).unwrap();
        stop_relay_worker_client(&client).unwrap();
        let response = crate::relay::persistence::call(&client, crate::relay::service::Request::Status { configured: true, enabled: true, broker: None }).unwrap();
        let crate::relay::service::Response::Status(status) = response else {
            panic!("Status response");
        };
        assert_eq!(status.status, None);
    }
}
