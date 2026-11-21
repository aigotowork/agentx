//! Passive observation listener projection; no daemon acquisition or public report/mutation input.
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(
    info(
        title = "AgentX Observation",
        version = "1",
        description = "Passive fixed existing scope. Every integer uses the native $integer marker codec. Exact numeric local Host and same Origin admission; POST application/json only. BrowserRead is the closed thirteen-read boundary. LocalFailure remains a 200 observation outcome, distinct from HTTP errors; an unavailable daemon is never acquired or started. Preserve daemon/epoch/stream references, stale/gap and clock comparability. No report, mutation, arbitrary native request or scope-selection body is exposed."
    ),
    paths(super::scope, super::read)
)]
struct ObservationApi;
pub(crate) fn document() -> serde_json::Value {
    crate::browser::openapi::marker_document(ObservationApi::openapi())
}
