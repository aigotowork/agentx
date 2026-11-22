//! Caller-owned PTY delivery coordination over typed daemon observations.
//! Cached snapshots feed local gates; only fenced semantic observations write
//! durable state. Terminal bytes, native transport, timing and titles stay here.

use super::*;
use crate::agents::attachments::ParticipantRef;
use crate::agents::recovery::{HostAction, Ticket};
use crate::messaging::delivery_service::{Client as PolicyClient, DeliverySnapshot, GateObservation, LaunchObservation, Request as PolicyRequest, Response as PolicyResponse, heartbeat_due};
use crate::runtime::hosts::{EndpointKind, HostService, Request as HostRequest};
use crate::runtime::process_host::ProcessHost;
use crate::service::{ServiceError, ServiceResult};
use std::{path::Path, sync::RwLock};

fn pending_gate_wait(tool: &str, reason: &str, has_recovery: bool) -> Duration {
    if tool == "codex" && reason == "not_idle" && !has_recovery { Duration::from_secs(2) } else { RETRY_DELAY }
}

fn staggered_idle_wait() -> Duration {
    use rand::RngExt;
    IDLE_WAIT - Duration::from_millis(rand::rng().random_range(0..=5_000))
}

#[cfg(test)]
#[test]
fn pty_slow_poll_only_applies_while_codex_is_waiting_for_a_hook() {
    assert_eq!(pending_gate_wait("codex", "not_idle", false), Duration::from_secs(2));
    assert_eq!(pending_gate_wait("codex", "not_idle", true), RETRY_DELAY);
    assert_eq!(pending_gate_wait("codex", "prompt_has_text", false), RETRY_DELAY);
    assert_eq!(pending_gate_wait("claude", "not_idle", false), RETRY_DELAY);
}

pub(crate) struct DeliveryHost {
    pub(super) process: ProcessHost,
    pub(super) snapshot: DeliverySnapshot,
    pub(super) policy: PolicyClient,
    poll_retry: Duration,
}

impl DeliveryHost {
    pub(crate) fn new(process: ProcessHost) -> ServiceResult<Self> {
        let policy = PolicyClient(process.client().clone());
        let snapshot = match policy.call(PolicyRequest::Poll)? {
            PolicyResponse::Snapshot(snapshot) => snapshot,
            _ => return Err(ServiceError::Internal("unexpected delivery snapshot".into())),
        };
        Ok(Self { process, snapshot, policy, poll_retry: Duration::ZERO })
    }

    pub(super) fn refresh(&mut self) -> ServiceResult<()> {
        let snapshot = match self.policy.call(PolicyRequest::Poll)? {
            PolicyResponse::Snapshot(snapshot) => snapshot,
            _ => return Err(ServiceError::Internal("unexpected delivery snapshot".into())),
        };
        self.process.rebind(snapshot.actor.participant.clone()).map_err(|error| ServiceError::Unavailable(error.to_string()))?;
        self.snapshot = snapshot;
        Ok(())
    }

    pub(super) fn refresh_for_delivery(&mut self, notify: &NotifyServer, running: &AtomicBool) -> bool {
        if !running.load(Ordering::Acquire) {
            return false;
        }
        match self.refresh() {
            Ok(()) => {
                self.poll_retry = Duration::ZERO;
                running.load(Ordering::Acquire)
            }
            Err(error) => {
                if self.poll_retry.is_zero() {
                    log_warn("native", "delivery.snapshot_failed", &error.to_string());
                }
                self.poll_retry = (self.poll_retry * 2).clamp(RETRY_DELAY, Duration::from_secs(4));
                // Stagger hosts after a shared outage. Wakes are drained but do
                // not start a new RPC before the retry deadline.
                let until = Instant::now() + self.poll_retry + Duration::from_millis(rand::random_range(0..=250));
                while running.load(Ordering::Acquire) {
                    let remaining = until.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        break;
                    }
                    notify.wait(RETRY_DELAY.min(remaining));
                }
                false
            }
        }
    }

    fn refresh_current(&mut self, participant: &ParticipantRef, notify: &NotifyServer, running: &AtomicBool) -> bool {
        self.refresh_for_delivery(notify, running) && self.snapshot.actor.participant == *participant
    }

    pub(super) fn endpoint(&self, kind: &str) -> Option<u16> {
        self.snapshot.endpoints.iter().find(|(candidate, port)| candidate == kind && *port > 0).map(|(_, port)| *port)
    }

    fn in_listen(&self) -> bool {
        self.endpoint("listen").is_some() || self.endpoint("listen_filter").is_some()
    }

    fn status(&self) -> ServiceResult<Option<(String, String)>> {
        Ok(Some((self.snapshot.actor.record.status.clone(), self.snapshot.actor.record.status_context.clone())))
    }

    pub(super) fn update_titles(&self, shared_name: &Option<Arc<RwLock<String>>>, shared_status: &Option<Arc<RwLock<String>>>, title_wake: &Option<TitleWake>, tool: &str, label: &mut host_label::HostLabel) {
        let mut changed = false;
        let display = crate::agents::launch_title::pane_display_from_record(&self.snapshot.actor.record);
        for (shared, next) in [(shared_name, &display), (shared_status, &self.snapshot.actor.record.status)] {
            if let Some(shared) = shared
                && let Ok(mut current) = shared.write()
                && *current != *next
            {
                *current = next.clone();
                changed = true;
            }
        }
        if changed && let Some(wake) = title_wake {
            wake();
        }
        label.sync_display(&display, &self.snapshot.actor.record.status, tool);
    }

    pub(crate) fn publish_endpoints(&self, wake_port: u16, injection_port: u16) -> anyhow::Result<()> {
        self.process.set_endpoint(EndpointKind::Wake(crate::runtime::notify::WakeKind::Pty), wake_port)?;
        self.process.set_endpoint(EndpointKind::Injection, injection_port)
    }

    pub(super) fn restore_missing_endpoints(&self, wake_port: u16, injection_port: u16) -> anyhow::Result<()> {
        for (kind, port) in [(EndpointKind::Wake(crate::runtime::notify::WakeKind::Pty), wake_port), (EndpointKind::Injection, injection_port)] {
            if self.endpoint(kind.as_str()) != Some(port) {
                self.process.set_endpoint(kind, port)?;
            }
        }
        Ok(())
    }

    pub(super) fn heartbeat(&self) {
        if !heartbeat_due(&self.snapshot.actor.record, crate::shared::time::now_epoch_i64()) {
            return;
        }
        if let Err(error) = self.policy.call(PolicyRequest::Heartbeat { registration: self.process.registration().clone() }) {
            log_warn("native", "delivery.heartbeat_fail", &error.to_string());
        }
    }

    pub(super) fn launch(&self, observation: LaunchObservation) -> ServiceResult<bool> {
        match self.policy.call(PolicyRequest::ObserveLaunch { registration: self.process.registration().clone(), observation })? {
            PolicyResponse::Changed(changed) => Ok(changed),
            _ => Err(ServiceError::Internal("unexpected launch observation response".into())),
        }
    }

    fn gate(&self, observation: GateObservation) -> ServiceResult<bool> {
        match self.policy.call(PolicyRequest::ObserveGate { registration: self.process.registration().clone(), observation })? {
            PolicyResponse::Changed(changed) => Ok(changed),
            _ => Err(ServiceError::Internal("unexpected gate observation response".into())),
        }
    }

    fn recover_idle(&self) -> ServiceResult<bool> {
        self.gate(GateObservation::RecoveredIdle { previous_context: self.snapshot.actor.record.status_context.clone() })
    }

    /// Check the terminal twice around the bounded local rollout read. The
    /// daemon compares the resulting observation to the latest durable state.
    fn reconcile_codex_turn(&self, state: &DeliveryState, config: &ToolConfig) -> ServiceResult<bool> {
        use crate::agents::model_request::{ERROR_CONTEXT, RequestPhase};
        let row = &self.snapshot.actor.record;
        let Some(model) = row.model_request.as_ref().filter(|model| model.error.is_none() && matches!(model.phase, RequestPhase::Started | RequestPhase::Completed)) else { return Ok(false) };
        let Some(progress_at) = model.last_progress_at else { return Ok(false) };
        if row.tool != "codex" || row.status != ST_ACTIVE || row.status_context != ERROR_CONTEXT || row.session_id.as_deref() != Some(&model.session_id) || self.snapshot.pending.is_empty() || !evaluate_gate(config, state, true).safe {
            return Ok(false);
        }
        let input_stamp = || {
            let screen = state.screen.read().ok()?;
            (screen.ready && screen.prompt_empty && !screen.approval && !screen.nav_overlay && !screen.startup_loading).then_some((screen.last_user_input, screen.last_prompt_submit))
        };
        let Some(before) = input_stamp() else { return Ok(false) };
        let Some(completed_at_ms) = crate::transcript::codex_completion::latest_turn_completion(Path::new(&row.transcript_path), &model.session_id, &model.turn_id) else { return Ok(false) };
        if input_stamp() != Some(before) || !evaluate_gate(config, state, true).safe || chrono::Utc::now().timestamp_millis().saturating_sub(completed_at_ms) < 2_000 || completed_at_ms < progress_at.max(row.status_time).saturating_mul(1000) {
            return Ok(false);
        }
        self.gate(GateObservation::CodexTurnCompleted { session: model.session_id.clone(), turn: model.turn_id.clone(), phase: model.phase, progress_at, status_time: row.status_time, cursor: row.last_event_id, completed_at_ms })
    }

    fn recovery(&self, ticket: &Ticket, action: HostAction) -> ServiceResult<bool> {
        if self.process.registration().participant != ticket.target.participant {
            return Ok(false);
        }
        match self.policy.call(PolicyRequest::Recovery { registration: self.process.registration().clone(), ticket: ticket.clone(), action })? {
            PolicyResponse::Changed(changed) => Ok(changed),
            _ => Err(ServiceError::Internal("unexpected recovery response".into())),
        }
    }

    fn block_gate(&self, context: &str, detail: &str) -> ServiceResult<bool> {
        self.gate(GateObservation::Blocked { cursor: self.snapshot.actor.record.last_event_id, context: context.to_owned(), detail: detail.to_owned() })
    }

    fn clear_gate(&self, context: &str) -> ServiceResult<bool> {
        self.gate(GateObservation::Cleared { cursor: self.snapshot.actor.record.last_event_id, context: context.to_owned() })
    }

    fn report_mod_health(&self) -> ServiceResult<()> {
        HostService::call(self.process.client(), HostRequest::ObserveClaudeModHealth { registration: self.process.registration().clone() })?;
        Ok(())
    }

    fn wake_kind(&self, kind: &str) {
        if let Some(port) = self.endpoint(kind) {
            crate::runtime::notify::wake_ports(&[port], crate::runtime::notify::WAKE_TARGETED_MS);
        }
    }

    fn wake_plugin(&self) {
        self.wake_kind("plugin");
    }

    fn wake_preview(&self, max_len: usize) -> String {
        let Some(message) = self.snapshot.pending.first() else {
            return WAKE_TRIGGER.to_owned();
        };
        let recipient = sanitize_wake_preview_part(&self.snapshot.actor.record.name);
        let sender = sanitize_wake_preview_part(&message.from);
        let first_line = format!("{} {sender} -> {recipient}", wake_message_prefix(message));
        let inner = if self.snapshot.pending.len() == 1 { first_line } else { format!("[{} new messages] | {first_line}", self.snapshot.pending.len()) };
        let preview = format!("<agentx>{inner}</agentx>");
        if preview.chars().count() > max_len || preview.contains('@') { WAKE_TRIGGER.to_owned() } else { preview }
    }

    pub(super) fn finish(&self) {
        self.process.release_endpoints();
        let exit = CHILD_EXIT.lock().ok().and_then(|mut exit| exit.take());
        let (exit_code, output) = exit.map(|(code, output)| (Some(code), output)).unwrap_or_default();
        let request = PolicyRequest::Finish { registration: self.process.registration().clone(), killed: EXIT_WAS_KILLED.load(Ordering::Acquire), exit_code, output };
        if let Err(error) = self.policy.call(request) {
            log_warn("native", "delivery.finish_failed", &error.to_string());
        }
    }
}

/// Continue only this failed turn. This path never wakes or drains the mailbox.
fn drive_recovery(host: &DeliveryHost, state: &DeliveryState, config: &ToolConfig, ticket: &Ticket) -> ServiceResult<()> {
    use crate::agents::recovery::PROMPT;
    if !evaluate_gate(config, state, true).safe || !host.recovery(ticket, HostAction::Claim)? {
        return Ok(());
    }

    let submitted = (|| {
        if !evaluate_gate(config, state, true).safe || !inject_text(state.inject_port, PROMPT) {
            return Ok(false);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            let screen = state.screen.read().unwrap();
            if state.is_user_active(&screen) || screen.approval || screen.nav_overlay {
                return Ok(false);
            }
            let ownership = submit_ownership(&screen, PROMPT, false);
            drop(screen);
            match ownership {
                PromptOwnership::Exclusive => {
                    if !host.recovery(ticket, HostAction::Enter)? {
                        return Ok(false);
                    }
                    let screen = state.screen.read().unwrap();
                    return Ok(!state.is_user_active(&screen) && !screen.approval && !screen.nav_overlay && submit_ownership(&screen, PROMPT, false) == PromptOwnership::Exclusive && inject_enter(state.inject_port));
                }
                PromptOwnership::Mixed => return Ok(false),
                _ => std::thread::sleep(Duration::from_millis(20)),
            }
        }
        Ok(false)
    })();
    let recorded = host.recovery(ticket, HostAction::Finish { submitted: submitted.as_ref().is_ok_and(|value| *value) });
    submitted.and(recorded).map(|_| ())
}

pub(super) fn drive_launch_outcome(host: &DeliveryHost, state: &DeliveryState, current_name: &str, current_status: &str, config: &ToolConfig, launch_outcome: &mut LaunchOutcome) {
    match *launch_outcome {
        LaunchOutcome::Pending => {
            if launch_ready_observed(host, current_name, config, state) {
                emit_launch_ready_once(host, state, current_name, launch_outcome);
            } else {
                maybe_emit_launch_blocked(host, state, current_name, current_status, config, launch_outcome);
            }
        }
        LaunchOutcome::Blocked => {
            if launch_ready_observed(host, current_name, config, state) {
                emit_launch_ready_once(host, state, current_name, launch_outcome);
            }
        }
        LaunchOutcome::Ready | LaunchOutcome::Failed => {}
    }
}

fn launch_ready_observed(host: &DeliveryHost, _name: &str, config: &ToolConfig, state: &DeliveryState) -> bool {
    let screen = state.screen.read().unwrap();
    if config.block_on_approval && screen.approval {
        return false;
    }
    // Copilot's SessionStart hook binds the real CLI session after startup and
    // only after the initial prompt has completed. That binding is authoritative
    // readiness evidence even when newer Copilot versions omit or redraw the
    // historical "/ commands" footer before the screen scraper observes it.
    if config.tool == "copilot" && host.snapshot.has_session {
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
        return host.endpoint("plugin").is_some();
    }
    if config.launch_requires_ready && !screen.ready {
        return false;
    }
    if config.require_prompt_empty && !screen.prompt_empty {
        return false;
    }
    true
}

fn emit_launch_ready_once(host: &DeliveryHost, state: &DeliveryState, _current_name: &str, outcome: &mut LaunchOutcome) {
    // Allow Pending → Ready (first readiness) and Blocked → Ready (recovery,
    // e.g. user accepted agy's trust-folder prompt after launch_blocked fired).
    // Ready/Failed are terminal and re-fire is a no-op.
    let was_blocked = matches!(outcome, LaunchOutcome::Blocked);
    if !outcome.is_pending() && !was_blocked {
        return;
    }
    if let Err(error) = host.launch(LaunchObservation::Ready { cleared_block: was_blocked }) {
        log_warn("native", "delivery.launch_ready_event_fail", &error.to_string());
        return;
    }
    mark_launch_phase_complete(state, outcome, LaunchOutcome::Ready);
}

fn emit_launch_failed_if_needed(host: &DeliveryHost, state: &DeliveryState, current_name: &str, outcome: &mut LaunchOutcome, reason: &str) {
    if !outcome.is_pending() || !state.launch_phase_active.load(Ordering::Acquire) || std::env::var("AGENTX_LAUNCHED").as_deref() != Ok("1") {
        return;
    }
    let detail = "launch failed: readiness was never observed before the PTY delivery loop exited";
    if let Err(e) = host.launch(LaunchObservation::Failed { reason: reason.to_owned(), detail: detail.to_owned() }) {
        log_warn("native", "delivery.launch_failed_event_fail", &format!("Failed to emit launch_failed for {}: {}", current_name, e));
    }
    mark_launch_phase_complete(state, outcome, LaunchOutcome::Failed);
}

fn emit_launch_blocked_once(host: &DeliveryHost, state: &DeliveryState, _current_name: &str, outcome: &mut LaunchOutcome, detail: &str) {
    if !outcome.is_pending() || std::env::var("AGENTX_LAUNCHED").as_deref() != Ok("1") {
        return;
    }

    if let Err(error) = host.launch(LaunchObservation::Blocked { detail: detail.to_owned() }) {
        log_warn("native", "delivery.launch_blocked_event_fail", &error.to_string());
        return;
    }
    mark_launch_phase_complete(state, outcome, LaunchOutcome::Blocked);
}

fn maybe_emit_launch_blocked(host: &DeliveryHost, state: &DeliveryState, current_name: &str, current_status: &str, config: &ToolConfig, outcome: &mut LaunchOutcome) {
    // Plugin-driven tools bind their extension slightly after the TUI settles;
    // give that bind a generous grace so a slow-but-valid launch is not
    // transient-blocked (drive_launch_outcome would recover it to Ready, but the
    // spurious blocked event is noisy). A genuinely dead extension still blocks
    // once the grace elapses with no kind='plugin' endpoint.
    const SETTLE_THRESHOLD: Duration = Duration::from_millis(1500);
    const PLUGIN_BIND_GRACE: Duration = Duration::from_secs(10);
    let settle_threshold = if config.launch_ready_on_plugin_bind { PLUGIN_BIND_GRACE } else { SETTLE_THRESHOLD };

    if !outcome.is_pending() || current_status == ST_ACTIVE {
        return;
    }

    let screen = state.screen.read().unwrap();
    if screen.startup_loading {
        return;
    }
    let tail_text = screen.visible_tail.as_deref().unwrap_or("");
    // Gemini's animated startup banner keeps emitting output for ~60s, defeating
    // the settle heuristic. Its trust prompt is distinctive — fire immediately
    // when it appears rather than waiting for the banner animation to stop.
    let trust_prompt_visible = tail_text.contains("Do you trust the files in this folder?");
    if !trust_prompt_visible && screen.last_output.elapsed() < settle_threshold {
        return;
    }
    let Some(tail) = screen.visible_tail.as_deref().filter(|s| !s.trim().is_empty()) else {
        return;
    };

    let reason = if config.launch_ready_on_plugin_bind { "screen settled but the agentx plugin never connected" } else { "screen settled before readiness" };
    let detail = format!("launch blocked: {reason}; run `agentx term {}`\n{}", current_name, tail);
    drop(screen);
    emit_launch_blocked_once(host, state, current_name, outcome, &detail);
}

fn start_plugin_session(host: &DeliveryHost, state: &DeliveryState, name: &str) -> bool {
    use crate::runtime::notify::WakeKind;
    if host.endpoint(WakeKind::SessionStart.as_str()).is_some() {
        host.wake_kind(WakeKind::SessionStart.as_str());
        log_info("native", "delivery.session_start_wake", &format!("{}: asked the plugin to start a session", name));
        return false;
    }
    let cols = state.screen.read().map(|s| s.cols).unwrap_or(80);
    let input_box_width = (cols as usize).saturating_sub(15).max(10);
    let text = host.wake_preview(input_box_width);
    if !inject_text(state.inject_port, &text) {
        return false;
    }
    // No prompt-text parser here, so give the TUI time to render the text before Enter.
    std::thread::sleep(Duration::from_millis(800));
    if !inject_enter(state.inject_port) {
        return false;
    }
    log_info("native", "delivery.bootstrap_inject", &format!("Bootstrap inject for {}: '{}'", name, truncate_chars(&text, 40)));
    true
}

fn watch_extension_stall(host: &DeliveryHost, name: &str, since: &mut Option<Instant>, warned: &mut bool) {
    if !(!host.snapshot.pending.is_empty() && host.snapshot.actor.record.status == ST_LISTENING) {
        *since = None;
        *warned = false;
        return;
    }
    let started = *since.get_or_insert_with(Instant::now);
    if *warned || started.elapsed() < EXTENSION_STALL_WARN {
        return;
    }
    *warned = true;
    let detail = match host.endpoint("plugin") {
        None => "the agentx Copilot extension never registered; check `/extensions` in Copilot \
                 and its logs (~/.copilot/logs), or `agentx config copilot_delivery inject`"
            .to_string(),
        Some(port) => {
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
            if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
                format!(
                    "the agentx Copilot extension answers on port {port} but has not delivered; \
                     look for plugin.copilot_* events in agentx.log"
                )
            } else {
                format!(
                    "the agentx Copilot extension stopped (port {port} refused); Copilot restarts \
                     extensions on /clear, or check `/extensions`"
                )
            }
        }
    };
    log_warn("native", "delivery.copilot_extension_stalled", &format!("{name}: messages pending {}s while idle; {detail}", started.elapsed().as_secs()));
}

#[allow(clippy::too_many_arguments)] // Tracked: hook-comms-8vs (refactor delivery loop)
pub(crate) fn run_delivery_loop_client(
    running: Arc<AtomicBool>,
    host: &mut DeliveryHost,
    notify: &NotifyServer,
    state: &DeliveryState,
    _instance_name: &str,
    config: &ToolConfig,
    shared_name: Option<Arc<std::sync::RwLock<String>>>,
    shared_status: Option<Arc<std::sync::RwLock<String>>>,
    title_wake: Option<TitleWake>,
) {
    let process_id = Config::get().process_id.unwrap_or_default();
    let mut current_name = host.snapshot.actor.record.name.clone();
    let claude_mod = config.tool == "claude" && crate::integrations::hooks::claude::native::launched_native();
    let copilot_extension = config.tool == "copilot" && crate::integrations::hooks::copilot::extension_delivery_enabled();
    let mut launch_outcome = LaunchOutcome::Pending;
    // Resolve once: only delivery-loop iterations push labels, so a single
    // backend handle (or None) is captured at startup. First iteration will
    // push the initial label, subsequent iterations only push on change.
    let mut host_label = host_label::HostLabel::resolve();

    // Plugin tools (OpenCode, Kilo, Pi, OMP, Hermes): the plugin delivers once it
    // has a session. Pi, OMP and Hermes bind theirs at startup; OpenCode and Kilo
    // create one on the first prompt, so mail that waits before then needs a
    // session started.
    use crate::integrations::tool::Tool;
    use std::str::FromStr;
    if let Some(launch) = state.grok_acp.as_ref() {
        grok::run_client(launch, &running, host, notify, state, &process_id, &mut current_name, config, &shared_name, &shared_status, &title_wake, &mut host_label, &mut launch_outcome);
    } else if let Ok(tool) = Tool::from_str(&config.tool)
        && (matches!(tool, Tool::OpenCode | Tool::Kilo | Tool::Pi | Tool::Omp | Tool::Hermes) || claude_mod || copilot_extension)
    {
        log_info("native", "delivery.plugin_mode", &format!("Plugin mode for {}: the plugin handles delivery (tool={}, claude_mod={}, copilot_extension={})", current_name, config.tool, claude_mod, copilot_extension));
        // Only OpenCode and Kilo need a first session started; a Claude mod or
        // Copilot extension never bootstraps by typing into the terminal.
        let mut session_started = !matches!(tool, Tool::OpenCode | Tool::Kilo);
        // Mail sent before this thread registered its notify port never wakes
        // it, so the first pass checks for pending mail without waiting.
        let mut first_pass = true;
        // Mail is waiting for the TUI to come up; poll its screen.
        let mut awaiting_ready = false;
        let loop_started = Instant::now();
        let mut was_idle = true;
        let mut stall_since: Option<Instant> = None;
        let mut stall_warned = false;

        while running.load(Ordering::Acquire) {
            // One poll per notification/timeout; the first pass handles mail
            // that arrived before the host registered its wake endpoint.
            if !std::mem::take(&mut first_pass) {
                notify.wait(if awaiting_ready {
                    RETRY_DELAY
                } else if claude_mod {
                    Duration::from_secs(1)
                } else {
                    staggered_idle_wait()
                });
            }
            if !host.refresh_for_delivery(notify, &running) {
                first_pass = true; // Retry after the RPC backoff, not an idle wait.
                continue;
            }
            host.update_titles(&shared_name, &shared_status, &title_wake, &config.tool, &mut host_label);
            current_name = host.snapshot.actor.record.name.clone();
            let current_status = host.snapshot.actor.record.status.clone();
            drive_launch_outcome(host, state, &current_name, &current_status, config, &mut launch_outcome);

            // Claude needs a few seconds to start and load the mod; reporting
            // before then flashes a false "unavailable" on every launch.
            if claude_mod
                && loop_started.elapsed() >= crate::integrations::hooks::claude::native::CONNECT_GRACE
                && let Err(error) = host.report_mod_health()
            {
                log_warn("native", "claude.mod_health_failed", &error.to_string());
            }
            if copilot_extension {
                watch_extension_stall(host, &current_name, &mut stall_since, &mut stall_warned);
            }

            // The user typed first, or the session was resumed.
            if !session_started && host.snapshot.has_session {
                session_started = true;
                log_info("native", "delivery.session_exists", &format!("{}: session exists, plugin handles delivery", current_name));
            }
            awaiting_ready = false;
            if !session_started && !host.snapshot.pending.is_empty() {
                let ready = state.screen.read().map(|s| s.ready).unwrap_or(false);
                awaiting_ready = !ready;
                if ready {
                    session_started = start_plugin_session(host, state, &current_name);
                }
            }

            // Escape fires no Stop hook, so status stays "active" and the mod
            // never wakes. Same recovery as PTY delivery, plus a visible prompt.
            if claude_mod && !host.snapshot.pending.is_empty() {
                let quiet_prompt = state.screen.read().is_ok_and(|s| s.ready && !s.approval && s.last_output.elapsed() > Duration::from_secs(10));
                if quiet_prompt && !host.in_listen() && matches!(host.status(), Ok(Some((s, _))) if s == ST_ACTIVE) {
                    match host.recover_idle() {
                        Ok(_) => log_info("native", "delivery.recovered", "Status recovered: prompt visible, output stable 10s, active -> listening"),
                        Err(e) => log_warn("native", "delivery.set_status_fail", &e.to_string()),
                    }
                }
            }
            // Status changes wake this loop but not the mod's poll, which would
            // otherwise sleep out its wait after Stop. Forward each idle edge.
            if claude_mod {
                let idle = host.snapshot.actor.record.status == ST_LISTENING;
                if idle && !was_idle && !host.snapshot.pending.is_empty() {
                    host.wake_plugin();
                }
                was_idle = idle;
            }

            // Heartbeat + port re-registration
            host.heartbeat();
            if let Err(e) = host.restore_missing_endpoints(notify.port(), state.inject_port) {
                log_warn("native", "delivery.register_endpoints_fail", &format!("{}", e));
            }
        }
    } else {
        // Active delivery mode (existing state machine)

        // State machine
        let mut delivery_state = State::Pending; // Start pending to check immediately
        let mut attempt: u32 = 0;
        let mut inject_attempt: u32 = 0;
        let mut enter_attempt: u32 = 0;
        let mut injected_text = String::new();
        let mut submitting_leftover = false;
        let mut phase_started_at = Instant::now();
        let mut cursor_before: i64 = 0;
        // Gate block tracking for TUI status updates
        let mut block_since: Option<Instant> = None;
        let mut last_block_context: String = String::new();
        let mut participant = host.snapshot.actor.participant.clone();
        let mut idle_snapshot_ready = false;
        let mut next_completion_probe = Instant::now();

        while running.load(Ordering::Acquire) {
            // Looking at screen rendering requires no daemon state. Refresh
            // before any Enter and again when verifying the mailbox cursor.
            if !matches!(delivery_state, State::WaitTextRender | State::WaitTextClear) {
                if !std::mem::take(&mut idle_snapshot_ready) && !host.refresh_for_delivery(notify, &running) {
                    continue;
                }
                if host.snapshot.actor.participant != participant {
                    participant = host.snapshot.actor.participant.clone();
                    next_completion_probe = Instant::now();
                    // A same-name replacement cannot confirm the old wake.
                    delivery_state = State::Idle;
                    last_block_context.clear();
                    block_since = None;
                    attempt = 0;
                    inject_attempt = 0;
                }
                host.update_titles(&shared_name, &shared_status, &title_wake, &config.tool, &mut host_label);
                current_name = host.snapshot.actor.record.name.clone();
                let current_status = &host.snapshot.actor.record.status;
                drive_launch_outcome(host, state, &current_name, current_status, config, &mut launch_outcome);
            }

            if config.tool == "codex"
                && matches!(delivery_state, State::Idle | State::Pending)
                && let Some(ticket) = &host.snapshot.recovery
                && let Err(error) = drive_recovery(host, state, config, ticket)
            {
                log_warn("native", "delivery.recovery_failed", &error.to_string());
            }

            match delivery_state {
                State::Idle => {
                    // Capture wall clock before wait to detect system sleep
                    let wall_before = crate::shared::time::now_epoch_i64() as u64;

                    // Recheck launch readiness promptly while the TUI is still
                    // painting its initial screen. Some tools can start the
                    // delivery loop just before their input prompt appears.
                    // Session switches create a new identity before it has endpoints.
                    if let Err(e) = host.restore_missing_endpoints(notify.port(), state.inject_port) {
                        log_warn("native", "delivery.register_endpoints_fail", &format!("{e}"));
                    }
                    let idle_wait = if host.snapshot.recovery.is_some() || matches!(launch_outcome, LaunchOutcome::Pending | LaunchOutcome::Blocked) { RETRY_DELAY } else { staggered_idle_wait() };
                    let notified = notify.wait(idle_wait);

                    if !running.load(Ordering::Acquire) {
                        log_info("native", "delivery.shutdown", "Running flag cleared, exiting loop");
                        break;
                    }

                    // Detect sleep/wake: wall clock jumped more than expected for IDLE_WAIT
                    let wall_after = crate::shared::time::now_epoch_i64() as u64;
                    let wall_elapsed = wall_after.saturating_sub(wall_before);
                    if wall_elapsed > 45 {
                        log_info("native", "delivery.sleep_wake", &format!("System sleep detected for {}: wall clock jumped {}s during 30s poll", current_name, wall_elapsed));
                    }

                    // A stop + resume may have happened during the wait.
                    if !host.refresh_current(&participant, notify, &running) {
                        continue;
                    }

                    // Heartbeat (also re-asserts tcp_mode=true) + wake state.
                    host.heartbeat();
                    // Check for pending messages
                    let has_pending = !host.snapshot.pending.is_empty();
                    if has_pending {
                        log_info("native", "delivery.wake", &format!("Woke up (notified={}) with pending messages for {}", notified, current_name));
                        delivery_state = State::Pending;
                    } else {
                        // The next idle iteration can act on this same observation.
                        idle_snapshot_ready = true;
                        if notified {
                            log_info("native", "delivery.wake_no_pending", &format!("Woke up (notified=true) but no pending messages for {}", current_name));
                        }
                    }
                }

                State::Pending => {
                    // Check if still pending
                    if host.snapshot.pending.is_empty() {
                        log_info("native", "delivery.no_pending", &format!("No pending messages for {}", current_name));
                        delivery_state = State::Idle;
                        attempt = 0;
                        continue;
                    }

                    // Evaluate gate
                    let is_idle = if config.require_idle { host.snapshot.actor.record.status == ST_LISTENING } else { true };

                    let gate = evaluate_gate(config, state, is_idle);
                    if config.tool == "codex" && gate.reason == "not_idle" && Instant::now() >= next_completion_probe {
                        next_completion_probe = Instant::now() + Duration::from_secs(5);
                        match host.reconcile_codex_turn(state, config) {
                            Ok(true) => continue, // Re-read the exact current gate before typing.
                            Ok(false) => {}
                            Err(error) => log_warn("native", "delivery.completion_reconcile_failed", &error.to_string()),
                        }
                    }
                    // Codex hooks wake this host when the turn becomes idle.
                    // Recheck periodically in case a wake is lost, without
                    // polling for local screen changes that cannot open it.
                    let retry = pending_gate_wait(&config.tool, gate.reason, host.snapshot.recovery.is_some());

                    if gate.safe {
                        log_info("native", "delivery.gate_pass", &format!("Gate passed, injecting to port {}", state.inject_port));

                        // Snapshot cursor before injection
                        cursor_before = host.snapshot.actor.record.last_event_id;

                        // The prompt already holds our `<agentx>`: type nothing and
                        // let the render phase see it as this attempt's text. It
                        // re-checks ownership, approval and typing from a fresh
                        // snapshot before the Enter, then verifies as usual.
                        if gate.submits_leftover() {
                            log_info("native", "delivery.leftover_wake", "Prompt holds only a leftover wake; submitting it with Enter");
                            injected_text = WAKE_TRIGGER.to_string();
                            submitting_leftover = true;
                            phase_started_at = Instant::now();
                            enter_attempt = 0;
                            delivery_state = State::WaitTextRender;
                            continue;
                        }

                        // Claude/Codex hooks show the full delivery in the TUI, so
                        // they only need a trigger. Gemini-style paths use a
                        // compact, prompt-safe preview for human visibility.
                        use crate::integrations::tool::Tool;
                        use std::str::FromStr;

                        let parsed_tool = Tool::from_str(&config.tool).ok();
                        let cols = state.screen.read().map(|s| s.cols).unwrap_or(80);
                        let input_box_width = (cols as usize).saturating_sub(15).max(10);
                        let text = match parsed_tool {
                            Some(Tool::Claude) | Some(Tool::Codex) | Some(Tool::Cursor) | Some(Tool::Kimi) | Some(Tool::Copilot) | Some(Tool::Qoder) | Some(Tool::Pi) | Some(Tool::Omp) => WAKE_TRIGGER.to_string(),
                            _ => host.wake_preview(input_box_width),
                        };

                        if inject_text(state.inject_port, &text) {
                            log_info("native", "delivery.injected", &format!("Injected '{}' (len={}, inject_attempt={})", truncate_chars(&text, 40), text.len(), inject_attempt));
                            injected_text = text;
                            submitting_leftover = false;
                            phase_started_at = Instant::now();
                            enter_attempt = 0;
                            delivery_state = State::WaitTextRender;
                            continue; // Skip retry delay - now in WaitTextRender phase
                        } else {
                            log_warn("native", "delivery.inject_fail", "TCP inject failed");
                            attempt += 1;
                        }
                    } else {
                        // Gate blocked - refresh liveness so we don't go stale while
                        // waiting (DB status is still "listening" until the message is
                        // delivered and hooks fire). This path can hold for a long time
                        // behind an approval prompt, so it publishes wake state too.
                        host.heartbeat();

                        // Log gate failure
                        if attempt == 0 || attempt.is_multiple_of(5) {
                            let screen = state.screen.read().unwrap();
                            log_info("native", "delivery.gate_blocked", &format!("Gate blocked: {} (attempt={}, ready={}, approval={}, user_active={})", gate.reason, attempt, screen.ready, screen.approval, state.is_user_active(&screen)));
                        }

                        // Track when blocking started
                        if block_since.is_none() {
                            block_since = Some(Instant::now());
                        }

                        let approval_showing = {
                            let screen = state.screen.read().unwrap();
                            screen.approval
                        };
                        // Codex Stop/Interrupt hooks own idle status; quiet
                        // terminal output can still mean active work.
                        if config.tool != "codex" && !approval_showing && gate.reason == "not_idle" {
                            // Stability-based recovery: if status stuck "active" but output stable 10s,
                            // or stale PTY approval was left behind after the PTY cleared,
                            // flip back to listening.
                            // NOTE: stability tracking has false positives from escape sequences,
                            // but still useful for true idle detection when no data arrives at all.
                            match host.status() {
                                Ok(Some((status, _))) if status == ST_ACTIVE => {
                                    let screen = state.screen.read().unwrap();
                                    let stable_10s = screen.last_output.elapsed().as_millis() > 10000;
                                    drop(screen);
                                    if stable_10s && !host.in_listen() {
                                        if let Err(e) = host.recover_idle() {
                                            log_warn("native", "delivery.set_status_fail", &format!("Failed to set recovered status: {}", e));
                                        }
                                        log_info("native", "delivery.recovered", &format!("Status recovered: output stable 10s, {} -> listening", status));
                                        attempt = 0;
                                        continue;
                                    }
                                }
                                Ok(Some(_)) | Ok(None) => {
                                    // Status not "active" or not found - skip recovery
                                }
                                Err(e) => {
                                    log_error("native", "delivery.recovery_check", &format!("DB error checking status: {}", e));
                                }
                            }
                        }
                        if host.snapshot.actor.record.status == ST_LISTENING
                            && let Some(since) = block_since
                        {
                            // After 2 seconds of blocking, update TUI status context
                            if since.elapsed().as_secs_f64() >= 2.0 {
                                let context = format!("tui:{}", gate.reason.replace('_', "-"));
                                if context != last_block_context {
                                    let detail = gate_block_detail(gate.reason);
                                    match host.block_gate(&context, detail) {
                                        Ok(true) => last_block_context = context,
                                        Ok(false) => {}
                                        Err(e) => log_warn("native", "delivery.gate_status_fail", &format!("{e}")),
                                    }
                                }
                            }
                        }

                        attempt += 1;
                    }

                    // TCP notify handles the fast path.
                    if attempt > 0 {
                        let notified = notify.wait(retry);
                        if notified {
                            attempt = 0;
                        }
                    }
                }

                State::WaitTextRender => {
                    let elapsed = phase_started_at.elapsed();

                    // Inspect the latest screen before applying the deadline. This
                    // avoids rejecting a render that completed at the timeout edge.
                    let screen = state.screen.read().unwrap();
                    let input_text = screen.input_text.clone();
                    let ready = screen.ready;
                    drop(screen);

                    // Debug: log what we see at start and every 500ms
                    if elapsed.as_millis() < 50 || elapsed.as_millis() % 500 < 50 {
                        log_info("native", "delivery.phase1_poll", &format!("t={}ms input={:?} want={} ready={}", elapsed.as_millis(), input_text.as_deref().unwrap_or("None"), truncate_chars(&injected_text, 25), ready));
                    }

                    match phase1_decision(input_text.as_deref(), &injected_text, elapsed) {
                        Phase1Decision::Rendered => {
                            log_info("native", "delivery.text_rendered", "Injected text exclusively owns the input box");

                            if let Some(settle) = pre_enter_settle(&config.tool) {
                                std::thread::sleep(settle);
                            }

                            if !host.refresh_current(&participant, notify, &running) {
                                if host.snapshot.actor.participant != participant {
                                    delivery_state = State::Idle;
                                }
                                continue;
                            }
                            // Re-check the screen after the RPC and settle.
                            // The prompt can change between render detection and Enter.
                            let screen = state.screen.read().unwrap();
                            let user_active = state.is_user_active(&screen);
                            let approval = screen.approval;
                            let ownership = submit_ownership(&screen, &injected_text, submitting_leftover);
                            drop(screen);

                            if ownership != PromptOwnership::Exclusive {
                                log_warn("native", "delivery.prompt_ownership_lost", "Prompt changed before Enter; refusing automatic submission");
                                delivery_state = unconfirmed_prompt_state(&config.tool);
                                inject_attempt += 1;
                                attempt += 1;
                                continue;
                            }

                            delivery_state = State::WaitTextClear;
                            phase_started_at = Instant::now();
                            enter_attempt = 0;

                            if !user_active && !approval {
                                log_info("native", "delivery.send_enter", "Sending Enter key");
                                inject_enter(state.inject_port);
                            } else if approval {
                                log_info("native", "delivery.enter_blocked", "Enter blocked by approval prompt");
                            } else {
                                log_info("native", "delivery.enter_blocked", "Enter blocked by user activity");
                            }
                            continue;
                        }
                        Phase1Decision::MixedPrompt => {
                            log_warn("native", "delivery.mixed_prompt", concat!("Injected text is mixed with unrelated prompt text; ", "refusing automatic submission"));
                            delivery_state = unconfirmed_prompt_state(&config.tool);
                            inject_attempt += 1;
                            attempt += 1;
                            continue;
                        }
                        Phase1Decision::TimedOut => {
                            log_warn("native", "delivery.phase1_timeout", &format!("Text render timeout after {:?}, inject_attempt={}", elapsed, inject_attempt));
                            delivery_state = unconfirmed_prompt_state(&config.tool);
                            inject_attempt += 1;
                            attempt += 1;
                            continue;
                        }
                        Phase1Decision::Waiting => {}
                    }

                    std::thread::sleep(Duration::from_millis(10));
                }

                State::WaitTextClear => {
                    let elapsed = phase_started_at.elapsed();

                    // Check if text cleared (prompt is empty)
                    let screen = state.screen.read().unwrap();
                    let input_text = screen.input_text.clone();
                    let text_cleared = input_text.as_ref().map(|t| t.is_empty()).unwrap_or(false);
                    drop(screen);

                    if text_cleared {
                        // Text cleared - verify cursor advance
                        log_info("native", "delivery.text_cleared", "Input box cleared, verifying cursor");
                        delivery_state = State::VerifyCursor;
                        phase_started_at = Instant::now();
                        continue;
                    }

                    if elapsed > PHASE2_TIMEOUT {
                        if enter_attempt < MAX_ENTER_ATTEMPTS {
                            let blocked = {
                                let screen = state.screen.read().unwrap();
                                state.is_user_active(&screen) || screen.approval
                            };
                            if blocked {
                                std::thread::sleep(RETRY_DELAY);
                                continue;
                            }
                            if !host.refresh_current(&participant, notify, &running) {
                                if host.snapshot.actor.participant != participant {
                                    delivery_state = State::Idle;
                                }
                                continue;
                            }
                            // Retry Enter with backoff.
                            let screen = state.screen.read().unwrap();
                            let user_active = state.is_user_active(&screen);
                            let approval = screen.approval;
                            let ownership = submit_ownership(&screen, &injected_text, submitting_leftover);
                            drop(screen);

                            if ownership != PromptOwnership::Exclusive {
                                log_warn("native", "delivery.prompt_ownership_lost", concat!("Prompt changed before Enter retry; ", "refusing automatic submission"));
                                delivery_state = unconfirmed_prompt_state(&config.tool);
                                inject_attempt += 1;
                                attempt += 1;
                                continue;
                            }

                            let can_send = !user_active && !approval;
                            if can_send {
                                log_info("native", "delivery.retry_enter", &format!("Retrying Enter (attempt={}, input_text={:?})", enter_attempt, input_text));
                                inject_enter(state.inject_port);
                                enter_attempt += 1;
                                phase_started_at = Instant::now();
                                let backoff = Duration::from_millis(200 * (1 << enter_attempt));
                                std::thread::sleep(backoff);
                            } else {
                                log_info("native", "delivery.enter_retry_blocked", &format!("Enter retry blocked (user_active={})", user_active));
                                std::thread::sleep(RETRY_DELAY);
                            }
                            continue;
                        }

                        // Do not retype a Codex wake whose submission is uncertain.
                        log_warn("native", "delivery.phase2_max_retries", &format!("Max Enter retries ({}) reached; submission remains unconfirmed", MAX_ENTER_ATTEMPTS));
                        delivery_state = unconfirmed_prompt_state(&config.tool);
                        inject_attempt += 1;
                        attempt += 1;
                        continue;
                    }

                    std::thread::sleep(Duration::from_millis(10));
                }

                State::VerifyCursor => {
                    let elapsed = phase_started_at.elapsed();

                    // Check if cursor advanced (hook processed messages)
                    let current_cursor = host.snapshot.actor.record.last_event_id;
                    if current_cursor > cursor_before {
                        // Success! Clear gate block status
                        if !last_block_context.is_empty() {
                            if let Err(e) = host.clear_gate(&last_block_context) {
                                log_warn("native", "delivery.gate_clear_fail", &format!("{}", e));
                            }
                            last_block_context.clear();
                        }
                        block_since = None;

                        log_info("native", "delivery.success", &format!("Cursor advanced {} -> {}, delivery successful", cursor_before, current_cursor));
                        if !host.snapshot.pending.is_empty() {
                            log_info("native", "delivery.more_pending", "More messages pending, continuing");
                            delivery_state = State::Pending;
                        } else {
                            log_info("native", "delivery.complete", "All messages delivered, going idle");
                            delivery_state = State::Idle;
                        }
                        attempt = 0;
                        inject_attempt = 0;
                        continue;
                    }

                    if elapsed > VERIFY_TIMEOUT {
                        inject_attempt += 1;
                        let has_pending = !host.snapshot.pending.is_empty();
                        let parsed_tool = Tool::from_str(&config.tool).ok();
                        let decision = verify_timeout_decision(parsed_tool, has_pending, inject_attempt);
                        log_warn("native", "delivery.verify_timeout", &format!("Cursor verify timeout (before={}, current={}, inject_attempt={}, decision={:?})", cursor_before, current_cursor, inject_attempt, decision));

                        match decision {
                            VerifyTimeoutDecision::DeliveredWithoutCursor => {
                                // Cursor advance is the primary proof, but "no
                                // pending rows" is also sufficient — avoids
                                // wedging when hook delivery succeeded but
                                // cursor bookkeeping did not advance.
                                if !last_block_context.is_empty() {
                                    if let Err(e) = host.clear_gate(&last_block_context) {
                                        log_warn("native", "delivery.gate_clear_fail", &format!("{}", e));
                                    }
                                    last_block_context.clear();
                                }
                                block_since = None;
                                log_info("native", "delivery.success_no_cursor", "Messages gone despite cursor not advancing - delivery successful");
                                delivery_state = State::Idle;
                                attempt = 0;
                                inject_attempt = 0;
                                continue;
                            }
                            VerifyTimeoutDecision::Retry => {
                                log_info("native", "delivery.retry", &format!("Retrying delivery (inject_attempt={})", inject_attempt));
                                delivery_state = State::Pending;
                                attempt += 1;
                                continue;
                            }
                            VerifyTimeoutDecision::FastFail => {
                                delivery_state = State::WakeUnacknowledged;
                                attempt = 0;
                                continue;
                            }
                            VerifyTimeoutDecision::Reset => {
                                log_warn("native", "delivery.failed", &format!("Delivery failed after {} attempts, resetting", inject_attempt));
                                delivery_state = State::Pending;
                                attempt = 0;
                            }
                        }
                    }

                    // Cursor progress is already durable; there is no reason
                    // to fetch the entire delivery snapshot on each screen tick.
                    std::thread::sleep(RETRY_DELAY);
                }

                State::WakeUnacknowledged => {
                    // A timeout cannot distinguish compaction/queued input from
                    // rejection. Keep the outstanding wake until hook progress.
                    let context = "tui:wake-unacknowledged";
                    if last_block_context != context {
                        let detail = "wake awaiting hook confirmation; automatic resend paused";
                        if let Err(e) = host.block_gate(context, detail) {
                            log_warn("native", "delivery.gate_status_fail", &format!("{}", e));
                        }
                        last_block_context = context.to_owned();
                        block_since = Some(Instant::now());
                        log_warn("native", "delivery.wake_unacknowledged", &format!("{} wake was not acknowledged for {}; leaving messages pending and stopping automatic retries", config.tool, current_name));
                    }
                    notify.wait(staggered_idle_wait());
                    if !running.load(Ordering::Acquire) {
                        break;
                    }

                    if !host.refresh_current(&participant, notify, &running) {
                        continue;
                    }
                    host.heartbeat();
                    if let Err(e) = host.restore_missing_endpoints(notify.port(), state.inject_port) {
                        log_warn("native", "delivery.register_endpoints_fail", &format!("{}", e));
                    }

                    let current_cursor = host.snapshot.actor.record.last_event_id;
                    let has_pending = !host.snapshot.pending.is_empty();
                    let next = wake_wait_state(cursor_before, current_cursor, has_pending);
                    if next != State::WakeUnacknowledged {
                        if let Err(e) = host.clear_gate(&last_block_context) {
                            log_warn("native", "delivery.gate_clear_fail", &format!("{}", e));
                        }
                        last_block_context.clear();
                        block_since = None;
                        attempt = 0;
                        inject_attempt = 0;
                        delivery_state = next;
                        log_info("native", "delivery.wake_rearmed", &format!("{} delivery rearmed for {} (cursor {} -> {}, pending={})", config.tool, current_name, cursor_before, current_cursor, has_pending));
                    }
                }
            }
        }
    } // end active delivery mode else block

    // Cleanup on exit — tear down PTY and stop instance
    log_info("native", "delivery.cleanup", &format!("Cleaning up instance {}", current_name));

    emit_launch_failed_if_needed(host, state, &current_name, &mut launch_outcome, "ready_never_observed");

    host.finish();
}
