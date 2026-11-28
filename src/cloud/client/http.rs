//! Bounded bearer-authenticated JSON for the native scope and admin clients.

use std::io::Read;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use serde::de::DeserializeOwned;
use serde_json::Value;
use ureq::http::{HeaderValue, Uri};
use ureq::tls::{PemItem, RootCerts, parse_pem};

#[derive(Debug, thiserror::Error)]
#[error("cloud HTTP {status} ({code}): {message}")]
pub(crate) struct HttpError {
    pub(crate) status: u16,
    pub(crate) code: String,
    pub(crate) message: String,
}

#[derive(Clone)]
pub(crate) struct Http {
    base: String,
    authorization: HeaderValue,
    agent: ureq::Agent,
}

impl Http {
    pub(crate) fn new(base_url: &str, token: &str) -> Result<Self> {
        let base = base_url.trim_end_matches('/');
        let uri: Uri = base.parse().map_err(|_| anyhow::anyhow!("invalid cloud URL"))?;
        let host = uri.host().context("cloud URL requires a host")?;
        let loopback = host.eq_ignore_ascii_case("localhost") || host.trim_matches(['[', ']']).parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
        ensure!(uri.scheme_str() == Some("https") || (uri.scheme_str() == Some("http") && loopback), "cloud URL requires HTTPS (HTTP is allowed for loopback only)");
        ensure!(uri.query().is_none() && !base.contains('#') && uri.authority().is_some_and(|authority| !authority.as_str().contains('@')), "cloud URL cannot contain credentials, a query or a fragment");
        let mut authorization = HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| anyhow::anyhow!("invalid cloud bearer"))?;
        ensure!(!token.is_empty() && token.bytes().all(|byte| byte.is_ascii_graphic()), "invalid cloud bearer");
        authorization.set_sensitive(true);
        let mut tls = ureq::tls::TlsConfig::builder().unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::aws_lc_rs::default_provider()));
        if uri.scheme_str() == Some("https")
            && let Some(path) = std::env::var_os("SSL_CERT_FILE")
        {
            tls = tls.root_certs(read_roots(&path)?);
        }
        let agent = ureq::Agent::config_builder().tls_config(tls.build()).http_status_as_error(false).max_redirects(0).timeout_global(Some(Duration::from_secs(8))).proxy(None).build().new_agent();
        Ok(Self { base: base.to_owned(), authorization, agent })
    }

    #[cfg(target_os = "linux")]
    pub(super) fn redact_bearer(&self, value: &mut String) {
        if let Some(token) = self.authorization.to_str().ok().and_then(|header| header.strip_prefix("Bearer "))
            && value.contains(token)
        {
            *value = value.replace(token, "[redacted]");
        }
    }

    pub(crate) fn request<T: DeserializeOwned>(&self, method: &str, path: &str, body: Option<&Value>) -> Result<T> {
        ensure!(path.starts_with('/') && !path.starts_with("//"), "cloud HTTP path must be relative to the configured base");
        let mut request = ureq::http::Request::builder().method(method).uri(format!("{}{path}", self.base)).header("authorization", self.authorization.clone()).header("accept", "application/json");
        let bytes = if let Some(body) = body {
            request = request.header("content-type", "application/json");
            serde_json::to_vec(body)?
        } else {
            Vec::new()
        };
        let mut response = self.agent.run(request.body(bytes)?).context("cloud HTTP request failed")?;
        let status = response.status();
        let reader = response.body_mut().with_config().limit(8 * 1024 * 1024);
        if status.is_success() {
            return reader.read_json().context("invalid cloud JSON response");
        }
        let error: Value = reader.read_json().unwrap_or(Value::Null);
        let code = error["code"].as_str().unwrap_or("http_error").to_owned();
        let message = error["message"].as_str().unwrap_or("request rejected").to_owned();
        Err(HttpError { status: status.as_u16(), code, message }.into())
    }
}

/// An explicitly selected PEM bundle replaces the default WebPKI roots.
/// Certificate identity, chain signatures and hostname still go through rustls.
fn read_roots(path: &std::ffi::OsStr) -> Result<RootCerts> {
    const MAX_CA_FILE: u64 = 8 * 1024 * 1024;
    let path = std::path::Path::new(path);
    ensure!(path.metadata().context("inspect SSL_CERT_FILE")?.is_file(), "SSL_CERT_FILE must be a regular file");
    let file = std::fs::File::open(path).context("open SSL_CERT_FILE")?;
    let mut pem = Vec::new();
    file.take(MAX_CA_FILE + 1).read_to_end(&mut pem).context("read SSL_CERT_FILE")?;
    ensure!(pem.len() as u64 <= MAX_CA_FILE, "SSL_CERT_FILE exceeds 8 MiB");
    let mut roots = Vec::new();
    for item in parse_pem(&pem) {
        if let PemItem::Certificate(certificate) = item.context("parse SSL_CERT_FILE PEM bundle")? {
            roots.push(certificate);
        }
    }
    ensure!(!roots.is_empty(), "SSL_CERT_FILE contains no PEM certificates");
    Ok(roots.into())
}
