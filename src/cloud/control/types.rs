//! Contracts shared by the deployment, the native uplink and both OpenAPI exports.
use crate::cloud::communication::types::AgentView;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub(crate) const MAX_WORKSPACES: usize = 64;
pub(crate) const MAX_ROLES: usize = 512;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct RoleCapability {
    pub name: String,
    pub available: bool,
    #[schema(required = true)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct WorkspaceCapability {
    pub alias: String,
    pub default_available: bool,
    #[schema(required = true)]
    pub default_reason: Option<String>,
    pub roles: Vec<RoleCapability>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub(in crate::cloud) struct HostCapabilities {
    pub generation: String,
    pub host: String,
    pub installation_id: String,
    pub online: bool,
    #[schema(required = true)]
    pub observed_at: Option<i64>,
    pub workspaces: Vec<WorkspaceCapability>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::cloud) enum ControlCommand {
    Launch {
        workspace: String,
        role: Option<String>,
        name: Option<String>,
        title: Option<String>,
        #[schema(max_length = 16384)]
        initial_prompt: Option<String>,
    },
    Stop {
        name: String,
        incarnation: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::cloud) enum ControlOutcome {
    Succeeded { agent: AgentView },
    Failed { code: String },
    Uncertain { code: String },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(in crate::cloud) enum ControlStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Uncertain,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub(in crate::cloud) struct ControlReceipt {
    pub generation: String,
    pub request_id: String,
    pub host: String,
    pub installation_id: String,
    pub command: ControlCommand,
    pub status: ControlStatus,
    #[schema(required = true)]
    pub claim_id: Option<String>,
    #[schema(required = true)]
    pub outcome: Option<ControlOutcome>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct CreateControlOperation {
    pub generation: String,
    pub request_id: String,
    pub host: String,
    pub installation_id: String,
    pub command: ControlCommand,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct RenewControlLease {
    pub generation: String,
    pub installation_id: String,
    pub lease_id: String,
    pub previous_lease_id: Option<String>,
    pub workspaces: Vec<WorkspaceCapability>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct PollControlOperation {
    pub generation: String,
    pub installation_id: String,
    pub lease_id: String,
    #[schema(maximum = 10000)]
    pub wait_ms: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub(in crate::cloud) struct ControlPoll {
    #[schema(required = true)]
    pub operation: Option<ControlReceipt>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::cloud) struct ReportControlOperation {
    pub generation: String,
    pub installation_id: String,
    pub lease_id: String,
    pub claim_id: String,
    pub outcome: ControlOutcome,
}
