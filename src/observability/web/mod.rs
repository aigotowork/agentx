//! Explicit one-scope browser transport. Native readers retain all domain policy.
//! See `.docs/observability/browser-api.md` and this module's AGENTS.md.

mod api;
mod assets;
pub(crate) mod openapi;
use crate::browser::{
    admission::{allowed_boundary, cors, preflight, single_header},
    wire,
};

#[cfg(test)]
mod tests;

use crate::observability::{
    context::ScopeRef,
    runtime::{ExistingScopeInput, ScopeLookup},
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
use std::{net::IpAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Semaphore;

const BODY_LIMIT: usize = 32 * 1024;
const RESPONSE_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Parser)]
#[command(name = "agentx observe-web", about = "Serve finite diagnostics for one existing scope")]
struct Launch {
    #[arg(long)]
    scope: PathBuf,
    /// Shared invocation option; diagnostics do not initialize an identity
    #[arg(long = "iam")]
    _name: Option<String>,
    /// Directory containing the finite same-origin manifest.json asset bundle
    #[arg(long)]
    assets: Option<PathBuf>,
    /// Numeric interface address; use 0.0.0.0 for LAN access
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,
    /// HTTP port (0 chooses an available ephemeral port); nonzero must be >=1024
    #[arg(long, default_value_t = 0)]
    port: u16,
}

pub(crate) fn help_text() -> String {
    use clap::CommandFactory;
    Launch::command().render_long_help().to_string()
}

struct Adapter {
    scope: ScopeRef,
    reads: Arc<Semaphore>,
    assets: assets::Assets,
}

/// Early passive dispatch: no ordinary config/identity/service setup is invoked.
pub(crate) fn dispatch(argv: &[String]) -> Option<i32> {
    let args = match argv.first().map(String::as_str) {
        Some("observe-web") => argv[1..].to_vec(),
        Some("help") if argv.get(1).map(String::as_str) == Some("observe-web") => vec!["--help".into()],
        _ => return None,
    };
    Some(match Launch::try_parse_from(std::iter::once("agentx observe-web".to_owned()).chain(args)) {
        Ok(launch) => match run(launch) {
            Ok(()) => 0,
            Err(message) => {
                eprintln!("agentx observe-web: {message}");
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

fn run(launch: Launch) -> Result<(), &'static str> {
    if launch.port != 0 && launch.port < 1024 {
        return Err("port must be 0 or >=1024");
    }
    let cwd = std::env::current_dir().map_err(|_| "current directory unavailable")?;
    let scope = match crate::daemon::observation::resolve_existing_scope(ExistingScopeInput { requested: launch.scope, cwd }) {
        ScopeLookup::Existing(scope) => scope,
        _ => return Err("explicit scope must be an existing readable directory"),
    };
    let assets = match launch.assets {
        Some(directory) => assets::load(&directory)?,
        None => assets::embedded(),
    };
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).max_blocking_threads(8).enable_all().build().map_err(|_| "adapter runtime unavailable")?;
    runtime.block_on(serve(scope, assets, launch.host, launch.port))
}

async fn serve(scope: ScopeRef, assets: assets::Assets, host: IpAddr, port: u16) -> Result<(), &'static str> {
    let listener = tokio::net::TcpListener::bind((host, port)).await.map_err(|_| "HTTP listener unavailable")?;
    let address = listener.local_addr().map_err(|_| "listener address unavailable")?;
    {
        use std::io::Write;
        let mut output = std::io::stdout().lock();
        writeln!(output, "http://{address}/").and_then(|_| output.flush()).map_err(|_| "launch address unavailable")?;
    }
    let state = Arc::new(Adapter { scope, reads: Arc::new(Semaphore::new(8)), assets });
    let router = Router::new().fallback(handle).with_state(state);
    let connections = Arc::new(Semaphore::new(8));
    let mut tasks = tokio::task::JoinSet::new();
    let mut shutdown = std::pin::pin!(tokio::signal::ctrl_c());
    loop {
        // Use std polling instead of adding Tokio's optional proc-macro crate.
        // Reap completed handles before accepting more to bound bookkeeping.
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
    // spawn_blocking native reads cannot be forcibly stopped; runtime drop waits
    // for their native one-second deadline, without stopping any product work.
    Ok(())
}

async fn handle(State(state): State<Arc<Adapter>>, request: Request) -> Response {
    let mut response = route(state, request).await;
    let headers = response.headers_mut();
    for (name, value) in [
        ("cache-control", "no-store"),
        ("pragma", "no-cache"),
        ("referrer-policy", "no-referrer"),
        ("x-content-type-options", "nosniff"),
        ("content-security-policy", "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; object-src 'none'; worker-src 'none'"),
    ] {
        headers.insert(name, axum::http::HeaderValue::from_static(value));
    }
    cors(&mut response);
    response
}

async fn route(state: Arc<Adapter>, request: Request) -> Response {
    let path = request.uri().path().to_owned();
    let api = path.starts_with("/api/");
    if let Err(error) = allowed_boundary(&request) {
        return error_response(StatusCode::FORBIDDEN, error);
    }
    if request.uri().query().is_some() {
        return error_response(StatusCode::FORBIDDEN, "InvalidTarget");
    }
    if let Some(response) = preflight(&request) {
        return response;
    }
    if !api {
        if request.method() != axum::http::Method::GET {
            return error_response(StatusCode::METHOD_NOT_ALLOWED, "MethodNotAllowed");
        }
        return match state.assets.get(&path) {
            Some(asset) => Response::builder().status(200).header(header::CONTENT_TYPE, &asset.content_type).body(Body::from(asset.bytes.clone())).unwrap(),
            None => error_response(StatusCode::NOT_FOUND, "NotFound"),
        };
    }
    if request.method() != axum::http::Method::POST {
        return error_response(StatusCode::METHOD_NOT_ALLOWED, "MethodNotAllowed");
    }
    if !matches!(path.as_str(), "/api/v1/scope" | "/api/v1/read") {
        return error_response(StatusCode::NOT_FOUND, "NotFound");
    }
    if single_header(request.headers(), "content-type") != Some("application/json") || request.headers().contains_key("content-encoding") {
        return error_response(StatusCode::UNSUPPORTED_MEDIA_TYPE, "InvalidContentType");
    }
    let bytes = match tokio::time::timeout(Duration::from_secs(2), to_bytes(request.into_body(), BODY_LIMIT)).await {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(_)) => return error_response(StatusCode::PAYLOAD_TOO_LARGE, "BodyTooLarge"),
        Err(_) => return error_response(StatusCode::REQUEST_TIMEOUT, "BodyTimeout"),
    };
    if path == "/api/v1/scope" {
        if wire::decode::<crate::browser::openapi::Empty>(&bytes).is_err() {
            return error_response(StatusCode::BAD_REQUEST, "InvalidInput");
        }
        return scope(&state);
    }
    let Ok(call) = wire::decode::<api::ReadCall>(&bytes) else {
        return error_response(StatusCode::BAD_REQUEST, "InvalidInput");
    };
    read(state, call).await
}

fn error_response(status: StatusCode, error: &'static str) -> Response {
    json_response(status, &crate::browser::openapi::HttpError { error: error.into() })
}

fn json_response<T: Serialize>(status: StatusCode, value: &T) -> Response {
    match wire::encode(value) {
        Ok(bytes) if bytes.len() <= RESPONSE_LIMIT => Response::builder().status(status).header(header::CONTENT_TYPE, "application/json").body(Body::from(bytes)).unwrap(),
        _ => Response::builder().status(StatusCode::SERVICE_UNAVAILABLE).header(header::CONTENT_TYPE, "application/json").body(Body::from("{\"error\":\"ResponseUnavailable\"}")).unwrap(),
    }
}

/// Return the fixed existing scope without daemon acquisition or configuration.
#[utoipa::path(
    post, path = "/api/v1/scope", operation_id = "observationScope",
    request_body = crate::browser::openapi::Empty,
    responses(
        (status = 200, body = ScopeRef),
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
fn scope(state: &Adapter) -> Response {
    json_response(StatusCode::OK, &state.scope)
}

/// Execute one of thirteen passive reads; preserve LocalFailure and native refusal.
#[utoipa::path(
    post, path = "/api/v1/read", operation_id = "observationRead",
    request_body = api::ReadCall,
    responses(
        (status = 200, body = api::ReadResult),
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
async fn read(state: Arc<Adapter>, call: api::ReadCall) -> Response {
    let (expected, input) = match call.prepare(&state.scope) {
        Ok(call) => call,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, error),
    };
    let Ok(permit) = state.reads.clone().try_acquire_owned() else {
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "Busy");
    };
    let scope = state.scope.clone();
    let result = tokio::task::spawn_blocking(move || {
        // Guards live in the actual native operation, even after HTTP disconnect.
        let _permit = permit;
        match crate::daemon::observation::inspect_existing(&scope, expected, input) {
            Ok(result) => api::ReadResult::Inspection(result),
            Err(failure) => api::ReadResult::LocalFailure(failure),
        }
    })
    .await;
    match result {
        Ok(value) => json_response(StatusCode::OK, &value),
        Err(_) => error_response(StatusCode::SERVICE_UNAVAILABLE, "AdapterUnavailable"),
    }
}
