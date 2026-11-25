//! Origin-local attempted facts retained before transport errors are flattened.
//! No background publication, business retry, environment/body retention or
//! server timestamp is introduced. Explicit consumers may drain safe reports.

use crate::daemon::{DaemonError, DaemonInfo};
use crate::observability::{configuration::InvocationCaptureRef, context::*, runtime::*};
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

thread_local! { static ATTEMPT: RefCell<Option<ClientAttemptReport>> = const { RefCell::new(None) }; }

fn retained() -> &'static Mutex<LocalAttemptReports> {
    static REPORTS: OnceLock<Mutex<LocalAttemptReports>> = OnceLock::new();
    REPORTS.get_or_init(|| Mutex::new(LocalAttemptReports::default()))
}

pub(crate) struct AttemptGuard {
    previous: Option<ClientAttemptReport>,
    operation: Option<OperationLabel>,
    trace_started: Option<std::time::Instant>,
}

impl AttemptGuard {
    pub(crate) fn begin(kind: ClientAttemptKind, requested_scope: PathBuf, invocation: Option<InvocationCaptureRef>, request_id: Option<super::RequestId>) -> Option<Self> {
        let trace_started = crate::observability::trace::active().then(std::time::Instant::now);
        let at = super::context::originating_clock().now().ok()?;
        let knowledge = if kind == ClientAttemptKind::Call { ClientCallKnowledge::NotWritten } else { ClientCallKnowledge::DiscoveryOnly };
        let report = ClientAttemptReport {
            attempt: ClientAttemptId::new(),
            kind,
            invocation,
            request_id,
            requested_scope,
            resolved_scope: EvidenceValue::Unknown(UnknownReason::NotCaptured),
            started_at: at.clone(),
            ended_at: at,
            stages: Vec::new(),
            omitted_stages: 0,
            last_ready: None,
            original_epoch: None,
            knowledge,
            failure: None,
        };
        let previous = ATTEMPT.with(|slot| slot.replace(Some(report)));
        Some(Self { previous, operation: None, trace_started })
    }

    pub(crate) fn name_operation(&mut self, request: &super::Request) {
        if self.trace_started.is_some() {
            self.operation = Some(OperationLabel::of(request));
        }
    }

    pub(crate) fn name_operation_label(&mut self, label: OperationLabel) {
        if self.trace_started.is_some() {
            self.operation = Some(label);
        }
    }

    pub(crate) fn origin(&self) -> Option<OriginRef> {
        ATTEMPT.with(|slot| slot.borrow().as_ref().and_then(|report| Some(OriginRef { invocation: report.invocation.clone()?, attempt: report.attempt })))
    }
}

impl Drop for AttemptGuard {
    fn drop(&mut self) {
        let report = ATTEMPT.with(|slot| slot.replace(self.previous.take()));
        let Some(mut report) = report else {
            return;
        };
        let Ok(at) = super::context::originating_clock().now() else {
            return;
        };
        report.ended_at = at;
        if let Some(started) = self.trace_started {
            crate::observability::trace::attempt(started, &report, self.operation.take());
        }
        if let Ok(mut retained) = retained().try_lock() {
            let _ = retained.push(report);
        }
    }
}

#[expect(dead_code, reason = "Explicit origin-local report intake remains available to native consumers; the passive CLI returns its current attempt directly and must not auto-publish or drain unrelated retained attempts.")]
pub(crate) fn drain_local_reports() -> Vec<ClientAttemptReport> {
    let Ok(mut reports) = retained().try_lock() else {
        return Vec::new();
    };
    let mut result = Vec::with_capacity(reports.len());
    while let Some(report) = reports.pop_oldest() {
        result.push(report);
    }
    result
}

/// Nested work is expressed as ordered leaf spans, with at most twelve kept.
pub(crate) fn stage<T>(stage: ClientStage, action: impl FnOnce() -> Result<T, DaemonError>) -> Result<T, DaemonError> {
    let clock = super::context::originating_clock();
    let started_at = clock.now().ok();
    let result = action();
    if let (Some(started_at), Ok(ended_at)) = (started_at, clock.now()) {
        ATTEMPT.with(|slot| {
            let mut slot = slot.borrow_mut();
            let Some(report) = slot.as_mut() else {
                return;
            };
            if report.stages.len() < 12 {
                report.stages.push(ClientStageSpan { stage, started_at, ended_at, failure: result.as_ref().err().map(crate::daemon::observation::failure_kind) });
            } else {
                report.omitted_stages = report.omitted_stages.saturating_add(1);
            }
            if let Err(error) = &result {
                report.failure = Some(crate::daemon::observation::local_failure(error.clone(), stage, clock));
            }
        });
    }
    result
}

pub(crate) fn resolved_scope(scope: ScopeRef) {
    ATTEMPT.with(|slot| {
        if let Some(report) = slot.borrow_mut().as_mut() {
            report.resolved_scope = EvidenceValue::Present(scope);
        }
    });
}

pub(crate) fn invocation(reference: InvocationCaptureRef) {
    ATTEMPT.with(|slot| {
        if let Some(report) = slot.borrow_mut().as_mut() {
            report.invocation = Some(reference);
        }
    });
}

pub(crate) fn ready(info: &DaemonInfo, process: ObservedProcess) {
    let clock = super::context::originating_clock();
    let Ok(at) = clock.now() else {
        return;
    };
    let Ok(scope) = ScopeRef::from_canonical_scope(info.canonical_scope.clone()) else {
        return;
    };
    let Ok(daemon) = DaemonRef::from_lease(scope, info.daemon_instance.clone()) else {
        return;
    };
    ATTEMPT.with(|slot| {
        if let Some(report) = slot.borrow_mut().as_mut() {
            report.original_epoch = ObservationEpoch::from_committed_generation(daemon.clone(), info.database_generation.clone()).ok();
            report.last_ready = Some(InspectionReady {
                daemon,
                protocol_version: info.protocol_version,
                build_version: info.build_version.clone(),
                build_id: info.build_id.clone(),
                process,
                generation: Evidence { value: EvidenceValue::Present(info.database_generation.clone()), stamp: EvidenceStamp { origin: EvidenceOrigin::ClientReported, at: at.clone() } },
                sampled_at: at,
            });
            // A successful later discovery supersedes a failed earlier probe.
            report.failure = None;
        }
    });
}

pub(crate) fn may_write() {
    ATTEMPT.with(|slot| {
        if let Some(report) = slot.borrow_mut().as_mut() {
            report.knowledge = ClientCallKnowledge::MayHaveBeenWritten;
        }
    });
}

pub(crate) fn application_result(result: &super::ServiceResult<super::Response>) {
    ATTEMPT.with(|slot| {
        if let Some(report) = slot.borrow_mut().as_mut() {
            report.knowledge = ClientCallKnowledge::ReceivedApplicationResult(ApplicationResultClass::of(result));
            report.failure = None;
        }
    });
}

pub(crate) fn received_failure(error: &DaemonError) {
    let code = match error {
        DaemonError::Busy => RuntimeFailureCode::Busy,
        DaemonError::Deadline => RuntimeFailureCode::Deadline,
        DaemonError::StaleGeneration => RuntimeFailureCode::StaleGeneration,
        DaemonError::Incompatible(_) => RuntimeFailureCode::Incompatible,
        DaemonError::OutcomeUnknown(_) => RuntimeFailureCode::OutcomeUnknown,
        DaemonError::Protocol(_) | DaemonError::InvalidInput(_) => RuntimeFailureCode::Protocol,
        DaemonError::Unsupported(_) => RuntimeFailureCode::Unsupported,
        _ => RuntimeFailureCode::Io,
    };
    ATTEMPT.with(|slot| {
        if let Some(report) = slot.borrow_mut().as_mut() {
            report.knowledge = ClientCallKnowledge::ReceivedFailure(code);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current() -> ClientAttemptReport {
        let mut report = ATTEMPT.with(|slot| slot.borrow().as_ref().unwrap().clone());
        report.ended_at = super::super::context::originating_clock().now().unwrap();
        report
    }

    #[test]
    fn structured_failure_survives_before_service_error_flattening() {
        let _guard = AttemptGuard::begin(ClientAttemptKind::Call, PathBuf::from("explicit-scope"), None, Some(super::super::RequestId("origin-error".into()))).unwrap();
        #[cfg(unix)]
        let code = libc::EACCES;
        #[cfg(windows)]
        let code = 5;
        #[cfg(not(any(unix, windows)))]
        let code = 1;
        let result: Result<(), DaemonError> = stage(ClientStage::Connect, || Err(std::io::Error::from_raw_os_error(code).into()));
        assert!(result.is_err());
        let report = current();
        assert!(matches!(report.knowledge, ClientCallKnowledge::NotWritten));
        let failure = report.failure.as_ref().unwrap();
        assert_eq!(failure.stage, ClientStage::Connect);
        assert_eq!(failure.os_code, Some(code));
        assert!(failure.at.is_some());
        assert!(report.validate());
        assert!(!serde_json::to_string(&report).unwrap().contains("daemon I/O failed"));
    }

    #[test]
    fn nested_unwind_restores_original_attempt_without_erasing_write_uncertainty() {
        let _guard = AttemptGuard::begin(ClientAttemptKind::Call, PathBuf::from("scope"), None, Some(super::super::RequestId("uncertain-call".into()))).unwrap();
        let original = current().attempt;
        may_write();
        let unwound = std::panic::catch_unwind(|| {
            let _nested = AttemptGuard::begin(ClientAttemptKind::Probe, PathBuf::from("other"), None, None).unwrap();
            assert_ne!(current().attempt, original);
            panic!("unwind nested instrumentation scope");
        });
        assert!(unwound.is_err());
        let _: Result<(), DaemonError> = stage(ClientStage::ReplyRead, || Err(DaemonError::Deadline));
        let report = current();
        assert_eq!(report.attempt, original);
        assert!(matches!(report.knowledge, ClientCallKnowledge::MayHaveBeenWritten));
        assert_eq!(report.failure.as_ref().unwrap().kind, LocalFailureKind::TimedOut);
        assert!(report.validate());
    }

    #[test]
    fn stage_pressure_preserves_bounded_order_and_terminal_knowledge() {
        let _guard = AttemptGuard::begin(ClientAttemptKind::Call, PathBuf::from("scope"), None, Some(super::super::RequestId("bounded-stages".into()))).unwrap();
        for _ in 0..15 {
            stage(ClientStage::Discovery, || Ok(())).unwrap();
        }
        may_write();
        let _: Result<(), DaemonError> = stage(ClientStage::ReplyRead, || Err(DaemonError::Busy));
        received_failure(&DaemonError::Busy);
        let report = current();
        assert_eq!(report.stages.len(), 12);
        assert_eq!(report.omitted_stages, 4);
        assert!(matches!(report.knowledge, ClientCallKnowledge::ReceivedFailure(RuntimeFailureCode::Busy)));
        assert!(report.validate());
    }
}
