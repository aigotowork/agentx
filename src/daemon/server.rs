//! Bounded connection workers feed one application owner. The serving thread
//! alone invokes Application, including generation replacement transactions.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

use super::observation::ObservationState;
use super::transport::{IO_TIMEOUT, request_deadline};
use super::wire::{BUILD_VERSION, ClientFrame, DaemonInfo, PROTOCOL_VERSION, ServerFrame};
use super::{DaemonError, ScopeLease, frame, platform};
use crate::log::DispatchLog;
use crate::observability::{
    context::{EvidenceValue, UnknownReason},
    runtime::*,
};
use crate::service::{Application, CallerProcess, Request, RequestContext};

const MAX_CONNECTIONS: usize = 32;
const QUEUE_CAPACITY: usize = 16;
const READ_QUEUE_CAPACITY: usize = 4;
const CONTROL_TICK: Duration = Duration::from_millis(5);

struct Job {
    operation: Operation,
    submitted: Instant,
    deadline: Instant,
    reply: mpsc::SyncSender<ServerFrame>,
    observation: Option<RequestObservationRef>,
}

type ReadQueue = (fn(&Request) -> bool, mpsc::SyncSender<Job>);

#[derive(Clone)]
struct JobQueues {
    writer: mpsc::SyncSender<Job>,
    reader: Option<ReadQueue>,
    diagnostics: Option<DispatchLog>,
}

impl JobQueues {
    fn select(&self, operation: &Operation) -> (&mpsc::SyncSender<Job>, &'static str) {
        if let (Some((accepts, reader)), Operation::Call { request, .. }) = (&self.reader, operation)
            && accepts(request)
        {
            return (reader, "read");
        }
        (&self.writer, "write")
    }
}

#[derive(Clone, Copy)]
struct DispatchRuntime<'a> {
    observation: Option<&'a ObservationState>,
    diagnostics: Option<&'a DispatchLog>,
    writer: bool,
}

/// The connection worker owns transport progress and its final close reason.
/// Request execution and reply-wait evidence have separate registry lifetimes.
struct ConnectionObservation<'a> {
    state: Option<&'a ObservationState>,
    reference: Option<&'a ConnectionRef>,
    close_reason: ConnectionCloseReason,
    request: Option<RequestObservationRef>,
    worker: Option<RuntimeWorkerGuard>,
    call_decoded: bool,
    reply_write_observed: bool,
}

impl ConnectionObservation<'_> {
    fn write(&mut self, stream: &mut platform::Stream, response: &ServerFrame, deadline: Instant) -> Result<(), DaemonError> {
        self.close_reason = ConnectionCloseReason::WriteFailure;
        let result = frame::write(stream, response, deadline);
        if result.is_ok() {
            self.close_reason = ConnectionCloseReason::Completed;
        }
        result
    }
}

#[expect(clippy::large_enum_variant, reason = "The 16-job dispatcher queue carries finite native request/context variants directly; retain their ownership shape without adding indirection only for this envelope.")]
enum Operation {
    Call { context: RequestContext, request: Request, peer: Option<PeerObservationRef>, accepted: Option<AcceptedPeer> },
    Shutdown { instance: String },
}

/// Stop the other loop on every exit, including unwinding before listener
/// construction. Neither loop may outlive the scoped application/lease owner.
struct StopOnDrop(Arc<AtomicBool>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

/// Listener and worker teardown belongs to the admission thread, including
/// error/panic paths. Closing the listener precedes waiting for bounded I/O.
struct Admission {
    listener: Option<platform::Listener>,
    workers: Vec<std::thread::JoinHandle<()>>,
    stop: Arc<AtomicBool>,
}

impl Admission {
    fn reap_finished(&mut self) {
        let mut index = 0;
        while index < self.workers.len() {
            if self.workers[index].is_finished() {
                let _ = self.workers.swap_remove(index).join();
            } else {
                index += 1;
            }
        }
    }
}

impl Drop for Admission {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        drop(self.listener.take());
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(all(test, unix))]
pub(super) fn run(lease: &ScopeLease, application: &mut impl Application, stop: Arc<AtomicBool>) -> Result<(), DaemonError> {
    run_observed(lease, application, stop, None)
}

pub(super) fn run_observed(lease: &ScopeLease, application: &mut impl Application, stop: Arc<AtomicBool>, observation: Option<Arc<ObservationState>>) -> Result<(), DaemonError> {
    crate::sys::signal::register_term(&stop);
    crate::sys::signal::register_int(&stop);
    let (sender, receiver) = mpsc::sync_channel::<Job>(QUEUE_CAPACITY);
    let diagnostics = DispatchLog::start(lease.scope().root());
    let runtime = DispatchRuntime { observation: observation.as_deref(), diagnostics: diagnostics.as_ref(), writer: true };
    std::thread::scope(|threads| {
        let reader = application.read_lane().map(|lane| {
            let (sender, receiver) = mpsc::sync_channel(READ_QUEUE_CAPACITY);
            let reader_stop = stop.clone();
            let reader = threads.spawn(move || {
                let mut application = lane.application;
                let _ = dispatch_jobs_observed(lease, &mut *application, &receiver, &reader_stop, runtime, false);
                reject_queued(&receiver, runtime.observation);
            });
            ((lane.accepts, sender), reader)
        });
        let (reader_queue, reader) = reader.unzip();
        let queues = JobQueues { writer: sender, reader: reader_queue, diagnostics: diagnostics.clone() };
        let (listening, ready) = mpsc::sync_channel(1);
        let uplink = crate::cloud::uplink::spawn(threads, lease, stop.clone(), ready);
        let admission_stop = stop.clone();
        let admission_observation = observation.clone();
        let admission = threads.spawn(move || {
            let _stop_on_exit = StopOnDrop(admission_stop.clone());
            accept_connections(lease, queues, admission_stop, admission_observation, listening)
        });
        let _stop_on_exit = StopOnDrop(stop.clone());
        // Write authority and maintenance remain on this thread.
        let dispatched = dispatch_jobs_observed(lease, application, &receiver, &stop, runtime, true);
        stop.store(true, Ordering::Release);
        let admitted = admission.join().unwrap_or_else(|_| Err(DaemonError::Unavailable("admission thread panicked; daemon stopped".into())));
        reject_queued(&receiver, observation.as_deref());
        if let Some(reader) = reader {
            let _ = reader.join();
        }
        if let Some(uplink) = uplink {
            let _ = uplink.join();
        }
        dispatched.and(admitted)
    })
}

fn check_admission(lease: &ScopeLease) -> Result<(), DaemonError> {
    lease.ensure_ownership()?;
    if lease.database_generation().current().is_empty() {
        return Err(DaemonError::Unavailable("database generation mirror invalid; stopping admission".into()));
    }
    Ok(())
}

fn accept_connections(lease: &ScopeLease, queues: JobQueues, stop: Arc<AtomicBool>, observation: Option<Arc<ObservationState>>, listening: mpsc::SyncSender<()>) -> Result<(), DaemonError> {
    // In particular, Windows DACL/RefCell/OVERLAPPED ownership is constructed
    // and dropped here; no unsafe Send implementation is needed for Listener.
    let mut admission = Admission { listener: Some(platform::Listener::bind(lease)?), workers: Vec::new(), stop: stop.clone() };
    let _ = listening.try_send(());
    drop(listening);
    if let Some(state) = &observation {
        state.lifecycle(DaemonLifecycle::Serving);
    }
    while !stop.load(Ordering::Acquire) {
        check_admission(lease)?;
        if let Some(state) = &observation
            && let Ok(at) = state.clock.now()
        {
            state.runtime.admission_tick(at);
        }
        admission.reap_finished();
        let listener = admission.listener.as_ref().expect("listener lives through admission");
        if !listener.wait_ready(Instant::now() + CONTROL_TICK)? {
            continue;
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        check_admission(lease)?;
        if let Some(mut stream) = listener.accept()? {
            if admission.workers.len() >= MAX_CONNECTIONS {
                // A slot can finish while the listener is waiting for input.
                admission.reap_finished();
            }
            if admission.workers.len() >= MAX_CONNECTIONS {
                if let Some(state) = &observation {
                    state.runtime.business_connection_busy();
                }
                if let Some(log) = &queues.diagnostics {
                    log.offer_event("WARN", "daemon", "admission.busy", "Native connection capacity reached", &[("daemon_instance", lease.instance()), ("stage", "connection")]);
                }
                let _ = frame::write(&mut stream, &ServerFrame::Failure(DaemonError::Busy), Instant::now() + Duration::from_millis(50));
            } else {
                let queues = queues.clone();
                let info = daemon_info(lease);
                let stop = stop.clone();
                let observation = observation.clone();
                let mut observed_connection = observation.as_ref().map(|state| state.runtime.connection_guard(RuntimeConnectionKind::Business));
                admission.workers.push(std::thread::Builder::new().spawn(move || {
                    let deadline = Instant::now() + IO_TIMEOUT;
                    let mut trace = ConnectionObservation {
                        state: observation.as_deref(),
                        reference: observed_connection.as_ref().and_then(|guard| guard.reference()),
                        close_reason: ConnectionCloseReason::ReadFailure,
                        request: None,
                        worker: None,
                        call_decoded: false,
                        reply_write_observed: false,
                    };
                    let result = connection(&mut stream, info, queues, &stop, deadline, &mut trace);
                    if let Err(error) = &result {
                        let written = frame::write(&mut stream, &ServerFrame::Failure(error.clone()), deadline);
                        if trace.call_decoded && !trace.reply_write_observed {
                            record_request(trace.state, trace.request.as_ref(), RequestUpdate::ReplyWriteFinished { failure: written.as_ref().err().map(runtime_failure) });
                        }
                        if written.is_err() {
                            trace.close_reason = ConnectionCloseReason::WriteFailure;
                        }
                    }
                    let close_reason = if matches!(&result, Err(DaemonError::Deadline)) { ConnectionCloseReason::Deadline } else { trace.close_reason };
                    // Release only after the actual final write attempt. In
                    // particular, timeout does not discard later write evidence.
                    drop(trace);
                    if let Some(guard) = &mut observed_connection {
                        guard.close_as(close_reason);
                    }
                })?);
            }
        }
    }
    if let Some(state) = &observation {
        state.lifecycle(DaemonLifecycle::Draining);
    }
    Ok(())
}

#[cfg(test)]
fn dispatch_jobs(lease: &ScopeLease, application: &mut impl Application, receiver: &mpsc::Receiver<Job>, stop: &AtomicBool) -> Result<(), DaemonError> {
    dispatch_jobs_observed(lease, application, receiver, stop, DispatchRuntime { observation: None, diagnostics: None, writer: true }, true)
}

fn dispatch_jobs_observed(lease: &ScopeLease, application: &mut (impl Application + ?Sized), receiver: &mpsc::Receiver<Job>, stop: &AtomicBool, runtime: DispatchRuntime<'_>, writer: bool) -> Result<(), DaemonError> {
    let result = (|| {
        while !stop.load(Ordering::Acquire) {
            check_admission(lease)?;
            if writer
                && let Some(state) = runtime.observation
                && let Ok(at) = state.clock.now()
            {
                state.runtime.dispatcher_progress(at);
            }
            match receiver.recv_timeout(CONTROL_TICK) {
                Ok(job) => {
                    let response = dispatch_observed(lease, application, job.operation, job.deadline, stop, DispatchRuntime { writer, ..runtime }, (job.observation.as_ref(), Some(job.submitted.elapsed())));
                    let _ = job.reply.send(response);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => application.idle(),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(())
    })();
    if writer {
        stop.store(true, Ordering::Release);
        if let Some(state) = runtime.observation {
            state.lifecycle(DaemonLifecycle::Draining);
        }
    }
    reject_queued(receiver, runtime.observation);
    result
}

fn reject_queued(receiver: &mpsc::Receiver<Job>, observation: Option<&ObservationState>) {
    // No queued request executes after shutdown; current handle() has returned.
    // A worker racing with stop can enqueue later and observes stop in its
    // reply wait. The owner drains once again after joining admission/workers.
    while let Ok(job) = receiver.try_recv() {
        if matches!(&job.operation, Operation::Call { .. }) {
            record_request(observation, job.observation.as_ref(), RequestUpdate::Dequeued);
            record_request(observation, job.observation.as_ref(), RequestUpdate::DispatchRejected(RuntimeFailureCode::Stopping));
        }
        let _ = job.reply.send(ServerFrame::Failure(DaemonError::Unavailable("daemon stopping".into())));
    }
}

fn daemon_info(lease: &ScopeLease) -> DaemonInfo {
    DaemonInfo {
        protocol_version: PROTOCOL_VERSION,
        build_version: BUILD_VERSION.into(),
        build_id: super::wire::build_identity().into(),
        canonical_scope: lease.scope().root().to_path_buf(),
        daemon_instance: lease.instance().into(),
        database_generation: lease.database_generation().current(),
        pid: std::process::id(),
        codex_hook: true,
        agent_names: true,
        workspace_members: true,
        message_history: true,
    }
}

fn connection(stream: &mut platform::Stream, info: DaemonInfo, queues: JobQueues, stop: &AtomicBool, deadline: Instant, trace: &mut ConnectionObservation<'_>) -> Result<(), DaemonError> {
    let (observation, connection) = (trace.state, trace.reference);
    trace.close_reason = ConnectionCloseReason::Rejected;
    let (pid, incarnation) = platform::peer(stream)?;
    trace.close_reason = ConnectionCloseReason::ReadFailure;
    let process = CallerProcess { pid, incarnation };
    let peer = match (observation, connection) {
        (Some(state), Some(connection)) => state.clock.now().ok().and_then(|at| state.runtime.peer_observed(connection, &process, at).into_reference()).map(EvidenceValue::Present).unwrap_or(EvidenceValue::Unknown(UnknownReason::NotCaptured)),
        _ => EvidenceValue::Unknown(UnknownReason::NotCaptured),
    };
    match frame::read(stream, deadline)? {
        ClientFrame::Hello { protocol_version, build_version, build_id, canonical_scope } => {
            if protocol_version != PROTOCOL_VERSION || canonical_scope != info.canonical_scope {
                trace.close_reason = ConnectionCloseReason::Rejected;
                return Err(DaemonError::Incompatible(format!(
                    "daemon protocol={} version={} cohort={} scope={}; client protocol={protocol_version} version={build_version} cohort={build_id} scope={}",
                    info.protocol_version,
                    info.build_version,
                    info.build_id,
                    info.canonical_scope.display(),
                    canonical_scope.display()
                )));
            }
        }
        _ => {
            trace.close_reason = ConnectionCloseReason::Rejected;
            return Err(DaemonError::Protocol("first frame must be Hello".into()));
        }
    }
    record_connection(observation, connection, ConnectionUpdate::HelloValidated);
    trace.write(stream, &ServerFrame::Ready(info.clone()), deadline)?;
    record_connection(observation, connection, ConnectionUpdate::ReadyWritten);
    record_connection(observation, connection, ConnectionUpdate::ReadingOperation);
    trace.close_reason = ConnectionCloseReason::ReadFailure;
    let operation = match frame::read_optional(stream, deadline)? {
        None => {
            trace.close_reason = ConnectionCloseReason::HelloOnlyEof;
            return Ok(());
        }
        Some(ClientFrame::Call { daemon_instance, caller, request }) => {
            trace.call_decoded = true;
            trace.close_reason = ConnectionCloseReason::Rejected;
            let effective = request_deadline(caller.deadline_unix_ms).map(|request| deadline.min(request));
            let observed_peer = match &peer {
                EvidenceValue::Present(peer) => Some(peer.clone()),
                _ => None,
            };
            let accepted = observation.and_then(|state| Some(AcceptedPeer { connection: connection.cloned()?, process: ObservedProcess { pid: process.pid, incarnation: process.incarnation.clone() }, observed_at: state.clock.now().ok()? }));
            let reference = observation.and_then(|state| state.clock.now().ok().map(|at| (state, at))).and_then(|(state, at)| {
                state
                    .runtime
                    .call_decoded(
                        DecodedCallInput {
                            connection: connection.cloned(),
                            peer,
                            origin: caller.invocation.observation_origin.clone(),
                            request_id: &caller.request_id,
                            operation: OperationLabel::of(&request),
                            caller_deadline_unix_ms: caller.deadline_unix_ms,
                            effective_deadline: effective.as_ref().ok().and_then(|deadline| state.clock.at_instant(*deadline).ok()).map(EvidenceValue::Present).unwrap_or(EvidenceValue::Unknown(UnknownReason::ObservationFailed)),
                            generation_at_decode: info.database_generation.clone(),
                        },
                        at,
                    )
                    .into_reference()
            });
            trace.worker = observation.map(|state| state.runtime.worker_guard(reference.clone()));
            trace.request = reference.clone();
            let deadline = match effective {
                Ok(deadline) => deadline,
                Err(error) => {
                    record_request(observation, reference.as_ref(), RequestUpdate::AdmissionRejected(runtime_failure(&error)));
                    return Err(error);
                }
            };
            let context = RequestContext { caller, process, daemon_instance, database_generation: info.database_generation };
            record_connection(observation, connection, ConnectionUpdate::AwaitingReply);
            return enqueue(stream, Operation::Call { context, request, peer: observed_peer, accepted }, &queues, stop, deadline, trace, reference);
        }
        Some(ClientFrame::Shutdown { daemon_instance }) => Operation::Shutdown { instance: daemon_instance },
        _ => {
            trace.close_reason = ConnectionCloseReason::Rejected;
            return Err(DaemonError::Protocol("expected one operation after Hello".into()));
        }
    };
    enqueue(stream, operation, &queues, stop, deadline, trace, None)?;
    trace.close_reason = ConnectionCloseReason::Shutdown;
    Ok(())
}

fn enqueue(stream: &mut platform::Stream, operation: Operation, queues: &JobQueues, stop: &AtomicBool, deadline: Instant, trace: &mut ConnectionObservation<'_>, reference: Option<RequestObservationRef>) -> Result<(), DaemonError> {
    let observation = trace.state.filter(|_| matches!(&operation, Operation::Call { .. }));
    trace.close_reason = ConnectionCloseReason::Rejected;
    if stop.load(Ordering::Acquire) {
        record_request(observation, reference.as_ref(), RequestUpdate::AdmissionRejected(RuntimeFailureCode::Stopping));
        return Err(DaemonError::Unavailable("daemon stopping".into()));
    }
    let (reply, result) = mpsc::sync_channel(1);
    let (sender, lane) = queues.select(&operation);
    let job = Job { operation, submitted: Instant::now(), deadline, reply, observation: reference.clone() };
    let mut refused = None;
    // Retain only safe labels on rejection. Logging runs outside the registry's
    // queue publication guard, and never waits for the filesystem.
    let publish = || {
        sender.try_send(job).map_err(|error| match error {
            mpsc::TrySendError::Full(job) => {
                if queues.diagnostics.is_some() {
                    refused = Some(match job.operation {
                        Operation::Call { context, request, .. } => (context.daemon_instance, OperationLabel::of(&request).variant),
                        Operation::Shutdown { instance } => (instance, "Shutdown".into()),
                    });
                }
                RuntimeFailureCode::Busy
            }
            mpsc::TrySendError::Disconnected(_) => RuntimeFailureCode::DispatcherClosed,
        })
    };
    let published = if let Some(state) = observation { state.runtime.publish_queued(reference.as_ref(), publish, |error| *error) } else { publish() };
    match published {
        Ok(()) => {}
        Err(RuntimeFailureCode::Busy) => {
            if let (Some(log), Some((instance, operation))) = (&queues.diagnostics, refused) {
                log.offer_event("WARN", "daemon", "admission.busy", "Native dispatch queue full", &[("daemon_instance", &instance), ("stage", "queue"), ("dispatch_lane", lane), ("operation", &operation)]);
            }
            return Err(DaemonError::Busy);
        }
        Err(_) => return Err(DaemonError::Unavailable("dispatcher closed".into())),
    }
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            record_request(observation, reference.as_ref(), RequestUpdate::ReplyWaitAbandoned(RuntimeFailureCode::Deadline));
            return Err(DaemonError::OutcomeUnknown("deadline while awaiting dispatcher".into()));
        }
        match result.recv_timeout(remaining.min(Duration::from_millis(20))) {
            Ok(response) => {
                record_request(observation, reference.as_ref(), RequestUpdate::ReplyResponseAvailable);
                let result = trace.write(stream, &response, deadline);
                trace.reply_write_observed = true;
                record_request(observation, reference.as_ref(), RequestUpdate::ReplyWriteFinished { failure: result.as_ref().err().map(runtime_failure) });
                return result;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                record_request(observation, reference.as_ref(), RequestUpdate::ReplyWaitAbandoned(RuntimeFailureCode::DispatcherClosed));
                return Err(DaemonError::OutcomeUnknown("dispatcher closed before reply".into()));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if stop.load(Ordering::Acquire) {
                    record_request(observation, reference.as_ref(), RequestUpdate::ReplyWaitAbandoned(RuntimeFailureCode::Stopping));
                    return Err(DaemonError::OutcomeUnknown("daemon stopping before reply".into()));
                }
            }
        }
    }
}

#[cfg(test)]
fn dispatch(lease: &ScopeLease, application: &mut impl Application, operation: Operation, deadline: Instant, stop: &AtomicBool) -> ServerFrame {
    dispatch_observed(lease, application, operation, deadline, stop, DispatchRuntime { observation: None, diagnostics: None, writer: true }, (None, None))
}

fn dispatch_observed(
    lease: &ScopeLease,
    application: &mut (impl Application + ?Sized),
    operation: Operation,
    deadline: Instant,
    stop: &AtomicBool,
    runtime: DispatchRuntime<'_>,
    timing: (Option<&RequestObservationRef>, Option<Duration>),
) -> ServerFrame {
    let (reference, _) = timing;
    let observed_call = matches!(&operation, Operation::Call { .. });
    let observation = runtime.observation.filter(|_| observed_call);
    record_request(observation, reference, RequestUpdate::Dequeued);
    let mut guard = observation.map(|state| state.runtime.dispatch_guard(reference.cloned()));
    let response = dispatch_inner(lease, application, operation, deadline, stop, DispatchRuntime { observation, ..runtime }, timing);
    if let Some(guard) = &mut guard {
        guard.finish();
    }
    response
}

fn dispatch_inner(
    lease: &ScopeLease,
    application: &mut (impl Application + ?Sized),
    operation: Operation,
    deadline: Instant,
    stop: &AtomicBool,
    runtime: DispatchRuntime<'_>,
    timing: (Option<&RequestObservationRef>, Option<Duration>),
) -> ServerFrame {
    let DispatchRuntime { observation, diagnostics, writer } = runtime;
    let (reference, queue_wait) = timing;
    if stop.load(Ordering::Acquire) {
        record_request(observation, reference, RequestUpdate::DispatchRejected(RuntimeFailureCode::Stopping));
        return ServerFrame::Failure(DaemonError::Unavailable("daemon stopping".into()));
    }
    if Instant::now() >= deadline {
        record_request(observation, reference, RequestUpdate::DispatchRejected(RuntimeFailureCode::Deadline));
        return ServerFrame::Failure(DaemonError::Deadline);
    }
    if let Err(error) = lease.ensure_ownership() {
        record_request(observation, reference, RequestUpdate::DispatchRejected(RuntimeFailureCode::ScopeOwnershipLost));
        stop.store(true, Ordering::Release);
        return ServerFrame::Failure(error);
    }
    match operation {
        Operation::Call { mut context, request, peer, accepted } => {
            let generation = lease.database_generation().current();
            if let Err(error) = validate(&context, lease.instance(), &generation, deadline) {
                record_request(observation, reference, RequestUpdate::DispatchRejected(runtime_failure(&error)));
                return ServerFrame::Failure(error);
            }
            context.database_generation = generation;
            record_request(observation, reference, RequestUpdate::DispatchStarted { generation: context.database_generation.clone() });
            let capture = observation.and_then(|state| state.dispatch_context(&context, reference.cloned(), peer, accepted)).map(|active| active.enter());
            let label = OperationLabel::of(&request);
            let performance = capture.as_ref().map(|_| crate::observability::post_commit::Dispatch::begin());
            let started = Instant::now();
            let mut result = application.handle(&context, request);
            let elapsed = started.elapsed();
            let post_commit_work = performance.as_ref().and_then(|capture| capture.work());
            drop(performance);
            drop(capture);
            let generation = lease.database_generation().current();
            if !writer && generation != context.database_generation {
                result = Err(crate::service::ServiceError::StaleFence);
            }
            if let Some(state) = observation {
                state.synchronize_epoch();
            }
            let after = if generation.is_empty() { EvidenceValue::Unknown(UnknownReason::GenerationUnavailable) } else { EvidenceValue::Present(generation.clone()) };
            record_request(observation, reference, RequestUpdate::ApplicationReturned { result: ApplicationResultClass::of(&result), generation: after, post_commit_work: post_commit_work.clone() });
            // Preserve the actual application return before optional log I/O.
            let wait = queue_wait.unwrap_or_default();
            if let Some(diagnostics) = diagnostics.filter(|_| result.is_err() || elapsed.saturating_add(wait) >= Duration::from_millis(500)) {
                let duration = elapsed.as_millis().to_string();
                let queue = queue_wait.map(|value| value.as_millis().to_string()).unwrap_or_default();
                let origin = context.caller.invocation.observation_origin.as_ref();
                let trace = origin.map(|value| value.invocation.invocation.0.to_string()).unwrap_or_default();
                let attempt = origin.map(|value| value.attempt.0.to_string()).unwrap_or_default();
                let error = result.as_ref().err().map(|error| serde_json::to_string(&ServiceFailureClass::from(error)).unwrap_or_default()).unwrap_or_default();
                let post_commit = post_commit_work.and_then(|work| serde_json::to_string(&work).ok()).unwrap_or_default();
                diagnostics.offer(
                    result.is_err(),
                    &[
                        ("daemon_instance", lease.instance()),
                        ("generation", &context.database_generation),
                        ("native_request_id", &context.caller.request_id.0),
                        ("trace_id", &trace),
                        ("client_attempt_id", &attempt),
                        ("operation", &label.variant),
                        ("dispatch_lane", if writer { "write" } else { "read" }),
                        ("error_code", &error),
                        ("duration_ms", &duration),
                        ("queue_wait_ms", &queue),
                        ("post_commit_work", &post_commit),
                    ],
                );
            }
            if generation.is_empty() {
                stop.store(true, Ordering::Release);
            }
            ServerFrame::Result { result, database_generation: generation }
        }
        Operation::Shutdown { instance } => {
            if instance != lease.instance() {
                return ServerFrame::Failure(DaemonError::Incompatible("daemon instance changed".into()));
            }
            stop.store(true, Ordering::Release);
            ServerFrame::Stopping
        }
    }
}

fn record_request(observation: Option<&ObservationState>, reference: Option<&RequestObservationRef>, update: RequestUpdate) {
    if let Some(state) = observation
        && let Ok(at) = state.clock.now()
    {
        state.runtime.request_changed(reference, update, at);
    }
}

fn record_connection(observation: Option<&ObservationState>, reference: Option<&ConnectionRef>, update: ConnectionUpdate) {
    if let Some(state) = observation
        && let Ok(at) = state.clock.now()
    {
        state.runtime.connection_changed(reference, update, at);
    }
}

fn runtime_failure(error: &DaemonError) -> RuntimeFailureCode {
    match error {
        DaemonError::Busy => RuntimeFailureCode::Busy,
        DaemonError::Deadline => RuntimeFailureCode::Deadline,
        DaemonError::Incompatible(_) => RuntimeFailureCode::Incompatible,
        DaemonError::Protocol(_) => RuntimeFailureCode::Protocol,
        DaemonError::StaleGeneration => RuntimeFailureCode::StaleGeneration,
        DaemonError::OutcomeUnknown(_) => RuntimeFailureCode::OutcomeUnknown,
        DaemonError::Unsupported(_) => RuntimeFailureCode::Unsupported,
        DaemonError::Io { .. } => RuntimeFailureCode::Io,
        _ => RuntimeFailureCode::GenerationUnavailable,
    }
}

fn validate(context: &RequestContext, instance: &str, generation: &str, deadline: Instant) -> Result<(), DaemonError> {
    if Instant::now() >= deadline {
        return Err(DaemonError::Deadline);
    }
    if context.daemon_instance != instance {
        return Err(DaemonError::Incompatible("daemon instance changed".into()));
    }
    if generation.is_empty() {
        return Err(DaemonError::Unavailable("database generation mirror invalid".into()));
    }
    if context.caller.expected_database_generation.as_deref() != Some(generation) {
        return Err(DaemonError::StaleGeneration);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{CallerContext, InvocationContext, RequestId, Response, ServiceResult};

    struct HeldApplication {
        entered: mpsc::SyncSender<()>,
        release: mpsc::Receiver<()>,
        calls: usize,
    }

    impl Application for HeldApplication {
        fn handle(&mut self, _: &RequestContext, _: Request) -> ServiceResult<Response> {
            self.calls += 1;
            if self.calls == 1 {
                self.entered.send(()).unwrap();
                self.release.recv_timeout(Duration::from_secs(10)).expect("release held application call");
            }
            Ok(Response::Participants(crate::agents::service::Response::Participant(None)))
        }
    }

    struct ReleaseOnDrop {
        stop: Arc<AtomicBool>,
        release: Option<mpsc::SyncSender<()>>,
    }

    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Some(release) = self.release.take() {
                let _ = release.try_send(());
            }
        }
    }

    fn fixture_ready(scope: &super::super::DaemonScope) -> DaemonInfo {
        let started = Instant::now();
        loop {
            if let Ok(info) = super::super::transport::probe(scope) {
                return info;
            }
            assert!(started.elapsed() < Duration::from_secs(3), "fixture readiness failed");
            std::thread::park_timeout(CONTROL_TICK);
        }
    }

    struct CountingApplication(usize);
    impl Application for CountingApplication {
        fn handle(&mut self, _: &RequestContext, _: Request) -> ServiceResult<Response> {
            self.0 += 1;
            Ok(Response::Participants(crate::agents::service::Response::Participant(None)))
        }
    }

    fn queued_call(lease: &ScopeLease, generation: &str) -> Operation {
        Operation::Call {
            context: RequestContext {
                caller: CallerContext { request_id: RequestId("queued".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some(generation.into()) },
                process: CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
                daemon_instance: lease.instance().into(),
                database_generation: generation.into(),
            },
            request: Request::Participants(crate::agents::service::Request::Lookup { name: "fixture".into() }),
            peer: None,
            accepted: None,
        }
    }

    #[test]
    fn queued_request_cannot_cross_reset_or_deadline() {
        let directory = tempfile::tempdir().unwrap();
        let lease = ScopeLease::acquire(super::super::DaemonScope::prepare(directory.path()).unwrap()).unwrap();
        let old = uuid::Uuid::new_v4().to_string();
        lease.database_generation().publish(old.clone()).unwrap();
        let queued = queued_call(&lease, &old);
        lease.database_generation().publish(uuid::Uuid::new_v4().to_string()).unwrap();
        let mut application = CountingApplication(0);
        let stop = AtomicBool::new(false);
        assert!(matches!(dispatch(&lease, &mut application, queued, Instant::now() + Duration::from_secs(1), &stop), ServerFrame::Failure(DaemonError::StaleGeneration)));
        let current = lease.database_generation().current();
        assert!(matches!(dispatch(&lease, &mut application, queued_call(&lease, &current), Instant::now() - Duration::from_millis(1), &stop), ServerFrame::Failure(DaemonError::Deadline)));
        assert_eq!(application.0, 0);
    }

    #[test]
    fn readiness_admission_does_not_wait_for_running_application() {
        let directory = tempfile::tempdir().unwrap();
        let scope = super::super::DaemonScope::prepare(directory.path()).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let (entered, running) = mpsc::sync_channel(1);
        let (release, held) = mpsc::sync_channel(1);
        std::thread::scope(|threads| {
            let serving_scope = scope.clone();
            let serving_stop = stop.clone();
            let server = threads.spawn(move || {
                super::super::serve_with_stop(serving_scope, serving_stop, |generation| {
                    generation.publish(uuid::Uuid::new_v4().to_string()).unwrap();
                    Ok(HeldApplication { entered, release: held, calls: 0 })
                })
            });
            // Unwinding must release the application before scope joins server.
            let mut cleanup = ReleaseOnDrop { stop: stop.clone(), release: Some(release) };
            let info = fixture_ready(&scope);
            let call_scope = scope.clone();
            let call = threads.spawn(move || {
                super::super::transport::call(
                    &call_scope,
                    CallerContext { request_id: RequestId("held-call".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some(info.database_generation.clone()) },
                    Request::Participants(crate::agents::service::Request::Lookup { name: "fixture".into() }),
                )
            });
            running.recv_timeout(Duration::from_secs(3)).expect("application call entered");
            // This must complete while handle() is still held at the barrier;
            // it asserts event independence rather than a latency threshold.
            let ready_while_held = super::super::transport::probe(&scope);
            cleanup.release.take().unwrap().send(()).unwrap();
            let response = call.join().unwrap();
            stop.store(true, Ordering::Release);
            server.join().unwrap().unwrap();
            assert!(ready_while_held.is_ok(), "readiness waited for application release: {ready_while_held:?}");
            assert!(matches!(response, Ok(Ok(Response::Participants(_)))));
        });
        // Returning from serve must have joined admission and released lease.
        assert!(ScopeLease::acquire(scope).is_ok());
    }

    #[test]
    fn held_management_reader_does_not_block_business_dispatch() {
        struct SplitApplication(std::cell::RefCell<Option<HeldApplication>>);
        impl Application for SplitApplication {
            fn read_lane(&self) -> Option<crate::service::ReadLane> {
                let application = self.0.borrow_mut().take()?;
                Some(crate::service::ReadLane { accepts: |request| matches!(request, Request::Management(_)), application: Box::new(application) })
            }

            fn handle(&mut self, _: &RequestContext, request: Request) -> ServiceResult<Response> {
                assert!(matches!(request, Request::Participants(_)), "management read must bypass this dispatcher");
                Ok(Response::Participants(crate::agents::service::Response::Participant(None)))
            }
        }
        let directory = tempfile::tempdir().unwrap();
        let scope = super::super::DaemonScope::prepare(directory.path()).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let (entered, running) = mpsc::sync_channel(1);
        let (release, held) = mpsc::sync_channel(1);
        std::thread::scope(|threads| {
            let serving_scope = scope.clone();
            let serving_stop = stop.clone();
            let server = threads.spawn(move || {
                super::super::serve_with_stop_and_management(serving_scope, serving_stop, false, |generation| {
                    generation.publish(uuid::Uuid::new_v4().to_string()).unwrap();
                    Ok(SplitApplication(std::cell::RefCell::new(Some(HeldApplication { entered, release: held, calls: 0 }))))
                })
            });
            let mut cleanup = ReleaseOnDrop { stop: stop.clone(), release: Some(release) };
            let info = fixture_ready(&scope);
            let call_scope = &scope;
            let generation = info.database_generation;
            let call = move |request| {
                let deadline = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64 + 2_000;
                super::super::transport::call(
                    call_scope,
                    CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: InvocationContext::default(), deadline_unix_ms: Some(deadline), expected_database_generation: Some(generation.clone()) },
                    request,
                )
            };
            let reader_call = call.clone();
            let management = threads.spawn(move || reader_call(Request::Management(crate::management::contract::Request::AdoptedWorkspaces)));
            running.recv_timeout(Duration::from_secs(3)).expect("reader entered on another worker");
            let (read_result, read_results) = mpsc::channel();
            let mut burst = Vec::new();
            for _ in 0..READ_QUEUE_CAPACITY + 2 {
                let reader_call = call.clone();
                let result = read_result.clone();
                burst.push(threads.spawn(move || {
                    let reply = reader_call(Request::Management(crate::management::contract::Request::AdoptedWorkspaces));
                    let _ = result.send(reply);
                }));
            }
            assert!(matches!(read_results.recv_timeout(Duration::from_secs(1)), Ok(Err(DaemonError::Busy))), "a full read queue must reject independently");
            let business = call(Request::Participants(crate::agents::service::Request::Lookup { name: "fixture".into() }));
            assert!(matches!(business, Ok(Ok(Response::Participants(_)))));
            cleanup.release.take().unwrap().send(()).unwrap();
            assert!(management.join().unwrap().is_ok());
            for request in burst {
                request.join().unwrap();
            }
            stop.store(true, Ordering::Release);
            server.join().unwrap().unwrap();
        });
        let path = scope.root().join(".tmp/logs/agentx.log");
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let log = std::fs::read_to_string(&path).unwrap_or_default();
            if log
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .any(|entry| entry["event"] == "admission.busy" && entry["stage"] == "queue" && entry["dispatch_lane"] == "read" && entry["operation"] == "AdoptedWorkspaces")
            {
                break;
            }
            assert!(Instant::now() < deadline, "missing read admission diagnostic: {log}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn stopping_drains_current_call_and_rejects_queued_call() {
        let directory = tempfile::tempdir().unwrap();
        let lease = ScopeLease::acquire(super::super::DaemonScope::prepare(directory.path()).unwrap()).unwrap();
        let generation = uuid::Uuid::new_v4().to_string();
        lease.database_generation().publish(generation.clone()).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let (entered, running) = mpsc::sync_channel(1);
        let (release, held) = mpsc::sync_channel(1);
        let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let (first, first_reply) = mpsc::sync_channel(1);
        let (queued, queued_reply) = mpsc::sync_channel(1);
        for reply in [first, queued] {
            assert!(sender.send(Job { operation: queued_call(&lease, &generation), submitted: Instant::now(), deadline: Instant::now() + IO_TIMEOUT, reply, observation: None }).is_ok());
        }
        drop(sender);
        std::thread::scope(|threads| {
            let serving_stop = stop.clone();
            let lease = &lease;
            let dispatcher = threads.spawn(move || {
                let mut application = HeldApplication { entered, release: held, calls: 0 };
                let result = dispatch_jobs(lease, &mut application, &receiver, &serving_stop);
                (result, application.calls)
            });
            let mut cleanup = ReleaseOnDrop { stop: stop.clone(), release: Some(release) };
            running.recv_timeout(Duration::from_secs(3)).expect("application call entered");
            stop.store(true, Ordering::Release);
            cleanup.release.take().unwrap().send(()).unwrap();
            let (result, calls) = dispatcher.join().unwrap();
            result.unwrap();
            assert_eq!(calls, 1);
            assert!(matches!(first_reply.recv().unwrap(), ServerFrame::Result { result: Ok(_), .. }));
            assert!(matches!(queued_reply.recv().unwrap(), ServerFrame::Failure(DaemonError::Unavailable(_))));
        });
    }

    #[cfg(unix)]
    #[test]
    fn listener_construction_failure_stops_and_joins_before_scope_release() {
        let directory = tempfile::tempdir().unwrap();
        let scope = super::super::DaemonScope::prepare(directory.path()).unwrap();
        let lease = ScopeLease::acquire(scope.clone()).unwrap();
        lease.database_generation().publish(uuid::Uuid::new_v4().to_string()).unwrap();
        let endpoint = scope.socket_path().unwrap();
        std::fs::write(&endpoint, b"fixture prevents listener bind").unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let mut application = CountingApplication(0);
        let result = run(&lease, &mut application, stop.clone());
        std::fs::remove_file(&endpoint).unwrap();
        assert!(matches!(result, Err(DaemonError::Unavailable(_))));
        assert!(stop.load(Ordering::Acquire));
        assert_eq!(application.0, 0);
        drop(lease);
        assert!(ScopeLease::acquire(scope).is_ok());
    }

    #[test]
    fn application_panic_stops_and_joins_admission_before_scope_release() {
        struct PanickingApplication;
        impl Application for PanickingApplication {
            fn handle(&mut self, _: &RequestContext, _: Request) -> ServiceResult<Response> {
                panic!("fixture application panicked");
            }
        }
        let directory = tempfile::tempdir().unwrap();
        let scope = super::super::DaemonScope::prepare(directory.path()).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        std::thread::scope(|threads| {
            let serving_scope = scope.clone();
            let serving_stop = stop.clone();
            let server = threads.spawn(move || {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    super::super::serve_with_stop(serving_scope, serving_stop, |generation| {
                        generation.publish(uuid::Uuid::new_v4().to_string()).unwrap();
                        Ok(PanickingApplication)
                    })
                }))
            });
            let _stop_on_exit = StopOnDrop(stop.clone());
            let info = fixture_ready(&scope);
            let response = super::super::transport::call(
                &scope,
                CallerContext { request_id: RequestId("panic-call".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some(info.database_generation) },
                Request::Participants(crate::agents::service::Request::Lookup { name: "fixture".into() }),
            );
            let exited = server.join().unwrap();
            assert!(matches!(response, Err(DaemonError::OutcomeUnknown(_))));
            assert!(exited.is_err());
            assert!(stop.load(Ordering::Acquire));
        });
        assert!(ScopeLease::acquire(scope).is_ok());
    }
}
