//! Source-side projection/capture cases; execution belongs to native integration.

use super::super::context::{ClockId, MonotonicMillis};
use super::*;
use std::path::PathBuf;

fn at(ms: u64) -> ObservationTime {
    ObservationTime { clock: ClockId::parse("33333333-3333-4333-8333-333333333333".into()).unwrap(), monotonic_ms: MonotonicMillis(ms), unix_ms: None }
}
fn rules() -> RulesRevision {
    RulesRevision::current("test-loaded-build")
}
fn scope(path: &Path) -> ScopeRef {
    ScopeRef::from_canonical_scope(path.to_path_buf()).unwrap()
}
fn effective_body(handle: &InvocationCaptureHandle, path: &Path) -> ConfigSnapshotBody {
    let mut body = ConfigSnapshotBody::invocation(handle.reference(), at(10), rules());
    body.stage = ConfigStage::ScopedEffective;
    body.purpose = ConfigPurpose::ActualResolution;
    body.resolution_scope = EvidenceValue::Present(scope(path));
    body
}
fn trace(path_present: bool) -> ConfigResolutionTrace {
    ConfigResolutionTrace { source_reads: vec![SourceRead::new(1, SourceResource::ConfigToml, at(9), if path_present { SourceReadState::Read } else { SourceReadState::Missing })], fields: Vec::new(), terminal_is_builtin: true }
}
fn field<'a>(body: &'a ConfigSnapshotBody, name: &str) -> &'a FieldEvidence {
    let id = ConfigFieldId::setting(name).unwrap();
    body.fields.iter().find(|f| f.field == id).unwrap()
}

#[test]
fn native_loader_fallback_and_empty_exceptions_survive_safe_projection() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("config.toml"), "[preferences]\ntimeout=31\nauto_approve=true\n[launch]\nauto_subscribe='collision'\n[relay]\nenabled=true\n").unwrap();
    let handle = InvocationCaptureHandle::begin(ClientInvocationId::new());
    for (environment, expected) in
        [(HashMap::from([("AGENTX_TIMEOUT".into(), "bad-secret-candidate".into())]), 86400), (HashMap::from([("AGENTX_TIMEOUT".into(), "".into())]), 86400), (HashMap::from([("AGENTX_TIMEOUT".into(), "7".into())]), 7), (HashMap::new(), 31)]
    {
        let (config, _, observed) = AgentxConfig::load_for_scope_observed(root.path(), &environment);
        let projected = project_effective_config(&config, effective_body(&handle, root.path()), observed).unwrap();
        assert_eq!(field(&projected, "timeout").value, EvidenceValue::Present(ConfigValue::Seconds(expected)));
        assert!(!serde_json::to_string(&projected).unwrap().contains("bad-secret-candidate"));
        match environment.get("AGENTX_TIMEOUT").map(String::as_str) {
            Some("bad-secret-candidate") => {
                assert!(matches!(field(&projected, "timeout").resolution, EvidenceValue::Present(FieldResolution::Defaulted { rejected: ConfigSourceRef { kind: ConfigSourceKind::InvocationEnvironment, .. }, issue: ConfigIssueCode::InvalidInteger })))
            }
            Some("") => assert!(field(&projected, "timeout").steps.iter().any(|step| step.disposition == CandidateDisposition::IgnoredEmpty)),
            _ => assert!(matches!(field(&projected, "timeout").resolution, EvidenceValue::Present(FieldResolution::Effective { .. }))),
        }
    }
    let environment = HashMap::from([("AGENTX_AUTO_APPROVE".into(), "".into()), ("AGENTX_AUTO_SUBSCRIBE".into(), "".into()), ("AGENTX_RELAY_ENABLED".into(), "false".into())]);
    let (config, _, observed) = AgentxConfig::load_for_scope_observed(root.path(), &environment);
    assert!(!config.auto_approve);
    assert_eq!(config.auto_subscribe, "");
    let projected = project_effective_config(&config, effective_body(&handle, root.path()), observed).unwrap();
    assert_eq!(field(&projected, "relay_enabled").value, EvidenceValue::Present(ConfigValue::Boolean(true)));
    assert_eq!(field(&projected, "auto_approve").value, EvidenceValue::Present(ConfigValue::Boolean(false)));
    assert!(matches!(field(&projected, "auto_subscribe").value, EvidenceValue::Withheld));
}

#[test]
fn file_edit_does_not_rewrite_an_actual_use_or_reevaluation_purpose() {
    let root = tempfile::tempdir().unwrap();
    let handle = InvocationCaptureHandle::begin(ClientInvocationId::new());
    let config_path = root.path().join("config.toml");
    std::fs::write(&config_path, "[preferences]\ntimeout=31\n").unwrap();
    let (config, _, observed) = AgentxConfig::load_for_scope_observed(root.path(), &HashMap::new());
    let actual = handle.capture_resolved(project_effective_config(&config, effective_body(&handle, root.path()), observed).unwrap()).unwrap();
    std::fs::write(&config_path, "[preferences]\ntimeout=44\n").unwrap();
    let invocation = crate::service::InvocationContext { cwd: root.path().to_path_buf(), ..Default::default() };
    let later = handle
        .capture_current_resolution(CaptureCurrent {
            inputs: CaptureInputs { invocation: &invocation, at: at(20), rules: rules() },
            explicitly_selected_scope: root.path(),
            resolution_scope: EvidenceValue::Present(scope(root.path())),
            source: &crate::config::observation::NativeConfigResolutionSource,
        })
        .unwrap();
    assert_eq!(field(&actual.body, "timeout").value, EvidenceValue::Present(ConfigValue::Seconds(31)));
    assert_eq!(field(&later.body, "timeout").value, EvidenceValue::Present(ConfigValue::Seconds(44)));
    assert_ne!(actual.reference, later.reference);
    assert_ne!(actual.body.source_reads[0].read_revision, later.body.source_reads[0].read_revision);
    assert!(later.body.consumed_fields.is_empty());
    match compare_local_snapshots(&actual, &later, &ConfigFieldSelection::Only(vec![ConfigFieldId::setting("timeout").unwrap()])).unwrap() {
        LocalConfigComparison::Compared { left_body, right_body, fields, .. } => {
            assert_eq!(left_body.purpose, ConfigPurpose::ActualResolution);
            assert_eq!(right_body.purpose, ConfigPurpose::DiagnosticReevaluation);
            assert_eq!(fields[0].value, ValueRelation::Different);
        }
        _ => panic!("same stage/scope should compare"),
    }
}

#[test]
fn source_trace_explains_shadowed_invalid_and_fallback_without_candidate_text() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("config.toml"), "[preferences]\ntimeout=[1,2]\n").unwrap();
    let handle = InvocationCaptureHandle::begin(ClientInvocationId::new());
    let environment = HashMap::from([("AGENTX_TIMEOUT".into(), "7".into())]);
    let (config, errors, observed) = AgentxConfig::load_for_scope_observed(root.path(), &environment);
    assert!(!errors.contains_key("timeout"));
    let body = project_effective_config(&config, effective_body(&handle, root.path()), observed).unwrap();
    assert_eq!(field(&body, "timeout").value, EvidenceValue::Present(ConfigValue::Seconds(7)));
    assert!(field(&body, "timeout").steps.iter().any(|step| step.source.kind == ConfigSourceKind::ScopeFile && step.disposition == CandidateDisposition::Shadowed && step.validation == CandidateValidation::Invalid(ConfigIssueCode::WrongScalarType)));
    let serialized = serde_json::to_string(&body).unwrap();
    assert!(!serialized.contains("[1,2]"));
}

#[test]
fn projector_drops_all_credentials_and_custom_terminal_even_for_same_ref() {
    let root = tempfile::tempdir().unwrap();
    let handle = InvocationCaptureHandle::begin(ClientInvocationId::new());
    let config = AgentxConfig { relay_psk: "NEVER_RETAIN_PSK".into(), relay_token: "NEVER_RETAIN_TOKEN".into(), hints: "NEVER_RETAIN_HINT".into(), terminal: "cmd {script} secret".into(), ..AgentxConfig::default() };
    let mut source = trace(false);
    source.terminal_is_builtin = false;
    let body = project_effective_config(&config, effective_body(&handle, root.path()), source).unwrap();
    for key in ["relay_psk", "relay_token", "hints", "terminal"] {
        assert!(matches!(field(&body, key).value, EvidenceValue::Withheld));
    }
    assert!(matches!(field(&body, "relay_psk").resolution, EvidenceValue::Withheld));
    assert!(field(&body, "relay_psk").steps.is_empty());
    let serialized = serde_json::to_string(&body).unwrap();
    for secret in ["NEVER_RETAIN", "cmd {script} secret"] {
        assert!(!serialized.contains(secret));
    }
    let snapshot = handle.capture_resolved(body).unwrap();
    match compare_local_snapshots(&snapshot, &snapshot, &ConfigFieldSelection::Only(vec![ConfigFieldId::setting("relay_psk").unwrap()])).unwrap() {
        LocalConfigComparison::Compared { fields, .. } => assert_eq!(fields[0].value, ValueRelation::Unknown(ComparisonUnknown::Withheld)),
        _ => panic!("same safely held snapshot"),
    }
}

#[test]
fn input_capture_never_exports_arbitrary_environment_and_revision_branches_do_not_alias() {
    let handle = InvocationCaptureHandle::begin(ClientInvocationId::new());
    let a = handle.clone().revise().unwrap();
    let b = handle.clone().revise().unwrap();
    assert_ne!(a.reference(), b.reference());
    assert_eq!(a.reference().invocation, b.reference().invocation);
    let invocation = crate::service::InvocationContext {
        cwd: if cfg!(windows) { PathBuf::from(r"C:\fixtures") } else { PathBuf::from("/fixtures") },
        environment: HashMap::from([("PRIVATE_UNKNOWN_API_KEY".into(), "NEVER_DUMP_THIS".into()), ("AGENTX_RELAY_PSK".into(), "NEVER_DUMP_THIS_EITHER".into()), ("AGENTX_TIMEOUT".into(), "7".into())]),
        ..Default::default()
    };
    let snapshot = a.capture_invocation_inputs(CaptureInputs { invocation: &invocation, at: at(0), rules: rules() }).unwrap();
    let serialized = serde_json::to_string(&snapshot).unwrap();
    assert!(!serialized.contains("PRIVATE_UNKNOWN_API_KEY"));
    assert!(!serialized.contains("NEVER_DUMP_THIS"));
    assert!(matches!(snapshot.body.resolution_scope, EvidenceValue::Unknown(UnknownReason::NotCaptured)));
    assert_eq!(field(&snapshot.body, "timeout").value, EvidenceValue::Present(ConfigValue::Seconds(7)));
    let first = snapshot.reference.clone();
    for _ in 0..8 {
        a.capture_invocation_inputs(CaptureInputs { invocation: &invocation, at: at(0), rules: rules() }).unwrap();
    }
    assert!(a.retained(&first).unwrap().is_none());
}

#[test]
fn unestablished_source_scope_is_not_inferred_from_equal_setting_values() {
    let handle = InvocationCaptureHandle::begin(ClientInvocationId::new());
    let mut a = ConfigSnapshotBody::invocation(handle.reference(), at(0), rules());
    a.stage = ConfigStage::ScopedEffective;
    a.purpose = ConfigPurpose::ActualResolution;
    let snapshot = handle.capture_resolved(a).unwrap();
    assert!(matches!(compare_local_snapshots(&snapshot, &snapshot, &ConfigFieldSelection::All).unwrap(), LocalConfigComparison::Incomparable(ComparisonBoundary::ScopeUnestablished)));
}

#[test]
fn capture_allocator_exhaustion_never_recycles_a_local_reference() {
    let handle = InvocationCaptureHandle::begin(ClientInvocationId::new());
    handle.family.next_revision.store(u64::MAX, Ordering::Relaxed);
    assert!(matches!(handle.revise(), Err(CaptureSkipReason::Unsupported)));
    handle.next_snapshot.store(u32::MAX, Ordering::Relaxed);
    let body = ConfigSnapshotBody::invocation(handle.reference(), at(0), rules());
    assert!(matches!(handle.capture_resolved(body), Err(CaptureSkipReason::Unsupported)));
    assert_eq!(handle.reference().revision, 0);
}

#[test]
fn same_pass_trace_preserves_normalization_syntax_failure_and_builtin_override_privacy() {
    let root = tempfile::tempdir().unwrap();
    let handle = InvocationCaptureHandle::begin(ClientInvocationId::new());
    let config_path = root.path().join("config.toml");
    std::fs::write(&config_path, "[terminal]\nactive='WezTerm'\n[terminal.presets.wezterm]\nopen=['echo','{script}']\n").unwrap();
    let environment = HashMap::from([("AGENTX_TIMEOUT".into(), " 0007 ".into()), ("AGENTX_AUTO_APPROVE".into(), "YeS".into())]);
    let (config, errors, observed) = AgentxConfig::load_for_scope_observed(root.path(), &environment);
    assert!(!errors.contains_key("terminal"));
    let projected = project_effective_config(&config, effective_body(&handle, root.path()), observed).unwrap();
    assert!(matches!(field(&projected, "terminal").value, EvidenceValue::Withheld));
    assert!(matches!(field(&projected, "timeout").resolution, EvidenceValue::Present(FieldResolution::Effective { normalized: true, .. })));
    assert_eq!(field(&projected, "timeout").value, EvidenceValue::Present(ConfigValue::Seconds(7)));
    assert_eq!(field(&projected, "auto_approve").value, EvidenceValue::Present(ConfigValue::Boolean(true)));
    assert!(!serde_json::to_string(&projected).unwrap().contains("WezTerm"));

    std::fs::write(&config_path, "[preferences\ntimeout=NEVER_EXPORT_SYNTAX_TEXT").unwrap();
    let (config, _, observed) = AgentxConfig::load_for_scope_observed(root.path(), &HashMap::new());
    let projected = project_effective_config(&config, effective_body(&handle, root.path()), observed).unwrap();
    assert!(projected.source_reads.iter().any(|read| read.state == SourceReadState::SyntaxInvalid));
    assert_eq!(field(&projected, "timeout").value, EvidenceValue::Present(ConfigValue::Seconds(86400)));
    assert!(!serde_json::to_string(&projected).unwrap().contains("NEVER_EXPORT_SYNTAX_TEXT"));
}

#[test]
fn admission_rejects_mismatched_purpose_receipts_and_unbounded_preparation() {
    let handle = InvocationCaptureHandle::begin(ClientInvocationId::new());
    let original = ConfigSnapshotBody::invocation(handle.reference(), at(0), rules());
    let mut wrong_purpose = original.clone();
    wrong_purpose.purpose = ConfigPurpose::ValidatedHostReport;
    assert!(matches!(validate_body(&wrong_purpose), Err(ConfigReportError::InvalidSubject)));
    let mut invalid_receipt = original.clone();
    invalid_receipt.source_reads.push(SourceRead::new(1, SourceResource::ConfigToml, at(0), SourceReadState::Read));
    invalid_receipt.source_reads[0].read_revision = uuid::Uuid::nil().to_string();
    assert!(matches!(validate_body(&invalid_receipt), Err(ConfigReportError::InvalidFields)));
    assert!(serde_json::from_str::<ConfigSettingId>("\"UNKNOWN_PRIVATE_ENV_NAME\"").is_err());

    let config = AgentxConfig::default();
    let mut oversized = trace(false);
    oversized.fields.push(ConfigFieldTrace {
        field: ConfigSettingId::parse("timeout").unwrap(),
        resolution: FieldResolution::Unknown(UnknownReason::NotCaptured),
        steps: vec![ResolutionStep { source: ConfigSourceRef { kind: ConfigSourceKind::BuiltInDefault, read: None }, disposition: CandidateDisposition::Selected, validation: CandidateValidation::Valid }; MAX_STEPS + 1],
    });
    assert!(matches!(project_effective_config(&config, original.clone(), oversized), Err(CaptureSkipReason::Oversize)));

    let first = handle.capture_resolved(original.clone()).unwrap();
    let mut missing_hidden = original;
    missing_hidden.fields.clear();
    let second = handle.capture_resolved(missing_hidden).unwrap();
    match compare_local_snapshots(&first, &second, &ConfigFieldSelection::Only(vec![ConfigFieldId::setting("relay_psk").unwrap()])).unwrap() {
        LocalConfigComparison::Compared { fields, .. } => assert_eq!(fields[0].value, ValueRelation::Unknown(ComparisonUnknown::Withheld)),
        _ => panic!("same input stage"),
    }
}
