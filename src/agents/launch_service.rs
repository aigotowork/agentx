//! Participant launch intentions. Process creation and terminal I/O remain in
//! runtime; reservation ownership is an authenticated caller plus incarnation.
use super::attachments::ParticipantRef;
use crate::runtime::hosts::ChildProcess;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct Reserve {
    pub name: Option<String>,
    pub tool: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "super::labels::deserialize_unique")]
    pub labels: super::labels::LabelSet,
    pub directory: std::path::PathBuf,
    pub process_binding: String,
    pub prior_session: Option<String>,
    pub resume_cursor: Option<i64>,
    pub background: bool,
    pub launch_args: Vec<String>,
    #[serde(default)]
    pub launch_profile: Option<crate::runtime::launch_profile::LaunchProvenance>,
    #[serde(default)]
    pub collaboration: Option<super::workspaces::LaunchCollaboration>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reservation {
    pub token: String,
    pub participant: ParticipantRef,
    pub process_binding: String,
    pub launch_cursor: i64,
    pub initial_cursor: i64,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct BackgroundProcess {
    pub child: ChildProcess,
    pub log_file: String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct Batch {
    pub launcher: String,
    pub batch_id: String,
    pub tool: String,
    pub requested: usize,
    pub launched: usize,
    pub failed: usize,
    pub background: bool,
    pub instances: Vec<String>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[allow(clippy::large_enum_variant, reason = "Keep the native domain payload inline in this serialized envelope")]
pub(crate) enum Request {
    /// Probe native collaboration support before sending optional launch data.
    WorkspaceSupport,
    CreationSupport,
    /// Reject title-bearing launches against older daemons before reservation.
    TitleSupport,
    LabelsSupport,
    /// Read endpoint policy without adopting, clearing, or reserving.
    EndpointInspect {
        workspace: Option<String>,
        role_path: String,
    },
    /// Resolve or adopt one exact workspace endpoint without creating a process.
    EndpointLookup {
        workspace: Option<String>,
        role_path: String,
    },
    /// Explicitly clear a definitively retired endpoint before changing its role.
    EndpointReset {
        workspace: Option<String>,
    },
    Reserve(Reserve),
    TerminalLaunched {
        reservation: Reservation,
        requested: Option<String>,
        effective: String,
        background: Option<BackgroundProcess>,
    },
    RecordSupervisor(Reservation),
    RefreshSupervisor {
        reservation: Reservation,
        registration: crate::runtime::hosts::ProcessRegistration,
    },
    FinishSupervisor(Reservation),
    CancelFailed(Reservation),
    /// A PTY wrapper failed before it could register a Provider child.
    SpawnFailed {
        participant: ParticipantRef,
        launch_cursor: i64,
        detail: String,
    },
    BatchLaunched(Batch),
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum Response {
    Endpoint(Option<ParticipantRef>),
    EndpointInspected(super::workspaces::EndpointInspection),
    Reserved(Reservation),
    Finished { name: String },
    Changed,
}
