//! Build the one-time bootstrap prompt injected into a newly connected agent.
//!
//! Teaches the agent its identity and the `agentx` CLI contract (messaging,
//! listing, spawning). Routine commands are executable directly; errors and
//! optional setup route agents to command help when needed.

use std::collections::BTreeMap;
use std::fs;
use std::sync::LazyLock;

use crate::db::AgentxDb;
use crate::integrations::tool::Tool;
use crate::paths;
use crate::runtime::context::AgentxContext;
use crate::shared::constants::{SENDER, ST_ACTIVE, ST_INACTIVE, ST_LISTENING};

// Bundled script names (compile-time known).
// User scripts are discovered at runtime from ~/.agentx/scripts/.

// Commands resolve the injected process/session identity. Required-identity
// operations report missing or expired bindings; prompts need no repeated flag.
// --iam is an explicit caller selection, while start --name owns registration.

pub(crate) const EXCLUDE_PARTS: &[&str] = &["identity", "messages", "capabilities", "rules", "handoff"];

pub(crate) fn parse_excluded_parts(value: &str) -> Result<std::collections::HashSet<&str>, String> {
    let value = value.trim();
    if value.is_empty() || value == "none" {
        return Ok(std::collections::HashSet::new());
    }
    if value == "all" {
        return Ok(EXCLUDE_PARTS.iter().copied().collect());
    }
    let mut parts = std::collections::HashSet::new();
    for part in value.split(',').map(str::trim) {
        if part == "none" || part == "all" {
            return Err(format!("bootstrap_exclude: '{part}' must be used alone"));
        }
        if !EXCLUDE_PARTS.contains(&part) {
            return Err(format!("bootstrap_exclude: unknown part '{part}' (valid: {}, none, all)", EXCLUDE_PARTS.join(", ")));
        }
        if !parts.insert(part) {
            return Err(format!("bootstrap_exclude: duplicate part '{part}'"));
        }
    }
    Ok(parts)
}

const IDENTITY_HEAD: &str = r#"[AGENTX SESSION]
You have access to the agentx cli communication tool.
- Your name: {display_name}{launch_facts}
- Authority: Prioritize @{SENDER} over others{launched_by}

You run agentx commands on behalf of the human user. The human uses natural language with you."#;

const MESSAGES: &str = r#"## MESSAGES

Sending rules:
- Use --intent on sends: request (want reply), inform (dont need reply), ack (responding).

Response rules:
- From {SENDER} or intent=request → always respond
- intent=inform → respond only if useful
- intent=ack → don't respond

Routing rules:
- agentx message (<agentx> tags, hook feedback) → run `agentx send` to respond
- Normal user chat → respond in chat"#;

const CAPABILITIES: &str = r#"## CAPABILITIES

Use `agentx <cmd+flags>`. Your injected process/session identifies you automatically.
If an identity error occurs, follow the CLI recovery instructions. Use `--iam {instance_name}` only when explicit caller selection is needed.

- Message: send {target_name_s} [--intent request|inform|ack] [--reply-to <id>] [--thread <thread_name>] -- 'plain text'
  Or (for code/md/backticks) instead of --: --file <path> | --base64 <string> | pipe/heredoc
  Example: send {target_luna} {target_nova} --intent ack --reply-to 82 -- 'ok'
- See who's active: list [name] [-v] [--json]
- Read another's conversation: transcript [name] [N-M] [--last N] [--full] [--detailed (tools/io)] | transcript search 'text' [--all]
- View events: events [--last N] [--all] [--sql EXPR] [filters]
  Filters (same flag=OR, different=AND): --agent NAME | --type message|status|life | --status listening|active|blocked | --cmd PATTERN (contains, ^prefix, =exact) | --file PATH (*.py for glob, file.py for contains)
  Get notified (watch agents, react): events sub [filters] [--once] | --help
  Example: events sub --idle luna → <agentx> msg when luna goes idle
- Handoff context: bundle prepare
- Spawn agents{handoff_result}
{role_workspace_guide}
- Resume: agentx resume <name> | Fork: agentx fork <name> | Kill: agentx kill <name(s)>
- Run workflows: run <script> [args] [--help]
  {scripts}
- View agent screen: term [name] | inject text/enter: term inject <name> ['text'] [--enter]
- Other commands: status (diagnostics), config (set terminal, etc), relay (remote)

Run the documented commands directly. Consult `agentx <command> --help` for options not covered here or when an error requires recovery."#;

// Shared by the full startup guide and native subagent guide. Keep detailed
// role-file setup in shipped roles help, which run docs/help-all also consume.
const ROLE_WORKSPACE_GUIDE: &str = r#"
Routine task delegation:
  Example: `{agentx_cmd} launch --go --headless --title "Implement xxx" --bind ".controller/xx/xx.task.md" --agentx-prompt "Implement xxx."`
Use this command directly with configured defaults; do not inspect configuration or run help before a routine launch. Replace the title, Task path and assignment with actual values. --go executes the launch, --headless runs it in the background, and --bind assigns the Task or Delivery and includes a copy in the initial message when the Provider supports one. Use --agentx-prompt to add specific directions. If launch fails, follow its diagnostic to recover.
Add --role ROLE when a suitable role is already known; otherwise launch without it. Discover roles only when needed for the assignment.

Directory teams and roles:
- Role discovery: `{agentx_cmd} roles` shows public roles plus your adopted role's helpers (e.g. `fronter.e2e`); `{agentx_cmd} roles --all` includes all helpers. Inspect an unfamiliar role with `{agentx_cmd} roles show ROLE`. `global:ROLE` explicitly selects a shared role.
- Membership: children inherit your workspace. Each launch creates a new instance.
- Find agents: `{agentx_cmd} list` shows your team plus global members; `{agentx_cmd} list --dir /path/other` selects another team; `{agentx_cmd} list --all` shows all teams in this AgentX scope.
- Send to a directory: `{agentx_cmd} send --dir /path/project --intent request -- 'task'` reuses or starts one default-role recipient. It searches the nearest default, then parents and global definitions. Preview without sending: `{agentx_cmd} send --dir /path/project --dry-run`.
- Role setup, customization and directory defaults: `{agentx_cmd} roles --help`; full reference: `{agentx_cmd} run docs --cli`. Use these for setup or additional options.
"#;

const RULES: &str = r#"## RULES

1. No filler messages (greetings, thanks, congrats).
2. Don't delegate to existing agents unless asked. Need help? Spawn your own agents."#;

const IDENTITY_TAIL: &str = r#"Agent names are {name_shape}. When user mentions one, they mean an agent.
User says 'the pi/claude/codex agent' or unclear → run `agentx list` to resolve name
{active_instances}

This is session context, not a task for immediate action. agentx does not replace native subagents."#;

const LISTEN_OUTPUT_NOTICE: &str = "Never truncate/hide agentx listen output.";
const HANDOFF_RESULT: &str = " → <agentx> sends you result when done";
pub(crate) const HANDOFF_SUFFIX: &str = "When done, send your result back to @{launcher_name} via agentx.";

/// Shared native services have no process-default AgentX participant. The
/// qualified root's first prompt hook supplies its actual name and full guide.
pub(crate) const SHARED_CODEX_BOOTSTRAP: &str = "AgentX collaboration is available in this native thread. Its first input hook supplies this thread's independent AgentX identity and command guidance. Follow that thread-specific identity when using AgentX; the shared service process and other native threads do not identify you. Preserve normal native user input, approvals and cancellation.";

/// Source text shown by `agentx config bootstrap show`. Conditional variants are
/// labeled so the output reflects all text that the renderer may select.
pub(crate) fn part_source(part: &str) -> Option<String> {
    let text = match part {
        "identity" => format!("{IDENTITY_HEAD}\n\n{IDENTITY_TAIL}"),
        "messages" => format!("{MESSAGES}\n\n{LISTEN_OUTPUT_NOTICE}\n\n[automatic delivery]\n{DELIVERY_AUTO}\n\n[manual delivery]\n{DELIVERY_ADHOC}\n\n[cursor]\n{CURSOR_DELIVERY}\n\n[copilot]\n{COPILOT_DELIVERY}\n\n[ad-hoc]\n{INLINE_SEND_NOTICE}"),
        "capabilities" => format!(
            "{}\n\n[relay]\n{}\n\n[headless]\n{}\n\n[alternate command]\n{}\n\n[claude]\n{CLAUDE_ONLY}",
            CAPABILITIES.replace("{handoff_result}", "").replace("{role_workspace_guide}", ROLE_WORKSPACE_GUIDE),
            RELAY_NOTICE.trim(),
            HEADLESS_NOTICE.trim(),
            UVX_CMD_NOTICE.trim()
        ),
        "rules" => RULES.to_string(),
        "handoff" => format!("[spawn notice suffix]\n{HANDOFF_RESULT}\n\n[agent-launched task prompt suffix]\n{HANDOFF_SUFFIX}"),
        _ => return None,
    };
    Some(text)
}

const RELAY_NOTICE: &str = r#"
Remote agents have suffix (e.g., `luna:BOXE`). @luna = local only; @luna:BOXE = remote. Remote event IDs 42:BOXE. Remote launch needs --device BOXE and --dir passed in. Remote agentx events needs --remote-fetch --device BOXE; remote agent details in list are previews, add --full for the whole text. Remote events sub needs --device BOXE. transcript, term, kill, r, f take name:BOXE."#;

const HEADLESS_NOTICE: &str = r#"
Headless mode: No one sees your chat, only agentx messages. Communicate via agentx send."#;

const UVX_CMD_NOTICE: &str = r#"
Note: agentx command in this environment is `{agentx_cmd}`."#;

// Tool-specific delivery
//
// "end your turn to receive" — a behavioral nudge, not a technical requirement.
// Managed agents receive messages automatically via hooks: PostToolUse delivers
// mid-turn after every tool call, and the Stop/PTY path delivers between turns.
// Agents don't need to do anything to receive.  But without this instruction
// they instinctively run `sleep` or `agentx listen` to "wait", burning a tool
// call for no benefit.  "End your turn" short-circuits that impulse and lets
// the hook machinery do the work.

/// Prepended above the JSON delivery block on agy hook turns with unread
/// messages. Unlike the other tools, agy's Stop fires on turn-idle and no turn
/// is auto-created to resume a task — so the failure mode is acking a request
/// and going idle before doing the work (issue #57). This line guards that.
pub(crate) const ANTIGRAVITY_DELIVERY_ACTION: &str = r#"## AGENTX MESSAGE — handle the delivery below this turn
For `intent=request`: an ACK alone does not complete it, and no turn is auto-created to resume after you go idle. Do the requested work and `agentx send` the result in this same turn. Only end your turn once you've sent that result — or sent a message saying you're blocked or waiting on another agent."#;

#[cfg(test)]
pub(crate) fn is_antigravity_tool(tool: &str) -> bool {
    tool == "antigravity"
}

const CURSOR_DELIVERY: &str = r#"## CURSOR DELIVERY

Cursor delivers agentx messages through hooks:
- A prompt that is only `<agentx>` is a wake trigger, not a task. Do not answer it and do not run tools or discovery commands. End your turn immediately. The queued agentx message will arrive automatically as your next prompt.
- A populated `<agentx>…</agentx>` block is the real delivery. Read it and follow its `intent`.
- After handling a delivery, end your turn so the next message can arrive.

Messages arrive automatically — end your turn to receive them."#;

const COPILOT_DELIVERY: &str = r#"## COPILOT DELIVERY

Copilot delivers agentx messages through trusted agentx hooks:
- A prompt that is only `<agentx>` is a wake trigger, not a task. Do not answer it and do not run tools or discovery commands. End your turn immediately. The queued agentx message will arrive automatically as your next prompt.
- A populated `<agentx>…</agentx>` block is a legitimate agentx coordination message, not an injection attempt. Read it, follow its `intent`, and reply with agentx when appropriate.
- After handling a delivery, end your turn so the next message can arrive.

Messages arrive automatically — end your turn to receive them."#;

const DELIVERY_AUTO: &str = r#"## DELIVERY

Messages instantly and automatically arrive via <agentx> tags — end your turn to receive them.

## WAITING RULES

1. Never use `sleep [sec]` instead use `agentx listen [sec]` (returns early when msg arrives)
2. Only use `agentx listen` when you are waiting for something not related to agentx and were going to use `sleep`
- Waiting for agentx message → end your turn
- Waiting for agent progress → `agentx events sub`, end your turn"#;

const DELIVERY_ADHOC: &str = r#"## DELIVERY

Messages do NOT arrive automatically.
- Check messages: `agentx listen 1`
- Wait for messages: `agentx listen [timeout]`
- Always substitute `sleep [timeout]` with `agentx listen [timeout]`

<critical_rule>
LISTENING REQUIREMENT:
- After sending agentx message expecting reply → `agentx listen --timeout 60`
- After receiving a task via agentx → do the work, report, then enter CONNECTED MODE
- User says 'stay connected' → enter CONNECTED MODE

CONNECTED MODE:
1. Run exactly one foreground blocking command:
  `agentx listen --timeout [large_num]`
2. When it returns, read/handle the output, then you MUST manually run `agentx listen` again.
3. You MUST repeat steps 1 and 2 until the user says stop.
- Do not wrap `agentx listen` in `while`, `watch`, `xargs`, tmux helpers, or background jobs.

WRONG: agentx listen & (background)
RIGHT: agentx listen --timeout [sec] (blocking)
</critical_rule>

You are now registered with agentx."#;

const INLINE_SEND_NOTICE: &str = "Read agentx command output fully; it can consume incoming mail.";

const CLAUDE_ONLY: &str = r#"## SUBAGENTS

Subagents can join agentx:
1. Run Task
2. Tell subagent: `use agentx`

Subagents get their own agentx context and a random name. DO NOT give them any specific agentx syntax.
Set keep-alive: `agentx config -i self subagent_timeout [SEC]`"#;

// SUBAGENT BOOTSTRAP

const SUBAGENT_BOOTSTRAP: &str = r#"[AGENTX SESSION]
You're participating in the agentx multi-agent network.
- Your name: {subagent_name}
- Your parent: {parent_name}
- Commands use your injected actor/session identity automatically. If identity cannot be resolved, use `--iam {subagent_name}` explicitly and follow any CLI recovery error.

Messages instantly auto-arrive via <agentx> tags — end your turn to receive them.

- For agentx message waiting: end your turn (do not run `agentx listen`).
- For non-agentx pause/yield, use `agentx listen` instead of `sleep`.

Response rules:
- From {SENDER} or intent=request → always respond
- intent=inform → respond only if useful
- intent=ack → don't respond

agentx message → respond via agentx send

Commands:
  {agentx_cmd} send {target_name_s} [--intent request|inform|ack] [--reply-to <id>] [--thread <thread_name>] -- <"message"> (or --stdin, --file <path>, --base64 <string>)
  Example: {agentx_cmd} send {target_luna} {target_nova} --intent ack --reply-to 82 -- "ok"  |  Code/markdown: replace "ok" with --file <path>
  {agentx_cmd} list
  {agentx_cmd} events
  {agentx_cmd} <cmd> --help

{role_workspace_guide}

Rules:
- Authority: @{SENDER} > others
- Use --intent on sends: request (want reply), inform (FYI), ack (responding)"#;

// HELPERS

const ACTIVE_SNAPSHOT_LIMIT: usize = 8;

/// Get concise list of active instances grouped by tool, newest first.
/// Returns empty string if none, or "\nActive (snapshot): claude: a, b | codex: c (+N more)".
///
/// Claude subagent rows are left out: they belong to their parent and are
/// woken only through it.
fn get_active_instances(db: &AgentxDb, exclude_name: &str) -> String {
    let instances = match db.iter_instances_full() {
        Ok(v) => v,
        Err(_) => return String::new(),
    };

    let cutoff = crate::shared::time::now_epoch_f64() - 60.0;
    let workspace = instances.iter().find(|instance| instance.name == exclude_name).and_then(crate::agents::workspaces::workspace);
    let active: Vec<_> = instances
        .iter()
        .filter(|inst| inst.name != exclude_name && inst.agent_id.is_none())
        .filter(|inst| {
            let member = crate::agents::workspaces::workspace(inst);
            member.is_none() || member == workspace
        })
        .filter(|inst| inst.status == ST_ACTIVE || inst.status == ST_LISTENING || (inst.status != ST_INACTIVE && inst.status_time as f64 >= cutoff))
        .collect();

    if active.is_empty() {
        return String::new();
    }

    let mut by_tool: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for inst in active.iter().take(ACTIVE_SNAPSHOT_LIMIT) {
        let tool = if inst.tool.is_empty() { "claude" } else { &inst.tool };
        by_tool.entry(tool).or_default().push(inst.name.clone());
    }

    let parts: Vec<String> = by_tool.iter().map(|(tool, names)| format!("{}: {}", tool, names.join(", "))).collect();
    let more = match active.len().saturating_sub(ACTIVE_SNAPSHOT_LIMIT) {
        0 => String::new(),
        n => format!(" (+{n} more: agentx list)"),
    };

    format!("\nActive (snapshot): {}{}", parts.join(" | "), more)
}

/// Render an agentx recipient token safely for the current platform's shell.
///
/// PowerShell parses a bare `@name` as splatting syntax and removes it before
/// agentx can see it, which can turn an intended direct message into a broadcast.
fn recipient_token(name: &str) -> String {
    let token = format!("@{name}");
    if cfg!(windows) { format!("'{token}'") } else { token }
}

/// Get combined list of bundled + user scripts.
/// Returns empty string if none, or "Scripts: clone, debate, ...".
fn get_scripts(agentx_dir: &std::path::Path) -> String {
    let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    // Bundled scripts (compile-time known)
    for (name, _) in crate::workflows::SCRIPTS {
        names.insert(name.to_string());
    }

    // User scripts from ~/.agentx/scripts/
    let user_dir = agentx_dir.join(paths::SCRIPTS_DIR);
    if let Ok(entries) = fs::read_dir(&user_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !name.is_empty() && !name.starts_with('_') && (ext == "py" || ext == "sh") {
                names.insert(name.to_string());
            }
        }
    }

    if names.is_empty() {
        return String::new();
    }

    format!("Scripts: {}", names.into_iter().collect::<Vec<_>>().join(", "))
}

/// "\n- Launched by: @x" when another agent launched this one; empty for
/// human/API launches (the launcher records those as api/user) and self.
fn launched_by_line(launched_by: Option<&str>, instance_name: &str) -> String {
    match launched_by {
        Some(name) if !matches!(name, "" | "api" | "user" | "unknown" | SENDER) && name != instance_name => {
            format!("\n- Launched by: {}", recipient_token(name))
        }
        _ => String::new(),
    }
}

// CONTEXT BUILDER

/// All context needed to render bootstrap templates.
#[derive(Clone)]
struct BootstrapContext {
    instance_name: String,
    display_name: String,
    launch_facts: String,
    relay_enabled: bool,
    agentx_cmd: String,
    is_headless: bool,
    active_instances: String,
    name_shape: String,
    scripts: String,
    launched_by: String,
    notes: String,
}

/// Build context for template substitution.
///
#[cfg(test)]
fn build_context(db: &AgentxDb, agentx_ctx: &AgentxContext, instance_name: &str, relay_enabled: bool) -> BootstrapContext {
    build_context_from_snapshot(snapshot(db, instance_name), agentx_ctx, instance_name, relay_enabled)
}

/// Durable projection only. Caller-owned scripts, configuration, launch
/// command and environment are deliberately filled by the renderer's host.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct BootstrapSnapshot {
    pub record: Option<crate::db::InstanceRow>,
    pub active_instances: String,
    pub name_shape: String,
}

pub(crate) fn snapshot(db: &AgentxDb, name: &str) -> BootstrapSnapshot {
    BootstrapSnapshot { record: db.get_instance_full(name).ok().flatten(), active_instances: get_active_instances(db, name), name_shape: crate::agents::names::describe_generated_names(db).to_owned() }
}

fn build_context_from_snapshot(snapshot: BootstrapSnapshot, agentx_ctx: &AgentxContext, instance_name: &str, relay_enabled: bool) -> BootstrapContext {
    let collaboration = snapshot.record.as_ref().and_then(crate::agents::workspaces::collaboration);
    let workspace = collaboration.as_ref().and_then(|value| value.workspace.as_deref()).unwrap_or("global");
    let role = collaboration.as_ref().and_then(|value| value.role.as_ref()).map(|role| role.name.as_str()).unwrap_or("none");
    let mut launch_facts = format!("\n- Workspace: {workspace}\n- Role: {role}\n- Initial working directory: {}", agentx_ctx.cwd.display());
    if let Some(title) = crate::agents::launch_title::from_context(snapshot.record.as_ref().and_then(|record| record.launch_context.as_deref())) {
        launch_facts.push_str(&format!("\n- Launch title: {title}"));
    }
    BootstrapContext {
        instance_name: instance_name.to_string(),
        display_name: instance_name.to_string(),
        launch_facts,
        relay_enabled,
        agentx_cmd: crate::runtime::env::build_agentx_command(),
        is_headless: agentx_ctx.is_background,
        active_instances: snapshot.active_instances,
        name_shape: snapshot.name_shape,
        scripts: get_scripts(&agentx_ctx.agentx_dir),
        launched_by: launched_by_line(agentx_ctx.launched_by.as_deref(), instance_name),
        notes: agentx_ctx.notes.clone(),
    }
}

/// Apply string substitutions on template text.
/// Replaces {key} patterns with context values.
fn render_template(template: &str, ctx: &BootstrapContext) -> String {
    template
        .replace("{role_workspace_guide}", ROLE_WORKSPACE_GUIDE)
        .replace("{display_name}", &ctx.display_name)
        .replace("{instance_name}", &ctx.instance_name)
        .replace("{SENDER}", SENDER)
        .replace("{agentx_cmd}", &ctx.agentx_cmd)
        .replace("{active_instances}", &ctx.active_instances)
        .replace("{name_shape}", &ctx.name_shape)
        .replace("{scripts}", &ctx.scripts)
        .replace("{launched_by}", &ctx.launched_by)
        .replace("{target_name_s}", &recipient_token("name(s)"))
        .replace("{target_luna}", &recipient_token("luna"))
        .replace("{target_nova}", &recipient_token("nova"))
}

static AGENTX_WORD: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\bagentx\b").expect("valid regex"));

/// Rewrite bare `agentx` command references to the alternate command (e.g.
/// a dev binary), leaving `<agentx>` tags untouched.
fn rewrite_agentx_command(text: &str, agentx_cmd: &str) -> String {
    const COMMAND: &str = "__AGENTX_CMD__";
    const OPEN_TAG: &str = "__AGENTX_OPEN_TAG__";
    const CLOSE_TAG: &str = "__AGENTX_CLOSE_TAG__";
    let protected = text.replace(agentx_cmd, COMMAND).replace("<agentx>", OPEN_TAG).replace("</agentx>", CLOSE_TAG);
    AGENTX_WORD.replace_all(&protected, agentx_cmd).replace(COMMAND, agentx_cmd).replace(OPEN_TAG, "<agentx>").replace(CLOSE_TAG, "</agentx>")
}

/// Extra delivery guidance for tools whose hooks wake the agent with a bare
/// `<agentx>` prompt. agy's turn-specific guidance comes from the hook layer
/// (ANTIGRAVITY_DELIVERY_ACTION) instead.
fn tool_delivery_section(tool: Tool) -> Option<&'static str> {
    match tool {
        Tool::Cursor => Some(CURSOR_DELIVERY),
        Tool::Copilot => Some(COPILOT_DELIVERY),
        _ => None,
    }
}

// PUBLIC API

/// Build bootstrap text for an instance.
///
/// `agentx_ctx` is the environment of the agent being bootstrapped (hook or
/// `agentx start` process, or the launch env for codex). `tool` is the
/// canonical integration name, or "adhoc".
#[cfg(test)]
pub fn get_bootstrap(db: &AgentxDb, agentx_ctx: &AgentxContext, instance_name: &str, tool: &str) -> String {
    let config = crate::config::AgentxConfig::load_for_scope(&agentx_ctx.agentx_dir, &agentx_ctx.raw_env);
    let excluded = parse_excluded_parts(&config.bootstrap_exclude).unwrap_or_default();
    render_bootstrap_with_exclusions(db, agentx_ctx, instance_name, tool, crate::relay::is_relay_enabled(&config), &excluded)
}

/// Product clients render against one daemon projection and their own captured
/// cwd/environment/data scope. No managed database is opened by the renderer.
pub(crate) fn get_bootstrap_client(client: &crate::service::client::Client, agentx_ctx: &AgentxContext, instance_name: &str, tool: &str) -> anyhow::Result<String> {
    get_context_client(client, agentx_ctx, instance_name, tool, super::agent_context::ContextReason::Startup)
}

/// One typed entry to native prompt state; no host-side managed storage fallback.
pub(crate) fn context_state(client: &crate::service::client::Client, request: crate::agents::context_service::Request) -> anyhow::Result<crate::agents::context_service::Response> {
    use crate::agents::{lifecycle_service, service};
    match client.call(crate::service::Request::Participants(service::Request::Lifecycle(lifecycle_service::Request::Context(request))))? {
        crate::service::Response::Participants(service::Response::Lifecycle(lifecycle_service::Response::Context(response))) => Ok(response),
        _ => anyhow::bail!("unexpected context state response"),
    }
}

fn context_actor(client: &crate::service::client::Client, name: &str) -> anyhow::Result<crate::agents::service::ResolvedActor> {
    use crate::agents::service;
    match client.call(crate::service::Request::Participants(service::Request::Resolve))? {
        crate::service::Response::Participants(service::Response::Actor(actor)) if actor.participant.canonical_name == name => Ok(actor),
        _ => anyhow::bail!("context participant differs from the current attachment"),
    }
}

fn context_config(ctx: &AgentxContext, instructions: bool) -> anyhow::Result<crate::config::AgentxConfig> {
    let (mut config, errors) = crate::config::AgentxConfig::load_for_scope_with_errors(&ctx.agentx_dir, &ctx.raw_env);
    for field in ["context_sources", "instruction_sources", "config.toml"] {
        if field == "instruction_sources" && !instructions {
            if errors.contains_key(field) {
                config.instruction_sources = Some(String::new());
            }
            continue;
        }
        if let Some(error) = errors.get(field) {
            anyhow::bail!("{error}");
        }
    }
    Ok(config)
}

struct BuiltinContext<'a> {
    context: BootstrapContext,
    host: &'a AgentxContext,
    excluded: std::collections::HashSet<&'static str>,
}
impl BuiltinContext<'_> {
    fn render(&self, tool: &str) -> String {
        render_bootstrap_context(self.context.clone(), self.host, tool, &self.excluded)
    }
}
impl super::agent_context::InstructionContributor for BuiltinContext<'_> {
    fn id(&self) -> &str {
        "agentx"
    }
    fn instructions(&self, request: &super::agent_context::InstructionRequest<'_>) -> anyhow::Result<String> {
        let mut fixed = self.context.clone();
        // A startup roster is not a permanent instruction. Query current peers
        // with agentx list, or select the agentx-runtime snapshot contributor.
        fixed.active_instances.clear();
        Ok(render_bootstrap_context(fixed, self.host, request.tool, &self.excluded))
    }
}
impl super::agent_context::ContextContributor for BuiltinContext<'_> {
    fn id(&self) -> &str {
        "agentx"
    }
    fn snapshot(&self, request: &super::agent_context::SnapshotRequest<'_>) -> anyhow::Result<String> {
        Ok(self.render(request.identity.tool))
    }
}
struct RuntimeFacts<'a>(&'a BuiltinContext<'a>);
impl super::agent_context::ContextContributor for RuntimeFacts<'_> {
    fn id(&self) -> &str {
        "agentx-runtime"
    }
    fn snapshot(&self, _: &super::agent_context::SnapshotRequest<'_>) -> anyhow::Result<String> {
        Ok(self.0.context.active_instances.clone())
    }
}
fn builtin_context<'a>(client: &crate::service::client::Client, ctx: &'a AgentxContext, name: &str, config: &crate::config::AgentxConfig) -> anyhow::Result<BuiltinContext<'a>> {
    use super::hosts::{HostService, Request, Response};
    let snapshot = match HostService::call(client, Request::BootstrapSnapshot { name: name.to_owned() })? {
        Response::BootstrapSnapshot(snapshot) => snapshot,
        _ => anyhow::bail!("unexpected bootstrap snapshot response"),
    };
    let selected = parse_excluded_parts(&config.bootstrap_exclude).map_err(anyhow::Error::msg)?;
    let excluded = EXCLUDE_PARTS.iter().copied().filter(|part| selected.contains(part)).collect();
    Ok(BuiltinContext { context: build_context_from_snapshot(snapshot, ctx, name, crate::relay::is_relay_enabled(config)), host: ctx, excluded })
}

/// Read-only CLI preview; it never creates an instruction snapshot or claims delivery.
pub(crate) fn preview_context_client(client: &crate::service::client::Client, ctx: &AgentxContext, name: &str, tool: &str) -> anyhow::Result<String> {
    let config = context_config(ctx, true)?;
    Ok(builtin_context(client, ctx, name, &config)?.render(tool))
}

/// Frozen on first generation. Later file/config changes cannot rewrite this Agent's instructions.
pub(crate) fn get_instructions_client(client: &crate::service::client::Client, ctx: &AgentxContext, name: &str, tool: &str) -> anyhow::Result<String> {
    use crate::agents::context_service::{Request, Response};
    let actor = context_actor(client, name)?;
    if let Response::Instructions(Some(instructions)) = context_state(client, Request::ReadInstructions { participant: actor.participant.clone() })? {
        return Ok(instructions.text);
    }
    let config = context_config(ctx, true)?;
    let (mut sources, _) = super::agent_context::select_sources(config.instruction_sources.as_deref(), &config.context_sources)?;
    if let Some(collaboration) = crate::agents::workspaces::collaboration(&actor.record) {
        for (id, text) in [("agentx-role", collaboration.role.map(|role| role.prompt)), ("agentx-additions", collaboration.instruction_additions)] {
            let Some(text) = text else { continue };
            let mut source_id = id.to_owned();
            while sources.iter().any(|source| source.id == source_id) {
                source_id.push('_');
            }
            sources.push(super::agent_context::ContextSource { id: source_id, builtin: None, text: Some(text), file: None });
        }
    }
    let builtin = builtin_context(client, ctx, name, &config)?;
    let request = super::agent_context::InstructionRequest { participant: &actor.participant, tool, cwd: &ctx.cwd };
    let instructions = super::agent_context::prepare_instructions(&request, &sources, &[&builtin])?;
    match context_state(client, Request::FreezeInstructions { participant: actor.participant, instructions })? {
        Response::Instructions(Some(instructions)) => Ok(instructions.text),
        _ => anyhow::bail!("instruction snapshot was not retained"),
    }
}

pub(crate) fn get_initial_context_client(client: &crate::service::client::Client, ctx: &AgentxContext, name: &str, tool: &str) -> anyhow::Result<String> {
    get_runtime_context(client, ctx, name, tool, super::agent_context::SnapshotReason::Initial)
}
pub(crate) fn get_compact_context_client(client: &crate::service::client::Client, ctx: &AgentxContext, name: &str, tool: &str) -> anyhow::Result<String> {
    get_runtime_context(client, ctx, name, tool, super::agent_context::SnapshotReason::AfterCompact)
}
fn get_runtime_context(client: &crate::service::client::Client, ctx: &AgentxContext, name: &str, tool: &str, reason: super::agent_context::SnapshotReason) -> anyhow::Result<String> {
    use super::agent_context::*;
    let config = context_config(ctx, false)?;
    let sources = if config.instruction_sources.is_some() { if config.context_sources.trim().is_empty() { Vec::new() } else { parse_sources(&config.context_sources)? } } else { select_sources(None, &config.context_sources)?.1 };
    if sources.is_empty() && reason == SnapshotReason::Initial {
        return Ok(String::new());
    }
    let actor = context_actor(client, name)?;
    let session = actor.record.session_id.as_deref().filter(|s| !s.is_empty()).ok_or_else(|| anyhow::anyhow!("runtime context requires a bound native session"))?;
    let builtin = builtin_context(client, ctx, name, &config)?;
    let runtime = RuntimeFacts(&builtin);
    let request = SnapshotRequest { identity: InstructionRequest { participant: &actor.participant, tool, cwd: &ctx.cwd }, session_id: session, reason };
    let prepared = match reason {
        SnapshotReason::Initial => prepare_initial_context(&request, &sources, &[&builtin, &runtime])?,
        SnapshotReason::AfterCompact => prepare_compact_context(&request, &sources, &[&builtin, &runtime])?,
    };
    let text = if sources.is_empty() && reason == SnapshotReason::Initial { String::new() } else { prepared.snapshot_text() };
    if text.len() > MAX_CONTEXT_BYTES {
        anyhow::bail!("rendered runtime context exceeds {MAX_CONTEXT_BYTES} bytes");
    }
    crate::log::log_info("context", "prepared", &format!("participant={name} reason={reason:?} sources={:?}", prepared.blocks.iter().map(|block| &block.source).collect::<Vec<_>>()));
    Ok(text)
}

/// Compatibility adapter for native channels that carry both frozen and current context.
/// Codex uses the separate instruction and runtime entries instead.
pub(crate) fn get_context_client(client: &crate::service::client::Client, ctx: &AgentxContext, name: &str, tool: &str, reason: super::agent_context::ContextReason) -> anyhow::Result<String> {
    let instructions = get_instructions_client(client, ctx, name, tool)?;
    let context = if reason == super::agent_context::ContextReason::Compact { get_compact_context_client(client, ctx, name, tool)? } else { get_initial_context_client(client, ctx, name, tool)? };
    let text = [instructions.as_str(), context.as_str()].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join("\n\n");
    if text.len() > super::agent_context::MAX_CONTEXT_BYTES {
        anyhow::bail!("combined AgentX context exceeds the context limit");
    }
    Ok(text)
}

#[cfg(test)]
fn render_bootstrap(db: &AgentxDb, agentx_ctx: &AgentxContext, instance_name: &str, tool: &str, relay_enabled: bool) -> String {
    render_bootstrap_with_exclusions(db, agentx_ctx, instance_name, tool, relay_enabled, &std::collections::HashSet::new())
}

#[cfg(test)]
fn render_bootstrap_with_exclusions(db: &AgentxDb, agentx_ctx: &AgentxContext, instance_name: &str, tool: &str, relay_enabled: bool, excluded: &std::collections::HashSet<&str>) -> String {
    let ctx = build_context(db, agentx_ctx, instance_name, relay_enabled);
    render_bootstrap_context(ctx, agentx_ctx, tool, excluded)
}

fn render_bootstrap_context(ctx: BootstrapContext, agentx_ctx: &AgentxContext, tool: &str, excluded: &std::collections::HashSet<&str>) -> String {
    let tool = tool.parse::<Tool>().unwrap_or(Tool::Adhoc);

    let mut parts: Vec<String> = Vec::new();
    if !excluded.contains("identity") {
        parts.push(IDENTITY_HEAD.to_string());
    }
    if !excluded.contains("messages") {
        parts.push(MESSAGES.to_string());
    }
    if !excluded.contains("capabilities") {
        let handoff = if excluded.contains("handoff") { "" } else { HANDOFF_RESULT };
        parts.push(CAPABILITIES.replace("{handoff_result}", handoff));
        if ctx.relay_enabled {
            parts.push(RELAY_NOTICE.to_string());
        }
        if ctx.is_headless {
            parts.push(HEADLESS_NOTICE.to_string());
        }
        if ctx.agentx_cmd != "agentx" {
            parts.push(UVX_CMD_NOTICE.to_string());
        }
    }
    if !excluded.contains("rules") {
        parts.push(RULES.to_string());
    }
    if !excluded.contains("identity") {
        let mut tail = IDENTITY_TAIL.to_string();
        if !excluded.contains("messages") {
            tail.push(' ');
            tail.push_str(LISTEN_OUTPUT_NOTICE);
        }
        parts.push(tail);
    } else if !excluded.contains("messages") {
        parts.push(LISTEN_OUTPUT_NOTICE.to_string());
    }

    // Every integration delivers automatically when launched through agentx;
    // anything else (plain `agentx start`) has to poll with `agentx listen`.
    if !excluded.contains("messages") {
        if agentx_ctx.is_launched && tool != Tool::Adhoc {
            parts.push(DELIVERY_AUTO.to_string());
            parts.extend(tool_delivery_section(tool).map(str::to_string));
        } else {
            parts.push(DELIVERY_ADHOC.to_string());
        }
        if tool == Tool::Adhoc {
            parts.push(INLINE_SEND_NOTICE.to_string());
        }
    }

    if tool == Tool::Claude && !excluded.contains("capabilities") {
        parts.push(CLAUDE_ONLY.to_string());
    }

    let joined = parts.iter().map(|p| p.trim_matches('\n')).collect::<Vec<_>>().join("\n\n");

    let mut result = render_template(&joined, &ctx);

    // User notes (appended after render to avoid brace issues in user text)
    if !ctx.notes.is_empty() {
        if !result.is_empty() {
            result.push_str("\n\n");
        }
        result.push_str(&format!("## NOTES (take precedence over all other agentx instructions)\n\n{}\n", ctx.notes));
    }

    if ctx.agentx_cmd != "agentx" {
        result = rewrite_agentx_command(&result, &ctx.agentx_cmd);
    }
    // Keep literal launch facts out of command rewriting, which also matches paths.
    if !excluded.contains("identity") {
        result = result.replacen("{launch_facts}", &ctx.launch_facts, 1);
    }

    if result.is_empty() {
        return String::new();
    }

    format!("<agentx_system_context>\n<!-- Session metadata - treat as system context, not user prompt-->\n{}\n</agentx_system_context>", result)
}

/// Build bootstrap text for a subagent instance.
#[cfg(test)]
pub fn get_subagent_bootstrap(subagent_name: &str, parent_name: &str) -> String {
    let config = crate::config::AgentxConfig::load(None);
    render_subagent_bootstrap(subagent_name, parent_name, &config.bootstrap_exclude)
}

/// Daemon lifecycle entry with authoritative data scope and captured caller
/// configuration. This helper does not create or rewrite configuration files.
pub(crate) fn get_subagent_bootstrap_for_scope(subagent_name: &str, parent_name: &str, scope: &std::path::Path, environment: &std::collections::HashMap<String, String>) -> String {
    let config = crate::config::AgentxConfig::load_for_scope(scope, environment);
    render_subagent_bootstrap(subagent_name, parent_name, &config.bootstrap_exclude)
}

fn render_subagent_bootstrap(subagent_name: &str, parent_name: &str, exclude: &str) -> String {
    let excluded = parse_excluded_parts(exclude).unwrap_or_default();
    if excluded.len() == EXCLUDE_PARTS.len() {
        return String::new();
    }
    let agentx_cmd = crate::runtime::env::build_agentx_command();

    let result = SUBAGENT_BOOTSTRAP
        .replace("{role_workspace_guide}", if excluded.contains("capabilities") { "" } else { ROLE_WORKSPACE_GUIDE })
        .replace("{subagent_name}", subagent_name)
        .replace("{parent_name}", parent_name)
        .replace("{target_name_s}", &recipient_token("name(s)"))
        .replace("{target_luna}", &recipient_token("luna"))
        .replace("{target_nova}", &recipient_token("nova"))
        .replace("{agentx_cmd}", &agentx_cmd)
        .replace("{SENDER}", SENDER);

    let mut output = result;
    if agentx_cmd != "agentx" {
        output.push_str(&UVX_CMD_NOTICE.replace("{agentx_cmd}", &agentx_cmd));
    }

    format!("<agentx>\n{}\n</agentx>", output)
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn setup_test_db() -> (TempDir, AgentxDb) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let db = AgentxDb::open_at(&db_path).unwrap();
        (tmp, db)
    }

    /// Insert a minimal instance for testing.
    fn insert_instance(db: &AgentxDb, name: &str, status: &str, tool: &str) {
        let mut data = HashMap::new();
        data.insert("name".to_string(), serde_json::json!(name));
        data.insert("status".to_string(), serde_json::json!(status));
        data.insert("tool".to_string(), serde_json::json!(tool));
        data.insert("status_time".to_string(), serde_json::json!(crate::shared::time::now_epoch_i64() as u64));
        data.insert("created_at".to_string(), serde_json::json!(1000.0));
        db.save_instance(&data).unwrap();
    }

    #[test]
    fn test_get_scripts_bundled_only() {
        let tmp = TempDir::new().unwrap();
        let result = get_scripts(tmp.path());
        // Should list all bundled scripts
        assert!(result.starts_with("Scripts: "));
        assert!(result.contains("confess"));
        assert!(result.contains("debate"));
        assert!(result.contains("fatcow"));
    }

    #[test]
    fn test_get_scripts_with_user_scripts() {
        let tmp = TempDir::new().unwrap();
        let scripts = tmp.path().join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(scripts.join("custom.sh"), "#!/bin/bash").unwrap();
        fs::write(scripts.join("_hidden.py"), "# skip").unwrap();
        fs::write(scripts.join("other.py"), "# include").unwrap();

        let result = get_scripts(tmp.path());
        assert!(result.contains("custom"));
        assert!(result.contains("other"));
        assert!(!result.contains("_hidden"));
    }

    #[test]
    fn test_get_active_instances_empty_db() {
        let (_tmp, db) = setup_test_db();
        let result = get_active_instances(&db, "test");
        assert_eq!(result, "");
    }

    #[test]
    fn test_get_active_instances_with_instances() {
        let (_tmp, db) = setup_test_db();
        insert_instance(&db, "luna", "active", "claude");

        let result = get_active_instances(&db, "other");
        assert!(result.contains("luna"));
        assert!(result.contains("Active (snapshot)"));
    }

    #[test]
    fn test_get_active_instances_excludes_self() {
        let (_tmp, db) = setup_test_db();
        insert_instance(&db, "luna", "active", "claude");

        let result = get_active_instances(&db, "luna");
        assert_eq!(result, "");
    }

    #[test]
    fn test_get_active_instances_grouped_by_tool() {
        let (_tmp, db) = setup_test_db();
        insert_instance(&db, "luna", "active", "claude");
        insert_instance(&db, "nova", "active", "claude");
        insert_instance(&db, "kira", "active", "codex");

        let result = get_active_instances(&db, "other");
        assert!(result.contains("claude: "));
        assert!(result.contains("codex: "));
        assert!(result.contains("luna"));
        assert!(result.contains("nova"));
        assert!(result.contains("kira"));
    }

    #[test]
    fn test_get_active_instances_skips_subagents_and_inactive() {
        let (_tmp, db) = setup_test_db();
        insert_instance(&db, "luna", "active", "claude");
        insert_instance(&db, "luna_task_1", "active", "claude");
        db.conn().execute("UPDATE instances SET agent_id = 'agent-1' WHERE name = 'luna_task_1'", []).unwrap();
        insert_instance(&db, "gone", "inactive", "codex");

        let result = get_active_instances(&db, "other");
        assert!(result.contains("luna"));
        assert!(!result.contains("luna_task_1"));
        assert!(!result.contains("gone"));
    }

    #[test]
    fn test_get_active_instances_reports_overflow() {
        let (_tmp, db) = setup_test_db();
        for i in 0..ACTIVE_SNAPSHOT_LIMIT + 2 {
            insert_instance(&db, &format!("ag{i:02}"), "active", "claude");
        }

        let result = get_active_instances(&db, "other");
        assert!(result.ends_with(" (+2 more: agentx list)"), "{result}");
    }

    /// Context of the agent being bootstrapped, as its hook/start process sees it.
    fn test_ctx(agentx_dir: &std::path::Path, launched: bool, background: Option<&str>, notes: &str, launched_by: Option<&str>) -> AgentxContext {
        let mut env = HashMap::from([("AGENTX_DIR".to_string(), agentx_dir.to_string_lossy().into_owned())]);
        if launched {
            env.insert("AGENTX_LAUNCHED".into(), "1".into());
        }
        if let Some(bg) = background {
            env.insert("AGENTX_BACKGROUND".into(), bg.into());
        }
        if !notes.is_empty() {
            env.insert("AGENTX_NOTES".into(), notes.into());
        }
        if let Some(by) = launched_by {
            env.insert("AGENTX_LAUNCHED_BY".into(), by.into());
        }
        AgentxContext::from_env(&env, agentx_dir.to_path_buf())
    }

    /// Render for a launched, foreground agent named luna.
    fn render(db: &AgentxDb, agentx_dir: &std::path::Path, tool: &str) -> String {
        render_bootstrap(db, &test_ctx(agentx_dir, true, None, "", None), "luna", tool, false)
    }

    #[test]
    fn test_get_bootstrap_claude() {
        let (tmp, db) = setup_test_db();
        let result = render(&db, tmp.path(), "claude");

        assert!(result.starts_with("<agentx_system_context>"));
        assert!(result.contains("Your name: luna"));
        assert!(result.contains("--iam luna"));
        assert!(!result.contains("--name"));
        assert!(result.contains("identifies you automatically"));
        assert!(result.contains("SUBAGENTS"));
        assert!(!result.contains("Headless mode"));
        assert!(!result.contains('{'), "unrendered placeholder: {result}");
        assert!(result.ends_with("</agentx_system_context>"));
    }

    #[test]
    fn inline_send_notice_only_for_inline_receivers() {
        let (tmp, db) = setup_test_db();
        assert!(render(&db, tmp.path(), "adhoc").contains(INLINE_SEND_NOTICE));
        for tool in ["claude", "codex", "gemini", "cursor", "copilot", "qoder", "antigravity", "grok", "kimi", "pi", "omp", "opencode", "kilo"] {
            assert!(!render(&db, tmp.path(), tool).contains(INLINE_SEND_NOTICE), "{tool}");
        }
    }

    /// Every integration launched through agentx gets automatic delivery; the
    /// same tool without a launch, or a plain `agentx start`, polls.
    #[test]
    fn test_delivery_section_follows_launch_state() {
        let (tmp, db) = setup_test_db();
        for spec in crate::integrations::registry::ALL {
            let launched = render(&db, tmp.path(), spec.name);
            let unlaunched = render_bootstrap(&db, &test_ctx(tmp.path(), false, None, "", None), "luna", spec.name, false);
            assert!(unlaunched.contains("Messages do NOT arrive automatically"), "{}", spec.name);
            let auto = launched.contains("Messages instantly and automatically arrive");
            assert_eq!(auto, spec.tool != Tool::Adhoc, "{}", spec.name);
            assert_eq!(launched.contains("SUBAGENTS"), spec.tool == Tool::Claude, "{}", spec.name);
        }

        let cursor = render(&db, tmp.path(), "cursor");
        assert!(cursor.contains("CURSOR DELIVERY"));
        let copilot = render(&db, tmp.path(), "copilot");
        assert!(copilot.contains("COPILOT DELIVERY"));
        // Qoder's hooks attach the message to the wake prompt itself, so it
        // must not be told to ignore a bare `<agentx>` prompt.
        let qoder = render(&db, tmp.path(), "qoder");
        assert!(!qoder.contains("wake trigger"));
    }

    #[test]
    fn test_get_bootstrap_with_relay() {
        let (tmp, db) = setup_test_db();
        let ctx = test_ctx(tmp.path(), true, None, "", None);
        assert!(render_bootstrap(&db, &ctx, "luna", "claude", true).contains("Remote agents have suffix"));
        assert!(!render_bootstrap(&db, &ctx, "luna", "claude", false).contains("Remote agents"));
    }

    #[test]
    fn test_get_bootstrap_background_is_headless() {
        let (tmp, db) = setup_test_db();
        let ctx = test_ctx(tmp.path(), true, Some("agent.log"), "", None);
        assert!(render_bootstrap(&db, &ctx, "luna", "claude", false).contains("Headless mode"));
    }

    #[test]
    fn test_get_bootstrap_with_notes() {
        let (tmp, db) = setup_test_db();
        let ctx = test_ctx(tmp.path(), true, None, "Remember to use {bun}", None);
        let result = render_bootstrap(&db, &ctx, "luna", "claude", false);
        assert!(result.contains("## NOTES (take precedence over all other agentx instructions)\n\nRemember to use {bun}"));
    }

    #[test]
    fn excluded_parts_keep_spawn_example_and_notes() {
        let (tmp, db) = setup_test_db();
        let ctx = test_ctx(tmp.path(), true, None, "Team instructions", None);
        let handoff = parse_excluded_parts("handoff").unwrap();
        let result = render_bootstrap_with_exclusions(&db, &ctx, "luna", "claude", false, &handoff);
        assert!(result.contains(r#"Example: `agentx launch --go --headless --title "Implement xxx" --bind ".controller/xx/xx.task.md" --agentx-prompt "Implement xxx."`"#));
        assert!(!result.contains(HANDOFF_RESULT));
        assert!(result.contains("## NOTES (take precedence over all other agentx instructions)"));

        let all = parse_excluded_parts("all").unwrap();
        let result = render_bootstrap_with_exclusions(&db, &ctx, "luna", "claude", false, &all);
        assert!(!result.contains("## CAPABILITIES"));
        assert!(!result.contains("## MESSAGES"));
        assert!(!result.contains("## RULES"));
        assert!(result.contains("Team instructions"));

        let empty_ctx = test_ctx(tmp.path(), true, None, "", None);
        assert!(render_bootstrap_with_exclusions(&db, &empty_ctx, "luna", "claude", false, &all).is_empty());
    }

    #[test]
    fn conditional_notices_follow_capabilities_exclusion() {
        let (tmp, db) = setup_test_db();
        insert_instance(&db, "luna", "active", "claude");
        let ctx = test_ctx(tmp.path(), true, Some("bg.log"), "", None);
        let identity = parse_excluded_parts("identity").unwrap();
        let result = render_bootstrap_with_exclusions(&db, &ctx, "luna", "claude", true, &identity);
        assert!(result.contains("Remote agents have suffix"));
        assert!(result.contains("Headless mode"));

        let capabilities = parse_excluded_parts("capabilities").unwrap();
        let result = render_bootstrap_with_exclusions(&db, &ctx, "luna", "claude", true, &capabilities);
        assert!(!result.contains("Remote agents have suffix"));
        assert!(!result.contains("Headless mode"));
        assert!(result.contains("Your name: luna"));
    }

    #[test]
    fn excluded_parts_parse_exact_names() {
        assert!(parse_excluded_parts("none").unwrap().is_empty());
        assert_eq!(parse_excluded_parts("all").unwrap().len(), EXCLUDE_PARTS.len());
        assert!(parse_excluded_parts("rules,handoff").unwrap().contains("handoff"));
        assert!(parse_excluded_parts("rules,rules").is_err());
        assert!(parse_excluded_parts("rule").is_err());
    }

    #[test]
    fn test_get_subagent_bootstrap() {
        let result = get_subagent_bootstrap("luna_reviewer_1", "luna");

        assert!(result.contains("<agentx>"));
        assert!(result.contains("Your name: luna_reviewer_1"));
        assert!(result.contains("Your parent: luna"));
        assert!(result.contains("--iam luna_reviewer_1"));
        assert!(!result.contains("--name"));
        assert!(result.contains("identity automatically"));
        assert!(result.contains(SENDER));
        assert!(result.contains("</agentx>"));
        if cfg!(windows) {
            assert!(result.contains("send '@name(s)'"));
        } else {
            assert!(result.contains("send @name(s)"));
        }
    }

    #[test]
    fn all_excludes_subagent_bootstrap() {
        assert!(render_subagent_bootstrap("luna_reviewer_1", "luna", "all").is_empty());
        assert!(!render_subagent_bootstrap("luna_reviewer_1", "luna", "rules").is_empty());
    }

    fn bootstrap_launched_by(launched_by: Option<&str>) -> String {
        let (tmp, db) = setup_test_db();
        let ctx = test_ctx(tmp.path(), true, None, "", launched_by);
        render_bootstrap(&db, &ctx, "luna", "claude", false)
    }

    #[test]
    fn test_get_bootstrap_shows_launching_agent() {
        let result = bootstrap_launched_by(Some("nova"));
        if cfg!(windows) {
            assert!(result.contains("- Launched by: '@nova'"));
        } else {
            assert!(result.contains("- Launched by: @nova"));
        }
    }

    #[test]
    fn bootstrap_shows_adopted_workspace_role_and_launch_directory() {
        let (tmp, db) = setup_test_db();
        insert_instance(&db, "luna", "active", "claude");
        let launch_context = serde_json::json!({
            "collaboration": {
                "workspace": "/projects/agentx",
                "role": {"name":"coder", "path":"/roles/coder.md", "namespace":"local:/roles", "profile":"codex", "prompt":"Implement carefully.", "description":null, "default":false},
                "endpoint": false,
                "instruction_additions": null
            }
        });
        db.conn().execute("UPDATE instances SET launch_context=?1 WHERE name='luna'", [launch_context.to_string()]).unwrap();
        let mut ctx = test_ctx(tmp.path(), true, None, "", Some("nova"));
        ctx.cwd = std::path::PathBuf::from("/projects/agentx/crate");

        let result = render_bootstrap(&db, &ctx, "luna", "claude", false);
        assert!(result.contains("- Workspace: /projects/agentx"));
        assert!(result.contains("- Role: coder"));
        assert!(result.contains("- Initial working directory: /projects/agentx/crate"));
        assert!(!result.contains("list self workspace"));
    }

    #[test]
    fn test_get_bootstrap_omits_non_agent_launcher() {
        for launcher in [None, Some("api"), Some("user"), Some(SENDER), Some("luna")] {
            let result = bootstrap_launched_by(launcher);
            assert!(!result.contains("Launched by"), "launcher={launcher:?}");
            assert!(!result.contains("{launched_by}"));
        }
    }

    #[test]
    fn test_bootstrap_quotes_send_recipients_on_windows() {
        let (tmp, db) = setup_test_db();
        let result = render(&db, tmp.path(), "claude");

        if cfg!(windows) {
            assert!(result.contains("send '@name(s)'"));
            assert!(result.contains("send '@luna' '@nova'"));
        } else {
            assert!(result.contains("send @name(s)"));
            assert!(result.contains("send @luna @nova"));
        }
    }

    #[test]
    fn test_rewrite_agentx_command_keeps_tags() {
        let text = "run `agentx list`, <agentx>x</agentx>, already /dev/agentx send";
        assert_eq!(rewrite_agentx_command(text, "/dev/agentx"), "run `/dev/agentx list`, <agentx>x</agentx>, already /dev/agentx send");
    }

    #[test]
    fn test_antigravity_delivery_action_guards_against_ack_only_stall() {
        // The per-turn preamble must tell agy that an ACK alone doesn't finish a
        // request and that no turn is auto-created to resume after it goes idle.
        assert!(ANTIGRAVITY_DELIVERY_ACTION.contains("AGENTX MESSAGE"));
        assert!(ANTIGRAVITY_DELIVERY_ACTION.contains("ACK alone does not complete"));
        assert!(ANTIGRAVITY_DELIVERY_ACTION.contains("no turn is auto-created"));
    }

    /// Catch drift between scripts::SCRIPTS const and actual files in scripts/bundled/.
    #[test]
    fn test_bundled_scripts_matches_directory() {
        use crate::workflows as scripts;

        // Resolve the bundled scripts directory relative to the crate root.
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let bundled_dir = std::path::Path::new(manifest_dir).join("src/workflows/bundled");

        if !bundled_dir.exists() {
            // In CI or worktrees, the scripts source may not be present — skip gracefully.
            return;
        }

        let mut actual: Vec<String> = Vec::new();
        for entry in fs::read_dir(&bundled_dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".sh") && !name.starts_with('_') {
                actual.push(name.trim_end_matches(".sh").to_string());
            }
        }
        actual.sort();

        let mut expected: Vec<String> = scripts::SCRIPTS.iter().map(|(name, _)| name.to_string()).collect();
        expected.sort();

        assert_eq!(
            expected, actual,
            "scripts::SCRIPTS const is out of sync with scripts/bundled/. \
             Expected: {:?}, Actual: {:?}",
            expected, actual
        );
    }
}
