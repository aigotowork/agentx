//! Volatile diagnosis records and their truthful, fixed-frontier continuation.
//! Business facts/receipts stay with their native owner. See
//! `.docs/observability/events.md` for correlation and evidence limits.

use serde::{Deserialize, Serialize};

use super::configuration::{ConfigObservationEvent, ConfigSnapshotRef};
use super::context::{DaemonRef, EvidenceOrigin, EvidenceStamp, ObservationEpoch, ObservationTime, ScopeRef};
use super::runtime::{AcceptedPeer, ClientAttemptId, ClientAttemptReport, RequestObservationId, RuntimeEvent};
use crate::agents::attachments::{ActivationTarget, ParticipantRef};
use crate::messaging::inbox::{BatchId, ClaimPhase, EventId};
use crate::service::RequestId;

pub(crate) use super::journal::ObservationJournal;

pub(crate) const MAX_RECORDS: usize = 8192;
pub(crate) const MAX_RETAINED_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_RECORD_BYTES: usize = 16 * 1024;
pub(crate) const MAX_AGE_MS: u64 = 900_000;
pub(crate) const MAX_PAGE_RECORDS: u16 = 256;
pub(crate) const MAX_PAGE_BYTES: usize = 512 * 1024;
pub(crate) const MAX_SCAN_RECORDS: usize = 1024;
pub(crate) const MAX_FILTER_BYTES: usize = 8 * 1024;
pub(crate) const MAX_POSTCOMMIT_RECORDS: usize = 64;
pub(crate) const MAX_POSTCOMMIT_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservationStreamId {
    pub epoch: ObservationEpoch,
    pub nonce: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservationCursor {
    pub stream: ObservationStreamId,
    pub sequence: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservationWindow {
    pub after: ObservationCursor,
    pub through: u64,
    pub filter: ObservationFilter,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservationRead {
    pub window: ObservationWindow,
    pub limit: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservationHead {
    pub stream: ObservationStreamId,
    pub oldest: u64,
    pub newest: u64,
    pub observed_at: ObservationTime,
    pub losses: CollectionLosses,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservationPage {
    pub records: Vec<ObservationRecord>,
    pub next: ObservationWindow,
    pub head: ObservationHead,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant, reason = "Finite native request values preserve the public value contract without another heap allocation")]
pub(crate) enum ObservationRequest {
    Describe,
    Read(ObservationRead),
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[allow(clippy::large_enum_variant, reason = "Bounded native response values preserve the existing reader and transport contract")]
pub(crate) enum ObservationResponse {
    Head(ObservationHead),
    Read(ObservationReadResult),
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[allow(clippy::large_enum_variant, reason = "Explicit bounded read outcomes retain their native inline value contract")]
pub(crate) enum ObservationReadResult {
    Page(ObservationPage),
    Gap { first_missing: u64, last_missing: u64, head: ObservationHead },
    EpochChanged { requested: ObservationEpoch, head: ObservationHead },
    StreamChanged { requested: ObservationStreamId, head: ObservationHead },
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ObservationReadError {
    InvalidInput(ReadInputError),
    Forbidden,
    GenerationUnavailable { daemon: DaemonRef },
    Busy,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ReadInputError {
    LimitOutOfRange,
    BoundaryOutOfRange,
    WrongScope,
    InvalidReference,
    FilterTooLarge,
}

pub(crate) trait ObservationReader: Send + Sync {
    fn describe(&self) -> Result<ObservationHead, ObservationReadError>;
    fn read(&self, input: ObservationRead, response_budget: usize) -> Result<ObservationReadResult, ObservationReadError>;
}

/// A handle is fixed to one stream. Rotation never redirects a late producer.
pub(crate) trait ObservationSink: Send + Sync {
    fn record(&self, input: ObservationInput) -> RecordDisposition;
    fn note_drop(&self, reason: DropReason, count: u64);
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum RecordDisposition {
    Recorded(ObservationCursor),
    Dropped(DropReason),
    Suppressed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum DropReason {
    Contended,
    Oversize,
    InvalidInput,
    RetiredEpoch,
    CollectorUnavailable,
    PreparationBudget,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservationInput {
    pub epoch: ObservationEpoch,
    pub source: EvidenceStamp,
    pub payload: ObservationPayload,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservationRecord {
    pub cursor: ObservationCursor,
    pub collected_at: ObservationTime,
    pub input: ObservationInput,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[allow(clippy::large_enum_variant, reason = "Closed producer payloads are byte-bounded at admission and keep the accepted native value contract")]
pub(crate) enum ObservationPayload {
    Runtime(RuntimeEvent),
    ClientReported(ClientReportObservation),
    Configuration(ConfigObservationEvent),
    DomainCommitted(DomainCommitObservation),
}

/// Current submitter is authenticated separately from the claimed historic origin.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ClientReportObservation {
    pub report: ClientAttemptReport,
    pub submitted_by: AcceptedPeer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ObservationFamily {
    Runtime,
    ClientReported,
    Configuration,
    DomainCommitted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ObservationSeverity {
    Info,
    Notice,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct DurableEventRef {
    pub scope: ScopeRef,
    pub database_generation: String,
    pub event: EventId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct MessageAcceptanceRef {
    pub scope: ScopeRef,
    pub database_generation: String,
    pub request_id: RequestId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct InboxClaimRef {
    pub scope: ScopeRef,
    pub database_generation: String,
    pub target: ActivationTarget,
    pub batch: BatchId,
}

/// Observation-only UUID. Never derive this from a native output receipt token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct OutputObservationId(String);

impl OutputObservationId {
    pub(crate) fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct OutputEvidenceRef {
    pub scope: ScopeRef,
    pub database_generation: String,
    pub participant: ParticipantRef,
    pub observation: OutputObservationId,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct DomainCorrelation {
    pub request: Option<RequestObservationId>,
    pub client_attempt: Option<ClientAttemptId>,
    pub request_id: Option<RequestId>,
    pub actor: Option<ParticipantRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct DomainCommitObservation {
    pub correlation: DomainCorrelation,
    pub fact: DomainCommitFact,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum DomainCommitFact {
    EventAppended { event: DurableEventRef, kind: DomainEventKind },
    MessageAccepted { acceptance: MessageAcceptanceRef, event: DurableEventRef, replayed: bool },
    InboxPhaseStored { claim: InboxClaimRef, phase: ClaimPhase },
    DeliveryConfirmationStored { claim: InboxClaimRef, boundary: DeliveryBoundary, confirmation: ConfirmationDisposition },
    OutputConfirmationStored { output: OutputEvidenceRef, through: EventId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum DomainEventKind {
    Message,
    Status,
    Life,
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum DeliveryBoundary {
    HookOutputWritten,
    ToolOutputWritten,
    ProviderInputRecorded,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum ConfirmationDisposition {
    Advanced,
    AlreadyConfirmed,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservationFilter {
    pub family: Option<ObservationFamily>,
    pub minimum_severity: Option<ObservationSeverity>,
    pub origin: Option<EvidenceOrigin>,
    pub request: Option<RequestObservationId>,
    pub client_attempt: Option<ClientAttemptId>,
    pub request_id: Option<RequestId>,
    pub participant: Option<ParticipantRef>,
    pub configuration: Option<ConfigSnapshotRef>,
    pub domain_event: Option<DurableEventRef>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct DroppedCounts {
    pub contended: u64,
    pub oversize: u64,
    pub invalid_input: u64,
    pub retired_epoch: u64,
    pub collector_unavailable: u64,
    pub preparation_budget: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct EvictedCounts {
    pub age: u64,
    pub count_pressure: u64,
    pub byte_pressure: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct CollectionLosses {
    pub sampled_at: ObservationTime,
    pub dropped: DroppedCounts,
    pub evicted: EvictedCounts,
    pub suppressed: u64,
}

impl ObservationInput {
    /// Validate cross-owner fences only; payload owners retain their own rules.
    pub(crate) fn validate(&self) -> bool {
        if self.epoch.validate().is_err() || self.source.at.clock.validate().is_err() {
            return false;
        }
        match &self.payload {
            ObservationPayload::Runtime(event) => {
                event.validate() && event.correlation.daemon == self.epoch.daemon && event.correlation.epoch.as_ref() == Some(&self.epoch) && self.source.origin == EvidenceOrigin::DaemonBoundary && self.source.at == event.at
            }
            ObservationPayload::ClientReported(value) => {
                value.report.validate()
                    && value.submitted_by.connection.daemon == self.epoch.daemon
                    && value.submitted_by.connection.id.0 > 0
                    && value.submitted_by.process.pid > 0
                    && value.submitted_by.observed_at.clock.validate().is_ok()
                    && value.submitted_by.process.incarnation.as_ref().is_none_or(|v| valid_key(v))
                    && self.source.origin == EvidenceOrigin::ClientReported
                    && self.source.at == value.report.ended_at
            }
            ObservationPayload::Configuration(event) => match event {
                ConfigObservationEvent::SnapshotPublished { snapshot, origin, captured_at, invocation, attempt, .. } => {
                    snapshot.epoch == self.epoch && snapshot.sequence > 0 && *origin == self.source.origin && *captured_at == self.source.at && invocation.as_ref().is_none_or(valid_invocation) && attempt.as_ref().is_none_or(ClientAttemptId::validate)
                }
                ConfigObservationEvent::CaptureSkipped { invocation, attempt, .. } => {
                    invocation.as_ref().is_none_or(valid_invocation) && attempt.as_ref().is_none_or(ClientAttemptId::validate) && matches!(self.source.origin, EvidenceOrigin::DaemonBoundary | EvidenceOrigin::ApplicationResolved)
                }
                ConfigObservationEvent::SnapshotsRetired { through_sequence, count, .. } => {
                    *through_sequence > 0 && *count > 0 && u64::from(*count) <= *through_sequence && matches!(self.source.origin, EvidenceOrigin::DaemonBoundary | EvidenceOrigin::ApplicationResolved)
                }
            },
            ObservationPayload::DomainCommitted(value) => self.source.origin == EvidenceOrigin::StoredDomainFact && value.validate(&self.epoch),
        }
    }

    pub(crate) fn family(&self) -> ObservationFamily {
        match &self.payload {
            ObservationPayload::Runtime(_) => ObservationFamily::Runtime,
            ObservationPayload::ClientReported(_) => ObservationFamily::ClientReported,
            ObservationPayload::Configuration(_) => ObservationFamily::Configuration,
            ObservationPayload::DomainCommitted(_) => ObservationFamily::DomainCommitted,
        }
    }

    pub(crate) fn severity(&self) -> ObservationSeverity {
        use super::runtime::{
            ApplicationResultClass as ResultClass, ClientCallKnowledge as Knowledge, ConnectionCloseReason as Close, DaemonLifecycle, HostAssociationState as Host, RuntimeFact as Fact, RuntimeFailureCode as Code, ServiceFailureClass as Service,
        };
        use ObservationSeverity::*;
        let result_severity = |class: &ResultClass| match class {
            ResultClass::ReturnedError(Service::Internal) => Error,
            ResultClass::ReturnedError(_) => Warning,
            ResultClass::ReturnedOk => Info,
        };
        let reason_severity = |code: &Code| match code {
            Code::Internal | Code::ScopeOwnershipLost => Error,
            _ => Warning,
        };
        match &self.payload {
            ObservationPayload::Runtime(value) => match &value.fact {
                Fact::LifecycleChanged(DaemonLifecycle::Failed) => Error,
                Fact::AdmissionRejected(code) | Fact::DispatchRejected(code) | Fact::ReplyWaitAbandoned(code) | Fact::ReplyWriteFinished { failure: Some(code) } => reason_severity(code),
                Fact::ApplicationReturned(class) => result_severity(class),
                Fact::ConnectionClosed(Close::ReadFailure | Close::WriteFailure | Close::Deadline | Close::Rejected) => Warning,
                Fact::LifecycleChanged(DaemonLifecycle::Draining | DaemonLifecycle::Stopping) | Fact::HostAssociationObserved { state: Host::Released | Host::InvalidatedByReset, .. } => Notice,
                _ => Info,
            },
            ObservationPayload::ClientReported(value) => {
                let result = match &value.report.knowledge {
                    Knowledge::ReceivedApplicationResult(class) => result_severity(class),
                    Knowledge::ReceivedFailure(code) => reason_severity(code),
                    Knowledge::MayHaveBeenWritten => Warning,
                    _ => Info,
                };
                if value.report.failure.is_some() { result.max(Warning) } else { result }
            }
            ObservationPayload::Configuration(ConfigObservationEvent::CaptureSkipped { .. }) => Warning,
            ObservationPayload::Configuration(ConfigObservationEvent::SnapshotsRetired { .. }) => Notice,
            _ => Info,
        }
    }
}

fn valid_key(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024 && !value.contains('\0')
}
fn valid_participant(value: &ParticipantRef) -> bool {
    valid_key(&value.canonical_name)
}
fn valid_invocation(value: &super::configuration::InvocationCaptureRef) -> bool {
    value.invocation.validate()
}

impl DomainCommitObservation {
    fn validate(&self, epoch: &ObservationEpoch) -> bool {
        let correlation = &self.correlation;
        if correlation.request.as_ref().is_some_and(|v| v.0 == 0)
            || correlation.client_attempt.as_ref().is_some_and(|v| !v.validate())
            || correlation.request_id.as_ref().is_some_and(|v| !valid_key(&v.0))
            || correlation.actor.as_ref().is_some_and(|v| !valid_participant(v))
        {
            return false;
        }
        let same_store = |scope: &ScopeRef, generation: &str| scope == &epoch.daemon.scope && generation == epoch.database_generation;
        let event_ok = |event: &DurableEventRef| event.event.0 > 0 && same_store(&event.scope, &event.database_generation);
        let claim_ok =
            |claim: &InboxClaimRef| same_store(&claim.scope, &claim.database_generation) && valid_participant(&claim.target.participant) && valid_key(&claim.target.execution.0) && valid_key(&claim.target.binding.0) && valid_key(&claim.batch.0);
        match &self.fact {
            DomainCommitFact::EventAppended { event, .. } => event_ok(event),
            DomainCommitFact::MessageAccepted { acceptance, event, .. } => {
                event_ok(event) && same_store(&acceptance.scope, &acceptance.database_generation) && valid_key(&acceptance.request_id.0) && correlation.request_id.as_ref().is_none_or(|id| id == &acceptance.request_id)
            }
            DomainCommitFact::InboxPhaseStored { claim, .. } | DomainCommitFact::DeliveryConfirmationStored { claim, .. } => claim_ok(claim),
            DomainCommitFact::OutputConfirmationStored { output, through } => {
                same_store(&output.scope, &output.database_generation)
                    && valid_participant(&output.participant)
                    && through.0 >= 0
                    && uuid::Uuid::parse_str(&output.observation.0).is_ok_and(|v| v.get_version_num() == 4 && v.get_variant() == uuid::Variant::RFC4122)
            }
        }
    }
}

impl ObservationFilter {
    pub(crate) fn validate(&self) -> Result<(), ReadInputError> {
        if self.request.as_ref().is_some_and(|v| v.0 == 0)
            || self.client_attempt.as_ref().is_some_and(|v| !v.validate())
            || self.request_id.as_ref().is_some_and(|v| !valid_key(&v.0))
            || self.participant.as_ref().is_some_and(|v| !valid_participant(v))
            || self.configuration.as_ref().is_some_and(|v| v.sequence == 0 || v.epoch.validate().is_err())
            || self.domain_event.as_ref().is_some_and(|v| v.event.0 <= 0 || v.scope.validate().is_err() || uuid::Uuid::parse_str(&v.database_generation).is_err())
        {
            return Err(ReadInputError::InvalidReference);
        }
        if super::journal::encode_bounded(self, MAX_FILTER_BYTES).is_none() {
            return Err(ReadInputError::FilterTooLarge);
        }
        Ok(())
    }

    pub(super) fn matches(&self, input: &ObservationInput) -> bool {
        if self.family.is_some_and(|v| v != input.family()) || self.minimum_severity.is_some_and(|v| input.severity() < v) || self.origin.is_some_and(|v| v != input.source.origin) {
            return false;
        }
        let (request, attempt, request_id) = match &input.payload {
            ObservationPayload::Runtime(v) => (v.correlation.request.as_ref(), v.correlation.client_attempt.as_ref(), v.correlation.request_id.as_ref()),
            ObservationPayload::DomainCommitted(v) => (v.correlation.request.as_ref(), v.correlation.client_attempt.as_ref(), v.correlation.request_id.as_ref()),
            ObservationPayload::ClientReported(v) => (None, Some(&v.report.attempt), v.report.request_id.as_ref()),
            ObservationPayload::Configuration(ConfigObservationEvent::SnapshotPublished { attempt, .. } | ConfigObservationEvent::CaptureSkipped { attempt, .. }) => (None, attempt.as_ref(), None),
            _ => (None, None, None),
        };
        if self.request.as_ref().is_some_and(|v| Some(v) != request) || self.client_attempt.as_ref().is_some_and(|v| Some(v) != attempt) || self.request_id.as_ref().is_some_and(|v| Some(v) != request_id) {
            return false;
        }
        if let Some(expected) = &self.configuration
            && !matches!(&input.payload, ObservationPayload::Configuration(ConfigObservationEvent::SnapshotPublished { snapshot, .. }) if snapshot == expected)
        {
            return false;
        }
        if let Some(expected) = &self.domain_event
            && !matches!(&input.payload, ObservationPayload::DomainCommitted(DomainCommitObservation {
                fact: DomainCommitFact::EventAppended { event, .. } | DomainCommitFact::MessageAccepted { event, .. }, ..
            }) if event == expected)
        {
            return false;
        }
        if let Some(expected) = &self.participant {
            let ObservationPayload::DomainCommitted(value) = &input.payload else {
                return false;
            };
            let in_fact = match &value.fact {
                DomainCommitFact::InboxPhaseStored { claim, .. } | DomainCommitFact::DeliveryConfirmationStored { claim, .. } => Some(&claim.target.participant),
                DomainCommitFact::OutputConfirmationStored { output, .. } => Some(&output.participant),
                _ => None,
            };
            if value.correlation.actor.as_ref() != Some(expected) && in_fact != Some(expected) {
                return false;
            }
        }
        true
    }
}
