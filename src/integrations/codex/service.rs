//! Native shared-daemon acquisition and one client's connection capability.
//! Concrete discovery/proxy behavior is owned by the native-service assignment.
//! See `.docs/codex-activation/topology.md` for compatibility and lifecycle meaning.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::agents::attachments::ExecutionId;

#[cfg(target_os = "linux")]
#[path = "service/managed.rs"]
mod managed;
#[cfg(target_os = "linux")]
#[path = "service/transport.rs"]
mod transport;

#[cfg(target_os = "linux")]
pub(super) use transport::UnixConnection;

/// Attach to an explicitly owned private app-server socket without acquiring the shared daemon.
#[cfg(target_os = "linux")]
pub(crate) fn connect_private_socket(path: &std::path::Path) -> Result<Box<dyn NativeConnection>, ServiceFailure> {
    Ok(Box::new(UnixConnection::connect(path)?))
}

pub(super) const SHARED_NATIVE_VERSIONS: &[&str] = &["0.167.0", "0.170.0"];

pub(super) fn supports_shared_native(version: &str) -> bool {
    SHARED_NATIVE_VERSIONS.contains(&version.strip_prefix("codex-cli ").unwrap_or(version))
}

#[cfg(all(test, target_os = "linux"))]
#[path = "service/acceptance_tests.rs"]
mod acceptance_tests;

/// Native daemon acquisition; this value owns no backend process lifetime.
#[derive(Default)]
pub(crate) struct ManagedCodexService;

/// Names whose ordinary native tool/session provenance is normalized during
/// acquisition. A configured dependency on one must retain its original input
/// or be rejected explicitly; this predicate never grants reuse by itself.
pub(crate) fn environment_input_is_normalized(key: &str) -> bool {
    matches!(key, "PATH" | "LC_CTYPE") || client_variable(key)
}

fn client_variable(key: &str) -> bool {
    (key.starts_with("AGENTX_") && key != "AGENTX_DIR" && key != "AGENTX_DEV_ROOT")
        || key.starts_with("PASEO_")
        || key.starts_with("DIRENV_")
        || matches!(
            key,
            "CODEX_SESSION_ID"
                | "CODEX_THREAD_ID"
                | "CODEX_TOOL_CALL_ID"
                | "CODEX_VERSION"
                | "CODEX_CI"
                | "CODEX_APPLY_PATCH_PRESERVE_LINE_ENDINGS"
                | "CODEX_INTERNAL_ORIGINATOR_OVERRIDE"
                | "PWD"
                | "OLDPWD"
                | "SHLVL"
                | "_"
                | "LOGNAME"
                | "TERM"
                | "COLORTERM"
                | "NO_COLOR"
                | "PAGER"
                | "GIT_PAGER"
                | "GH_PAGER"
                | "TERM_SESSION_ID"
                | "WINDOWID"
                | "WEZTERM_PANE"
                | "TMUX"
                | "TMUX_PANE"
        )
}

#[cfg(not(target_os = "linux"))]
impl CodexService for ManagedCodexService {
    fn connect(&mut self, _request: &ServiceRequest) -> Result<ServiceConnection, ServiceFailure> {
        Err(ServiceFailure::UnsupportedPlatform)
    }
}

/// Explicit launch inputs after the existing project-environment preparation.
/// `args` is the native service argv including app-server, after the composition
/// boundary has translated per-thread configuration into protocol overlays.
/// Acquisition must reject incompatible service requirements, not start a
/// second private backend. Per-client identity variables are not backend identity.
#[derive(Clone, Debug)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code, reason = "Only the Linux backend consumes these shared acquisition inputs"))]
pub(crate) struct ServiceRequest {
    pub program: PathBuf,
    pub prefix_args: Vec<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub canonical_home: PathBuf,
    pub agentx_dir: PathBuf,
    pub required_version: Option<&'static str>,
}

/// Observed live backend identity, stable across client proxies and replaced
/// after backend restart. An OS PID alone is not a generation token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceIdentity {
    pub canonical_home: PathBuf,
    pub execution: ExecutionId,
    pub native_version: String,
    pub control_socket: PathBuf,
    pub compatibility: ServiceCompatibility,
}

/// Service owner computes these from actual backend-level requirements.
/// Fingerprints contain no credentials. Different thread cwd alone is not a
/// reason to select a different service; hooks and AgentX storage must agree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceCompatibility {
    pub native_program: PathBuf,
    pub environment_fingerprint: String,
    pub configuration_fingerprint: String,
    pub agentx_dir: PathBuf,
}

/// A client owns only this transport. Closing or dropping it must never stop,
/// restart, signal or otherwise take ownership of the shared daemon.
pub(crate) struct ServiceConnection {
    pub identity: ServiceIdentity,
    pub transport: Box<dyn NativeConnection>,
}

/// Complete native JSON frames, independent of newline/socket framing.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code, reason = "The shared bridge handles frames from the Linux backend"))]
pub(crate) enum Receive {
    Frame(String),
    TimedOut,
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code, reason = "Only the Linux transport produces every connection failure"))]
pub(crate) enum ConnectionFailure {
    #[error("native connection closed")]
    Closed,
    #[error("native connection I/O: {0}")]
    Io(String),
    #[error("invalid native protocol frame: {0}")]
    InvalidFrame(String),
}

/// One connection's sole native reader/writer. `receive` returns within the
/// supplied timeout so user control, cancellation and queue checks can progress.
/// Send failure can follow a partial write; it never proves an input absent.
/// Adapters supply a bounded lossless queue and keep diagnostics off stdout.
pub(crate) trait NativeConnection: Send {
    fn send(&mut self, frame: &str) -> Result<(), ConnectionFailure>;
    fn receive(&mut self, timeout: Duration) -> Result<Receive, ConnectionFailure>;
    fn close(&mut self) -> Result<(), ConnectionFailure>;
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code, reason = "Other platforms return UnsupportedPlatform before acquisition"))]
pub(crate) enum ServiceFailure {
    #[cfg(not(target_os = "linux"))]
    #[error("shared Codex service is unsupported on this platform")]
    UnsupportedPlatform,
    #[error("unsupported shared Codex invocation: {0}")]
    UnsupportedInvocation(String),
    #[error("existing Codex service is incompatible: {0}")]
    Incompatible(String),
    #[error("shared Codex service unavailable: {0}")]
    Unavailable(String),
    #[error("shared Codex service protocol: {0}")]
    Protocol(String),
}

/// Reuses or first-starts the selected compatible native backend and acquires
/// one independent client connection. Concurrent acquisition must converge.
/// No method grants daemon termination authority to the caller.
pub(crate) trait CodexService {
    fn connect(&mut self, request: &ServiceRequest) -> Result<ServiceConnection, ServiceFailure>;
}
