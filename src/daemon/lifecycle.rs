//! Explicit selected-scope diagnosis and recovery. Compatibility is never
//! bypassed for business calls; incompatible-protocol recovery requires native ownership.

use std::fs::{OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::{DaemonError, DaemonInfo, DaemonScope, platform, transport};

const HELLO_TIMEOUT: Duration = Duration::from_secs(2);
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
    let deadline = Instant::now() + HELLO_TIMEOUT;
    let mut result = Status { scope: scope.root().into(), state: State::Unavailable, peer: None, info: None, detail: None };
    match platform::connect_existing(&scope, deadline) {
        Ok(mut stream) => {
            result.peer = Some(platform::peer(&stream)?);
            match transport::hello(&scope, &mut stream, deadline) {
                Ok(info) => {
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
    let scope = DaemonScope::prepare(path)?;
    let _startup = transport::startup_lock(&scope, Instant::now() + START_TIMEOUT)?;
    let deadline = Instant::now() + STOP_TIMEOUT;
    stop_existing(&scope, deadline)?;
    wait_for_release(&scope, deadline)?;
    transport::acquire_locked(&scope, Instant::now() + START_TIMEOUT)
}

fn stop_existing(scope: &DaemonScope, deadline: Instant) -> Result<(), DaemonError> {
    let hello_deadline = deadline.min(Instant::now() + HELLO_TIMEOUT);
    let mut stream = match platform::connect_existing(scope, hello_deadline) {
        Ok(stream) => stream,
        Err(error) if missing_endpoint(&error) && owner_released(scope)? => return Ok(()),
        Err(error) => return Err(DaemonError::Unavailable(format!("restart refused: cannot authenticate selected-scope daemon ({error}); no replacement started"))),
    };
    let (pid, incarnation) = platform::peer(&stream)?;
    match transport::hello(scope, &mut stream, hello_deadline) {
        Ok(info) => transport::shutdown_connection(&mut stream, &info, deadline),
        Err(DaemonError::Incompatible(_)) => stop_incompatible(scope, pid, incarnation.as_deref(), &stream, deadline),
        Err(error) => Err(DaemonError::Unavailable(format!("restart refused: daemon identity/handshake failed ({error}); no replacement started"))),
    }
}

fn absolute(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() { path.into() } else { cwd.join(path) }
}

fn missing_endpoint(error: &DaemonError) -> bool {
    match error {
        #[cfg(unix)]
        DaemonError::Io { os_code: Some(code), .. } => matches!(*code, libc::ENOENT | libc::ECONNREFUSED),
        #[cfg(windows)]
        DaemonError::Io { os_code: Some(code), .. } => matches!(*code, 2 | 3),
        _ => false,
    }
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

/// Legacy recovery is deliberately limited to Linux's process handle + lock
/// owner evidence. Other platforms keep compatible native Shutdown and refuse
/// old-build signalling rather than falling back to a reusable numeric PID.
#[cfg(not(target_os = "linux"))]
fn stop_incompatible(_scope: &DaemonScope, _pid: u32, _incarnation: Option<&str>, _stream: &platform::Stream, _deadline: Instant) -> Result<(), DaemonError> {
    Err(DaemonError::Unsupported("incompatible-daemon recovery requires Linux pidfd and verified scope-lock ownership; use its original compatible executable to shut it down on this platform".into()))
}

#[cfg(target_os = "linux")]
fn stop_incompatible(scope: &DaemonScope, pid: u32, incarnation: Option<&str>, stream: &platform::Stream, deadline: Instant) -> Result<(), DaemonError> {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    let incarnation = incarnation.filter(|value| !value.is_empty()).ok_or_else(|| refusal("kernel peer incarnation unavailable"))?;
    if pid == std::process::id() {
        return Err(refusal("peer is the invoking process"));
    }
    // SAFETY: pidfd_open returns a new owned process descriptor or -1; no
    // signal is sent until the peer, incarnation and scope ownership recheck.
    let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0u32) };
    if raw == -1 {
        return Err(refusal(&format!("cannot pin peer process: {}", std::io::Error::last_os_error())));
    }
    // SAFETY: raw is a fresh descriptor owned exactly once.
    let handle = unsafe { OwnedFd::from_raw_fd(raw as i32) };
    validate_legacy_owner(scope, pid, incarnation)?;
    let (current_pid, current_incarnation) = platform::peer(stream)?;
    if current_pid != pid || current_incarnation.as_deref() != Some(incarnation) || crate::sys::process::identity(pid).as_deref() != Some(incarnation) {
        return Err(refusal("authenticated peer incarnation changed"));
    }
    if Instant::now() >= deadline {
        return Err(DaemonError::Deadline);
    }
    // SAFETY: pidfd targets only the pinned process, never a reused numeric PID.
    // SIGTERM enters its existing stop flag and drains the current call.
    if unsafe { libc::syscall(libc::SYS_pidfd_send_signal, handle.as_raw_fd(), libc::SIGTERM, std::ptr::null::<libc::siginfo_t>(), 0u32) } == -1 {
        return Err(refusal(&format!("graceful stop request failed: {}", std::io::Error::last_os_error())));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn refusal(detail: &str) -> DaemonError {
    DaemonError::Unavailable(format!("restart refused: {detail}; no signal/replacement authorized"))
}

#[cfg(target_os = "linux")]
fn validate_legacy_owner(scope: &DaemonScope, pid: u32, incarnation: &str) -> Result<(), DaemonError> {
    use std::os::unix::fs::MetadataExt;
    let process = PathBuf::from(format!("/proc/{pid}"));
    let command = std::fs::read(process.join("cmdline")).map_err(|error| refusal(&format!("peer argv unavailable: {error}")))?;
    let arguments: Vec<&[u8]> = command.strip_suffix(&[0]).unwrap_or(&command).split(|byte| *byte == 0).collect();
    if !legacy_arguments(&arguments, scope.root()) {
        return Err(refusal("peer argv is not exact daemon serve --scope for selected scope"));
    }
    // Observe the loaded image, including an unlinked old executable. Its current
    // pathname never supplies a cohort; it only links argv[0] to this daemon.
    let image = std::fs::read_link(process.join("exe")).map_err(|error| refusal(&format!("loaded executable unavailable: {error}")))?;
    let metadata = std::fs::metadata(process.join("exe")).map_err(|error| refusal(&format!("loaded executable metadata unavailable: {error}")))?;
    if !metadata.is_file() || metadata.mode() & 0o111 == 0 || !loaded_image_matches(arguments[0], &image) {
        return Err(refusal("loaded executable does not match daemon argv"));
    }
    let lock_path = scope.state().join("owner.lock");
    let lock = std::fs::symlink_metadata(&lock_path).map_err(|error| refusal(&format!("scope lock unavailable: {error}")))?;
    if !lock.is_file() || lock.file_type().is_symlink() {
        return Err(refusal("scope lock is not a regular owned inode"));
    }
    // std File::try_lock uses flock on Linux. Require a whole-file exclusive
    // lock held by the authenticated PID on this exact device/inode.
    let locks = std::fs::read_to_string("/proc/locks").map_err(|error| refusal(&format!("kernel lock owners unavailable: {error}")))?;
    if !owns_lock(&locks, pid, lock.dev(), lock.ino()) {
        return Err(refusal("kernel peer does not hold the selected scope singleton lock"));
    }
    let after = std::fs::symlink_metadata(lock_path)?;
    if after.dev() != lock.dev() || after.ino() != lock.ino() || crate::sys::process::identity(pid).as_deref() != Some(incarnation) {
        return Err(refusal("scope lock or peer changed during ownership verification"));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn legacy_arguments(arguments: &[&[u8]], scope: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    arguments.len() == 5 && !arguments[0].is_empty() && arguments[1..4] == [b"daemon".as_slice(), b"serve".as_slice(), b"--scope".as_slice()] && arguments[4] == scope.as_os_str().as_bytes()
}

#[cfg(target_os = "linux")]
fn loaded_image_matches(argument: &[u8], image: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let bytes = image.as_os_str().as_bytes();
    bytes == argument || bytes.strip_suffix(b" (deleted)") == Some(argument)
}

#[cfg(target_os = "linux")]
fn owns_lock(locks: &str, pid: u32, device: u64, inode: u64) -> bool {
    locks.lines().any(|line| {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 8 || fields[1] != "FLOCK" || fields[2] != "ADVISORY" || fields[3] != "WRITE" || fields[4].parse::<u32>().ok() != Some(pid) || fields[6] != "0" || fields[7] != "EOF" {
            return false;
        }
        let object: Vec<&str> = fields[5].split(':').collect();
        object.len() == 3 && u64::from_str_radix(object[0], 16).ok() == Some(libc::major(device) as u64) && u64::from_str_radix(object[1], 16).ok() == Some(libc::minor(device) as u64) && object[2].parse::<u64>().ok() == Some(inode)
    })
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
