//! PTY message delivery loop — injects messages via TCP, verifies via cursor advance.

#[cfg(test)]
mod antigravity;
pub(crate) mod grok;
mod host;
pub(crate) use host::{DeliveryHost, run_delivery_loop_client};

use std::io::Write;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::Config;
#[cfg(test)]
use crate::db::AgentxDb;
use crate::integrations::tool::Tool;
use crate::log::{log_error, log_info, log_warn};
use crate::runtime::notify::NotifyServer;
use crate::shared::{ST_ACTIVE, ST_LISTENING};
#[cfg(test)]
use crate::shared::{ST_BLOCKED, ST_INACTIVE};

/// Wakes the PTY proxy after the delivery thread changes title state.
///
/// The proxy remains the sole writer to the terminal. This callback only
/// interrupts its I/O poll so it can serialize the new OSC title promptly.
pub type TitleWake = Arc<dyn Fn() + Send + Sync>;

/// Whether the wrapped child exited because agentx killed it (vs. closed on its
/// own). Set by the PTY proxy (Unix) and read here during delivery cleanup to
/// choose the exit status context. Lives here rather than in `pty` so the
/// delivery loop compiles on platforms without the PTY wrapper.
pub static EXIT_WAS_KILLED: AtomicBool = AtomicBool::new(false);

/// How the wrapped child exited: its code and, for a failure, the last screen
/// lines. Set by the PTY proxy before it stops delivery, so cleanup can say why
/// a tool that crashed after launch went away instead of a bare "closed".
static CHILD_EXIT: Mutex<Option<(i32, Option<String>)>> = Mutex::new(None);

pub fn record_child_exit(exit_code: i32, tail: Option<String>) {
    let tail = tail.filter(|_| exit_code != 0);
    if let Ok(mut exit) = CHILD_EXIT.lock() {
        *exit = Some((exit_code, tail));
    }
}

/// Safely truncate a string to at most `max_chars` characters.
/// Unlike byte slicing `&s[..n]`, this won't panic on multi-byte UTF-8.
pub(crate) fn truncate_chars(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

/// Refresh both delivery-local and PTY-shared status from the database.
#[cfg(test)]
pub(crate) fn refresh_status(db: &AgentxDb, current_name: &str, current_status: &mut String, shared_status: &Option<Arc<std::sync::RwLock<String>>>) -> bool {
    let new_status = match db.get_status(current_name) {
        Ok(Some((status, _))) => status,
        Ok(None) => "stopped".to_string(),
        Err(e) => {
            log_error("native", "delivery.status_check", &format!("DB error getting status: {}", e));
            // Fail closed: don't inject into a PTY whose state we can't verify.
            "stopped".to_string()
        }
    };
    let local_changed = new_status != *current_status;
    let mut shared_changed = false;
    if let Some(shared) = shared_status
        && let Ok(mut status) = shared.write()
        && *status != new_status
    {
        *status = new_status.clone();
        shared_changed = true;
    }
    *current_status = new_status;
    local_changed || shared_changed
}

#[cfg(test)]
fn refresh_status_and_wake(db: &AgentxDb, current_name: &str, current_status: &mut String, shared_status: &Option<Arc<std::sync::RwLock<String>>>, title_wake: &Option<TitleWake>) {
    if refresh_status(db, current_name, current_status, shared_status)
        && let Some(wake) = title_wake
    {
        wake();
    }
}

/// Mirror the OSC 1/2 title into the terminal's own label API for terminals
/// whose chrome doesn't render OSC titles. Currently only herdr; add a
/// `Backend` variant and a `resolve` arm to support another.
mod host_label {
    #[cfg(unix)]
    use std::time::Duration;

    use crate::shared::format_pane_title;

    /// Long enough to absorb a slow herdr server tick, short enough that a
    /// dead socket doesn't visibly stall the delivery loop.
    #[cfg(unix)]
    const SOCKET_TIMEOUT: Duration = Duration::from_millis(200);

    /// A run of this many consecutive transient socket failures (with no
    /// intervening success) disables the backend, as a safety valve against a
    /// wedged-but-connectable socket. Set high enough that ordinary
    /// busy-at-startup EAGAIN churn — several ops per tick during pane creation
    /// — doesn't trip it; any single success resets the count. herdr actually
    /// *exiting* is caught immediately by the connect-failure (Unreachable)
    /// path, so this only guards the rare "socket alive but server stuck" case.
    const MAX_CONSECUTIVE_FAILURES: u32 = 10;

    /// Per-loop state: which backend (if any) we resolved at startup, and the
    /// last label we successfully pushed (for dedupe). A connect failure (herdr
    /// exited) drops the backend immediately; transient I/O only skips the
    /// current op and retries next tick, so a momentary EAGAIN no longer
    /// permanently kills label updates (issue #102, F1).
    pub(super) struct HostLabel {
        backend: Option<Backend>,
        last_pushed: Option<String>,
        /// Consecutive transient socket failures since the last success; any
        /// success resets it. See [`MAX_CONSECUTIVE_FAILURES`].
        consecutive_failures: u32,
    }

    enum Backend {
        Herdr { socket_path: String, pane_id: String },
    }

    impl HostLabel {
        pub(super) fn sync_display(&mut self, display: &str, status: &str, tool: &str) {
            let label = format_pane_title(status, display, tool);
            if !label.is_empty() && self.last_pushed.as_deref() != Some(label.as_str()) && self.send(|backend| backend.push(&label)) {
                self.last_pushed = Some(label);
            }
        }
        pub(super) fn resolve() -> Self {
            // `last_pushed` starts unset so the first delivery-loop iteration
            // *always* pushes a styled label. The built-in herdr preset opens
            // the pane via `tab create --label {instance_name}`, so herdr's
            // initial label is the bare instance name; the styled
            // `◉ luna [claude]` label only appears once we push it. Seeding
            // from AGENTX_PANE_TITLE (which a custom template might or might
            // not have applied) would silently skip that first push and leave
            // the pane stuck on the bare name until a later status change.
            Self { backend: Backend::resolve(), last_pushed: None, consecutive_failures: 0 }
        }

        /// Run one socket op against the backend, taking it so we can drop it on
        /// failure without holding a borrow across the I/O call. Returns whether
        /// the op *applied* (herdr answered with success). The backend is only
        /// disabled (dropped) when herdr is unreachable, or after a run of
        /// transient failures — a single EAGAIN/timeout just skips this op and
        /// retries next tick, and a semantic `Rejected` keeps the healthy
        /// backend so the caller can retry (issue #102, F1).
        fn send<F>(&mut self, op: F) -> bool
        where
            F: FnOnce(&Backend) -> Result<(), SocketError>,
        {
            let Some(backend) = self.backend.take() else {
                return false;
            };
            match op(&backend) {
                Ok(()) => {
                    self.consecutive_failures = 0;
                    self.backend = Some(backend);
                    true
                }
                Err(SocketError::Rejected(msg)) => {
                    // herdr is alive and answered "no" (e.g. the pane is gone or
                    // not ready yet). Keep the backend and let the
                    // caller retry; this isn't a connectivity problem.
                    self.consecutive_failures = 0;
                    crate::log::log_info("host_label", "op_rejected", &format!("{}: {msg}", backend.kind()));
                    self.backend = Some(backend);
                    false
                }
                Err(SocketError::Transient(msg)) => {
                    self.consecutive_failures = self.consecutive_failures.saturating_add(1);
                    if self.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                        crate::log::log_info("host_label", "disabling_after_transient", &format!("{}: {msg} ({} consecutive)", backend.kind(), self.consecutive_failures));
                        // Drop the backend (leave self.backend = None).
                    } else {
                        self.backend = Some(backend);
                    }
                    false
                }
                Err(SocketError::Unreachable(msg)) => {
                    crate::log::log_info("host_label", "disabling_unreachable", &format!("{}: {msg}", backend.kind()));
                    // Drop the backend (leave self.backend = None).
                    false
                }
            }
        }
    }

    impl Backend {
        fn resolve() -> Option<Self> {
            if std::env::var("HERDR_ENV").ok().as_deref() == Some("1") {
                let socket_path = std::env::var("HERDR_SOCKET_PATH").ok().filter(|s| !s.is_empty())?;
                let pane_id = std::env::var("HERDR_PANE_ID").ok().filter(|s| !s.is_empty())?;
                return Some(Backend::Herdr { socket_path, pane_id });
            }
            None
        }

        fn kind(&self) -> &'static str {
            match self {
                Backend::Herdr { .. } => "herdr",
            }
        }

        /// Push a visual label via `pane.rename` (herdr's manual pane label).
        fn push(&self, label: &str) -> Result<(), SocketError> {
            match self {
                Backend::Herdr { socket_path, pane_id } => {
                    let request = serde_json::json!({
                        "id": "agentx:pane:rename",
                        "method": "pane.rename",
                        "params": { "pane_id": pane_id, "label": label },
                    });
                    send_unix_request(socket_path, &request)
                }
            }
        }
    }

    /// Outcome of one socket round-trip, classified so the caller can react
    /// correctly instead of failing closed on every hiccup (issue #102, F1).
    ///
    /// `Transient`/`Rejected` are only produced by the `#[cfg(unix)]`
    /// round-trip; on non-unix the stub yields `Unreachable`, so allow the
    /// otherwise-unconstructed variants there.
    #[cfg_attr(not(unix), allow(dead_code))]
    enum SocketError {
        /// herdr is unreachable (connect failed, or the connection dropped
        /// mid-request). Treated as fatal — the backend is disabled.
        Unreachable(String),
        /// Transient I/O against a live-looking socket (EAGAIN / timeout /
        /// interrupt). herdr is likely just busy; skip this op and retry next
        /// tick. Only a run of these disables the backend.
        Transient(String),
        /// herdr received the request and answered with a JSON `error` (e.g. a
        /// pane id it doesn't know yet). The socket is healthy; only this op
        /// didn't apply, so we neither disable the backend nor record the op as
        /// done — we retry next tick.
        Rejected(String),
    }

    #[cfg(unix)]
    fn send_unix_request(socket_path: &str, request: &serde_json::Value) -> Result<(), SocketError> {
        use std::io::{BufRead, BufReader, ErrorKind, Write};
        use std::os::unix::net::UnixStream;

        // A read/write error against an already-connected socket: EAGAIN /
        // timeout / interrupt are transient (retry), anything else means herdr
        // dropped the connection (fatal).
        fn classify_io(stage: &str, e: &std::io::Error) -> SocketError {
            match e.kind() {
                ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted => SocketError::Transient(format!("{stage}: {e}")),
                _ => SocketError::Unreachable(format!("{stage}: {e}")),
            }
        }

        let mut stream = UnixStream::connect(socket_path).map_err(|e| SocketError::Unreachable(format!("connect: {socket_path}: {e}")))?;
        let _ = stream.set_read_timeout(Some(SOCKET_TIMEOUT));
        let _ = stream.set_write_timeout(Some(SOCKET_TIMEOUT));
        writeln!(stream, "{request}").map_err(|e| classify_io("write", &e))?;
        let mut response = String::new();
        BufReader::new(&stream).read_line(&mut response).map_err(|e| classify_io("read", &e))?;

        classify_response(&response)
    }

    /// Inspect herdr's one-line JSON response. A top-level `error` field means a
    /// semantic rejection; a `result` (or any other parseable body) is success.
    /// An empty line means herdr closed the connection without answering —
    /// treated as unreachable.
    #[cfg(unix)]
    fn classify_response(response: &str) -> Result<(), SocketError> {
        let trimmed = response.trim();
        if trimmed.is_empty() {
            return Err(SocketError::Unreachable("empty response".into()));
        }
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(val) => match val.get("error") {
                Some(err) => {
                    let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("rejected");
                    Err(SocketError::Rejected(msg.to_string()))
                }
                None => Ok(()),
            },
            // A non-empty but unparseable line: herdr answered with something, so
            // it's alive — be lenient and count it as success rather than
            // wedging retries on a body we mostly don't read anyway.
            Err(_) => Ok(()),
        }
    }

    #[cfg(not(unix))]
    fn send_unix_request(_socket_path: &str, _request: &serde_json::Value) -> Result<(), SocketError> {
        Err(SocketError::Unreachable("unix sockets unavailable on this platform".into()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::shared::ST_LISTENING;
        use serial_test::serial;

        #[test]
        fn pane_title_skips_when_tool_empty() {
            assert_eq!(format_pane_title(ST_LISTENING, "luna", ""), "");
        }

        #[test]
        #[serial]
        fn resolve_does_not_seed_last_pushed_from_pane_title_env() {
            // The built-in herdr preset opens the pane via `tab create --label
            // {instance_name}`, so herdr's initial tab label is the bare name
            // (e.g. `luna`). Seeding `last_pushed` from AGENTX_PANE_TITLE would
            // silently swallow the first push and leave the pane stuck on
            // `luna` until the next status transition.
            // SAFETY: test is #[serial].
            unsafe {
                std::env::set_var("AGENTX_PANE_TITLE", "\u{25c9} luna [claude]");
            }
            let label = HostLabel::resolve();
            // SAFETY: clear before assert so a panic doesn't leak env.
            unsafe {
                std::env::remove_var("AGENTX_PANE_TITLE");
            }
            assert!(
                label.last_pushed.is_none(),
                "last_pushed must start unset so the first delivery-loop \
                 iteration always pushes a styled label"
            );
        }

        #[cfg(unix)]
        #[test]
        fn classify_response_distinguishes_error_success_and_closed() {
            // A JSON `error` envelope is a semantic rejection (herdr alive but
            // said no) — keep the backend, retry the op (issue #102, F1).
            let err = r#"{"id":"agentx:pane:rename","error":{"code":"pane_not_found","message":"pane w1:p1 not found"}}"#;
            match classify_response(err) {
                Err(SocketError::Rejected(msg)) => assert!(msg.contains("not found")),
                _ => panic!("expected Rejected for an error envelope"),
            }

            // A `result` envelope is success — the op applied.
            let ok = r#"{"id":"agentx:pane:rename","result":{"type":"pane_renamed"}}"#;
            assert!(classify_response(ok).is_ok());

            // A non-empty but unparseable line: herdr answered, so treat as Ok
            // rather than wedging retries on a body we don't read.
            assert!(classify_response("not json at all\n").is_ok());

            // An empty line means herdr closed the connection without a reply.
            assert!(matches!(classify_response("   \n"), Err(SocketError::Unreachable(_))));
        }
    }
}

/// The bare wake typed into a tool whose hooks show the delivery themselves.
pub(crate) const WAKE_TRIGGER: &str = "<agentx>";

/// Human-readable descriptions for gate block reasons.
pub(crate) fn gate_block_detail(reason: &str) -> &'static str {
    match reason {
        "not_idle" => "waiting for idle status",
        "user_active" => "user is typing",
        "submit_settle" => "waiting for prompt submit to settle",
        "not_ready" => "prompt not visible",
        "output_unstable" => "output still streaming",
        "prompt_has_text" => "uncommitted text in prompt",
        "approval" => "waiting for user approval",
        "nav_overlay" => "waiting for subagent nav / session switcher to close",
        _ => "blocked",
    }
}

/// Build PTY wake text for tools whose delivery path is not human-visible.
///
/// Claude and Codex inject the plain `<agentx>` trigger because their hooks show
/// the full message in the TUI. Gemini, Antigravity, and OpenCode bootstrap
/// need a human-visible prompt line, but it must stay prompt-safe: metadata only,
/// no message body, no `@` autocomplete triggers, and no wrapping. If the compact
/// preview will not fit the current input width, use the same minimal trigger.
#[cfg(test)]
pub(crate) fn build_wake_inject_text(db: &AgentxDb, recipient: &str, max_len: usize) -> String {
    let messages = db.get_unread_messages(recipient);
    if messages.is_empty() {
        return "<agentx>".to_string();
    }

    let recipient_display = sanitize_wake_preview_part(recipient);
    let first_line = format_wake_message_line(&messages[0], &recipient_display);
    let inner = if messages.len() == 1 { first_line } else { format!("[{} new messages] | {}", messages.len(), first_line) };
    let preview = format!("<agentx>{inner}</agentx>");

    if preview.chars().count() > max_len || preview.contains('@') { "<agentx>".to_string() } else { preview }
}

fn sanitize_wake_preview_part(text: &str) -> String {
    let without_tags = strip_agentx_wrapper_tags(text);
    without_tags.split_whitespace().collect::<Vec<_>>().join(" ").replace('@', "")
}

fn wake_message_prefix(msg: &crate::db::Message) -> String {
    let prefix = match (&msg.intent, &msg.thread) {
        (Some(i), Some(t)) => format!("{}:{}", i, sanitize_wake_preview_part(t)),
        (Some(i), None) => sanitize_wake_preview_part(i),
        (None, Some(t)) => format!("thread:{}", sanitize_wake_preview_part(t)),
        (None, None) => "new message".to_string(),
    };
    let id_ref = msg.event_id.map(|id| format!(" #{}", id)).unwrap_or_default();
    format!("[{}{}]", prefix, id_ref)
}

/// Strip tag-like sequences that could break the PTY `<agentx>...</agentx>` wrapper.
fn strip_agentx_wrapper_tags(text: &str) -> String {
    let mut s = text.to_string();
    for tag in ["</agentx>", "<agentx>"] {
        loop {
            let lower = s.to_lowercase();
            if let Some(i) = lower.find(tag) {
                s.replace_range(i..i + tag.len(), "");
            } else {
                break;
            }
        }
    }
    s
}

#[cfg(test)]
fn format_wake_message_line(msg: &crate::db::Message, recipient_display: &str) -> String {
    let envelope = wake_message_prefix(msg);
    let sender_display = sanitize_wake_preview_part(&msg.from);
    format!("{envelope} {sender_display} -> {recipient_display}")
}

/// Tool-specific configuration for delivery gate.
///
/// ## Status Semantics
///
/// - `status="blocked"` - Permission prompt showing. Set by:
///   - Claude/Gemini: hooks detect approval prompt
///   - Codex: PTY detects OSC9 escape sequence (primary mechanism, no hooks)
/// - `status="active"` - Agent processing. Messages not delivering is normal, no alert.
/// - `status="listening"` - Agent idle. Can show status_context for delivery issues.
///
/// ## Gate Logic
///
/// The gate answers one question: "If we inject a single line + Enter right now,
/// will it land as a fresh user turn without clobbering an approval prompt,
/// a running command, or the user's typing?"
///
/// NOTE: Gate check order determines gate.reason, but status updates check
/// screen.approval directly so Codex OSC9 works even when agent is active.
///
/// Gate checks are evaluated in order (fails fast):
/// 1. `require_idle` - DB status must be "listening" (set by hooks after turn completes).
///    Claude/Gemini hooks also set status="blocked" on approval which fails this check.
/// 2. `block_on_approval` - No pending approval prompt (OSC9 detection in PTY).
/// 3. `block_on_user_activity` - No keystrokes within cooldown (default 0.5s, 3s for Claude).
/// 4. Submit-settle cooldown - Do not inject during the short screen/hook race after submit.
/// 5. `require_ready_prompt` - Ready pattern visible on screen (e.g., "? for shortcuts").
///    Pattern hidden when user has uncommitted text or is in a submenu (slash menu).
///    Note: Claude hides this in accept-edits mode, so Claude disables this check.
/// 6. `require_prompt_empty` - Check if prompt has no user text.
///    Claude-specific: Uses VT100 dim attribute detection to distinguish placeholder text
///    (dim) from user input (not dim). Implemented in screen.rs get_claude_input_text().
///    A Claude box holding only a leftover `<agentx>` wake is submitted with Enter
///    rather than waited on (`ScreenState::leftover_wake`).
#[derive(Clone)]
pub struct ToolConfig {
    /// Tool name (claude, gemini, codex)
    pub tool: String,
    /// Require DB status == ST_LISTENING before inject
    pub require_idle: bool,
    /// Require ready pattern visible on screen
    pub require_ready_prompt: bool,
    /// Require prompt to be empty (no user text)
    pub require_prompt_empty: bool,
    /// Block if user is actively typing
    pub block_on_user_activity: bool,
    /// Block if approval prompt detected
    pub block_on_approval: bool,
    /// Whether the launch-readiness gate (separate from the delivery gate)
    /// requires the on-screen ready pattern. Decoupled from
    /// `require_ready_prompt` so tools can disable runtime delivery gates and
    /// still demand the ready pattern at launch time (opencode).
    pub launch_requires_ready: bool,
    /// Launch readiness is proven by the plugin's extension bind rather than the
    /// on-screen ready pattern. See [`GatesSpec::launch_ready_on_plugin_bind`];
    /// the PTY runtime also sets it for native Claude delivery (the mod's poll).
    pub launch_ready_on_plugin_bind: bool,
}

impl ToolConfig {
    /// Build a `ToolConfig` from the per-tool [`IntegrationSpec.gates`].
    ///
    /// Gate booleans (and their rationale) live in `integration_spec.rs`.
    pub fn for_tool(tool: crate::integrations::tool::Tool) -> Self {
        let g = &tool.spec().gates;
        Self {
            tool: tool.as_str().to_string(),
            require_idle: g.require_idle,
            require_ready_prompt: g.require_ready_prompt,
            require_prompt_empty: g.require_prompt_empty,
            block_on_user_activity: g.block_on_user_activity,
            block_on_approval: g.block_on_approval,
            launch_requires_ready: g.launch_requires_ready,
            launch_ready_on_plugin_bind: g.launch_ready_on_plugin_bind,
        }
    }

    // Per-tool constructors retained as test helpers.
    #[cfg(test)]
    pub fn claude() -> Self {
        Self::for_tool(crate::integrations::tool::Tool::Claude)
    }
    #[cfg(test)]
    pub fn gemini() -> Self {
        Self::for_tool(crate::integrations::tool::Tool::Gemini)
    }
    #[cfg(test)]
    pub fn codex() -> Self {
        Self::for_tool(crate::integrations::tool::Tool::Codex)
    }
    #[cfg(test)]
    pub fn opencode() -> Self {
        Self::for_tool(crate::integrations::tool::Tool::OpenCode)
    }
    #[cfg(test)]
    pub fn antigravity() -> Self {
        Self::for_tool(crate::integrations::tool::Tool::Antigravity)
    }
    #[cfg(test)]
    pub fn cursor() -> Self {
        Self::for_tool(crate::integrations::tool::Tool::Cursor)
    }
    #[cfg(test)]
    pub fn copilot() -> Self {
        Self::for_tool(crate::integrations::tool::Tool::Copilot)
    }
    #[cfg(test)]
    pub fn qoder() -> Self {
        Self::for_tool(crate::integrations::tool::Tool::Qoder)
    }
}

/// Gate evaluation result
pub struct GateResult {
    pub safe: bool,
    pub reason: &'static str,
}

/// Gate reason for an open gate whose prompt already holds a leftover wake.
const REASON_LEFTOVER_WAKE: &str = "leftover_wake";

impl GateResult {
    /// The prompt already holds only a leftover `<agentx>` wake: deliver by
    /// pressing Enter on it rather than typing another one beside it.
    pub fn submits_leftover(&self) -> bool {
        self.safe && self.reason == REASON_LEFTOVER_WAKE
    }
}

/// Shared state for delivery thread
pub struct DeliveryState {
    pub screen: Arc<std::sync::RwLock<ScreenState>>,
    /// Grok's native ACP transport; set for every agentx-launched Grok.
    pub grok_acp: Option<grok::Launch>,
    /// True while the launch outcome is still Pending. Cleared once any
    /// terminal outcome (ready/failed/blocked) fires, so the PTY proxy can
    /// stop computing launch-only signals (e.g. `visible_tail`).
    pub launch_phase_active: Arc<AtomicBool>,
    pub inject_port: u16,
    pub user_activity_cooldown_ms: u64,
}

/// Terminal state of a single launch from the PTY delivery loop's perspective.
///
/// At most one terminal outcome (Ready/Failed/Blocked) is ever recorded per
/// loop. The Pending → terminal transition gates `maybe_emit_launch_blocked`
/// and the PTY-side `visible_tail` computation via `launch_phase_active`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchOutcome {
    Pending,
    Ready,
    Failed,
    Blocked,
}

impl LaunchOutcome {
    fn is_pending(&self) -> bool {
        matches!(self, LaunchOutcome::Pending)
    }
}

/// Screen state snapshot for gate checks
#[derive(Clone)]
pub struct ScreenState {
    pub ready: bool,
    pub approval: bool,
    pub prompt_empty: bool,
    pub input_text: Option<String>,
    pub visible_tail: Option<String>,
    pub last_user_input: Instant,
    /// Timestamp of last output (for stability-based recovery)
    pub last_output: Instant,
    /// Terminal width in columns
    pub cols: u16,
    /// Set when input_text transitions from non-empty to empty or temporarily
    /// undetected, i.e. a prompt was likely just submitted. The DB-side
    /// `status=active` update from the tool's UserPromptSubmit hook lags this
    /// screen-visible transition by a few hundred milliseconds, so the delivery
    /// gate must wait out that window or it will double-deliver: once via the
    /// hook (after the user's prompt runs) and once via PTY inject (during the
    /// race window where the gate sees
    /// `listening` + `prompt_empty`). See `SUBMIT_SETTLE_COOLDOWN_MS`.
    pub last_prompt_submit: Option<Instant>,
    /// Latched Cursor/Codex approval signal. Their TUI redraws can briefly erase
    /// both the dialog and title, which would flicker `approval` false while the
    /// prompt is still up. Latch true on any positive detection and only clear
    /// once the scrape has stayed negative. Antigravity keeps its immediate
    /// scrape. See `APPROVAL_SCRAPE_CLEAR_MS`.
    pub approval_scrape_latched: bool,
    /// When the approval scrape last turned negative (`None` while positive).
    pub approval_scrape_negative_since: Option<Instant>,
    /// A Claude TUI overlay is focused whose input box is NOT the current
    /// session's root prompt — the subagent navigator (a human may be typing
    /// into a subagent's box) or the `←` session switcher (input box is a
    /// new-session creator). Both share the parent's single PTY, so injecting the
    /// wake trigger would land in the wrong box; the gate defers while this is
    /// set. Only ever true for Claude (see `ScreenTracker::is_claude_subagent_nav_visible`
    /// / `is_claude_session_switcher_visible`).
    pub nav_overlay: bool,
    /// Claude's input box holds only a leftover `<agentx>` wake on one row (see
    /// `ScreenTracker::is_claude_leftover_wake`): typically a bare wake its hook
    /// blocked with nothing pending, which Claude put back into the box. It is
    /// agentx's own trigger, not a draft, so the gate submits it with Enter
    /// instead of waiting for an empty prompt that never comes. Only ever true
    /// for Claude.
    pub leftover_wake: bool,
    /// Codex is still starting with nothing to answer (see
    /// `ScreenTracker::is_codex_startup_loading`); the launch-blocked heuristic
    /// waits it out rather than reading a quiet loading screen as settled.
    pub startup_loading: bool,
}

impl Default for ScreenState {
    fn default() -> Self {
        Self {
            ready: false,
            approval: false,
            prompt_empty: false,
            input_text: None,
            visible_tail: None,
            last_user_input: Instant::now(),
            last_output: Instant::now(),
            cols: 80,
            last_prompt_submit: None,
            approval_scrape_latched: false,
            approval_scrape_negative_since: None,
            nav_overlay: false,
            leftover_wake: false,
            startup_loading: false,
        }
    }
}

/// Window after an observed prompt-submit during which the delivery gate refuses
/// to inject. Covers the lag between the screen-visible input clear and the tool
/// hook's `status=active` update. Tuned from PTY test traces where the gap was
/// about 1s; round up for headroom.
pub(crate) const SUBMIT_SETTLE_COOLDOWN_MS: u64 = 1500;

/// Pause between seeing the injected text in the composer and pressing Enter
/// (`PtySpec::pre_enter_settle_ms`).
pub(crate) fn pre_enter_settle(tool: &str) -> Option<Duration> {
    let ms = tool.parse::<crate::integrations::tool::Tool>().ok()?.spec().pty.pre_enter_settle_ms;
    (ms > 0).then(|| Duration::from_millis(ms))
}

/// How long a negative approval scrape must hold before it clears the latched
/// signal. Redraw bursts (cursor's approval prompt animating its selection /
/// spinner, Codex on focus changes) emit partial frames that scrape as "no
/// approval"; the next full frame shows the prompt again and restarts the wait,
/// so the gate keeps reporting `approval`, not `prompt_has_text`.
///
/// The wait is measured on the scrape, not on quiet output: a TUI that never
/// stops redrawing (Codex's /voice meters, an animated status line) would
/// otherwise hold an answered approval forever.
pub(crate) const APPROVAL_SCRAPE_CLEAR_MS: u64 = 400;

/// Whether a negative approval scrape has held for `APPROVAL_SCRAPE_CLEAR_MS`.
pub(crate) fn approval_scrape_cleared(negative_since: Option<Instant>, now: Instant) -> bool {
    negative_since.is_some_and(|t| now.saturating_duration_since(t) >= Duration::from_millis(APPROVAL_SCRAPE_CLEAR_MS))
}

/// Latch decision for screen-scraped approval (Cursor, Codex). A partial-render
/// frame mid-redraw scrapes as "no approval"; holding the previous latch through
/// such transient false reads keeps the gate reporting `approval` instead of
/// falling through to `prompt_has_text`. The latch only clears once `settled`
/// confirms the prompt has genuinely left the screen.
pub(crate) fn latch_scraped_approval(prev: bool, scraped: bool, settled: bool) -> bool {
    if scraped {
        true
    } else if settled {
        false
    } else {
        prev
    }
}

impl DeliveryState {
    /// Read activity from the caller's snapshot. Never reacquire the screen lock:
    /// a queued PTY writer can deadlock a recursive read on writer-preferring locks.
    fn is_user_active(&self, screen: &ScreenState) -> bool {
        screen.last_user_input.elapsed().as_millis() < self.user_activity_cooldown_ms as u128
    }
}

/// Evaluate gate conditions for message injection.
///
/// Returns whether it's safe to inject AND the reason if not.
/// NOTE: This only determines injection safety. Status updates (setting "blocked")
/// happen separately in the delivery loop by checking screen.approval directly.
///
/// Check order determines gate.reason but NOT status behavior:
/// 1. require_idle - if agent active, reason="not_idle"
/// 2. approval - if approval showing, reason="approval"
/// 3. block_on_user_activity - if user recently typed, reason="user_active"
/// 4. submit-settle cooldown - if prompt just submitted, reason="submit_settle"
/// 5. require_ready_prompt - if prompt not visible, reason="not_ready"
/// 6. require_prompt_empty - if prompt has user text, reason="prompt_has_text",
///    unless it is only a leftover `<agentx>` wake (Claude): then the gate opens
///    with reason="leftover_wake" and the loop presses Enter alone
///
/// The delivery loop checks screen.approval directly for status="blocked",
/// so Codex OSC9 detection works even when agent is active (gate returns "not_idle").
pub(crate) fn evaluate_gate(config: &ToolConfig, state: &DeliveryState, is_idle: bool) -> GateResult {
    let screen = state.screen.read().unwrap();

    // Check idle FIRST - if agent is busy, that's normal, don't alert
    if config.require_idle && !is_idle {
        return GateResult { safe: false, reason: "not_idle" };
    }
    // Approval check only runs if agent is idle (passed require_idle)
    if config.block_on_approval && screen.approval {
        return GateResult { safe: false, reason: "approval" };
    }
    if config.block_on_user_activity && state.is_user_active(&screen) {
        return GateResult { safe: false, reason: "user_active" };
    }
    // A Claude nav overlay (subagent navigator or `←` session switcher) is
    // focused: the wake trigger writes to the shared stdin, which the tool routes
    // to the focused view — not the root prompt. Defer, or the box-emptiness
    // checks below would scrape the overlay's box and pass. Only ever set for
    // Claude, so no config flag is needed.
    if screen.nav_overlay {
        return GateResult { safe: false, reason: "nav_overlay" };
    }
    // Submit-edge cooldown: after the screen shows the input clearing, the
    // tool's hook hasn't yet flipped DB status to active. Without this,
    // `require_idle + prompt_empty` both look true and we double-inject. Only
    // applies to tools that gate on idleness; bootstrap-style paths (opencode)
    // run with `require_idle=false` and skip this entirely.
    if config.require_idle
        && let Some(submit_at) = screen.last_prompt_submit
        && submit_at.elapsed().as_millis() < SUBMIT_SETTLE_COOLDOWN_MS as u128
    {
        return GateResult { safe: false, reason: "submit_settle" };
    }
    if config.require_ready_prompt && !screen.ready {
        return GateResult { safe: false, reason: "not_ready" };
    }
    if config.require_prompt_empty && !screen.prompt_empty {
        // Our own `<agentx>`, alone in Claude's box (put back after its hook
        // blocked a bare wake), is not a draft to protect: treating it as one
        // held every later message at `prompt_has_text`.
        // Submitting it hands the pending batch to the UserPromptSubmit hook.
        if screen.leftover_wake {
            return GateResult { safe: true, reason: REASON_LEFTOVER_WAKE };
        }
        return GateResult { safe: false, reason: "prompt_has_text" };
    }

    GateResult { safe: true, reason: "ok" }
}

#[cfg(test)]
fn launch_ready_observed(db: &AgentxDb, name: &str, config: &ToolConfig, state: &DeliveryState) -> bool {
    let screen = state.screen.read().unwrap();
    if config.block_on_approval && screen.approval {
        return false;
    }
    // Copilot's SessionStart hook binds the real CLI session after startup and
    // only after the initial prompt has completed. That binding is authoritative
    // readiness evidence even when newer Copilot versions omit or redraw the
    // historical "/ commands" footer before the screen scraper observes it.
    if config.tool == "copilot" && db.has_session(name) {
        return true;
    }
    if config.launch_ready_on_plugin_bind {
        // Authoritative readiness for plugin-driven tools (Pi, OMP, native
        // Claude): the extension's bind (a kind='plugin' notify endpoint)
        // proves both TUI construction and extension load. It deliberately REPLACES on-screen scraping rather
        // than OR-ing with it — their visible chrome is configurable (Pi's
        // header, OMP's status-line presets), and a syntactically broken /
        // non-running extension could still render default chrome and be falsely
        // declared ready. Requiring the bind makes a dead extension block.
        return db.has_notify_endpoint_kind(name, "plugin");
    }
    if config.launch_requires_ready && !screen.ready {
        return false;
    }
    if config.require_prompt_empty && !screen.prompt_empty {
        return false;
    }
    true
}

/// Mark launch phase complete: clears the shared flag so the PTY proxy can
/// stop publishing launch-only signals.
fn mark_launch_phase_complete(state: &DeliveryState, outcome: &mut LaunchOutcome, next: LaunchOutcome) {
    *outcome = next;
    state.launch_phase_active.store(false, Ordering::Release);
}

/// Inject text to PTY via TCP (text only, no Enter).
/// Strips all C0 control chars (0x00-0x1F) except tab. This blocks ESC (0x1B),
/// so ANSI escape sequences cannot pass through.
pub(crate) fn inject_text(port: u16, text: &str) -> bool {
    let safe_text: String = text
        .chars()
        .filter(|c| *c >= ' ' || *c == '\t') // >= 0x20 or tab; blocks ESC, NULL, BEL, etc.
        .collect();

    if safe_text.is_empty() {
        return false;
    }

    match TcpStream::connect(format!("127.0.0.1:{}", port)) {
        Ok(mut stream) => stream.write_all(safe_text.as_bytes()).is_ok(),
        Err(_) => false,
    }
}

/// Inject Enter key to PTY via TCP
pub(crate) fn inject_enter(port: u16) -> bool {
    match TcpStream::connect(format!("127.0.0.1:{}", port)) {
        Ok(mut stream) => stream.write_all(b"\r").is_ok(),
        Err(_) => false,
    }
}

/// Fixed retry delay between gate-blocked delivery attempts.
/// TCP notify handles the fast path (instant wake on status change);
/// this is the fallback polling interval for missed notifications.
/// Initial retry delay: 0.25s.
const RETRY_DELAY: Duration = Duration::from_millis(250);

/// Timeout for phase 1 (text render verification).
const PHASE1_TIMEOUT: Duration = Duration::from_secs(10);

/// Classify the prompt text relative to the text injected by this delivery attempt.
///
/// Only an exact match grants submit authority. A substring match is deliberately
/// classified as mixed because pressing Enter would also submit unrelated text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptOwnership {
    Exclusive,
    Mixed,
    Other,
}

fn prompt_ownership(input_text: Option<&str>, injected_text: &str) -> PromptOwnership {
    match input_text {
        Some(input) if !injected_text.is_empty() && input == injected_text => PromptOwnership::Exclusive,
        Some(input) if !injected_text.is_empty() && input.contains(injected_text) => PromptOwnership::Mixed,
        _ => PromptOwnership::Other,
    }
}

/// Ownership of the prompt before an Enter (first or retry). A leftover wake is
/// only ours while the scrape still sees it alone on one row: the prompt row
/// keeps reading `<agentx>` after the user adds a second line, so text alone
/// cannot tell the leftover from a draft that starts with it.
fn submit_ownership(screen: &ScreenState, injected_text: &str, leftover: bool) -> PromptOwnership {
    match prompt_ownership(screen.input_text.as_deref(), injected_text) {
        PromptOwnership::Exclusive if leftover && !screen.leftover_wake => PromptOwnership::Mixed,
        ownership => ownership,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase1Decision {
    Rendered,
    MixedPrompt,
    Waiting,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VerifyTimeoutDecision {
    DeliveredWithoutCursor,
    Retry,
    FastFail,
    Reset,
}

fn verify_timeout_decision(tool: Option<Tool>, has_pending: bool, inject_attempt: u32) -> VerifyTimeoutDecision {
    if !has_pending {
        return VerifyTimeoutDecision::DeliveredWithoutCursor;
    }
    if matches!(tool, Some(Tool::Claude | Tool::Codex)) {
        return VerifyTimeoutDecision::FastFail;
    }
    if inject_attempt < 3 { VerifyTimeoutDecision::Retry } else { VerifyTimeoutDecision::Reset }
}

/// Once Codex has received text, screen uncertainty is not proof that input
/// was rejected. Keep its one outstanding wake instead of typing another.
fn unconfirmed_prompt_state(tool: &str) -> State {
    if tool == "codex" { State::WakeUnacknowledged } else { State::Pending }
}

/// New mail and elapsed time cannot acknowledge an outstanding wake.
fn wake_wait_state(cursor_before: i64, current_cursor: i64, has_pending: bool) -> State {
    if current_cursor > cursor_before || !has_pending { if has_pending { State::Pending } else { State::Idle } } else { State::WakeUnacknowledged }
}

/// Decide phase-1 state from one screen snapshot. Ownership checks intentionally
/// precede the deadline so a complete render observed at the boundary succeeds.
fn phase1_decision(input_text: Option<&str>, injected_text: &str, elapsed: Duration) -> Phase1Decision {
    match prompt_ownership(input_text, injected_text) {
        PromptOwnership::Exclusive => Phase1Decision::Rendered,
        PromptOwnership::Mixed => Phase1Decision::MixedPrompt,
        PromptOwnership::Other if elapsed > PHASE1_TIMEOUT => Phase1Decision::TimedOut,
        PromptOwnership::Other => Phase1Decision::Waiting,
    }
}

/// Timeout for phase 2 (text clear verification).
const PHASE2_TIMEOUT: Duration = Duration::from_secs(2);

/// Overall verification timeout for cursor advance.
const VERIFY_TIMEOUT: Duration = Duration::from_secs(10);

/// How long to wait in idle state before checking again.
const IDLE_WAIT: Duration = Duration::from_secs(30);

/// How long messages may sit pending at an idle Copilot with extension
/// delivery before the PTY loop logs that the extension is not delivering.
const EXTENSION_STALL_WARN: Duration = Duration::from_secs(20);

/// Maximum number of Enter-key retries during phase 2 (text clear).
const MAX_ENTER_ATTEMPTS: u32 = 3;

/// Delivery state machine for the native PTY path (Claude/Gemini/Codex/Antigravity).
///
/// OpenCode bypasses this entirely — it early-returns with its own loop
/// inside `run_delivery_loop`.
/// - `Pending`: evaluates gate + idle checks, performs text injection
/// - `WaitTextRender`: confirms injected text appeared in the prompt, sends Enter on match
/// - `WaitTextClear`: verifies prompt cleared after Enter, retries Enter on timeout
/// - `VerifyCursor`: waits for hook-side cursor advance (falls back to has_pending==false)
/// - `WakeUnacknowledged`: a Claude/Codex wake lacks acknowledgement (or Codex
///   screen observation is inconclusive); reinjection stays latched until hook-side
///   progress, a subsequent session-switcher cycle, or a process restart
///
/// Other providers return to `Pending` on failed verification; success goes to `Idle`
/// or `Pending` (if more queued).
#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Idle,
    Pending,
    WaitTextRender,
    WaitTextClear,
    VerifyCursor,
    WakeUnacknowledged,
}

/// Run the delivery loop — surfaces out-of-band agentx messages into the tool's
/// conversation by injecting text at a safe prompt state.
///
/// This is the main delivery thread function. It:
/// 1. Waits for messages (notify-driven)
/// 2. Evaluates gate conditions
/// 3. Injects text and verifies delivery
/// 4. Retries with backoff on failure
///
/// The optional `shared_name` and `shared_status` Arcs are updated on rebind/status change
/// to keep the main PTY loop's OSC title override in sync.
/// True when this delivery thread's process_id still owns `current_name`.
#[cfg(test)]
pub(crate) fn instance_owns_process_binding(db: &AgentxDb, process_id: &str, current_name: &str) -> bool {
    if process_id.is_empty() {
        return true;
    }
    match db.get_process_binding(process_id) {
        Ok(Some(bound_name)) => bound_name == current_name,
        Ok(None) => false,
        Err(_) => false,
    }
}

/// Hard PTY exit cleanup: inactive status, life event, delete instance row.
///
/// Publishes through the same atomic delete gate as `stop_instance`, so a
/// concurrent `agentx kill` and this cleanup produce exactly one stopped event.
#[cfg(test)]
pub(crate) fn cleanup_deleted_instance(db: &mut AgentxDb, current_name: &str) {
    let child_exit = CHILD_EXIT.lock().ok().and_then(|exit| exit.clone());
    finalize_pty_stop(db, current_name, child_exit);
}

#[cfg(test)]
fn finalize_pty_stop(db: &mut AgentxDb, current_name: &str, child_exit: Option<(i32, Option<String>)>) {
    let skip = |why: &str| {
        log_info("native", "delivery.cleanup_skipped", &format!("Skipping PTY stop event for {current_name} because {why}"));
    };
    let inst = match db.get_instance_full(current_name) {
        Ok(Some(inst)) => inst,
        Ok(None) => return skip("the instance row is already gone"),
        Err(e) => {
            log_error("native", "delivery.cleanup", &format!("DB error loading instance {current_name}: {e}"));
            return;
        }
    };
    let snapshot = db.get_instance_snapshot(current_name).ok().flatten();

    // `agentx kill` records exit:killed + its initiator before signalling.
    let kill_recorded = inst.status_context == "exit:killed";
    let was_killed = kill_recorded || EXIT_WAS_KILLED.load(std::sync::atomic::Ordering::Acquire);
    let (exit_context, exit_reason) = if was_killed { ("exit:killed", "killed") } else { ("exit:closed", "closed") };
    let by = if kill_recorded && !inst.status_detail.is_empty() { inst.status_detail.clone() } else { "pty".to_string() };
    if !kill_recorded && let Err(e) = db.set_status(current_name, "inactive", exit_context) {
        log_warn("native", "delivery.set_status_fail", &format!("Failed to set inactive status: {}", e));
    }

    // Subscriptions are removed by finalize_instance_stop, which first saves
    // them into the snapshot for `agentx r`.
    let mut event_data = serde_json::json!({
        "action": "stopped",
        "by": by,
        "reason": exit_reason,
    });
    if !was_killed && let Some((exit_code, tail)) = child_exit {
        event_data["exit_code"] = exit_code.into();
        if let Some(tail) = tail {
            event_data["output"] = tail.into();
        }
    }
    if let Some(snapshot) = snapshot {
        event_data["snapshot"] = snapshot;
    }
    match db.finalize_instance_stop(current_name, inst.created_at, inst.session_id.as_deref(), inst.agent_id.as_deref(), &event_data) {
        Ok(true) => {}
        Ok(false) => skip("another stop finalized it first"),
        Err(e) => log_warn("native", "delivery.life_event_fail", &format!("Failed to finalize stop for {current_name}: {e}")),
    }
}

/// Log why PTY exit cleanup was skipped when this thread no longer owns the instance.
#[cfg(test)]
pub(crate) fn log_pty_cleanup_skipped(db: &AgentxDb, current_name: &str) {
    let reason = if db.get_status(current_name).ok().flatten().is_some_and(|(status, _)| status == ST_INACTIVE) { "instance inactive (soft-finalize); process binding cleared or reassigned" } else { "name reassigned to new process" };
    log_info("native", "delivery.cleanup_skipped", &format!("Skipping instance cleanup for {current_name} — {reason}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create DeliveryState with given screen state
    fn make_state(screen: ScreenState, cooldown_ms: u64) -> DeliveryState {
        DeliveryState { grok_acp: None, screen: Arc::new(std::sync::RwLock::new(screen)), launch_phase_active: Arc::new(AtomicBool::new(true)), inject_port: 0, user_activity_cooldown_ms: cooldown_ms }
    }

    /// Helper: screen state where everything is safe for injection
    fn safe_screen() -> ScreenState {
        ScreenState {
            ready: true,
            approval: false,
            prompt_empty: true,
            input_text: None,
            visible_tail: None,
            last_user_input: Instant::now() - Duration::from_secs(10),
            last_output: Instant::now() - Duration::from_secs(10),
            cols: 80,
            last_prompt_submit: None,
            approval_scrape_latched: false,
            approval_scrape_negative_since: None,
            nav_overlay: false,
            leftover_wake: false,
            startup_loading: false,
        }
    }

    /// Linux blocks new readers behind a queued writer. Activity inspection must
    /// finish from the existing guard so both delivery and PTY updates can proceed.
    #[cfg(target_os = "linux")]
    #[test]
    fn user_activity_snapshot_progresses_with_queued_screen_writer() {
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let state = make_state(safe_screen(), 500);
            let screen = state.screen.read().unwrap();
            let writer_screen = state.screen.clone();
            let writer = std::thread::spawn(move || {
                writer_screen.write().unwrap().ready = false;
            });

            // Observe actual writer queuing instead of relying on a timed sleep.
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                match state.screen.try_read() {
                    Err(std::sync::TryLockError::WouldBlock) => break,
                    Ok(guard) => drop(guard),
                    Err(error) => panic!("screen lock poisoned: {error}"),
                }
                assert!(Instant::now() < deadline, "PTY writer did not queue");
                std::thread::yield_now();
            }

            assert!(!state.is_user_active(&screen));
            assert!(screen.ready);
            drop(screen);
            writer.join().unwrap();
            assert!(!state.screen.read().unwrap().ready);
            done_tx.send(()).unwrap();
        });

        // A recursive read regression fails within a bound, rather than hanging
        // the entire test runner on the same deadlock as the live PTY host.
        done_rx.recv_timeout(Duration::from_secs(5)).expect("delivery and PTY writer must both progress");
        worker.join().unwrap();
    }

    #[test]
    fn status_refresh_repairs_codex_approval_cache_divergence() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances
                 (name, tool, status, status_context, status_time, created_at)
                 VALUES ('halo', 'codex', 'active', 'tool:Bash', 0, 0)",
                [],
            )
            .unwrap();

        let shared_status = Arc::new(std::sync::RwLock::new(ST_BLOCKED.to_string()));
        let wake_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let wake_count_for_callback = wake_count.clone();
        let title_wake: TitleWake = Arc::new(move || {
            wake_count_for_callback.fetch_add(1, Ordering::Relaxed);
        });
        // Codex approval detection updates the PTY-owned shared status directly.
        // The delivery loop's private cache can therefore still say active when
        // the approval clears and the database returns to active.
        let mut current_status = ST_ACTIVE.to_string();

        refresh_status_and_wake(&db, "halo", &mut current_status, &Some(shared_status.clone()), &Some(title_wake.clone()));

        assert_eq!(current_status, ST_ACTIVE);
        assert_eq!(*shared_status.read().unwrap(), ST_ACTIVE);
        assert_eq!(wake_count.load(Ordering::Relaxed), 1);

        // A context/detail-only status event does not change the title icon and
        // must not create redundant proxy wakeups.
        refresh_status_and_wake(&db, "halo", &mut current_status, &Some(shared_status), &Some(title_wake));
        assert_eq!(wake_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn pty_cleanup_does_not_log_stop_after_instance_already_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, tool, status, status_context, status_time, created_at)
                 VALUES ('buli', 'pi', 'active', 'running', 0, 0)",
                [],
            )
            .unwrap();

        let snapshot = db.get_instance_snapshot("buli").unwrap();
        db.log_life_event("buli", "stopped", "samu", "killed", snapshot).unwrap();
        db.delete_instance("buli").unwrap();

        cleanup_deleted_instance(&mut db, "buli");

        let events: Vec<(String, String)> = db
            .conn()
            .prepare(
                "SELECT json_extract(data, '$.by'), json_extract(data, '$.reason')
                 FROM events
                 WHERE type = 'life'
                   AND instance = 'buli'
                   AND json_extract(data, '$.action') = 'stopped'
                 ORDER BY id",
            )
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(|row| row.unwrap())
            .collect();

        assert_eq!(events, vec![("samu".to_string(), "killed".to_string())]);
    }

    #[test]
    fn pty_stop_records_child_exit_code_and_output_unless_killed() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = AgentxDb::open_raw(&dir.path().join("test.db")).unwrap();
        db.init_db().unwrap();
        for name in ["crsh", "kild"] {
            db.conn()
                .execute(
                    "INSERT INTO instances (name, tool, status, status_context, status_time, created_at)
                     VALUES (?1, 'pi', 'active', 'running', 0, 0)",
                    [name],
                )
                .unwrap();
        }
        db.mark_killed("kild", "samu").unwrap();
        let exit = Some((1, Some("TypeError: boom".to_string())));
        finalize_pty_stop(&mut db, "crsh", exit.clone());
        finalize_pty_stop(&mut db, "kild", exit);

        let stop = |name: &str| -> (String, Option<i64>, Option<String>) {
            db.conn()
                .query_row(
                    "SELECT json_extract(data, '$.reason'), json_extract(data, '$.exit_code'),
                            json_extract(data, '$.output')
                     FROM events WHERE type = 'life' AND instance = ?1
                       AND json_extract(data, '$.action') = 'stopped'",
                    [name],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .unwrap()
        };
        assert_eq!(stop("crsh"), ("closed".into(), Some(1), Some("TypeError: boom".into())));
        assert_eq!(stop("kild"), ("killed".into(), None, None));
    }

    #[test]
    fn pty_cleanup_keeps_kill_reason_and_initiator_with_single_stop_event() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, tool, status, status_context, status_time, created_at)
                 VALUES ('kiro', 'codex', 'active', 'running', 0, 0)",
                [],
            )
            .unwrap();
        // `agentx kill` records the reason before signalling; the PTY cleanup
        // can then win the race without having seen the signal itself.
        db.mark_killed("kiro", "samu").unwrap();

        cleanup_deleted_instance(&mut db, "kiro");
        // The kill path's own finalize loses the gate and logs nothing.
        crate::integrations::hooks::common::stop_instance(&db, "kiro", "samu", "killed");

        let stops: Vec<(String, String)> = db
            .conn()
            .prepare(
                "SELECT json_extract(data, '$.by'), json_extract(data, '$.reason') FROM events
                 WHERE type = 'life' AND instance = 'kiro'
                   AND json_extract(data, '$.action') = 'stopped'",
            )
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(|row| row.unwrap())
            .collect();
        assert_eq!(stops, vec![("samu".to_string(), "killed".to_string())]);
        assert!(db.get_instance_full("kiro").unwrap().is_none());
    }

    #[test]
    fn soft_stopped_instance_survives_pty_exit_cleanup() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, tool, status, status_context, status_time, created_at, session_id)
                 VALUES ('luna', 'omp', 'inactive', 'exit:turn_end', 0, 0, 'sid-soft')",
                [],
            )
            .unwrap();
        db.set_process_binding("pid-soft", "sid-soft", "luna").unwrap();

        antigravity::cleanup_antigravity_pty_exit(&mut db, "luna", "pid-soft", true);

        assert!(db.get_instance_full("luna").unwrap().is_some());
        assert_eq!(db.get_status("luna").unwrap().map(|(s, _)| s), Some(ST_INACTIVE.to_string()));
    }

    // ---- phase-1 ownership tests ----

    #[test]
    fn phase1_timeout_is_ten_seconds() {
        assert_eq!(PHASE1_TIMEOUT, Duration::from_secs(10));
    }

    #[test]
    fn phase1_complete_render_wins_at_deadline() {
        assert_eq!(phase1_decision(Some("<agentx>"), "<agentx>", PHASE1_TIMEOUT + Duration::from_millis(1),), Phase1Decision::Rendered,);
    }

    #[test]
    fn phase1_rejects_user_text_after_injected_text() {
        assert_eq!(phase1_decision(Some("<agentx> user draft"), "<agentx>", Duration::ZERO), Phase1Decision::MixedPrompt,);
    }

    #[test]
    fn phase1_rejects_user_text_before_injected_text() {
        assert_eq!(phase1_decision(Some("user draft <agentx>"), "<agentx>", Duration::ZERO), Phase1Decision::MixedPrompt,);
    }

    #[test]
    fn phase1_rejects_mixed_prompt_after_activity_cooldown() {
        assert_eq!(phase1_decision(Some("<agentx> user draft"), "<agentx>", Duration::from_millis(501),), Phase1Decision::MixedPrompt,);
    }

    #[test]
    fn claude_fast_fails_after_first_unacknowledged_wake() {
        assert_eq!(verify_timeout_decision(Some(Tool::Claude), true, 1), VerifyTimeoutDecision::FastFail);
    }

    #[test]
    fn claude_accepts_consumed_queue_without_cursor_advance() {
        assert_eq!(verify_timeout_decision(Some(Tool::Claude), false, 1), VerifyTimeoutDecision::DeliveredWithoutCursor);
    }

    #[test]
    fn codex_wake_requires_progress_before_rearming() {
        assert_eq!(verify_timeout_decision(Some(Tool::Codex), true, 1), VerifyTimeoutDecision::FastFail);
        // A delayed hook or new pending mail is not an acknowledgement.
        assert_eq!(wake_wait_state(42, 42, true), State::WakeUnacknowledged);
        assert_eq!(wake_wait_state(42, 43, true), State::Pending);
        assert_eq!(wake_wait_state(42, 42, false), State::Idle);
        assert_eq!(wake_wait_state(42, 43, false), State::Idle);
        assert_eq!(wake_wait_state(42, 41, true), State::WakeUnacknowledged);
    }

    #[test]
    fn codex_screen_uncertainty_does_not_authorize_retyping() {
        assert_eq!(unconfirmed_prompt_state("codex"), State::WakeUnacknowledged);
        assert_eq!(unconfirmed_prompt_state("gemini"), State::Pending);
        assert_eq!(verify_timeout_decision(Some(Tool::Codex), false, 1), VerifyTimeoutDecision::DeliveredWithoutCursor);
    }

    #[test]
    fn other_providers_keep_existing_verify_retry_contract() {
        assert_eq!(verify_timeout_decision(Some(Tool::Gemini), true, 1), VerifyTimeoutDecision::Retry);
        assert_eq!(verify_timeout_decision(Some(Tool::Gemini), true, 3), VerifyTimeoutDecision::Reset);
    }

    #[test]
    fn phase1_unrelated_text_times_out_normally() {
        assert_eq!(phase1_decision(Some("user draft"), "<agentx>", PHASE1_TIMEOUT + Duration::from_millis(1),), Phase1Decision::TimedOut,);
    }

    #[test]
    fn submit_authority_requires_exact_prompt_ownership() {
        assert_eq!(prompt_ownership(Some("<agentx>"), "<agentx>"), PromptOwnership::Exclusive,);
        assert_eq!(prompt_ownership(Some("<agentx> user draft"), "<agentx>"), PromptOwnership::Mixed,);
        assert_eq!(prompt_ownership(Some("user draft"), "<agentx>"), PromptOwnership::Other,);
    }

    // ---- evaluate_gate tests ----

    #[test]
    fn gate_all_conditions_pass() {
        let config = ToolConfig::claude();
        let state = make_state(safe_screen(), 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(result.safe);
        assert_eq!(result.reason, "ok");
    }

    #[test]
    fn gate_blocks_when_not_idle() {
        let config = ToolConfig::claude();
        let state = make_state(safe_screen(), 500);
        let result = evaluate_gate(&config, &state, false);
        assert!(!result.safe);
        assert_eq!(result.reason, "not_idle");
    }

    #[test]
    fn gate_blocks_on_approval() {
        let config = ToolConfig::claude();
        let mut screen = safe_screen();
        screen.approval = true;
        let state = make_state(screen, 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(!result.safe);
        assert_eq!(result.reason, "approval");
    }

    #[test]
    fn antigravity_config_allows_ready_footer_with_placeholder_text() {
        let config = ToolConfig::antigravity();
        assert!(config.require_ready_prompt);
        assert!(config.require_prompt_empty);
        assert!(!config.block_on_user_activity);
    }

    #[test]
    fn gate_antigravity_blocks_prompt_text() {
        let config = ToolConfig::antigravity();
        let mut screen = safe_screen();
        screen.prompt_empty = false;
        screen.input_text = Some("uncommitted".to_string());
        let state = make_state(screen, 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(!result.safe);
        assert_eq!(result.reason, "prompt_has_text");
    }

    #[test]
    fn gate_blocks_on_user_activity() {
        let config = ToolConfig::claude();
        let mut screen = safe_screen();
        screen.last_user_input = Instant::now(); // just typed
        let state = make_state(screen, 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(!result.safe);
        assert_eq!(result.reason, "user_active");
    }

    #[test]
    fn gate_blocks_while_nav_overlay_open() {
        // A Claude nav overlay (subagent view or session switcher) is focused:
        // the box-emptiness checks would otherwise scrape the overlay's (empty)
        // input box and pass, landing the wake trigger in the wrong box.
        let config = ToolConfig::claude();
        let mut screen = safe_screen(); // ready + prompt_empty: would pass otherwise
        screen.nav_overlay = true;
        let state = make_state(screen, 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(!result.safe);
        assert_eq!(result.reason, "nav_overlay");
    }

    #[test]
    fn only_qoder_waits_before_enter() {
        assert_eq!(pre_enter_settle("qoder"), Some(Duration::from_millis(250)));
        for tool in ["claude", "codex", "copilot", "kimi", "adhoc"] {
            assert_eq!(pre_enter_settle(tool), None, "{tool}");
        }
    }

    #[test]
    fn gate_blocks_during_submit_settle_window() {
        let config = ToolConfig::codex();
        let mut screen = safe_screen();
        screen.last_prompt_submit = Some(Instant::now());
        let state = make_state(screen, 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(!result.safe, "gate must block during submit-settle window to prevent racing hook delivery");
        assert_eq!(result.reason, "submit_settle");
    }

    #[test]
    fn gate_passes_after_submit_settle_expires() {
        let config = ToolConfig::codex();
        let mut screen = safe_screen();
        screen.last_prompt_submit = Some(Instant::now() - Duration::from_millis(SUBMIT_SETTLE_COOLDOWN_MS + 100));
        let state = make_state(screen, 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(result.safe);
        assert_eq!(result.reason, "ok");
    }

    #[test]
    fn gate_skips_submit_settle_when_idle_not_required() {
        // OpenCode bootstrap path runs with require_idle=false. The hook-vs-PTY
        // race that submit_settle guards against can't happen there, so the
        // cooldown shouldn't apply.
        let config = ToolConfig::opencode();
        let mut screen = safe_screen();
        screen.last_prompt_submit = Some(Instant::now());
        let state = make_state(screen, 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(result.safe);
    }

    #[test]
    fn gate_blocks_when_not_ready_for_gemini() {
        let config = ToolConfig::gemini();
        let mut screen = safe_screen();
        screen.ready = false;
        let state = make_state(screen, 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(!result.safe);
        assert_eq!(result.reason, "not_ready");
    }

    #[test]
    fn gate_claude_skips_ready_check() {
        // Claude has require_ready_prompt=false
        let config = ToolConfig::claude();
        let mut screen = safe_screen();
        screen.ready = false;
        let state = make_state(screen, 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(result.safe);
    }

    #[test]
    fn gate_blocks_on_prompt_text_for_claude() {
        let config = ToolConfig::claude();
        let mut screen = safe_screen();
        screen.prompt_empty = false;
        let state = make_state(screen, 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(!result.safe);
        assert_eq!(result.reason, "prompt_has_text");
    }

    /// Claude's prompt as the PTY scrape reports it: `text` typed in the box,
    /// with `leftover_wake` as `ScreenTracker::is_claude_leftover_wake` sets it.
    fn claude_prompt(text: &str, leftover_wake: bool) -> ScreenState {
        ScreenState { prompt_empty: text.is_empty(), input_text: Some(text.to_string()), leftover_wake, ..safe_screen() }
    }

    #[test]
    fn gate_submits_a_leftover_wake_but_never_a_draft() {
        let config = ToolConfig::claude();

        // Only our own wake in the box: open, and press Enter on it.
        let state = make_state(claude_prompt("<agentx>", true), 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(result.safe, "leftover wake held: {}", result.reason);
        assert!(result.submits_leftover());
        assert_eq!(result.reason, "leftover_wake");

        // A draft that merely starts with the wake stays protected.
        let state = make_state(claude_prompt("<agentx> hi", false), 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(!result.safe);
        assert!(!result.submits_leftover());
        assert_eq!(result.reason, "prompt_has_text");

        // An empty box takes the normal typed wake.
        let state = make_state(claude_prompt("", false), 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(result.safe);
        assert!(!result.submits_leftover());
        assert_eq!(result.reason, "ok");
    }

    #[test]
    fn gate_leftover_wake_still_waits_for_idle_and_the_user() {
        // The leftover only stands in for an empty prompt; every earlier gate
        // (idle, approval, typing, overlays, submit settle) still applies.
        let config = ToolConfig::claude();
        let gate = |screen: ScreenState, idle: bool| {
            let result = evaluate_gate(&config, &make_state(screen, 500), idle);
            assert!(!result.safe, "{}: gate opened", result.reason);
            assert!(!result.submits_leftover(), "{}", result.reason);
            result.reason
        };
        assert_eq!(gate(claude_prompt("<agentx>", true), false), "not_idle");

        type Tweak = fn(&mut ScreenState);
        let tweaks: [(&str, Tweak); 4] = [("approval", |s| s.approval = true), ("user_active", |s| s.last_user_input = Instant::now()), ("nav_overlay", |s| s.nav_overlay = true), ("submit_settle", |s| s.last_prompt_submit = Some(Instant::now()))];
        for (reason, tweak) in tweaks {
            let mut screen = claude_prompt("<agentx>", true);
            tweak(&mut screen);
            assert_eq!(gate(screen, true), reason);
        }
    }

    #[test]
    fn leftover_wake_is_rendered_text_for_the_submit_phase() {
        // The loop hands the leftover to the render phase as this attempt's
        // text, so the usual exclusive-ownership check guards the Enter.
        assert_eq!(phase1_decision(Some("<agentx>"), WAKE_TRIGGER, Duration::ZERO), Phase1Decision::Rendered);
        assert_eq!(phase1_decision(Some("<agentx> hi"), WAKE_TRIGGER, Duration::ZERO), Phase1Decision::MixedPrompt);
    }

    #[test]
    fn leftover_wake_enter_needs_the_leftover_to_still_hold() {
        // After the gate opened on a leftover, the user adds a second line:
        // the prompt row still reads `<agentx>`, but the scrape no longer sees a
        // lone wake. Neither the first Enter nor a retry may submit it.
        let draft = claude_prompt("<agentx>", false);
        assert_eq!(submit_ownership(&draft, WAKE_TRIGGER, true), PromptOwnership::Mixed);
        assert_eq!(submit_ownership(&claude_prompt("<agentx>", true), WAKE_TRIGGER, true), PromptOwnership::Exclusive);
        // A wake agentx typed itself keeps the usual text check.
        assert_eq!(submit_ownership(&draft, WAKE_TRIGGER, false), PromptOwnership::Exclusive);
    }

    fn open_ready_test_db() -> (tempfile::TempDir, AgentxDb) {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_raw(&dir.path().join("test.db")).unwrap();
        db.init_db().unwrap();
        (dir, db)
    }

    #[test]
    fn launch_ready_observed_follows_tool_gate_shape() {
        let (_dir, db) = open_ready_test_db();
        let n = "toli";
        let mut screen = safe_screen();
        screen.ready = false;
        screen.prompt_empty = true;

        let state = make_state(screen.clone(), 500);
        assert!(launch_ready_observed(&db, n, &ToolConfig::codex(), &state));
        assert!(launch_ready_observed(&db, n, &ToolConfig::claude(), &state));
        assert!(!launch_ready_observed(&db, n, &ToolConfig::opencode(), &state));
        assert!(!launch_ready_observed(&db, n, &ToolConfig::cursor(), &state));

        let state = make_state(screen.clone(), 500);
        assert!(!launch_ready_observed(&db, n, &ToolConfig::gemini(), &state));

        screen.ready = true;
        let state = make_state(screen.clone(), 500);
        assert!(launch_ready_observed(&db, n, &ToolConfig::opencode(), &state));
        assert!(launch_ready_observed(&db, n, &ToolConfig::cursor(), &state));

        screen.prompt_empty = false;
        let state = make_state(screen, 500);
        assert!(!launch_ready_observed(&db, n, &ToolConfig::codex(), &state));
        assert!(!launch_ready_observed(&db, n, &ToolConfig::cursor(), &state));
    }

    #[test]
    fn pi_family_launch_ready_requires_plugin_bind_not_screen() {
        // Pi/OMP readiness is bind-driven: a rendered/ready screen must NOT be
        // enough, and a kind='plugin' notify endpoint must flip it ready even
        // with no on-screen marker.
        for tool in [crate::integrations::tool::Tool::Pi, crate::integrations::tool::Tool::Omp] {
            assert_plugin_bind_readiness(tool);
        }
    }

    fn assert_plugin_bind_readiness(tool: crate::integrations::tool::Tool) {
        let (_dir, db) = open_ready_test_db();
        let config = ToolConfig::for_tool(tool);
        assert!(config.launch_ready_on_plugin_bind, "{tool:?}");
        assert!(tool.ready_patterns().is_empty(), "{tool:?}");

        let mut screen = safe_screen();
        screen.ready = true; // empty pattern => is_ready() always true
        screen.prompt_empty = true;
        let state = make_state(screen, 500);

        // No plugin endpoint yet -> not ready despite the "ready" screen.
        assert!(!launch_ready_observed(&db, "vupo", &config, &state));

        // A pty endpoint (registered at launch, before the extension binds) must
        // not count as readiness.
        db.upsert_notify_endpoint("vupo", "pty", 4001).unwrap();
        assert!(!launch_ready_observed(&db, "vupo", &config, &state));

        // The extension bind is the authoritative signal.
        db.upsert_notify_endpoint("vupo", "plugin", 4002).unwrap();
        assert!(launch_ready_observed(&db, "vupo", &config, &state));
    }

    #[test]
    fn copilot_session_binding_satisfies_launch_readiness() {
        let (_dir, db) = open_ready_test_db();
        db.conn()
            .execute(
                "INSERT INTO instances (name, tool, session_id, created_at)
                 VALUES ('mira', 'copilot', 'copilot-session-1', 0)",
                [],
            )
            .unwrap();
        let mut screen = safe_screen();
        screen.ready = false;
        screen.prompt_empty = false;
        let state = make_state(screen, 500);

        assert!(launch_ready_observed(&db, "mira", &ToolConfig::for_tool(crate::integrations::tool::Tool::Copilot), &state));
    }

    #[test]
    fn gate_gemini_skips_prompt_empty_check() {
        // Gemini has require_prompt_empty=false
        let config = ToolConfig::gemini();
        let mut screen = safe_screen();
        screen.prompt_empty = false;
        let state = make_state(screen, 500);
        let result = evaluate_gate(&config, &state, true);
        assert!(result.safe);
    }

    #[test]
    fn gate_fail_fast_order() {
        // When multiple gates fail, first one wins
        let config = ToolConfig::gemini();
        let mut screen = safe_screen();
        screen.approval = true;
        screen.ready = false;
        let state = make_state(screen, 500);
        // not idle + approval + not ready → not_idle wins
        let result = evaluate_gate(&config, &state, false);
        assert_eq!(result.reason, "not_idle");
    }

    // ---- Screen-scraped approval latch ----

    #[test]
    fn latch_holds_through_transient_false_scrape() {
        // A positive scrape latches true regardless of prior state.
        assert!(latch_scraped_approval(false, true, false));
        assert!(latch_scraped_approval(false, true, true));
        // Latched true survives a transient false scrape while output is still
        // churning (a partial-render frame, not a real dismissal).
        assert!(latch_scraped_approval(true, false, false));
        // Once output settles and the scrape is still false, the prompt has
        // genuinely left the screen -> clear.
        assert!(!latch_scraped_approval(true, false, true));
        // Never spuriously latches from a clean idle state.
        assert!(!latch_scraped_approval(false, false, false));
        assert!(!latch_scraped_approval(false, false, true));
    }

    // ---- Lookup functions ----

    #[test]
    fn gate_block_detail_known_reasons() {
        assert_eq!(gate_block_detail("not_idle"), "waiting for idle status");
        assert_eq!(gate_block_detail("approval"), "waiting for user approval");
        assert_eq!(gate_block_detail("submit_settle"), "waiting for prompt submit to settle");
        assert_eq!(gate_block_detail("nav_overlay"), "waiting for subagent nav / session switcher to close");
        assert_eq!(gate_block_detail("unknown"), "blocked");
    }

    // ---- ToolConfig ----

    #[test]
    fn tool_config_for_adhoc_uses_adhoc_identity_and_gates() {
        let config = ToolConfig::for_tool(crate::integrations::tool::Tool::Adhoc);
        let gates = &crate::integrations::tool::Tool::Adhoc.spec().gates;
        assert_eq!(config.tool, "adhoc");
        assert_eq!(config.require_idle, gates.require_idle);
        assert_eq!(config.require_ready_prompt, gates.require_ready_prompt);
        assert_eq!(config.require_prompt_empty, gates.require_prompt_empty);
        assert_eq!(config.block_on_user_activity, gates.block_on_user_activity);
        assert_eq!(config.block_on_approval, gates.block_on_approval);
        assert_eq!(config.launch_requires_ready, gates.launch_requires_ready);
    }

    #[test]
    fn tool_configs_match_expected_differences() {
        let claude = ToolConfig::claude();
        let gemini = ToolConfig::gemini();
        let codex = ToolConfig::codex();

        // Claude: no ready_prompt, yes prompt_empty
        assert!(!claude.require_ready_prompt);
        assert!(claude.require_prompt_empty);

        // Gemini: yes ready_prompt, no prompt_empty
        assert!(gemini.require_ready_prompt);
        assert!(!gemini.require_prompt_empty);

        // Codex: composer glyph must be visible (a modal over it is not_ready)
        assert!(codex.require_ready_prompt);
        assert!(codex.require_prompt_empty);

        // All require idle
        assert!(claude.require_idle);
        assert!(gemini.require_idle);
        assert!(codex.require_idle);

        // Copilot: footer-gated ready prompt + empty-prompt + approval gating.
        let copilot = ToolConfig::copilot();
        assert!(copilot.require_idle);
        assert!(copilot.require_ready_prompt);
        assert!(copilot.require_prompt_empty);
        assert!(copilot.block_on_user_activity);
        assert!(copilot.block_on_approval);

        // Qoder: same gates as Copilot. Its composer stays on screen while busy,
        // so the ready markers only prove the TUI is up; idle comes from hooks.
        let qoder = ToolConfig::qoder();
        assert!(qoder.require_idle);
        assert!(qoder.require_ready_prompt);
        assert!(qoder.require_prompt_empty);
        assert!(qoder.block_on_user_activity);
        assert!(qoder.block_on_approval);
        assert!(qoder.launch_requires_ready);
    }

    #[test]
    fn wake_inject_includes_prompt_safe_metadata_only() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("agentx.db");
        let db = AgentxDb::open_at(&db_path).unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, status, status_context, created_at, last_event_id)
                 VALUES ('keno', 'listening', '', 1.0, 0)",
                [],
            )
            .unwrap();
        let data = serde_json::json!({
            "from": "life",
            "text": "ping. Always reply to @life, not @bigboss.",
            "scope": "mentions",
            "mentions": ["keno"],
            "intent": "request",
            "thread": "agentx-routing-test",
        });
        db.conn()
            .execute(
                "INSERT INTO events (type, timestamp, instance, data)
                 VALUES ('message', '2026-05-25T12:00:00Z', 'keno', ?1)",
                rusqlite::params![data.to_string()],
            )
            .unwrap();

        let text = build_wake_inject_text(&db, "keno", 120);
        assert!(text.starts_with("<agentx>"), "text={text}");
        assert!(text.ends_with("</agentx>"), "text={text}");
        assert!(text.contains("life"), "text={text}");
        assert!(text.contains("request"), "text={text}");
        assert!(!text.contains('@'));
        assert!(!text.contains("Always reply"));
    }

    #[test]
    fn wake_inject_falls_back_to_minimal_trigger_when_preview_would_wrap() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("agentx.db");
        let db = AgentxDb::open_at(&db_path).unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, status, status_context, created_at, last_event_id)
                 VALUES ('keno', 'listening', '', 1.0, 0)",
                [],
            )
            .unwrap();
        let data = serde_json::json!({
            "from": "life",
            "text": "short",
            "scope": "mentions",
            "mentions": ["keno"],
            "intent": "request",
            "thread": "a-thread-name-that-is-too-wide-for-the-input",
        });
        db.conn()
            .execute(
                "INSERT INTO events (type, timestamp, instance, data)
                 VALUES ('message', '2026-05-25T12:00:00Z', 'keno', ?1)",
                rusqlite::params![data.to_string()],
            )
            .unwrap();

        assert_eq!(build_wake_inject_text(&db, "keno", 24), "<agentx>");
    }
}
