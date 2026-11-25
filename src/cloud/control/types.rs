//! Contracts shared by the deployment, the native uplink and both OpenAPI exports.
use crate::cloud::communication::types::AgentView;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub(crate) const MAX_WORKSPACES: usize = 64;
pub(crate) const MAX_ROLES: usize = 512;
pub(in crate::cloud) const MAX_DIRECTORY_BYTES: usize = 1024;
pub(in crate::cloud) const LEASE_FEATURES_HEADER: &str = "X-AgentX-Features";
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

/// Shared fields also composed into the flattened WorkspaceView schema.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub(in crate::cloud) struct WorkspaceCapability {
    #[schema(pattern = "^WS[A-Za-z0-9]{10}$")]
    pub workspace_id: String,
    pub name: String,
    pub default_available: bool,
    #[schema(required = true)]
    pub default_reason: Option<String>,
    pub roles: Vec<RoleCapability>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(super) struct WorkspaceQuery {
    /// Expected Namespace generation. Bind the first page's generation when continuing a cursor.
    pub(super) generation: Option<String>,
    pub(super) host: Option<String>,
    /// Exclusive cursor from a previous page.
    pub(super) after: Option<String>,
    #[param(minimum = 1, maximum = 100)]
    pub(super) limit: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(super) struct WorkspaceView {
    pub(super) host: String,
    pub(super) online: bool,
    #[serde(flatten)]
    pub(super) capability: WorkspaceCapability,
}

#[derive(Debug, Serialize, ToSchema)]
pub(super) struct WorkspacePage {
    /// Namespace generation verified in the same transaction as the page.
    pub(super) generation: String,
    pub(super) items: Vec<WorkspaceView>,
    #[schema(required = true)]
    pub(super) next: Option<String>,
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
    pub project_roots: Vec<String>,
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
        #[schema(pattern = "^WS[A-Za-z0-9]{10}$")]
        workspace_id: String,
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
        #[schema(pattern = "^[A-Za-z0-9]{12}$")]
        incarnation: String,
    },
    AddWorkspace {
        /// Alias of an eligible configured export.
        parent: String,
        /// Nonempty slash-separated path relative to the parent export.
        #[schema(min_length = 1, max_length = 1024)]
        directory: String,
        /// Display name to publish for this project.
        name: String,
        /// Create only the final component if absent. Parents must already exist.
        create_directory: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::cloud) enum ControlOutcome {
    Succeeded {
        agent: AgentView,
    },
    WorkspaceAdded {
        #[schema(pattern = "^WS[A-Za-z0-9]{10}$")]
        workspace_id: String,
    },
    Failed {
        code: String,
    },
    Uncertain {
        code: String,
    },
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
    pub project_roots: Vec<String>,
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
