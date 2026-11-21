//! Lifecycle retention is separate from canonical retirement policy. These
//! cases never execute returned host effects or signal fixture processes.
use super::*;
use crate::agents::hook_service::OperatorStop;

fn fixture(body: impl FnOnce(&AgentxDb, RequestContext, ParticipantRef)) {
    let root = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    db.save_instance_named("root", json!({"tool":"qoder","created_at":1.0,"session_id":"root-session","status":"active"}).as_object().unwrap()).unwrap();
    db.set_session_binding("root-session", "root").unwrap();
    db.set_process_binding("root-binding", "root-session", "root").unwrap();
    db.save_instance_named("child", json!({"tool":"claude","created_at":2.0,"parent_name":"root","parent_session_id":"root-session","status":"active"}).as_object().unwrap()).unwrap();
    let context = RequestContext {
        caller: crate::service::CallerContext {
            request_id: crate::service::RequestId("operator-case".into()),
            invocation: crate::service::InvocationContext { cwd: root.path().into(), ..Default::default() },
            deadline_unix_ms: None,
            expected_database_generation: None,
        },
        process: crate::service::CallerProcess { pid: std::process::id(), incarnation: Some("fixture-operator-peer".into()) },
        daemon_instance: "fixture".into(),
        database_generation: db.database_generation().unwrap(),
    };
    body(&db, context, ParticipantRef { canonical_name: "root".into(), incarnation_bits: 1.0f64.to_bits() });
}
fn invoke(db: &AgentxDb, context: &RequestContext, fence: &ParticipantRef, reason: &str) -> ServiceResult<crate::agents::hook_service::SessionStopOutcome> {
    let request = Request::StopOperator { participant: fence.clone(), initiator: "cli".into(), decision: OperatorStop::Participation { reason: reason.into() } };
    match crate::agents::service::handle(db, context, crate::agents::service::Request::Lifecycle(request))? {
        crate::agents::service::Response::Lifecycle(Response::OperatorStopped(outcome)) => Ok(outcome),
        _ => panic!("operator response"),
    }
}
#[test]
fn native_operator_stop_recovers_deleted_root_and_child_before_live_row_check() {
    fixture(|db, context, fence| {
        let first = invoke(db, &context, &fence, "external").unwrap();
        assert_eq!(first.effects.iter().map(|effect| effect.record.name.as_str()).collect::<Vec<_>>(), vec!["child", "root"]);
        let cursor = db.get_last_event_id();
        let replay = invoke(db, &context, &fence, "external").unwrap();
        assert_eq!(serde_json::to_value(&first).unwrap(), serde_json::to_value(replay).unwrap());
        assert_eq!(cursor, db.get_last_event_id());
        assert!(matches!(invoke(db, &context, &fence, "other"), Err(ServiceError::Conflict(_))));
        let mut other = context.clone();
        other.process.incarnation = Some("other-peer".into());
        assert!(matches!(invoke(db, &other, &fence, "external"), Err(ServiceError::Conflict(_))));
        db.save_instance_named("child", json!({"tool":"claude","created_at":99.0}).as_object().unwrap()).unwrap();
        assert!(matches!(invoke(db, &context, &fence, "external"), Err(ServiceError::StaleFence)));
    });
}
#[test]
fn native_operator_result_write_failure_rolls_back_tree_bindings_and_event() {
    fixture(|db, context, fence| {
        db.conn().execute_batch("CREATE TRIGGER fail_stop_result BEFORE INSERT ON kv WHEN NEW.key LIKE 'participant_operator_stop:%' BEGIN SELECT RAISE(ABORT,'stop result failure'); END;").unwrap();
        assert!(invoke(db, &context, &fence, "external").is_err());
        assert!(db.get_instance_full("root").unwrap().is_some());
        assert!(db.get_instance_full("child").unwrap().is_some());
        assert_eq!(db.get_process_binding("root-binding").unwrap().as_deref(), Some("root"));
        assert_eq!(db.get_last_event_id(), 0);
        db.conn().execute_batch("DROP TRIGGER fail_stop_result").unwrap();
        assert!(invoke(db, &context, &fence, "external").unwrap().stopped);
    });
}
#[test]
fn native_operator_no_pid_kill_cannot_retire_an_intervening_namesake() {
    fixture(|db, context, fence| {
        db.update_instance_fields("root", json!({"created_at":99.0}).as_object().unwrap()).unwrap();
        assert!(matches!(stop_operator(db, &context, &fence, "cli", &OperatorStop::KilledWithoutProcess), Err(ServiceError::StaleFence)));
        assert!(db.get_instance_full("child").unwrap().is_some());
        assert_eq!(db.get_last_event_id(), 0);
    });
}
#[test]
fn native_adoption_replays_exact_pid_and_rejects_changed_peer_or_owner() {
    fixture(|db, context, _| {
        let pid = std::process::id();
        let candidate = pidtrack::OrphanCandidate {
            orphan: pidtrack::OrphanProcess {
                pid,
                tool: "qoder".into(),
                names: vec!["root".into()],
                directory: "/project".into(),
                process_id: "root-binding".into(),
                session_id: "root-session".into(),
                terminal_preset: String::new(),
                pane_id: String::new(),
                terminal_id: String::new(),
                kitty_listen_on: String::new(),
                zellij_session_name: String::new(),
                herdr_socket_path: String::new(),
                wezterm_unix_socket: String::new(),
                tmux: String::new(),
                notify_port: 0,
                inject_port: 0,
            },
            process_identity: crate::sys::process::identity(pid).unwrap(),
        };
        let outcome = claim_orphans(db, &context, std::slice::from_ref(&candidate)).unwrap();
        assert_eq!(outcome.len(), 1);
        assert_eq!(outcome[0].record.pid, Some(i64::from(pid)));
        assert_eq!(claim_orphans(db, &context, std::slice::from_ref(&candidate)).unwrap()[0].process_identity, candidate.process_identity);
        let mut changed = context.clone();
        changed.process.incarnation = Some("other".into());
        assert!(matches!(claim_orphans(db, &changed, std::slice::from_ref(&candidate)), Err(ServiceError::Conflict(_))));
        db.update_instance_fields("root", json!({"created_at":99.0}).as_object().unwrap()).unwrap();
        assert!(matches!(claim_orphans(db, &context, &[candidate]), Err(ServiceError::StaleFence)));
    });
}
#[test]
fn native_adoption_result_failure_preserves_unadopted_row_and_same_id_retry() {
    fixture(|db, context, _| {
        let pid = std::process::id();
        let orphan = pidtrack::OrphanProcess {
            pid,
            tool: "qoder".into(),
            names: vec!["root".into()],
            directory: "/project".into(),
            process_id: "root-binding".into(),
            session_id: "root-session".into(),
            terminal_preset: String::new(),
            pane_id: String::new(),
            terminal_id: String::new(),
            kitty_listen_on: String::new(),
            zellij_session_name: String::new(),
            herdr_socket_path: String::new(),
            wezterm_unix_socket: String::new(),
            tmux: String::new(),
            notify_port: 12345,
            inject_port: 0,
        };
        let candidate = pidtrack::OrphanCandidate { orphan, process_identity: crate::sys::process::identity(pid).unwrap() };
        db.conn().execute_batch("CREATE TRIGGER fail_adoption BEFORE INSERT ON kv WHEN NEW.key LIKE 'participant_orphan_adoption:%' BEGIN SELECT RAISE(ABORT,'adoption result failure'); END;").unwrap();
        assert!(claim_orphans(db, &context, std::slice::from_ref(&candidate)).is_err());
        assert!(db.get_instance_full("root").unwrap().unwrap().pid.is_none());
        assert!(db.get_notify_endpoint_port("root", "pty").is_none());
        db.conn().execute_batch("DROP TRIGGER fail_adoption").unwrap();
        assert_eq!(claim_orphans(db, &context, &[candidate]).unwrap().len(), 1);
        assert_eq!(db.get_notify_endpoint_port("root", "pty"), Some(12345));
    });
}
