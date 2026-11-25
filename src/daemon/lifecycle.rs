//! Read and restart one verified scope owner without opening its persistence.

use std::fs::{OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::{DaemonError, DaemonInfo, DaemonScope, legacy, platform, transport};
use transport::{STATUS_TIMEOUT, missing_endpoint};
const START_TIMEOUT: Duration = Duration::from_secs(15);
// Connection workers have 30-second I/O deadlines. A current application call
// can outlive that wait; timeout is a refusal to replace it, never a force kill.
const STOP_TIMEOUT: Duration = Duration::from_secs(35);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Stopped,
    Running,
    Incompatible,
    Unavailable,
}

#[derive(Debug)]
pub(crate) struct Status {
    pub scope: PathBuf,
    pub state: State,
    pub peer: Option<(u32, Option<String>)>,
    pub info: Option<DaemonInfo>,
    pub detail: Option<String>,
}

/// Read existing native state without creating directories, repairing modes,
/// starting a daemon, resolving an actor or opening managed SQLite.
pub(crate) fn status(path: &Path, cwd: &Path) -> Result<Status, DaemonError> {
    let scope = match DaemonScope::existing(path, cwd) {
        Ok(scope) => scope,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Status { scope: absolute(path, cwd), state: State::Stopped, peer: None, info: None, detail: None });
        }
        Err(error) => return Err(error.into()),
    };
    let deadline = Instant::now() + STATUS_TIMEOUT;
    let mut result = Status { scope: scope.root().into(), state: State::Unavailable, peer: None, info: None, detail: None };
    match platform::connect_existing(&scope, deadline) {
        Ok(mut stream) => {
            result.peer = Some(platform::peer(&stream)?);
            match transport::status_on(&scope, &mut stream, None, deadline) {
                Ok((info, _)) => {
                    result.state = State::Running;
                    result.info = Some(info);
                }
                Err(error @ DaemonError::Incompatible(_)) => {
                    result.state = State::Incompatible;
                    result.detail = Some(error.to_string());
                }
                Err(error) => result.detail = Some(error.to_string()),
            }
        }
        Err(error) => {
            if missing_endpoint(&error) && owner_released(&scope)? {
                result.state = State::Stopped;
            } else {
                result.detail = Some(format!("{error}; scope may be initializing, stopping or inaccessible"));
            }
        }
    }
    Ok(result)
}

/// Stop only an authenticated selected-scope owner, wait for ownership release,
/// then acquire current code while retaining the common startup lock. A failed
/// or uncertain stop never authorizes spawning, force killing or lock deletion.
pub(crate) fn restart(path: &Path) -> Result<DaemonInfo, DaemonError> {
    restart_with_upgrade(path).map(|(info, _)| info)
}

pub(crate) fn restart_with_upgrade(path: &Path) -> Result<(DaemonInfo, bool), DaemonError> {
    let scope = DaemonScope::prepare(path)?;
    let _startup = transport::startup_lock(&scope, Instant::now() + START_TIMEOUT)?;
    let deadline = Instant::now() + STOP_TIMEOUT;
    let upgraded = stop_existing(&scope, deadline)?;
    wait_for_release(&scope, deadline)?;
    let info = transport::acquire_locked(&scope, Instant::now() + START_TIMEOUT)?;
    Ok((info, upgraded))
}

fn stop_existing(scope: &DaemonScope, deadline: Instant) -> Result<bool, DaemonError> {
    let status_deadline = deadline.min(Instant::now() + STATUS_TIMEOUT);
    let mut stream = match platform::connect_existing(scope, status_deadline) {
        Ok(stream) => stream,
        Err(error) if missing_endpoint(&error) && owner_released(scope)? => return Ok(false),
        Err(error) => return Err(DaemonError::Unavailable(format!("restart refused: cannot authenticate selected-scope daemon ({error}); no replacement started"))),
    };
    let peer = platform::peer(&stream)?;
    match transport::status_on(scope, &mut stream, None, status_deadline) {
        Ok((info, observed)) if observed.pid == peer.0 && observed.incarnation == peer.1 => {
            transport::shutdown_instance(scope, &info, deadline)?;
            Ok(false)
        }
        Err(DaemonError::Incompatible(detail)) if detail.contains("legacy daemon requires its one-time upgrade") && !owner_released(scope)? => {
            drop(stream);
            legacy::shutdown(scope, &peer, deadline)?;
            Ok(true)
        }
        Ok(_) => Err(DaemonError::Incompatible("daemon peer changed during restart".into())),
        Err(error) => Err(DaemonError::Unavailable(format!("restart refused: daemon identity/status failed ({error}); no replacement started"))),
    }
}

fn absolute(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() { path.into() } else { cwd.join(path) }
}

/// A noncreating lock check distinguishes absent admission from an initializing
/// or draining owner. Never remove, truncate or replace the lifetime lock.
fn owner_released(scope: &DaemonScope) -> Result<bool, DaemonError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let lock = match options.open(scope.state().join("owner.lock")) {
        Ok(lock) => lock,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(error.into()),
    };
    if !lock.metadata()?.is_file() {
        return Err(DaemonError::Unavailable("restart refused: owner.lock is not a regular file".into()));
    }
    match lock.try_lock() {
        Ok(()) => Ok(true),
        Err(TryLockError::WouldBlock) => Ok(false),
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

fn wait_for_release(scope: &DaemonScope, deadline: Instant) -> Result<(), DaemonError> {
    while !owner_released(scope)? {
        if Instant::now() >= deadline {
            return Err(DaemonError::Unavailable("restart timed out waiting for graceful daemon drain; no force kill or replacement attempted; inspect daemon status before retrying".into()));
        }
        std::thread::park_timeout(Duration::from_millis(20).min(deadline.saturating_duration_since(Instant::now())));
    }
    Ok(())
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
