//! Connection acquisition and one-shot native calls. Only discovery is retried;
//! once a call frame may have been written its outcome requires reconciliation.

use std::fs::{File, OpenOptions};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::wire::{BUILD_VERSION, ClientFrame, DaemonInfo, PROTOCOL_VERSION, ServerFrame};
use super::{DaemonError, DaemonScope, frame, platform};
use crate::observability::runtime::{ClientStage, ObservedProcess};
use crate::service::observation;
use crate::service::{CallerContext, Request, Response, ServiceResult};

pub(super) const IO_TIMEOUT: Duration = Duration::from_secs(30);
const START_TIMEOUT: Duration = Duration::from_secs(15);
const HELLO_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn probe(scope: &DaemonScope) -> Result<DaemonInfo, DaemonError> {
    handshake(scope, Instant::now() + HELLO_TIMEOUT).map(|(_, info)| info)
}

/// Serialize client auto-start independently of the daemon's lifetime lock.
/// A held startup lock never permits opening the managed database.
pub(crate) fn acquire(scope: &DaemonScope) -> Result<DaemonInfo, DaemonError> {
    match probe(scope) {
        Ok(info) => return Ok(info),
        Err(error @ (DaemonError::Busy | DaemonError::Incompatible(_) | DaemonError::Protocol(_) | DaemonError::Unsupported(_))) => return Err(error),
        Err(_) => {}
    }
    let deadline = Instant::now() + START_TIMEOUT;
    let _startup = startup_lock(scope, deadline)?;
    acquire_locked(scope, deadline)
}

/// Shared by explicit restart and cold acquisition; the returned file retains
/// the startup lock across stop and spawn without touching the lifetime lock.
pub(super) fn startup_lock(scope: &DaemonScope, deadline: Instant) -> Result<File, DaemonError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE};
        options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let startup = observation::stage(ClientStage::StartupLock, || options.open(scope.state().join("startup.lock")).map_err(DaemonError::from))?;
    loop {
        if Instant::now() >= deadline {
            return Err(DaemonError::Deadline);
        }
        let acquired = observation::stage(ClientStage::StartupLock, || match startup.try_lock() {
            Ok(()) => Ok(true),
            Err(std::fs::TryLockError::WouldBlock) => Ok(false),
            Err(std::fs::TryLockError::Error(error)) => Err(error.into()),
        })?;
        if acquired {
            return Ok(startup);
        } else {
            observation::stage(ClientStage::Discovery, || pause(deadline))?;
        }
    }
}

pub(super) fn acquire_locked(scope: &DaemonScope, deadline: Instant) -> Result<DaemonInfo, DaemonError> {
    match handshake(scope, deadline.min(Instant::now() + HELLO_TIMEOUT)) {
        Ok((_, info)) => return Ok(info),
        Err(error @ (DaemonError::Busy | DaemonError::Incompatible(_) | DaemonError::Protocol(_) | DaemonError::Unsupported(_))) => return Err(error),
        Err(_) => {}
    }
    // The singleton lease is checked by the child before application creation;
    // an already initializing explicit daemon cannot become a second DB owner.
    if Instant::now() >= deadline {
        return Err(DaemonError::Deadline);
    }
    let mut child = observation::stage(ClientStage::Spawn, || spawn_daemon(scope))?;
    let mut exit_status = None;
    loop {
        match handshake(scope, deadline.min(Instant::now() + HELLO_TIMEOUT)) {
            Ok((_, info)) => return Ok(info),
            Err(error @ (DaemonError::Busy | DaemonError::Incompatible(_) | DaemonError::Protocol(_) | DaemonError::Unsupported(_))) => return Err(error),
            Err(_) => {}
        }
        if exit_status.is_none() {
            exit_status = observation::stage(ClientStage::Discovery, || child.try_wait().map_err(DaemonError::from))?;
        }
        if let Err(error) = observation::stage(ClientStage::Discovery, || pause(deadline)) {
            return Err(DaemonError::Unavailable(format!("readiness failed ({error}); child exit: {exit_status:?}; see {}", scope.state().join("stderr.log").display())));
        }
    }
}

fn spawn_daemon(scope: &DaemonScope) -> Result<std::process::Child, DaemonError> {
    let mut logs = OpenOptions::new();
    logs.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        logs.mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let stderr = logs.open(scope.state().join("stderr.log"))?;
    let mut command = Command::new(std::env::current_exe()?);
    command.args(["daemon", "serve", "--scope"]).arg(scope.root()).env("AGENTX_DIR", scope.root()).current_dir(scope.root()).stdin(Stdio::null()).stdout(Stdio::null()).stderr(stderr);
    crate::sys::process::spawn_detached(&mut command).map_err(DaemonError::from)
}

pub(crate) fn call(scope: &DaemonScope, caller: CallerContext, request: Request) -> Result<ServiceResult<Response>, DaemonError> {
    let deadline = observation::stage(ClientStage::Capture, || request_deadline(caller.deadline_unix_ms))?;
    let (mut stream, info) = handshake(scope, deadline)?;
    if caller.expected_database_generation.as_deref() != Some(info.database_generation.as_str()) {
        return observation::stage(ClientStage::Hello, || Err(DaemonError::StaleGeneration));
    }
    if matches!(request, Request::Queries(crate::service::queries::Request::AgentNames(_))) && !info.agent_names {
        return Err(DaemonError::Unsupported("daemon does not support the Agent names query".into()));
    }
    if matches!(request, Request::Management(crate::management::contract::Request::ListWorkspaceMembers { .. })) && !info.workspace_members {
        return Err(DaemonError::Unsupported("daemon does not support workspace member pages".into()));
    }
    if matches!(request, Request::Management(crate::management::contract::Request::MessageHistory { .. })) && !info.message_history {
        return Err(DaemonError::Unsupported("daemon does not support exact message history pages".into()));
    }
    exchange(&mut stream, &info, caller, request, deadline)
}

/// An already-launched hook only talks to an existing daemon. Authenticate
/// and discover capability on the same connection used for the first write.
pub(crate) fn call_codex_hook(scope: &DaemonScope, mut caller: CallerContext, event: crate::service::codex_hook::HookEvent) -> Result<Option<(DaemonInfo, ServiceResult<Response>)>, DaemonError> {
    let deadline = observation::stage(ClientStage::Capture, || request_deadline(caller.deadline_unix_ms))?;
    let mut stream = observation::stage(ClientStage::Connect, || platform::connect_existing(scope, deadline))?;
    let info = hello(scope, &mut stream, deadline)?;
    if caller.expected_database_generation.as_ref().is_some_and(|expected| expected != &info.database_generation) {
        return observation::stage(ClientStage::Hello, || Err(DaemonError::StaleGeneration));
    }
    if !info.codex_hook {
        return Ok(None);
    }
    caller.expected_database_generation = Some(info.database_generation.clone());
    let result = exchange(&mut stream, &info, caller, Request::CodexHook(event), deadline)?;
    Ok(Some((info, result)))
}

fn exchange(stream: &mut platform::Stream, info: &DaemonInfo, caller: CallerContext, request: Request, deadline: Instant) -> Result<ServiceResult<Response>, DaemonError> {
    let operation = ClientFrame::Call { daemon_instance: info.daemon_instance.clone(), caller, request };
    // Even a partial write can have delivered the complete request before the
    // local error was observed. Conservatively never retry this operation.
    observation::may_write();
    observation::stage(ClientStage::CallWrite, || frame::write(stream, &operation, deadline)).map_err(unknown)?;
    match observation::stage(ClientStage::ReplyRead, || frame::read(stream, deadline)).map_err(unknown)? {
        ServerFrame::Result { result, .. } => {
            observation::application_result(&result);
            Ok(result)
        }
        ServerFrame::Failure(error) => {
            observation::received_failure(&error);
            Err(error)
        }
        _ => Err(DaemonError::OutcomeUnknown("unexpected reply after call".into())),
    }
}

/// Authenticated cleanup for in-process fixtures.
#[cfg(test)]
pub(crate) fn shutdown(scope: &DaemonScope) -> Result<(), DaemonError> {
    let deadline = Instant::now() + IO_TIMEOUT;
    let (mut stream, info) = handshake(scope, deadline)?;
    shutdown_connection(&mut stream, &info, deadline)
}

pub(super) fn shutdown_connection(stream: &mut platform::Stream, info: &DaemonInfo, deadline: Instant) -> Result<(), DaemonError> {
    frame::write(stream, &ClientFrame::Shutdown { daemon_instance: info.daemon_instance.clone() }, deadline).map_err(unknown)?;
    match frame::read(stream, deadline).map_err(unknown)? {
        ServerFrame::Stopping => Ok(()),
        ServerFrame::Failure(error) => Err(error),
        _ => Err(DaemonError::Protocol("expected shutdown acknowledgement".into())),
    }
}

pub(super) fn handshake(scope: &DaemonScope, deadline: Instant) -> Result<(platform::Stream, DaemonInfo), DaemonError> {
    let mut stream = observation::stage(ClientStage::Connect, || platform::connect(scope, deadline))?;
    let info = hello(scope, &mut stream, deadline)?;
    Ok((stream, info))
}

/// Lifecycle diagnosis retains this same authenticated connection even when
/// Hello rejects compatibility. No ordinary operation bypasses this fence.
pub(super) fn hello(scope: &DaemonScope, stream: &mut platform::Stream, deadline: Instant) -> Result<DaemonInfo, DaemonError> {
    let build_id = super::wire::build_identity();
    let (pid, incarnation) = observation::stage(ClientStage::PeerAuthentication, || platform::peer(stream))?;
    let sent = observation::stage(ClientStage::Hello, || {
        frame::write(stream, &ClientFrame::Hello { protocol_version: PROTOCOL_VERSION, build_version: BUILD_VERSION.into(), build_id: build_id.into(), canonical_scope: scope.root().to_path_buf() }, deadline)
    });
    if let Err(error) = sent {
        // At capacity, admission can send Busy and close before reading our
        // Hello. A failed write does not discard that peer's buffered rejection.
        // Accept only the explicit pre-Hello Busy frame; otherwise preserve the
        // original I/O error and existing startup recovery.
        if matches!(&error, DaemonError::Io { os_code: Some(code), .. } if matches!(
            std::io::Error::from_raw_os_error(*code).kind(),
            std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::NotConnected
        )) && let Err(DaemonError::Busy) = observation::stage(ClientStage::Hello, || match frame::read_limited(stream, deadline, 256)? {
            ServerFrame::Failure(DaemonError::Busy) => Err(DaemonError::Busy),
            _ => Ok(()),
        }) {
            return Err(DaemonError::Busy);
        }
        return Err(error);
    }
    let response = observation::stage(ClientStage::Hello, || frame::read(stream, deadline))?;
    observation::stage(ClientStage::Hello, || match response {
        ServerFrame::Ready(info) => {
            if info.protocol_version != PROTOCOL_VERSION || info.canonical_scope != scope.root() || info.pid != pid {
                return Err(mismatch(scope, pid, &format!("Ready differs: daemon protocol={} version={} cohort={} scope={} pid={}", info.protocol_version, info.build_version, info.build_id, info.canonical_scope.display(), info.pid)));
            }
            if info.database_generation.is_empty() || info.daemon_instance.is_empty() {
                return Err(DaemonError::Protocol("incomplete readiness identity".into()));
            }
            observation::ready(&info, ObservedProcess { pid, incarnation });
            Ok(info)
        }
        ServerFrame::Failure(DaemonError::Incompatible(detail)) => Err(mismatch(scope, pid, &format!("daemon rejected Hello: {detail}; daemon version/cohort unavailable unless present in that reply"))),
        ServerFrame::Failure(error) => Err(error),
        _ => Err(DaemonError::Protocol("expected readiness handshake".into())),
    })
}

fn mismatch(scope: &DaemonScope, pid: u32, detail: &str) -> DaemonError {
    let executable = std::env::current_exe().unwrap_or_else(|_| "agentx".into());
    #[cfg(windows)]
    let invocation_prefix = "& ";
    #[cfg(not(windows))]
    let invocation_prefix = "";
    DaemonError::Incompatible(format!(
        "scope={}; client protocol={} version={} cohort={}; authenticated daemon pid={pid}; {detail}. Recover explicitly: {invocation_prefix}{} daemon restart --scope {}",
        scope.root().display(),
        PROTOCOL_VERSION,
        BUILD_VERSION,
        super::wire::build_identity(),
        quote_argument(executable.as_os_str()),
        quote_argument(scope.root().as_os_str())
    ))
}

/// Quote the two path operands for a copyable local recovery command.
fn quote_argument(value: &std::ffi::OsStr) -> String {
    let value = value.to_string_lossy();
    #[cfg(not(windows))]
    {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
    #[cfg(windows)]
    {
        format!("'{}'", value.replace('\'', "''"))
    }
}

#[cfg(test)]
mod lifecycle_diagnostic_tests {
    use super::*;

    #[test]
    fn mismatch_reports_loaded_client_scope_peer_and_copyable_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let scope = DaemonScope::prepare(directory.path().join("a scope's data")).unwrap();
        let detail = mismatch(&scope, 321, "daemon version/cohort unavailable").to_string();
        assert!(detail.contains(super::super::wire::build_identity()));
        assert!(detail.contains(&scope.root().display().to_string()));
        assert!(detail.contains("authenticated daemon pid=321"));
        assert!(detail.contains("daemon restart --scope"));
        assert!(detail.contains("daemon version/cohort unavailable"));
        assert!(detail.contains(&quote_argument(scope.root().as_os_str())));
    }
}

pub(super) fn request_deadline(unix_ms: Option<u64>) -> Result<Instant, DaemonError> {
    let now = Instant::now();
    let maximum = now + IO_TIMEOUT;
    let Some(unix_ms) = unix_ms else {
        return Ok(maximum);
    };
    let wall = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| DaemonError::Unavailable(error.to_string()))?;
    let remaining = Duration::from_millis(unix_ms).checked_sub(wall).ok_or(DaemonError::Deadline)?;
    if remaining.is_zero() {
        return Err(DaemonError::Deadline);
    }
    Ok(maximum.min(now + remaining.min(IO_TIMEOUT)))
}

fn unknown(error: DaemonError) -> DaemonError {
    DaemonError::OutcomeUnknown(error.to_string())
}

fn pause(deadline: Instant) -> Result<(), DaemonError> {
    if Instant::now() >= deadline {
        return Err(DaemonError::Deadline);
    }
    std::thread::park_timeout(Duration::from_millis(20).min(deadline.saturating_duration_since(Instant::now())));
    Ok(())
}
