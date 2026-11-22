//! Public Space document and finite static assets; cloud credentials stay on API routes.
use super::validate_id;
use axum::{
    Router,
    extract::{Path, Query},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use serde::Deserialize;

const ASSETS: &[(&str, &str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/agent-space_assets.rs"));
const POLICY: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

pub(super) fn router() -> Router {
    Router::new().route("/scopes/{id}/space", get(canonical)).route("/scopes/{id}/space/", get(document)).route("/space/{*asset}", get(resource))
}

#[derive(Default, Deserialize)]
struct Navigation {
    access: Option<String>,
    view: Option<String>,
    host: Option<String>,
    name: Option<String>,
    incarnation: Option<String>,
    generation: Option<String>,
}

async fn canonical(Path(id): Path<String>, uri: Uri) -> Response {
    if !valid_route_id(&id, &uri) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Query(navigation) = Query::<Navigation>::try_from_uri(&uri).unwrap_or_default();
    let mut location = format!("/scopes/{id}/space/");
    let mut pairs = Vec::new();
    if let Some(access @ ("admin" | "scope")) = navigation.access.as_deref() {
        pairs.push(("access", access));
    }
    if navigation.view.as_deref() == Some("list") {
        pairs.push(("view", "list"));
    }
    if let (Some(host), Some(name), Some(incarnation)) = (&navigation.host, &navigation.name, &navigation.incarnation) {
        use super::communication::store;
        if store::host_name(host).is_ok() && store::name(name).is_ok() && store::opaque(incarnation).is_ok() {
            pairs.extend([("host", host.as_str()), ("name", name.as_str()), ("incarnation", incarnation.as_str())]);
            if let Some(generation) = navigation.generation.as_deref().filter(|value| store::opaque(value).is_ok() && uuid::Uuid::parse_str(value).is_ok()) {
                pairs.push(("generation", generation));
            }
        }
    }
    // Only URL-safe, validated navigation fields can enter Location.
    for (separator, (name, value)) in std::iter::once('?').chain(std::iter::repeat('&')).zip(pairs) {
        location.push(separator);
        location.push_str(name);
        location.push('=');
        location.push_str(value);
    }
    Redirect::temporary(&location).into_response()
}

async fn document(Path(id): Path<String>, uri: Uri) -> Response {
    if !valid_route_id(&id, &uri) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut response = embedded("/space/");
    response.headers_mut().insert(header::CONTENT_SECURITY_POLICY, POLICY.parse().expect("constant CSP"));
    response
}

fn valid_route_id(id: &str, uri: &Uri) -> bool {
    // Keep the shell's ID spelling identical to the API guard's raw path segment.
    validate_id(id).is_ok() && uri.path().split('/').nth(2) == Some(id)
}

async fn resource(uri: Uri) -> Response {
    embedded(uri.path())
}

fn embedded(key: &str) -> Response {
    let Some((_, mime, bytes)) = ASSETS.iter().find(|entry| entry.0 == key) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut response = ([(header::CONTENT_TYPE, *mime), (header::REFERRER_POLICY, "no-referrer")], *bytes).into_response();
    // Vite assigns content hashes in assets/; public resources keep stable URLs elsewhere.
    if key.starts_with("/space/assets/") {
        response.headers_mut().insert(header::CACHE_CONTROL, "public, max-age=31536000, immutable".parse().expect("constant cache control"));
    }
    response
}
