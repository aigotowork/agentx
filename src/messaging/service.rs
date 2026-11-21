//! Durable acceptance and mailbox capabilities. Accepted is distinct from
//! Issuing, external delivery, exact receipt and completed agent work.

use super::activation::AttemptId;
use super::inbox::*;
use super::routing::MessageEnvelope;
use crate::agents::attachments::{ActivationTarget, ParticipantRef};
use crate::db::Message;

pub(crate) use super::service_remote::RemoteInbox;
pub(crate) use super::service_store::{handle, prepare_hook_output};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct Submission {
    pub recipients: Vec<String>,
    pub text: String,
    pub envelope: MessageEnvelope,
    pub external_sender: Option<String>,
    pub broadcast_confirmed: bool,
    pub inline_bundle: Option<serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct Preview {
    pub recipients: Vec<String>,
    pub broadcast: bool,
    pub thread_resolved: bool,
    pub normalized_envelope: MessageEnvelope,
}

/// Calls preserve the existing PendingInbox port's result types and fences.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum InboxOperation {
    Snapshot { participant: ParticipantRef },
    HasPendingAfter { participant: ParticipantRef, after: EventId },
    Outstanding { target: ActivationTarget },
    DeliveryMode { target: ActivationTarget },
    SelectDeliveryMode { target: ActivationTarget, mode: DeliveryMode },
    Prepare { target: ActivationTarget, attempt: AttemptId, snapshot: PendingSnapshot, consumer: InboxConsumer },
    PrepareProduct { target: ActivationTarget, attempt: AttemptId },
    MarkIssuing { target: ActivationTarget, batch: BatchId },
    RecordPhase { target: ActivationTarget, batch: BatchId, phase: ClaimPhase },
    ReleaseProvenUnrecorded { target: ActivationTarget, batch: BatchId, proof: NoInputEvidence },
    Confirm(DeliveryReceipt),
    ReconcilePredecessor { predecessor: BatchId, successor: ActivationTarget, proof: ReconciliationEvidence },
    DeliveredThrough { participant: ParticipantRef },
    PrepareOutput { participant: ParticipantRef, consumer: InboxConsumer },
    ConfirmOutput { receipt: OutputToken, announce: bool },
    ConfirmOutputAndStatus { receipt: OutputConfirmation, announce: bool },
}

/// Exact frozen output correlation for legacy mailboxes. A cursor alone cannot
/// prove which output was successfully flushed; the store retains this token.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutputToken {
    pub id: String,
    pub participant: ParticipantRef,
    pub through: EventId,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum OutputConfirmation {
    Unmanaged(OutputToken),
    Claimed(DeliveryReceipt),
}

/// An output claim is already Issuing before it leaves the daemon. The host
/// confirms only after exact output write+flush; loss keeps the claim uncertain.
/// Remaining/backlog_end describe eligible messages in the same preparation
/// snapshot; neither field extends the receipt's bounded delivery frontier.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum PreparedOutput {
    Unmanaged { messages: Vec<Message>, receipt: OutputToken, remaining: usize, backlog_end: Option<EventId>, formatted_context: String },
    Deferred,
    Claimed { target: ActivationTarget, batch: PreparedBatch, messages: Vec<Message>, remaining: usize, backlog_end: Option<EventId> },
}

/// Only the bytes the callback can render and its existing confirmation.
/// Bulk readers use PreparedOutput when they need exact pagination metadata.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct HookOutput {
    pub formatted_context: String,
    pub confirmation: OutputConfirmation,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum InboxOutcome {
    Snapshot(Result<Option<PendingSnapshot>, InboxFailure>),
    HasPendingAfter(Result<bool, InboxFailure>),
    Outstanding(Result<Option<OutstandingClaim>, InboxFailure>),
    DeliveryMode(Result<DeliveryMode, InboxFailure>),
    Prepared(Result<PreparedBatch, PrepareFailure>),
    ProductPrepared(Result<Option<super::inbox::ProductBatch>, PrepareFailure>),
    Confirmed(Result<Confirmation, InboxFailure>),
    Recovered(Result<RecoveryOutcome, InboxFailure>),
    DeliveredThrough(Result<EventId, InboxFailure>),
    Output(Result<PreparedOutput, InboxFailure>),
    Done(Result<(), InboxFailure>),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum Request {
    Bundle(super::bundle_service::Request),
    Delivery(super::delivery_service::Request),
    Submit(Submission),
    Preview(Submission),
    Acceptance { request_id: crate::service::RequestId },
    DeliveryReport { event: EventId },
    Inbox(InboxOperation),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct Acceptance {
    pub event: EventId,
    pub recipients: Vec<String>,
    pub replayed: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[expect(clippy::large_enum_variant, reason = "Each messaging call returns one direct native domain result, including at most one delivery snapshot")]
pub(crate) enum Response {
    Bundle(super::bundle_service::Response),
    Delivery(super::delivery_service::Response),
    Accepted(Acceptance),
    Preview(Preview),
    Acceptance(Option<Acceptance>),
    DeliveryReport(super::delivery_report::DeliveryReport),
    Inbox(InboxOutcome),
}
