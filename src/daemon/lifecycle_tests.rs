//! Authored lifecycle cases. Independent startup acceptance owns execution.
use super::*;

#[test]
fn passive_missing_scope_reports_stopped_without_creating_anything() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing").join("scope");
    let result = status(&path, directory.path()).unwrap();
    assert_eq!(result.state, State::Stopped);
    assert_eq!(result.scope, path);
    assert!(!directory.path().join("missing").exists());
    assert!(result.info.is_none());
    assert!(result.peer.is_none());
}

#[test]
fn passive_empty_scope_has_no_database_or_ownership_side_effects() {
    let directory = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let result = status(directory.path(), directory.path()).unwrap();
    assert_eq!(result.state, State::Stopped);
    assert!(!directory.path().join(".daemon").exists());
    assert!(!directory.path().join("agentx.db").exists());
}

#[cfg(unix)]
#[test]
fn passive_insecure_scope_is_refused_without_permission_repair() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(status(directory.path(), directory.path()).is_err());
    assert_eq!(std::fs::metadata(directory.path()).unwrap().permissions().mode() & 0o777, 0o755);
    assert!(!directory.path().join(".daemon").exists());
}

#[test]
fn held_owner_without_endpoint_is_unavailable_and_restart_refuses() {
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let lease = super::super::ScopeLease::acquire(scope.clone()).unwrap();
    assert_eq!(status(scope.root(), scope.root()).unwrap().state, State::Unavailable);
    let error = stop_existing(&scope, Instant::now() + HELLO_TIMEOUT).unwrap_err();
    assert!(error.to_string().contains("restart refused"));
    assert!(!owner_released(&scope).unwrap());
    lease.ensure_ownership().unwrap();
    assert!(!scope.root().join("agentx.db").exists());
}

#[test]
fn stop_deadline_retains_singleton_and_never_deletes_data() {
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let lease = super::super::ScopeLease::acquire(scope.clone()).unwrap();
    let marker = scope.root().join("retained-data");
    std::fs::write(&marker, b"keep").unwrap();
    assert!(wait_for_release(&scope, Instant::now()).unwrap_err().to_string().contains("timed out"));
    lease.ensure_ownership().unwrap();
    assert_eq!(std::fs::read(marker).unwrap(), b"keep");
    assert!(!owner_released(&scope).unwrap());
}

#[test]
fn startup_lock_serializes_recovery_and_cold_acquisition_without_data_access() {
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let first = transport::startup_lock(&scope, Instant::now() + HELLO_TIMEOUT).unwrap();
    assert!(matches!(transport::startup_lock(&scope, Instant::now() + Duration::from_millis(20)), Err(DaemonError::Deadline)));
    assert!(scope.state().join("startup.lock").is_file());
    assert!(!scope.state().join("owner.lock").exists());
    assert!(!scope.root().join("agentx.db").exists());
    drop(first);
    assert!(transport::startup_lock(&scope, Instant::now() + HELLO_TIMEOUT).is_ok());
}

#[cfg(target_os = "linux")]
#[test]
fn legacy_process_evidence_requires_exact_scope_argv_and_loaded_image() {
    let scope = Path::new("/selected");
    assert!(legacy_arguments(&[b"/old/agentx", b"daemon", b"serve", b"--scope", b"/selected"], scope));
    for arguments in [
        vec![b"/old/agentx".as_slice(), b"daemon", b"serve", b"--scope", b"/other"],
        vec![b"/old/agentx".as_slice(), b"relay-worker", b"serve", b"--scope", b"/selected"],
        vec![b"/old/agentx".as_slice(), b"daemon", b"serve", b"--scope", b"/selected", b"extra"],
        vec![b"/old/agentx".as_slice(), b"daemon", b"", b"serve", b"--scope", b"/selected"],
    ] {
        assert!(!legacy_arguments(&arguments, scope));
    }
    assert!(loaded_image_matches(b"/old/agentx", Path::new("/old/agentx (deleted)")));
    assert!(!loaded_image_matches(b"/old/agentx", Path::new("/unrelated/python")));
}

#[cfg(target_os = "linux")]
#[test]
fn legacy_lock_evidence_rejects_wrong_owner_inode_shared_and_waiting_locks() {
    let device = libc::makedev(8, 2);
    let valid = "13: FLOCK ADVISORY WRITE 123 08:02:987 0 EOF\n";
    assert!(owns_lock(valid, 123, device, 987));
    assert!(!owns_lock(valid, 124, device, 987));
    assert!(!owns_lock(valid, 123, device, 988));
    assert!(!owns_lock("13: FLOCK ADVISORY READ 123 08:02:987 0 EOF", 123, device, 987));
    assert!(!owns_lock("13: -> FLOCK ADVISORY WRITE 123 08:02:987 0 EOF", 123, device, 987));
    assert!(!owns_lock("13: POSIX ADVISORY WRITE 123 08:02:987 0 EOF", 123, device, 987));
}

#[cfg(unix)]
#[test]
fn incompatible_status_is_actionable_and_restart_does_not_signal_unverified_peer() {
    use super::super::{
        frame,
        wire::{ClientFrame, ServerFrame},
    };
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let lease = super::super::ScopeLease::acquire(scope.clone()).unwrap();
    let listener = platform::Listener::bind(&lease).unwrap();
    std::thread::scope(|threads| {
        let server = &listener;
        threads.spawn(move || {
            for _ in 0..2 {
                let deadline = Instant::now() + HELLO_TIMEOUT;
                assert!(server.wait_ready(deadline).unwrap());
                let mut stream = server.accept().unwrap().unwrap();
                assert!(matches!(frame::read::<ClientFrame>(&mut stream, deadline).unwrap(), ClientFrame::Hello { .. }));
                frame::write(&mut stream, &ServerFrame::Failure(DaemonError::Incompatible("old daemon rejects Hello".into())), deadline).unwrap();
            }
        });
        let result = status(scope.root(), scope.root()).unwrap();
        assert_eq!(result.state, State::Incompatible);
        assert_eq!(result.peer.unwrap().0, std::process::id());
        let detail = result.detail.unwrap();
        assert!(detail.contains("daemon restart --scope"));
        assert!(detail.contains("daemon version/cohort unavailable"));
        assert!(detail.contains(&scope.root().display().to_string()));
        assert!(stop_existing(&scope, Instant::now() + HELLO_TIMEOUT).is_err());
        lease.ensure_ownership().unwrap();
        assert!(!scope.root().join("agentx.db").exists());
    });
    drop(listener);
}
