use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub(super) const DIRECTORY_LIMIT: u32 = 200;
pub(super) const BATCH_LIMIT: u32 = 100;
pub(super) const MAX_WAIT_MS: u32 = 20_000;
pub(in crate::cloud) const MAX_BODY_BYTES: usize = 64 * 1024;
pub(super) const MAX_BATCH_BODY_BYTES: usize = 1024 * 1024;
pub(in crate::cloud) const NATURAL_REPLY_PREFIX: &str = "space-final:";
pub(in crate::cloud) const OPERATOR_HOST: &str = "~space";
pub(in crate::cloud) const OPERATOR_NAME: &str = "operator";
pub(in crate::cloud) const OPERATOR_INCARNATION: &str = "~human";

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct Descriptor {
    pub scope_id: String,
    pub generation: String,
    pub protocol_version: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub(in crate::cloud) struct HostView {
    pub name: String,
    pub last_seen: i64,
    pub online: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct AgentView {
    pub name: String,
    pub host: String,
    pub incarnation: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(in crate::cloud) struct HostQuery {
    pub after: Option<String>,
    #[param(minimum = 1, maximum = 200)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(in crate::cloud) struct AgentQuery {
    pub after: Option<String>,
    #[param(minimum = 1, maximum = 200)]
    pub limit: Option<u32>,
    pub host: Option<String>,
    /// Include host-reported Agent facts. Omit for identity-only legacy clients.
    pub facts: Option<bool>,
}

/// Select one exact cloud registration and page backward through its accepted messages.
#[derive(Debug, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(in crate::cloud) struct MessageHistoryQuery {
    pub generation: String,
    pub host: String,
    pub name: String,
    pub incarnation: String,
    /// Exclusive forward cursor. Zero starts at the beginning. Forward pages use ascending IDs.
    pub after: Option<String>,
    /// Exclusive positive decimal message ID; omit to start with the latest messages.
    pub before: Option<String>,
    #[param(minimum = 1, maximum = 100)]
    pub limit: Option<u32>,
}

/// Durable scope traffic, in descending message ID order. This is not a Provider transcript.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct MessageHistoryPage {
    pub items: Vec<MessageView>,
    #[schema(required = true)]
    pub next: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct HostPage {
    pub items: Vec<HostView>,
    #[schema(required = true)]
    pub next: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct AgentPage {
    pub items: Vec<CloudAgentSummary>,
    #[schema(required = true)]
    pub next: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct CloudAgentSummary {
    pub name: String,
    pub host: String,
    pub incarnation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facts: Option<CloudAgentFacts>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct CloudAgentFacts {
    #[schema(required = true)]
    pub role: Option<String>,
    #[schema(required = true)]
    pub title: Option<String>,
    pub cwd: String,
    pub status: CloudAgentStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(in crate::cloud) enum CloudAgentStatus {
    Active,
    Idle,
    Error,
    Unknown,
    Stopped,
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(in crate::cloud) struct TranscriptHistoryQuery {
    pub generation: String,
    pub host: String,
    pub name: String,
    pub incarnation: String,
    /// Exclusive positive decimal cursor for older records, newest first.
    pub before: Option<String>,
    /// Exclusive decimal cursor for newer records, oldest first. Use 0 to follow an empty transcript.
    pub after: Option<String>,
    #[param(minimum = 1, maximum = 50)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct TranscriptHistoryPage {
    pub agent: AgentView,
    pub available: bool,
    pub items: Vec<CloudTranscriptRecord>,
    #[schema(required = true)]
    pub next: Option<String>,
    #[schema(required = true)]
    pub latest: Option<String>,
    #[schema(required = true)]
    pub facts: Option<CloudAgentFacts>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(in crate::cloud) enum CloudTranscriptRole {
    User,
    Assistant,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct CloudTranscriptRecord {
    pub id: String,
    pub role: CloudTranscriptRole,
    pub text: String,
}

pub(in crate::cloud) const TRANSCRIPT_BATCH: usize = 12;
pub(in crate::cloud) const TRANSCRIPT_TEXT_BYTES: usize = 8 * 1024;
pub(in crate::cloud) const TRANSCRIPT_FACT_BYTES: usize = 2048;

#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct PublishTranscriptRecord {
    /// Host-generated digest of the exact Provider item identity, used only for retries.
    pub key: String,
    pub role: CloudTranscriptRole,
    pub text: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct PublishTranscript {
    pub generation: String,
    pub installation_id: String,
    pub agent: AgentView,
    pub facts: CloudAgentFacts,
    pub items: Vec<PublishTranscriptRecord>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct HostRegistration {
    pub generation: String,
    pub installation_id: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct AgentRegistration {
    pub generation: String,
    pub installation_id: String,
    pub host: String,
    pub incarnation: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(in crate::cloud) enum Intent {
    Request,
    Inform,
    Ack,
}

impl Intent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Inform => "inform",
            Self::Ack => "ack",
        }
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct SendMessage {
    pub generation: String,
    pub installation_id: String,
    pub request_id: String,
    pub from: AgentView,
    pub to: String,
    pub body: String,
    pub intent: Intent,
    pub reply_to: Option<String>,
    pub thread: Option<String>,
}

/// Submit an operator message to one exact Agent registration.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct SendHumanMessage {
    pub generation: String,
    #[schema(min_length = 1, max_length = 128)]
    pub request_id: String,
    pub to: AgentView,
    /// Nonblank message text, at most 64 KiB of UTF-8.
    pub body: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub(in crate::cloud) struct MessageView {
    pub id: String,
    pub from: AgentView,
    pub to: AgentView,
    pub body: String,
    pub intent: Intent,
    #[schema(required = true)]
    pub reply_to: Option<String>,
    #[schema(required = true)]
    pub thread: Option<String>,
    pub created_at: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(in crate::cloud) enum OperationStatus {
    Accepted,
    Claimed,
    Confirmed,
    Undeliverable,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct MessageOperation {
    pub request_id: String,
    pub status: OperationStatus,
    pub message: MessageView,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct ClaimInbox {
    pub generation: String,
    pub installation_id: String,
    pub agent: AgentView,
    pub claim_id: String,
    #[schema(minimum = 1, maximum = 100)]
    pub limit: Option<u32>,
    #[schema(maximum = 20_000)]
    pub wait_ms: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct InboxBatch {
    pub claim_id: String,
    pub items: Vec<MessageView>,
    #[schema(required = true)]
    pub next: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct ConfirmInbox {
    pub generation: String,
    pub installation_id: String,
    pub agent: AgentView,
    pub claim_id: String,
    pub message_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct InboxConfirmation {
    pub confirmed: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct RecoverInbox {
    pub generation: String,
    pub installation_id: String,
    pub agent: AgentView,
    pub claim_id: String,
    #[serde(default)]
    pub release: bool,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct ScopeError {
    pub code: &'static str,
    pub message: String,
}
