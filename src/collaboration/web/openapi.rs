//! Workbench listener projection: fixed project, operator-only input, actual integer markers.
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(
    info(
        title = "AgentX Workbench",
        version = "1",
        description = "Fixed-scope/project workbench. Every integer uses the native $integer marker codec. Exact numeric local Host and same Origin admission; POST application/json only. Collaboration writes require listener collaboration=true. Input exposes only OperatorCommand, never AgentReport or caller actor context. A 200 Rejected response is a domain refusal, distinct from transport errors. Original command_id/generation and payload recover durable native changes; do not silently rebase revisions or replace identities."
    ),
    paths(super::connection, super::read, super::apply)
)]
struct WorkbenchApi;
pub(crate) fn document() -> serde_json::Value {
    crate::browser::openapi::marker_document(WorkbenchApi::openapi())
}
