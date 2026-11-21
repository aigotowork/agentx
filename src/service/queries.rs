//! Source-owned product read models. TUI presentation extends these records;
//! persistence lives in queries_store and consumers cross typed IPC only.
use crate::integrations::tool::Tool;

#[path = "queries_info.rs"]
pub(crate) mod info;
#[path = "queries_status.rs"]
pub(crate) mod status;

/// Selection within the already acquired AgentX data scope.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum AgentWorkspace {
    #[default]
    All,
    Team(String),
    Global,
}

impl AgentWorkspace {
    pub(crate) fn visible(&self, member: Option<&str>) -> bool {
        Self::visible_for_selector(member, self.as_selector())
    }

    pub(crate) fn visible_for_selector(member: Option<&str>, workspace: Option<&str>) -> bool {
        workspace.is_none_or(|workspace| Self::visible_in_team(member, (workspace != "global").then_some(workspace)))
    }

    pub(crate) fn visible_in_team(member: Option<&str>, workspace: Option<&str>) -> bool {
        member.is_none() || member == workspace
    }

    pub(crate) fn as_selector(&self) -> Option<&str> {
        match self {
            Self::All => None,
            Self::Team(path) => Some(path),
            Self::Global => Some("global"),
        }
    }

    pub(crate) fn display(&self) -> String {
        match self {
            Self::All => "all workspaces".into(),
            Self::Team(path) => crate::shared::platform::shorten_path(path).chars().filter(|ch| !ch.is_control()).collect(),
            Self::Global => "global".into(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub(crate) struct AgentFilter {
    pub workspace: AgentWorkspace,
    pub labels: Vec<crate::agents::labels::LabelPredicate>,
}

impl AgentFilter {
    pub(crate) fn is_unfiltered(&self) -> bool {
        self.workspace == AgentWorkspace::All && self.labels.is_empty()
    }

    pub(crate) fn matches_workspace(&self, launch_context: Option<&str>) -> bool {
        self.workspace == AgentWorkspace::All || self.workspace.visible(crate::agents::workspaces::workspace_from_context(launch_context).as_deref())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct SnapshotOptions {
    pub timeline_limit: usize,
    pub relay_configured: bool,
    pub relay_enabled: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum Request {
    FileBindings(crate::file_bindings::Query),
    AgentInfo { target: info::Target, section: info::Section },
    Diagnostics,
    Listing { target: Option<String> },
    ListingLabels { target: Option<String>, selectors: Vec<String>, workspace: Option<String> },
    AgentNames(AgentFilter),
    StoppedHistory { target: Option<String>, limit: usize },
    Snapshot(SnapshotOptions),
    FilteredSnapshot { options: SnapshotOptions, filter: AgentFilter },
    Stopped,
    FilteredStopped(AgentWorkspace),
    StoppedTargets { targets: Vec<String> },
    Terminal { participant: crate::agents::attachments::ParticipantRef },
    Terminals,
    MatchingTerminals { name: Option<String>, filter: AgentFilter },
    UnreadPreview { participant: crate::agents::attachments::ParticipantRef },
    TranscriptSource { name: String, expected: Option<crate::agents::attachments::ParticipantRef> },
    TranscriptInventory { live_only: bool },
    TranscriptCorrelations { targets: Vec<(String, Option<String>)> },
    SearchTimeline { query: String, limit: usize },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[expect(clippy::large_enum_variant, reason = "The finite query envelope carries one inline TerminalEndpoint for an exact participant; variable data remains in owned buffers")]
pub(crate) enum Response {
    FileBindings(crate::file_bindings::index::Projection),
    AgentInfo(info::Projection),
    Diagnostics(Diagnostics),
    Listing(Listing),
    AgentNames(Vec<String>),
    StoppedHistory(Vec<StoppedEntry>),
    Snapshot(DataState),
    Stopped(Vec<Agent>),
    StoppedTargets(Vec<crate::agents::identity::StoppedAgent>),
    Terminal(TerminalEndpoint),
    Terminals(Vec<TerminalEndpoint>),
    UnreadPreview(Option<String>),
    TranscriptSource(crate::transcript::service::Selection),
    TranscriptInventory(Vec<crate::transcript::service::TrackedSource>),
    TranscriptCorrelations(std::collections::HashMap<String, String>),
    Timeline { messages: Vec<Message>, events: Vec<Event> },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub(crate) struct ParticipantCounts {
    pub active: i64,
    pub listening: i64,
    pub blocked: i64,
    pub error: i64,
    pub launching: i64,
    pub inactive: i64,
    pub total: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct Diagnostics {
    pub counts: ParticipantCounts,
    pub launch_failures: Vec<crate::db::InstanceRow>,
    pub dev_root: Option<String>,
    pub cleanup: crate::agents::service::CleanupOutcome,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct Bindings {
    pub session: bool,
    pub process: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct Listing {
    #[serde(default)]
    pub labels: std::collections::BTreeMap<String, crate::agents::labels::LabelSnapshot>,
    pub cleanup: crate::agents::service::CleanupOutcome,
    pub records: Vec<crate::db::InstanceRow>,
    pub unread: std::collections::HashMap<String, i64>,
    pub bindings: std::collections::HashMap<String, Bindings>,
    pub statuses: std::collections::HashMap<String, crate::agents::lifecycle::ComputedStatus>,
    pub plugin_ports: std::collections::HashMap<String, i64>,
    pub recently_stopped: Vec<String>,
    pub selected_name: Option<String>,
    pub missing_target: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct StoppedEntry {
    pub instance: String,
    pub timestamp: String,
    pub data: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct TerminalEndpoint {
    pub participant: crate::agents::attachments::ParticipantRef,
    pub record: crate::db::InstanceRow,
    pub port: Option<u16>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum AgentStatus {
    Error,
    Active,
    Listening,
    Blocked,
    Launching,
    Inactive,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AgentTool {
    Known(Tool),
    Unknown(String),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Agent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) info_reference: Option<info::Reference>,
    pub name: String,
    pub tool: AgentTool,
    pub status: AgentStatus,
    pub status_context: String,
    pub status_detail: String,
    pub created_at: f64,
    pub status_time: f64,
    pub last_heartbeat: f64,
    pub has_tcp: bool,
    pub directory: String,
    pub unread: usize,
    pub last_event_id: Option<u64>,
    pub device_name: Option<String>,
    pub sync_age: Option<String>,
    pub headless: bool,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub pid: Option<u32>,
    pub terminal_preset: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum SenderKind {
    External,
    Instance,
    System,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Message {
    pub event_id: u64,
    pub sender: String,
    pub recipients: Vec<String>,
    pub body: String,
    pub time: f64,
    pub sender_kind: SenderKind,
    pub intent: Option<String>,
    pub reply_to: Option<u64>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum EventKind {
    Tool,
    Activity(ActivityKind),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum ActivityKind {
    Started,
    Active,
    Listening,
    Stopped,
    Blocked,
    StateChange,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Event {
    pub row_id: u64, // DB row id — monotonic, used as ejection watermark
    pub agent: String,
    pub time: f64,
    pub kind: EventKind,
    pub tool: String,           // for Tool events: "Read", "Edit", etc.
    pub detail: String,         // for Tool: target path; for Activity: description
    pub sub_lines: Vec<String>, // extra detail lines (e.g. stopped snapshot)
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct OrphanProcess {
    pub pid: u32,
    pub tool: AgentTool,
    pub names: Vec<String>,
    pub launched_at: f64,
    pub directory: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DataState {
    pub agents: Vec<Agent>,
    pub remote_agents: Vec<Agent>,
    pub stopped_agents: Vec<Agent>,
    pub orphans: Vec<OrphanProcess>,
    pub messages: Vec<Message>,
    pub events: Vec<Event>,
    pub relay_enabled: bool,
    /// Canonical effective relay state. All UI render branches should switch
    /// on this rather than on `relay_enabled` + raw KV to avoid the false-green
    /// / disabled-but-showing-ok class of bugs. Raw underlying signals (status
    /// KV, last_error, heartbeat age, pid) are intentionally not held here —
    /// they're only meaningful via the derivation, and the JSON output exposes
    /// them under `raw` for forensics.
    pub relay_health: crate::relay::RelayHealth,
    /// FTS search results. `Some` when a text search query is active (both inline and vertical modes).
    pub search_results: Option<(Vec<Message>, Vec<Event>)>,
}

// Known tool values use their canonical name; unknown persisted names survive IPC.
impl serde::Serialize for AgentTool {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let name = match self {
            Self::Known(tool) => tool.as_str(),
            Self::Unknown(raw) => raw,
        };
        serializer.serialize_str(name)
    }
}
impl<'de> serde::Deserialize<'de> for AgentTool {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = <String as serde::Deserialize>::deserialize(deserializer)?;
        Ok(raw.parse().map(Self::Known).unwrap_or_else(|_| Self::Unknown(raw)))
    }
}

pub(crate) fn parse_iso_to_epoch(ts: &str) -> f64 {
    use chrono::{DateTime, NaiveDateTime};
    // DB timestamps are RFC 3339 with microseconds: "2026-02-17T08:32:37.525519+00:00"
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        dt.timestamp() as f64 + (dt.timestamp_subsec_millis() as f64 / 1000.0)
    } else if let Ok(dt) = NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S") {
        dt.and_utc().timestamp() as f64
    } else {
        0.0
    }
}
