//! Internal operator admission cases. Lifecycle owns retained wire intent and
//! the host owns effects; these fixtures never execute a returned process plan.
use super::*;
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn fixture(body: impl FnOnce(&AgentxDb, &RequestContext, &ParticipantRef)) {
    let directory = tempfile::Builder::new().prefix("agentx-operator-stop-").tempdir().unwrap();
    let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
    db.save_instance_named("root", json!({"created_at":1.0,"tool":"qoder","session_id":"session","status":"active","last_event_id":0}).as_object().unwrap()).unwrap();
    db.set_session_binding("session", "root").unwrap();
    db.set_process_binding("root-worker", "session", "root").unwrap();
    db.save_instance_named("child", json!({"created_at":2.0,"tool":"claude","parent_name":"root","parent_session_id":"session","status":"active","last_event_id":0}).as_object().unwrap()).unwrap();
    let row = db.get_instance_full("root").unwrap().unwrap();
    let context = RequestContext {
        caller: CallerContext { request_id: RequestId("operator-stop".into()), invocation: InvocationContext { cwd: directory.path().to_path_buf(), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid: std::process::id(), incarnation: Some("operator-peer".into()) },
        daemon_instance: "test".into(),
        database_generation: db.database_generation().unwrap(),
    };
    body(&db, &context, &participant(&row));
}
fn invoke(db: &AgentxDb, context: &RequestContext, fence: &ParticipantRef, decision: &OperatorStop) -> ServiceResult<SessionStopOutcome> {
    db.with_write_scope(|| stop_operator(db, context, fence, "cli", decision).map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

#[test]
fn operator_can_stop_other_participant_with_frozen_child_first_process_policy() {
    fixture(|db, context, fence| {
        db.update_instance_pid_with_identity("root", std::process::id(), Some("frozen-root-identity")).unwrap();
        db.update_instance_fields("root", json!({"background":true,"last_event_id":0}).as_object().unwrap()).unwrap();
        db.update_instance_pid_with_identity("child", std::process::id(), Some("frozen-child-identity")).unwrap();
        let outcome = invoke(db, context, fence, &OperatorStop::Participation { reason: "external".into() }).unwrap();
        assert_eq!(outcome.effects.iter().map(|effect| effect.record.name.as_str()).collect::<Vec<_>>(), vec!["child", "root"]);
        assert!(matches!(outcome.effects[0].process_action, StopProcessAction::TrackSurvivor));
        assert!(matches!(outcome.effects[1].process_action, StopProcessAction::TerminateGroup));
        assert_eq!(outcome.effects[1].process_identity.as_deref(), Some("frozen-root-identity"));
        assert!(db.get_instance_full("root").unwrap().is_none());
        assert!(!context.caller.invocation.cwd.join(".tmp/pids").exists());
    });
}

#[test]
fn operator_no_pid_kill_records_exact_provenance_and_rejects_tracked_root() {
    fixture(|db, context, fence| {
        db.update_instance_pid_with_identity("root", std::process::id(), Some("tracked")).unwrap();
        assert!(matches!(invoke(db, context, fence, &OperatorStop::KilledWithoutProcess), Err(ServiceError::InvalidInput(_))));
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().status, "active");
        db.clear_instance_pid("root").unwrap();
        let result = invoke(db, context, fence, &OperatorStop::KilledWithoutProcess).unwrap();
        let root = result.effects.last().unwrap();
        assert_eq!(root.record.status_context, "exit:killed");
        assert_eq!(root.record.status_detail, "cli");
        assert!(root.record.pid.is_none());
        let event: String = db.conn().query_row("SELECT data FROM events WHERE type='life' AND instance='root' ORDER BY id DESC LIMIT 1", [], |row| row.get(0)).unwrap();
        let event: serde_json::Value = serde_json::from_str(&event).unwrap();
        assert_eq!(event["by"], "cli");
        assert_eq!(event["reason"], "killed");
    });
}

#[test]
fn operator_admission_rejects_missing_transaction_peer_remote_and_successor() {
    fixture(|db, context, fence| {
        let decision = OperatorStop::Participation { reason: "external".into() };
        assert!(matches!(stop_operator(db, context, fence, "cli", &decision), Err(ServiceError::Internal(_))));
        let mut unauthenticated = context.clone();
        unauthenticated.process.incarnation = None;
        assert!(matches!(invoke(db, &unauthenticated, fence, &decision), Err(ServiceError::Forbidden(_))));
        db.update_instance_fields("root", json!({"origin_device_id":"remote"}).as_object().unwrap()).unwrap();
        assert!(matches!(invoke(db, context, fence, &decision), Err(ServiceError::Forbidden(_))));
        db.update_instance_fields("root", json!({"origin_device_id":null,"created_at":9.0}).as_object().unwrap()).unwrap();
        assert!(matches!(invoke(db, context, fence, &decision), Err(ServiceError::StaleFence)));
        assert!(db.get_instance_full("child").unwrap().is_some());
        assert_eq!(db.get_last_event_id(), 0);
    });
}

#[test]
fn caught_operator_stop_failure_rolls_back_marker_child_bindings_and_events() {
    fixture(|db, context, fence| {
        db.conn().execute_batch("CREATE TRIGGER fail_operator_stop BEFORE DELETE ON instances WHEN OLD.name='root' BEGIN SELECT RAISE(ABORT,'root delete failed'); END;").unwrap();
        db.with_write_scope(|| {
            assert!(stop_operator(db, context, fence, "cli", &OperatorStop::KilledWithoutProcess).is_err());
            assert!(db.get_instance_full("child")?.is_some());
            assert_eq!(db.get_instance_full("root")?.unwrap().status_context, "");
            assert_eq!(db.get_process_binding("root-worker")?.as_deref(), Some("root"));
            assert_eq!(db.get_last_event_id(), 0);
            Ok(())
        })
        .unwrap();
        db.conn().execute_batch("DROP TRIGGER fail_operator_stop").unwrap();
        let result = invoke(db, context, fence, &OperatorStop::KilledWithoutProcess).unwrap();
        assert_eq!(result.effects.len(), 2);
    });
}
