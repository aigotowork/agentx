//! Independent callbacks and retained attachment failure/replay fences.
use super::*;
use crate::service::{CallerContext, InvocationContext, RequestId};

fn with_db(body: impl FnOnce(&AgentxDb, RequestContext)) {
    let directory = tempfile::Builder::new().prefix("agentx-participant-fences-").tempdir().unwrap();
    let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
    for (index, name) in ["lobo", "nava"].into_iter().enumerate() {
        db.save_instance_named(name, json!({"created_at":10.0+index as f64,"tool":"codex","status":"active","last_event_id":0}).as_object().unwrap()).unwrap();
    }
    let pid = std::process::id();
    let context = RequestContext {
        caller: CallerContext { request_id: RequestId("participant-fence-stable".into()), invocation: InvocationContext { explicit_name: Some("lobo".into()), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid, incarnation: crate::sys::process::identity(pid) },
        daemon_instance: "participant-fixture".into(),
        database_generation: db.database_generation().unwrap(),
    };
    body(&db, context);
}

#[test]
fn status_and_heartbeat_callbacks_cannot_mutate_a_sibling() {
    with_db(|db, context| {
        let sibling = row(db, "nava").unwrap().participant;
        let before = db.get_instance("nava").unwrap().unwrap();
        let watermark = db.get_last_event_id();
        assert!(matches!(handle(db, &context, Request::Status { participant: sibling.clone(), status: "blocked".into(), context: "fixture".into(), detail: "stolen callback".into() }), Err(ServiceError::StaleFence)));
        assert!(matches!(handle(db, &context, Request::Heartbeat { participant: sibling }), Err(ServiceError::StaleFence)));
        assert_eq!(db.get_instance("nava").unwrap().unwrap(), before);
        assert_eq!(db.get_last_event_id(), watermark);
        handle(db, &context, Request::Heartbeat { participant: row(db, "lobo").unwrap().participant }).unwrap();
    });
}

fn attach_request(db: &AgentxDb) -> Request {
    Request::Attachments(AttachmentOperation::Attach(AttachmentRequest {
        execution: ExecutionId("attachment-retry-fixture".into()),
        owner: AttachmentOwnerId("fixture-owner".into()),
        session: ProviderSessionKey { provider: "codex".into(), scope: "/fixture/attachment".into(), session_id: "root".into() },
        intent: AttachmentIntent::Create { provisional: row(db, "lobo").unwrap().participant },
    }))
}

fn attachment(response: Response) -> Attachment {
    let Response::Attachment(AttachmentOutcome::Attached(Ok(value))) = response else {
        panic!("successful attachment response");
    };
    value
}

#[test]
fn transient_attach_storage_failure_does_not_poison_retained_request() {
    with_db(|db, context| {
        let request = attach_request(db);
        db.conn().execute_batch("CREATE TRIGGER fixture_attach_failure BEFORE INSERT ON participant_attachments BEGIN SELECT RAISE(ABORT, 'fixture attach write failure'); END;").unwrap();
        assert!(matches!(handle(db, &context, request.clone()), Err(ServiceError::Unavailable(_))));
        db.conn().execute_batch("DROP TRIGGER fixture_attach_failure").unwrap();
        let first = attachment(handle(db, &context, request.clone()).unwrap());
        let replay = attachment(handle(db, &context, request).unwrap());
        assert_eq!(replay.target, first.target);
        assert_eq!(db.conn().query_row::<i64, _, _>("SELECT count(*) FROM participant_attachments", [], |row| row.get(0)).unwrap(), 1);
    });
}

#[test]
fn retained_attach_reply_cannot_resurrect_retired_target() {
    with_db(|db, context| {
        let request = attach_request(db);
        let first = attachment(handle(db, &context, request.clone()).unwrap());
        handle(db, &context, Request::Attachments(AttachmentOperation::Retire { target: first.target.clone() })).unwrap();
        assert!(matches!(handle(db, &context, request), Err(ServiceError::StaleFence)));
        assert!(!SqliteAttachmentStore::new(db).validate(&first.target).unwrap());
    });
}

#[test]
fn launched_session_binding_preserves_status_cursor_and_replays_exact_result() {
    for canonical in [false, true] {
        with_db(|db, mut context| {
            db.update_instance_fields("lobo", json!({"status":"blocked","status_context":"hook:permission","last_event_id":7}).as_object().unwrap()).unwrap();
            db.update_instance_fields("nava", json!({"status":"listening","status_context":"hook:idle","last_event_id":9}).as_object().unwrap()).unwrap();
            db.set_process_binding("launch-binding", "", "lobo").unwrap();
            if canonical {
                db.set_session_binding("native-session", "nava").unwrap();
            }
            context.caller.invocation.environment.insert("AGENTX_LAUNCHED".into(), "1".into());
            context.caller.invocation.process_binding_id = Some("launch-binding".into());
            let request = Request::BindLaunchedSession { participant: row(db, "lobo").unwrap().participant, session_id: "native-session".into() };
            let Response::Registered(first) = handle(db, &context, request.clone()).unwrap() else {
                panic!("bound actor response");
            };
            assert_eq!(first.participant.canonical_name, if canonical { "nava" } else { "lobo" });
            let expected = if canonical { ("listening", "hook:idle", 9) } else { ("blocked", "hook:permission", 7) };
            let record = db.get_instance_full(&first.participant.canonical_name).unwrap().unwrap();
            assert_eq!((record.status.as_str(), record.status_context.as_str(), record.last_event_id), expected);
            let count = db.iter_instances_full().unwrap().len();
            let Response::Registered(replay) = handle(db, &context, request.clone()).unwrap() else {
                panic!("bound actor replay");
            };
            assert_eq!(first.participant, replay.participant);
            assert_eq!(db.iter_instances_full().unwrap().len(), count);
            db.update_instance_fields(&first.participant.canonical_name, json!({"created_at":99.0}).as_object().unwrap()).unwrap();
            assert!(matches!(handle(db, &context, request), Err(ServiceError::StaleFence)));
        });
    }
}

#[test]
fn launched_session_rejects_missing_binding_launch_flag_and_tool_without_allocation() {
    for missing in ["binding", "launch_flag", "tool"] {
        with_db(|db, mut context| {
            db.set_process_binding("launch-binding", "", "lobo").unwrap();
            context.caller.invocation.environment.insert("AGENTX_LAUNCHED".into(), "1".into());
            context.caller.invocation.process_binding_id = Some("launch-binding".into());
            match missing {
                "binding" => {
                    context.caller.invocation.process_binding_id = None;
                }
                "launch_flag" => {
                    context.caller.invocation.environment.remove("AGENTX_LAUNCHED");
                }
                "tool" => {
                    db.update_instance_fields("lobo", json!({"tool":"claude"}).as_object().unwrap()).unwrap();
                }
                _ => unreachable!(),
            }
            let request = Request::BindLaunchedSession { participant: row(db, "lobo").unwrap().participant, session_id: "native-session".into() };
            let watermark = db.get_last_event_id();
            assert!(handle(db, &context, request).is_err(), "{missing}");
            assert_eq!(db.iter_instances_full().unwrap().len(), 2);
            assert!(db.get_session_binding("native-session").unwrap().is_none());
            assert_eq!(db.get_last_event_id(), watermark);
        });
    }
}

#[test]
fn late_session_end_after_rebind_leaves_successor_and_child_intact() {
    with_db(|db, context| {
        db.update_instance_fields("lobo", json!({"session_id":"new-session"}).as_object().unwrap()).unwrap();
        db.update_instance_fields("nava", json!({"parent_name":"lobo","parent_session_id":"new-session","session_id":"child-session"}).as_object().unwrap()).unwrap();
        db.set_session_binding("new-session", "lobo").unwrap();
        db.set_session_binding("child-session", "nava").unwrap();
        let before = db.get_last_event_id();
        let result = handle(db, &context, Request::StopSession(StopSession { participant: row(db, "lobo").unwrap().participant, session_id: Some("retired-session".into()), reason: "session_end".into(), preserve_live_pty: false, soft: false }));
        if let Ok(Response::SessionStopped(outcome)) = result {
            assert!(!outcome.stopped);
            assert!(outcome.effects.is_empty());
        } else {
            assert!(result.is_err(), "unexpected session stop response");
        }
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().session_id.as_deref(), Some("new-session"));
        assert!(db.get_instance_full("nava").unwrap().is_some());
        assert_eq!(db.get_session_binding("new-session").unwrap().as_deref(), Some("lobo"));
        assert_eq!(db.get_last_event_id(), before);
    });
}

#[test]
fn required_stop_delete_failure_rolls_back_parent_children_and_events() {
    with_db(|db, context| {
        db.update_instance_fields("lobo", json!({"session_id":"parent-session"}).as_object().unwrap()).unwrap();
        db.update_instance_fields("nava", json!({"parent_name":"lobo","parent_session_id":"parent-session","session_id":"child-session"}).as_object().unwrap()).unwrap();
        db.set_session_binding("parent-session", "lobo").unwrap();
        db.set_session_binding("child-session", "nava").unwrap();
        db.conn().execute_batch("CREATE TRIGGER fixture_stop_failure BEFORE DELETE ON instances WHEN OLD.name='lobo' BEGIN SELECT RAISE(ABORT, 'fixture parent delete failure'); END;").unwrap();
        let before = db.get_last_event_id();
        assert!(handle(db, &context, Request::StopSession(StopSession { participant: row(db, "lobo").unwrap().participant, session_id: Some("parent-session".into()), reason: "session_end".into(), preserve_live_pty: false, soft: false })).is_err());
        assert!(db.get_instance_full("lobo").unwrap().is_some());
        assert!(db.get_instance_full("nava").unwrap().is_some());
        assert_eq!(db.get_session_binding("parent-session").unwrap().as_deref(), Some("lobo"));
        assert_eq!(db.get_session_binding("child-session").unwrap().as_deref(), Some("nava"));
        assert_eq!(db.get_last_event_id(), before);
    });
}

#[test]
fn lost_stop_reply_replays_frozen_effects_without_duplicate_events_and_rejects_namesake() {
    with_db(|db, context| {
        db.update_instance_fields("lobo", json!({"session_id":"parent-session"}).as_object().unwrap()).unwrap();
        db.update_instance_fields("nava", json!({"parent_name":"lobo","parent_session_id":"parent-session","session_id":"child-session"}).as_object().unwrap()).unwrap();
        for name in ["lobo", "nava"] {
            db.update_instance_pid_with_identity(name, context.process.pid, context.process.incarnation.as_deref()).unwrap();
        }
        let request = Request::StopSession(StopSession { participant: row(db, "lobo").unwrap().participant, session_id: Some("parent-session".into()), reason: "session_end".into(), preserve_live_pty: false, soft: false });
        let Response::SessionStopped(first) = handle(db, &context, request.clone()).unwrap() else {
            panic!("stopped session outcome");
        };
        assert!(first.stopped);
        assert_eq!(first.effects.len(), 2, "overlapping parent/name links must retire each child once");
        for effect in &first.effects {
            assert_eq!(effect.record.pid, Some(i64::from(context.process.pid)));
            assert_eq!(effect.process_identity, context.process.incarnation);
            assert_eq!(effect.scope, db.path().parent().unwrap());
        }
        let watermark = db.get_last_event_id();
        let Response::SessionStopped(replay) = handle(db, &context, request.clone()).unwrap() else {
            panic!("retained stop outcome");
        };
        assert_eq!(serde_json::to_value(&replay).unwrap(), serde_json::to_value(&first).unwrap());
        assert_eq!(db.get_last_event_id(), watermark);
        assert_eq!(db.conn().query_row::<i64, _, _>("SELECT count(*) FROM events WHERE type='life' AND json_extract(data,'$.action')='stopped'", [], |row| row.get(0)).unwrap(), 2);
        db.save_instance_named("lobo", json!({"created_at":99.0,"tool":"codex","status":"active"}).as_object().unwrap()).unwrap();
        assert!(matches!(handle(db, &context, request), Err(ServiceError::StaleFence)));
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().created_at, 99.0);
    });
}

#[test]
fn soft_session_stop_retains_omp_process_and_subscriptions_but_clears_antigravity_process() {
    for tool in ["omp", "antigravity"] {
        with_db(|db, context| {
            db.update_instance_fields("lobo", json!({"tool":tool,"session_id":"session"}).as_object().unwrap()).unwrap();
            db.set_process_binding("soft-binding", "session", "lobo").unwrap();
            db.set_session_binding("session", "lobo").unwrap();
            crate::db::subscriptions::create_filter_subscription(db, &std::collections::HashMap::from([("action".to_owned(), vec!["created".to_owned()])]), &[], "lobo", false, None).unwrap();
            let subscriptions = db.kv_prefix("events_sub:").unwrap();
            let Response::SessionStopped(outcome) =
                handle(db, &context, Request::StopSession(StopSession { participant: row(db, "lobo").unwrap().participant, session_id: Some("session".into()), reason: "session_end".into(), preserve_live_pty: false, soft: true })).unwrap()
            else {
                panic!("soft session stop");
            };
            assert!(outcome.retained && outcome.stopped);
            assert_eq!(outcome.effects.len(), 1);
            assert!(matches!(outcome.effects[0].process_action, super::super::hook_service::StopProcessAction::KeepRunning));
            assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().status, "inactive");
            assert_eq!(db.get_process_binding("soft-binding").unwrap().is_some(), tool == "omp");
            assert_eq!(db.kv_prefix("events_sub:").unwrap(), subscriptions);
        });
    }
}
