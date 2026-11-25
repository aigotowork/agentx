//! Caller-owned launch waits. Only typed status snapshots and fenced temporary
//! notification registration cross IPC; a disconnected caller never opens DB.

use std::time::{Duration, Instant};

use crate::runtime::hosts::{HostService, LaunchWaitRegistration, Request, Response};
use crate::runtime::notify::NotifyServer;
use crate::service::client::Client;
use crate::service::{CallerContext, Request as ServiceRequest, RequestId, Response as ServiceResponse, ServiceError, ServiceResult};

use super::{LaunchResult, LaunchStatus};

struct LaunchWaiter<'a> {
    client: &'a Client,
    registration: LaunchWaitRegistration,
    server: NotifyServer,
}

impl<'a> LaunchWaiter<'a> {
    fn new(client: &'a Client, batch_prefix: Option<&str>) -> ServiceResult<Self> {
        let server = NotifyServer::new().map_err(|error| ServiceError::Unavailable(error.to_string()))?;
        let response = HostService::call(client, Request::RegisterLaunchWaiter { batch_prefix: batch_prefix.map(str::to_owned), port: server.port() })?;
        match response {
            Response::LaunchWaiter(registration) => Ok(Self { client, registration, server }),
            _ => Err(ServiceError::Internal("unexpected launch waiter response".into())),
        }
    }
}

impl Drop for LaunchWaiter<'_> {
    fn drop(&mut self) {
        // Exact peer/name/port validation prevents stale cleanup from clearing
        // another waiter. A lost registration response is reaped by liveness.
        let _ = HostService::call(self.client, Request::ReleaseLaunchWaiter(self.registration.clone()));
    }
}

pub(super) fn error_result(message: String) -> LaunchResult {
    LaunchResult {
        status: LaunchStatus::Error,
        expected: None,
        ready: None,
        failed: None,
        blocked: None,
        instances: Vec::new(),
        failures: Vec::new(),
        blockers: Vec::new(),
        launcher: None,
        timestamp: None,
        batch_id: None,
        batches: None,
        hint: None,
        message: Some(message),
    }
}

/// Reconcile the retained cleanup plan after a missing reply with the original
/// request identity. A new observation cannot recover an earlier retirement.
fn observe(client: &Client, launcher: Option<&str>, batch_id: Option<&str>, timeout_secs: u64) -> ServiceResult<Response> {
    let caller = CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: client.invocation().clone(), deadline_unix_ms: None, expected_database_generation: None };
    let request = Request::ObserveLaunch { launcher: launcher.map(str::to_owned), batch_id: batch_id.map(str::to_owned), timeout_secs };
    let call = || client.call_with_context(caller.clone(), ServiceRequest::Hosts(request.clone()));
    let response = match call() {
        Err(ServiceError::OutcomeUnknown(_)) => call(),
        other => other,
    }?;
    match response {
        ServiceResponse::Hosts(response) => Ok(response),
        _ => Err(ServiceError::Internal("unexpected launch status response".into())),
    }
}

/// Log/terminal evidence changes presentation only. The daemon's durable
/// readiness, failure counts and exact frozen cleanup authority remain intact.
fn enrich_diagnostics(result: &mut LaunchResult, records: &[crate::db::InstanceRow]) {
    for record in records {
        let prefix = format!("{}: ", record.name);
        let failure = result.failures.iter_mut().chain(result.blockers.iter_mut()).find(|detail| detail.starts_with(&prefix));
        if failure.is_none() && (result.status != LaunchStatus::Timeout || result.instances.contains(&record.name)) {
            continue;
        }
        let Some(evidence) = crate::agents::lifecycle::capture_launch_failure_evidence(record) else {
            continue;
        };
        if let Some(detail) = failure {
            if detail.contains(&evidence) {
                continue;
            }
            let before = detail.clone();
            detail.push('\n');
            detail.push_str(&evidence);
            if let Some(hint) = &mut result.hint {
                *hint = hint.replace(before.as_str(), detail.as_str());
            }
        } else if let Some(hint) = &mut result.hint
            && !hint.contains(&evidence)
        {
            hint.push_str(&format!(" {}: {evidence}", record.name));
        }
    }
}

/// Register before the initial snapshot, then wait locally. Missed or absent
/// notifications use bounded polling; IPC failures return an explicit error.
pub(crate) fn wait_for_launch_client(client: &Client, launcher: Option<&str>, batch_id: Option<&str>, timeout_secs: u64) -> LaunchResult {
    let waiter = LaunchWaiter::new(client, batch_id).ok();
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let mut observed = false;
    loop {
        let (mut result, diagnostics) = match observe(client, launcher, batch_id, timeout_secs) {
            Ok(Response::LaunchStatus { result, cleanup, diagnostics }) => {
                for effect in cleanup.effects {
                    super::super::stop_effects::apply(effect);
                }
                (result, diagnostics)
            }
            Ok(_) => return error_result("Unexpected launch status response".into()),
            Err(error) => return error_result(format!("Launch status unavailable: {error}")),
        };
        if result.status == LaunchStatus::NoLaunches && observed {
            return error_result("Launch data disappeared (DB reset or pruned)".into());
        }
        if result.status != LaunchStatus::Timeout || start.elapsed() >= timeout {
            enrich_diagnostics(&mut result, &diagnostics);
            return result;
        }
        observed = true;
        let wait_time = timeout.saturating_sub(start.elapsed()).min(Duration::from_millis(500));
        if let Some(waiter) = &waiter {
            waiter.server.wait(wait_time);
        } else {
            std::thread::park_timeout(wait_time);
        }
    }
}
