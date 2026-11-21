//! Causal operator completion after the target's own retirement. These cases
//! exercise native handlers only; no host signal or external cleanup is run.
use super::*;
use crate::agents::lifecycle_service::{KillTicket, Request as LifecycleRequest, Response as LifecycleResponse};
use crate::agents::service::{Request as ParticipantRequest, Response as ParticipantResponse};
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn lifecycle(db: &AgentxDb, context: &RequestContext, request: LifecycleRequest) -> ServiceResult<LifecycleResponse> {
    match crate::agents::service::handle(db, context, ParticipantRequest::Lifecycle(request))? {
        ParticipantResponse::Lifecycle(response) => Ok(response),
        _ => panic!("native lifecycle response required"),
    }
}

fn with_issued_kill(body: impl FnOnce(&AgentxDb, &RequestContext, &RequestContext, &KillTicket, i64)) {
    let directory = tempfile::Builder::new().prefix("agentx-kill-retirement-").tempdir().unwrap();
    let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
    for (name, created, pid, session, identity) in [("lobo", 10.0, 4242, "root-session", "root-process"), ("nava", 11.0, 4243, "child-session", "child-process")] {
        db.save_instance_named(
            name,
            json!({
                "created_at":created,"tool":"qoder","pid":pid,"session_id":session,
                "status":"listening","status_context":"ready","directory":"/target",
                "launch_context":json!({"pid_identity":identity}).to_string()
            })
            .as_object()
            .unwrap(),
        )
        .unwrap();
        db.set_session_binding(session, name).unwrap();
        db.set_process_binding(&format!("binding-{name}"), session, name).unwrap();
    }
    db.update_instance_fields("nava", json!({"parent_name":"lobo","parent_session_id":"root-session"}).as_object().unwrap()).unwrap();
    let operator = RequestContext {
        caller: CallerContext { request_id: RequestId("stable-operator-prepare".into()), invocation: InvocationContext { cwd: directory.path().to_path_buf(), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid: std::process::id(), incarnation: Some("trusted-fixture-peer".into()) },
        daemon_instance: "retirement-fixture".into(),
        database_generation: db.database_generation().unwrap(),
    };
    let mut target = operator.clone();
    target.caller.request_id = RequestId("stable-target-stop".into());
    target.caller.invocation.explicit_name = Some("lobo".into());
    target.caller.invocation.process_binding_id = Some("binding-lobo".into());
    let fence = participant(&db.get_instance_full("lobo").unwrap().unwrap());
    let LifecycleResponse::KillPrepared(plan) = lifecycle(&db, &operator, LifecycleRequest::PrepareKill { participant: fence, initiator: "operator".into() }).unwrap() else {
        panic!("kill plan required");
    };
    let marker_time = db.get_instance_full("lobo").unwrap().unwrap().status_time;
    assert!(matches!(lifecycle(&db, &operator, LifecycleRequest::BeginKill { ticket: plan.ticket.clone() }).unwrap(), LifecycleResponse::KillGranted));
    body(&db, &operator, &target, &plan.ticket, marker_time);
}

fn self_stop(db: &AgentxDb, target: &RequestContext, ticket: &KillTicket) -> SessionStopOutcome {
    let request = ParticipantRequest::StopSession(StopSession { participant: ticket.participant.clone(), session_id: Some("root-session".into()), reason: "session_end".into(), preserve_live_pty: false, soft: false });
    match crate::agents::service::handle(db, target, request).unwrap() {
        ParticipantResponse::SessionStopped(outcome) => outcome,
        _ => panic!("native session stop response required"),
    }
}

fn complete(db: &AgentxDb, operator: &RequestContext, ticket: &KillTicket) -> ServiceResult<SessionStopOutcome> {
    match lifecycle(db, operator, LifecycleRequest::CompleteKill { ticket: ticket.clone() })? {
        LifecycleResponse::KillCompleted(outcome) => Ok(outcome),
        _ => panic!("native kill completion response required"),
    }
}

#[test]
fn self_retirement_recovers_exact_plan_after_lost_completion_reply() {
    for plugin_stop in [false, true] {
        with_issued_kill(|db, operator, target, ticket, _| {
            let hook_outcome = if plugin_stop {
                assert!(matches!(crate::agents::service::handle(db, target, ParticipantRequest::Stop { participant: ticket.participant.clone(), reason: "session_end".into() }).unwrap(), ParticipantResponse::Stopped(true)));
                None
            } else {
                Some(self_stop(db, target, ticket))
            };
            assert!(db.get_instance_full("lobo").unwrap().is_none());
            assert!(db.get_instance_full("nava").unwrap().is_none());
            let before = db.get_last_event_id();
            let first = complete(db, operator, ticket).unwrap();
            assert!(first.stopped && !first.retained);
            assert_eq!(first.effects.iter().map(|effect| effect.record.name.as_str()).collect::<Vec<_>>(), vec!["nava", "lobo"]);
            assert_eq!(first.effects[1].process_identity.as_deref(), Some("root-process"));
            if let Some(hook_outcome) = hook_outcome {
                assert_eq!(serde_json::to_value(hook_outcome).unwrap(), serde_json::to_value(&first).unwrap());
            }
            // Treat the committed first reply as lost: the original ticket
            // recovers the exact plan without another signal or lifecycle event.
            let replay = complete(db, operator, ticket).unwrap();
            assert_eq!(serde_json::to_value(&first).unwrap(), serde_json::to_value(replay).unwrap());
            assert_eq!(db.get_last_event_id(), before);
            let by: String = db.conn().query_row("SELECT json_extract(data,'$.by') FROM events WHERE type='life' AND instance='lobo' AND json_extract(data,'$.action')='stopped' ORDER BY id DESC LIMIT 1", [], |row| row.get(0)).unwrap();
            assert_eq!(by, "operator");
        });
    }
}

#[test]
fn retirement_requires_original_marker_and_rejects_namesake_successor() {
    with_issued_kill(|db, operator, target, ticket, marker_time| {
        self_stop(db, target, ticket);
        let recover = |ticket: &KillTicket, initiator: &str, marker: i64| db.with_write_scope(|| recover_killed(db, operator, ticket, initiator, marker).map_err(anyhow::Error::new));
        for (initiator, marker) in [("another-operator", marker_time), ("operator", marker_time + 1)] {
            let error = recover(ticket, initiator, marker).unwrap_err();
            assert_eq!(error.downcast_ref::<ServiceError>(), Some(&ServiceError::StaleFence));
        }
        let mut changed = ticket.clone();
        changed.pid += 1;
        assert_eq!(recover(&changed, "operator", marker_time).unwrap_err().downcast_ref::<ServiceError>(), Some(&ServiceError::StaleFence));
        changed = ticket.clone();
        changed.process_identity = Some("reused-pid".into());
        assert_eq!(recover(&changed, "operator", marker_time).unwrap_err().downcast_ref::<ServiceError>(), Some(&ServiceError::StaleFence));
        changed = ticket.clone();
        changed.generation = "replacement-generation".into();
        assert_eq!(recover(&changed, "operator", marker_time).unwrap_err().downcast_ref::<ServiceError>(), Some(&ServiceError::StaleFence));
        db.save_instance_named("lobo", json!({"created_at":99.0,"tool":"qoder","pid":4242}).as_object().unwrap()).unwrap();
        assert_eq!(complete(db, operator, ticket).unwrap_err(), ServiceError::StaleFence);
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().created_at, 99.0);
    });
}

#[test]
fn retirement_record_failure_rolls_back_tree_and_same_stop_can_retry() {
    with_issued_kill(|db, operator, target, ticket, _| {
        let before = db.get_last_event_id();
        db.conn().execute_batch("CREATE TRIGGER fail_killed_retirement BEFORE INSERT ON kv WHEN NEW.key LIKE 'participant_killed_retirement:%' BEGIN SELECT RAISE(ABORT,'injected retirement failure'); END;").unwrap();
        let input = StopSession { participant: ticket.participant.clone(), session_id: Some("root-session".into()), reason: "session_end".into(), preserve_live_pty: false, soft: false };
        assert!(stop_session(db, target, input).is_err());
        assert_eq!(db.iter_instances_full().unwrap().len(), 2);
        assert_eq!(db.get_process_binding("binding-nava").unwrap().as_deref(), Some("nava"));
        assert_eq!(db.get_session_binding("root-session").unwrap().as_deref(), Some("lobo"));
        assert_eq!(db.get_last_event_id(), before);
        assert!(db.kv_get(&killed_retirement_key(&ticket.generation, &ticket.participant)).unwrap().is_none());
        db.conn().execute_batch("DROP TRIGGER fail_killed_retirement").unwrap();
        self_stop(db, target, ticket);
        assert!(complete(db, operator, ticket).unwrap().stopped);
    });
}

#[test]
fn ticket_save_failure_preserves_committed_retirement_for_recovery() {
    with_issued_kill(|db, operator, target, ticket, _| {
        let retired = self_stop(db, target, ticket);
        let before = db.get_last_event_id();
        db.conn().execute_batch("CREATE TRIGGER fail_kill_completion BEFORE INSERT ON kv WHEN NEW.key LIKE 'participant_kill:%' BEGIN SELECT RAISE(ABORT,'injected completion failure'); END;").unwrap();
        assert!(complete(db, operator, ticket).is_err());
        assert!(db.iter_instances_full().unwrap().is_empty());
        assert!(db.kv_get(&killed_retirement_key(&ticket.generation, &ticket.participant)).unwrap().is_some());
        assert_eq!(db.get_last_event_id(), before);
        db.conn().execute_batch("DROP TRIGGER fail_kill_completion").unwrap();
        let recovered = complete(db, operator, ticket).unwrap();
        assert_eq!(serde_json::to_value(retired).unwrap(), serde_json::to_value(recovered).unwrap());
        assert_eq!(db.get_last_event_id(), before);
    });
}

#[test]
fn missing_row_without_canonical_retirement_is_unknown() {
    with_issued_kill(|db, operator, _, ticket, _| {
        db.delete_instance("lobo").unwrap();
        assert!(matches!(complete(db, operator, ticket), Err(ServiceError::OutcomeUnknown(_))));
        assert!(db.get_instance_full("nava").unwrap().is_some());
        assert!(db.kv_get(&killed_retirement_key(&ticket.generation, &ticket.participant)).unwrap().is_none());
    });
}
