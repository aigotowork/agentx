//! Operator continuation for a failed model turn in a live managed Codex session.
use super::{attachments::ParticipantRef, labels::LabelPredicate};
use serde::{Deserialize, Serialize};

pub(crate) const PROMPT: &str = "Continue the current task.";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct Target {
    pub participant: ParticipantRef,
    pub failure: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct Selection {
    pub names: Vec<String>,
    pub workspace: Option<String>,
    pub labels: Vec<LabelPredicate>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Candidate {
    pub target: Target,
    pub reason: String,
    pub error_since: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) enum Request {
    Select(Selection),
    Submit { target: Target, selection: Selection },
    ObserveTurn { participant: ParticipantRef, session: String, turn: Option<String>, stopped: bool },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) enum Response {
    Candidates(Vec<Candidate>),
    Accepted(Attempt),
    Observed,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) enum Phase {
    Queued,
    Issuing,
    Submitted,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Attempt {
    pub id: String,
    pub phase: Phase,
    pub replayed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct Ticket {
    pub target: Target,
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) enum HostAction {
    Claim,
    Enter,
    Finish { submitted: bool },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) enum NativeAction {
    Poll,
    FailedTurnEnded { turn: String },
    Begin(Ticket),
    Finish { ticket: Ticket, recorded: bool },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) enum NativeOutcome {
    Pending { ticket: Option<Ticket>, model_error: bool },
    Changed(bool),
}
