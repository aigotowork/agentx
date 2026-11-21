//! Provider-neutral request observations; a failed attempt is not process death.
use super::attachments::ParticipantRef;

pub(crate) const ERROR_CONTEXT: &str = "model:request";

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequestPhase {
    Started,
    Completed,
    Failed,
    Cancelled,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct ObserveModelRequest {
    pub participant: ParticipantRef,
    pub session_id: String,
    pub turn_id: String,
    pub model: String,
    pub phase: RequestPhase,
    pub error: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct RequestError {
    pub turn_id: String,
    pub source: String,
    pub reason: String,
    pub error_since: i64,
    pub last_error_at: i64,
    pub consecutive_failures: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct ModelRequestState {
    pub session_id: String,
    pub turn_id: String,
    pub model: String,
    pub phase: RequestPhase,
    pub last_progress_at: Option<i64>,
    pub error: Option<RequestError>,
    pub retired_turns: Vec<String>,
}
