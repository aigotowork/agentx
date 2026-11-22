//! Public management projection, generated from native serde DTOs and real HTTP bindings.
//! No Application request, worker ticket, database or runtime startup is exported.
use serde::Serialize;
use utoipa::OpenApi;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) enum ErrorCode {
    InvalidInput,
    Forbidden,
    NotFound,
    Conflict,
    StaleReference,
    Unsupported,
    Unavailable,
    OutcomeUnknown,
    Internal,
}
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct PublicError {
    pub code: ErrorCode,
    pub message: String,
}
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct ErrorEnvelope {
    pub error: PublicError,
}

/// Exhaustive native-to-v1 mapping shared by actual HTTP failure encoding.
impl ErrorEnvelope {
    pub(crate) fn from_service(error: crate::service::ServiceError) -> (axum::http::StatusCode, Self) {
        use crate::service::ServiceError;
        use axum::http::StatusCode;
        let (status, code) = match &error {
            ServiceError::InvalidInput(_) => (StatusCode::BAD_REQUEST, ErrorCode::InvalidInput),
            ServiceError::Forbidden(_) | ServiceError::IdentityRequired(_) => (StatusCode::FORBIDDEN, ErrorCode::Forbidden),
            ServiceError::NotFound(_) => (StatusCode::NOT_FOUND, ErrorCode::NotFound),
            ServiceError::Conflict(_) => (StatusCode::CONFLICT, ErrorCode::Conflict),
            ServiceError::StaleFence => (StatusCode::CONFLICT, ErrorCode::StaleReference),
            ServiceError::Unsupported(_) => (StatusCode::UNPROCESSABLE_ENTITY, ErrorCode::Unsupported),
            ServiceError::Unavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, ErrorCode::Unavailable),
            ServiceError::OutcomeUnknown(_) => (StatusCode::SERVICE_UNAVAILABLE, ErrorCode::OutcomeUnknown),
            ServiceError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::Internal),
        };
        (status, Self { error: PublicError { code, message: error.to_string() } })
    }
}

/// HTTP body is raw bytes, never a JSON array or a host filesystem path.
#[allow(dead_code)]
#[derive(utoipa::ToSchema)]
#[schema(value_type = String, format = Binary)]
pub(crate) struct BinaryBody(Vec<u8>);

// Route-specific external tags reuse the exact native payloads; no internal
// collection/execution variants are reachable through these public schemas.
#[derive(utoipa::ToSchema)]
#[allow(dead_code, reason = "Route-specific OpenAPI schema only")]
pub(crate) enum ConfigureBody {
    Configure(crate::inspection::contract::ConfigureInput),
}
#[derive(utoipa::ToSchema)]
#[allow(dead_code, reason = "Route-specific OpenAPI schema only")]
pub(crate) enum AnalyzeBody {
    Analyze(crate::inspection::contract::AnalyzeInput),
}
#[derive(utoipa::ToSchema)]
#[allow(dead_code, reason = "Route-specific OpenAPI schema only")]
pub(crate) enum CancelAnalysisBody {
    CancelAnalysis(crate::inspection::contract::CancelAnalysisInput),
}
#[derive(utoipa::ToSchema)]
#[allow(dead_code, reason = "Route-specific OpenAPI schema only")]
pub(crate) enum ContinuationBody {
    RequestContinuation(crate::inspection::contract::RequestContinuationInput),
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "AgentX Management",
        version = "1",
        description = "Complete public management projection. Ordinary v1 JSON. Requests require the selected listener authority in Host; Origin, when present, must exactly match that authority. No cross-origin permission or arbitrary actor context. Integer widths/signs come from native types: u-int64/int64 must use lossless JSON parsing and exact bigint conversion; int32/u-int32 are safe numbers. Nullable fields preserve null; missing optional input fields retain native serde defaults. Query keys cannot repeat except label; selectors require current_only=true. AgentRef requires exactly one incarnation/stopped_event; semantic checks remain native. Attachment upload/download use raw bytes. Inspector mutations retain route-specific external tags; internal collection/execution variants are excluded."
    ),
    paths(
        super::web::describe,
        super::web::list_agents,
        super::web::get_agent,
        super::web::submit_message,
        super::web::get_operation,
        super::web::list_workspaces,
        super::web::list_workspace_members,
        super::web::list_roles,
        super::web::inspect_directory,
        super::web::get_labels,
        super::web::patch_labels,
        super::web::list_messages,
        super::web::message_history,
        super::web::list_launch_relations,
        super::web::list_office_activity,
        super::web::list_human_mailbox,
        super::web::handle_human_messages,
        super::web::submit_launch,
        super::web::submit_directory_message,
        super::web::stop_agent,
        super::web::kill_agent,
        super::web::stop_participation,
        super::web::fork_agent,
        super::web::resume_agent,
        super::web::request_transcript,
        super::web::inspection_snapshot,
        super::web::configure_inspection,
        super::web::analyze_inspection,
        super::web::cancel_inspection_analysis,
        super::web::request_continuation,
        super::web::upload_attachment,
        super::web::download_attachment
    )
)]
struct ManagementApi;

pub(crate) fn document() -> serde_json::Value {
    let mut document = serde_json::to_value(ManagementApi::openapi()).expect("OpenAPI is serializable");
    document["x-agentx-codec"] = serde_json::json!({
        "wire": "ordinary-json-v1",
        "integerFormats": { "u-int64": "bigint", "int64": "bigint", "u-int32": "number", "int32": "number" },
        "decode": "Preserve numeric tokens before schema-based conversion; never JSON.parse then BigInt, and never convert digit strings globally.",
        "encode": "Emit bigint as unquoted decimal integer tokens only at schema-declared integer fields. Reject fractional/out-of-range values.",
        "identities": "Agent incarnation/stopped_event, generation, request_id, message_id and revisions remain opaque strings."
    });
    document
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::management::contract::*;

    #[test]
    fn native_completeness_openapi_keeps_workspace_query_and_optional_launch_identity() {
        let document = document();
        let route = &document["paths"]["/api/v1/workspaces/members"]["get"];
        assert_eq!(route["operationId"], "managementListWorkspaceMembers");
        let params = route["parameters"].as_array().unwrap();
        assert!(params.iter().any(|parameter| parameter["name"] == "workspace" && parameter["required"] == true));
        let schemas = &document["components"]["schemas"];
        let launched = &schemas["LaunchOutcome"];
        assert!(launched["properties"]["agent_refs"].is_object());
        assert!(!launched["required"].as_array().unwrap().iter().any(|property| property == "agent_refs"));
        assert!(schemas["Descriptor"]["properties"]["supports_workspace_members"].is_object());
    }

    #[test]
    fn message_history_openapi_keeps_optional_capability_and_required_nullable_cursors() {
        let document = document();
        let path = &document["paths"]["/api/v1/agents/{name}/message-history"]["get"];
        assert_eq!(path["operationId"], "managementMessageHistory");
        let parameters = path["parameters"].as_array().unwrap();
        assert!(parameters.iter().any(|parameter| parameter["name"] == "before"));
        assert!(!parameters.iter().any(|parameter| parameter["name"] == "after"));
        let schemas = &document["components"]["schemas"];
        let descriptor = &schemas["Descriptor"];
        assert_eq!(descriptor["properties"]["supports_message_history"]["type"], "boolean");
        assert!(!descriptor["required"].as_array().unwrap().contains(&serde_json::json!("supports_message_history")));
        let history = &schemas["MessageHistoryPage"];
        for cursor in ["next_before", "live_after"] {
            assert!(history["required"].as_array().unwrap().contains(&serde_json::json!(cursor)));
            assert_eq!(history["properties"][cursor]["type"], serde_json::json!(["string", "null"]));
        }
    }

    #[test]
    fn native_wire_and_schema_preserve_precision_tags_null_and_omission() {
        let document = document();
        let schemas = &document["components"]["schemas"];
        assert_eq!(schemas["AgentView"]["properties"]["pending_messages"]["format"], "u-int64");
        assert_eq!(schemas["RequestError"]["properties"]["error_since"]["format"], "int64");
        assert_eq!(schemas["Descriptor"]["properties"]["max_page"]["format"], "u-int32");
        assert_eq!(schemas["SubmitMessage"]["additionalProperties"], false);
        assert_eq!(schemas["MessageInput"]["properties"]["intent"]["type"], serde_json::json!(["string", "null"]));
        let error = crate::agents::model_request::RequestError { turn_id: "opaque".into(), source: "fixture".into(), reason: "fixture".into(), error_since: i64::MIN, last_error_at: i64::MAX, consecutive_failures: u64::MAX };
        let text = serde_json::to_string(&error).unwrap();
        assert!(text.contains("18446744073709551615"));
        assert!(text.contains("-9223372036854775808"));
        let decoded: crate::agents::model_request::RequestError = serde_json::from_str(&text).unwrap();
        assert_eq!(decoded, error);
        let input: MessageInput = serde_json::from_str(r#"{"text":"hello"}"#).unwrap();
        assert!(input.attachments.is_empty());
        assert!(input.intent.is_none());
        assert!(serde_json::from_str::<MessageInput>(r#"{"text":"hello","actor":"forged"}"#).is_err());
        let operation = Operation {
            request_id: "original".into(),
            generation: "original-generation".into(),
            status: OperationStatus::Succeeded,
            result: Some(CommandResult::Sent(SendOutcome { message_id: "18446744073709551615".into(), recipients: vec!["exact".into()] })),
            error: None,
        };
        let value = serde_json::to_value(operation).unwrap();
        assert_eq!(value["result"]["Sent"]["message_id"], "18446744073709551615");
        assert!(value["error"].is_null());
        assert_eq!(schemas["CommandResult"]["oneOf"].as_array().unwrap().len(), 6);
        for private in ["WorkerInput", "WorkGrant", "InvocationContext", "Request", "Response"] {
            assert!(schemas.get(private).is_none(), "private type leaked: {private}");
        }
    }
}
