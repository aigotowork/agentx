//! Contracts shared by the deployment, the native uplink and both OpenAPI exports.
use crate::cloud::communication::types::AgentView;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub(crate) const MAX_WORKSPACES: usize = 64;
pub(crate) const MAX_ROLES: usize = 512;
pub(in crate::cloud) const MAX_DIRECTORY_BYTES: usize = 1024;
pub(in crate::cloud) const LEASE_FEATURES_HEADER: &str = "X-AgentX-Features";
pub(in crate::cloud) const WORKSPACE_PARENTS_HEADER: &str = "X-AgentX-Workspace-Parents";
pub(in crate::cloud) const WORKSPACE_MANAGEMENT_FEATURE: &str = "workspace_management";

/// Empty means the export root. The host additionally resolves each component without following symlinks.
pub(in crate::cloud) fn valid_working_directory(path: &str) -> bool {
    path.is_empty() || (path.len() <= MAX_DIRECTORY_BYTES && path.split('/').all(|part| !part.is_empty() && part.len() <= 255 && !matches!(part, "." | "..") && !part.chars().any(|ch| ch.is_control() || matches!(ch, '\\' | ':'))))
}

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
    /// This configured export is eligible as a parent for remote workspace registration. Set by the server from the lease header.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub managed_parent: Option<bool>,
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
    /// This control lease accepts workspace-relative launch directories.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory_launch: Option<bool>,
    /// This active control lease accepts remote workspace registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_management: Option<bool>,
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
        /// Slash-separated path relative to the workspace export. Omit or send an empty string for its root.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schema(max_length = 1024)]
        working_directory: Option<String>,
        /// Create the final component if absent. Parents must already exist.
        #[serde(skip_serializing_if = "Option::is_none")]
        create_directory: Option<bool>,
    },
    Stop {
        name: String,
        incarnation: String,
    },
    AddWorkspace {
        /// Alias of an eligible configured export.
        parent: String,
        /// Nonempty slash-separated path relative to the parent export.
        #[schema(min_length = 1, max_length = 1024)]
        directory: String,
        /// New workspace alias to publish on this host.
        alias: String,
        /// Create only the final component if absent. Parents must already exist.
        create_directory: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::cloud) enum ControlOutcome {
    Succeeded { agent: AgentView },
    WorkspaceAdded { alias: String },
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
