//! Native runtime evidence and bounded producer/query ports.
//! See `.docs/observability/runtime.md`; inspection transport remains daemon-owned.

use super::configuration::{ConfigCaptureResult, ConfigStage, InvocationCaptureRef};
use super::context::*;
use super::events::{ClientReportObservation, RecordDisposition};
use crate::agents::attachments::{ActivationTarget, ParticipantRef};
use crate::service::RequestId;
use serde::{Deserialize, Serialize};

#[allow(unused_imports, reason = "Public leaf exposes guard types without requiring consumers to import the private registry")]
pub(crate) use super::registry::{RuntimeConnectionGuard, RuntimeDispatchGuard};
pub(crate) use super::registry::{RuntimeRegistry, RuntimeWorkerGuard};

macro_rules! uuid_identity {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub(crate) struct $name(pub uuid::Uuid);
        impl $name {
            pub fn new() -> Self {
                Self(uuid::Uuid::new_v4())
            }
            pub fn validate(&self) -> bool {
                self.0.get_version_num() == 4 && self.0.get_variant() == uuid::Variant::RFC4122
            }
        }
        // The native serializer emits a validated UUID string, not a Uuid tuple.
        impl utoipa::PartialSchema for $name {
            fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
                <String as utoipa::PartialSchema>::schema()
            }
        }
        impl utoipa::ToSchema for $name {}
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0.to_string())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                let id = uuid::Uuid::parse_str(&value).map_err(serde::de::Error::custom)?;
                if id.get_version_num() != 4 || id.get_variant() != uuid::Variant::RFC4122 {
                    return Err(serde::de::Error::custom("expected UUID v4"));
                }
                Ok(Self(id))
            }
        }
    };
}
uuid_identity!(ClientInvocationId);
uuid_identity!(ClientAttemptId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ConnectionId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RequestObservationId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct PeerObservationId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct HostObservationId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ClientReportId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ConnectionRef {
    pub daemon: DaemonRef,
    pub id: ConnectionId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RequestObservationRef {
    pub daemon: DaemonRef,
    pub id: RequestObservationId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct PeerObservationRef {
    pub daemon: DaemonRef,
    pub id: PeerObservationId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct HostObservationRef {
    pub daemon: DaemonRef,
    pub id: HostObservationId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ClientReportRef {
    pub daemon: DaemonRef,
    pub id: ClientReportId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct OriginRef {
    pub invocation: InvocationCaptureRef,
    pub attempt: ClientAttemptId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservedProcess {
    pub pid: u32,
    pub incarnation: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct AcceptedPeer {
    pub connection: ConnectionRef,
    pub process: ObservedProcess,
    pub observed_at: ObservationTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ExistingScopeInput {
    pub requested: std::path::PathBuf,
    pub cwd: std::path::PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum ScopeLookup {
    Existing(ScopeRef),
    Missing,
    NotDirectory,
    Unreadable { os_code: Option<i32> },
    InvalidPath,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum LocalFailureKind {
    MissingScope,
    NotDirectory,
    PermissionDenied,
    EndpointMissing,
    ConnectionRefused,
    TimedOut,
    Busy,
    Incompatible,
    Protocol,
    Unsupported,
    PeerRejected,
    StaleGeneration,
    OutcomeUnknown,
    ClockUnavailable,
    Io,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct LocalFailure {
    pub stage: ClientStage,
    pub kind: LocalFailureKind,
    pub os_code: Option<i32>,
    /// None establishes no ordering or age; never substitute a sentinel or a new clock.
    pub at: Option<ObservationTime>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct InspectionReady {
    pub daemon: DaemonRef,
    pub protocol_version: u32,
    pub build_version: String,
    pub build_id: String,
    pub process: ObservedProcess,
    pub generation: Evidence<String>,
    pub sampled_at: ObservationTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum OperationDomain {
    Inspection,
    Management,
    Collaboration,
    Participants,
    Messages,
    Hosts,
    Events,
    Queries,
    Maintenance,
    Relay,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct OperationLabel {
    pub domain: OperationDomain,
    pub variant: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum RuntimeFailureCode {
    Busy,
    Deadline,
    Stopping,
    DispatcherClosed,
    ScopeOwnershipLost,
    Incompatible,
    Protocol,
    StaleGeneration,
    GenerationUnavailable,
    OutcomeUnknown,
    Unsupported,
    Io,
    Internal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ServiceFailureClass {
    InvalidInput,
    IdentityRequired,
    Forbidden,
    NotFound,
    Conflict,
    StaleFence,
    Unavailable,
    Unsupported,
    OutcomeUnknown,
    Internal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ApplicationResultClass {
    ReturnedOk,
    ReturnedError(ServiceFailureClass),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum NotRunReason {
    Admission(RuntimeFailureCode),
    Dispatch(RuntimeFailureCode),
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ExecutionObservation {
    Decoded,
    Queued { enqueued_at: ObservationTime },
    NotRun { at: ObservationTime, reason: NotRunReason },
    Running { started_at: ObservationTime },
    Returned { started_at: ObservationTime, ended_at: ObservationTime, result: ApplicationResultClass },
    Interrupted { last_observed_at: ObservationTime },
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ReplyObservation {
    Pending,
    Observed { wait: ReplyWaitObservation, write: ReplyWriteObservation },
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ReplyWaitObservation {
    Waiting,
    ResponseAvailable { at: ObservationTime },
    Abandoned { at: ObservationTime, code: RuntimeFailureCode },
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ReplyWriteObservation {
    NotAttempted,
    Finished { at: ObservationTime },
    Failed { at: ObservationTime, code: RuntimeFailureCode },
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ConfigurationUse {
    pub stage: ConfigStage,
    pub capture: ConfigCaptureResult,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ConfigurationTrace {
    NotEvaluated,
    Captured { uses: Vec<ConfigurationUse>, omitted_uses: u32 },
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RequestObservation {
    pub reference: RequestObservationRef,
    pub connection: ConnectionRef,
    pub peer: EvidenceValue<PeerObservationRef>,
    pub origin: Option<OriginRef>,
    pub request_id: RequestId,
    pub operation: OperationLabel,
    pub decoded_at: ObservationTime,
    pub enqueued_at: Option<ObservationTime>,
    pub dequeued_at: Option<ObservationTime>,
    pub caller_deadline_unix_ms: Option<u64>,
    pub effective_deadline: EvidenceValue<ObservationTime>,
    pub generation_at_decode: String,
    pub generation_at_dispatch: Option<String>,
    pub generation_after_return: EvidenceValue<String>,
    pub execution: ExecutionObservation,
    pub reply: ReplyObservation,
    pub configuration: ConfigurationTrace,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_commit_work: Option<super::post_commit::PostCommitWork>,
    pub last_updated_at: ObservationTime,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ConnectionPhase {
    Accepted,
    PeerValidated,
    HelloValidated,
    ReadyWritten,
    ReadingOperation,
    AwaitingReply,
    Closed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ConnectionCloseReason {
    HelloOnlyEof,
    Completed,
    ReadFailure,
    WriteFailure,
    Deadline,
    Rejected,
    Shutdown,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ConnectionObservation {
    pub reference: ConnectionRef,
    pub peer: EvidenceValue<PeerObservationRef>,
    pub origin: Option<OriginRef>,
    pub accepted_at: ObservationTime,
    pub phase: ConnectionPhase,
    pub request: Option<RequestObservationRef>,
    pub closed: Option<(ObservationTime, ConnectionCloseReason)>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct PeerObservation {
    pub reference: PeerObservationRef,
    pub process: ObservedProcess,
    pub first_seen_at: ObservationTime,
    pub last_seen_at: ObservationTime,
    pub open_connections: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum HostObservationKind {
    NativeRoot,
    ProcessHost,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum HostAssociationState {
    Registered,
    Renewed,
    Released,
    InvalidatedByReset,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct HostObservation {
    pub reference: HostObservationRef,
    pub epoch: ObservationEpoch,
    pub kind: HostObservationKind,
    pub peer: PeerObservationRef,
    pub participant: ParticipantRef,
    pub target: Option<ActivationTarget>,
    pub child: Option<ObservedProcess>,
    pub state: HostAssociationState,
    pub last_validated_at: ObservationTime,
    pub last_request: RequestObservationRef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum DaemonLifecycle {
    Initializing,
    Serving,
    Draining,
    Stopping,
    Failed,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum DispatchActivity {
    Initializing,
    Idle,
    Running(RequestObservationRef),
    Draining(Option<RequestObservationRef>),
    Unavailable,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RuntimeGauges {
    pub business_connections: u16,
    pub queued_jobs: u16,
    pub executing_jobs: u8,
    pub inspection_connections: u8,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RuntimeCounters {
    pub accepted_connections: u64,
    pub decoded_calls: u64,
    pub queue_admissions: u64,
    pub admission_busy: u64,
    pub dispatch_started: u64,
    pub dispatch_not_run: u64,
    pub application_ok: u64,
    pub application_error: u64,
    pub reply_write_ok: u64,
    pub reply_write_failed: u64,
    pub reply_wait_abandoned: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct LatencyHistogram {
    pub counts: [u64; 8],
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RuntimeMetricWindow {
    pub epoch: Option<ObservationEpoch>,
    pub from: ObservationTime,
    pub through: ObservationTime,
    pub counters: RuntimeCounters,
    pub queue_wait_ms: LatencyHistogram,
    pub application_ms: LatencyHistogram,
    pub server_to_reply_ms: LatencyHistogram,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RuntimeCoverage {
    pub request_details_dropped: u64,
    pub retained_rows_evicted: u64,
    pub peer_projections_dropped: u64,
    pub host_projections_dropped: u64,
    pub local_reports_dropped: u64,
    pub full_host_census: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RuntimeSnapshot {
    pub daemon: DaemonRef,
    pub sampled_at: ObservationTime,
    pub generation: Evidence<String>,
    pub lifecycle: DaemonLifecycle,
    pub activity: DispatchActivity,
    pub lifecycle_changed_at: ObservationTime,
    pub admission_tick_at: ObservationTime,
    pub dispatcher_progress_at: ObservationTime,
    pub gauges: RuntimeGauges,
    pub oldest_queued_at: Option<ObservationTime>,
    pub lifetime: RuntimeCounters,
    pub recent: RuntimeMetricWindow,
    pub coverage: RuntimeCoverage,
    pub revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum RequestSelection {
    AllRetained,
    Active,
    LogicalRequest(RequestId),
    Attempt(ClientAttemptId),
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RuntimeList {
    pub before: Option<u64>,
    pub limit: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RuntimePage<T> {
    pub rows: Vec<T>,
    pub next_before: Option<u64>,
    pub sampled_at: ObservationTime,
    pub revision: u64,
    pub coverage: RuntimeCoverage,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant, reason = "Finite native inspection request keeps its capped terminal report inline without changing the wire contract")]
pub(crate) enum RuntimeRequest {
    Snapshot,
    Requests { select: RequestSelection, page: RuntimeList },
    InvocationRequests { invocation: ClientInvocationId, page: RuntimeList },
    RequestDetail(RequestObservationRef),
    Connections(RuntimeList),
    Peers(RuntimeList),
    Hosts(RuntimeList),
    ClientReports(RuntimeList),
    ClientReportDetail(ClientReportRef),
    ReportClient(ClientAttemptReport),
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum RuntimeResponse {
    Snapshot(RuntimeSnapshot),
    Requests(RuntimePage<RequestObservation>),
    RequestDetail(RequestObservation),
    Connections(RuntimePage<ConnectionObservation>),
    Peers(RuntimePage<PeerObservation>),
    Hosts(RuntimePage<HostObservation>),
    ClientReport(ClientReportReceipt),
    ClientReports(RuntimePage<RetainedClientReport>),
    ClientReportDetail(RetainedClientReport),
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum RuntimeReadError {
    InvalidInput,
    ScopeMismatch,
    DaemonChanged { actual: DaemonRef },
    DetailUnavailable(UnknownReason),
    Forbidden,
    Busy,
    Unavailable,
    Unsupported,
}
pub(crate) trait RuntimeReader {
    fn read(&self, expected: &DaemonRef, peer: &AcceptedPeer, input: RuntimeRequest) -> Result<RuntimeResponse, RuntimeReadError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ClientAttemptKind {
    Acquire,
    Probe,
    Call,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ClientStage {
    Capture,
    ScopeLookup,
    EndpointLookup,
    Discovery,
    StartupLock,
    Spawn,
    Connect,
    PeerAuthentication,
    Hello,
    CallWrite,
    ReplyRead,
    InspectionRead,
    InspectionWrite,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ClientStageSpan {
    pub stage: ClientStage,
    pub started_at: ObservationTime,
    pub ended_at: ObservationTime,
    pub failure: Option<LocalFailureKind>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ClientCallKnowledge {
    NotWritten,
    MayHaveBeenWritten,
    ReceivedFailure(RuntimeFailureCode),
    ReceivedApplicationResult(ApplicationResultClass),
    DiscoveryOnly,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ClientAttemptReport {
    pub attempt: ClientAttemptId,
    pub kind: ClientAttemptKind,
    pub invocation: Option<InvocationCaptureRef>,
    pub request_id: Option<RequestId>,
    #[schema(value_type = String)]
    pub requested_scope: std::path::PathBuf,
    pub resolved_scope: EvidenceValue<ScopeRef>,
    pub started_at: ObservationTime,
    pub ended_at: ObservationTime,
    pub stages: Vec<ClientStageSpan>,
    pub omitted_stages: u32,
    pub last_ready: Option<InspectionReady>,
    pub original_epoch: Option<ObservationEpoch>,
    pub knowledge: ClientCallKnowledge,
    pub failure: Option<LocalFailure>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ClientReportReceipt {
    pub attempt: ClientAttemptId,
    pub received_at: ObservationTime,
    pub retained: bool,
    pub reference: Option<ClientReportRef>,
    pub event: RecordDisposition,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RetainedClientReport {
    pub reference: ClientReportRef,
    pub observation: ClientReportObservation,
    pub received_at: ObservationTime,
}

#[derive(Clone, Debug)]
#[allow(dead_code, reason = "Producer result exposes detail coverage and skipped reason even when composition only needs the reference")]
pub(crate) enum RuntimeCapture<T> {
    Tracked { reference: T, detail_retained: bool },
    Skipped(UnknownReason),
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum RuntimeConnectionKind {
    Business,
    Inspection,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum ConnectionUpdate {
    ReadingOperation,
    AwaitingReply,
    Closed(ConnectionCloseReason),
}
#[derive(Clone, Debug)]
pub(crate) struct DecodedCallInput<'a> {
    pub connection: Option<ConnectionRef>,
    pub peer: EvidenceValue<PeerObservationRef>,
    pub origin: Option<OriginRef>,
    pub request_id: &'a RequestId,
    pub operation: OperationLabel,
    pub caller_deadline_unix_ms: Option<u64>,
    pub effective_deadline: EvidenceValue<ObservationTime>,
    pub generation_at_decode: String,
}
#[derive(Clone, Debug)]
pub(crate) enum RequestUpdate {
    Queued,
    Dequeued,
    AdmissionRejected(RuntimeFailureCode),
    DispatchRejected(RuntimeFailureCode),
    DispatchStarted { generation: String },
    ApplicationReturned { result: ApplicationResultClass, generation: EvidenceValue<String>, post_commit_work: Option<super::post_commit::PostCommitWork> },
    DispatcherInterrupted,
    WorkerFinished,
    ReplyResponseAvailable,
    ReplyWaitAbandoned(RuntimeFailureCode),
    ReplyWriteFinished { failure: Option<RuntimeFailureCode> },
    ConfigurationUsed(ConfigurationUse),
}
#[derive(Clone, Debug)]
pub(crate) struct ValidatedHostObservation {
    pub epoch: ObservationEpoch,
    pub previous: Option<HostObservationRef>,
    pub kind: HostObservationKind,
    pub peer: PeerObservationRef,
    pub participant: ParticipantRef,
    pub target: Option<ActivationTarget>,
    pub child: Option<ObservedProcess>,
    pub state: HostAssociationState,
    pub request: RequestObservationRef,
}
pub(crate) trait RuntimeRecorder: Send + Sync {
    fn lifecycle(&self, state: DaemonLifecycle, at: ObservationTime);
    fn admission_tick(&self, at: ObservationTime);
    fn dispatcher_progress(&self, at: ObservationTime);
    fn generation_published(&self, epoch: ObservationEpoch, at: ObservationTime);
    fn connection_opened(&self, kind: RuntimeConnectionKind, at: ObservationTime) -> RuntimeCapture<ConnectionRef>;
    fn peer_observed(&self, connection: &ConnectionRef, process: &crate::service::CallerProcess, at: ObservationTime) -> RuntimeCapture<PeerObservationRef>;
    fn connection_changed(&self, connection: Option<&ConnectionRef>, update: ConnectionUpdate, at: ObservationTime);
    fn call_decoded(&self, input: DecodedCallInput<'_>, at: ObservationTime) -> RuntimeCapture<RequestObservationRef>;
    fn request_changed(&self, request: Option<&RequestObservationRef>, update: RequestUpdate, at: ObservationTime);
    fn host_validated(&self, input: ValidatedHostObservation, at: ObservationTime) -> RuntimeCapture<HostObservationRef>;
    fn maintain(&self, at: ObservationTime);
}
pub(crate) trait RuntimeReporter {
    fn report_client(&self, expected: &DaemonRef, peer: &AcceptedPeer, report: ClientAttemptReport) -> Result<ClientReportReceipt, RuntimeReadError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum RuntimeProducer {
    Admission,
    ConnectionWorker,
    Dispatcher,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RuntimeCorrelation {
    pub daemon: DaemonRef,
    pub epoch: Option<ObservationEpoch>,
    pub connection: Option<ConnectionId>,
    pub request: Option<RequestObservationId>,
    pub client_attempt: Option<ClientAttemptId>,
    pub request_id: Option<RequestId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum RuntimeFact {
    LifecycleChanged(DaemonLifecycle),
    ConnectionAccepted,
    HelloValidated,
    ConnectionClosed(ConnectionCloseReason),
    CallDecoded(OperationLabel),
    Queued,
    AdmissionRejected(RuntimeFailureCode),
    DispatchStarted,
    DispatchRejected(RuntimeFailureCode),
    ApplicationReturned(ApplicationResultClass),
    ReplyWriteFinished { failure: Option<RuntimeFailureCode> },
    ReplyWaitAbandoned(RuntimeFailureCode),
    HostAssociationObserved { host: HostObservationRef, state: HostAssociationState },
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct RuntimeEvent {
    pub at: ObservationTime,
    pub source: RuntimeProducer,
    pub correlation: RuntimeCorrelation,
    pub fact: RuntimeFact,
}

impl<T> RuntimeCapture<T> {
    #[allow(dead_code, reason = "Borrowing producer adapters can inspect a reference without consuming its coverage result")]
    pub fn reference(&self) -> Option<&T> {
        match self {
            Self::Tracked { reference, .. } => Some(reference),
            Self::Skipped(_) => None,
        }
    }
    pub fn into_reference(self) -> Option<T> {
        match self {
            Self::Tracked { reference, .. } => Some(reference),
            Self::Skipped(_) => None,
        }
    }
}

impl From<&crate::service::ServiceError> for ServiceFailureClass {
    fn from(error: &crate::service::ServiceError) -> Self {
        use crate::service::ServiceError as E;
        match error {
            E::InvalidInput(_) => Self::InvalidInput,
            E::IdentityRequired(_) => Self::IdentityRequired,
            E::Forbidden(_) => Self::Forbidden,
            E::NotFound(_) => Self::NotFound,
            E::Conflict(_) => Self::Conflict,
            E::StaleFence | E::StaleGeneration => Self::StaleFence,
            E::Unavailable(_) => Self::Unavailable,
            E::Unsupported(_) => Self::Unsupported,
            E::OutcomeUnknown(_) => Self::OutcomeUnknown,
            E::Internal(_) => Self::Internal,
        }
    }
}
impl ApplicationResultClass {
    pub fn of<T>(result: &crate::service::ServiceResult<T>) -> Self {
        match result {
            Ok(_) => Self::ReturnedOk,
            Err(error) => Self::ReturnedError(error.into()),
        }
    }
}
impl OperationLabel {
    pub(crate) fn of_codex_hook(event: &crate::service::codex_hook::HookEvent) -> Self {
        Self { domain: OperationDomain::Participants, variant: event.activity.operation().into() }
    }

    /// Safe outer-domain fallback; callers may replace variant with a static native sub-operation.
    pub fn of(request: &crate::service::Request) -> Self {
        use crate::agents::launch_service::Request as L;
        use crate::agents::lifecycle_service::Request as A;
        use crate::agents::service::Request as P;
        use crate::management::contract::Request as G;
        use crate::messaging::service::{InboxOperation as I, Request as M};
        use crate::runtime::hosts::Request as H;
        use crate::service::Request as R;
        use crate::service::queries::Request as Q;
        let (domain, variant) = match request {
            R::Inspection(_) => (OperationDomain::Inspection, "Inspection"),
            R::Management(operation) => (
                OperationDomain::Management,
                match operation {
                    G::List { .. } => "ManagementList",
                    G::SelectLabels { .. } => "ManagementSelectLabels",
                    G::Agent(_) => "ManagementAgent",
                    G::AdoptedWorkspaces => "AdoptedWorkspaces",
                    G::OfficeActivity { .. } => "OfficeActivity",
                    G::HumanMailbox { .. } => "HumanMailbox",
                    G::Messages { .. } => "ManagementMessages",
                    G::LaunchRelations { .. } => "LaunchRelations",
                    G::HandleHumanMessages { .. } => "HandleHumanMessages",
                    G::Operation { .. } => "OperationReceipt",
                    _ => "Management",
                },
            ),
            R::Collaboration(_) => (OperationDomain::Collaboration, "Collaboration"),
            R::CodexHook(event) => return Self::of_codex_hook(event),
            R::Participants(operation) => (
                OperationDomain::Participants,
                match operation {
                    P::Resolve => "Resolve",
                    P::HookSession(_) => "HookSession",
                    P::ObserveHookPosition(_) => "ObserveHookPosition",
                    P::ObserveModelRequest(_) => "ObserveModelRequest",
                    P::ToolActivity(_) => "ToolActivity",
                    P::Status { .. } => "Status",
                    P::Lifecycle(A::Context(_)) => "ParticipantContext",
                    P::Lifecycle(A::ResolveResume { .. }) => "ResolveResume",
                    P::Lifecycle(A::PreviewResume { .. }) => "PreviewResume",
                    P::Lifecycle(A::ReserveResumeFork { .. }) => "ReserveResumeFork",
                    P::Lifecycle(A::ReserveResume { .. }) => "ReserveResume",
                    P::Launch(L::Reserve(_)) => "ReserveLaunch",
                    P::Launch(L::TerminalLaunched { .. }) => "TerminalLaunched",
                    P::Launch(L::CancelFailed(_)) => "CancelFailedLaunch",
                    P::Claude(_) => "ClaudeHook",
                    _ => "Participants",
                },
            ),
            R::Messages(operation) => (
                OperationDomain::Messages,
                match operation {
                    M::Submit(_) => "MessageSubmit",
                    M::Preview(_) => "MessagePreview",
                    M::Acceptance { .. } => "MessageAcceptance",
                    M::DeliveryReport { .. } => "DeliveryReport",
                    M::Delivery(crate::messaging::delivery_service::Request::Poll) => "DeliveryPoll",
                    M::Delivery(crate::messaging::delivery_service::Request::Heartbeat { .. }) => "DeliveryHeartbeat",
                    M::Delivery(crate::messaging::delivery_service::Request::ObserveGate { .. }) => "DeliveryObserveGate",
                    M::Delivery(crate::messaging::delivery_service::Request::Recovery { .. }) => "DeliveryRecover",
                    M::Inbox(I::PrepareOutput { .. }) => "PrepareOutput",
                    M::Inbox(I::ConfirmOutputAndStatus { .. }) => "ConfirmOutputAndStatus",
                    _ => "Messages",
                },
            ),
            R::Hosts(operation) => (
                OperationDomain::Hosts,
                match operation {
                    H::BootstrapSnapshot { .. } => "BootstrapSnapshot",
                    H::WakeHook { .. } => "WakeHook",
                    _ => "Hosts",
                },
            ),
            R::Events(_) => (OperationDomain::Events, "Events"),
            R::Queries(operation) => (
                OperationDomain::Queries,
                match operation {
                    Q::AgentNames(_) => "AgentNames",
                    Q::Listing { .. } => "Listing",
                    Q::ListingLabels { .. } => "ListingLabels",
                    Q::Snapshot(_) | Q::FilteredSnapshot { .. } => "DashboardSnapshot",
                    Q::StoppedHistory { .. } => "StoppedHistory",
                    Q::FileBindings(_) => "FileBindings",
                    Q::SearchTimeline { .. } => "SearchTimeline",
                    _ => "Queries",
                },
            ),
            R::Maintenance(_) => (OperationDomain::Maintenance, "Maintenance"),
            R::Relay(_) => (OperationDomain::Relay, "Relay"),
        };
        Self { domain, variant: variant.into() }
    }
    pub fn validate(&self) -> bool {
        !self.variant.is_empty() && self.variant.len() <= 64 && self.variant.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    }
}

/// Capped counting writer never allocates a serialized copy of oversized input.
pub(crate) fn encoded_size<T: Serialize>(value: &T, limit: usize) -> Option<usize> {
    struct Counter {
        bytes: usize,
        limit: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let next = self.bytes.checked_add(bytes.len()).filter(|n| *n <= self.limit).ok_or_else(|| std::io::Error::other("observation size limit"))?;
            self.bytes = next;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { bytes: 0, limit };
    serde_json::to_writer(&mut counter, value).ok()?;
    Some(counter.bytes)
}

fn valid_time(time: &ObservationTime) -> bool {
    time.clock.validate().is_ok()
}
fn valid_daemon(daemon: &DaemonRef) -> bool {
    daemon.validate().is_ok() && daemon.daemon_instance.len() <= 128
}
fn valid_epoch(epoch: &ObservationEpoch) -> bool {
    epoch.validate().is_ok() && valid_daemon(&epoch.daemon)
}
impl RuntimeEvent {
    pub fn validate(&self) -> bool {
        let c = &self.correlation;
        if !valid_time(&self.at)
            || !valid_daemon(&c.daemon)
            || c.epoch.as_ref().is_some_and(|e| !valid_epoch(e) || e.daemon != c.daemon)
            || c.connection.as_ref().is_some_and(|id| id.0 == 0)
            || c.request.as_ref().is_some_and(|id| id.0 == 0)
            || c.client_attempt.as_ref().is_some_and(|attempt| !attempt.validate())
            || c.request_id.as_ref().is_some_and(|id| id.0.is_empty() || id.0.len() > 1024)
        {
            return false;
        }
        match &self.fact {
            RuntimeFact::CallDecoded(op) => op.validate() && c.request.is_some() && c.connection.is_some(),
            RuntimeFact::Queued
            | RuntimeFact::AdmissionRejected(_)
            | RuntimeFact::DispatchStarted
            | RuntimeFact::DispatchRejected(_)
            | RuntimeFact::ApplicationReturned(_)
            | RuntimeFact::ReplyWriteFinished { .. }
            | RuntimeFact::ReplyWaitAbandoned(_) => c.request.is_some(),
            RuntimeFact::HostAssociationObserved { host, .. } => host.id.0 > 0 && host.daemon == c.daemon,
            RuntimeFact::ConnectionAccepted | RuntimeFact::HelloValidated | RuntimeFact::ConnectionClosed(_) => c.connection.is_some(),
            RuntimeFact::LifecycleChanged(_) => true,
        }
    }
}
impl ClientAttemptReport {
    /// Validation labels claims as claims; it never authenticates source-era identity.
    pub fn validate(&self) -> bool {
        if !self.attempt.validate()
            || self.invocation.as_ref().is_some_and(|r| !r.invocation.validate())
            || !valid_time(&self.started_at)
            || self.started_at.clock != self.ended_at.clock
            || self.ended_at.monotonic_ms.0 < self.started_at.monotonic_ms.0
            || self.stages.len() > 12
            || self.request_id.as_ref().is_some_and(|id| id.0.is_empty() || id.0.len() > 1024)
            || self.original_epoch.as_ref().is_some_and(|epoch| !valid_epoch(epoch))
            || encoded_size(self, 4096).is_none()
        {
            return false;
        }
        let mut previous = self.started_at.monotonic_ms.0;
        for span in &self.stages {
            if span.started_at.clock != self.started_at.clock
                || span.ended_at.clock != self.started_at.clock
                || span.started_at.monotonic_ms.0 < previous
                || span.ended_at.monotonic_ms.0 < span.started_at.monotonic_ms.0
                || span.ended_at.monotonic_ms.0 > self.ended_at.monotonic_ms.0
            {
                return false;
            }
            previous = span.ended_at.monotonic_ms.0;
        }
        if let EvidenceValue::Present(scope) = &self.resolved_scope
            && !scope.canonical_scope.is_absolute()
        {
            return false;
        }
        if self.failure.as_ref().is_some_and(|failure| {
            (failure.kind == LocalFailureKind::ClockUnavailable && (failure.at.is_some() || failure.os_code.is_some()))
                || failure.at.as_ref().is_some_and(|at| at.clock != self.started_at.clock || at.monotonic_ms.0 < self.started_at.monotonic_ms.0 || at.monotonic_ms.0 > self.ended_at.monotonic_ms.0)
        }) {
            return false;
        }
        match self.kind {
            ClientAttemptKind::Call => self.request_id.is_some() && !matches!(self.knowledge, ClientCallKnowledge::DiscoveryOnly),
            _ => matches!(self.knowledge, ClientCallKnowledge::DiscoveryOnly | ClientCallKnowledge::NotWritten),
        }
    }
}

/// Per-origin bounded completed reports. No background send, file spool or operation replay.
#[derive(Default)]
pub(crate) struct LocalAttemptReports {
    reports: std::collections::VecDeque<(ClientAttemptReport, usize)>,
    bytes: usize,
    dropped: u64,
}
impl LocalAttemptReports {
    pub fn push(&mut self, report: ClientAttemptReport) -> bool {
        let Some(bytes) = encoded_size(&report, 4096).filter(|_| report.validate()) else {
            self.dropped = self.dropped.saturating_add(1);
            return false;
        };
        while self.reports.len() >= 32 || self.bytes + bytes > 128 * 1024 {
            if let Some((_, old)) = self.reports.pop_front() {
                self.bytes -= old;
                self.dropped = self.dropped.saturating_add(1);
            }
        }
        self.bytes += bytes;
        self.reports.push_back((report, bytes));
        true
    }
    #[allow(dead_code, reason = "Explicit consumer transfer drains retained reports; there is no automatic retry or spool")]
    pub fn pop_oldest(&mut self) -> Option<ClientAttemptReport> {
        let (report, bytes) = self.reports.pop_front()?;
        self.bytes -= bytes;
        Some(report)
    }
    #[allow(dead_code, reason = "Origin consumers can expose bounded-report loss without claiming complete capture")]
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
    #[allow(dead_code, reason = "Bounded origin-report consumer inspection")]
    pub fn len(&self) -> usize {
        self.reports.len()
    }
    #[allow(dead_code, reason = "Bounded origin-report consumer inspection")]
    pub fn is_empty(&self) -> bool {
        self.reports.is_empty()
    }
}
