//! Passive, authenticated observation transport under the ordinary scope lease.
//! Leaf native contracts remain in observability; this aggregate owns routing,
//! framing and lifetime only. It never constructs Application or managed storage.

use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::{DaemonError, DaemonScope, frame, platform, wire};
use crate::observability::configuration::ConfigurationStore;
use crate::observability::configuration::{ConfigurationError, ConfigurationReader, ConfigurationRequest, ConfigurationResponse};
use crate::observability::context::*;
use crate::observability::events::ObservationJournal;
use crate::observability::events::{ObservationReadError, ObservationRequest, ObservationResponse};
use crate::observability::runtime::*;

#[cfg(all(test, unix))]
#[path = "observation_tests.rs"]
mod tests;

pub(crate) const INSPECTION_TIMEOUT: Duration = Duration::from_millis(1_000);
pub(crate) const INSPECTION_REQUEST_BYTES: usize = 64 * 1024;
pub(crate) const INSPECTION_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_INSPECTION_WORKERS: usize = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[expect(clippy::large_enum_variant, reason = "One finite native request per capped inspection frame; preserve direct leaf constructors for CLI and fixture consumers without lint-only wire-adapter boxing.")]
pub(crate) enum InspectionRequest {
    Runtime(RuntimeRequest),
    Configuration(ConfigurationRequest),
    Events(ObservationRequest),
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[expect(clippy::large_enum_variant, reason = "One finite native result per capped inspection response; retain direct owner result types and the existing consumer contract.")]
pub(crate) enum InspectionResponse {
    DaemonChanged {
        expected: DaemonRef,
        actual: DaemonRef,
    },
    #[schema(value_type = crate::shared::serde_schema::SerdeResult<RuntimeResponse, RuntimeReadError>)]
    Runtime(Result<RuntimeResponse, RuntimeReadError>),
    #[schema(value_type = crate::shared::serde_schema::SerdeResult<ConfigurationResponse, ConfigurationError>)]
    Configuration(Result<ConfigurationResponse, ConfigurationError>),
    #[schema(value_type = crate::shared::serde_schema::SerdeResult<ObservationResponse, ObservationReadError>)]
    Events(Result<ObservationResponse, ObservationReadError>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct InspectionCall {
    pub expected_daemon: DaemonRef,
    pub request: InspectionRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct InspectionResult {
    pub ready: InspectionReady,
    pub response: InspectionResponse,
    pub received_at: ObservationTime,
}

/// The inspection listener uses its own operations, limits and peer policy.
#[derive(Serialize, Deserialize)]
enum InspectionClientFrame<R = InspectionRequest> {
    Call {
        scope: ScopeRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_daemon: Option<DaemonRef>,
        request: R,
    },
}

#[derive(Serialize, Deserialize)]
#[allow(clippy::large_enum_variant, reason = "Retain direct native result types within a capped frame.")]
enum InspectionServerFrame {
    Result { ready: InspectionReady, response: InspectionResponse },
    Failure(DaemonError),
}

/// One retained origin for this originating process, shared by its adapters.
/// Daemon-side stores receive their own explicitly composed clock instead.
pub(crate) fn originating_clock() -> &'static ObservationClock {
    crate::service::context::originating_clock()
}

pub(crate) fn resolve_existing_scope(input: ExistingScopeInput) -> ScopeLookup {
    if input.requested.as_os_str().is_empty() {
        return ScopeLookup::InvalidPath;
    }
    match DaemonScope::existing(&input.requested, &input.cwd) {
        Ok(scope) => match ScopeRef::from_canonical_scope(scope.root().to_path_buf()) {
            Ok(reference) => ScopeLookup::Existing(reference),
            Err(_) => ScopeLookup::InvalidPath,
        },
        Err(error) => match error.kind() {
            std::io::ErrorKind::NotFound => ScopeLookup::Missing,
            std::io::ErrorKind::NotADirectory => ScopeLookup::NotDirectory,
            std::io::ErrorKind::InvalidInput => ScopeLookup::InvalidPath,
            _ => ScopeLookup::Unreadable { os_code: error.raw_os_error() },
        },
    }
}

/// Exactly one authenticated read or report. No acquire/start, Session,
/// ordinary Call, retry, directory repair or managed database access occurs.
pub(crate) fn inspect_existing(scope: &ScopeRef, expected: Option<DaemonRef>, request: InspectionRequest) -> Result<InspectionResult, LocalFailure> {
    let clock = originating_clock();
    let deadline = Instant::now() + INSPECTION_TIMEOUT;
    let local = DaemonScope::existing(&scope.canonical_scope, &scope.canonical_scope).map_err(|error| io_local_failure(error, ClientStage::ScopeLookup, clock))?;
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (local, expected, request, deadline);
        Err(local_failure(DaemonError::Unsupported("authenticated inspection transport unavailable".into()), ClientStage::Connect, clock))
    }
    #[cfg(any(unix, windows))]
    {
        let mut stream = platform::connect_observation(&local, deadline).map_err(|error| local_failure(error, ClientStage::Connect, clock))?;
        let (pid, incarnation) = platform::peer(&stream).map_err(|error| local_failure(error, ClientStage::PeerAuthentication, clock))?;
        let call = InspectionClientFrame::Call { scope: scope.clone(), expected_daemon: expected.clone(), request };
        frame::write_limited(&mut stream, &call, deadline, INSPECTION_REQUEST_BYTES).map_err(|error| local_failure(error, ClientStage::InspectionWrite, clock))?;
        let (ready, response) = match frame::read_limited(&mut stream, deadline, INSPECTION_RESPONSE_BYTES).map_err(|error| local_failure(error, ClientStage::InspectionRead, clock))? {
            InspectionServerFrame::Result { ready, response } => (ready, response),
            InspectionServerFrame::Failure(error) => return Err(local_failure(error, ClientStage::InspectionRead, clock)),
        };
        if ready.daemon.scope != *scope || ready.process.pid != pid || incarnation.as_ref().is_some_and(|value| ready.process.incarnation.as_ref() != Some(value)) {
            return Err(local_failure(DaemonError::Incompatible("inspection scope or kernel peer differs".into()), ClientStage::InspectionRead, clock));
        }
        if let Some(expected) = expected
            && expected != ready.daemon
            && !matches!(&response, InspectionResponse::DaemonChanged { expected: rejected, actual } if *rejected == expected && *actual == ready.daemon)
        {
            return Err(local_failure(DaemonError::Incompatible("inspection daemon instance changed".into()), ClientStage::InspectionRead, clock));
        }
        let received_at = clock.now().map_err(|_| LocalFailure { stage: ClientStage::InspectionRead, kind: LocalFailureKind::ClockUnavailable, os_code: None, at: None })?;
        Ok(InspectionResult { ready, response, received_at })
    }
}

pub(crate) fn failure_kind(error: &DaemonError) -> LocalFailureKind {
    match error {
        DaemonError::Busy => LocalFailureKind::Busy,
        DaemonError::Deadline => LocalFailureKind::TimedOut,
        DaemonError::Incompatible(_) => LocalFailureKind::Incompatible,
        DaemonError::Protocol(_) | DaemonError::InvalidInput(_) => LocalFailureKind::Protocol,
        DaemonError::Unsupported(_) => LocalFailureKind::Unsupported,
        DaemonError::StaleGeneration => LocalFailureKind::StaleGeneration,
        DaemonError::OutcomeUnknown(_) => LocalFailureKind::OutcomeUnknown,
        DaemonError::Io { os_code: Some(code), .. } => io_failure_kind(std::io::Error::from_raw_os_error(*code).kind()),
        _ => LocalFailureKind::Io,
    }
}

pub(crate) fn local_failure(error: DaemonError, stage: ClientStage, clock: &ObservationClock) -> LocalFailure {
    let mut kind = failure_kind(&error);
    if stage == ClientStage::ScopeLookup && kind == LocalFailureKind::EndpointMissing {
        kind = LocalFailureKind::MissingScope;
    }
    let os_code = match error {
        DaemonError::Io { os_code, .. } => os_code,
        _ => None,
    };
    LocalFailure { stage, kind, os_code, at: clock.now().ok() }
}

fn io_failure_kind(kind: std::io::ErrorKind) -> LocalFailureKind {
    match kind {
        std::io::ErrorKind::NotFound => LocalFailureKind::EndpointMissing,
        std::io::ErrorKind::PermissionDenied => LocalFailureKind::PermissionDenied,
        std::io::ErrorKind::ConnectionRefused => LocalFailureKind::ConnectionRefused,
        std::io::ErrorKind::TimedOut => LocalFailureKind::TimedOut,
        std::io::ErrorKind::NotADirectory => LocalFailureKind::NotDirectory,
        _ => LocalFailureKind::Io,
    }
}

pub(crate) fn io_local_failure(error: std::io::Error, stage: ClientStage, clock: &ObservationClock) -> LocalFailure {
    let mut kind = io_failure_kind(error.kind());
    if stage == ClientStage::ScopeLookup && kind == LocalFailureKind::EndpointMissing {
        kind = LocalFailureKind::MissingScope;
    }
    LocalFailure { stage, kind, os_code: error.raw_os_error(), at: clock.now().ok() }
}

/// Same-lease composition only. The publication gate protects a consistent
/// owner-era selection, never Application, SQLite or serialization. Every read
/// also rechecks the authoritative generation mirror before returning a copy.
pub(crate) struct ObservationState {
    pub daemon: DaemonRef,
    pub clock: ObservationClock,
    pub runtime: RuntimeRegistry,
    pub configuration: ConfigurationStore,
    pub journal: ObservationJournal,
    generation: super::DatabaseGeneration,
    publication: RwLock<Option<ObservationEpoch>>,
    transitioning: AtomicBool,
    startup: Option<crate::observability::configuration::ConfigSnapshotBody>,
}

impl ObservationState {
    pub(crate) fn new(lease: &super::ScopeLease) -> Result<Arc<Self>, DaemonError> {
        let scope = ScopeRef::from_canonical_scope(lease.scope().root().to_path_buf()).map_err(|error| DaemonError::Unavailable(error.to_string()))?;
        let daemon = DaemonRef::from_lease(scope, lease.instance().to_owned()).map_err(|error| DaemonError::Unavailable(error.to_string()))?;
        let clock = ObservationClock::new();
        let journal = ObservationJournal::new(daemon.clone(), clock.clone()).map_err(|_| DaemonError::Unavailable("observation journal unavailable".into()))?;
        let runtime = RuntimeRegistry::new(daemon.clone(), clock.clone(), None).map_err(|error| DaemonError::Unavailable(error.to_string()))?;
        let configuration = ConfigurationStore::new(daemon.clone(), clock.clone(), None);
        let startup = (|| {
            use crate::observability::configuration::*;
            let invocation = crate::service::InvocationContext::capture().ok()?;
            let capture = InvocationCaptureHandle::begin(ClientInvocationId::new());
            let mut body = capture.capture_invocation_inputs(CaptureInputs { invocation: &invocation, at: clock.now().ok()?, rules: RulesRevision::current(wire::build_identity()) }).ok()?.body;
            body.subject = ConfigSubject::DaemonStartup { daemon: daemon.clone() };
            body.stage = ConfigStage::StartupInputs;
            body.purpose = ConfigPurpose::StartupCapture;
            body.correlation.invocation = None;
            body.resolution_scope = EvidenceValue::Present(daemon.scope.clone());
            Some(body)
        })();
        let state = Arc::new(Self { daemon, clock, runtime, configuration, journal, generation: lease.database_generation(), publication: RwLock::new(None), transitioning: AtomicBool::new(false), startup });
        state.generation.bind_observation(&state);
        Ok(state)
    }

    pub(crate) fn lifecycle(&self, state: DaemonLifecycle) {
        if let Ok(at) = self.clock.now() {
            self.runtime.lifecycle(state, at);
        }
    }

    /// Commit authority stays in Application. A busy publication gate simply
    /// leaves dataset inspection unavailable until the next control tick.
    pub(crate) fn synchronize_epoch(&self) {
        if self.transitioning.load(Ordering::Acquire) {
            return;
        }
        let current = self.generation.current();
        let Ok(mut publication) = self.publication.try_write() else {
            return;
        };
        if publication.is_none() && current.is_empty() {
            return;
        }
        if publication.as_ref().is_some_and(|epoch| epoch.database_generation == current) {
            return;
        }
        let _ = self.journal.unpublish_epoch();
        self.configuration.invalidate_generation();
        self.configuration.set_sink(None);
        self.runtime.set_sink(None);
        if let Ok(at) = self.clock.now() {
            self.runtime.generation_unavailable(at);
        }
        *publication = None;
        let Ok(epoch) = ObservationEpoch::from_committed_generation(self.daemon.clone(), current.clone()) else {
            return;
        };
        if self.journal.publish_epoch(epoch.clone()).is_err() {
            return;
        }
        let Ok(sink) = self.journal.sink() else {
            return;
        };
        if self.configuration.rotate_epoch(epoch.clone()).is_err() {
            let _ = self.journal.unpublish_epoch();
            return;
        }
        self.configuration.set_sink(Some(sink.clone()));
        self.runtime.set_sink(Some(sink));
        if let Ok(at) = self.clock.now() {
            self.runtime.generation_published(epoch.clone(), at);
        }
        if self.transitioning.load(Ordering::Acquire) || self.generation.current() != current {
            let _ = self.journal.unpublish_epoch();
            return;
        }
        if let Some(body) = &self.startup {
            use crate::observability::configuration::{ConfigurationRecorder, OwnedConfigCapture, OwnedConfigOrigin};
            // Reset republishes only this original safe capture, retaining its
            // startup time. It never reads a new daemon environment as history.
            let _ = self.configuration.record_resolved(OwnedConfigCapture { expected_epoch: epoch.clone(), body: body.clone(), provenance: OwnedConfigOrigin::DaemonCaptured });
        }
        *publication = Some(epoch);
    }

    /// Called by the committed generation mirror before publication/invalidation,
    /// outside SQLite. The journal withdraws its availability atomically even
    /// if a bounded query currently owns its copy lock.
    pub(crate) fn withdraw_epoch(&self) {
        self.transitioning.store(true, Ordering::Release);
        let _ = self.journal.unpublish_epoch();
        self.configuration.invalidate_generation();
        self.configuration.set_sink(None);
        self.runtime.set_sink(None);
        if let Ok(at) = self.clock.now() {
            self.runtime.generation_unavailable(at);
        }
        if let Ok(mut publication) = self.publication.try_write() {
            *publication = None;
        }
    }

    pub(crate) fn generation_committed(&self) {
        self.transitioning.store(false, Ordering::Release);
    }

    fn ready(&self) -> Result<InspectionReady, DaemonError> {
        let sampled_at = self.clock.now().map_err(|error| DaemonError::Unavailable(error.to_string()))?;
        let generation = self.generation.current();
        let value = if !self.transitioning.load(Ordering::Acquire) && uuid::Uuid::parse_str(&generation).is_ok() { EvidenceValue::Present(generation) } else { EvidenceValue::Unknown(UnknownReason::GenerationUnavailable) };
        Ok(InspectionReady {
            daemon: self.daemon.clone(),
            protocol_version: wire::PROTOCOL_VERSION,
            build_version: wire::BUILD_VERSION.into(),
            build_id: wire::build_identity().into(),
            process: ObservedProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
            generation: Evidence { value, stamp: EvidenceStamp { origin: EvidenceOrigin::DaemonBoundary, at: sampled_at.clone() } },
            sampled_at,
        })
    }

    fn route(&self, peer: &AcceptedPeer, call: InspectionCall, response_budget: usize) -> InspectionResponse {
        if call.expected_daemon != self.daemon {
            return InspectionResponse::DaemonChanged { expected: call.expected_daemon, actual: self.daemon.clone() };
        }
        let unavailable = unavailable_response(&call.request);
        if self.transitioning.load(Ordering::Acquire) {
            return unavailable;
        }
        let Ok(publication) = self.publication.try_read() else {
            return unavailable;
        };
        let current = self.generation.current();
        let selected = publication.as_ref().map(|epoch| epoch.database_generation.as_str()).unwrap_or("");
        if current != selected {
            return unavailable;
        }
        let response = match call.request {
            InspectionRequest::Runtime(request) => InspectionResponse::Runtime(self.runtime.read(&self.daemon, peer, request)),
            InspectionRequest::Configuration(request) => InspectionResponse::Configuration(self.configuration.handle(peer, request)),
            InspectionRequest::Events(request) => InspectionResponse::Events(self.journal.handle(request, response_budget)),
        };
        if self.transitioning.load(Ordering::Acquire) || self.generation.current() != current { unavailable } else { response }
    }

    pub(crate) fn dispatch_context(&self, context: &crate::service::RequestContext, request: Option<RequestObservationRef>, peer: Option<PeerObservationRef>, accepted: Option<AcceptedPeer>) -> Option<crate::service::context::ActiveObservation> {
        let publication = self.publication.try_read().ok()?;
        let epoch = publication.as_ref()?.clone();
        if epoch.database_generation != context.database_generation {
            return None;
        }
        Some(crate::service::context::ActiveObservation {
            epoch,
            clock: self.clock.clone(),
            request,
            peer,
            accepted,
            origin: context.caller.invocation.observation_origin.clone(),
            request_id: context.caller.request_id.clone(),
            runtime: Arc::new(self.runtime.clone()),
            configuration: Arc::new(self.configuration.clone()),
            sink: self.journal.sink().ok()?,
            pending_configuration: Default::default(),
        })
    }
}

fn unavailable_response(request: &InspectionRequest) -> InspectionResponse {
    match request {
        InspectionRequest::Runtime(_) => InspectionResponse::Runtime(Err(RuntimeReadError::Unavailable)),
        InspectionRequest::Configuration(_) => InspectionResponse::Configuration(Err(ConfigurationError::Query(crate::observability::configuration::ConfigurationQueryError::Unavailable))),
        InspectionRequest::Events(_) => InspectionResponse::Events(Err(ObservationReadError::Unavailable)),
    }
}

/// Admission alone owns these handles. Completion can occur while waiting for
/// a socket, so every admission reaps again immediately before checking capacity.
#[cfg(any(unix, windows))]
struct InspectionWorkers(Vec<std::thread::JoinHandle<()>>);

#[cfg(any(unix, windows))]
impl InspectionWorkers {
    fn reap_finished(&mut self) {
        let mut index = 0;
        while index < self.0.len() {
            if self.0[index].is_finished() {
                let _ = self.0.swap_remove(index).join();
            } else {
                index += 1;
            }
        }
    }

    /// False drops the accepted connection captured by `work`; real saturation
    /// remains finite rejection, without a queue or a third active worker.
    fn try_spawn(&mut self, work: impl FnOnce() + Send + 'static) -> std::io::Result<bool> {
        self.reap_finished();
        if self.0.len() >= MAX_INSPECTION_WORKERS {
            return Ok(false);
        }
        self.0.push(std::thread::Builder::new().name("agentx-observe".into()).spawn(work)?);
        Ok(true)
    }
}

#[cfg(any(unix, windows))]
impl Drop for InspectionWorkers {
    fn drop(&mut self) {
        for worker in self.0.drain(..) {
            let _ = worker.join();
        }
    }
}

/// Listener is created/dropped on its admission thread; only two short workers
/// can run, with no dispatcher job queue and no live persistence reference.
#[cfg(any(unix, windows))]
pub(crate) fn run(lease: &super::ScopeLease, state: Arc<ObservationState>, stop: Arc<AtomicBool>, started: mpsc::SyncSender<Result<(), DaemonError>>) {
    let listener = match platform::Listener::bind_observation(lease) {
        Ok(listener) => {
            let _ = started.send(Ok(()));
            listener
        }
        Err(error) => {
            let _ = started.send(Err(error));
            return;
        }
    };
    // Declaration order closes the listener before joining its bounded workers.
    let mut workers = InspectionWorkers(Vec::new());
    let listener = listener;
    let mut maintained_at = Instant::now();
    while !stop.load(Ordering::Acquire) {
        if lease.ensure_ownership().is_err() {
            break;
        }
        state.synchronize_epoch();
        if maintained_at.elapsed() >= Duration::from_secs(1) {
            if let Ok(at) = state.clock.now() {
                state.runtime.maintain(at);
            }
            let _ = state.configuration.head(None);
            let _ = state.journal.prune();
            maintained_at = Instant::now();
        }
        workers.reap_finished();
        match listener.wait_ready(Instant::now() + Duration::from_millis(5)) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(_) => break,
        }
        let stream = match listener.accept() {
            Ok(Some(stream)) => stream,
            Ok(None) => continue,
            Err(_) => break,
        };
        let state = state.clone();
        let _ = workers.try_spawn(move || {
            let mut stream = stream;
            let deadline = Instant::now() + INSPECTION_TIMEOUT;
            if let Err(error) = connection(&mut stream, &state, deadline) {
                let _ = frame::write_limited(&mut stream, &InspectionServerFrame::Failure(error), deadline, INSPECTION_RESPONSE_BYTES);
            }
        });
    }
    drop(listener);
    drop(workers);
    state.journal.close();
}

#[cfg(any(unix, windows))]
fn connection(stream: &mut platform::Stream, state: &ObservationState, deadline: Instant) -> Result<(), DaemonError> {
    let _connection = state.runtime.connection_guard(RuntimeConnectionKind::Inspection);
    let reference = _connection.reference().cloned().ok_or(DaemonError::Busy)?;
    let (pid, incarnation) = platform::peer(stream)?;
    let at = state.clock.now().map_err(|error| DaemonError::Unavailable(error.to_string()))?;
    let peer = AcceptedPeer { connection: reference, process: ObservedProcess { pid, incarnation }, observed_at: at };
    let InspectionClientFrame::Call { scope, expected_daemon, request } = frame::read_limited::<InspectionClientFrame<Box<serde_json::value::RawValue>>>(stream, deadline, INSPECTION_REQUEST_BYTES)?;
    if scope != state.daemon.scope {
        return Err(DaemonError::Incompatible("inspection call belongs to another data scope".into()));
    }
    use super::operation::{decode, recognize, recognize_leaf};
    let family = recognize::<InspectionRequest>(request.get())?;
    match family.as_str() {
        "Runtime" => {
            recognize_leaf::<RuntimeRequest>(&request, &family)?;
        }
        "Configuration" => {
            recognize_leaf::<ConfigurationRequest>(&request, &family)?;
        }
        "Events" => {
            recognize_leaf::<ObservationRequest>(&request, &family)?;
        }
        _ => {}
    }
    let call = InspectionCall { expected_daemon: expected_daemon.unwrap_or_else(|| state.daemon.clone()), request: decode(&request)? };
    let ready = state.ready()?;
    // Journal paging already reserves the fixed envelope. Subtract the actual
    // description size so the complete one-frame result fits even for long paths.
    let ready_bytes = frame::encode_limited(&ready, INSPECTION_RESPONSE_BYTES)?.len();
    let response = state.route(&peer, call, INSPECTION_RESPONSE_BYTES.saturating_sub(ready_bytes));
    frame::write_limited(stream, &InspectionServerFrame::Result { ready, response }, deadline, INSPECTION_RESPONSE_BYTES)
}
