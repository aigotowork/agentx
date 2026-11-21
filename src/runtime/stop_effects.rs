//! Host execution of the canonical frozen participant-stop plan.
//! Every signal/tracking action rechecks the captured nonempty OS incarnation;
//! durable retirement is already committed before entering this module.

/// Enumerate/prune only on the host, then atomically reattach the supplied
/// process facts. A lost reply recovers committed rows before pidfile removal.
pub(crate) fn claim_orphans(client: &crate::service::client::Client) -> crate::service::ServiceResult<()> {
    use crate::agents::{lifecycle_service, service};
    use crate::service::{CallerContext, Request, RequestId, Response, ServiceError};
    let candidates = crate::runtime::process_tracking::capture_orphan_candidates(client.scope_root()).map_err(ServiceError::InvalidInput)?;
    if candidates.is_empty() {
        return Ok(());
    }
    let request = Request::Participants(service::Request::Lifecycle(lifecycle_service::Request::ClaimOrphans { candidates }));
    let caller = CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: client.invocation().clone(), deadline_unix_ms: None, expected_database_generation: Some(client.info().database_generation.clone()) };
    let response = match client.call_with_context(caller.clone(), request.clone()) {
        Err(ServiceError::OutcomeUnknown(_)) => client.call_with_context(caller, request),
        result => result,
    }?;
    let Response::Participants(service::Response::Lifecycle(lifecycle_service::Response::OrphansClaimed(adopted))) = response else {
        return Err(ServiceError::Internal("unexpected orphan adoption response".into()));
    };
    for adopted in adopted {
        if let Some(pid) = adopted.record.pid.and_then(|pid| u32::try_from(pid).ok()) {
            crate::runtime::process_tracking::remove_pid(client.scope_root(), pid);
        }
    }
    Ok(())
}

/// Execute one exact native operator retirement. Only its transactional result
/// may be recovered after a lost reply; host effects run after successful reply.
pub(crate) fn retire(
    client: &crate::service::client::Client,
    participant: &crate::agents::attachments::ParticipantRef,
    initiator: &str,
    decision: &crate::agents::hook_service::OperatorStop,
) -> crate::service::ServiceResult<crate::agents::hook_service::SessionStopOutcome> {
    use crate::agents::{lifecycle_service, service};
    use crate::service::{CallerContext, Request, RequestId, Response, ServiceError};
    let caller = CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: client.invocation().clone(), deadline_unix_ms: None, expected_database_generation: Some(client.info().database_generation.clone()) };
    let request = Request::Participants(service::Request::Lifecycle(lifecycle_service::Request::StopOperator { participant: participant.clone(), initiator: initiator.into(), decision: decision.clone() }));
    let response = match client.call_with_context(caller.clone(), request.clone()) {
        Err(ServiceError::OutcomeUnknown(_)) => client.call_with_context(caller, request),
        result => result,
    }?;
    let Response::Participants(service::Response::Lifecycle(lifecycle_service::Response::OperatorStopped(outcome))) = response else {
        return Err(ServiceError::Internal("unexpected operator stop response".into()));
    };
    for effect in outcome.effects.iter().cloned() {
        apply(effect);
    }
    Ok(outcome)
}

pub(crate) fn apply(effect: crate::agents::hook_service::RetiredParticipant) {
    use crate::agents::hook_service::StopProcessAction;
    let row = &effect.record;
    if let (Some(pid), Some(identity)) = (row.pid.and_then(|pid| u32::try_from(pid).ok()), effect.process_identity.as_deref().filter(|identity| !identity.is_empty())) {
        let still_ours = || crate::sys::process::identity(pid).as_deref() == Some(identity);
        if still_ours() {
            match effect.process_action {
                StopProcessAction::KeepRunning => {}
                StopProcessAction::TerminateGroup => {
                    if crate::sys::process::terminate_group(pid) == crate::sys::process::GroupSignal::Sent {
                        for _ in 0..20 {
                            if crate::sys::process::has_exited(pid) || !still_ours() {
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        if still_ours() {
                            let _ = crate::sys::process::kill_group(pid);
                        }
                    }
                }
                StopProcessAction::TrackSurvivor => {
                    let info = crate::runtime::terminal::resolve_terminal_info(row.terminal_preset_effective.as_deref(), row.launch_context.as_deref());
                    let binding = if info.process_id.is_empty() { effect.process_bindings.first().map(String::as_str).unwrap_or("") } else { &info.process_id };
                    let port = |kind: &str| effect.endpoints.iter().find(|(candidate, _)| candidate == kind).map(|(_, port)| *port).unwrap_or(0);
                    crate::runtime::process_tracking::record_pid(&crate::runtime::process_tracking::PidRecord {
                        agentx_dir: &effect.scope,
                        pid,
                        tool: &row.tool,
                        name: &row.name,
                        directory: &row.directory,
                        process_id: binding,
                        terminal_preset: &info.preset_name,
                        pane_id: &info.pane_id,
                        terminal_id: &info.terminal_id,
                        kitty_listen_on: &info.kitty_listen_on,
                        zellij_session_name: &info.zellij_session_name,
                        herdr_socket_path: &info.herdr_socket_path,
                        wezterm_unix_socket: &info.wezterm_unix_socket,
                        tmux: &info.tmux,
                        session_id: row.session_id.as_deref().unwrap_or(""),
                        notify_port: port("pty"),
                        inject_port: port("inject"),
                    });
                }
            }
        }
    }
    let ports: Vec<_> = effect.endpoints.iter().filter(|(kind, _)| kind != "inject").map(|(_, port)| *port).collect();
    crate::runtime::notify::wake_ports(&ports, crate::runtime::notify::WAKE_TARGETED_MS);
}
