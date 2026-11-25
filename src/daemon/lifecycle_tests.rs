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
    #[cfg(windows)]
    super::super::windows::private_directory(directory.path()).unwrap();
    #[cfg(not(windows))]
    crate::sys::fs::set_private_dir(directory.path()).unwrap();
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
    let error = stop_existing(&scope, Instant::now() + STATUS_TIMEOUT).unwrap_err();
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
    let first = transport::startup_lock(&scope, Instant::now() + STATUS_TIMEOUT).unwrap();
    assert!(matches!(transport::startup_lock(&scope, Instant::now() + Duration::from_millis(20)), Err(DaemonError::Deadline)));
    assert!(scope.state().join("startup.lock").is_file());
    assert!(!scope.state().join("owner.lock").exists());
    assert!(!scope.root().join("agentx.db").exists());
    drop(first);
    assert!(transport::startup_lock(&scope, Instant::now() + STATUS_TIMEOUT).is_ok());
}

#[cfg(unix)]
#[test]
fn legacy_shutdown_uses_verified_ready_connection_and_exact_instance() {
    use super::super::{
        frame,
        legacy::{LegacyCall, LegacyReply},
        wire::{ClientFrame, ServerFrame},
    };
    for selected in [3, 4] {
        for wrong_scope in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let scope = DaemonScope::prepare(directory.path()).unwrap();
            let lease = super::super::ScopeLease::acquire(scope.clone()).unwrap();
            let listener = platform::Listener::bind(&lease).unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            std::thread::scope(|threads| {
                threads.spawn(|| {
                    let accept = || {
                        assert!(listener.wait_ready(deadline).unwrap());
                        listener.accept().unwrap().unwrap()
                    };
                    let mut direct = accept();
                    assert!(matches!(frame::read::<ClientFrame<String>>(&mut direct, deadline).unwrap(), ClientFrame::Call { request, .. } if request == "DaemonStatus"));
                    frame::write(&mut direct, &ServerFrame::Failure(DaemonError::Protocol("missing field `daemon_instance`".into())), deadline).unwrap();
                    drop(direct);
                    for expected_epoch in (selected..=4).rev() {
                        let mut stream = accept();
                        assert!(matches!(frame::read::<LegacyCall>(&mut stream, deadline).unwrap(), LegacyCall::Hello { protocol_version, canonical_scope, .. } if protocol_version == expected_epoch && canonical_scope == scope.root()));
                        if expected_epoch != selected {
                            frame::write(&mut stream, &LegacyReply::Failure(DaemonError::Incompatible("epoch differs".into())), deadline).unwrap();
                            continue;
                        }
                        let instance = "verified-old-instance";
                        let root = if wrong_scope { directory.path().join("neighbor") } else { scope.root().into() };
                        let info = DaemonInfo {
                            protocol_version: selected,
                            build_version: "old".into(),
                            build_id: "old".into(),
                            canonical_scope: root,
                            daemon_instance: instance.into(),
                            database_generation: "generation".into(),
                            pid: std::process::id(),
                            codex_hook: false,
                            agent_names: false,
                            workspace_members: false,
                            message_history: false,
                        };
                        frame::write(&mut stream, &LegacyReply::Ready(info), deadline).unwrap();
                        if wrong_scope {
                            assert!(frame::read_optional::<LegacyCall>(&mut stream, deadline).unwrap().is_none());
                        } else {
                            assert!(matches!(frame::read::<LegacyCall>(&mut stream, deadline).unwrap(), LegacyCall::Shutdown { daemon_instance } if daemon_instance == instance));
                            frame::write(&mut stream, &LegacyReply::Stopping, deadline).unwrap();
                        }
                    }
                });
                let stopped = stop_existing(&scope, deadline);
                if wrong_scope {
                    assert!(matches!(stopped, Err(DaemonError::Incompatible(_))));
                } else {
                    assert!(stopped.unwrap());
                }
            });
            lease.ensure_ownership().unwrap();
            assert!(!scope.root().join("agentx.db").exists());
        }
    }
}
