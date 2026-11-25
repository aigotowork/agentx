//! Resume/fork host preparation and Provider-owned transcript discovery.
//! Durable source selection and launch reservation are native Participants
//! operations; this module never opens the managed AgentX database.
use super::launch_options::{extract_launch_flags, json_run_here};
use crate::agents::attachments::ParticipantRef;
use crate::agents::lifecycle_service::{Request as Lifecycle, Response as LifecycleResult, ResumeSelection, ResumeSnapshot};
use crate::integrations::hooks::codex::derive_codex_transcript_path;
use crate::integrations::hooks::gemini::derive_gemini_transcript_path;
use crate::integrations::hooks::kimi::derive_kimi_transcript_path;
use crate::integrations::tool::Tool;
use crate::log::log_info;
use crate::runtime::launcher::{self, LaunchParams, LaunchResult, LaunchTool};
use crate::service::{Request, Response, client::Client};
use crate::transcript::{claude_projects_dir, detect_tool_from_path, qoder_projects_dir};
use anyhow::{Result, bail};
use std::io::BufRead;

fn lifecycle(client: &Client, input: Lifecycle) -> Result<LifecycleResult> {
    let caller = crate::service::CallerContext { request_id: crate::service::RequestId(uuid::Uuid::new_v4().to_string()), invocation: client.invocation().clone(), deadline_unix_ms: None, expected_database_generation: None };
    let request = Request::Participants(crate::agents::service::Request::Lifecycle(input));
    let result = match client.call_with_context(caller.clone(), request.clone()) {
        Err(crate::service::ServiceError::OutcomeUnknown(_)) => client.call_with_context(caller, request),
        result => result,
    }?;
    match result {
        Response::Participants(crate::agents::service::Response::Lifecycle(result)) => Ok(result),
        _ => bail!("unexpected native resume response"),
    }
}

fn selection(client: &Client, input: Lifecycle) -> Result<ResumeSelection> {
    let LifecycleResult::ResumeSelected(resolved) = lifecycle(client, input)? else {
        bail!("unexpected native resume selection response");
    };
    for effect in resolved.cleanup.effects {
        super::stop_effects::apply(effect);
    }
    Ok(resolved.selection)
}

/// Execute the admitted source using this Client's original generation. The
/// exact source is validated again inside native launch reservation, not by a
/// host name lookup immediately before spawning.
pub(crate) fn resume_fenced(client: &Client, target: &str, expected: Option<&ParticipantRef>, fork: bool, extra_args: &[String], launcher: Option<&str>) -> Result<LaunchResult> {
    let plan = prepare(client, target, expected, fork, extra_args, launcher)?;
    execute(client, &plan)
}

/// Provider discovery uses the current host's files; forbid silently reading a
/// different environment/cwd than the originating invocation. A remote worker
/// carries its own captured host context and must not borrow the daemon's env.
fn host_context(client: &Client) -> Result<crate::runtime::context::AgentxContext> {
    let invocation = client.invocation();
    let actual: std::collections::HashMap<String, String> = std::env::vars().collect();
    if invocation.environment != actual || std::env::current_dir()? != invocation.cwd {
        bail!("resume discovery requires its originating host environment and working directory");
    }
    let mut context = crate::runtime::context::AgentxContext::from_env(&invocation.environment, invocation.cwd.clone()).with_tty(invocation.stdin_is_tty, invocation.stdout_is_tty);
    context.agentx_dir = client.scope_root().to_path_buf();
    Ok(context)
}

pub(crate) fn prepare(client: &Client, target: &str, expected: Option<&ParticipantRef>, fork: bool, extra_args: &[String], launcher: Option<&str>) -> Result<PreparedResume> {
    prepare_inner(client, target, expected, fork, extra_args, launcher, false)
}
pub(crate) fn prepare_preview(client: &Client, target: &str, fork: bool, extra_args: &[String], launcher: Option<&str>) -> Result<PreparedResume> {
    prepare_inner(client, target, None, fork, extra_args, launcher, true)
}
fn prepare_inner(client: &Client, target: &str, expected: Option<&ParticipantRef>, fork: bool, extra_args: &[String], launcher: Option<&str>, preview: bool) -> Result<PreparedResume> {
    let context = host_context(client)?;
    let (dir_override, flags, clean_extra) = extract_resume_flags(extra_args)?;
    if flags.stdio {
        bail!("--stdio is supported only by a fresh agentx launch --tool codex invocation; pass native resume arguments after --");
    }
    let run_here = json_run_here(&flags)?;
    let selected = selection(client, if preview { Lifecycle::PreviewResume { target: target.into(), fork } } else { Lifecycle::ResolveResume { target: target.into(), expected: expected.cloned(), fork } })?;
    let selected = if let ResumeSelection::Missing { diagnostic } = &selected {
        if expected.is_some() || crate::relay::control::split_device_suffix(target).is_some() {
            bail!("{diagnostic}");
        }
        if let Some(session) = resolve_thread_name(target)? {
            selection(client, Lifecycle::ResolveResume { target: session, expected: None, fork })?
        } else {
            bail!("{diagnostic}");
        }
    } else {
        selected
    };
    let (display_name, saved, source, preview_child_name, is_adoption) = match selected {
        ResumeSelection::Tracked { name, snapshot, fence, preview_child_name } => (name, snapshot, fence, preview_child_name, false),
        ResumeSelection::Disk { session_id, fence } => {
            let (tool, cwd) = adopt_snapshot(&session_id, fork)?;
            (
                session_id.clone(),
                ResumeSnapshot {
                    tool: tool.as_str().into(),
                    session_id,
                    launch_args: String::new(),
                    launch_profile: None,
                    collaboration: None,
                    title: None,
                    background: false,
                    last_event_id: 0,
                    directory: cwd.unwrap_or_default(),
                    background_log_file: String::new(),
                },
                fence,
                None,
                true,
            )
        }
        ResumeSelection::Missing { diagnostic } => bail!("{diagnostic}"),
    };
    if saved.session_id.is_empty() {
        let mut message = format!("No session ID recorded for '{display_name}' — cannot {}", if fork { "fork" } else { "resume" });
        if let Some(tail) = (!saved.background_log_file.is_empty()).then(|| crate::agents::lifecycle::read_launch_log_tail(&saved.background_log_file)).flatten() {
            message.push_str("\nSaved PTY output:\n");
            message.push_str(&tail);
        }
        bail!("{message}");
    }
    // Legacy and adopted sessions are global; edited caller environment cannot reassign history.
    let mut collaboration = saved.collaboration.clone().unwrap_or(crate::agents::workspaces::LaunchCollaboration { workspace: None, role: None, endpoint: false, instruction_additions: None });
    if fork {
        collaboration.endpoint = false;
    }
    let tool_name = resolve_adhoc_resume_tool(&display_name, saved.tool, &saved.session_id, fork)?;
    let launch_tool = validate_resume_operation(&tool_name, fork)?;
    let tool = launch_tool.tool();
    let effective_cwd = if let Some(dir) = dir_override {
        let path = std::path::Path::new(&dir);
        if !path.is_dir() {
            bail!("--dir path does not exist or is not a directory: {dir}");
        }
        path.canonicalize().map(|path| crate::shared::platform::child_process_path(&path).to_string_lossy().into_owned()).unwrap_or(dir)
    } else if fork && !is_adoption {
        context.cwd.to_string_lossy().into_owned()
    } else if !saved.directory.is_empty() && std::path::Path::new(&saved.directory).is_dir() {
        saved.directory.clone()
    } else {
        if !saved.directory.is_empty() {
            eprintln!("Warning: original directory '{}' no longer exists, using current directory", saved.directory);
        }
        context.cwd.to_string_lossy().into_owned()
    };
    let original = serde_json::from_str::<Vec<String>>(&saved.launch_args).unwrap_or_default();
    let session_args = build_resume_args(tool, &saved.session_id, fork);
    let (persisted_args, mut args) = compose_resume_args(tool, &original, &session_args, &clean_extra);
    let background = flags.headless || launcher::is_background_from_args(&launch_tool, &args) || saved.background;
    if tool == Tool::Claude && background && args.iter().any(|arg| matches!(arg.as_str(), "-p" | "--print")) {
        args.extend(["--output-format".into(), "stream-json".into(), "--verbose".into()]);
    }
    let launcher = match launcher {
        Some(value) => value.to_string(),
        None => match client.call(Request::Participants(crate::agents::service::Request::Resolve)) {
            Ok(Response::Participants(crate::agents::service::Response::Actor(actor))) => actor.participant.canonical_name,
            Err(crate::service::ServiceError::IdentityRequired(_)) => "user".into(),
            Err(error) => return Err(error.into()),
            _ => bail!("unexpected resume launcher response"),
        },
    };
    let (system_prompt, initial_prompt, append_reply_handoff) = build_resume_prompts(ResumePromptInput {
        tool,
        display_name: &display_name,
        fork,
        is_adoption,
        child_name: preview_child_name.as_deref(),
        custom_system_prompt: flags.system_prompt.as_deref(),
        custom_initial_prompt: flags.initial_prompt.as_deref(),
    });
    let tracked_fork_identity = if fork && !is_adoption { Some(TrackedForkIdentity { parent_name: display_name.clone(), custom_initial_prompt: flags.initial_prompt.clone(), custom_system_prompt: flags.system_prompt.clone() }) } else { None };
    Ok(PreparedResume {
        tool,
        session_id: saved.session_id.clone(),
        source: Some(source),
        tracked_fork_identity,
        output: ResumeOutputContext { action: if fork { "fork" } else { "resume" }.into(), tool: launch_tool.as_str().into(), terminal: flags.terminal.clone(), background, run_here, launcher_name: launcher.clone(), json: flags.json },
        launch: LaunchParams {
            labels: Default::default(),
            bindings: Vec::new(),
            tool: launch_tool.as_str().into(),
            count: 1,
            args,
            persisted_args: Some(persisted_args),
            launch_profile: saved.launch_profile,
            collaboration: Some(collaboration),
            title: flags.title.clone().or(saved.title),
            prior_session_id: (!fork).then_some(saved.session_id),
            resume_cursor: (!fork && !is_adoption).then_some(saved.last_event_id),
            system_prompt,
            initial_prompt,
            background,
            stdio: false,
            cwd: Some(effective_cwd),
            env: None,
            launcher: Some(launcher),
            run_here,
            batch_id: flags.batch_id,
            name: if is_adoption {
                None
            } else if fork {
                preview_child_name
            } else {
                Some(display_name)
            },
            skip_validation: false,
            terminal: flags.terminal,
            append_reply_handoff,
        },
    })
}

pub(crate) fn execute(client: &Client, plan: &PreparedResume) -> Result<LaunchResult> {
    let context = host_context(client)?;
    let mut source = plan.source.clone().ok_or_else(|| anyhow::anyhow!("native resume source required"))?;
    let mut launch = plan.launch.clone();
    if let Some(identity) = &plan.tracked_fork_identity {
        source = match lifecycle(client, Lifecycle::ReserveResumeFork { source })? {
            LifecycleResult::ResumeFork(source) => source,
            _ => bail!("unexpected native fork reservation response"),
        };
        let child = source.child.as_ref().ok_or_else(|| anyhow::anyhow!("native fork reservation has no child"))?;
        let (system, initial, handoff) = build_resume_prompts(ResumePromptInput {
            tool: plan.tool,
            display_name: &identity.parent_name,
            fork: true,
            is_adoption: false,
            child_name: Some(&child.canonical_name),
            custom_system_prompt: identity.custom_system_prompt.as_deref(),
            custom_initial_prompt: identity.custom_initial_prompt.as_deref(),
        });
        launch.name = Some(child.canonical_name.clone());
        launch.system_prompt = system;
        launch.initial_prompt = initial;
        launch.append_reply_handoff = handoff;
    }
    launcher::launch_resuming_with_client(launch, &context, client, source)
}

pub(crate) struct ResumePromptInput<'a> {
    pub tool: Tool,
    pub display_name: &'a str,
    pub fork: bool,
    pub is_adoption: bool,
    pub child_name: Option<&'a str>,
    pub custom_system_prompt: Option<&'a str>,
    pub custom_initial_prompt: Option<&'a str>,
}

pub(crate) fn should_preview_resume_rpc(extra_args: &[String]) -> bool {
    let Ok((_, launch_flags, clean_extra)) = extract_resume_flags(extra_args) else {
        return false;
    };
    should_preview_resume(&launch_flags, &clean_extra)
}

/// Extract resume-only flags, then reuse the shared launch flag parser.
pub(crate) fn extract_resume_flags(args: &[String]) -> Result<(Option<String>, super::launch_options::AgentxLaunchFlags, Vec<String>)> {
    let (flags, remaining) = extract_launch_flags(args)?;
    if !flags.labels.is_empty() {
        bail!("--label only sets labels on fresh launches; use agentx labels set after resume or fork");
    }
    if !flags.bindings.is_empty() {
        anyhow::bail!("--bind supports fresh local launch only; resume/fork are unsupported");
    }
    if flags.role.is_some() || flags.workspace.is_some() {
        bail!("resume/fork preserve the adopted role and workspace; use launch for a new role or workspace");
    }
    Ok((flags.dir.clone(), flags, remaining))
}

pub(crate) fn should_preview_resume(launch_flags: &super::launch_options::AgentxLaunchFlags, tool_args: &[String]) -> bool {
    // `--json` only changes how the outcome is reported, not what launches.
    let launch_flags = super::launch_options::AgentxLaunchFlags { json: false, ..launch_flags.clone() };
    !tool_args.is_empty() || launch_flags != super::launch_options::AgentxLaunchFlags::default()
}

/// A direct Claude/Codex run that joined with `agentx start` is stored as
/// `adhoc` but keeps its native session id. Resolve the owning tool from the
/// transcript on disk so resume/fork relaunches that session under agentx (with
/// hooks) under the same name.
pub(crate) fn resolve_adhoc_resume_tool(name: &str, tool: String, sid: &str, fork: bool) -> Result<String> {
    if tool != "adhoc" {
        return Ok(tool);
    }
    let op = if fork { "fork" } else { "resume" };
    if sid.is_empty() {
        bail!("'{name}' joined ad-hoc with no native session, so there is nothing to {op}");
    }
    match find_session_on_disk(sid) {
        Some((tool, _)) => Ok(tool.as_str().to_string()),
        None => bail!("'{name}' joined ad-hoc from session {sid}, but no transcript for it was found, so agentx cannot tell which tool to {op}"),
    }
}

/// Parse the saved tool name (canonical, alias, or `claude-pty`) and check it
/// supports the operation.
pub(crate) fn validate_resume_operation(tool: &str, fork: bool) -> Result<LaunchTool> {
    let launch_tool = LaunchTool::from_str(tool).map_err(|_| anyhow::anyhow!("Unknown tool '{}' in saved session metadata", tool))?;
    // Drive fork support from the spec so help text and validation can't drift.
    let spec = launch_tool.spec();
    if fork && spec.resume.and_then(|r| r.fork).is_none() {
        bail!("{} does not support session forking (agentx f)", spec.label);
    }
    Ok(launch_tool)
}

pub(crate) fn build_resume_prompts(input: ResumePromptInput<'_>) -> (Option<String>, Option<String>, bool) {
    let ResumePromptInput { tool, display_name, fork, is_adoption, child_name, custom_system_prompt, custom_initial_prompt } = input;

    // Codex and Qoder fork identity resets belong in the initial prompt.
    // Qoder must preserve its saved system prompt across resume/fork.
    // Adoption-fork has no prior agentx identity, so normal bootstrap handles it.
    let initial_prompt = if fork && matches!(tool, Tool::Codex | Tool::Qoder) && !is_adoption {
        let child_name = child_name.expect("tracked fork child name should be available");
        let child_display = child_name.to_string();
        let identity_reset = format!(
            "You are a fork of {display_name}, but your new agentx identity is now {child_display}.\n\
             Your agentx name is {child_name}.\n\
             Do not use {display_name}'s agentx identity anymore, even if it appears in inherited thread history.\n\
             Agentx commands resolve your new identity from the injected process/session context.\n\
             If asked about your identity, answer exactly: {child_display}"
        );
        Some(match custom_initial_prompt {
            Some(user_prompt) if !user_prompt.trim().is_empty() => {
                format!("{identity_reset}\n\n{user_prompt}")
            }
            _ => identity_reset,
        })
    } else if tool == Tool::Qoder && !is_adoption {
        let identity = resume_system_prompt(tool, display_name, false, None);
        Some(match custom_initial_prompt {
            Some(prompt) => format!("{identity}\n\n{prompt}"),
            None => identity,
        })
    } else {
        custom_initial_prompt.map(ToString::to_string)
    };

    // System prompt:
    // - Tracked-instance resume/fork: identity-carrying prompt (existing behavior).
    // - Adoption: None — SessionStart issues the normal fresh-launch bootstrap.
    let base_system_prompt = if is_adoption || tool == Tool::Qoder { None } else { Some(resume_system_prompt(tool, display_name, fork, child_name)) };
    let system_prompt = match (base_system_prompt, custom_system_prompt) {
        (Some(base), Some(custom)) if !custom.trim().is_empty() => Some(format!("{base}\n\n{custom}")),
        (Some(base), _) => Some(base),
        (None, Some(custom)) if !custom.trim().is_empty() => Some(custom.to_string()),
        (None, _) => None,
    };

    // Codex tracked-instance fork uses initial_prompt for an identity reset;
    // don't dilute it with a reply-handoff suffix. Adoption-fork has no reset.
    let append_reply_handoff = !(fork && matches!(tool, Tool::Codex | Tool::Qoder) && !is_adoption);
    (system_prompt, initial_prompt, append_reply_handoff)
}

pub(crate) fn resume_system_prompt(tool: Tool, name: &str, fork: bool, child_name: Option<&str>) -> String {
    if fork {
        if tool == Tool::Codex {
            format!(
                "YOU ARE A FORK of agent '{}'. \
                 You have the same session history but are a NEW agent with an already-assigned agentx identity. \
                 Use that assigned identity for all agentx commands.",
                name
            )
        } else {
            // Claude gets its identity from the SessionStart hook bootstrap.
            // State the new name explicitly so it overrides the inherited history.
            match child_name {
                Some(child) => format!(
                    "YOU ARE A FORK of agent '{name}'. \
                     You have the same session history but are a NEW agent. \
                     Your new agentx identity is '{child}'. \
                     Agentx commands resolve your new identity from the injected process/session context. \
                     Do NOT use '{name}'s identity, even if it appears in the inherited history.",
                ),
                None => format!(
                    "YOU ARE A FORK of agent '{name}'. \
                     You have the same session history but are a NEW agent with an already-assigned agentx identity. \
                     Use that assigned identity for all agentx commands.",
                ),
            }
        }
    } else {
        format!("YOUR SESSION HAS BEEN RESUMED! You are still '{}'.", name)
    }
}

/// Build tool-specific resume/fork args from the integration spec.
pub(crate) fn build_resume_args(tool: Tool, session_id: &str, fork: bool) -> Vec<String> {
    use crate::integrations::registry::{ForkArgs, ResumeArgs};
    let Some(resume_spec) = tool.spec().resume.as_ref() else {
        return Vec::new();
    };

    let mut args = match resume_spec.resume {
        ResumeArgs::Flag(flag) => vec![flag.to_string(), session_id.to_string()],
        ResumeArgs::Subcommand(sub) => vec![sub.to_string(), session_id.to_string()],
    };

    if fork {
        match resume_spec.fork {
            Some(ForkArgs::AppendFlag(flag)) => args.push(flag.to_string()),
            Some(ForkArgs::Subcommand(sub)) => {
                // Replace the resume subcommand with the fork subcommand.
                if let Some(first) = args.first_mut() {
                    *first = sub.to_string();
                }
            }
            None => {}
        }
    }

    args
}

/// Resolve reusable settings once, then render the session operation from the
/// same settings that will be persisted. Session controls never enter the saved
/// argv, and explicit prompts/cwd flags survive for this invocation.
pub(crate) fn compose_resume_args(tool: Tool, original: &[String], session: &[String], extra: &[String]) -> (Vec<String>, Vec<String>) {
    use crate::integrations::launch_args::overrides::{override_args, saved_args};
    use crate::integrations::registry::ResumeMerge;
    let inherited = saved_args(tool, original);
    let resolved = override_args(tool, &inherited, extra);
    let merge = tool.spec().resume.as_ref().expect("resume spec").merge;
    let command = match merge {
        ResumeMerge::Append => {
            let mut command = resolved.clone();
            crate::integrations::hooks::runtime::insert_before_separator(&mut command, session.iter().cloned());
            command
        }
        ResumeMerge::Prepend => [session, resolved.as_slice()].concat(),
    };
    (resolved, command)
}

// ── Thread-name resolution ───────────────────────────────────────────────

/// Resolve a user-visible thread name (e.g. a Claude `/rename` title or a
/// Codex thread name) to a session UUID by scanning tool-specific indexes.
///
/// Returns `Ok(Some(uuid))` on a unique match, `Ok(None)` if no tool
/// recognizes the name, or `Err` on within-tool or cross-tool ambiguity.
pub(crate) fn resolve_thread_name(name: &str) -> Result<Option<String>> {
    let claude_match = resolve_claude_thread_name(name)?;
    let codex_match = resolve_codex_thread_name(name)?;

    match (claude_match, codex_match) {
        (Some(claude_id), Some(codex_id)) => {
            bail!(
                "Thread name '{}' matches both Claude (session {}) and Codex (session {}).\n\
                 Use `agentx launch --tool claude -- --resume '{}'` or `agentx launch --tool codex -- resume '{}'` to disambiguate.",
                name,
                claude_id,
                codex_id,
                name,
                name
            );
        }
        (Some(id), None) => {
            log_info("resume", "resolve_thread_name.claude", &format!("name={} session_id={}", name, id));
            Ok(Some(id))
        }
        (None, Some(id)) => {
            log_info("resume", "resolve_thread_name.codex", &format!("name={} session_id={}", name, id));
            Ok(Some(id))
        }
        (None, None) => Ok(None),
    }
}

/// One candidate match produced while scanning a tool's thread-name index.
struct ThreadMatch {
    session_id: String,
    /// Human-readable "when last touched" (mtime ISO-ish for Claude,
    /// `updated_at` for Codex). Used only in ambiguity error messages.
    when: String,
}

/// Resolve a Claude Code custom title to a session UUID by scanning
/// `~/.claude/projects/*/*.jsonl` for `{"type":"custom-title","customTitle":"..."}`.
///
/// `/rename` appends a new `custom-title` entry each time, so within a single
/// transcript only the LAST entry reflects the session's current title. A
/// session renamed `A → B → C` must not match for `A` or `B`.
///
/// Across files, if multiple distinct sessions currently have the same title,
/// we bail rather than silently pick the most recent — the user may have
/// accidentally reused a name.
fn resolve_claude_thread_name(name: &str) -> Result<Option<String>> {
    let projects_dir = claude_projects_dir();
    if !projects_dir.is_dir() {
        return Ok(None);
    }

    let mut matches: Vec<ThreadMatch> = Vec::new();

    let entries = match std::fs::read_dir(&projects_dir) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let sub_entries = match std::fs::read_dir(entry.path()) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for sub_entry in sub_entries.flatten() {
            let path = sub_entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let file = match std::fs::File::open(&path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let reader = std::io::BufReader::new(file);
            // Track the LAST custom-title entry in the file — that's the
            // session's current title.
            let mut last_title: Option<(String, String)> = None;
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => break,
                };
                // Fast pre-filter: skip lines that can't contain a custom-title entry.
                if !line.contains("custom-title") {
                    continue;
                }
                let parsed: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if parsed.get("type").and_then(|v| v.as_str()) != Some("custom-title") {
                    continue;
                }
                let title = match parsed.get("customTitle").and_then(|v| v.as_str()) {
                    Some(t) => t.to_string(),
                    None => continue,
                };
                let session_id = match parsed.get("sessionId").and_then(|v| v.as_str()) {
                    Some(s) => s.to_string(),
                    None => continue,
                };
                last_title = Some((title, session_id));
            }
            if let Some((title, session_id)) = last_title
                && title == name
            {
                let when = sub_entry.metadata().ok().and_then(|m| m.modified().ok()).map(format_system_time).unwrap_or_else(|| "unknown".to_string());
                matches.push(ThreadMatch { session_id, when });
            }
        }
    }

    resolve_one_match("Claude", name, matches)
}

/// Resolve a Codex thread name to a session UUID by scanning
/// `~/.codex/session_index.jsonl`. If multiple rows currently share the
/// name, bail instead of silently picking by `updated_at`.
fn resolve_codex_thread_name(name: &str) -> Result<Option<String>> {
    let index_path = match dirs::home_dir() {
        Some(h) => h.join(".codex/session_index.jsonl"),
        None => return Ok(None),
    };
    let file = match std::fs::File::open(&index_path) {
        Ok(f) => f,
        Err(_) => return Ok(None),
    };
    let reader = std::io::BufReader::new(file);
    let mut matches: Vec<ThreadMatch> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.is_empty() {
            continue;
        }
        let parsed: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if parsed.get("thread_name").and_then(|v| v.as_str()) != Some(name) {
            continue;
        }
        let Some(id) = parsed.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        let when = parsed.get("updated_at").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        matches.push(ThreadMatch { session_id: id.to_string(), when });
    }

    resolve_one_match("Codex", name, matches)
}

/// Turn a vec of candidate matches into at most one. >1 → ambiguity bail.
fn resolve_one_match(tool: &str, name: &str, matches: Vec<ThreadMatch>) -> Result<Option<String>> {
    if matches.len() <= 1 {
        return Ok(matches.into_iter().next().map(|m| m.session_id));
    }

    let mut lines = String::new();
    for m in &matches {
        lines.push_str(&format!("  - {} (touched {})\n", m.session_id, m.when));
    }
    bail!(
        "Thread name '{}' matches {} {} sessions:\n{}\
         Pass the UUID directly to disambiguate.",
        name,
        matches.len(),
        tool,
        lines,
    );
}

/// Format a SystemTime as an ISO-8601-ish UTC string for error messages.
fn format_system_time(t: std::time::SystemTime) -> String {
    let secs = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0);
    dt.map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_else(|| "unknown".to_string())
}

// ── Session-ID adoption ──────────────────────────────────────────────────

/// Opencode session IDs are `ses_` followed by 26 hex+base62 chars
/// (see opencode/packages/opencode/src/id/id.ts).
pub(crate) fn is_opencode_session_id(s: &str) -> bool {
    s.starts_with("ses_") && s.len() >= 8 && s[4..].chars().all(|c| c.is_ascii_alphanumeric())
}

pub(crate) fn opencode_data_dir() -> Option<std::path::PathBuf> {
    crate::runtime::env::opencode_family_data_dir("opencode")
}

pub(crate) fn opencode_family_db_path(tool: Tool) -> Option<std::path::PathBuf> {
    crate::runtime::env::opencode_family_db_path(tool.as_str())
}

/// Query an OpenCode-family SQLite DB for a session's working directory.
///
/// OpenCode 1 and Kilo keep sessions in `session`; OpenCode 2 keeps them in
/// `session_v2`. Either table may be absent, so a failed query (missing
/// table or row) just falls through to the next.
pub(crate) fn lookup_opencode_family_session(tool: Tool, session_id: &str) -> Option<String> {
    let db_path = opencode_family_db_path(tool)?;
    if !db_path.exists() {
        return None;
    }
    let conn = rusqlite::Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX).ok()?;
    ["session", "session_v2"].into_iter().find_map(|table| conn.query_row(&format!("SELECT directory FROM {table} WHERE id = ?1"), rusqlite::params![session_id], |row| row.get::<_, String>(0)).ok())
}

fn lookup_family_session(session_id: &str) -> Option<(Tool, String)> {
    [Tool::OpenCode, Tool::Kilo].into_iter().find_map(|tool| lookup_opencode_family_session(tool, session_id).map(|cwd| (tool, cwd)))
}

/// Resolve a session ID to the owning tool and (optionally) a pre-recovered
/// working directory.
///
/// - Claude, Codex, Gemini: returns `(tool, Some(transcript_path))`.
///   Caller reads the transcript's first line to recover CWD.
/// - OpenCode family: returns `(tool, None)` — opencode stores sessions
///   in SQLite; CWD comes from a separate DB query, not a transcript file.
pub(crate) fn find_session_on_disk(session_id: &str) -> Option<(Tool, Option<String>)> {
    // 1. OpenCode family: prefix-scoped, query the SQLite DBs directly.
    if is_opencode_session_id(session_id) {
        if let Some((tool, _)) = lookup_family_session(session_id) {
            return Some((tool, None));
        }
        return None;
    }

    // 2. Claude: iterate project dirs, check for exact filename
    let projects_dir = claude_projects_dir();
    if projects_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&projects_dir)
    {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let candidate = entry.path().join(format!("{}.jsonl", session_id));
                if candidate.exists() {
                    let path_str = candidate.to_string_lossy().to_string();
                    if let Some(tool) = detect_tool_from_path(&path_str) {
                        return Some((tool, Some(path_str)));
                    }
                }
            }
        }
    }

    // 3. Codex
    if let Some(path) = derive_codex_transcript_path(session_id)
        && let Some(tool) = detect_tool_from_path(&path)
    {
        return Some((tool, Some(path)));
    }

    // 4. Gemini
    if let Some(path) = derive_gemini_transcript_path(session_id)
        && let Some(tool) = detect_tool_from_path(&path)
    {
        return Some((tool, Some(path)));
    }

    // 5. Cursor
    if let Some(path) = derive_cursor_transcript_path(session_id)
        && let Some(tool) = detect_tool_from_path(&path)
    {
        return Some((tool, Some(path)));
    }

    // 6. Kimi
    if let Some(path) = derive_kimi_transcript_path(session_id)
        && let Some(tool) = detect_tool_from_path(&path)
    {
        return Some((tool, Some(path)));
    }

    // 6b. Qoder (Claude-format transcripts under its own config dir)
    if let Some(path) = derive_qoder_transcript_path(session_id) {
        return Some((Tool::Qoder, Some(path)));
    }

    // 7. Copilot
    if let Some(path) = derive_copilot_transcript_path(session_id)
        && let Some(tool) = detect_tool_from_path(&path)
    {
        return Some((tool, Some(path)));
    }

    // 8. Grok
    if let Some(path) = derive_grok_transcript_path(session_id) {
        return Some((Tool::Grok, Some(path)));
    }

    // 9. Pi / OMP. Attribute by root PROVENANCE, not probe order:
    //   - `PI_CODING_AGENT_SESSION_DIR` + `~/.pi`            => Pi-exclusive
    //   - XDG-omp / `~/.omp` / OMP profile trees             => OMP-exclusive
    //   - `PI_CODING_AGENT_DIR/sessions`                     => genuinely shared
    // Exclusive roots are authoritative. The shared root is attributed only by
    // the path's product marker (managed configs carry `.pi`/`.omp`); a
    // markerless arbitrary shared dir is genuinely ambiguous (both tools write
    // identical headers) and is surfaced as an error by the caller rather than
    // guessed. `PiOmpMatch::Ambiguous` carries the path so the caller can name it.
    match resolve_pi_omp_on_disk(session_id) {
        PiOmpMatch::Found { tool, path } => return Some((tool, Some(path))),
        // Ambiguous is handled by build_adopt_plan (which re-derives via
        // `ambiguous_pi_omp_session`); fall through to the not-found path here so
        // untracked callers still get a definite answer.
        PiOmpMatch::Ambiguous { .. } | PiOmpMatch::None => {}
    }

    None
}

enum PiOmpMatch {
    Found { tool: Tool, path: String },
    Ambiguous { path: String },
    None,
}

/// Locate a Pi/OMP session id on disk and attribute it by root ownership.
fn resolve_pi_omp_on_disk(session_id: &str) -> PiOmpMatch {
    use crate::transcript::{omp_exclusive_roots, omp_session_roots, pi_exclusive_roots, pi_session_roots, shared_agent_dir_root};

    let shared = shared_agent_dir_root();
    let shared_owner = shared.as_ref().and_then(|root| {
        let pi_active = pi_session_roots().contains(root);
        let omp_active = omp_session_roots().contains(root);
        match (pi_active, omp_active) {
            (true, true) => Some(None),
            (true, false) => Some(Some(Tool::Pi)),
            (false, true) => Some(Some(Tool::Omp)),
            (false, false) => None,
        }
    });

    let roots = pi_exclusive_roots().into_iter().map(|root| (root, Some(Tool::Pi))).chain(omp_exclusive_roots().into_iter().map(|root| (root, Some(Tool::Omp)))).chain(shared.zip(shared_owner));

    for (root, owner) in roots {
        if root.exists()
            && let Some(path) = find_pi_transcript_in_root(&root, session_id)
        {
            return match owner.or_else(|| pi_omp_path_owner(&path)) {
                Some(tool) => PiOmpMatch::Found { tool, path },
                None => PiOmpMatch::Ambiguous { path },
            };
        }
    }
    PiOmpMatch::None
}

/// Return the owner encoded in a shared path's `.pi` or `.omp` marker.
fn pi_omp_path_owner(path: &str) -> Option<Tool> {
    detect_tool_from_path(path).filter(|tool| matches!(tool, Tool::Pi | Tool::Omp))
}

/// The path of a Pi/OMP session that was found but is genuinely ambiguous
/// (markerless shared `PI_CODING_AGENT_DIR`). Used by the adoption path to raise
/// a precise disambiguation error instead of "not found".
pub(crate) fn ambiguous_pi_omp_session(session_id: &str) -> Option<String> {
    match resolve_pi_omp_on_disk(session_id) {
        PiOmpMatch::Ambiguous { path } => Some(path),
        _ => None,
    }
}

fn find_pi_transcript_in_root(root: &std::path::Path, session_id: &str) -> Option<String> {
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_pi_transcript_in_root(&path, session_id) {
                return Some(found);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") && path.file_name().and_then(|n| n.to_str()).is_some_and(|name| name.contains(session_id)) {
            return Some(path.to_string_lossy().to_string());
        }
    }
    None
}

/// Locate a cursor-agent transcript by conversation UUID (== agentx session_id).
/// cursor writes it at `~/.cursor/projects/<slug>/agent-transcripts/<uuid>/
/// <uuid>.jsonl` — the same path the hook reports. The `<slug>` can't be
/// derived from the UUID, so scan the per-project dirs for the nested file.
/// (The flat `projects/agent-transcripts/<uuid>.jsonl` mirror is skipped: it
/// has no sibling `.workspace-trusted`, so no cwd could be recovered from it.)
fn derive_cursor_transcript_path(session_id: &str) -> Option<String> {
    let projects = dirs::home_dir()?.join(".cursor").join("projects");
    let file = format!("{session_id}.jsonl");
    for entry in std::fs::read_dir(&projects).ok()?.flatten() {
        let candidate = entry.path().join("agent-transcripts").join(session_id).join(&file);
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

/// Locate a Qoder CLI transcript by session UUID:
/// `<QODER_CONFIG_DIR or ~/.qoder>/projects/<slug>/<uuid>.jsonl`.
fn derive_qoder_transcript_path(session_id: &str) -> Option<String> {
    let projects = qoder_projects_dir();
    std::fs::read_dir(projects).ok()?.flatten().map(|entry| entry.path().join(format!("{session_id}.jsonl"))).find(|path| path.is_file()).map(|path| path.to_string_lossy().into_owned())
}

/// Locate a Copilot CLI transcript by session UUID.
/// Copilot stores transcripts at `$COPILOT_HOME/session-state/<uuid>/events.jsonl`
/// where `COPILOT_HOME` defaults to `~/.copilot`.
fn derive_copilot_transcript_path(session_id: &str) -> Option<String> {
    let copilot_home = if let Ok(dir) = std::env::var("COPILOT_HOME")
        && !dir.is_empty()
    {
        std::path::PathBuf::from(dir)
    } else {
        dirs::home_dir()?.join(".copilot")
    };
    let candidate = copilot_home.join("session-state").join(session_id).join("events.jsonl");
    if candidate.exists() { Some(candidate.to_string_lossy().to_string()) } else { None }
}

/// Grok keeps each session in `$GROK_HOME/sessions/<url-encoded cwd>/<id>/`.
fn derive_grok_transcript_path(session_id: &str) -> Option<String> {
    let sessions = crate::transcript::grok::grok_config_dir().join("sessions");
    std::fs::read_dir(sessions).ok()?.flatten().map(|entry| entry.path().join(session_id).join("updates.jsonl")).find(|path| path.exists()).map(|path| path.to_string_lossy().into_owned())
}

/// Grok's `summary.json`, next to `updates.jsonl`, records the session cwd.
fn recover_grok_cwd(transcript_path: &str) -> Option<String> {
    let summary = std::path::Path::new(transcript_path).parent()?.join("summary.json");
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(summary).ok()?).ok()?;
    value["info"]["cwd"].as_str().map(str::to_string)
}

/// cursor transcripts carry no `cwd`, so recover it from the per-workspace
/// `.workspace-trusted` marker cursor writes at
/// `~/.cursor/projects/<slug>/.workspace-trusted` (`workspacePath` field). The
/// transcript lives under `<slug>/agent-transcripts/<uuid>/<uuid>.jsonl`, so
/// walk up to the project-slug dir (the direct child of `projects/`) and read
/// it. Returns the recorded (canonicalized) workspace path.
fn recover_cursor_cwd(transcript_path: &str) -> Option<String> {
    let path = std::path::Path::new(transcript_path);
    let mut slug_dir = None;
    let mut cursor = path.parent();
    while let Some(dir) = cursor {
        if dir.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()) == Some("projects") {
            slug_dir = Some(dir);
            break;
        }
        cursor = dir.parent();
    }
    let marker = slug_dir?.join(".workspace-trusted");
    let data = std::fs::read_to_string(&marker).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&data).ok()?;
    parsed.get("workspacePath").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string())
}

/// Recover the session's working directory from the tool's on-disk state.
///
/// - **Claude**: `cwd` is on most entry lines but NOT on line 1 (which is a
///   `permission-mode` header). Scan the first few lines until one carries a
///   non-empty `cwd`; give up after a small cap to keep I/O bounded.
/// - **Codex**: first line is `session_meta` with `payload.cwd`; read line 1.
/// - **Gemini**: the session JSON has `projectHash = sha256(cwd)` (hex).
///   `~/.gemini/projects.json` maps `cwd → short-id`. Hash each key and
///   match against `projectHash` to recover the original CWD.
/// - **Cursor**: the transcript has no `cwd`; recover it from the sibling
///   `~/.cursor/projects/<slug>/.workspace-trusted` marker (`workspacePath`).
/// - **Copilot**: `session.start` entry carries `cwd` in the `data` object;
///   scan the first few lines for it.
pub(crate) fn extract_cwd_from_transcript(path: &str, tool: Tool) -> Option<String> {
    match tool {
        // Qoder transcripts are Claude-format: `cwd` rides on the first user record.
        Tool::Claude | Tool::Qoder => scan_lines_for_cwd(path, 20, |v| v.get("cwd").and_then(|c| c.as_str())),
        Tool::Codex => scan_lines_for_cwd(path, 1, |v| v.get("payload").and_then(|p| p.get("cwd")).and_then(|c| c.as_str())),
        // Antigravity shares Gemini's session tree/format (GEMINI_CLI_HOME);
        // an agy-pathed transcript detected as Antigravity still has the
        // gemini `projectHash` field, so reuse the same recovery.
        Tool::Gemini | Tool::Antigravity => recover_gemini_cwd(path),
        Tool::Cursor => recover_cursor_cwd(path),
        Tool::Kimi => None, // Kimi context.jsonl does not store cwd
        Tool::Grok => recover_grok_cwd(path),
        Tool::Copilot => scan_lines_for_cwd(path, 20, |v| {
            v.get("event").or_else(|| v.get("type")).and_then(|event| event.as_str()).filter(|event| *event == "session.start").and_then(|_| v.get("data")).and_then(|data| data.get("cwd")).and_then(|cwd| cwd.as_str())
        }),
        Tool::Pi | Tool::Omp => scan_lines_for_cwd(path, 10, |v| if v.get("type").and_then(|t| t.as_str()) == Some("session") { v.get("cwd").or_else(|| v.get("session").and_then(|s| s.get("cwd"))).and_then(|c| c.as_str()) } else { None }),
        // Found through their own session stores, not a transcript path.
        Tool::OpenCode | Tool::Kilo | Tool::Hermes | Tool::Adhoc => None,
    }
}

/// Read up to `max_lines` lines from a JSONL transcript and return the first
/// non-empty CWD produced by `pick`. Malformed lines are skipped.
fn scan_lines_for_cwd(path: &str, max_lines: usize, pick: impl Fn(&serde_json::Value) -> Option<&str>) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines().take(max_lines) {
        let Ok(line) = line else { continue };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        if let Some(cwd) = pick(&parsed)
            && !cwd.is_empty()
        {
            return Some(cwd.to_string());
        }
    }
    None
}

/// Gemini session JSON has `projectHash = hex(sha256(cwd))`. Read it, then
/// reverse-lookup in `~/.gemini/projects.json` which maps `cwd → short-id`.
fn recover_gemini_cwd(transcript_path: &str) -> Option<String> {
    // The session file is a plain JSON object (not JSONL), so read the whole
    // file — but only as far as needed to parse the outer object. For
    // moderately large transcripts this is still comparable to a handful of
    // line reads, and there is no cheaper path.
    let data = std::fs::read_to_string(transcript_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&data).ok()?;
    let target_hash = parsed.get("projectHash").and_then(|v| v.as_str())?;

    let gemini_base = if let Ok(cli_home) = std::env::var("GEMINI_CLI_HOME") { std::path::PathBuf::from(cli_home).join(".gemini") } else { dirs::home_dir()?.join(".gemini") };
    let registry_path = gemini_base.join("projects.json");
    let registry_data = std::fs::read_to_string(&registry_path).ok()?;
    let registry: serde_json::Value = serde_json::from_str(&registry_data).ok()?;
    let projects = registry.get("projects").and_then(|v| v.as_object())?;

    use sha2::{Digest, Sha256};
    for cwd in projects.keys() {
        let digest = Sha256::digest(cwd.as_bytes());
        let hex = digest.iter().fold(String::with_capacity(64), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{:02x}", b);
            acc
        });
        if hex == target_hash {
            return Some(cwd.clone());
        }
    }
    None
}

pub(crate) struct PreparedResume {
    pub tool: Tool,
    pub output: ResumeOutputContext,
    pub launch: LaunchParams,
    pub session_id: String,
    pub source: Option<crate::agents::lifecycle_service::ResumeFence>,
    pub tracked_fork_identity: Option<TrackedForkIdentity>,
}

pub(crate) struct ResumeOutputContext {
    pub action: String,
    pub tool: String,
    pub terminal: Option<String>,
    pub background: bool,
    pub run_here: Option<bool>,
    pub launcher_name: String,
    pub json: bool,
}

pub(crate) struct TrackedForkIdentity {
    pub parent_name: String,
    pub custom_initial_prompt: Option<String>,
    pub custom_system_prompt: Option<String>,
}

fn adopt_snapshot(session_id: &str, fork: bool) -> Result<(Tool, Option<String>)> {
    // A markerless session under a shared PI_CODING_AGENT_DIR cannot be
    // attributed to Pi vs OMP without guessing; surface it explicitly rather
    // than launching the wrong tool against the transcript.
    if let Some(path) = ambiguous_pi_omp_session(session_id) {
        bail!(
            "Session {session_id} was found under a shared PI_CODING_AGENT_DIR \
             ({path}) with no product marker, so agentx cannot tell whether it is a \
             Pi or Oh My Pi session (both write identical transcripts).\n\
             Disambiguate by launching the owning tool explicitly, e.g.\n  \
             agentx launch --tool pi -- --resume {session_id}\n  agentx launch --tool omp -- --resume {session_id}"
        );
    }

    let (tool, transcript_path) = find_session_on_disk(session_id).ok_or_else(|| {
        // find_session_on_disk short-circuits for `ses_` IDs (the OpenCode
        // family is searched), so scope the error to match that lookup.
        let opencode_db = opencode_data_dir().map(|d| d.join("opencode.db").display().to_string()).unwrap_or_else(|| "(no data dir)".to_string());
        let kilo_db = opencode_family_db_path(Tool::Kilo).map(|path| path.display().to_string()).unwrap_or_else(|| "(no data dir)".to_string());
        if is_opencode_session_id(session_id) {
            anyhow::anyhow!(
                "Session {sid} not found. Searched:\n  \
                 - Opencode: {opencode_db} (table 'session')\n  \
                 - Kilo:     {kilo_db} (table 'session')",
                sid = session_id,
                opencode_db = opencode_db,
                kilo_db = kilo_db,
            )
        } else {
            let claude_projects = claude_projects_dir();
            anyhow::anyhow!(
                "Session {sid} not found. Searched:\n  \
                 - Claude:   {claude}/*/{sid}.jsonl\n  \
                 - Codex:    ~/.codex/sessions/**/*-{sid}.jsonl\n  \
                 - Gemini:   ~/.gemini/tmp/*/chats/session-*-{short}*.json\n  \
                 - Cursor:   ~/.cursor/projects/*/agent-transcripts/{sid}/{sid}.jsonl\n  \
                 - Qoder:    ~/.qoder/projects/*/{sid}.jsonl\n  \
                 - Copilot:  ~/.copilot/session-state/{sid}/events.jsonl\n  \
                 - Grok:     {grok}/*/{sid}/updates.jsonl\n  \
                 - Oh My Pi: ~/.omp/agent/sessions/**/*{sid}*.jsonl
                 - Pi:       ~/.pi/agent/sessions/**/*{sid}*.jsonl",
                sid = session_id,
                claude = claude_projects.display(),
                grok = crate::transcript::grok::grok_config_dir().join("sessions").display(),
                short = session_id.split('-').next().unwrap_or(session_id),
            )
        }
    })?;

    // CWD recovery: for opencode, the session row carries `directory` directly.
    // For Claude/Codex, scan transcript entries for the recorded cwd (CWD is
    // fixed at session start; no tool changes CWD mid-session). For fork we
    // start in $PWD (or --dir) by design.
    let cwd_hint = if fork {
        None
    } else {
        let raw = if matches!(tool, Tool::OpenCode | Tool::Kilo) {
            lookup_opencode_family_session(tool, session_id)
        } else if let Some(ref path) = transcript_path {
            extract_cwd_from_transcript(path, tool)
        } else {
            None
        };

        match raw {
            Some(cwd) if std::path::Path::new(&cwd).is_dir() => Some(cwd),
            Some(cwd) => {
                eprintln!("Warning: original directory '{}' no longer exists, using current directory", cwd);
                None
            }
            None => None,
        }
    };

    Ok((tool, cwd_hint))
}
