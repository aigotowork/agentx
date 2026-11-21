//! Native session observations. Provider payloads/rendering stay in the host;
//! durable session policy returns only fenced, finite external stop effects.
use super::attachments::ParticipantRef;
use super::service::ParticipantView;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct HookSession {
    pub tool: String,
    pub session_id: String,
    pub transcript_path: Option<String>,
    pub directory: Option<std::path::PathBuf>,
    pub create_if_unbound: bool,
}

/// Parsed tool activity, without a Provider payload or row mutation map. Patch
/// paths retain their source order; the first path is the row's single detail.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct ToolActivity {
    pub participant: ParticipantRef,
    pub tool_name: String,
    pub detail: String,
    pub written_files: Vec<String>,
}

/// Position metadata for an already-resolved hook actor. None preserves a
/// value; this observation never creates or rebinds session/process ownership.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct ObserveHookPosition {
    pub participant: ParticipantRef,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub directory: Option<std::path::PathBuf>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct StopSession {
    pub participant: ParticipantRef,
    pub session_id: Option<String>,
    pub reason: String,
    pub preserve_live_pty: bool,
    pub soft: bool,
}

/// Internal lifecycle admission for explicit local operator stop. A tracked
/// process kill remains the separate ticket/Issuing/CompleteKill contract.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum OperatorStop {
    Participation { reason: String },
    KilledWithoutProcess,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub(crate) enum StopProcessAction {
    TerminateGroup,
    TrackSurvivor,
    KeepRunning,
}

/// The record and endpoints were captured before exact row deletion. Signal or
/// tracking I/O is authorized only for its independently matched OS incarnation.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct RetiredParticipant {
    pub participant: ParticipantRef,
    pub record: ParticipantView,
    pub process_identity: Option<String>,
    pub scope: std::path::PathBuf,
    pub process_bindings: Vec<String>,
    pub endpoints: Vec<(String, u16)>,
    pub process_action: StopProcessAction,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub(crate) struct SessionStopOutcome {
    /// Durable session/row retirement only; host cleanup is still external.
    pub stopped: bool,
    pub retained: bool,
    pub effects: Vec<RetiredParticipant>,
}
