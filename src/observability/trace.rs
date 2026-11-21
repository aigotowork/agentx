//! Bounded timing for ordinary Provider hook invocations. Native attempts keep
//! their own identities and clocks; the existing capture family supplies the
//! correlation key across acquisitions and calls in this process.

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Serialize;

use super::configuration::InvocationCaptureHandle;
use super::runtime::{ClientAttemptId, ClientAttemptKind, ClientAttemptReport, ClientCallKnowledge, ClientInvocationId, ClientStage, LocalFailureKind, OperationLabel};
use crate::service::RequestId;

const MAX_PHASES: usize = 24;
const MAX_ATTEMPTS: usize = 16;
const SLOW: Duration = Duration::from_millis(500);

thread_local! { static HOOK: RefCell<Option<HookTrace>> = const { RefCell::new(None) }; }

pub(crate) struct HookGuard;

struct HookTrace {
    capture: InvocationCaptureHandle,
    started: Instant,
    provider: &'static str,
    event: String,
    scope: Option<PathBuf>,
    failure: Option<Phase>,
    phases: Vec<Timing>,
    attempts: Vec<Attempt>,
    omitted_phases: u32,
    omitted_attempts: u32,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Phase {
    Environment,
    DevRoot,
    HookContext,
    ReadInput,
    Route,
    TranscriptLookup,
    LaunchContext,
    Bootstrap,
    Inbox,
    Output,
    DaemonAcquire,
}

#[derive(Serialize)]
struct Timing {
    phase: Phase,
    offset_us: u64,
    duration_us: u64,
}

#[derive(Serialize)]
struct Attempt {
    attempt: ClientAttemptId,
    kind: ClientAttemptKind,
    request_id: Option<RequestId>,
    operation: Option<OperationLabel>,
    offset_us: u64,
    duration_us: u64,
    knowledge: ClientCallKnowledge,
    failure: Option<LocalFailureKind>,
    stages: Vec<IpcTiming>,
    omitted_stages: u32,
}

#[derive(Serialize)]
struct IpcTiming {
    stage: ClientStage,
    offset_us: u64,
    duration_us: u64,
    failure: Option<LocalFailureKind>,
}

impl HookGuard {
    pub(crate) fn begin(started: Instant, provider: &'static str, event: &str) -> Option<Self> {
        HOOK.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot.is_some() {
                return None;
            }
            *slot = Some(HookTrace {
                capture: InvocationCaptureHandle::begin(ClientInvocationId::new()),
                started,
                provider,
                event: event.to_owned(),
                scope: None,
                failure: None,
                phases: Vec::new(),
                attempts: Vec::new(),
                omitted_phases: 0,
                omitted_attempts: 0,
            });
            Some(Self)
        })
    }
}

fn retention(id: ClientInvocationId, elapsed: Duration, failed: bool, sample_percent: u8) -> Option<&'static str> {
    if failed {
        Some("failed")
    } else if elapsed >= SLOW {
        Some("slow")
    } else if u32::from(u16::from_be_bytes([id.0.as_bytes()[0], id.0.as_bytes()[1]])) * 100 < u32::from(sample_percent) * 65_536 {
        Some("sampled")
    } else {
        None
    }
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        let Some(trace) = HOOK.with(|slot| slot.borrow_mut().take()) else { return };
        let Some(scope) = trace.scope else { return };
        let trace_id = trace.capture.reference().invocation;
        let elapsed = trace.started.elapsed();
        let sample_percent = std::env::var("AGENTX_HOOK_TRACE_SAMPLE_PERCENT").ok().and_then(|value| value.parse().ok()).filter(|percent| *percent <= 100).unwrap_or(1);
        let Some(selection) = retention(trace_id, elapsed, trace.failure.is_some(), sample_percent) else { return };
        let summary = serde_json::json!({
            "trace_id": trace_id, "provider": trace.provider, "hook": trace.event, "selection": selection,
            "duration_us": micros(elapsed), "failure_phase": trace.failure,
            "phases": trace.phases, "attempts": trace.attempts, "omitted_phases": trace.omitted_phases, "omitted_attempts": trace.omitted_attempts,
        });
        crate::log::hook_trace(&scope, selection, summary);
    }
}

pub(crate) fn claim_scope(scope: &std::path::Path) {
    HOOK.with(|slot| {
        if let Some(trace) = slot.borrow_mut().as_mut() {
            trace.scope = Some(scope.to_owned());
        }
    });
}

pub(crate) fn fail(phase: Phase) {
    HOOK.with(|slot| {
        if let Some(trace) = slot.borrow_mut().as_mut() {
            trace.failure.get_or_insert(phase);
        }
    });
}

pub(crate) fn time<T>(phase: Phase, work: impl FnOnce() -> T) -> T {
    let active = HOOK.with(|slot| slot.borrow().is_some());
    if !active {
        return work();
    }
    let start = Instant::now();
    let output = work();
    let duration_us = micros(start.elapsed());
    HOOK.with(|slot| {
        if let Some(trace) = slot.borrow_mut().as_mut() {
            if trace.phases.len() < MAX_PHASES {
                trace.phases.push(Timing { phase, offset_us: micros(start.saturating_duration_since(trace.started)), duration_us });
            } else {
                trace.omitted_phases = trace.omitted_phases.saturating_add(1);
            }
        }
    });
    output
}

pub(crate) fn invocation_capture() -> Option<InvocationCaptureHandle> {
    HOOK.with(|slot| slot.borrow().as_ref().and_then(|trace| trace.capture.revise().ok()))
}

pub(crate) fn active() -> bool {
    HOOK.with(|slot| slot.borrow().is_some())
}

pub(crate) fn attempt(started: Instant, report: &ClientAttemptReport, operation: Option<OperationLabel>) {
    HOOK.with(|slot| {
        let mut binding = slot.borrow_mut();
        let Some(trace) = binding.as_mut() else { return };
        if trace.attempts.len() == MAX_ATTEMPTS {
            trace.omitted_attempts = trace.omitted_attempts.saturating_add(1);
            return;
        }
        let start = report.started_at.monotonic_ms.0;
        trace.attempts.push(Attempt {
            attempt: report.attempt,
            kind: report.kind,
            request_id: report.request_id.clone(),
            operation,
            offset_us: micros(started.saturating_duration_since(trace.started)),
            duration_us: report.ended_at.monotonic_ms.0.saturating_sub(start).saturating_mul(1000),
            knowledge: report.knowledge.clone(),
            failure: report.failure.as_ref().map(|failure| failure.kind),
            stages: report
                .stages
                .iter()
                .map(|span| IpcTiming {
                    stage: span.stage,
                    offset_us: span.started_at.monotonic_ms.0.saturating_sub(start).saturating_mul(1000),
                    duration_us: span.ended_at.monotonic_ms.0.saturating_sub(span.started_at.monotonic_ms.0).saturating_mul(1000),
                    failure: span.failure,
                })
                .collect(),
            omitted_stages: report.omitted_stages,
        });
    });
}

fn micros(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}
