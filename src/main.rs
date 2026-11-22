//! agentx — inter-agent communication for AI coding tools.
//!
//! Humans usually launch agents with `agentx launch --tool <tool>` and talk to them through
//! each tool's own UI. Agents use `agentx` CLI commands (learnt through bootstrap.rs)
//! as a side-channel for messaging and coordination with other agents.

pub mod agents;
mod browser;
pub mod cli;
mod cloud;
mod collaboration;
mod config;
mod daemon;
mod db;
pub mod events;
mod file_bindings;
mod inspection;
pub mod integrations;
mod log;
mod management;
pub mod messaging;
mod observability;
mod paths;
pub mod relay;
pub mod runtime;
mod service;
pub mod shared;
mod sys;
pub mod transcript;
mod tui;
pub mod workflows;

use crate::cli::router;
use crate::integrations::tool;
use crate::messaging::delivery;
use crate::runtime::{pty, terminal};

use anyhow::{Context, Result, bail};
use std::panic;
use std::str::FromStr;

fn main() -> Result<()> {
    let started = std::time::Instant::now();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if let Some(result) = cloud::dispatch(&argv) {
        return result;
    }
    #[cfg(target_os = "linux")]
    if argv.first().is_some_and(|value| value == "__product-execution") {
        return crate::integrations::codex::product_execution::serve(&argv[1..]);
    }
    if crate::integrations::hooks::codex::dispatch_managed_entry(&argv, started) {
        return Ok(());
    }
    let _trace = argv.first().and_then(|event| {
        let provider = match tool::Tool::from_hook_name(event) {
            Some(tool::Tool::Codex) => "codex",
            Some(tool::Tool::Claude) if event != "claude-mod-poll" => "claude",
            _ => return None,
        };
        observability::trace::HookGuard::begin(started, provider, event)
    });
    if let Some(code) = browser::openapi::dispatch(&argv) {
        std::process::exit(code);
    }
    let argv = cli::commands::launch::normalize_selection_args(&argv);
    if let Some(code) = management::worker::dispatch(&argv) {
        std::process::exit(code);
    }
    if let Some(code) = cli::commands::help::dispatch_all(&argv) {
        std::process::exit(code);
    }
    // Authorized Codex stdio direnv preparation may replace this process and
    // must happen before dotenv selection. It does not pin the Provider
    // executable; that happens only after the complete effective environment
    // is available to the launch path.
    let argv = if cli::commands::launch::is_dry_run(&argv) { argv } else { runtime::codex_project_env::prepare(argv)? };
    if !runtime::environment::is_lightweight(&argv) {
        observability::trace::time(observability::trace::Phase::Environment, || runtime::environment::prepare(&argv))?;
    }
    let (argv, _) = runtime::environment::extract_options(&argv)?;
    if let Some(result) = cloud::dispatch_scope_client(&argv) {
        return result;
    }
    if cli::commands::launch::stdio_project_request(&argv)?.is_some() {
        runtime::codex_stdio::pin()?;
    }
    if let Some(code) = router::dispatch_passive_observation(&argv) {
        if code != 0 {
            std::process::exit(code);
        }
        return Ok(());
    }
    if let Some(scope) = daemon::entry_scope(&argv)? {
        let database_path = scope.root().join("agentx.db");
        return daemon::serve(scope, move |generation| {
            let db = db::AgentxDb::open_at(&database_path).map_err(|error| service::ServiceError::Internal(format!("{error:#}")))?;
            service::application::SqliteApplication::new(db, generation)
        })
        .map_err(Into::into);
    }
    // Hooks of an agent nested under an agentx-launched one must not act as it.
    // Runs before configuration: later readers see only the child's view.
    let nested = observability::trace::time(observability::trace::Phase::HookContext, || argv.first().and_then(|arg| tool::Tool::from_hook_name(arg)).and_then(crate::integrations::nested_env::scrub_inherited_identity));

    if std::env::args().nth(1).as_deref() == Some("relay-worker") {
        relay::worker::scrub_inherited_env();
    }

    // Initialize global config from environment variables
    config::Config::init();

    if let Some(nested) = nested {
        log::log_info("identity", "nested_agent.scrubbed", &format!("child={} parent={}: ignoring inherited agentx identity", nested.child, nested.parent));
    }

    // Set custom panic hook to log to file instead of stderr (prevents TUI corruption)
    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_else(|| "unknown".to_string());
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };
        log::log_error("native", "panic", &format!("{} at {}", message, location));
    }));

    // Dispatch via router (replaces manual MainAction matching)
    router::dispatch(&argv)
}

/// Run PTY wrapper mode.
///
/// Uses Unix pseudo-terminals on Unix and ConPTY (via `portable-pty`) on
/// Windows; the proxy backend is selected inside `pty::Proxy`.
pub fn run_pty(args: &[String]) -> Result<()> {
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        eprintln!("agentx pty - PTY wrapper for agentx");
        eprintln!();
        eprintln!("Usage: agentx pty <tool> [args...]");
        eprintln!();
        let tools =
            crate::integrations::registry::ALL.iter().filter(|spec| spec.released).map(|spec| if spec.aliases.is_empty() { spec.name.to_string() } else { format!("{} ({})", spec.name, spec.aliases.join(", ")) }).collect::<Vec<_>>().join(", ");
        eprintln!("Tools: {tools}");
        eprintln!();
        eprintln!("The PTY wrapper provides:");
        eprintln!("  - Text injection via TCP port (INJECT_PORT)");
        eprintln!("  - State queries via TCP port (STATE_PORT)");
        eprintln!("  - Ready detection for tool startup");
        eprintln!();
        eprintln!("Environment:");
        eprintln!("  AGENTX_INSTANCE_NAME    Instance name for logging");
        eprintln!("  AGENTX_DIR              Custom agentx directory");
        if args.is_empty() {
            bail!("Tool name required");
        }
        return Ok(());
    }

    let tool_str = &args[0];

    // Generated runners pin the tool resolved from the caller's PATH before
    // installing their runtime-first PATH. Windows runner scripts additionally
    // pass tool args via a JSON sidecar because the PowerShell → native-exe
    // boundary corrupts arguments with embedded double quotes.
    let mut arg_index = 1;
    let tool_path = if args.get(arg_index).map(String::as_str) == Some("--agentx-tool-path") {
        let Some(path) = args.get(arg_index + 1) else {
            bail!("--agentx-tool-path requires a path");
        };
        arg_index += 2;
        Some(path.as_str())
    } else {
        None
    };

    let sidecar_args: Vec<String>;
    let tool_args: Vec<&str> = if args.get(arg_index).map(String::as_str) == Some("--agentx-args-file") {
        let Some(path) = args.get(arg_index + 1) else {
            bail!("--agentx-args-file requires a path");
        };
        let content = std::fs::read_to_string(path).with_context(|| format!("Failed to read args file {path}"))?;
        let _ = std::fs::remove_file(path);
        sidecar_args = serde_json::from_str(&content).with_context(|| format!("Invalid JSON in args file {path}"))?;
        sidecar_args.iter().map(|s| s.as_str()).collect()
    } else {
        args[arg_index..].iter().map(|s| s.as_str()).collect()
    };

    // Keep arbitrary commands explicit so they cannot inherit a known tool's
    // delivery behavior merely because parsing failed.
    let (ready_patterns, target) = match tool::Tool::from_str(tool_str) {
        Ok(tool) => (tool.ready_patterns().iter().map(|p| p.to_string()).collect(), pty::PtyTarget::Known(tool)),
        Err(_) => (vec![], pty::PtyTarget::AdhocCommand(tool_str.to_string())),
    };

    let instance_name = config::Config::get().instance_name;

    // Resolve tool to full path (PATH may be minimal in launched environments)
    let tool_exe = tool_str.parse::<tool::Tool>().map(|t| t.spec().cli_binary).unwrap_or(tool_str);
    let resolved = tool_path.map(ToString::to_string).or_else(|| terminal::which_bin(tool_exe)).unwrap_or_else(|| tool_exe.to_string());

    // On Termux, some wrapped tools need a launcher override instead of direct exec.
    let (command, extra_args): (String, Vec<String>);
    #[cfg(windows)]
    let windows_launcher = terminal::resolve_windows_tool_launcher(tool_exe, &resolved);
    #[cfg(not(windows))]
    let windows_launcher: Option<(String, Vec<String>)> = None;
    if let Some((launcher, prefix_args)) = windows_launcher {
        command = launcher;
        extra_args = prefix_args;
    } else if let Some((launcher, prefix_args)) = terminal::resolve_termux_tool_launcher(tool_exe, &resolved) {
        command = launcher;
        extra_args = prefix_args;
    } else {
        command = resolved;
        extra_args = vec![];
    }
    let mut child_env = pty_child_env();
    #[cfg(target_os = "linux")]
    let mut native_session = None;
    #[cfg(unix)]
    #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
    let mut native_ready = None;
    let grok_acp = if target.known_tool() == Some(tool::Tool::Grok) && instance_name.is_some() {
        let launch = delivery::grok::Launch::new(&command, &extra_args, &tool_args)?;
        child_env.extend(launch.child_env());
        Some(launch)
    } else {
        None
    };
    let leader_args = grok_acp.as_ref().map(|l| l.tui_args()).unwrap_or_default();
    let grok_leader = grok_acp.clone();
    // Create and run PTY
    let instance_name_for_failure = instance_name.clone();
    #[cfg_attr(not(target_os = "linux"), allow(clippy::redundant_closure_call, reason = "Linux preparation errors must reach the same failure handler as PTY spawn errors"))]
    let spawn = (|| -> Result<_> {
        #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
        let mut full_args: Vec<&str> = extra_args.iter().map(String::as_str).chain(leader_args.iter().map(String::as_str)).chain(tool_args.iter().copied()).collect();
        #[cfg(target_os = "linux")]
        let fallback_args;
        #[cfg(target_os = "linux")]
        if target.known_tool() == Some(tool::Tool::Codex) {
            let context = crate::runtime::context::AgentxContext::from_os();
            if let Some(title) = context.raw_env.get(crate::integrations::codex::terminal::RUNNER_MODE).filter(|title| !title.is_empty()).cloned() {
                let auto_approve = crate::config::AgentxConfig::load(Some(&context.raw_env)).auto_approve;
                let launch_context = serde_json::from_str(&pty::build_early_launch_context())?;
                let session = crate::integrations::codex::terminal::TerminalSession::prepare(&command, &extra_args, &tool_args, &title, auto_approve, context.clone(), launch_context)?;
                child_env.push((crate::integrations::codex::terminal::RUNNER_MODE.into(), String::new()));
                if let Some(session) = session {
                    let session = native_session.insert(session);
                    child_env.push(("CODEX_HOME".into(), session.home.to_string_lossy().into_owned()));
                    full_args = extra_args.iter().map(String::as_str).chain(session.argv.iter().map(String::as_str)).collect();
                    native_ready = Some(session.ready.clone());
                } else {
                    let ctx = crate::integrations::hooks::runtime::LaunchCtx { tool: tool::Tool::Codex, env: context.raw_env, cwd: context.cwd, args: tool_args.iter().map(|arg| (*arg).to_owned()).collect(), auto_approve };
                    let injection = crate::integrations::hooks::codex::plan_managed_pty(&ctx)?;
                    child_env.extend(injection.env);
                    fallback_args = injection.args;
                    full_args = extra_args.iter().chain(&fallback_args).map(String::as_str).collect();
                }
            }
        }
        pty::Proxy::spawn(
            &command,
            &full_args,
            pty::ProxyConfig {
                ready_patterns,
                instance_name,
                target,
                env_vars: child_env,
                grok_acp,
                #[cfg(unix)]
                native_ready,
            },
        )
    })();
    let mut proxy = match spawn {
        Ok(proxy) => proxy,
        Err(e) => {
            let err = e.context("Failed to spawn PTY");
            // The launched terminal window may close on process exit before
            // anyone can read stderr (depends on the terminal's own
            // exit/profile behavior, which agentx doesn't control), so stderr
            // alone can't be relied on. Log so the failure survives, and —
            // same path used for a child that exits before binding — push it
            // through the launch-failure event so the launcher (human or the
            // agent that ran `agentx launch --tool <tool> --count N`) is notified immediately instead
            // of waiting on the generic stale-placeholder timeout.
            log::log_error("pty", "spawn_failed", &format!("{err:#}"));
            // Grok may have started its persistent leader before the failure.
            if let Some(launch) = grok_leader.as_ref() {
                launch.stop_leader();
            }
            if let Some(name) = instance_name_for_failure.as_deref() {
                let context = crate::runtime::context::AgentxContext::from_os();
                if let Err(report_error) = crate::runtime::launcher::report_spawn_failure(&context, name, &format!("{err:#}")) {
                    log::log_warn("pty", "spawn_failure_report", &report_error.to_string());
                }
            }
            return Err(err);
        }
    };

    #[cfg(target_os = "linux")]
    if let Some(session) = &mut native_session {
        session.run_mailbox();
    }
    let exit_code = proxy.run().context("PTY run failed");

    // Drop proxy to run cleanup (join delivery thread, which does DB cleanup)
    drop(proxy);
    #[cfg(target_os = "linux")]
    drop(native_session);
    // Only once the TUI is gone: a live TUI would respawn its leader.
    if let Some(launch) = grok_leader {
        launch.stop_leader();
    }
    let exit_code = exit_code?;

    std::process::exit(exit_code);
}

fn pty_child_env() -> Vec<(String, String)> {
    // Terminal capability defaults belong to the shared PTY child boundary.
    vec![("AGENTX_LAUNCHED".to_string(), "1".to_string())]
}

#[cfg(test)]
mod tests {
    use crate::cli::router::{self, Action};

    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    /// Test that no args runs Rust TUI
    #[test]
    fn test_no_args_runs_rust_tui() {
        let action = router::resolve_action(&[]);
        assert_eq!(action, Action::Tui);
    }

    /// Test that PTY mode is correctly identified
    #[test]
    fn test_pty_mode() {
        let action = router::resolve_action(&args(&["pty", "claude"]));
        assert_eq!(action, Action::Pty { args: args(&["claude"]) });
    }

    /// Test that client mode is correctly identified for non-pty commands
    #[test]
    fn test_client_mode() {
        let action = router::resolve_action(&args(&["list"]));
        match action {
            Action::Command { cmd, .. } => assert_eq!(cmd, "list"),
            _ => panic!("Expected Command action, got {:?}", action),
        }
    }

    /// Test PTY mode with multiple args
    #[test]
    fn test_pty_mode_with_args() {
        let action = router::resolve_action(&args(&["pty", "claude", "--arg1", "--arg2"]));
        assert_eq!(action, Action::Pty { args: args(&["claude", "--arg1", "--arg2"]) });
    }

    #[test]
    fn test_pty_child_env_marks_launched() {
        assert!(super::pty_child_env().contains(&("AGENTX_LAUNCHED".to_string(), "1".to_string())));
    }
}
