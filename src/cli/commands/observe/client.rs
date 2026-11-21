//! Native passive reads and local evidence lifetime, never product mutations.

use std::path::PathBuf;

use crate::daemon::observation::{self, InspectionRequest, InspectionResponse, InspectionResult};
use crate::observability::configuration::{CaptureCurrent, CaptureInputs, CaptureSkipReason, ConfigurationRequest, InvocationCaptureHandle, LocalConfigSnapshot, RulesRevision};
use crate::observability::context::{DaemonRef, EvidenceValue, ObservationEpoch, ObservationTime, UnknownReason};
use crate::observability::events::{ObservationCursor, ObservationFilter, ObservationHead, ObservationRead, ObservationRequest, ObservationResponse, ObservationWindow};
use crate::observability::runtime::{ClientAttemptId, ClientAttemptKind, ClientAttemptReport, ClientCallKnowledge, ClientInvocationId, ClientStage, ClientStageSpan, ExistingScopeInput, LocalFailure, LocalFailureKind, RuntimeRequest, ScopeLookup};
use crate::service::InvocationContext;

use super::args::{ObserveArgs, PreparedCommand};
use super::output::{LocalEvidence, ObserveOutput, ObserveResult};

pub(super) struct ObservationClient {
    invocation: InvocationContext,
    capture: InvocationCaptureHandle,
    requested_scope: PathBuf,
    lookup: ScopeLookup,
    lookup_failure: Option<LocalFailure>,
    inputs: Option<Result<LocalConfigSnapshot, CaptureSkipReason>>,
    attempt: ClientAttemptId,
    started_at: Option<ObservationTime>,
    scope_span: Option<ClientStageSpan>,
}

impl ObservationClient {
    pub fn capture(args: &ObserveArgs, command: &PreparedCommand) -> Result<Self, Box<ObserveOutput>> {
        let started_at = observation::originating_clock().now().ok();
        let invocation = match InvocationContext::capture_observed() {
            Ok(invocation) => invocation,
            Err(failure) => {
                // The native structured capture path preserves the original
                // I/O kind/code even without a usable report clock.
                return Err(Box::new(ObserveOutput { command: command.name(), requested_scope: args.scope.clone(), scope: None, local: LocalEvidence { inputs: None, attempt: None }, result: ObserveResult::LocalFailure(failure) }));
            }
        };
        let requested_scope = args.scope.clone().unwrap_or_else(|| crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd).0);
        let capture = InvocationCaptureHandle::begin(ClientInvocationId::new());
        let inputs = Some(match observation::originating_clock().now() {
            Ok(at) => capture.capture_invocation_inputs(capture_inputs(&invocation, at)),
            Err(_) => Err(CaptureSkipReason::ObservationFailed),
        });
        let before_lookup = observation::originating_clock().now().ok();
        let lookup = observation::resolve_existing_scope(ExistingScopeInput { requested: requested_scope.clone(), cwd: invocation.cwd.clone() });
        let failure = scope_failure(&lookup);
        let scope_span = before_lookup.zip(observation::originating_clock().now().ok()).map(|(started_at, ended_at)| ClientStageSpan { stage: ClientStage::ScopeLookup, started_at, ended_at, failure: failure.as_ref().map(|value| value.kind) });
        Ok(Self { invocation, capture, requested_scope, lookup, lookup_failure: failure, inputs, attempt: ClientAttemptId::new(), started_at, scope_span })
    }

    pub fn validate_scope(&self, command: &PreparedCommand) -> Result<(), String> {
        match &self.lookup {
            ScopeLookup::Existing(scope) => command.validate_scope(scope),
            _ => Ok(()),
        }
    }

    pub fn read(&self, command: &PreparedCommand) -> ObserveOutput {
        let name = command.name();
        if let Some(failure) = &self.lookup_failure {
            return self.record(name, ObserveResult::LocalFailure(failure.clone()));
        }
        match command {
            PreparedCommand::Trace { .. } => unreachable!("trace has an independent local log source"),
            PreparedCommand::Inspect => self.call(name, None, InspectionRequest::Runtime(RuntimeRequest::Snapshot)),
            PreparedCommand::Request(reference) => self.call(name, Some(reference.daemon.clone()), InspectionRequest::Runtime(RuntimeRequest::RequestDetail(reference.clone()))),
            PreparedCommand::Config(reference) => self.call(name, Some(reference.epoch.daemon.clone()), InspectionRequest::Configuration(ConfigurationRequest::Lookup(reference.clone()))),
            // Let the native comparison owner classify different operand eras;
            // pinning the aggregate to one operand would preempt Incomparable.
            PreparedCommand::Compare(input) => self.call(name, None, InspectionRequest::Configuration(ConfigurationRequest::Compare(input.clone()))),
            PreparedCommand::ConfigCurrent => {
                let result = match observation::originating_clock().now() {
                    Ok(at) => {
                        let ScopeLookup::Existing(scope) = &self.lookup else { unreachable!("scope failure returned above") };
                        self.capture.capture_current_resolution(CaptureCurrent {
                            inputs: capture_inputs(&self.invocation, at),
                            explicitly_selected_scope: &scope.canonical_scope,
                            resolution_scope: EvidenceValue::Present(scope.clone()),
                            source: &crate::config::observation::NativeConfigResolutionSource,
                        })
                    }
                    Err(_) => Err(CaptureSkipReason::ObservationFailed),
                };
                self.record(name, ObserveResult::ConfigurationCurrent(result))
            }
            PreparedCommand::Events { window: Some(window), limit, .. } => self.read_window(window.clone(), *limit),
            PreparedCommand::Events { window: None, limit, .. } => {
                let described = self.call(name, None, InspectionRequest::Events(ObservationRequest::Describe));
                let Some(head) = event_head(&described) else {
                    return described;
                };
                match first_window(head) {
                    Some(window) => self.read_window(window, *limit),
                    None => self.record(name, ObserveResult::LocalFailure(local_failure(ClientStage::InspectionRead, LocalFailureKind::Protocol, None))),
                }
            }
        }
    }

    /// Continue the returned immutable window exactly. Only an exhausted
    /// same-stream interval may open a new tail interval under explicit follow.
    pub fn follow_page(&self, previous: &ObserveOutput, limit: u16) -> ObserveOutput {
        let Some(page) = previous.event_page() else {
            return self.record("events", ObserveResult::LocalFailure(local_failure(ClientStage::InspectionRead, LocalFailureKind::Protocol, None)));
        };
        let next = page.next.clone();
        if next.after.sequence < next.through {
            return self.read_window(next, limit);
        }
        let described = self.call("events", Some(next.after.stream.epoch.daemon.clone()), InspectionRequest::Events(ObservationRequest::Describe));
        let Some(head) = event_head(&described) else {
            return described;
        };
        let Some(window) = next_follow_window(&next, head) else {
            return self.record("events", ObserveResult::LocalFailure(local_failure(ClientStage::InspectionRead, LocalFailureKind::Protocol, None)));
        };
        self.read_window(window, limit)
    }

    fn read_window(&self, window: ObservationWindow, limit: u16) -> ObserveOutput {
        self.call("events", Some(window.after.stream.epoch.daemon.clone()), InspectionRequest::Events(ObservationRequest::Read(ObservationRead { window, limit })))
    }

    fn call(&self, command: &'static str, expected: Option<DaemonRef>, request: InspectionRequest) -> ObserveOutput {
        let ScopeLookup::Existing(scope) = &self.lookup else {
            return self.record(command, ObserveResult::LocalFailure(self.lookup_failure.clone().expect("non-existing scope has a failure")));
        };
        let result = match observation::inspect_existing(scope, expected, request) {
            Ok(result) => ObserveResult::Inspection(result),
            Err(failure) => ObserveResult::LocalFailure(failure),
        };
        self.record(command, result)
    }

    fn record(&self, command: &'static str, result: ObserveResult) -> ObserveOutput {
        let ended_at = observation::originating_clock().now().ok();
        let ready = match &result {
            ObserveResult::Inspection(value) => Some(value.ready.clone()),
            _ => None,
        };
        let original_epoch = ready.as_ref().and_then(|ready| match &ready.generation.value {
            EvidenceValue::Present(generation) => ObservationEpoch::from_committed_generation(ready.daemon.clone(), generation.clone()).ok(),
            _ => None,
        });
        let failure = match &result {
            ObserveResult::LocalFailure(failure) => Some(failure.clone()),
            _ => None,
        };
        let attempt = self.started_at.clone().zip(ended_at).and_then(|(started_at, ended_at)| {
            ended_at.elapsed_since(&started_at).ok()?;
            Some(ClientAttemptReport {
                attempt: self.attempt,
                kind: ClientAttemptKind::Probe,
                invocation: Some(self.capture.reference()),
                request_id: None,
                requested_scope: self.requested_scope.clone(),
                resolved_scope: match &self.lookup {
                    ScopeLookup::Existing(scope) => EvidenceValue::Present(scope.clone()),
                    _ => EvidenceValue::Unknown(UnknownReason::ObservationFailed),
                },
                started_at,
                ended_at,
                stages: self.scope_span.clone().into_iter().collect(),
                omitted_stages: 0,
                last_ready: ready,
                original_epoch,
                knowledge: ClientCallKnowledge::DiscoveryOnly,
                failure,
            })
        });
        ObserveOutput { command, requested_scope: Some(self.requested_scope.clone()), scope: Some(self.lookup.clone()), local: LocalEvidence { inputs: self.inputs.clone(), attempt }, result }
    }
}

fn capture_inputs(invocation: &InvocationContext, at: ObservationTime) -> CaptureInputs<'_> {
    CaptureInputs { invocation, at, rules: RulesRevision::current(env!("AGENTX_NATIVE_BUILD_ID")) }
}

fn local_failure(stage: ClientStage, kind: LocalFailureKind, os_code: Option<i32>) -> LocalFailure {
    LocalFailure { stage, kind, os_code, at: observation::originating_clock().now().ok() }
}

fn scope_failure(lookup: &ScopeLookup) -> Option<LocalFailure> {
    let (kind, os_code) = match lookup {
        ScopeLookup::Existing(_) => return None,
        ScopeLookup::Missing => (LocalFailureKind::MissingScope, None),
        ScopeLookup::NotDirectory => (LocalFailureKind::NotDirectory, None),
        ScopeLookup::InvalidPath => (LocalFailureKind::Io, None),
        ScopeLookup::Unreadable { os_code: Some(code) } => {
            return Some(observation::io_local_failure(std::io::Error::from_raw_os_error(*code), ClientStage::ScopeLookup, observation::originating_clock()));
        }
        ScopeLookup::Unreadable { os_code: None } => (LocalFailureKind::Io, None),
    };
    Some(local_failure(ClientStage::ScopeLookup, kind, os_code))
}

fn event_head(record: &ObserveOutput) -> Option<&ObservationHead> {
    match &record.result {
        ObserveResult::Inspection(InspectionResult { response: InspectionResponse::Events(Ok(ObservationResponse::Head(head))), .. }) => Some(head),
        _ => None,
    }
}

pub(super) fn first_window(head: &ObservationHead) -> Option<ObservationWindow> {
    Some(ObservationWindow { after: ObservationCursor { stream: head.stream.clone(), sequence: head.oldest.checked_sub(1)? }, through: head.newest, filter: ObservationFilter::default() })
}

/// Changed eras are sent unchanged to the native owner, so its actual typed
/// EpochChanged/StreamChanged (or aggregate DaemonChanged) is shown to callers.
pub(super) fn next_follow_window(next: &ObservationWindow, head: &ObservationHead) -> Option<ObservationWindow> {
    if next.after.stream != head.stream || next.after.sequence < next.through {
        return Some(next.clone());
    }
    if head.newest < next.after.sequence {
        return None;
    }
    Some(ObservationWindow { after: next.after.clone(), through: head.newest, filter: next.filter.clone() })
}
