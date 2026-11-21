//! Disposable native boundary cases; integration owns execution, not this author's evidence claim.
use super::*;
use crate::agents::model_request::{ModelRequestState, RequestError, RequestPhase};
use crate::management::contract::AgentRef;
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};
use serde_json::json;

fn setup() -> (tempfile::TempDir, AgentxDb, RequestContext) {
    let root = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    let context = RequestContext {
        caller: CallerContext { request_id: RequestId("collect-test".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
        daemon_instance: "inspection-test".into(),
        database_generation: db.database_generation().unwrap(),
    };
    (root, db, context)
}
fn read(db: &AgentxDb, context: &RequestContext) -> Snapshot {
    let Response::Snapshot(snapshot) = handle(db, context, Request::Read { generation: context.database_generation.clone() }).unwrap() else {
        panic!("snapshot");
    };
    snapshot
}
fn configure(db: &AgentxDb, context: &RequestContext, enabled: bool, recovery: bool) {
    let current = read(db, context);
    handle(db, context, Request::Configure(ConfigureInput { generation: context.database_generation.clone(), expected_revision: current.settings.revision, enabled, recovery: RecoveryPolicy { enabled: recovery, error_duration_seconds: 10 } }))
        .unwrap();
}
fn collect(db: &AgentxDb, context: &RequestContext) -> ServiceResult<Response> {
    handle(db, context, Request::Collect { generation: context.database_generation.clone(), expected_revision: read(db, context).settings.revision, samples: vec![], discovery_complete: true })
}
fn failed(db: &AgentxDb, name: &str) -> Episode {
    let now = crate::shared::time::now_epoch_i64();
    let state = ModelRequestState {
        session_id: format!("session-{name}"),
        turn_id: "turn-a".into(),
        model: "fixture".into(),
        phase: RequestPhase::Failed,
        last_progress_at: Some(now - 100),
        retired_turns: vec![],
        error: Some(RequestError { turn_id: "turn-a".into(), source: "model_request".into(), reason: "fixture failure".into(), error_since: now - 100, last_error_at: now - 100, consecutive_failures: 1 }),
    };
    db.save_instance_named(
        name,
        json!({"tool":"codex", "session_id":state.session_id, "created_at": now as f64 - 200.0, "status":"error", "status_context":"model:request", "status_time":now, "pid":std::process::id(), "model_request":serde_json::to_string(&state).unwrap()})
            .as_object()
            .unwrap(),
    )
    .unwrap();
    recovery::episode(&db.get_instance_full(name).unwrap().unwrap()).unwrap()
}
fn continue_request(context: &RequestContext, episode: Episode) -> Request {
    Request::RequestContinuation(RequestContinuationInput { generation: context.database_generation.clone(), episode })
}

#[test]
fn collection_enable_revision_gaps_and_generation_are_fenced() {
    let (_root, db, context) = setup();
    assert!(!read(&db, &context).settings.enabled);
    assert!(matches!(collect(&db, &context), Err(ServiceError::StaleFence)));
    configure(&db, &context, true, false);
    let initial = read(&db, &context);
    assert!(initial.settings.collected_from.is_some());
    configure(&db, &context, false, false);
    configure(&db, &context, true, false);
    assert!(matches!(handle(&db, &context, Request::Collect { generation: context.database_generation.clone(), expected_revision: initial.settings.revision, samples: vec![], discovery_complete: true }), Err(ServiceError::StaleFence)));
    let next = read(&db, &context);
    assert_eq!(initial.settings.collected_from, next.settings.collected_from);
    assert_eq!(next.coverage.gaps[0].reason, "collection_disabled");
    assert!(next.coverage.gaps[0].through.is_some());
    assert!(matches!(handle(&db, &context, Request::Read { generation: "old".into() }), Err(ServiceError::StaleFence)));
}

#[test]
fn automatic_recovery_has_one_message_per_episode_and_survives_reopen() {
    let (root, db, context) = setup();
    let episode = failed(&db, "kera");
    failed(&db, "lobo");
    configure(&db, &context, true, true);
    collect(&db, &context).unwrap();
    collect(&db, &context).unwrap();
    let first = read(&db, &context);
    assert_eq!(first.attempts.len(), 2);
    assert!(first.attempts.iter().all(|a| a.status == "queued" && a.message_id.is_some()));
    assert_ne!(first.attempts[0].message_id, first.attempts[1].message_id);
    let count: i64 = db.conn().query_row("SELECT COUNT(*) FROM events WHERE type='message'", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 2);
    drop(db);
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    let Response::Attempt(replay) = handle(&db, &context, continue_request(&context, episode)).unwrap() else {
        panic!("attempt");
    };
    assert_eq!(replay.id, first.attempts[0].id);
    let count: i64 = db.conn().query_row("SELECT COUNT(*) FROM events WHERE type='message'", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn retry_started_stale_incarnation_changed_episode_approval_and_threshold_refuse() {
    let (_root, db, context) = setup();
    let expected = failed(&db, "kera");
    configure(&db, &context, true, true);
    let original = db.get_instance_full("kera").unwrap().unwrap();
    let mut changed = original.model_request.as_deref().unwrap().clone();
    changed.phase = RequestPhase::Started;
    db.update_instance_fields("kera", json!({"model_request":serde_json::to_string(&changed).unwrap()}).as_object().unwrap()).unwrap();
    assert!(handle(&db, &context, continue_request(&context, expected.clone())).is_err());
    changed.phase = RequestPhase::Failed;
    changed.error.as_mut().unwrap().error_since += 1;
    db.update_instance_fields("kera", json!({"model_request":serde_json::to_string(&changed).unwrap()}).as_object().unwrap()).unwrap();
    assert!(matches!(handle(&db, &context, continue_request(&context, expected.clone())), Err(ServiceError::StaleFence)));
    db.update_instance_fields("kera", json!({"model_request":serde_json::to_string(&original.model_request).unwrap(),"status":"blocked"}).as_object().unwrap()).unwrap();
    assert!(handle(&db, &context, continue_request(&context, expected.clone())).is_err());
    db.update_instance_fields("kera", json!({"status":"error","created_at": original.created_at + 1.0}).as_object().unwrap()).unwrap();
    assert!(matches!(handle(&db, &context, continue_request(&context, expected.clone())), Err(ServiceError::StaleFence)));
    db.update_instance_fields("kera", json!({"created_at":original.created_at}).as_object().unwrap()).unwrap();
    let mut ledger = load(&db, &context).unwrap();
    ledger.settings.recovery.error_duration_seconds = 600;
    save(&db, &context, &mut ledger).unwrap();
    assert!(handle(&db, &context, continue_request(&context, expected)).is_err());
    assert!(read(&db, &context).attempts.is_empty());
}

#[test]
fn audit_write_failure_rolls_back_message_and_once_barrier_together() {
    let (_root, db, context) = setup();
    let episode = failed(&db, "kera");
    configure(&db, &context, true, true);
    db.conn().execute_batch("CREATE TRIGGER fail_inspection_audit BEFORE INSERT ON kv WHEN NEW.key LIKE 'inspection:%:ledger' BEGIN SELECT RAISE(ABORT, 'fixture audit failure'); END;").unwrap();
    assert!(handle(&db, &context, continue_request(&context, episode.clone())).is_err());
    assert!(get::<RecoveryAttempt>(&db, &attempt_key(&context, &episode)).unwrap().is_none());
    let count: i64 = db.conn().query_row("SELECT COUNT(*) FROM events WHERE type='message'", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 0);
    db.conn().execute_batch("DROP TRIGGER fail_inspection_audit").unwrap();
    assert!(handle(&db, &context, continue_request(&context, episode)).is_ok());
}

#[test]
fn unknown_episode_barrier_never_retries_even_after_restart_or_turn_change() {
    let (root, db, context) = setup();
    let mut episode = failed(&db, "kera");
    configure(&db, &context, true, true);
    let prior = RecoveryAttempt { id: "uncertain".into(), episode: episode.clone(), at: 1, status: "unknown".into(), message_id: None, reason: Some("retained uncertain external outcome".into()) };
    put(&db, &attempt_key(&context, &episode), &prior).unwrap();
    drop(db);
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    episode.failing_turn = "later-failure-same-episode".into();
    let Response::Attempt(result) = handle(&db, &context, continue_request(&context, episode)).unwrap() else {
        panic!("attempt");
    };
    assert_eq!(result.status, "unknown");
    let count: i64 = db.conn().query_row("SELECT COUNT(*) FROM events WHERE type='message'", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn analysis_pins_inputs_and_coverage_reopens_runs_once_and_rejects_id_conflict() {
    let (root, db, context) = setup();
    failed(&db, "kera");
    configure(&db, &context, true, false);
    collect(&db, &context).unwrap();
    let before = read(&db, &context);
    let id = before.observations[0].id.clone();
    let Response::Job(job) = handle(&db, &context, Request::Analyze(AnalyzeInput { generation: context.database_generation.clone(), request_id: "report-a".into(), observation_ids: vec![id.clone()] })).unwrap() else {
        panic!("job");
    };
    assert_eq!(job.status, "queued");
    let mut ledger = load(&db, &context).unwrap();
    ledger.observations.clear();
    save(&db, &context, &mut ledger).unwrap();
    drop(db);
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    for _ in 0..2 {
        let Response::Job(job) = handle(&db, &context, Request::RunAnalysis { generation: context.database_generation.clone(), id: "report-a".into() }).unwrap() else {
            panic!("job");
        };
        assert_eq!(job.status, "succeeded");
        let report = job.report.unwrap();
        assert_eq!(report.algorithm, "native-evidence-v1");
        assert_eq!(report.native_error_count, 1);
        assert_eq!(report.unknown_usage_count, 1);
        assert_eq!(report.observation_ids, vec![id.clone()]);
    }
    assert!(matches!(handle(&db, &context, Request::Analyze(AnalyzeInput { generation: context.database_generation.clone(), request_id: "report-a".into(), observation_ids: vec!["other".into()] })), Err(ServiceError::Conflict(_))));
}

#[test]
fn usage_projection_preserves_full_u64_unknowns_and_shared_source_identity() {
    use crate::transcript::{
        Transcript,
        service::Source,
        shared::{SessionUsage, TokenUsage},
    };
    let source = Source { name: "kera".into(), path: "/fixture/transcript".into(), tool: "codex".into(), session_id: Some("session-a".into()) };
    let agent = AgentRef { name: "kera".into(), incarnation: Some("1".into()), stopped_event: None };
    let transcript = Transcript { exchanges: vec![], usage: Some(SessionUsage { tokens: TokenUsage { input: u64::MAX, output: 7, reasoning: Some(3), ..Default::default() }, ..Default::default() }) };
    let sample = crate::inspection::collection::usage_sample(agent.clone(), &source, &transcript);
    assert_eq!(sample.usage.unwrap().input, u64::MAX.to_string());
    assert!(crate::inspection::collection::usage_sample(agent, &source, &Transcript::default()).usage.is_none());
    let renamed = Source { name: "lobo".into(), ..source.clone() };
    assert_eq!(crate::inspection::collection::source_key(&source), crate::inspection::collection::source_key(&renamed));
}

#[test]
fn retention_eviction_discloses_losses_and_does_not_make_old_ids_analyzable() {
    let (_root, db, context) = setup();
    failed(&db, "kera");
    configure(&db, &context, true, false);
    collect(&db, &context).unwrap();
    let mut ledger = load(&db, &context).unwrap();
    let original = ledger.observations[0].clone();
    for n in 0..MAX_OBSERVATIONS {
        let mut observation = original.clone();
        observation.id = format!("fixture-{n}");
        ledger.observations.push(observation);
    }
    save(&db, &context, &mut ledger).unwrap();
    let after = read(&db, &context);
    assert_eq!(after.observations.len(), MAX_OBSERVATIONS);
    assert_eq!(after.coverage.dropped_observations, 1);
    assert!(after.coverage.gaps.iter().any(|g| g.reason == "retention_eviction"));
    assert!(matches!(handle(&db, &context, Request::Analyze(AnalyzeInput { generation: context.database_generation.clone(), request_id: "old-input".into(), observation_ids: vec![original.id] })), Err(ServiceError::StaleFence)));
}

#[test]
fn stale_session_usage_is_not_admitted_and_cancelled_analysis_cannot_run() {
    let (_root, db, context) = setup();
    let episode = failed(&db, "kera");
    db.update_instance_fields("kera", json!({"transcript_path":"/fixture/source"}).as_object().unwrap()).unwrap();
    configure(&db, &context, true, false);
    let selected = crate::transcript::store::select(&db, "kera", None).unwrap().source.unwrap();
    let sample = UsageSample { agent: episode.agent, session_id: Some("prior-session".into()), source_key: crate::inspection::collection::source_key(&selected), usage: None, availability: "available".into() };
    handle(&db, &context, Request::Collect { generation: context.database_generation.clone(), expected_revision: read(&db, &context).settings.revision, samples: vec![sample], discovery_complete: true }).unwrap();
    let observation = read(&db, &context).observations.pop().unwrap();
    assert!(observation.sample.is_none());
    handle(&db, &context, Request::Analyze(AnalyzeInput { generation: context.database_generation.clone(), request_id: "cancel-me".into(), observation_ids: vec![observation.id] })).unwrap();
    handle(&db, &context, Request::CancelAnalysis(CancelAnalysisInput { generation: context.database_generation.clone(), id: "cancel-me".into() })).unwrap();
    let Response::Job(job) = handle(&db, &context, Request::RunAnalysis { generation: context.database_generation.clone(), id: "cancel-me".into() }).unwrap() else {
        panic!("job");
    };
    assert_eq!(job.status, "cancelled");
    assert!(job.report.is_none());
}
