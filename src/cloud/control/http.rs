//! Admin and scope routes differ in bearer authority and share the same state transitions.
use super::{store, types::*};
use crate::cloud::{
    Deployment, ScopeContext,
    admin::{AdminError, scope_context},
    communication::{fence, path, payload, store::Error, types::ScopeError},
};
use axum::{
    Extension, Json, Router,
    extract::{
        Path, Query, State,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::HeaderMap,
    routing::{get, post, put},
};
use rusqlite::Connection;
use serde::Deserialize;
use std::time::Duration;
use tokio::{sync::Semaphore, time::Instant};
use utoipa::{OpenApi, openapi};

// Deployments admit 128 total calls; inbox may wait on 64. Preserve 32 slots for short requests.
static WAIT_SLOTS: Semaphore = Semaphore::const_new(32);
const MAX_WAIT_MS: u32 = 10_000;

pub(in crate::cloud) fn admin_router() -> Router<Deployment> {
    Router::new().route("/scopes/{id}/hosts/{host}/capabilities", get(admin_capabilities)).route("/scopes/{id}/operations", post(admin_create)).route("/scopes/{id}/operations/{request_id}", get(admin_get))
}

pub(in crate::cloud) fn scope_router() -> Router {
    Router::new().route("/hosts/{host}/control/lease", put(renew_lease)).route("/hosts/{host}/control/poll", post(poll_operation)).route("/hosts/{host}/control/operations/{request_id}/report", post(report_operation))
}

#[derive(Deserialize)]
struct AdminPath {
    id: String,
}
#[derive(Deserialize)]
struct AdminHostPath {
    id: String,
    host: String,
}
#[derive(Deserialize)]
struct AdminOperationPath {
    id: String,
    request_id: String,
}
#[derive(Deserialize)]
struct HostPath {
    host: String,
}
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct LeaseQuery {
    /// Advertises native support for scoped working-directory launch.
    directory_launch: Option<bool>,
}
#[derive(Deserialize)]
struct ReportPath {
    host: String,
    request_id: String,
}

fn header_items<'a>(headers: &'a HeaderMap, name: &str) -> impl Iterator<Item = &'a str> {
    headers.get_all(name).iter().filter_map(|value| value.to_str().ok()).flat_map(|value| value.split(',').map(str::trim)).filter(|item| !item.is_empty())
}

fn lease_features(headers: &HeaderMap, query: LeaseQuery) -> store::LeaseFeatures {
    let workspace_management = header_items(headers, LEASE_FEATURES_HEADER).any(|item| item == WORKSPACE_MANAGEMENT_FEATURE);
    let managed_parents = if workspace_management { header_items(headers, WORKSPACE_PARENTS_HEADER).map(str::to_owned).collect() } else { Default::default() };
    store::LeaseFeatures { directory_launch: query.directory_launch.unwrap_or_default(), workspace_management, managed_parents }
}

#[derive(Clone, Copy)]
enum Access {
    AdminRead,
    AdminSubmit,
    ScopeWrite,
    ScopeRenew,
    ScopeReport,
}

async fn run<T: Send + 'static>(scope: ScopeContext, access: Access, operation: impl Fn(&Connection, i64) -> Result<T, Error> + Send + 'static) -> Result<T, Error> {
    let scope_bearer = matches!(access, Access::ScopeWrite | Access::ScopeRenew | Access::ScopeReport);
    let active = scope.clone();
    let result = tokio::task::spawn_blocking(move || {
        let output = store::transaction(&scope.root, &scope.generation, !matches!(access, Access::AdminRead), || !scope_bearer || scope.valid(), operation)?;
        // Publish after commit even when the HTTP caller has already disconnected.
        if matches!(access, Access::AdminSubmit | Access::ScopeRenew) {
            scope.control_changed.send_replace(());
        }
        if matches!(access, Access::ScopeReport) {
            scope.space_changed.send_modify(|snapshot| snapshot.roster = snapshot.roster.wrapping_add(1));
            scope.current.send_modify(|_| {});
        }
        Ok::<_, Error>(output)
    })
    .await
    .map_err(|_| Error::Worker)??;
    if scope_bearer && !active.valid() {
        return Err(Error::Credential);
    }
    Ok(result)
}

#[utoipa::path(get, path="/api/v1/admin/scopes/{id}/hosts/{host}/capabilities", operation_id="getAdminHostCapabilities",
    params(("id"=String,Path),("host"=String,Path)), responses((status=200,body=HostCapabilities)))]
async fn admin_capabilities(State(state): State<Deployment>, parts: Result<Path<AdminHostPath>, PathRejection>) -> Result<Json<HostCapabilities>, AdminError> {
    let AdminHostPath { id, host } = path(parts).map_err(AdminError::Scope)?;
    let scope = scope_context(state, id).await?;
    let generation = scope.generation.clone();
    run(scope, Access::AdminRead, move |db, now| store::capabilities(db, now, generation.clone(), host.clone())).await.map(Json).map_err(AdminError::Scope)
}

#[utoipa::path(post, path="/api/v1/admin/scopes/{id}/operations", operation_id="createAdminControlOperation",
    params(("id"=String,Path)), request_body=CreateControlOperation, responses((status=200,body=ControlReceipt)))]
async fn admin_create(State(state): State<Deployment>, parts: Result<Path<AdminPath>, PathRejection>, input: Result<Json<CreateControlOperation>, JsonRejection>) -> Result<Json<ControlReceipt>, AdminError> {
    let AdminPath { id } = path(parts).map_err(AdminError::Scope)?;
    let input = payload(input).map_err(AdminError::Scope)?;
    let scope = scope_context(state, id).await?;
    fence(&scope, &input.generation).map_err(AdminError::Scope)?;
    run(scope, Access::AdminSubmit, move |db, now| store::create(db, now, &input)).await.map(Json).map_err(AdminError::Scope)
}

#[utoipa::path(get, path="/api/v1/admin/scopes/{id}/operations/{request_id}", operation_id="getAdminControlOperation",
    params(("id"=String,Path),("request_id"=String,Path)), responses((status=200,body=ControlReceipt)))]
async fn admin_get(State(state): State<Deployment>, parts: Result<Path<AdminOperationPath>, PathRejection>) -> Result<Json<ControlReceipt>, AdminError> {
    let AdminOperationPath { id, request_id } = path(parts).map_err(AdminError::Scope)?;
    let scope = scope_context(state, id).await?;
    let generation = scope.generation.clone();
    run(scope, Access::AdminRead, move |db, now| store::get(db, now, &generation, &request_id)).await.map(Json).map_err(AdminError::Scope)
}

#[utoipa::path(put, path="/hosts/{host}/control/lease", operation_id="scopeRenewControlLease",
    params(("host"=String,Path), LeaseQuery,
        ("X-AgentX-Features"=Option<String>,Header,description="Comma-separated supported features. Advertise workspace_management to accept remote workspace registration."),
        ("X-AgentX-Workspace-Parents"=Option<String>,Header,description="Comma-separated aliases of configured parent exports present in the lease catalogue; used with workspace_management.")),
    request_body=RenewControlLease, security(("scopeBearer"=[])),
    responses((status=200,body=HostCapabilities),(status="default",body=ScopeError)))]
async fn renew_lease(
    Extension(scope): Extension<ScopeContext>,
    parts: Result<Path<HostPath>, PathRejection>,
    query: Result<Query<LeaseQuery>, QueryRejection>,
    headers: HeaderMap,
    input: Result<Json<RenewControlLease>, JsonRejection>,
) -> Result<Json<HostCapabilities>, Error> {
    let HostPath { host } = path(parts)?;
    let Query(query) = query.map_err(|_| Error::Invalid("invalid control lease features"))?;
    let features = lease_features(&headers, query);
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    run(scope, Access::ScopeRenew, move |db, now| store::renew(db, now, host.clone(), &input, &features)).await.map(Json)
}

#[utoipa::path(post, path="/hosts/{host}/control/poll", operation_id="scopePollControlOperation",
    params(("host"=String,Path)), request_body=PollControlOperation, security(("scopeBearer"=[])),
    responses((status=200,body=ControlPoll),(status="default",body=ScopeError)))]
async fn poll_operation(Extension(scope): Extension<ScopeContext>, parts: Result<Path<HostPath>, PathRejection>, input: Result<Json<PollControlOperation>, JsonRejection>) -> Result<Json<ControlPoll>, Error> {
    let HostPath { host } = path(parts)?;
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    let wait = input.wait_ms.unwrap_or_default();
    if wait > MAX_WAIT_MS {
        return Err(Error::Invalid("wait_ms exceeds the maximum control poll wait"));
    }
    // Subscribe before the first durable check so a racing submission cannot miss its wake.
    let mut updates = scope.control_changed.subscribe();
    let mut credential = scope.current.subscribe();
    let deadline = Instant::now() + Duration::from_millis(wait.into());
    let mut slot = None;
    loop {
        let (host, input) = (host.clone(), input.clone());
        let result = run(scope.clone(), Access::ScopeWrite, move |db, now| store::poll(db, now, &host, &input)).await?;
        if result.operation.is_some() || Instant::now() >= deadline {
            return Ok(Json(result));
        }
        if slot.is_none() {
            slot = Some(WAIT_SLOTS.try_acquire().map_err(|_| Error::WaitCapacity)?);
        }
        tokio::select! {
            _ = credential.wait_for(|version| *version != scope.credential_version) => return Err(Error::Credential),
            _ = updates.changed() => {},
            _ = tokio::time::sleep_until(deadline) => {},
        }
    }
}

#[utoipa::path(post, path="/hosts/{host}/control/operations/{request_id}/report", operation_id="scopeReportControlOperation",
    params(("host"=String,Path),("request_id"=String,Path)), request_body=ReportControlOperation, security(("scopeBearer"=[])),
    responses((status=200,body=ControlReceipt),(status="default",body=ScopeError)))]
async fn report_operation(Extension(scope): Extension<ScopeContext>, parts: Result<Path<ReportPath>, PathRejection>, input: Result<Json<ReportControlOperation>, JsonRejection>) -> Result<Json<ControlReceipt>, Error> {
    let ReportPath { host, request_id } = path(parts)?;
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    run(scope, Access::ScopeReport, move |db, now| store::report(db, now, &host, &request_id, &input)).await.map(Json)
}

pub(in crate::cloud) fn admin_document() -> openapi::OpenApi {
    #[derive(OpenApi)]
    #[openapi(paths(admin_capabilities, admin_create, admin_get))]
    struct ControlAdminApi;
    ControlAdminApi::openapi()
}

pub(in crate::cloud) fn scope_document() -> openapi::OpenApi {
    #[derive(OpenApi)]
    #[openapi(paths(renew_lease, poll_operation, report_operation))]
    struct ControlScopeApi;
    ControlScopeApi::openapi()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn lease_features_are_optional_and_independently_negotiated() {
        let mut headers = HeaderMap::new();
        headers.insert(WORKSPACE_PARENTS_HEADER, HeaderValue::from_static("main, east"));
        let legacy = lease_features(&headers, LeaseQuery { directory_launch: Some(true) });
        assert!(legacy.directory_launch);
        assert!(!legacy.workspace_management);
        assert!(legacy.managed_parents.is_empty());

        headers.append(LEASE_FEATURES_HEADER, HeaderValue::from_static("future_workspace_management"));
        headers.append(LEASE_FEATURES_HEADER, HeaderValue::from_static("another_feature, workspace_management"));
        let negotiated = lease_features(&headers, LeaseQuery { directory_launch: None });
        assert!(!negotiated.directory_launch);
        assert!(negotiated.workspace_management);
        assert_eq!(negotiated.managed_parents, ["east".into(), "main".into()].into());
    }

    #[test]
    fn cloud_openapi_exposes_compatible_workspace_negotiation_and_receipts() {
        let scope = serde_json::to_value(scope_document()).unwrap();
        let admin = serde_json::to_value(admin_document()).unwrap();
        let params = scope["paths"]["/hosts/{host}/control/lease"]["put"]["parameters"].as_array().unwrap();
        for header in [LEASE_FEATURES_HEADER, WORKSPACE_PARENTS_HEADER] {
            assert!(params.iter().any(|p| p["name"] == header && p["in"] == "header" && p["required"] == false));
        }
        for name in ["ControlCommand", "ControlOutcome", "HostCapabilities", "WorkspaceCapability"] {
            let schemas = &scope["components"]["schemas"];
            assert!(schemas[name].is_object());
            assert_eq!(schemas[name], admin["components"]["schemas"][name]);
        }
        let schemas = &scope["components"]["schemas"];
        let add = schemas["ControlCommand"]["oneOf"].as_array().unwrap().iter().find(|variant| variant["properties"]["kind"]["enum"][0] == "add_workspace").unwrap();
        assert_eq!(add["required"].as_array().unwrap().len(), 5);
        assert!(["kind", "parent", "directory", "alias", "create_directory"].iter().all(|name| add["required"].as_array().unwrap().contains(&serde_json::json!(name))));
        let added = schemas["ControlOutcome"]["oneOf"].as_array().unwrap().iter().find(|variant| variant["properties"]["status"]["enum"][0] == "workspace_added").unwrap();
        assert!(added["required"].as_array().unwrap().contains(&serde_json::json!("alias")));
        assert_eq!(schemas["WorkspaceCapability"]["properties"]["managed_parent"]["readOnly"], true);
    }
}
