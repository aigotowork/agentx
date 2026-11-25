//! AgentX runtime-host capabilities. Provider protocols and process I/O stay in
//! their caller; only durable root/registration effects cross this boundary.
//! See `.docs/architecture/runtime-hosts.md` for ownership and receipt limits.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::agents::attachments::{ActivationTarget, AttachmentOwnerId, ExecutionId, ParticipantRef};
use crate::service::ServiceResult;

/// Configuration captured for one native root. It is not caller authority.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct NativeRootReservation {
    pub explicit_name: Option<String>,
    pub initial_cursor: Option<i64>,
    pub launch_args: Vec<String>,
    #[serde(default)]
    pub launch_profile: Option<crate::runtime::launch_profile::LaunchProvenance>,
    pub cwd: PathBuf,
    /// Only AgentX identity/configuration and the resolved Provider home are
    /// retained in launch history. Arbitrary credentials must not be persisted.
    pub environment: HashMap<String, String>,
    pub canonical_home: PathBuf,
    pub execution: ExecutionId,
    pub owner: AttachmentOwnerId,
}

/// Provider-observed state at successful qualified root attachment.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeRootState {
    Idle,
    Busy,
}

/// One registration is fenced by participant, attachment, controller, and a
/// fresh daemon-issued token. Trusted PID/incarnation are retained server-side.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct HostRegistration {
    pub token: String,
    pub target: ActivationTarget,
    pub owner: AttachmentOwnerId,
    /// Connect-and-close wake only; never a terminal injection endpoint.
    pub wake_port: u16,
}

/// Temporary caller-owned launch notification. The unique name records the
/// authenticated process and batch scope; release also compares the port.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct LaunchWaitRegistration {
    pub name: String,
    pub port: u16,
}

/// A caller-owned child process. The daemon independently observes its process
/// incarnation before retaining it; the host peer is authenticated separately.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChildProcess {
    pub pid: u32,
    pub incarnation: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProcessRegistration {
    pub token: String,
    pub participant: ParticipantRef,
    pub child: ChildProcess,
}

/// Last successful authenticated PTY snapshot, independent of Provider hooks.
/// Missing progress on legacy hosts is unknown, never a healthy connection.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct DeliveryHostProgress {
    pub pid: u32,
    pub incarnation: String,
    pub child: ChildProcess,
    pub last_poll: Option<i64>,
    pub process_alive: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClaudeModPoll {
    pub token: String,
    pub participant: ParticipantRef,
    pub session: String,
    pub process_binding: String,
    pub port: u16,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum ClaudeModReport {
    SubmissionFailed { detail: String },
    PollFailed { detail: String },
}

/// Injection is a request/response endpoint, distinct from wake listeners.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum EndpointKind {
    Wake(super::notify::WakeKind),
    Injection,
}

impl EndpointKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Wake(kind) => kind.as_str(),
            Self::Injection => "inject",
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum Request {
    DirectTerminalSupport,
    ReserveNativeRoot(NativeRootReservation),
    ReserveProductRoot {
        root: NativeRootReservation,
        role: String,
        workspace: std::path::PathBuf,
    },
    RegisterNativeRoot {
        target: ActivationTarget,
        owner: AttachmentOwnerId,
        wake_port: u16,
        state: NativeRootState,
    },
    RenewNativeRoot(HostRegistration),
    /// The originating terminal observed a ready prompt for this native host.
    NativeRootReady(HostRegistration),
    NativeRecovery {
        registration: HostRegistration,
        action: crate::agents::recovery::NativeAction,
    },
    ReleaseNativeRoot(HostRegistration),
    ObserveLaunch {
        launcher: Option<String>,
        batch_id: Option<String>,
        timeout_secs: u64,
    },
    RegisterLaunchWaiter {
        batch_prefix: Option<String>,
        port: u16,
    },
    ReleaseLaunchWaiter(LaunchWaitRegistration),
    RegisterProcess {
        participant: ParticipantRef,
        child: ChildProcess,
        launch_context: Option<String>,
    },
    CaptureProcessContext {
        registration: ProcessRegistration,
        launch_context: String,
    },
    SetProcessEndpoint {
        registration: ProcessRegistration,
        kind: EndpointKind,
        port: u16,
    },
    ReleaseProcessEndpoint {
        registration: ProcessRegistration,
        kind: EndpointKind,
        port: u16,
    },
    ObserveApproval {
        participant: ParticipantRef,
        approval: bool,
    },
    ObserveClaudeModHealth {
        registration: ProcessRegistration,
    },
    ReportClaudeMod {
        session: String,
        report: ClaudeModReport,
    },
    BeginClaudeModPoll {
        session: String,
        port: u16,
    },
    FinishClaudeModPoll {
        registration: ClaudeModPoll,
        claim: bool,
    },
    BootstrapSnapshot {
        name: String,
    },
    SetPluginEndpoint {
        participant: ParticipantRef,
        port: u16,
        standby: bool,
    },
    SetCopilotEndpoint {
        participant: ParticipantRef,
        session_id: String,
        port: u16,
    },
    WakeHook {
        participant: ParticipantRef,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[expect(clippy::large_enum_variant, reason = "One typed host reply carries a finite native snapshot by value; preserve the existing direct serde envelope.")]
pub(crate) enum Response {
    Reserved(ParticipantRef),
    Registered(HostRegistration),
    Renewed,
    Released,
    LaunchStatus {
        result: super::launch_status::LaunchResult,
        cleanup: crate::agents::service::CleanupOutcome,
        /// Frozen launch records for caller-only log/terminal diagnosis.
        diagnostics: Vec<crate::agents::service::ParticipantView>,
    },
    LaunchWaiter(LaunchWaitRegistration),
    ProcessRegistered {
        registration: ProcessRegistration,
        display_name: String,
    },
    ProcessStatus(String),
    NativeRecovery(crate::agents::recovery::NativeOutcome),
    BootstrapSnapshot(super::bootstrap::BootstrapSnapshot),
    ClaudeModPoll(Option<(ClaudeModPoll, bool)>),
    ClaudeModWake(bool),
}

/// Offered AgentX capability injected into a Provider connection. It never
/// starts/stops a Provider, writes terminal bytes, or commits delivery receipts.
/// IPC failure is returned to the host, with no local database fallback.
pub(crate) trait HostService {
    fn call(&self, request: Request) -> ServiceResult<Response>;
}
