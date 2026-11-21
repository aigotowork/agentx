//! Unified launcher pipeline for released AI-tool integrations.
//!
//!
//! Provides a single entry point for launching all supported AI tools
//! with consistent batch tracking, environment setup, and error handling.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context as _, Result, bail};
use rand::RngExt;
use serde_json::json;

#[cfg(test)]
use crate::agents::binding as instance_binding;
use crate::agents::launch_service::{self, Reservation};
#[cfg(test)]
use crate::agents::names as instance_names;
use crate::config::{self, AgentxConfig};
#[cfg(test)]
use crate::db::AgentxDb;
use crate::integrations::detection::tool_marker_vars;
use crate::integrations::launch_args::validation::{
    ANTIGRAVITY_REJECTED_ARGS, GEMINI_REJECTED_ARGS, KILO_REJECTED_ARGS, KIMI_REJECTED_ARGS, OMP_REJECTED_ARGS, OPENCODE_REJECTED_ARGS, PI_REJECTED_ARGS, QODER_REJECTED_ARGS, validate_grok_args, validate_rejected_args,
};
use crate::integrations::launch_args::{codex as codex_preprocessing, copilot as copilot_preprocessing, cursor as cursor_preprocessing, opencode as opencode_preprocessing, qoder as qoder_preprocessing};
use crate::integrations::tool::Tool;
use crate::paths;
use crate::runtime::terminal;
use crate::service::client::Client;
use crate::shared::constants::AGENTX_IDENTITY_VARS;

/// What `agentx launch --tool <tool>` launches: a launchable [`Tool`](crate::integrations::tool::Tool), and
/// for Claude whether it was asked for as `claude-pty` (always PTY-wrapped).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaunchTool {
    tool: crate::integrations::tool::Tool,
    claude_pty: bool,
}

impl From<crate::integrations::tool::Tool> for LaunchTool {
    fn from(tool: crate::integrations::tool::Tool) -> Self {
        Self { tool, claude_pty: false }
    }
}

impl LaunchTool {
    pub const CLAUDE_PTY: Self = Self { tool: crate::integrations::tool::Tool::Claude, claude_pty: true };

    /// Parse a spec name or alias, or `claude-pty`. Adhoc is not launchable.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self> {
        if s == Self::CLAUDE_PTY.as_str() {
            return Ok(Self::CLAUDE_PTY);
        }
        match s.parse::<crate::integrations::tool::Tool>() {
            Ok(tool) if tool != crate::integrations::tool::Tool::Adhoc => Ok(tool.into()),
            _ => bail!("Unknown tool: {}", s),
        }
    }

    pub fn as_str(&self) -> &'static str {
        if self.claude_pty { "claude-pty" } else { self.tool.as_str() }
    }

    pub fn tool(&self) -> crate::integrations::tool::Tool {
        self.tool
    }

    /// True for `claude-pty`.
    pub fn is_claude_pty(&self) -> bool {
        self.claude_pty
    }

    pub fn spec(&self) -> &'static crate::integrations::registry::IntegrationSpec {
        self.tool.spec()
    }

    /// Base tool name (`claude` for `claude-pty`).
    pub fn base_tool(&self) -> &'static str {
        self.tool.as_str()
    }

    /// Whether this tool uses the PTY wrapper.
    pub fn uses_pty(&self) -> bool {
        self.claude_pty || self.spec().launch.uses_pty_default
    }

    /// Executable name on PATH for this tool.
    pub fn cli_binary(&self) -> &'static str {
        self.spec().cli_binary
    }
}

/// How the child process is hosted. Computed from (tool, background, pty) at
/// launch time so dispatch doesn't have to re-derive the combination.
///
/// - `InteractiveVisible`: foreground, user-visible terminal. All tools.
/// - `HeadlessPty`:       background, PTY wrapper in a detached runner. Default
///   for gemini/codex/opencode/kilo/pi/omp/antigravity/cursor/kimi/copilot/qoder and for default claude `--headless`.
/// - `NativePrint`:       background, direct claude spawn in print mode
///   (`-p --output-format stream-json --verbose`). Claude only, opt-in via an
///   explicit `-p`/`--print`; kept alive across turns by agentx's stop-hook loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchBackend {
    InteractiveVisible,
    HeadlessPty,
    NativePrint,
}

impl LaunchBackend {
    /// Resolve from the already-prepared (tool, background) pair.
    ///
    /// The PTY-vs-print decision for Claude is encoded in the [`LaunchTool`]
    /// chosen by `launch()`: plain `claude` is the `-p`/`--print` surface
    /// (→ `NativePrint`), `claude-pty` the live PTY surface (→ `HeadlessPty`).
    /// Background Claude defaults to `claude-pty`; it only becomes
    /// `NativePrint` when the caller passes `-p`/`--print`.
    pub fn resolve(tool: &LaunchTool, background: bool) -> Self {
        use crate::integrations::registry::BackgroundMode;
        if !background {
            return LaunchBackend::InteractiveVisible;
        }
        match tool.spec().launch.background {
            BackgroundMode::NativePrint if !tool.is_claude_pty() => LaunchBackend::NativePrint,
            BackgroundMode::NativePrint | BackgroundMode::HeadlessPty => LaunchBackend::HeadlessPty,
            BackgroundMode::Unsupported => unreachable!("{} is not launchable", tool.as_str()),
        }
    }
}

#[cfg(test)]
pub(crate) fn prepare_launch_execution(tool: &LaunchTool, cli_args: &[String], config: &AgentxConfig, headless: bool) -> (Vec<String>, bool) {
    prepare_resolved_execution(tool, &merge_tool_args(tool, cli_args, config), headless)
}

pub(crate) fn prepare_resolved_execution(tool: &LaunchTool, args: &[String], headless: bool) -> (Vec<String>, bool) {
    let mut merged_args = args.to_vec();
    let background = headless || is_background_from_args(tool, &merged_args);

    // Print mode needs stream-json for the stop-hook loop. Keep this deliberately
    // grammar-free: agentx appends its required defaults and lets Claude resolve
    // duplicates or reject incompatible combinations.
    if tool.tool() == crate::integrations::tool::Tool::Claude && background && args_contain_any(&merged_args, &["-p", "--print"]) {
        merged_args.extend(["--output-format".to_string(), "stream-json".to_string(), "--verbose".to_string()]);
    }

    (merged_args, background)
}

/// Split configured defaults before composing explicit arguments.
fn config_arg_tokens(config_args: &str) -> Vec<String> {
    if config_args.is_empty() {
        Vec::new()
    } else {
        // Don't silently drop hand-edited config args on a parse error (e.g. an
        // unterminated quote) — surface it so the launch isn't quietly missing
        // flags the user configured.
        crate::integrations::launch_args::shell::shell_split(config_args, cfg!(windows)).unwrap_or_else(|err| {
            eprintln!("agentx: ignoring malformed configured args ({err}): {config_args}");
            Vec::new()
        })
    }
}

pub(crate) fn merge_tool_args(tool: &LaunchTool, cli_args: &[String], config: &AgentxConfig) -> Vec<String> {
    crate::integrations::launch_args::overrides::override_args(tool.tool(), &config_arg_tokens(config.tool_args(tool.tool())), cli_args)
}

fn args_contain_any(args: &[String], needles: &[&str]) -> bool {
    args.iter().any(|arg| needles.contains(&arg.as_str()))
}

/// Check if args indicate background/headless mode.
pub(crate) fn is_background_from_args(tool: &LaunchTool, args: &[String]) -> bool {
    // Other tools are always hosted in agentx's PTY. Their native
    // non-interactive modes are rejected by validate_tool_args.
    tool.spec().launch.background == crate::integrations::registry::BackgroundMode::NativePrint && args_contain_any(args, &["-p", "--print"])
}

/// Launch parameters.
#[derive(Clone)]
pub struct LaunchParams {
    pub(crate) labels: std::collections::BTreeMap<String, String>,
    pub(crate) bindings: Vec<std::path::PathBuf>,
    pub tool: String,
    pub count: usize,
    pub args: Vec<String>,
    /// Raw user/config args to persist for future resume, before agentx injections.
    pub persisted_args: Option<Vec<String>>,
    pub(crate) launch_profile: Option<crate::runtime::launch_profile::LaunchProvenance>,
    pub(crate) collaboration: Option<crate::agents::workspaces::LaunchCollaboration>,
    /// Session id being resumed, inherited by the recreated instance row so a
    /// kill before the tool's first turn (no hook re-bind yet) stays resumable.
    pub prior_session_id: Option<String>,
    /// Saved unread position for a tracked resume; zero is a valid cursor.
    pub resume_cursor: Option<i64>,
    pub title: Option<String>,
    pub system_prompt: Option<String>,
    pub initial_prompt: Option<String>,
    pub background: bool,
    /// Codex-only foreground execution on inherited standard streams.
    pub stdio: bool,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub launcher: Option<String>,
    pub run_here: Option<bool>,
    pub batch_id: Option<String>,
    pub name: Option<String>,
    pub skip_validation: bool,
    pub terminal: Option<String>,
    pub append_reply_handoff: bool,
}

impl Default for LaunchParams {
    fn default() -> Self {
        Self {
            labels: Default::default(),
            bindings: Vec::new(),
            tool: "claude".to_string(),
            count: 1,
            args: Vec::new(),
            persisted_args: None,
            launch_profile: None,
            collaboration: None,
            prior_session_id: None,
            resume_cursor: None,
            title: None,
            system_prompt: None,
            initial_prompt: None,
            background: false,
            stdio: false,
            cwd: None,
            env: None,
            launcher: None,
            run_here: None,
            batch_id: None,
            name: None,
            skip_validation: false,
            terminal: None,
            append_reply_handoff: true,
        }
    }
}

/// Resolve one immutable collaboration choice before reserving a participant.
pub(crate) fn resolve_launch_collaboration(scope: &Path, cwd: &Path, environment: &HashMap<String, String>, dir: Option<&str>, workspace: Option<&str>, role: Option<&str>) -> Result<crate::agents::workspaces::LaunchCollaboration> {
    let workspace = crate::config::roles::select_workspace(cwd, environment, dir, workspace)?;
    let role = role.map(|selector| crate::config::roles::resolve_role(scope, workspace.as_deref(), selector)).transpose()?;
    Ok(crate::agents::workspaces::LaunchCollaboration { workspace: workspace.map(|path| path.to_string_lossy().into_owned()), role, endpoint: false, instruction_additions: None })
}

/// Freeze only raw fresh-launch additions. Resume/fork identity prompts remain
/// transient and must never replace the adopted instruction history.
fn retain_instruction_additions(params: &mut LaunchParams, fresh: bool) {
    let Some(collaboration) = params.collaboration.as_mut() else { return };
    if fresh && collaboration.instruction_additions.is_none() {
        collaboration.instruction_additions = params.system_prompt.clone();
    }
    if collaboration.instruction_additions.is_some() && params.system_prompt == collaboration.instruction_additions {
        // The fixed bootstrap now carries these bytes for every native provider.
        params.system_prompt = None;
    }
}

/// Launch result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LaunchResult {
    pub tool: String,
    pub batch_id: String,
    pub launched: usize,
    pub failed: usize,
    pub background: bool,
    pub log_files: Vec<String>,
    pub handles: Vec<serde_json::Value>,
    pub errors: Vec<serde_json::Value>,
    /// Native stdio child status; absent for asynchronous/terminal launches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdio_exit_code: Option<i32>,
    /// Local presentation intent only; never forwarded through Relay.
    #[serde(skip)]
    pub(crate) attach_participant: Option<crate::agents::attachments::ParticipantRef>,
    /// Exact successful terminal reservations for local Management receipts.
    /// Keep the CLI/Relay result wire unchanged.
    #[serde(skip)]
    pub(crate) launched_participants: Vec<crate::agents::attachments::ParticipantRef>,
}

/// Predict if launch will block current terminal (run in same window).
/// Find tool executable path with fallbacks.
/// Claude has special fallback locations; other tools just use PATH.
fn find_tool_path(tool: &str) -> Option<String> {
    crate::runtime::terminal::which_bin(tool)
}

/// Check if tool CLI is installed (PATH + fallbacks).
fn is_tool_installed(tool: &str) -> bool {
    find_tool_path(tool).is_some()
}

/// Whether a launch replaces the current process instead of opening a new
/// terminal. Without an interactive stdin (TUI RPC, pipes, scripts) there is no
/// terminal to take over, so only an explicit `--run-here`/`here` does it.
pub fn will_run_in_current_terminal(count: usize, background: bool, run_here: Option<bool>, terminal: Option<&str>, inside_ai_tool: bool, stdin_is_terminal: bool) -> bool {
    if let Some(rh) = run_here {
        return rh;
    }
    // terminal=here forces current terminal
    if terminal == Some("here") {
        return true;
    }
    if terminal.is_some_and(|mode| mode != "default") {
        return false;
    }
    if inside_ai_tool {
        return false;
    }
    if background || !stdin_is_terminal {
        return false;
    }
    count == 1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchEnvRegime {
    HumanShell,
    ContaminatedParent,
    RunHere,
}

pub fn launch_env_regime(run_here: bool, inside_ai_tool: bool) -> LaunchEnvRegime {
    if run_here {
        LaunchEnvRegime::RunHere
    } else if contaminated_parent_with_inside_ai_tool(inside_ai_tool) {
        LaunchEnvRegime::ContaminatedParent
    } else {
        LaunchEnvRegime::HumanShell
    }
}

pub fn contaminated_parent() -> bool {
    contaminated_parent_with_inside_ai_tool(crate::shared::platform::is_inside_ai_tool())
}

fn contaminated_parent_with_inside_ai_tool(inside_ai_tool: bool) -> bool {
    inside_ai_tool || std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some() || std::env::vars_os().any(|(key, _)| key.to_string_lossy().starts_with("CARGO_"))
}

/// Build base environment, then overlay config.toml + ~/.agentx/env (these win).
///
/// This makes the new-window/runner-script path behave like the PTY path
/// (`Command::new` inherits parent env by default). The runner script
/// already `unset`s TOOL_MARKER_VARS + AGENTX_IDENTITY_VARS before exec,
/// so those categories are safe to include here (they're cleared in-script).
pub fn build_launch_env(agentx_config: &AgentxConfig, regime: LaunchEnvRegime) -> HashMap<String, String> {
    build_launch_env_with_resolver(agentx_config, regime, crate::runtime::shell_env::resolved_shell_env)
}

/// Apply the launcher's inherited `AGENTX_NOTES` to a child instance env as a
/// fallback only.
///
/// `instance_env` (via `base_env`) already carries any notes resolved from the
/// explicit sources — config.toml, `~/.agentx/env`, and `LaunchParams.env` — and
/// those must win. The inherited parent value only fills the gap for a nested
/// launch where no explicit notes were configured, so it must not clobber a
/// value already present (including an intentional empty string used to clear
/// notes).
fn apply_inherited_notes(instance_env: &mut HashMap<String, String>, inherited: Option<String>) {
    if let Some(val) = inherited {
        instance_env.entry("AGENTX_NOTES".to_string()).or_insert(val);
    }
}

fn reply_handoff_suffix(launcher_name: &str, append_reply_handoff: bool, excluded: &std::collections::HashSet<&str>) -> String {
    if append_reply_handoff && launcher_name != "api" && launcher_name != "user" && !excluded.contains("handoff") { format!("\n\n{}", crate::runtime::bootstrap::HANDOFF_SUFFIX.replace("{launcher_name}", launcher_name)) } else { String::new() }
}

/// Start a new Grok conversation under an explicit session id.
///
/// Without one the TUI opens on its welcome screen over a hidden "home"
/// session, and never draws turns another client (agentx's queue) runs there.
/// `--session-id` opens the conversation view directly. Resume/continue
/// already open a conversation.
fn ensure_grok_session_id(args: &mut Vec<String>) {
    let opens_conversation = args.iter().take_while(|arg| *arg != "--").any(|arg| {
        let flag = arg.split('=').next().unwrap_or(arg);
        matches!(flag, "--resume" | "--load" | "--continue" | "--session-id")
            || arg.strip_prefix('-').filter(|short| !short.starts_with('-')).is_some_and(|short| {
                // clap accepts attached values (-rID, -sUUID) and boolean
                // clusters (-cmMODEL). Stop at any value-taking option so a
                // model/worktree name containing c/r/s is never a selector.
                short.chars().take_while(|c| matches!(c, 'c' | 'r' | 's' | 'h' | 'v')).any(|c| matches!(c, 'c' | 'r' | 's'))
            })
    });
    if !opens_conversation {
        args.insert(0, "--session-id".to_string());
        args.insert(1, uuid::Uuid::new_v4().to_string());
    }
}

/// Prepend `extra` to Grok's `--rules` (text appended to its system prompt).
fn inject_grok_rules(args: &mut Vec<String>, extra: &str) {
    if extra.is_empty() {
        return;
    }
    let mut i = 0;
    // Flags only: after `--` it is the prompt, whatever it looks like.
    while i < args.len() && args[i] != "--" {
        let token = &args[i];
        if token == "--rules" {
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                args[i + 1] = format!("{}\n\n{}", extra, args[i + 1]);
            } else {
                args.insert(i + 1, extra.to_string());
            }
            return;
        }
        if let Some(rest) = token.strip_prefix("--rules=") {
            args[i] = format!("--rules={}\n\n{}", extra, rest);
            return;
        }
        i += 1;
    }
    args.insert(0, "--rules".to_string());
    args.insert(1, extra.to_string());
}

#[cfg(test)]
fn build_codex_bootstrap(db: &AgentxDb, agentx_dir: &Path, instance_name: &str, background: bool, instance_env: &HashMap<String, String>) -> String {
    build_launch_bootstrap(db, agentx_dir, instance_name, "codex", background, instance_env)
}

#[cfg(test)]
fn build_grok_bootstrap(db: &AgentxDb, agentx_dir: &Path, instance_name: &str, background: bool, instance_env: &HashMap<String, String>) -> String {
    build_launch_bootstrap(db, agentx_dir, instance_name, "grok", background, instance_env)
}

#[cfg(test)]
fn build_launch_bootstrap(db: &AgentxDb, agentx_dir: &Path, instance_name: &str, tool: &str, background: bool, instance_env: &HashMap<String, String>) -> String {
    // Codex and Grok take their bootstrap at launch, so render it from the env
    // the instance will run with. AGENTX_BACKGROUND is only added to the runner
    // env later, hence the explicit override.
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut ctx = crate::shared::AgentxContext::from_env(instance_env, cwd);
    ctx.agentx_dir = agentx_dir.to_path_buf();
    ctx.is_launched = true;
    ctx.is_background = background;
    crate::runtime::bootstrap::get_bootstrap(db, &ctx, instance_name, tool)
}

fn launch_request(client: &Client, request: launch_service::Request) -> Result<launch_service::Response> {
    match client.call(crate::service::Request::Participants(crate::agents::service::Request::Launch(request)))? {
        crate::service::Response::Participants(crate::agents::service::Response::Launch(response)) => Ok(response),
        _ => bail!("unexpected participant launch response"),
    }
}

fn record_terminal(client: &Client, reservation: &Reservation, requested: Option<&str>, effective: &str, background: Option<launch_service::BackgroundProcess>) -> Result<()> {
    launch_request(client, launch_service::Request::TerminalLaunched { reservation: reservation.clone(), requested: requested.map(str::to_owned), effective: effective.to_owned(), background })?;
    Ok(())
}

/// Report failure before a Provider child acquired its own ProcessHost fence.
/// Only the originating launch's retained participant identity may be observed.
pub(crate) fn report_spawn_failure(context: &crate::runtime::context::AgentxContext, name: &str, detail: &str) -> Result<()> {
    let participant: crate::agents::attachments::ParticipantRef = serde_json::from_str(context.raw_env.get("AGENTX_LAUNCH_PARTICIPANT").context("launch participant fence missing")?)?;
    if participant.canonical_name != name {
        bail!("spawn failure name differs from launch participant");
    }
    let launch_cursor = context.raw_env.get("AGENTX_LAUNCH_START_EVENT_ID").context("launch cursor missing")?.parse()?;
    let client = Client::connect(&context.agentx_dir, super::host_client::invocation(context))?;
    if context.raw_env.get("AGENTX_LAUNCH_GENERATION").map(String::as_str) != Some(client.info().database_generation.as_str()) {
        bail!("launch failure belongs to another database generation");
    }
    launch_request(&client, launch_service::Request::SpawnFailed { participant, launch_cursor, detail: detail.to_owned() })?;
    Ok(())
}

fn build_launch_bootstrap_client(client: &Client, scope: &Path, cwd: &Path, name: &str, tool: &str, background: bool, environment: &HashMap<String, String>) -> Result<String> {
    let mut context = crate::runtime::context::AgentxContext::from_env(environment, cwd.to_path_buf());
    context.agentx_dir = scope.to_path_buf();
    context.is_launched = true;
    context.is_background = background;
    super::bootstrap::get_instructions_client(client, &context, name, tool)
}

fn finish_supervisor(client: &Client, context: &crate::runtime::context::AgentxContext, reservation: &Reservation) -> Result<String> {
    let refreshed = match client.call(crate::service::Request::Participants(crate::agents::service::Request::Resolve)) {
        Ok(crate::service::Response::Participants(crate::agents::service::Response::Actor(actor))) => {
            // Explicitly register the current canonical row before refreshing
            // the launch fence. The old incarnation alone cannot stop it.
            let host = super::process_host::ProcessHost::register_with_client(client, context, &actor.participant.canonical_name, std::process::id(), None)?;
            match launch_request(client, launch_service::Request::RefreshSupervisor { reservation: reservation.clone(), registration: host.registration().clone() })? {
                launch_service::Response::Reserved(refreshed) => refreshed,
                _ => bail!("unexpected supervisor refresh response"),
            }
        }
        Err(crate::service::ServiceError::IdentityRequired(_) | crate::service::ServiceError::NotFound(_)) => reservation.clone(),
        Err(error) => return Err(error.into()),
        _ => bail!("unexpected supervisor identity response"),
    };
    match launch_request(client, launch_service::Request::FinishSupervisor(refreshed))? {
        launch_service::Response::Finished { name } => Ok(name),
        _ => bail!("unexpected supervisor finish response"),
    }
}

fn build_launch_env_captured(config: &AgentxConfig, regime: LaunchEnvRegime, context: &crate::runtime::context::AgentxContext) -> (HashMap<String, String>, Option<crate::observability::configuration::SourceRead>) {
    let mut base = if regime == LaunchEnvRegime::ContaminatedParent { super::shell_env::resolved_shell_env().unwrap_or_else(|| context.raw_env.clone()) } else { context.raw_env.clone() };
    crate::runtime::environment::restore_originating_values(&mut base, &context.raw_env, &context.environment_file_keys);
    overlay_launch_env_observed(config, regime, base, &context.agentx_dir)
}

fn build_launch_env_with_resolver<F>(agentx_config: &AgentxConfig, regime: LaunchEnvRegime, resolved_shell_env: F) -> HashMap<String, String>
where
    F: Fn() -> Option<HashMap<String, String>>,
{
    let base: HashMap<String, String> = match regime {
        LaunchEnvRegime::HumanShell | LaunchEnvRegime::RunHere => std::env::vars().collect(),
        LaunchEnvRegime::ContaminatedParent => resolved_shell_env().unwrap_or_else(|| std::env::vars().collect()),
    };
    overlay_launch_env(agentx_config, regime, base, &paths::agentx_dir())
}

fn overlay_launch_env(agentx_config: &AgentxConfig, regime: LaunchEnvRegime, base: HashMap<String, String>, scope: &Path) -> HashMap<String, String> {
    overlay_launch_env_observed(agentx_config, regime, base, scope).0
}

fn overlay_launch_env_observed(agentx_config: &AgentxConfig, regime: LaunchEnvRegime, base: HashMap<String, String>, scope: &Path) -> (HashMap<String, String>, Option<crate::observability::configuration::SourceRead>) {
    let strip = match regime {
        LaunchEnvRegime::RunHere => run_here_env_strip_set(),
        LaunchEnvRegime::HumanShell | LaunchEnvRegime::ContaminatedParent => env_strip_set(),
    };
    let mut env: HashMap<String, String> = base.into_iter().filter(|(k, _)| !strip.contains(k.as_str())).collect();

    // AGENTX_* settings from config.toml
    for (key, value) in agentx_config.to_env_dict() {
        if !value.is_empty() {
            insert_effective_env(&mut env, key, value, cfg!(windows));
        }
    }

    // Passthrough vars from env file (these win over everything)
    let env_path = scope.join("env");
    let (extras, receipt) = config::load_env_extras_observed(&env_path);
    for (key, value) in extras {
        if !value.is_empty() {
            insert_effective_env(&mut env, key, value, cfg!(windows));
        }
    }

    (env, receipt)
}

/// Build the set of env var names to strip from inherited env.
///
/// Three closed categories (owned by agentx):
/// 1. AGENTX_IDENTITY_VARS
/// 2. TOOL_MARKER_VARS
/// 3. TERMINAL_CONTEXT_VARS
///
/// Per-tool instance-state vars are NOT in the initial strip — they are
/// stripped per-instance later via `strip_instance_state_vars` so that
/// cross-tool nesting doesn't strip vars the child tool doesn't own.
pub(crate) fn env_strip_set() -> std::collections::HashSet<String> {
    let mut strip: std::collections::HashSet<String> = std::collections::HashSet::new();

    strip.extend(run_here_env_strip_set());
    for v in crate::runtime::terminal::TERMINAL_CONTEXT_VARS {
        strip.insert((*v).to_string());
    }

    strip
}

fn run_here_env_strip_set() -> std::collections::HashSet<String> {
    let mut strip: std::collections::HashSet<String> = std::collections::HashSet::new();

    for v in crate::shared::constants::AGENTX_IDENTITY_VARS {
        strip.insert((*v).to_string());
    }
    for v in tool_marker_vars() {
        strip.insert((*v).to_string());
    }
    strip.insert("AGENTX_LAUNCHED_PRESET".to_string());
    strip.extend(["AGENTX_LAUNCH_PARTICIPANT".to_owned(), "AGENTX_LAUNCH_GENERATION".to_owned()]);
    // A new plugin host must claim ownership itself; an inherited marker would
    // make agentx's OpenCode/Kilo plugin stay inert in the process agentx launched.
    strip.insert(crate::integrations::hooks::runtime::PLUGIN_HOST_PID_ENV.to_string());

    strip
}

/// Insert an environment override using the target platform's key semantics.
/// Windows environment names are case-insensitive, while `HashMap` keys are
/// not; remove an earlier spelling so the child receives one authoritative
/// value instead of an order-dependent pair.
fn insert_effective_env(env: &mut HashMap<String, String>, key: String, value: String, case_insensitive: bool) {
    if case_insensitive {
        env.retain(|existing, _| !existing.eq_ignore_ascii_case(&key));
    }
    env.insert(key, value);
}

fn effective_env_value<'a>(env: &'a HashMap<String, String>, key: &str, case_insensitive: bool) -> Option<&'a str> {
    if case_insensitive { env.iter().find(|(existing, _)| existing.eq_ignore_ascii_case(key)).map(|(_, value)| value.as_str()) } else { env.get(key).map(String::as_str) }
}

/// Make the tool config directory explicit in the child environment.
///
/// Some launch backends clear the inherited environment while others do not.
/// Copying an ambient override into the effective launch map keeps preflight,
/// hook setup, and the child process on the same directory in both cases.
fn ensure_tool_config_env(tool: &LaunchTool, env: &mut HashMap<String, String>) {
    let Some(env_var) = tool.spec().launch.config_dir_env else {
        return;
    };
    let case_insensitive = cfg!(windows);
    if let Some(value) = effective_env_value(env, env_var, case_insensitive).map(str::to_owned) {
        insert_effective_env(env, env_var.to_string(), value, case_insensitive);
        return;
    }
    if let Some(value) = std::env::var(env_var).ok().filter(|value| !value.is_empty()) {
        insert_effective_env(env, env_var.to_string(), value, case_insensitive);
    }
}

/// Get system prompt file path for Gemini/Codex.
fn get_system_prompt_path(tool: &str) -> std::path::PathBuf {
    let prompts_dir = paths::agentx_path(&["system-prompts"]);
    fs::create_dir_all(&prompts_dir).ok();
    prompts_dir.join(format!("{}.md", tool))
}

/// Write system prompt to file (only if content differs).
fn write_system_prompt_file(system_prompt: &str, tool: &str) -> String {
    let filepath = get_system_prompt_path(tool);

    // Only write if content differs
    if let Ok(existing) = fs::read_to_string(&filepath)
        && existing == system_prompt
    {
        return filepath.to_string_lossy().to_string();
    }

    if let Err(e) = fs::write(&filepath, system_prompt) {
        eprintln!("[agentx] warn: failed to write system prompt to {}: {e}", filepath.display());
    }
    filepath.to_string_lossy().to_string()
}

/// Generate a UUID v4-like process ID string.
fn generate_process_id() -> String {
    let mut rng = rand::rng();
    let a: u32 = rng.random();
    let b: u16 = rng.random();
    let c: u16 = (rng.random::<u16>() & 0x0FFF) | 0x4000; // version 4
    let d: u16 = (rng.random::<u16>() & 0x3FFF) | 0x8000; // variant 1
    let e: u64 = rng.random::<u64>() & 0xFFFFFFFFFFFF; // 48 bits
    format!("{:08x}-{:04x}-{:04x}-{:04x}-{:012x}", a, b, c, d, e)
}

fn install_diag_context(tool: &LaunchTool, paths: &[(&str, std::path::PathBuf)]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "Diagnostic context:");
    for (label, p) in paths {
        let _ = writeln!(out, "  resolved {label}={}", p.display());
    }
    let _ = writeln!(out, "  AGENTX_DIR={}", std::env::var("AGENTX_DIR").unwrap_or_else(|_| "<unset>".into()));
    let tool_env_var = tool.spec().launch.config_dir_env;
    if let Some(env_var) = tool_env_var {
        let _ = writeln!(out, "  {env_var}={}", std::env::var(env_var).unwrap_or_else(|_| "<unset>".into()));
    }
    let _ = writeln!(out, "  cwd={}", std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_else(|_| "<unknown>".into()));
    out
}

/// Verify hooks are installed for the target tool, auto-install if needed.
///
/// Uses verify-first pattern: read-only check first, only write if needed.
/// Strict gate: refuses to launch if hooks can't be installed.
fn ensure_hooks_installed(tool: &LaunchTool, include_permissions: bool) -> Result<()> {
    if tool.tool() == Tool::Gemini && !crate::integrations::hooks::gemini::is_gemini_version_supported() {
        if let Some(ver) = crate::integrations::hooks::gemini::get_gemini_version() {
            bail!("Gemini CLI version {}.{}.{} is too old. Update: npm i -g @google/gemini-cli@latest", ver.0, ver.1, ver.2);
        } else {
            eprintln!("Warning: Could not detect Gemini CLI version");
        }
    }
    let integration = tool.tool();
    if integration.verify_hooks_installed(include_permissions) {
        return Ok(());
    }
    if let Err(e) = integration.try_setup_hooks(include_permissions) {
        let path_label = if integration == crate::integrations::tool::Tool::Gemini { "settings_path" } else { "hooks_path" };
        let diag = install_diag_context(tool, &[(path_label, integration.hooks_settings_path().into())]);
        bail!("Failed to setup {} hooks: {e}\nRun: agentx hooks add {}\n{diag}", tool.spec().label, tool.base_tool());
    }
    Ok(())
}

/// Build a command string for Claude (non-PTY mode). Quoting matches the
/// shell that will run the resulting script: PowerShell on Windows, POSIX
/// shell elsewhere (see `launch_terminal`'s `create_powershell_script` /
/// `create_bash_script` split).
fn build_claude_command(args: &[String]) -> String {
    let mut parts = vec!["claude".to_string()];
    for arg in args {
        if cfg!(windows) {
            parts.push(terminal::ps_quote(arg));
        } else {
            parts.push(crate::integrations::launch_args::shell::shell_quote(arg));
        }
    }
    parts.join(" ")
}

/// Tool-specific extra environment variables for PTY mode.
fn tool_extra_env(tool: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    // herdr's wrapper hint: it reads this from the pane's foreground process
    // (`agentx pty`, which hides the tool in its own PTY) to classify the pane
    // and pick the tool's screen-detection manifest. agentx tool names are all
    // valid herdr agent labels; outside herdr nothing reads it.
    m.insert("HERDR_AGENT".to_string(), tool.to_string());
    // Claude is driven by the PTY wrapper (ConPTY on Windows, openpty on Unix),
    // which handles injection; AGENTX_PTY_MODE tells the Stop hook to defer to it.
    if tool == "claude" {
        m.insert("AGENTX_PTY_MODE".to_string(), "1".to_string());
    }
    if tool == "antigravity" {
        m.insert("ANTIGRAVITY_AGENT".to_string(), "1".to_string());
    }
    m
}

fn background_runner_env(tool: &str, env: &HashMap<String, String>, instance_name: &str) -> HashMap<String, String> {
    let mut runner_env = env.clone();
    runner_env.insert("AGENTX_INSTANCE_NAME".to_string(), instance_name.to_string());
    // Default AGENTX_TOOL when the caller didn't already set it (most callers
    // come from `launch()` which inserts it; this keeps the standalone PTY
    // and headless paths consistent so `{tool}` template substitution and
    // delivery-loop label formatting see the right value).
    runner_env.entry("AGENTX_TOOL".to_string()).or_insert_with(|| tool.to_string());
    runner_env.extend(tool_extra_env(tool));
    runner_env
}

/// Non-AGENTX ambient env to forward through the sidecar, with marker/identity/
/// instance-state/terminal-color vars stripped. On Windows env names are
/// case-insensitive (and the paired PowerShell `Remove-Item Env:` folds case),
/// so `case_insensitive` folds case for the match; Unix keeps exact-case.
fn sidecar_ambient_env<'a>(env: &HashMap<String, String>, strip_vars: impl Iterator<Item = &'a str>, case_insensitive: bool) -> HashMap<String, String> {
    let norm = |k: &str| {
        if case_insensitive { k.to_ascii_lowercase() } else { k.to_string() }
    };
    let strip: std::collections::HashSet<String> = strip_vars.map(&norm).collect();
    env.iter().filter(|(k, _)| !k.starts_with("AGENTX_") && !strip.contains(&norm(k))).map(|(k, v)| (k.clone(), v.clone())).collect()
}

struct RunnerBinaries {
    path_dirs: Vec<String>,
    tool_path: Option<String>,
}

/// Resolve the executables needed by generated runners and order their directories.
///
/// The selected Node runtime must precede every other prepended directory: tool,
/// agentx, and Python directories can all contain an older `node`/`npx`. The tool is
/// resolved before PATH changes and passed explicitly to `agentx pty`, so moving its
/// directory after Node does not change which tool executable is launched.
fn resolve_runner_binaries(initial_dirs: Vec<String>, tool_bin: &str, mut which_bin: impl FnMut(&str) -> Option<String>) -> RunnerBinaries {
    let tool_path = which_bin(tool_bin);
    let agentx_path = which_bin("agentx");
    let node_path = which_bin("node");
    let python_path = which_bin("python3");
    let mut path_dirs = Vec::new();

    fn append_binary_dir(path_dirs: &mut Vec<String>, path: &str) {
        if let Some(dir) = Path::new(path).parent() {
            let dir = dir.to_string_lossy().into_owned();
            if !path_dirs.contains(&dir) {
                path_dirs.push(dir);
            }
        }
    }

    for path in node_path.iter() {
        append_binary_dir(&mut path_dirs, path);
    }
    for dir in initial_dirs {
        if !path_dirs.contains(&dir) {
            path_dirs.push(dir);
        }
    }
    for path in tool_path.iter().chain(agentx_path.iter()).chain(python_path.iter()) {
        append_binary_dir(&mut path_dirs, path);
    }

    RunnerBinaries { path_dirs, tool_path }
}

/// Warn (once per process) when the `agentx` a launched agent resolves by name is
/// not this binary. Runner dirs are prepended to PATH with the selected Node's
/// dir first (#117), so a second agentx install next to that Node, or a launch
/// by absolute path, silently routes every hook and agent command elsewhere.
fn warn_if_agent_agentx_differs(runner_dirs: &[String]) {
    static WARNED: std::sync::Once = std::sync::Once::new();
    // Dev mode re-execs whichever agentx runs into the dev build.
    if crate::cli::router::resolve_effective_dev_root(&crate::paths::db_path()).is_some() {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let inherited = std::env::var_os("PATH");
    let dirs = runner_dirs.iter().map(std::path::PathBuf::from).chain(inherited.iter().flat_map(std::env::split_paths));
    let message = match first_agentx_in(dirs) {
        Some(found) if same_binary(&found, &exe) => return,
        Some(found) => format!("agents will run {} as `agentx`, not this agentx ({}). Remove the other install or put this one first on PATH.", found.display(), exe.display()),
        None => format!("agents can't run this agentx ({}) as `agentx`: its directory isn't on PATH. Add it to PATH.", exe.display()),
    };
    WARNED.call_once(|| {
        eprintln!("[agentx] warn: {message}");
        crate::log::log_warn("launch", "agent_agentx_mismatch", &message);
    });
}

/// First `agentx` a shell would run from `dirs`: shells skip non-executable files.
fn first_agentx_in(dirs: impl Iterator<Item = std::path::PathBuf>) -> Option<std::path::PathBuf> {
    dirs.flat_map(|dir| terminal::which_candidates(&dir, "agentx")).find(|candidate| {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(candidate).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            candidate.is_file()
        }
    })
}

/// Same path after resolving symlinks, or identical bytes (uv copies rather
/// than links tool executables on Windows).
fn same_binary(a: &Path, b: &Path) -> bool {
    if let (Ok(a), Ok(b)) = (a.canonicalize(), b.canonicalize())
        && a == b
    {
        return true;
    }
    let same_len = match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(a), Ok(b)) => a.len() == b.len(),
        _ => false,
    };
    same_len && matches!((std::fs::read(a), std::fs::read(b)), (Ok(a), Ok(b)) if a == b)
}

/// Windows runner: a PowerShell script that launches the tool through the agentx
/// ConPTY wrapper (`agentx pty <tool>`), mirroring the Unix bash runner. The
/// wrapper runs the delivery loop so idle agents can be woken. Mirrors the bash
/// runner's env scrubbing, AGENTX env, secret sidecar, and PATH setup.
fn create_runner_script_windows(tool: &str, cwd: &str, instance_name: &str, env: &HashMap<String, String>, tool_args: &[String], run_here: bool) -> Result<String> {
    let tool_spec = tool.parse::<crate::integrations::tool::Tool>().map(|t| t.spec()).ok();
    let instance_state_env: &[&str] = tool_spec.map(|s| s.instance_state_env).unwrap_or(&[]);

    let launch_dir = paths::agentx_path(&[paths::LAUNCH_DIR]);
    fs::create_dir_all(&launch_dir).ok();
    let script_file = launch_dir.join(format!("{}_{}_{}_{}.ps1", tool, instance_name, std::process::id(), rand::random::<u16>() % 9000 + 1000));

    // Visible AGENTX_* env, plus the managed-launch marker so hooks engage.
    let mut agentx_env: HashMap<String, String> = env.iter().filter(|(k, _)| k.starts_with("AGENTX_")).map(|(k, v)| (k.clone(), v.clone())).collect();
    agentx_env.insert("AGENTX_LAUNCHED".to_string(), "1".to_string());
    let env_block = terminal::build_env_string(&agentx_env, "powershell");

    // Non-AGENTX ambient env (may carry secrets) goes through a private sidecar
    // that is dot-sourced then deleted, matching the bash runner.
    let pane_identity_vars = if run_here { std::collections::HashSet::new() } else { crate::config::pane_identity_env_vars() };
    let ambient_env = sidecar_ambient_env(
        env,
        tool_marker_vars().iter().chain(AGENTX_IDENTITY_VARS.iter()).chain(instance_state_env.iter()).chain(crate::runtime::terminal::TERMINAL_COLOR_VARS.iter()).copied().chain(pane_identity_vars.iter().map(String::as_str)),
        true,
    );
    let sidecar_source = if ambient_env.is_empty() {
        String::new()
    } else {
        let env_file = launch_dir.join(format!("{}_{}_{}_{}.ps1", tool, instance_name, std::process::id(), rand::random::<u16>() % 9000 + 1000));
        let mut file = crate::sys::fs::create_private_new(&env_file)?;
        // Windows PowerShell 5.1 reads BOM-less files using the legacy ANSI
        // code page, corrupting non-ASCII env values; a UTF-8 BOM forces it
        // to read as UTF-8.
        file.write_all(b"\xEF\xBB\xBF")?;
        writeln!(file, "{}", terminal::build_env_string(&ambient_env, "powershell"))?;
        let q = terminal::ps_quote(&env_file.to_string_lossy());
        format!("if (Test-Path {q}) {{ . {q}; Remove-Item -Force {q} }}")
    };

    // Scrub inherited tool markers / identity / instance-state vars.
    let unset_names: Vec<String> = tool_marker_vars().iter().chain(AGENTX_IDENTITY_VARS.iter()).chain(instance_state_env.iter()).map(|v| format!("Env:{v}")).collect();
    let unset_line = if unset_names.is_empty() { String::new() } else { format!("Remove-Item {} -ErrorAction SilentlyContinue", unset_names.join(",")) };

    // Resolve binary directories for minimal PATH environments.
    let mut path_dirs: Vec<String> = Vec::new();
    if let Ok(dev_root) = std::env::var("AGENTX_DEV_ROOT")
        && let Some(bin) = crate::shared::dev_root_binary(Path::new(&dev_root))
        && let Some(dir) = bin.parent()
    {
        path_dirs.push(dir.to_string_lossy().into_owned());
    }
    // Ensure the launched tool (and its hooks) can call back to *this* agentx by
    // name — a dev binary may not be on the global PATH.
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let d = dir.to_string_lossy().to_string();
        if !path_dirs.contains(&d) {
            path_dirs.push(d);
        }
    }
    let tool_bin = tool.parse::<crate::integrations::tool::Tool>().map(|t| t.spec().cli_binary).unwrap_or(tool);
    let binaries = resolve_runner_binaries(path_dirs, tool_bin, terminal::which_bin);
    warn_if_agent_agentx_differs(&binaries.path_dirs);
    let path_dirs = binaries.path_dirs;
    let path_line = if path_dirs.is_empty() { String::new() } else { format!("$env:PATH = {} + $env:PATH", terminal::ps_quote(&format!("{};", path_dirs.join(";")))) };

    // Run through the agentx PTY wrapper (ConPTY) so the tool is driven by the
    // delivery loop — this is what wakes an idle agent on Windows. Mirrors the
    // Unix runner's `agentx pty <tool>` call.
    //
    // Tool args travel via a JSON sidecar file, not the command line: the
    // PowerShell → native-exe argv boundary mangles embedded double quotes
    // (powershell.exe passes them unescaped, so the child's command-line
    // parser re-splits at quote/space boundaries). Codex args always contain
    // quotes (`-c projects={ "path" = ... }`, developer_instructions), which
    // made `agentx launch --tool codex` fail with "unexpected argument" (#66). Only the
    // sidecar path — generated by agentx, never quote-bearing — goes on the
    // command line. `agentx pty` reads and deletes the file.
    let agentx_bin = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "agentx".to_string());
    let tool_path_arg = binaries.tool_path.as_deref().map(|path| format!(" --agentx-tool-path {}", terminal::ps_quote(path))).unwrap_or_default();
    let run_line = if tool_args.is_empty() {
        format!("& {} pty {}{}", terminal::ps_quote(&agentx_bin), tool, tool_path_arg)
    } else {
        let args_file = launch_dir.join(format!("{}_{}_{}_{}.args.json", tool, instance_name, std::process::id(), rand::random::<u16>() % 9000 + 1000));
        let mut file = crate::sys::fs::create_private_new(&args_file)?;
        file.write_all(serde_json::to_string(tool_args)?.as_bytes())?;
        format!("& {} pty {}{} --agentx-args-file {}", terminal::ps_quote(&agentx_bin), tool, tool_path_arg, terminal::ps_quote(&args_file.to_string_lossy()))
    };
    // `powershell -File` returns 0 unless the script exits with an explicit
    // code, so surface the wrapper's real exit status (agent failures, PTY
    // crashes, kill signals) instead of always reporting success.
    let run_line = format!("{run_line}\nexit $LASTEXITCODE");

    let display = tool.chars().next().unwrap_or('?').to_uppercase().collect::<String>() + &tool[1..];
    let content = format!(
        "# {display} agentx native runner ({instance_name})\n\
         Set-Location {cwd}\n\
         {unset_line}\n\
         {env_block}\n\
         if ($env:AGENTX_BACKGROUND) {{ Write-Host \"[agentx runner] environment ready $((Get-Date).ToString('o'))\" }}\n\
         {sidecar_source}\n\
         {path_line}\n\
         if ($env:AGENTX_BACKGROUND) {{ Write-Host \"[agentx runner] starting PTY wrapper $((Get-Date).ToString('o'))\" }}\n\
         \n\
         {run_line}\n",
        cwd = terminal::ps_quote(cwd),
    );

    // Windows PowerShell 5.1 reads BOM-less files using the legacy ANSI code
    // page, corrupting non-ASCII usernames/paths/prompts/env values; a UTF-8
    // BOM forces it to read as UTF-8.
    let mut bytes = Vec::with_capacity(content.len() + 3);
    bytes.extend_from_slice(b"\xEF\xBB\xBF");
    bytes.extend_from_slice(content.as_bytes());
    fs::write(&script_file, &bytes)?;

    crate::log::log_info("pty", "native.script", &format!("script={} tool={} instance={} (windows ConPTY launch)", script_file.display(), tool, instance_name));

    Ok(script_file.to_string_lossy().to_string())
}

/// Create a bash script that runs a tool via the agentx native PTY wrapper.
///
/// The script sets up the environment and calls `agentx pty <tool> [args...]`.
pub fn create_runner_script(tool: &str, cwd: &str, instance_name: &str, env: &HashMap<String, String>, tool_args: &[String], run_here: bool) -> Result<String> {
    if cfg!(windows) {
        return create_runner_script_windows(tool, cwd, instance_name, env, tool_args, run_here);
    }
    // Resolve the tool's IntegrationSpec for instance-state env stripping
    let tool_spec = tool.parse::<crate::integrations::tool::Tool>().map(|t| t.spec()).ok();
    let instance_state_env: &[&str] = tool_spec.map(|s| s.instance_state_env).unwrap_or(&[]);
    let native_bin = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("agentx"));
    let native_bin_str = native_bin.to_string_lossy();

    let launch_dir = paths::agentx_path(&[paths::LAUNCH_DIR]);
    fs::create_dir_all(&launch_dir).ok();

    let script_file = launch_dir.join(format!("{}_{}_{}_{}.sh", tool, instance_name, std::process::id(), rand::random::<u16>() % 9000 + 1000));

    // Route ALL forwarded non-AGENTX env vars through the 0600 sidecar.
    // The visible .sh only exports AGENTX_* vars + PATH + cwd (minimal).
    // This avoids the sensitivity-classification heuristic entirely — no
    // secret ever lands in the 0755 world-readable script.
    let agentx_env: HashMap<String, String> = env.iter().filter(|(k, _)| k.starts_with("AGENTX_")).map(|(k, v)| (k.clone(), v.clone())).collect();
    let pane_identity_vars = if run_here { std::collections::HashSet::new() } else { crate::config::pane_identity_env_vars() };
    let ambient_env = sidecar_ambient_env(
        env,
        tool_marker_vars().iter().chain(AGENTX_IDENTITY_VARS.iter()).chain(instance_state_env.iter()).chain(crate::runtime::terminal::TERMINAL_COLOR_VARS.iter()).copied().chain(pane_identity_vars.iter().map(String::as_str)),
        false,
    );
    let env_block = terminal::build_env_string(&agentx_env, "bash_export");
    let sensitive_env_source = if ambient_env.is_empty() {
        String::new()
    } else {
        let env_file = launch_dir.join(format!("{}_{}_{}_{}.env", tool, instance_name, std::process::id(), rand::random::<u16>() % 9000 + 1000));
        let mut file = crate::sys::fs::create_private_new(&env_file)?;
        writeln!(file, "{}", terminal::build_env_string(&ambient_env, "bash_export"))?;
        let quoted = crate::integrations::launch_args::shell::shell_quote(&env_file.to_string_lossy());
        format!("if [ -f {quoted} ]; then\n  . {quoted}\n  rm -f {quoted}\nfi")
    };
    let tool_args_str: String = tool_args.iter().map(|a| crate::integrations::launch_args::shell::shell_quote(a)).collect::<Vec<_>>().join(" ");

    // Resolve binary paths for minimal PATH environments
    let mut path_dirs: Vec<String> = Vec::new();

    // Dev mode: prepend the worktree's Cargo output dir
    if let Ok(dev_root) = std::env::var("AGENTX_DEV_ROOT")
        && let Some(bin) = crate::shared::dev_root_binary(Path::new(&dev_root))
        && let Some(dir) = bin.parent()
    {
        path_dirs.push(dir.to_string_lossy().into_owned());
    }

    let tool_bin = tool.parse::<crate::integrations::tool::Tool>().map(|t| t.spec().cli_binary).unwrap_or(tool);
    let binaries = resolve_runner_binaries(path_dirs, tool_bin, terminal::which_bin);
    warn_if_agent_agentx_differs(&binaries.path_dirs);
    let path_dirs = binaries.path_dirs;

    let path_export = if !path_dirs.is_empty() { format!("export PATH=\"{}:$PATH\"", path_dirs.join(":")) } else { String::new() };

    let use_exec = if run_here { "" } else { "exec " };
    let tool_path_arg = binaries.tool_path.as_deref().map(|path| format!(" --agentx-tool-path {}", crate::integrations::launch_args::shell::shell_quote(path))).unwrap_or_default();

    let content = format!(
        "#!/bin/bash\n\
         # {} agentx native PTY runner ({})\n\
         # Using: {}\n\
         cd {}\n\
         \n\
         unset {}\n\
         unset {}\n\
         unset {}\n\
         {}\n\
         {}\n\
         {}\n\
         \n\
         {}{} pty {}{} {}\n",
        tool.chars().next().unwrap_or('?').to_uppercase().collect::<String>() + &tool[1..],
        instance_name,
        native_bin_str,
        crate::integrations::launch_args::shell::shell_quote(cwd),
        tool_marker_vars().join(" "),
        AGENTX_IDENTITY_VARS.join(" "),
        instance_state_env.join(" "),
        env_block,
        sensitive_env_source,
        path_export,
        use_exec,
        crate::integrations::launch_args::shell::shell_quote(&native_bin_str),
        tool,
        tool_path_arg,
        tool_args_str,
    );

    fs::write(&script_file, &content)?;
    crate::sys::fs::set_executable(&script_file)?;

    crate::log::log_info("pty", "native.script", &format!("script={} tool={} instance={} forwarded_keys=[{}]", script_file.display(), tool, instance_name, ambient_env.keys().cloned().collect::<Vec<_>>().join(", ")));

    Ok(script_file.to_string_lossy().to_string())
}

/// Build the command that runs a generated runner script in the launched
/// terminal. On Windows the outer launcher is already PowerShell, so invoke
/// the runner in that process instead of starting a second PowerShell host.
/// Besides avoiding needless startup cost, this removes a launch stage that
/// can intermittently stall before `agentx pty` is reached.
fn runner_invocation_command_for_platform(script_file: &str, windows: bool) -> String {
    if windows { format!("& {}", crate::runtime::terminal::ps_quote(script_file)) } else { format!("bash {}", crate::integrations::launch_args::shell::shell_quote(script_file)) }
}

fn runner_invocation_command(script_file: &str) -> String {
    runner_invocation_command_for_platform(script_file, cfg!(windows))
}

/// Launch a tool via PTY wrapper in a terminal.
#[allow(clippy::too_many_arguments)]
fn launch_pty(client: &Client, reservation: &Reservation, tool: &str, cwd: &str, env: &HashMap<String, String>, instance_name: &str, tool_args: &[String], run_here: bool, terminal: Option<&str>, inside_ai_tool: bool) -> Result<bool> {
    if env.get("AGENTX_PROCESS_ID").is_none_or(|v| v.is_empty()) {
        crate::log::log_error("pty", "pty.exit", &format!("AGENTX_PROCESS_ID not set in env for {}", instance_name));
        return Ok(false);
    }

    let mut runner_env = env.clone();
    runner_env.insert("AGENTX_INSTANCE_NAME".to_string(), instance_name.to_string());
    runner_env.entry("AGENTX_TOOL".to_string()).or_insert_with(|| tool.to_string());
    runner_env.extend(tool_extra_env(tool));

    let script_file = create_runner_script(tool, cwd, instance_name, &runner_env, tool_args, run_here)?;

    let command = runner_invocation_command(&script_file);
    let terminal_env: HashMap<String, String> = runner_env.iter().filter(|(k, _)| k.starts_with("AGENTX_")).map(|(k, v)| (k.clone(), v.clone())).collect();

    let (launch_result, effective_preset) = terminal::launch_terminal(
        &command,
        &terminal_env,
        Some(cwd),
        false, // not background
        run_here,
        terminal,
        inside_ai_tool,
        Some(tool),
    )?;

    record_terminal(client, reservation, terminal, &effective_preset, None)?;

    match launch_result {
        terminal::LaunchResult::Success => Ok(true),
        terminal::LaunchResult::Background(_, _) => Ok(true),
        terminal::LaunchResult::Failed(_) => Ok(false),
    }
}

/// Identity and tracking context for a background launch, shared across tool types.
struct BackgroundLaunchCtx<'a> {
    client: &'a Client,
    reservation: &'a Reservation,
    tool: &'a str,
    instance_name: &'a str,
    process_id: &'a str,
    terminal_mode: Option<&'a str>,
    working_dir: &'a str,
    log_files: &'a mut Vec<String>,
    handles: &'a mut Vec<serde_json::Value>,
}

/// Shared bookkeeping after a successful background launch for gemini/codex/opencode.
/// Persists the launch context, updates position, records the PID, and appends
/// log_file / handle entries. Per-tool differences (args, prompt) stay in the caller.
fn finalize_background_launch(ctx: &mut BackgroundLaunchCtx<'_>, log_file: String, pid: u32, effective_preset: String) -> Result<()> {
    let identity = crate::sys::process::identity(pid);
    let tracked = launch_service::BackgroundProcess { child: super::hosts::ChildProcess { pid, incarnation: identity.clone() }, log_file: log_file.clone() };
    if let Err(error) = record_terminal(ctx.client, ctx.reservation, ctx.terminal_mode, &effective_preset, Some(tracked)) {
        if identity.as_ref().is_some_and(|expected| crate::sys::process::identity(pid).as_ref() == Some(expected)) {
            let _ = crate::sys::process::kill_group(pid);
        }
        bail!("failed to persist background process for '{}': {error}", ctx.instance_name);
    }
    crate::runtime::process_tracking::record_pid(&crate::runtime::process_tracking::PidRecord {
        process_id: ctx.process_id,
        terminal_preset: &effective_preset,
        ..crate::runtime::process_tracking::PidRecord::new(&crate::paths::agentx_dir(), pid, ctx.tool, ctx.instance_name, ctx.working_dir)
    });
    if !log_file.is_empty() {
        ctx.log_files.push(log_file.clone());
    }
    ctx.handles.push(json!({
        "tool": ctx.tool,
        "instance_name": ctx.instance_name,
        "log_file": log_file,
        "pid": pid,
    }));
    Ok(())
}

fn launch_background_runner(tool: &str, cwd: &str, instance_name: &str, instance_env: &mut HashMap<String, String>, tool_args: &[String], terminal_mode: Option<&str>, inside_ai_tool: bool, config: &AgentxConfig) -> Result<(String, u32, String)> {
    let log_filename = format!("background_{}_{}.log", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0), rand::random::<u16>() % 9000 + 1000);
    let mut runner_env = background_runner_env(tool, instance_env, instance_name);
    runner_env.insert("AGENTX_BACKGROUND".to_string(), log_filename);
    if terminal_mode == Some("tmux") {
        runner_env.insert("AGENTX_LAUNCHED_PRESET".into(), "tmux".into());
        // The new pane supplies its server metadata; the captured sidecar must
        // not restore the invoking terminal's TMUX over that observed value.
        runner_env.remove("TMUX");
    }
    let script_file = create_runner_script(tool, cwd, instance_name, &runner_env, tool_args, false)?;
    if terminal_mode == Some("tmux") {
        let settings = super::tmux::Settings::resolve(&paths::agentx_dir(), config)?;
        let process_id = runner_env.get("AGENTX_PROCESS_ID").context("missing launch process identity")?;
        let pid = super::tmux::launch(&settings, &script_file, process_id, cwd)?;
        return Ok((String::new(), pid, "tmux".into()));
    }
    let command = runner_invocation_command(&script_file);
    let terminal_env: HashMap<String, String> = runner_env.iter().filter(|(k, _)| k.starts_with("AGENTX_")).map(|(k, v)| (k.clone(), v.clone())).collect();
    let (launch_result, effective_preset) = terminal::launch_terminal(&command, &terminal_env, Some(cwd), true, false, terminal_mode, inside_ai_tool, Some(tool))?;
    match launch_result {
        terminal::LaunchResult::Background(log_file, pid) => Ok((log_file, pid, effective_preset)),
        _ => bail!("background launch failed"),
    }
}

/// Common launch path for gemini/codex/opencode: background or PTY foreground.
///
/// Handles `launch_background_runner` + `finalize_background_launch` for background,
/// and `will_run_in_current_terminal` + `launch_pty` for current/window terminals.
/// Managed tmux always starts detached; CLI owns optional foreground attachment.
fn launch_pty_or_background(ctx: &mut BackgroundLaunchCtx<'_>, instance_env: &mut HashMap<String, String>, tool_args: &[String], params: &LaunchParams, inside_ai_tool: bool, config: &AgentxConfig) -> Result<bool> {
    let effective_run_here = will_run_in_current_terminal(params.count, params.background, params.run_here, params.terminal.as_deref(), inside_ai_tool, std::io::IsTerminal::is_terminal(&std::io::stdin()));
    if params.background || (ctx.terminal_mode == Some("tmux") && !effective_run_here) {
        let (log_file, pid, effective_preset) = launch_background_runner(ctx.tool, ctx.working_dir, ctx.instance_name, instance_env, tool_args, ctx.terminal_mode, inside_ai_tool, config)?;
        finalize_background_launch(ctx, log_file, pid, effective_preset)?;
        Ok(true)
    } else {
        let ok = launch_pty(ctx.client, ctx.reservation, ctx.tool, ctx.working_dir, instance_env, ctx.instance_name, tool_args, effective_run_here, ctx.terminal_mode, inside_ai_tool)?;
        if ok {
            ctx.handles.push(json!({"tool": ctx.tool, "instance_name": ctx.instance_name}));
        }
        Ok(ok)
    }
}

/// Resolve a naming conflict for an explicit instance name.
///
/// - Name is free → Ok(()).
/// - Name held by an inactive row → consume the row (delete) and return Ok(()).
///   An inactive row is a resume handle from agy soft-finalize; launch will
///   re-create a fresh row with the same name.
/// - Name held by a `pending` placeholder reservation → Ok(()) without
///   deleting. This is *our own* reservation: the fork/resume path calls
///   `reserve_generated_name` (under flock, against an unused name) before the
///   launch, then passes that name as `params.name`. The pre-register step
///   (`initialize_instance_in_position_file`) promotes the placeholder in
///   place, so it must survive — bailing here broke every tracked `agentx f`.
/// - Name held by anything else (listening/active/blocked) → Err.
#[cfg(test)]
fn resolve_explicit_name_conflict(db: &AgentxDb, name: &str) -> Result<()> {
    let Some(row) = db.get_instance(name).ok().flatten() else {
        return Ok(());
    };
    let status = row.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if status == "inactive" {
        db.delete_instance(name).map_err(|e| anyhow::anyhow!("Failed to clear inactive resume row '{}': {}", name, e))?;
        return Ok(());
    }
    // A pending placeholder with no session yet is a reservation, not a live
    // agent — leave it for the launcher's pre-register promotion.
    let status_context = row.get("status_context").and_then(|v| v.as_str()).unwrap_or("");
    let session_empty = row.get("session_id").and_then(|v| v.as_str()).map(|s| s.is_empty()).unwrap_or(true);
    if status == instance_names::PLACEHOLDER_STATUS && status_context == instance_names::PLACEHOLDER_CONTEXT && session_empty {
        return Ok(());
    }
    bail!("Instance '{}' already exists (stop it first or use a different name)", name);
}

/// Inject ephemeral workspace trust flags into args for gemini and codex.
///
/// - gemini: adds `--skip-trust` (session-scoped, no persisted state)
/// - codex: adds `-c` with a TOML inline-table value that sets trust for the
///   canonical CWD. Uses key `projects` (no dots in the key path) so codex's
///   naive `.`-splitting never touches the path itself, making it robust to
///   dotted directory components. The path is TOML-escaped (`\` and `"`) —
///   every Windows path carries backslashes, and unescaped they made codex
///   reject the value as "invalid type: string, expected a map".
///
/// When `auto_trust` is false, returns immediately without modifying args.
/// Idempotent: no-op if the relevant flag is already present.
/// Cursor is handled separately via `ensure_cursor_workspace_trusted`.
pub(crate) fn inject_workspace_trust_args(tool: &LaunchTool, canonical_dir: &std::path::Path, args: &mut Vec<String>, auto_trust: bool) {
    if !auto_trust {
        return;
    }
    match tool.tool() {
        Tool::Gemini if !args.iter().any(|a| a == "--skip-trust") => {
            args.push("--skip-trust".to_string());
        }
        Tool::Gemini => {}
        Tool::Codex => {
            let already_set = args.windows(2).any(|w| w[0] == "-c" && w[1].contains("trust_level"));
            if !already_set {
                let canonical_str = canonical_dir.to_string_lossy();
                // std::fs::canonicalize returns \\?\-prefixed verbatim paths on
                // Windows; codex tracks projects by their ordinary absolute
                // form, so strip the prefix or the trust key never matches.
                let simplified = if let Some(unc) = canonical_str.strip_prefix(r"\\?\UNC\") { format!(r"\\{unc}") } else { canonical_str.strip_prefix(r"\\?\").unwrap_or(&canonical_str).to_string() };
                // codex -c: key="projects" (no dots → no split issue), value is a TOML
                // inline table with the quoted path as key. apply_single_override replaces
                // the projects table for this session only (file stays untouched).
                // Escape backslashes before quotes: the quote escape introduces
                // a backslash that must not itself be doubled.
                let toml_escaped = crate::runtime::env::toml_escape_path(&simplified);
                let trust_override = format!("projects={{ \"{}\" = {{ trust_level = \"trusted\" }} }}", toml_escaped);
                args.push("-c".to_string());
                args.push(trust_override);
            }
        }
        _ => {}
    }
}

fn validate_launch_count(tool: &LaunchTool, count: usize) -> Result<()> {
    if count == 0 {
        bail!("Count must be positive");
    }
    let max = tool.spec().launch.max_launch_count;
    if count > max {
        bail!("Too many {} instances requested (max {})", tool.as_str(), max);
    }
    Ok(())
}

fn append_initial_prompt_args(tool: &LaunchTool, args: &mut Vec<String>, prompt: String) -> Result<()> {
    match tool.spec().launch.initial_prompt {
        crate::integrations::registry::InitialPromptShape::Unsupported { reason } => bail!("{reason}"),
        crate::integrations::registry::InitialPromptShape::DashDashPositional => {
            args.push("--".to_string());
            args.push(prompt);
        }
        crate::integrations::registry::InitialPromptShape::Positional => args.push(prompt),
        crate::integrations::registry::InitialPromptShape::Flag(flag) => {
            args.push(flag.to_string());
            args.push(prompt);
        }
    }
    Ok(())
}

#[cfg(test)]
fn initialize_launch_instance(db: &AgentxDb, params: &LaunchParams, instance_name: &str, tool: &str, working_dir: &str, initial_event_id: i64) -> Result<()> {
    db.with_write_scope(|| {
        if !instance_binding::initialize_instance_in_position_file(
            db,
            instance_name,
            params.prior_session_id.as_deref(),
            None, // parent_session_id
            None, // parent_name
            None, // agent_id
            None, // transcript_path
            Some(tool),
            params.background,
            None, // wait_timeout
            None, // subagent_timeout
            None, // hints
            Some(working_dir),
        ) {
            bail!("Failed to initialize instance '{instance_name}'");
        }
        // The initializer can read the caller's inherited launch cursor or promote
        // a fork reservation. Set this launch's authoritative position before any
        // process binding or child spawn makes the instance available for delivery.
        db.update_instance_fields(instance_name, &serde_json::Map::from_iter([("last_event_id".to_string(), json!(initial_event_id))]))?;
        if db.get_instance_full(instance_name)?.is_none() {
            bail!("Failed to initialize instance '{instance_name}'");
        }
        // Only a tracked resume carries a cursor: the same identity returns,
        // so it gets back the subscriptions its stop saved, like its unread
        // position. Done before spawn so replayed matches queue as unread.
        if params.resume_cursor.is_some()
            && let Err(e) = crate::db::subscriptions::restore_stopped_subscriptions(db, instance_name)
        {
            crate::log::log_warn("launcher", "resume.restore_subscriptions_fail", &format!("instance={instance_name} err={e}"));
        }
        Ok(())
    })
}

/// Launch one or more AI tool instances with consistent tracking.
///
/// This is the unified entry point for launching Claude, Gemini, Codex,
/// and OpenCode instances with batch tracking, environment setup, and
/// error handling.
pub fn launch(params: LaunchParams) -> Result<LaunchResult> {
    launch_with_context(params, &crate::runtime::context::AgentxContext::from_os())
}

/// Execute in a caller host with the same canonical AgentX scope. The captured
/// invocation supplies child configuration/cwd; terminal presets and artifacts
/// are owned by this scoped host process. A different scope requires its own
/// originating worker, never a global environment change in a shared daemon.
pub fn launch_with_context(params: LaunchParams, caller_context: &crate::runtime::context::AgentxContext) -> Result<LaunchResult> {
    let client = Client::connect(&caller_context.agentx_dir, super::host_client::invocation(caller_context))?;
    launch_with_client(params, caller_context, &client)
}

/// Execute using the admitted caller's retained database generation. Rebinding
/// per-child invocation claims must never acquire a replacement generation.
pub(crate) fn launch_with_client(params: LaunchParams, caller_context: &crate::runtime::context::AgentxContext, client: &Client) -> Result<LaunchResult> {
    launch_with_client_inner(params, caller_context, client, None)
}

/// Preserve the selected resume/fork source through the actual reservation.
/// Validation and native launch initialization share one daemon transaction.
pub(crate) fn launch_resuming_with_client(params: LaunchParams, caller_context: &crate::runtime::context::AgentxContext, client: &Client, source: crate::agents::lifecycle_service::ResumeFence) -> Result<LaunchResult> {
    if params.count != 1 {
        bail!("resume/fork requires exactly one launch");
    }
    launch_with_client_inner(params, caller_context, client, Some(source))
}

fn launch_with_client_inner(mut params: LaunchParams, caller_context: &crate::runtime::context::AgentxContext, client: &Client, resume: Option<crate::agents::lifecycle_service::ResumeFence>) -> Result<LaunchResult> {
    if !params.bindings.is_empty() && (params.stdio || resume.is_some() || params.prior_session_id.is_some()) {
        bail!("--bind supports fresh local terminal launches only; stdio/resume/fork are unsupported");
    }
    launch_request(client, launch_service::Request::CreationSupport)?;
    let binding_paths = crate::file_bindings::preflight(&params.bindings, &caller_context.cwd)?;
    let host_scope = paths::agentx_dir();
    if host_scope.canonicalize().unwrap_or(host_scope) != caller_context.agentx_dir.canonicalize().unwrap_or_else(|_| caller_context.agentx_dir.clone()) {
        bail!("launch context scope differs from this host worker");
    }
    if client.scope_root() != caller_context.agentx_dir.canonicalize()?.as_path() {
        bail!("launch client scope differs from captured context");
    }
    let client = client.with_invocation(client.invocation().clone());
    let refresh_bindings = || -> Result<()> {
        if binding_paths.is_empty() {
            return Ok(());
        }
        let query = crate::file_bindings::Query { project: crate::file_bindings::index::project_root(&caller_context.cwd), paths: binding_paths.clone(), refresh: false, agent: None };
        match client.call(crate::service::Request::Queries(crate::service::queries::Request::FileBindings(query)))? {
            crate::service::Response::Queries(crate::service::queries::Response::FileBindings(_)) => Ok(()),
            _ => bail!("daemon does not support file bindings; no Agent started"),
        }
    };
    refresh_bindings()?;
    if params.collaboration.is_none() {
        let mut environment = caller_context.raw_env.clone();
        match client.call(crate::service::Request::Participants(crate::agents::service::Request::Resolve)) {
            Ok(crate::service::Response::Participants(crate::agents::service::Response::Actor(actor))) => {
                environment.insert("AGENTX_WORKSPACE".into(), crate::agents::workspaces::workspace(&actor.record).unwrap_or_else(|| "global".into()));
            }
            Err(crate::service::ServiceError::IdentityRequired(_) | crate::service::ServiceError::NotFound(_)) => {}
            Err(error) => return Err(error.into()),
            Ok(_) => bail!("unexpected launch caller response"),
        }
        params.collaboration = Some(resolve_launch_collaboration(&caller_context.agentx_dir, &caller_context.cwd, &environment, params.cwd.as_deref(), None, None)?);
    }
    // Shared app-server roots use a separate reservation contract and retain
    // their existing native prompt channel until that contract carries history.
    crate::agents::labels::validate_set(&params.labels)?;
    if !params.labels.is_empty() {
        if params.stdio || resume.is_some() || params.prior_session_id.is_some() { bail!("initial labels support fresh terminal launches only"); }
        launch_request(&client, launch_service::Request::LabelsSupport)?;
    }
    params.title = crate::agents::launch_title::normalize(params.title.as_deref()).map_err(anyhow::Error::msg)?;
    let shared_app_server = params.stdio && crate::integrations::codex::configuration::is_shared_app_server(&params.args);
    if shared_app_server && params.title.is_some() {
        bail!("--title is not supported by shared Codex app-server stdio; use a terminal launch");
    }
    if params.title.is_some() {
        let request = crate::service::Request::Participants(crate::agents::service::Request::Launch(crate::agents::launch_service::Request::TitleSupport));
        match client.call(request) {
            Ok(crate::service::Response::Participants(crate::agents::service::Response::Launch(crate::agents::launch_service::Response::Changed))) => {}
            _ => bail!("cannot confirm daemon launch title support; no agent was started; use a matching daemon for this scope"),
        }
    }
    if !shared_app_server {
        let fresh = resume.is_none() && params.prior_session_id.is_none();
        retain_instruction_additions(&mut params, fresh);
    }
    let collaboration = params.collaboration.as_ref().expect("launch collaboration resolved");
    if collaboration.role.is_some() || collaboration.workspace.is_some() || collaboration.endpoint || collaboration.instruction_additions.is_some() {
        let request = crate::service::Request::Participants(crate::agents::service::Request::Launch(crate::agents::launch_service::Request::WorkspaceSupport));
        match client.call(request) {
            Ok(crate::service::Response::Participants(crate::agents::service::Response::Launch(crate::agents::launch_service::Response::Changed))) => {}
            Ok(_) => bail!("daemon does not support role/workspace launch; use a matching daemon for this scope"),
            Err(error) => bail!("cannot confirm daemon role/workspace support; no agent was started: {error}"),
        }
    }
    if params.stdio && crate::integrations::codex::configuration::is_shared_app_server(&params.args) && (collaboration.role.is_some() || collaboration.workspace.is_some() || collaboration.endpoint) {
        bail!("Shared Codex app-server stdio does not yet support role/workspace launches; use a terminal launch");
    }

    // Claude background defaults to the live PTY surface (`ClaudePty`); it only
    // drops to the `-p`/`--print` surface (`Claude`/`NativePrint`) when the
    // caller explicitly passes `-p`/`--print` in the args. Both stay alive —
    // PTY hosts the live TUI, print mode loops via the stop hook. `-p` is gated
    // behind explicit opt-in because, from 2026-06-15, `claude -p` draws from a
    // separate Agent SDK credit pool on subscription plans.
    // Exact-token match is intentional: `--print` is a boolean flag, so an
    // equals form (`--print=x`) is malformed for claude and surfaces as a
    // launch failure rather than being silently mis-routed to the PTY surface.
    let claude_print = (params.tool == "claude" || params.tool == "claude-pty") && params.args.iter().any(|arg| matches!(arg.as_str(), "-p" | "--print"));

    // `claude-pty` is the PTY surface; `-p`/`--print` selects the NativePrint
    // surface. The two are mutually exclusive: passing both would route print
    // mode through the HeadlessPty runner, which is broken. Fail fast.
    if params.tool == "claude-pty" && claude_print {
        bail!(
            "The PTY surface does not support `-p`/`--print`.\n\
             Use one of:\n\
             • `agentx launch --tool claude --headless`  — live PTY session\n\
             • `agentx launch --tool claude -- -p ...`      — print/pipe mode"
        );
    }

    let normalized = if params.tool == "claude" && !claude_print { LaunchTool::CLAUDE_PTY } else { LaunchTool::from_str(&params.tool)? };
    // Keep file binding available to tools without a native initial message.
    if !binding_paths.is_empty() && !matches!(normalized.spec().launch.initial_prompt, crate::integrations::registry::InitialPromptShape::Unsupported { .. }) {
        params.initial_prompt = crate::file_bindings::launch_message(&binding_paths, params.initial_prompt.as_deref())?;
    }
    if params.stdio && (normalized.tool() != Tool::Codex || params.count != 1 || params.background || params.terminal.is_some() || params.run_here == Some(false)) {
        bail!("Native stdio requires one local foreground Codex launch without terminal selection");
    }
    let base_tool = normalized.base_tool();
    let backend = LaunchBackend::resolve(&normalized, params.background);

    // Validation
    validate_launch_count(&normalized, params.count)?;

    // AGENTX_DIR placement: refuse if it sits under a tool-protected metadata
    // directory. codex hard-denies apply_patch into these via
    // FileSystemSandboxPolicy with no approval path; claude/gemini gate them
    // behind permission prompts on every agentx write. Either way the user gets
    // a broken session — fail fast at launch with a clear message instead.
    let agentx_dir_path = paths::agentx_dir();
    if let Some(protected) = paths::protected_agentx_dir_component(&agentx_dir_path) {
        bail!(
            "AGENTX_DIR ({}) sits under a protected directory component '{}'.\n\
             AI tools (codex/claude/gemini) deny writes under .git/.codex/.claude/.agents,\n\
             which would block agentx DB writes from the launched agent.\n\
             Set AGENTX_DIR to a path outside these directories.",
            agentx_dir_path.display(),
            protected
        );
    }

    let tool_binary = normalized.cli_binary();
    if !is_tool_installed(tool_binary) {
        bail!("'{}' is not installed or not in PATH", tool_binary);
    }

    if !params.skip_validation {
        let validation_errors = validate_tool_args(&normalized, &params.args);
        if !validation_errors.is_empty() {
            bail!("{}", validation_errors.join("\n"));
        }
    }

    // Load config before hook setup so auto_approve is authoritative for
    // wrapped launches as well as manual `agentx hooks add`.
    let (agentx_config, config_errors, config_trace) = AgentxConfig::load_for_scope_observed(&caller_context.agentx_dir, &caller_context.raw_env);
    client.capture_effective_configuration(&agentx_config, config_trace);
    for msg in config_errors.values() {
        eprintln!("[agentx] warn: {msg} (using the default)");
    }

    let inside_ai_tool = caller_context.is_inside_ai_tool();
    if params.background && params.run_here == Some(true) {
        bail!("--headless cannot be combined with --run-here");
    }
    if params.run_here == Some(true) && params.terminal.as_deref().is_some_and(|mode| !matches!(mode, "here" | "default")) {
        bail!("--run-here cannot be combined with an explicit terminal; omit --run-here to use that terminal");
    }
    if backend == LaunchBackend::NativePrint && params.terminal.as_deref().is_some_and(|terminal| !matches!(terminal, "pty" | "default")) {
        bail!("print mode has no interactive terminal; use --terminal pty or omit --terminal");
    }
    let terminal_mode = if params.background && backend != LaunchBackend::NativePrint {
        let selected = params.terminal.as_deref().unwrap_or(&agentx_config.headless_terminal);
        if !matches!(selected, "pty" | "tmux") {
            bail!("headless terminal must be pty or tmux; omit --headless to open {selected}");
        }
        if selected == "tmux" {
            super::tmux::Settings::resolve(&caller_context.agentx_dir, &agentx_config)?;
        }
        Some(selected)
    } else if backend == LaunchBackend::NativePrint {
        Some("pty")
    } else {
        params.terminal.as_deref().or(Some(agentx_config.terminal.as_str()).filter(|t| !t.is_empty()))
    };
    let base_env_run_here = params.stdio || will_run_in_current_terminal(params.count, params.background, params.run_here, params.terminal.as_deref(), inside_ai_tool, std::io::IsTerminal::is_terminal(&std::io::stdin()));

    let managed_tmux = terminal_mode == Some("tmux") && !base_env_run_here;
    if managed_tmux {
        super::tmux::Settings::resolve(&caller_context.agentx_dir, &agentx_config)?;
    }
    let auto_attach = managed_tmux && !params.background && params.run_here != Some(false) && params.count == 1 && !inside_ai_tool && std::io::IsTerminal::is_terminal(&std::io::stdin()) && std::io::IsTerminal::is_terminal(&std::io::stdout());

    // Build base environment for the current launch regime, then overlay
    // config.toml + ~/.agentx/env which win.
    let fresh_codex_terminal = normalized.tool() == Tool::Codex && !params.stdio && params.prior_session_id.is_none() && resume.is_none()
        && !params.args.windows(2).any(|pair| pair[0] == "fork" && uuid::Uuid::parse_str(&pair[1]).is_ok());
    let inherited_direct_terminal = fresh_codex_terminal && client.invocation().activation_target.is_some()
        && caller_context.raw_env.get("AGENTX_CODEX_DIRECT_TERMINAL").is_some_and(|value| value == "1");
    let regime = if base_env_run_here {
        LaunchEnvRegime::RunHere
    } else if inherited_direct_terminal {
        // A qualified native parent already has the originating launch env.
        // Another login shell would add unrelated backend configuration.
        LaunchEnvRegime::HumanShell
    } else if inside_ai_tool || caller_context.raw_env.contains_key("CI") || caller_context.raw_env.contains_key("GITHUB_ACTIONS") || caller_context.raw_env.keys().any(|key| key.starts_with("CARGO_")) {
        LaunchEnvRegime::ContaminatedParent
    } else {
        LaunchEnvRegime::HumanShell
    };
    let (mut base_env, launch_env_read) = build_launch_env_captured(&agentx_config, regime, caller_context);
    if let Some(ref caller_env) = params.env {
        for (key, value) in caller_env {
            insert_effective_env(&mut base_env, key.clone(), value.clone(), cfg!(windows));
        }
    }
    let bootstrap_exclude_value = base_env.get("AGENTX_BOOTSTRAP_EXCLUDE").cloned().unwrap_or_else(|| "none".to_string());
    let bootstrap_excluded = crate::runtime::bootstrap::parse_excluded_parts(&bootstrap_exclude_value).map_err(anyhow::Error::msg)?;
    base_env.remove("AGENTX_TERMINAL");
    base_env.remove("AGENTX_CODEX_DIRECT_ROOT");
    base_env.remove("AGENTX_CODEX_DIRECT_TERMINAL");
    let terminal_mode_override = base_env.get("AGENTX_CODEX_TERMINAL").or_else(|| caller_context.raw_env.get("AGENTX_CODEX_TERMINAL")).map(String::as_str).unwrap_or("auto").to_owned();
    if fresh_codex_terminal && terminal_mode_override == "shared" {
        anyhow::ensure!(cfg!(target_os = "linux"), "shared direct Codex terminals currently require Linux");
    }
    let shared_terminal = fresh_codex_terminal && cfg!(target_os = "linux") && matches!(terminal_mode_override.as_str(), "auto" | "shared");
    if shared_terminal {
        base_env.insert("AGENTX_CODEX_TERMINAL".into(), terminal_mode_override);
    }
    if params.stdio {
        // Terminal runners normally remove these before invoking the tool.
        for key in normalized.spec().instance_state_env {
            base_env.remove(*key);
        }
    }
    ensure_tool_config_env(&normalized, &mut base_env);
    // An explicit global marker prevents descendants from rediscovering a workspace from cwd.
    base_env.insert("AGENTX_WORKSPACE".into(), collaboration.workspace.clone().unwrap_or_else(|| "global".into()));

    let requested_dir = params.cwd.as_deref().map(Path::new).unwrap_or(&caller_context.cwd);
    let requested_dir = if requested_dir.is_absolute() { requested_dir.to_path_buf() } else { caller_context.cwd.join(requested_dir) };
    let canonical_dir = std::fs::canonicalize(&requested_dir).unwrap_or(requested_dir);
    let working_dir = canonical_dir.to_str().ok_or_else(|| anyhow::anyhow!("launch directory is not valid UTF-8"))?;

    // Codex preflight and hook setup must use the same effective CODEX_HOME as
    // the child, including overrides from ~/.agentx/env and caller-provided env.
    let codex_home = if normalized.tool() == Tool::Codex { crate::integrations::launch_args::codex::resolve_codex_home_from_env(&base_env, &canonical_dir) } else { None };
    let observed_backend = if params.stdio {
        "stdio"
    } else {
        match backend {
            LaunchBackend::InteractiveVisible => "terminal",
            LaunchBackend::HeadlessPty => "pty",
            LaunchBackend::NativePrint => "headless",
        }
    };
    let observed_regime = match regime {
        LaunchEnvRegime::HumanShell => "human_shell",
        LaunchEnvRegime::ContaminatedParent => "contaminated_parent",
        LaunchEnvRegime::RunHere => "run_here",
    };
    client.capture_launch_plan(base_tool, observed_backend, observed_regime, &canonical_dir, codex_home.as_ref().map(|(path, _)| path.as_path()), launch_env_read);
    if let Some((ref path, explicit_env)) = codex_home {
        insert_effective_env(&mut base_env, "CODEX_HOME".to_string(), path.to_string_lossy().into_owned(), cfg!(windows));
        crate::integrations::launch_args::codex::ensure_codex_home_writable_at(path, explicit_env)?;
    }

    if normalized.tool() == Tool::Qoder {
        qoder_preprocessing::apply_system_prompt(&mut params.args, params.system_prompt.as_deref());
        if let Some(args) = params.persisted_args.as_mut() {
            qoder_preprocessing::apply_system_prompt(args, params.system_prompt.as_deref());
        }
    }

    // Hooks: per-run tools get a fresh injection built from the effective env,
    // cwd and args (applied to args after the persisted snapshot below);
    // persistent tools must have their global install (strict: refuse to launch
    // without hooks).
    let shared_stdio = params.stdio && normalized.tool() == Tool::Codex && crate::integrations::codex::configuration::is_shared_app_server(&params.args);
    let runtime_injection = if shared_stdio || shared_terminal {
        None
    } else {
        match crate::integrations::hooks::runtime::adapter(normalized.tool()) {
            Some(adapter) => {
                // Replayed args (resume/fork) may carry agentx-injected values from a
                // previous launch; drop them so the injection is rebuilt, not doubled.
                crate::integrations::hooks::runtime::strip_replayed_args(adapter, &mut params.args);
                if let Some(persisted) = params.persisted_args.as_mut() {
                    crate::integrations::hooks::runtime::strip_replayed_args(adapter, persisted);
                }
                let ctx = crate::integrations::hooks::runtime::LaunchCtx { tool: normalized.tool(), env: base_env.clone(), cwd: canonical_dir.clone(), args: params.args.clone(), auto_approve: agentx_config.auto_approve };
                let injection = if normalized.tool() == Tool::Codex && !params.stdio && backend != LaunchBackend::NativePrint {
                    crate::integrations::hooks::codex::plan_managed_pty(&ctx)?
                } else {
                    crate::integrations::hooks::runtime::plan(adapter, &ctx)?
                };
                for (key, value) in &injection.env {
                    insert_effective_env(&mut base_env, key.clone(), value.clone(), cfg!(windows));
                }
                Some(injection)
            }
            None if normalized.tool() == Tool::Hermes => {
                // Installed per profile, so it needs the launch's args and env.
                crate::integrations::hooks::hermes::ensure_installed(&crate::integrations::hooks::runtime::LaunchCtx {
                    tool: normalized.tool(),
                    env: base_env.clone(),
                    cwd: canonical_dir.clone(),
                    args: params.args.clone(),
                    auto_approve: agentx_config.auto_approve,
                })?;
                None
            }
            None => {
                ensure_hooks_installed(&normalized, agentx_config.auto_approve)?;
                None
            }
        }
    };

    // Explicit name validation
    if let Some(ref name) = params.name {
        if params.count > 1 {
            bail!("Cannot use explicit name with count > 1 (count={})", params.count);
        }
        if !crate::agents::identity::is_valid_name(name) {
            bail!("Invalid instance name: {name}");
        }
    }

    // System prompt file for Gemini/Codex
    if let Some(ref sp) = params.system_prompt
        && normalized.tool() == Tool::Gemini
    {
        let path = write_system_prompt_file(sp, "gemini");
        base_env.insert("GEMINI_SYSTEM_MD".to_string(), path);
    }

    // Folder trust: on first run each tool shows a "do you trust this folder?"
    // prompt — the user accepts to continue or declines and it exits. When an
    // agent launches another agent via agentx, auto-approve the prompt for the
    // launch dir to smooth the process. Cursor's lever is a marker file (its
    // `--trust` flag is print-only, inert in our PTY), so it's seeded here;
    // gemini/codex use arg injection below.
    if agentx_config.auto_trust_workspace && normalized.tool() == Tool::Cursor {
        cursor_preprocessing::ensure_cursor_workspace_trusted(&canonical_dir)?;
    }
    if agentx_config.auto_trust_workspace && normalized.tool() == Tool::Copilot {
        copilot_preprocessing::ensure_copilot_workspace_trusted(&canonical_dir)?;
    }
    if agentx_config.auto_trust_workspace && normalized.tool() == Tool::Qoder {
        qoder_preprocessing::ensure_qoder_workspace_trusted(&canonical_dir, &params.args, &base_env)?;
    }

    // Capture the persistable args BEFORE any agentx launch injection below.
    // Resume replays only user/config args; workspace-trust injection
    // (gemini `--skip-trust`, codex `-c projects=…trust_level`) and the `--agentx-prompt`
    // translation are session/path-specific and must not be baked into
    // launch_args, or they would replay stale state on resume/fork.
    let stored_launch_args = params.persisted_args.clone().unwrap_or_else(|| params.args.clone());

    // Same for per-run hook injection: `injection.args` is the complete argv
    // (caller args + agentx's merged flags), built from `params.args` above.
    // Every instance of `agentx launch --tool <tool> --count N` shares it; it holds no per-instance state.
    if let Some(injection) = runtime_injection {
        params.args = injection.args;
    }

    inject_workspace_trust_args(&normalized, &canonical_dir, &mut params.args, agentx_config.auto_trust_workspace);
    let launcher_name: String = match params.launcher.take() {
        Some(name) => name,
        None => {
            // Try to resolve caller identity from the live process binding.
            let ctx = caller_context;
            if crate::integrations::codex::roots::is_shared_context(ctx) {
                let store = crate::agents::service::RemoteAttachments::new(client.clone());
                crate::integrations::codex::roots::resolve_shared_attachment(&store, ctx, ctx.codex_thread_id.as_deref().unwrap_or(""))?
                    .ok_or_else(|| anyhow::anyhow!("Shared Codex launch caller has no qualified thread attachment"))?
                    .target
                    .participant
                    .canonical_name
            } else {
                match client.call(crate::service::Request::Participants(crate::agents::service::Request::Resolve)) {
                    Ok(crate::service::Response::Participants(crate::agents::service::Response::Actor(actor))) => actor.participant.canonical_name,
                    Ok(_) | Err(crate::service::ServiceError::IdentityRequired(_)) => "api".to_owned(),
                    Err(error) => return Err(error.into()),
                }
            }
        }
    };

    if normalized.tool() == Tool::Hermes {
        params.args = crate::integrations::launch_args::hermes::with_chat_subcommand(std::mem::take(&mut params.args));
    }

    // Inject --agentx-prompt into tool args (translated per-tool).
    // When a real agentx participant launched us, append a reply instruction so
    // the spawned agent knows to send its result back.
    if let Some(ref prompt) = params.initial_prompt {
        if shared_stdio {
            bail!("Native app-server input must arrive through its turn protocol, not --agentx-prompt");
        }
        let reply_suffix = reply_handoff_suffix(&launcher_name, params.append_reply_handoff, &bootstrap_excluded);
        let full_prompt = format!("{prompt}{reply_suffix}");
        append_initial_prompt_args(&normalized, &mut params.args, full_prompt)?;
    }
    let batch_id = params.batch_id.take().unwrap_or_else(|| format!("{:08x}", rand::rng().random::<u32>()));

    if shared_stdio {
        // A proxy is not a participant. Each correlated native root reserves
        // its own identity, then attaches before its success reaches the client.
        base_env.insert("AGENTX_DIR".to_owned(), caller_context.agentx_dir.to_string_lossy().into_owned());
        base_env.insert("AGENTX_LAUNCHED_BY".to_owned(), launcher_name);
        base_env.insert("AGENTX_LAUNCH_BATCH_ID".to_owned(), batch_id.clone());
        apply_inherited_notes(&mut base_env, caller_context.raw_env.get("AGENTX_NOTES").cloned());
        if let Some(prompt) = params.system_prompt {
            params.args.extend(["-c".to_owned(), format!("developer_instructions={}", toml::Value::String(prompt))]);
        }
        let args = codex_preprocessing::preprocess_codex_args(&params.args, "", codex_home.as_ref().map(|(path, _)| path.as_path()));
        let launch = crate::integrations::codex::bridge::RootLaunch { explicit_name: params.name, initial_cursor: params.resume_cursor, launch_args: stored_launch_args, launch_profile: params.launch_profile.clone() };
        let status = crate::integrations::codex::bridge::run_with_client(&args, &base_env, &canonical_dir, launch, agentx_config.auto_approve, &client)?;
        return Ok(LaunchResult { tool: base_tool.to_owned(), batch_id, launched: 1, failed: 0, background: false, log_files: Vec::new(), handles: Vec::new(), errors: Vec::new(), stdio_exit_code: Some(status), attach_participant: None, launched_participants: Vec::new() });
    }

    let mut launched = 0usize;
    let mut log_files: Vec<String> = Vec::new();
    let mut handles: Vec<serde_json::Value> = Vec::new();
    let mut errors: Vec<serde_json::Value> = Vec::new();
    let mut stdio_exit_code = None;
    let mut attach_participant = None;
    let mut launched_participants = Vec::new();

    for _ in 0..params.count {
        let mut instance_env = base_env.clone();
        // A resume's message cursor predates its previous incarnation's
        // events (e.g. the kill that stopped it), so "did this launch get
        // killed" needs its own marker for where this launch began.

        instance_env.insert("AGENTX_LAUNCHED".to_string(), "1".to_string());
        instance_env.insert("AGENTX_LAUNCHED_BY".to_string(), launcher_name.to_string());
        instance_env.insert("AGENTX_LAUNCH_BATCH_ID".to_string(), batch_id.clone());
        instance_env.insert("AGENTX_DIR".to_string(), caller_context.agentx_dir.to_string_lossy().to_string());

        // Propagate dev root
        if let Some(val) = caller_context.raw_env.get("AGENTX_DEV_ROOT") {
            instance_env.insert("AGENTX_DEV_ROOT".to_string(), val.clone());
        }
        // Propagate AGENTX_NOTES from the launcher's own environment so a nested
        // launch (an agent that inherited notes spawning a child) doesn't lose
        // them — but only as a fallback (see `apply_inherited_notes`).
        apply_inherited_notes(&mut instance_env, caller_context.raw_env.get("AGENTX_NOTES").cloned());

        let process_id = generate_process_id();
        instance_env.insert("AGENTX_PROCESS_ID".to_string(), process_id.clone());

        // Fork mode detection
        if normalized.tool() == Tool::Claude && params.args.iter().any(|a| a == "--fork-session") {
            instance_env.insert("AGENTX_IS_FORK".to_string(), "1".to_string());
        }

        let mut launch_context = crate::runtime::context::AgentxContext::from_env(&instance_env, canonical_dir.clone()).with_environment_file_keys(caller_context.environment_file_keys.clone());
        launch_context.agentx_dir = caller_context.agentx_dir.clone();
        let mut invocation = client.invocation().clone();
        invocation.launch_context = Some(serde_json::Value::Object(crate::agents::binding::capture_context_snapshot(&caller_context.agentx_dir, &canonical_dir, &instance_env)));
        let reservation_client = client.with_invocation(invocation);
        let input = launch_service::Reserve {
            name: params.name.clone(),
            tool: base_tool.to_owned(),
            directory: canonical_dir.clone(),
            process_binding: process_id.clone(),
            prior_session: params.prior_session_id.clone(),
            resume_cursor: params.resume_cursor,
            background: params.background,
            launch_args: stored_launch_args.clone(),
            launch_profile: params.launch_profile.clone(),
            collaboration: params.collaboration.clone(),
            title: params.title.clone(),
            labels: params.labels.clone(),
        };
        let reserved = if let Some(source) = &resume {
            match reservation_client.call(crate::service::Request::Participants(crate::agents::service::Request::Lifecycle(crate::agents::lifecycle_service::Request::ReserveResume { source: source.clone(), launch: input }))) {
                Ok(crate::service::Response::Participants(crate::agents::service::Response::Lifecycle(crate::agents::lifecycle_service::Response::ResumeReserved(reservation)))) => Ok(launch_service::Response::Reserved(reservation)),
                Ok(_) => Err(anyhow::anyhow!("unexpected native resume reservation response")),
                Err(error @ crate::service::ServiceError::OutcomeUnknown(_)) => Err(anyhow::anyhow!(
                    "resume reservation for '{}' did not return a definite result: {error}. This attempt did not start the provider; check 'agentx list {}' and 'agentx list --stopped {}' in the same scope",
                    source.name.as_deref().unwrap_or(&source.session_id),
                    source.name.as_deref().unwrap_or(&source.session_id),
                    source.name.as_deref().unwrap_or(&source.session_id)
                )),
                Err(error) => Err(anyhow::Error::new(error)),
            }
        } else {
            launch_request(&reservation_client, launch_service::Request::Reserve(input))
        };
        let reservation = match reserved {
            Ok(launch_service::Response::Reserved(reservation)) => reservation,
            Ok(_) => bail!("unexpected launch reservation response"),
            Err(error) => {
                errors.push(json!({"tool":base_tool,"error":error.to_string()}));
                continue;
            }
        };
        // Reservation observes the initiating caller. Bootstrap and supervisor
        // operations instead act as the newly reserved child through its own
        // process binding, retaining this client's admitted generation/peer.
        let launch_client = client.with_invocation(super::host_client::invocation(&launch_context));
        let instance_name = reservation.participant.canonical_name.clone();
        instance_env.insert("AGENTX_LAUNCH_PARTICIPANT".to_owned(), serde_json::to_string(&reservation.participant)?);
        instance_env.insert("AGENTX_LAUNCH_GENERATION".to_owned(), launch_client.info().database_generation.clone());
        instance_env.insert("AGENTX_LAUNCH_EVENT_ID".to_owned(), reservation.initial_cursor.to_string());
        instance_env.insert("AGENTX_LAUNCH_START_EVENT_ID".to_owned(), reservation.launch_cursor.to_string());
        instance_env.insert("AGENTX_INSTANCE_NAME".to_string(), instance_name.clone());
        if shared_terminal {
            instance_env.insert("AGENTX_CODEX_DIRECT_ROOT".into(), params.title.clone().filter(|title| !title.trim().is_empty()).unwrap_or_else(|| instance_name.clone()));
        }

        // Process ID export: allow custom env var name
        if let Some(export_var) = caller_context.raw_env.get("AGENTX_PROCESS_ID_EXPORT").cloned()
            && !export_var.is_empty()
        {
            instance_env.insert(export_var, process_id.clone());
        }

        // Name/process export vars
        if let Some(export_var) = caller_context.raw_env.get("AGENTX_NAME_EXPORT").cloned() {
            if !export_var.is_empty() {
                instance_env.insert(export_var, instance_name.clone());
            }
        } else if !agentx_config.name_export.is_empty() {
            instance_env.insert(agentx_config.name_export.clone(), instance_name.clone());
        }

        let tool_type = base_tool;
        instance_env.insert("AGENTX_TOOL".to_string(), tool_type.to_string());
        // A resumed OpenCode session emits no session event until the first
        // prompt, so the plugin binds at startup from this instead. Always set
        // (empty unless resuming) so a launch from inside a resumed agent does
        // not inherit it.
        if matches!(tool_type, "opencode" | "kilo") {
            instance_env.insert("AGENTX_RESUME_SESSION_ID".to_string(), params.prior_session_id.clone().unwrap_or_default());
        }

        // Pre-format the pane title for templates that substitute
        // `{pane_title}` (custom user templates only — the built-in herdr
        // preset passes `{instance_name}` and the delivery loop pushes the
        // styled label via `pane.rename`). Use the same identity projection as
        // the PTY host, based on this launch's adopted role and title.
        let display_for_title = crate::agents::launch_title::pane_display(&instance_name, collaboration.role.as_ref().map(|role| role.name.as_str()), params.title.as_deref());
        instance_env.insert("AGENTX_PANE_TITLE".to_string(), crate::shared::format_pane_title(crate::shared::ST_LISTENING, &display_for_title, tool_type));

        // Claude's background daemon keeps settings but not launch env; see
        // `pin_launch_environment`.
        let mut params = params.clone();
        if normalized.tool() == Tool::Claude
            && let Err(error) = crate::integrations::hooks::claude::pin_launch_environment(&mut params.args, &instance_env, normalized.is_claude_pty())
        {
            let _ = launch_request(&reservation_client, launch_service::Request::CancelFailed(reservation.clone()));
            errors.push(json!({"tool": base_tool, "error": error.to_string()}));
            continue;
        }

        let assignment = crate::file_bindings::Assignment { scope: client.scope_root().to_string_lossy().into_owned(), generation: client.info().database_generation.clone(), participant: reservation.participant.clone() };
        let mut binding_changes = Vec::new();
        // Dispatch to tool-specific launcher
        let launch_result = (|| -> Result<bool> {
            binding_changes = crate::file_bindings::bind(&binding_paths, &assignment)?;
            refresh_bindings()?;
            // Freeze once before any Provider child starts; later hooks only restore it.
            let frozen_instructions = build_launch_bootstrap_client(&launch_client, &caller_context.agentx_dir, &canonical_dir, &instance_name, base_tool, params.background, &instance_env)?;
            if normalized.tool() == Tool::Grok {
                let (_, dynamic) = super::agent_context::select_sources(agentx_config.instruction_sources.as_deref(), &agentx_config.context_sources)?;
                if !dynamic.is_empty() {
                    bail!("Grok does not provide a runtime context channel; move fixed content to instructions.sources");
                }
            }
            match normalized.tool() {
                Tool::Claude if !normalized.is_claude_pty() => {
                    let claude_cmd = build_claude_command(&params.args);

                    // Plain `claude` only resolves to NativePrint (background,
                    // direct spawn in print mode) or InteractiveVisible — the
                    // PTY-backed `claude-pty` takes the shared arm below.
                    if matches!(backend, LaunchBackend::NativePrint) {
                        let log_filename = format!("background_{}_{}.log", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0), rand::random::<u16>() % 9000 + 1000);
                        instance_env.insert("AGENTX_BACKGROUND".to_string(), log_filename.clone());

                        let (launch_result, effective_preset) = terminal::launch_terminal(
                            &claude_cmd,
                            &instance_env,
                            Some(working_dir),
                            true, // background
                            false,
                            terminal_mode,
                            inside_ai_tool,
                            Some("claude"),
                        )?;
                        match launch_result {
                            terminal::LaunchResult::Background(log_file, pid) => {
                                finalize_background_launch(
                                    &mut BackgroundLaunchCtx {
                                        client: &reservation_client,
                                        reservation: &reservation,
                                        tool: "claude",
                                        instance_name: &instance_name,
                                        process_id: &process_id,
                                        terminal_mode,
                                        working_dir,
                                        log_files: &mut log_files,
                                        handles: &mut handles,
                                    },
                                    log_file,
                                    pid,
                                    effective_preset,
                                )?;
                                Ok(true)
                            }
                            _ => Ok(false),
                        }
                    } else {
                        let effective_run_here = will_run_in_current_terminal(params.count, false, params.run_here, params.terminal.as_deref(), inside_ai_tool, std::io::IsTerminal::is_terminal(&std::io::stdin()));
                        let (launch_result, effective_preset) = terminal::launch_terminal(&claude_cmd, &instance_env, Some(working_dir), false, effective_run_here, terminal_mode, inside_ai_tool, Some("claude"))?;
                        record_terminal(&reservation_client, &reservation, terminal_mode, &effective_preset, None)?;

                        match launch_result {
                            terminal::LaunchResult::Success => {
                                handles.push(json!({"tool": "claude", "instance_name": instance_name}));
                                Ok(true)
                            }
                            _ => Ok(false),
                        }
                    }
                }

                _ => {
                    let mut effective_args = params.args.clone();

                    match normalized.tool() {
                        Tool::Codex => {
                            // Build effective args: system_prompt + preprocessing
                            if let Some(ref sp) = params.system_prompt {
                                let mut pre = vec!["-c".to_string(), format!("developer_instructions={}", sp)];
                                pre.extend(effective_args);
                                effective_args = pre;
                            }

                            // Generate bootstrap text for preprocessing
                            let bootstrap = &frozen_instructions;

                            effective_args = codex_preprocessing::preprocess_codex_args(&effective_args, &bootstrap, codex_home.as_ref().map(|(path, _)| path.as_path()));
                        }
                        Tool::OpenCode | Tool::Kilo | Tool::Pi | Tool::Omp | Tool::Hermes => {
                            opencode_preprocessing::preprocess_opencode_env(&mut instance_env, &instance_name);
                            effective_args = if normalized.tool() == Tool::OpenCode { opencode_preprocessing::preprocess_opencode_args(&params.args, &mut instance_env, std::path::Path::new(working_dir))? } else { params.args.clone() };
                        }
                        Tool::Antigravity => {
                            // Bootstrap arrives through SessionStart; leave name_announced unset.
                            instance_env.insert("ANTIGRAVITY_AGENT".to_string(), "1".to_string());
                        }
                        Tool::Grok => {
                            // Grok hook output never reaches the model, so the bootstrap
                            // goes in at launch through `--rules` (appended to the
                            // system prompt).
                            let bootstrap = &frozen_instructions;

                            inject_grok_rules(&mut effective_args, &bootstrap);
                            ensure_grok_session_id(&mut effective_args);
                            if let Some(ref sp) = params.system_prompt {
                                inject_grok_rules(&mut effective_args, sp);
                            }
                        }
                        _ => {}
                    }

                    if params.stdio {
                        // Track the supervisor for stale-process detection even
                        // between turns, without advertising any PTY endpoints.
                        launch_request(&reservation_client, launch_service::Request::RecordSupervisor(reservation.clone()))?;
                        let outcome = crate::runtime::codex_stdio::run(&effective_args, &instance_env, working_dir);
                        let current_name = finish_supervisor(&reservation_client, &launch_context, &reservation).unwrap_or_else(|error| {
                            eprintln!("[agentx] warn: stdio session cleanup failed: {error}");
                            instance_name.clone()
                        });
                        stdio_exit_code = Some(outcome?);
                        handles.push(json!({"tool": base_tool, "instance_name": current_name}));
                        return Ok(true);
                    }
                    launch_pty_or_background(
                        &mut BackgroundLaunchCtx {
                            client: &reservation_client,
                            reservation: &reservation,
                            tool: base_tool,
                            instance_name: &instance_name,
                            process_id: &process_id,
                            terminal_mode,
                            working_dir,
                            log_files: &mut log_files,
                            handles: &mut handles,
                        },
                        &mut instance_env,
                        &effective_args,
                        &params,
                        inside_ai_tool,
                        &agentx_config,
                    )
                }
            }
        })();

        if !matches!(launch_result, Ok(true)) {
            let recovery = crate::file_bindings::rollback(&binding_changes, &assignment);
            if !recovery.is_empty() {
                errors.push(json!({"tool": base_tool, "error": recovery}));
            }
            let _ = refresh_bindings();
        }
        match launch_result {
            Ok(true) => {
                launched += 1;
                if !params.stdio {
                    launched_participants.push(reservation.participant.clone());
                }
                if auto_attach {
                    attach_participant = Some(reservation.participant.clone());
                }
            }
            Ok(false) => {
                let _ = launch_request(&reservation_client, launch_service::Request::CancelFailed(reservation.clone()));
            }
            Err(e) => {
                let _ = launch_request(&reservation_client, launch_service::Request::CancelFailed(reservation.clone()));
                errors.push(json!({"tool": base_tool, "error": e.to_string()}));
            }
        }
    }

    let failed = params.count - launched;
    if launched == 0 {
        if !errors.is_empty() {
            let details: Vec<String> = errors.iter().filter_map(|e| e.get("error").and_then(|v| v.as_str()).map(|s| s.to_string())).collect();
            bail!("No instances launched (0/{}): {}", params.count, details.join("; "));
        }
        bail!("No instances launched (0/{})", params.count);
    }

    let _ = launch_request(
        &client,
        launch_service::Request::BatchLaunched(launch_service::Batch {
            launcher: launcher_name,
            batch_id: batch_id.clone(),
            tool: base_tool.to_owned(),
            requested: params.count,
            launched,
            failed,
            background: params.background,
            instances: handles.iter().filter_map(|handle| handle.get("instance_name").and_then(serde_json::Value::as_str).map(str::to_owned)).collect(),
        }),
    );

    // Push launch event to relay (best-effort)
    crate::relay::spawn_background_push();

    Ok(LaunchResult {
        // User-facing identity. The PTY-vs-print backend distinction lives in
        // `LaunchBackend`, not the tool string, so consumers see `claude`.
        tool: base_tool.to_string(),
        batch_id,
        launched,
        failed,
        background: params.background,
        log_files,
        handles,
        errors,
        stdio_exit_code,
        attach_participant,
        launched_participants,
    })
}

/// Validate tool args (pure parsing, no mutation).
pub(crate) fn validate_tool_args(tool: &LaunchTool, args: &[String]) -> Vec<String> {
    match tool.tool() {
        Tool::Claude | Tool::Codex | Tool::Adhoc => Vec::new(),
        Tool::Gemini => validate_rejected_args("Gemini", "agentx launch --tool gemini", args, GEMINI_REJECTED_ARGS),
        Tool::Cursor => crate::integrations::launch_args::cursor::validate_cursor_args(args),
        Tool::Kimi => validate_rejected_args("Kimi", "agentx launch --tool kimi", args, KIMI_REJECTED_ARGS),
        Tool::OpenCode => validate_rejected_args("OpenCode", "agentx launch --tool opencode", args, OPENCODE_REJECTED_ARGS),
        Tool::Kilo => validate_rejected_args("Kilo", "agentx launch --tool kilo", args, KILO_REJECTED_ARGS),
        Tool::Pi => validate_rejected_args("Pi", "agentx launch --tool pi", args, PI_REJECTED_ARGS),
        Tool::Omp => validate_rejected_args("Oh My Pi", "agentx launch --tool omp", args, OMP_REJECTED_ARGS),
        Tool::Hermes => crate::integrations::launch_args::hermes::validate_hermes_args(args),
        Tool::Antigravity => validate_rejected_args("Antigravity", "agentx launch --tool antigravity", args, ANTIGRAVITY_REJECTED_ARGS),
        Tool::Copilot => crate::integrations::launch_args::copilot::validate_copilot_args(args),
        Tool::Qoder => validate_rejected_args("Qoder", "agentx launch --tool qoder", args, QODER_REJECTED_ARGS),
        Tool::Grok => {
            let borrowed: Vec<_> = args.iter().map(String::as_str).collect();
            validate_grok_args(&borrowed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    use crate::integrations::hooks::test_helpers::EnvVarGuard;

    /// Unset every variable launch-context detection reads.
    fn clean_detection_env() -> EnvVarGuard {
        let mut keys: Vec<String> = tool_marker_vars().iter().copied().chain(["AGENTX_LAUNCHED", "CI", "GITHUB_ACTIONS", "CARGO_TEST_PARENT"]).map(str::to_string).collect();
        keys.extend(std::env::vars_os().map(|(key, _)| key).filter_map(|key| key.into_string().ok()).filter(|key| key.starts_with("CARGO_")));
        EnvVarGuard::remove(keys)
    }

    #[test]
    fn instruction_additions_preserve_raw_history_and_separate_resume_identity() {
        let collaboration = crate::agents::workspaces::LaunchCollaboration { workspace: None, role: None, endpoint: false, instruction_additions: None };
        let mut params = LaunchParams { collaboration: Some(collaboration), system_prompt: Some("raw custom instructions".into()), ..Default::default() };
        retain_instruction_additions(&mut params, true);
        assert_eq!(params.collaboration.as_ref().unwrap().instruction_additions.as_deref(), Some("raw custom instructions"));
        assert!(params.system_prompt.is_none());

        params.system_prompt = Some("new incarnation identity".into());
        retain_instruction_additions(&mut params, false);
        assert_eq!(params.collaboration.as_ref().unwrap().instruction_additions.as_deref(), Some("raw custom instructions"));
        assert_eq!(params.system_prompt.as_deref(), Some("new incarnation identity"));

        params.system_prompt = Some("raw custom instructions".into());
        retain_instruction_additions(&mut params, false);
        assert!(params.system_prompt.is_none());

        params.collaboration.as_mut().unwrap().instruction_additions = None;
        params.system_prompt = Some("legacy resume identity".into());
        retain_instruction_additions(&mut params, false);
        assert!(params.collaboration.as_ref().unwrap().instruction_additions.is_none());
        assert_eq!(params.system_prompt.as_deref(), Some("legacy resume identity"));
    }

    #[test]
    fn launch_workspace_global_prevents_cwd_rediscovery() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join(".agentx/roles")).unwrap();
        let env = HashMap::from([("AGENTX_WORKSPACE".into(), "global".into())]);
        let global = resolve_launch_collaboration(temp.path(), temp.path(), &env, None, None, None).unwrap();
        assert!(global.workspace.is_none());
        let local = resolve_launch_collaboration(temp.path(), temp.path(), &env, Some("."), None, None).unwrap();
        assert_eq!(local.workspace, Some(temp.path().canonicalize().unwrap().to_string_lossy().into_owned()));
        assert!(!local.endpoint);
    }

    #[test]
    fn test_launch_tool_from_str() {
        assert_eq!(LaunchTool::from_str("claude").unwrap(), LaunchTool::from(crate::integrations::tool::Tool::Claude));
        assert_eq!(LaunchTool::from_str("claude-pty").unwrap(), LaunchTool::CLAUDE_PTY);
        assert_eq!(LaunchTool::from_str("gemini").unwrap(), LaunchTool::from(crate::integrations::tool::Tool::Gemini));
        assert_eq!(LaunchTool::from_str("codex").unwrap(), LaunchTool::from(crate::integrations::tool::Tool::Codex));
        assert_eq!(LaunchTool::from_str("opencode").unwrap(), LaunchTool::from(crate::integrations::tool::Tool::OpenCode));
        assert_eq!(LaunchTool::from_str("kilo").unwrap(), LaunchTool::from(crate::integrations::tool::Tool::Kilo));
        assert_eq!(LaunchTool::from_str("kilocode").unwrap(), LaunchTool::from(crate::integrations::tool::Tool::Kilo));
        assert_eq!(LaunchTool::from_str("antigravity").unwrap(), LaunchTool::from(crate::integrations::tool::Tool::Antigravity));
        assert_eq!(LaunchTool::from_str("agy").unwrap(), LaunchTool::from(crate::integrations::tool::Tool::Antigravity));
        assert_eq!(LaunchTool::from_str("copilot").unwrap(), LaunchTool::from(crate::integrations::tool::Tool::Copilot));
        assert_eq!(LaunchTool::from_str("grok").unwrap(), LaunchTool::from(crate::integrations::tool::Tool::Grok));
        assert_eq!(LaunchTool::from_str("grok-build").unwrap(), LaunchTool::from(crate::integrations::tool::Tool::Grok));
        assert!(LaunchTool::from_str("unknown").is_err());
    }

    #[test]
    fn launch_count_uses_per_tool_spec_limit() {
        assert!(validate_launch_count(&LaunchTool::from(crate::integrations::tool::Tool::Kimi), 10).is_ok());
        let err = validate_launch_count(&LaunchTool::from(crate::integrations::tool::Tool::Kimi), 11).unwrap_err();
        assert!(err.to_string().contains("max 10"));

        assert!(validate_launch_count(&LaunchTool::from(crate::integrations::tool::Tool::Claude), 100).is_ok());
        let err = validate_launch_count(&LaunchTool::from(crate::integrations::tool::Tool::Claude), 101).unwrap_err();
        assert!(err.to_string().contains("max 100"));
    }

    #[test]
    fn unsupported_initial_prompt_is_spec_driven() {
        let mut args = Vec::new();
        let err = append_initial_prompt_args(&LaunchTool::from(crate::integrations::tool::Tool::Kimi), &mut args, "task".into()).unwrap_err();
        assert!(err.to_string().contains("kimi does not support an initial prompt"));
        assert!(args.is_empty());

        append_initial_prompt_args(&LaunchTool::from(crate::integrations::tool::Tool::Gemini), &mut args, "task".into()).unwrap();
        assert_eq!(args, vec!["task"]);
    }

    #[test]
    fn initial_prompt_flag_shape_appends_after_native_prompt() {
        let mut args = vec!["--prompt".to_string(), "native prompt".to_string()];
        append_initial_prompt_args(&LaunchTool::from(crate::integrations::tool::Tool::OpenCode), &mut args, "agentx prompt".into()).unwrap();
        assert_eq!(args, vec!["--prompt", "native prompt", "--prompt", "agentx prompt"]);
    }

    #[test]
    fn initial_prompt_positional_shape_appends_after_native_prompt() {
        let mut args = vec!["native prompt".to_string()];
        append_initial_prompt_args(&LaunchTool::from(crate::integrations::tool::Tool::Gemini), &mut args, "agentx prompt".into()).unwrap();
        assert_eq!(args, vec!["native prompt", "agentx prompt"]);
    }

    #[test]
    fn initial_prompt_dash_dash_shape_appends_after_native_prompt() {
        let mut args = vec!["--".to_string(), "native prompt".to_string()];
        append_initial_prompt_args(&LaunchTool::from(crate::integrations::tool::Tool::Claude), &mut args, "agentx prompt".into()).unwrap();
        assert_eq!(args, vec!["--", "native prompt", "--", "agentx prompt"]);
    }

    #[test]
    fn agentx_prompt_alone_is_injected_for_every_shape() {
        for (tool, expected) in [
            (LaunchTool::from(crate::integrations::tool::Tool::OpenCode), vec!["--prompt".to_string(), "agentx".to_string()]),
            (LaunchTool::from(crate::integrations::tool::Tool::Gemini), vec!["agentx".to_string()]),
            (LaunchTool::from(crate::integrations::tool::Tool::Claude), vec!["--".to_string(), "agentx".to_string()]),
        ] {
            let mut args = Vec::new();
            append_initial_prompt_args(&tool, &mut args, "agentx".into()).unwrap();
            assert_eq!(args, expected);
        }
    }

    #[test]
    fn positional_shape_does_not_treat_model_value_as_prompt() {
        let mut args = vec!["--model".to_string(), "safe-model".to_string()];
        append_initial_prompt_args(&LaunchTool::from(crate::integrations::tool::Tool::Gemini), &mut args, "agentx".into()).unwrap();
        assert_eq!(args.last().map(String::as_str), Some("agentx"));
    }

    #[test]
    fn validate_grok_rejects_one_shot_and_ignored_flags() {
        let errors = validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Grok), &["-p".to_string(), "task".to_string()]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("-p") || errors[0].contains("single"));
        assert!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Grok), &["--always-approve".to_string()]).is_empty());
        assert_eq!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Grok), &["--no-leader".into()]).len(), 1);
        for flag in ["--deny=bash", "--disable-web-search", "--allow", "--no-memory", "--experimental-memory", "--storage-mode=local", "--tools=Read", "--disallowed-tools=Bash", "--agents={}", "--max-turns=1", "--memory-flush"] {
            assert_eq!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Grok), &[flag.to_string()]).len(), 1);
        }
        assert!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Grok), &["--".into(), "--tools=Read".into()]).is_empty());
        assert!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Grok), &["--no-subagents".to_string()]).is_empty());
    }

    #[test]
    fn validate_cursor_print_mode_fails_fast() {
        let errors = validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Cursor), &["--print".to_string()]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not supported"));
    }

    #[test]
    fn validate_kimi_rejects_prompt_mode_but_allows_resume() {
        let errors = validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Kimi), &["--prompt".to_string()]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("--prompt"));
        assert!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Kimi), &["--session".to_string()]).is_empty());
    }

    #[test]
    fn validate_opencode_rejects_run_but_allows_session() {
        let errors = validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::OpenCode), &["run".to_string()]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("run"));
        assert!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::OpenCode), &["--session".to_string()]).is_empty());
        assert!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::OpenCode), &["--prompt".to_string()]).is_empty());
    }

    #[test]
    fn validate_kilo_rejects_serve_but_allows_continue() {
        let errors = validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Kilo), &["serve".to_string()]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("serve"));
        assert!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Kilo), &["--continue".to_string()]).is_empty());
    }

    #[test]
    fn validate_pi_rejects_print_but_allows_fork() {
        let errors = validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Pi), &["--print".to_string()]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("--print"));
        assert!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Pi), &["--fork".to_string()]).is_empty());
    }

    #[test]
    fn validate_antigravity_rejects_print_alias_but_allows_conversation() {
        let errors = validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Antigravity), &["--prompt".to_string()]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("--prompt"));
        assert!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Antigravity), &["--conversation".to_string()]).is_empty());
        assert!(validate_tool_args(&LaunchTool::from(crate::integrations::tool::Tool::Antigravity), &["--prompt-interactive".to_string()]).is_empty());
    }

    #[test]
    fn test_launch_tool_as_str() {
        assert_eq!(LaunchTool::from(crate::integrations::tool::Tool::Claude).as_str(), "claude");
        assert_eq!(LaunchTool::CLAUDE_PTY.as_str(), "claude-pty");
        assert_eq!(LaunchTool::from(crate::integrations::tool::Tool::Gemini).as_str(), "gemini");
        assert_eq!(LaunchTool::from(crate::integrations::tool::Tool::Antigravity).as_str(), "antigravity");
        assert_eq!(LaunchTool::from(crate::integrations::tool::Tool::Copilot).as_str(), "copilot");
    }

    #[test]
    fn test_launch_tool_base_tool() {
        assert_eq!(LaunchTool::from(crate::integrations::tool::Tool::Claude).base_tool(), "claude");
        assert_eq!(LaunchTool::CLAUDE_PTY.base_tool(), "claude");
        assert_eq!(LaunchTool::from(crate::integrations::tool::Tool::Codex).base_tool(), "codex");
        assert_eq!(LaunchTool::from(crate::integrations::tool::Tool::Antigravity).base_tool(), "antigravity");
        assert_eq!(LaunchTool::from(crate::integrations::tool::Tool::Copilot).base_tool(), "copilot");
    }

    #[test]
    fn test_launch_tool_uses_pty() {
        assert!(!LaunchTool::from(crate::integrations::tool::Tool::Claude).uses_pty());
        assert!(LaunchTool::CLAUDE_PTY.uses_pty());
        assert!(LaunchTool::from(crate::integrations::tool::Tool::Gemini).uses_pty());
        assert!(LaunchTool::from(crate::integrations::tool::Tool::Codex).uses_pty());
        assert!(LaunchTool::from(crate::integrations::tool::Tool::OpenCode).uses_pty());
        assert!(LaunchTool::from(crate::integrations::tool::Tool::Kilo).uses_pty());
        assert!(LaunchTool::from(crate::integrations::tool::Tool::Copilot).uses_pty());
    }

    #[test]
    fn test_launch_backend_resolve_interactive() {
        // Any tool, !background → InteractiveVisible (visible terminal).
        for tool in [
            LaunchTool::from(crate::integrations::tool::Tool::Claude),
            LaunchTool::CLAUDE_PTY,
            LaunchTool::from(crate::integrations::tool::Tool::Gemini),
            LaunchTool::from(crate::integrations::tool::Tool::Codex),
            LaunchTool::from(crate::integrations::tool::Tool::OpenCode),
            LaunchTool::from(crate::integrations::tool::Tool::Kilo),
            LaunchTool::from(crate::integrations::tool::Tool::Antigravity),
            LaunchTool::from(crate::integrations::tool::Tool::Copilot),
        ] {
            assert_eq!(LaunchBackend::resolve(&tool, false), LaunchBackend::InteractiveVisible, "{:?} should resolve to InteractiveVisible without background", tool);
        }
    }

    #[test]
    fn test_launch_backend_resolve_claude_native_print() {
        // claude `-p`/`--print` surface (`Claude`) + background → NativePrint
        // (detached -p stream-json). Only chosen when the caller passes -p.
        assert_eq!(LaunchBackend::resolve(&LaunchTool::from(crate::integrations::tool::Tool::Claude), true), LaunchBackend::NativePrint);
    }

    #[test]
    fn test_launch_backend_resolve_claude_pty_headless() {
        // claude --headless default surface (`ClaudePty`) → HeadlessPty
        // (PTY wrapper, live TUI).
        assert_eq!(LaunchBackend::resolve(&LaunchTool::CLAUDE_PTY, true), LaunchBackend::HeadlessPty);
    }

    #[test]
    fn test_launch_backend_resolve_other_tools_headless() {
        // gemini/codex/opencode + --headless → HeadlessPty (unchanged from today).
        for tool in [
            LaunchTool::from(crate::integrations::tool::Tool::Gemini),
            LaunchTool::from(crate::integrations::tool::Tool::Codex),
            LaunchTool::from(crate::integrations::tool::Tool::OpenCode),
            LaunchTool::from(crate::integrations::tool::Tool::Kilo),
            LaunchTool::from(crate::integrations::tool::Tool::Antigravity),
            LaunchTool::from(crate::integrations::tool::Tool::Copilot),
        ] {
            assert_eq!(LaunchBackend::resolve(&tool, true), LaunchBackend::HeadlessPty, "{:?} --headless should be HeadlessPty", tool);
        }
    }

    #[test]
    fn test_will_run_in_current_terminal() {
        // Explicit override
        assert!(will_run_in_current_terminal(5, false, Some(true), None, false, false));
        assert!(!will_run_in_current_terminal(1, false, Some(false), None, false, true));

        // terminal=here
        assert!(will_run_in_current_terminal(5, false, None, Some("here"), false, false));

        // Inside AI tool → always new window
        assert!(!will_run_in_current_terminal(1, false, None, None, true, true));

        // Background → never run here
        assert!(!will_run_in_current_terminal(1, true, None, None, false, true));

        // No interactive stdin (TUI RPC, pipes) → new window
        assert!(!will_run_in_current_terminal(1, false, None, None, false, false));

        // Single → run here, multiple → new window
        assert!(will_run_in_current_terminal(1, false, None, None, false, true));
        assert!(!will_run_in_current_terminal(2, false, None, None, false, true));
        // Explicit terminal overrides implicit current-terminal selection.
        assert!(!will_run_in_current_terminal(1, false, None, Some("tmux"), false, true));
        assert!(!will_run_in_current_terminal(1, false, None, Some("iterm"), false, true));
        assert!(will_run_in_current_terminal(1, false, None, Some("default"), false, true));
    }

    #[test]
    fn test_build_claude_command() {
        let args = vec!["--model".to_string(), "sonnet".to_string()];
        let cmd = build_claude_command(&args);
        if cfg!(windows) {
            // ps_quote() always quotes, unlike the POSIX shell_quote() used
            // elsewhere, which leaves plain args unquoted.
            assert_eq!(cmd, "claude '--model' 'sonnet'");
        } else {
            assert_eq!(cmd, "claude --model sonnet");
        }
    }

    #[test]
    fn test_build_claude_command_with_spaces() {
        let args = vec!["--prompt".to_string(), "fix all tests".to_string()];
        let cmd = build_claude_command(&args);
        assert!(cmd.contains("'fix all tests'"));
    }

    #[test]
    fn test_background_runner_env_includes_instance_name() {
        let mut env = HashMap::new();
        env.insert("AGENTX_PROCESS_ID".to_string(), "pid-123".to_string());

        let runner_env = background_runner_env("codex", &env, "nita");

        assert_eq!(runner_env.get("AGENTX_INSTANCE_NAME").map(String::as_str), Some("nita"));
        assert_eq!(runner_env.get("AGENTX_PROCESS_ID").map(String::as_str), Some("pid-123"));
        assert!(!runner_env.contains_key("AGENTX_PTY_MODE"));
        assert_eq!(runner_env.get("HERDR_AGENT").map(String::as_str), Some("codex"));
    }

    #[test]
    fn test_background_runner_env_includes_claude_pty_mode() {
        let env = HashMap::new();

        let runner_env = background_runner_env("claude", &env, "hone");

        assert_eq!(runner_env.get("AGENTX_INSTANCE_NAME").map(String::as_str), Some("hone"));
        assert_eq!(runner_env.get("AGENTX_PTY_MODE").map(String::as_str), Some("1"));
    }

    #[test]
    fn test_background_runner_env_antigravity_sets_agent() {
        let runner_env = background_runner_env("antigravity", &HashMap::new(), "nabe");
        assert_eq!(runner_env.get("ANTIGRAVITY_AGENT").map(String::as_str), Some("1"));
    }

    #[test]
    fn test_env_strip_set_strips_closed_categories() {
        let strip = env_strip_set();
        // AGENTX identity
        assert!(strip.contains("AGENTX_PROCESS_ID"));
        assert!(strip.contains("AGENTX_LAUNCHED"));
        // Tool markers
        assert!(strip.contains("CLAUDECODE"));
        assert!(strip.contains("CLAUDE_ENV_FILE"));
        assert!(strip.contains("CODEX_SANDBOX"));
        assert!(strip.contains("CODEX_THREAD_ID"));
        assert!(strip.contains("GEMINI_SYSTEM_MD"));
        assert!(strip.contains("AGENTX_TOOL"));
        assert!(strip.contains("AGENTX_PI"));
        assert!(!strip.contains("PI_CODING_AGENT_DIR"));
        // Terminal context
        assert!(strip.contains("KITTY_WINDOW_ID"));
        assert!(strip.contains("TMUX_PANE"));
        assert!(!strip.contains("COLORTERM"));
        assert!(!strip.contains("TERM"));
    }

    #[test]
    #[serial]
    fn test_clean_detection_env_clears_and_restores_all_markers() {
        let _original = clean_detection_env();
        for var in tool_marker_vars() {
            unsafe { std::env::set_var(var, "1") };
        }
        assert!(contaminated_parent());

        {
            let _clean = clean_detection_env();
            for var in tool_marker_vars() {
                assert!(std::env::var_os(var).is_none(), "{var} was not cleared");
            }
            assert!(!contaminated_parent());
        }

        for var in tool_marker_vars() {
            assert_eq!(std::env::var(var).as_deref(), Ok("1"), "{var} was not restored");
        }
    }

    #[test]
    #[serial]
    fn test_contaminated_parent_detection() {
        let _guard = clean_detection_env();
        assert!(!contaminated_parent());

        use crate::integrations::detection::{AGENTX_TOOL, EnvMatch, EnvPredicate};
        for spec in crate::integrations::registry::ALL.iter().filter(|s| s.tool != crate::integrations::tool::Tool::Adhoc) {
            let fallback = [EnvPredicate::equals(AGENTX_TOOL, spec.name)];
            for predicate in spec.detection.markers.iter().chain(&fallback) {
                let value = match predicate.condition {
                    EnvMatch::Equals(value) => value,
                    _ => "1",
                };
                unsafe { std::env::set_var(predicate.var, value) };
                assert!(contaminated_parent(), "{}={value}", predicate.var);
                unsafe { std::env::remove_var(predicate.var) };
                assert!(!contaminated_parent(), "after removing {}", predicate.var);
            }
        }

        unsafe { std::env::set_var("AGENTX_LAUNCHED", "1") };
        assert!(contaminated_parent());
        unsafe { std::env::remove_var("AGENTX_LAUNCHED") };

        unsafe { std::env::set_var("CI", "1") };
        assert!(contaminated_parent());
        unsafe { std::env::remove_var("CI") };

        unsafe { std::env::set_var("GITHUB_ACTIONS", "true") };
        assert!(contaminated_parent());
        unsafe { std::env::remove_var("GITHUB_ACTIONS") };

        unsafe { std::env::set_var("CARGO_TEST_PARENT", "1") };
        assert!(contaminated_parent());
    }

    #[test]
    #[serial]
    fn test_build_launch_env_inherits_parent_env() {
        unsafe { std::env::set_var("RORI_TEST_MY_VAR", "hello") }
        unsafe { std::env::set_var("RORI_TEST_OPENROUTER_API_KEY", "sk-test-123") }
        unsafe { std::env::set_var("RORI_TEST_PI_OFFLINE", "1") }

        let config = crate::config::AgentxConfig::default();
        let env = build_launch_env(&config, LaunchEnvRegime::HumanShell);

        assert_eq!(env.get("RORI_TEST_MY_VAR").map(String::as_str), Some("hello"));
        assert_eq!(env.get("RORI_TEST_OPENROUTER_API_KEY").map(String::as_str), Some("sk-test-123"));
        assert_eq!(env.get("RORI_TEST_PI_OFFLINE").map(String::as_str), Some("1"));

        unsafe { std::env::remove_var("RORI_TEST_MY_VAR") }
        unsafe { std::env::remove_var("RORI_TEST_OPENROUTER_API_KEY") }
        unsafe { std::env::remove_var("RORI_TEST_PI_OFFLINE") }
    }

    #[test]
    #[serial]
    fn test_build_launch_env_agent_regime_uses_resolved_shell_base() {
        let _guard = EnvVarGuard::remove(vec!["RORI_PARENT_CONTAMINATION".to_string(), "RORI_RESOLVED_AUTH".to_string()]);
        unsafe { std::env::set_var("RORI_PARENT_CONTAMINATION", "leak") };

        let config = crate::config::AgentxConfig::default();
        let env = build_launch_env_with_resolver(&config, LaunchEnvRegime::ContaminatedParent, || Some(HashMap::from([("RORI_RESOLVED_AUTH".to_string(), "auth-token".to_string())])));

        assert!(!env.contains_key("RORI_PARENT_CONTAMINATION"));
        assert_eq!(env.get("RORI_RESOLVED_AUTH").map(String::as_str), Some("auth-token"));
    }

    #[test]
    #[serial]
    fn test_tool_config_env_copies_ambient_override_into_clean_child_env() {
        let _guard = EnvVarGuard::remove(vec!["CODEX_HOME".to_string()]);
        unsafe { std::env::set_var("CODEX_HOME", "/isolated/codex-home") };
        let mut env = HashMap::from([("HOME".to_string(), "/clean-shell-home".to_string())]);

        ensure_tool_config_env(&LaunchTool::from(crate::integrations::tool::Tool::Codex), &mut env);

        assert_eq!(env.get("CODEX_HOME").map(String::as_str), Some("/isolated/codex-home"));
    }

    #[test]
    #[serial]
    fn test_project_local_agentx_dir_does_not_redirect_tool_config() {
        let _guard = EnvVarGuard::remove(vec!["AGENTX_DIR".to_string(), "CLAUDE_CONFIG_DIR".to_string(), "CODEX_HOME".to_string()]);
        let dir = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("AGENTX_DIR", dir.path().join(".agentx")) };

        for tool in [LaunchTool::from(crate::integrations::tool::Tool::Claude), LaunchTool::from(crate::integrations::tool::Tool::Codex)] {
            let mut env = HashMap::new();
            ensure_tool_config_env(&tool, &mut env);
            assert!(env.is_empty(), "{tool:?} got {env:?}");
        }
    }

    #[test]
    fn test_windows_env_override_replaces_different_key_casing() {
        let mut env = HashMap::from([("CODEX_HOME".to_string(), r"C:\ambient-codex-home".to_string())]);

        insert_effective_env(&mut env, "Codex_Home".to_string(), r"C:\caller-codex-home".to_string(), true);

        assert_eq!(env.len(), 1);
        assert_eq!(effective_env_value(&env, "CODEX_HOME", true), Some(r"C:\caller-codex-home"));
    }

    #[test]
    #[serial]
    fn test_build_launch_env_strips_closed_categories() {
        unsafe { std::env::set_var("AGENTX_PROCESS_ID", "pid-stale") }
        unsafe { std::env::set_var("CLAUDECODE", "1") }
        unsafe { std::env::set_var("CODEX_THREAD_ID", "thread-stale") }
        unsafe { std::env::set_var("PI_CODING_AGENT_DIR", "/tmp/pi-config") }
        unsafe { std::env::set_var("KITTY_WINDOW_ID", "1337") }

        let config = crate::config::AgentxConfig::default();
        let env = build_launch_env(&config, LaunchEnvRegime::HumanShell);

        assert!(!env.contains_key("AGENTX_PROCESS_ID"));
        assert!(!env.contains_key("CLAUDECODE"));
        assert!(!env.contains_key("CODEX_THREAD_ID"));
        assert_eq!(env.get("PI_CODING_AGENT_DIR").map(String::as_str), Some("/tmp/pi-config"));
        assert!(!env.contains_key("KITTY_WINDOW_ID"));

        unsafe { std::env::remove_var("AGENTX_PROCESS_ID") }
        unsafe { std::env::remove_var("CLAUDECODE") }
        unsafe { std::env::remove_var("CODEX_THREAD_ID") }
        unsafe { std::env::remove_var("PI_CODING_AGENT_DIR") }
        unsafe { std::env::remove_var("KITTY_WINDOW_ID") }
    }

    #[test]
    #[serial]
    fn test_build_launch_env_run_here_inherits_terminal_vars() {
        let _guard = EnvVarGuard::remove(vec!["NO_COLOR".to_string(), "AGENTX_PROCESS_ID".to_string()]);
        unsafe { std::env::set_var("NO_COLOR", "1") };
        unsafe { std::env::set_var("AGENTX_PROCESS_ID", "pid-stale") };

        let config = crate::config::AgentxConfig::default();
        let env = build_launch_env_with_resolver(&config, LaunchEnvRegime::RunHere, || panic!("run_here must not resolve shell env"));

        assert_eq!(env.get("NO_COLOR").map(String::as_str), Some("1"));
        assert!(!env.contains_key("AGENTX_PROCESS_ID"));
    }

    #[test]
    #[serial]
    fn test_build_launch_env_fail_open_to_parent_env() {
        let _guard = EnvVarGuard::remove(vec!["RORI_FAIL_OPEN_PARENT".to_string()]);
        unsafe { std::env::set_var("RORI_FAIL_OPEN_PARENT", "present") };

        let config = crate::config::AgentxConfig::default();
        let env = build_launch_env_with_resolver(&config, LaunchEnvRegime::ContaminatedParent, || None);

        assert_eq!(env.get("RORI_FAIL_OPEN_PARENT").map(String::as_str), Some("present"));
    }

    #[test]
    #[serial]
    fn test_build_launch_env_config_overrides_ambient() {
        unsafe { std::env::set_var("AGENTX_HINTS", "ambient hints") }

        let config = crate::config::AgentxConfig { hints: "config hints".to_string(), ..Default::default() };
        let env = build_launch_env(&config, LaunchEnvRegime::HumanShell);

        assert_eq!(env.get("AGENTX_HINTS").map(String::as_str), Some("config hints"));

        unsafe { std::env::remove_var("AGENTX_HINTS") }
    }

    #[test]
    fn test_inject_grok_rules_prepends_when_absent() {
        let mut args = vec!["--always-approve".to_string()];
        inject_grok_rules(&mut args, "BOOT");
        assert_eq!(args, vec!["--rules".to_string(), "BOOT".to_string(), "--always-approve".to_string()]);
    }

    #[test]
    fn grok_new_launch_gets_session_id_unless_it_opens_one() {
        let mut args = vec!["--always-approve".to_string()];
        ensure_grok_session_id(&mut args);
        assert_eq!(args[0], "--session-id");
        assert!(uuid::Uuid::parse_str(&args[1]).is_ok());
        for existing in [vec!["--resume", "abc"], vec!["-c"], vec!["--session-id=abc"], vec!["--resume", "abc", "--fork-session"], vec!["--load=abc"], vec!["-rabc"], vec!["-sabc"], vec!["-cmopenrouter-free"]] {
            let mut args: Vec<String> = existing.iter().map(|s| s.to_string()).collect();
            let before = args.clone();
            ensure_grok_session_id(&mut args);
            assert_eq!(args, before);
        }
        let mut literal = vec!["--".to_string(), "--resume".to_string()];
        ensure_grok_session_id(&mut literal);
        assert_eq!(literal[0], "--session-id");
        for args in [["-m", "crs"], ["-mcrs", "--always-approve"], ["-wcrs", "--always-approve"]] {
            let mut args = args.map(str::to_string).to_vec();
            ensure_grok_session_id(&mut args);
            assert_eq!(args[0], "--session-id");
        }
    }

    #[test]
    fn grok_prompt_that_looks_like_a_flag_stays_a_prompt() {
        let mut args = Vec::new();
        append_initial_prompt_args(&LaunchTool::from(crate::integrations::tool::Tool::Grok), &mut args, "--rules=x".into()).unwrap();
        assert_eq!(args, ["--", "--rules=x"]);
        inject_grok_rules(&mut args, "BOOT");
        assert_eq!(args, ["--rules", "BOOT", "--", "--rules=x"]);
        let mut args = Vec::new();
        append_initial_prompt_args(&LaunchTool::from(crate::integrations::tool::Tool::Grok), &mut args, "--continue".into()).unwrap();
        ensure_grok_session_id(&mut args);
        assert_eq!(args[0], "--session-id");
    }

    #[test]
    fn test_inject_grok_rules_prefixes_existing_value() {
        let mut args = vec!["--rules".to_string(), "user-rules".to_string()];
        inject_grok_rules(&mut args, "BOOT");
        assert_eq!(args[0], "--rules");
        assert!(args[1].starts_with("BOOT"));
        assert!(args[1].contains("user-rules"));
    }

    #[test]
    fn test_grok_bootstrap_uses_automatic_delivery() {
        let db = launcher_test_db();
        let agentx_dir = tempfile::tempdir().unwrap();
        let bootstrap = build_grok_bootstrap(&db, agentx_dir.path(), "kumo", true, &HashMap::new());
        assert!(bootstrap.contains("Your name: kumo"));
        assert!(bootstrap.contains("Messages instantly and automatically arrive"));
    }

    #[test]
    fn test_handoff_exclusion_controls_reply_suffix() {
        let included = crate::runtime::bootstrap::parse_excluded_parts("none").unwrap();
        let excluded = crate::runtime::bootstrap::parse_excluded_parts("handoff").unwrap();
        assert_eq!(reply_handoff_suffix("nova", true, &included), "\n\nWhen done, send your result back to @nova via agentx.");
        assert!(reply_handoff_suffix("nova", true, &excluded).is_empty());
        assert!(reply_handoff_suffix("user", true, &included).is_empty());
        assert!(reply_handoff_suffix("nova", false, &included).is_empty());
    }

    #[test]
    fn test_codex_bootstrap_uses_effective_exclusion_env() {
        let db = launcher_test_db();
        let agentx_dir = tempfile::tempdir().unwrap();
        let env = HashMap::from([("AGENTX_BOOTSTRAP_EXCLUDE".to_string(), "messages".to_string())]);
        let bootstrap = build_codex_bootstrap(&db, agentx_dir.path(), "luna", false, &env);
        assert!(!bootstrap.contains("## MESSAGES"));
        assert!(!bootstrap.contains("## DELIVERY"));
        assert!(bootstrap.contains("## CAPABILITIES"));
    }

    #[test]
    fn test_codex_bootstrap_includes_notes_from_effective_instance_env() {
        let db = launcher_test_db();
        let agentx_dir = tempfile::tempdir().unwrap();
        let instance_env = HashMap::from([("AGENTX_NOTES".to_string(), "instance-specific notes".to_string())]);

        let bootstrap = build_codex_bootstrap(&db, agentx_dir.path(), "luna", false, &instance_env);

        assert!(bootstrap.contains("## NOTES"));
        assert!(bootstrap.contains("instance-specific notes"));
    }

    #[test]
    fn test_codex_bootstrap_omits_notes_section_when_effective_env_has_none() {
        let db = launcher_test_db();
        let agentx_dir = tempfile::tempdir().unwrap();

        let bootstrap = build_codex_bootstrap(&db, agentx_dir.path(), "luna", false, &HashMap::new());

        assert!(!bootstrap.contains("## NOTES"));
    }

    #[test]
    fn test_codex_bootstrap_omits_notes_section_for_empty_env_value() {
        // `AGENTX_NOTES=""` is the documented way to clear notes; it is
        // structurally different from a missing key and must still produce no
        // `## NOTES` section.
        let db = launcher_test_db();
        let agentx_dir = tempfile::tempdir().unwrap();
        let instance_env = HashMap::from([("AGENTX_NOTES".to_string(), String::new())]);

        let bootstrap = build_codex_bootstrap(&db, agentx_dir.path(), "luna", false, &instance_env);

        assert!(!bootstrap.contains("## NOTES"));
    }

    #[test]
    fn test_inherited_notes_fill_gap_when_absent() {
        // Nested launch with no explicit notes: the parent's inherited value
        // propagates so the child doesn't lose it.
        let mut env = HashMap::new();
        apply_inherited_notes(&mut env, Some("from-parent".to_string()));
        assert_eq!(env.get("AGENTX_NOTES").map(String::as_str), Some("from-parent"));
    }

    #[test]
    fn test_inherited_notes_do_not_override_explicit_value() {
        // Explicit notes (config.toml / ~/.agentx/env / LaunchParams.env, already
        // in instance_env via base_env) must win over an inherited parent value.
        let mut env = HashMap::from([("AGENTX_NOTES".to_string(), "explicit".to_string())]);
        apply_inherited_notes(&mut env, Some("from-parent".to_string()));
        assert_eq!(env.get("AGENTX_NOTES").map(String::as_str), Some("explicit"));
    }

    #[test]
    fn test_inherited_notes_do_not_override_intentional_clear() {
        // An intentional empty value (`AGENTX_NOTES=""`) clears notes and must not
        // be resurrected by an inherited parent value.
        let mut env = HashMap::from([("AGENTX_NOTES".to_string(), String::new())]);
        apply_inherited_notes(&mut env, Some("from-parent".to_string()));
        assert_eq!(env.get("AGENTX_NOTES").map(String::as_str), Some(""));
    }

    #[test]
    fn test_inherited_notes_noop_when_parent_unset() {
        let mut env = HashMap::new();
        apply_inherited_notes(&mut env, None);
        assert!(!env.contains_key("AGENTX_NOTES"));
    }

    #[test]
    fn test_codex_notes_survive_developer_instructions_toml_transport() {
        // The helper only produces the intermediate bootstrap string. Notes
        // actually reach Codex through `preprocess_codex_args`, which
        // TOML-encodes the bootstrap into `-c developer_instructions=...`.
        // Assert on the final, TOML-decoded argument so quotes, backslashes,
        // braces, and newlines are proven to survive the real transport.
        let db = launcher_test_db();
        let agentx_dir = tempfile::tempdir().unwrap();
        let notes = "Use \"review mode\".\nWindows path: C:\\work\\repo\n{literal braces}\nSecond line";
        let instance_env = HashMap::from([("AGENTX_NOTES".to_string(), notes.to_string())]);

        let bootstrap = build_codex_bootstrap(&db, agentx_dir.path(), "luna", false, &instance_env);

        let args = crate::integrations::launch_args::codex::preprocess_codex_args(&[], &bootstrap, None);

        // Locate the `-c developer_instructions=<TOML>` value and decode it.
        // Other per-run config overrides may precede it, so match by prefix rather
        // than by the first `-c` position.
        let encoded = args.iter().find_map(|a| a.strip_prefix("developer_instructions=")).expect("developer_instructions arg present");
        let decoded: toml::Table = toml::from_str(&format!("x = {encoded}")).expect("developer_instructions is valid TOML");
        let dev_instructions = decoded["x"].as_str().expect("string value");

        assert!(dev_instructions.contains("## NOTES"));
        assert!(dev_instructions.contains("Use \"review mode\"."));
        assert!(dev_instructions.contains("C:\\work\\repo"));
        assert!(dev_instructions.contains("{literal braces}"));
        assert!(dev_instructions.contains("Second line"));
    }

    #[test]
    #[serial]
    fn test_background_runner_env_uses_upstream_resolved_base() {
        let _guard = EnvVarGuard::remove(vec!["RORI_BACKGROUND_CONTAMINATION".to_string()]);
        unsafe { std::env::set_var("RORI_BACKGROUND_CONTAMINATION", "leak") };

        let config = crate::config::AgentxConfig::default();
        let env = build_launch_env_with_resolver(&config, LaunchEnvRegime::ContaminatedParent, || Some(HashMap::from([("RORI_BACKGROUND_AUTH".to_string(), "auth-token".to_string())])));
        let runner_env = background_runner_env("codex", &env, "nita");

        assert!(!runner_env.contains_key("RORI_BACKGROUND_CONTAMINATION"));
        assert_eq!(runner_env.get("RORI_BACKGROUND_AUTH").map(String::as_str), Some("auth-token"));
    }

    #[test]
    fn test_runner_binary_dirs_prioritize_selected_node_and_deduplicate() {
        let resolved = HashMap::from([("codex", "pnpm/bin/codex"), ("agentx", "target/debug/agentx"), ("node", "nvm/bin/node"), ("python3", "system/bin/python3")]);
        // Model an earlier dev-root/current-exe insertion. Resolving agentx to the
        // same directory must not add it twice.
        let binaries = resolve_runner_binaries(vec!["target/debug".to_string()], "codex", |name| resolved.get(name).map(ToString::to_string));

        assert_eq!(binaries.path_dirs, ["nvm/bin", "target/debug", "pnpm/bin", "system/bin"].map(ToString::to_string));
        assert_eq!(binaries.tool_path.as_deref(), Some("pnpm/bin/codex"));
        assert!(binaries.path_dirs.iter().position(|dir| dir == "nvm/bin") < binaries.path_dirs.iter().position(|dir| dir == "system/bin"), "the selected Node directory must precede every generic system directory");
    }

    #[test]
    fn test_runner_binary_dirs_keep_system_tool_explicit_behind_selected_node() {
        let resolved = HashMap::from([("codex", "system/bin/codex"), ("agentx", "system/bin/agentx"), ("node", "nvm/bin/node"), ("python3", "system/bin/python3")]);

        let binaries = resolve_runner_binaries(vec![], "codex", |name| resolved.get(name).map(ToString::to_string));

        assert_eq!(binaries.path_dirs, ["nvm/bin", "system/bin"].map(ToString::to_string));
        assert_eq!(binaries.tool_path.as_deref(), Some("system/bin/codex"));
    }

    #[cfg(unix)]
    #[test]
    fn test_agent_agentx_resolution_follows_runner_dir_order() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let install = |dir: &str, bytes: &[u8]| {
            let dir = temp.path().join(dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("agentx"), bytes).unwrap();
            std::fs::set_permissions(dir.join("agentx"), std::fs::Permissions::from_mode(0o755)).unwrap();
            dir
        };
        let node_dir = install("usr-local-bin", b"brew agentx");
        let uv_tool_bin = install("uv/tools/agentx/bin", b"uv agentx");
        let local_bin = temp.path().join("local-bin");
        std::fs::create_dir_all(&local_bin).unwrap();
        std::os::unix::fs::symlink(uv_tool_bin.join("agentx"), local_bin.join("agentx")).unwrap();
        let launching = uv_tool_bin.join("agentx");

        // A second install in the selected Node's dir shadows the launcher.
        let found = first_agentx_in([node_dir.clone(), local_bin.clone()].into_iter()).unwrap();
        assert!(!same_binary(&found, &launching));

        // A symlinked entry point and a byte-identical copy are the same agentx.
        let found = first_agentx_in([temp.path().join("missing"), local_bin].into_iter()).unwrap();
        assert!(same_binary(&found, &launching));
        let copy_dir = install("copy", b"uv agentx");
        assert!(same_binary(&copy_dir.join("agentx"), &launching));

        assert!(first_agentx_in([temp.path().join("missing")].into_iter()).is_none());

        // Shells skip a non-executable `agentx`, so must the check.
        let stale = install("stale", b"uv agentx");
        std::fs::set_permissions(stale.join("agentx"), std::fs::Permissions::from_mode(0o644)).unwrap();
        let found = first_agentx_in([stale, node_dir.clone()].into_iter()).unwrap();
        assert_eq!(found, node_dir.join("agentx"));
    }

    // Unix-only: asserts the bash runner's `. 'sidecar'` sourcing + unset block;
    // Windows generates a PowerShell runner with a different shape.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn test_runner_script_strips_instance_state_vars() {
        // The runner lands in $AGENTX_DIR/.tmp/launch, which other tests repoint.
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let env = HashMap::from([
            ("GEMINI_PTY_INFO".to_string(), "child_process".to_string()),
            ("GEMINI_API_KEY".to_string(), "gem-key".to_string()),
            ("GEMINI_CLI".to_string(), "1".to_string()),
            ("HERDR_PANE_ID".to_string(), "w1:old".to_string()),
            ("HERDR_SOCKET_PATH".to_string(), "/tmp/herdr.sock".to_string()),
            ("KITTY_WINDOW_ID".to_string(), "17".to_string()),
            ("TERM".to_string(), "dumb".to_string()),
            ("COLORTERM".to_string(), "truecolor".to_string()),
            ("NO_COLOR".to_string(), "1".to_string()),
            ("FORCE_COLOR".to_string(), "1".to_string()),
            ("RORI_MY_VAR".to_string(), "myval".to_string()),
        ]);

        let script = create_runner_script("gemini", "/tmp", "test", &env, &[], false).unwrap();

        let content = std::fs::read_to_string(&script).unwrap();
        // Instance-state stripped from unset block
        assert!(content.contains("GEMINI_PTY_INFO"), "GEMINI_PTY_INFO should appear in unset");
        let env_file = content.lines().find_map(|line| line.trim().strip_prefix(". ")).map(|path| path.trim_matches('\'').to_string()).expect("runner script should source a sidecar env file");
        let sidecar = std::fs::read_to_string(&env_file).unwrap();
        assert!(!sidecar.contains("GEMINI_PTY_INFO"));
        assert!(!sidecar.contains("GEMINI_CLI"));
        assert!(!sidecar.contains("TERM="));
        assert!(!sidecar.contains("COLORTERM="));
        assert!(!sidecar.contains("NO_COLOR="));
        assert!(!sidecar.contains("FORCE_COLOR="));
        assert!(!sidecar.contains("HERDR_PANE_ID="));
        assert!(!sidecar.contains("KITTY_WINDOW_ID="));
        assert!(sidecar.contains("HERDR_SOCKET_PATH="));
        assert!(sidecar.contains("GEMINI_API_KEY"));
        assert!(sidecar.contains("RORI_MY_VAR"));

        std::fs::remove_file(&script).ok();
        std::fs::remove_file(env_file).ok();
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn test_run_here_runner_preserves_current_pane_identity() {
        // The runner lands in $AGENTX_DIR/.tmp/launch, which other tests repoint.
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let env = HashMap::from([("HERDR_PANE_ID".to_string(), "w1:current".to_string()), ("RORI_MY_VAR".to_string(), "myval".to_string())]);

        let script = create_runner_script("gemini", "/tmp", "test", &env, &[], true).unwrap();
        let content = std::fs::read_to_string(&script).unwrap();
        let env_file = content.lines().find_map(|line| line.trim().strip_prefix(". ")).map(|path| path.trim_matches('\'').to_string()).expect("runner script should source a sidecar env file");
        let sidecar = std::fs::read_to_string(&env_file).unwrap();
        assert!(sidecar.contains("HERDR_PANE_ID="));

        std::fs::remove_file(&script).ok();
        std::fs::remove_file(env_file).ok();
    }

    // create_runner_script_windows() isn't cfg(windows)-gated (only its call
    // site is, via a runtime cfg!(windows) check), so this runs on any host.
    #[test]
    #[serial]
    fn test_runner_script_windows_has_bom_and_propagates_exit_code() {
        // The runner lands in $AGENTX_DIR/.tmp/launch, which other tests repoint.
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let env = HashMap::from([("SOME_SECRET".to_string(), "sekrit".to_string()), ("herdr_pane_id".to_string(), "w1:old".to_string()), ("HERDR_SOCKET_PATH".to_string(), r"C:\tmp\herdr.sock".to_string())]);

        let script = create_runner_script_windows("gemini", "/tmp", "test-win", &env, &[], false).unwrap();

        let bytes = std::fs::read(&script).unwrap();
        assert_eq!(&bytes[..3], b"\xEF\xBB\xBF", "PowerShell 5.1 misreads BOM-less files as the legacy ANSI code page");
        let content = String::from_utf8(bytes[3..].to_vec()).unwrap();
        assert!(content.contains("exit $LASTEXITCODE"), "runner must surface the wrapped process's real exit code, not always report success");
        let environment_ready = content.find("[agentx runner] environment ready").expect("background launches should expose the environment stage");
        let sidecar_source = content.find("Test-Path '").expect("ambient env should be sourced from a sidecar file");
        let wrapper_start = content.find("[agentx runner] starting PTY wrapper").expect("background launches should expose the wrapper stage");
        assert!(environment_ready < sidecar_source);
        assert!(sidecar_source < wrapper_start);

        let sidecar = content.split("Test-Path '").nth(1).and_then(|s| s.split('\'').next()).expect("ambient env should be sourced from a sidecar file");
        let sidecar_bytes = std::fs::read(sidecar).unwrap();
        assert_eq!(&sidecar_bytes[..3], b"\xEF\xBB\xBF", "sidecar env file needs the same BOM as the runner script");
        let sidecar_content = String::from_utf8_lossy(&sidecar_bytes);
        assert!(sidecar_content.contains("SOME_SECRET"));
        assert!(!sidecar_content.to_ascii_lowercase().contains("herdr_pane_id"));
        assert!(sidecar_content.contains("HERDR_SOCKET_PATH"));

        std::fs::remove_file(&script).ok();
        std::fs::remove_file(sidecar).ok();
    }

    #[test]
    fn test_windows_runner_invocation_reuses_outer_powershell() {
        let command = runner_invocation_command_for_platform(r"C:\tmp\it's runner.ps1", true);
        assert_eq!(command, r"& 'C:\tmp\it''s runner.ps1'");
        assert!(!command.to_ascii_lowercase().contains("powershell"), "the outer PowerShell must not launch a redundant nested host");
    }

    #[test]
    fn test_unix_runner_invocation_uses_bash() {
        let command = runner_invocation_command_for_platform("/tmp/it's runner.sh", false);
        assert_eq!(command, "bash '/tmp/it'\\''s runner.sh'");
    }

    // Tool args must travel via the JSON sidecar, never inline on the run
    // line: powershell.exe passes embedded double quotes unescaped to native
    // executables, so the child re-splits argv at quote boundaries (#66 —
    // `agentx launch --tool codex` failed on its quote-bearing `-c` values).
    #[test]
    #[serial]
    fn test_runner_script_windows_passes_args_via_sidecar_file() {
        // The runner lands in $AGENTX_DIR/.tmp/launch, which other tests repoint.
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let env = HashMap::new();
        let args = vec!["-c".to_string(), r#"projects={ "C:\repo" = { trust_level = "trusted" } }"#.to_string(), "-c".to_string(), "developer_instructions=multi\nline \"quoted\" text".to_string()];

        let script = create_runner_script_windows("codex", "/tmp", "test-args", &env, &args, false).unwrap();
        let content = std::fs::read_to_string(&script).unwrap();

        let run_line = content.lines().find(|l| l.contains(" pty codex")).expect("runner must invoke agentx pty");
        assert!(run_line.contains("--agentx-args-file"));
        assert!(!run_line.contains("trust_level"));
        assert!(!run_line.contains("developer_instructions"));

        let args_file = run_line.split("--agentx-args-file '").nth(1).and_then(|s| s.split('\'').next()).expect("run line should quote the args file path");
        let json = std::fs::read_to_string(args_file).unwrap();
        let roundtrip: Vec<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, args, "args must survive the file round-trip exactly");

        std::fs::remove_file(&script).ok();
        std::fs::remove_file(args_file).ok();
    }

    #[test]
    #[serial]
    fn test_runner_script_windows_no_args_skips_sidecar_file() {
        // The runner lands in $AGENTX_DIR/.tmp/launch, which other tests repoint.
        let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        let env = HashMap::new();
        let script = create_runner_script_windows("gemini", "/tmp", "test-noargs", &env, &[], false).unwrap();
        let content = std::fs::read_to_string(&script).unwrap();
        let run_line = content.lines().find(|l| l.contains(" pty gemini")).expect("runner must invoke agentx pty");
        assert!(!run_line.contains("--agentx-args-file"));
        std::fs::remove_file(&script).ok();
    }

    #[test]
    #[serial]
    fn test_same_tool_nesting_strips_instance_state() {
        unsafe { std::env::set_var("GEMINI_PTY_INFO", "child_process") }
        unsafe { std::env::set_var("GEMINI_API_KEY", "parent-key") }

        let config = crate::config::AgentxConfig::default();
        let mut env = build_launch_env(&config, LaunchEnvRegime::HumanShell);

        let gemini_spec: &'static crate::integrations::registry::IntegrationSpec = crate::integrations::tool::Tool::Gemini.spec();
        for var in gemini_spec.instance_state_env {
            env.remove(*var);
        }

        assert!(!env.contains_key("GEMINI_PTY_INFO"));
        assert_eq!(env.get("GEMINI_API_KEY").map(String::as_str), Some("parent-key"));

        unsafe { std::env::remove_var("GEMINI_PTY_INFO") }
        unsafe { std::env::remove_var("GEMINI_API_KEY") }
    }

    #[test]
    #[serial]
    fn test_cross_tool_nesting_forwards_auth() {
        unsafe { std::env::set_var("OPENROUTER_API_KEY", "sk-parent") }

        let config = crate::config::AgentxConfig::default();
        let env = build_launch_env(&config, LaunchEnvRegime::HumanShell);

        assert_eq!(env.get("OPENROUTER_API_KEY").map(String::as_str), Some("sk-parent"));

        unsafe { std::env::remove_var("OPENROUTER_API_KEY") }
    }

    fn launcher_test_db() -> crate::db::AgentxDb {
        let db = crate::db::AgentxDb::open_raw(std::path::Path::new(":memory:")).unwrap();
        db.init_db().unwrap();
        db
    }

    #[test]
    #[serial]
    fn launch_registration_preserves_resume_backlog_including_zero() {
        // A nested launcher inherits its parent's cursor. It must initialize
        // the resumed identity from the stopped snapshot instead.
        let _env = EnvVarGuard::remove(["AGENTX_LAUNCH_EVENT_ID".to_string()]);
        unsafe { std::env::set_var("AGENTX_LAUNCH_EVENT_ID", "1") };
        for saved_cursor in [0, 1] {
            let db = launcher_test_db();
            db.log_event("life", "luna", &json!({"action": "stopped"})).unwrap();
            let queued_id = db.log_event("message", "user", &json!({"from": "user", "text": "queued", "scope": "mentions", "mentions": ["luna"]})).unwrap();
            let params = LaunchParams { prior_session_id: Some("resumed-session".into()), resume_cursor: Some(saved_cursor), ..Default::default() };
            initialize_launch_instance(&db, &params, "luna", "codex", "/tmp", saved_cursor).unwrap();
            let row = db.get_instance_full("luna").unwrap().unwrap();
            assert_eq!(row.last_event_id, saved_cursor);
            assert_eq!(row.session_id.as_deref(), Some("resumed-session"));
            let unread = db.get_unread_messages("luna");
            assert_eq!(unread.len(), 1);
            assert_eq!(unread[0].event_id, Some(queued_id));

            db.ack_hook_delivery("luna", queued_id, false, None).unwrap();
            assert!(db.get_unread_messages("luna").is_empty());
        }
    }

    #[test]
    #[serial]
    fn launch_registration_restores_subscriptions_only_for_tracked_resume() {
        let _env = EnvVarGuard::remove(["AGENTX_LAUNCH_EVENT_ID".to_string()]);
        for resume in [true, false] {
            let db = launcher_test_db();
            let saved = json!({"id": "sub-saved", "caller": "luna", "sql": "0", "last_id": 0});
            db.log_event("life", "luna", &json!({"action": "stopped", "snapshot": {"subscriptions": [saved]}})).unwrap();
            let params = LaunchParams { prior_session_id: resume.then(|| "resumed-session".into()), resume_cursor: resume.then_some(0), ..Default::default() };
            initialize_launch_instance(&db, &params, "luna", "codex", "/tmp", 0).unwrap();
            assert_eq!(db.kv_get("events_sub:sub-saved").unwrap().is_some(), resume, "only a tracked resume inherits the stopped identity's subscriptions");
        }
    }

    #[test]
    #[serial]
    fn launch_registration_uses_fresh_cursor_and_keeps_startup_messages() {
        let _env = EnvVarGuard::remove(["AGENTX_LAUNCH_EVENT_ID".to_string()]);
        unsafe { std::env::set_var("AGENTX_LAUNCH_EVENT_ID", "0") };
        for reserved in [false, true] {
            let db = launcher_test_db();
            if reserved {
                let mut row = serde_json::Map::new();
                row.insert("status".into(), json!(instance_names::PLACEHOLDER_STATUS));
                row.insert("status_context".into(), json!(instance_names::PLACEHOLDER_CONTEXT));
                row.insert("created_at".into(), json!(1.0));
                db.save_instance_named("luna", &row).unwrap();
            }
            let message = json!({"from": "user", "text": "old", "scope": "mentions", "mentions": ["luna"]});
            let initial_cursor = db.log_event("message", "user", &message).unwrap();
            initialize_launch_instance(&db, &LaunchParams::default(), "luna", "codex", "/tmp", initial_cursor).unwrap();
            assert!(db.get_unread_messages("luna").is_empty());
            let startup_message = db.log_event("message", "user", &message).unwrap();
            let unread = db.get_unread_messages("luna");
            assert_eq!(unread.len(), 1);
            assert_eq!(unread[0].event_id, Some(startup_message));
        }
    }

    fn insert_test_instance(db: &crate::db::AgentxDb, name: &str, status: &str) {
        let now = chrono::Utc::now().timestamp() as f64;
        db.conn().execute("INSERT INTO instances (name, status, created_at, tool) VALUES (?1, ?2, ?3, 'antigravity')", rusqlite::params![name, status, now]).unwrap();
    }

    #[test]
    fn resolve_explicit_name_conflict_allows_free_name() {
        let db = launcher_test_db();
        assert!(resolve_explicit_name_conflict(&db, "luna").is_ok());
    }

    #[test]
    fn resolve_explicit_name_conflict_consumes_inactive_resume_row() {
        let db = launcher_test_db();
        insert_test_instance(&db, "zeno", "inactive");
        assert!(resolve_explicit_name_conflict(&db, "zeno").is_ok());
        // Row must be gone so the launcher can create a fresh row with the same name.
        assert!(db.get_instance("zeno").unwrap().is_none());
    }

    #[test]
    fn resolve_explicit_name_conflict_allows_pending_placeholder() {
        // A pending placeholder is the fork/resume path's own reservation
        // (reserve_generated_name). It must pass through so the launcher's
        // pre-register step can promote it — bailing here broke `agentx f`.
        let db = launcher_test_db();
        let now = chrono::Utc::now().timestamp() as f64;
        db.conn()
            .execute(
                "INSERT INTO instances (name, status, status_context, created_at, tool) \
                 VALUES (?1, ?2, ?3, ?4, 'claude')",
                rusqlite::params!["milo", instance_names::PLACEHOLDER_STATUS, instance_names::PLACEHOLDER_CONTEXT, now],
            )
            .unwrap();
        assert!(resolve_explicit_name_conflict(&db, "milo").is_ok());
        // Row must survive — the launcher promotes it in place.
        assert!(db.get_instance("milo").unwrap().is_some());
    }

    #[test]
    fn resolve_explicit_name_conflict_rejects_active_row() {
        let db = launcher_test_db();
        insert_test_instance(&db, "rune", "listening");
        let err = resolve_explicit_name_conflict(&db, "rune").unwrap_err().to_string();
        assert!(err.contains("already exists"), "unexpected: {err}");
        // Row should still be present — no deletion on conflict.
        assert!(db.get_instance("rune").unwrap().is_some());
    }

    // ── inject_workspace_trust_args ──────────────────────────────────────────

    #[test]
    fn test_auto_trust_workspace_default_true() {
        assert!(crate::config::AgentxConfig::default().auto_trust_workspace);
    }

    #[test]
    fn test_gemini_flag_on_injects_skip_trust() {
        let dir = std::path::Path::new("/some/workspace");
        let mut args = vec!["--model".to_string(), "gemini-2.5-flash".to_string()];
        inject_workspace_trust_args(&LaunchTool::from(crate::integrations::tool::Tool::Gemini), dir, &mut args, true);
        assert!(args.contains(&"--skip-trust".to_string()));
    }

    #[test]
    fn test_gemini_flag_off_no_injection() {
        let dir = std::path::Path::new("/some/workspace");
        let mut args = vec!["--model".to_string(), "gemini-2.5-flash".to_string()];
        inject_workspace_trust_args(&LaunchTool::from(crate::integrations::tool::Tool::Gemini), dir, &mut args, false);
        assert!(!args.contains(&"--skip-trust".to_string()));
    }

    #[test]
    fn test_gemini_inject_idempotent_when_present() {
        let dir = std::path::Path::new("/some/workspace");
        let mut args = vec!["--skip-trust".to_string(), "--model".to_string(), "x".to_string()];
        inject_workspace_trust_args(&LaunchTool::from(crate::integrations::tool::Tool::Gemini), dir, &mut args, true);
        assert_eq!(args.iter().filter(|a| a.as_str() == "--skip-trust").count(), 1);
    }

    #[test]
    fn test_codex_flag_on_injects_trust_level() {
        let dir = std::path::Path::new("/my/project");
        let mut args = vec!["--model".to_string(), "o4-mini".to_string()];
        inject_workspace_trust_args(&LaunchTool::from(crate::integrations::tool::Tool::Codex), dir, &mut args, true);
        let c_idx = args.iter().position(|a| a == "-c").expect("-c not injected");
        let val = &args[c_idx + 1];
        assert!(val.contains("/my/project"), "path missing: {val}");
        assert!(val.contains("trust_level"), "trust_level missing: {val}");
        assert!(val.contains("\"trusted\""), "trusted value missing: {val}");
    }

    #[test]
    fn test_codex_flag_off_no_injection() {
        let dir = std::path::Path::new("/my/project");
        let mut args = vec!["--model".to_string(), "o4-mini".to_string()];
        inject_workspace_trust_args(&LaunchTool::from(crate::integrations::tool::Tool::Codex), dir, &mut args, false);
        assert!(!args.iter().any(|a| a == "-c"));
    }

    #[test]
    fn test_codex_inject_idempotent_when_present() {
        let dir = std::path::Path::new("/my/project");
        // Any -c value containing "trust_level" suppresses re-injection.
        let existing = r#"projects={ "/my/project" = { trust_level = "trusted" } }"#.to_string();
        let mut args = vec!["-c".to_string(), existing];
        inject_workspace_trust_args(&LaunchTool::from(crate::integrations::tool::Tool::Codex), dir, &mut args, true);
        assert_eq!(args.iter().filter(|a| a.as_str() == "-c").count(), 1);
    }

    #[test]
    fn test_codex_dotted_path_encoded_as_single_key() {
        // A path like /Users/x/proj.v2 has a dot in a component. The inline-table
        // format keeps the whole path as one TOML quoted key — no dot-splitting.
        let dir = std::path::Path::new("/Users/x/proj.v2");
        let mut args: Vec<String> = vec![];
        inject_workspace_trust_args(&LaunchTool::from(crate::integrations::tool::Tool::Codex), dir, &mut args, true);
        let c_idx = args.iter().position(|a| a == "-c").expect("-c not injected");
        let val = &args[c_idx + 1];
        // The full dotted path must survive intact as one quoted string.
        assert!(val.contains("\"/Users/x/proj.v2\""), "full dotted path must be a single quoted key: {val}");
        assert!(val.contains("trust_level"), "trust_level missing: {val}");
    }

    #[test]
    fn test_codex_windows_verbatim_path_stripped_and_toml_escaped() {
        // std::fs::canonicalize yields \\?\-prefixed paths on Windows. The
        // prefix must go (codex keys projects by the plain absolute form) and
        // backslashes must be TOML-escaped or codex rejects the inline table.
        let dir = std::path::Path::new(r"\\?\C:\Users\x\proj");
        let mut args: Vec<String> = vec![];
        inject_workspace_trust_args(&LaunchTool::from(crate::integrations::tool::Tool::Codex), dir, &mut args, true);
        let val = &args[1];
        assert!(val.contains(r#""C:\\Users\\x\\proj""#), "path must be prefix-stripped and backslash-escaped: {val}");
        assert!(!val.contains(r"\\?\"), "verbatim prefix must be stripped: {val}");
    }

    #[test]
    fn test_codex_windows_verbatim_unc_path_keeps_server_form() {
        let dir = std::path::Path::new(r"\\?\UNC\server\share\dir");
        let mut args: Vec<String> = vec![];
        inject_workspace_trust_args(&LaunchTool::from(crate::integrations::tool::Tool::Codex), dir, &mut args, true);
        let val = &args[1];
        // \\?\UNC\server\... → \\server\... → TOML-escaped \\\\server\\...
        assert!(val.contains(r#""\\\\server\\share\\dir""#), "UNC path must keep its \\\\server form, escaped: {val}");
    }

    #[test]
    fn test_non_trust_tools_unaffected() {
        let dir = std::path::Path::new("/some/workspace");
        for tool in &[LaunchTool::from(crate::integrations::tool::Tool::Claude), LaunchTool::CLAUDE_PTY, LaunchTool::from(crate::integrations::tool::Tool::OpenCode)] {
            let mut args = vec!["--model".to_string(), "x".to_string()];
            inject_workspace_trust_args(tool, dir, &mut args, true);
            assert_eq!(args, vec!["--model".to_string(), "x".to_string()], "{tool:?} args should be unchanged");
        }
    }

    #[test]
    fn sidecar_ambient_env_folds_case_on_windows_only() {
        let mut env = std::collections::HashMap::new();
        env.insert("no_color".to_string(), "1".to_string());
        env.insert("NO_COLOR".to_string(), "1".to_string());
        env.insert("MY_SECRET".to_string(), "x".to_string());
        env.insert("AGENTX_X".to_string(), "y".to_string());
        let strip = ["NO_COLOR"];
        let win = sidecar_ambient_env(&env, strip.iter().copied(), true);
        assert!(!win.contains_key("no_color") && !win.contains_key("NO_COLOR"));
        assert!(win.contains_key("MY_SECRET") && !win.contains_key("AGENTX_X"));
        let unix = sidecar_ambient_env(&env, strip.iter().copied(), false);
        assert!(!unix.contains_key("NO_COLOR") && unix.contains_key("no_color")); // Unix exact-case preserved
    }
}
