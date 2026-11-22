//! HTTP deployment boundary. Only this router publishes cloud operations.
mod admin;
mod catalog;
pub(crate) mod client;
mod communication;
pub(crate) mod connection;
mod control;
mod tui;
pub(crate) mod uplink;

use admin::ScopeReceipt;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, OriginalUri, Request, State},
    http::{HeaderMap, Method, StatusCode, Uri, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use catalog::Catalog;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use utoipa::ToSchema;

#[derive(Clone)]
pub(super) struct ScopeContext {
    pub(super) id: String,
    pub(super) root: PathBuf,
    pub(super) generation: String,
    pub(super) credential_version: u32,
    current: tokio::sync::watch::Sender<u32>,
    control_changed: tokio::sync::watch::Sender<()>,
}
impl ScopeContext {
    pub(super) fn valid(&self) -> bool {
        *self.current.borrow() == self.credential_version
    }
}

#[derive(Clone)]
struct Deployment {
    admin_hash: [u8; 32],
    catalog: Arc<Mutex<Catalog>>,
    calls: Arc<tokio::sync::Semaphore>,
}
impl Deployment {
    async fn catalog<T: Send + 'static>(&self, f: impl FnOnce(&mut Catalog) -> Result<T, Failure> + Send + 'static) -> Result<T, Failure> {
        let catalog = self.catalog.clone();
        tokio::task::spawn_blocking(move || {
            let mut owner = catalog.lock().map_err(|_| Failure::internal())?;
            f(&mut owner)
        })
        .await
        .map_err(|_| Failure::internal())?
    }
}

#[derive(Serialize, ToSchema)]
pub(super) struct ApiError {
    code: String,
    message: String,
}
#[derive(Debug)]
pub(super) struct Failure(StatusCode, &'static str, &'static str);
impl Failure {
    fn invalid(message: &'static str) -> Self {
        Self(StatusCode::BAD_REQUEST, "invalid_input", message)
    }
    fn conflict(message: &'static str) -> Self {
        Self(StatusCode::CONFLICT, "conflict", message)
    }
    fn unauthorized() -> Self {
        Self(StatusCode::UNAUTHORIZED, "unauthorized", "invalid bearer credential")
    }
    fn missing() -> Self {
        Self(StatusCode::NOT_FOUND, "not_found", "scope not found")
    }
    fn internal() -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, "internal", "deployment storage unavailable")
    }
}
impl From<rusqlite::Error> for Failure {
    fn from(_: rusqlite::Error) -> Self {
        Self::internal()
    }
}
impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        (self.0, Json(ApiError { code: self.1.into(), message: self.2.into() })).into_response()
    }
}

fn validate_token(value: &str) -> Result<(), Failure> {
    if !(32..=256).contains(&value.len()) || !value.bytes().all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c)) {
        return Err(Failure::invalid("token requires 32 to 256 URL-safe letters, digits, underscores or hyphens"));
    }
    Ok(())
}
fn validate_id(id: &str) -> Result<(), Failure> {
    let b = id.as_bytes();
    if b.is_empty() || b.len() > 63 || !b[0].is_ascii_alphanumeric() || !b[b.len() - 1].is_ascii_alphanumeric() || !b.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-') {
        return Err(Failure::invalid("invalid scope id"));
    }
    Ok(())
}
fn bearer(headers: &HeaderMap) -> Result<&str, Failure> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let raw = values.next().and_then(|v| v.to_str().ok()).and_then(|v| v.split_once(' '));
    match raw {
        Some((scheme, value)) if scheme.eq_ignore_ascii_case("bearer") && values.next().is_none() && !value.is_empty() => Ok(value),
        _ => Err(Failure::unauthorized()),
    }
}
async fn admin_guard(State(state): State<Deployment>, req: Request, next: Next) -> Result<Response, Failure> {
    let given = catalog::hash(bearer(req.headers())?);
    if !catalog::matches(&given, &state.admin_hash) {
        return Err(Failure::unauthorized());
    }
    Ok(next.run(req).await)
}
async fn scope_guard(State(state): State<Deployment>, OriginalUri(uri): OriginalUri, mut req: Request, next: Next) -> Result<Response, Failure> {
    let token = bearer(req.headers())?.to_owned();
    let id = uri.path().split('/').nth(2).ok_or_else(Failure::unauthorized)?.to_owned();
    let ctx = state.catalog(move |db| db.authenticate(&id, &token)).await?;
    req.extensions_mut().insert(ctx);
    Ok(next.run(req).await)
}
async fn bounded(State(state): State<Deployment>, req: Request, next: Next) -> Result<Response, Failure> {
    let _permit = state.calls.try_acquire().map_err(|_| Failure(StatusCode::SERVICE_UNAVAILABLE, "busy", "too many concurrent requests"))?;
    let mut response = next.run(req).await;
    response.headers_mut().insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response.headers_mut().insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    Ok(response)
}
async fn asset(method: Method, uri: Uri) -> Response {
    if method != Method::GET && method != Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    if uri.path() == "/" {
        return Redirect::temporary("/deployment").into_response();
    }
    let key = if uri.path() == "/deployment" { "/" } else { uri.path() };
    let entries: &[(&str, &str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/management_assets.rs"));
    if let Some((_, mime, bytes)) = entries.iter().find(|entry| entry.0 == key) {
        return ([(header::CONTENT_TYPE, *mime), (header::REFERRER_POLICY, "no-referrer")], *bytes).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

fn router(state: Deployment) -> Router {
    let admin = admin::router(state.clone()).layer(middleware::from_fn_with_state(state.clone(), admin_guard));
    let scopes = communication::router().layer(middleware::from_fn_with_state(state.clone(), scope_guard));
    Router::new().nest("/api/v1/admin", admin).nest("/scopes/{id}/api/v1", scopes).fallback(asset).layer(DefaultBodyLimit::max(256 * 1024)).layer(middleware::from_fn_with_state(state, bounded))
}

#[derive(clap::Parser)]
#[command(name = "agentx daemon cloud", about = "Serve the cloud management console and scoped HTTP API behind a TLS reverse proxy")]
pub(crate) struct Serve {
    /// Private persistent deployment directory
    #[arg(long)]
    root: PathBuf,
    #[arg(long, default_value = "127.0.0.1:18755")]
    listen: SocketAddr,
    /// Public HTTPS origin advertised in connection receipts
    #[arg(long)]
    public_url: String,
}
fn serve(args: &[String]) -> anyhow::Result<()> {
    use clap::Parser;
    let opts = Serve::parse_from(std::iter::once("agentx daemon cloud".into()).chain(args.iter().cloned()));
    serve_config(opts)
}

pub(crate) fn serve_config(opts: Serve) -> anyhow::Result<()> {
    anyhow::ensure!(opts.listen.ip().is_loopback(), "--listen must be loopback; terminate public HTTPS at your reverse proxy");
    let url = opts.public_url.trim_end_matches('/');
    let uri = url.parse::<Uri>()?;
    let host = uri.host().unwrap_or("");
    let local = matches!(host, "localhost" | "127.0.0.1" | "[::1]");
    anyhow::ensure!(
        !host.is_empty() && !uri.authority().unwrap().as_str().contains('@') && uri.path() == "/" && uri.query().is_none() && (uri.scheme_str() == Some("https") || local && uri.scheme_str() == Some("http")),
        "--public-url requires an HTTPS origin (loopback HTTP is permitted)"
    );
    let token = std::env::var("AGENTX_ADMIN_TOKEN").map_err(|_| anyhow::anyhow!("AGENTX_ADMIN_TOKEN is required"))?;
    validate_token(&token).map_err(|e| anyhow::anyhow!("{}", e.2))?;
    let catalog = Catalog::open(&opts.root, url.into())?;
    let admin_hash = catalog::hash(&token);
    catalog.check_admin_token(&admin_hash)?;
    let state = Deployment { admin_hash, catalog: Arc::new(Mutex::new(catalog)), calls: Arc::new(tokio::sync::Semaphore::new(128)) };
    tokio::runtime::Builder::new_multi_thread().enable_all().build()?.block_on(async move {
        let listener = tokio::net::TcpListener::bind(opts.listen).await?;
        eprintln!("Cloud daemon listening on {}", listener.local_addr()?);
        axum::serve(listener, router(state)).await?;
        Ok(())
    })
}

#[derive(clap::Parser)]
#[command(name = "agentx tui", about = "Manage cloud scopes in the terminal using AGENTX_ADMIN_TOKEN")]
struct AdminTerminal {
    /// Deployment origin (defaults to AGENTX_ADMIN_URL)
    #[arg(long)]
    admin_url: Option<String>,
}
fn terminal(args: &[String]) -> anyhow::Result<()> {
    use clap::Parser;
    let opts = AdminTerminal::parse_from(std::iter::once("agentx tui".into()).chain(args.iter().cloned()));
    let url = opts.admin_url.or_else(|| std::env::var("AGENTX_ADMIN_URL").ok()).ok_or_else(|| anyhow::anyhow!("set --admin-url or AGENTX_ADMIN_URL"))?;
    tui::run(&url)
}

pub(crate) fn dispatch(argv: &[String]) -> Option<anyhow::Result<()>> {
    match argv {
        [a, b, rest @ ..] if a == "daemon" && b == "cloud" => Some(serve(rest)),
        [a, b] if a == "cloud-openapi" => Some((|| {
            let document = match b.as_str() {
                "admin" => admin::document(),
                "scope" => communication::document(),
                _ => anyhow::bail!("expected admin or scope"),
            };
            println!("{}", serde_json::to_string_pretty(&document)?);
            Ok(())
        })()),
        [a, rest @ ..] if a == "tui" => Some(terminal(rest)),
        [a, ..] if a == "__cloud-host" => client::dispatch(argv),
        _ => None,
    }
}

pub(crate) fn dispatch_scope_client(argv: &[String]) -> Option<anyhow::Result<()>> {
    client::dispatch(argv)
}
