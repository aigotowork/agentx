//! CLI router: clap-based dispatch for hooks, commands, PTY, and TUI.
//!
//! All hooks and commands are handled natively in Rust.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;

use crate::integrations::tool::Tool;
use crate::log::{log_error, log_info, log_warn};
#[cfg(test)]
use crate::shared::AgentxError;
use crate::shared::dev_root_binary;

/// All known hook names (for fast lookup)
fn is_hook(name: &str) -> bool {
    Tool::is_hook_name(name)
}

// ── Known CLI commands ──────────────────────────────────────────────────

const COMMANDS: &[&str] = &[
    "labels",
    "bindings",
    "profile",
    "send",
    "recover",
    "list",
    "events",
    "stop",
    "start",
    "listen",
    "status",
    "daemon",
    "observe",
    "observe-web",
    "workbench",
    "work",
    "config",
    "hooks",
    "archive",
    "reset",
    "transcript",
    "bundle",
    "kill",
    "attach",
    "term",
    "relay",
    "run",
    "update",
];

/// Resolve public command shortcuts at command boundaries, never in operand values.
pub(crate) fn canonical_command(name: &str) -> &str {
    match name {
        "l" => "launch",
        "r" => "resume",
        "f" => "fork",
        "k" => "kill",
        "a" => "attach",
        _ => name,
    }
}

fn is_command(name: &str) -> bool {
    COMMANDS.contains(&canonical_command(name))
}

fn is_launch_tool(name: &str) -> bool {
    matches!(canonical_command(name), "launch" | "resume" | "fork")
}

/// Unknown top-level tokens are command errors; launch selection belongs to --tool.
fn unknown_command_message(cmd: &str, _args: &[String]) -> String {
    use crate::shared::suggest::did_you_mean;
    let suggestion = did_you_mean(cmd, COMMANDS.iter().copied().chain(["launch", "resume", "fork"]));
    format!("Unknown command '{cmd}'{suggestion}")
}

#[cfg(test)]
fn maybe_external_send_name_hint(cmd: &str, explicit_name: Option<&str>, has_from_flag: bool, process_id: Option<&str>, is_inside_ai_tool: bool, err: &AgentxError) -> Option<String> {
    let name = explicit_name?;
    if cmd != "send" || has_from_flag || process_id.is_some() || is_inside_ai_tool || !matches!(err, AgentxError::NotFound(_)) {
        return None;
    }

    let agentx_cmd = crate::runtime::env::build_agentx_command();
    Some(format!("{err}\nHint: If '{name}' is an external sender (cron/script/manual alert), use:\n  {agentx_cmd} send --from {name} ..."))
}

fn dispatch_hook_for_tool(tool: Tool, hook: &str, args: &[String]) -> (i32, String) {
    match tool {
        Tool::Claude if hook == "claude-mod-poll" => crate::integrations::hooks::claude::native::dispatch(args),
        Tool::Claude => (crate::integrations::hooks::claude::dispatch_claude_hook(hook), String::new()),
        Tool::Gemini => (crate::integrations::hooks::gemini::dispatch_gemini_hook(hook), String::new()),
        Tool::Codex => (crate::integrations::hooks::codex::dispatch_codex_hook_native(hook), String::new()),
        Tool::OpenCode => crate::integrations::hooks::opencode::dispatch_opencode_hook(hook, args),
        Tool::Kilo => crate::integrations::hooks::opencode::dispatch_opencode_hook(hook, args),
        Tool::Pi => crate::integrations::hooks::pi::dispatch_pi_hook(hook, args),
        Tool::Omp => crate::integrations::hooks::omp::dispatch_omp_hook(hook, args),
        Tool::Hermes => crate::integrations::hooks::hermes::dispatch_hermes_hook(hook, args),
        Tool::Antigravity => (crate::integrations::hooks::gemini::dispatch_gemini_hook(hook), String::new()),
        Tool::Cursor => (crate::integrations::hooks::cursor::dispatch_cursor_hook_native(hook), String::new()),
        Tool::Kimi => (crate::integrations::hooks::kimi::dispatch_kimi_hook(hook), String::new()),
        Tool::Copilot if hook.starts_with("copilot-ext-") => crate::integrations::hooks::copilot::dispatch_copilot_ext(hook, args),
        Tool::Copilot => (crate::integrations::hooks::copilot::dispatch_copilot_hook_native(hook), String::new()),
        Tool::Qoder => (crate::integrations::hooks::qoder::dispatch_qoder_hook_native(hook), String::new()),
        Tool::Grok | Tool::Adhoc => unreachable!("{} has no hooks", tool.as_str()),
    }
}

// ── Dispatch types ──────────────────────────────────────────────────────

/// Resolved action after argv inspection.
#[derive(Debug, PartialEq)]
pub enum Action {
    /// Run a hook handler. Args are the full argv[1..] passed through.
    Hook { hook: String, args: Vec<String> },
    /// Run a CLI command. Args are the full argv[1..] passed through.
    Command { cmd: String, args: Vec<String> },
    /// Launch tool (e.g. `agentx launch --tool claude --count 3 -- --model haiku`)
    Launch { args: Vec<String> },
    /// Run PTY wrapper mode
    Pty { args: Vec<String> },
    /// Run the default TUI, optionally with Agent filters.
    Tui,
    /// Show version
    Version,
    /// Show help
    Help,
    /// `agentx help <topic...>`: print that topic's help page. Never dispatches
    /// the target, since not every target (scripts, relay-worker) honors --help.
    TopicHelp { args: Vec<String> },
    /// Open TUI in new terminal window
    NewTerminal,
    /// Run relay-worker process
    RelayWorker,
}

/// Global flags extracted from argv before dispatch.
#[derive(Debug, Default, PartialEq)]
pub struct GlobalFlags {
    pub name: Option<String>,
    pub go: bool,
}

// ── Argv parsing (clap for flags, manual for command routing) ───────────
//
// Top-level command/hook routing stays manual because agentx's CLI is unusual:
// hooks appear as bare subcommands (`agentx sessionstart`), launch commands can
// select an explicit launch action, and ~40 hook/command/tool names
// need classification. clap will be used for per-command arg parsing as commands
// are ported to Rust.

/// Clap parser for global flags only. Remaining args collected as positionals.
#[derive(Parser, Debug)]
#[command(no_binary_name = true, disable_help_flag = true, disable_version_flag = true)]
struct GlobalFlagParser {
    /// Existing caller identity; otherwise use the injected process/session
    #[arg(long = "iam")]
    name: Option<String>,

    /// Skip confirmation prompts
    #[arg(long, action = clap::ArgAction::SetTrue)]
    go: bool,

    /// Everything after global flags (command/hook name + its args)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    rest: Vec<String>,
}

/// Extract global flags (--iam VALUE, --go) from argv using clap.
/// Returns (remaining_args, flags).
///
/// NOTE: clap's trailing_var_arg means --iam after a positional (command token)
/// is NOT extracted. Use `extract_global_flags_full()` when you need to find
/// --iam anywhere in argv (e.g., `agentx send --iam vami @luna -- hello`).
pub fn extract_global_flags(argv: &[String]) -> (Vec<String>, GlobalFlags) {
    match GlobalFlagParser::try_parse_from(argv) {
        Ok(parsed) => (parsed.rest, GlobalFlags { name: parsed.name, go: parsed.go }),
        Err(_) => {
            // Clap couldn't parse (e.g. --iam without value). Fall back to
            // manual extraction so the proper error can be reported.
            extract_global_flags_manual(argv)
        }
    }
}

/// Manual fallback for global flag extraction (handles edge cases clap rejects).
fn extract_global_flags_manual(argv: &[String]) -> (Vec<String>, GlobalFlags) {
    let mut remaining = Vec::with_capacity(argv.len());
    let mut flags = GlobalFlags::default();
    let mut i = 0;

    while i < argv.len() {
        match argv[i].as_str() {
            "--iam" if i + 1 < argv.len() => {
                flags.name = Some(argv[i + 1].clone());
                i += 2;
            }
            "--go" => {
                flags.go = true;
                i += 1;
            }
            _ => {
                remaining.push(argv[i].clone());
                i += 1;
            }
        }
    }

    (remaining, flags)
}

/// Extract global flags from anywhere in argv, respecting `--` separator.
///
/// Unlike `extract_global_flags()` (clap-based, only finds flags before first
/// positional), this scans the full argv up to `--`. Used by `dispatch_native_command()`
/// to handle `agentx send --iam vami @luna -- hello` correctly.
///
/// Also detects --help/-h requests (before `--`) for per-command help dispatch.
pub fn extract_global_flags_full(argv: &[String]) -> (Vec<String>, GlobalFlags, bool) {
    let sep_pos = argv.iter().position(|a| a == "--");
    let scan_end = sep_pos.unwrap_or(argv.len());

    let mut remaining = Vec::with_capacity(argv.len());
    let mut flags = GlobalFlags::default();
    let mut help_requested = false;
    let mut i = 0;

    while i < scan_end {
        if crate::runtime::environment::takes_opaque_value(&argv[i]) && argv[i] != "--iam" && i + 1 < scan_end {
            remaining.extend_from_slice(&argv[i..i + 2]);
            i += 2;
            continue;
        }
        match argv[i].as_str() {
            "--iam" if i + 1 < scan_end => {
                flags.name = Some(argv[i + 1].clone());
                i += 2;
            }
            "--go" => {
                flags.go = true;
                i += 1;
            }
            "--help" | "-h" => {
                help_requested = true;
                i += 1;
            }
            _ => {
                remaining.push(argv[i].clone());
                i += 1;
            }
        }
    }

    // Append everything from separator onwards unchanged
    if i < argv.len() {
        remaining.extend_from_slice(&argv[i..]);
    }

    (remaining, flags, help_requested)
}

/// Determine action from argv (after binary name is stripped).
///
/// High-level precedence: no args -> TUI; top-level global flags / special modes
/// (`relay-worker`, `pty`); then first non-flag token as hook, CLI command, or
/// launch verb.
pub fn resolve_action(argv: &[String]) -> Action {
    // No args: TUI
    if argv.is_empty() {
        return Action::Tui;
    }

    let first = argv[0].as_str();
    if crate::cli::workspace::starts_dashboard(first) {
        return Action::Tui;
    }

    // Global flags as commands
    match first {
        "--help" | "-h" => return Action::Help,
        // `agentx help [cmd...]` == `agentx [cmd...] --help`
        "help" if argv.len() == 1 => return Action::Help,
        "help" => {
            return Action::TopicHelp { args: argv[1..].to_vec() };
        }
        "--version" | "-v" => return Action::Version,
        "--new-terminal" => return Action::NewTerminal,
        _ => {}
    }

    // Relay worker mode: `agentx relay-worker`
    if first == "relay-worker" {
        return Action::RelayWorker;
    }

    // PTY mode: `agentx pty <tool> [args...]`
    if first == "pty" {
        return Action::Pty { args: argv[1..].to_vec() };
    }

    // Strip global flags for command/hook detection
    let (stripped, _flags) = extract_global_flags(argv);

    // Find the first non-flag token in stripped args
    let cmd_token = stripped.first().map(|s| s.as_str()).unwrap_or("");
    if crate::cli::workspace::starts_dashboard(cmd_token) {
        return Action::Tui;
    }

    // Hook detection: argv[1] matches a known hook name
    if is_hook(cmd_token) {
        return Action::Hook { hook: cmd_token.to_string(), args: argv.to_vec() };
    }

    // Command detection
    if is_command(cmd_token) {
        return Action::Command { cmd: canonical_command(cmd_token).to_string(), args: argv.to_vec() };
    }

    // Explicit launch/resume/fork actions.
    if is_launch_tool(cmd_token) {
        return Action::Launch { args: argv.to_vec() };
    }
    // --new-terminal can appear after flags: `agentx --iam foo --new-terminal`
    if stripped.iter().any(|a| a == "--new-terminal") {
        return Action::NewTerminal;
    }

    // Unknown — fall through to client
    Action::Command { cmd: cmd_token.to_string(), args: argv.to_vec() }
}

// ── AGENTX_DEV_ROOT re-exec ───────────────────────────────────────────────

/// If AGENTX_DEV_ROOT is set and points to a different worktree, re-exec using
/// that worktree's binary.
/// so worktree development works: `AGENTX_DEV_ROOT=/path/to/worktree agentx list`
/// will run the worktree's agentx binary instead of the installed one.
pub fn maybe_reexec_dev_root() {
    let (dev_root, source) = match resolve_effective_dev_root(&crate::paths::db_path()) {
        Some(v) => v,
        None => return,
    };

    // Find current binary's location
    let current_exe = match env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };

    let target_binary = match dev_root_binary(&dev_root) {
        Some(p) => p,
        None => {
            log_warn("router", "dev_root_no_binary", &format!("dev_root={} ({source}) but no dev binary found. Run `cargo build` or `cargo build --release` in the worktree.", dev_root.display(),));
            return;
        }
    };

    // Don't re-exec if we're already running the right binary
    if is_same_file(&current_exe, &target_binary) {
        return;
    }

    log_info("router", "dev_root_reexec", &format!("re-exec to {} (current={}, source={})", target_binary.display(), current_exe.display(), source,));

    // Re-exec: replace this process with the dev root's binary
    let args: Vec<String> = env::args().collect();
    let mut cmd = Command::new(&target_binary);
    cmd.args(&args[1..]);
    let err = crate::sys::process::exec_replace(cmd);
    // exec_replace only returns on error
    log_error("router", "dev_root_reexec_failed", &format!("failed to exec {}: {}", target_binary.display(), err));
}

/// True if argv is `agentx config dev_root [...]` (after stripping global flags).
fn is_config_dev_root_invocation(argv: &[String]) -> bool {
    let (positional, _, _) = extract_global_flags_full(argv);
    let mut iter = positional.iter().take_while(|a| a.as_str() != "--");
    matches!((iter.next().map(String::as_str), iter.next().map(String::as_str)), (Some("config"), Some("dev_root")))
}

/// `agentx update` must run from the binary the user invoked. Re-executing a
/// configured dev-root binary would compare the checkout version and inspect
/// the checkout executable path instead of updating the installed binary.
fn is_update_invocation(argv: &[String]) -> bool {
    let (positional, _, _) = extract_global_flags_full(argv);
    match positional.as_slice() {
        [first, ..] if first == "update" => true,
        [first, second, ..] => first == "help" && second == "update",
        _ => false,
    }
}

/// Print help for `agentx help <topic...>` without running anything.
fn print_topic_help(args: &[String]) -> i32 {
    let (stripped, _, _) = extract_global_flags_full(args);
    let words = stripped;
    let Some(name) = words.first().map(String::as_str) else {
        crate::cli::commands::help::print_help();
        return 0;
    };
    let name = canonical_command(name);
    let topic = if is_command(name) {
        crate::cli::commands::help::help_topic(name, &words[1..])
    } else if is_launch_tool(name) || matches!(name, "resume" | "fork") {
        name.to_string()
    } else {
        eprintln!("Error: {}", unknown_command_message(name, &words));
        return 1;
    };
    crate::cli::commands::help::print_command_help(&topic);
    0
}

pub(crate) fn resolve_effective_dev_root(db_path: &Path) -> Option<(PathBuf, &'static str)> {
    if let Ok(r) = env::var("AGENTX_DEV_ROOT")
        && !r.is_empty()
    {
        return Some((PathBuf::from(r), "env"));
    }

    read_dev_root_from_kv(db_path).map(|path| (path, "kv"))
}

fn read_dev_root_from_kv(db_path: &Path) -> Option<PathBuf> {
    if !db_path.exists() {
        return None;
    }
    let scope = db_path.parent()?;
    let result = crate::service::InvocationContext::capture()
        .and_then(|invocation| crate::service::client::Client::connect(scope, invocation))
        .and_then(|client| client.call(crate::service::Request::Maintenance(crate::service::maintenance::Request::DevRoot)));
    match result {
        Ok(crate::service::Response::Maintenance(crate::service::maintenance::Response::DevRoot(value))) => value.filter(|value| !value.is_empty()).map(PathBuf::from),
        Ok(_) => {
            log_warn("router", "dev_root_kv_read_failed", "daemon returned a different maintenance response");
            None
        }
        Err(error) => {
            log_warn("router", "dev_root_kv_read_failed", &error.to_string());
            None
        }
    }
}

/// Check if two paths refer to the same file (follows symlinks).
fn is_same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

// ── Dispatch ────────────────────────────────────────────────────────────

const PERSISTENT_INSTALL_INSTRUCTION: &str = "agentx: uvx is no longer supported; run uv tool install agentx, then restart this session.";

/// Handle ephemeral invocations before dev-root lookup, update checks, or
/// normal command dispatch. Legacy removal is dispatched directly, though tool
/// cleanup (such as Hermes) may still open state.
fn dispatch_ephemeral(action: &Action) -> i32 {
    let mut exit_code = 1;
    match action {
        Action::Hook { hook, .. } => {
            if Tool::from_hook_name(hook) == Some(Tool::Gemini) && env::var_os("ANTIGRAVITY_AGENT").is_some() {
                // Antigravity requires decisions on these events. Its docs
                // do not promise nonzero hook exits fail open, so stay safe.
                if matches!(hook.as_str(), "gemini-beforetool" | "gemini-sessionend") {
                    print!("{{\"decision\":\"allow\"}}");
                }
                exit_code = 0;
            } else if hook == "copilot-pretooluse" {
                // Copilot denies tool execution on ANY nonzero PreToolUse exit.
                exit_code = 0;
            }
        }
        Action::Pty { args } if args.first().is_some_and(|arg| matches!(arg.as_str(), "--help" | "-h")) => {
            return match crate::run_pty(args) {
                Ok(()) => 0,
                Err(_) => 1,
            };
        }
        Action::Version => {
            println!("agentx {}", env!("CARGO_PKG_VERSION"));
            return 0;
        }
        Action::Help => {
            crate::cli::commands::help::print_help();
            return 0;
        }
        Action::TopicHelp { args } => return print_topic_help(args),
        Action::Command { args, .. } | Action::Launch { args } => {
            let (stripped, _, help) = extract_global_flags_full(args);
            if help {
                // Never run a script to ask for help under uvx.
                return print_topic_help(&stripped);
            }
            if let Action::Command { cmd, .. } = action
                && cmd == "hooks"
                && matches!(stripped.get(1).map(String::as_str), Some("remove" | "uninstall"))
            {
                return crate::cli::commands::hooks::cmd_hooks_remove(&stripped[2..]);
            }
        }
        _ => {}
    }
    eprintln!("{PERSISTENT_INSTALL_INSTRUCTION}");
    exit_code
}

/// Explicit finite browser/work entries and passive diagnostics, dispatched
/// before ordinary main/router setup or dev-root reads. Each entry parses its
/// own flags; workbench collaboration is selected only at local startup.
pub(crate) fn dispatch_passive_observation(argv: &[String]) -> Option<i32> {
    if argv.first().is_some_and(|arg| arg == "roles") {
        return Some(match crate::cli::commands::roles::run(argv) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("Error: {error:#}");
                1
            }
        });
    }
    if let Some(code) = crate::cli::commands::profile::dispatch(argv) {
        return Some(code);
    }
    if crate::cli::commands::launch::is_dry_run(argv) {
        let (_, flags, _) = extract_global_flags_full(argv);
        return Some(match crate::cli::commands::launch::run(argv, &flags) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("Error: {error:#}");
                1
            }
        });
    }
    if let Some(code) = crate::cli::commands::daemon::dispatch_native(argv) {
        return Some(code);
    }
    if let Some(code) = crate::collaboration::web::dispatch(argv) {
        return Some(code);
    }
    if let Some(code) = crate::cli::commands::bindings::dispatch(argv) {
        return Some(code);
    }
    if let Some(code) = crate::cli::commands::work::dispatch(argv) {
        return Some(code);
    }
    if let Some(code) = crate::observability::web::dispatch(argv) {
        return Some(code);
    }
    match argv.first().map(String::as_str) {
        Some("observe") => Some(crate::cli::commands::observe::cmd_observe(&argv[1..])),
        Some("help") if argv.get(1).map(String::as_str) == Some("observe") => {
            let mut args = argv[2..].to_vec();
            args.push("--help".into());
            Some(crate::cli::commands::observe::cmd_observe(&args))
        }
        _ => None,
    }
}

fn is_local_metadata(action: &Action) -> bool {
    if let Action::Launch { args } = action
        && crate::cli::commands::launch::is_dry_run(args)
    {
        return true;
    }
    matches!(action, Action::Command { cmd, .. } if matches!(cmd.as_str(), "run" | "hooks"))
        || matches!(action, Action::Command { cmd, args } if cmd == "config" && args.iter().any(|arg| arg == "environment" || crate::config::file::environment_key(arg).is_some()))
}

/// Update cache reads may create flags or spawn a worker. Keep that effect
/// behind the same passive metadata boundary as originating source loading.
fn command_update_notice(action: &Action, argv: &[String], read_notice: impl FnOnce() -> Option<String>) -> Option<String> {
    if is_local_metadata(action) || crate::runtime::environment::is_lightweight(argv) {
        return None;
    }
    match action {
        Action::Command { cmd, .. } if cmd != "update" => read_notice(),
        Action::Launch { .. } => read_notice(),
        _ => None,
    }
}

/// Main entry point: resolve action and dispatch.
pub fn dispatch(argv: &[String]) -> anyhow::Result<()> {
    if let Some(code) = dispatch_passive_observation(argv) {
        if code != 0 {
            std::process::exit(code);
        }
        return Ok(());
    }
    let action = resolve_action(argv);
    if env::current_exe().ok().and_then(|exe| exe.canonicalize().ok()).is_some_and(|exe| crate::runtime::env::is_uvx_ephemeral_exe(&exe)) {
        // Normalize global flags only for the ephemeral help/version surface;
        // installed dispatch retains its existing argument resolution.
        let (stripped, _) = extract_global_flags(argv);
        let ephemeral_action = resolve_action(&stripped);
        let code = dispatch_ephemeral(&ephemeral_action);
        if code != 0 {
            std::process::exit(code);
        }
        return Ok(());
    }

    // Internal background update workers must pass the same install gate.
    if argv.first().map(String::as_str) == Some("__update-check") {
        let mut args = env::args_os().skip(2);
        if let (Some(flag), Some(current)) = (args.next(), args.next()) {
            crate::cli::update::run_background_check(Path::new(&flag), &current.to_string_lossy());
        }
        return Ok(());
    }

    // Skip dev_root re-exec for `config dev_root` so a stale pointer can't
    // trap the user. Also keep `update` on the invoked (installed) binary;
    // otherwise the checkout's version and path would drive update decisions.
    let local_help = match &action {
        Action::Command { args, .. } | Action::Launch { args } => extract_global_flags_full(args).2,
        Action::Pty { args } => args.iter().any(|arg| matches!(arg.as_str(), "--help" | "-h")),
        Action::Tui | Action::NewTerminal => extract_global_flags_full(argv).2,
        _ => false,
    };
    let local_metadata = is_local_metadata(&action);
    if !matches!(action, Action::Help | Action::Version | Action::TopicHelp { .. }) && !local_help && !local_metadata && !is_config_dev_root_invocation(argv) && !is_update_invocation(argv) {
        crate::observability::trace::time(crate::observability::trace::Phase::DevRoot, maybe_reexec_dev_root);
    }

    if let Some(notice) = command_update_notice(&action, argv, crate::cli::update::get_update_notice) {
        eprintln!("{notice}");
    }

    match action {
        Action::Tui | Action::NewTerminal => {
            let exit_code = run_dashboard(argv)?;
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Action::Pty { args } => {
            crate::run_pty(&args)?;
        }
        Action::RelayWorker => {
            let exit_code = crate::relay::worker::run();
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Action::Hook { ref hook, ref args } => {
            let Some(tool) = Tool::from_hook_name(hook) else {
                // Defensive: unreachable while resolve_action's is_hook() and
                // from_hook_name() share Tool's hook registry.
                log_error("router", "hook.unknown", &format!("hook={hook}"));
                eprintln!("Error: Unknown hook '{}'", hook);
                std::process::exit(1);
            };

            let (exit_code, output) = dispatch_hook_for_tool(tool, hook, args);
            if !output.is_empty() {
                print!("{}", output);
            }
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Action::Launch { ref args } => {
            // Launch/resume/fork handled natively.
            let (stripped, flags, help) = extract_global_flags_full(args);
            let first_cmd = stripped.first().map(|s| s.as_str()).unwrap_or("");
            let cmd = canonical_command(first_cmd);
            if help {
                crate::cli::commands::help::print_command_help(cmd);
                return Ok(());
            }
            let exit_code = match cmd {
                "resume" => crate::cli::commands::resume::run(args, &flags)?,
                "fork" => crate::cli::commands::fork::run(args, &flags)?,
                _ => crate::cli::commands::launch::run(args, &flags)?,
            };
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Action::Command { ref cmd, ref args } if matches!(cmd.as_str(), "start" | "kill") => {
            let (_, flags, help) = extract_global_flags_full(args);
            if help {
                crate::cli::commands::help::print_command_help(cmd);
                return Ok(());
            }
            let exit_code = match cmd.as_str() {
                "start" => crate::cli::commands::start::run(args, &flags)?,
                "kill" => crate::cli::commands::kill::run(args, &flags)?,
                _ => unreachable!(),
            };
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Action::Command { ref cmd, ref args }
            if matches!(cmd.as_str(), "labels" | "send" | "recover" | "list" | "stop" | "listen" | "events" | "transcript" | "config" | "status" | "bundle" | "archive" | "reset" | "hooks" | "attach" | "term" | "relay" | "run" | "update") =>
        {
            let exit_code = dispatch_native_command(cmd, args);
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Action::Command { ref cmd, ref args } => {
            eprintln!("Error: {}", unknown_command_message(cmd, args));
            std::process::exit(1);
        }
        Action::Version => {
            println!("agentx {}", env!("CARGO_PKG_VERSION"));
        }
        Action::Help => {
            crate::cli::commands::help::print_help();
        }
        Action::TopicHelp { ref args } => {
            let exit_code = print_topic_help(args);
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
    }

    Ok(())
}

// ── New terminal ─────────────────────────────────────────────────────────

fn run_dashboard(argv: &[String]) -> anyhow::Result<i32> {
    use crate::cli::workspace::DashboardArgs;
    use crate::service::queries::{AgentFilter, AgentWorkspace};
    use std::io::IsTerminal;

    let (args, flags, help) = extract_global_flags_full(argv);
    if help {
        crate::cli::commands::help::print_help();
        return Ok(0);
    }
    let args = match DashboardArgs::try_parse_from(std::iter::once("agentx".to_owned()).chain(args)) {
        Ok(args) => args,
        Err(error) => {
            crate::cli::commands::help::print_clap_error(&error)?;
            return Ok(error.exit_code());
        }
    };
    if !args.new_terminal && (!std::io::stdin().is_terminal() || !std::io::stdout().is_terminal()) {
        anyhow::bail!("agentx dashboard requires an interactive terminal; use agentx list for command output");
    }
    let labels = args.filters.label_predicates()?;
    let workspace = if !args.all && args.filters.has_workspace_selector() {
        let session = crate::cli::client::Session::connect(flags.name.as_deref(), flags.go)?;
        args.filters.workspace(&session, false)?
    } else {
        AgentWorkspace::All
    };
    if args.new_terminal {
        return Ok(launch_new_terminal(workspace.as_selector(), &args.filters.labels));
    }
    crate::tui::run(AgentFilter { workspace, labels })?;
    Ok(0)
}

/// Open TUI in a new terminal window using native terminal.rs.
fn launch_new_terminal(workspace: Option<&str>, labels: &[String]) -> i32 {
    use std::collections::HashMap;

    let exe = match env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(e) => {
            eprintln!("Error: Cannot determine agentx binary path: {}", e);
            return 1;
        }
    };

    let cwd = env::current_dir().ok().map(|p| p.to_string_lossy().to_string());

    // Pass through AgentX configuration and explicitly loaded originating
    // values, including empties, to a newly opened TUI process.
    let file_keys = crate::runtime::environment::originating_keys();
    let mut env_vars = HashMap::new();
    for (k, v) in env::vars() {
        if k.starts_with("AGENTX_") || file_keys.contains(&k) {
            env_vars.insert(k, v);
        }
    }

    let inside_ai = crate::shared::is_inside_ai_tool();
    let quote: fn(&str) -> String = if cfg!(windows) { crate::runtime::terminal::ps_quote } else { crate::integrations::launch_args::shell::shell_quote };
    let mut command = quote(&exe);
    if cfg!(windows) {
        command.insert_str(0, "& ");
    }
    if let Some(workspace) = workspace {
        command.push_str(&format!(" {}", quote(&format!("--workspace={workspace}"))));
    }
    for label in labels {
        command.push_str(&format!(" {}", quote(&format!("--label={label}"))));
    }

    match crate::runtime::terminal::launch_terminal(
        &command,
        &env_vars,
        cwd.as_deref(),
        false, // not background
        false, // not run_here (open new window)
        None,  // default terminal
        inside_ai,
        None, // not launching a specific tool (agentx TUI itself)
    ) {
        Ok((crate::runtime::terminal::LaunchResult::Success, _)) => 0,
        Ok((crate::runtime::terminal::LaunchResult::Failed(msg), _)) => {
            eprintln!("Error: {}", msg);
            1
        }
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Error: Failed to open new terminal: {}", e);
            1
        }
    }
}

// ── Native command dispatch ──────────────────────────────────────────────

/// Ordinary native commands share capture, status and exact post-output inbox
/// handling. Command-specific parsing and pure-local bypass happen first.
fn run_client_command(flags: &GlobalFlags, cmd: &str, json: bool, run: impl FnOnce(&crate::cli::client::Session) -> i32) -> i32 {
    let connect = if cmd == "listen" { crate::cli::client::Session::connect_required } else { crate::cli::client::Session::connect };
    let session = match connect(flags.name.as_deref(), flags.go) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    if let Some(warning) = &session.command.identity_warning {
        eprintln!("{warning}");
    }
    if let Err(error) = session.set_command_status(cmd) {
        eprintln!("Error: {error}");
        return 1;
    }
    let result = run(&session);
    if let Err(error) = session.deliver_pending(json) {
        eprintln!("agentx: {error}");
        return 1;
    }
    result
}

/// Dispatch a natively-handled CLI command.
///
/// Parse each command before its typed client session or host-only bypass.
/// Args are the full argv[1..] (includes the command name and global flags).
fn dispatch_native_command(cmd: &str, args: &[String]) -> i32 {
    // Extract global flags (--iam, --go, --help) from anywhere in args,
    // respecting -- separator. Uses full scan (not clap) so --iam works
    // regardless of position: `agentx send --iam vami @luna -- hello`.
    let (stripped, flags, help_requested) = extract_global_flags_full(args);

    // Per-command --help: native help text
    // ("run" handles --help itself for script-level help)
    if help_requested && cmd != "run" {
        let topic = crate::cli::commands::help::help_topic(cmd, stripped.get(1..).unwrap_or_default());
        crate::cli::commands::help::print_command_help(&topic);
        return 0;
    }

    // Strip command name from stripped args to get command-specific argv.
    // For "run", re-inject --help so the script itself can handle it.
    let cmd_argv: Vec<String> = {
        let mut v: Vec<String> = stripped.iter().skip(1).cloned().collect();
        if cmd == "run" && help_requested {
            v.push("--help".to_string());
        }
        v
    };

    // "relay daemon" subcommand doesn't need DB or identity context
    if cmd == "relay" && cmd_argv.first().map(|s| s.as_str()) == Some("daemon") {
        return crate::cli::commands::daemon::cmd_daemon(&cmd_argv[1..]);
    }

    if cmd == "archive" {
        let args = match crate::cli::commands::archive::ArchiveArgs::try_parse_from(std::iter::once("archive".to_string()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("archive", &error),
        };
        let client = match crate::service::InvocationContext::capture().and_then(|mut invocation| {
            invocation.explicit_name = flags.name.clone();
            crate::service::client::Client::connect(crate::paths::agentx_dir(), invocation)
        }) {
            Ok(client) => client,
            Err(error) => {
                eprintln!("Error: {error}");
                return 1;
            }
        };
        return crate::cli::commands::archive::cmd_archive(&client, &args, None);
    }

    if cmd == "run" {
        return match crate::cli::commands::run::RunArgs::try_parse_from(std::iter::once("run".to_string()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => crate::cli::commands::run::cmd_run(&args, flags.name.as_deref()),
            Err(error) => crate::cli::commands::help::clap_usage_error("run", &error),
        };
    }
    if cmd == "hooks" {
        return match crate::cli::commands::hooks::HooksArgs::try_parse_from(std::iter::once("hooks".to_string()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => crate::cli::commands::hooks::cmd_hooks(&args),
            Err(error) => crate::cli::commands::help::clap_usage_error("hooks", &error),
        };
    }
    if cmd == "recover" {
        use crate::cli::commands::recover::{self, RecoverArgs};
        let parsed = match RecoverArgs::try_parse_from(std::iter::once("recover".to_owned()).chain(cmd_argv)) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("recover", &error),
        };
        return match recover::run(parsed, &flags) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("Error: {error}");
                1
            }
        };
    }
    if cmd == "update" {
        return match crate::cli::commands::update::UpdateArgs::try_parse_from(std::iter::once("update".to_string()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => crate::cli::commands::update::cmd_update(&args),
            Err(error) => crate::cli::commands::help::clap_usage_error("update", &error),
        };
    }

    if cmd == "attach" {
        let parsed = match crate::cli::commands::attach::AttachArgs::try_parse_from(std::iter::once("attach".to_string()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("attach", &error),
        };
        return run_client_command(&flags, cmd, false, |session| crate::cli::commands::attach::run(session, &parsed));
    }

    if cmd == "labels" {
        let parsed = match crate::cli::commands::labels::Arguments::try_parse_from(std::iter::once("labels".to_string()).chain(cmd_argv.iter().cloned())) {
            Ok(parsed) => parsed,
            Err(error) => {
                let code = error.exit_code();
                let _ = crate::cli::commands::help::print_clap_error(&error);
                return code;
            }
        };
        return run_client_command(&flags, cmd, parsed.json, |session| crate::cli::commands::labels::run(session, &parsed));
    }

    if cmd == "list" {
        let parsed = match crate::cli::commands::list::ListArgs::try_parse_from(std::iter::once("list".to_string()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("list", &error),
        };
        return run_client_command(&flags, cmd, parsed.json, |session| crate::cli::commands::list::cmd_list(session, &parsed));
    }

    if cmd == "listen" {
        let parsed = match crate::cli::commands::listen::ListenArgs::try_parse_from(std::iter::once("listen".to_string()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("listen", &error),
        };
        return run_client_command(&flags, cmd, parsed.json, |session| crate::cli::commands::listen_client::cmd_listen(session, &parsed));
    }

    if cmd == "events" {
        let parsed = match crate::cli::commands::events::EventsArgs::try_parse_from(std::iter::once("events".to_string()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("events", &error),
        };
        if let Err(error) = parsed.validate_reminder_options() {
            eprintln!("Error: {error}");
            return 2;
        }
        return run_client_command(&flags, cmd, false, |session| crate::cli::commands::events_client::cmd_events(session, &parsed));
    }

    if cmd == "term" {
        let parsed = crate::cli::commands::term::TermArgs { args: cmd_argv.clone() };
        if let Some(code) = crate::cli::commands::term::try_local(&parsed) {
            return code;
        }
        return run_client_command(&flags, cmd, cmd_argv.iter().any(|argument| argument == "--json"), |session| crate::cli::commands::term::cmd_term_client(session, &parsed));
    }

    if cmd == "send" {
        let mut parsed = match crate::cli::commands::send::SendArgs::try_parse_from(std::iter::once("send".to_string()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("send", &error),
        };
        parsed.had_separator = cmd_argv.iter().any(|argument| argument == "--");
        let directory_selected = parsed.dir.is_some() || parsed.workspace.is_some();
        let connect = if parsed.delivery_status.is_some() || parsed.sender_name().is_some() || directory_selected { crate::cli::client::Session::connect } else { crate::cli::client::Session::connect_required };
        let session = match connect(flags.name.as_deref(), flags.go) {
            Ok(session) => session,
            Err(error) => {
                eprintln!("Error: {error}");
                if matches!(error, crate::service::ServiceError::NotFound(_))
                    && parsed.sender_name().is_none()
                    && flags.name.is_some()
                    && std::env::var("AGENTX_PROCESS_ID").ok().filter(|value| !value.is_empty()).is_none()
                    && !crate::shared::is_inside_ai_tool()
                {
                    eprintln!("Hint: If '{}' is an external sender (cron/script/manual alert), use:\n  {} send --from {} ...", flags.name.as_deref().unwrap(), crate::runtime::env::build_agentx_command(), flags.name.as_deref().unwrap());
                }
                return 1;
            }
        };
        let runtime = crate::runtime::context::AgentxContext::from_env(&session.invocation.environment, session.invocation.cwd.clone());
        if parsed.delivery_status.is_some() {
            return crate::cli::commands::send::cmd_send_client(&session, &parsed);
        }
        if directory_selected && parsed.sender_name().is_none() && session.actor.is_none() && flags.name.is_none() && !runtime.is_inside_ai_tool() {
            parsed.bigboss = true;
        }
        let has_from = parsed.sender_name().is_some();
        if !has_from && let Some(warning) = &session.command.identity_warning {
            eprintln!("{warning}");
        }
        if let Err(error) = crate::cli::context::check_identity_gate(cmd, &session.command, has_from, runtime.is_inside_ai_tool()) {
            eprintln!("Error: {error}");
            return 1;
        }
        if let Err(error) = session.set_command_status(cmd) {
            eprintln!("Error: {error}");
            return 1;
        }
        let result = crate::cli::commands::send::cmd_send_client(&session, &parsed);
        if let Err(error) = session.deliver_pending(parsed.json) {
            eprintln!("agentx: {error}");
            return 1;
        }
        return result;
    }

    if cmd == "config" {
        let parsed = match crate::cli::commands::config::ConfigArgs::try_parse_from(std::iter::once("config".to_owned()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("config", &error),
        };
        if let Some(code) = crate::cli::commands::config::try_local(&parsed) {
            return code;
        }
        return run_client_command(&flags, cmd, parsed.json, |session| crate::cli::commands::config::cmd_config_client(session, &parsed));
    }

    if cmd == "transcript" {
        let parsed = match crate::cli::commands::transcript::TranscriptArgs::try_parse_from(std::iter::once("transcript".to_owned()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("transcript", &error),
        };
        return run_client_command(&flags, cmd, cmd_argv.iter().any(|argument| argument == "--json"), |session| crate::cli::commands::transcript::cmd_transcript(session, &parsed));
    }

    if cmd == "status" {
        let parsed = match crate::cli::commands::status::StatusArgs::try_parse_from(std::iter::once("status".to_string()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("status", &error),
        };
        return run_client_command(&flags, cmd, parsed.json, |session| crate::cli::commands::status::cmd_status(session, &parsed));
    }

    if cmd == "relay" {
        let parsed = crate::cli::commands::relay::RelayArgs { args: cmd_argv.clone() };
        if let Some(code) = crate::cli::commands::relay::try_local(&parsed) {
            return code;
        }
        return run_client_command(&flags, cmd, false, |session| crate::cli::commands::relay::cmd_relay(&session.client, &parsed));
    }

    if cmd == "bundle" {
        let parsed = crate::cli::commands::bundle::BundleArgs { args: cmd_argv.clone() };
        return run_client_command(&flags, cmd, cmd_argv.iter().any(|argument| argument == "--json"), |session| crate::cli::commands::bundle::cmd_bundle(session, &parsed));
    }

    if cmd == "stop" {
        let parsed = match crate::cli::commands::stop::StopArgs::try_parse_from(std::iter::once("stop".to_owned()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("stop", &error),
        };
        return run_client_command(&flags, cmd, false, |session| crate::cli::commands::stop::cmd_stop_client(session, &parsed));
    }

    if cmd == "reset" {
        let parsed = match crate::cli::commands::reset::ResetArgs::try_parse_from(std::iter::once("reset".to_owned()).chain(cmd_argv.iter().cloned())) {
            Ok(args) => args,
            Err(error) => return crate::cli::commands::help::clap_usage_error("reset", &error),
        };
        if let Some(code) = crate::cli::commands::reset::try_local(&parsed, flags.go) {
            return code;
        }
        return run_client_command(&flags, cmd, false, |session| crate::cli::commands::reset::cmd_reset_client(session, &parsed));
    }

    eprintln!("Error: Unknown native command '{cmd}'");
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sv(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn metadata_requests_never_enter_update_cache_or_worker_effect() {
        for input in [
            vec!["--help"],
            vec!["--help-all"],
            vec!["events", "sub", "--help-all"],
            vec!["work", "report", "--help-all"],
            vec!["--version"],
            vec!["help", "codex"],
            vec!["help", "config"],
            vec!["help", "daemon"],
            vec!["daemon", "--help"],
            vec!["config", "--help"],
            vec!["launch", "--tool", "codex", "--help"],
            vec!["launch", "--tool", "codex", "--stdio", "--", "--version"],
            vec!["config", "environment", "--json"],
            vec!["config", "env_files"],
            vec!["run", "envprobe"],
            vec!["hooks", "list"],
        ] {
            let argv = sv(&input);
            let calls = std::cell::Cell::new(0);
            let notice = command_update_notice(&resolve_action(&argv), &argv, || {
                calls.set(calls.get() + 1);
                Some("cached notice".into())
            });
            assert!(notice.is_none(), "{input:?}");
            assert_eq!(calls.get(), 0, "metadata must not read/create update flags or spawn a checker: {input:?}");
        }
    }

    #[test]
    fn ordinary_commands_keep_lazy_notices_and_explicit_update_owns_its_output() {
        for (input, expected) in [(vec!["list"], 1), (vec!["launch", "--tool", "codex"], 1), (vec!["send", "--", "--help"], 1), (vec!["update", "--check"], 0), (vec!["relay-worker"], 0)] {
            let argv = sv(&input);
            let calls = std::cell::Cell::new(0);
            let notice = command_update_notice(&resolve_action(&argv), &argv, || {
                calls.set(calls.get() + 1);
                Some("cached notice".into())
            });
            assert_eq!(calls.get(), expected, "{input:?}");
            assert_eq!(notice.as_deref(), if expected == 1 { Some("cached notice") } else { None }, "{input:?}");
        }
    }

    #[test]
    fn unknown_commands_have_no_legacy_launch_handling() {
        assert!(unknown_command_message("evnets", &[]).contains("Did you mean: events?"));
        assert_eq!(unknown_command_message("3", &[]), "Unknown command '3'");
        assert_eq!(unknown_command_message("codex", &[]), "Unknown command 'codex'");
    }

    #[test]
    fn hook_tools_are_derived_from_released_specs() {
        let actual = crate::cli::commands::hooks::hook_tools();
        let expected: Vec<Tool> = crate::integrations::registry::ALL.iter().filter(|spec| spec.released).map(|spec| spec.tool).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn launch_tools_covers_every_released_spec() {
        // Launch recognition is derived from canonical Tool parsing; every
        // released canonical name and alias must therefore route automatically.
        for spec in crate::integrations::registry::ALL {
            if !spec.released {
                continue;
            }
            assert!(!is_launch_tool(spec.name), "router did not recognise released tool {}", spec.name);
            for alias in spec.aliases {
                assert!(!is_launch_tool(alias), "router did not recognise alias {} for {}", alias, spec.name);
            }
        }
    }

    #[test]
    fn read_dev_root_missing_scope_does_not_start_daemon() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_dev_root_from_kv(&dir.path().join("absent").join("agentx.db")), None);
        assert!(!dir.path().join("absent").exists());
    }

    #[test]
    fn is_config_dev_root_invocation_matches_expected_shapes() {
        assert!(is_config_dev_root_invocation(&sv(&["config", "dev_root"])));
        assert!(is_config_dev_root_invocation(&sv(&["config", "dev_root", "/tmp/x"])));
        assert!(is_config_dev_root_invocation(&sv(&["config", "dev_root", "--unset"])));
        assert!(is_config_dev_root_invocation(&sv(&["--iam", "vami", "config", "dev_root", "/tmp/x"])));

        assert!(!is_config_dev_root_invocation(&sv(&["config"])));
        assert!(!is_config_dev_root_invocation(&sv(&["config", "terminal", "tmux"])));
        assert!(!is_config_dev_root_invocation(&sv(&["list"])));
        assert!(!is_config_dev_root_invocation(&[]));
    }

    #[test]
    fn is_update_invocation_matches_expected_shapes() {
        assert!(is_update_invocation(&sv(&["update"])));
        assert!(is_update_invocation(&sv(&["update", "--check"])));
        assert!(is_update_invocation(&sv(&["--go", "update"])));
        assert!(is_update_invocation(&sv(&["--iam", "lovi", "update", "--check"])));
        assert!(is_update_invocation(&sv(&["help", "update"])));
        assert!(!is_update_invocation(&sv(&["config", "update"])));
        assert!(!is_update_invocation(&sv(&["send", "@lovi", "--", "update"])));
        assert!(!is_update_invocation(&[]));
    }

    #[test]
    fn installed_resolution_does_not_normalize_global_help_or_version() {
        for flag in ["--help", "-h", "--version", "-v", "help"] {
            assert!(matches!(resolve_action(&sv(&["--iam", "test", flag])), Action::Command { .. }));
        }
    }

    // ── resolve_action tests ────────────────────────────────────────────

    #[test]
    fn no_args_runs_tui() {
        let action = resolve_action(&[]);
        assert_eq!(action, Action::Tui);
    }

    #[test]
    fn pty_mode() {
        let action = resolve_action(&sv(&["pty", "claude"]));
        assert_eq!(action, Action::Pty { args: sv(&["claude"]) });
    }

    #[test]
    fn pty_mode_with_args() {
        let action = resolve_action(&sv(&["pty", "claude", "--arg1"]));
        assert_eq!(action, Action::Pty { args: sv(&["claude", "--arg1"]) });
    }

    // ── Hook detection ──────────────────────────────────────────────────

    #[test]
    fn claude_hooks_detected() {
        for hook_name in Tool::Claude.hooks() {
            let action = resolve_action(&sv(&[hook_name]));
            match &action {
                Action::Hook { hook, .. } => assert_eq!(hook, hook_name),
                _ => panic!("expected Hook for {}, got {:?}", hook_name, action),
            }
        }
    }

    #[test]
    fn gemini_hooks_detected() {
        for hook_name in Tool::Gemini.hooks() {
            let action = resolve_action(&sv(&[hook_name]));
            match &action {
                Action::Hook { hook, .. } => assert_eq!(hook, hook_name),
                _ => panic!("expected Hook for {}, got {:?}", hook_name, action),
            }
        }
    }

    #[test]
    fn codex_hook_detected() {
        let action = resolve_action(&sv(&["codex-sessionstart"]));
        match &action {
            Action::Hook { hook, .. } => {
                assert_eq!(hook, "codex-sessionstart");
            }
            _ => panic!("expected Hook, got {:?}", action),
        }
    }

    #[test]
    fn opencode_hooks_detected() {
        for hook_name in Tool::OpenCode.hooks() {
            let action = resolve_action(&sv(&[hook_name]));
            match &action {
                Action::Hook { hook, .. } => assert_eq!(hook, hook_name),
                _ => panic!("expected Hook for {}, got {:?}", hook_name, action),
            }
        }
    }

    // ── Command detection ───────────────────────────────────────────────

    #[test]
    fn cli_commands_detected() {
        for cmd_name in COMMANDS {
            let action = resolve_action(&sv(&[cmd_name]));
            match &action {
                Action::Command { cmd, .. } => assert_eq!(cmd, cmd_name),
                _ => panic!("expected Command for {}, got {:?}", cmd_name, action),
            }
        }
    }

    #[test]
    fn command_with_args() {
        let action = resolve_action(&sv(&["send", "@luna", "--", "hello"]));
        match &action {
            Action::Command { cmd, args } => {
                assert_eq!(cmd, "send");
                assert_eq!(*args, sv(&["send", "@luna", "--", "hello"]));
            }
            _ => panic!("expected Command, got {:?}", action),
        }
    }

    // ── Launch detection ────────────────────────────────────────────────

    #[test]
    fn explicit_launch_and_resume_actions_are_recognized() {
        for args in [sv(&["launch", "--tool", "codex", "--count", "2", "--", "--model", "x"]), sv(&["--iam", "caller", "--go", "launch", "--tool", "claude"]), sv(&["resume", "luna"]), sv(&["fork", "luna"])] {
            assert_eq!(resolve_action(&args), Action::Launch { args });
        }
    }
    #[test]
    fn bare_tools_and_numeric_prefixes_are_unknown_commands() {
        for args in [sv(&["codex"]), sv(&["claude"]), sv(&["3", "codex"]), sv(&["agy"])] {
            assert!(matches!(resolve_action(&args), Action::Command { .. }));
            assert!(!is_command(&args[0]));
        }
    }
    #[test]
    fn global_scanner_preserves_native_and_prompt_controls() {
        let args = sv(&["launch", "--tool", "codex", "--agentx-prompt", "--help", "--", "--iam", "native", "--go", "--help"]);
        let (remaining, flags, help) = extract_global_flags_full(&args);
        assert_eq!(remaining, args);
        assert_eq!(flags, GlobalFlags::default());
        assert!(!help);
    }

    // ── Global flags ────────────────────────────────────────────────────

    #[test]
    fn extract_name_flag() {
        let (remaining, flags) = extract_global_flags(&sv(&["--iam", "foo", "list"]));
        assert_eq!(remaining, sv(&["list"]));
        assert_eq!(flags.name, Some("foo".to_string()));
        assert!(!flags.go);
    }

    #[test]
    fn extract_go_flag() {
        let (remaining, flags) = extract_global_flags(&sv(&["--go", "stop", "all"]));
        assert_eq!(remaining, sv(&["stop", "all"]));
        assert!(flags.go);
        assert!(flags.name.is_none());
    }

    #[test]
    fn extract_both_flags() {
        let (remaining, flags) = extract_global_flags(&sv(&["--iam", "bot", "--go", "stop"]));
        assert_eq!(remaining, sv(&["stop"]));
        assert_eq!(flags.name, Some("bot".to_string()));
        assert!(flags.go);
    }

    #[test]
    fn flags_before_command_extracted() {
        let (remaining, flags) = extract_global_flags(&sv(&["--iam", "x", "send", "@luna", "--", "hi"]));
        assert_eq!(remaining, sv(&["send", "@luna", "--", "hi"]));
        assert_eq!(flags.name, Some("x".to_string()));
    }

    #[test]
    fn flags_after_command_not_extracted() {
        // --iam after a positional stays in rest (clap's trailing_var_arg behavior).
        // This is correct: global flags belong before the command. The full argv
        // is still passed to handlers, which extract --iam at the command level.
        let (remaining, flags) = extract_global_flags(&sv(&["send", "--iam", "x", "@luna", "--", "hi"]));
        assert_eq!(remaining, sv(&["send", "--iam", "x", "@luna", "--", "hi"]));
        assert!(flags.name.is_none());
    }

    // ── extract_global_flags_full (full argv scan) ────────────────────

    #[test]
    fn full_extract_name_after_command() {
        // --iam after command token is extracted (unlike clap-based version)
        let (remaining, flags, help) = extract_global_flags_full(&sv(&["send", "--iam", "vami", "@luna", "--", "hi"]));
        assert_eq!(remaining, sv(&["send", "@luna", "--", "hi"]));
        assert_eq!(flags.name, Some("vami".to_string()));
        assert!(!help);
    }

    #[test]
    fn full_extract_name_before_command() {
        let (remaining, flags, help) = extract_global_flags_full(&sv(&["--iam", "vami", "list", "-v"]));
        assert_eq!(remaining, sv(&["list", "-v"]));
        assert_eq!(flags.name, Some("vami".to_string()));
        assert!(!help);
    }

    #[test]
    fn full_extract_respects_separator() {
        // --iam after -- should NOT be extracted (it's message text)
        let (remaining, flags, _) = extract_global_flags_full(&sv(&["send", "@luna", "--", "--iam", "not-a-flag"]));
        assert_eq!(remaining, sv(&["send", "@luna", "--", "--iam", "not-a-flag"]));
        assert!(flags.name.is_none());
    }

    #[test]
    fn full_extract_help_detected() {
        let (remaining, flags, help) = extract_global_flags_full(&sv(&["send", "--iam", "vami", "--help"]));
        assert_eq!(remaining, sv(&["send"]));
        assert_eq!(flags.name, Some("vami".to_string()));
        assert!(help);
    }

    #[test]
    fn full_extract_help_short() {
        let (_, _, help) = extract_global_flags_full(&sv(&["list", "-h"]));
        assert!(help);
    }

    #[test]
    fn full_extract_help_after_separator_ignored() {
        // --help in message text after -- is not a help request
        let (_, _, help) = extract_global_flags_full(&sv(&["send", "@luna", "--", "--help"]));
        assert!(!help);
    }

    #[test]
    fn full_extract_go_flag() {
        let (remaining, flags, _) = extract_global_flags_full(&sv(&["stop", "--go", "all"]));
        assert_eq!(remaining, sv(&["stop", "all"]));
        assert!(flags.go);
    }

    #[test]
    fn full_extract_combined() {
        let (remaining, flags, help) = extract_global_flags_full(&sv(&["config", "--iam", "bot", "--go", "-h"]));
        assert_eq!(remaining, sv(&["config"]));
        assert_eq!(flags.name, Some("bot".to_string()));
        assert!(flags.go);
        assert!(help);
    }

    // ── Version / Help / NewTerminal ────────────────────────────────────

    #[test]
    fn version_flag() {
        assert_eq!(resolve_action(&sv(&["--version"])), Action::Version);
        assert_eq!(resolve_action(&sv(&["-v"])), Action::Version);
    }

    #[test]
    fn help_flag() {
        assert_eq!(resolve_action(&sv(&["--help"])), Action::Help);
        assert_eq!(resolve_action(&sv(&["-h"])), Action::Help);
        assert_eq!(resolve_action(&sv(&["help"])), Action::Help);
        assert_eq!(resolve_action(&sv(&["help", "relay-worker"])), Action::TopicHelp { args: sv(&["relay-worker"]) });
        assert_eq!(print_topic_help(&sv(&["relay-worker"])), 1);
        assert_eq!(print_topic_help(&sv(&["events", "sub"])), 0);
        assert_eq!(print_topic_help(&sv(&["3", "claude"])), 1);
    }

    #[test]
    fn new_terminal_flag() {
        assert_eq!(resolve_action(&sv(&["--new-terminal"])), Action::NewTerminal);
    }

    #[test]
    fn new_terminal_after_flags() {
        assert_eq!(resolve_action(&sv(&["--iam", "foo", "--new-terminal"])), Action::NewTerminal);
    }

    #[test]
    fn dashboard_selectors_route_without_consuming_subcommand_filters() {
        use crate::cli::workspace::DashboardArgs;
        let argv = sv(&["--dir", ".", "--label", "task=build", "--new-terminal"]);
        assert_eq!(resolve_action(&argv), Action::Tui);
        let args = DashboardArgs::try_parse_from(std::iter::once("agentx".into()).chain(argv)).unwrap();
        assert_eq!(args.filters.dir.as_deref(), Some("."));
        assert_eq!(args.filters.labels, ["task=build"]);
        assert!(args.new_terminal);
        assert!(DashboardArgs::try_parse_from(["agentx", "--dir", ".", "attach"]).is_err());
        assert!(matches!(resolve_action(&sv(&["a", "--dir", "."])), Action::Command { cmd, .. } if cmd == "attach"));
        let (args, _, help) = extract_global_flags_full(&sv(&["--workspace", "--help"]));
        assert!(!help);
        assert_eq!(args, sv(&["--workspace", "--help"]));
    }

    // ── Hook with global flags ──────────────────────────────────────────

    #[test]
    fn hook_with_name_flag() {
        let action = resolve_action(&sv(&["--iam", "foo", "sessionstart"]));
        match &action {
            Action::Hook { hook, args } => {
                assert_eq!(hook, "sessionstart");
                assert_eq!(*args, sv(&["--iam", "foo", "sessionstart"]));
            }
            _ => panic!("expected Hook, got {:?}", action),
        }
    }

    #[test]
    fn send_not_found_gets_external_sender_hint_outside_ai() {
        let err = AgentxError::NotFound("Instance 'healthcheck' not found. Run 'agentx start --as healthcheck' to reclaim your identity.".into());
        let msg = maybe_external_send_name_hint("send", Some("healthcheck"), false, None, false, &err).expect("expected hint");
        assert!(msg.contains("Hint: If 'healthcheck' is an external sender"));
        assert!(msg.contains("send --from healthcheck"));
    }

    #[test]
    fn send_not_found_keeps_agent_recovery_path_inside_ai() {
        let err = AgentxError::NotFound("Instance 'luna' not found. Run 'agentx start --as luna' to reclaim your identity.".into());
        let msg = maybe_external_send_name_hint("send", Some("luna"), false, Some("pid-123"), true, &err);
        assert!(msg.is_none());
    }

    #[test]
    fn non_not_found_name_errors_do_not_get_external_sender_hint() {
        let err = AgentxError::InvalidInput("Invalid instance name 'Invalid-Name!'. Use base name only (lowercase letters, numbers, underscore).".into());
        let msg = maybe_external_send_name_hint("send", Some("Invalid-Name!"), false, None, false, &err);
        assert!(msg.is_none());
    }

    // ── is_hook / is_command ────────────────────────────────────────────

    #[test]
    fn hook_registry_complete() {
        assert!(is_hook("poll"));
        assert!(is_hook("sessionstart"));
        assert!(is_hook("gemini-beforeagent"));
        assert!(is_hook("codex-sessionstart"));
        assert!(is_hook("opencode-start"));
        assert!(is_hook("pi-start"));
        assert!(is_hook("copilot-sessionstart"));
        assert!(is_hook("qoder-sessionstart"));
        assert!(!is_launch_tool("qoder"));
        assert!(!is_launch_tool("qodercli"));
        assert!(!is_hook("send"));
        assert!(!is_hook("unknown"));
    }

    #[test]
    fn hooks_do_not_collide_with_commands_or_launch_tools() {
        for tool in [Tool::Claude, Tool::Gemini, Tool::Codex, Tool::OpenCode, Tool::Copilot, Tool::Qoder, Tool::Pi, Tool::Omp, Tool::Hermes] {
            for hook in tool.hooks() {
                assert!(!COMMANDS.contains(hook), "{hook} collides with command");
                assert!(!is_launch_tool(hook), "{hook} collides with launch tool");
            }
        }
    }

    #[test]
    fn command_registry_complete() {
        assert!(is_command("send"));
        assert!(is_command("list"));
        assert!(is_command("run"));
        assert!(!is_command("poll"));
        assert!(!is_command("claude"));
    }

    // ── Dev root re-exec path building ──────────────────────────────────

    #[test]
    fn is_same_file_works() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        assert!(is_same_file(tmp.path(), tmp.path()));
    }
}
