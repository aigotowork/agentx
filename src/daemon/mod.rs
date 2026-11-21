//! Local daemon scope ownership and bounded IPC. Application operations remain
//! in `crate::service`; see `.docs/architecture/daemon.md` for lifecycle rules.

#[cfg(test)]
mod acceptance_tests;
#[cfg(all(test, unix))]
mod compatibility_tests;
pub(crate) mod frame;
pub(crate) mod lifecycle;
pub(crate) mod observation;
#[cfg(test)]
mod observation_acceptance_tests;
pub(crate) mod platform;
mod scope;
mod server;
#[cfg(test)]
pub(crate) mod test_support;
mod transport;
#[cfg(windows)]
mod windows;
mod wire;

pub use scope::{DaemonScope, DatabaseGeneration, ScopeLease};
#[cfg(test)]
pub(crate) use transport::shutdown;
pub(crate) use transport::{acquire, call, call_codex_hook, probe};
pub(crate) use wire::{DaemonInfo, PROTOCOL_VERSION, build_identity};

use serde::{Deserialize, Serialize};

/// sha2 0.11 returns a byte array without a LowerHex implementation.
fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Transport/lifecycle failures never authorize direct persistence access or
/// imply that an operation whose reply was lost did not commit.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum DaemonError {
    #[error("daemon unavailable: {0}")]
    Unavailable(String),
    #[error("daemon I/O failed: {message}")]
    Io { message: String, os_code: Option<i32> },
    #[error("another daemon owns this data scope")]
    AlreadyRunning,
    #[error("daemon is incompatible: {0}")]
    Incompatible(String),
    #[error("invalid daemon protocol: {0}")]
    Protocol(String),
    #[error("daemon is busy")]
    Busy,
    #[error("daemon request deadline expired")]
    Deadline,
    #[error("database generation changed; explicitly reconnect before issuing new work")]
    StaleGeneration,
    #[error("request outcome is unknown: {0}")]
    OutcomeUnknown(String),
    #[error("daemon transport is unsupported: {0}")]
    Unsupported(String),
}

impl From<std::io::Error> for DaemonError {
    fn from(error: std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::TimedOut { Self::Deadline } else { Self::Io { message: error.to_string(), os_code: error.raw_os_error() } }
    }
}

/// Enter the one-binary daemon mode before project environment/identity setup.
/// Parsing a non-daemon invocation has no scope or endpoint side effects.
pub(crate) fn entry_scope(argv: &[String]) -> Result<Option<DaemonScope>, DaemonError> {
    if argv.first().map(String::as_str) != Some("daemon") {
        return Ok(None);
    }
    if argv.len() != 4 || argv[1] != "serve" || argv[2] != "--scope" {
        return Err(DaemonError::Protocol("usage: agentx daemon serve --scope PATH".into()));
    }
    Ok(Some(DaemonScope::prepare(&argv[3])?))
}

/// Acquire the singleton before invoking the sole persistence constructor.
/// The application publishes its committed database generation before return;
/// its database is dropped before the ownership guard on every return path.
pub(crate) fn serve<A: crate::service::Application>(scope: DaemonScope, initialize: impl FnOnce(DatabaseGeneration) -> crate::service::ServiceResult<A>) -> Result<(), DaemonError> {
    serve_with_stop(scope, std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)), initialize)
}

/// Shared lifecycle entry so test fixtures can request cleanup even when a
/// readiness handshake fails. Production stop sources are signals and IPC.
fn serve_with_stop<A: crate::service::Application>(scope: DaemonScope, stop: std::sync::Arc<std::sync::atomic::AtomicBool>, initialize: impl FnOnce(DatabaseGeneration) -> crate::service::ServiceResult<A>) -> Result<(), DaemonError> {
    serve_with_stop_and_management(scope, stop, true, initialize)
}

/// Scheduling fixtures can omit management's autonomous business requests while
/// retaining the same application, ordinary listener and observation lifecycle.
fn serve_with_stop_and_management<A: crate::service::Application>(
    scope: DaemonScope,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    management: bool,
    initialize: impl FnOnce(DatabaseGeneration) -> crate::service::ServiceResult<A>,
) -> Result<(), DaemonError> {
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (scope, stop, management, initialize);
        Err(DaemonError::Unsupported("this build has no authenticated Windows local IPC binding".into()))
    }
    #[cfg(any(unix, windows))]
    {
        let lease = ScopeLease::acquire(scope)?;
        if stop.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }
        let observation = observation::ObservationState::new(&lease).ok();
        std::thread::scope(|threads| {
            struct StopObservation(std::sync::Arc<std::sync::atomic::AtomicBool>);
            impl Drop for StopObservation {
                fn drop(&mut self) {
                    self.0.store(true, std::sync::atomic::Ordering::Release);
                }
            }
            let observer_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let _observer_lifetime = StopObservation(observer_stop.clone());
            if let Some(state) = &observation {
                let (started, ready) = std::sync::mpsc::sync_channel(1);
                let state = state.clone();
                let observer_stop = observer_stop.clone();
                let lease = &lease;
                if std::thread::Builder::new().name("agentx-observation".into()).spawn_scoped(threads, move || observation::run(lease, state, observer_stop, started)).is_ok() {
                    // Startup only: no business transaction or dispatcher wait.
                    // Failure of observation does not change initialization.
                    let _ = ready.recv_timeout(observation::INSPECTION_TIMEOUT);
                }
            }
            let mut application = match initialize(lease.database_generation()) {
                Ok(application) => application,
                Err(error) => {
                    if let Some(state) = &observation {
                        state.lifecycle(crate::observability::runtime::DaemonLifecycle::Failed);
                    }
                    return Err(DaemonError::Unavailable(format!("application initialization failed: {error}")));
                }
            };
            if stop.load(std::sync::atomic::Ordering::Acquire) {
                return Ok(());
            }
            if lease.database_generation().current().is_empty() {
                if let Some(state) = &observation {
                    state.lifecycle(crate::observability::runtime::DaemonLifecycle::Failed);
                }
                return Err(DaemonError::Unavailable("application did not publish its durable database generation".into()));
            }
            if let Some(state) = &observation {
                state.synchronize_epoch();
            }
            // HTTP owns no SQLite handles. Native dispatch selects the read or
            // write lane. Stop and join HTTP before releasing the lease.
            let _management_lifetime = StopObservation(stop.clone());
            if management {
                let management_scope = lease.scope().root().to_path_buf();
                let management_instance = lease.instance().to_owned();
                let management_stop = stop.clone();
                threads.spawn(move || crate::management::web::run(management_scope, management_instance, management_stop));
            }
            let result = server::run_observed(&lease, &mut application, stop, observation.clone());
            drop(application);
            if let Some(state) = &observation {
                state.lifecycle(crate::observability::runtime::DaemonLifecycle::Stopping);
            }
            result
            // Application drops first; the guard stops observer admission;
            // thread::scope joins its <=2 workers before ScopeLease can drop.
        })
    }
}
