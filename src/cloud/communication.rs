//! Authenticated Namespace communication. Deployment owns credentials and supplies a fenced context.
pub(super) mod store;
pub(super) mod types;

pub(super) use self::types::{AgentView, HostView};
use self::{store::Error, types::*};
use super::{
    ScopeContext,
    changes::{ChangeQuery, SpaceChanges},
};
use axum::{
    Extension, Json, Router,
    extract::{
        DefaultBodyLimit, Path as AxumPath, Query,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use std::{path::Path, time::Duration};
use utoipa::OpenApi;

// Leave capacity for sends and credential rotations while clients hold open claims.
static WAIT_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(64);

pub(super) fn router() -> Router {
    Router::new()
        .route("/changes", get(namespace_changes))
        .route("/descriptor", get(describe))
        .route("/hosts", get(list_hosts))
        .route("/hosts/{host}", put(renew_host))
        .route("/agents", get(list_agents))
        .route("/agents/{name}", put(register_agent))
        .route("/agents/{name}/retire", post(retire_agent))
        .route("/transcript", get(list_transcript).post(publish_transcript))
        .route("/messages", get(list_messages).post(send_message))
        .route("/operations/{request_id}", get(get_operation))
        .route("/inbox/claim", post(claim_inbox))
        .route("/inbox/confirm", post(confirm_inbox))
        .route("/inbox/recover", post(recover_inbox))
        .merge(super::control::namespace_router())
        .fallback(super::unknown_cloud_operation)
        .method_not_allowed_fallback(super::unsupported_cloud_method)
        .layer(DefaultBodyLimit::max(512 * 1024))
}

pub(super) fn initialize(root: &Path, generation: &str) -> anyhow::Result<()> {
    store::initialize(root, generation)
}

pub(super) fn summary(root: &Path, generation: &str) -> anyhow::Result<(u32, u32, u32)> {
    Ok(store::transaction(root, generation, false, || true, store::counts)?)
}

pub(super) fn metadata_failure(error: anyhow::Error) -> super::Failure {
    if let Some(store::Error::Invalid(reason)) = error.downcast_ref::<store::Error>() {
        return super::Failure::invalid(reason);
    }
    super::Failure(StatusCode::SERVICE_UNAVAILABLE, "namespace_unavailable", "Namespace inventory unavailable")
}

fn failure(status: StatusCode, code: &'static str, message: impl Into<String>) -> Response {
    (status, Json(NamespaceError { code, message: message.into() })).into_response()
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::SchemaVersion => failure(StatusCode::SERVICE_UNAVAILABLE, "namespace_unavailable", self.to_string()),
            Self::Invalid(message) => failure(StatusCode::BAD_REQUEST, "invalid_input", message),
            Self::InvalidMessage(message) => failure(StatusCode::BAD_REQUEST, "invalid_input", message),
            Self::NotFound(message) => failure(StatusCode::NOT_FOUND, "not_found", message),
            Self::Conflict(message) => failure(StatusCode::CONFLICT, "conflict", message),
            Self::ClaimOutstanding => failure(StatusCode::CONFLICT, "claim_conflict", "agent already has an outstanding inbox claim; recover that claim ID"),
            Self::Generation => failure(StatusCode::CONFLICT, "stale_generation", "Namespace generation has changed"),
            Self::Credential => failure(StatusCode::UNAUTHORIZED, "unauthorized", "Namespace credential has changed"),
            Self::Offline => failure(StatusCode::CONFLICT, "host_offline", "host is offline; renew its heartbeat"),
            Self::Installation => failure(StatusCode::CONFLICT, "host_conflict", "host name belongs to another installation"),
            Self::Agent => failure(StatusCode::CONFLICT, "stale_agent", "exact agent registration is no longer current"),
            Self::Lease => failure(StatusCode::CONFLICT, "stale_lease", "exact daemon lease is no longer current"),
            Self::Capability => failure(StatusCode::CONFLICT, "capability_unavailable", "requested workspace or role is unavailable"),
            Self::AgentWorkspace => failure(StatusCode::CONFLICT, "agent_workspace_mismatch", self.to_string()),
            Self::ControlClaim => failure(StatusCode::CONFLICT, "control_claim_conflict", "report does not match the exact control claim"),
            Self::ControlOutcome => failure(StatusCode::CONFLICT, "control_outcome_conflict", "control claim already has a different terminal outcome"),
            Self::PayloadTooLarge => failure(StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large", "request body exceeds the Namespace size limit"),
            Self::WaitCapacity => failure(StatusCode::SERVICE_UNAVAILABLE, "unavailable", "all Namespace wait slots are occupied"),
            Self::Sql(error) if matches!(error.sqlite_error_code(), Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)) => failure(StatusCode::SERVICE_UNAVAILABLE, "unavailable", "Namespace storage is temporarily busy"),
            Self::Sql(_) | Self::Json(_) | Self::Worker => failure(StatusCode::INTERNAL_SERVER_ERROR, "internal", "Namespace operation failed"),
        }
    }
}

pub(super) fn payload<T>(input: Result<Json<T>, JsonRejection>) -> store::Result<T> {
    input.map(|Json(value)| value).map_err(|error| if error.status() == StatusCode::PAYLOAD_TOO_LARGE { Error::PayloadTooLarge } else { Error::Invalid("request body is not valid for this endpoint") })
}

pub(super) fn query<T>(input: Result<Query<T>, QueryRejection>) -> store::Result<T> {
    input.map(|Query(value)| value).map_err(|_| Error::Invalid("request query is not valid for this endpoint"))
}

pub(super) fn path<T>(input: Result<AxumPath<T>, PathRejection>) -> store::Result<T> {
    input.map(|AxumPath(value)| value).map_err(|_| Error::Invalid("request path is not valid for this endpoint"))
}

// The mounted path also captures the parent Namespace ID. Named extraction ignores that capture.
#[derive(serde::Deserialize)]
struct HostPath {
    host: String,
}
#[derive(serde::Deserialize)]
struct AgentPath {
    name: String,
}
#[derive(serde::Deserialize)]
struct OperationPath {
    request_id: String,
}

#[derive(Clone, Copy)]
enum Access {
    Read,
    Write,
    WriteAndWake,
    Roster,
    Retire,
    AdminRead,
    AdminWriteAndWake,
}

async fn run<T: Send + 'static>(scope: ScopeContext, access: Access, action: impl FnOnce(&rusqlite::Connection, i64) -> store::Result<T> + Send + 'static) -> store::Result<T> {
    run_committed(scope, access, action, |_, _| {}).await
}

async fn run_committed<T: Send + 'static>(scope: ScopeContext, access: Access, action: impl FnOnce(&rusqlite::Connection, i64) -> store::Result<T> + Send + 'static, committed: impl FnOnce(&ScopeContext, &T) + Send + 'static) -> store::Result<T> {
    let scope_bearer = !matches!(access, Access::AdminRead | Access::AdminWriteAndWake);
    let write = !matches!(access, Access::Read | Access::AdminRead);
    let active = scope.clone();
    let result = tokio::task::spawn_blocking(move || {
        let result = store::transaction(&scope.root, &scope.generation, write, || !scope_bearer || scope.valid(), action)?;
        // Publish only after commit, even if the HTTP caller disconnected during dispatch.
        if matches!(access, Access::Roster | Access::Retire) {
            scope.space_changed.send_modify(|value| {
                value.roster = value.roster.wrapping_add(1);
                if matches!(access, Access::Retire) {
                    value.transcript = value.transcript.wrapping_add(1);
                }
            });
        }
        if matches!(access, Access::WriteAndWake | Access::AdminWriteAndWake) {
            scope.space_changed.send_modify(|value| value.messages = value.messages.wrapping_add(1));
        }
        if matches!(access, Access::WriteAndWake | Access::AdminWriteAndWake | Access::Retire) {
            scope.current.send_modify(|_| {});
        }
        committed(&scope, &result);
        Ok::<_, Error>(result)
    })
    .await
    .map_err(|_| Error::Worker)??;
    if scope_bearer && !active.valid() {
        return Err(Error::Credential);
    }
    Ok(result)
}

pub(super) fn fence(scope: &ScopeContext, generation: &str) -> store::Result<()> {
    if scope.generation != generation {
        return Err(Error::Generation);
    }
    Ok(())
}

async fn message_history(scope: ScopeContext, access: Access, input: MessageHistoryQuery) -> store::Result<MessageHistoryPage> {
    fence(&scope, &input.generation)?;
    run(scope, access, move |conn, _| store::list_messages(conn, input)).await
}

pub(super) async fn admin_message_history(scope: ScopeContext, input: MessageHistoryQuery) -> store::Result<MessageHistoryPage> {
    message_history(scope, Access::AdminRead, input).await
}

pub(super) async fn admin_send_human_message(scope: ScopeContext, input: SendHumanMessage) -> store::Result<MessageView> {
    fence(&scope, &input.generation)?;
    run(scope, Access::AdminWriteAndWake, move |conn, now| store::send_human_message(conn, now, input)).await
}

pub(super) async fn space_changes(scope: ScopeContext, query: ChangeQuery, member: bool) -> store::Result<SpaceChanges> {
    super::control::observe_workspace_availability(&scope);
    super::changes::wait(scope, query, member).await
}

#[utoipa::path(get, path="/changes", operation_id="namespaceSpaceChanges", params(ChangeQuery), security(("namespaceBearer"=[])), responses((status=200, body=SpaceChanges),(status="default", body=NamespaceError)))]
async fn namespace_changes(Extension(scope): Extension<ScopeContext>, input: Result<Query<ChangeQuery>, QueryRejection>) -> store::Result<Json<SpaceChanges>> {
    space_changes(scope, query(input)?, true).await.map(Json)
}

#[utoipa::path(get, path = "/descriptor", operation_id = "namespaceDescribe", security(("namespaceBearer" = [])), responses(
    (status = 200, body = Descriptor),
    (status = "default", body = NamespaceError)
))]
async fn describe(Extension(scope): Extension<ScopeContext>) -> store::Result<Json<Descriptor>> {
    let descriptor = Descriptor { namespace_id: scope.id.clone(), generation: scope.generation.clone(), protocol_version: super::PROTOCOL_VERSION };
    run(scope, Access::Read, move |_, _| Ok(descriptor)).await.map(Json)
}

pub(super) async fn host_page(scope: ScopeContext, member: bool, mut input: HostQuery) -> store::Result<HostPage> {
    fence_page(&scope, input.generation.as_deref())?;
    input.after = input.after.filter(|cursor| !cursor.is_empty());
    let generation = scope.generation.clone();
    let access = if member { Access::Read } else { Access::AdminRead };
    run(scope, access, move |conn, now| store::hosts(conn, now, generation, input)).await
}

pub(super) async fn agent_page(scope: ScopeContext, member: bool, mut input: AgentQuery) -> store::Result<AgentPage> {
    fence_page(&scope, input.generation.as_deref())?;
    input.after = input.after.filter(|cursor| !cursor.is_empty());
    let generation = scope.generation.clone();
    let access = if member { Access::Read } else { Access::AdminRead };
    run(scope, access, move |conn, now| store::agents(conn, now, generation, input)).await
}

/// For existing v2 callers, an unbound read reports the generation it actually observed.
pub(super) fn fence_page(scope: &ScopeContext, expected: Option<&str>) -> store::Result<()> {
    expected.map_or(Ok(()), |expected| fence(scope, expected))
}

#[utoipa::path(get, path = "/hosts", operation_id = "namespaceListHosts", params(HostQuery), security(("namespaceBearer" = [])), responses(
    (status = 200, body = HostPage),
    (status = 409, description = "Stale Namespace generation", body = NamespaceError),
    (status = "default", body = NamespaceError)
))]
async fn list_hosts(Extension(scope): Extension<ScopeContext>, input: Result<Query<HostQuery>, QueryRejection>) -> store::Result<Json<HostPage>> {
    host_page(scope, true, query(input)?).await.map(Json)
}

#[utoipa::path(get, path = "/agents", operation_id = "namespaceListAgents", params(AgentQuery), security(("namespaceBearer" = [])), responses(
    (status = 200, body = AgentPage),
    (status = 409, description = "Stale Namespace generation", body = NamespaceError),
    (status = "default", body = NamespaceError)
))]
async fn list_agents(Extension(scope): Extension<ScopeContext>, input: Result<Query<AgentQuery>, QueryRejection>) -> store::Result<Json<AgentPage>> {
    agent_page(scope, true, query(input)?).await.map(Json)
}

#[utoipa::path(put, path = "/hosts/{host}", operation_id = "namespaceRenewHost", params(("host" = String, Path)), request_body = HostRegistration, security(("namespaceBearer" = [])), responses(
    (status = 200, body = HostView),
    (status = "default", body = NamespaceError)
))]
async fn renew_host(Extension(scope): Extension<ScopeContext>, parts: Result<AxumPath<HostPath>, PathRejection>, input: Result<Json<HostRegistration>, JsonRejection>) -> store::Result<Json<HostView>> {
    let HostPath { host } = path(parts)?;
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    run(scope, Access::Write, move |conn, now| store::renew_host(conn, now, host, input)).await.map(Json)
}

#[utoipa::path(put, path = "/agents/{name}", operation_id = "namespaceRegisterAgent", params(("name" = String, Path)), request_body = AgentRegistration, security(("namespaceBearer" = [])), responses(
    (status = 200, body = AgentView),
    (status = "default", body = NamespaceError)
))]
async fn register_agent(Extension(scope): Extension<ScopeContext>, parts: Result<AxumPath<AgentPath>, PathRejection>, input: Result<Json<AgentRegistration>, JsonRejection>) -> store::Result<Json<AgentView>> {
    let AgentPath { name } = path(parts)?;
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    run(scope, Access::Roster, move |conn, now| store::register_agent(conn, now, name, input)).await.map(Json)
}

#[utoipa::path(post, path = "/agents/{name}/retire", operation_id = "namespaceRetireAgent", params(("name" = String, Path)), request_body = AgentRegistration, security(("namespaceBearer" = [])), responses(
    (status = 200, body = AgentView),
    (status = "default", body = NamespaceError)
))]
async fn retire_agent(Extension(scope): Extension<ScopeContext>, parts: Result<AxumPath<AgentPath>, PathRejection>, input: Result<Json<AgentRegistration>, JsonRejection>) -> store::Result<Json<AgentView>> {
    let AgentPath { name } = path(parts)?;
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    run(scope, Access::Retire, move |conn, now| store::retire_agent(conn, now, name, input)).await.map(Json)
}

async fn transcript_history(scope: ScopeContext, access: Access, input: TranscriptHistoryQuery) -> store::Result<TranscriptHistoryPage> {
    fence(&scope, &input.generation)?;
    run(scope, access, move |conn, now| store::transcript::history(conn, now, input)).await
}

pub(super) async fn admin_transcript_history(scope: ScopeContext, input: TranscriptHistoryQuery) -> store::Result<TranscriptHistoryPage> {
    transcript_history(scope, Access::AdminRead, input).await
}

#[utoipa::path(get, path = "/transcript", operation_id = "namespaceListTranscript", params(TranscriptHistoryQuery), security(("namespaceBearer" = [])), responses(
    (status = 200, body = TranscriptHistoryPage),
    (status = "default", body = NamespaceError)
))]
async fn list_transcript(Extension(scope): Extension<ScopeContext>, input: Result<Query<TranscriptHistoryQuery>, QueryRejection>) -> store::Result<Json<TranscriptHistoryPage>> {
    transcript_history(scope, Access::Read, query(input)?).await.map(Json)
}

#[utoipa::path(post, path = "/transcript", operation_id = "namespacePublishTranscript", request_body = PublishTranscript, security(("namespaceBearer" = [])), responses(
    (status = 200, body = bool),
    (status = "default", body = NamespaceError)
))]
async fn publish_transcript(Extension(scope): Extension<ScopeContext>, input: Result<Json<PublishTranscript>, JsonRejection>) -> store::Result<Json<bool>> {
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    run_committed(
        scope,
        Access::Write,
        move |conn, now| store::transcript::publish(conn, now, input),
        |scope, (transcript, facts)| {
            if *transcript {
                scope.space_changed.send_modify(|value| {
                    value.transcript = value.transcript.wrapping_add(1);
                    if *facts {
                        value.roster = value.roster.wrapping_add(1);
                    }
                });
            }
        },
    )
    .await
    .map(|(changed, _)| Json(changed))
}

#[utoipa::path(get, path = "/messages", operation_id = "namespaceListMessages", params(MessageHistoryQuery), security(("namespaceBearer" = [])), responses(
    (status = 200, body = MessageHistoryPage),
    (status = "default", body = NamespaceError)
))]
async fn list_messages(Extension(scope): Extension<ScopeContext>, input: Result<Query<MessageHistoryQuery>, QueryRejection>) -> store::Result<Json<MessageHistoryPage>> {
    message_history(scope, Access::Read, query(input)?).await.map(Json)
}

#[utoipa::path(post, path = "/messages", operation_id = "namespaceSendMessage", request_body = SendMessage, security(("namespaceBearer" = [])), responses(
    (status = 200, body = MessageView),
    (status = "default", body = NamespaceError)
))]
async fn send_message(Extension(scope): Extension<ScopeContext>, input: Result<Json<SendMessage>, JsonRejection>) -> store::Result<Json<MessageView>> {
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    run(scope, Access::WriteAndWake, move |conn, now| store::send(conn, now, input)).await.map(Json)
}

#[utoipa::path(get, path = "/operations/{request_id}", operation_id = "namespaceGetOperation", params(("request_id" = String, Path)), security(("namespaceBearer" = [])), responses(
    (status = 200, body = MessageOperation),
    (status = "default", body = NamespaceError)
))]
async fn get_operation(Extension(scope): Extension<ScopeContext>, parts: Result<AxumPath<OperationPath>, PathRejection>) -> store::Result<Json<MessageOperation>> {
    let OperationPath { request_id } = path(parts)?;
    run(scope, Access::Read, move |conn, _| store::operation(conn, request_id)).await.map(Json)
}

#[utoipa::path(post, path = "/inbox/claim", operation_id = "namespaceClaimInbox", request_body = ClaimInbox, security(("namespaceBearer" = [])), responses(
    (status = 200, body = InboxBatch),
    (status = "default", body = NamespaceError)
))]
async fn claim_inbox(Extension(scope): Extension<ScopeContext>, input: Result<Json<ClaimInbox>, JsonRejection>) -> store::Result<Json<InboxBatch>> {
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    let wait = input.wait_ms.unwrap_or(0);
    if wait > MAX_WAIT_MS {
        return Err(Error::Invalid("claim wait is capped at 20 seconds"));
    }
    let _slot = if wait == 0 { None } else { Some(WAIT_SLOTS.try_acquire().map_err(|_| Error::WaitCapacity)?) };
    let deadline = tokio::time::Instant::now() + Duration::from_millis(u64::from(wait));
    // Subscribe before reading durable state to close the read-to-wait race.
    let mut changes = scope.current.subscribe();
    let input = std::sync::Arc::new(input);
    loop {
        let request = input.clone();
        let (batch, reusable) = run(scope.clone(), Access::Write, move |conn, now| store::claim(conn, now, &request)).await?;
        let time_left = deadline.saturating_duration_since(tokio::time::Instant::now());
        if !reusable || time_left.is_zero() {
            return Ok(Json(batch));
        }
        // No database guard or blocking worker remains held between attempts.
        if tokio::time::timeout_at(deadline, changes.changed()).await.is_err() {
            if !scope.valid() {
                return Err(Error::Credential);
            }
            return Ok(Json(batch));
        }
    }
}

#[utoipa::path(post, path = "/inbox/confirm", operation_id = "namespaceConfirmInbox", request_body = ConfirmInbox, security(("namespaceBearer" = [])), responses(
    (status = 200, body = InboxConfirmation),
    (status = "default", body = NamespaceError)
))]
async fn confirm_inbox(Extension(scope): Extension<ScopeContext>, input: Result<Json<ConfirmInbox>, JsonRejection>) -> store::Result<Json<InboxConfirmation>> {
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    run(scope, Access::Write, move |conn, now| store::confirm(conn, now, input)).await.map(Json)
}

#[utoipa::path(post, path = "/inbox/recover", operation_id = "namespaceRecoverInbox", request_body = RecoverInbox, security(("namespaceBearer" = [])), responses(
    (status = 200, body = InboxBatch),
    (status = "default", body = NamespaceError)
))]
async fn recover_inbox(Extension(scope): Extension<ScopeContext>, input: Result<Json<RecoverInbox>, JsonRejection>) -> store::Result<Json<InboxBatch>> {
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    run(scope, if input.release { Access::WriteAndWake } else { Access::Read }, move |conn, now| store::recover(conn, now, input)).await.map(Json)
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "AgentX Namespace Communication",
        version = "1",
        description = "Base: configured Namespace URL plus /api/v1. All calls require the Namespace Bearer. Delivery is confirmed by the exact recipient host after Provider evidence. Opaque identifiers and decimal string message IDs remain strings."
    ),
    paths(namespace_changes, describe, list_hosts, list_agents, renew_host, register_agent, retire_agent, list_transcript, publish_transcript, list_messages, send_message, get_operation, claim_inbox, confirm_inbox, recover_inbox)
)]
struct NamespaceApi;

pub(super) fn document() -> serde_json::Value {
    let mut api = NamespaceApi::openapi();
    api.merge(super::control::namespace_document());
    let mut document = serde_json::to_value(api).expect("Namespace OpenAPI is serializable");
    document["components"]["securitySchemes"]["namespaceBearer"] = serde_json::json!({ "type": "http", "scheme": "bearer" });
    document["x-agentx-codec"] = serde_json::json!({
        "wire": "ordinary-json-v1",
        "integerFormats": { "int64": "bigint", "u-int32": "number" },
        "identities": "Namespace generation, participant incarnation, request_id, claim_id and message IDs remain strings."
    });
    document
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emitted_wire_keeps_relative_paths_bearer_security_and_exact_ids() {
        let spec = document();
        let paths = spec["paths"].as_object().unwrap();
        assert!(paths.keys().all(|path| !path.starts_with("/scopes")));
        let claim = &paths["/inbox/claim"]["post"];
        assert_eq!(claim["security"], serde_json::json!([{ "namespaceBearer": [] }]));
        assert_eq!(paths["/messages"]["get"]["security"], claim["security"]);
        assert_eq!(paths["/messages"]["get"]["operationId"], "namespaceListMessages");
        assert_eq!(paths["/transcript"]["get"]["operationId"], "namespaceListTranscript");
        assert_eq!(paths["/transcript"]["get"]["security"], claim["security"]);
        assert_eq!(paths["/transcript"]["post"]["security"], claim["security"]);
        assert_eq!(paths["/hosts/{host}/control/poll"]["post"]["security"], claim["security"]);
        assert!(claim["responses"]["default"].is_object());
        assert_eq!(spec["components"]["securitySchemes"]["namespaceBearer"]["scheme"], "bearer");
        let schemas = &spec["components"]["schemas"];
        assert_eq!(schemas["MessageView"]["properties"]["id"]["type"], "string");
        assert_eq!(schemas["MessageView"]["properties"]["created_at"]["format"], "int64");
        assert_eq!(schemas["InboxBatch"]["properties"]["next"]["type"], serde_json::json!(["string", "null"]));
        assert!(schemas["InboxBatch"]["required"].as_array().unwrap().contains(&serde_json::json!("next")));
    }

    #[test]
    fn admin_openapi_projects_the_same_history_and_exact_human_send() {
        let admin = super::super::admin::document();
        let scope = document();
        let route = &admin["paths"]["/api/v1/admin/namespaces/{id}/messages"];
        let query: Vec<_> = route["get"]["parameters"].as_array().unwrap().iter().filter(|param| param["in"] == "query").cloned().collect();
        assert_eq!(query, *scope["paths"]["/messages"]["get"]["parameters"].as_array().unwrap());
        assert_eq!(admin["security"], serde_json::json!([{ "adminBearer": [] }]));
        assert_eq!(route["get"]["operationId"], "listAdminNamespaceMessages");
        assert_eq!(route["post"]["operationId"], "sendAdminNamespaceMessage");
        assert_eq!(route["get"]["responses"]["200"]["content"]["application/json"]["schema"]["$ref"], "#/components/schemas/MessageHistoryPage");
        assert_eq!(route["post"]["responses"]["200"]["content"]["application/json"]["schema"]["$ref"], "#/components/schemas/MessageView");
        assert_eq!(route["post"]["requestBody"]["content"]["application/json"]["schema"]["$ref"], "#/components/schemas/SendHumanMessage");
        let schemas = &admin["components"]["schemas"];
        for contract in ["AgentView", "MessageHistoryPage", "MessageView"] {
            assert!(schemas[contract].is_object());
            assert_eq!(schemas[contract], scope["components"]["schemas"][contract]);
        }
        assert_eq!(schemas["SendHumanMessage"]["properties"]["to"]["$ref"], "#/components/schemas/AgentView");
        assert_eq!(schemas["SendHumanMessage"]["required"], serde_json::json!(["generation", "request_id", "to", "body"]));
    }
}
