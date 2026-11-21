//! Bounded volatile runtime state; no Application, storage or transport ownership.
//! Public contracts are in runtime.rs; composition supplies the one process clock.

use super::context::*;
use super::events::{ClientReportObservation, ObservationInput, ObservationPayload, ObservationSink, RecordDisposition};
use super::runtime::*;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

const ACTIVE_REQUESTS: usize = 64;
// Requests repeat scoped identity across refs and add chronology plus up to eight
// configuration refs. Keep their finite budget separate from connection detail.
const REQUEST_DETAIL_BYTES: usize = 8 * 1024;
const CONNECTION_DETAIL_BYTES: usize = 2 * 1024;
const HISTORY_ROWS: usize = 1024;
const HISTORY_BYTES: usize = 2 * 1024 * 1024;
const HISTORY_AGE_MS: u64 = 300_000;
const PROJECTION_ROWS: usize = 256;
const PROJECTION_BYTES: usize = 256 * 1024;
const REPORT_ROWS: usize = 256;
const REPORT_BYTES: usize = 1024 * 1024;

struct SizedRow<T> {
    value: T,
    bytes: usize,
    touched_ms: u64,
}
struct RequestEntry {
    row: Option<SizedRow<RequestObservation>>,
    generation: String,
    enqueued_ms: Option<u64>,
    started_ms: Option<u64>,
    worker_done: bool,
    dispatcher_done: bool,
    queued: bool,
    running: bool,
}
struct ConnectionEntry {
    kind: RuntimeConnectionKind,
    row: Option<ConnectionObservation>,
    peer: Option<u64>,
}
struct MetricBucket {
    second: u64,
    counters: RuntimeCounters,
    queue: LatencyHistogram,
    application: LatencyHistogram,
    reply: LatencyHistogram,
}
struct State {
    daemon: DaemonRef,
    generation: Evidence<String>,
    lifecycle: DaemonLifecycle,
    activity: DispatchActivity,
    changed_at: ObservationTime,
    admission_at: ObservationTime,
    progress_at: ObservationTime,
    revision: u64,
    next_id: u64,
    failed: bool,
    gauges: RuntimeGauges,
    lifetime: RuntimeCounters,
    coverage: RuntimeCoverage,
    buckets: VecDeque<MetricBucket>,
    metric_start: ObservationTime,
    active: BTreeMap<u64, RequestEntry>,
    connections: BTreeMap<u64, ConnectionEntry>,
    requests: BTreeMap<u64, SizedRow<RequestObservation>>,
    closed: BTreeMap<u64, SizedRow<ConnectionObservation>>,
    peers: BTreeMap<u64, SizedRow<PeerObservation>>,
    hosts: BTreeMap<u64, SizedRow<HostObservation>>,
    reports: BTreeMap<u64, SizedRow<RetainedClientReport>>,
}

/// Cloneable narrow runtime capability; all retained state is bounded by constants above.
#[derive(Clone)]
pub(crate) struct RuntimeRegistry {
    state: Arc<Mutex<State>>,
    gate: Arc<Mutex<()>>,
    clock: ObservationClock,
    sink: Arc<Mutex<Option<Arc<dyn ObservationSink>>>>,
}

impl RuntimeRegistry {
    pub(crate) fn new(daemon: DaemonRef, clock: ObservationClock, sink: Option<Arc<dyn ObservationSink>>) -> Result<Self, ObservationValueError> {
        daemon.validate()?;
        let at = clock.now()?;
        let state = State {
            daemon,
            generation: Evidence { value: EvidenceValue::Unknown(UnknownReason::GenerationUnavailable), stamp: EvidenceStamp { origin: EvidenceOrigin::DaemonBoundary, at: at.clone() } },
            lifecycle: DaemonLifecycle::Initializing,
            activity: DispatchActivity::Initializing,
            changed_at: at.clone(),
            admission_at: at.clone(),
            progress_at: at.clone(),
            revision: 0,
            next_id: 0,
            failed: false,
            gauges: RuntimeGauges { business_connections: 0, queued_jobs: 0, executing_jobs: 0, inspection_connections: 0 },
            lifetime: RuntimeCounters::default(),
            coverage: RuntimeCoverage::default(),
            buckets: VecDeque::with_capacity(60),
            metric_start: at,
            active: BTreeMap::new(),
            connections: BTreeMap::new(),
            requests: BTreeMap::new(),
            closed: BTreeMap::new(),
            peers: BTreeMap::new(),
            hosts: BTreeMap::new(),
            reports: BTreeMap::new(),
        };
        Ok(Self { state: Arc::new(Mutex::new(state)), gate: Arc::new(Mutex::new(())), clock, sink: Arc::new(Mutex::new(sink)) })
    }
    #[allow(dead_code, reason = "Composition may share the registry's exact clock origin with additional native consumers")]
    pub(crate) fn clock(&self) -> &ObservationClock {
        &self.clock
    }
    pub(crate) fn now(&self) -> Result<ObservationTime, ObservationValueError> {
        self.clock.now()
    }
    /// Composition supplies a fresh stream-fenced handle after committed reset.
    pub(crate) fn set_sink(&self, sink: Option<Arc<dyn ObservationSink>>) {
        if let Ok(mut current) = self.sink.lock() {
            *current = sink;
        }
    }
    pub(crate) fn generation_unavailable(&self, at: ObservationTime) {
        if let Ok(mut state) = self.state.lock() {
            state.buckets.clear();
            state.metric_start = at.clone();
            state.generation = Evidence { value: EvidenceValue::Unknown(UnknownReason::GenerationUnavailable), stamp: EvidenceStamp { origin: EvidenceOrigin::DaemonBoundary, at } };
            state.bump();
        }
    }
    /// Only the nonblocking native try_send belongs inside publish; never I/O or Application.
    pub(crate) fn publish_queued<T, E>(&self, request: Option<&RequestObservationRef>, publish: impl FnOnce() -> Result<T, E>, rejected: impl FnOnce(&E) -> RuntimeFailureCode) -> Result<T, E> {
        let guard = self.gate.lock().ok();
        let result = publish();
        let update = match &result {
            Ok(_) => RequestUpdate::Queued,
            Err(error) => RequestUpdate::AdmissionRejected(rejected(error)),
        };
        let event = self.update_request(request, update, self.now().ok());
        drop(guard);
        self.emit(event);
        result
    }
    fn emit(&self, event: Option<RuntimeEvent>) {
        let Some(event) = event else {
            return;
        };
        let Some(epoch) = event.correlation.epoch.clone() else {
            return;
        };
        let input = ObservationInput { epoch, source: EvidenceStamp { origin: EvidenceOrigin::DaemonBoundary, at: event.at.clone() }, payload: ObservationPayload::Runtime(event) };
        let sink = self.sink.try_lock().ok().and_then(|s| s.clone());
        if let Some(sink) = sink {
            let _ = sink.record(input);
        }
    }
    fn event(state: &State, at: ObservationTime, source: RuntimeProducer, connection: Option<u64>, request: Option<u64>, fact: RuntimeFact) -> RuntimeEvent {
        let row = request.and_then(|id| state.active.get(&id).and_then(|entry| entry.row.as_ref().map(|row| &row.value)).or_else(|| state.requests.get(&id).map(|row| &row.value)));
        let source_generation = request.and_then(|id| state.active.get(&id).map(|r| r.generation.clone()).or_else(|| state.requests.get(&id).map(|r| r.value.generation_at_decode.clone())));
        let epoch = source_generation.map(|database_generation| ObservationEpoch { daemon: state.daemon.clone(), database_generation }).or_else(|| state.epoch());
        RuntimeEvent {
            at,
            source,
            correlation: RuntimeCorrelation {
                daemon: state.daemon.clone(),
                epoch,
                connection: connection.map(ConnectionId),
                request: request.map(RequestObservationId),
                client_attempt: row.and_then(|row| row.origin.as_ref().map(|origin| origin.attempt)),
                request_id: row.map(|row| row.request_id.clone()),
            },
            fact,
        }
    }
}

impl State {
    fn bump(&mut self) {
        if let Some(next) = self.revision.checked_add(1) {
            self.revision = next;
        } else {
            self.failed = true;
        }
    }
    fn id(&mut self) -> Option<u64> {
        let next = self.next_id.checked_add(1);
        if let Some(next) = next {
            self.next_id = next;
        } else {
            self.failed = true;
        }
        next
    }
    fn epoch(&self) -> Option<ObservationEpoch> {
        match &self.generation.value {
            EvidenceValue::Present(generation) => Some(ObservationEpoch { daemon: self.daemon.clone(), database_generation: generation.clone() }),
            _ => None,
        }
    }
    fn bucket(&mut self, at: &ObservationTime) -> &mut MetricBucket {
        let second = at.monotonic_ms.0 / 1000;
        while self.buckets.front().is_some_and(|b| second.saturating_sub(b.second) >= 60) {
            self.buckets.pop_front();
        }
        if self.buckets.back().is_none_or(|b| b.second != second) {
            if self.buckets.len() == 60 {
                self.buckets.pop_front();
            }
            self.buckets.push_back(MetricBucket { second, counters: RuntimeCounters::default(), queue: LatencyHistogram::default(), application: LatencyHistogram::default(), reply: LatencyHistogram::default() });
        }
        self.buckets.back_mut().expect("bucket inserted")
    }
    fn prune(&mut self, now: u64) {
        let before = self.requests.len() + self.closed.len();
        let all_before = before + self.peers.len() + self.hosts.len() + self.reports.len();
        self.requests.retain(|_, row| now.saturating_sub(row.touched_ms) < HISTORY_AGE_MS);
        self.closed.retain(|_, row| now.saturating_sub(row.touched_ms) < HISTORY_AGE_MS);
        self.peers.retain(|_, row| row.value.open_connections > 0 || now.saturating_sub(row.touched_ms) < 60_000);
        self.hosts.retain(|_, row| now.saturating_sub(row.touched_ms) < HISTORY_AGE_MS);
        self.reports.retain(|_, row| now.saturating_sub(row.touched_ms) < HISTORY_AGE_MS);
        let expired = before - self.requests.len() - self.closed.len();
        if let Some(total) = self.coverage.retained_rows_evicted.checked_add(expired as u64) {
            self.coverage.retained_rows_evicted = total;
        } else {
            self.failed = true;
        }
        while self.requests.len() + self.closed.len() > HISTORY_ROWS || self.requests.values().map(|r| r.bytes).sum::<usize>() + self.closed.values().map(|r| r.bytes).sum::<usize>() > HISTORY_BYTES {
            let request = self.requests.iter().min_by_key(|(_, r)| r.touched_ms).map(|(id, r)| (*id, r.touched_ms));
            let connection = self.closed.iter().min_by_key(|(_, r)| r.touched_ms).map(|(id, r)| (*id, r.touched_ms));
            match (request, connection) {
                (Some((_id, t)), Some((cid, ct))) if t > ct => {
                    self.closed.remove(&cid);
                }
                (Some((id, _)), _) => {
                    self.requests.remove(&id);
                }
                (_, Some((id, _))) => {
                    self.closed.remove(&id);
                }
                _ => break,
            }
            if !increment(&mut self.coverage.retained_rows_evicted) {
                self.failed = true;
            }
        }
        trim(&mut self.peers, PROJECTION_ROWS, PROJECTION_BYTES, |row| row.open_connections == 0);
        trim(&mut self.hosts, PROJECTION_ROWS, PROJECTION_BYTES, |_| true);
        trim(&mut self.reports, REPORT_ROWS, REPORT_BYTES, |_| true);
        if self.requests.len() + self.closed.len() + self.peers.len() + self.hosts.len() + self.reports.len() != all_before {
            self.bump();
        }
    }
    fn complete(&mut self, id: u64, at: &ObservationTime) {
        let complete = self.active.get(&id).is_some_and(|e| e.worker_done && e.dispatcher_done);
        if !complete {
            return;
        }
        if let Some(entry) = self.active.remove(&id)
            && let Some(mut row) = entry.row
        {
            row.touched_ms = at.monotonic_ms.0;
            if let Some(bytes) = encoded_size(&row.value, REQUEST_DETAIL_BYTES) {
                row.bytes = bytes;
                self.requests.insert(id, row);
            } else if !increment(&mut self.coverage.request_details_dropped) {
                self.failed = true;
            }
        }
        self.prune(at.monotonic_ms.0);
    }
}

fn trim<T>(rows: &mut BTreeMap<u64, SizedRow<T>>, count: usize, bytes: usize, may_remove: impl Fn(&T) -> bool) {
    while rows.len() > count || rows.values().map(|r| r.bytes).sum::<usize>() > bytes {
        let oldest = rows.iter().filter(|(_, row)| may_remove(&row.value)).min_by_key(|(_, row)| row.touched_ms).map(|(id, _)| *id);
        if let Some(id) = oldest {
            rows.remove(&id);
        } else {
            break;
        }
    }
}
fn increment(value: &mut u64) -> bool {
    if let Some(next) = value.checked_add(1) {
        *value = next;
        true
    } else {
        false
    }
}
fn histogram(histogram: &mut LatencyHistogram, millis: u64) -> bool {
    let bucket = [1, 5, 20, 100, 500, 2000, 10000].iter().position(|limit| millis <= *limit).unwrap_or(7);
    increment(&mut histogram.counts[bucket])
}
macro_rules! count {
    ($state:expr, $at:expr, $recent:expr, $field:ident) => {{
        let state = &mut *$state;
        if !increment(&mut state.lifetime.$field) {
            state.failed = true;
        }
        if $recent && !increment(&mut state.bucket($at).counters.$field) {
            state.failed = true;
        }
    }};
}

impl RuntimeRegistry {
    /// Rejection before decoding a Call has no request row to update.
    pub(crate) fn business_connection_busy(&self) {
        if let Ok(at) = self.clock.now()
            && let Ok(mut state) = self.state.lock()
            && !state.failed
        {
            count!(state, &at, true, admission_busy);
            state.bump();
        }
    }
}

impl RuntimeRecorder for RuntimeRegistry {
    fn lifecycle(&self, lifecycle: DaemonLifecycle, at: ObservationTime) {
        let event = self.state.lock().ok().and_then(|mut state| {
            if at.clock != *self.clock.id() || state.failed {
                return None;
            }
            state.lifecycle = lifecycle;
            state.changed_at = at.clone();
            state.activity = match lifecycle {
                DaemonLifecycle::Initializing => DispatchActivity::Initializing,
                DaemonLifecycle::Serving => match &state.activity {
                    DispatchActivity::Running(r) => DispatchActivity::Running(r.clone()),
                    _ => DispatchActivity::Idle,
                },
                DaemonLifecycle::Draining => DispatchActivity::Draining(state.active.iter().find(|(_, r)| r.running).map(|(id, _)| RequestObservationRef { daemon: state.daemon.clone(), id: RequestObservationId(*id) })),
                _ => DispatchActivity::Unavailable,
            };
            state.bump();
            Some(Self::event(&state, at, RuntimeProducer::Admission, None, None, RuntimeFact::LifecycleChanged(lifecycle)))
        });
        self.emit(event);
    }
    fn admission_tick(&self, at: ObservationTime) {
        if let Ok(mut state) = self.state.lock()
            && at.clock == *self.clock.id()
        {
            state.admission_at = at;
        }
    }
    fn dispatcher_progress(&self, at: ObservationTime) {
        if let Ok(mut state) = self.state.lock()
            && at.clock == *self.clock.id()
        {
            state.progress_at = at;
        }
    }
    fn generation_published(&self, epoch: ObservationEpoch, at: ObservationTime) {
        if epoch.validate().is_err() || at.clock != *self.clock.id() {
            return;
        }
        if let Ok(mut state) = self.state.lock() {
            if state.daemon != epoch.daemon {
                return;
            }
            if !matches!(&state.generation.value, EvidenceValue::Present(g) if g == &epoch.database_generation) {
                state.buckets.clear();
                state.metric_start = at.clone();
                for host in state.hosts.values_mut() {
                    host.value.state = HostAssociationState::InvalidatedByReset;
                }
            }
            state.generation = Evidence { value: EvidenceValue::Present(epoch.database_generation), stamp: EvidenceStamp { origin: EvidenceOrigin::DaemonBoundary, at } };
            state.bump();
        }
    }
    fn connection_opened(&self, kind: RuntimeConnectionKind, at: ObservationTime) -> RuntimeCapture<ConnectionRef> {
        let Ok(mut state) = self.state.lock() else {
            return RuntimeCapture::Skipped(UnknownReason::ObservationFailed);
        };
        if state.failed || at.clock != *self.clock.id() {
            return RuntimeCapture::Skipped(UnknownReason::ObservationFailed);
        }
        let business = matches!(kind, RuntimeConnectionKind::Business);
        if (business && state.gauges.business_connections >= 32) || (!business && state.gauges.inspection_connections >= 2) {
            return RuntimeCapture::Skipped(UnknownReason::Pressure);
        }
        let Some(id) = state.id() else {
            return RuntimeCapture::Skipped(UnknownReason::ObservationFailed);
        };
        let reference = ConnectionRef { daemon: state.daemon.clone(), id: ConnectionId(id) };
        if business {
            state.gauges.business_connections += 1;
            count!(state, &at, true, accepted_connections);
        } else {
            state.gauges.inspection_connections += 1;
        }
        let row =
            business.then(|| ConnectionObservation { reference: reference.clone(), peer: EvidenceValue::Unknown(UnknownReason::NotCaptured), origin: None, accepted_at: at.clone(), phase: ConnectionPhase::Accepted, request: None, closed: None });
        state.connections.insert(id, ConnectionEntry { kind, row, peer: None });
        state.bump();
        let event = business.then(|| Self::event(&state, at, RuntimeProducer::Admission, Some(id), None, RuntimeFact::ConnectionAccepted));
        drop(state);
        self.emit(event);
        RuntimeCapture::Tracked { reference, detail_retained: business }
    }
    fn peer_observed(&self, connection: &ConnectionRef, process: &crate::service::CallerProcess, at: ObservationTime) -> RuntimeCapture<PeerObservationRef> {
        let Ok(mut state) = self.state.lock() else {
            return RuntimeCapture::Skipped(UnknownReason::ObservationFailed);
        };
        if connection.daemon != state.daemon || process.pid == 0 || at.clock != *self.clock.id() {
            return RuntimeCapture::Skipped(UnknownReason::ObservationFailed);
        }
        let Some(conn) = state.connections.get(&connection.id.0) else {
            return RuntimeCapture::Skipped(UnknownReason::NotCaptured);
        };
        if let Some(id) = conn.peer {
            return RuntimeCapture::Tracked { reference: PeerObservationRef { daemon: state.daemon.clone(), id: PeerObservationId(id) }, detail_retained: true };
        }
        if matches!(conn.kind, RuntimeConnectionKind::Inspection) {
            return RuntimeCapture::Skipped(UnknownReason::NotCaptured);
        }
        let observed = ObservedProcess { pid: process.pid, incarnation: process.incarnation.clone().filter(|s| !s.is_empty()) };
        let existing = observed.incarnation.as_ref().and_then(|_| state.peers.iter().find(|(_, p)| p.value.process == observed).map(|(id, _)| *id));
        let id = if let Some(id) = existing {
            id
        } else {
            if encoded_size(&observed, 512).is_none() {
                if !increment(&mut state.coverage.peer_projections_dropped) {
                    state.failed = true;
                }
                return RuntimeCapture::Skipped(UnknownReason::Pressure);
            }
            if state.peers.len() >= PROJECTION_ROWS {
                if let Some(old) = state.peers.iter().filter(|(_, p)| p.value.open_connections == 0).min_by_key(|(_, p)| p.touched_ms).map(|(id, _)| *id) {
                    state.peers.remove(&old);
                } else {
                    if !increment(&mut state.coverage.peer_projections_dropped) {
                        state.failed = true;
                    }
                    return RuntimeCapture::Skipped(UnknownReason::Pressure);
                }
            }
            let Some(id) = state.id() else {
                return RuntimeCapture::Skipped(UnknownReason::ObservationFailed);
            };
            let value = PeerObservation { reference: PeerObservationRef { daemon: state.daemon.clone(), id: PeerObservationId(id) }, process: observed, first_seen_at: at.clone(), last_seen_at: at.clone(), open_connections: 0 };
            let Some(bytes) = encoded_size(&value, 1024) else {
                return RuntimeCapture::Skipped(UnknownReason::Pressure);
            };
            state.peers.insert(id, SizedRow { value, bytes, touched_ms: at.monotonic_ms.0 });
            id
        };
        let reference = PeerObservationRef { daemon: state.daemon.clone(), id: PeerObservationId(id) };
        if let Some(row) = state.peers.get_mut(&id) {
            row.value.open_connections += 1;
            row.value.last_seen_at = at.clone();
            row.touched_ms = at.monotonic_ms.0;
        }
        if let Some(conn) = state.connections.get_mut(&connection.id.0) {
            conn.peer = Some(id);
            if let Some(row) = &mut conn.row {
                row.peer = EvidenceValue::Present(reference.clone());
                row.phase = ConnectionPhase::PeerValidated;
            }
        }
        state.bump();
        RuntimeCapture::Tracked { reference, detail_retained: true }
    }
    fn connection_changed(&self, connection: Option<&ConnectionRef>, update: ConnectionUpdate, at: ObservationTime) {
        let Some(reference) = connection else {
            return;
        };
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if reference.daemon != state.daemon || at.clock != *self.clock.id() {
            return;
        }
        let Some(mut entry) = state.connections.remove(&reference.id.0) else {
            return;
        };
        let business = matches!(entry.kind, RuntimeConnectionKind::Business);
        let mut closed = false;
        let mut fact = None;
        if let Some(row) = &mut entry.row {
            row.phase = match update {
                ConnectionUpdate::HelloValidated => {
                    fact = Some(RuntimeFact::HelloValidated);
                    ConnectionPhase::HelloValidated
                }
                ConnectionUpdate::ReadyWritten => ConnectionPhase::ReadyWritten,
                ConnectionUpdate::ReadingOperation => ConnectionPhase::ReadingOperation,
                ConnectionUpdate::AwaitingReply => ConnectionPhase::AwaitingReply,
                ConnectionUpdate::Closed(reason) => {
                    row.closed = Some((at.clone(), reason));
                    fact = Some(RuntimeFact::ConnectionClosed(reason));
                    ConnectionPhase::Closed
                }
            };
        }
        if matches!(update, ConnectionUpdate::Closed(_)) {
            closed = true;
            if business {
                state.gauges.business_connections = state.gauges.business_connections.saturating_sub(1);
            } else {
                state.gauges.inspection_connections = state.gauges.inspection_connections.saturating_sub(1);
            }
            if let Some(peer) = entry.peer.and_then(|id| state.peers.get_mut(&id)) {
                peer.value.open_connections = peer.value.open_connections.saturating_sub(1);
                peer.touched_ms = at.monotonic_ms.0;
            }
            if let Some(row) = entry.row.take()
                && let Some(bytes) = encoded_size(&row, CONNECTION_DETAIL_BYTES)
            {
                state.closed.insert(reference.id.0, SizedRow { value: row, bytes, touched_ms: at.monotonic_ms.0 });
            }
        }
        if !closed {
            state.connections.insert(reference.id.0, entry);
        }
        state.bump();
        state.prune(at.monotonic_ms.0);
        let event = fact.map(|fact| Self::event(&state, at, RuntimeProducer::ConnectionWorker, Some(reference.id.0), None, fact));
        drop(state);
        self.emit(event);
    }
    fn call_decoded(&self, input: DecodedCallInput<'_>, at: ObservationTime) -> RuntimeCapture<RequestObservationRef> {
        let Ok(mut state) = self.state.lock() else {
            return RuntimeCapture::Skipped(UnknownReason::ObservationFailed);
        };
        if state.failed || at.clock != *self.clock.id() || uuid::Uuid::parse_str(&input.generation_at_decode).is_err() || state.active.len() >= ACTIVE_REQUESTS {
            if !increment(&mut state.coverage.request_details_dropped) {
                state.failed = true;
            }
            return RuntimeCapture::Skipped(UnknownReason::Pressure);
        }
        let Some(id) = state.id() else {
            return RuntimeCapture::Skipped(UnknownReason::ObservationFailed);
        };
        let recent = matches!(&state.generation.value, EvidenceValue::Present(g) if g == &input.generation_at_decode);
        count!(state, &at, recent, decoded_calls);
        let reference = RequestObservationRef { daemon: state.daemon.clone(), id: RequestObservationId(id) };
        let mut row = None;
        if let Some(connection) = &input.connection {
            if connection.daemon == state.daemon && input.operation.validate() && input.request_id.0.len() <= 1024 && input.origin.as_ref().is_none_or(|origin| encoded_size(origin, 256).is_some()) {
                let value = RequestObservation {
                    reference: reference.clone(),
                    connection: connection.clone(),
                    peer: input.peer,
                    origin: input.origin,
                    request_id: input.request_id.clone(),
                    operation: input.operation.clone(),
                    decoded_at: at.clone(),
                    enqueued_at: None,
                    dequeued_at: None,
                    caller_deadline_unix_ms: input.caller_deadline_unix_ms,
                    effective_deadline: input.effective_deadline,
                    generation_at_decode: input.generation_at_decode.clone(),
                    generation_at_dispatch: None,
                    generation_after_return: EvidenceValue::Unknown(UnknownReason::NotCaptured),
                    execution: ExecutionObservation::Decoded,
                    reply: ReplyObservation::Pending,
                    configuration: ConfigurationTrace::NotEvaluated,
                    post_commit_work: None,
                    last_updated_at: at.clone(),
                };
                if let Some(bytes) = encoded_size(&value, REQUEST_DETAIL_BYTES) {
                    row = Some(SizedRow { value, bytes, touched_ms: at.monotonic_ms.0 });
                }
            }
            if let Some(entry) = state.connections.get_mut(&connection.id.0)
                && let Some(c) = &mut entry.row
            {
                c.request = Some(reference.clone());
                c.origin = row.as_ref().and_then(|r| r.value.origin.clone());
            }
        }
        let retained = row.is_some();
        if !retained && !increment(&mut state.coverage.request_details_dropped) {
            state.failed = true;
        }
        state.active.insert(id, RequestEntry { row, generation: input.generation_at_decode, enqueued_ms: None, started_ms: None, worker_done: false, dispatcher_done: false, queued: false, running: false });
        state.bump();
        let event = input.connection.map(|c| Self::event(&state, at, RuntimeProducer::ConnectionWorker, Some(c.id.0), Some(id), RuntimeFact::CallDecoded(input.operation)));
        drop(state);
        self.emit(event);
        RuntimeCapture::Tracked { reference, detail_retained: retained }
    }
    fn request_changed(&self, request: Option<&RequestObservationRef>, update: RequestUpdate, at: ObservationTime) {
        let gate = self.gate.lock().ok();
        let event = self.update_request(request, update, Some(at));
        drop(gate);
        self.emit(event);
    }
    fn host_validated(&self, input: ValidatedHostObservation, at: ObservationTime) -> RuntimeCapture<HostObservationRef> {
        let Ok(mut state) = self.state.lock() else {
            return RuntimeCapture::Skipped(UnknownReason::ObservationFailed);
        };
        if state.epoch().as_ref() != Some(&input.epoch) || input.request.daemon != state.daemon || input.peer.daemon != state.daemon {
            return RuntimeCapture::Skipped(UnknownReason::GenerationUnavailable);
        }
        let id = input.previous.as_ref().filter(|r| r.daemon == state.daemon && state.hosts.contains_key(&r.id.0)).map(|r| r.id.0).or_else(|| state.id());
        let Some(id) = id else {
            return RuntimeCapture::Skipped(UnknownReason::ObservationFailed);
        };
        let reference = HostObservationRef { daemon: state.daemon.clone(), id: HostObservationId(id) };
        let value = HostObservation {
            reference: reference.clone(),
            epoch: input.epoch,
            kind: input.kind,
            peer: input.peer,
            participant: input.participant,
            target: input.target,
            child: input.child,
            state: input.state,
            last_validated_at: at.clone(),
            last_request: input.request,
        };
        let Some(bytes) = encoded_size(&value, 1024) else {
            if !increment(&mut state.coverage.host_projections_dropped) {
                state.failed = true;
            }
            return RuntimeCapture::Skipped(UnknownReason::Pressure);
        };
        state.hosts.insert(id, SizedRow { value, bytes, touched_ms: at.monotonic_ms.0 });
        state.prune(at.monotonic_ms.0);
        state.bump();
        let event = Self::event(&state, at, RuntimeProducer::Dispatcher, None, None, RuntimeFact::HostAssociationObserved { host: reference.clone(), state: input.state });
        drop(state);
        self.emit(Some(event));
        RuntimeCapture::Tracked { reference, detail_retained: true }
    }
    fn maintain(&self, at: ObservationTime) {
        if let Ok(mut state) = self.state.try_lock()
            && at.clock == *self.clock.id()
        {
            state.prune(at.monotonic_ms.0);
            state.bucket(&at);
        }
    }
}

impl RuntimeRegistry {
    fn update_request(&self, request: Option<&RequestObservationRef>, update: RequestUpdate, at: Option<ObservationTime>) -> Option<RuntimeEvent> {
        let at = at?;
        let mut state = self.state.lock().ok()?;
        if at.clock != *self.clock.id() || state.failed {
            return None;
        }
        let Some(reference) = request else {
            // Ref exhaustion/poison is an observation failure, never fictitious idle gauges.
            state.failed = true;
            return None;
        };
        if reference.daemon != state.daemon {
            return None;
        }
        let id = reference.id.0;
        let mut entry = state.active.remove(&id)?;
        let recent = matches!(&state.generation.value, EvidenceValue::Present(g) if g == &entry.generation);
        let mut fact = None;
        let mut producer = RuntimeProducer::Dispatcher;
        let row = entry.row.as_mut().map(|r| &mut r.value);
        match update {
            RequestUpdate::Queued if !entry.queued && !entry.dispatcher_done && entry.started_ms.is_none() => {
                entry.queued = true;
                entry.enqueued_ms = Some(at.monotonic_ms.0);
                state.gauges.queued_jobs += 1;
                count!(state, &at, recent, queue_admissions);
                if let Some(row) = row {
                    row.execution = ExecutionObservation::Queued { enqueued_at: at.clone() };
                    row.enqueued_at = Some(at.clone());
                }
                fact = Some(RuntimeFact::Queued);
                producer = RuntimeProducer::ConnectionWorker;
            }
            RequestUpdate::Dequeued if entry.queued => {
                entry.queued = false;
                state.gauges.queued_jobs = state.gauges.queued_jobs.saturating_sub(1);
                if let Some(row) = row {
                    row.dequeued_at = Some(at.clone());
                }
                if recent
                    && let Some(enqueued) = entry.enqueued_ms
                    && !histogram(&mut state.bucket(&at).queue, at.monotonic_ms.0.saturating_sub(enqueued))
                {
                    state.failed = true;
                }
                state.progress_at = at.clone();
            }
            RequestUpdate::AdmissionRejected(code) if !entry.queued && entry.started_ms.is_none() && !entry.dispatcher_done => {
                entry.dispatcher_done = true;
                if code == RuntimeFailureCode::Busy {
                    count!(state, &at, recent, admission_busy);
                }
                if let Some(row) = row {
                    row.execution = ExecutionObservation::NotRun { at: at.clone(), reason: NotRunReason::Admission(code) };
                }
                fact = Some(RuntimeFact::AdmissionRejected(code));
                producer = RuntimeProducer::ConnectionWorker;
            }
            RequestUpdate::DispatchRejected(code) if !entry.dispatcher_done && !entry.running => {
                if entry.queued {
                    entry.queued = false;
                    state.gauges.queued_jobs = state.gauges.queued_jobs.saturating_sub(1);
                }
                entry.dispatcher_done = true;
                count!(state, &at, recent, dispatch_not_run);
                if let Some(row) = row {
                    row.execution = ExecutionObservation::NotRun { at: at.clone(), reason: NotRunReason::Dispatch(code) };
                }
                state.progress_at = at.clone();
                fact = Some(RuntimeFact::DispatchRejected(code));
            }
            RequestUpdate::DispatchStarted { generation } if !entry.running && !entry.dispatcher_done && entry.started_ms.is_none() => {
                if entry.queued {
                    entry.queued = false;
                    state.gauges.queued_jobs = state.gauges.queued_jobs.saturating_sub(1);
                }
                entry.running = true;
                entry.started_ms = Some(at.monotonic_ms.0);
                state.gauges.executing_jobs += 1;
                if state.gauges.executing_jobs == 1 {
                    state.activity = if state.lifecycle == DaemonLifecycle::Draining { DispatchActivity::Draining(Some(reference.clone())) } else { DispatchActivity::Running(reference.clone()) };
                }
                count!(state, &at, recent, dispatch_started);
                if let Some(row) = row {
                    row.execution = ExecutionObservation::Running { started_at: at.clone() };
                    row.generation_at_dispatch = Some(generation);
                }
                state.progress_at = at.clone();
                fact = Some(RuntimeFact::DispatchStarted);
            }
            RequestUpdate::ApplicationReturned { result, generation, post_commit_work } if entry.running && !entry.dispatcher_done => {
                entry.running = false;
                entry.dispatcher_done = true;
                state.gauges.executing_jobs = state.gauges.executing_jobs.saturating_sub(1);
                match result {
                    ApplicationResultClass::ReturnedOk => count!(state, &at, recent, application_ok),
                    ApplicationResultClass::ReturnedError(_) => count!(state, &at, recent, application_error),
                }
                let start = entry.started_ms.unwrap_or(at.monotonic_ms.0);
                if recent && !histogram(&mut state.bucket(&at).application, at.monotonic_ms.0.saturating_sub(start)) {
                    state.failed = true;
                }
                if let Some(row) = row {
                    let started_at = match &row.execution {
                        ExecutionObservation::Running { started_at } => started_at.clone(),
                        _ => at.clone(),
                    };
                    row.execution = ExecutionObservation::Returned { started_at, ended_at: at.clone(), result };
                    row.generation_after_return = generation;
                    row.post_commit_work = post_commit_work;
                }
                update_activity(&mut state, DispatchActivity::Idle);
                state.progress_at = at.clone();
                fact = Some(RuntimeFact::ApplicationReturned(result));
            }
            RequestUpdate::DispatcherInterrupted if !entry.dispatcher_done => {
                if entry.queued {
                    state.gauges.queued_jobs = state.gauges.queued_jobs.saturating_sub(1);
                    entry.queued = false;
                }
                if entry.running {
                    state.gauges.executing_jobs = state.gauges.executing_jobs.saturating_sub(1);
                    entry.running = false;
                }
                entry.dispatcher_done = true;
                update_activity(&mut state, DispatchActivity::Unavailable);
                if let Some(row) = row {
                    row.execution = ExecutionObservation::Interrupted { last_observed_at: at.clone() };
                }
            }
            RequestUpdate::ReplyResponseAvailable => {
                if let Some(row) = row {
                    let (wait, write) = reply_parts(&mut row.reply);
                    if matches!(wait, ReplyWaitObservation::Waiting) {
                        *wait = ReplyWaitObservation::ResponseAvailable { at: at.clone() };
                    }
                    let _ = write;
                }
                producer = RuntimeProducer::ConnectionWorker;
            }
            RequestUpdate::ReplyWaitAbandoned(code) => {
                let mut changed = true;
                if let Some(row) = row {
                    let (wait, _) = reply_parts(&mut row.reply);
                    if matches!(wait, ReplyWaitObservation::Abandoned { .. }) {
                        changed = false;
                    } else {
                        *wait = ReplyWaitObservation::Abandoned { at: at.clone(), code };
                    }
                }
                if changed {
                    count!(state, &at, recent, reply_wait_abandoned);
                    fact = Some(RuntimeFact::ReplyWaitAbandoned(code));
                }
                producer = RuntimeProducer::ConnectionWorker;
            }
            RequestUpdate::ReplyWriteFinished { failure } => {
                let mut changed = true;
                if let Some(row) = row {
                    let (_, write) = reply_parts(&mut row.reply);
                    if !matches!(write, ReplyWriteObservation::NotAttempted) {
                        changed = false;
                    } else {
                        *write = match failure {
                            Some(code) => ReplyWriteObservation::Failed { at: at.clone(), code },
                            None => ReplyWriteObservation::Finished { at: at.clone() },
                        };
                    }
                    if changed && failure.is_none() && recent && !histogram(&mut state.bucket(&at).reply, at.monotonic_ms.0.saturating_sub(row.decoded_at.monotonic_ms.0)) {
                        state.failed = true;
                    }
                }
                if changed {
                    if failure.is_some() {
                        count!(state, &at, recent, reply_write_failed);
                    } else {
                        count!(state, &at, recent, reply_write_ok);
                    }
                    fact = Some(RuntimeFact::ReplyWriteFinished { failure });
                }
                producer = RuntimeProducer::ConnectionWorker;
            }
            RequestUpdate::WorkerFinished if !entry.worker_done => {
                entry.worker_done = true;
            }
            RequestUpdate::ConfigurationUsed(capture) => {
                if let Some(row) = row {
                    if matches!(row.configuration, ConfigurationTrace::NotEvaluated) {
                        row.configuration = ConfigurationTrace::Captured { uses: Vec::with_capacity(8), omitted_uses: 0 };
                    }
                    if let ConfigurationTrace::Captured { uses, omitted_uses } = &mut row.configuration {
                        if uses.len() < 8 {
                            uses.push(capture);
                        } else if let Some(next) = omitted_uses.checked_add(1) {
                            *omitted_uses = next;
                        } else {
                            state.failed = true;
                        }
                    }
                }
            }
            _ => {}
        }
        if let Some(row) = &mut entry.row {
            row.value.last_updated_at = at.clone();
            row.touched_ms = at.monotonic_ms.0;
            if let Some(bytes) = encoded_size(&row.value, REQUEST_DETAIL_BYTES) {
                row.bytes = bytes;
            } else {
                entry.row = None;
                if !increment(&mut state.coverage.request_details_dropped) {
                    state.failed = true;
                }
            }
        }
        let connection = entry.row.as_ref().map(|r| r.value.connection.id.0);
        state.active.insert(id, entry);
        state.bump();
        let event = fact.map(|fact| Self::event(&state, at.clone(), producer, connection, Some(id), fact));
        state.complete(id, &at);
        event
    }
}
fn update_activity(state: &mut State, resting: DispatchActivity) {
    let running = state.active.iter().find(|(_, entry)| entry.running).map(|(id, _)| RequestObservationRef { daemon: state.daemon.clone(), id: RequestObservationId(*id) });
    state.activity = if state.lifecycle == DaemonLifecycle::Draining { DispatchActivity::Draining(running) } else { running.map_or(resting, DispatchActivity::Running) };
}

fn reply_parts(reply: &mut ReplyObservation) -> (&mut ReplyWaitObservation, &mut ReplyWriteObservation) {
    if matches!(reply, ReplyObservation::Pending) {
        *reply = ReplyObservation::Observed { wait: ReplyWaitObservation::Waiting, write: ReplyWriteObservation::NotAttempted };
    }
    match reply {
        ReplyObservation::Observed { wait, write } => (wait, write),
        _ => unreachable!("reply initialized"),
    }
}

impl RuntimeReader for RuntimeRegistry {
    fn read(&self, expected: &DaemonRef, peer: &AcceptedPeer, input: RuntimeRequest) -> Result<RuntimeResponse, RuntimeReadError> {
        if matches!(&input, RuntimeRequest::ReportClient(_)) {
            if let RuntimeRequest::ReportClient(report) = input {
                return self.report_client(expected, peer, report).map(RuntimeResponse::ClientReport);
            }
            unreachable!();
        }
        let at = self.now().map_err(|_| RuntimeReadError::Unavailable)?;
        let mut state = self.state.try_lock().map_err(|error| match error {
            std::sync::TryLockError::WouldBlock => RuntimeReadError::Busy,
            std::sync::TryLockError::Poisoned(_) => RuntimeReadError::Unavailable,
        })?;
        authorize(&state.daemon, expected, peer)?;
        if state.failed {
            return Err(RuntimeReadError::Unavailable);
        }
        state.prune(at.monotonic_ms.0);
        match input {
            RuntimeRequest::Snapshot => {
                state.bucket(&at);
                let mut recent = RuntimeMetricWindow {
                    epoch: state.epoch(),
                    from: state.metric_start.clone(),
                    through: at.clone(),
                    counters: RuntimeCounters::default(),
                    queue_wait_ms: LatencyHistogram::default(),
                    application_ms: LatencyHistogram::default(),
                    server_to_reply_ms: LatencyHistogram::default(),
                };
                recent.from.monotonic_ms.0 = recent.from.monotonic_ms.0.max(at.monotonic_ms.0.saturating_sub(59_999));
                recent.from.unix_ms = None;
                for bucket in &state.buckets {
                    if !sum_counters(&mut recent.counters, &bucket.counters)
                        || !sum_hist(&mut recent.queue_wait_ms, &bucket.queue)
                        || !sum_hist(&mut recent.application_ms, &bucket.application)
                        || !sum_hist(&mut recent.server_to_reply_ms, &bucket.reply)
                    {
                        return Err(RuntimeReadError::Unavailable);
                    }
                }
                let oldest_queued_at = state.active.values().filter(|e| e.queued).filter_map(|e| e.enqueued_ms).min().map(|ms| ObservationTime { clock: at.clock.clone(), monotonic_ms: MonotonicMillis(ms), unix_ms: None });
                Ok(RuntimeResponse::Snapshot(RuntimeSnapshot {
                    daemon: state.daemon.clone(),
                    sampled_at: at,
                    generation: state.generation.clone(),
                    lifecycle: state.lifecycle,
                    activity: state.activity.clone(),
                    lifecycle_changed_at: state.changed_at.clone(),
                    admission_tick_at: state.admission_at.clone(),
                    dispatcher_progress_at: state.progress_at.clone(),
                    gauges: state.gauges.clone(),
                    oldest_queued_at,
                    lifetime: state.lifetime.clone(),
                    recent,
                    coverage: state.coverage.clone(),
                    revision: state.revision,
                }))
            }
            RuntimeRequest::Requests { select, page: input } => {
                validate_page(&input)?;
                if matches!(&select, RequestSelection::LogicalRequest(id) if id.0.len() > 1024 || id.0.is_empty()) {
                    return Err(RuntimeReadError::InvalidInput);
                }
                let mut rows: Vec<(u64, &RequestObservation)> = state.active.iter().filter_map(|(id, e)| e.row.as_ref().map(|r| (*id, &r.value))).collect();
                if !matches!(select, RequestSelection::Active) {
                    rows.extend(state.requests.iter().map(|(id, r)| (*id, &r.value)));
                }
                rows.retain(|(_, row)| match &select {
                    RequestSelection::LogicalRequest(id) => row.request_id == *id,
                    RequestSelection::Attempt(attempt) => row.origin.as_ref().is_some_and(|o| o.attempt == *attempt),
                    _ => true,
                });
                Ok(RuntimeResponse::Requests(page(rows, input, &state, at)))
            }
            RuntimeRequest::InvocationRequests { invocation, page: input } => {
                validate_page(&input)?;
                let matching = |row: &RequestObservation| row.origin.as_ref().is_some_and(|origin| origin.invocation.invocation == invocation);
                let rows = state.active.iter().filter_map(|(id, entry)| entry.row.as_ref().map(|row| (*id, &row.value))).chain(state.requests.iter().map(|(id, row)| (*id, &row.value))).filter(|(_, row)| matching(row)).collect();
                Ok(RuntimeResponse::Requests(page(rows, input, &state, at)))
            }
            RuntimeRequest::RequestDetail(reference) => {
                validate_ref(&state.daemon, &reference.daemon, reference.id.0)?;
                state
                    .active
                    .get(&reference.id.0)
                    .and_then(|e| e.row.as_ref().map(|r| r.value.clone()))
                    .or_else(|| state.requests.get(&reference.id.0).map(|r| r.value.clone()))
                    .map(RuntimeResponse::RequestDetail)
                    .ok_or(RuntimeReadError::DetailUnavailable(UnknownReason::NotCaptured))
            }
            RuntimeRequest::Connections(input) => {
                validate_page(&input)?;
                let mut rows: Vec<_> = state.connections.iter().filter_map(|(id, c)| c.row.as_ref().map(|r| (*id, r))).collect();
                rows.extend(state.closed.iter().map(|(id, r)| (*id, &r.value)));
                Ok(RuntimeResponse::Connections(page(rows, input, &state, at)))
            }
            RuntimeRequest::Peers(input) => {
                validate_page(&input)?;
                Ok(RuntimeResponse::Peers(page(state.peers.iter().map(|(id, r)| (*id, &r.value)).collect(), input, &state, at)))
            }
            RuntimeRequest::Hosts(input) => {
                validate_page(&input)?;
                Ok(RuntimeResponse::Hosts(page(state.hosts.iter().map(|(id, r)| (*id, &r.value)).collect(), input, &state, at)))
            }
            RuntimeRequest::ClientReports(input) => {
                validate_page(&input)?;
                Ok(RuntimeResponse::ClientReports(page(state.reports.iter().map(|(id, r)| (*id, &r.value)).collect(), input, &state, at)))
            }
            RuntimeRequest::ClientReportDetail(reference) => {
                validate_ref(&state.daemon, &reference.daemon, reference.id.0)?;
                state.reports.get(&reference.id.0).map(|row| RuntimeResponse::ClientReportDetail(row.value.clone())).ok_or(RuntimeReadError::DetailUnavailable(UnknownReason::NotCaptured))
            }
            RuntimeRequest::ReportClient(_) => unreachable!(),
        }
    }
}
fn authorize(actual: &DaemonRef, expected: &DaemonRef, peer: &AcceptedPeer) -> Result<(), RuntimeReadError> {
    validate_ref(actual, expected, 1)?;
    if peer.connection.daemon != *actual || peer.connection.id.0 == 0 || peer.process.pid == 0 {
        return Err(RuntimeReadError::Forbidden);
    }
    Ok(())
}
fn validate_ref(actual: &DaemonRef, expected: &DaemonRef, id: u64) -> Result<(), RuntimeReadError> {
    if actual.scope != expected.scope {
        Err(RuntimeReadError::ScopeMismatch)
    } else if actual != expected {
        Err(RuntimeReadError::DaemonChanged { actual: actual.clone() })
    } else if id == 0 {
        Err(RuntimeReadError::InvalidInput)
    } else {
        Ok(())
    }
}
fn validate_page(page: &RuntimeList) -> Result<(), RuntimeReadError> {
    if page.limit == 0 || page.limit > 64 || page.before == Some(0) { Err(RuntimeReadError::InvalidInput) } else { Ok(()) }
}
fn page<T: Clone>(mut rows: Vec<(u64, &T)>, input: RuntimeList, state: &State, at: ObservationTime) -> RuntimePage<T> {
    rows.retain(|(id, _)| input.before.is_none_or(|before| *id < before));
    rows.sort_unstable_by_key(|(id, _)| std::cmp::Reverse(*id));
    let limit = usize::from(input.limit);
    let next_before = (rows.len() > limit).then(|| rows[limit - 1].0);
    RuntimePage { rows: rows.into_iter().take(limit).map(|(_, r)| r.clone()).collect(), next_before, sampled_at: at, revision: state.revision, coverage: state.coverage.clone() }
}
fn sum_hist(target: &mut LatencyHistogram, source: &LatencyHistogram) -> bool {
    for (target, source) in target.counts.iter_mut().zip(source.counts) {
        let Some(next) = target.checked_add(source) else {
            return false;
        };
        *target = next;
    }
    true
}
fn sum_counters(target: &mut RuntimeCounters, source: &RuntimeCounters) -> bool {
    macro_rules! sum { ($($field:ident),+) => {$(
        let Some(next) = target.$field.checked_add(source.$field) else { return false; }; target.$field = next;
    )+}; }
    sum!(accepted_connections, decoded_calls, queue_admissions, admission_busy, dispatch_started, dispatch_not_run, application_ok, application_error, reply_write_ok, reply_write_failed, reply_wait_abandoned);
    true
}

impl RuntimeReporter for RuntimeRegistry {
    fn report_client(&self, expected: &DaemonRef, peer: &AcceptedPeer, report: ClientAttemptReport) -> Result<ClientReportReceipt, RuntimeReadError> {
        if !report.validate() {
            return Err(RuntimeReadError::InvalidInput);
        }
        let at = self.now().map_err(|_| RuntimeReadError::Unavailable)?;
        let mut state = self.state.try_lock().map_err(|e| match e {
            std::sync::TryLockError::WouldBlock => RuntimeReadError::Busy,
            std::sync::TryLockError::Poisoned(_) => RuntimeReadError::Unavailable,
        })?;
        authorize(&state.daemon, expected, peer)?;
        if state.failed {
            return Err(RuntimeReadError::Unavailable);
        }
        state.prune(at.monotonic_ms.0);
        if peer.process.incarnation.is_some() {
            for retained in state.reports.values() {
                let previous = &retained.value.observation;
                if previous.submitted_by.process == peer.process && previous.report.attempt == report.attempt {
                    if previous.report != report {
                        return Err(RuntimeReadError::InvalidInput);
                    }
                    return Ok(ClientReportReceipt { attempt: report.attempt, received_at: at, retained: true, reference: Some(retained.value.reference.clone()), event: RecordDisposition::Suppressed });
                }
            }
        }
        let id = state.id().ok_or(RuntimeReadError::Unavailable)?;
        let reference = ClientReportRef { daemon: state.daemon.clone(), id: ClientReportId(id) };
        let observation = ClientReportObservation { report: report.clone(), submitted_by: peer.clone() };
        let value = RetainedClientReport { reference: reference.clone(), observation: observation.clone(), received_at: at.clone() };
        let mut retained = false;
        if let Some(bytes) = encoded_size(&value, 8192) {
            state.reports.insert(id, SizedRow { value, bytes, touched_ms: at.monotonic_ms.0 });
            state.prune(at.monotonic_ms.0);
            retained = state.reports.contains_key(&id);
        }
        if !retained && !increment(&mut state.coverage.local_reports_dropped) {
            state.failed = true;
        }
        let epoch = state.epoch();
        state.bump();
        drop(state);
        let sink = self.sink.try_lock().ok().and_then(|s| s.clone());
        let event = match (epoch, sink) {
            (Some(epoch), Some(sink)) => sink.record(ObservationInput { epoch, source: EvidenceStamp { origin: EvidenceOrigin::ClientReported, at: report.ended_at.clone() }, payload: ObservationPayload::ClientReported(observation) }),
            _ => RecordDisposition::Dropped(super::events::DropReason::CollectorUnavailable),
        };
        Ok(ClientReportReceipt { attempt: report.attempt, received_at: at, retained, reference: retained.then_some(reference), event })
    }
}

/// Admission owns this guard; dropping it closes observation only, never the OS stream.
pub(crate) struct RuntimeConnectionGuard {
    registry: RuntimeRegistry,
    reference: Option<ConnectionRef>,
    close_reason: ConnectionCloseReason,
}
impl RuntimeConnectionGuard {
    pub(crate) fn reference(&self) -> Option<&ConnectionRef> {
        self.reference.as_ref()
    }
    pub(crate) fn close_as(&mut self, reason: ConnectionCloseReason) {
        self.close_reason = reason;
    }
}
impl Drop for RuntimeConnectionGuard {
    fn drop(&mut self) {
        if let Ok(at) = self.registry.now() {
            self.registry.connection_changed(self.reference.as_ref(), ConnectionUpdate::Closed(self.close_reason), at);
        }
    }
}
/// Worker pin survives independent dispatcher work; drop cannot cancel a job.
pub(crate) struct RuntimeWorkerGuard {
    registry: RuntimeRegistry,
    reference: Option<RequestObservationRef>,
}
impl Drop for RuntimeWorkerGuard {
    fn drop(&mut self) {
        if let Ok(at) = self.registry.now() {
            self.registry.request_changed(self.reference.as_ref(), RequestUpdate::WorkerFinished, at);
        }
    }
}
/// Unexpected dispatcher unwind is a positive local observation, not proof of rollback.
pub(crate) struct RuntimeDispatchGuard {
    registry: RuntimeRegistry,
    reference: Option<RequestObservationRef>,
    finished: bool,
}
impl RuntimeDispatchGuard {
    pub(crate) fn finish(&mut self) {
        self.finished = true;
    }
}
impl Drop for RuntimeDispatchGuard {
    fn drop(&mut self) {
        if !self.finished
            && let Ok(at) = self.registry.now()
        {
            self.registry.request_changed(self.reference.as_ref(), RequestUpdate::DispatcherInterrupted, at);
        }
    }
}
impl RuntimeRegistry {
    pub(crate) fn connection_guard(&self, kind: RuntimeConnectionKind) -> RuntimeConnectionGuard {
        let reference = self.now().ok().and_then(|at| self.connection_opened(kind, at).into_reference());
        RuntimeConnectionGuard { registry: self.clone(), reference, close_reason: ConnectionCloseReason::ReadFailure }
    }
    pub(crate) fn worker_guard(&self, reference: Option<RequestObservationRef>) -> RuntimeWorkerGuard {
        RuntimeWorkerGuard { registry: self.clone(), reference }
    }
    pub(crate) fn dispatch_guard(&self, reference: Option<RequestObservationRef>) -> RuntimeDispatchGuard {
        RuntimeDispatchGuard { registry: self.clone(), reference, finished: false }
    }
    #[allow(dead_code, reason = "Typed snapshot convenience is a native reader offering; transport may use the finite read facade instead")]
    pub(crate) fn snapshot(&self, expected: &DaemonRef, peer: &AcceptedPeer) -> Result<RuntimeSnapshot, RuntimeReadError> {
        match self.read(expected, peer, RuntimeRequest::Snapshot)? {
            RuntimeResponse::Snapshot(snapshot) => Ok(snapshot),
            _ => unreachable!("snapshot route"),
        }
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
