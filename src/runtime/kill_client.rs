//! Operator kill host: exact native admission precedes signal/pane I/O; durable
//! completion returns the same canonical descendant plan as Provider stop.
use super::{process_tracking, terminal};
use crate::agents::attachments::ParticipantRef;
use crate::agents::lifecycle_service::{KillTicket, Request as Lifecycle, Response as LifecycleResult};
use crate::agents::service::{Request as Participants, Response as ParticipantResult};
use crate::service::{CallerContext, Request, RequestId, Response, ServiceError, client::Client};

pub(crate) struct KillTrackedResult {
    pub target: String,
    pub pid: u32,
    pub kill_result: terminal::KillResult,
    pub pane_closed: bool,
    pub pane_retry_command: Option<String>,
    pub preset_name: String,
    pub pane_id: String,
    pub retired: Vec<ParticipantRef>,
}

fn call(client: &Client, request: Lifecycle) -> Result<LifecycleResult, ServiceError> {
    match client.call(Request::Participants(Participants::Lifecycle(request)))? {
        Response::Participants(ParticipantResult::Lifecycle(response)) => Ok(response),
        _ => Err(ServiceError::Internal("unexpected operator kill response".into())),
    }
}

/// Kill only the admitted participant; never reacquire a Client or resolve its
/// name again. An uncertain issuance/completion never repeats external effects.
pub(crate) fn kill(client: &Client, participant: &ParticipantRef, initiator: &str) -> Result<KillTrackedResult, String> {
    let caller = CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: client.invocation().clone(), deadline_unix_ms: None, expected_database_generation: Some(client.info().database_generation.clone()) };
    let prepare = || client.call_with_context(caller.clone(), Request::Participants(Participants::Lifecycle(Lifecycle::PrepareKill { participant: participant.clone(), initiator: initiator.into() })));
    let prepared = match prepare() {
        Err(ServiceError::OutcomeUnknown(_)) => prepare(),
        other => other,
    };
    let plan = match prepared.map_err(|error| error.to_string())? {
        Response::Participants(ParticipantResult::Lifecycle(LifecycleResult::KillPrepared(plan))) => plan,
        _ => return Err("unexpected operator kill preparation response".into()),
    };
    let observed = match process_state(plan.ticket.pid, plan.ticket.process_identity.as_deref()) {
        Ok(state) => state,
        Err(error) => {
            deny(client, &plan.ticket)?;
            return Err(error);
        }
    };
    let info = if plan.record.background != 0 { terminal::TerminalInfo::default() } else { terminal::resolve_terminal_info(plan.record.terminal_preset_effective.as_deref(), plan.record.launch_context.as_deref()) };
    match call(client, Lifecycle::BeginKill { ticket: plan.ticket.clone() }).map_err(|error| error.to_string())? {
        LifecycleResult::KillGranted => {}
        _ => return Err("unexpected operator kill issuance response".into()),
    }
    let (result, pane_closed, pane_retry_command) = signal(client.scope_root(), plan.ticket.pid, plan.ticket.process_identity.as_deref(), &info, observed)?;
    let mut retired = Vec::new();
    if result == terminal::KillResult::PermissionDenied {
        deny(client, &plan.ticket)?;
    } else {
        let request = || call(client, Lifecycle::CompleteKill { ticket: plan.ticket.clone() });
        let completed = match request() {
            // This operation retains the complete outcome by ticket. Only the
            // durable result is reconciled; the signal/pane is never replayed.
            Err(ServiceError::OutcomeUnknown(_)) => request(),
            other => other,
        }
        .map_err(|error| format!("kill host effect may have completed; native completion must be reconciled: {error}"))?;
        let LifecycleResult::KillCompleted(outcome) = completed else {
            return Err("unexpected operator kill completion response".into());
        };
        retired = outcome.effects.iter().map(|effect| effect.participant.clone()).collect();
        for effect in outcome.effects {
            super::stop_effects::apply(effect);
        }
        if crate::sys::process::has_exited(plan.ticket.pid) || plan.ticket.process_identity.as_deref().is_some_and(|expected| crate::sys::process::identity(plan.ticket.pid).is_some_and(|actual| actual != expected)) {
            process_tracking::remove_pid(client.scope_root(), plan.ticket.pid);
        }
    }
    Ok(KillTrackedResult { target: participant.canonical_name.clone(), pid: plan.ticket.pid, kill_result: result, pane_closed, pane_retry_command, preset_name: info.preset_name, pane_id: info.pane_id, retired })
}

fn deny(client: &Client, ticket: &KillTicket) -> Result<(), String> {
    match call(client, Lifecycle::KillDenied { ticket: ticket.clone() }).map_err(|error| error.to_string())? {
        LifecycleResult::Changed => Ok(()),
        _ => Err("unexpected operator kill denial response".into()),
    }
}

/// A positive mismatch or exit proves the recorded process is no longer the
/// target. Observation failure is neither ownership nor completed termination.
fn process_state(pid: u32, expected: Option<&str>) -> Result<bool, String> {
    let current = crate::sys::process::identity(pid);
    match (expected.filter(|value| !value.is_empty()), current.as_deref()) {
        (Some(expected), Some(actual)) => Ok(expected == actual),
        (_, None) if crate::sys::process::has_exited(pid) => Ok(false),
        _ => Err(format!("cannot verify the recorded OS incarnation of process {pid}; no signal or pane close was attempted")),
    }
}

fn signal(scope: &std::path::Path, pid: u32, expected: Option<&str>, info: &terminal::TerminalInfo, observed: bool) -> Result<(terminal::KillResult, bool, Option<String>), String> {
    if !observed || !process_state(pid, expected)? {
        return Ok((terminal::KillResult::AlreadyDead, false, None));
    }
    let pane = if info.preset_name.is_empty() { terminal::PaneCloseResult { closed: false, retry_command: None } } else { terminal::close_terminal_pane_for_scope(scope, pid, info) };
    // Pane-close commands may synchronously end the original process. Recheck
    // before the separate group signal so PID reuse cannot redirect it.
    let result = if !process_state(pid, expected)? {
        if pane.closed { terminal::KillResult::Sent } else { terminal::KillResult::AlreadyDead }
    } else {
        use crate::sys::process::GroupSignal;
        match crate::sys::process::terminate_group(pid) {
            GroupSignal::Sent => terminal::KillResult::Sent,
            #[cfg(unix)]
            GroupSignal::PermissionDenied => terminal::KillResult::PermissionDenied,
            _ => {
                if pane.closed {
                    terminal::KillResult::Sent
                } else {
                    terminal::KillResult::AlreadyDead
                }
            }
        }
    };
    let result = if result == terminal::KillResult::PermissionDenied {
        if pane.closed {
            terminal::KillResult::AlreadyDead
        } else {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if !process_state(pid, expected)? { terminal::KillResult::AlreadyDead } else { result }
        }
    } else {
        result
    };
    Ok((result, pane.closed, pane.retry_command))
}

/// Leftover PTYs have no live participant to retire. They share the same scoped
/// host signal/pane implementation and original OS incarnation checks.
pub(crate) fn kill_orphan(scope: &std::path::Path, candidate: &process_tracking::OrphanCandidate) -> Result<KillTrackedResult, String> {
    let orphan = &candidate.orphan;
    let info = orphan.terminal_info();
    let observed = process_state(orphan.pid, Some(&candidate.process_identity))?;
    let (kill_result, pane_closed, pane_retry_command) = signal(scope, orphan.pid, Some(&candidate.process_identity), &info, observed)?;
    if kill_result != terminal::KillResult::PermissionDenied {
        process_tracking::remove_pid(scope, orphan.pid);
    }
    Ok(KillTrackedResult { target: orphan.names.join(", "), pid: orphan.pid, kill_result, pane_closed, pane_retry_command, preset_name: info.preset_name, pane_id: info.pane_id, retired: Vec::new() })
}
