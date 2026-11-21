//! Codex process hosting on inherited standard streams. See the native stdio
//! contract in `src/README.md`; this module never reads or writes protocol bytes.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;

struct NativeCodex {
    program: String,
    prefix: Vec<String>,
}

static NATIVE: OnceLock<NativeCodex> = OnceLock::new();

/// Resolve once after project environment preparation, before hook preflight.
/// Never allow a symlink or hardlink back to the running agentx executable.
pub(crate) fn pin() -> Result<()> {
    if NATIVE.get().is_some() {
        return Ok(());
    }
    let resolved = crate::runtime::terminal::which_bin("codex").context("'codex' is not installed or not in PATH")?;
    let resolved = Path::new(&resolved).canonicalize().context("Cannot resolve native Codex executable")?;
    let current = std::env::current_exe()?.canonicalize()?;
    let same = resolved == current;
    #[cfg(unix)]
    let same = {
        use std::os::unix::fs::MetadataExt;
        let a = resolved.metadata()?;
        let b = current.metadata()?;
        same || (a.dev() == b.dev() && a.ino() == b.ino())
    };
    if same {
        bail!("Native Codex resolves to agentx itself; refusing recursive launch");
    }
    let resolved = resolved.to_string_lossy().into_owned();
    #[cfg(windows)]
    let launcher = crate::runtime::terminal::resolve_windows_tool_launcher("codex", &resolved);
    #[cfg(not(windows))]
    let launcher = None;
    let (program, prefix) = launcher.or_else(|| crate::runtime::terminal::resolve_termux_tool_launcher("codex", &resolved)).unwrap_or((resolved, Vec::new()));
    let _ = NATIVE.set(NativeCodex { program, prefix });
    Ok(())
}

/// Stdio launches share this pinned command with hook version and preflight.
/// Other launch modes keep their existing executable resolution behavior.
pub(crate) fn pinned_command() -> Option<Command> {
    NATIVE.get().map(|native| {
        let mut command = Command::new(&native.program);
        command.args(&native.prefix);
        command
    })
}

/// Run Codex directly, forwarding termination and returning its exit status.
/// The launcher owns identity preparation and finalization around this call.
pub(crate) fn run(args: &[String], env: &HashMap<String, String>, cwd: &str) -> Result<i32> {
    pin()?;
    let mut command = pinned_command().expect("native Codex was pinned");
    command.args(args).env_clear().envs(env).current_dir(cwd).stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit());

    #[cfg(unix)]
    let status = wait_with_signals(&mut command)?;
    #[cfg(not(unix))]
    let status = command.status().context("Failed to run Codex on stdio")?;
    Ok(exit_code(status))
}

fn exit_code(status: ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(1))
    }
    #[cfg(not(unix))]
    {
        status.code().unwrap_or(1)
    }
}

/// Keep the supervisor available for DB cleanup while forwarding caller signals
/// to Codex and its subprocesses. Standard streams remain inherited throughout.
#[cfg(unix)]
fn wait_with_signals(command: &mut Command) -> Result<ExitStatus> {
    use nix::sys::signal::{Signal, killpg};
    use nix::unistd::Pid;
    use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
    use signal_hook::iterator::Signals;

    let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP, SIGQUIT])?;
    let handle = signals.handle();
    crate::sys::process::detach_session(command);

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        let parent = std::process::id() as libc::pid_t;
        // SAFETY: only async-signal-safe libc calls run between fork and exec.
        // The parent check closes the race where it died before prctl ran.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() != parent {
                    libc::_exit(128 + libc::SIGKILL);
                }
                Ok(())
            });
        }
    }

    let mut child = command.spawn().context("Failed to spawn Codex on stdio")?;
    let group = Pid::from_raw(child.id() as i32);
    let forwarder = std::thread::spawn(move || {
        for signal in signals.forever() {
            if let Ok(signal) = Signal::try_from(signal) {
                let _ = killpg(group, signal);
            }
        }
    });
    let status = child.wait();
    if status.is_err() {
        let _ = killpg(group, Signal::SIGKILL);
        let _ = child.wait();
    }
    handle.close();
    let _ = forwarder.join();
    status.context("Failed to wait for Codex on stdio")
}
