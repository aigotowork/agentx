//! Native help text for all agentx commands.
//!
//! Each command has a list of (usage, description) entries formatted by `get_command_help()`.

use std::env;

mod all;
pub(crate) use all::dispatch as dispatch_all;
pub(crate) use all::is_requested as all_requested;

/// Help entry: (usage, description). See `format_entry()` for rendering rules.
type HelpEntry = (&'static str, &'static str);

// ── Shared filter help (events, listen, events sub) ─────────────────────

const FILTER_HELP: &[HelpEntry] = &[
    ("  --agent NAME", "Agent name"),
    ("  --type TYPE", "message | status | life"),
    ("  --status VAL", "active | listening | blocked | inactive | launching | error"),
    ("  --context PATTERN", "tool:Bash | deliver:X (supports * wildcard)"),
    ("  --action VAL", "created | started | ready | stopped | batch_launched | launch_failed | launch_blocked"),
    ("  --cmd PATTERN", "Shell command (contains, ^prefix, =exact)"),
    ("  --file PATH", "File write (*.py for glob, file.py for contains)"),
    ("  --collision", "Two agents edit same file within 30s"),
    ("  --from NAME", "Sender"),
    ("  --participant NAME", "Message sender or delivery recipient"),
    ("  --mention NAME", "@mention target"),
    ("  --intent VAL", "request | inform | ack"),
    ("  --thread NAME", "Thread name"),
    ("  --reply-to ID", "Message replying to this event"),
    ("  --after TIME", "After timestamp (ISO-8601)"),
    ("  --before TIME", "Before timestamp (ISO-8601)"),
];

// ── Per-command help registries ─────────────────────────────────────────
//
// A command page is a list of sections; a subcommand page (`SUBCOMMAND_HELP`)
// reuses the same sections, so `agentx X --help` and `agentx X sub --help` can't
// drift apart.

const FILTERS_HEADER: &[HelpEntry] = &[("Filters (same flag repeated = OR, different flags = AND):", "")];

const EVENTS_QUERY: &[HelpEntry] = &[
    ("", "Query the event stream (messages, status changes, file edits, lifecycle)"),
    ("Query:", ""),
    ("  events", "Last 20 events as JSON Lines (one object per line)"),
    ("  --last N", "Limit count (default: 20)"),
    ("  --all", "Include archived sessions"),
    ("  --full", "Full event JSON (default: streamlined)"),
    ("  --wait [SEC]", "Block until match (default: 60s)"),
    ("  --sql EXPR", "Raw SQL WHERE (ANDed with flags)"),
    ("  --remote-fetch --device ID", "One-shot fetch from remote device (with --full: whole status details)"),
    ("", "Remote agents' details in list and TUI are previews. Full text:"),
    ("", "  events --remote-fetch --device ID --agent NAME --type status --last 1 --full"),
    ("", "A trailing {\"truncated\": true} line means the answer hit the relay size limit"),
];

const EVENTS_FOLLOW: &[HelpEntry] = &[
    ("Follow (streams until Ctrl-C or the reader exits):", ""),
    ("events --follow [filters]", "New matching events as JSON Lines, oldest first"),
    ("  --last N", "Replay the N most recent matches first"),
    ("  --after-id ID", "Resume after a saved event id (replays every match since)"),
    ("", "A {\"db_reset\": true} line means agentx reset ran and ids restarted"),
];

const EVENTS_SHORTCUTS: &[HelpEntry] = &[("Shortcuts:", ""), ("  --idle NAME", "--agent NAME --status listening"), ("  --blocked NAME", "--agent NAME --status blocked")];

const EVENTS_LAUNCH: &[HelpEntry] = &[
    ("Wait for a launch batch:", ""),
    ("events launch [batch_id]", "Block until batch reaches a terminal state (JSON LaunchResult)"),
    ("  --timeout SEC", "Max seconds to wait (default: 30)"),
    ("", "Exit codes: 0 ready, 1 error/no_launches, 2 timeout/blocked"),
];

const EVENTS_SUB: &[HelpEntry] = &[
    ("Subscribe (next matching event arrives as a message from [agentx-events]):", ""),
    ("events sub [filters]", "Subscribe using filter flags"),
    ("events sub \"SQL WHERE\"", "Subscribe using raw SQL"),
    ("  --once", "Auto-remove after first match"),
    ("  --for <name>", "Subscribe on behalf of another agent"),
    ("  --on-hit <text>", "Also send this message each time it fires (@mentions ok)"),
    ("  --as <name> / -b", "Own the sub without an agent identity, e.g. from a shell (-b = bigboss)"),
    ("  --device ID", "Install/list on a remote device (create requires --for, --as, or -b)"),
];

const EVENTS_REMINDER: &[HelpEntry] = &[
    ("events sub --in DURATION --message TEXT", "Schedule one local reminder; no event filters"),
    ("  --in DURATION", "Relative delay from 1s to 365d (s, m, h, d)"),
    ("  --message TEXT", "Message to enqueue when due"),
    ("  --for NAME", "Select another local Agent; defaults to the current Agent"),
];
const EVENTS_SUB_LIST: &[HelpEntry] =
    &[("events sub list", "List active subscriptions and pending local reminders"), ("  --history", "Include enqueued, failed and canceled local reminders"), ("  --device ID", "List event subscriptions on a remote device")];
const EVENTS_SUB_SHOW: &[HelpEntry] = &[("events sub show REMINDER_ID", "Show a local reminder and its outcome as JSON")];

const EVENTS_UNSUB: &[HelpEntry] = &[("events unsub <id>", "Remove an event subscription or cancel a pending local reminder"), ("  --device ID", "Remove an event subscription from a remote device")];

const EXAMPLES_HEADER: &[HelpEntry] = &[("Examples:", "")];

const EVENTS_EXAMPLES: &[HelpEntry] = &[("  agentx events --cmd git --agent peso", "")];

const EVENTS_SUB_EXAMPLES: &[HelpEntry] = &[("  agentx events sub --idle peso", "Notified when peso goes idle"), ("  agentx events sub --file '*.py' --once", "One-shot: next .py file write")];

const EVENTS_SQL_REFERENCE: &[HelpEntry] = &[
    ("SQL reference (events_v view):", ""),
    ("  Base", "id, timestamp, type, instance"),
    ("  msg_*", "from, text, scope, sender_kind, delivered_to[], mentions[], intent, thread, reply_to"),
    ("  status_*", "val, context, detail"),
    ("  life_*", "action, by, batch_id, reason"),
    ("", ""),
    ("  type", "message, status, life"),
    ("  msg_scope", "broadcast, mentions"),
    ("  msg_sender_kind", "instance, external, system"),
    ("  status_context", "tool:X, deliver:X, approval, prompt, exit:X"),
    ("  life_action", "created, started, ready, stopped, batch_launched, launch_failed, launch_blocked"),
    ("", ""),
    ("", "delivered_to/mentions are JSON arrays \u{2014} query exact values with json_each(...)"),
    ("", "Use <> instead of != for SQL negation"),
];

const EVENTS_SQL_POINTER: &[HelpEntry] = &[("", ""), ("", "SQL columns: agentx events --help")];

const EVENTS_PAGE: &[&[HelpEntry]] = &[
    EVENTS_QUERY,
    FILTERS_HEADER,
    FILTER_HELP,
    EVENTS_SHORTCUTS,
    EVENTS_FOLLOW,
    EVENTS_LAUNCH,
    EVENTS_SUB,
    EVENTS_REMINDER,
    EVENTS_SUB_LIST,
    EVENTS_SUB_SHOW,
    EVENTS_UNSUB,
    EXAMPLES_HEADER,
    EVENTS_EXAMPLES,
    EVENTS_SUB_EXAMPLES,
    EVENTS_SQL_REFERENCE,
];

const EVENTS_SUB_PAGE: &[&[HelpEntry]] = &[EVENTS_SUB, EVENTS_REMINDER, EVENTS_SUB_LIST, EVENTS_SUB_SHOW, EVENTS_UNSUB, FILTERS_HEADER, FILTER_HELP, EVENTS_SHORTCUTS, EXAMPLES_HEADER, EVENTS_SUB_EXAMPLES, EVENTS_SQL_POINTER];

const LIST_HELP: &[HelpEntry] = &[
    ("labels", "Read and atomically patch Agent labels"),
    ("list", "Current workspace and global agents, read receipts"),
    ("  --dir PATH", "Discover the workspace containing PATH"),
    ("  --workspace PATH|global", "Select an exact workspace or global members"),
    ("  --all", "All workspaces in this AgentX scope"),
    ("  -v", "Verbose (directory, session, etc)"),
    ("  --json", "JSON array of selected agents"),
    ("  --names", "Just names, one per line"),
    ("  --format TPL", "Template per agent: --format '{name} {status}'"),
    ("", "  Fields: name, status, status_context, status_detail,"),
    ("", "  status_age_seconds, description, unread_count, tool, directory,"),
    ("", "  session_id, parent_name, agent_id, headless, created_at,"),
    ("", "  hooks_bound, process_bound, transcript_path, background_log_file,"),
    ("", "  launch_context, workspace, role"),
    ("", ""),
    ("list [self|<name>]", "Single agent details, including recorded session usage"),
    ("  [field]", "Print specific field (status, directory, session_id, usage, ...)"),
    ("  --json", "Output as JSON"),
    ("  --sh", "Shell exports: eval \"$(agentx list self --sh)\""),
    ("", ""),
    ("list --stopped [name]", "Stopped agents (from events)"),
    ("  --all", "All stopped (default: last 20)"),
    ("", ""),
    ("Status icons:", ""),
    ("", "\u{25b6}  active      processing, reads messages very soon"),
    ("", "\u{25c9}  listening   idle, reads messages in <1s"),
    ("", "\u{25a0}  blocked     needs human approval"),
    ("", "\u{25cb}  inactive    dead or stale"),
    ("", "\u{25e6}  unknown     neutral"),
    ("", ""),
    ("Tool labels:", ""),
    ("", "[CLAUDE] [GEMINI] [CODEX] [OPENCODE] [KILO] [PI] [OMP] [ANTIGRAVITY] [CURSOR] [KIMI] [COPILOT] [QODER] [GROK]  agentx-launched, automatic delivery"),
    ("", "[CLAUDE*]   hooks or process not bound (starting up, or lost); see: list <name>"),
    ("", "[AD-HOC]    no hooks, checks messages manually"),
];

const SEND_HELP: &[HelpEntry] = &[
    ("send @name -- message text", "Direct message"),
    ("send @name1 @name2 -- message", "Multiple targets"),
    ("send --dir PATH -- message", "Reuse/start the directory's default role recipient"),
    ("send --workspace PATH -- message", "Address an exact workspace's default recipient"),
    ("send --dir PATH --dry-run", "Inspect default routing without launch or message"),
    ("send -- message text", "Broadcast to all"),
    ("send @name", "Message from stdin (pipe or heredoc)"),
    ("send @name --file <path>", "Message from file"),
    ("send @name --base64 <encoded>", "Message from base64 string"),
    ("send --receipt REQUEST_ID [--json]", "Read the original send receipt by ID without resending"),
    ("send --delivery-status ID [--json]", "Read accepted-message delivery facts without resending or draining mail"),
    ("", ""),
    ("", "Everything after -- is the message (no quotes needed)."),
    ("", "All flags must come before --."),
    ("", ""),
    ("Directory routing:", ""),
    ("  --dir PATH", "Discover the target workspace from PATH"),
    ("  --workspace PATH|global", "Select an exact workspace or the global default"),
    ("", "  Without @recipients, reuse/start one default = true role recipient."),
    ("", "  Search the selected workspace, then parents, then shared roles; no default is an error."),
    ("", "  Parent fallback uses the parent's endpoint and retains the originally requested directory."),
    ("", "  Concurrent first sends share one endpoint; startup-pending messages enter its durable mailbox."),
    ("", "  With @recipients, check their workspace instead; never create a replacement."),
    ("", "  Setup and examples: agentx roles --help"),
    ("", ""),
    ("Target matching:", ""),
    ("  @luna", "exact base name"),
    ("  @luna:BOXE", "exact or uniquely prefixed remote agent"),
    ("", "Partial local names are rejected to avoid accidental fan-out."),
    ("", ""),
    ("Envelope:", ""),
    ("  --intent <type>", "request | inform | ack"),
    ("", "  request: expect a response"),
    ("", "  inform: FYI, no response needed"),
    ("", "  ack: replying to a request (requires --reply-to)"),
    ("  --reply-to <id>", "Link to event ID (42 or 42:BOXE)"),
    ("  --thread <name>", "Threaded routing: seed recipients once, then reuse thread members"),
    ("", "  broadcast + --thread reuses prior thread members; seed with @mentions first"),
    ("", ""),
    ("Sender:", ""),
    ("  --from <name>", "External sender identity"),
    ("  -b", "Shorthand for --from bigboss"),
    ("  --iam <name>", "Caller identity (default: injected process/session binding)"),
    ("", ""),
    ("Output:", ""),
    ("  --json", "Acceptance receipt plus current delivery observation; leaves mail unread"),
    ("  --quiet", "No output; leaves mail unread"),
    ("", "  Acceptance, mailbox confirmation and model execution are separate facts."),
    ("", ""),
    ("Inline bundle (attach structured context):", ""),
    ("  --title <text>", "Create and attach bundle inline"),
    ("  --description <text>", "Bundle description (required with --title)"),
    ("  --events <ids>", "Event IDs/ranges: 1,2,5-10"),
    ("  --files <paths>", "Comma-separated file paths"),
    ("  --transcript <ranges>", "Format: 3-14:normal,6:full,22-30:detailed"),
    ("  --extends <id>", "Parent bundle (optional)"),
    ("", "See 'agentx bundle --help' for bundle details"),
    ("", ""),
    ("Examples:", ""),
    ("  agentx send @luna -- Hello there!", ""),
    ("  agentx send @luna @nova --intent request -- Can you help?", ""),
    ("  agentx send --dir /path/project --intent request -- Handle this task", ""),
    ("  agentx send -- Broadcast message to everyone", ""),
    ("  echo 'Complex message' | agentx send @luna", ""),
    ("  agentx send @luna <<'EOF'", ""),
    ("  Multi-line message with special chars", ""),
    ("  EOF", ""),
];

const BLANK: &[HelpEntry] = &[("", "")];

const BUNDLE_DEFAULT: &[HelpEntry] = &[("bundle", "List recent bundles (same as bundle list)")];

const BUNDLE_LIST: &[HelpEntry] = &[("bundle list", "List recent bundles"), ("  --last N", "Limit count (default: 20)"), ("  --json", "Output JSON")];

const BUNDLE_CAT: &[HelpEntry] = &[("bundle cat <id>", "Expand full bundle content"), ("", "Shows: metadata, files (metadata only), transcript (respects detail level), events")];

const BUNDLE_PREPARE: &[HelpEntry] = &[
    ("bundle prepare", "Show recent context, suggest template"),
    ("  --for <agent>", "Prepare for specific agent (default: self)"),
    ("  --last-transcript N", "Transcript exchanges to suggest (default: 40)"),
    ("  --last-events N", "Events to scan per category (default: 10)"),
    ("  --json", "Output JSON"),
    ("  --compact", "Hide how-to section"),
    ("", "Shows suggested transcript ranges, relevant events, files"),
    ("", "Outputs ready-to-use bundle create command"),
    ("", "TIP: Skip 'bundle create' \u{2014} use bundle flags directly in 'agentx send'"),
];

const BUNDLE_SHOW: &[HelpEntry] = &[("bundle show <id>", "Show bundle by id/prefix"), ("  --json", "Output JSON")];

const BUNDLE_CREATE: &[HelpEntry] = &[
    ("bundle create \"title\"", "Create bundle (positional or --title)"),
    ("  --title <text>", "Bundle title (alternative to positional)"),
    ("  --description <text>", "Bundle description (required)"),
    ("  --events 1,2,5-10", "Event IDs/ranges, comma-separated (required)"),
    ("  --files a.py,b.py", "Comma-separated file paths (required)"),
    ("  --transcript RANGES", "Transcript with detail levels (required)"),
    ("", "    Format: range:detail (3-14:normal,6:full,22-30:detailed)"),
    ("", "    normal = truncated | full = complete text | detailed = tool I/O+edits+errors"),
    ("  --extends <id>", "Parent bundle for chaining"),
    ("  --bundle JSON", "Create from JSON payload (format below)"),
    ("  --bundle-file FILE", "Create from JSON file (format below)"),
    ("  --json", "Output JSON"),
];

const BUNDLE_CHAIN: &[HelpEntry] = &[("bundle chain <id>", "Show bundle lineage"), ("  --json", "Output JSON")];

const BUNDLE_JSON_FORMAT: &[HelpEntry] = &[
    ("JSON format:", ""),
    ("", "{"),
    ("", "  \"title\": \"Bundle Title\","),
    ("", "  \"description\": \"What happened, decisions, state, next steps\","),
    ("", "  \"refs\": {"),
    ("", "    \"events\": [\"123\", \"124-130\"],"),
    ("", "    \"files\": [\"src/auth.py\", \"tests/test_auth.py\"],"),
    ("", "    \"transcript\": [\"10-15:normal\", \"20:full\", \"30-35:detailed\"]"),
    ("", "  },"),
    ("", "  \"extends\": \"bundle:abc123\""),
    ("", "}"),
];

const BUNDLE_PAGE: &[&[HelpEntry]] = &[BUNDLE_DEFAULT, BUNDLE_LIST, BLANK, BUNDLE_CAT, BLANK, BUNDLE_PREPARE, BLANK, BUNDLE_SHOW, BLANK, BUNDLE_CREATE, BLANK, BUNDLE_CHAIN, BUNDLE_JSON_FORMAT];

const STOP_HELP: &[HelpEntry] = &[("stop", "Disconnect self from agentx"), ("stop <name>", "Disconnect specific agent"), ("stop <n1> <n2> ...", "Disconnect multiple"), ("stop all", "Disconnect all agents")];

const START_HELP: &[HelpEntry] = &[
    ("start", "Connect to agentx (from inside any AI session)"),
    ("start --name <name-or-agent-id>", "Register with a name or the subagent ID supplied by SubagentStart"),
    ("start --as <name>", "Reclaim identity (after compaction/resume/clear)"),
    ("", ""),
    ("", ""),
    ("", "Inside a sandbox? Prefix all agentx commands with: AGENTX_DIR=$PWD/.agentx"),
    ("", ""),
    ("", "Tip: Do not truncate the agentx start output, it contains important information."),
];

const LISTEN_HELP: &[HelpEntry] = &[
    ("listen [timeout]", "Block until message arrives"),
    ("  [timeout]", "Timeout in seconds (alias for --timeout)"),
    ("  --timeout N", "Timeout in seconds (default: 86400)"),
    ("  --json", "Output messages as JSON"),
    ("", ""),
    ("Filter flags:", ""),
    ("", "Supports all filter flags from 'events' command"),
    ("", "(--agent, --type, --status, --file, --cmd, --from, --intent, etc.)"),
    ("", "Run 'agentx events --help' for full list"),
    ("", "Filters combine with --sql using AND logic"),
    ("", ""),
    ("SQL filter mode:", ""),
    ("  --sql \"type='message'\"", "Custom SQL against events_v"),
    ("  --sql stopped:name", "Preset: wait for agent to stop"),
    ("  --idle NAME", "Shortcut: wait for agent to go idle"),
    ("", ""),
    ("Exit codes:", ""),
    ("  0", "Message received / event matched"),
    ("  1", "Timeout or error"),
    ("", ""),
    ("", "Quick unread check: agentx listen 1"),
];

const RESET_HELP: &[HelpEntry] = &[
    ("reset", "Archive conversation, clear database"),
    ("reset all", "Stop all + clear db + remove hooks + reset config"),
    ("", ""),
    ("Sandbox / local mode:", ""),
    ("", "If you can't write to ~/.agentx, set:"),
    ("", "  export AGENTX_DIR=\"$PWD/.agentx\""),
    ("", "Hooks install under the parent of AGENTX_DIR; state stays in AGENTX_DIR."),
    ("", "  AGENTX_DIR=$PWD/.agentx -> $PWD/.claude, .gemini, .codex, .opencode, .kilo, .pi, .omp, .antigravity, .cursor, .kimi, .copilot, .grok"),
    ("", ""),
    ("", "To remove local setup:"),
    ("", "  agentx hooks remove && rm -rf \"$AGENTX_DIR\""),
    ("", ""),
    ("", "Explicit location:"),
    ("", "  export AGENTX_DIR=/your/path/.agentx"),
    ("", ""),
];

const CONFIG_HELP: &[HelpEntry] = &[
    ("config", "Show effective config (with sources)"),
    ("config <key>", "Get one key"),
    ("config <key> <value>", "Set one key"),
    ("config <key> --info", "Detailed help for a key"),
    ("  --json / --edit / --reset", "JSON output / open in $EDITOR / archive + restore defaults"),
    ("config environment", "Show selected environment-file sources/provenance without values"),
    ("  --json", "Output source/provenance metadata as JSON"),
    ("config env_files '[\".env\",\".env.local\"]'", "Set ordered dotenv sources; [] disables"),
    ("config env_override true|false", "Let selected files override inherited values (default false)"),
    ("", ""),
    ("Per-agent:", ""),
    ("config -i <name|self> [key] [val]", "timeout, hints, subagent_timeout"),
];

/// Static head of `agentx config --help`; key list and file paths follow.
const CONFIG_PAGE: &[&[HelpEntry]] = &[CONFIG_HELP, BLANK, CONFIG_BOOTSTRAP];

const CONFIG_BOOTSTRAP: &[HelpEntry] =
    &[("Bootstrap (the agentx context agents get):", ""), ("config bootstrap show [part]", "Print each part, or one part"), ("config bootstrap render <name> <tool> [launched]", "Print the full bootstrap an agent would get")];

/// Keys a user sets by hand. Every config field must appear here or in
/// CONFIG_KEYS_NOT_IN_HELP (enforced by `config_help_covers_registry_keys`).
const CONFIG_KEYS_HELP: &[HelpEntry] = &[
    ("", ""),
    ("Keys:", ""),
    ("  terminal", "Where new agent windows open"),
    ("  headless_terminal", "Background host: pty (default) or tmux"),
    ("  tmux_socket / tmux_config", "Scoped tmux socket and optional native config file paths"),
    ("  hints", "Text appended to all messages agent receives"),
    ("  notes", "Notes appended to agent bootstrap"),
    ("  instruction_sources", "Frozen instruction sources as JSON; [] disables"),
    ("  context_sources", "Ordered dynamic context sources as JSON; [] disables"),
    ("  bootstrap_exclude", "Bootstrap parts to leave out (see config bootstrap show)"),
    ("  subagent_timeout", "Subagent keep-alive seconds after task"),
    ("  claude_args / gemini_args / codex_args / opencode_args / kilo_args / pi_args / omp_args / cursor_args / kimi_args / copilot_args / qoder_args / grok_args / hermes_args", "Default launch args per tool"),
    ("  claude_delivery / copilot_delivery", "Idle delivery: native (default; claude: mod, copilot: CLI extension) | inject (PTY)"),
    ("  gemini_system_prompt / codex_system_prompt", "Default system prompt"),
    ("  auto_approve", "Auto-approve safe agentx commands"),
    ("  auto_subscribe", "Event auto-subscribe presets"),
    ("  auto_trust_workspace", "Auto-trust launch dir (skip folder-trust prompt)"),
    ("  name_export", "Export agent name to custom env var"),
    ("  title_mode", "Terminal/tab title: combined, label, or off"),
];

/// Registry keys left out of help: relay keys are managed by `agentx relay`;
/// `timeout` only affects headless/vanilla Claude (see `config timeout --info`).
#[cfg(test)]
const CONFIG_KEYS_NOT_IN_HELP: &[&str] = &["relay", "relay_id", "relay_token", "relay_enabled", "timeout"];

// config help continued with dynamic config files hint
const CONFIG_HELP_2: &[HelpEntry] = &[("", "AGENTX_DIR: isolate per project (see 'agentx reset --help')")];

const RELAY_MAIN: &[HelpEntry] = &[
    ("relay", "Show relay status"),
    ("relay status", "Show relay status"),
    ("relay new", "Create new relay group"),
    ("relay token", "Show join token for other devices"),
    ("relay connect <token>", "Join relay group"),
    ("relay on", "Re-enable sync"),
    ("relay off", "Disable sync"),
    ("  --all", "Also disable on all known peers"),
    ("Setup:", ""),
    ("", "1. 'relay new' to get token"),
    ("", "2. 'relay connect <token>' on each device"),
    ("Custom broker:", ""),
    ("relay new --broker mqtts://host:port --password <broker-auth-secret>", ""),
    ("relay connect <token> --password <broker-auth-secret>", ""),
    ("Daemon:", ""),
];

const RELAY_DAEMON: &[HelpEntry] = &[("relay daemon", "Show daemon status"), ("relay daemon start", "Start the relay daemon"), ("relay daemon stop", "Stop the daemon (SIGKILL after 5s)"), ("relay daemon restart", "Restart the daemon")];

const RELAY_PAGE: &[&[HelpEntry]] = &[RELAY_MAIN, RELAY_DAEMON];

const TRANSCRIPT_HELP: &[HelpEntry] = &[
    ("transcript <name>", "View agent's conversation (last 10)"),
    ("transcript <name> N", "Show exchange N"),
    ("transcript <name> N-M", "Show exchanges N through M (N- = to the end)"),
    ("transcript timeline", "User prompts across all agents by time"),
    ("  --last N", "Show the last N exchanges (default: 10)"),
    ("  --full", "Show complete assistant responses"),
    ("  --detailed", "Show tool I/O, file edits, errors, token usage"),
    ("  --json", "JSON output"),
    ("", ""),
    ("transcript search \"pattern\"", "Search agentx agents' conversations (regex), newest first"),
    ("  --live", "Only currently alive agents"),
    ("  --all", "Raw search of all transcripts on disk (needs rg)"),
    ("  --limit N", "Max matching exchanges shown (default: 20)"),
    ("  --agent TYPE", "Filter: {transcript_agents}"),
    ("  --exclude-self", "Exclude the searching agent's own transcript"),
    ("  --json", "JSON output"),
    ("", ""),
    ("", "Tip: Reference ranges in messages instead of copying:"),
    ("", "\"read my transcript range 7-10 --full\""),
];

const ARCHIVE_HELP: &[HelpEntry] = &[
    ("archive", "List archived sessions (numbered)"),
    ("archive <N>", "Query events from archive (1 = most recent)"),
    ("archive <N> agents", "Query agents from archive"),
    ("archive <name>", "Query by stable name (prefix match)"),
    ("  --here", "Filter to archives from current directory"),
    ("  --sql \"expr\"", "SQL WHERE filter"),
    ("  --last N", "Max archives listed (default: all); events when one is selected (default: 20)"),
    ("  --json", "JSON output"),
];

const RUN_HELP: &[HelpEntry] = &[
    ("run", "List available workflow/launch scripts and more info"),
    ("run <name> [args]", "Execute script"),
    ("run <name> --help", "Script options"),
    ("run docs", "CLI reference + config + script creation guide"),
    ("", ""),
    ("", "Docs sections:"),
    ("  agentx run docs --cli", "CLI reference only"),
    ("  agentx run docs --config", "Config settings only"),
    ("  agentx run docs --scripts", "Script creation guide"),
    ("", ""),
    ("", "User scripts: ~/.agentx/scripts/"),
];

const STATUS_HELP: &[HelpEntry] = &[("status", "Installation status and diagnostics"), ("status --logs", "Include recent errors and warnings"), ("status --json", "Machine-readable output")];

const UPDATE_HELP: &[HelpEntry] = &[
    ("update", "Check for and apply updates"),
    ("update --check", "Only check — print status without applying"),
    ("", ""),
    ("", "Detects install method and runs the right update command:"),
    ("", "  brew install    → brew upgrade agentx"),
    ("", "  uv tool install → uv tool upgrade agentx"),
    ("", "  pip install     → pip install -U agentx"),
    ("", "  curl installer  → re-run agentx-installer.sh"),
];

const HOOKS_STATUS: &[HelpEntry] = &[("hooks", "Show how each tool loads agentx's hooks"), ("hooks status [tool]", "Same as above, for one tool"), ("", "  hermes: also asks Hermes whether the plugin is enabled (slow)")];

const HOOKS_ADD: &[HelpEntry] = &[("hooks add [tool]", "Install persistent hooks ({persistent_tools} | all)"), ("", "  No tool: auto-detect the current tool, or add all outside one."), ("", "  Restart the tool after adding hooks to activate.")];

const HOOKS_REMOVE: &[HelpEntry] = &[("hooks remove [tool]", "Remove hooks and legacy installs ({hook_tools} | all)"), ("", "  No tool: remove from all tools."), ("", "  Cleans both global (~/) and AGENTX_DIR-local hooks if set.")];

const HOOKS_ABOUT: &[HelpEntry] = &[
    ("", "Per-run: {per_run_tools}"),
    ("", "  Hooks load only in sessions launched with `agentx launch --tool <tool>`. Nothing to"),
    ("", "  install; remove clears installs left by older agentx versions."),
    ("", "Persistent: {persistent_list}"),
    ("", "  Hooks live in the tool's config; install them with `hooks add`."),
    ("", "No hooks: {hookless_list}"),
    ("", "  `agentx launch --tool <tool>` gets status and messages over the tool's own connection."),
    ("", ""),
    ("", "Without hooks, run `agentx start` inside any AI tool (manual delivery)."),
];

const HOOKS_PAGE: &[&[HelpEntry]] = &[HOOKS_STATUS, HOOKS_ADD, HOOKS_REMOVE, BLANK, HOOKS_ABOUT];

const TERM_SCREEN: &[HelpEntry] = &[
    ("term", "Screen dump (all PTY instances)"),
    ("term [name]", "Screen dump for specific agent"),
    ("  --json", "Raw JSON output"),
    ("", "    Fields: lines[], size[rows,cols], cursor[row,col], ready, prompt_empty, input_text"),
    ("", "    prompt_empty/input_text are null when no composer is read (not parsed for this tool)"),
    ("  --clean", "No status header or line numbers ([name] labels kept for all)"),
];

const TERM_INJECT: &[HelpEntry] = &[("term inject <name> [text]", "Inject text into agent PTY"), ("  --enter", "Append \\r (submit). Works alone or with text.")];

const TERM_DEBUG: &[HelpEntry] = &[
    ("term debug on", "Enable PTY debug logging (all instances)"),
    ("term debug off", "Disable PTY debug logging"),
    ("term debug logs", "List debug log files"),
    ("", "Instances pick up the toggle within ~10s."),
    ("", "Logs: ~/.agentx/.tmp/logs/pty_debug/"),
];

const TERM_PAGE: &[&[HelpEntry]] = &[TERM_SCREEN, BLANK, TERM_INJECT, BLANK, TERM_DEBUG];

/// Subcommands with their own `--help` page, built from their parent's sections.
const SUBCOMMAND_HELP: &[(&str, &[&[HelpEntry]])] = &[
    ("events sub", EVENTS_SUB_PAGE),
    ("events sub list", &[EVENTS_SUB_LIST]),
    ("events sub show", &[EVENTS_SUB_SHOW]),
    ("events unsub", &[EVENTS_UNSUB]),
    ("events launch", &[EVENTS_LAUNCH]),
    ("config bootstrap", &[CONFIG_BOOTSTRAP]),
    ("bundle list", &[BUNDLE_LIST]),
    ("bundle show", &[BUNDLE_SHOW]),
    ("bundle cat", &[BUNDLE_CAT]),
    ("bundle chain", &[BUNDLE_CHAIN]),
    ("bundle prepare", &[BUNDLE_PREPARE]),
    ("bundle create", &[BUNDLE_CREATE, BUNDLE_JSON_FORMAT]),
    ("relay daemon", &[RELAY_DAEMON]),
    ("hooks add", &[HOOKS_ADD]),
    ("hooks remove", &[HOOKS_REMOVE]),
    ("term inject", &[TERM_INJECT]),
    ("term debug", &[TERM_DEBUG]),
];

/// Help topic for `agentx <cmd> <args...> --help`: `"<cmd> <sub>"` when the first
/// arg is a subcommand with its own page, otherwise just `cmd`.
pub fn help_topic(cmd: &str, args: &[String]) -> String {
    if cmd == "events"
        && args.first().is_some_and(|arg| arg == "sub")
        && let Some(action) = args.get(1).filter(|action| matches!(action.as_str(), "list" | "show"))
    {
        return format!("events sub {action}");
    }
    if matches!(cmd, "profile" | "labels")
        && let Some(sub) = args.first()
        && native_command(cmd).is_some_and(|command| command.find_subcommand(sub).is_some())
    {
        return format!("{cmd} {sub}");
    }
    if cmd == "observe-web" {
        return "observe-web".into();
    }
    if cmd == "observe" {
        return args.first().map(|sub| format!("observe {sub}")).unwrap_or_else(|| "observe".into());
    }
    if let Some(sub) = args.first() {
        let sub = match (cmd, sub.as_str()) {
            ("bundle", "preview") => "prepare",
            ("hooks", "install") => "add",
            ("hooks", "uninstall") => "remove",
            (_, sub) => sub,
        };
        let topic = format!("{cmd} {sub}");
        if SUBCOMMAND_HELP.iter().any(|(t, _)| *t == topic) {
            return topic;
        }
    }
    cmd.to_string()
}

/// Fill hook-tool placeholders from released integrations.
fn expand_hook_tools(text: &str) -> String {
    use crate::integrations::hooks::runtime::HookMode;
    let tools = crate::cli::commands::hooks::hook_tools();
    let names = |keep: &dyn Fn(HookMode) -> bool, sep: &str| tools.iter().filter(|tool| keep(HookMode::of(**tool))).map(|tool| tool.as_str()).collect::<Vec<_>>().join(sep);
    text.replace("{hook_tools}", &names(&|m| m != HookMode::None, " | "))
        .replace("{persistent_tools}", &names(&|m| m == HookMode::Persistent, " | "))
        .replace("{persistent_list}", &names(&|m| m == HookMode::Persistent, ", "))
        .replace("{per_run_tools}", &names(&|m| m == HookMode::PerRun, ", "))
        .replace("{hookless_list}", &names(&|m| m == HookMode::None, ", "))
}

// ── Tool launch help (claude/gemini/codex/opencode/kilo/pi/omp/antigravity/cursor/kimi/copilot/qoder) ─────────────────────

// ── Format a single help entry ──────────────────────────────────────────

/// Align short entries and keep long entries beside their descriptions.
fn aligned(left: &str, desc: &str, col: usize) -> String {
    let width = left.chars().count();
    if desc.is_empty() {
        left.trim_end().to_string()
    } else if width < col {
        format!("{left:<col$} {desc}")
    } else {
        format!("{left}  {desc}")
    }
}

fn format_entry(usage: &str, desc: &str) -> String {
    if usage.is_empty() {
        // Empty usage: plain text or blank line
        if desc.is_empty() { String::new() } else { format!("  {}", desc) }
    } else if usage.starts_with("  ") {
        // Indented: option/setting line
        aligned(&format!("  {usage}"), desc, 34)
    } else if usage.ends_with(':') {
        // Section header
        if desc.is_empty() { format!("\n{}", usage) } else { format!("\n{} {}", usage, desc) }
    } else {
        // Command line
        aligned(&format!("  agentx {usage}"), desc, 33)
    }
}

/// Format entries from a static slice.
fn format_entries(entries: &[HelpEntry]) -> Vec<String> {
    entries.iter().map(|(u, d)| format_entry(u, d)).collect()
}

// ── Lookup and render ───────────────────────────────────────────────────

/// Canonical public commands for docs and recursive help; tools are option values.
pub const COMMAND_NAMES: &[&str] = &[
    "labels",
    "bindings",
    "attach",
    "profile",
    "roles",
    "launch",
    "resume",
    "recover",
    "fork",
    "send",
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
    "term",
    "relay",
    "run",
    "update",
];

fn forkable_tool_names() -> String {
    crate::integrations::registry::ALL.iter().filter(|spec| spec.released && spec.resume.as_ref().is_some_and(|resume| resume.fork.is_some())).map(|spec| spec.name).collect::<Vec<_>>().join("/")
}

/// Get the top-level help text as a String.
pub fn get_help_text() -> String {
    let forkable = forkable_tool_names();
    let launchable = crate::integrations::registry::released_tool_names().join("|");
    compact_help(&format!(
        "agentx (hook-comms) v{} - multi-agent communication\n\
\n\
Usage:\n\
\x20 agentx                                  TUI dashboard (all workspaces)\n\
\x20 agentx --dir PATH                        Dashboard for the enclosing workspace\n\
\x20   --workspace PATH|global                Select an exact workspace or global members\n\
\x20   --label CONDITION                      Filter current local Agents (repeatable)\n\
\x20   --new-terminal                          Open the dashboard in another terminal\n\
\x20 agentx <command>                        Run command\n\
\x20 agentx [command...] --help              Show help for this command\n\
\x20 agentx [command...] --help-all          Expand all help below this command\n\
\n\
Launch:\n\
\x20 agentx launch --go --headless --title \"Task title\" --bind \".controller/xx/xx.task.md\" --agentx-prompt \"Assignment\"\n\
\x20 Configured defaults are selected automatically; add --role NAME when needed.\n\
\x20 Tools: {launchable}\n\
\x20 agentx recover [@name...]               Continue Agents after model failures\n\
\x20 agentx resume <target>                  Resume stopped agent\n\
\x20 agentx fork <target>                    Fork agent session ({forkable})\n\
\x20 agentx kill <name(s)|all>               Kill + close terminal pane\n\
\x20 Shortcuts: l = launch, r = resume, f = fork, k = kill, a = attach\n\
\n\
Directory teams:\n\
\x20 agentx roles --dir PATH                 Discover reusable roles\n\
\x20 Add --dir PATH / --role NAME to the launch command above for a team member.\n\
\x20 agentx list --dir PATH                   Show that team and global members\n\
\x20 agentx attach --dir PATH                 Choose a local tmux Agent in that team\n\
\x20 agentx list --all                        Show all teams in this AgentX scope\n\
\x20 agentx send --dir PATH -- message        Reuse/start the default recipient\n\
\x20 Setup and examples: agentx roles --help\n\
\n\
Commands:\n\
\x20 send         Send to agents or a directory's default recipient\n\
\x20 listen       Block until message or event arrives\n\
\x20 list         Show current team, global agents, status and unread counts\n\
\x20 bindings     Read file-owned Task/Delivery assignments and status\n\
\x20 events       Query event stream, manage subscriptions\n\
\x20 bundle       Structured context packages for handoffs\n\
\x20 transcript   Read another agent's conversation\n\
\x20 start        Connect to agentx (run inside any AI tool)\n\
\x20 stop         Disconnect from agentx\n\
\x20 profile      Manage reusable launch configurations\n\
\x20 roles        Discover directory-scoped role definitions\n\
\x20 config       Get/set global and per-agent settings\n\
\x20 attach       Attach to a running agent's tmux terminal\n\
\x20 run          Execute workflow scripts\n\
\x20 relay        Cross-device sync + relay daemon\n\
\x20 archive      Query past agentx sessions\n\
\x20 reset        Archive and clear database\n\
\x20 hooks        Add or remove hooks\n\
\x20 status       Installation and diagnostics\n\
\x20 observe      Inspect existing daemon/config/event evidence\n\
\x20 observe-web  Serve read-only browser diagnostics\n\
\x20 workbench    Open project collaboration in a browser\n\
\x20 daemon       Inspect or explicitly restart the native scope daemon\n\
\x20 work         Read work and report assigned results/checks\n\
\x20 term         View/inject into agent PTY screens\n\
\x20 update       Check and apply updates\n\
\n\
Environment files (before --):\n\
\x20 --env-file PATH                         Ordered dotenv file (repeatable)\n\
\x20 --env-override / --no-env-override       Replace / preserve inherited values\n\
\x20 AGENTX_ENV_FILES                        Comma-separated file list\n\
\x20 agentx config environment --json        Inspect sources/provenance without values",
        env!("CARGO_PKG_VERSION"),
    ))
}

/// Flags accepted by both `agentx launch --tool <tool>` (fresh launch) and `agentx r` / `agentx f`
/// (resume/fork). Indented to 4 spaces for tool help, re-indented for resume.
const SHARED_LAUNCH_FLAGS: &[(&str, &str)] = &[
    ("--terminal <preset>", "Override terminal; tmux attaches unless headless/no-run-here"),
    ("--device <device>", "Run on a remote device via relay"),
    ("--dir <path>", "Working directory"),
    ("--env-file <path>", "Load dotenv data (repeat for ordered files)"),
    ("--env-override / --no-env-override", "Replace / preserve inherited environment values"),
    ("--headless", "Run in background"),
    ("--run-here / --no-run-here", "Use current terminal / open elsewhere (tmux: detach)"),
    ("--agentx-prompt <text>", "Initial prompt"),
    ("--agentx-system-prompt <text>", "System prompt"),
    ("--json", "Print the result as one JSON object"),
    ("--go", "Skip preview, run immediately"),
];

/// Shared help body for `agentx r` / `agentx f` (both accept the same target
/// forms and launch flags; only the header, blurb, and see-also differ).
fn resume_fork_help(usage_line: &str, blurb: &str, see_also_line: &str) -> String {
    let mut flags = String::new();
    for (flag, desc) in SHARED_LAUNCH_FLAGS {
        flags.push_str(&format!("  {:<34}{}\n", flag, desc));
    }

    format!(
        "Usage:\n\
         \x20 {usage_line}\n\
         \n\
         Omit <target> in a terminal to select an eligible session.\n\
         <target> can be:\n\
         \x20 <name>                            agentx name (e.g. luna)\n\
         \x20 <uuid>                            claude/codex/gemini session UUID\n\
         \x20 ses_<id>                          opencode/kilo session ID\n\
         \x20 <thread-name>                     claude /rename title or codex thread_name\n\
         \x20 <target>:<device>                 run on a remote device via relay\n\
         \n\
         {blurb}\n\
         \n\
         Flags (parsed before tool args; pass `--` to stop parsing):\n\
         {flags}\n\
         \n\
         Native tool arguments must follow --; unknown AgentX options are errors.\n\
         \n\
         See also:\n\
         \x20 {see_also_line}",
    )
}

/// Get formatted help for a single command.
pub fn get_command_help(name: &str) -> String {
    compact_help(&command_help_text(crate::cli::router::canonical_command(name)))
}

fn command_help_text(name: &str) -> String {
    let words: Vec<_> = name.split_whitespace().collect();
    if let Some(mut command) = words.first().and_then(|name| native_command(name)) {
        command.build();
        let mut selected = &mut command;
        for word in &words[1..] {
            if selected.find_subcommand(word).is_none() {
                break;
            }
            selected = selected.find_subcommand_mut(word).expect("checked native subcommand");
        }
        return selected.render_long_help().to_string();
    }
    if name == "launch" {
        return crate::cli::commands::launch::help_text();
    }
    if name == "profile" {
        return crate::cli::commands::profile::help_text();
    }
    if name == "roles" {
        return crate::cli::commands::roles::help_text();
    }
    if name == "daemon" {
        return crate::cli::commands::daemon::native_help();
    }
    if name == "observe-web" {
        return crate::observability::web::help_text();
    }
    if name == "workbench" {
        return crate::collaboration::web::help_text();
    }
    if name == "bindings" {
        return crate::cli::commands::bindings::help_command().render_long_help().to_string();
    }
    if name == "work" {
        return crate::cli::commands::work::help_text();
    }
    if name == "observe" || name.starts_with("observe ") {
        return crate::cli::commands::observe::help_text(&name.split_whitespace().skip(1).collect::<Vec<_>>());
    }
    command_help_raw(name)
}

/// Native declarations shared by ordinary and recursive help; never parse or execute.
fn native_command(name: &str) -> Option<clap::Command> {
    match name {
        "labels" => Some(crate::cli::commands::labels::help_command()),
        "recover" => Some(crate::cli::commands::recover::help_command()),
        "attach" => Some(crate::cli::commands::attach::help_command()),
        "kill" => Some(crate::cli::commands::kill::help_command()),
        "profile" => Some(crate::cli::commands::profile::help_command()),
        "roles" => Some(crate::cli::commands::roles::help_command()),
        "bindings" => Some(crate::cli::commands::bindings::help_command()),
        "work" => Some(crate::cli::commands::work::help_command()),
        "daemon" => Some(crate::cli::commands::daemon::help_command()),
        "observe" => Some(crate::cli::commands::observe::help_command()),
        _ => None,
    }
}

/// The existing static pages also supply recursive topology and scoped excerpts.
fn manual_sections(name: &str) -> Option<&'static [&'static [HelpEntry]]> {
    match name {
        "list" => Some(&[LIST_HELP]),
        "send" => Some(&[SEND_HELP]),
        "events" => Some(EVENTS_PAGE),
        "bundle" => Some(BUNDLE_PAGE),
        "stop" => Some(&[STOP_HELP]),
        "start" => Some(&[START_HELP]),
        "listen" => Some(&[LISTEN_HELP]),
        "reset" => Some(&[RESET_HELP]),
        "relay" => Some(RELAY_PAGE),
        "archive" => Some(&[ARCHIVE_HELP]),
        "run" => Some(&[RUN_HELP]),
        "status" => Some(&[STATUS_HELP]),
        "update" => Some(&[UPDATE_HELP]),
        "hooks" => Some(HOOKS_PAGE),
        "term" => Some(TERM_PAGE),
        "config" => Some(CONFIG_PAGE),
        "transcript" => Some(&[TRANSCRIPT_HELP]),
        _ => SUBCOMMAND_HELP.iter().find(|(topic, _)| *topic == name).map(|(_, page)| *page),
    }
}

/// Share compact layout across static help, clap help and recursive help pages.
pub(crate) fn compact_help(text: &str) -> String {
    let mut output = String::new();
    let mut argument = false;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        if argument && line.starts_with("          ") {
            output.pop();
            output.push_str("  ");
            output.push_str(line.trim());
            argument = false;
        } else {
            output.push_str(line.trim_end());
            argument = line.starts_with(' ') && !line.starts_with("          ") && line.trim_start().starts_with(['-', '<', '[']);
        }
        output.push('\n');
    }
    output.pop();
    output
}

/// Compact clap's help displays while preserving error streams and exit codes.
pub(crate) fn print_clap_error(error: &clap::Error) -> std::io::Result<()> {
    use clap::error::ErrorKind;
    use std::io::Write;
    if matches!(error.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand) {
        let text = compact_help(&error.to_string());
        if error.use_stderr() { writeln!(std::io::stderr().lock(), "{text}") } else { writeln!(std::io::stdout().lock(), "{text}") }
    } else {
        error.print()
    }
}

fn format_page(sections: &[&[HelpEntry]]) -> Vec<String> {
    sections.iter().flat_map(|s| format_entries(s)).collect()
}

fn command_help_raw(name: &str) -> String {
    let mut lines = vec!["Usage:".to_string()];

    // Resume / Fork shortcuts — share the target/flag body, differ only on header + see-also.
    if name == "resume" {
        let see_also = format!("agentx f <target>                   Fork an agent session ({})", forkable_tool_names());
        return resume_fork_help(
            "agentx resume <target> [flags] [-- tool-args...]    Resume a stopped agent",
            "Adopting by UUID or thread-name reclaims the original agentx\n\
             identity if one existed; otherwise a new identity is assigned.\n\
             CWD is recovered from the session's transcript/DB.",
            &see_also,
        );
    }
    if name == "fork" {
        let blurb = format!(
            "Creates a new agent that continues from the forked session.\n\
             Supported tools: {}.\n\
             Remote fork (`:<device>`) requires --dir to pin the target cwd.",
            forkable_tool_names().replace('/', ", ")
        );
        let see_also = "agentx r <target>                   Resume a stopped agent";
        return resume_fork_help("agentx fork <target> [flags] [-- tool-args...]    Fork an agent session (active or stopped)", &blurb, see_also);
    }

    let page = if matches!(name, "config" | "transcript") { None } else { manual_sections(name) };
    if let Some(page) = page {
        // A page that opens with a section header needs no "Usage:" above it.
        if page[0][0].0.ends_with(':') {
            lines.clear();
        }
        lines.extend(format_page(page));
        return expand_hook_tools(&lines.join("\n"));
    }

    // Transcript agent filters come from the same canonical backend registry
    // used by parser selection and disk discovery.
    if name == "transcript" {
        lines.extend(format_entries(TRANSCRIPT_HELP));
        let agents = crate::transcript::transcript_tool_names().join(" | ");
        return lines.join("\n").replace("{transcript_agents}", &agents);
    }

    // Config is special: has dynamic config files hint
    if name == "config" {
        lines.extend(format_page(CONFIG_PAGE));
        lines.extend(format_entries(CONFIG_KEYS_HELP));
        lines.push(String::new());
        lines.push(format_entry("", "agentx config <key> --info for details"));
        lines.push(format_entry("", "Precedence: defaults < config.toml < env vars"));
        // Dynamic: resolved config file paths
        let agentx_dir = env::var("AGENTX_DIR").map(std::path::PathBuf::from).unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".agentx"));
        let config_toml = agentx_dir.join("config.toml");
        let env_file = agentx_dir.join("config.env");
        lines.push(format!("  Files: {}, {}", config_toml.display(), env_file.display()));
        lines.extend(format_entries(CONFIG_HELP_2));
        return lines.join("\n");
    }

    // Try parent command (e.g. "bundle foo" -> "bundle")
    if let Some(pos) = name.rfind(' ') {
        return command_help_raw(&name[..pos]);
    }
    format!("Usage: agentx {}", name)
}

/// Print help for a command to stdout.
pub fn print_command_help(name: &str) {
    println!("{}", get_command_help(name));
}

/// Report a CLI syntax mistake: the error first, then the help page for
/// `topic` (to stderr). The page saves the caller a follow-up `--help`.
/// Returns the exit code (1).
pub fn usage_error(topic: &str, msg: &str) -> i32 {
    eprintln!("Error: {msg}\n\n{}", get_command_help(topic));
    1
}

/// `usage_error` for an unrecognized subcommand, with a typo suggestion.
pub fn unknown_subcommand(topic: &str, got: &str, valid: &[&str]) -> i32 {
    let hint = crate::shared::suggest::did_you_mean(got, valid.iter().copied());
    usage_error(topic, &format!("Unknown agentx {topic} subcommand '{got}'{hint}"))
}

/// Report a clap parse failure via `usage_error`, keeping clap's message and
/// tips (e.g. "a similar argument exists") but not its generated usage line.
pub fn clap_usage_error(topic: &str, err: &clap::Error) -> i32 {
    if !err.use_stderr() {
        print_command_help(topic);
        return 0;
    }
    usage_error(topic, &clap_error_message(err))
}

fn clap_error_message(err: &clap::Error) -> String {
    let rendered = err.render().to_string();
    let body: Vec<&str> = rendered
        .lines()
        .take_while(|line| !line.starts_with("Usage:"))
        // The help page follows, so drop clap's pointer to it.
        .filter(|line| !line.starts_with("For more information"))
        .collect();
    // clap names missing positionals by field (`<ID>`); help pages say `<id>`.
    if err.kind() == clap::error::ErrorKind::MissingRequiredArgument {
        let names: Vec<String> = body.iter().filter_map(|line| line.trim().strip_prefix('<')).map(|name| format!("<{}", name.to_lowercase())).collect();
        if !names.is_empty() {
            return format!("Missing required argument: {}", names.join(" "));
        }
    }
    let body = body.join("\n");
    let body = body.trim();
    let body = body.strip_prefix("error: ").unwrap_or(body);
    let mut chars = body.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

pub fn print_help() {
    println!("{}", get_help_text());
    println!();
    println!("Identity:");
    println!("  1. Run agentx start to get a name");
    println!("  2. Commands use the injected process/session identity automatically");
    println!("     Use --iam <name> when you need to select an existing caller explicitly");
    println!();
    println!("Run 'agentx <command> --help' for details.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_text_contains_version() {
        // Capture what print_help would output by checking the format string
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }

    #[test]
    fn all_commands_have_help() {
        let commands = ["send", "list", "events", "stop", "start", "listen", "status", "config", "hooks", "archive", "reset", "transcript", "bundle", "kill", "term", "relay", "run", "resume", "fork"];
        for cmd in commands {
            let help = get_command_help(cmd);
            assert!(help.lines().any(|line| line.starts_with("Usage:")), "help for '{cmd}' should include a usage line");
            assert!(help.len() > 20, "help for '{}' should have content", cmd);
        }
    }

    #[test]
    fn unknown_command_fallback() {
        let help = get_command_help("nonexistent");
        assert_eq!(help, "Usage: agentx nonexistent");
    }

    #[test]
    fn events_sub_resolves_to_events() {
        let help = get_command_help("events sub");
        assert!(help.contains("Subscribe"), "events sub help should contain Subscribe section");
    }

    #[test]
    fn format_entry_rules() {
        // Blank line
        assert_eq!(format_entry("", ""), "");
        // Plain text
        assert_eq!(format_entry("", "some text"), "  some text");
        // Option line (indented)
        assert!(format_entry("  --json", "Output JSON").contains("--json"));
        // Section header
        assert!(format_entry("Examples:", "").starts_with('\n'));
        // Command line
        assert!(format_entry("list", "Show agents").contains("agentx list"));
    }

    #[test]
    fn launch_help_exposes_explicit_selectors_and_boundary() {
        let help = get_command_help("launch");
        for text in ["--tool", "--count", "--stdio", "--headless", "after --", "codex", "claude"] {
            assert!(help.contains(text), "missing {text}: {help}");
        }
    }

    #[test]
    fn capability_driven_help_lists_current_integrations() {
        let transcript_help = get_command_help("transcript");
        for tool in crate::transcript::transcript_tool_names() {
            assert!(transcript_help.contains(tool), "transcript help omitted {tool}");
        }

        let hooks_help = get_command_help("hooks");
        for tool in crate::cli::commands::hooks::hook_tools() {
            assert!(hooks_help.contains(tool.as_str()), "hooks help omitted {tool}");
        }

        // --headless works for every tool (PTY headless), so resume/fork help
        // must not advertise a per-tool restriction.
        for cmd in ["r", "f"] {
            let help = get_command_help(cmd);
            let headless_line = help.lines().find(|l| l.trim_start().starts_with("--headless")).unwrap_or_else(|| panic!("{cmd} help missing --headless"));
            assert!(!headless_line.contains("only"), "{cmd} help restricts --headless: {headless_line}");
        }
        assert!(!get_command_help("f").contains("Kimi"));
    }

    #[test]
    fn overlong_usage_keeps_description_on_the_same_line() {
        assert_eq!(aligned("  agentx kill all", "Kill", 20), "  agentx kill all    Kill");
        assert_eq!(aligned("  agentx events sub \"SQL WHERE\" [--once]", "Raw SQL", 20), "  agentx events sub \"SQL WHERE\" [--once]  Raw SQL");
        assert_eq!(aligned("  --json ", "", 20), "  --json");
    }

    #[test]
    fn help_topic_resolves_subcommands_and_aliases() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(help_topic("events", &args(&["sub"])), "events sub");
        assert_eq!(help_topic("events", &args(&["--agent", "sub"])), "events");
        assert_eq!(help_topic("bundle", &args(&["preview"])), "bundle prepare");
        assert_eq!(help_topic("hooks", &args(&["install"])), "hooks add");
        assert_eq!(help_topic("hooks", &args(&["uninstall"])), "hooks remove");
        assert_eq!(help_topic("list", &args(&["self"])), "list");
    }

    #[test]
    fn clap_error_message_drops_generated_usage() {
        use clap::Parser;
        #[derive(Parser, Debug)]
        struct Args {
            #[arg(long)]
            idle: Option<String>,
        }
        let err = Args::try_parse_from(["events", "--idel", "x"]).unwrap_err();
        let msg = clap_error_message(&err);
        assert!(msg.starts_with("Unexpected argument '--idel'"), "{msg}");
        assert!(msg.contains("'--idle'"), "keeps the did-you-mean tip: {msg}");
        assert!(!msg.contains("Usage:"), "{msg}");
        assert!(!msg.contains("--help"), "{msg}");

        #[derive(Parser, Debug)]
        struct Listen {
            #[arg(value_name = "timeout")]
            timeout: Option<u64>,
        }
        let err = Listen::try_parse_from(["listen", "x"]).unwrap_err();
        let msg = clap_error_message(&err);
        assert!(msg.starts_with("Invalid value 'x' for '[timeout]'"), "{msg}");
        assert!(!msg.contains("--help"), "{msg}");

        #[derive(Parser, Debug)]
        struct Unsub {
            id: String,
        }
        let err = Unsub::try_parse_from(["unsub"]).unwrap_err();
        assert_eq!(clap_error_message(&err), "Missing required argument: <id>");
    }

    #[test]
    fn nested_help_pages_are_specific() {
        let events_sub = get_command_help("events sub");
        assert!(events_sub.contains("--on-hit <text>"));
        assert!(events_sub.contains("--as <name>"));
        assert!(events_sub.contains("events unsub <id>"));
        assert!(!events_sub.contains("Wait for a launch batch:"));

        let events_launch = get_command_help("events launch");
        assert!(events_launch.contains("--timeout SEC"));
        assert!(!events_launch.contains("--on-hit"));

        let bundle_create = get_command_help("bundle create");
        assert!(bundle_create.contains("--bundle-file FILE"));
        assert!(!bundle_create.contains("bundle prepare"));

        let relay_daemon = get_command_help("relay daemon");
        assert!(relay_daemon.contains("relay daemon restart"));
        assert!(!relay_daemon.contains("relay connect <token>"));
    }

    #[test]
    fn subcommand_sections_appear_on_parent_page() {
        let sub_only: &[&[HelpEntry]] = &[EVENTS_SQL_POINTER];
        for (topic, sections) in SUBCOMMAND_HELP {
            let parent = topic.split(' ').next().unwrap();
            let parent_page: &[&[HelpEntry]] = match parent {
                "events" => EVENTS_PAGE,
                "bundle" => BUNDLE_PAGE,
                "relay" => RELAY_PAGE,
                "hooks" => HOOKS_PAGE,
                "term" => TERM_PAGE,
                "config" => CONFIG_PAGE,
                other => panic!("no parent page for {other}"),
            };
            for section in *sections {
                let shared = |s: &&[HelpEntry]| std::ptr::eq(*s, *section);
                assert!(parent_page.iter().any(shared) || sub_only.iter().any(shared), "{topic}: section missing from `agentx {parent} --help`");
            }
        }
    }

    #[test]
    fn help_pages_have_no_double_blank_lines() {
        for (topic, _) in SUBCOMMAND_HELP {
            assert!(!get_command_help(topic).contains("\n\n\n"), "{topic}");
        }
        for name in ["events", "bundle", "relay", "hooks", "term", "config", "start"] {
            assert!(!get_command_help(name).contains("\n\n\n"), "{name}");
        }
    }

    #[test]
    fn config_help_covers_registry_keys() {
        let help = get_command_help("config");
        for key in crate::config::user_fields().map(|f| f.name) {
            let shown = help.contains(key) || key.ends_with("_args");
            assert!(shown || CONFIG_KEYS_NOT_IN_HELP.contains(&key), "config key {key} is neither in help nor CONFIG_KEYS_NOT_IN_HELP");
        }
        assert!(!help.contains("relay_token") && !help.contains("dev_root"));
    }

    #[test]
    fn top_level_help_scopes_fork_to_supported_tools() {
        let help = get_help_text();
        assert!(help.contains("agentx fork <target>                    Fork agent session (claude/codex/opencode/kilo/pi/omp/qoder/grok)"));
        assert!(!help.contains("Fork agent session (claude/codex/opencode/kilo/pi/omp/kimi)"));
        assert_eq!(forkable_tool_names(), "claude/codex/opencode/kilo/pi/omp/qoder/grok");
    }
}
