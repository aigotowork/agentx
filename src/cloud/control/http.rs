//! Admin and Namespace routes differ in bearer authority and share the same state transitions.
use super::{store, types::*, workspaces};
use crate::cloud::{
    Deployment, ScopeContext,
    admin::{AdminError, namespace_context},
    communication::{
        fence, fence_page, path, payload, query,
        store::{Error, transaction},
        types::NamespaceError,
    },
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
    Router::new()
        .route("/namespaces/{id}/workspaces", get(admin_workspaces))
        .route("/namespaces/{id}/hosts/{host}/capabilities", get(admin_capabilities))
        .route("/namespaces/{id}/operations", post(admin_create))
        .route("/namespaces/{id}/operations/{request_id}", get(admin_get))
}

pub(in crate::cloud) fn namespace_router() -> Router {
    Router::new()
        .route("/workspaces", get(namespace_workspaces))
        .route("/hosts/{host}/control/lease", put(renew_lease))
        .route("/hosts/{host}/control/poll", post(poll_operation))
        .route("/hosts/{host}/control/operations/{request_id}/report", post(report_operation))
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
    /// Advertises native support for a project-relative launch directory.
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
    store::LeaseFeatures { directory_launch: query.directory_launch.unwrap_or_default(), workspace_management }
}

#[derive(Clone, Copy)]
enum Access {
    AdminRead,
    AdminSubmit,
    NamespaceRead,
    NamespaceWrite,
    NamespaceReport,
}

async fn run<T: Send + 'static>(scope: ScopeContext, access: Access, operation: impl Fn(&Connection, i64) -> Result<T, Error> + Send + 'static) -> Result<T, Error> {
    run_committed(scope, access, operation, |_, _| {}).await
}

async fn run_committed<T: Send + 'static>(scope: ScopeContext, access: Access, operation: impl Fn(&Connection, i64) -> Result<T, Error> + Send + 'static, committed: impl FnOnce(&ScopeContext, &T) + Send + 'static) -> Result<T, Error> {
    let scope_bearer = !matches!(access, Access::AdminRead | Access::AdminSubmit);
    let write = !matches!(access, Access::AdminRead | Access::NamespaceRead);
    let active = scope.clone();
    let result = tokio::task::spawn_blocking(move || {
        let output = transaction(&scope.root, &scope.generation, write, || !scope_bearer || scope.valid(), operation)?;
        // Publish after commit even when the HTTP caller has already disconnected.
        if matches!(access, Access::AdminSubmit) {
            scope.control_changed.send_replace(());
        }
        if matches!(access, Access::NamespaceReport) {
            scope.space_changed.send_modify(|snapshot| snapshot.roster = snapshot.roster.wrapping_add(1));
            scope.current.send_modify(|_| {});
        }
        committed(&scope, &output);
        Ok::<_, Error>(output)
    })
    .await
    .map_err(|_| Error::Worker)??;
    if scope_bearer && !active.valid() {
        return Err(Error::Credential);
    }
    Ok(result)
}

async fn workspace_page(scope: ScopeContext, access: Access, query: WorkspaceQuery) -> Result<Json<WorkspacePage>, Error> {
    fence_page(&scope, query.generation.as_deref())?;
    let generation = scope.generation.clone();
    workspaces::observe_workspace_availability(&scope);
    run(scope, access, move |db, now| workspaces::page(db, now, generation.clone(), &query)).await.map(Json)
}

#[utoipa::path(get, path="/api/v1/admin/namespaces/{id}/workspaces", operation_id="listAdminWorkspaces",
    params(("id"=String,Path),WorkspaceQuery), responses((status=200,body=WorkspacePage)))]
async fn admin_workspaces(State(state): State<Deployment>, parts: Result<Path<AdminPath>, PathRejection>, input: Result<Query<WorkspaceQuery>, QueryRejection>) -> Result<Json<WorkspacePage>, AdminError> {
    let AdminPath { id } = path(parts).map_err(AdminError::Namespace)?;
    let query = query(input).map_err(AdminError::Namespace)?;
    workspace_page(namespace_context(state, id).await?, Access::AdminRead, query).await.map_err(AdminError::Namespace)
}

#[utoipa::path(get, path="/workspaces", operation_id="namespaceListWorkspaces",
    params(WorkspaceQuery), security(("namespaceBearer"=[])), responses((status=200,body=WorkspacePage),(status=409,description="Stale Namespace generation",body=NamespaceError),(status="default",body=NamespaceError)))]
async fn namespace_workspaces(Extension(scope): Extension<ScopeContext>, input: Result<Query<WorkspaceQuery>, QueryRejection>) -> Result<Json<WorkspacePage>, Error> {
    workspace_page(scope, Access::NamespaceRead, query(input)?).await
}

#[utoipa::path(get, path="/api/v1/admin/namespaces/{id}/hosts/{host}/capabilities", operation_id="getAdminHostCapabilities",
    params(("id"=String,Path),("host"=String,Path)), responses((status=200,body=HostCapabilities)))]
async fn admin_capabilities(State(state): State<Deployment>, parts: Result<Path<AdminHostPath>, PathRejection>) -> Result<Json<HostCapabilities>, AdminError> {
    let AdminHostPath { id, host } = path(parts).map_err(AdminError::Namespace)?;
    let scope = namespace_context(state, id).await?;
    let generation = scope.generation.clone();
    run(scope, Access::AdminRead, move |db, now| store::capabilities(db, now, generation.clone(), host.clone())).await.map(Json).map_err(AdminError::Namespace)
}

#[utoipa::path(post, path="/api/v1/admin/namespaces/{id}/operations", operation_id="createAdminControlOperation",
    params(("id"=String,Path)), request_body=CreateControlOperation, responses((status=200,body=ControlReceipt)))]
async fn admin_create(State(state): State<Deployment>, parts: Result<Path<AdminPath>, PathRejection>, input: Result<Json<CreateControlOperation>, JsonRejection>) -> Result<Json<ControlReceipt>, AdminError> {
    let AdminPath { id } = path(parts).map_err(AdminError::Namespace)?;
    let input = payload(input).map_err(AdminError::Namespace)?;
    let scope = namespace_context(state, id).await?;
    fence(&scope, &input.generation).map_err(AdminError::Namespace)?;
    run(scope, Access::AdminSubmit, move |db, now| store::create(db, now, &input)).await.map(Json).map_err(AdminError::Namespace)
}

#[utoipa::path(get, path="/api/v1/admin/namespaces/{id}/operations/{request_id}", operation_id="getAdminControlOperation",
    params(("id"=String,Path),("request_id"=String,Path)), responses((status=200,body=ControlReceipt)))]
async fn admin_get(State(state): State<Deployment>, parts: Result<Path<AdminOperationPath>, PathRejection>) -> Result<Json<ControlReceipt>, AdminError> {
    let AdminOperationPath { id, request_id } = path(parts).map_err(AdminError::Namespace)?;
    let scope = namespace_context(state, id).await?;
    let generation = scope.generation.clone();
    run(scope, Access::AdminRead, move |db, now| store::get(db, now, &generation, &request_id)).await.map(Json).map_err(AdminError::Namespace)
}

#[utoipa::path(put, path="/hosts/{host}/control/lease", operation_id="namespaceRenewControlLease",
    params(("host"=String,Path), LeaseQuery,
        ("X-AgentX-Features"=Option<String>,Header,description="Comma-separated supported features. Advertise workspace_management to accept remote workspace registration.")),
    request_body=RenewControlLease, security(("namespaceBearer"=[])),
    responses((status=200,body=HostCapabilities),(status="default",body=NamespaceError)))]
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
    let replaces_lease = input.previous_lease_id.is_some();
    run_committed(
        scope,
        Access::NamespaceWrite,
        move |db, now| store::renew(db, now, host.clone(), &input, &features),
        move |scope, (_, changed)| {
            if *changed {
                scope.space_changed.send_modify(|snapshot| snapshot.roster = snapshot.roster.wrapping_add(1));
            }
            if *changed || replaces_lease {
                scope.control_changed.send_replace(());
            }
            workspaces::observe_workspace_availability(scope);
        },
    )
    .await
    .map(|(capabilities, _)| Json(capabilities))
}

#[utoipa::path(post, path="/hosts/{host}/control/poll", operation_id="namespacePollControlOperation",
    params(("host"=String,Path)), request_body=PollControlOperation, security(("namespaceBearer"=[])),
    responses((status=200,body=ControlPoll),(status="default",body=NamespaceError)))]
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
        let result = run(scope.clone(), Access::NamespaceWrite, move |db, now| store::poll(db, now, &host, &input)).await?;
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

#[utoipa::path(post, path="/hosts/{host}/control/operations/{request_id}/report", operation_id="namespaceReportControlOperation",
    params(("host"=String,Path),("request_id"=String,Path)), request_body=ReportControlOperation, security(("namespaceBearer"=[])),
    responses((status=200,body=ControlReceipt),(status="default",body=NamespaceError)))]
async fn report_operation(Extension(scope): Extension<ScopeContext>, parts: Result<Path<ReportPath>, PathRejection>, input: Result<Json<ReportControlOperation>, JsonRejection>) -> Result<Json<ControlReceipt>, Error> {
    let ReportPath { host, request_id } = path(parts)?;
    let input = payload(input)?;
    fence(&scope, &input.generation)?;
    run(scope, Access::NamespaceReport, move |db, now| store::report(db, now, &host, &request_id, &input)).await.map(Json)
}

pub(in crate::cloud) fn admin_document() -> openapi::OpenApi {
    #[derive(OpenApi)]
    #[openapi(paths(admin_workspaces, admin_capabilities, admin_create, admin_get))]
    struct ControlAdminApi;
    ControlAdminApi::openapi()
}

pub(in crate::cloud) fn namespace_document() -> openapi::OpenApi {
    #[derive(OpenApi)]
    #[openapi(paths(namespace_workspaces, renew_lease, poll_operation, report_operation))]
    struct ControlNamespaceApi;
    ControlNamespaceApi::openapi()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn workspace_http_reads_share_authority_and_report_lease_availability_changes() {
        use crate::cloud::{
            catalog,
            client::http::{Http, HttpError},
            router,
        };
        use serde_json::{Value, json};
        use std::sync::{Arc, Mutex};

        let temp = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
        let listener = runtime.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let admin_token = "a".repeat(12);
        let namespace_token = "b".repeat(12);
        let mut catalog = catalog::Catalog::open(temp.path(), origin.clone()).unwrap();
        let receipt = catalog.change("team", "provision", &namespace_token, None).unwrap();
        let root = catalog.root_for("team");
        let generation = receipt.generation;
        let mut lease = json!({
            "generation": generation, "installation_id": "installation", "lease_id": "first",
            "workspaces": [{"workspace_id": "WS0000000001", "name": "Project", "default_available": true, "default_reason": null, "roles": []}],
            "project_roots": ["default"]
        });
        let first: RenewControlLease = serde_json::from_value(lease.clone()).unwrap();
        // Simulate a deployment restart with a durable lease near its expiry.
        transaction(&root, &generation, true, || true, |db, now| store::renew(db, now - 26, "host".into(), &first, &store::LeaseFeatures::default())).unwrap();

        let app = router(Deployment { admin_hash: catalog::hash(&admin_token), catalog: Arc::new(Mutex::new(catalog)), calls: Arc::new(tokio::sync::Semaphore::new(16)) });
        runtime.spawn(async move { axum::serve(listener, app).await.unwrap() });
        let member = Http::new(&format!("{origin}/namespaces/team/api/v1"), &namespace_token).unwrap();
        let admin = Http::new(&format!("{origin}/api/v1/admin"), &admin_token).unwrap();
        let unauthorized = Http::new(&format!("{origin}/api/v1/admin"), &namespace_token).unwrap();
        let changes = |cursor: Option<&str>| {
            let suffix = cursor.map_or_else(String::new, |cursor| format!("&cursor={cursor}"));
            member.request::<Value>("GET", &format!("/changes?generation={generation}{suffix}"), None).unwrap()
        };

        assert_eq!(member.request::<Value>("GET", "/descriptor", None).unwrap()["protocol_version"], 2);
        assert_eq!(admin.request::<Value>("GET", "/overview", None).unwrap()["protocol_version"], 2);
        let bad_workspace = member.request::<Value>("GET", "/agents?workspace_id=old-alias", None).unwrap_err();
        assert_eq!(bad_workspace.downcast_ref::<HttpError>().unwrap().status, 400);
        let bad_cursor = member.request::<Value>("GET", "/workspaces?after=host/old-alias", None).unwrap_err();
        assert_eq!(bad_cursor.downcast_ref::<HttpError>().unwrap().status, 400);
        let online = member.request::<Value>("GET", "/workspaces?host=host&limit=1", None).unwrap();
        assert_eq!(online["items"][0]["workspace_id"], "WS0000000001");
        assert_eq!(online["items"][0]["online"], true);
        assert_eq!(admin.request::<Value>("GET", "/namespaces/team/workspaces?host=host&limit=1", None).unwrap(), online);
        let denied = unauthorized.request::<Value>("POST", "/namespaces/team/operations", Some(&json!({}))).unwrap_err();
        assert_eq!(denied.downcast_ref::<HttpError>().unwrap().status, 401);

        let initial = changes(None);
        let cursor = format!("{}.{}.{}.{}", initial["epoch"].as_str().unwrap(), initial["roster"], initial["messages"], initial["transcript"]);
        let expired = changes(Some(&cursor));
        assert_ne!(expired["roster"], initial["roster"]);
        let offline = member.request::<Value>("GET", "/workspaces", None).unwrap();
        assert_eq!(offline["items"][0]["workspace_id"], "WS0000000001");
        assert_eq!(offline["items"][0]["online"], false);

        lease["lease_id"] = json!("replacement");
        let renew = |lease: &Value| member.request::<Value>("PUT", "/hosts/host/control/lease", Some(lease)).unwrap();
        assert_eq!(renew(&lease)["online"], true);
        let restored = changes(None);
        assert_ne!(restored["roster"], expired["roster"]);
        renew(&lease);
        assert_eq!(changes(None), restored);
        lease["workspaces"][0]["name"] = json!("Renamed");
        renew(&lease);
        assert_ne!(changes(None)["roster"], restored["roster"]);
    }

    #[test]
    fn inventory_pages_fence_both_audiences_and_distinguish_access_and_operation_errors() {
        use crate::cloud::{
            catalog,
            client::http::{Http, HttpError},
            router,
        };
        use serde_json::{Value, json};
        use std::sync::{Arc, Mutex};

        let temp = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let listener = runtime.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let mut catalog = catalog::Catalog::open(temp.path(), origin.clone()).unwrap();
        let generation = catalog.change("lab", "provision", "MemberABC123", None).unwrap().generation;
        let root = catalog.root_for("lab");
        let deployment = Arc::new(Mutex::new(catalog));
        let app = router(Deployment { admin_hash: catalog::hash("AdminABCD123"), catalog: deployment.clone(), calls: Arc::new(tokio::sync::Semaphore::new(16)) });
        runtime.spawn(async move { axum::serve(listener, app).await.unwrap() });
        let admin = Http::new(&format!("{origin}/api/v1/admin"), "AdminABCD123").unwrap();
        let member = Http::new(&format!("{origin}/namespaces/lab/api/v1"), "MemberABC123").unwrap();
        let get = |client: &Http, path: &str| client.request::<Value>("GET", path, None);
        let failure = |client: &Http, path: &str, status, code| {
            let error = get(client, path).unwrap_err();
            let error = error.downcast_ref::<HttpError>().unwrap();
            assert_eq!((error.status, error.code.as_str()), (status, code), "{path}");
        };

        for (host, name, incarnation, id) in [("east", "alpha", "AgentRun0001", "WS0000000001"), ("west", "bravo", "AgentRun0002", "WS0000000002")] {
            let lease = json!({"generation":generation, "installation_id":host, "lease_id":host, "workspaces":[
                {"workspace_id":id, "name":"Project", "default_available":true, "default_reason":null, "roles":[]}
            ], "project_roots":["default"]});
            member.request::<Value>("PUT", &format!("/hosts/{host}/control/lease"), Some(&lease)).unwrap();
            let agent = json!({"generation":generation, "installation_id":host, "host":host, "incarnation":incarnation,
                "workspace":{"kind":"project","workspace_id":id}});
            member.request::<Value>("PUT", &format!("/agents/{name}"), Some(&agent)).unwrap();
        }
        let surfaces = [(&admin, "/namespaces/lab"), (&member, "")];
        let mut cursors = Vec::new();
        for (client, prefix) in surfaces {
            for resource in ["hosts", "agents", "workspaces"] {
                let path = format!("{prefix}/{resource}?limit=1");
                let first = get(client, &format!("{path}&generation={generation}")).unwrap();
                assert_eq!(first["generation"], generation);
                assert_eq!(first["items"].as_array().unwrap().len(), 1);
                let next = first["next"].as_str().unwrap();
                let continuation = format!("{path}&after={next}");
                let second = get(client, &format!("{continuation}&generation={generation}")).unwrap();
                assert_eq!(second["generation"], generation);
                assert_ne!(first["items"], second["items"]);
                assert!(second["next"].is_null());
                cursors.push((client, continuation));
            }
            failure(client, &format!("{prefix}/future-resource"), 404, "unsupported_operation");
        }
        failure(&admin, "/namespaces/missing/agents", 404, "not_found");

        // Keep the bound catalogue view but replace the communication database's generation.
        // A route which checks only its guard/metadata and reads separately would leak this page.
        let replacement = "replacement-generation";
        Connection::open(root.join("communication.sqlite3")).unwrap().execute("UPDATE scope_metadata SET generation=?1", [replacement]).unwrap();
        for (client, continuation) in &cursors {
            failure(client, &format!("{continuation}&generation={generation}"), 409, "stale_generation");
        }
        Connection::open(temp.path().join("deployment.sqlite3")).unwrap().execute("UPDATE namespaces SET generation=?1 WHERE id='lab'", [replacement]).unwrap();
        for (client, continuation) in &cursors {
            failure(client, &format!("{continuation}&generation={generation}"), 409, "stale_generation");
            assert_eq!(get(client, &format!("{continuation}&generation={replacement}")).unwrap()["generation"], replacement);
        }

        deployment.lock().unwrap().change("lab", "rotate", "MemberNew123", Some(1)).unwrap();
        failure(&member, &format!("/hosts?generation={replacement}"), 401, "unauthorized");
        assert_eq!(get(&admin, &format!("/namespaces/lab/hosts?generation={replacement}")).unwrap()["generation"], replacement);
    }

    #[test]
    fn lease_features_are_optional_and_independently_negotiated() {
        let mut headers = HeaderMap::new();
        let directory_only = lease_features(&headers, LeaseQuery { directory_launch: Some(true) });
        assert!(directory_only.directory_launch);
        assert!(!directory_only.workspace_management);

        headers.append(LEASE_FEATURES_HEADER, HeaderValue::from_static("future_workspace_management"));
        headers.append(LEASE_FEATURES_HEADER, HeaderValue::from_static("another_feature, workspace_management"));
        let negotiated = lease_features(&headers, LeaseQuery { directory_launch: None });
        assert!(!negotiated.directory_launch);
        assert!(negotiated.workspace_management);
    }

    #[test]
    fn cloud_openapi_exposes_shared_workspace_paging_and_control_contracts() {
        let scope = serde_json::to_value(namespace_document()).unwrap();
        let admin = serde_json::to_value(admin_document()).unwrap();
        let params = scope["paths"]["/hosts/{host}/control/lease"]["put"]["parameters"].as_array().unwrap();
        assert!(params.iter().any(|p| p["name"] == LEASE_FEATURES_HEADER && p["in"] == "header" && p["required"] == false));
        let member = &scope["paths"]["/workspaces"]["get"];
        let operator = &admin["paths"]["/api/v1/admin/namespaces/{id}/workspaces"]["get"];
        assert_eq!(member["operationId"], "namespaceListWorkspaces");
        assert_eq!(operator["operationId"], "listAdminWorkspaces");
        assert_eq!(member["security"], serde_json::json!([{"namespaceBearer":[]}]));
        let filters: Vec<_> = operator["parameters"].as_array().unwrap().iter().filter(|p| p["in"] == "query").cloned().collect();
        assert_eq!(filters, *member["parameters"].as_array().unwrap());
        for name in ["ControlCommand", "ControlOutcome", "HostCapabilities", "WorkspaceCapability", "WorkspacePage", "WorkspaceView"] {
            let schemas = &scope["components"]["schemas"];
            assert!(schemas[name].is_object());
            assert_eq!(schemas[name], admin["components"]["schemas"][name]);
        }
        let schemas = &scope["components"]["schemas"];
        let add = schemas["ControlCommand"]["oneOf"].as_array().unwrap().iter().find(|variant| variant["properties"]["kind"]["enum"][0] == "add_workspace").unwrap();
        assert_eq!(add["required"].as_array().unwrap().len(), 5);
        assert!(["kind", "parent", "directory", "name", "create_directory"].iter().all(|name| add["required"].as_array().unwrap().contains(&serde_json::json!(name))));
        let added = schemas["ControlOutcome"]["oneOf"].as_array().unwrap().iter().find(|variant| variant["properties"]["status"]["enum"][0] == "workspace_added").unwrap();
        assert!(added["required"].as_array().unwrap().contains(&serde_json::json!("workspace_id")));
        assert!(schemas["RenewControlLease"]["required"].as_array().unwrap().contains(&serde_json::json!("project_roots")));
        assert!(schemas["WorkspaceCapability"]["required"].as_array().unwrap().contains(&serde_json::json!("workspace_id")));
        let launch = schemas["ControlCommand"]["oneOf"].as_array().unwrap().iter().find(|variant| variant["properties"]["kind"]["enum"][0] == "launch").unwrap();
        assert!(launch["required"].as_array().unwrap().contains(&serde_json::json!("workspace_id")));
    }
}
