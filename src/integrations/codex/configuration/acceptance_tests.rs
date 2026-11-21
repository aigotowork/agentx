//! A9 selects only the supported app-server composition and preserves caller
//! configuration semantics. No native executable or service is needed here.

use super::*;

fn arguments(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn acceptance_a9_command_position_never_confuses_values_or_management_with_jsonl() {
    for args in [vec!["app-server"], vec!["-c", "model='app-server'", "app-server"], vec!["--enable", "hooks", "app-server", "--listen", "stdio://"], vec!["app-server", "--listen=stdio://"]] {
        assert!(is_shared_app_server(&arguments(&args)), "supported entry was missed: {args:?}");
    }
    for args in [
        vec!["--version"],
        vec!["-V"],
        vec!["--help"],
        vec!["--model", "app-server", "exec", "literal"],
        vec!["exec", "app-server"],
        vec!["--", "app-server"],
        vec!["app-server", "daemon", "start"],
        vec!["app-server", "proxy", "--sock", "/fixture/native.sock"],
        vec!["app-server", "generate-ts"],
        vec!["app-server", "generate-json-schema"],
        vec!["app-server", "--help"],
        vec!["app-server", "--listen", "unix:///fixture/native.sock"],
        vec!["app-server", "--listen=ws://127.0.0.1:1000"],
    ] {
        assert!(!is_shared_app_server(&arguments(&args)), "unsupported entry selected JSONL composition: {args:?}");
    }
}

#[test]
fn acceptance_a9_thread_config_preserves_literal_values_and_last_override() {
    let input = arguments(&[
        "-c",
        "model='before'",
        "app-server",
        "--listen=stdio://",
        "--config=model='after'",
        "-c",
        "shell_environment_policy.set.LITERAL='text=literal \\中文'",
        "-c",
        "hooks.\"UserPromptSubmit\"=[]",
        "--enable",
        "hooks",
        "--disable",
        "memories",
    ]);
    let overrides = thread_overrides(&input).unwrap();
    let value: Value = serde_json::from_str(&overrides.render().unwrap()).unwrap();
    assert_eq!(value["model"], "after");
    assert_eq!(value["shell_environment_policy"]["set"]["LITERAL"], "text=literal \\中文");
    assert_eq!(value["hooks"]["UserPromptSubmit"], serde_json::json!([]));
    assert_eq!(value["features"]["hooks"], true);
    assert_eq!(value["features"]["memories"], false);
}

#[test]
fn acceptance_a9_untranslated_options_and_missing_values_fail_explicitly() {
    for args in [
        vec!["app-server", "--profile", "named"],
        vec!["app-server", "--future-native-flag"],
        vec!["app-server", "-c"],
        vec!["app-server", "--enable"],
        vec!["app-server", "-c", "without_equals"],
        vec!["app-server", "--listen", "unix:///fixture/socket"],
    ] {
        assert!(thread_overrides(&arguments(&args)).is_err(), "ignored unsupported option: {args:?}");
    }
}

#[test]
fn acceptance_a9_environment_dependencies_reject_normalized_names_even_when_absent() {
    for raw in [
        r#"{"model_providers":{"fixture":{"env_key":"CODEX_TOOL_CALL_ID"}}}"#,
        r#"{"model_providers":{"fixture":{"env_http_headers":{"Authorization":"CODEX_SESSION_ID"}}}}"#,
        r#"{"mcp_servers":{"fixture":{"env_vars":["TERM"]}}}"#,
        r#"{"mcp_servers":{"fixture":{"bearer_token_env_var":"PAGER"}}}"#,
        r#"{"mcp_servers":{"fixture":{"env":{"AGENTX_PROCESS_ID":"literal"}}}}"#,
        r#"{"profiles":{"nested":{"model_providers":{"fixture":{"env_key":"LC_CTYPE"}}}}}"#,
    ] {
        let config = RawObject::parse(raw).unwrap();
        let error = validate_environment_dependencies(&config).unwrap_err().to_string();
        assert!(error.contains("normalized environment input"), "{error}");
    }
}

#[test]
fn acceptance_a9_environment_dependencies_preserve_ordinary_keys_and_opaque_unknown_config() {
    let raw = r#"{"model_providers":{"fixture":{"env_key":"OPENAI_API_KEY","env_http_headers":{"Authorization":"FIXTURE_TOKEN"}}},"mcp_servers":{"fixture":{"env_vars":["HTTPS_PROXY"],"bearer_token_env_var":"MCP_TOKEN","env":{"FIXTURE_LITERAL":"kept"}}},"future":{"env_key":"TERM","id":1e400,"nested":[-0,"中文"]}}"#;
    let config = RawObject::parse(raw).unwrap();
    let opaque = config.get("future").unwrap().get().to_owned();
    validate_environment_dependencies(&config).unwrap();
    assert_eq!(config.get("future").unwrap().get(), opaque);
}

#[test]
fn acceptance_a9_environment_dependency_guard_runs_after_defaults_merge_before_hook_effects() {
    for (defaults, request, expected) in [
        (arguments(&["app-server", "-c", "model_providers.fixture.env_key='CODEX_TOOL_CALL_ID'"]), r#"{"id":"root","method":"thread/start","params":{}}"#, "CODEX_TOOL_CALL_ID"),
        (arguments(&["app-server", "-c", "model_providers.fixture.env_key='OPENAI_API_KEY'"]), r#"{"id":"root","method":"thread/start","params":{"config":{"model_providers":{"fixture":{"env_key":"PAGER"}}}}}"#, "PAGER"),
    ] {
        let config = SessionConfiguration { env: HashMap::new(), defaults: thread_overrides(&defaults).unwrap(), cwd: "/unused-fixture-path".into(), auto_approve: false };
        let participant = ParticipantRef { canonical_name: "lobo".into(), incarnation_bits: 10.0f64.to_bits() };
        let error = config.prepare_root(request, &participant).unwrap_err().to_string();
        assert!(error.contains("normalized environment input") && error.contains(expected), "guard did not precede hook/native work: {error}");
    }
}
