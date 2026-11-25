//! Native scope-daemon CLI entry, plus the distinct historical Relay worker CLI.
//!
//! Accessed via `agentx relay daemon [start|stop|restart|status]`.
//! Manages the `agentx relay-worker` background process for MQTT relay.

use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "agentx daemon",
    about = "Inspect or explicitly restart the selected native daemon",
    after_help = "Scope: --scope overrides AGENTX_DIR; otherwise the normal home scope is used.\nOrdinary commands auto-start a missing daemon. Restart preserves managed data. After the one-time IPC upgrade, reopen Agents still running older hosts.\nSee .docs/architecture/daemon.md for recovery and platform limits."
)]
struct NativeArgs {
    #[command(subcommand)]
    command: NativeCommand,
}

#[derive(Subcommand)]
enum NativeCommand {
    /// Serve the HTTPS-proxied cloud deployment and scope communication API
    Cloud(crate::cloud::Serve),
    /// Enroll the selected local scope daemon into a cloud namespace
    Connect(crate::cloud::connection::Connect),
    /// Enable the daemon's embedded Agent management API and print its URL
    Web {
        #[arg(long)]
        scope: Option<std::path::PathBuf>,
        /// Default working directory for Agents launched from the Web
        #[arg(long)]
        project: Option<std::path::PathBuf>,
        #[arg(long)]
        host: Option<std::net::IpAddr>,
        #[arg(long)]
        port: Option<u16>,
    },
    /// Read state and compatibility without starting a daemon or resolving identity
    Status {
        /// Selected data scope; defaults to AGENTX_DIR or the normal home scope
        #[arg(long)]
        scope: Option<std::path::PathBuf>,
    },
    /// Gracefully stop the verified scope owner and acquire this executable
    Restart {
        /// Selected data scope; defaults to AGENTX_DIR or the normal home scope
        #[arg(long)]
        scope: Option<std::path::PathBuf>,
    },
}

pub(crate) fn native_help() -> String {
    help_command().render_long_help().to_string()
}
pub(crate) fn help_command() -> clap::Command {
    use clap::CommandFactory;
    NativeArgs::command()
}

/// Called before project environment/config/identity/dev-root setup. The
/// internal serve entry alone continues to main's persistence constructor.
pub(crate) fn dispatch_native(argv: &[String]) -> Option<i32> {
    let args = match argv.first().map(String::as_str) {
        Some("daemon") if argv.get(1).map(String::as_str) == Some("serve") => return None,
        Some("daemon") => argv[1..].to_vec(),
        Some("help") if argv.get(1).map(String::as_str) == Some("daemon") => {
            let mut args = argv[2..].to_vec();
            args.push("--help".into());
            args
        }
        _ => return None,
    };
    let parsed = match NativeArgs::try_parse_from(std::iter::once("agentx daemon".to_owned()).chain(args)) {
        Ok(parsed) => parsed,
        Err(error) => {
            let code = error.exit_code();
            let _ = crate::cli::commands::help::print_clap_error(&error);
            return Some(code);
        }
    };
    Some(match run_native(parsed) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("agentx daemon: {error}");
            1
        }
    })
}

fn run_native(args: NativeArgs) -> Result<i32, crate::daemon::DaemonError> {
    use crate::daemon::lifecycle::{self, State};
    let cwd = std::env::current_dir()?;
    let environment = std::env::vars().collect();
    let default_scope = crate::paths::resolve_agentx_dir_from_env(&environment, &cwd).0;
    match args.command {
        NativeCommand::Cloud(config) => crate::cloud::serve_config(config).map(|()| 0).map_err(|error| crate::daemon::DaemonError::Unavailable(error.to_string())),
        NativeCommand::Connect(config) => {
            let scope = crate::daemon::DaemonScope::prepare(config.scope.as_deref().unwrap_or(&default_scope))?;
            let configure_only = config.configure_only;
            config.save(scope.root(), &cwd).map_err(|e| crate::daemon::DaemonError::Protocol(e.to_string()))?;
            if configure_only {
                println!("Cloud configuration saved for {}. Start or restart its service to activate it.", scope.root().display());
                return Ok(0);
            }
            let info = lifecycle::restart(scope.root())?;
            println!("Cloud configuration saved for {} (daemon instance {}). Check agentx daemon status for connection readiness.", scope.root().display(), info.daemon_instance);
            Ok(0)
        }
        NativeCommand::Status { scope } => {
            let status = lifecycle::status(scope.as_deref().unwrap_or(&default_scope), &cwd)?;
            println!("Scope: {}", status.scope.display());
            let state = match status.state {
                State::Stopped => "stopped",
                State::Running => "running",
                State::Incompatible => "running-incompatible",
                State::Unavailable => "unavailable",
            };
            println!("Daemon: {state}");
            if let Some(cloud) = crate::cloud::connection::describe(&status.scope, status.info.as_ref().map(|info| info.daemon_instance.as_str())) {
                println!("Cloud: {cloud}");
            }
            println!("Client: version={} cohort={}", env!("CARGO_PKG_VERSION"), env!("AGENTX_NATIVE_BUILD_ID"));
            if let Some((pid, incarnation)) = status.peer {
                println!("Peer: pid={pid} incarnation={}", incarnation.as_deref().unwrap_or("unavailable"));
            }
            if let Some(ref info) = status.info {
                println!("Daemon: version={} cohort={} instance={} generation={}", info.build_version, info.build_id, info.daemon_instance, info.database_generation);
            }
            if let Some(detail) = status.detail {
                println!("Detail: {detail}");
            }
            if let Some(endpoint) = crate::management::web::endpoint(&status.scope)
                && status.info.as_ref().is_some_and(|info| endpoint.daemon_instance == info.daemon_instance)
            {
                println!("Management: {}\nProject: {}", endpoint.url, endpoint.project);
            }
            Ok(if status.state == State::Unavailable { 1 } else { 0 })
        }
        NativeCommand::Restart { scope } => {
            let (info, upgraded) = lifecycle::restart_with_upgrade(scope.as_deref().unwrap_or(&default_scope))?;
            println!("Scope: {}\nDaemon: running (pid={} instance={})\nBuild: version={} cohort={}", info.canonical_scope.display(), info.pid, info.daemon_instance, info.build_version, info.build_id);
            if upgraded {
                println!("Native IPC upgraded. Reopen Agents still running an older host once so they can poll this daemon.");
            }
            Ok(0)
        }
        NativeCommand::Web { scope, project, host, port } => {
            let scope = crate::daemon::DaemonScope::prepare(scope.as_deref().unwrap_or(&default_scope))?;
            let project = project.unwrap_or(cwd).canonicalize()?;
            if !project.is_dir() {
                return Err(crate::daemon::DaemonError::Protocol("project must be an existing directory".into()));
            }
            crate::management::web::configure(scope.root(), &project, host, port).map_err(crate::daemon::DaemonError::Protocol)?;
            let info = crate::daemon::acquire(&scope)?;
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            loop {
                if let Some(endpoint) = crate::management::web::endpoint(scope.root())
                    && endpoint.daemon_instance == info.daemon_instance
                    && endpoint.project == project.to_string_lossy()
                    && host.is_none_or(|host| host == endpoint.bind_host)
                    && port.is_none_or(|port| port == endpoint.bind_port)
                {
                    println!("{}", endpoint.url);
                    return Ok(0);
                }
                if std::time::Instant::now() >= deadline {
                    return Err(crate::daemon::DaemonError::Unavailable("management endpoint not ready; inspect .daemon/stderr.log and explicitly restart an older daemon if needed".into()));
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

pub(crate) fn daemon_status() -> i32 {
    match crate::relay::worker::relay_worker_pid() {
        Some(pid) => println!("Daemon: running (PID {pid})"),
        None => println!("Daemon not running"),
    }
    0
}

pub(crate) fn daemon_start() -> i32 {
    let was_running = crate::relay::worker::is_relay_worker_running();
    if crate::relay::worker::ensure_worker() {
        let pid = crate::relay::worker::relay_worker_pid();
        let pid_str = pid.map(|p| format!(" (PID {p})")).unwrap_or_default();
        if was_running {
            println!("Daemon already running{pid_str}");
        } else {
            println!("Daemon started{pid_str}");
        }
        0
    } else {
        // ensure_worker may have timed out on readiness — check if process actually started
        if crate::relay::worker::is_relay_worker_running() {
            let pid = crate::relay::worker::relay_worker_pid();
            let pid_str = pid.map(|p| format!(" (PID {p})")).unwrap_or_default();
            println!("Daemon started{pid_str} (notify port not yet ready)");
            0
        } else {
            if crate::sys::process::is_sandboxed() {
                eprintln!("Failed to start daemon: running inside a sandbox, which the daemon would inherit. Rerun with approval/escalation.");
            } else {
                eprintln!("Failed to start daemon (relay disabled or config error)");
            }
            1
        }
    }
}

pub(crate) fn daemon_stop() -> i32 {
    let pid = match crate::relay::worker::relay_worker_pid() {
        Some(p) => p,
        None => {
            println!("Daemon not running");
            return 0;
        }
    };

    crate::sys::process::terminate(pid);
    println!("Requested daemon shutdown (PID {pid})");

    for _ in 0..50 {
        thread::sleep(Duration::from_millis(100));
        if !crate::runtime::process_tracking::is_alive(pid) {
            println!("Daemon stopped");
            crate::relay::worker::remove_relay_pid_file();
            return 0;
        }
    }

    println!("Daemon did not exit in time, forcing termination");
    if !crate::sys::process::kill(pid) {
        eprintln!("Force-kill failed (errno {}), PID file retained", std::io::Error::last_os_error());
        return 1;
    }
    println!("Daemon killed");
    crate::relay::worker::remove_relay_pid_file();
    0
}

pub fn cmd_daemon(argv: &[String]) -> i32 {
    let subcmd = argv.first().map(|s| s.as_str()).unwrap_or("status");

    match subcmd {
        "status" => daemon_status(),
        "start" => daemon_start(),
        "stop" => daemon_stop(),
        "restart" => {
            daemon_stop();
            thread::sleep(Duration::from_millis(500));
            daemon_start()
        }
        other => crate::cli::commands::help::unknown_subcommand("relay daemon", other, &["status", "start", "stop", "restart"]),
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;

    #[test]
    fn native_grammar_keeps_lifecycle_separate_from_relay_and_identity() {
        let parsed = NativeArgs::try_parse_from(["agentx daemon", "status"]).unwrap();
        assert!(matches!(parsed.command, NativeCommand::Status { scope: None }));
        let parsed = NativeArgs::try_parse_from(["agentx daemon", "restart", "--scope", "selected"]).unwrap();
        assert!(matches!(parsed.command, NativeCommand::Restart { scope: Some(path) } if path == std::path::Path::new("selected")));
        let parsed = NativeArgs::try_parse_from(["agentx daemon", "web", "--scope", "selected", "--project", "/project", "--host", "127.0.0.1", "--port", "18749"]).unwrap();
        assert!(matches!(parsed.command, NativeCommand::Web { scope: Some(scope), project: Some(project), host: Some(host), port: Some(18749) }
            if scope == std::path::Path::new("selected") && project == std::path::Path::new("/project") && host.is_loopback()));
        for args in [["agentx daemon", "stop"], ["agentx daemon", "start"]] {
            assert!(NativeArgs::try_parse_from(args).is_err());
        }
        assert!(NativeArgs::try_parse_from(["agentx daemon", "status", "--name", "actor"]).is_err());
        assert!(dispatch_native(&["daemon".into(), "serve".into(), "--scope".into(), "/scope".into()]).is_none());
    }

    #[test]
    fn native_help_is_available_without_scope_or_participant() {
        let text = native_help();
        assert!(text.contains("status"));
        assert!(text.contains("restart"));
        assert!(text.contains("AGENTX_DIR"));
        let error = NativeArgs::try_parse_from(["agentx daemon", "status", "--help"]).err().unwrap();
        assert_eq!(error.exit_code(), 0);
        assert!(error.to_string().contains("--scope"));
    }
}
