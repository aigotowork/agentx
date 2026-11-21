//! Owner-authored cases. Independent consumer integration executes these;
//! no fixture below establishes real daemon or browser acceptance.
use super::*;
use crate::observability::{context::*, runtime::*};
use std::net::SocketAddr;

fn scope() -> ScopeRef {
    ScopeRef::from_canonical_scope(std::env::temp_dir()).unwrap()
}

fn adapter() -> Arc<Adapter> {
    Arc::new(Adapter { scope: scope(), reads: Arc::new(Semaphore::new(8)), assets: std::collections::HashMap::from([("/".into(), assets::Asset { bytes: b"asset fixture".to_vec(), content_type: "text/html; charset=utf-8".into() })]) })
}

fn request(path: &str, body: &str) -> Request {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("host", "127.0.0.1:18761")
        .header("origin", "http://127.0.0.1:18761")
        .header("content-type", "application/json")
        .extension("127.0.0.1:18761".parse::<SocketAddr>().unwrap())
        .body(Body::from(body.to_owned()))
        .unwrap()
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
}

async fn value(response: Response) -> serde_json::Value {
    serde_json::from_slice(&to_bytes(response.into_body(), RESPONSE_LIMIT).await.unwrap()).unwrap()
}

#[test]
fn integer_codec_preserves_u64_i64_strings_and_rejects_ambiguous_values() {
    let native = serde_json::json!({"max":u64::MAX,"min":i64::MIN,"text":"18446744073709551615","unknown":"Withheld"});
    let encoded = wire::encode(&native).unwrap();
    let restored: serde_json::Value = wire::decode(&encoded).unwrap();
    assert_eq!(restored, native);
    for input in [r#"{"$integer":"01"}"#, r#"{"$integer":"-0"}"#, r#"{"$integer":"18446744073709551616"}"#, r#"{"$integer":"1","other":true}"#, "1", "1.0", r#"{"x":true,"x":false}"#] {
        assert!(wire::decode::<serde_json::Value>(input.as_bytes()).is_err(), "{input}");
    }
}

#[test]
fn finite_api_rejects_mutation_unknown_nested_fields_and_cross_scope_refs() {
    for input in [
        r#"{"read":{"ReportClient":{}},"expected_daemon":null}"#,
        r#"{"read":{"ReportSafeCapture":{}},"expected_daemon":null}"#,
        r#"{"read":"Snapshot","scope":"/other"}"#,
        r#"{"read":{"Connections":{"limit":{"$integer":"1"},"before":null,"report":true}}}"#,
        r#"{"read":{"EventsRead":{"window":{},"limit":{"$integer":"1"}}}}"#,
    ] {
        assert!(wire::decode::<api::ReadCall>(input.as_bytes()).is_err());
    }
    let selected = scope();
    let other = ScopeRef::from_canonical_scope(selected.canonical_scope.join("other")).unwrap();
    let daemon = DaemonRef::from_lease(other, "other-instance".into()).unwrap();
    let input = api::ReadCall { expected_daemon: Some(daemon), read: api::BrowserRead::Snapshot };
    assert!(input.prepare(&selected).is_err());
    for limit in [0, 65] {
        assert!(api::ReadCall { expected_daemon: None, read: api::BrowserRead::Connections(RuntimeList { before: None, limit }) }.prepare(&selected).is_err());
    }
}

#[test]
fn public_origins_and_fetch_metadata_reach_native_input_validation() {
    runtime().block_on(async {
        for origin in [None, Some("null"), Some("http://example.com"), Some("chrome-extension://example")] {
            let mut req = request("/api/v1/read", "{}");
            req.headers_mut().remove("origin");
            if let Some(origin) = origin {
                req.headers_mut().insert("origin", origin.parse().unwrap());
            }
            req.headers_mut().insert("sec-fetch-site", "cross-site".parse().unwrap());
            req.headers_mut().insert("host", "localhost:18761".parse().unwrap());
            let response = handle(State(adapter()), req).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert_eq!(response.headers()["access-control-allow-origin"], "*");
        }
    });
}

#[test]
fn preflight_is_public_and_independent_of_native_availability() {
    runtime().block_on(async {
        let mut req = request("/api/v1/read", "ignored body");
        *req.method_mut() = axum::http::Method::OPTIONS;
        req.headers_mut().insert("origin", "null".parse().unwrap());
        req.headers_mut().insert("access-control-request-method", "POST".parse().unwrap());
        req.headers_mut().insert("access-control-request-headers", "content-type, x-client-test".parse().unwrap());
        req.headers_mut().insert("access-control-request-private-network", "true".parse().unwrap());
        let response = handle(State(adapter()), req).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(response.headers()["access-control-allow-origin"], "*");
        assert_eq!(response.headers()["access-control-allow-headers"], "*");
        assert_eq!(response.headers()["access-control-allow-private-network"], "true");
        assert!(!response.headers().contains_key("access-control-allow-credentials"));
        assert!(to_bytes(response.into_body(), 1).await.unwrap().is_empty());
    });
}

#[test]
fn scope_is_repeatable_credential_free_and_preserves_security_headers() {
    runtime().block_on(async {
        let state = adapter();
        for _ in 0..3 {
            let response = handle(State(state.clone()), request("/api/v1/scope", "{}")).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert!(response.headers().get("set-cookie").is_none());
            assert_eq!(response.headers()["access-control-allow-origin"], "*");
            assert_eq!(value(response).await, serde_json::to_value(&state.scope).unwrap());
        }
        assert_eq!(handle(State(state.clone()), request("/api/v1/scope", r#"{"scope":"/other"}"#)).await.status(), StatusCode::BAD_REQUEST);
    });
}

#[test]
fn host_aliases_preserve_fixed_scope_but_invalid_http_framing_is_rejected() {
    for host in ["localhost:18761", "example.com", "192.168.1.20:18761", "[::1]:18761", "[2001:db8::1]:18761"] {
        let mut req = request("/api/v1/scope", "{}");
        req.headers_mut().insert("host", host.parse().unwrap());
        assert!(allowed_boundary(&req).is_ok());
    }
    for host in ["", "a b", "user@example.com", "a,b"] {
        let mut req = request("/api/v1/scope", "{}");
        req.headers_mut().insert("host", host.parse().unwrap());
        assert_eq!(allowed_boundary(&req), Err("WrongHost"));
    }
    let mut req = request("/api/v1/scope", "{}");
    req.headers_mut().append("host", "other".parse().unwrap());
    assert_eq!(allowed_boundary(&req), Err("WrongHost"));
    req.headers_mut().remove("host");
    assert_eq!(allowed_boundary(&req), Err("WrongHost"));
    let req = request("http://example.com/api/v1/scope", "{}");
    assert_eq!(allowed_boundary(&req), Err("InvalidTarget"));
}

#[test]
fn routes_cannot_tunnel_mutation_files_query_or_wrong_media() {
    runtime().block_on(async {
        let state = adapter();
        for path in ["/api/v1/report", "/api/v1/runtime", "/api/v1/read/file", "/api/v1/mutate"] {
            assert_eq!(handle(State(state.clone()), request(path, "{}")).await.status(), StatusCode::NOT_FOUND);
        }
        assert_eq!(handle(State(state.clone()), request("/api/v1/read?scope=/other", "{}")).await.status(), StatusCode::FORBIDDEN);
        let mut req = request("/api/v1/read", "{}");
        req.headers_mut().insert("content-type", "text/plain".parse().unwrap());
        assert_eq!(handle(State(state.clone()), req).await.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        let req = request("/api/v1/read", &"x".repeat(BODY_LIMIT + 1));
        assert_eq!(handle(State(state), req).await.status(), StatusCode::PAYLOAD_TOO_LARGE);
    });
}

#[test]
fn bounded_reads_share_capacity_without_global_event_poll_exclusivity() {
    runtime().block_on(async {
        let state = adapter();
        let all = state.reads.clone().acquire_many_owned(8).await.unwrap();
        assert_eq!(handle(State(state.clone()), request("/api/v1/read", r#"{"read":"Snapshot"}"#)).await.status(), StatusCode::SERVICE_UNAVAILABLE);
        // Scope discovery remains independent of native worker pressure.
        assert_eq!(handle(State(state.clone()), request("/api/v1/scope", "{}")).await.status(), StatusCode::OK);
        assert_eq!(state.reads.available_permits(), 0);
        drop(all);
        assert_eq!(state.reads.available_permits(), 8);
    });
}

#[test]
fn static_assets_are_exact_manifest_entries_and_use_security_headers() {
    runtime().block_on(async {
        let state = adapter();
        for (path, expected) in [("/", StatusCode::OK), ("/../secret", StatusCode::NOT_FOUND), ("/manifest.json", StatusCode::NOT_FOUND)] {
            let req = Request::builder().uri(path).header("host", "127.0.0.1:18761").extension("127.0.0.1:18761".parse::<SocketAddr>().unwrap()).body(Body::empty()).unwrap();
            let response = handle(State(state.clone()), req).await;
            assert_eq!(response.status(), expected);
            assert!(!response.headers().contains_key("x-frame-options"));
            assert!(!response.headers()["content-security-policy"].to_str().unwrap().contains("frame-ancestors"));
            assert_eq!(response.headers()["cross-origin-resource-policy"], "cross-origin");
        }
    });
}

#[test]
fn asset_loader_rejects_missing_root_traversal_and_unknown_manifest_fields() {
    let root = tempfile::tempdir().unwrap();
    assert!(assets::load(root.path()).is_err());
    std::fs::write(root.path().join("manifest.json"), r#"{"files":[],"extra":true}"#).unwrap();
    assert!(assets::load(root.path()).is_err());
    std::fs::write(root.path().join("manifest.json"), r#"{"files":[{"url":"/","file":"../secret","content_type":"text/html; charset=utf-8"}]}"#).unwrap();
    assert!(assets::load(root.path()).is_err());
}

#[test]
fn native_unknown_withheld_gaps_errors_and_coverage_are_not_flattened() {
    use crate::observability::events::*;
    let daemon = DaemonRef::from_lease(scope(), "codec-fixture".into()).unwrap();
    let epoch = ObservationEpoch::from_committed_generation(daemon.clone(), uuid::Uuid::new_v4().to_string()).unwrap();
    let at = ObservationClock::new().now().unwrap();
    let head = ObservationHead {
        stream: ObservationStreamId { epoch: epoch.clone(), nonce: uuid::Uuid::new_v4().to_string() },
        oldest: 5,
        newest: u64::MAX,
        observed_at: at.clone(),
        losses: CollectionLosses { sampled_at: at, dropped: DroppedCounts::default(), evicted: EvictedCounts::default(), suppressed: 17 },
    };
    for result in [
        ObservationReadResult::Gap { first_missing: 1, last_missing: 4, head: head.clone() },
        ObservationReadResult::EpochChanged { requested: epoch, head: head.clone() },
        ObservationReadResult::StreamChanged { requested: head.stream.clone(), head },
    ] {
        let native = ObservationResponse::Read(result);
        let restored: ObservationResponse = wire::decode(&wire::encode(&native).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), serde_json::to_value(native).unwrap());
    }
    for evidence in [EvidenceValue::<String>::Unknown(UnknownReason::GenerationUnavailable), EvidenceValue::Withheld, EvidenceValue::Absent] {
        assert_eq!(wire::decode::<EvidenceValue<String>>(&wire::encode(&evidence).unwrap()).unwrap(), evidence);
    }
    let unavailable = api::ReadResult::LocalFailure(LocalFailure { kind: LocalFailureKind::Unsupported, stage: ClientStage::Connect, os_code: None, at: None });
    let mapped: serde_json::Value = wire::decode(&wire::encode(&unavailable).unwrap()).unwrap();
    assert_eq!(mapped["LocalFailure"]["kind"], "Unsupported");
    assert!(mapped["LocalFailure"]["at"].is_null());
}

#[test]
fn launch_requires_scope_defaults_to_embedded_loopback_and_accepts_numeric_lan() {
    assert!(Launch::try_parse_from(["observe-web"]).is_err());
    let launch = Launch::try_parse_from(["observe-web", "--scope", "/selected"]).unwrap();
    assert!(launch.assets.is_none());
    assert_eq!(launch.host, "127.0.0.1".parse::<IpAddr>().unwrap());
    for host in ["0.0.0.0", "192.168.1.20", "::", "::1"] {
        assert!(Launch::try_parse_from(["observe-web", "--scope", "/selected", "--host", host]).is_ok());
    }
    assert!(Launch::try_parse_from(["observe-web", "--scope", "/selected", "--host", "example.com"]).is_err());
    assert!(Launch::try_parse_from(["observe-web", "--iam", "nano", "--scope", "/selected"]).is_ok());
    assert!(Launch::try_parse_from(["observe-web", "--name", "nano", "--scope", "/selected"]).is_err());
    assert!(help_text().contains("--assets"));
    assert_eq!(crate::cli::commands::help::help_topic("observe-web", &["--scope".into()]), "observe-web");
    assert!(crate::cli::commands::help::get_command_help("observe-web").contains("--scope"));
}
