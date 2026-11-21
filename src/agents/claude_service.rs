//! Claude hook policy expressed as native participant observations. Provider
//! envelopes, bounded transcript reads, rendering and polling belong to hosts.
use super::attachments::ParticipantRef;
use super::hook_service::SessionStopOutcome;
use super::service::ParticipantView;
use crate::messaging::inbox::EventId;
use crate::messaging::service::{OutputConfirmation, OutputToken, PreparedOutput};
use serde::{Deserialize, Serialize};

/// Envelope facts only: never message content, summaries or a selected owner.
/// The host reads at most 512 KiB / 2048 records and ignores a truncated tail.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub(crate) enum TranscriptEvidence {
    #[default]
    NotRead,
    Missing,
    ReadFailed,
    Envelopes {
        top_level_sessions: Vec<String>,
        message_sessions: Vec<String>,
        bytes_read: usize,
        records_read: usize,
    },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub(crate) enum StartSource {
    Startup,
    Resume,
    Fork,
    Compact,
    Other,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum ObservationKind {
    SessionStart { source: StartSource },
    Callback,
    SubagentStart { agent_type: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Observation {
    pub kind: ObservationKind,
    pub session_id: String,
    pub env_session_candidate: Option<String>,
    pub agent_id: Option<String>,
    pub transcript_path: Option<String>,
    pub directory: Option<std::path::PathBuf>,
    pub evidence: TranscriptEvidence,
    /// Parsed terminal shell tool-use, revoked even when actor routing fails.
    pub terminal_shell_tool_use: Option<String>,
}

/// A correlation value, not authority by itself. Every operation revalidates
/// its exact root, primary session, worker binding and optional child relation.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Actor {
    pub root: ParticipantRef,
    pub participant: ParticipantRef,
    pub session_id: String,
    pub agent_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ActorView {
    pub actor: Actor,
    pub record: ParticipantView,
    pub root_record: ParticipantView,
    pub bootstrap: Option<BootstrapOffer>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[expect(clippy::large_enum_variant, reason = "A resolution carries at most two participant snapshots as one direct native observation result")]
pub(crate) enum Resolution {
    Unbound { session_id: String },
    NeedsLineage { session_id: String },
    Ambiguous { session_id: String },
    Historical { session_id: String, root: ParticipantRef },
    Ready(ActorView),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct BootstrapOffer {
    pub receipt: String,
    pub kind: super::lifecycle_service::Bootstrap,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Activity {
    ToolStarted { tool_name: String, detail: String, tool_use_id: Option<String>, visible_agentx_shell: bool },
    ToolSucceeded { tool_name: String, tool_use_id: Option<String> },
    ToolFailed { tool_name: String, detail: String, tool_use_id: Option<String> },
    PermissionDenied { tool_name: String, detail: String, tool_use_id: Option<String> },
    PermissionRequested { tool_name: String, detail: String, tool_use_id: Option<String> },
    StopFailed { error: String, detail: String },
    Prompt { bare_wake: bool },
    Idle,
    Approval,
    PollTimedOut,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub(crate) enum BackgroundTask {
    Shell,
    Monitor,
    Mcp,
    MayRunAgent,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct StopClaim {
    pub token: String,
    pub generation: String,
    pub root: ParticipantRef,
    pub session_id: String,
    pub agent_id: String,
    pub prompt_id: String,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[expect(clippy::large_enum_variant, reason = "An acquired stop carries one exact child snapshot and its claim in a direct native result")]
pub(crate) enum ChildStopResult {
    Duplicate,
    Missing,
    Retired(SessionStopOutcome),
    Acquired { claim: StopClaim, actor: Actor, record: ParticipantView, timeout_secs: u64, dormant: bool },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub(crate) enum FinishChildStop {
    Released,
    TimedOut,
    TaskCompleted,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Output {
    pub mailbox: PreparedOutput,
    pub bootstrap: Option<BootstrapOffer>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct TaskMessage {
    pub event: EventId,
    pub from: String,
    pub text: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ChildIdentity {
    pub participant: ParticipantRef,
    pub agent_id: Option<String>,
}

/// Informational history has its own receipt/frontier; it cannot acknowledge a
/// mailbox by passing a global watermark. Root mail still uses the canonical
/// frozen mailbox receipt and managed claim arbitration.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct TaskOutput {
    pub mailbox: PreparedOutput,
    pub messages: Vec<TaskMessage>,
    pub children: Vec<ChildIdentity>,
    pub history_receipt: Option<OutputToken>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Request {
    Observe(Observation),
    Activity {
        actor: Actor,
        activity: Activity,
    },
    PrepareOutput {
        actor: Actor,
        mailbox: bool,
        bootstrap: bool,
        stop_claim: Option<StopClaim>,
    },
    TaskCompleted {
        actor: Actor,
        async_launched: bool,
    },
    /// Sent only after the entire combined stdout is successfully flushed.
    ConfirmFlushed {
        actor: Actor,
        mailbox: Option<OutputConfirmation>,
        bootstrap: Option<String>,
        history: Option<OutputToken>,
    },
    BeginRootStop {
        actor: Actor,
        background_tasks: Option<Vec<BackgroundTask>>,
    },
    BeginChildStop {
        root: Actor,
        agent_id: String,
        prompt_id: String,
        fingerprint: String,
        transcript_path: Option<String>,
    },
    FinishChildStop {
        claim: StopClaim,
        decision: FinishChildStop,
    },
    /// Endpoint is the originating poller's listener, never daemon-created.
    PollEndpoint {
        actor: Actor,
        port: Option<u16>,
        stop_claim: Option<StopClaim>,
    },
    SessionEnd {
        root: Actor,
        reason: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[expect(clippy::large_enum_variant, reason = "Each hook operation returns one finite native result, including at most one resolved actor view")]
pub(crate) enum Response {
    Observed(Resolution),
    Activity { actor_capability: Option<String> },
    Output(Output),
    TaskOutput(Option<TaskOutput>),
    RootStop { timeout_secs: u64, pty: bool, retired_children: SessionStopOutcome },
    ChildStop(ChildStopResult),
    Stopped(SessionStopOutcome),
    Changed,
}
