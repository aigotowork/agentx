//! Provider-neutral participant attachments and stale-callback fences.
//! The identity store composes the existing canonical/placeholder rules and
//! SQLite transactions. Codex home/thread mapping belongs to its integration.

pub(crate) use super::attachments_store::SqliteAttachmentStore;

/// Canonical name plus exact existing `InstanceRow.created_at.to_bits()`.
/// Names, OS PIDs and provider session ids alone are not row identity.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) struct ParticipantRef {
    pub canonical_name: String,
    pub incarnation_bits: u64,
}

/// Runtime generation. Shared hosting uses the backend's stable generation;
/// other providers retain their existing launcher execution scope.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) struct ExecutionId(pub String);

/// Fresh control-owner token per client proxy, distinct from backend generation.
/// Sharing a backend grants no authority over a sibling's live attachment.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct AttachmentOwnerId(pub String);

/// Observed long-lived controller process, supplied by trusted daemon context.
/// A transient command proxy must use validated runtime-host registration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AttachmentOwnerProcess {
    pub pid: u32,
    pub process_identity: Option<String>,
}

/// Fresh attachment token on every detach/reattach or control-owner replacement.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) struct BindingId(pub String);

/// Checked both when scheduling and immediately before a provider effect.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) struct ActivationTarget {
    pub participant: ParticipantRef,
    pub execution: ExecutionId,
    pub binding: BindingId,
}

/// Provider-defined scope and session identity. The store treats both as opaque;
/// Codex supplies canonical home and native thread id without importing its DTOs.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ProviderSessionKey {
    pub provider: String,
    pub scope: String,
    pub session_id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Attachment {
    pub target: ActivationTarget,
    pub owner: AttachmentOwnerId,
    pub session: ProviderSessionKey,
}

/// Qualification supplied only after a correlated root lifecycle result.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum AttachmentIntent {
    Create { provisional: ParticipantRef },
    Resume { provisional: Option<ParticipantRef> },
    Fork { parent: ProviderSessionKey, provisional: ParticipantRef },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct AttachmentRequest {
    pub execution: ExecutionId,
    pub owner: AttachmentOwnerId,
    pub session: ProviderSessionKey,
    pub intent: AttachmentIntent,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum AttachmentFailure {
    #[error("stale runtime generation")]
    StaleExecution,
    #[error("stale participant incarnation")]
    StaleParticipant,
    #[error("session is owned by a live controller")]
    OwnedByLiveController,
    #[error("unqualified provider session")]
    UnqualifiedSession,
    #[error("identity store unavailable")]
    StoreUnavailable,
}

/// Owns canonical identity lookup, restoration and durable attachment.
/// `attach` atomically validates participant, ownership and session lineage;
/// a participant has at most one current attachment. Resume preserves logical
/// identity. Neither a heartbeat timeout nor a sibling on the same execution
/// proves a live owner dead. Known-session lookup never falls back to a process.
pub(crate) trait AttachmentStore {
    fn participant(&self, name: &str) -> Result<ParticipantRef, AttachmentFailure>;
    /// Persistent logical identity before resume, independent of live execution.
    fn canonical_participant(&self, session: &ProviderSessionKey) -> Result<Option<ParticipantRef>, AttachmentFailure>;
    fn attach(&mut self, request: AttachmentRequest) -> Result<Attachment, AttachmentFailure>;
    fn lookup(&self, execution: &ExecutionId, session: &ProviderSessionKey) -> Result<Option<Attachment>, AttachmentFailure>;
    fn current_for_participant(&self, participant: &ParticipantRef) -> Result<Option<Attachment>, AttachmentFailure>;
    fn owned(&self, owner: &AttachmentOwnerId) -> Result<Vec<Attachment>, AttachmentFailure>;
    fn validate(&self, target: &ActivationTarget) -> Result<bool, AttachmentFailure>;
    /// Retiring an old fence cannot revoke a successor. Claims survive retirement.
    fn retire(&mut self, target: &ActivationTarget) -> Result<(), AttachmentFailure>;
    /// Atomically retire only this controller's attachments, preserving claims.
    fn retire_owner(&mut self, owner: &AttachmentOwnerId) -> Result<(), AttachmentFailure>;
}
