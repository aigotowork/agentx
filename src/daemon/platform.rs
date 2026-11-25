//! OS-authenticated Unix peers and deadline-bounded local connect.

#[cfg(unix)]
use std::io;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
#[cfg(unix)]
use std::os::unix::net::UnixStream;

#[cfg(unix)]
use super::{DaemonError, DaemonScope, ScopeLease};

#[cfg(unix)]
pub(crate) type Stream = UnixStream;
#[cfg(windows)]
pub(crate) use super::windows::connect as connect_existing;
#[cfg(windows)]
pub(crate) use super::windows::{Listener, Stream, connect, connect_observation, peer};

#[cfg(unix)]
pub(crate) fn connect(scope: &DaemonScope, deadline: std::time::Instant) -> Result<Stream, DaemonError> {
    unix_connect(&scope.socket_path()?, deadline)
}

/// Native lifecycle diagnosis never creates or repairs endpoint directories.
#[cfg(unix)]
pub(crate) fn connect_existing(scope: &DaemonScope, deadline: std::time::Instant) -> Result<Stream, DaemonError> {
    unix_connect(&scope.existing_socket_path()?, deadline)
}

#[cfg(unix)]
pub(crate) fn connect_observation(scope: &DaemonScope, deadline: std::time::Instant) -> Result<Stream, DaemonError> {
    unix_connect(&scope.observation_socket_path(false)?, deadline)
}

#[cfg(target_os = "linux")]
pub(crate) fn connect_cloud(scope: &DaemonScope, deadline: std::time::Instant) -> Result<Stream, DaemonError> {
    unix_connect(&scope.cloud_socket_path(false)?, deadline)
}

#[cfg(unix)]
fn unix_connect(path: &std::path::Path, deadline: std::time::Instant) -> Result<UnixStream, DaemonError> {
    use std::os::unix::ffi::OsStrExt;
    let name = path.as_os_str().as_bytes();
    // SAFETY: sockaddr_un is a plain OS value, initialized before use.
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if name.is_empty() || name.contains(&0) || name.len() >= address.sun_path.len() {
        return Err(DaemonError::Protocol("invalid Unix socket pathname".into()));
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (slot, byte) in address.sun_path.iter_mut().zip(name) {
        *slot = *byte as libc::c_char;
    }
    let length = (std::mem::offset_of!(libc::sockaddr_un, sun_path) + name.len() + 1) as libc::socklen_t;
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly"))]
    {
        address.sun_len = length as u8;
    }
    // SAFETY: socket creates a new owned descriptor; failure is checked.
    let raw = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if raw == -1 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: raw is newly created and is owned exactly once by this guard.
    let descriptor = unsafe { OwnedFd::from_raw_fd(raw) };
    // SAFETY: both fcntl calls operate on the owned live descriptor.
    if unsafe { libc::fcntl(raw, libc::F_SETFD, libc::FD_CLOEXEC) } == -1 || unsafe { libc::fcntl(raw, libc::F_SETFL, libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: address contains a terminated pathname within the supplied size.
    let connected = unsafe { libc::connect(raw, (&address as *const libc::sockaddr_un).cast(), length) };
    // SAFETY: ownership transfers from descriptor to the stream exactly once.
    let stream = unsafe { UnixStream::from_raw_fd(descriptor.into_raw_fd()) };
    if connected == -1 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(error.into());
        }
        super::frame::wait(&stream, libc::POLLOUT, deadline)?;
        let mut failure: libc::c_int = 0;
        let mut length = std::mem::size_of_val(&failure) as libc::socklen_t;
        // SAFETY: correctly sized integer storage and live stream.
        if unsafe { libc::getsockopt(stream.as_raw_fd(), libc::SOL_SOCKET, libc::SO_ERROR, (&mut failure as *mut libc::c_int).cast(), &mut length) } == -1 {
            return Err(io::Error::last_os_error().into());
        }
        if failure != 0 {
            return Err(io::Error::from_raw_os_error(failure).into());
        }
    }
    Ok(stream)
}

/// The kernel supplies PID/UID. The process incarnation is observed locally;
/// none of these facts are accepted from the serialized request body.
#[cfg(unix)]
pub(crate) fn peer(stream: &UnixStream) -> Result<(u32, Option<String>), DaemonError> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let (pid, uid) = {
        // SAFETY: ucred is plain output storage for SO_PEERCRED.
        let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
        let mut length = std::mem::size_of_val(&credentials) as libc::socklen_t;
        // SAFETY: all output pointers have the supplied valid size.
        if unsafe { libc::getsockopt(stream.as_raw_fd(), libc::SOL_SOCKET, libc::SO_PEERCRED, (&mut credentials as *mut libc::ucred).cast(), &mut length) } == -1 {
            return Err(io::Error::last_os_error().into());
        }
        (credentials.pid as u32, credentials.uid)
    };
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let (pid, uid) = {
        let (mut uid, mut gid, mut pid) = (0, 0, 0 as libc::pid_t);
        // SAFETY: getpeereid receives valid output pointers and a live socket.
        if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } == -1 {
            return Err(io::Error::last_os_error().into());
        }
        let mut length = std::mem::size_of_val(&pid) as libc::socklen_t;
        // SAFETY: pid is correctly sized output storage for LOCAL_PEERPID.
        if unsafe { libc::getsockopt(stream.as_raw_fd(), libc::SOL_LOCAL, libc::LOCAL_PEERPID, (&mut pid as *mut libc::pid_t).cast(), &mut length) } == -1 {
            return Err(io::Error::last_os_error().into());
        }
        (pid as u32, uid)
    };
    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos", target_os = "ios")))]
    return Err(DaemonError::Unsupported("authenticated local peer PID is unavailable on this platform".into()));
    #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos", target_os = "ios"))]
    {
        // SAFETY: geteuid has no preconditions.
        if pid == 0 || uid != unsafe { libc::geteuid() } {
            return Err(DaemonError::Protocol("local peer must have a valid PID and the daemon owner's UID".into()));
        }
        Ok((pid, crate::sys::process::identity(pid)))
    }
}

/// Owns only the endpoint inode created under the scope lease. Cleanup cannot
/// remove a replacement endpoint after an external unlink/rebind.
#[cfg(unix)]
pub(crate) struct Listener {
    socket: std::os::unix::net::UnixListener,
    path: std::path::PathBuf,
    device: u64,
    inode: u64,
    checked_at: std::time::Instant,
}

#[cfg(unix)]
impl Listener {
    pub(crate) fn bind(lease: &ScopeLease) -> Result<Self, DaemonError> {
        Self::bind_path(lease, lease.scope().socket_path()?)
    }

    pub(crate) fn bind_observation(lease: &ScopeLease) -> Result<Self, DaemonError> {
        Self::bind_path(lease, lease.scope().observation_socket_path(true)?)
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn bind_cloud(lease: &ScopeLease) -> Result<Self, DaemonError> {
        Self::bind_path(lease, lease.scope().cloud_socket_path(true)?)
    }

    fn bind_path(lease: &ScopeLease, path: std::path::PathBuf) -> Result<Self, DaemonError> {
        use std::os::unix::fs::FileTypeExt;
        lease.ensure_ownership()?;
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_socket() => std::fs::remove_file(&path)?,
            Ok(_) => return Err(DaemonError::Unavailable("daemon endpoint is not a socket".into())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Self::bind_empty(path)
    }

    fn bind_empty(path: std::path::PathBuf) -> Result<Self, DaemonError> {
        use std::os::unix::fs::MetadataExt;
        let socket = std::os::unix::net::UnixListener::bind(&path)?;
        socket.set_nonblocking(true)?;
        crate::sys::fs::set_private(&path)?;
        let metadata = std::fs::symlink_metadata(&path)?;
        Ok(Self { socket, path, device: metadata.dev(), inode: metadata.ino(), checked_at: std::time::Instant::now() })
    }

    /// Temp-directory cleanup may unlink a live listener. Only bind a missing
    /// pathname under the original lease. Existing connections keep their fds.
    pub(crate) fn restore_if_missing(&mut self, lease: &ScopeLease) -> Result<(), DaemonError> {
        use std::os::unix::fs::MetadataExt;
        if self.checked_at.elapsed() < std::time::Duration::from_millis(250) {
            return Ok(());
        }
        self.checked_at = std::time::Instant::now();
        match std::fs::symlink_metadata(&self.path) {
            Ok(meta) if (meta.dev(), meta.ino()) == (self.device, self.inode) => return Ok(()),
            Ok(_) => return Err(DaemonError::Unavailable("daemon endpoint was replaced".into())),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
        lease.ensure_ownership()?;
        super::scope::private_directory(self.path.parent().expect("absolute socket path"))?;
        *self = Self::bind_empty(self.path.clone())?;
        Ok(())
    }

    /// Socket readiness wakes admission immediately. The deadline bounds only
    /// control/stop observation; application Jobs have a separate receiver.
    pub(crate) fn wait_ready(&self, deadline: std::time::Instant) -> Result<bool, DaemonError> {
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Ok(false);
            }
            let millis = remaining.as_nanos().div_ceil(1_000_000).min(i32::MAX as u128) as i32;
            let mut descriptor = libc::pollfd { fd: self.socket.as_raw_fd(), events: libc::POLLIN, revents: 0 };
            // SAFETY: one live owned listener descriptor and bounded timeout.
            let result = unsafe { libc::poll(&mut descriptor, 1, millis) };
            if result == 0 {
                return Ok(false);
            }
            if result > 0 {
                if descriptor.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                    return Err(DaemonError::Unavailable("listener readiness failed".into()));
                }
                return Ok(descriptor.revents & libc::POLLIN != 0);
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error.into());
            }
        }
    }

    pub(crate) fn accept(&self) -> Result<Option<Stream>, DaemonError> {
        match self.socket.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(true)?;
                Ok(Some(stream))
            }
            Err(error) if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(unix)]
impl Drop for Listener {
    fn drop(&mut self) {
        use std::os::unix::fs::MetadataExt;
        if let Ok(metadata) = std::fs::symlink_metadata(&self.path)
            && metadata.dev() == self.device
            && metadata.ino() == self.inode
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
