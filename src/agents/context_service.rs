//! Durable prompt lifecycle: frozen instructions and session-scoped initial output.
//! Host generators own content; this port only retains bounded results and receipts.
use super::attachments::ParticipantRef;
use crate::runtime::agent_context::FrozenInstructions;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) enum Request {
    ReadInstructions { participant: ParticipantRef },
    FreezeInstructions { participant: ParticipantRef, instructions: FrozenInstructions },
    PrepareInitial { participant: ParticipantRef, session_id: String },
    ConfirmInitial { participant: ParticipantRef, receipt: String },
    ReleaseInitial { participant: ParticipantRef, receipt: String },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) enum Response {
    Instructions(Option<FrozenInstructions>),
    InitialPrepared { receipt: Option<String> },
    Confirmed,
}
