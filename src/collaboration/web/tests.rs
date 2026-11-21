//! Closed HTTP cases, authored without product/browser execution.
use super::*;

#[test]
fn launch_accepts_explicit_name_without_a_browser_actor_claim() {
    assert!(Launch::try_parse_from(["agentx workbench", "--name", "wb-operator"]).is_err());
    let launch = Launch::try_parse_from(["agentx workbench", "--scope", "/scope", "--project", "/project", "--iam", "wb-operator", "--collaboration"]).unwrap();
    assert_eq!(launch._name.as_deref(), Some("wb-operator"));
    assert!(launch.collaboration);
    let caller = InvocationContext {
        explicit_name: Some("caller".into()),
        process_binding_id: Some("caller-process".into()),
        actor_capability: Some("caller-capability".into()),
        environment: HashMap::from([
            ("AGENTX_TIMEOUT".into(), "17".into()),
            ("EMPTY".into(), "".into()),
            ("AGENTX_PROCESS_ID".into(), "caller-process".into()),
            ("AGENTX_INSTANCE_NAME".into(), "caller".into()),
            (crate::integrations::hooks::claude::actor::ENV_VAR.into(), "caller-actor".into()),
        ]),
        environment_file_keys: ["EMPTY".into()].into(),
        ..Default::default()
    };
    let invocation = operator_invocation(caller, launch.project.unwrap().to_str().unwrap());
    assert_eq!(invocation.cwd, PathBuf::from("/project"));
    assert_eq!(invocation.environment["AGENTX_TIMEOUT"], "17");
    assert_eq!(invocation.environment["EMPTY"], "");
    assert!(invocation.environment_file_keys.contains("EMPTY"));
    for key in ["AGENTX_PROCESS_ID", "AGENTX_INSTANCE_NAME", crate::integrations::hooks::claude::actor::ENV_VAR] {
        assert!(!invocation.environment.contains_key(key));
    }
    assert!(invocation.explicit_name.is_none());
    assert!(invocation.activation_target.is_none());
    assert!(invocation.actor_capability.is_none());
    assert!(invocation.process_binding_id.is_none());
}

#[test]
fn launch_defaults_follow_originating_environment_and_cwd() {
    let launch = Launch::try_parse_from(["agentx workbench"]).unwrap();
    let cwd = Path::new("/work/project");
    let environment = HashMap::from([("HOME".into(), "/home/example".into()), ("AGENTX_DIR".into(), "../state".into())]);
    assert_eq!(launch_paths(&launch, cwd, &environment), (cwd.join("../state"), cwd.into()));
    assert!(!launch.collaboration);
    let environment = HashMap::from([("HOME".into(), "/home/example".into())]);
    assert_eq!(launch_paths(&launch, cwd, &environment).0, PathBuf::from("/home/example/.agentx"));
}

#[test]
fn explicit_launch_paths_override_environment_and_keep_finite_mode_explicit() {
    let launch = Launch::try_parse_from(["agentx workbench", "--scope", "selected", "--project", "other-project", "--collaboration"]).unwrap();
    let cwd = Path::new("/work/project");
    let environment = HashMap::from([("AGENTX_DIR".into(), "/unselected".into())]);
    assert_eq!(launch_paths(&launch, cwd, &environment), (cwd.join("selected"), cwd.join("other-project")));
    assert!(launch.collaboration);
}
fn adapter(collaboration: bool) -> Arc<Adapter> {
    Arc::new(Adapter {
        scope: "/missing-fixture".into(),
        project: "/project".into(),
        invocation: InvocationContext::default(),
        collaboration,
        assets: assets::from_entries(&[("/", "text/html; charset=utf-8", b"fixture")]),
        calls: Arc::new(Semaphore::new(8)),
    })
}
fn request(path: &str, body: &str) -> Request {
    Request::builder()
        .uri(path)
        .method("POST")
        .header("host", "127.0.0.1:18762")
        .header("origin", "http://127.0.0.1:18762")
        .header("content-type", "application/json")
        .extension("127.0.0.1:18762".parse::<std::net::SocketAddr>().unwrap())
        .body(Body::from(body.to_owned()))
        .unwrap()
}
#[test]
fn reports_scope_selection_raw_numbers_and_duplicate_keys_are_not_browser_commands() {
    for body in [
        r#"{"generation":"g","command_id":"c","work":null,"expected_revision":null,"command":{"Report":{"assignment_id":"a","brief_revision":{"$integer":"1"},"report":{}}}}"#,
        r#"{"generation":"g","project":"/other","read":{"List":{"limit":{"$integer":"32"},"after":null}}}"#,
        r#"{"generation":"g","generation":"h","read":"Candidates"}"#,
        r#"{"generation":"g","read":{"List":{"limit":32,"after":null}}}"#,
    ] {
        assert!(wire::decode::<api::ApplyCall>(body.as_bytes()).is_err());
    }
    assert!(wire::decode::<api::ReadCall>(br#"{"generation":"g","project":"/other","read":{"List":{"limit":{"$integer":"32"},"after":null}}}"#).is_err());
    assert!(wire::decode::<api::ReadCall>(br#"{"generation":"g","read":{"List":{"limit":32,"after":null}}}"#).is_err());
}
#[test]
fn readonly_and_input_denials_preserve_public_cors_headers() {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    runtime.block_on(async {
        let readonly = handle(State(adapter(false)), request("/api/v1/apply", "{}")).await;
        assert_eq!(readonly.status(), StatusCode::FORBIDDEN);
        assert_eq!(readonly.headers()["cache-control"], "no-store");
        assert!(readonly.headers().contains_key("content-security-policy"));
        let generic = route(adapter(true), request("/api/v1/native", "{}")).await;
        assert_eq!(generic.status(), StatusCode::NOT_FOUND);
        let mut wrong = request("/api/v1/read", "{}");
        wrong.headers_mut().insert("origin", "http://example.com".parse().unwrap());
        let invalid_origin_input = handle(State(adapter(true)), wrong).await;
        assert_eq!(invalid_origin_input.status(), StatusCode::BAD_REQUEST);
        assert_eq!(invalid_origin_input.headers()["access-control-allow-origin"], "*");
        assert_eq!(readonly.headers()["access-control-allow-origin"], "*");
        let mut options = request("/api/v1/apply", "");
        *options.method_mut() = axum::http::Method::OPTIONS;
        let preflight = handle(State(adapter(false)), options).await;
        assert_eq!(preflight.status(), StatusCode::NO_CONTENT);
        assert_eq!(preflight.headers()["access-control-allow-origin"], "*");
        let invalid = route(adapter(true), request("/api/v1/read", r#"{"generation":"g","read":{"List":{"limit":32,"after":null}}}"#)).await;
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    });
}
