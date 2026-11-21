//! Fixed project/scope/mode workbench host; finite API through typed Client.
mod api;
pub(crate) mod openapi;

#[cfg(test)]
mod tests;

use crate::{
    browser::{
        admission::{allowed_boundary, cors, preflight, single_header},
        assets, wire,
    },
    collaboration::service as work,
    service::{InvocationContext, client::Client},
};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{StatusCode, header},
    response::Response,
};
use clap::Parser;
use serde::Serialize;
use std::{
    collections::HashMap,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::Semaphore;

const BODY_LIMIT: usize = 512 * 1024;
const RESPONSE_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Parser)]
#[command(name = "agentx workbench", about = "Open bounded project collaboration with embedded resources")]
struct Launch {
    /// Data scope; defaults to AGENTX_DIR or the normal home scope
    #[arg(long)]
    scope: Option<PathBuf>,
    /// Project directory; defaults to the invoking working directory
    #[arg(long)]
    project: Option<PathBuf>,
    /// Shared invocation option; browser operator calls carry no participant claim
    #[arg(long = "iam")]
    _name: Option<String>,
    /// Enable finite operator mutations for clients reaching this listener
    #[arg(long)]
    collaboration: bool,
    #[arg(long)]
    assets: Option<PathBuf>,
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,
    #[arg(long, default_value_t = 0)]
    port: u16,
}
pub(crate) fn help_text() -> String {
    use clap::CommandFactory;
    Launch::command().render_long_help().to_string()
}
pub(crate) fn dispatch(argv: &[String]) -> Option<i32> {
    let args = match argv.first().map(String::as_str) {
        Some("workbench") => argv[1..].to_vec(),
        Some("help") if argv.get(1).map(String::as_str) == Some("workbench") => vec!["--help".into()],
        _ => return None,
    };
    Some(match Launch::try_parse_from(std::iter::once("agentx workbench".to_owned()).chain(args)) {
        Ok(launch) => match run(launch) {
            Ok(()) => 0,
            Err(message) => {
                eprintln!("agentx workbench: {message}");
                1
            }
        },
        Err(error) => {
            let code = error.exit_code();
            let _ = crate::cli::commands::help::print_clap_error(&error);
            code
        }
    })
}
struct Adapter {
    scope: String,
    project: String,
    invocation: InvocationContext,
    collaboration: bool,
    assets: assets::Assets,
    calls: Arc<Semaphore>,
}
impl Adapter {
    fn client(&self) -> crate::service::ServiceResult<Client> {
        Client::connect_existing(&self.scope, self.invocation.clone())
    }
    fn descriptor(&self) -> api::Descriptor {
        let client = self.client().ok();
        api::Descriptor {
            scope: self.scope.clone(),
            project: self.project.clone(),
            project_label: std::path::Path::new(&self.project).file_name().and_then(|p| p.to_str()).unwrap_or(&self.project).into(),
            generation: client.as_ref().map(|c| c.info().database_generation.clone()),
            collaboration: self.collaboration,
            available: client.is_some(),
            limits: api::Limits {
                text: work::MAX_TEXT as u32,
                title: work::MAX_TITLE as u32,
                source_reference: work::MAX_SOURCE_REFERENCE as u32,
                source_revision: work::MAX_SOURCE_REVISION as u32,
                criteria: work::MAX_CRITERIA as u32,
                artifacts: work::MAX_ARTIFACTS as u32,
                page: work::MAX_PAGE,
                history: work::MAX_HISTORY as u32,
            },
        }
    }
    fn call(&self, request: work::Request) -> work::Response {
        let command_id = match &request {
            work::Request::Apply(call) => Some(call.command_id.clone()),
            _ => None,
        };
        let result = self.client().and_then(|client| client.call(crate::service::Request::Collaboration(request)));
        match result {
            Ok(crate::service::Response::Collaboration(value)) => value,
            Err(crate::service::ServiceError::StaleFence) => work::Response::Rejected(work::Failure::StaleReference),
            Err(crate::service::ServiceError::Forbidden(_) | crate::service::ServiceError::IdentityRequired(_)) => work::Response::Rejected(work::Failure::Forbidden),
            Err(crate::service::ServiceError::InvalidInput(reason)) => work::Response::Rejected(work::Failure::InvalidInput { field: "command".into(), reason }),
            Err(crate::service::ServiceError::OutcomeUnknown(_)) if command_id.is_some() => work::Response::Rejected(work::Failure::OutcomeUnknown { command_id: command_id.expect("apply command present") }),
            _ => work::Response::Rejected(work::Failure::Unavailable),
        }
    }
}
/// Resolve only local launch inputs. HTTP has no path-selection equivalent.
fn launch_paths(launch: &Launch, cwd: &Path, environment: &HashMap<String, String>) -> (PathBuf, PathBuf) {
    let resolve = |path: &Path| if path.is_absolute() { path.to_path_buf() } else { cwd.join(path) };
    let scope = launch.scope.as_deref().map(resolve).unwrap_or_else(|| resolve(&crate::paths::resolve_agentx_dir_from_env(environment, cwd).0));
    let project = launch.project.as_deref().map(resolve).unwrap_or_else(|| cwd.to_path_buf());
    (scope, project)
}

/// Keep originating configuration while removing caller actor/session claims.
/// Every browser request clones this fixed startup invocation.
fn operator_invocation(mut invocation: InvocationContext, project: &str) -> InvocationContext {
    invocation.cwd = project.into();
    invocation.explicit_name = None;
    invocation.process_binding_id = None;
    invocation.provider_session = None;
    invocation.activation_target = None;
    invocation.actor_capability = None;
    invocation.environment.remove(crate::integrations::hooks::claude::actor::ENV_VAR);
    invocation.environment.remove(crate::integrations::hooks::claude::actor::SESSION_ENV_VAR);
    invocation.environment.remove("AGENTX_INSTANCE_NAME");
    for key in crate::runtime::launcher::env_strip_set() {
        invocation.environment.remove(&key);
    }
    invocation
}

fn run(launch: Launch) -> Result<(), String> {
    if launch.port != 0 && launch.port < 1024 {
        return Err("port must be 0 or >=1024".into());
    }
    let cwd = std::env::current_dir().map_err(|error| format!("invoking directory unavailable: {error}"))?;
    let (scope, project) = launch_paths(&launch, &cwd, &std::env::vars().collect());
    let project = project.canonicalize().map_err(|error| format!("project must be an existing directory: {error}"))?;
    if !project.is_dir() {
        return Err("project must be a directory".into());
    }
    let project = project.to_str().ok_or("project must be UTF-8")?.to_owned();
    let assets = match launch.assets {
        Some(directory) => assets::load(&directory)?,
        None => {
            let entries: &[(&str, &str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/workbench_assets.rs"));
            assets::from_entries(entries)
        }
    };
    // Both local workbench modes acquire once. HTTP can only reconnect to this
    // fixed scope; passive observe/observe-web retain their separate policy.
    let invocation = InvocationContext::capture().map_err(|error| format!("cannot capture originating environment: {error}"))?;
    let invocation = operator_invocation(invocation, &project);
    let client = Client::connect(&scope, invocation.clone()).map_err(|error| format!("native service acquisition failed: {error}"))?;
    let scope = client.scope_root().to_str().ok_or("scope must be UTF-8")?.to_owned();
    let state = Arc::new(Adapter { scope, project, invocation, collaboration: launch.collaboration, assets, calls: Arc::new(Semaphore::new(8)) });
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).max_blocking_threads(8).enable_all().build().map_err(|_| "adapter runtime unavailable")?;
    runtime.block_on(serve(state, launch.host, launch.port)).map_err(String::from)
}
async fn serve(state: Arc<Adapter>, host: IpAddr, port: u16) -> Result<(), &'static str> {
    let listener = tokio::net::TcpListener::bind((host, port)).await.map_err(|_| "HTTP listener unavailable")?;
    let address = listener.local_addr().map_err(|_| "listener address unavailable")?;
    {
        use std::io::Write;
        let mut output = std::io::stdout().lock();
        writeln!(output, "http://{address}/").and_then(|_| output.flush()).map_err(|_| "launch address unavailable")?;
    }
    let router = Router::new().fallback(handle).with_state(state);
    let connections = Arc::new(Semaphore::new(8));
    let mut tasks = tokio::task::JoinSet::new();
    let mut shutdown = std::pin::pin!(tokio::signal::ctrl_c());
    loop {
        let accepted = std::future::poll_fn(|cx| {
            use std::{future::Future, task::Poll};
            if shutdown.as_mut().poll(cx).is_ready() {
                return Poll::Ready(None);
            }
            while let Poll::Ready(Some(_)) = tasks.poll_join_next(cx) {}
            listener.poll_accept(cx).map(Some)
        })
        .await;
        let Some(accepted) = accepted else {
            break;
        };
        let (stream, _) = accepted.map_err(|_| "listener failed")?;
        let local = stream.local_addr().map_err(|_| "connection address unavailable")?;
        let Ok(permit) = connections.clone().try_acquire_owned() else {
            continue;
        };
        let service = hyper_util::service::TowerToHyperService::new(router.clone().layer(axum::Extension(local)));
        tasks.spawn(async move {
            let _permit = permit;
            let mut builder = hyper::server::conn::http1::Builder::new();
            builder.timer(hyper_util::rt::TokioTimer::new()).header_read_timeout(Duration::from_secs(5)).max_headers(32).max_buf_size(16 * 1024).keep_alive(false);
            let connection = builder.serve_connection(hyper_util::rt::TokioIo::new(stream), service);
            let _ = tokio::time::timeout(Duration::from_secs(10), connection).await;
        });
    }
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    Ok(())
}
async fn handle(State(state): State<Arc<Adapter>>, request: Request) -> Response {
    let mut response = route(state, request).await;
    for (name, value) in [
        ("cache-control", "no-store"),
        ("pragma", "no-cache"),
        ("referrer-policy", "no-referrer"),
        ("x-content-type-options", "nosniff"),
        ("content-security-policy", "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; object-src 'none'; worker-src 'none'"),
    ] {
        response.headers_mut().insert(name, axum::http::HeaderValue::from_static(value));
    }
    cors(&mut response);
    response
}
async fn route(state: Arc<Adapter>, request: Request) -> Response {
    let path = request.uri().path().to_owned();
    let api = path.starts_with("/api/");
    if let Err(reason) = allowed_boundary(&request) {
        return error(StatusCode::FORBIDDEN, reason);
    }
    if request.uri().query().is_some() {
        return error(StatusCode::FORBIDDEN, "InvalidTarget");
    }
    if let Some(response) = preflight(&request) {
        return response;
    }
    if !api {
        if request.method() != axum::http::Method::GET {
            return error(StatusCode::METHOD_NOT_ALLOWED, "MethodNotAllowed");
        }
        return match state.assets.get(&path) {
            Some(asset) => Response::builder().status(200).header(header::CONTENT_TYPE, &asset.content_type).body(Body::from(asset.bytes.clone())).unwrap(),
            None => error(StatusCode::NOT_FOUND, "NotFound"),
        };
    }
    if request.method() != axum::http::Method::POST {
        return error(StatusCode::METHOD_NOT_ALLOWED, "MethodNotAllowed");
    }
    if !matches!(path.as_str(), "/api/v1/connection" | "/api/v1/read" | "/api/v1/apply") {
        return error(StatusCode::NOT_FOUND, "NotFound");
    }
    if path == "/api/v1/apply" && !state.collaboration {
        return error(StatusCode::FORBIDDEN, "ReadOnly");
    }
    if single_header(request.headers(), "content-type") != Some("application/json") || request.headers().contains_key("content-encoding") {
        return error(StatusCode::UNSUPPORTED_MEDIA_TYPE, "InvalidContentType");
    }
    let body = match tokio::time::timeout(Duration::from_secs(2), to_bytes(request.into_body(), BODY_LIMIT)).await {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(_)) => return error(StatusCode::PAYLOAD_TOO_LARGE, "BodyTooLarge"),
        Err(_) => return error(StatusCode::REQUEST_TIMEOUT, "BodyTimeout"),
    };
    let call = match path.as_str() {
        "/api/v1/connection" => {
            if wire::decode::<crate::browser::openapi::Empty>(&body).is_err() {
                return error(StatusCode::BAD_REQUEST, "InvalidInput");
            }
            None
        }
        "/api/v1/read" => match wire::decode::<api::ReadCall>(&body) {
            Ok(call) => Some(BrowserCall::Read(call)),
            Err(_) => return error(StatusCode::BAD_REQUEST, "InvalidInput"),
        },
        _ => match wire::decode::<api::ApplyCall>(&body) {
            Ok(call) => Some(BrowserCall::Apply(call)),
            Err(_) => return error(StatusCode::BAD_REQUEST, "InvalidInput"),
        },
    };
    let Ok(permit) = state.calls.clone().try_acquire_owned() else {
        return error(StatusCode::SERVICE_UNAVAILABLE, "Busy");
    };
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        match call {
            Some(BrowserCall::Read(call)) => read(&state, call),
            Some(BrowserCall::Apply(call)) => apply(&state, call),
            None => connection(&state),
        }
    })
    .await;
    match result {
        Ok(response) => response,
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "AdapterUnavailable"),
    }
}
fn error(status: StatusCode, reason: &'static str) -> Response {
    json(status, &crate::browser::openapi::HttpError { error: reason.into() })
}
fn json<T: Serialize>(status: StatusCode, value: &T) -> Response {
    match wire::encode(value) {
        Ok(bytes) if bytes.len() <= RESPONSE_LIMIT => Response::builder().status(status).header(header::CONTENT_TYPE, "application/json").body(Body::from(bytes)).unwrap(),
        _ => Response::builder().status(503).header(header::CONTENT_TYPE, "application/json").body(Body::from("{\"error\":\"ResponseUnavailable\"}")).unwrap(),
    }
}

enum BrowserCall {
    Read(api::ReadCall),
    Apply(api::ApplyCall),
}

/// Describe this fixed listener; unavailable generation is retained as null.
#[utoipa::path(
    post, path = "/api/v1/connection", operation_id = "workbenchConnection",
    request_body = crate::browser::openapi::Empty,
    responses(
        (status = 200, body = api::Descriptor),
        (status = 400, body = crate::browser::openapi::HttpError),
        (status = 403, body = crate::browser::openapi::HttpError),
        (status = 404, body = crate::browser::openapi::HttpError),
        (status = 405, body = crate::browser::openapi::HttpError),
        (status = 408, body = crate::browser::openapi::HttpError),
        (status = 413, body = crate::browser::openapi::HttpError),
        (status = 415, body = crate::browser::openapi::HttpError),
        (status = 503, body = crate::browser::openapi::HttpError)
    )
)]
fn connection(state: &Adapter) -> Response {
    json(StatusCode::OK, &state.descriptor())
}

/// Execute the closed five-read native boundary in the fixed project.
#[utoipa::path(
    post, path = "/api/v1/read", operation_id = "workbenchRead",
    request_body = api::ReadCall,
    responses(
        (status = 200, body = work::Response),
        (status = 400, body = crate::browser::openapi::HttpError),
        (status = 403, body = crate::browser::openapi::HttpError),
        (status = 404, body = crate::browser::openapi::HttpError),
        (status = 405, body = crate::browser::openapi::HttpError),
        (status = 408, body = crate::browser::openapi::HttpError),
        (status = 413, body = crate::browser::openapi::HttpError),
        (status = 415, body = crate::browser::openapi::HttpError),
        (status = 503, body = crate::browser::openapi::HttpError)
    )
)]
fn read(state: &Adapter, call: api::ReadCall) -> Response {
    let result: work::Response = state.call(work::Request::Read { generation: call.generation, project: state.project.clone(), read: call.read });
    json(StatusCode::OK, &result)
}

/// Execute only an operator command; a 200 Rejected is not transport failure.
#[utoipa::path(
    post, path = "/api/v1/apply", operation_id = "workbenchApply",
    request_body = api::ApplyCall,
    responses(
        (status = 200, body = work::Response),
        (status = 400, body = crate::browser::openapi::HttpError),
        (status = 403, body = crate::browser::openapi::HttpError),
        (status = 404, body = crate::browser::openapi::HttpError),
        (status = 405, body = crate::browser::openapi::HttpError),
        (status = 408, body = crate::browser::openapi::HttpError),
        (status = 413, body = crate::browser::openapi::HttpError),
        (status = 415, body = crate::browser::openapi::HttpError),
        (status = 503, body = crate::browser::openapi::HttpError)
    )
)]
fn apply(state: &Adapter, call: api::ApplyCall) -> Response {
    let result: work::Response = state.call(work::Request::Apply(work::CommandCall {
        command_id: call.command_id,
        generation: call.generation,
        project: state.project.clone(),
        work: call.work,
        expected_revision: call.expected_revision,
        command: work::Command::Operator(call.command),
    }));
    json(StatusCode::OK, &result)
}
