use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub(super) const DIRECTORY_LIMIT: u32 = 200;
pub(super) const BATCH_LIMIT: u32 = 100;
pub(super) const MAX_WAIT_MS: u32 = 20_000;
pub(super) const MAX_BODY_BYTES: usize = 64 * 1024;
pub(super) const MAX_BATCH_BODY_BYTES: usize = 1024 * 1024;

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
    pub items: Vec<AgentView>,
    #[schema(required = true)]
    pub next: Option<String>,
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
