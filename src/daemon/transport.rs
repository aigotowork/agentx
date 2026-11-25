//! Connection acquisition and one-shot native calls. Only discovery is retried;
//! once a call frame may have been written its outcome requires reconciliation.

use std::fs::{File, OpenOptions};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::wire::{BUILD_VERSION, ClientFrame, DaemonInfo, PROTOCOL_VERSION, SHUTDOWN, STATUS, ServerFrame};
use super::{DaemonError, DaemonScope, frame, platform};
use crate::observability::runtime::{ClientStage, ObservedProcess};
use crate::service::observation;
use crate::service::{CallerContext, Request, RequestId, Response, ServiceError, ServiceResult};

pub(super) const IO_TIMEOUT: Duration = Duration::from_secs(30);
const START_TIMEOUT: Duration = Duration::from_secs(15);
pub(super) const STATUS_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn describe(scope: &DaemonScope, start: bool, expected: Option<&str>, deadline: Option<Instant>) -> Result<DaemonInfo, DaemonError> {
    let end = deadline.unwrap_or_else(|| Instant::now() + START_TIMEOUT);
    let mut stream = connect(scope, start, end)?;
    status_on(scope, &mut stream, expected, end.min(Instant::now() + STATUS_TIMEOUT)).map(|(info, _)| info)
}

#[cfg(test)]
pub(crate) fn probe(scope: &DaemonScope) -> Result<DaemonInfo, DaemonError> {
    describe(scope, false, None, Some(Instant::now() + STATUS_TIMEOUT))
}

pub(crate) fn acquire(scope: &DaemonScope) -> Result<DaemonInfo, DaemonError> {
    describe(scope, true, None, None)
}

fn connect(scope: &DaemonScope, start: bool, deadline: Instant) -> Result<platform::Stream, DaemonError> {
    let attempt = || observation::stage(ClientStage::Connect, || platform::connect_existing(scope, deadline));
    match attempt() {
        Ok(stream) => return Ok(stream),
        Err(error) if start && missing_endpoint(&error) => {}
        Err(error) => return Err(error),
    }
    let deadline = deadline.min(Instant::now() + START_TIMEOUT);
    let _startup = startup_lock(scope, deadline)?;
    connect_locked(scope, deadline)
}

pub(super) fn missing_endpoint(error: &DaemonError) -> bool {
    match error {
        #[cfg(unix)]
        DaemonError::Io { os_code: Some(code), .. } => matches!(*code, libc::ENOENT | libc::ECONNREFUSED),
        #[cfg(windows)]
        DaemonError::Io { os_code: Some(code), .. } => matches!(*code, 2 | 3),
        _ => false,
    }
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
    let mut stream = connect_locked(scope, deadline)?;
    status_on(scope, &mut stream, None, deadline.min(Instant::now() + STATUS_TIMEOUT)).map(|(info, _)| info)
}

fn connect_locked(scope: &DaemonScope, deadline: Instant) -> Result<platform::Stream, DaemonError> {
    let attempt = || observation::stage(ClientStage::Connect, || platform::connect(scope, deadline));
    match attempt() {
        Ok(stream) => return Ok(stream),
        Err(error) if missing_endpoint(&error) => {}
        Err(error) => return Err(error),
    }
    // The child takes the lifetime singleton before constructing Application.
    if Instant::now() >= deadline {
        return Err(DaemonError::Deadline);
    }
    let mut child = None;
    let mut exit_status = None;
    loop {
        match attempt() {
            Ok(stream) => return Ok(stream),
            Err(error) if missing_endpoint(&error) => {}
            Err(error) => return Err(error),
        }
        if child.is_none() && super::lifecycle::owner_released(scope)? {
            child = Some(observation::stage(ClientStage::Spawn, || spawn_daemon(scope))?);
        }
        if exit_status.is_none()
            && let Some(child) = &mut child
        {
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

#[cfg(test)]
pub(crate) fn call(scope: &DaemonScope, caller: CallerContext, request: Request) -> Result<ServiceResult<Response>, DaemonError> {
    call_bound(scope, false, caller, request).map(|(_, result)| result)
}

pub(crate) fn call_bound(scope: &DaemonScope, start: bool, caller: CallerContext, request: Request) -> Result<(DaemonInfo, ServiceResult<Response>), DaemonError> {
    let deadline = observation::stage(ClientStage::Capture, || request_deadline(caller.deadline_unix_ms))?;
    let mut stream = connect(scope, start, deadline)?;
    let process = authenticated_peer(&stream)?;
    let request_id = caller.request_id.clone();
    let expected = caller.expected_database_generation.clone();
    let resetting = matches!(request, Request::Maintenance(crate::service::maintenance::Request::ResetConversation));
    let operation = call_frame(scope, None, caller, request);
    // Serialize before marking the possible write. After attempting it,
    // retain the original request ID and never retry the operation.
    let body = frame::encode_limited(&operation, frame::MAX_FRAME_BYTES)?;
    observation::may_write();
    if let Err(error) = observation::stage(ClientStage::CallWrite, || frame::write_encoded(&mut stream, &body, deadline)) {
        return Err(pre_admission_busy(&mut stream, deadline, error).map_unknown(&request_id));
    }
    match observation::stage(ClientStage::ReplyRead, || frame::read(&mut stream, deadline)).map_err(|error| error.map_unknown(&request_id))? {
        ServerFrame::Result { result, info, executed_generation, request_id: reply_id } => {
            validate_info(scope, &process, &info).map_err(|error| error.map_unknown(&request_id))?;
            if request_id != reply_id || executed_generation.is_empty() || expected.as_ref().is_some_and(|expected| expected != &executed_generation) {
                return Err(DaemonError::Protocol("result does not match request ID or generation".into()).map_unknown(&request_id));
            }
            if info.database_generation != executed_generation && !resetting {
                if matches!(result, Err(ServiceError::StaleGeneration)) {
                    return Err(DaemonError::StaleGeneration);
                }
                return Err(DaemonError::Protocol("result generation changed during execution".into()).map_unknown(&request_id));
            }
            observation::ready(&info, process);
            observation::application_result(&result);
            Ok((info, result))
        }
        ServerFrame::Failure(error) => {
            let error = match diagnose_legacy(scope, process.pid, error) {
                DaemonError::OutcomeUnknown(detail) => DaemonError::OutcomeUnknown(format!("request {}: {detail}", request_id.0)),
                other => other,
            };
            observation::received_failure(&error);
            Err(error)
        }
        _ => Err(DaemonError::Protocol("unexpected reply after call".into()).map_unknown(&request_id)),
    }
}

fn call_frame<R>(scope: &DaemonScope, instance: Option<String>, caller: CallerContext, request: R) -> ClientFrame<R> {
    ClientFrame::Call { canonical_scope: scope.root().to_path_buf(), daemon_instance: instance, caller, request }
}

fn lifecycle_caller(expected: Option<&str>) -> CallerContext {
    CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: Default::default(), deadline_unix_ms: None, expected_database_generation: expected.map(str::to_owned) }
}

fn authenticated_peer(stream: &platform::Stream) -> Result<ObservedProcess, DaemonError> {
    let (pid, incarnation) = observation::stage(ClientStage::PeerAuthentication, || platform::peer(stream))?;
    Ok(ObservedProcess { pid, incarnation })
}

pub(super) fn status_on(scope: &DaemonScope, stream: &mut platform::Stream, expected: Option<&str>, deadline: Instant) -> Result<(DaemonInfo, ObservedProcess), DaemonError> {
    let process = authenticated_peer(stream)?;
    let call = call_frame(scope, None, lifecycle_caller(expected), STATUS);
    if let Err(error) = observation::stage(ClientStage::CallWrite, || frame::write(stream, &call, deadline)) {
        return Err(pre_admission_busy(stream, deadline, error));
    }
    match observation::stage(ClientStage::ReplyRead, || frame::read(stream, deadline))? {
        ServerFrame::Status(info) => {
            validate_info(scope, &process, &info)?;
            if expected.is_some_and(|expected| expected != info.database_generation) {
                return Err(DaemonError::StaleGeneration);
            }
            observation::ready(&info, process.clone());
            Ok((info, process))
        }
        ServerFrame::Failure(error) => Err(diagnose_legacy(scope, process.pid, error)),
        _ => Err(DaemonError::Protocol("expected daemon status".into())),
    }
}

fn pre_admission_busy(stream: &mut platform::Stream, deadline: Instant, error: DaemonError) -> DaemonError {
    if matches!(&error, DaemonError::Io { os_code: Some(code), .. } if matches!(
        std::io::Error::from_raw_os_error(*code).kind(),
        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::NotConnected
    )) && matches!(frame::read_limited(stream, deadline, 256), Ok(ServerFrame::Failure(DaemonError::Busy)))
    {
        return DaemonError::Busy;
    }
    error
}

fn diagnose_legacy(scope: &DaemonScope, pid: u32, error: DaemonError) -> DaemonError {
    match error {
        DaemonError::Protocol(detail) if detail.contains("first frame must be Hello") || detail.contains("missing field `daemon_instance`") || detail.contains("unknown variant `DaemonStatus`") => {
            mismatch(scope, pid, "legacy daemon requires its one-time upgrade")
        }
        error => error,
    }
}

pub(super) fn validate_info(scope: &DaemonScope, peer: &ObservedProcess, info: &DaemonInfo) -> Result<(), DaemonError> {
    if info.canonical_scope != scope.root() || info.pid != peer.pid {
        return Err(DaemonError::Incompatible("response scope or daemon PID differs from the authenticated connection".into()));
    }
    if info.database_generation.is_empty() || info.daemon_instance.is_empty() {
        return Err(DaemonError::Protocol("incomplete daemon identity".into()));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn shutdown(scope: &DaemonScope) -> Result<(), DaemonError> {
    let deadline = Instant::now() + IO_TIMEOUT;
    let info = probe(scope)?;
    shutdown_instance(scope, &info, deadline)
}

pub(super) fn shutdown_instance(scope: &DaemonScope, info: &DaemonInfo, deadline: Instant) -> Result<(), DaemonError> {
    let mut stream = platform::connect_existing(scope, deadline)?;
    if platform::peer(&stream)?.0 != info.pid {
        return Err(DaemonError::Incompatible("shutdown peer changed".into()));
    }
    let caller = lifecycle_caller(None);
    let id = caller.request_id.clone();
    frame::write(&mut stream, &call_frame(scope, Some(info.daemon_instance.clone()), caller, SHUTDOWN), deadline).map_err(|error| error.map_unknown(&id))?;
    match frame::read(&mut stream, deadline).map_err(|error| error.map_unknown(&id))? {
        ServerFrame::Stopping => Ok(()),
        ServerFrame::Failure(error) => Err(error),
        _ => Err(DaemonError::Protocol("expected shutdown acknowledgement".into()).map_unknown(&id)),
    }
}

impl DaemonError {
    fn map_unknown(self, request_id: &RequestId) -> Self {
        if matches!(self, Self::Busy) { self } else { Self::OutcomeUnknown(format!("request {}: {self}", request_id.0)) }
    }
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

pub(crate) fn request_deadline(unix_ms: Option<u64>) -> Result<Instant, DaemonError> {
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

fn pause(deadline: Instant) -> Result<(), DaemonError> {
    if Instant::now() >= deadline {
        return Err(DaemonError::Deadline);
    }
    std::thread::park_timeout(Duration::from_millis(20).min(deadline.saturating_duration_since(Instant::now())));
    Ok(())
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
