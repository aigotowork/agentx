//! Scoped tmux hosting. AgentX retains identity and delivery; tmux owns terminal lifetime.
//! See `.docs/architecture/terminal-sessions.md` for configuration and ownership.
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use crate::config::AgentxConfig;

const DEFAULT_CONFIG: &str = "# AgentX managed defaults; use tmux_config for custom settings.\nset -g history-limit 100000\nset -g mouse on\n";

pub(crate) struct Settings {
    socket: PathBuf,
    config: PathBuf,
    managed_config: bool,
}

impl Settings {
    pub(crate) fn resolve(scope: &Path, config: &AgentxConfig) -> Result<Self> {
        if !cfg!(unix) {
            bail!("tmux hosting requires Unix; use --terminal pty for native headless execution");
        }
        let resolve = |value: &str| {
            let path = Path::new(value);
            if path.is_absolute() { path.to_owned() } else { scope.join(path) }
        };
        let socket = resolve(if config.tmux_socket.is_empty() { ".tmp/tmux/socket" } else { &config.tmux_socket });
        // Leave space for sockaddr_un's terminating NUL on supported Unix hosts.
        if socket.as_os_str().len() > 100 {
            bail!("tmux socket path is too long; configure tmux_socket with a shorter absolute path");
        }
        let conf = if config.tmux_config.is_empty() { scope.join(".tmp/tmux/default.conf") } else { resolve(&config.tmux_config) };
        if !config.tmux_config.is_empty() && !conf.is_file() {
            bail!("tmux config file does not exist: {}", conf.display());
        }
        Ok(Self { socket, config: conf, managed_config: config.tmux_config.is_empty() })
    }

    fn command(&self) -> Result<Command> {
        let binary = super::terminal::which_bin("tmux").context("tmux is not installed; install it or select --terminal pty")?;
        let mut command = Command::new(binary);
        command.arg("-S").arg(&self.socket).arg("-f").arg(&self.config).env_remove("TMUX").env_remove("TMUX_PANE");
        Ok(command)
    }
}

/// Run only short tmux control operations with a bounded wait, never a live client.
fn control(command: &mut Command) -> Result<Output> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().context("could not execute tmux")?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait()?.is_some() {
            let output = child.wait_with_output()?;
            if !output.status.success() {
                bail!("tmux: {}", String::from_utf8_lossy(&output.stderr).trim());
            }
            return Ok(output);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("tmux control command timed out");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Start one uniquely named detached session and return its actual pane process.
pub(crate) fn launch(settings: &Settings, script: &str, process_id: &str, cwd: &str) -> Result<u32> {
    let parent = settings.socket.parent().context("tmux socket needs a parent directory")?;
    let new_directory = !parent.exists();
    std::fs::create_dir_all(parent)?;
    #[cfg(unix)]
    if new_directory {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    let _ = new_directory;
    if settings.managed_config {
        std::fs::create_dir_all(settings.config.parent().unwrap())?;
        crate::paths::atomic_write_io(&settings.config, DEFAULT_CONFIG)?;
    }
    let session = format!("agentx-{}", uuid::Uuid::new_v4().simple());
    let bash = super::terminal::which_bin("bash").context("bash is required for the AgentX runner")?;
    let mut command = settings.command()?;
    command.args(["new-session", "-d", "-P", "-F", "AGENTX_TMUX_PID:#{pane_pid}", "-s", &session, "-c", cwd, "--"]);
    command.arg(bash).arg(script);
    // The runner has a captured environment; these identify the selected host before PTY startup.
    command.env("AGENTX_LAUNCHED_PRESET", "tmux");
    let result = control(&mut command)
        .and_then(|output| String::from_utf8_lossy(&output.stdout).lines().find_map(|line| line.strip_prefix("AGENTX_TMUX_PID:").and_then(|pid| pid.parse::<u32>().ok()).filter(|pid| *pid > 0)).context("tmux did not return its pane process ID"));
    let pid = match result {
        Ok(pid) => pid,
        Err(error) => {
            // The exact random session name belongs to this attempted launch only.
            if let Ok(mut cleanup) = settings.command() {
                let _ = control(cleanup.args(["kill-session", "-t", &session]));
            }
            return Err(error);
        }
    };
    crate::log::log_info("terminal", "tmux.started", &format!("process_id={process_id} session={session} pid={pid}"));
    Ok(pid)
}

/// Resolve only a still-existing recorded server/pane, never the default server.
fn recorded_session(info: &super::terminal::TerminalInfo) -> Result<(Settings, String)> {
    if !matches!(info.preset_name.as_str(), "tmux" | "tmux-split") {
        bail!("this Agent has no tmux session; launch with --headless --terminal tmux");
    }
    let mut metadata = info.tmux.rsplitn(3, ',');
    let _session_index = metadata.next();
    let server_pid = metadata.next().filter(|value| value.parse::<u32>().is_ok()).context("tmux server metadata is not available yet")?;
    let socket = metadata.next().filter(|value| !value.is_empty()).context("tmux socket metadata is not available yet")?;
    let pane = if info.pane_id.is_empty() { &info.terminal_id } else { &info.pane_id };
    if !pane.strip_prefix('%').is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit())) {
        bail!("tmux pane metadata is not available yet; the Agent may still be starting");
    }
    let settings = Settings { socket: socket.into(), config: "/dev/null".into(), managed_config: false };
    let output = control(settings.command()?.args(["display-message", "-p", "-t", pane, "#{pid}\t#{session_id}\t#{pane_id}"]))?;
    let fields: Vec<_> = std::str::from_utf8(&output.stdout)?.trim().split('\t').collect();
    if fields.len() != 3 || fields[0] != server_pid || fields[2] != pane || !fields[1].starts_with('$') {
        bail!("the recorded tmux server or pane has exited; refusing to attach to a replacement");
    }
    Ok((settings, fields[1].to_owned()))
}

/// Capture startup diagnostics from the same validated host used for attachment.
pub(crate) fn capture(info: &super::terminal::TerminalInfo) -> Result<String> {
    let (settings, _) = recorded_session(info)?;
    let pane = if info.pane_id.is_empty() { &info.terminal_id } else { &info.pane_id };
    let output = control(settings.command()?.args(["capture-pane", "-p", "-t", pane]))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Prepare an interactive client without recreating an exited session.
pub(crate) fn attach_command(info: &super::terminal::TerminalInfo) -> Result<Command> {
    let (settings, session) = recorded_session(info)?;
    let mut command = settings.command()?;
    if let Ok(current) = std::env::var("TMUX") {
        let recorded_server = info.tmux.rsplit_once(',').map(|(server, _)| server);
        let invoking_server = current.rsplit_once(',').map(|(server, _)| server);
        if invoking_server != recorded_server {
            bail!("detach from the current tmux server before attaching to this Agent");
        }
        command.args(["switch-client", "-t", &session]).env("TMUX", current);
    } else {
        command.args(["attach-session", "-t", &session]);
    }
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn socket_and_config_are_scoped_and_explicit() {
        let scope = tempfile::tempdir().unwrap();
        let config = AgentxConfig::default();
        let settings = Settings::resolve(scope.path(), &config).unwrap();
        assert_eq!(settings.socket, scope.path().join(".tmp/tmux/socket"));
        assert_eq!(settings.config, scope.path().join(".tmp/tmux/default.conf"));
        assert!(!settings.config.exists(), "resolving configuration must not create runtime files");
        let config = AgentxConfig { tmux_config: "missing.conf".into(), ..config };
        assert!(Settings::resolve(scope.path(), &config).is_err());
    }

    #[test]
    fn unrelated_or_incomplete_terminal_cannot_attach() {
        assert!(attach_command(&super::super::terminal::TerminalInfo::default()).is_err());
        let info = super::super::terminal::TerminalInfo { preset_name: "tmux".into(), tmux: "/tmp/socket,123,0".into(), pane_id: "-a".into(), ..Default::default() };
        assert!(attach_command(&info).is_err());
    }
}
