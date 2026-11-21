//! Finite authenticated peer intentions and the worker execution grant. These
//! are Relay contracts, not CLI commands or a generic remote effect protocol.
use crate::agents::attachments::ParticipantRef;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) enum PeerCapabilities {
    NotSynced,
    Stale(u64),
    Legacy,
    Advertised(Vec<String>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct RemoteLaunch {
    pub tool: Option<String>,
    #[serde(default)]
    pub selection: crate::runtime::launch_profile::LaunchSelection,
    pub count: usize,
    pub args: Vec<String>,
    pub launcher: Option<String>,
    pub system_prompt: Option<String>,
    pub initial_prompt: Option<String>,
    pub background: bool,
    pub terminal: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct RemoteResume {
    pub target: String,
    pub fork: bool,
    pub extra_args: Vec<String>,
    pub launcher: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct RemoteTranscript {
    pub target: String,
    pub last: usize,
    pub range: Option<String>,
    pub json: bool,
    pub full: bool,
    pub detailed: bool,
    pub display_target: Option<String>,
    pub origin_device: Option<String>,
}

/// Only real process/terminal/Provider/file operations leave the dispatcher.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) enum HostAction {
    Launch(RemoteLaunch),
    PrepareLaunch(RemoteLaunch),
    Kill { target: String },
    Resume(RemoteResume),
    ConfigGet { fields: Vec<String> },
    ConfigSet { field: String, value: String },
    InstanceTimeoutDefault { target: String, requested: String },
    RelayOff,
    TermScreen { target: String, json: bool, clean: bool },
    TermInject { target: String, text: String, enter: bool },
    Transcript(RemoteTranscript),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ControlEffect {
    pub effect_id: String,
    pub token: String,
    pub source_device: String,
    pub request_id: Option<String>,
    pub initiated_by: String,
    /// Captured at admission for actions on a current local participant.
    pub participant: Option<ParticipantRef>,
    pub action: HostAction,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct ControlReply {
    pub ok: bool,
    pub result: Value,
}

impl ControlReply {
    pub(crate) fn error(error: impl ToString) -> Self {
        Self { ok: false, result: serde_json::json!({"error": error.to_string()}) }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub(crate) struct ReceiveResult {
    pub changed: bool,
    pub effects: Vec<ControlEffect>,
    pub wake_ports: Vec<u16>,
}
