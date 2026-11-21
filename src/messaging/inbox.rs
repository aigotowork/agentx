//! Exclusive prepared-message claims, normalized delivery evidence and recovery.
//! Existing events/cursors remain the source; SQLite implements this capability.
//! Hooks, explicit listen/inline output and autonomous input share this boundary
//! for attached participants. Query errors must never mean empty or delivered.

use std::num::NonZeroUsize;

pub(crate) use super::inbox_store::{OutputClaim, OutputPreparation, SqlitePendingInbox};

#[cfg(test)]
#[path = "inbox/acceptance_tests.rs"]
mod acceptance_tests;

use crate::agents::attachments::{ActivationTarget, ParticipantRef};
use crate::messaging::activation::{AttemptId, TurnEvidence};

/// Local SQLite event id, ordered only inside this store.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, utoipa::ToSchema)]
pub(crate) struct EventId(pub i64);

/// Advisory observation; `through` is the last eligible message in this bounded
/// batch, not the store's unrelated global event high-water mark.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingSnapshot {
    pub after: EventId,
    pub through: EventId,
    pub count: NonZeroUsize,
}

/// Durable batch correlation. No provider-side idempotency is assumed.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) struct BatchId(pub String);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedBatch {
    pub id: BatchId,
    pub window: PendingSnapshot,
    pub formatted_context: String,
}

/// Exactly one autonomous medium per attachment; explicit tool output still
/// competes for the same exclusive claim. Identity itself does not own this choice.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum DeliveryMode {
    HookContext,
    NativeInput,
    DedicatedProduct,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct ProductBatch {
    pub batch: PreparedBatch,
    pub message_id: String,
    pub request_id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProductSend {
    pub participant: ParticipantRef,
    pub execution: crate::agents::attachments::ExecutionId,
    pub request_id: String,
}

impl ProductSend {
    pub(crate) fn key(generation: &str, event_id: i64) -> String {
        format!("product_send:{generation}:{event_id}")
    }

    pub(crate) fn execution(row: &crate::db::InstanceRow) -> anyhow::Result<Option<crate::agents::attachments::ExecutionId>> {
        let context = row.launch_context.as_deref().map(serde_json::from_str::<serde_json::Value>).transpose()?;
        Ok(context.and_then(|context| context.get("product_execution").cloned()).map(serde_json::from_value).transpose()?)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum InboxConsumer {
    Hook,
    NativeInput,
    ExplicitListen,
    ProductInput,
}

/// Boundary evidence, never proof of model action, reply or business completion.
/// Provider input is emitted only after the adapter checks exact thread, batch
/// correlation and frozen content. Raw provider ids remain inside that adapter.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum DeliveryEvidence {
    HookOutputWritten,
    ToolOutputWritten,
    ProviderInputRecorded { turn: TurnEvidence },
    ProductInputRecorded(ProductInput),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProductInput {
    pub request_id: String,
    pub message_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub payload_digest: String,
}

impl ProductInput {
    pub(crate) fn digest(content: &str) -> String {
        use sha2::{Digest, Sha256};
        use std::fmt::Write;
        let mut hex = String::with_capacity(71);
        hex.push_str("sha256:");
        for byte in Sha256::digest(content.as_bytes()) {
            write!(hex, "{byte:02x}").expect("formatting into a String");
        }
        hex
    }
}

/// Successful RPC/turn admission alone cannot construct honest delivery proof.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeliveryReceipt {
    pub target: ActivationTarget,
    pub batch: BatchId,
    pub through: EventId,
    pub evidence: DeliveryEvidence,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum InboxFailure {
    #[error("inbox store unavailable")]
    Unavailable,
    #[error("stale participant or attachment")]
    StaleParticipant,
    #[error("invalid delivery evidence")]
    InvalidEvidence,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PrepareFailure {
    #[error(transparent)]
    Store(InboxFailure),
    #[error("mail is claimed by another consumer")]
    ClaimedByAnotherConsumer,
    #[error("unresolved predecessor claim")]
    UnresolvedPredecessor,
    #[error("pending snapshot changed")]
    SnapshotChanged,
    #[error("no pending mail")]
    NoPendingMail,
    #[error("consumer does not own the selected delivery medium")]
    WrongDeliveryMode,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Confirmation {
    Advanced,
    AlreadyConfirmed,
    StaleFence,
    WrongBatch,
}

/// Evidence of no message-input effect, not absence of all provider effects.
/// A hook rejection can create/end a turn without recording the supplied input.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum NoInputEvidence {
    NeverIssued,
    ProviderRejectedBeforeInput { attempt: AttemptId },
    OldExecutionEndedAndHistoryComplete { snapshot_reference: String },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum ClaimPhase {
    Prepared,
    Issuing,
    Admitted,
    Uncertain,
    Blocked,
}

/// Includes unresolved predecessor attachments in the same mailbox lineage.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutstandingClaim {
    pub target: ActivationTarget,
    pub attempt: AttemptId,
    pub batch: PreparedBatch,
    pub consumer: InboxConsumer,
    pub phase: ClaimPhase,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReconciliationEvidence {
    PreviouslyRecorded(DeliveryReceipt),
    ProvenUnrecorded(NoInputEvidence),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum RecoveryOutcome {
    RecoveredDelivery,
    ReleasedForSuccessor,
}

/// Prepare/freeze/claim and confirm/cursor advance are distinct transactions.
/// All operations validate exact row/binding fences. Preparing also excludes
/// unresolved predecessor claims across reattachment or instance replacement.
/// Confirmation is forward-only and validates the exact batch event window.
/// A timeout, new owner or empty partial history is never release authority.
pub(crate) trait PendingInbox {
    fn snapshot(&self, participant: &ParticipantRef) -> Result<Option<PendingSnapshot>, InboxFailure>;
    fn outstanding(&self, current_target: &ActivationTarget) -> Result<Option<OutstandingClaim>, InboxFailure>;
    fn delivery_mode(&self, target: &ActivationTarget) -> Result<DeliveryMode, InboxFailure>;
    fn select_delivery_mode(&mut self, target: &ActivationTarget, mode: DeliveryMode) -> Result<(), InboxFailure>;
    fn prepare(&mut self, target: &ActivationTarget, attempt: &AttemptId, snapshot: &PendingSnapshot, consumer: InboxConsumer) -> Result<PreparedBatch, PrepareFailure>;
    fn mark_issuing(&mut self, target: &ActivationTarget, batch: &BatchId) -> Result<(), InboxFailure>;
    fn record_phase(&mut self, target: &ActivationTarget, batch: &BatchId, phase: ClaimPhase) -> Result<(), InboxFailure>;
    fn release_proven_unrecorded(&mut self, target: &ActivationTarget, batch: &BatchId, proof: NoInputEvidence) -> Result<(), InboxFailure>;
    fn confirm(&mut self, receipt: DeliveryReceipt) -> Result<Confirmation, InboxFailure>;
    /// Separate recovery authority validates history, mailbox lineage and both
    /// old/new fences; late normal receipts cannot perform this takeover.
    fn reconcile_predecessor(&mut self, predecessor: &BatchId, successor: &ActivationTarget, proof: ReconciliationEvidence) -> Result<RecoveryOutcome, InboxFailure>;
    /// Read-only committed cursor observation remains part of the released
    /// inbox contract; the current production bridge observes via snapshot.
    #[cfg_attr(not(test), allow(dead_code))]
    fn delivered_through(&self, participant: &ParticipantRef) -> Result<EventId, InboxFailure>;
}
