//! Authored consumer cases; independent native integration owns execution.

use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use clap::Parser;
use serde_json::json;

use crate::daemon::observation::{InspectionResponse, InspectionResult};
use crate::observability::configuration::*;
use crate::observability::context::*;
use crate::observability::events::*;
use crate::observability::runtime::*;

use super::args::{self, ObserveArgs, PreparedCommand};
use super::client::{first_window, next_follow_window};
use super::output::{LocalEvidence, ObserveOutput, ObserveResult, write_record};

const GENERATION: &str = "11111111-1111-4111-8111-111111111111";
const NEXT_GENERATION: &str = "22222222-2222-4222-8222-222222222222";
const SERVER_CLOCK: &str = "33333333-3333-4333-8333-333333333333";
const CLIENT_CLOCK: &str = "44444444-4444-4444-8444-444444444444";
const STREAM: &str = "55555555-5555-4555-8555-555555555555";

// These are explicit semantic/presentation fixtures, never reported as real
// daemon or business executions. Artifact/process scenarios belong to integration.
fn scope() -> ScopeRef {
    let root = if cfg!(windows) { PathBuf::from(r"C:\fixtures\observe") } else { PathBuf::from("/fixtures/observe") };
    ScopeRef::from_canonical_scope(root).unwrap()
}

fn epoch() -> ObservationEpoch {
    ObservationEpoch::from_committed_generation(DaemonRef::from_lease(scope(), "fixture-daemon".into()).unwrap(), GENERATION.into()).unwrap()
}

fn time(clock: &str, ms: u64) -> ObservationTime {
    ObservationTime { clock: ClockId::parse(clock.into()).unwrap(), monotonic_ms: MonotonicMillis(ms), unix_ms: None }
}

fn server_time(ms: u64) -> ObservationTime {
    time(SERVER_CLOCK, ms)
}

fn parsed(argv: &[&str]) -> ObserveArgs {
    ObserveArgs::try_parse_from(std::iter::once("agentx observe").chain(argv.iter().copied())).unwrap()
}

fn inspected_in(epoch: ObservationEpoch, response: InspectionResponse) -> ObserveOutput {
    let sampled_at = server_time(100);
    ObserveOutput {
        command: "inspect",
        requested_scope: Some(epoch.daemon.scope.canonical_scope.clone()),
        scope: Some(ScopeLookup::Existing(epoch.daemon.scope.clone())),
        local: LocalEvidence { inputs: None, attempt: None },
        result: ObserveResult::Inspection(InspectionResult {
            ready: InspectionReady {
                daemon: epoch.daemon,
                protocol_version: 1,
                build_version: "fixture".into(),
                build_id: "fixture".into(),
                process: ObservedProcess { pid: 42, incarnation: None },
                generation: Evidence { value: EvidenceValue::Present(epoch.database_generation), stamp: EvidenceStamp { origin: EvidenceOrigin::DaemonBoundary, at: sampled_at.clone() } },
                sampled_at,
            },
            response,
            received_at: time(CLIENT_CLOCK, 7),
        }),
    }
}

fn inspected(response: InspectionResponse) -> ObserveOutput {
    inspected_in(epoch(), response)
}

fn failure_record(kind: LocalFailureKind) -> ObserveOutput {
    ObserveOutput {
        command: "inspect",
        requested_scope: Some(PathBuf::from("../missing/selection")),
        scope: Some(ScopeLookup::Missing),
        local: LocalEvidence { inputs: None, attempt: None },
        result: ObserveResult::LocalFailure(LocalFailure { stage: ClientStage::ScopeLookup, kind, os_code: Some(2), at: None }),
    }
}

fn encoded(record: &ObserveOutput, json: bool) -> String {
    let mut bytes = Vec::new();
    write_record(&mut bytes, record, json).unwrap();
    String::from_utf8(bytes).unwrap()
}

fn head() -> ObservationHead {
    ObservationHead {
        stream: ObservationStreamId { epoch: epoch(), nonce: STREAM.into() },
        oldest: 1,
        newest: 0,
        observed_at: server_time(100),
        losses: CollectionLosses { sampled_at: server_time(100), dropped: DroppedCounts::default(), evicted: EvictedCounts::default(), suppressed: 0 },
    }
}

fn request() -> RequestObservation {
    let daemon = epoch().daemon;
    RequestObservation {
        reference: RequestObservationRef { daemon: daemon.clone(), id: RequestObservationId(u64::MAX) },
        connection: ConnectionRef { daemon, id: ConnectionId(1) },
        peer: EvidenceValue::Unknown(UnknownReason::NotCaptured),
        origin: None,
        request_id: crate::service::RequestId("original-logical-request".into()),
        operation: OperationLabel { domain: OperationDomain::Messages, variant: "Submit".into() },
        decoded_at: server_time(100),
        enqueued_at: Some(server_time(101)),
        dequeued_at: Some(server_time(102)),
        caller_deadline_unix_ms: None,
        effective_deadline: EvidenceValue::Unknown(UnknownReason::NotCaptured),
        generation_at_decode: GENERATION.into(),
        generation_at_dispatch: Some(GENERATION.into()),
        generation_after_return: EvidenceValue::Present(GENERATION.into()),
        execution: ExecutionObservation::Returned { started_at: server_time(102), ended_at: server_time(110), result: ApplicationResultClass::ReturnedOk },
        reply: ReplyObservation::Observed { wait: ReplyWaitObservation::Abandoned { at: server_time(105), code: RuntimeFailureCode::Deadline }, write: ReplyWriteObservation::Failed { at: server_time(110), code: RuntimeFailureCode::Io } },
        configuration: ConfigurationTrace::NotEvaluated,
        post_commit_work: None,
        last_updated_at: server_time(110),
    }
}

#[test]
fn family_globals_work_before_and_after_every_leaf() {
    let commands: &[&[&str]] = &[
        &[],
        &["inspect"],
        &["request", "--ref-file", "request.json"],
        &["config", "--ref-file", "config.json"],
        &["config-current"],
        &["compare", "--left-file", "left.json", "--right-file", "right.json", "--field", "timeout"],
        &["events", "--follow", "--limit", "256"],
    ];
    for command in commands {
        let before = [vec!["--scope", "../selected", "--json"], command.to_vec()].concat();
        let after = [command.to_vec(), vec!["--scope", "../selected", "--json"]].concat();
        for argv in [before, after] {
            let args = parsed(&argv);
            assert_eq!(args.scope, Some(PathBuf::from("../selected")));
            assert!(args.json);
        }
    }
    assert!(matches!(parsed(&[]).prepare().unwrap(), PreparedCommand::Inspect));
}

#[test]
fn family_rejects_business_identity_flags_and_invalid_limits() {
    for argv in [vec!["--name", "someone"], vec!["--go"], vec!["request"], vec!["events", "--limit", "-1"], vec!["unknown"]] {
        assert!(ObserveArgs::try_parse_from(std::iter::once("observe").chain(argv)).is_err());
    }
    for limit in ["0", "257"] {
        assert!(parsed(&["events", "--limit", limit]).prepare().is_err());
    }
    assert!(matches!(parsed(&["events"]).prepare().unwrap(), PreparedCommand::Events { limit: 128, window: None, follow: false }));
}

#[test]
fn generated_help_has_family_options_at_every_leaf_without_input_reads() {
    for topic in ["inspect", "request", "config", "config-current", "compare", "events"] {
        let text = super::help_text(&[topic]);
        assert!(text.contains("--scope"), "{topic}");
        assert!(text.contains("--json"), "{topic}");
        assert!(text.contains(topic), "{topic}");
    }
    assert!(super::help_text(&[]).contains("config-current"));
}

#[test]
fn native_operands_reject_utf8_unknown_nested_duplicate_and_oversize() {
    let reference = request().reference;
    let valid = serde_json::to_vec(&reference).unwrap();
    assert_eq!(args::decode_operand::<RequestObservationRef>(&valid).unwrap(), reference);
    assert!(args::decode_operand::<RequestObservationRef>(b"\xff").is_err());
    assert!(args::decode_operand::<RequestObservationRef>(&vec![b' '; args::MAX_OPERAND_BYTES + 1]).is_err());
    let mut unknown = serde_json::to_value(&reference).unwrap();
    unknown["daemon"]["scope"]["injected"] = json!("must not become scope authority");
    assert!(args::decode_operand::<RequestObservationRef>(&serde_json::to_vec(&unknown).unwrap()).is_err());
    let duplicate = format!("{{\"daemon\":{},\"id\":1,\"id\":2}}", serde_json::to_string(&reference.daemon).unwrap());
    assert!(args::decode_operand::<RequestObservationRef>(duplicate.as_bytes()).is_err());
}

#[test]
fn explicit_file_operands_validate_before_scope_or_transport_work() {
    let directory = tempfile::tempdir().unwrap();
    let reference_path = directory.path().join("ref.json");
    let mut reference = request().reference;
    reference.id = RequestObservationId(0);
    std::fs::write(&reference_path, serde_json::to_vec(&reference).unwrap()).unwrap();
    let args = parsed(&["request", "--ref-file", reference_path.to_str().unwrap(), "--scope", "missing-do-not-create"]);
    assert!(args.prepare().is_err());
    assert!(args::read_operand::<RequestObservationRef>(directory.path()).is_err());
    std::fs::write(&reference_path, b"{ not JSON").unwrap();
    assert!(args.prepare().is_err());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn native_field_selection_rejects_arbitrary_keys_and_duplicate_fields() {
    assert_eq!(args::field_selector("timeout").unwrap(), ConfigFieldId::setting("timeout").unwrap());
    assert_eq!(args::field_selector("context.cwd").unwrap(), ConfigFieldId::Context(ContextFieldId::Cwd));
    assert!(args::field_selector("OPENAI_API_KEY").is_err());
    let duplicate = ConfigFieldSelection::Only(vec![args::field_selector("timeout").unwrap(); 2]);
    assert!(validate_selection(&duplicate).is_err());
}

#[test]
fn obs01_known_failure_without_time_remains_structured_and_nonzero() {
    let record = failure_record(LocalFailureKind::MissingScope);
    assert_eq!(record.exit_code(), 1);
    let value: serde_json::Value = serde_json::from_str(&encoded(&record, true)).unwrap();
    assert_eq!(value["result"]["LocalFailure"]["stage"], "ScopeLookup");
    assert_eq!(value["result"]["LocalFailure"]["kind"], "MissingScope");
    assert_eq!(value["result"]["LocalFailure"]["os_code"], 2);
    assert!(value["result"]["LocalFailure"]["at"].is_null());
    let text = encoded(&record, false);
    assert!(text.contains("Failure time: unknown"));
    assert!(text.contains("running state and product outcomes remain unknown"));
}

#[test]
fn obs02_obs03_request_and_reply_facts_never_become_effect_confirmation() {
    let record = inspected(InspectionResponse::Runtime(Ok(RuntimeResponse::RequestDetail(request()))));
    let text = encoded(&record, false);
    for retained in ["Execution observation", "ReturnedOk", "Reply observation", "Abandoned", "Failed", "Product effects require", "Clock domains are not subtracted"] {
        assert!(text.contains(retained), "{retained}");
    }
    assert_eq!(record.exit_code(), 0);
}

#[test]
fn json_round_trip_keeps_exact_u64_request_ref_and_one_value_per_line() {
    let reference = request().reference;
    let record = inspected(InspectionResponse::Runtime(Ok(RuntimeResponse::RequestDetail(request()))));
    let bytes = encoded(&record, true);
    assert_eq!(bytes.lines().count(), 1);
    assert!(bytes.ends_with('\n'));
    assert!(bytes.contains("18446744073709551615"));
    let value: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    let extracted = &value["result"]["Inspection"]["response"]["Runtime"]["Ok"]["RequestDetail"]["reference"];
    assert_eq!(serde_json::from_value::<RequestObservationRef>(extracted.clone()).unwrap(), reference);
    assert!(!bytes.contains("Observation:"));
}

#[test]
fn obs04_withheld_comparison_is_not_equal_or_current_reevaluation() {
    let capture = InvocationCaptureRef { invocation: ClientInvocationId::new(), revision: 0 };
    let mut body = ConfigSnapshotBody::invocation(capture, server_time(10), RulesRevision::current("fixture"));
    body.stage = ConfigStage::ScopedEffective;
    body.purpose = ConfigPurpose::ActualResolution;
    body.resolution_scope = EvidenceValue::Present(scope());
    let field = ConfigFieldId::setting("terminal").unwrap();
    body.fields.push(projected_field(field.clone(), EvidenceValue::Withheld, FieldResolution::InputOnly).unwrap());
    let left = ConfigSnapshot { reference: ConfigSnapshotRef { epoch: epoch(), sequence: 1 }, body: body.clone(), provenance: ConfigProvenance::ApplicationResolved, retained_at: server_time(20) };
    let mut right = left.clone();
    right.reference.sequence = 2;
    let result = compare_snapshots(left, right, &ConfigFieldSelection::Only(vec![field])).unwrap();
    let CompareConfigResult::Compared { fields, .. } = &result else {
        panic!("same-meaning fixtures must compare");
    };
    assert!(matches!(fields[0].value, ValueRelation::Unknown(ComparisonUnknown::Withheld)));
    let record = inspected(InspectionResponse::Configuration(Ok(ConfigurationResponse::Comparison(ConfigurationRead { epoch: epoch(), observed_at: server_time(30), result }))));
    assert!(encoded(&record, true).contains("Withheld"));
    assert!(encoded(&record, false).contains("exact retained operands"));
}

#[test]
fn obs04_current_capture_keeps_diagnostic_purpose_and_safe_native_projection() {
    let handle = InvocationCaptureHandle::begin(ClientInvocationId::new());
    let mut invocation = crate::service::InvocationContext { cwd: scope().canonical_scope.clone(), explicit_name: Some("concealed-identity-sentinel".into()), ..Default::default() };
    invocation.environment.insert("OPENAI_API_KEY".into(), "concealed-credential-sentinel".into());
    let snapshot = handle.capture_invocation_inputs(CaptureInputs { invocation: &invocation, at: server_time(10), rules: RulesRevision::current("fixture") }).unwrap();
    let record = ObserveOutput {
        command: "config-current",
        requested_scope: Some(scope().canonical_scope),
        scope: Some(ScopeLookup::Existing(scope())),
        local: LocalEvidence { inputs: Some(Ok(snapshot)), attempt: None },
        result: ObserveResult::ConfigurationCurrent(Err(CaptureSkipReason::Pressure)),
    };
    let text = encoded(&record, true);
    assert!(!text.contains("concealed-identity-sentinel"));
    assert!(!text.contains("concealed-credential-sentinel"));
    assert!(!text.contains("OPENAI_API_KEY"));
    assert!(encoded(&record, false).contains("DiagnosticReevaluation"));
    assert_eq!(record.exit_code(), 1);
}

#[test]
fn obs04_explicit_current_resolution_uses_selected_scope_without_daemon_or_database() {
    let directory = tempfile::tempdir().unwrap();
    let selected = directory.path().canonicalize().unwrap();
    std::fs::write(selected.join("config.toml"), "[preferences]\ntimeout = 37\n").unwrap();
    let invocation = crate::service::InvocationContext { cwd: selected.clone(), ..Default::default() };
    let selected_ref = ScopeRef::from_canonical_scope(selected.clone()).unwrap();
    let capture = InvocationCaptureHandle::begin(ClientInvocationId::new());
    let snapshot = capture
        .capture_current_resolution(CaptureCurrent {
            inputs: CaptureInputs { invocation: &invocation, at: server_time(10), rules: RulesRevision::current("fixture") },
            explicitly_selected_scope: &selected,
            resolution_scope: EvidenceValue::Present(selected_ref),
            source: &crate::config::observation::NativeConfigResolutionSource,
        })
        .unwrap();
    assert_eq!(snapshot.body.purpose, ConfigPurpose::DiagnosticReevaluation);
    assert_eq!(snapshot.body.stage, ConfigStage::ScopedEffective);
    let timeout = snapshot.body.fields.iter().find(|field| field.field == ConfigFieldId::setting("timeout").unwrap()).unwrap();
    assert_eq!(timeout.value, EvidenceValue::Present(ConfigValue::Seconds(37)));
    assert!(!selected.join("agentx.db").exists());
    assert_eq!(std::fs::read_dir(&selected).unwrap().count(), 1);
}

#[test]
fn obs05_empty_page_retains_collection_loss_and_differs_from_gap() {
    let mut head = head();
    head.losses.dropped.contended = 3;
    let next = first_window(&head).unwrap();
    assert_eq!(next.after.sequence, 0);
    assert_eq!(next.through, 0);
    let empty = inspected(InspectionResponse::Events(Ok(ObservationResponse::Read(ObservationReadResult::Page(ObservationPage { records: Vec::new(), next, head: head.clone() })))));
    let gap = inspected(InspectionResponse::Events(Ok(ObservationResponse::Read(ObservationReadResult::Gap { first_missing: 1, last_missing: 4, head }))));
    assert!(empty.event_page().is_some());
    assert!(gap.event_page().is_none());
    assert!(encoded(&empty, true).contains("\"contended\":3"));
    assert!(encoded(&gap, false).contains("Gap missing_sequences=1..=4"));
    assert_eq!(empty.exit_code(), 0);
    assert_eq!(gap.exit_code(), 0); // Complete one-shot diagnosis, not uninterrupted follow.
    assert_eq!(empty.follow_exit_code(), None);
    assert_eq!(gap.follow_exit_code(), Some(1));
}

#[test]
fn obs05_continuation_keeps_filter_frontier_and_does_not_rebase_loss() {
    let mut initial = head();
    initial.oldest = 5;
    initial.newest = 10;
    let mut next = first_window(&initial).unwrap();
    next.after.sequence = 7;
    next.filter.request = Some(RequestObservationId(99));
    let mut later = initial.clone();
    later.oldest = 9;
    later.newest = 30;
    let continued = next_follow_window(&next, &later).unwrap();
    assert_eq!(serde_json::to_value(continued).unwrap(), serde_json::to_value(&next).unwrap());
    next.after.sequence = next.through;
    let tail = next_follow_window(&next, &later).unwrap();
    assert_eq!(tail.after, next.after);
    assert_eq!(tail.through, 30);
    assert_eq!(tail.filter.request, next.filter.request);
}

#[test]
fn obs06_operands_never_select_another_scope_implicitly() {
    let command = PreparedCommand::Request(request().reference);
    assert!(command.validate_scope(&scope()).is_ok());
    let other = ScopeRef::from_canonical_scope(scope().canonical_scope.join("other-scope")).unwrap();
    assert!(command.validate_scope(&other).is_err());
    let mut window = first_window(&head()).unwrap();
    window.filter.configuration = Some(ConfigSnapshotRef { epoch: ObservationEpoch { daemon: DaemonRef { scope: other, daemon_instance: "other".into() }, database_generation: GENERATION.into() }, sequence: 1 });
    assert!(PreparedCommand::Events { window: Some(window), limit: 128, follow: false }.validate_scope(&scope()).is_err());
}

#[test]
fn obs01_obs06_passive_scope_alias_and_missing_selection_have_no_setup_effects() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let resolve = |requested| crate::daemon::observation::resolve_existing_scope(ExistingScopeInput { requested, cwd: root.clone() });
    let canonical = resolve(root.clone());
    let relative = resolve(PathBuf::from("."));
    assert_eq!(serde_json::to_value(canonical).unwrap(), serde_json::to_value(relative).unwrap());
    assert!(matches!(resolve(PathBuf::from("absent/nested")), ScopeLookup::Missing));
    assert!(!root.join("absent").exists());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    #[cfg(unix)]
    {
        let target = root.join("target");
        std::fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, root.join("alias")).unwrap();
        assert_eq!(serde_json::to_value(resolve(PathBuf::from("alias"))).unwrap(), serde_json::to_value(resolve(target)).unwrap());
    }
}

#[test]
fn obs07_pid_reuse_and_unknown_incarnation_stay_distinct_native_rows() {
    let daemon = epoch().daemon;
    let rows: Vec<_> = [Some("first-process"), Some("successor-process"), None]
        .into_iter()
        .enumerate()
        .map(|(index, incarnation)| PeerObservation {
            reference: PeerObservationRef { daemon: daemon.clone(), id: PeerObservationId(index as u64 + 1) },
            process: ObservedProcess { pid: 42, incarnation: incarnation.map(str::to_owned) },
            first_seen_at: server_time(100),
            last_seen_at: server_time(101),
            open_connections: 0,
        })
        .collect();
    let record = inspected(InspectionResponse::Runtime(Ok(RuntimeResponse::Peers(RuntimePage { rows, next_before: None, sampled_at: server_time(110), revision: 1, coverage: RuntimeCoverage::default() }))));
    let value: serde_json::Value = serde_json::from_str(&encoded(&record, true)).unwrap();
    let peers = value["result"]["Inspection"]["response"]["Runtime"]["Ok"]["Peers"]["rows"].as_array().unwrap();
    assert_eq!(peers.len(), 3);
    assert_ne!(peers[0]["reference"], peers[1]["reference"]);
    assert!(peers[2]["process"]["incarnation"].is_null());
}

#[test]
fn obs08_obs09_follow_does_not_rebind_restart_reset_or_replaced_stream() {
    let original = head();
    let window = first_window(&original).unwrap();
    for changed in [
        ObservationStreamId { epoch: ObservationEpoch { daemon: DaemonRef { daemon_instance: "successor-daemon".into(), ..original.stream.epoch.daemon.clone() }, ..original.stream.epoch.clone() }, nonce: STREAM.into() },
        ObservationStreamId { epoch: ObservationEpoch { database_generation: NEXT_GENERATION.into(), ..original.stream.epoch.clone() }, nonce: STREAM.into() },
        ObservationStreamId { nonce: "66666666-6666-4666-8666-666666666666".into(), ..original.stream.clone() },
    ] {
        let changed_head = ObservationHead { stream: changed, ..original.clone() };
        assert_eq!(serde_json::to_value(next_follow_window(&window, &changed_head).unwrap()).unwrap(), serde_json::to_value(&window).unwrap());
    }
}

#[test]
fn obs08_obs09_native_era_outcomes_remain_visible() {
    let old = epoch();
    let mut current = old.clone();
    current.database_generation = NEXT_GENERATION.into();
    let mut changed_head = head();
    changed_head.stream.epoch = current.clone();
    let reset = inspected_in(current.clone(), InspectionResponse::Events(Ok(ObservationResponse::Read(ObservationReadResult::EpochChanged { requested: old.clone(), head: changed_head }))));
    assert!(encoded(&reset, true).contains("EpochChanged"));
    assert!(reset.event_page().is_none());
    let mut restarted = current.clone();
    restarted.daemon.daemon_instance = "new-daemon".into();
    let restart = inspected_in(restarted.clone(), InspectionResponse::DaemonChanged { expected: old.daemon, actual: restarted.daemon });
    assert_eq!(restart.exit_code(), 1);
    assert!(encoded(&restart, true).contains("DaemonChanged"));
    let stale_config = inspected_in(current.clone(), InspectionResponse::Configuration(Err(ConfigurationError::Query(ConfigurationQueryError::EpochChanged { requested: epoch(), current }))));
    assert_eq!(stale_config.exit_code(), 1);
}

#[test]
fn obs10_unavailable_unsupported_evicted_and_busy_are_not_empty_success() {
    for response in [
        InspectionResponse::Runtime(Err(RuntimeReadError::Unsupported)),
        InspectionResponse::Runtime(Err(RuntimeReadError::Busy)),
        InspectionResponse::Runtime(Err(RuntimeReadError::DetailUnavailable(UnknownReason::Evicted))),
        InspectionResponse::Events(Err(ObservationReadError::GenerationUnavailable { daemon: epoch().daemon })),
        InspectionResponse::Configuration(Err(ConfigurationError::Query(ConfigurationQueryError::Unavailable))),
    ] {
        let record = inspected(response);
        assert_eq!(record.exit_code(), 1);
        assert!(record.event_page().is_none());
        assert!(encoded(&record, true).contains("\"Err\""));
    }
    let missing = inspected(InspectionResponse::Configuration(Ok(ConfigurationResponse::Snapshot(ConfigurationRead {
        epoch: epoch(),
        observed_at: server_time(100),
        result: SnapshotLookup::Missing { reference: ConfigSnapshotRef { epoch: epoch(), sequence: 4 }, reason: ConfigMissing::NoLongerRetained { retired_through: 5 } },
    }))));
    assert_eq!(missing.exit_code(), 0);
    assert!(encoded(&missing, true).contains("NoLongerRetained"));
}

#[test]
fn interruption_and_closed_stdout_stop_only_the_consumer_path() {
    struct ClosedOutput;
    impl Write for ClosedOutput {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let error = write_record(&mut ClosedOutput, &failure_record(LocalFailureKind::Unsupported), true).unwrap_err();
    assert_eq!(super::output_error(error), 0);
    assert!(!super::wait_for_poll(&AtomicBool::new(true)));
}
