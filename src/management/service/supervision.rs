//! Host-only worker lifecycle. Retain the loaded image across package replacement,
//! bound diagnostics and waits, and keep private execution input out of logs/receipts.
use super::*;
use std::{
    io::{Read, Write},
    path::Path,
    process::{ChildStderr, Command as ProcessCommand, Stdio},
    sync::atomic::{AtomicBool, Ordering},
};

const STDERR_LIMIT: usize = 16 * 1024;

/// A Linux deleted executable pathname cannot be used by later launcher steps.
/// Pin that same loaded image once per build, rather than selecting a new installed binary.
pub(super) fn self_executable(scope: &Path) -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|error| format!("worker executable lookup: {error}"))?;
    #[cfg(target_os = "linux")]
    if !executable.is_file() {
        let directory = scope.join(".tmp/management-hosts");
        std::fs::create_dir_all(&directory).map_err(|error| format!("worker image directory: {error}"))?;
        crate::sys::fs::set_private_dir(&directory).map_err(|error| format!("worker image permissions: {error}"))?;
        let lock_path = directory.join("image.lock");
        let lock = std::fs::OpenOptions::new().create(true).read(true).write(true).truncate(false).open(&lock_path).map_err(|error| format!("worker image lock: {error}"))?;
        crate::sys::fs::set_private(&lock_path).map_err(|error| format!("worker image lock permissions: {error}"))?;
        crate::sys::fs::lock_exclusive(&lock).map_err(|error| format!("worker image lock: {error}"))?;
        let image = directory.join(crate::daemon::build_identity().replace(':', "-"));
        if !image.is_file() {
            let temporary = directory.join(format!("{}.tmp", uuid::Uuid::new_v4()));
            let result = (|| -> std::io::Result<()> {
                let mut source = std::fs::File::open("/proc/self/exe")?;
                let mut target = crate::sys::fs::create_private_new(&temporary)?;
                std::io::copy(&mut source, &mut target)?;
                target.sync_all()?;
                use std::os::unix::fs::PermissionsExt;
                target.set_permissions(std::fs::Permissions::from_mode(0o700))?;
                std::fs::rename(&temporary, &image)
            })();
            if let Err(error) = result {
                let _ = std::fs::remove_file(&temporary);
                return Err(format!("worker image retention: {error}"));
            }
            crate::log::log_in_scope(scope, "INFO", "management", "worker.image_retained", "Retained loaded executable after pathname replacement", &[("executable", &image.to_string_lossy())]);
        }
        return Ok(image);
    }
    Ok(executable)
}

pub(super) fn run_worker(input: &WorkerInput) -> Result<(), String> {
    let executable = self_executable(Path::new(&input.scope))?;
    supervise(input, &executable, Duration::from_secs(120))
}

/// Separate seam for disposable helper executables. Never accepts a remote executable.
pub(super) fn supervise(input: &WorkerInput, executable: &Path, timeout: Duration) -> Result<(), String> {
    let bytes = serde_json::to_vec(input).map_err(|error| format!("worker input encoding: {error}"))?;
    let context = format!("executable={} cwd={}", executable.display(), input.invocation.cwd.display());
    let mut child = ProcessCommand::new(executable)
        .arg("__management-worker")
        .env_clear()
        .envs(&input.invocation.environment)
        .env("AGENTX_DIR", &input.scope)
        .current_dir(&input.invocation.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("worker spawn ({context}): {error}"))?;
    let pid = child.id().to_string();
    crate::log::log_in_scope(
        Path::new(&input.scope),
        "INFO",
        "management",
        "worker.started",
        "Execution host spawned",
        &[("generation", &input.generation), ("operation_id", &input.request_id), ("worker_pid", &pid), ("executable", &executable.to_string_lossy()), ("cwd", &input.invocation.cwd.to_string_lossy())],
    );
    let capture = match child.stderr.take().ok_or_else(|| "worker stderr unavailable".to_string()).and_then(StderrCapture::new) {
        Ok(capture) => capture,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    // Cancellation also covers a descendant retaining an unread stdin pipe.
    let mut stdin = child.stdin.take().ok_or_else(|| "worker stdin unavailable".to_owned())?;
    #[cfg(unix)]
    if let Err(error) = make_nonblocking(&stdin) {
        let _ = child.kill(); let _ = child.wait(); capture.finish();
        return Err(format!("worker input setup: {error}"));
    }
    let input_stop = Arc::new(AtomicBool::new(false));
    let writer_stop = input_stop.clone();
    let writer = std::thread::Builder::new().name("management-input".into()).spawn(move || {
        let mut offset = 0;
        while offset < bytes.len() {
            if writer_stop.load(Ordering::Acquire) { return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "input cancelled before completion")) }
            match stdin.write(&bytes[offset..]) {
                Ok(0) => return Err(std::io::ErrorKind::WriteZero.into()),
                Ok(count) => offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(20)),
                Err(error) => return Err(error),
            }
        }
        Ok(())
    });
    let writer = match writer {
        Ok(writer) => writer,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            capture.finish();
            return Err(format!("worker input thread: {error}"));
        }
    };
    let started = std::time::Instant::now();
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => break if status.success() { Ok(()) } else { Err(format!("worker exited ({status}; {context}); inspect original operation before retry")) },
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(format!("worker wait ({context}): {error}"));
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(format!("worker timeout after {}ms ({context}); inspect original operation before retry", timeout.as_millis()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    input_stop.store(true, Ordering::Release);
    #[cfg(windows)]
    if !writer.is_finished() {
        use std::os::windows::io::AsRawHandle;
        // SAFETY: live writer-thread handle, cancelling only its synchronous pipe IO.
        unsafe { windows_sys::Win32::System::IO::CancelSynchronousIo(writer.as_raw_handle() as _) };
    }
    let written = writer.join().map_err(|_| "worker input thread panicked".to_string()).and_then(|result| result.map_err(|error| format!("worker input write ({context}): {error}")));
    let stderr = capture.finish();
    if !stderr.is_empty() {
        // Diagnostics stay in the private rotated host log, never public receipts.
        // Redact captured credential values if a subprocess echoes one.
        let mut diagnostic = String::from_utf8_lossy(&stderr).into_owned();
        for (key, value) in &input.invocation.environment {
            let key = key.to_ascii_uppercase();
            if value.len() >= 4 && ["TOKEN", "SECRET", "PASSWORD", "AUTH", "COOKIE", "API_KEY"].iter().any(|part| key.contains(part)) {
                diagnostic = diagnostic.replace(value, "[redacted]");
            }
        }
        crate::log::log_in_scope(Path::new(&input.scope), if result.is_err() { "ERROR" } else { "WARN" }, "management", "worker.stderr", &diagnostic, &[("generation", &input.generation), ("operation_id", &input.request_id), ("worker_pid", &pid)]);
    }
    result.and(written)
}

struct StderrCapture {
    stop: Arc<AtomicBool>,
    reader: std::thread::JoinHandle<Vec<u8>>,
}
impl StderrCapture {
    fn new(mut stderr: ChildStderr) -> Result<Self, String> {
        #[cfg(unix)]
        {
            make_nonblocking(&stderr).map_err(|error| format!("worker stderr setup: {error}"))?;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let cancelled = stop.clone();
        let reader = std::thread::Builder::new()
            .name("management-stderr".into())
            .spawn(move || {
                let mut tail = Vec::new();
                let mut bytes = [0u8; 4096];
                let mut drained_after_stop = 0;
                loop {
                    match read_available(&mut stderr, &mut bytes) {
                        Ok(0) => break,
                        Ok(count) => {
                            tail.extend_from_slice(&bytes[..count]);
                            if tail.len() > STDERR_LIMIT {
                                tail.drain(..tail.len() - STDERR_LIMIT);
                            }
                            if cancelled.load(Ordering::Acquire) {
                                drained_after_stop += count;
                                // Drain the completed host's pipe tail, but never
                                // follow a continuously writing descendant forever.
                                if drained_after_stop >= 1024 * 1024 { break }
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if cancelled.load(Ordering::Acquire) {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(20));
                        }
                        Err(_) => break,
                    }
                }
                tail
            })
            .map_err(|error| format!("worker stderr thread: {error}"))?;
        Ok(Self { stop, reader })
    }
    fn finish(self) -> Vec<u8> {
        self.stop.store(true, Ordering::Release);
        self.reader.join().unwrap_or_default()
    }
}

#[cfg(unix)]
fn make_nonblocking(pipe: &impl std::os::fd::AsRawFd) -> std::io::Result<()> {
    let fd = pipe.as_raw_fd();
    // SAFETY: owned pipe descriptor; preserve existing status flags.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn read_available(stderr: &mut ChildStderr, bytes: &mut [u8]) -> std::io::Result<usize> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        let mut available = 0;
        // SAFETY: valid child pipe; only the available-byte output is supplied.
        let ok = unsafe { windows_sys::Win32::System::Pipes::PeekNamedPipe(stderr.as_raw_handle() as _, std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut available, std::ptr::null_mut()) };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        if available == 0 {
            return Err(std::io::ErrorKind::WouldBlock.into());
        }
        let count = bytes.len().min(available as usize);
        return stderr.read(&mut bytes[..count]);
    }
    #[cfg(not(windows))]
    stderr.read(bytes)
}
