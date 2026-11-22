//! Shared HTTP framing and credential-free browser access. Native scope and
//! operator/actor authority stay with the fixed listener and owning service.
use axum::{extract::Request, http::HeaderMap};
use std::net::SocketAddr;

pub(crate) fn single_header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    Some(value)
}
/// Validate HTTP framing only. The listener captures native scope and authority;
/// Host, Origin and Fetch Metadata never select or grant those capabilities.
pub(crate) fn allowed_boundary(request: &Request) -> Result<(), &'static str> {
    if request.extensions().get::<SocketAddr>().is_none() {
        return Err("WrongHost");
    }
    let host = single_header(request.headers(), "host").ok_or("WrongHost")?;
    let authority = host.parse::<axum::http::uri::Authority>().map_err(|_| "WrongHost")?;
    if authority.host().is_empty() || host.contains('@') || host.contains(',') || host.bytes().any(|b| b.is_ascii_whitespace()) {
        return Err("WrongHost");
    }
    if request.uri().authority().is_some() || request.uri().scheme().is_some() || !request.uri().path().starts_with('/') {
        return Err("InvalidTarget");
    }
    Ok(())
}

/// Credential-free public browser access, including native errors and raw bytes.
/// Actual methods, payloads and authority remain enforced by each owning route.
pub(crate) fn cors(response: &mut axum::response::Response) {
    for (name, value) in [
        ("access-control-allow-origin", "*"),
        ("access-control-allow-methods", "GET, POST, OPTIONS"),
        ("access-control-allow-headers", "*"),
        ("access-control-expose-headers", "Content-Disposition, Content-Type"),
        ("access-control-allow-private-network", "true"),
        ("cross-origin-resource-policy", "cross-origin"),
    ] {
        response.headers_mut().insert(name, axum::http::HeaderValue::from_static(value));
    }
}

/// OPTIONS never reads a daemon or performs native work. Route handlers still
/// validate the actual request and preserve readonly/operator/actor boundaries.
pub(crate) fn preflight(request: &Request) -> Option<axum::response::Response> {
    (request.method() == axum::http::Method::OPTIONS).then(|| axum::response::Response::builder().status(axum::http::StatusCode::NO_CONTENT).body(axum::body::Body::empty()).unwrap())
}
