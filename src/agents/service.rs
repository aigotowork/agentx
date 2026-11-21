//! Participant use cases and native operations. Process/session configuration
//! comes from explicit invocation context; SQLite adapters enforce durable fences.

use super::attachments::*;
use crate::db::InstanceRow;

pub(crate) use super::service_remote::RemoteAttachments;
pub(crate) use super::service_store::{attachment_owned_by_process, handle, reserve_native_root, reserve_product_root, resolve_actor};
// Daemon composition only: callers validate their ticket or retained host exit
// registration in the same transaction; this is not another IPC operation.
pub(crate) use super::cleanup_store::{cleanup_system, tracked_process_state};
pub(crate) use super::hook_service::{HookSession, ObserveHookPosition, SessionStopOutcome, StopSession, ToolActivity};
pub(crate) use super::hook_store::stop_killed;
pub(crate) use super::hook_store::stop_operator;
pub(crate) use super::hook_store::resolve_codex_pty;

/// Daemon composition only. Each adapter chooses its existing finite cleanup
/// policy; none of these decisions is an independently callable stop RPC.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum SystemCleanup {
    Listing,
    DeadProcesses,
    LaunchPlaceholders,
    Resume { participant: ParticipantRef },
}

/// Durable row retirement and its still-external cleanup plan. A deferred row
/// is retained because a tracked descendant may outlive its original process.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub(crate) struct CleanupOutcome {
    pub retired: usize,
    pub deferred: Vec<ParticipantRef>,
    pub effects: Vec<super::hook_service::RetiredParticipant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrackedProcessState {
    Alive,
    Gone,
    Unverifiable,
}

/// Source-owned participant projection shared by read-only product consumers.
pub(crate) type ParticipantView = InstanceRow;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct ResolvedActor {
    pub participant: ParticipantRef,
    pub record: ParticipantView,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct Registration {
    pub name: Option<String>,
    pub tool: String,
    pub session_id: Option<String>,
    pub process_binding_id: Option<String>,
    pub transcript_path: Option<String>,
    pub wait_timeout: i64,
    pub subagent_timeout: Option<i64>,
    pub hints: Option<String>,
    pub auto_subscribe: String,
}

/// Provider plugin lifecycle input. Runtime owns endpoint registration and
/// bootstrap rendering; the participant domain owns canonical session binding.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct PluginStart {
    pub tool: String,
    pub session_id: String,
    pub transcript_path: Option<String>,
    pub cwd: Option<std::path::PathBuf>,
    pub recover_soft_stopped: bool,
}

/// Existing attachment port projected without changing identity/evidence types.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum AttachmentOperation {
    Participant { name: String },
    CanonicalParticipant { session: ProviderSessionKey },
    Attach(AttachmentRequest),
    Lookup { execution: ExecutionId, session: ProviderSessionKey },
    Current { participant: ParticipantRef },
    Owned { owner: AttachmentOwnerId },
    Validate { target: ActivationTarget },
    Retire { target: ActivationTarget },
    RetireOwner { owner: AttachmentOwnerId },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum AttachmentOutcome {
    Participant(Result<ParticipantRef, AttachmentFailure>),
    CanonicalParticipant(Result<Option<ParticipantRef>, AttachmentFailure>),
    Attached(Result<Attachment, AttachmentFailure>),
    Attachment(Result<Option<Attachment>, AttachmentFailure>),
    Attachments(Result<Vec<Attachment>, AttachmentFailure>),
    Valid(Result<bool, AttachmentFailure>),
    Done(Result<(), AttachmentFailure>),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum Request {
    Lifecycle(super::lifecycle_service::Request),
    Launch(super::launch_service::Request),
    Labels(super::labels::Request),
    Hermes(super::hermes_service::Request),
    Copilot(super::copilot_service::Request),
    Claude(super::claude_service::Request),
    PluginStart(PluginStart),
    HookSession(HookSession),
    ToolActivity(ToolActivity),
    ObserveHookPosition(ObserveHookPosition),
    ObserveModelRequest(super::model_request::ObserveModelRequest),
    Recovery(super::recovery::Request),
    StopSession(StopSession),
    /// Recover a launched Codex placeholder's reported session without plugin
    /// startup status/cursor effects. Resolve itself remains read-only.
    BindLaunchedSession {
        participant: ParticipantRef,
        session_id: String,
    },
    PluginStop {
        participant: ParticipantRef,
        reason: String,
        soft: bool,
    },
    Lookup {
        name: String,
    },
    Resolve,
    Record {
        name: String,
    },
    List,
    Register(Registration),
    Status {
        participant: ParticipantRef,
        status: String,
        context: String,
        detail: String,
    },
    Heartbeat {
        participant: ParticipantRef,
    },
    Stop {
        participant: ParticipantRef,
        reason: String,
    },
    Attachments(AttachmentOperation),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[expect(clippy::large_enum_variant, reason = "Each participant call returns one direct native domain result, including at most one Claude actor view")]
pub(crate) enum Response {
    Recovery(super::recovery::Response),
    Lifecycle(super::lifecycle_service::Response),
    Launch(super::launch_service::Response),
    Labels(super::labels::Response),
    Hermes(super::hermes_service::Response),
    Copilot(super::copilot_service::Response),
    Claude(super::claude_service::Response),
    Participant(Option<ParticipantRef>),
    Actor(ResolvedActor),
    Record(Option<ParticipantView>),
    Records(Vec<ParticipantView>),
    Registered(ResolvedActor),
    SessionStopped(SessionStopOutcome),
    Changed,
    Stopped(bool),
    Attachment(AttachmentOutcome),
}
