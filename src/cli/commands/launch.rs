//! Launch command: `agentx launch --tool <tool> [--count N] [--terminal X] [--headless] [--agentx-prompt X] [--agentx-system-prompt X] [--batch-id X] [tool-args...]`
//!
//!
//! Parses agentx-level flags, merges env config with CLI args via tool-specific
//! parsers, then delegates to `launcher::launch()`.

pub(crate) use crate::runtime::launcher::is_background_from_args;
#[cfg(test)]
use crate::runtime::launcher::merge_tool_args;
#[cfg(test)]
use crate::runtime::launcher::prepare_launch_execution;

#[cfg(test)]
use crate::agents::identity;
use crate::cli::router::GlobalFlags;
use crate::cli::tips::{self, LaunchTipsContext};
use crate::config::AgentxConfig;
#[cfg(test)]
use crate::db::AgentxDb;
use crate::log::log_info;
use crate::runtime::launch_status::{self, LaunchStatus};
use crate::runtime::launcher::{self, LaunchParams, LaunchResult, LaunchTool};
use crate::service::{Request, Response, client::Client};
use crate::shared::AgentxContext;
use anyhow::{Result, bail};
use serde_json::json;
use std::time::Instant;

mod selection;
pub(crate) use selection::normalize_args as normalize_selection_args;

pub(crate) const INLINE_SINGLE_LAUNCH_WAIT_SECS: u64 = 15;

pub(crate) fn launcher_name_client(session: &crate::cli::client::Session) -> String {
    session.actor.as_ref().map(|actor| actor.participant.canonical_name.clone()).or_else(|| session.invocation.explicit_name.clone()).unwrap_or_else(|| "user".into())
}

/// Run the launch command. `argv` is the full argv[1..] including count/tool.
pub fn run(argv: &[String], flags: &GlobalFlags) -> Result<i32> {
    let mut parsed = parse_arguments(argv)?;
    let count = parsed.count as usize;
    let dry_run = parsed.dry_run;
    let (mut agentx_flags, native_args) = parsed.options.into_parts();
    let mut invocation = crate::service::InvocationContext::capture()?;
    invocation.explicit_name = flags.name.clone();
    if !selection::resolve(&mut parsed.tool, &mut parsed.selection, &mut agentx_flags, &invocation)? {
        return Ok(130);
    }
    if !agentx_flags.bindings.is_empty() && (agentx_flags.device.is_some() || agentx_flags.stdio) {
        bail!("--bind supports fresh local terminal launches only; remote and stdio are unsupported");
    }
    let labels = crate::cli::commands::labels::parse_set(&agentx_flags.labels).map_err(anyhow::Error::msg)?;
    if !labels.is_empty() && (agentx_flags.device.is_some() || agentx_flags.stdio) {
        bail!("--label supports fresh local terminal launches only");
    }
    let binding_paths = crate::file_bindings::preflight(&agentx_flags.bindings, &invocation.cwd)?;
    if agentx_flags.device.is_some() && agentx_flags.title.is_some() {
        bail!("--title is not supported by remote Relay launches; use a local launch on the target");
    }
    if agentx_flags.device.is_some() {
        return run_remote(parsed.tool, parsed.selection, native_args, count, agentx_flags, dry_run, invocation, flags);
    }
    let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
    let probe = agentx_flags.stdio && native_args.len() == 1 && matches!(native_args[0].as_str(), "--version" | "-V");
    let bound_caller = invocation.process_binding_id.is_some() || invocation.activation_target.is_some() || invocation.explicit_name.is_some() || invocation.environment.contains_key("AGENTX_CLAUDE_ACTOR_SESSION");
    let session = if !probe && (!dry_run || bound_caller) { Some(crate::cli::client::Session::for_launch(invocation.clone(), flags.go)?) } else { None };
    let mut selection_env = invocation.environment.clone();
    if let Some(actor) = session.as_ref().and_then(|session| session.actor.as_ref()) {
        selection_env.insert("AGENTX_WORKSPACE".into(), crate::agents::workspaces::workspace(&actor.record).unwrap_or_else(|| "global".into()));
    }
    let mut selection = parsed.selection;
    if probe {
        selection.replace_args = true;
    }
    let (mut collaboration, resolved) = crate::runtime::launch_profile::prepare_with_role(&scope, &invocation.cwd, &selection_env, &agentx_flags, parsed.tool.as_deref(), &selection, &native_args)?;
    let tool = resolved.tool.clone();
    let tool_args = resolved.args.clone();
    if agentx_flags.stdio && crate::integrations::codex::configuration::is_shared_app_server(&tool_args) {
        if agentx_flags.role.is_some() || agentx_flags.workspace.is_some() {
            bail!("Shared Codex app-server stdio does not yet support --role or --workspace; use a terminal launch");
        }
        // The proxy registers no participant. Automatic workspace discovery
        // must not turn a bare shared app-server invocation into a role launch.
        collaboration.workspace = None;
    }
    let launch_tool = LaunchTool::from_str(&tool)?;
    if count > launch_tool.spec().launch.max_launch_count {
        bail!("Too many agents requested (max {})", launch_tool.spec().launch.max_launch_count);
    }
    let mut validation_flags = agentx_flags.clone();
    if dry_run {
        validation_flags.json = false;
    }
    validate_stdio_launch(count, &launch_tool, &validation_flags)?;
    if dry_run {
        let project_hint = if agentx_flags.stdio { invocation.environment.get("PASEO_AGENT_CWD").filter(|value| !value.is_empty()).map(String::as_str) } else { None };
        agentx_flags.dir = Some(launch_working_dir(agentx_flags.dir.as_deref().or(project_hint))?);
        let mut preview = crate::runtime::launch_profile::preview(&resolved, count, &agentx_flags);
        preview["labels"] = serde_json::to_value(&labels)?;
        preview["title"] = serde_json::to_value(&agentx_flags.title)?;
        preview["bindings"] = serde_json::to_value(&binding_paths)?;
        preview["collaboration"] = serde_json::to_value(&collaboration)?;
        preview["instructions"] = json!({"composition": ["builtin", "role", "explicit additions"], "role_prompt": collaboration.role.as_ref().map(|role| &role.prompt), "additional_prompt": agentx_flags.system_prompt});
        print_profile_preview(&preview, agentx_flags.json);
        return Ok(0);
    }
    agentx_flags.run_here = json_run_here(&agentx_flags)?;
    let json_output = agentx_flags.json;
    let stdio = agentx_flags.stdio;
    let terminal = agentx_flags.terminal.clone();
    let headless = agentx_flags.headless;
    let dir_override = agentx_flags.dir.clone();
    let terminal_for_output = terminal.clone();
    let ctx = AgentxContext::from_env(&invocation.environment, invocation.cwd.clone()).with_environment_file_keys(invocation.environment_file_keys.clone()).with_tty(invocation.stdin_is_tty, invocation.stdout_is_tty);
    let agentx_config = AgentxConfig::load_for_scope(&scope, &invocation.environment);
    // A provider probes its executable with this same prefix. Do not merge
    // conversational defaults, create an identity or install hooks for a probe.
    if stdio && tool_args.len() == 1 && matches!(tool_args[0].as_str(), "--version" | "-V") {
        let mut env = launcher::build_launch_env(&agentx_config, launcher::LaunchEnvRegime::RunHere);
        let cwd = launch_working_dir(dir_override.as_deref())?;
        if let Some((home, _)) = crate::integrations::launch_args::codex::resolve_codex_home_from_env(&env, std::path::Path::new(&cwd)) {
            env.insert("CODEX_HOME".to_string(), home.to_string_lossy().into_owned());
        }
        return crate::runtime::codex_stdio::run(&tool_args, &env, &cwd);
    }
    let preview_background = headless || is_background_from_args(&launch_tool, &tool_args);

    if ctx.is_inside_ai_tool() && !flags.go && (!tool_args.is_empty() || count > 5) {
        if stdio {
            bail!("Native stdio launch requires --go inside an AI tool; use agentx launch --tool codex --stdio --go -- <codex arguments>");
        }
        print_launch_preview(LaunchPreview {
            action: "launch",
            tool: &tool,
            count,
            background: preview_background,
            args: &tool_args,
            cwd: dir_override.as_deref(),
            terminal: terminal.as_deref(),
            config: &agentx_config,
            show_config_args: false,
            notes: &[],
            json: json_output,
        });
        return Ok(0);
    }

    // System/initial prompt handling
    let system_prompt = agentx_flags.system_prompt;
    let initial_prompt = agentx_flags.initial_prompt;

    // Merge env config args with CLI args
    let (merged_args, background) = launcher::prepare_resolved_execution(&launch_tool, &tool_args, headless);

    let session = session.expect("non-preview launch acquired a session");
    let launcher_name = launcher_name_client(&session);
    let launcher_name_ref = launcher_name.as_str();

    let output = LaunchOutputContext {
        action: "launch",
        tool: &tool,
        requested_count: count,
        launcher_name: launcher_name_ref,
        terminal: terminal_for_output.as_deref(),
        remote_terminal: None,
        background,
        run_here: agentx_flags.run_here,
        agentx_config: &agentx_config,
        inline_readiness_wait_secs: if ctx.is_inside_ai_tool() && count == 1 { Some(INLINE_SINGLE_LAUNCH_WAIT_SECS) } else { None },
        json: json_output,
    };

    let result = launcher::launch_with_client(
        LaunchParams {
            labels,
            bindings: binding_paths,
            tool: tool.clone(),
            count,
            args: merged_args,
            persisted_args: Some(resolved.args.clone()),
            launch_profile: resolved.profile,
            collaboration: Some(collaboration),
            prior_session_id: None,
            resume_cursor: None,
            title: agentx_flags.title,
            system_prompt,
            initial_prompt,
            background,
            stdio,
            cwd: Some(launch_working_dir(dir_override.as_deref())?),
            env: None,
            launcher: Some(launcher_name.clone()),
            run_here: agentx_flags.run_here,
            batch_id: agentx_flags.batch_id,
            name: None, // --iam is caller identity, not instance name
            skip_validation: false,
            terminal,
            append_reply_handoff: true,
        },
        &ctx,
        &session.client,
    )?;

    if stdio {
        return result.stdio_exit_code.ok_or_else(|| anyhow::anyhow!("Codex stdio launch returned no child exit status"));
    }
    let readiness_state = report_launch(&session.client, &result, &output)?;

    // Log summary
    log_info("launch", "cmd.launch", &format!("tool={} count={} launched={} failed={} batch={}", tool, count, result.launched, result.failed, result.batch_id));

    Ok(readiness_exit_code(readiness_state, result.failed))
}

fn launch_working_dir(dir: Option<&str>) -> Result<String> {
    if let Some(dir) = dir {
        let path = std::path::Path::new(dir);
        if !path.is_dir() {
            bail!("--dir path does not exist or is not a directory: {}", dir);
        }
        Ok(path.canonicalize().map(|p| crate::shared::platform::child_process_path(&p)).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| dir.to_string()))
    } else {
        Ok(std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| ".".to_string()))
    }
}

fn print_profile_preview(value: &serde_json::Value, json: bool) {
    if json {
        println!("{value}");
    } else {
        let config = &value["configuration"];
        println!(
            "[Preview] No agent started\nTool: {}\nSelected by: {}\nProfile: {}\nBase args: {}\nOverrides: {}\nResolved args: {}\nDirectory: {}",
            config["tool"].as_str().unwrap_or("unknown"),
            config["selected_by"].as_str().unwrap_or("unknown"),
            config["profile"]["name"].as_str().unwrap_or("(none)"),
            config["base_args"],
            config["overrides"],
            config["args"],
            value["cwd"]
        );
        if let Some(title) = value.get("title").and_then(serde_json::Value::as_str) {
            println!("Title: {title}");
        }
        if let Some(collaboration) = value.get("collaboration") {
            println!("Workspace: {}\nRole: {}", collaboration["workspace"].as_str().unwrap_or("global"), collaboration["role"]["name"].as_str().unwrap_or("(none)"));
        }
        println!("Provider protocol adjustments and process facts are resolved during execution. Add --go and omit --dry-run to execute.");
    }
}

#[allow(clippy::too_many_arguments)]
fn run_remote(
    tool: Option<String>,
    mut selection: crate::runtime::launch_profile::LaunchSelection,
    args: Vec<String>,
    count: usize,
    launch_flags: AgentxLaunchFlags,
    dry_run: bool,
    invocation: crate::service::InvocationContext,
    flags: &GlobalFlags,
) -> Result<i32> {
    if launch_flags.role.is_some() || launch_flags.workspace.is_some() {
        bail!("Remote launch does not support --role or --workspace; resolve and launch on the target device");
    }
    if launch_flags.stdio {
        bail!("--stdio only supports local launches");
    }
    if launch_flags.run_here == Some(true) {
        bail!("Remote launch does not support --run-here");
    }
    let device = launch_flags.device.as_deref().unwrap();
    let cwd = launch_flags.dir.as_deref().ok_or_else(|| anyhow::anyhow!("Remote launch requires --dir on the target device"))?;
    if tool.is_none() && selection.profile.is_none() {
        selection.profile = invocation.environment.get("AGENTX_PROFILE").filter(|value| !value.is_empty()).cloned();
    }
    let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
    let config = AgentxConfig::load_for_scope(&scope, &invocation.environment);
    let context = AgentxContext::from_env(&invocation.environment, invocation.cwd.clone());
    let preview = dry_run || (context.is_inside_ai_tool() && !flags.go);
    let session = crate::cli::client::Session::for_launch(invocation, flags.go)?;
    let launcher_name = launcher_name_client(&session);
    let params = json!({"tool": tool, "selection": selection, "count": count, "args": args,
        "launcher": launcher_name, "background": launch_flags.headless,
        "terminal": launch_flags.terminal, "cwd": cwd,
        "initial_prompt": launch_flags.initial_prompt, "system_prompt": launch_flags.system_prompt});
    let action = if preview { crate::relay::control::rpc_action::LAUNCH_PREVIEW } else { crate::relay::control::rpc_action::LAUNCH_PROFILE };
    let inner = crate::relay::control::dispatch_remote(&session.client, device, None, action, &params, crate::relay::control::RPC_LAUNCH_TIMEOUT).map_err(|error| anyhow::anyhow!("Remote launch failed for device {device}: {error}"))?;
    if preview {
        print_profile_preview(&inner, launch_flags.json);
        return Ok(0);
    }
    let result = launch_result_from_json(&inner).map_err(anyhow::Error::msg)?;
    let output = LaunchOutputContext {
        action: "launch",
        tool: &result.tool,
        requested_count: count,
        launcher_name: &launcher_name,
        terminal: launch_flags.terminal.as_deref(),
        remote_terminal: Some(remote_terminal_preset(&inner, launch_flags.terminal.as_deref())),
        background: result.background,
        run_here: launch_flags.run_here,
        agentx_config: &config,
        inline_readiness_wait_secs: None,
        json: launch_flags.json,
    };
    report_launch(&session.client, &result, &output)?;
    Ok(readiness_exit_code(None, result.failed))
}

fn validate_stdio_launch(count: usize, tool: &LaunchTool, flags: &AgentxLaunchFlags) -> Result<()> {
    if flags.stdio && (tool.tool() != crate::integrations::tool::Tool::Codex || count != 1 || flags.headless || flags.json || flags.device.is_some() || flags.terminal.is_some() || flags.run_here == Some(false)) {
        bail!("--stdio requires one local foreground Codex launch; incompatible with --headless, --json, --device, --terminal or --no-run-here");
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn launch_result_to_json(result: &LaunchResult) -> serde_json::Value {
    serde_json::to_value(result).unwrap_or_else(|_| json!({}))
}

/// Preset a remote device reported using; older devices don't report one.
pub(crate) fn remote_terminal_preset<'a>(inner: &'a serde_json::Value, requested: Option<&'a str>) -> &'a str {
    inner.get("terminal_preset").and_then(|v| v.as_str()).or(requested).unwrap_or("default")
}

pub(crate) fn launch_result_from_json(value: &serde_json::Value) -> Result<LaunchResult, String> {
    serde_json::from_value(value.clone()).map_err(|e| e.to_string())
}

#[cfg(test)]
struct RemoteLaunchOutput {
    tool: String,
    launcher_name: String,
    terminal: Option<String>,
    background: bool,
    run_here: Option<bool>,
}

#[cfg(test)]
fn build_remote_launch_output(db: &AgentxDb, flags: &GlobalFlags, launch_result: &LaunchResult, terminal: Option<String>, run_here: Option<bool>) -> Result<RemoteLaunchOutput> {
    let launcher_name = resolve_launcher_name(db, flags, std::env::var("AGENTX_PROCESS_ID").ok().as_deref())?;
    Ok(RemoteLaunchOutput { tool: launch_result.tool.clone(), launcher_name, terminal, background: launch_result.background, run_here })
}

#[cfg(test)]
pub(crate) fn resolve_launcher_name(db: &AgentxDb, flags: &GlobalFlags, process_id: Option<&str>) -> Result<String> {
    let context = AgentxContext::from_os();
    if crate::integrations::codex::roots::is_shared_context(&context) {
        let store = crate::agents::attachments::SqliteAttachmentStore::new(db);
        let attachment =
            crate::integrations::codex::roots::resolve_shared_attachment(&store, &context, context.codex_thread_id.as_deref().unwrap_or(""))?.ok_or_else(|| anyhow::anyhow!("Shared Codex launch caller has no qualified thread attachment"))?;
        let name = attachment.target.participant.canonical_name;
        if let Some(requested) = &flags.name {
            let requested = requested.clone();
            if requested != name {
                bail!("Shared Codex caller --iam must match its attached participant {name}");
            }
        }
        return Ok(name);
    }
    // Launch caller identity only needs explicit --iam, then process binding.
    Ok(flags.name.as_deref().map(str::to_string).or_else(|| flags.name.clone()).unwrap_or_else(|| identity::resolve_identity(db, None, None, None, process_id, None).map(|id| id.name).unwrap_or_else(|_| "user".to_string())))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

/// Print launch preview when --go gate blocks inside AI tool.
pub(crate) struct LaunchPreview<'a> {
    pub action: &'a str,
    pub tool: &'a str,
    pub count: usize,
    pub background: bool,
    pub args: &'a [String],
    pub cwd: Option<&'a str>,
    pub terminal: Option<&'a str>,
    pub config: &'a AgentxConfig,
    pub show_config_args: bool,
    pub notes: &'a [&'a str],
    pub json: bool,
}

pub(crate) fn print_launch_preview(preview: LaunchPreview<'_>) {
    let mode = if preview.background { "headless" } else { "interactive" };
    let cwd = preview.cwd.map(|s| s.to_string()).unwrap_or_else(|| std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| ".".to_string()));
    // Drive the args-env label from the spec so we never invent a key (e.g.
    // `AGENTX_ANTIGRAVITY_ARGS`) for tools that don't have one.
    let args_key: Option<&'static str> = preview.tool.parse::<crate::integrations::tool::Tool>().ok().and_then(|t| t.spec().launch.args_env);
    let env_args = if preview.show_config_args { preview.tool.parse::<crate::integrations::tool::Tool>().ok().map(|tool| preview.config.tool_args(tool)).unwrap_or("") } else { "" };

    let terminal = preview.terminal.map(str::to_owned).unwrap_or_else(|| if preview.background { preview.config.headless_terminal.clone() } else { preview.config.terminal.clone() });

    if preview.json {
        let notes: Vec<&str> = preview.notes.iter().copied().filter(|n| !n.is_empty()).collect();
        println!(
            "{}",
            json!({
                "preview": true,
                "action": preview.action,
                "tool": preview.tool,
                "count": preview.count,
                "mode": mode,
                "dir": cwd,
                "terminal": terminal,
                "notes": notes,
                "config_args": env_args,
                "config_args_key": args_key,
                "cli_args": preview.args,
                "hint": "Add --go to proceed.",
            })
        );
        return;
    }

    println!("\n== LAUNCH PREVIEW ==");
    println!("Add --go to proceed.\n");
    println!("Action: {}", preview.action);
    println!("Tool: {:<10} Count: {:<4} Mode: {}", preview.tool, preview.count, mode);
    println!("Directory: {}", cwd);
    println!("Terminal: {}", terminal);
    for note in preview.notes {
        println!("{note}");
    }

    // Args — only show if there's something to show
    if !env_args.is_empty() || !preview.args.is_empty() {
        println!("\nArgs:");
        if !env_args.is_empty() {
            match args_key {
                Some(key) => println!("  From config ({}): {}", key, env_args),
                None => println!("  From config: {}", env_args),
            }
        }
        if !preview.args.is_empty() {
            println!("  From CLI: {}", preview.args.join(" "));
        }
        if !env_args.is_empty() && !preview.args.is_empty() {
            println!("  (CLI overrides known singleton defaults; other args pass through in order)");
        }
    }

    println!("\n[Preview Mode] Add --go to proceed with launch.");
}

pub(crate) use crate::runtime::launch_options::{AgentxLaunchFlags, json_run_here};

/// agentx launch flags that take the next token as their value, matching the
/// arms of `extract_launch_flags` (a test keeps the two in step).
const LAUNCH_VALUE_FLAGS: &[&str] = &["--profile", "--tool", "--count", "--terminal", "--device", "--dir", "--agentx-system-prompt", "--agentx-prompt", "--batch-id", "--iam", "--env-file"];

/// Pure pre-configuration input to the native project environment boundary.
pub(crate) struct StdioProjectRequest {
    pub dir: Option<String>,
    pub version: bool,
}

pub(crate) fn stdio_project_request(argv: &[String]) -> Result<Option<StdioProjectRequest>> {
    let (cleaned, _) = crate::runtime::environment::extract_options(argv)?;
    let argv = cleaned.as_slice();
    if !matches!(crate::cli::router::resolve_action(argv), crate::cli::router::Action::Launch { .. }) {
        return Ok(None);
    }
    let (stripped, _, help) = crate::cli::router::extract_global_flags_full(argv);
    if help {
        return Ok(None);
    }
    if stripped.first().is_none_or(|arg| crate::cli::router::canonical_command(arg) != "launch") {
        return Ok(None);
    }
    // Only stdio needs early native parsing. Cloud terminal launch has its own
    // grammar after environment selection; literal --stdio values stay opaque.
    let mut options = stripped.iter().skip(1).map(String::as_str).take_while(|arg| *arg != "--");
    let mut stdio = false;
    while let Some(arg) = options.next() {
        if crate::runtime::environment::takes_opaque_value(arg) {
            options.next();
        } else if arg == "--stdio" {
            stdio = true;
            break;
        }
    }
    if !stdio {
        return Ok(None);
    }
    let parsed = parse_arguments(argv)?;
    if parsed.dry_run {
        return Ok(None);
    }
    let count = parsed.count as usize;
    let (flags, args) = parsed.options.into_parts();
    if !flags.stdio {
        return Ok(None);
    }
    if flags.role.as_deref() == Some("") || parsed.selection.profile.as_deref() == Some("") || parsed.tool.as_deref() == Some("") {
        bail!("Interactive selection is unavailable with --stdio; pass explicit values for --role, --profile and --tool");
    }
    // --stdio identifies the Codex project-preparation surface. A profile's
    // actual tool is checked after direnv/dotenv select the effective scope;
    // never resolve a profile against the pre-environment scope here.
    let tool = parsed.tool.unwrap_or_else(|| "codex".into());
    validate_stdio_launch(count, &LaunchTool::from_str(&tool)?, &flags)?;
    Ok(Some(StdioProjectRequest { dir: flags.dir, version: args.len() == 1 && matches!(args[0].as_str(), "--version" | "-V") }))
}

/// Pin the agentx directory without changing any opaque native argument.
pub(crate) fn with_stdio_project_dir(argv: Vec<String>, project: &std::path::Path) -> Vec<String> {
    let mut out = Vec::with_capacity(argv.len() + 2);
    let mut i = 0;
    while i < argv.len() && argv[i] != "--" {
        let arg = argv[i].as_str();
        if arg == "--dir" {
            i += 2;
            continue;
        }
        if arg.starts_with("--dir=") {
            i += 1;
            continue;
        }
        out.push(argv[i].clone());
        i += 1;
        if LAUNCH_VALUE_FLAGS.contains(&arg) && i < argv.len() {
            out.push(argv[i].clone());
            i += 1;
        }
    }
    out.extend(["--dir".to_string(), project.to_string_lossy().into_owned()]);
    out.extend_from_slice(&argv[i..]);
    out
}

/// `args` minus the `--json` tokens that `extract_launch_flags` reads as the
/// flag. A value that happens to be `--json` (`--agentx-prompt --json`) and
/// anything after `--` are kept.
pub(crate) fn without_json_flag(args: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--" {
            out.extend_from_slice(&args[i..]);
            break;
        }
        if LAUNCH_VALUE_FLAGS.contains(&arg) && i + 1 < args.len() {
            out.extend_from_slice(&args[i..i + 2]);
            i += 2;
            continue;
        }
        if arg != "--json" {
            out.push(args[i].clone());
        }
        i += 1;
    }
    out
}

/// Canonical fresh-launch grammar; native arguments have an explicit boundary.
#[derive(clap::Parser)]
#[command(
    name = "agentx launch",
    about = "Launch one or more agents",
    after_help = r#"Routine task delegation:
  agentx launch --go --headless --title "Task title" --bind ".controller/xx/xx.task.md" --agentx-prompt "Assignment"
Run directly with configured defaults; if launch fails, follow its diagnostic to recover.
Add --role NAME when needed. Role discovery/setup: agentx roles / agentx roles --help.
Omit the value of --role, --profile or --tool to select in a local terminal.
Use --count N for multiple agents, --dir PATH for cwd, or --workspace PATH|global for membership.
Workspace defaults: explicit --workspace > --dir project > AGENTX_WORKSPACE > cwd project > ~/.agentx.
Project discovery prefers the nearest Git/worktree root, then a project .agentx/ boundary.
Without --dir, cwd stays at the caller's current directory; workspace does not move it.
Put all native tool arguments after --.
Each launch creates a new member; agentx send --dir PATH reuses or starts its default recipient.
Role/workspace launches are local; shared Codex app-server stdio is unsupported."#
)]
struct LaunchArguments {
    /// CLI tool to launch; omit its value to select in a local terminal.
    #[arg(long, num_args = 0..=1, default_missing_value = "", value_parser = parse_tool)]
    tool: Option<String>,
    #[command(flatten)]
    selection: crate::runtime::launch_profile::LaunchSelection,
    /// Inspect the resolved configuration without creating an agent.
    #[arg(long)]
    dry_run: bool,
    /// Number of agents to launch; must be positive.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    count: u32,
    #[command(flatten)]
    options: crate::runtime::launch_options::LaunchOptions,
}
fn parse_tool(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }
    let tool: crate::integrations::tool::Tool = value.parse().map_err(|_| format!("Unknown tool '{value}'"))?;
    if !tool.spec().released {
        return Err(format!("Unsupported tool '{value}'"));
    }
    Ok(tool.as_str().to_owned())
}
pub(crate) fn help_text() -> String {
    let tools = crate::integrations::registry::ALL.iter().filter(|spec| spec.released).flat_map(|spec| std::iter::once(spec.name).chain(spec.aliases.iter().copied())).collect::<Vec<_>>().join(", ");
    format!("{}\nEnvironment options (before --):\n  --env-file PATH (repeatable)\n  --env-override / --no-env-override\n\nSupported tools: {tools}\n", launch_command().render_long_help())
}

/// Only fresh launch offers choices; shared resume/fork flags remain explicit.
fn launch_command() -> clap::Command {
    use clap::CommandFactory;
    LaunchArguments::command()
        .mut_arg("role", |arg| arg.num_args(0..=1).default_missing_value("").help("Adopt a role; omit its value to select in a local terminal"))
        .mut_arg("profile", |arg| arg.num_args(0..=1).default_missing_value("").help("Saved launch profile; omit its value to select in a local terminal"))
}

fn parse_arguments(argv: &[String]) -> Result<LaunchArguments> {
    use clap::FromArgMatches;
    let argv = normalize_selection_args(argv);
    let (cleaned, _) = crate::runtime::environment::extract_options(&argv)?;
    let (cleaned, _, _) = crate::cli::router::extract_global_flags_full(&cleaned);
    Ok(LaunchArguments::from_arg_matches(&launch_command().try_get_matches_from(cleaned)?)?)
}

pub(crate) fn is_dry_run(argv: &[String]) -> bool {
    let (words, _, _) = crate::cli::router::extract_global_flags_full(argv);
    words.first().is_some_and(|word| crate::cli::router::canonical_command(word) == "launch") && parse_arguments(argv).is_ok_and(|args| args.dry_run)
}

#[cfg(test)]
mod selection_tests {
    use super::*;

    fn words(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn selection_parser_distinguishes_omitted_bare_and_explicit_values() {
        let omitted = parse_arguments(&words(&["launch"])).unwrap();
        assert!(omitted.tool.is_none());
        assert!(omitted.selection.profile.is_none());
        assert!(omitted.options.flags.role.is_none());
        let selected = parse_arguments(&words(&["l", "--role", "--profile", "--tool", "--dry-run", "--", "--role", "provider"])).unwrap();
        assert_eq!(selected.tool.as_deref(), Some(""));
        assert_eq!(selected.selection.profile.as_deref(), Some(""));
        assert_eq!(selected.options.flags.role.as_deref(), Some(""));
        assert!(selected.dry_run);
        assert_eq!(selected.options.native_args, words(&["--role", "provider"]));
        let explicit = parse_arguments(&words(&["launch", "--role", "coder", "--profile", "fast", "--tool", "codex"])).unwrap();
        assert_eq!(explicit.tool.as_deref(), Some("codex"));
        assert_eq!(explicit.selection.profile.as_deref(), Some("fast"));
        assert_eq!(explicit.options.flags.role.as_deref(), Some("coder"));
        assert!(parse_arguments(&words(&["launch", "--tool", "does-not-exist"])).is_err());
    }

    #[test]
    fn selection_preserves_help_environment_and_stdio_boundaries() {
        let argv = normalize_selection_args(&words(&["launch", "--role", "--help"]));
        assert!(crate::cli::router::extract_global_flags_full(&argv).2);
        assert!(crate::runtime::environment::is_lightweight(&argv));
        let argv = words(&["launch", "--tool", "--env-file", ".env", "--dry-run"]);
        assert!(parse_arguments(&argv).unwrap().dry_run);
        // Match main: normalize bare launch selectors before the stdio scan.
        let argv = normalize_selection_args(&words(&["launch", "--tool", "--stdio"]));
        assert!(stdio_project_request(&argv).err().expect("bare tool selection must fail for stdio").to_string().contains("explicit values"));
        let argv = normalize_selection_args(&words(&["launch", "--name", "cloud-agent", "--agentx-prompt", "--stdio", "--", "--stdio"]));
        assert!(stdio_project_request(&argv).unwrap().is_none());
        assert!(crate::runtime::launch_options::extract_launch_flags(&words(&["--role"])).is_err(), "resume/fork still require role values");
        let help = help_text();
        assert!(help.contains("--role [<ROLE>]"));
        assert!(help.contains("--profile [<PROFILE>]"));
    }
}

#[cfg(test)]
fn parse_launch_argv(argv: &[String]) -> Result<(usize, String, AgentxLaunchFlags, Vec<String>)> {
    let parsed = parse_arguments(argv)?;
    let (flags, native) = parsed.options.into_parts();
    Ok((parsed.count as usize, parsed.tool.ok_or_else(|| anyhow::anyhow!("--tool required in this parser helper"))?, flags, native))
}

/// Inputs shared by launch, resume and fork result rendering.
pub(crate) struct LaunchOutputContext<'a> {
    pub action: &'a str,
    pub tool: &'a str,
    pub requested_count: usize,
    pub launcher_name: &'a str,
    pub terminal: Option<&'a str>,
    /// Remote launches: the preset the target device used. Resolving
    /// `terminal` here would describe this device, not the target.
    pub remote_terminal: Option<&'a str>,
    pub background: bool,
    pub run_here: Option<bool>,
    pub agentx_config: &'a AgentxConfig,
    pub inline_readiness_wait_secs: Option<u64>,
    pub json: bool,
}

/// Report a launch, resume, or fork: human feedback followed by the optional
/// inline readiness wait, or with `--json` a single object on stdout. Returns
/// the readiness outcome when a wait ran.
pub(crate) fn report_launch(client: &Client, result: &LaunchResult, ctx: &LaunchOutputContext<'_>) -> Result<Option<InlineLaunchReadiness>> {
    let wait_secs = ctx.inline_readiness_wait_secs.filter(|_| result.launched == 1);
    if !ctx.json {
        print_launch_feedback(client, result, ctx)?;
        let readiness = wait_secs.map(|secs| print_inline_launch_readiness(client, result, secs));
        if ctx.remote_terminal.is_none()
            && let Some(participant) = &result.attach_participant
        {
            let name = &participant.canonical_name;
            println!("Attaching to {name}; detach with Ctrl+B then D. Reconnect: agentx attach {name}");
            let status = crate::runtime::terminal_client::attach_after_launch(client, participant).map_err(|error| anyhow::anyhow!("Agent {name} was started, but attachment failed: {error}. Reconnect with: agentx attach {name}"))?;
            if status != 0 {
                bail!("tmux attachment exited with status {status}; reconnect with: agentx attach {name}");
            }
        }
        return Ok(readiness);
    }

    // Errors also go to stderr so a script that only parses stdout still
    // surfaces them to whoever is watching.
    let errors = launch_error_messages(result);
    for msg in &errors {
        eprintln!("Error: {}", msg);
    }
    let wait = wait_secs.map(|secs| launch_status::wait_for_launch_client(client, None, Some(&result.batch_id), secs));
    println!(
        "{}",
        json!({
            "action": ctx.action,
            "tool": ctx.tool,
            "requested": ctx.requested_count,
            "launched": result.launched,
            "failed": result.failed,
            "names": instance_names_from_launch_result(result),
            "batch_id": result.batch_id,
            "background": ctx.background,
            "log_files": result.log_files,
            "errors": errors,
            // Same shape as `agentx events launch`; null when no wait ran.
            "readiness": wait.as_ref().map(|w| w.to_json()),
        })
    );
    Ok(wait.map(|w| inline_readiness_from_status(&w.status)))
}

fn launch_error_messages(result: &LaunchResult) -> Vec<&str> {
    result.errors.iter().filter_map(|err| err.get("error").and_then(|v| v.as_str())).collect()
}

fn print_launch_feedback(client: &Client, result: &LaunchResult, ctx: &LaunchOutputContext<'_>) -> Result<()> {
    if result.failed > 0 {
        for msg in launch_error_messages(result) {
            eprintln!("Error: {}", msg);
        }
    }

    if result.launched == 0 && result.failed > 0 {
        return Ok(());
    }

    let tool_label = capitalize(ctx.tool);
    let plural = if ctx.requested_count != 1 { "s" } else { "" };
    if result.failed > 0 {
        println!("Started the {} process for {}/{} {} agent{} ({} failed)", ctx.action, result.launched, ctx.requested_count, tool_label, plural, result.failed);
    } else {
        let s = if result.launched != 1 { "s" } else { "" };
        println!("Started the {} process for {} {} agent{}", ctx.action, result.launched, tool_label, s);
    }

    let instance_names: Vec<&str> = result.handles.iter().filter_map(|h| h.get("instance_name").and_then(|v| v.as_str())).collect();
    if !instance_names.is_empty() {
        println!("Names: {}", instance_names.join(" "));
    }
    println!("Batch id: {}", result.batch_id);
    if ctx.inline_readiness_wait_secs.is_none() {
        println!("To block until ready or fail (30s timeout), run: agentx events launch {}", result.batch_id);
    }

    let launcher_participating = match client.call(Request::Participants(crate::agents::service::Request::Record { name: ctx.launcher_name.into() }))? {
        Response::Participants(crate::agents::service::Response::Record(record)) => record.is_some(),
        _ => bail!("unexpected launch participant response"),
    };
    let (terminal_mode, terminal_auto_detected) = match ctx.remote_terminal {
        Some(preset) => (preset.to_string(), false),
        None => crate::runtime::terminal::resolve_terminal_mode_for_tips(ctx.terminal, &ctx.agentx_config.terminal, ctx.background, ctx.run_here.unwrap_or(false)),
    };
    tips::print_launch_tips_client(client, LaunchTipsContext { launched: result.launched, launcher_name: Some(ctx.launcher_name), launcher_participating, background: ctx.background, terminal_mode: &terminal_mode, terminal_auto_detected })?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InlineLaunchReadiness {
    Ready,
    Failed,
    Blocked,
    Launching,
}

/// Map an inline-readiness outcome to a process exit code, shared by launch
/// and resume/fork so all three report readiness the same way. `None` means
/// no readiness wait ran (not inside an AI tool, or multi-launch).
pub(crate) fn readiness_exit_code(state: Option<InlineLaunchReadiness>, failed: usize) -> i32 {
    match state {
        Some(InlineLaunchReadiness::Failed) => 1,
        Some(InlineLaunchReadiness::Launching) | Some(InlineLaunchReadiness::Blocked) => 2,
        _ if failed == 0 => 0,
        _ => 1,
    }
}

fn inline_readiness_from_status(status: &LaunchStatus) -> InlineLaunchReadiness {
    match status {
        LaunchStatus::Ready => InlineLaunchReadiness::Ready,
        LaunchStatus::Error => InlineLaunchReadiness::Failed,
        LaunchStatus::Blocked => InlineLaunchReadiness::Blocked,
        LaunchStatus::Timeout | LaunchStatus::NoLaunches => InlineLaunchReadiness::Launching,
    }
}

fn print_inline_launch_readiness(client: &Client, result: &LaunchResult, timeout_secs: u64) -> InlineLaunchReadiness {
    println!("Waiting up to {timeout_secs}s for launch readiness...");
    let start = Instant::now();
    let wait = launch_status::wait_for_launch_client(client, None, Some(&result.batch_id), timeout_secs);
    let elapsed_secs = start.elapsed().as_secs_f64();

    let state = inline_readiness_from_status(&wait.status);
    let details = match state {
        InlineLaunchReadiness::Failed => wait.failures,
        InlineLaunchReadiness::Blocked => wait.blockers,
        InlineLaunchReadiness::Ready | InlineLaunchReadiness::Launching => Vec::new(),
    };

    println!("{}", format_inline_launch_readiness(state, result, &wait.instances, elapsed_secs, &details));
    state
}

fn instance_names_from_launch_result(result: &LaunchResult) -> Vec<String> {
    result.handles.iter().filter_map(|h| h.get("instance_name").and_then(|v| v.as_str()).map(ToString::to_string)).collect()
}

fn format_inline_launch_readiness(state: InlineLaunchReadiness, result: &LaunchResult, ready_instances: &[String], elapsed_secs: f64, failures: &[String]) -> String {
    let names = instance_names_from_launch_result(result);
    let target = if names.is_empty() { "agent".to_string() } else { names.join(" ") };
    let progress = format!("{}/{} ready", ready_instances.len(), result.launched);
    let elapsed = format!("{elapsed_secs:.1}s");

    match state {
        InlineLaunchReadiness::Ready => {
            let ready = if ready_instances.is_empty() { target } else { ready_instances.join(" ") };
            format!("Launch ready: {ready} ({progress}, {elapsed}).")
        }
        InlineLaunchReadiness::Failed => {
            let detail = if failures.is_empty() { "no failure detail available".to_string() } else { failures.join("; ") };
            format!("Launch failed: {detail} (batch: {}).", result.batch_id)
        }
        InlineLaunchReadiness::Blocked => {
            let detail = if failures.is_empty() { "human attention needed".to_string() } else { failures.join("; ") };
            format!("Launch blocked: {detail} (batch: {}).", result.batch_id)
        }
        InlineLaunchReadiness::Launching => {
            let list_cmd = match names.as_slice() {
                [name] => format!("agentx list {name}"),
                _ => "agentx list -v".to_string(),
            };
            format!(
                "Still launching after {elapsed}: {target} ({progress}).\n  \
                 agentx events launch {batch}   blocks until ready, failed, or blocked on approval (30s, --timeout N)\n  \
                 {list_cmd}   current status",
                batch = result.batch_id,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|i| i.to_string()).collect()
    }

    #[test]
    fn role_and_workspace_selectors_are_not_native_arguments() {
        let parsed = parse_arguments(&s(&["launch", "--role", "reviewer", "--workspace", "global", "--", "--model", "custom"])).unwrap();
        let (flags, native) = parsed.options.into_parts();
        assert_eq!(flags.role.as_deref(), Some("reviewer"));
        assert_eq!(flags.workspace.as_deref(), Some("global"));
        assert_eq!(native, s(&["--model", "custom"]));
    }

    fn lt(tool: &str) -> LaunchTool {
        LaunchTool::from_str(tool).unwrap()
    }

    #[test]
    fn test_parse_launch_argv_simple() {
        let (count, tool, _flags, args) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1"])).unwrap();
        assert_eq!(count, 1);
        assert_eq!(tool, "claude");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_launch_argv_with_count() {
        let (count, tool, _, args) = parse_launch_argv(&s(&["launch", "--tool", "gemini", "--count", "3", "--", "-m", "flash"])).unwrap();
        assert_eq!(count, 3);
        assert_eq!(tool, "gemini");
        assert_eq!(args, s(&["-m", "flash"]));
    }

    #[test]
    fn test_parse_launch_argv_headless() {
        let (_, _, flags, _) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--headless"])).unwrap();
        assert!(flags.headless);
    }

    #[test]
    fn test_parse_launch_argv_no_run_here() {
        let (_, _, flags, _) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--no-run-here"])).unwrap();
        assert_eq!(flags.run_here, Some(false));
    }

    #[test]
    fn test_parse_launch_argv_with_terminal() {
        let (_, _, flags, _) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--terminal", "kitty-tab"])).unwrap();
        assert_eq!(flags.terminal, Some("kitty-tab".to_string()));
    }

    #[test]
    fn test_parse_launch_argv_skips_global_flags() {
        let (count, tool, _, _) = parse_launch_argv(&s(&["--iam", "bot", "--go", "launch", "--tool", "codex", "--count", "2"])).unwrap();
        assert_eq!(count, 2);
        assert_eq!(tool, "codex");
    }

    #[test]
    fn test_parse_launch_argv_empty_fails() {
        assert!(parse_launch_argv(&[]).is_err());
    }

    #[test]
    fn explicit_args_override_configured_singletons_and_preserve_config_overrides() {
        let mut config = AgentxConfig::default();
        config.set_field("codex_args", "-m old -s read-only -c key=1").unwrap();
        assert_eq!(merge_tool_args(&lt("codex"), &s(&["--model=new", "--sandbox", "workspace-write", "--config", "key=2"]), &config), s(&["-c", "key=1", "--model=new", "--sandbox", "workspace-write", "--config", "key=2"]));
    }

    #[test]
    fn test_primary_tool_args_are_concatenated_verbatim() {
        for (tool, field) in [("claude", "claude_args"), ("gemini", "gemini_args"), ("codex", "codex_args")] {
            let mut config = AgentxConfig::default();
            config.set_field(field, "--future-config value").unwrap();
            let cli = s(&["--future-upstream-flag", "raw-value"]);
            let merged = merge_tool_args(&lt(tool), &cli, &config);
            assert_eq!(merged, s(&["--future-config", "value", "--future-upstream-flag", "raw-value"]));
        }
    }

    #[test]
    fn test_merge_tool_args_applies_config_for_opencode_family_kimi_and_qoder() {
        // These tools previously fell through to the `_` pass-through arm, which
        // silently dropped their `*_args` config at launch.
        let cli = s(&["--yolo"]);
        for (tool, field) in [("opencode", "opencode_args"), ("kilo", "kilo_args"), ("kimi", "kimi_args"), ("qoder", "qoder_args")] {
            let mut config = AgentxConfig::default();
            config.set_field(field, "--model from-config").unwrap();
            let merged = merge_tool_args(&lt(tool), &cli, &config);
            assert_eq!(merged, s(&["--model", "from-config", "--yolo"]), "config args must be merged for {tool}");
        }
    }

    #[test]
    fn test_parse_launch_argv_name_after_tool_args() {
        // --iam before the native separator selects the caller.
        let (count, tool, _, args) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--iam", "nafo", "--", "--model", "haiku"])).unwrap();
        assert_eq!(count, 1);
        assert_eq!(tool, "claude");
        assert_eq!(args, s(&["--model", "haiku"]));
    }

    #[test]
    fn test_parse_launch_argv_go_after_tool_args() {
        // --go after tool args should be stripped
        let (_, _, _, args) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--go", "--", "--model", "haiku"])).unwrap();
        assert_eq!(args, s(&["--model", "haiku"]));
    }

    #[test]
    fn test_parse_launch_argv_agentx_prompt() {
        let (_, _, flags, args) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--agentx-prompt", "do the thing", "--", "--model", "haiku"])).unwrap();
        assert_eq!(flags.initial_prompt, Some("do the thing".to_string()));
        assert_eq!(args, s(&["--model", "haiku"]));
    }

    #[test]
    fn test_parse_launch_argv_agentx_system_prompt() {
        let (_, _, flags, args) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--agentx-system-prompt", "you are helpful", "--", "--model", "haiku"])).unwrap();
        assert_eq!(flags.system_prompt, Some("you are helpful".to_string()));
        assert_eq!(args, s(&["--model", "haiku"]));
    }

    #[test]
    fn launch_rejects_unknown_options_and_invalid_selectors() {
        for args in [
            s(&["launch"]),
            s(&["launch", "--tool", "missing"]),
            s(&["launch", "--tool", "codex", "--count", "0"]),
            s(&["launch", "--tool", "codex", "--count", "-2"]),
            s(&["launch", "--tool", "codex", "--count", "no"]),
            s(&["launch", "--tool", "codex", "--model", "x"]),
            s(&["launch", "--tool", "codex", "app-server"]),
            s(&["launch", "--tool", "codex", "--system", "x"]),
        ] {
            assert!(parse_launch_argv(&args).is_err(), "{args:?}");
        }
    }
    #[test]
    fn launch_defaults_and_native_boundaries_are_exact() {
        let (count, tool, _, args) = parse_launch_argv(&s(&["launch", "--tool", "codex"])).unwrap();
        assert_eq!((count, tool.as_str(), args.len()), (1, "codex", 0));
        let native = s(&["app-server", "--json", "--name", "native", "--go", "--env-file", ".env", "", "two words", "--", "--count", "9"]);
        let mut argv = s(&["launch", "--tool", "codex", "--count", "2", "--json", "--"]);
        argv.extend(native.clone());
        let (count, _, flags, args) = parse_launch_argv(&argv).unwrap();
        assert_eq!(count, 2);
        assert!(flags.json);
        assert_eq!(args, native);
    }

    #[test]
    fn test_parse_launch_argv_batch_id() {
        let (_, _, flags, args) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--batch-id", "batch-123", "--", "--model", "haiku"])).unwrap();
        assert_eq!(flags.batch_id, Some("batch-123".to_string()));
        assert_eq!(args, s(&["--model", "haiku"]));
    }

    #[test]
    fn test_parse_launch_argv_json_is_agentx_flag_unless_after_dashdash() {
        let (_, _, flags, args) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--json", "--", "--model", "haiku"])).unwrap();
        assert!(flags.json);
        assert_eq!(args, s(&["--model", "haiku"]));

        let (_, _, flags, args) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--", "--json"])).unwrap();
        assert!(!flags.json);
        assert_eq!(args, s(&["--json"]));
    }

    #[test]
    fn test_without_json_flag_keeps_values_and_tool_args() {
        assert_eq!(without_json_flag(&s(&["--json", "--terminal", "kitty", "--model", "m", "--", "--json"])), s(&["--terminal", "kitty", "--model", "m", "--", "--json"]));
        // Every value flag swallows a following `--json` as its value, in the
        // parser and in the stripper alike.
        for flag in LAUNCH_VALUE_FLAGS {
            let args = s(&[flag, "--json"]);
            assert_eq!(without_json_flag(&args), args, "{flag}");
        }
    }

    #[test]
    fn test_json_run_here_opens_elsewhere_and_rejects_run_here() {
        let json = AgentxLaunchFlags { json: true, ..Default::default() };
        assert_eq!(json_run_here(&json).unwrap(), Some(false));
        for flags in [AgentxLaunchFlags { run_here: Some(true), ..json.clone() }, AgentxLaunchFlags { terminal: Some("here".to_string()), ..json.clone() }] {
            assert!(json_run_here(&flags).is_err());
        }
        // Without --json, run-here resolution is untouched.
        let plain = AgentxLaunchFlags { run_here: Some(true), ..Default::default() };
        assert_eq!(json_run_here(&plain).unwrap(), Some(true));
        assert_eq!(json_run_here(&AgentxLaunchFlags::default()).unwrap(), None);
    }

    #[test]
    fn test_parse_launch_argv_device() {
        let (_, _, flags, args) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--device", "ABCD", "--", "--model", "haiku"])).unwrap();
        assert_eq!(flags.device, Some("ABCD".to_string()));
        assert_eq!(args, s(&["--model", "haiku"]));
    }

    #[test]
    fn test_prepare_launch_execution_claude_print_adds_background_defaults() {
        // Explicit `-p` opts into print mode → detached print-mode defaults applied.
        let config = AgentxConfig::default();
        let (args, background) = prepare_launch_execution(&lt("claude"), &s(&["-p"]), &config, true);
        assert!(background);

        assert!(args.windows(2).any(|w| w == ["--output-format", "stream-json"]));
        assert!(args.iter().any(|arg| arg == "--verbose"));
    }

    #[test]
    fn test_prepare_launch_execution_headless_no_print_flag_stays_pty() {
        // `agentx launch --tool claude --headless` (no -p) is the live PTY session now — no -p is
        // injected and no print-mode defaults are added.
        let config = AgentxConfig::default();
        let (args, background) = prepare_launch_execution(&lt("claude"), &s(&[]), &config, true);
        assert!(background);
        assert!(!args.iter().any(|arg| matches!(arg.as_str(), "-p" | "--print")));
        assert!(!args.iter().any(|arg| arg == "--output-format"));
    }

    #[test]
    fn test_prepare_launch_execution_headless_positional_prompt_stays_pty() {
        // `agentx launch --tool claude --headless -- "task text"` — positional prompt, no -p → PTY.
        let config = AgentxConfig::default();
        let (args, _background) = prepare_launch_execution(&lt("claude"), &s(&["task text"]), &config, true);
        assert_eq!(args, s(&["task text"]));
    }

    #[test]
    fn test_prepare_launch_execution_headless_only_applies_to_claude() {
        // --headless on other tools must not grow a -p; that flag is Claude-specific.
        let config = AgentxConfig::default();
        let (args, _bg) = prepare_launch_execution(&lt("codex"), &s(&[]), &config, true);
        assert!(!args.iter().any(|t| t == "-p"));
    }

    #[test]
    fn test_prepare_launch_execution_interactive_claude_unchanged() {
        // Foreground `agentx launch --tool claude` (no --headless, no -p) stays untouched.
        let config = AgentxConfig::default();
        let (args, background) = prepare_launch_execution(&lt("claude"), &s(&[]), &config, false);
        assert!(!background);
        assert!(args.is_empty());
    }

    #[test]
    fn test_launch_result_json_roundtrip() {
        let result = LaunchResult {
            tool: "claude".to_string(),
            batch_id: "batch-1".to_string(),
            launched: 1,
            failed: 0,
            background: true,
            log_files: vec!["/tmp/test.log".to_string()],
            handles: vec![serde_json::json!({"instance_name": "luna"})],
            errors: Vec::new(),
            stdio_exit_code: None,
            attach_participant: None,
            launched_participants: Vec::new(),
        };
        let parsed = launch_result_from_json(&launch_result_to_json(&result)).unwrap();
        assert_eq!(parsed.tool, "claude");
        assert_eq!(parsed.batch_id, "batch-1");
        assert_eq!(parsed.launched, 1);
        assert!(parsed.background);
    }

    #[test]
    fn test_format_inline_launch_readiness_ready() {
        let result = LaunchResult {
            tool: "codex".to_string(),
            batch_id: "batch-1".to_string(),
            launched: 1,
            failed: 0,
            background: false,
            log_files: Vec::new(),
            handles: vec![serde_json::json!({"instance_name": "luna"})],
            errors: Vec::new(),
            stdio_exit_code: None,
            attach_participant: None,
            launched_participants: Vec::new(),
        };

        let line = format_inline_launch_readiness(InlineLaunchReadiness::Ready, &result, &["luna".to_string()], 2.2, &[]);

        assert_eq!(line, "Launch ready: luna (1/1 ready, 2.2s).");
    }

    #[test]
    fn test_format_inline_launch_readiness_launching_has_followup_command() {
        let result = LaunchResult {
            tool: "gemini".to_string(),
            batch_id: "batch-2".to_string(),
            launched: 1,
            failed: 0,
            background: false,
            log_files: Vec::new(),
            handles: vec![serde_json::json!({"instance_name": "mari"})],
            errors: Vec::new(),
            stdio_exit_code: None,
            attach_participant: None,
            launched_participants: Vec::new(),
        };

        let line = format_inline_launch_readiness(InlineLaunchReadiness::Launching, &result, &[], 15.0, &[]);

        assert!(line.contains("Still launching after 15.0s: mari (0/1 ready)."));
        assert!(line.contains("agentx events launch batch-2   blocks until ready"));
        assert!(line.contains("agentx list mari   current status"));
    }

    #[test]
    fn test_format_inline_launch_readiness_failed_includes_detail() {
        let result = LaunchResult {
            tool: "claude".to_string(),
            batch_id: "batch-3".to_string(),
            launched: 1,
            failed: 0,
            background: true,
            log_files: Vec::new(),
            handles: vec![serde_json::json!({"instance_name": "nola"})],
            errors: Vec::new(),
            stdio_exit_code: None,
            attach_participant: None,
            launched_participants: Vec::new(),
        };

        let line = format_inline_launch_readiness(InlineLaunchReadiness::Failed, &result, &[], 0.5, &["nola: executable not found".to_string()]);

        assert_eq!(line, "Launch failed: nola: executable not found (batch: batch-3).");
    }

    #[test]
    fn test_build_remote_launch_output_prefers_remote_background() {
        crate::config::Config::init();
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();

        let output = build_remote_launch_output(
            &db,
            &GlobalFlags::default(),
            &LaunchResult {
                tool: "claude".to_string(),
                batch_id: "batch-1".to_string(),
                launched: 1,
                failed: 0,
                background: false,
                log_files: Vec::new(),
                handles: Vec::new(),
                errors: Vec::new(),
                stdio_exit_code: None,
                attach_participant: None,
                launched_participants: Vec::new(),
            },
            Some("kitty".to_string()),
            Some(false),
        )
        .unwrap();

        assert!(!output.launcher_name.is_empty());
        assert_eq!(output.tool, "claude");
        assert_eq!(output.terminal.as_deref(), Some("kitty"));
        assert!(!output.background);
        assert_eq!(output.run_here, Some(false));
    }

    #[test]
    fn test_build_remote_launch_output_uses_remote_launch_result_background() {
        crate::config::Config::init();
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();

        let output = build_remote_launch_output(
            &db,
            &GlobalFlags::default(),
            &LaunchResult {
                tool: "codex".to_string(),
                batch_id: "batch-2".to_string(),
                launched: 1,
                failed: 0,
                background: false,
                log_files: Vec::new(),
                handles: Vec::new(),
                errors: Vec::new(),
                stdio_exit_code: None,
                attach_participant: None,
                launched_participants: Vec::new(),
            },
            None,
            None,
        )
        .unwrap();

        assert!(!output.launcher_name.is_empty());
        assert_eq!(output.tool, "codex");
        assert!(!output.background);
    }

    #[test]
    fn test_is_background_claude_headless() {
        assert!(is_background_from_args(&lt("claude"), &s(&["-p", "fix tests", "--output-format", "json"])));
    }

    #[test]
    fn test_is_background_claude_interactive() {
        assert!(!is_background_from_args(&lt("claude"), &s(&["--model", "haiku"])));
    }

    #[test]
    fn test_resolve_launcher_name_prefers_explicit_name() {
        crate::config::Config::init();
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();
        let flags = GlobalFlags { name: Some("explicit".to_string()), go: false };

        let name = resolve_launcher_name(&db, &flags, Some("pid-123")).unwrap();
        assert_eq!(name, "explicit");
    }

    #[test]
    fn test_resolve_launcher_name_falls_back_to_process_binding() {
        crate::config::Config::init();
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();
        let now = crate::shared::time::now_epoch_f64();
        db.conn()
            .execute(
                "INSERT INTO instances (name, session_id, directory, last_event_id, last_stop, created_at, status, status_time, status_context, tool)
                 VALUES (?1, '', '.', 0, 0, ?2, 'active', ?2, 'test', 'claude')",
                rusqlite::params!["bound", now],
            )
            .unwrap();
        db.set_process_binding("pid-123", "", "bound").unwrap();

        let name = resolve_launcher_name(&db, &GlobalFlags::default(), Some("pid-123")).unwrap();
        assert_eq!(name, "bound");
    }

    #[test]
    fn test_parse_launch_argv_dir_flag() {
        let (_, _, flags, args) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--dir", "/tmp/project", "--", "--model", "haiku"])).unwrap();
        assert_eq!(flags.dir, Some("/tmp/project".to_string()));
        assert_eq!(args, s(&["--model", "haiku"]));
    }

    #[test]
    fn test_parse_launch_argv_dir_equals() {
        let (_, _, flags, args) = parse_launch_argv(&s(&["launch", "--tool", "claude", "--count", "1", "--dir=/tmp/project", "--", "--model", "haiku"])).unwrap();
        assert_eq!(flags.dir, Some("/tmp/project".to_string()));
        assert_eq!(args, s(&["--model", "haiku"]));
    }

    #[test]
    fn test_parse_launch_argv_dir_not_passed_to_tool() {
        let (_, _, flags, args) = parse_launch_argv(&s(&["launch", "--tool", "gemini", "--count", "1", "--dir", "/tmp/proj", "--", "-m", "flash"])).unwrap();
        assert_eq!(flags.dir, Some("/tmp/proj".to_string()));
        assert_eq!(args, s(&["-m", "flash"]));
    }
}
