//! Native Copilot delivery ownership. The extension's enqueue response is not
//! delivery evidence: only its actual, exactly correlated prompt confirms mail.
use super::attachments::ParticipantRef;
pub(crate) use super::copilot_store::validate_endpoint;
use crate::messaging::{inbox::EventId, service::PreparedOutput};
use serde::{Deserialize, Serialize};

/// An opaque offered batch identity. An event watermark alone has no authority.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct OfferToken {
    pub id: String,
    pub participant: ParticipantRef,
    pub session_id: String,
    pub through: EventId,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Offer {
    pub receipt: OfferToken,
    pub text: String,
    pub count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct PollResult {
    pub participant: ParticipantRef,
    pub offer: Option<Offer>,
    pub inflight: Option<EventId>,
    pub session_mismatch: bool,
}

/// A failed/unknown SDK call cannot release an issued managed output claim.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub(crate) enum SendObservation {
    Accepted,
    Failed,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub(crate) enum PendingHook {
    ToolResult,
    Stop,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Request {
    Poll { participant: ParticipantRef, session_id: String, idle: bool },
    Sent { receipt: OfferToken, observation: SendObservation },
    ObservePrompt { participant: ParticipantRef, session_id: Option<String>, prompt: String },
    Pending { participant: ParticipantRef, session_id: Option<String>, hook: PendingHook },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Response {
    Poll(PollResult),
    Sent,
    PromptObserved { matched: bool, ignored: bool },
    Pending(PreparedOutput),
}
