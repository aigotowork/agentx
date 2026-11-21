use super::*;
use crate::agents::{attachments::ParticipantRef, service};
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn setup() -> (tempfile::TempDir, AgentxDb, RequestContext, ParticipantRef) {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    db.save_instance_named("kera", json!({"tool":"codex", "session_id":"session-a", "status":"active", "created_at":12.0}).as_object().unwrap()).unwrap();
    let context = RequestContext {
        caller: CallerContext { request_id: RequestId("setup".into()), invocation: InvocationContext { explicit_name: Some("kera".into()), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
        daemon_instance: "fixture".into(),
        database_generation: db.database_generation().unwrap(),
    };
    let actor = service::resolve_actor(&db, &context).unwrap().participant;
    (dir, db, context, actor)
}

fn report(db: &AgentxDb, context: &RequestContext, actor: &ParticipantRef, id: &str, turn: &str, phase: RequestPhase) -> ServiceResult<Response> {
    let mut context = context.clone();
    context.caller.request_id = RequestId(id.into());
    service::handle(
        db,
        &context,
        service::Request::ObserveModelRequest(ObserveModelRequest {
            participant: actor.clone(),
            session_id: "session-a".into(),
            turn_id: turn.into(),
            model: "fixture".into(),
            phase,
            error: (phase == RequestPhase::Failed).then(|| "upstream 503".into()),
        }),
    )
}

#[test]
fn failure_is_governable_sticky_and_cleared_only_by_success() {
    let (_dir, db, context, actor) = setup();
    report(&db, &context, &actor, "start", "turn-a", RequestPhase::Started).unwrap();
    report(&db, &context, &actor, "fail", "turn-a", RequestPhase::Failed).unwrap();
    let first = db.get_instance_full("kera").unwrap().unwrap();
    assert_eq!(first.status, ST_ERROR);
    assert_eq!(first.model_request.as_ref().unwrap().error.as_ref().unwrap().consecutive_failures, 1);
    for (status, reason) in [("active", "prompt"), ("active", "tool:Bash"), ("listening", ""), ("listening", "interrupted")] {
        crate::agents::lifecycle::try_set_status(&db, "kera", status, reason, Default::default()).unwrap();
        assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, ST_ERROR);
    }
    db.update_heartbeat("kera").unwrap();
    report(&db, &context, &actor, "retry", "turn-a", RequestPhase::Started).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, ST_ERROR);
    report(&db, &context, &actor, "fail-again", "turn-a", RequestPhase::Failed).unwrap();
    let current = db.get_instance_full("kera").unwrap().unwrap();
    let error = current.model_request.unwrap().error.unwrap();
    assert_eq!(error.consecutive_failures, 2);
    assert_eq!(error.error_since, first.model_request.unwrap().error.unwrap().error_since);
    report(&db, &context, &actor, "success", "turn-a", RequestPhase::Completed).unwrap();
    let recovered = db.get_instance_full("kera").unwrap().unwrap();
    assert_eq!(recovered.status, ST_ACTIVE);
    assert!(recovered.model_request.unwrap().error.is_none());
    crate::agents::lifecycle::try_set_status(&db, "kera", "listening", "", Default::default()).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, "listening");
    let row = db.get_instance("kera").unwrap().unwrap();
    assert_eq!(row["model_request"]["phase"], "completed");
}

#[test]
fn reports_are_fenced_and_replay_does_not_reapply_old_failure() {
    let (_dir, db, context, actor) = setup();
    report(&db, &context, &actor, "failed-a", "turn-a", RequestPhase::Failed).unwrap();
    report(&db, &context, &actor, "start-b", "turn-b", RequestPhase::Started).unwrap();
    report(&db, &context, &actor, "done-b", "turn-b", RequestPhase::Completed).unwrap();
    for phase in [RequestPhase::Failed, RequestPhase::Completed, RequestPhase::Started] {
        report(&db, &context, &actor, &format!("late-{phase:?}"), "turn-a", phase).unwrap();
    }
    report(&db, &context, &actor, "failed-a", "turn-a", RequestPhase::Failed).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, ST_ACTIVE);
    db.update_instance_fields("kera", json!({"session_id":"session-b"}).as_object().unwrap()).unwrap();
    assert!(matches!(report(&db, &context, &actor, "old-session", "turn-b", RequestPhase::Failed), Err(ServiceError::StaleFence)));
    db.update_instance_fields("kera", json!({"session_id":"session-a", "created_at":99.0}).as_object().unwrap()).unwrap();
    assert!(matches!(report(&db, &context, &actor, "old-actor", "turn-b", RequestPhase::Failed), Err(ServiceError::StaleFence)));
}

#[test]
fn persistence_events_approval_exit_and_cancellation_preserve_error_facts() {
    let (dir, db, context, actor) = setup();
    report(&db, &context, &actor, "failure", "turn-a", RequestPhase::Failed).unwrap();
    report(&db, &context, &actor, "cancel", "turn-a", RequestPhase::Cancelled).unwrap();
    crate::agents::lifecycle::try_set_status(&db, "kera", ST_BLOCKED, "approval", Default::default()).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, ST_BLOCKED);
    crate::agents::lifecycle::try_set_status(&db, "kera", ST_ACTIVE, "approved:Bash", Default::default()).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, ST_ERROR);
    let data: String = db.conn().query_row("SELECT data FROM events WHERE type='status' ORDER BY id DESC LIMIT 1", [], |row| row.get(0)).unwrap();
    let data: serde_json::Value = serde_json::from_str(&data).unwrap();
    assert_eq!(data["status"], "error");
    assert_eq!(data["model_request"]["error"]["reason"], "upstream 503");
    drop(db);
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, ST_ERROR);
    crate::agents::lifecycle::try_set_status(&db, "kera", ST_INACTIVE, "exit:closed", Default::default()).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, ST_INACTIVE);
    report(&db, &context, &actor, "late-success", "turn-a", RequestPhase::Completed).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, ST_INACTIVE);
}

#[test]
fn failed_status_event_rolls_back_projection_and_can_retry() {
    let (_dir, db, context, actor) = setup();
    db.conn().execute_batch("CREATE TRIGGER reject_status BEFORE INSERT ON events WHEN NEW.type='status' BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(report(&db, &context, &actor, "same", "turn-a", RequestPhase::Failed).is_err());
    assert!(db.get_instance_full("kera").unwrap().unwrap().model_request.is_none());
    db.conn().execute_batch("DROP TRIGGER reject_status").unwrap();
    report(&db, &context, &actor, "same", "turn-a", RequestPhase::Failed).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, ST_ERROR);
}

#[test]
fn version_24_migrates_in_place_without_losing_participants() {
    let (dir, db, _, _) = setup();
    db.conn().execute_batch("ALTER TABLE instances DROP COLUMN model_request; PRAGMA user_version=24;").unwrap();
    drop(db);
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let row = db.get_instance_full("kera").unwrap().unwrap();
    assert!(row.model_request.is_none());
    assert_eq!(row.session_id.as_deref(), Some("session-a"));
    assert_eq!(db.conn().query_row::<i64, _, _>("PRAGMA user_version", [], |row| row.get(0)).unwrap(), i64::from(crate::db::SCHEMA_VERSION));
    assert!(!std::fs::read_dir(dir.path()).unwrap().filter_map(Result::ok).any(|entry| entry.file_name().to_string_lossy().contains("archive")), "in-place migration must not archive the database");
}

#[test]
fn late_cancellation_or_completion_does_not_reactivate_a_stopped_turn() {
    let (_dir, db, context, actor) = setup();
    report(&db, &context, &actor, "start", "turn-a", RequestPhase::Started).unwrap();
    crate::agents::lifecycle::try_set_status(&db, "kera", "listening", "interrupted", Default::default()).unwrap();
    report(&db, &context, &actor, "cancel", "turn-a", RequestPhase::Cancelled).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, "listening");
    report(&db, &context, &actor, "late-complete", "turn-a", RequestPhase::Completed).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status, "listening");
}

#[test]
fn successful_request_cannot_clear_an_unrelated_error() {
    let (_dir, db, context, actor) = setup();
    crate::agents::lifecycle::try_set_status(&db, "kera", ST_ERROR, "runtime:transport", Default::default()).unwrap();
    report(&db, &context, &actor, "started", "turn-a", RequestPhase::Started).unwrap();
    report(&db, &context, &actor, "failed", "turn-a", RequestPhase::Failed).unwrap();
    assert_eq!(db.get_instance_full("kera").unwrap().unwrap().status_context, "runtime:transport");
    report(&db, &context, &actor, "completed", "turn-a", RequestPhase::Completed).unwrap();
    let row = db.get_instance_full("kera").unwrap().unwrap();
    assert_eq!(row.status, ST_ERROR);
    assert_eq!(row.status_context, "runtime:transport");
}
