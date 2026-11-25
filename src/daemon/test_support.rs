//! Same-process native IPC fixture for existing Provider adapter regressions.
//! Process exclusivity evidence belongs to `acceptance_tests`, not this helper.

use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(crate) struct TestDaemon {
    scope: super::DaemonScope,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), super::DaemonError>>>,
}

impl TestDaemon {
    pub(crate) fn start(scope_path: &Path, database_path: &Path) -> Self {
        assert!(crate::paths::is_test_temp_path(scope_path), "fixture scope must be disposable");
        let scope = super::DaemonScope::prepare(scope_path).unwrap();
        let daemon_scope = scope.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let daemon_stop = Arc::clone(&stop);
        let database_path = database_path.to_path_buf();
        let thread = std::thread::spawn(move || {
            super::serve_with_stop(daemon_scope, daemon_stop, |generation| {
                let db = crate::db::AgentxDb::open_at(&database_path).map_err(|error| crate::service::ServiceError::Internal(error.to_string()))?;
                crate::service::application::SqliteApplication::new(db, generation)
            })
        });
        let fixture = Self { scope, stop, thread: Some(thread) };
        let deadline = Instant::now() + Duration::from_secs(5);
        while super::probe(&fixture.scope).is_err() {
            assert!(!fixture.thread.as_ref().unwrap().is_finished(), "fixture daemon exited before readiness");
            assert!(Instant::now() < deadline, "fixture daemon readiness deadline");
            std::thread::sleep(Duration::from_millis(10));
        }
        fixture
    }
}

impl Drop for TestDaemon {
    fn drop(&mut self) {
        let _ = super::shutdown(&self.scope);
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let deadline = Instant::now() + Duration::from_secs(35);
            while !thread.is_finished() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            if !thread.is_finished() {
                if !std::thread::panicking() {
                    panic!("fixture daemon did not stop within worker deadline");
                }
                eprintln!("fixture daemon cleanup exceeded worker deadline");
                return;
            }
            let result = thread.join();
            if !std::thread::panicking() {
                result.unwrap().unwrap();
            }
        }
    }
}

#[test]
fn retained_client_daemon_disappearance_fails_without_reopening_managed_storage() {
    use crate::service::{InvocationContext, Request, ServiceError};
    let directory = tempfile::Builder::new().prefix("agentx-client-no-fallback-").tempdir().unwrap();
    let database = directory.path().join("agentx.db");
    let daemon = TestDaemon::start(directory.path(), &database);
    let client = crate::service::client::Client::connect(directory.path(), InvocationContext::default()).unwrap();
    client.call(Request::Events(crate::events::service::Request::Watermark)).unwrap();
    drop(daemon);
    // Removing only this closed fixture DB makes an illicit fallback observable:
    // even an empty replacement database would violate the client's contract.
    std::fs::remove_file(&database).unwrap();
    assert!(matches!(client.call(Request::Events(crate::events::service::Request::Watermark)), Err(ServiceError::Unavailable(_))));
    assert!(!database.exists(), "client recreated managed storage after IPC loss");
    assert!(matches!(client.call(Request::Participants(crate::agents::service::Request::List)), Err(ServiceError::Unavailable(_))));
    assert!(!database.exists());
    assert!(!directory.path().join(".daemon/startup.lock").exists(), "a retained caller attempted startup after losing its daemon");
}

#[cfg(unix)]
#[test]
fn missing_socket_paths_recover_under_the_original_owner() {
    let directory = tempfile::Builder::new().prefix("agentx-unlinked-").tempdir().unwrap();
    let daemon = TestDaemon::start(directory.path(), &directory.path().join("agentx.db"));
    let original = super::probe(&daemon.scope).unwrap();
    std::fs::remove_file(daemon.scope.observation_socket_path(false).unwrap()).unwrap();
    std::fs::remove_file(daemon.scope.existing_socket_path().unwrap()).unwrap();
    let current = super::transport::acquire(&daemon.scope).unwrap();
    assert_eq!(current.daemon_instance, original.daemon_instance);
    assert_eq!(current.database_generation, original.database_generation);
    let deadline = Instant::now() + Duration::from_secs(3);
    while super::platform::connect_observation(&daemon.scope, deadline).is_err() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    std::fs::remove_file(daemon.scope.existing_socket_path().unwrap()).unwrap();
    assert!(!super::lifecycle::stop_existing(&daemon.scope, deadline).unwrap());
}
