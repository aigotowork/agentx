//! Public Space document and finite static assets; cloud credentials stay on API routes.
use super::{communication::store, validate_id};
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
    Router::new().route("/namespaces/{id}/space", get(canonical)).route("/namespaces/{id}/space/", get(document)).route("/space/{*asset}", get(resource))
}

#[derive(Deserialize)]
struct Navigation {
    access: Option<String>,
    view: Option<String>,
    host: Option<String>,
    workspace_id: Option<String>,
    group: Option<String>,
    name: Option<String>,
    incarnation: Option<String>,
    generation: Option<String>,
}

async fn canonical(Path(id): Path<String>, uri: Uri) -> Response {
    if !valid_route_id(&id, &uri) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(navigation) = navigation(&uri) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let mut location = format!("/namespaces/{id}/space/");
    let mut pairs = Vec::new();
    if let Some(access @ ("admin" | "namespace")) = navigation.access.as_deref() {
        pairs.push(("access", access));
    }
    if navigation.view.as_deref() == Some("list") {
        pairs.push(("view", "list"));
    }
    if let Some(host) = navigation.host.as_deref() {
        pairs.push(("host", host));
        if let Some(workspace) = navigation.workspace_id.as_deref() {
            pairs.push(("workspace_id", workspace));
        } else if navigation.group.as_deref() == Some("host") {
            pairs.push(("group", "host"));
        }
        if let (Some(name), Some(incarnation)) = (&navigation.name, &navigation.incarnation) {
            pairs.extend([("name", name.as_str()), ("incarnation", incarnation.as_str())]);
            if let Some(generation) = navigation.generation.as_deref() {
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
    if navigation(&uri).is_none() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let mut response = embedded("/space/");
    response.headers_mut().insert(header::CONTENT_SECURITY_POLICY, POLICY.parse().expect("constant CSP"));
    response
}

fn navigation(uri: &Uri) -> Option<Navigation> {
    let Query(query) = Query::<Navigation>::try_from_uri(uri).ok()?;
    let host = query.host.as_deref();
    if host.is_some_and(|host| store::host_name(host).is_err()) || query.workspace_id.as_deref().is_some_and(|id| host.is_none() || store::workspace_id(id).is_err()) {
        return None;
    }
    if query.name.is_some() || query.incarnation.is_some() || query.generation.is_some() {
        store::name(query.name.as_deref()?).ok()?;
        store::incarnation(query.incarnation.as_deref()?).ok()?;
        host?;
        if let Some(generation) = query.generation.as_deref() {
            store::opaque(generation).ok()?;
        }
    }
    Some(query)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn both_space_routes_reject_bad_exact_identity() {
        for query in [
            "host=work&workspace_id=ws_g1IMogNN8O8DXrbzbvh4TOT3_Oxzn-06yjXb6OTskPI",
            "workspace_id=WS0000000001",
            "host=work&name=demo&incarnation=550e8400-e29b-41d4-a716-446655440000",
            "host=work&incarnation=Agent0000001",
            "host=work&workspace_id=WS0000000001&workspace_id=WS0000000002",
        ] {
            let bare = format!("/namespaces/team/space?{query}").parse().unwrap();
            let slash = format!("/namespaces/team/space/?{query}").parse().unwrap();
            assert_eq!(canonical(Path("team".into()), bare).await.status(), StatusCode::BAD_REQUEST);
            assert_eq!(document(Path("team".into()), slash).await.status(), StatusCode::BAD_REQUEST);
        }
        let exact = "host=work&workspace_id=WS0000000001&name=demo&incarnation=Agent0000001";
        let response = canonical(Path("team".into()), format!("/namespaces/team/space?{exact}").parse().unwrap()).await;
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(response.headers()[header::LOCATION], format!("/namespaces/team/space/?{exact}"));
    }
}
