//! Native participant lifecycle intentions; no CLI syntax or mutable record maps.
use super::attachments::ParticipantRef;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum StartMode {
    Participate,
    Reclaim { name: String },
    RecoverOrphan { target: String },
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum Bootstrap {
    Root { tool: String },
    Subagent { parent: String },
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub(crate) struct StartOutput {
    pub participant: Option<ParticipantRef>,
    pub stdout: String,
    pub warnings: Vec<String>,
    pub exit_code: i32,
    pub terminal_title: Option<String>,
    pub bootstrap_receipt: Option<String>,
    pub bootstrap: Option<Bootstrap>,
    pub activation_target: Option<super::attachments::ActivationTarget>,
}

/// Authority for one operator kill. A namesake, reused PID or another caller
/// never inherits this ticket, even after the live row has been retired.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct KillTicket {
    pub token: String,
    pub generation: String,
    pub participant: ParticipantRef,
    pub pid: u32,
    pub process_identity: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct KillPlan {
    pub ticket: KillTicket,
    pub record: super::service::ParticipantView,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct ResumeSnapshot {
    pub tool: String,
    pub session_id: String,
    pub launch_args: String,
    #[serde(default)]
    pub launch_profile: Option<crate::runtime::launch_profile::LaunchProvenance>,
    #[serde(default)]
    pub collaboration: Option<super::workspaces::LaunchCollaboration>,
    #[serde(default)]
    pub title: Option<String>,
    pub background: bool,
    pub last_event_id: i64,
    pub directory: String,
    pub background_log_file: String,
}

/// Source selection carried into the actual launch reservation transaction.
/// A host precheck followed by a fresh name lookup cannot replace this fence.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct ResumeFence {
    pub name: Option<String>,
    pub participant: Option<ParticipantRef>,
    pub stopped_event: Option<i64>,
    pub latest_stopped_event: Option<i64>,
    pub snapshot_digest: Option<String>,
    pub session_id: String,
    pub fork: bool,
    pub child: Option<ParticipantRef>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum ResumeSelection {
    Tracked { name: String, snapshot: ResumeSnapshot, fence: ResumeFence, preview_child_name: Option<String> },
    Disk { session_id: String, fence: ResumeFence },
    Missing { diagnostic: String },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct ResumeResolution {
    pub selection: ResumeSelection,
    pub cleanup: super::service::CleanupOutcome,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct AdoptedOrphan {
    pub record: super::service::ParticipantView,
    pub process_identity: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum Request {
    Context(super::context_service::Request),
    Start { mode: StartMode, requested_name: Option<String> },
    PrepareBootstrap { participant: ParticipantRef, force: bool },
    BootstrapFlushed { participant: ParticipantRef, receipt: String },
    SetTimeout { participant: ParticipantRef, timeout: i64 },
    SetSubagentTimeout { participant: ParticipantRef, timeout: Option<i64> },
    SetHints { participant: ParticipantRef, hints: Option<String> },
    PrepareKill { participant: ParticipantRef, initiator: String },
    BeginKill { ticket: KillTicket },
    CompleteKill { ticket: KillTicket },
    KillDenied { ticket: KillTicket },
    ClaimOrphans { candidates: Vec<crate::runtime::process_tracking::OrphanCandidate> },
    StopOperator { participant: ParticipantRef, initiator: String, decision: super::hook_service::OperatorStop },
    ResolveResume { target: String, expected: Option<ParticipantRef>, fork: bool },
    PreviewResume { target: String, fork: bool },
    ReserveResumeFork { source: ResumeFence },
    ReserveResume { source: ResumeFence, launch: super::launch_service::Reserve },
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[expect(clippy::large_enum_variant, reason = "The finite lifecycle envelope carries one inline KillPlan; variable data remains in owned buffers")]
pub(crate) enum Response {
    Context(super::context_service::Response),
    Started(StartOutput),
    BootstrapPrepared { receipt: Option<String> },
    KillPrepared(KillPlan),
    KillGranted,
    KillCompleted(super::hook_service::SessionStopOutcome),
    OperatorStopped(super::hook_service::SessionStopOutcome),
    OrphansClaimed(Vec<AdoptedOrphan>),
    ResumeSelected(ResumeResolution),
    ResumeFork(ResumeFence),
    ResumeReserved(super::launch_service::Reservation),
    Changed,
}
