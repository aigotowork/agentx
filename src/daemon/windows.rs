//! Native Windows byte pipes. Security is a protected current-user DACL plus
//! kernel peer PID/token checks; overlapped I/O is canceled and reaped before
//! any borrowed buffer or OVERLAPPED storage is released.

use std::cell::RefCell;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::time::Instant;

use sha2::{Digest, Sha256};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::Authorization::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::IO::*;
use windows_sys::Win32::System::Pipes::*;
use windows_sys::Win32::System::Threading::*;

use super::{DaemonError, DaemonScope, ScopeLease};

pub(crate) struct Stream {
    handle: OwnedHandle,
    server: bool,
}

impl Stream {
    pub(super) fn read_bytes(&mut self, bytes: &mut [u8], deadline: Instant) -> Result<(), DaemonError> {
        self.read_bytes_optional(bytes, deadline)?.then_some(()).ok_or_else(|| DaemonError::Unavailable("pipe closed during frame read".into()))
    }

    pub(super) fn read_bytes_optional(&mut self, mut bytes: &mut [u8], deadline: Instant) -> Result<bool, DaemonError> {
        let mut read_any = false;
        while !bytes.is_empty() {
            let count = match transfer(self.handle.as_raw_handle(), bytes.as_mut_ptr(), bytes.len(), true, deadline) {
                Ok(count) => count,
                Err(DaemonError::Io { os_code: Some(code), .. }) if !read_any && matches!(code as u32, ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED | ERROR_NO_DATA) => return Ok(false),
                Err(error) => return Err(error),
            };
            if count == 0 && !read_any {
                return Ok(false);
            }
            if count == 0 {
                return Err(DaemonError::Unavailable("pipe closed during frame read".into()));
            }
            read_any = true;
            bytes = &mut bytes[count..];
        }
        Ok(true)
    }

    pub(super) fn write_bytes(&mut self, mut bytes: &[u8], deadline: Instant) -> Result<(), DaemonError> {
        while !bytes.is_empty() {
            let count = transfer(self.handle.as_raw_handle(), bytes.as_ptr().cast_mut(), bytes.len(), false, deadline)?;
            if count == 0 {
                return Err(DaemonError::Unavailable("pipe closed during frame write".into()));
            }
            bytes = &bytes[count..];
        }
        Ok(())
    }
}

struct Overlapped {
    value: Box<OVERLAPPED>,
    _event: OwnedHandle,
}

impl Overlapped {
    fn new() -> Result<Self, DaemonError> {
        // SAFETY: no name/security pointer; the event is owned and noninheritable.
        let event = owned(unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) })?;
        // SAFETY: zero is the valid initial state of an OVERLAPPED structure.
        let mut value: Box<OVERLAPPED> = Box::new(unsafe { std::mem::zeroed() });
        value.hEvent = event.as_raw_handle();
        Ok(Self { value, _event: event })
    }

    /// Cancellation must be followed by completion before freeing its buffer.
    fn cancel(&self, file: HANDLE) {
        let mut transferred = 0;
        // SAFETY: the operation and file remain alive until cancellation is
        // reaped; ERROR_NOT_FOUND simply means completion won the race.
        unsafe {
            CancelIoEx(file, &*self.value);
            GetOverlappedResult(file, &*self.value, &mut transferred, 1);
        }
    }
}

fn transfer(handle: HANDLE, bytes: *mut u8, length: usize, read: bool, deadline: Instant) -> Result<usize, DaemonError> {
    if Instant::now() >= deadline {
        return Err(DaemonError::Deadline);
    }
    let mut operation = Overlapped::new()?;
    let mut transferred = 0;
    let length = length.min(u32::MAX as usize) as u32;
    // SAFETY: the caller holds a live buffer of this length until completion;
    // WriteFile borrows immutably. This handle was opened for overlapped I/O.
    let completed = unsafe { if read { ReadFile(handle, bytes, length, &mut transferred, &mut *operation.value) } else { WriteFile(handle, bytes.cast_const(), length, &mut transferred, &mut *operation.value) } };
    if completed != 0 {
        return Ok(transferred as usize);
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) {
        return Err(error.into());
    }
    let millis = deadline.saturating_duration_since(Instant::now()).as_nanos().div_ceil(1_000_000).min((u32::MAX - 1) as u128) as u32;
    // SAFETY: operation owns a live manual-reset event until completion/reap.
    let wait = unsafe { WaitForSingleObject(operation.value.hEvent, millis) };
    if wait != WAIT_OBJECT_0 {
        operation.cancel(handle);
        return if wait == WAIT_TIMEOUT { Err(DaemonError::Deadline) } else { Err(io::Error::last_os_error().into()) };
    }
    // SAFETY: signaled event establishes completion and output storage is valid.
    if unsafe { GetOverlappedResult(handle, &*operation.value, &mut transferred, 0) } == 0 {
        return Err(io::Error::last_os_error().into());
    }
    Ok(transferred as usize)
}

pub(crate) fn connect(scope: &DaemonScope, deadline: Instant) -> Result<Stream, DaemonError> {
    connect_named(pipe_name(scope)?, deadline)
}

pub(crate) fn connect_observation(scope: &DaemonScope, deadline: Instant) -> Result<Stream, DaemonError> {
    connect_named(pipe_name_for(scope, true)?, deadline)
}

fn connect_named(name: Vec<u16>, deadline: Instant) -> Result<Stream, DaemonError> {
    loop {
        if Instant::now() >= deadline {
            return Err(DaemonError::Deadline);
        }
        // SAFETY: terminated pipe name; no inherited handles. Identification
        // quality of service prevents a pipe server impersonating this client.
        let handle = unsafe { CreateFileW(name.as_ptr(), GENERIC_READ | GENERIC_WRITE, 0, std::ptr::null(), OPEN_EXISTING, FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION, std::ptr::null_mut()) };
        if handle != INVALID_HANDLE_VALUE {
            return Ok(Stream { handle: owned(handle)?, server: false });
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(ERROR_PIPE_BUSY as i32) {
            return Err(error.into());
        }
        let millis = deadline.saturating_duration_since(Instant::now()).as_millis().clamp(1, 20) as u32;
        // SAFETY: same terminated local pipe name and finite wait.
        unsafe {
            WaitNamedPipeW(name.as_ptr(), millis);
        }
    }
}

pub(crate) fn peer(stream: &Stream) -> Result<(u32, Option<String>), DaemonError> {
    let mut pid = 0;
    // SAFETY: the connected pipe handle and PID output pointer are valid.
    let ok = unsafe { if stream.server { GetNamedPipeClientProcessId(stream.handle.as_raw_handle(), &mut pid) } else { GetNamedPipeServerProcessId(stream.handle.as_raw_handle(), &mut pid) } };
    if ok == 0 || pid == 0 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: query-only handle to the kernel-observed peer PID.
    let process = owned(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) })?;
    let ours = user_sid(unsafe { GetCurrentProcess() })?;
    let theirs = user_sid(process.as_raw_handle())?;
    if ours != theirs {
        return Err(DaemonError::Protocol("pipe peer belongs to another user".into()));
    }
    let incarnation = crate::sys::process::identity(pid);
    if incarnation.is_none() {
        return Err(DaemonError::Unavailable("cannot observe pipe peer incarnation".into()));
    }
    Ok((pid, incarnation))
}

/// Keep one pending overlapped accept. The first-instance flag prevents taking
/// over an occupied pipe name; additional instances stay under the same DACL.
pub(crate) struct Listener {
    name: Vec<u16>,
    security: SecurityDescriptor,
    pending: RefCell<Pending>,
}

impl Listener {
    pub(crate) fn bind(lease: &ScopeLease) -> Result<Self, DaemonError> {
        Self::bind_named(lease, pipe_name(lease.scope())?)
    }

    pub(crate) fn bind_observation(lease: &ScopeLease) -> Result<Self, DaemonError> {
        Self::bind_named(lease, pipe_name_for(lease.scope(), true)?)
    }

    fn bind_named(lease: &ScopeLease, name: Vec<u16>) -> Result<Self, DaemonError> {
        lease.ensure_ownership()?;
        let security = SecurityDescriptor::current_user()?;
        let pending = Pending::new(&name, &security, true)?;
        Ok(Self { name, security, pending: RefCell::new(pending) })
    }

    /// Wait only on this listener's pending connect, never the application
    /// queue. Pending/OVERLAPPED remain owned here until accept or Drop reap.
    pub(crate) fn wait_ready(&self, deadline: Instant) -> Result<bool, DaemonError> {
        let pending = self.pending.borrow();
        if pending.connected {
            return Ok(true);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(false);
        }
        let millis = remaining.as_nanos().div_ceil(1_000_000).min((u32::MAX - 1) as u128) as u32;
        // SAFETY: pending owns the live event and OVERLAPPED throughout wait.
        match unsafe { WaitForSingleObject(pending.operation.value.hEvent, millis) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => Err(io::Error::last_os_error().into()),
        }
    }

    pub(crate) fn accept(&self) -> Result<Option<Stream>, DaemonError> {
        let mut pending = self.pending.borrow_mut();
        if !pending.connected {
            let mut transferred = 0;
            // SAFETY: stable boxed OVERLAPPED and live owned pipe; nonblocking.
            if unsafe { GetOverlappedResult(pending.stream.as_ref().unwrap().handle.as_raw_handle(), &*pending.operation.value, &mut transferred, 0) } == 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(ERROR_IO_INCOMPLETE as i32) {
                    return Ok(None);
                }
                return Err(error.into());
            }
            pending.connected = true;
        }
        let stream = pending.stream.take().unwrap();
        *pending = Pending::new(&self.name, &self.security, false)?;
        Ok(Some(stream))
    }
}

struct Pending {
    stream: Option<Stream>,
    operation: Overlapped,
    connected: bool,
}

impl Pending {
    fn new(name: &[u16], security: &SecurityDescriptor, first: bool) -> Result<Self, DaemonError> {
        let attributes = SECURITY_ATTRIBUTES { nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: security.0, bInheritHandle: 0 };
        let flags = PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | if first { FILE_FLAG_FIRST_PIPE_INSTANCE } else { 0 };
        // SAFETY: name/descriptor remain valid for creation. Reject network
        // clients; fixed buffers and maximum instances bound kernel resources.
        let handle = owned(unsafe { CreateNamedPipeW(name.as_ptr(), flags, PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS, 64, 64 * 1024, 64 * 1024, 0, &attributes) })?;
        let mut operation = Overlapped::new()?;
        // SAFETY: boxed OVERLAPPED never moves and lives until completion/reap.
        let connected = unsafe { ConnectNamedPipe(handle.as_raw_handle(), &mut *operation.value) } != 0;
        let error = io::Error::last_os_error();
        let connected = if connected || error.raw_os_error() == Some(ERROR_PIPE_CONNECTED as i32) {
            true
        } else if error.raw_os_error() == Some(ERROR_IO_PENDING as i32) {
            false
        } else {
            return Err(error.into());
        };
        Ok(Self { stream: Some(Stream { handle, server: true }), operation, connected })
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        if !self.connected {
            if let Some(stream) = &self.stream {
                self.operation.cancel(stream.handle.as_raw_handle());
            }
        }
    }
}

struct SecurityDescriptor(PSECURITY_DESCRIPTOR);
impl SecurityDescriptor {
    fn current_user() -> Result<Self, DaemonError> {
        // SAFETY: GetCurrentProcess returns a borrowed pseudo-handle.
        let sid = user_sid(unsafe { GetCurrentProcess() })?;
        Self::from_sddl(&format!("D:P(A;;GA;;;{sid})"))
    }

    fn from_sddl(value: &str) -> Result<Self, DaemonError> {
        let sddl = wide(value);
        let mut descriptor = std::ptr::null_mut();
        // SAFETY: valid terminated SDDL and output pointer; LocalFree owns it.
        if unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.as_ptr(), SDDL_REVISION_1, &mut descriptor, std::ptr::null_mut()) } == 0 {
            return Err(io::Error::last_os_error().into());
        }
        Ok(Self(descriptor))
    }
}
impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}

fn user_sid(process: HANDLE) -> Result<String, DaemonError> {
    let mut raw = std::ptr::null_mut();
    // SAFETY: process is live/borrowed; token output is owned on success.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut raw) } == 0 {
        return Err(io::Error::last_os_error().into());
    }
    let token = owned(raw)?;
    let mut length = 0;
    // SAFETY: initial size query intentionally has no output buffer.
    unsafe {
        GetTokenInformation(token.as_raw_handle(), TokenUser, std::ptr::null_mut(), 0, &mut length);
    }
    if length == 0 {
        return Err(io::Error::last_os_error().into());
    }
    // usize storage supplies TOKEN_USER's pointer alignment; SID tail bytes are
    // retained until ConvertSidToStringSidW has copied their representation.
    let mut storage = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
    if unsafe { GetTokenInformation(token.as_raw_handle(), TokenUser, storage.as_mut_ptr().cast(), length, &mut length) } == 0 {
        return Err(io::Error::last_os_error().into());
    }
    let user = unsafe { &*storage.as_ptr().cast::<TOKEN_USER>() };
    let mut text = std::ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut text) } == 0 {
        return Err(io::Error::last_os_error().into());
    }
    let mut count = 0;
    // SAFETY: successful conversion owns a NUL-terminated LocalAlloc string.
    unsafe {
        while *text.add(count) != 0 {
            count += 1;
        }
        let sid = String::from_utf16_lossy(std::slice::from_raw_parts(text, count));
        LocalFree(text.cast());
        Ok(sid)
    }
}

fn pipe_name(scope: &DaemonScope) -> Result<Vec<u16>, DaemonError> {
    pipe_name_for(scope, false)
}

fn pipe_name_for(scope: &DaemonScope, observation: bool) -> Result<Vec<u16>, DaemonError> {
    use std::os::windows::ffi::OsStrExt;
    let mut hash = Sha256::new();
    for unit in scope.root().as_os_str().encode_wide() {
        hash.update(unit.to_le_bytes());
    }
    // SID separates users even if a caller selects a shared data directory.
    hash.update(user_sid(unsafe { GetCurrentProcess() })?.as_bytes());
    let suffix = if observation { "-observe" } else { "" };
    Ok(wide(&format!(r"\\.\pipe\agentx-{}{suffix}", super::hex_digest(&hash.finalize()))))
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn owned(handle: HANDLE) -> Result<OwnedHandle, DaemonError> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: every caller passes a newly owned OS handle, never a pseudo-handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

/// A scope selected on a redirected/shared volume must not inherit permissive
/// ACLs for lifecycle metadata. Fail closed where the filesystem lacks ACLs.
pub(super) fn private_directory(path: &std::path::Path) -> Result<(), DaemonError> {
    use std::os::windows::ffi::OsStrExt;
    let sid = user_sid(unsafe { GetCurrentProcess() })?;
    let security = SecurityDescriptor::from_sddl(&format!("D:P(A;OICI;FA;;;{sid})"))?;
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: stable terminated path and live self-relative descriptor.
    if unsafe { SetFileSecurityW(path.as_ptr(), DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION, security.0) } == 0 {
        return Err(io::Error::last_os_error().into());
    }
    Ok(())
}

/// Check the protected single-user DACL installed by scope acquisition. This
/// inspection path never invokes SetFileSecurityW or repairs an unsafe scope.
pub(super) fn validate_existing_directory(path: &std::path::Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut needed = 0;
    // SAFETY: the first call obtains the descriptor's required byte count.
    unsafe {
        GetFileSecurityW(path.as_ptr(), DACL_SECURITY_INFORMATION, std::ptr::null_mut(), 0, &mut needed);
    }
    if needed == 0 {
        return Err(io::Error::last_os_error());
    }
    // u32 storage keeps the descriptor suitably aligned for Windows APIs.
    let mut storage = vec![0u32; (needed as usize).div_ceil(4)];
    let descriptor = storage.as_mut_ptr().cast();
    // SAFETY: descriptor is owned aligned storage with at least needed bytes.
    if unsafe { GetFileSecurityW(path.as_ptr(), DACL_SECURITY_INFORMATION, descriptor, needed, &mut needed) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut control = 0;
    let mut revision = 0;
    let mut present = 0;
    let mut defaulted = 0;
    let mut dacl: *mut ACL = std::ptr::null_mut();
    // SAFETY: descriptor remains owned until all returned ACL pointers expire.
    if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0 || unsafe { GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let denied = || io::Error::new(io::ErrorKind::PermissionDenied, "scope directory lacks a protected current-user DACL");
    if control & SE_DACL_PROTECTED == 0 || present == 0 || dacl.is_null() {
        return Err(denied());
    }
    // SAFETY: GetSecurityDescriptorDacl returned an ACL within live storage.
    if unsafe { IsValidAcl(dacl) } == 0 || unsafe { (*dacl).AceCount } != 1 {
        return Err(denied());
    }
    let mut ace: *mut std::ffi::c_void = std::ptr::null_mut();
    // SAFETY: the validated ACL has exactly one ACE.
    if unsafe { GetAce(dacl, 0, &mut ace) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let ace = ace.cast::<ACCESS_ALLOWED_ACE>();
    // SAFETY: ACE storage belongs to the validated descriptor.
    // WinNT ACCESS_ALLOWED_ACE_TYPE is zero; keep its byte representation
    // here without enabling the unrelated SystemServices feature for one tag.
    if unsafe { (*ace).Header.AceType } != 0 || unsafe { (*ace).Mask } != FILE_ALL_ACCESS {
        return Err(denied());
    }
    let sid = unsafe { std::ptr::addr_of_mut!((*ace).SidStart).cast() };
    let mut encoded = std::ptr::null_mut();
    // SAFETY: the SID belongs to the validated access-allowed ACE.
    if unsafe { ConvertSidToStringSidW(sid, &mut encoded) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut length = 0;
    // SAFETY: Windows allocated a nul-terminated SID string.
    while unsafe { *encoded.add(length) } != 0 {
        length += 1;
    }
    let actual = unsafe { String::from_utf16_lossy(std::slice::from_raw_parts(encoded, length)) };
    unsafe {
        LocalFree(encoded.cast());
    }
    let expected = user_sid(unsafe { GetCurrentProcess() }).map_err(|error| io::Error::other(error.to_string()))?;
    if actual != expected {
        return Err(denied());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn local_pipe_peer_and_frame_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let scope = DaemonScope::prepare(directory.path()).unwrap();
        let lease = ScopeLease::acquire(scope.clone()).unwrap();
        let listener = Listener::bind(&lease).unwrap();
        let child = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = connect(&scope, deadline).unwrap();
            assert_eq!(peer(&stream).unwrap().0, std::process::id());
            super::super::frame::write(&mut stream, &"ping", deadline).unwrap();
            assert_eq!(super::super::frame::read::<String>(&mut stream, deadline).unwrap(), "pong");
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            assert!(listener.wait_ready(deadline).unwrap());
            if let Some(stream) = listener.accept().unwrap() {
                break stream;
            }
            assert!(Instant::now() < deadline);
        };
        assert_eq!(peer(&stream).unwrap().0, std::process::id());
        assert_eq!(super::super::frame::read::<String>(&mut stream, deadline).unwrap(), "ping");
        super::super::frame::write(&mut stream, &"pong", deadline).unwrap();
        child.join().unwrap();
    }
}
