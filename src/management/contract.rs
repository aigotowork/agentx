//! Native management contract. Public JSON contains no PID, environment or host ticket.
use crate::agents::model_request::RequestError;
use serde::{Deserialize, Serialize};

pub(crate) const MAX_PAGE: u32 = 100;
pub(crate) const DEFAULT_MESSAGE_HISTORY_PAGE: u32 = 20;
pub(crate) const MAX_TEXT: usize = 64 * 1024;

/// Fixed selected dataset, obtained from the listener descriptor.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GenerationInput {
    pub generation: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PageInput {
    pub generation: String,
    pub after: Option<String>,
    pub limit: Option<u32>,
}

/// Native resource cursor; never reuse across messages and launch relations.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentPageInput {
    pub generation: String,
    pub agent: AgentRef,
    pub after: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MessageHistoryInput {
    pub generation: String,
    pub agent: AgentRef,
    pub before: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListHumanMailboxInput {
    pub generation: String,
    pub before: Option<String>,
    #[serde(default)]
    pub pending: bool,
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListRolesInput {
    pub generation: String,
    pub workspace: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectDirectoryInput {
    pub generation: String,
    pub directory: String,
}

/// Metadata only; raw upload bytes remain a separate bounded host argument.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UploadAttachmentInput {
    pub generation: String,
    pub name: String,
    #[serde(default)]
    pub media_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetAttachmentInput {
    pub generation: String,
    pub id: String,
}

/// Consumer list arguments. Labels are AND selectors and require current_only.
/// None uses the native 50-item default; cursors retain their native selector scope.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListAgentsInput {
    pub generation: String,
    pub after: Option<String>,
    pub limit: Option<u32>,
    #[serde(default)]
    pub current_only: bool,
    #[serde(default)]
    pub labels: Vec<String>,
}

/// Read matching local current and stopped members from saved adoption.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListWorkspaceMembersInput {
    pub generation: String,
    pub workspace: String,
    pub after: Option<String>,
    pub limit: Option<u32>,
    #[serde(default)]
    pub current_only: bool,
}

/// Exact object identity is obtained from a page or retained reference, never a name lookup.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetAgentInput {
    pub generation: String,
    pub agent: AgentRef,
}

/// Observe the original generation/request pair after acceptance or transport loss.
/// NotFound does not establish that an external effect never happened.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetOperationInput {
    pub generation: String,
    pub request_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentRef {
    pub name: String,
    pub incarnation: Option<String>,
    pub stopped_event: Option<String>,
}
/// File document state is independent of Agent runtime state; diagnostic marks retained data.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct FileBindingView {
    pub path: String,
    pub schema: Option<String>,
    pub description: Option<String>,
    pub raw_status: Option<String>,
    pub status: Option<String>,
    pub diagnostic: Option<String>,
    pub checked_at: f64,
    pub last_success_at: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct AgentView {
    /// None means historical labels were not recorded; never backfill from current state.
    #[serde(default)]
    pub labels: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default)]
    pub labels_revision: Option<String>,
    #[serde(default)]
    pub file_bindings: Vec<FileBindingView>,
    #[serde(default)]
    pub title: Option<String>,
    /// Pending native delivery cursor; not a model/human read acknowledgement.
    #[serde(default)]
    pub pending_messages: Option<u64>,
    pub agent: AgentRef,
    pub tool: String,
    pub status: String,
    pub stored_status: String,
    pub status_context: String,
    pub request_error: Option<RequestError>,
    pub detail: String,
    pub directory: String,
    pub session_id: Option<String>,
    pub collaboration: CollaborationView,
    pub can_send: bool,
    pub can_stop: bool,
    #[serde(default)]
    pub can_stop_participation: bool,
    #[serde(default)]
    pub can_fork: bool,
    pub can_resume: bool,
    pub has_transcript: bool,
}
/// Direct immutable creation provenance; status and office facts are fresh observations.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct RelatedAgent {
    pub name: String,
    pub incarnation: String,
    /// available | unavailable; unavailable never resolves by name.
    pub availability: String,
    /// Native failed-start observation, independently of retained conversation availability.
    pub launch_failure: Option<String>,
    pub agent: Option<AgentView>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct LaunchRelations {
    pub generation: String,
    /// agent | external | unknown. Legacy and unsupported origin paths are unknown.
    pub origin: String,
    pub upstream: Option<RelatedAgent>,
    /// Exact session used as the Fork input, independent of the creator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_source: Option<AgentRef>,
    pub downstream: Vec<RelatedAgent>,
    pub next: Option<String>,
}
/// Safe adopted/current Role metadata; never includes its prompt or instructions.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct RoleView {
    pub name: String,
    pub path: String,
    pub namespace: String,
    pub profile: String,
    pub description: Option<String>,
    pub default: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct RoleDefinitionView {
    pub name: String,
    pub path: String,
    pub namespace: String,
    pub profile: Option<String>,
    pub description: Option<String>,
    pub default: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct CollaborationView {
    /// adopted | legacy | unavailable
    pub status: String,
    pub workspace: Option<String>,
    pub role: Option<RoleView>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct WorkspaceView {
    pub path: String,
    pub sources: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct DiscoveryCoverage {
    pub agent_pages: u32,
    pub agent_limit_reached: bool,
    pub directories_examined: u32,
    pub scan_limit_reached: bool,
    pub errors: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct WorkspaceCatalog {
    pub generation: String,
    pub workspaces: Vec<WorkspaceView>,
    pub coverage: DiscoveryCoverage,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct RoleCatalog {
    pub generation: String,
    pub workspace: Option<String>,
    pub roles: Vec<RoleDefinitionView>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct EndpointView {
    /// bound | adoptable | absent | ambiguous | role_changed | stale | unknown
    pub status: String,
    pub participant: Option<AgentRef>,
    pub candidates: u32,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct DirectoryEntry {
    pub generation: String,
    pub directory: String,
    pub requested_workspace: Option<String>,
    pub resolved_workspace: Option<String>,
    pub role: RoleView,
    pub endpoint: EndpointView,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct AgentPage {
    pub current_complete: bool,
    pub generation: String,
    pub agents: Vec<AgentView>,
    pub next: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct MessageView {
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    pub id: String,
    pub at: String,
    pub from: String,
    pub text: String,
    pub recipients: Vec<String>,
    pub intent: Option<String>,
    pub reply_to: Option<String>,
    pub thread: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct MessagePage {
    pub generation: String,
    pub messages: Vec<MessageView>,
    pub next_after: String,
    pub has_more: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct MessageHistoryPage {
    pub generation: String,
    /// Chronological order within this page.
    pub messages: Vec<MessageView>,
    /// Exclusive bound for the next older page. Null means history is exhausted.
    #[schema(required = true)]
    pub next_before: Option<String>,
    /// Event watermark captured before reading a latest page. Null on older pages.
    #[schema(required = true)]
    pub live_after: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct ProfileView {
    pub name: String,
    pub tool: String,
    pub description: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct Descriptor {
    /// Missing on older services means unsupported, rather than an empty classification.
    #[serde(default)]
    pub supports_labels: bool,
    /// True when the selected native peer supports workspace member paging.
    /// Missing on older listeners means unsupported.
    #[serde(default)]
    pub supports_workspace_members: bool,
    /// True when the selected native peer supports latest and older message pages.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub supports_message_history: bool,
    pub generation: String,
    pub scope: String,
    pub project: String,
    pub tools: Vec<String>,
    pub profiles: Vec<ProfileView>,
    pub max_page: u32,
    pub max_text: u32,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LaunchInput {
    #[serde(default, deserialize_with = "crate::agents::labels::deserialize_unique")]
    pub labels: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub workspace: Option<String>,
    pub tool: Option<String>,
    pub profile: Option<String>,
    pub directory: Option<String>,
    pub prompt: Option<String>,
}
/// Browser projection of the native snapshot; incarnation stays a decimal string.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct LabelsSnapshot {
    pub generation: String,
    pub agent: AgentRef,
    pub labels: std::collections::BTreeMap<String, String>,
    pub revision: String,
}
/// Patch only named keys. Native labels validation owns bounds and conflicts.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct LabelsPatch {
    pub agent: AgentRef,
    #[serde(default, deserialize_with = "crate::agents::labels::deserialize_unique")]
    pub set: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub remove: Vec<String>,
    pub if_revision: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubmitLabels {
    pub request_id: String,
    pub generation: String,
    pub agent: AgentRef,
    #[serde(default, deserialize_with = "crate::agents::labels::deserialize_unique")]
    pub set: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub remove: Vec<String>,
    pub if_revision: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct LabelsOutcome {
    pub snapshot: LabelsSnapshot,
    pub changed: bool,
}
/// Durable host upload; browser URLs and native local paths are separate representations.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) struct Attachment {
    pub id: String,
    pub name: String,
    pub media_type: String,
    pub size: u64,
    pub url: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct StoredAttachment {
    pub attachment: Attachment,
    pub path: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MessageInput {
    #[serde(default)]
    pub attachments: Vec<String>,
    /// At most 65536 UTF-8 bytes; blank text requires at least one valid attachment. Native validation enforces byte and attachment constraints.
    pub text: String,
    /// Omit or null for no intent; otherwise request, inform or ack. Native validation rejects other strings.
    pub intent: Option<String>,
    /// At most 512 UTF-8 bytes; NUL forbidden.
    pub reply_to: Option<String>,
    /// At most 512 UTF-8 bytes; NUL forbidden.
    pub thread: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) enum Command {
    Launch(LaunchInput),
    PatchLabels(LabelsPatch),
    DirectorySend { directory: String, message: MessageInput },
    Send { agent: AgentRef, message: MessageInput },
    Stop(AgentRef),
    StopParticipation(AgentRef),
    Fork(AgentRef),
    Resume(AgentRef),
    Transcript(AgentRef),
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubmitLaunch {
    pub request_id: String,
    pub generation: String,
    pub launch: LaunchInput,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubmitMessage {
    pub request_id: String,
    pub generation: String,
    pub agent: AgentRef,
    pub message: MessageInput,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubmitDirectoryMessage {
    pub request_id: String,
    pub generation: String,
    pub directory: String,
    pub message: MessageInput,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct DirectorySendOutcome {
    pub entry: DirectoryEntry,
    pub recipient: AgentRef,
    pub sent: SendOutcome,
    pub created: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubmitTarget {
    pub request_id: String,
    pub generation: String,
    pub agent: AgentRef,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct LaunchOutcome {
    pub agents: Vec<String>,
    pub failed: u32,
    pub errors: Vec<String>,
    /// Exact launch reservations in agents order. Absent when no exact identities
    /// were retained. The incarnation need not still be current when read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_refs: Option<Vec<AgentRef>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct SendOutcome {
    pub message_id: String,
    pub recipients: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct StopOutcome {
    pub name: String,
    pub outcome: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct ExchangeView {
    pub position: String,
    pub at: String,
    pub user: String,
    pub assistant: String,
    pub action: String,
    pub files: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct TranscriptPage {
    pub agent: AgentRef,
    pub available: bool,
    pub reason: Option<String>,
    pub exchanges: Vec<ExchangeView>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[allow(clippy::large_enum_variant, reason = "Keep the native domain payload inline in this serialized envelope")]
pub(crate) enum CommandResult {
    Launched(LaunchOutcome),
    LabelsPatched(LabelsOutcome),
    Sent(SendOutcome),
    DirectorySent(DirectorySendOutcome),
    Stopped(StopOutcome),
    Transcript(TranscriptPage),
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum OperationStatus {
    Accepted,
    Running,
    Succeeded,
    Failed,
    Unknown,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct Operation {
    pub request_id: String,
    pub generation: String,
    pub status: OperationStatus,
    pub result: Option<CommandResult>,
    pub error: Option<String>,
}

/// Internal host capability. Never serialized to an HTTP response.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WorkerInput {
    pub scope: String,
    pub generation: String,
    pub request_id: String,
    pub ticket: String,
    pub invocation: crate::service::InvocationContext,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WorkGrant {
    pub operation: Operation,
    pub ticket: Option<String>,
}

/// Resolved human-addressed history. Handling is explicit, independent of reading/delivery.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct HumanMessage {
    pub id: String,
    pub at: String,
    pub from: String,
    /// Frozen accepted sender only; legacy display names never authorize navigation.
    pub sender: Option<AgentRef>,
    pub text: String,
    pub handled_at: Option<String>,
}

/// A bounded descending page and the total unhandled count across the whole mailbox.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct HumanMailbox {
    pub generation: String,
    pub messages: Vec<HumanMessage>,
    pub next_before: Option<String>,
    pub unhandled_count: u64,
}
/// Idempotent, monotonic handling of explicitly displayed message IDs only.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct HandleHumanMessages {
    pub generation: String,
    pub ids: Vec<String>,
}

/// Accepted message addressing, frozen at submission; does not establish delivery/read.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct OfficeMessage {
    pub id: String,
    pub at: String,
    pub sender: Option<AgentRef>,
    pub human_sender: bool,
    pub human_recipient: bool,
    pub recipients: Vec<AgentRef>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct OfficeActivity {
    pub human_messages: Vec<HumanMessage>,
    pub human_has_more: bool,
    pub human_unhandled_count: u64,
    pub generation: String,
    pub next_after: String,
    pub has_more: bool,
    pub messages: Vec<OfficeMessage>,
    pub created: Vec<AgentRef>,
}

/// The native leaf is callable through authenticated local IPC only.
/// Compact membership-only projection; not a public HTTP resource or full Agent page.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AdoptedWorkspaces {
    pub paths: Vec<String>,
    pub agent_pages: u32,
    pub agent_limit_reached: bool,
}

/// The native leaf is callable through authenticated local IPC only.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::large_enum_variant, reason = "Keep the native domain payload inline in this serialized envelope")]
pub(crate) enum Request {
    AdoptedWorkspaces,
    HumanMailbox { before: Option<String>, pending: bool, limit: u32 },
    HandleHumanMessages { ids: Vec<String> },
    RegisterAttachment(StoredAttachment),
    Attachment(String),
    List { after: Option<String>, limit: u32 },
    ListWorkspaceMembers { workspace: String, current_only: bool, after: Option<String>, limit: u32 },
    SelectLabels { after: Option<String>, limit: u32, selectors: Vec<String> },
    Labels(AgentRef),
    OfficeActivity { after: Option<String>, limit: u32 },
    Agent(AgentRef),
    LaunchRelations { agent: AgentRef, after: Option<String>, limit: u32 },
    Messages { agent: AgentRef, after: Option<String>, limit: u32 },
    MessageHistory { agent: AgentRef, before: Option<String>, limit: u32 },
    Send { agent: AgentRef, message: MessageInput },
    TranscriptSource(AgentRef),
    Submit { request_id: String, command: Command, admit: bool },
    Operation { request_id: String },
    Claim { request_id: String, ticket: String },
    Finish { request_id: String, ticket: String, result: Result<CommandResult, String> },
    Abandon { request_id: String, ticket: String, reason: String },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::large_enum_variant, reason = "Keep the native domain payload inline in this serialized envelope")]
pub(crate) enum Response {
    AdoptedWorkspaces(AdoptedWorkspaces),
    HumanMailbox(HumanMailbox),
    Attachment(StoredAttachment),
    Agents(AgentPage),
    OfficeActivity(OfficeActivity),
    Agent(AgentView),
    Labels(LabelsSnapshot),
    LaunchRelations(LaunchRelations),
    Messages(MessagePage),
    MessageHistory(MessageHistoryPage),
    Grant(WorkGrant),
    Operation(Operation),
    Claimed(Command),
    Sent(SendOutcome),
    TranscriptSource(crate::transcript::service::Selection),
}
