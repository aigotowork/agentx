//! Daemon-owned HTTP listener. Native dispatch, file reads and process waits stay separate.
use super::contract::*;
use crate::browser::{
    admission::{allowed_boundary, cors, preflight, single_header},
    assets, wire,
};
use crate::service::{InvocationContext, ServiceError};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request as HttpRequest, State},
    http::{StatusCode, header},
    response::Response as HttpResponse,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::Semaphore;

const BODY_LIMIT: usize = 128 * 1024;
const RESPONSE_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Config {
    pub enabled: bool,
    pub host: IpAddr,
    pub port: u16,
    pub project: Option<PathBuf>,
}
impl Default for Config {
    fn default() -> Self {
        Self { enabled: false, host: IpAddr::from([127, 0, 0, 1]), port: 0, project: None }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Endpoint {
    pub daemon_instance: String,
    pub url: String,
    pub project: String,
    pub bind_host: IpAddr,
    pub bind_port: u16,
}
pub(crate) fn endpoint(scope: &Path) -> Option<Endpoint> {
    serde_json::from_slice(&std::fs::read(scope.join(".daemon/management.json")).ok()?).ok()
}
fn config(scope: &Path) -> Result<Config, String> {
    let text = match std::fs::read_to_string(scope.join("config.toml")) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(error) => return Err(error.to_string()),
    };
    let value: toml::Value = toml::from_str(&text).map_err(|e: toml::de::Error| e.to_string())?;
    let config: Config = value.get("management").cloned().map(|v| v.try_into().map_err(|e: toml::de::Error| e.to_string())).transpose()?.unwrap_or_default();
    if config.port > 0 && config.port < 1024 {
        return Err("management port must be 0 or >=1024".into());
    }
    Ok(config)
}
pub(crate) fn configure(scope: &Path, project: &Path, host: Option<IpAddr>, port: Option<u16>) -> Result<(), String> {
    if port.is_some_and(|p| p > 0 && p < 1024) {
        return Err("port must be 0 or >=1024".into());
    }
    crate::config::file::edit_config_at_path(&scope.join("config.toml"), |doc| {
        doc["management"]["enabled"] = toml_edit::value(true);
        doc["management"]["project"] = toml_edit::value(project.to_string_lossy().as_ref());
        if let Some(host) = host {
            doc["management"]["host"] = toml_edit::value(host.to_string());
        }
        if let Some(port) = port {
            doc["management"]["port"] = toml_edit::value(i64::from(port));
        }
        Ok(true)
    })
}

/// The scope thread joins this loop before releasing daemon ownership.
pub(crate) fn run(scope: PathBuf, daemon_instance: String, stop: Arc<AtomicBool>) {
    let runtime = match tokio::runtime::Builder::new_multi_thread().worker_threads(2).max_blocking_threads(8).enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            crate::log::log_in_scope(&scope, "ERROR", "management", "runtime.failed", &error.to_string(), &[("daemon_instance", &daemon_instance)]);
            eprintln!("management runtime: {error}");
            return;
        }
    };
    let collector_scope = scope.clone();
    let collector_stop = stop.clone();
    let collector_invocation = invocation(&scope, &scope);
    let collector = std::thread::spawn(move || crate::runtime::inspection::run(collector_scope, collector_invocation, collector_stop));
    let reaper_scope = scope.clone();
    let reaper_stop = stop.clone();
    let reaper = std::thread::Builder::new().name("agentx-management-reap".into()).spawn(move || reap_periodically(&reaper_scope, &reaper_stop)).ok();
    runtime.block_on(async {
        let mut last_error = String::new();
        let workers = Arc::new(Semaphore::new(4));
        while !stop.load(Ordering::Acquire) {
            let result = match config(&scope) {
                Ok(selected) if selected.enabled => serve(&scope, &daemon_instance, selected, stop.clone(), workers.clone()).await,
                Ok(_) => Ok(()),
                Err(error) => Err(error),
            };
            if let Err(error) = result
                && error != last_error
            {
                crate::log::log_in_scope(&scope, "ERROR", "management", "listener.failed", &error, &[("daemon_instance", &daemon_instance)]);
                eprintln!("management HTTP: {error}");
                last_error = error;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
    if let Some(reaper) = reaper {
        reaper.thread().unpark();
        let _ = reaper.join();
    }
    let _ = collector.join();
}

/// Process cleanup is maintenance, independent of the number of browsers.
fn reap_periodically(scope: &Path, stop: &AtomicBool) {
    use crate::service::{Request, Response, client::Client, maintenance};
    while !stop.load(Ordering::Acquire) {
        std::thread::park_timeout(Duration::from_secs(30));
        if stop.load(Ordering::Acquire) {
            break;
        }
        if config(scope).is_ok_and(|config| config.enabled)
            && let Ok(client) = Client::connect_existing(scope, invocation(scope, scope))
            && let Ok(Response::Maintenance(maintenance::Response::Cleanup(outcome))) = client.call(Request::Maintenance(maintenance::Request::ReapDeadProcesses))
        {
            for effect in outcome.effects {
                crate::runtime::stop_effects::apply(effect);
            }
        }
    }
}
struct Adapter {
    scope: PathBuf,
    daemon_instance: String,
    service: super::service::ManagementService,
    assets: assets::Assets,
    calls: Arc<Semaphore>,
    diagnostics: Option<crate::log::DispatchLog>,
}

fn invocation(scope: &Path, project: &Path) -> InvocationContext {
    let mut environment: std::collections::HashMap<String, String> = std::env::vars().collect();
    for name in crate::runtime::launcher::env_strip_set() {
        environment.remove(&name);
    }
    environment.insert("AGENTX_DIR".into(), scope.to_string_lossy().into());
    InvocationContext { cwd: project.to_path_buf(), environment, ..Default::default() }
}
async fn serve(scope: &Path, daemon_instance: &str, selected: Config, stop: Arc<AtomicBool>, workers: Arc<Semaphore>) -> Result<(), String> {
    let project = selected.project.clone().unwrap_or_else(|| scope.to_path_buf()).canonicalize().map_err(|e| e.to_string())?;
    if !project.is_dir() {
        return Err("management project must be an existing directory".into());
    }
    let listener = tokio::net::TcpListener::bind((selected.host, selected.port)).await.map_err(|e| e.to_string())?;
    let address = listener.local_addr().map_err(|e| e.to_string())?;
    let display_host = if address.ip().is_unspecified() { IpAddr::from([127, 0, 0, 1]) } else { address.ip() };
    let endpoint = Endpoint { daemon_instance: daemon_instance.into(), url: format!("http://{}/", SocketAddr::new(display_host, address.port())), project: project.to_string_lossy().into(), bind_host: selected.host, bind_port: selected.port };
    let descriptor = scope.join(".daemon/management.json");
    let temp = scope.join(".daemon/management.json.tmp");
    std::fs::write(&temp, serde_json::to_vec(&endpoint).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, &descriptor).map_err(|e| e.to_string())?;
    struct EndpointGuard(PathBuf, String);
    impl Drop for EndpointGuard {
        fn drop(&mut self) {
            if let Ok(bytes) = std::fs::read(&self.0)
                && let Ok(current) = serde_json::from_slice::<Endpoint>(&bytes)
                && current.daemon_instance == self.1
            {
                let _ = std::fs::remove_file(&self.0);
            }
        }
    }
    let _guard = EndpointGuard(descriptor, daemon_instance.into());
    let entries: &[(&str, &str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/management_assets.rs"));
    let mut embedded = assets::from_entries(entries);
    let space: &[(&str, &str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/agent-space_assets.rs"));
    let inspector: &[(&str, &str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/inspector_assets.rs"));
    embedded.extend(assets::from_entries(space));
    embedded.extend(assets::from_entries(inspector));
    if let Some(page) = embedded.get_mut("/space/") {
        page.bytes = bind_space_document(&page.bytes, scope)?;
    }
    let service = super::service::ManagementService::new(scope.to_path_buf(), project.clone(), invocation(scope, &project), workers);
    let state = Arc::new(Adapter { scope: scope.to_path_buf(), daemon_instance: daemon_instance.into(), service, assets: embedded, calls: Arc::new(Semaphore::new(8)), diagnostics: crate::log::DispatchLog::start(scope) });
    let router = Router::new().fallback(handle).with_state(state);
    let connections = Arc::new(Semaphore::new(16));
    let mut tasks = tokio::task::JoinSet::new();
    while !stop.load(Ordering::Acquire) {
        while tasks.try_join_next().is_some() {}
        if config(scope).ok().as_ref() != Some(&selected) {
            break;
        }
        let accepted = match tokio::time::timeout(Duration::from_millis(500), listener.accept()).await {
            Ok(value) => value.map_err(|e| e.to_string())?,
            Err(_) => {
                if config(scope).ok().as_ref() != Some(&selected) {
                    break;
                }
                continue;
            }
        };
        let (stream, _) = accepted;
        let local = stream.local_addr().map_err(|e| e.to_string())?;
        let Ok(permit) = connections.clone().try_acquire_owned() else {
            continue;
        };
        let service = hyper_util::service::TowerToHyperService::new(router.clone().layer(axum::Extension(local)));
        tasks.spawn(async move {
            let _permit = permit;
            let mut builder = hyper::server::conn::http1::Builder::new();
            builder.timer(hyper_util::rt::TokioTimer::new()).header_read_timeout(Duration::from_secs(5)).max_headers(32).max_buf_size(16 * 1024).keep_alive(false);
            let _ = tokio::time::timeout(Duration::from_secs(35), builder.serve_connection(hyper_util::rt::TokioIo::new(stream), service)).await;
        });
    }
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    Ok(())
}

/// Bind the embedded UI to this listener's configured scope independently of the descriptor response.
fn bind_space_document(page: &[u8], scope: &Path) -> Result<Vec<u8>, String> {
    const META: &str = "<meta name=\"agentx-space-scope\"";
    let html = std::str::from_utf8(page).map_err(|error| error.to_string())?;
    if !html.contains(META) {
        return Err("embedded Space scope marker is missing".into());
    }
    let escaped = scope.to_string_lossy().replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;").replace('\r', "&#13;");
    Ok(html.replacen(META, &format!("{META} content=\"{escaped}\""), 1).into_bytes())
}
fn boundary(request: &HttpRequest) -> Result<(), ServiceError> {
    allowed_boundary(request).map_err(|reason| ServiceError::Forbidden(reason.into()))?;
    Ok(())
}
async fn handle(State(state): State<Arc<Adapter>>, request: HttpRequest) -> HttpResponse {
    // Existing React scene geometry and React Native Web register dynamic
    // styles. Permit those styles only on the two owned UI document routes.
    let styled_ui = matches!(request.uri().path(), "/space" | "/space/" | "/inspector" | "/inspector/");
    let api = request.uri().path().starts_with("/api/v1/");
    let diagnostic_route = diagnostic_route(request.uri().path());
    let method = request.method().to_string();
    let request_id = uuid::Uuid::new_v4().to_string();
    let started = std::time::Instant::now();
    let result = route(state.clone(), request).await;
    let elapsed = started.elapsed().as_millis().to_string();
    let diagnostic = if api {
        match &result {
            Err(error) => {
                let (status, envelope) = super::openapi::ErrorEnvelope::from_service(error.clone());
                let code = serde_json::to_value(envelope.error.code).ok().and_then(|value| value.as_str().map(str::to_owned)).unwrap_or_default();
                let reason = match error {
                    ServiceError::Unavailable(message) if message == "HTTP calls busy" => "http_admission",
                    ServiceError::Unavailable(message) if message.contains("daemon is busy") => "native_admission",
                    ServiceError::Unavailable(_) => "native_unavailable",
                    ServiceError::OutcomeUnknown(_) => "outcome_unknown",
                    _ => "request_failed",
                };
                Some((if status.is_server_error() { "ERROR" } else { "WARN" }, "http.failed", status.as_u16().to_string(), code, reason))
            }
            Ok(response) if started.elapsed() >= Duration::from_millis(500) => Some(("WARN", "http.slow", response.status().as_u16().to_string(), String::new(), "slow_response")),
            _ => None,
        }
    } else {
        None
    };
    if let (Some(log), Some((level, event, status, code, reason))) = (&state.diagnostics, diagnostic) {
        log.offer_event(
            level,
            "management",
            event,
            reason,
            &[("daemon_instance", &state.daemon_instance), ("http_request_id", &request_id), ("method", &method), ("route", &diagnostic_route), ("status", &status), ("error_code", &code), ("duration_ms", &elapsed)],
        );
    }
    let response = result.unwrap_or_else(failure);
    let mut response = response;
    if api {
        response.headers_mut().insert("x-request-id", axum::http::HeaderValue::from_str(&request_id).expect("UUID header"));
    }
    for (name, value) in [
        ("cache-control", "no-store"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        (
            "content-security-policy",
            if styled_ui {
                "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'"
            } else {
                "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'"
            },
        ),
    ] {
        response.headers_mut().insert(axum::http::HeaderName::from_static(name), axum::http::HeaderValue::from_static(value));
    }
    cors(&mut response);
    response
}
/// Only canonical resource names enter logs; names/IDs, query values and bodies do not.
fn diagnostic_route(path: &str) -> String {
    let parts: Vec<_> = path.trim_matches('/').split('/').collect();
    match parts.as_slice() {
        ["api", "v1", resource] if ["management", "agents", "roles", "workspaces", "directory", "inspection", "office-activity", "human-mailbox", "attachments"].contains(resource) => format!("/api/v1/{resource}"),
        ["api", "v1", "workspaces", "members"] => "/api/v1/workspaces/members".into(),
        ["api", "v1", "agents", _] => "/api/v1/agents/:agent".into(),
        ["api", "v1", resource, _] if ["operations", "attachments"].contains(resource) => format!("/api/v1/{resource}/:id"),
        ["api", "v1", "agents", _, action] if ["messages", "message-history", "stop", "kill", "stop-participation", "fork", "resume", "transcript", "labels", "launch-relations"].contains(action) => format!("/api/v1/agents/:agent/{action}"),
        ["api", "v1", "inspection", ..] => "/api/v1/inspection/:resource".into(),
        _ => "/api/v1/:unknown".into(),
    }
}
fn parse_query(query: Option<&str>) -> Result<BTreeMap<String, String>, ServiceError> {
    let mut fields = BTreeMap::new();
    for pair in query.unwrap_or("").split('&').filter(|p| !p.is_empty()) {
        let (name, value) = pair.split_once('=').ok_or_else(|| ServiceError::InvalidInput("invalid query".into()))?;
        let value = percent_encoding::percent_decode_str(&value.replace('+', " ")).decode_utf8().map_err(|_| ServiceError::InvalidInput("invalid query encoding".into()))?.into_owned();
        if name == "label" {
            let mut labels: Vec<String> = fields.get("label").map(|value: &String| serde_json::from_str(value).unwrap_or_default()).unwrap_or_default();
            if labels.len() >= 16 || value.len() > 4096 {
                return Err(ServiceError::InvalidInput("too many or oversized label selectors".into()));
            }
            labels.push(value);
            fields.insert("label".into(), serde_json::to_string(&labels).map_err(|e| ServiceError::Internal(e.to_string()))?);
            continue;
        }
        if !["current_only", "generation", "incarnation", "stopped_event", "after", "before", "pending", "limit", "workspace", "directory", "name", "media_type"].contains(&name) || value.len() > 4096 || fields.insert(name.into(), value).is_some() {
            return Err(ServiceError::InvalidInput("invalid or duplicate query field".into()));
        }
    }
    Ok(fields)
}
fn reference(name: &str, query: &BTreeMap<String, String>) -> Result<AgentRef, ServiceError> {
    let reference = AgentRef { name: name.into(), incarnation: query.get("incarnation").cloned(), stopped_event: query.get("stopped_event").cloned() };
    super::store::validate_reference(&reference)?;
    Ok(reference)
}
async fn route(state: Arc<Adapter>, request: HttpRequest) -> Result<HttpResponse, ServiceError> {
    boundary(&request)?;
    if let Some(response) = preflight(&request) {
        return Ok(response);
    }
    let path = request.uri().path().to_owned();
    if !path.starts_with("/api/") {
        if request.method() != "GET" {
            return Err(ServiceError::NotFound("route".into()));
        }
        let asset_path = match path.as_str() {
            "/space" => "/space/",
            "/inspector" => "/inspector/",
            _ => &path,
        };
        let asset = state.assets.get(asset_path).ok_or_else(|| ServiceError::NotFound("asset".into()))?;
        return Ok(HttpResponse::builder().status(StatusCode::OK).header(header::CONTENT_TYPE, &asset.content_type).body(Body::from(asset.bytes.clone())).unwrap());
    }
    let _permit = state.calls.clone().try_acquire_owned().map_err(|_| ServiceError::Unavailable("HTTP calls busy".into()))?;
    let method = request.method().clone();
    let query = parse_query(request.uri().query())?;
    let member_list = method == "GET" && path == "/api/v1/workspaces/members";
    if path != "/api/v1/agents" && (query.contains_key("label") || (query.contains_key("current_only") && !member_list)) {
        return Err(ServiceError::InvalidInput("label filters apply only to the current Agent list".into()));
    }
    if path == "/api/v1/attachments" && method == "POST" {
        allowed_query(&query, &["generation", "name", "media_type"])?;
        let input = UploadAttachmentInput { generation: required_query(&query, "generation")?, name: required_query(&query, "name")?, media_type: query.get("media_type").cloned().unwrap_or_default() };
        if single_header(request.headers(), "content-type") != Some("application/octet-stream") || request.headers().contains_key("content-encoding") {
            return Err(ServiceError::InvalidInput("raw application/octet-stream required".into()));
        }
        let before = state.clone();
        let generation = input.generation.clone();
        tokio::task::spawn_blocking(move || before.service.client(Some(&generation))).await.map_err(|e| ServiceError::Internal(e.to_string()))??;
        let bytes = tokio::time::timeout(Duration::from_secs(20), to_bytes(request.into_body(), super::uploads::MAX_FILE))
            .await
            .map_err(|_| ServiceError::InvalidInput("upload deadline".into()))?
            .map_err(|_| ServiceError::InvalidInput("file exceeds 10 MiB limit".into()))?;
        return blocking(move || upload_attachment(&state, input, &bytes)).await;
    }
    if method == "GET" && path.starts_with("/api/v1/attachments/") {
        if query.len() != 1 {
            return Err(ServiceError::InvalidInput("attachment needs generation only".into()));
        }
        let input = GetAttachmentInput { generation: required_query(&query, "generation")?, id: path.trim_start_matches("/api/v1/attachments/").into() };
        return blocking(move || download_attachment(&state, input)).await;
    }
    if method == "GET" && path == "/api/v1/management" {
        return blocking(move || describe(&state)).await;
    }
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if method == "GET" {
        let generation = required_query(&query, "generation")?;
        let limit = query.get("limit").map(|s| s.parse::<u32>().map_err(|_| ServiceError::InvalidInput("invalid limit".into()))).transpose()?;
        return match segments.as_slice() {
            ["api", "v1", "inspection"] => {
                allowed_query(&query, &["generation"])?;
                blocking(move || inspection_snapshot(&state, GenerationInput { generation })).await
            }
            ["api", "v1", "workspaces"] => {
                allowed_query(&query, &["generation"])?;
                blocking(move || list_workspaces(&state, GenerationInput { generation })).await
            }
            ["api", "v1", "workspaces", "members"] => {
                allowed_query(&query, &["generation", "workspace", "after", "limit", "current_only"])?;
                let input = ListWorkspaceMembersInput { generation, workspace: required_query(&query, "workspace")?, after: query.get("after").cloned(), limit, current_only: current_only(&query)? };
                blocking(move || list_workspace_members(&state, input)).await
            }
            ["api", "v1", "roles"] => {
                allowed_query(&query, &["generation", "workspace"])?;
                let input = ListRolesInput { generation, workspace: required_query(&query, "workspace")? };
                blocking(move || list_roles(&state, input)).await
            }
            ["api", "v1", "directory-entry"] => {
                allowed_query(&query, &["generation", "directory"])?;
                let input = InspectDirectoryInput { generation, directory: required_query(&query, "directory")? };
                blocking(move || inspect_directory(&state, input)).await
            }
            ["api", "v1", "agents"] => {
                let input = ListAgentsInput {
                    generation,
                    after: query.get("after").cloned(),
                    limit,
                    current_only: current_only(&query)?,
                    labels: query.get("label").map(|v| serde_json::from_str(v).map_err(|_| ServiceError::InvalidInput("invalid selectors".into()))).transpose()?.unwrap_or_default(),
                };
                blocking(move || list_agents(&state, input)).await
            }
            ["api", "v1", "agents", name] => {
                let input = GetAgentInput { generation, agent: reference(name, &query)? };
                blocking(move || get_agent(&state, input)).await
            }
            ["api", "v1", "agents", name, "labels"] => {
                let input = GetAgentInput { generation, agent: reference(name, &query)? };
                blocking(move || get_labels(&state, input)).await
            }
            ["api", "v1", "agents", name, "message-history"] => {
                allowed_query(&query, &["generation", "incarnation", "stopped_event", "before", "limit"])?;
                let input = MessageHistoryInput { generation, agent: reference(name, &query)?, before: query.get("before").cloned(), limit };
                blocking(move || message_history(&state, input)).await
            }
            ["api", "v1", "agents", name, action] if ["messages", "launch-relations"].contains(action) => {
                let input = AgentPageInput { generation, agent: reference(name, &query)?, after: query.get("after").cloned(), limit };
                if *action == "messages" { blocking(move || list_messages(&state, input)).await } else { blocking(move || list_launch_relations(&state, input)).await }
            }
            ["api", "v1", "office-activity"] => {
                let input = PageInput { generation, after: query.get("after").cloned(), limit };
                blocking(move || list_office_activity(&state, input)).await
            }
            ["api", "v1", "human-mailbox"] => {
                allowed_query(&query, &["generation", "before", "pending", "limit"])?;
                let input = ListHumanMailboxInput {
                    generation,
                    before: query.get("before").cloned(),
                    limit,
                    pending: query.get("pending").map(|v| v.parse::<bool>().map_err(|_| ServiceError::InvalidInput("invalid pending filter".into()))).transpose()?.unwrap_or(false),
                };
                blocking(move || list_human_mailbox(&state, input)).await
            }
            ["api", "v1", "operations", id] => {
                let input = GetOperationInput { generation, request_id: (*id).into() };
                blocking(move || get_operation(&state, input)).await
            }
            _ => Err(ServiceError::NotFound("route".into())),
        };
    }
    if method != "POST" {
        return Err(ServiceError::NotFound("route".into()));
    }
    if request.headers().contains_key("content-encoding") {
        return Err(ServiceError::InvalidInput("encoded bodies are unsupported".into()));
    }
    if single_header(request.headers(), "content-type") != Some("application/json") {
        return Err(ServiceError::InvalidInput("application/json required".into()));
    }
    let bytes = tokio::time::timeout(Duration::from_secs(5), to_bytes(request.into_body(), BODY_LIMIT)).await.map_err(|_| ServiceError::InvalidInput("body deadline".into()))?.map_err(|_| ServiceError::InvalidInput("body exceeds limit".into()))?;
    if path.starts_with("/api/v1/inspection/") {
        allowed_query(&query, &[])?;
        use crate::inspection::contract::PublicMutation as Inspect;
        let input: Inspect = wire::decode_json(&bytes).map_err(|_| ServiceError::InvalidInput("invalid inspection JSON".into()))?;
        return match (input, path.as_str()) {
            (Inspect::Configure(input), "/api/v1/inspection/configure") => blocking(move || configure_inspection(&state, input)).await,
            (Inspect::Analyze(input), "/api/v1/inspection/analyze") => blocking(move || analyze_inspection(&state, input)).await,
            (Inspect::CancelAnalysis(input), "/api/v1/inspection/cancel-analysis") => blocking(move || cancel_inspection_analysis(&state, input)).await,
            (Inspect::RequestContinuation(input), "/api/v1/inspection/continuation") => blocking(move || request_continuation(&state, input)).await,
            _ => Err(ServiceError::InvalidInput("inspection action does not match route".into())),
        };
    }
    match segments.as_slice() {
        ["api", "v1", "human-mailbox", "handle"] => {
            allowed_query(&query, &[])?;
            let input = decode_body::<HandleHumanMessages>(&bytes)?;
            blocking(move || handle_human_messages(&state, input)).await
        }
        ["api", "v1", "directory-messages"] => {
            let input = decode_body::<SubmitDirectoryMessage>(&bytes)?;
            blocking(move || submit_directory_message(&state, input)).await
        }
        ["api", "v1", "agents"] => {
            let input = decode_body::<SubmitLaunch>(&bytes)?;
            blocking(move || submit_launch(&state, input)).await
        }
        ["api", "v1", "agents", name, "labels"] => {
            let input = decode_body::<SubmitLabels>(&bytes)?;
            path_matches(name, &input.agent)?;
            blocking(move || patch_labels(&state, input)).await
        }
        ["api", "v1", "agents", name, "messages"] => {
            let input = decode_body::<SubmitMessage>(&bytes)?;
            path_matches(name, &input.agent)?;
            blocking(move || submit_message(&state, input)).await
        }
        ["api", "v1", "agents", name, action] if ["stop", "kill", "stop-participation", "fork", "resume", "transcript"].contains(action) => {
            let input = decode_body::<SubmitTarget>(&bytes)?;
            path_matches(name, &input.agent)?;
            match *action {
                "stop" => blocking(move || stop_agent(&state, input)).await,
                "kill" => blocking(move || kill_agent(&state, input)).await,
                "stop-participation" => blocking(move || stop_participation(&state, input)).await,
                "fork" => blocking(move || fork_agent(&state, input)).await,
                "resume" => blocking(move || resume_agent(&state, input)).await,
                _ => blocking(move || request_transcript(&state, input)).await,
            }
        }
        _ => Err(ServiceError::NotFound("route".into())),
    }
}

async fn blocking(call: impl FnOnce() -> Result<HttpResponse, ServiceError> + Send + 'static) -> Result<HttpResponse, ServiceError> {
    tokio::task::spawn_blocking(call).await.map_err(|error| ServiceError::Internal(error.to_string()))?
}
fn current_only(query: &BTreeMap<String, String>) -> Result<bool, ServiceError> {
    query.get("current_only").map_or(Ok(false), |value| value.parse().map_err(|_| ServiceError::InvalidInput("invalid current_only".into())))
}
fn required_query(query: &BTreeMap<String, String>, field: &str) -> Result<String, ServiceError> {
    query.get(field).cloned().ok_or_else(|| ServiceError::InvalidInput(format!("{field} required")))
}
fn allowed_query(query: &BTreeMap<String, String>, allowed: &[&str]) -> Result<(), ServiceError> {
    if query.keys().any(|key| !allowed.contains(&key.as_str())) { Err(ServiceError::InvalidInput("unexpected query field".into())) } else { Ok(()) }
}
fn path_matches(name: &str, agent: &AgentRef) -> Result<(), ServiceError> {
    if name != agent.name { Err(ServiceError::InvalidInput("Agent path mismatch".into())) } else { Ok(()) }
}
fn decode_body<T: serde::de::DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T, ServiceError> {
    wire::decode_json(bytes).map_err(|_| ServiceError::InvalidInput("invalid JSON".into()))
}

fn json<T: Serialize>(status: StatusCode, value: &T) -> Result<HttpResponse, ServiceError> {
    let bytes = serde_json::to_vec(value).map_err(|e| ServiceError::Internal(e.to_string()))?;
    if bytes.len() > RESPONSE_LIMIT {
        return Err(ServiceError::InvalidInput("response exceeds limit".into()));
    }
    Ok(HttpResponse::builder().status(status).header(header::CONTENT_TYPE, "application/json").body(Body::from(bytes)).unwrap())
}

fn failure(error: ServiceError) -> HttpResponse {
    let (status, envelope) = super::openapi::ErrorEnvelope::from_service(error);
    json(status, &envelope).unwrap()
}

#[cfg(test)]
mod label_query_tests {
    use super::*;
    #[test]
    fn browser_alias_origin_and_upload_preflight_do_not_select_native_authority() {
        for origin in ["null", "https://example.com", "chrome-extension://example"] {
            let request = HttpRequest::builder()
                .method("OPTIONS")
                .uri("/api/v1/attachments?generation=original")
                .header("host", "localhost:18761")
                .header("origin", origin)
                .header("sec-fetch-site", "cross-site")
                .extension("127.0.0.1:18761".parse::<SocketAddr>().unwrap())
                .body(Body::empty())
                .unwrap();
            assert!(boundary(&request).is_ok());
            let mut response = preflight(&request).unwrap();
            cors(&mut response);
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
            assert_eq!(response.headers()["access-control-allow-origin"], "*");
            assert_eq!(response.headers()["access-control-expose-headers"], "Content-Disposition, Content-Type");
        }
    }

    #[test]
    fn repeated_selectors_preserve_and_values_and_reject_duplicate_authority() {
        let query = parse_query(Some("label=note%3Da%3Db%2Cc&label=%21review%2Fstate&current_only=true")).unwrap();
        let selectors: Vec<String> = serde_json::from_str(&query["label"]).unwrap();
        assert_eq!(selectors, ["note=a=b,c", "!review/state"]);
        assert!(parse_query(Some("generation=a&generation=b")).is_err());
    }
}

// These are the real pilot HTTP bindings; OpenAPI consumes the same functions.
/// Describe the selected listener; does not acquire authority.
#[utoipa::path(
    get, path = "/api/v1/management", operation_id = "managementDescribe",
    tag = "management",
    responses(
        (status = 200, body = Descriptor),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn describe(state: &Adapter) -> Result<HttpResponse, ServiceError> {
    let result: Descriptor = state.service.describe()?;
    json(StatusCode::OK, &result)
}

/// Bounded list with coverage; first-page native maintenance can commit cleanup.
#[utoipa::path(
    get, path = "/api/v1/agents", operation_id = "managementListAgents",
    tag = "management",
    params(
        ("generation" = String, Query, description = "Original descriptor generation; stale values are rejected"),
        ("after" = Option<String>, Query, description = "Native opaque selector-bound cursor"),
        ("limit" = Option<u32>, Query, minimum = 1, maximum = 100, description = "Defaults to 50"),
        ("current_only" = Option<bool>, Query, description = "Defaults to false; labels require true"),
        ("label" = Option<Vec<String>>, Query, style = Form, explode, description = "Repeated AND selectors, maximum 16; each at most 4096 bytes")
    ),
    responses(
        (status = 200, body = AgentPage),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn list_agents(state: &Adapter, input: ListAgentsInput) -> Result<HttpResponse, ServiceError> {
    let result: AgentPage = state.service.list_agents(input)?;
    json(StatusCode::OK, &result)
}

/// Read the exact current or historical Agent. Never fall back to a name lookup.
#[utoipa::path(
    get, path = "/api/v1/agents/{name}", operation_id = "managementGetAgent",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from the exact Agent reference"),
        ("generation" = String, Query, description = "Original descriptor generation; stale values are rejected"),
        ("incarnation" = Option<String>, Query, description = "Exactly one of incarnation or stopped_event is required"),
        ("stopped_event" = Option<String>, Query, description = "Historical identity; never resolves a same-name successor")
    ),
    responses(
        (status = 200, body = AgentView),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn get_agent(state: &Adapter, input: GetAgentInput) -> Result<HttpResponse, ServiceError> {
    let result: AgentView = state.service.get_agent(input)?;
    json(StatusCode::OK, &result)
}

/// Accept the original request identity and payload. Acceptance is not delivery. Retain generation/request_id and reconcile after response loss; do not generate a new ID for retry.
#[utoipa::path(
    post, path = "/api/v1/agents/{name}/messages", operation_id = "managementSubmitMessage",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from the exact Agent reference")
    ),
    request_body(content = SubmitMessage, content_type = "application/json", description = "Path name must equal body agent.name. Exact reference required; no caller context or worker ticket."),
    responses(
        (status = 202, body = Operation),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn submit_message(state: &Adapter, input: SubmitMessage) -> Result<HttpResponse, ServiceError> {
    let result: Operation = state.service.submit_message(input)?;
    json(StatusCode::ACCEPTED, &result)
}

/// Observe/reconcile the original receipt. NotFound does not prove no effect. Unknown remains unknown; cancelling observation does not cancel work.
#[utoipa::path(
    get, path = "/api/v1/operations/{request_id}", operation_id = "managementGetOperation",
    tag = "management",
    params(
        ("request_id" = String, Path, description = "Original retained request ID"),
        ("generation" = String, Query, description = "Original descriptor generation; stale values are rejected")
    ),
    responses(
        (status = 200, body = Operation),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn get_operation(state: &Adapter, input: GetOperationInput) -> Result<HttpResponse, ServiceError> {
    let result: Operation = state.service.get_operation(input)?;
    json(StatusCode::OK, &result)
}

/// Bind the native list_workspaces capability without duplicating domain rules.
#[utoipa::path(
    get, path = "/api/v1/workspaces", operation_id = "managementListWorkspaces",
    tag = "management",
    params(
        ("generation" = String, Query, description = "Original selected generation")
    ),
    responses(
        (status = 200, body = WorkspaceCatalog),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn list_workspaces(state: &Adapter, input: GenerationInput) -> Result<HttpResponse, ServiceError> {
    let result: WorkspaceCatalog = state.service.list_workspaces(input)?;
    json(StatusCode::OK, &result)
}

/// Page matching saved workspace members before projection. Current names sort
/// ascending, then exact stopped events sort descending by native event ID.
#[utoipa::path(
    get, path = "/api/v1/workspaces/members", operation_id = "managementListWorkspaceMembers",
    tag = "management",
    params(
        ("generation" = String, Query, description = "Original selected generation"),
        ("workspace" = String, Query, description = "Exact saved absolute workspace path, or global for explicitly adopted global membership"),
        ("after" = Option<String>, Query, description = "Opaque cursor bound to generation, workspace and current_only"),
        ("limit" = Option<u32>, Query, minimum = 1, maximum = 100, description = "Defaults to 50"),
        ("current_only" = Option<bool>, Query, description = "Defaults to false. True excludes stopped executions")
    ),
    responses(
        (status = 200, description = "Only matching local, adopted members. current_complete is scoped to this selection.", body = AgentPage),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn list_workspace_members(state: &Adapter, input: ListWorkspaceMembersInput) -> Result<HttpResponse, ServiceError> {
    json(StatusCode::OK, &state.service.list_workspace_members(input)?)
}

/// Bind the native list_roles capability without duplicating domain rules.
#[utoipa::path(
    get, path = "/api/v1/roles", operation_id = "managementListRoles",
    tag = "management",
    params(
        ("generation" = String, Query, description = "Original selected generation"),
        ("workspace" = String, Query)
    ),
    responses(
        (status = 200, body = RoleCatalog),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn list_roles(state: &Adapter, input: ListRolesInput) -> Result<HttpResponse, ServiceError> {
    let result: RoleCatalog = state.service.list_roles(input)?;
    json(StatusCode::OK, &result)
}

/// Pure route/endpoint preview; never launch, adopt or reserve.
#[utoipa::path(
    get, path = "/api/v1/directory-entry", operation_id = "managementInspectDirectory",
    tag = "management",
    params(
        ("generation" = String, Query, description = "Original selected generation"),
        ("directory" = String, Query)
    ),
    responses(
        (status = 200, body = DirectoryEntry),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn inspect_directory(state: &Adapter, input: InspectDirectoryInput) -> Result<HttpResponse, ServiceError> {
    let result: DirectoryEntry = state.service.inspect_directory(input)?;
    json(StatusCode::OK, &result)
}

/// Bind the native get_labels capability without duplicating domain rules.
#[utoipa::path(
    get, path = "/api/v1/agents/{name}/labels", operation_id = "managementGetLabels",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from exact Agent reference"),
        ("generation" = String, Query, description = "Original selected generation"),
        ("incarnation" = Option<String>, Query, description = "Exactly one incarnation or stopped_event required"),
        ("stopped_event" = Option<String>, Query, description = "Exact historical reference, never name fallback")
    ),
    responses(
        (status = 200, body = LabelsSnapshot),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn get_labels(state: &Adapter, input: GetAgentInput) -> Result<HttpResponse, ServiceError> {
    let result: LabelsSnapshot = state.service.get_labels(input)?;
    json(StatusCode::OK, &result)
}

/// Bind the native patch_labels capability without duplicating domain rules.
#[utoipa::path(
    post, path = "/api/v1/agents/{name}/labels", operation_id = "managementPatchLabels",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from exact Agent reference")
    ),
    request_body(content = SubmitLabels, content_type = "application/json"),
    responses(
        (status = 202, body = Operation),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn patch_labels(state: &Adapter, input: SubmitLabels) -> Result<HttpResponse, ServiceError> {
    let result: Operation = state.service.patch_labels(input)?;
    json(StatusCode::ACCEPTED, &result)
}

/// Bind the native list_messages capability without duplicating domain rules.
#[utoipa::path(
    get, path = "/api/v1/agents/{name}/messages", operation_id = "managementListMessages",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from exact Agent reference"),
        ("generation" = String, Query, description = "Original selected generation"),
        ("incarnation" = Option<String>, Query, description = "Exactly one incarnation or stopped_event required"),
        ("stopped_event" = Option<String>, Query, description = "Exact historical reference, never name fallback"),
        ("after" = Option<String>, Query, description = "Native resource-specific opaque cursor"),
        ("limit" = Option<u32>, Query, minimum = 1, maximum = 100, description = "Defaults to 50")
    ),
    responses(
        (status = 200, body = MessagePage),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn list_messages(state: &Adapter, input: AgentPageInput) -> Result<HttpResponse, ServiceError> {
    let result: MessagePage = state.service.list_messages(input)?;
    json(StatusCode::OK, &result)
}

/// Bind the native message_history capability without duplicating domain rules.
#[utoipa::path(
    get, path = "/api/v1/agents/{name}/message-history", operation_id = "managementMessageHistory",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from exact Agent reference"),
        ("generation" = String, Query, description = "Original selected generation"),
        ("incarnation" = Option<String>, Query, description = "Exactly one incarnation or stopped_event required"),
        ("stopped_event" = Option<String>, Query, description = "Exact historical reference, never name fallback"),
        ("before" = Option<String>, Query, description = "Exclusive event bound from the preceding message history page"),
        ("limit" = Option<u32>, Query, minimum = 1, maximum = 100, description = "Defaults to 20")
    ),
    responses(
        (status = 200, body = MessageHistoryPage),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn message_history(state: &Adapter, input: MessageHistoryInput) -> Result<HttpResponse, ServiceError> {
    let result: MessageHistoryPage = state.service.message_history(input)?;
    json(StatusCode::OK, &result)
}

/// Bind the native list_launch_relations capability without duplicating domain rules.
#[utoipa::path(
    get, path = "/api/v1/agents/{name}/launch-relations", operation_id = "managementListLaunchRelations",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from exact Agent reference"),
        ("generation" = String, Query, description = "Original selected generation"),
        ("incarnation" = Option<String>, Query, description = "Exactly one incarnation or stopped_event required"),
        ("stopped_event" = Option<String>, Query, description = "Exact historical reference, never name fallback"),
        ("after" = Option<String>, Query, description = "Native resource-specific opaque cursor"),
        ("limit" = Option<u32>, Query, minimum = 1, maximum = 100, description = "Defaults to 50")
    ),
    responses(
        (status = 200, body = LaunchRelations),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn list_launch_relations(state: &Adapter, input: AgentPageInput) -> Result<HttpResponse, ServiceError> {
    let result: LaunchRelations = state.service.list_launch_relations(input)?;
    json(StatusCode::OK, &result)
}

/// Bind the native list_office_activity capability without duplicating domain rules.
#[utoipa::path(
    get, path = "/api/v1/office-activity", operation_id = "managementListOfficeActivity",
    tag = "management",
    params(
        ("generation" = String, Query, description = "Original selected generation"),
        ("after" = Option<String>, Query, description = "Native resource-specific opaque cursor"),
        ("limit" = Option<u32>, Query, minimum = 1, maximum = 100, description = "Defaults to 50")
    ),
    responses(
        (status = 200, body = OfficeActivity),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn list_office_activity(state: &Adapter, input: PageInput) -> Result<HttpResponse, ServiceError> {
    let result: OfficeActivity = state.service.list_office_activity(input)?;
    json(StatusCode::OK, &result)
}

/// Bind the native list_human_mailbox capability without duplicating domain rules.
#[utoipa::path(
    get, path = "/api/v1/human-mailbox", operation_id = "managementListHumanMailbox",
    tag = "management",
    params(
        ("generation" = String, Query, description = "Original selected generation"),
        ("before" = Option<String>, Query, description = "Native descending mailbox cursor"),
        ("pending" = Option<bool>, Query, description = "Defaults to false"),
        ("limit" = Option<u32>, Query, minimum = 1, maximum = 100)
    ),
    responses(
        (status = 200, body = HumanMailbox),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn list_human_mailbox(state: &Adapter, input: ListHumanMailboxInput) -> Result<HttpResponse, ServiceError> {
    let result: HumanMailbox = state.service.list_human_mailbox(input)?;
    json(StatusCode::OK, &result)
}

/// Monotonic explicit handling; reading/opening a message does not handle it.
#[utoipa::path(
    post, path = "/api/v1/human-mailbox/handle", operation_id = "managementHandleHumanMessages",
    tag = "management",
    request_body(content = HandleHumanMessages, content_type = "application/json"),
    responses(
        (status = 200, body = HumanMailbox),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn handle_human_messages(state: &Adapter, input: HandleHumanMessages) -> Result<HttpResponse, ServiceError> {
    let result: HumanMailbox = state.service.handle_human_messages(input)?;
    json(StatusCode::OK, &result)
}

/// Durable acceptance, not successful process launch. Preserve original request identity for recovery.
#[utoipa::path(
    post, path = "/api/v1/agents", operation_id = "managementSubmitLaunch",
    tag = "management",
    request_body(content = SubmitLaunch, content_type = "application/json"),
    responses(
        (status = 202, body = Operation),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn submit_launch(state: &Adapter, input: SubmitLaunch) -> Result<HttpResponse, ServiceError> {
    let result: Operation = state.service.submit_launch(input)?;
    json(StatusCode::ACCEPTED, &result)
}

/// Resolve and send through native directory policy; creation/adoption semantics stay native.
#[utoipa::path(
    post, path = "/api/v1/directory-messages", operation_id = "managementSubmitDirectoryMessage",
    tag = "management",
    request_body(content = SubmitDirectoryMessage, content_type = "application/json"),
    responses(
        (status = 202, body = Operation),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn submit_directory_message(state: &Adapter, input: SubmitDirectoryMessage) -> Result<HttpResponse, ServiceError> {
    let result: Operation = state.service.submit_directory_message(input)?;
    json(StatusCode::ACCEPTED, &result)
}

/// Historical stop alias shares kill behavior. Returns durable operation; acceptance is not completion.
#[utoipa::path(
    post, path = "/api/v1/agents/{name}/stop", operation_id = "managementStopAgent",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from exact Agent reference")
    ),
    request_body(content = SubmitTarget, content_type = "application/json"),
    responses(
        (status = 202, body = Operation),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn stop_agent(state: &Adapter, input: SubmitTarget) -> Result<HttpResponse, ServiceError> {
    let result: Operation = state.service.submit_target(input, super::service::TargetAction::Kill)?;
    json(StatusCode::ACCEPTED, &result)
}

/// Returns durable operation; acceptance is not completion.
#[utoipa::path(
    post, path = "/api/v1/agents/{name}/kill", operation_id = "managementKillAgent",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from exact Agent reference")
    ),
    request_body(content = SubmitTarget, content_type = "application/json"),
    responses(
        (status = 202, body = Operation),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn kill_agent(state: &Adapter, input: SubmitTarget) -> Result<HttpResponse, ServiceError> {
    let result: Operation = state.service.submit_target(input, super::service::TargetAction::Kill)?;
    json(StatusCode::ACCEPTED, &result)
}

/// Returns durable operation; acceptance is not completion.
#[utoipa::path(
    post, path = "/api/v1/agents/{name}/stop-participation", operation_id = "managementStopParticipation",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from exact Agent reference")
    ),
    request_body(content = SubmitTarget, content_type = "application/json"),
    responses(
        (status = 202, body = Operation),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn stop_participation(state: &Adapter, input: SubmitTarget) -> Result<HttpResponse, ServiceError> {
    let result: Operation = state.service.submit_target(input, super::service::TargetAction::StopParticipation)?;
    json(StatusCode::ACCEPTED, &result)
}

/// Returns durable operation; acceptance is not completion.
#[utoipa::path(
    post, path = "/api/v1/agents/{name}/fork", operation_id = "managementForkAgent",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from exact Agent reference")
    ),
    request_body(content = SubmitTarget, content_type = "application/json"),
    responses(
        (status = 202, body = Operation),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn fork_agent(state: &Adapter, input: SubmitTarget) -> Result<HttpResponse, ServiceError> {
    let result: Operation = state.service.submit_target(input, super::service::TargetAction::Fork)?;
    json(StatusCode::ACCEPTED, &result)
}

/// Returns durable operation; acceptance is not completion.
#[utoipa::path(
    post, path = "/api/v1/agents/{name}/resume", operation_id = "managementResumeAgent",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from exact Agent reference")
    ),
    request_body(content = SubmitTarget, content_type = "application/json"),
    responses(
        (status = 202, body = Operation),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn resume_agent(state: &Adapter, input: SubmitTarget) -> Result<HttpResponse, ServiceError> {
    let result: Operation = state.service.submit_target(input, super::service::TargetAction::Resume)?;
    json(StatusCode::ACCEPTED, &result)
}

/// Returns durable operation; acceptance is not completion.
#[utoipa::path(
    post, path = "/api/v1/agents/{name}/transcript", operation_id = "managementRequestTranscript",
    tag = "management",
    params(
        ("name" = String, Path, description = "Name from exact Agent reference")
    ),
    request_body(content = SubmitTarget, content_type = "application/json"),
    responses(
        (status = 202, body = Operation),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn request_transcript(state: &Adapter, input: SubmitTarget) -> Result<HttpResponse, ServiceError> {
    let result: Operation = state.service.submit_target(input, super::service::TargetAction::Transcript)?;
    json(StatusCode::ACCEPTED, &result)
}

/// Bind the native inspection_snapshot capability without duplicating domain rules.
#[utoipa::path(
    get, path = "/api/v1/inspection", operation_id = "managementInspectionSnapshot",
    tag = "management",
    params(
        ("generation" = String, Query, description = "Original selected generation")
    ),
    responses(
        (status = 200, body = crate::inspection::contract::Snapshot),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn inspection_snapshot(state: &Adapter, input: GenerationInput) -> Result<HttpResponse, ServiceError> {
    let result: crate::inspection::contract::Snapshot = state.service.inspection_snapshot(input)?;
    json(StatusCode::OK, &result)
}

/// Existing externally tagged body must match this action; no Collect/RunAnalysis authority.
#[utoipa::path(
    post, path = "/api/v1/inspection/configure", operation_id = "managementConfigureInspection",
    tag = "management",
    request_body(content = super::openapi::ConfigureBody, content_type = "application/json"),
    responses(
        (status = 200, body = crate::inspection::contract::Snapshot),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn configure_inspection(state: &Adapter, input: crate::inspection::contract::ConfigureInput) -> Result<HttpResponse, ServiceError> {
    let result: crate::inspection::contract::Snapshot = state.service.configure_inspection(input)?;
    json(StatusCode::OK, &result)
}

/// Existing externally tagged body must match this action; no Collect/RunAnalysis authority.
#[utoipa::path(
    post, path = "/api/v1/inspection/analyze", operation_id = "managementAnalyzeInspection",
    tag = "management",
    request_body(content = super::openapi::AnalyzeBody, content_type = "application/json"),
    responses(
        (status = 200, body = crate::inspection::contract::AnalysisJob),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn analyze_inspection(state: &Adapter, input: crate::inspection::contract::AnalyzeInput) -> Result<HttpResponse, ServiceError> {
    let result: crate::inspection::contract::AnalysisJob = state.service.analyze_inspection(input)?;
    json(StatusCode::OK, &result)
}

/// Existing externally tagged body must match this action; no Collect/RunAnalysis authority.
#[utoipa::path(
    post, path = "/api/v1/inspection/cancel-analysis", operation_id = "managementCancelInspectionAnalysis",
    tag = "management",
    request_body(content = super::openapi::CancelAnalysisBody, content_type = "application/json"),
    responses(
        (status = 200, body = crate::inspection::contract::AnalysisJob),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn cancel_inspection_analysis(state: &Adapter, input: crate::inspection::contract::CancelAnalysisInput) -> Result<HttpResponse, ServiceError> {
    let result: crate::inspection::contract::AnalysisJob = state.service.cancel_inspection_analysis(input)?;
    json(StatusCode::OK, &result)
}

/// Existing externally tagged body must match this action; no Collect/RunAnalysis authority.
#[utoipa::path(
    post, path = "/api/v1/inspection/continuation", operation_id = "managementRequestContinuation",
    tag = "management",
    request_body(content = super::openapi::ContinuationBody, content_type = "application/json"),
    responses(
        (status = 200, body = crate::inspection::contract::RecoveryAttempt),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn request_continuation(state: &Adapter, input: crate::inspection::contract::RequestContinuationInput) -> Result<HttpResponse, ServiceError> {
    let result: crate::inspection::contract::RecoveryAttempt = state.service.request_continuation(input)?;
    json(StatusCode::OK, &result)
}

/// Raw application/octet-stream up to 10 MiB; metadata is query-only and native generation is fenced.
#[utoipa::path(
    post, path = "/api/v1/attachments", operation_id = "managementUploadAttachment",
    tag = "management",
    params(
        ("generation" = String, Query, description = "Original selected generation"),
        ("name" = String, Query),
        ("media_type" = Option<String>, Query)
    ),
    request_body(content = super::openapi::BinaryBody, content_type = "application/octet-stream"),
    responses(
        (status = 201, body = Attachment),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn upload_attachment(state: &Adapter, input: UploadAttachmentInput, bytes: &[u8]) -> Result<HttpResponse, ServiceError> {
    let result: Attachment = state.service.upload_attachment(input, bytes)?;
    json(StatusCode::CREATED, &result)
}

/// Raw retained bytes; private host paths are never returned.
#[utoipa::path(
    get, path = "/api/v1/attachments/{id}", operation_id = "managementDownloadAttachment",
    tag = "management",
    params(
        ("id" = String, Path),
        ("generation" = String, Query, description = "Original selected generation")
    ),
    responses(
        (status = 200, content((super::openapi::BinaryBody = "application/octet-stream"), (super::openapi::BinaryBody = "image/png"), (super::openapi::BinaryBody = "image/jpeg"), (super::openapi::BinaryBody = "image/gif"), (super::openapi::BinaryBody = "image/webp")), headers(("Content-Disposition" = String))),
        (status = 400, body = super::openapi::ErrorEnvelope),
        (status = 403, body = super::openapi::ErrorEnvelope),
        (status = 404, body = super::openapi::ErrorEnvelope),
        (status = 409, body = super::openapi::ErrorEnvelope),
        (status = 422, body = super::openapi::ErrorEnvelope),
        (status = 500, body = super::openapi::ErrorEnvelope),
        (status = 503, body = super::openapi::ErrorEnvelope)
    )
)]
fn download_attachment(state: &Adapter, input: GetAttachmentInput) -> Result<HttpResponse, ServiceError> {
    let result = state.service.download_attachment(input)?;
    let raster = ["image/png", "image/jpeg", "image/gif", "image/webp"].contains(&result.attachment.media_type.as_str());
    let disposition = if raster { "inline".to_owned() } else { format!("attachment; filename*=UTF-8''{}", percent_encoding::utf8_percent_encode(&result.attachment.name, percent_encoding::NON_ALPHANUMERIC)) };
    Ok(HttpResponse::builder().header(header::CONTENT_TYPE, if raster { result.attachment.media_type.as_str() } else { "application/octet-stream" }).header(header::CONTENT_DISPOSITION, disposition).body(Body::from(result.bytes)).unwrap())
}
