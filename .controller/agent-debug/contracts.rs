//! DESIGN ONLY — not a crate module, generated output or implemented API.
//! Native host endpoint epoch 1 sketch; canonical semantics:
//! ../../.docs/architecture/agent-runtime-socket.md
//! Move into runtime/agent_endpoint/contracts.rs only under implementation scope.
//! Existing native identities below must be imported, never redefined in production.

use crate::agents::attachments::{ActivationTarget, ParticipantRef};
use crate::messaging::inbox::{BatchId, ClaimPhase, EventId};
use crate::observability::context::{DaemonRef, ObservationTime, ScopeRef};
use crate::service::RequestId;
use std::path::PathBuf;

// All wire values derive native Serialize/Deserialize during implementation.
// This sketch deliberately has no production mod declaration or Cargo changes.
pub struct EndpointId(pub String); // UUID v4; fresh per participant host lifetime
pub struct DatabaseGeneration(pub String); // use existing native generation representation
pub struct HostRevision(pub u64);
// A caller nonce correlates finite scheduling/read/reconnect results; it is not a durable idempotency key.
// Retained same-id + same canonical command/fences returns prior state; different arguments conflict.
// Nonterminal records remain reserved; terminal records expire after the advertised local duration.
// Missing lookup is NotRetained/Unknown, never proof of no effect; no automatic resend.
// Reuse after eviction has no dedup guarantee; this protocol MUST NOT grow arbitrary effects.
pub struct OperationId {
    pub endpoint: EndpointId,
    pub nonce: String,
}
pub struct SubscriptionId(pub String);
pub struct ProcessRef {
    pub pid: u32,
    pub incarnation: String, // kernel-observed; unsupported if trustworthy observation unavailable
}
pub struct EndpointRef {
    pub scope: ScopeRef,
    pub database_generation: DatabaseGeneration,
    pub participant: ParticipantRef,
    pub endpoint: EndpointId,
    pub host: ProcessRef,
}
pub enum LocalAddress {
    Unix(PathBuf),
    // Future Windows named-pipe adapter is not epoch-1 Linux support.
}
pub struct Manifest {
    pub manifest_version: u16,
    pub endpoint: EndpointRef,
    pub address: LocalAddress,
    pub host_epoch: u16,
    pub business_epoch: u32,
    pub build_identity: String,
}
pub enum ClaimedRole {
    Operator,
    Daemon(DaemonRef),
}
pub struct Hello {
    pub endpoint: EndpointRef,
    pub host_epoch: u16,
    pub role: ClaimedRole, // claim only; kernel peer + independent Ready validate daemon
}
pub enum Capability {
    Snapshot,
    Events,
    InboxHint,
    RescanInbox,
    ProbeProvider,
    ReconnectDaemon,
}
pub struct Limits {
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub max_inflight: u16,
    pub max_subscriptions: u16,
    pub max_page_records: u16,
    pub max_timeout_ms: u32,
    pub max_operation_records: u16,
    pub terminal_result_retention_ms: u32, // local duration since terminal transition; no client host-clock sample
}
pub struct Description {
    pub endpoint: EndpointRef,
    pub host_epoch: u16,
    pub business_epoch: u32,
    pub build_identity: String,
    pub capabilities: Vec<Capability>,
    pub limits: Limits,
    pub observed_at: ObservationTime,
}
pub enum Welcome {
    Ready(Description),
    UpgradeRequired { supported_host_epoch: u16 },
    Refused(EndpointError),
}
pub struct Call {
    pub request_id: RequestId,
    pub expected: EndpointRef,
    pub timeout_ms: u32, // validated/capped; converted once to local monotonic deadline
    pub request: Request,
}
pub enum Request {
    Describe,
    Ping,
    Snapshot,
    ReadEvents(EventRead),
    Subscribe(EventRead),
    CancelSubscription(SubscriptionId),
    InboxChanged { daemon: DaemonRef, hint_revision: u64 },
    Maintain { operation: OperationId, expected_revision: HostRevision, command: Maintenance },
    ObserveOperation(OperationId), // lookup never schedules; missing could mean expired OR never accepted
}
// One bounded maintenance worker; no Provider/daemon wait on diagnostic handlers.
// Rescan coalesces into the existing loop; native claim remains the input dedup owner.
// Probe is read-only and adapter-supported; timeout gives unknown observation, not new input.
// Reconnect joins the current connection coordinator, retains dataset and pending claim knowledge.
// An explicit fresh attempt rechecks endpoint/revision/binding; never blind effect retries.
pub enum Maintenance {
    RescanInbox,
    ProbeProvider { expected_target: Option<ActivationTarget> },
    ReconnectDaemon,
}
// None means registered process pre-session, NOT permission to skip a current binding.
// Adapter must recheck EndpointRef + current process/attachment before effects.
pub enum HintResult {
    Scheduled,
    AlreadyScheduled,
}
pub enum OperationState {
    Pending,
    Running,
    Completed(MaintenanceResult),
    FailedBeforeEffect(EndpointError),
    Unknown { stage: EffectStage },
    NotRetained, // never known OR result expired; neither proves no effect
}
pub enum MaintenanceResult {
    Scanned { revision: HostRevision, result: ScanResult },
    Probed { revision: HostRevision, state: Observed<ProviderState> },
    Reconnected { daemon: DaemonRef },
}
pub enum ScanResult {
    NoPending,
    Pending { through: EventId },
    Blocked(BlockReason),
}
pub enum EffectStage {
    Scheduled,
    Scanning,
    ProbingProvider,
    Reconnecting,
    Reply,
}
pub struct Reply {
    pub request_id: RequestId,
    pub endpoint: EndpointRef,
    pub result: Result<Response, EndpointError>,
}
pub enum Response {
    Description(Description),
    Pong { observed_at: ObservationTime },
    Snapshot(HostSnapshot),
    Events(EventReadResult),
    Subscribed { subscription: SubscriptionId, head: StreamHead },
    SubscriptionCanceled,
    Hint(HintResult),
    Operation { operation: OperationId, state: OperationState },
}
pub enum EndpointError {
    Forbidden,
    UnverifiedDaemon,
    WrongScope,
    DatasetChanged,
    StaleEndpoint,
    StaleParticipant,
    StaleBinding,
    StateChanged { current: HostRevision },
    Unsupported(Capability),
    InvalidInput,
    Oversize,
    Busy { retry_after_ms: u32 }, // before acceptance; does not mean operation failed later
    OperationConflict,
    DeadlineBeforeAcceptance,
    Unavailable { stage: LocalFailureStage, kind: FailureKind, os_code: Option<i32> },
}
pub enum LocalFailureStage {
    Connect,
    Hello,
    Generation,
    Call,
    Reply,
    Provider,
}
pub enum FailureKind {
    Timeout,
    Refused,
    Closed,
    Protocol,
    Io,
    Unknown,
}
// Client must separately retain knowledge of possible write/acceptance if Reply is lost.
pub enum CallKnowledge<T> {
    Replied(T),
    FailedBeforeWrite(EndpointError),
    OutcomeUnknown { request_id: RequestId, operation: Option<OperationId> },
}
pub enum Lifecycle {
    Binding,
    SocketReady,
    Registered,
    Closing,
}
pub enum Observed<T> {
    Known { value: T, observed_at: ObservationTime, source: EvidenceSource },
    NotObserved,
    Unsupported,
    Unavailable { stage: LocalFailureStage, kind: FailureKind },
}
pub enum EvidenceSource {
    Host,
    NativeDaemon,
    ProviderAdapter,
}
pub enum DaemonLink {
    Connecting,
    Connected { daemon: DaemonRef, business_epoch: u32 },
    Disconnected,
    ProtocolMismatch { host_epoch: u32, daemon_epoch: Option<u32> },
    DatasetChanged,
}
pub enum ProviderState {
    Starting,
    Idle,
    Busy,
    Approval,
    InputOwned,
    Stalled,
    Closed,
}
pub enum BlockReason {
    NoSession,
    ProviderBusy,
    Approval,
    InputOwned,
    DaemonUnavailable,
    ProtocolMismatch,
    DatasetChanged,
    ClaimUncertain,
    ReceiptPending,
    Unsupported,
}
pub enum ConsumptionKnowledge {
    NotConfirmed,
    NativeConfirmed { batch: BatchId, through: EventId },
    Unknown,
}
pub struct DeliveryView {
    pub known_pending_after: Option<EventId>,
    pub known_pending_through: Option<EventId>,
    pub outstanding: Option<(BatchId, ClaimPhase)>,
    pub blocked: Option<BlockReason>,
    pub consumed: ConsumptionKnowledge,
    pub last_native_poll: Observed<NativePoll>, // latest attempt, with prior-success time if known
    pub model_started: Observed<bool>, // may be unavailable; never inferred from consumption
}
pub struct NativePoll {
    pub last_success_at: Option<ObservationTime>,
    pub succeeded: bool,
    pub failure_stage: Option<LocalFailureStage>,
}
pub struct InboxHintObservation {
    pub hint_revision: u64, // advisory only; no mailbox ordering/receipt authority
}
pub struct HostSnapshot {
    pub endpoint: EndpointRef,
    pub revision: HostRevision,
    pub lifecycle: Lifecycle,
    pub target: Option<ActivationTarget>,
    pub daemon: Observed<DaemonLink>,
    pub provider: Observed<ProviderState>,
    pub delivery: Observed<DeliveryView>,
    pub last_hint: Observed<InboxHintObservation>,
    pub maintenance_queued: u16,
    pub active_subscriptions: u16,
    pub dropped_events: u64,
    pub observed_at: ObservationTime,
}
// Observation stream identity is host-owned; do not forge an ObservationEpoch/DaemonRef.
pub struct StreamId {
    pub endpoint: EndpointId,
    pub nonce: String,
}
pub struct Cursor {
    pub stream: StreamId,
    pub sequence: u64,
}
pub struct StreamHead {
    pub stream: StreamId,
    pub oldest: u64,
    pub newest: u64,
    pub lost: u64,
}
pub enum EventFilter {
    All,
    Delivery,
    Link,
    Maintenance,
    Lifecycle,
}
pub struct EventRead {
    pub after: Option<Cursor>, // None: begin at oldest retained event; empty stream returns sequence-zero cursor
    pub through: Option<u64>, // None freezes current head for this read window
    pub limit: u16,
    pub filter: EventFilter,
}
pub struct SafeEvent {
    pub cursor: Cursor,
    pub observed_at: ObservationTime,
    pub event: HostEvent,
}
pub enum HostEvent {
    StateChanged { revision: HostRevision },
    DeliveryChanged { batch: Option<BatchId>, phase: Option<ClaimPhase>, blocked: Option<BlockReason> },
    LinkChanged { link: DaemonLink },
    MaintenanceChanged { operation: OperationId, stage: EffectStage },
    OperationalLog { key: String, severity: Severity, failure: Option<FailureKind>, os_code: Option<i32> },
}
pub enum Severity {
    Info,
    Warning,
    Error,
}
// OperationalLog.key is a registered event key, never raw caller/provider text.
pub enum EventReadResult {
    Page { records: Vec<SafeEvent>, next: Cursor, through: u64, head: StreamHead },
    Gap { first_missing: u64, last_missing: u64, head: StreamHead },
    StreamChanged { head: StreamHead },
}
pub enum StreamFrame {
    Events { subscription: SubscriptionId, result: EventReadResult },
    Closed { subscription: SubscriptionId, reason: StreamClose },
}
pub enum StreamClose {
    Canceled,
    SlowConsumer,
    EndpointClosing,
    Deadline,
}
// Trusted context is built from transport, not deserialized from Call/Hello.
pub struct AuthenticatedCaller {
    pub process: ProcessRef,
    pub verified_daemon: Option<DaemonRef>,
}
pub trait HostEndpoint {
    fn describe(&self) -> Description;
    fn call(&self, actor: &AuthenticatedCaller, call: Call) -> Reply;
}
// Server handles I/O/subscription lifetime; trait calls must not block on Provider/SQLite.
// Actual delivery continues through PendingInbox/HostService, not a new socket receipt API.
