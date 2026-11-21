//! Authoritative finite native inspection contract; internal host samples are not browser input.
use crate::agents::model_request::ModelRequestState;
use crate::management::contract::AgentRef;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecoveryPolicy {
    pub enabled: bool,
    pub error_duration_seconds: u32,
}
impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self { enabled: false, error_duration_seconds: 600 }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct Settings {
    pub enabled: bool,
    pub collected_from: Option<i64>,
    pub revision: u64,
    pub recovery: RecoveryPolicy,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct Gap {
    pub from: i64,
    pub through: Option<i64>,
    pub reason: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct Coverage {
    pub observed_at: Option<i64>,
    pub retained_from: Option<i64>,
    pub discovery_complete: bool,
    pub dropped_observations: u64,
    pub gaps: Vec<Gap>,
    pub basis: String,
}
/// Parser-normalized cumulative snapshot. Zero does not establish field presence.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, utoipa::ToSchema)]
pub(crate) struct Usage {
    pub input: String,
    pub output: String,
    pub cache_read: String,
    pub cache_write: String,
    pub reasoning: Option<String>,
    pub context_tokens: Option<String>,
    pub context_window: Option<String>,
    pub context_usage_ratio: Option<f64>,
    pub model: Option<String>,
    pub unit: String,
    pub basis: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct UsageSample {
    pub agent: AgentRef,
    /// Opaque native tool/path/session correlation; no filesystem path is published.
    pub source_key: String,
    pub session_id: Option<String>,
    pub usage: Option<Usage>,
    /// available | missing | read_failed; a missing sample must not reuse old usage as current.
    pub availability: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct Observation {
    pub id: String,
    pub at: i64,
    pub agent: AgentRef,
    pub session_id: Option<String>,
    pub status: String,
    pub model_request: Option<ModelRequestState>,
    pub sample: Option<UsageSample>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Episode {
    pub agent: AgentRef,
    pub session_id: String,
    pub source: String,
    pub failing_turn: String,
    pub error_since: i64,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct RecoveryAttempt {
    pub id: String,
    pub episode: Episode,
    pub at: i64,
    /// queued: native mailbox accepted; never equate this with model execution/success.
    pub status: String,
    pub message_id: Option<String>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct AnalysisReport {
    pub algorithm: String,
    pub observation_ids: Vec<String>,
    pub observation_count: u32,
    pub native_error_count: u32,
    pub distinct_agents: u32,
    pub usage_sources: u32,
    pub unknown_usage_count: u32,
    pub caveats: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct AnalysisJob {
    pub id: String,
    pub input_revision: u64,
    pub input_coverage: Coverage,
    pub created_at: i64,
    /// queued | succeeded | cancelled; deterministic execution has no external unknown effect.
    pub status: String,
    pub observation_ids: Vec<String>,
    pub report: Option<AnalysisReport>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct Snapshot {
    pub generation: String,
    pub settings: Settings,
    pub coverage: Coverage,
    pub observations: Vec<Observation>,
    pub jobs: Vec<AnalysisJob>,
    pub attempts: Vec<RecoveryAttempt>,
    pub dropped_attempts: u64,
    pub observation_revision: u64,
}
/// Public payloads are shared with native Request. Newtype variants retain the
/// existing externally tagged JSON; only this closed mutation enum is public.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigureInput {
    pub generation: String,
    pub expected_revision: u64,
    pub enabled: bool,
    pub recovery: RecoveryPolicy,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AnalyzeInput {
    pub generation: String,
    pub request_id: String,
    pub observation_ids: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CancelAnalysisInput {
    pub generation: String,
    pub id: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequestContinuationInput {
    pub generation: String,
    pub episode: Episode,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) enum PublicMutation {
    Configure(ConfigureInput),
    Analyze(AnalyzeInput),
    CancelAnalysis(CancelAnalysisInput),
    RequestContinuation(RequestContinuationInput),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum Request {
    Read {
        generation: String,
    },
    Configure(ConfigureInput),
    /// Internal collector input, never a public HTTP mutation. Empty samples still collect native facts.
    Collect {
        generation: String,
        expected_revision: u64,
        samples: Vec<UsageSample>,
        discovery_complete: bool,
    },
    Analyze(AnalyzeInput),
    RunAnalysis {
        generation: String,
        id: String,
    },
    CancelAnalysis(CancelAnalysisInput),
    RequestContinuation(RequestContinuationInput),
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) enum Response {
    Snapshot(Snapshot),
    Job(AnalysisJob),
    Attempt(RecoveryAttempt),
}
