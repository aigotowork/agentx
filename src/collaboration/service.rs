//! Canonical native work contract. Commands are generation/actor/intent bound;
//! replay is checked before revision, and accepted work is closed.
use crate::agents::attachments::ParticipantRef;
use serde::{Deserialize, Serialize};

pub(crate) use super::store::handle;

pub(crate) const MAX_TEXT: usize = 8192;
pub(crate) const MAX_TITLE: usize = 240;
pub(crate) const MAX_SOURCE_REFERENCE: usize = 2048;
pub(crate) const MAX_SOURCE_REVISION: usize = 256;
pub(crate) const MAX_CRITERIA: usize = 32;
pub(crate) const MAX_ARTIFACTS: usize = 32;
pub(crate) const MAX_HISTORY: usize = 256;
pub(crate) const MAX_RECORD_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_WORKS: usize = 512;
pub(crate) const MAX_PAGE: u32 = 64;
/// Human/Agent usage examples only. The native enum below owns validation.
pub(crate) const REPORT_GUIDANCE: &str = r#"AgentReport payloads (ordinary JSON; do not infer completion from idle):
  {"Progress":{"kind":"Acknowledged","summary":"Received assignment"}}
  {"Progress":{"kind":"Started","summary":"Implementing the agreed brief"}}
  {"Question":{"prompt":"Which overwrite policy?","suggestions":["Refuse","Replace"]}}
  {"Result":{"summary":"Implemented change","source":{"reference":"git","revision":"FULL_IMMUTABLE_COMMIT_ID"},"artifacts":[{"Git":{"revision":"FULL_IMMUTABLE_COMMIT_ID","path":"src/export.rs"}}]}}
  {"Check":{"request_id":"EXACT_ACTIVE_CHECK_REQUEST","result_id":"EXACT_CURRENT_RESULT","observed_fingerprint":"FULL_IMMUTABLE_COMMIT_ID","verdict":"Passed","criteria":[{"criterion_id":"EXACT_REQUIRED_CRITERION_ID","verdict":"Passed","evidence":"Concrete command/result or observation"}],"evidence":["Independent source/check evidence"]}}
Progress kind is Acknowledged, Started or Progress. Check verdict is Passed, Failed or Unable; a pass requires matching immutable source and every required criterion passed with evidence. A Result never requests a check automatically. Only the explicitly requested independent checker may report Check. Read work show for current revision/assignment/brief/result/request/criterion IDs before reporting; retain one command ID per exact intent. On a lost reply, use work command with that ID before retrying; never generate a duplicate command for uncertain work. No merge, deployment or current working-tree certification is implied."#;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) struct WorkRef {
    pub scope: String,
    pub generation: String,
    pub project: String,
    pub id: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum Role {
    Implementer,
    Checker,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) struct SourceBasis {
    pub reference: String,
    pub revision: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) struct Criterion {
    pub id: String,
    pub text: String,
    pub required: bool,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) struct BriefInput {
    pub title: String,
    pub goal: String,
    pub criteria: Vec<Criterion>,
    pub source: Option<SourceBasis>,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Brief {
    pub revision: u64,
    pub input: BriefInput,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Assignment {
    pub id: String,
    pub role: Role,
    pub participant: ParticipantRef,
    pub dispatched_brief: Option<u64>,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct MessageReceipt {
    pub event: i64,
    pub recipients: Vec<ParticipantRef>,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) enum MessageKind {
    Assignment,
    Note,
    Answer,
    CheckRequest,
    Changes,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct WorkMessage {
    pub id: String,
    pub kind: MessageKind,
    pub text: String,
    pub receipt: MessageReceipt,
    pub at: String,
    pub assignment_ids: Vec<String>,
    pub brief_revision: u64,
    pub result_id: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) enum ProgressKind {
    Acknowledged,
    Started,
    Progress,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Progress {
    pub assignment_id: String,
    pub brief_revision: u64,
    pub kind: ProgressKind,
    pub summary: String,
    pub at: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Question {
    pub id: String,
    pub assignment_id: String,
    pub brief_revision: u64,
    pub prompt: String,
    pub suggestions: Vec<String>,
    pub answer: Option<Answer>,
    pub at: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Answer {
    pub text: String,
    pub message_id: String,
    pub brief_revision: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) enum Artifact {
    Path { path: String, fingerprint: String },
    Git { revision: String, path: String },
    Message { event: i64 },
    External { reference: String },
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct ResultInput {
    pub summary: String,
    pub source: SourceBasis,
    pub artifacts: Vec<Artifact>,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct WorkResult {
    pub id: String,
    pub assignment_id: String,
    pub brief_revision: u64,
    pub input: ResultInput,
    pub at: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum Verdict {
    Passed,
    Failed,
    Unable,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct CriterionCheck {
    pub criterion_id: String,
    pub verdict: Verdict,
    pub evidence: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct CheckInput {
    pub request_id: String,
    pub result_id: String,
    pub observed_fingerprint: String,
    pub verdict: Verdict,
    pub criteria: Vec<CriterionCheck>,
    pub evidence: Vec<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct CheckRequest {
    pub id: String,
    pub assignment_id: String,
    pub result_id: String,
    pub brief_revision: u64,
    pub reason: String,
    pub message_id: String,
    pub at: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct CheckReport {
    pub id: String,
    pub assignment_id: String,
    pub brief_revision: u64,
    pub input: CheckInput,
    pub at: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) struct ResultSelection {
    pub result_id: String,
    pub check_id: String,
    pub brief_revision: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) enum DecisionKind {
    Accepted,
    ChangesRequested,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Decision {
    pub id: String,
    pub kind: DecisionKind,
    pub result_id: String,
    pub check_id: Option<String>,
    pub brief_revision: u64,
    pub reason: String,
    pub message_id: Option<String>,
    pub at: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Work {
    pub reference: WorkRef,
    pub revision: u64,
    pub briefs: Vec<Brief>,
    pub assignments: Vec<Assignment>,
    pub implementer: String,
    pub checker: String,
    pub messages: Vec<WorkMessage>,
    pub progress: Vec<Progress>,
    pub questions: Vec<Question>,
    pub results: Vec<WorkResult>,
    pub check_requests: Vec<CheckRequest>,
    pub checks: Vec<CheckReport>,
    pub decisions: Vec<Decision>,
    pub created_at: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Eligibility {
    pub selection: Option<ResultSelection>,
    pub reasons: Vec<String>,
    pub closed: bool,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct WorkDetail {
    pub work: Work,
    pub eligibility: Eligibility,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Candidate {
    pub participant: ParticipantRef,
    pub label: String,
    pub tool: String,
    pub runtime_status: String,
    pub unavailable: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct WorkSummary {
    pub reference: WorkRef,
    pub revision: u64,
    pub title: String,
    pub brief_revision: u64,
    pub implementer: ParticipantRef,
    pub checker: ParticipantRef,
    pub open_questions: u32,
    pub current_result: Option<String>,
    pub closed: bool,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Page {
    pub limit: u32,
    pub after: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct ThreadPage {
    pub limit: u32,
    pub after: Option<i64>,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct ThreadMessage {
    pub event: i64,
    pub at: Option<String>,
    pub author: String,
    pub text: String,
    pub reply_to: Option<String>,
    pub intent: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) enum Read {
    List(Page),
    Detail(WorkRef),
    Candidates(Page),
    Command { command_id: String },
    Thread { work: WorkRef, page: ThreadPage },
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) enum ReadValue {
    List { items: Vec<WorkSummary>, next: Option<String> },
    Detail(Box<WorkDetail>),
    Candidates { items: Vec<Candidate>, next: Option<String> },
    Command(Option<Change>),
    Thread { items: Vec<ThreadMessage>, next: Option<i64> },
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) enum OperatorCommand {
    CreateWork { brief: BriefInput, implementer: ParticipantRef, checker: ParticipantRef },
    UpdateBrief(BriefInput),
    BindParticipant { role: Role, participant: ParticipantRef },
    DispatchAssignment { role: Role },
    SendNote { roles: Vec<Role>, text: String },
    AnswerQuestion { question_id: String, text: String, amendment: Option<BriefInput> },
    RequestCheck { result_id: String, reason: String },
    AcceptResult(ResultSelection),
    RequestChanges { result_id: String, check_id: Option<String>, reason: String },
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum AgentReport {
    Progress { kind: ProgressKind, summary: String },
    Question { prompt: String, suggestions: Vec<String> },
    Result(ResultInput),
    Check(CheckInput),
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Command {
    Operator(OperatorCommand),
    Report { assignment_id: String, brief_revision: u64, report: AgentReport },
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct CommandCall {
    pub command_id: String,
    pub generation: String,
    pub project: String,
    pub work: Option<WorkRef>,
    pub expected_revision: Option<u64>,
    pub command: Command,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) enum Effect {
    Saved,
    MessageAccepted,
    Reported,
    Accepted,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Change {
    pub command_id: String,
    pub reference: WorkRef,
    pub revision: u64,
    pub effect: Effect,
    pub messages: Vec<MessageReceipt>,
    pub replayed: bool,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum Failure {
    InvalidInput { field: String, reason: String },
    Forbidden,
    StaleReference,
    RevisionConflict { current_revision: u64 },
    NotFound,
    Capacity,
    Ineligible { reasons: Vec<String> },
    Closed,
    CommandConflict,
    Unavailable,
    OutcomeUnknown { command_id: String },
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Request {
    Read { generation: String, project: String, read: Read },
    Apply(CommandCall),
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) enum Response {
    Read(ReadValue),
    Applied(Change),
    Rejected(Failure),
}
