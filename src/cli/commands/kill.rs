//! Kill command: `agentx kill <name(s)|all>`
//!
//!
//! Sends SIGTERM to process groups and optionally closes terminal panes.

use std::collections::HashSet;

use crate::cli::router::GlobalFlags;
use crate::cli::workspace::AgentFilterArgs;
#[cfg(test)]
use crate::db::AgentxDb;
#[cfg(test)]
use crate::integrations::hooks::common::stop_instance;
#[cfg(test)]
use crate::log::log_info;
#[cfg(test)]
use crate::paths;
use crate::runtime::process_tracking as pidtrack;
use crate::runtime::terminal;
use anyhow::{Result, bail};

/// Parsed arguments for `agentx kill`.
#[derive(clap::Parser, Debug)]
#[command(
    name = "kill",
    about = "Kill Agent processes",
    after_help = "Examples: agentx kill @NAME | agentx kill PID | agentx kill all\nFilter the interactive picker: agentx kill --dir . --label project=atlas\nThe picker requires selection and confirmation. Filters cannot be combined with explicit targets."
)]
pub struct KillArgs {
    /// Names, PIDs or "all"; omit to select local tracked processes.
    #[arg(conflicts_with_all = ["dir", "workspace", "labels"])]
    pub targets: Vec<String>,
    #[command(flatten)]
    filters: AgentFilterArgs,
    /// Print one JSON object describing every outcome
    #[arg(long)]
    pub json: bool,
}

pub(crate) fn help_command() -> clap::Command {
    use clap::CommandFactory;
    KillArgs::command()
}

#[cfg(test)]
pub struct KillTrackedResult {
    pub target: String,
    pub pid: u32,
    pub kill_result: terminal::KillResult,
    pub pane_closed: bool,
    pub pane_retry_command: Option<String>,
    pub preset_name: String,
    pub pane_id: String,
}

#[cfg(test)]
const EPERM_RECHECK_DELAY: std::time::Duration = std::time::Duration::from_millis(50);

/// One kill outcome, as reported by `--json`.
#[derive(serde::Serialize, Debug)]
struct KillOutcome {
    target: String,
    /// `agent` (tracked process), `orphan` (leftover process of a stopped
    /// agent), `stopped` (already stopped; nothing to signal), `bridge` (chat
    /// disconnected), `untracked` (no process; row stopped), or `error`.
    kind: &'static str,
    pid: Option<u32>,
    /// `sent`, `already_dead`, `permission_denied`, `already_stopped`,
    /// `disconnected`, `stopped`, or `error`.
    result: &'static str,
    pane_closed: bool,
    pane_retry_command: Option<String>,
    /// Command that resumes the agent, when it can be resumed.
    resume: Option<String>,
    error: Option<String>,
}

impl KillOutcome {
    fn new(target: &str, kind: &'static str, result: &'static str) -> Self {
        Self { target: target.to_string(), kind, pid: None, result, pane_closed: false, pane_retry_command: None, resume: None, error: None }
    }

    fn signalled(target: &str, kind: &'static str, pid: u32, result: terminal::KillResult, pane_closed: bool, pane_retry_command: Option<String>) -> Self {
        Self { pid: Some(pid), pane_closed, pane_retry_command, ..Self::new(target, kind, result.as_str()) }
    }

    fn resumable(mut self, name: &str) -> Self {
        self.resume = Some(format!("agentx r {name}"));
        self
    }
}

/// Kill output. Human mode prints each line as it happens; `--json` collects
/// the outcomes and prints them as one object when the command finishes.
/// Warnings stay on stderr in both modes.
struct KillReport {
    json: bool,
    outcomes: Vec<KillOutcome>,
}

impl KillReport {
    fn new(json: bool) -> Self {
        Self { json, outcomes: Vec::new() }
    }

    fn say(&self, line: impl std::fmt::Display) {
        if !self.json {
            println!("{line}");
        }
    }

    fn record(&mut self, outcome: KillOutcome) {
        self.outcomes.push(outcome);
    }

    fn record_error(&mut self, target: &str, message: String) {
        self.record(KillOutcome { error: Some(message), ..KillOutcome::new(target, "error", "error") });
    }

    fn finish(self, exit: i32) -> i32 {
        if self.json {
            println!("{}", serde_json::json!({ "ok": exit == 0, "results": self.outcomes }));
        }
        exit
    }
}

#[derive(Clone, Copy)]
enum PaneCleanupProcessState {
    Terminated,
    AlreadyDead,
    NotTerminated,
}

impl From<terminal::KillResult> for PaneCleanupProcessState {
    fn from(result: terminal::KillResult) -> Self {
        match result {
            terminal::KillResult::Sent => Self::Terminated,
            terminal::KillResult::AlreadyDead => Self::AlreadyDead,
            terminal::KillResult::PermissionDenied => Self::NotTerminated,
        }
    }
}

fn report_incomplete_pane_cleanup(process_state: PaneCleanupProcessState, retry_command: Option<&str>) -> bool {
    if matches!(process_state, PaneCleanupProcessState::NotTerminated) {
        return false;
    }
    let Some(command) = retry_command else {
        return false;
    };
    let process_message = match process_state {
        PaneCleanupProcessState::Terminated => "Process terminated",
        PaneCleanupProcessState::AlreadyDead => "Process was already terminated",
        PaneCleanupProcessState::NotTerminated => unreachable!(),
    };
    eprintln!("{process_message}, but pane remains. Retry this command with approval/escalation:");
    eprintln!("{command}");
    true
}

#[cfg(test)]
fn normalize_kill_result(name: &str, pid: u32, result: terminal::KillResult, pane_closed: bool) -> terminal::KillResult {
    if !matches!(result, terminal::KillResult::PermissionDenied) {
        return result;
    }

    let pid_str = pid.to_string();
    log_info("kill", "kill.eperm", &format!("kill(2) returned EPERM for name={} pid={}; checking if process already exited", name, pid));
    if pane_closed {
        log_info("kill", "kill.eperm_resolved", &format!("name={} pid={} resolved to already_dead because terminal pane closed", name, pid_str));
        return terminal::KillResult::AlreadyDead;
    }

    std::thread::sleep(EPERM_RECHECK_DELAY);
    if !pidtrack::is_alive(pid) {
        log_info("kill", "kill.eperm_resolved", &format!("name={} pid={} resolved to already_dead", name, pid_str));
        terminal::KillResult::AlreadyDead
    } else {
        terminal::KillResult::PermissionDenied
    }
}

#[cfg(test)]
pub fn kill_tracked_instance(db: &AgentxDb, name: &str, initiator: &str) -> Result<KillTrackedResult, String> {
    let inst = db.get_instance_full(name).map_err(|e| e.to_string())?.ok_or_else(|| format!("Agent '{}' not found", name))?;
    let pid = inst.pid.ok_or_else(|| format!("No tracked PID for '{}'", name))? as u32;
    let is_headless = inst.background != 0;
    let (result, pane_closed, pane_retry_command, preset_name, pane_id) = kill_instance(db, name, pid, &inst, is_headless, initiator);
    if result != terminal::KillResult::PermissionDenied {
        stop_instance(db, name, initiator, "killed");
    }

    Ok(KillTrackedResult { target: name.to_string(), pid, kill_result: result, pane_closed, pane_retry_command, preset_name, pane_id })
}

fn handle_remote_kill_response(name: &str, response: &serde_json::Value, report: &mut KillReport) -> Result<i32> {
    let result = &response["result"];
    let kill_result = result["kill_result"].as_str();

    // No kill_result means an RPC-level failure (e.g. timeout, protocol error).
    if kill_result.is_none() {
        crate::relay::control::require_successful_rpc_result(response.clone()).map_err(anyhow::Error::msg)?;
        bail!("Remote kill returned no kill_result");
    }
    let kill_result = match kill_result.unwrap() {
        "sent" => terminal::KillResult::Sent,
        "already_dead" => terminal::KillResult::AlreadyDead,
        "permission_denied" => terminal::KillResult::PermissionDenied,
        other => bail!("Remote kill failed for {name}: unexpected kill_result {other}"),
    };

    let pid = result["pid"].as_u64().unwrap_or(0);
    let pane_closed = result["pane_closed"].as_bool().unwrap_or(false);
    let preset_name = result["preset_name"].as_str().unwrap_or("");
    let pane_id = result["pane_id"].as_str().unwrap_or("");
    let pane_retry_command = result["pane_retry_command"].as_str();
    let pane_info = pane_info_str(pane_closed, preset_name, pane_id);
    let outcome = KillOutcome::signalled(name, "agent", u32::try_from(pid).unwrap_or(0), kill_result, pane_closed, pane_retry_command.map(str::to_string));

    if kill_result == terminal::KillResult::PermissionDenied {
        eprintln!("Permission denied to kill process group {} for '{}'", pid, name);
        report.record(outcome);
        return Ok(1);
    }

    for line in render_remote_kill_feedback(name, kill_result.as_str(), &pane_info)? {
        report.say(line);
    }
    report.record(outcome.resumable(name));
    if pane_retry_command.is_some()
        && let Some((_, device)) = crate::relay::control::split_device_suffix(name)
    {
        eprintln!("Run the pane-close retry command on remote device {device}.");
    }
    Ok(if report_incomplete_pane_cleanup(kill_result.into(), pane_retry_command) { 1 } else { 0 })
}

fn render_remote_kill_feedback(name: &str, kill_result: &str, pane_info: &str) -> Result<Vec<String>> {
    match kill_result {
        "sent" => Ok(vec![format!("Sent SIGTERM to \'{}\'{}", name, pane_info), format!("  To resume: agentx r {}", name)]),
        "already_dead" => Ok(vec![format!("\'{}\' had already exited{}", name, pane_info), format!("  To resume: agentx r {}", name)]),
        other => bail!("Remote kill failed for {name}: unexpected kill_result {other}"),
    }
}

/// Run the kill command.
pub fn run(argv: &[String], flags: &GlobalFlags) -> Result<i32> {
    use clap::Parser;
    let (stripped, _, _) = crate::cli::router::extract_global_flags_full(argv);
    let skip = usize::from(stripped.first().is_some_and(|arg| crate::cli::router::canonical_command(arg) == "kill"));
    let kill_args = match KillArgs::try_parse_from(std::iter::once("kill".to_owned()).chain(stripped.into_iter().skip(skip))) {
        Ok(a) => a,
        Err(e) => return Ok(crate::cli::commands::help::clap_usage_error("kill", &e)),
    };

    let mut targets = kill_args.targets;
    if targets.is_empty() {
        super::target_selection::require_terminal("kill", kill_args.json)?;
    }
    let session = crate::cli::client::Session::connect(flags.name.as_deref(), flags.go)?;
    let interactive = targets.is_empty();
    if !interactive {
        crate::runtime::stop_effects::claim_orphans(&session.client)?;
    }
    let mut records = if interactive { Vec::new() } else { session.records()? };
    if interactive {
        let filter = kill_args.filters.resolve(&session, false)?;
        let Some(selected) = super::target_selection::kill(&session, &filter, &mut records)? else {
            return Ok(130);
        };
        targets = selected;
    }
    let orphan_facts = if interactive {
        Vec::new()
    } else {
        let active: HashSet<_> = records.iter().filter_map(|row| row.pid.and_then(|pid| u32::try_from(pid).ok())).collect();
        pidtrack::get_orphan_processes(session.client.scope_root(), Some(&active))
    };
    let orphans = orphan_facts
        .into_iter()
        .map(|orphan| {
            let process_identity = crate::sys::process::identity(orphan.pid).filter(|identity| !identity.is_empty()).ok_or_else(|| anyhow::anyhow!("Cannot verify leftover process {}", orphan.pid))?;
            Ok(pidtrack::OrphanCandidate { orphan, process_identity })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut report = KillReport::new(kill_args.json);
    let mut retired = HashSet::new();
    let mut worst_exit = 0;
    // A picked Agent named "all" is one exact row, not the CLI bulk operand.
    if !interactive && targets.iter().any(|target| target == "all") {
        let mut count = 0;
        for row in records.iter().filter(|row| !crate::agents::records::is_remote_instance(row)) {
            if retired.contains(&crate::cli::commands::stop::participant(row)) {
                continue;
            }
            let exit = kill_native_row(&session, row, true, &mut report, &mut retired)?;
            worst_exit = worst_exit.max(exit);
            count += 1;
        }
        for orphan in &orphans {
            worst_exit = worst_exit.max(kill_native_orphan(&session, orphan, &orphan_target(&orphan.orphan), &mut report)?);
            count += 1;
        }
        report.say(format!("Killed {count}"));
        return Ok(report.finish(worst_exit));
    }
    for target in &targets {
        let attempt = (|| -> Result<i32> {
            let row =
                if interactive { records.iter().find(|row| row.name == *target) } else { crate::cli::commands::stop::selected(&records, target).or_else(|| target.parse::<i64>().ok().and_then(|pid| records.iter().find(|row| row.pid == Some(pid)))) };
            if let Some(row) = row {
                if retired.contains(&crate::cli::commands::stop::participant(row)) {
                    report.say(format!("'{}' was already stopped with its parent", row.name));
                    report.record(KillOutcome::new(target, "stopped", "already_stopped").resumable(target));
                    return Ok(0);
                }
                return kill_native_row(&session, row, false, &mut report, &mut retired);
            }
            if interactive {
                bail!("Selected Agent '{target}' is no longer available");
            }
            let matches: Vec<_> = orphans.iter().filter(|candidate| orphan_matches(&candidate.orphan, target)).collect();
            if matches.len() > 1 {
                bail!(
                    "'{target}' matches {} leftover processes: {}\n  Kill one by PID: agentx kill <pid>",
                    matches.len(),
                    matches.iter().map(|candidate| format!("{} (pid {})", orphan_target(&candidate.orphan), candidate.orphan.pid)).collect::<Vec<_>>().join(", ")
                );
            }
            if let Some(orphan) = matches.first() {
                return kill_native_orphan(&session, orphan, target, &mut report);
            }
            if let Some(stopped) = crate::cli::commands::stop::stopped(&session, target)? {
                report.say(format!("'{target}' {}", stopped.summary()));
                let outcome = KillOutcome::new(target, "stopped", "already_stopped");
                if stopped.reason == crate::integrations::hooks::hermes::bridge::DISCONNECTED {
                    report.say(crate::integrations::hooks::hermes::bridge::reconnect_hint(&stopped.name));
                    report.record(outcome);
                } else {
                    report.say(format!("  To resume: agentx r {}", stopped.name));
                    report.record(outcome.resumable(&stopped.name));
                }
                return Ok(0);
            }
            // A remote suffix is explicit authority even without a current mirror.
            if let Some((base, device)) = crate::relay::control::split_device_suffix(target) {
                let result = crate::relay::control::dispatch_remote_raw(&session.client, device, Some(target), "kill", &serde_json::json!({"target":base}), crate::relay::control::RPC_DEFAULT_TIMEOUT).map_err(anyhow::Error::msg)?;
                return handle_remote_kill_response(target, &result, &mut report);
            }
            let suggestion = crate::shared::suggest::did_you_mean(target, records.iter().map(|record| record.name.as_str()));
            bail!("Agent '{target}' not found{suggestion}")
        })();
        let exit = match attempt {
            Ok(exit) => exit,
            Err(error) if report.json => {
                eprintln!("Error: {error}");
                report.record_error(target, error.to_string());
                1
            }
            Err(error) => return Err(error),
        };
        worst_exit = worst_exit.max(exit);
    }
    Ok(report.finish(worst_exit))
}

/// Whether a tracked orphan process belongs to `target` by exact name, process id, or PID.
fn orphan_matches(orphan: &pidtrack::OrphanProcess, target: &str) -> bool {
    orphan.names.iter().any(|name| name == target) || orphan.process_id == target || target.parse::<u32>().ok() == Some(orphan.pid)
}

/// Report target for an orphan: its label, or its PID when it has no name.
fn orphan_target(orphan: &pidtrack::OrphanProcess) -> String {
    if orphan.names.is_empty() { orphan.pid.to_string() } else { orphan.names[0].clone() }
}

/// The still-running process left behind by a specific stopped session.
#[cfg(test)]
pub(crate) fn find_orphan_for_session(session_id: &str) -> Option<pidtrack::OrphanProcess> {
    if session_id.is_empty() {
        return None;
    }
    pidtrack::get_orphan_processes(&paths::agentx_dir(), None).into_iter().find(|o| o.session_id == session_id)
}

/// Format pane close info
fn pane_info_str(pane_closed: bool, preset_name: &str, pane_id: &str) -> String {
    if pane_closed {
        if !pane_id.is_empty() {
            format!(" (closed {} pane {})", preset_name, pane_id)
        } else if !preset_name.is_empty() {
            format!(" (closed {} pane)", preset_name)
        } else {
            String::new()
        }
    } else if !preset_name.is_empty()
        && let Some(preset) = crate::config::get_merged_preset(preset_name)
        && preset.has_close(cfg!(windows))
    {
        if crate::runtime::terminal::is_zellij_merged(&preset) {
            return " (zellij pane close unconfirmed)".to_string();
        }
        format!(" (pane close failed for {})", preset_name)
    } else {
        String::new()
    }
}

#[cfg(test)]
fn restore_failed_kill_status(db: &AgentxDb, instance: &crate::db::InstanceRow, initiator: &str, result: terminal::KillResult) {
    if result != terminal::KillResult::PermissionDenied {
        return;
    }
    // Restore only our pending kill marker; hooks may have updated the row
    // while we were signalling or trying to close its pane.
    if let Err(e) = db.conn().execute(
        "UPDATE instances SET status = ?, status_context = ?, status_detail = ?, status_time = ?
         WHERE name = ? AND pid = ? AND status = 'inactive'
           AND status_context = 'exit:killed' AND status_detail = ?",
        rusqlite::params![instance.status, instance.status_context, instance.status_detail, instance.status_time, instance.name, instance.pid, initiator,],
    ) {
        log_info("kill", "lifecycle.kill_status_restore", &format!("name={} err={e}", instance.name));
    }
}

/// Kill a process and close its terminal pane.
/// Returns (KillResult, pane_closed, pane_retry_command, preset_name, pane_id).
#[cfg(test)]
fn kill_instance(db: &AgentxDb, name: &str, pid: u32, instance: &crate::db::InstanceRow, is_headless: bool, initiator: &str) -> (terminal::KillResult, bool, Option<String>, String, String) {
    // The PTY cleanup thread may run as soon as the signal lands. Record the
    // requested reason and initiator first so it cannot turn an explicit kill
    // into a stopped snapshot with reason "closed" or by "pty".
    if let Err(e) = db.mark_killed(name, initiator) {
        log_info("kill", "lifecycle.kill_status", &format!("name={name} err={e}"));
    }
    // The PID now belongs to an unrelated process (reboot, crash): don't
    // signal it or resolve a pane through it. A merely dead PID falls through
    // so its group and pane are still cleaned up.
    if instance.pid_reused(pid) {
        log_info("kill", "lifecycle.kill_pid_reused", &format!("name={name} pid={pid} now belongs to another process; not signalling"));
        return (terminal::KillResult::AlreadyDead, false, None, String::new(), String::new());
    }
    // Headless instances have no terminal pane — skip pane close
    if is_headless {
        let (result, pane_closed, pane_retry_command) = terminal::kill_process(pid, &terminal::TerminalInfo::default());
        let result = normalize_kill_result(name, pid, result, pane_closed);
        restore_failed_kill_status(db, instance, initiator, result);
        log_info("kill", "lifecycle.kill", &format!("name={} pid={} result={:?} pane_closed={} headless=true", name, pid, result, pane_closed));
        return (result, pane_closed, pane_retry_command, String::new(), String::new());
    }

    let ti = terminal::resolve_terminal_info(instance.terminal_preset_effective.as_deref(), instance.launch_context.as_deref());

    let (result, pane_closed, pane_retry_command) = terminal::kill_process(pid, &ti);
    let result = normalize_kill_result(name, pid, result, pane_closed);
    restore_failed_kill_status(db, instance, initiator, result);

    log_info("kill", "lifecycle.kill", &format!("name={} pid={} result={:?} pane_closed={}", name, pid, result, pane_closed));

    (result, pane_closed, pane_retry_command, ti.preset_name.clone(), ti.pane_id.clone())
}

fn record_native_signal(target: &str, kind: &'static str, result: crate::runtime::kill_client::KillTrackedResult, report: &mut KillReport) -> i32 {
    let pane = pane_info_str(result.pane_closed, &result.preset_name, &result.pane_id);
    match result.kill_result {
        terminal::KillResult::Sent => report.say(format!("Sent SIGTERM to '{target}'{pane}")),
        terminal::KillResult::AlreadyDead => report.say(format!("'{target}' had already exited{pane}")),
        terminal::KillResult::PermissionDenied => eprintln!("Permission denied to kill process group {} for '{target}'", result.pid),
    }
    let failed = result.kill_result == terminal::KillResult::PermissionDenied;
    let incomplete = report_incomplete_pane_cleanup(result.kill_result.into(), result.pane_retry_command.as_deref());
    let outcome = KillOutcome::signalled(target, kind, result.pid, result.kill_result, result.pane_closed, result.pane_retry_command);
    if !failed && kind == "agent" {
        report.say(format!("  To resume: agentx r {target}"));
        report.record(outcome.resumable(target));
    } else {
        report.record(outcome);
    }
    i32::from(failed || incomplete)
}
fn kill_native_row(session: &crate::cli::client::Session, row: &crate::db::InstanceRow, allow_untracked: bool, report: &mut KillReport, retired: &mut HashSet<crate::agents::attachments::ParticipantRef>) -> Result<i32> {
    if crate::agents::records::is_remote_instance(row) {
        let (base, device) = crate::relay::control::split_device_suffix(&row.name).ok_or_else(|| anyhow::anyhow!("Cannot kill remote '{}' - missing device suffix", row.name))?;
        let result = crate::relay::control::dispatch_remote_raw(&session.client, device, Some(&row.name), "kill", &serde_json::json!({"target":base}), crate::relay::control::RPC_DEFAULT_TIMEOUT).map_err(anyhow::Error::msg)?;
        return handle_remote_kill_response(&row.name, &result, report);
    }
    if row.pid.is_none() {
        if let Some((note, outcome)) = crate::cli::commands::stop::disconnect_bridge(session, row)? {
            retired.extend(outcome.effects.into_iter().map(|effect| effect.participant));
            report.say(note);
            report.record(KillOutcome::new(&row.name, "bridge", "disconnected"));
            return Ok(0);
        }
        if !allow_untracked {
            bail!("No tracked PID for '{}' — use 'agentx stop {}' instead", row.name, row.name);
        }
        let outcome = crate::runtime::stop_effects::retire(&session.client, &crate::cli::commands::stop::participant(row), crate::cli::commands::stop::initiator(session), &crate::agents::hook_service::OperatorStop::KilledWithoutProcess)?;
        retired.extend(outcome.effects.into_iter().map(|effect| effect.participant));
        report.say(format!("No tracked process for '{}', stopping instance.", row.name));
        report.record(KillOutcome::new(&row.name, "untracked", "stopped"));
        return Ok(0);
    }
    let result = crate::runtime::kill_client::kill(&session.client, &crate::cli::commands::stop::participant(row), crate::cli::commands::stop::initiator(session)).map_err(anyhow::Error::msg)?;
    retired.extend(result.retired.iter().cloned());
    Ok(record_native_signal(&row.name, "agent", result, report))
}
fn kill_native_orphan(session: &crate::cli::client::Session, orphan: &pidtrack::OrphanCandidate, target: &str, report: &mut KillReport) -> Result<i32> {
    let result = crate::runtime::kill_client::kill_orphan(session.client.scope_root(), orphan).map_err(anyhow::Error::msg)?;
    Ok(record_native_signal(target, "orphan", result, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denied_kill_restores_status_and_preserves_tracking() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, created_at, pid, status, status_context, status_detail, status_time)
             VALUES ('luna', 1.0, 4242, 'listening', 'ready', 'original', 123)",
                [],
            )
            .unwrap();
        let original = db.get_instance_full("luna").unwrap().unwrap();
        db.mark_killed("luna", "boss").unwrap();
        restore_failed_kill_status(&db, &original, "boss", terminal::KillResult::PermissionDenied);
        let restored = db.get_instance_full("luna").unwrap().unwrap();
        assert_eq!(restored.pid, original.pid);
        assert_eq!(restored.status, original.status);
        assert_eq!(restored.status_context, original.status_context);
        assert_eq!(restored.status_detail, original.status_detail);
        assert_eq!(restored.status_time, original.status_time);

        db.mark_killed("luna", "boss").unwrap();
        db.conn().execute("UPDATE instances SET status_context = 'new-hook' WHERE name = 'luna'", []).unwrap();
        restore_failed_kill_status(&db, &original, "boss", terminal::KillResult::PermissionDenied);
        assert_eq!(db.get_instance_full("luna").unwrap().unwrap().status_context, "new-hook");
    }

    fn orphan(name: &str) -> pidtrack::OrphanProcess {
        pidtrack::OrphanProcess {
            pid: 4242,
            tool: "claude".into(),
            names: vec![name.into()],
            directory: String::new(),
            process_id: "proc-1".into(),
            terminal_preset: String::new(),
            pane_id: String::new(),
            terminal_id: String::new(),
            kitty_listen_on: String::new(),
            zellij_session_name: String::new(),
            herdr_socket_path: String::new(),
            wezterm_unix_socket: String::new(),
            tmux: String::new(),
            session_id: String::new(),
            notify_port: 0,
            inject_port: 0,
        }
    }

    #[test]
    fn orphan_match_uses_agentx_name_semantics() {
        let process = orphan("tuna");
        assert!(orphan_matches(&process, "tuna"));
        assert!(!orphan_matches(&process, "nova"));
        assert!(orphan_matches(&process, "4242"));
        assert!(orphan_matches(&process, "proc-1"));
        assert_eq!(orphan_target(&process), "tuna");
    }
    use serde_json::json;

    #[test]
    fn test_kill_no_target_fails() {
        // JSON deterministically rejects before Session acquisition, even when
        // the test runner itself has a terminal. The CLI router owns exit code.
        let flags = GlobalFlags::default();
        let argv = vec!["kill".to_string(), "--json".to_string()];
        let error = run(&argv, &flags).unwrap_err();
        assert!(error.to_string().contains("needs an explicit target in noninteractive or --json mode"));
    }

    #[test]
    fn test_kill_args_parse_single() {
        use clap::Parser;
        let args = KillArgs::try_parse_from(["kill", "myagent"]).unwrap();
        assert_eq!(args.targets, vec!["myagent"]);
    }

    #[test]
    fn test_kill_args_parse_multiple() {
        use clap::Parser;
        let args = KillArgs::try_parse_from(["kill", "nozu", "zelu"]).unwrap();
        assert_eq!(args.targets, vec!["nozu", "zelu"]);
    }

    #[test]
    fn test_kill_args_no_target_is_empty_vec() {
        use clap::Parser;
        let args = KillArgs::try_parse_from(["kill"]).unwrap();
        assert!(args.targets.is_empty());
    }

    #[test]
    fn test_normalize_permission_denied_after_pane_close_succeeds() {
        let result = normalize_kill_result("luna", 42, terminal::KillResult::PermissionDenied, true);
        assert_eq!(result, terminal::KillResult::AlreadyDead);
    }

    #[test]
    fn test_incomplete_cleanup_requires_terminated_process_and_retry_command() {
        assert!(report_incomplete_pane_cleanup(PaneCleanupProcessState::Terminated, Some("wezterm cli kill-pane --pane-id 123")));
        assert!(!report_incomplete_pane_cleanup(PaneCleanupProcessState::NotTerminated, Some("wezterm cli kill-pane --pane-id 123")));
        assert!(!report_incomplete_pane_cleanup(PaneCleanupProcessState::AlreadyDead, None));
    }

    #[test]
    fn test_handle_remote_kill_response_permission_denied_returns_nonzero() {
        let result = handle_remote_kill_response(
            "luna:ABCD",
            &json!({
                "result": {
                    "pid": 42,
                    "kill_result": "permission_denied",
                    "pane_closed": false,
                    "preset_name": "",
                    "pane_id": ""
                }
            }),
            &mut KillReport::new(false),
        )
        .unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_handle_remote_kill_response_permission_denied_with_closed_pane_succeeds() {
        let result = handle_remote_kill_response(
            "luna:ABCD",
            &json!({
                "ok": true,
                "result": {
                    "pid": 42,
                    "kill_result": "already_dead",
                    "pane_closed": true,
                    "preset_name": "kitty",
                    "pane_id": "@1"
                }
            }),
            &mut KillReport::new(false),
        )
        .unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_render_remote_kill_feedback_sent_matches_cli_contract() {
        let lines = render_remote_kill_feedback("luna:ABCD", "sent", " (closed kitty pane @1)").unwrap();
        assert_eq!(lines, vec!["Sent SIGTERM to \'luna:ABCD\' (closed kitty pane @1)".to_string(), "  To resume: agentx r luna:ABCD".to_string(),]);
    }

    #[test]
    fn test_render_remote_kill_feedback_already_dead_matches_cli_contract() {
        let lines = render_remote_kill_feedback("luna:ABCD", "already_dead", "").unwrap();
        assert_eq!(lines, vec!["\'luna:ABCD\' had already exited".to_string(), "  To resume: agentx r luna:ABCD".to_string(),]);
    }

    #[test]
    fn test_handle_remote_kill_response_unknown_result_errors() {
        let err = handle_remote_kill_response(
            "luna:ABCD",
            &json!({
                "result": {
                    "pid": 42,
                    "kill_result": "mystery",
                    "pane_closed": false,
                    "preset_name": "",
                    "pane_id": ""
                }
            }),
            &mut KillReport::new(false),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("unexpected kill_result mystery"));
    }
}
