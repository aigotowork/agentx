//! Native cleanup/switch scenarios. Nothing in these cases executes a returned
//! process plan; fake stored incarnations must never authorize a host signal.
use super::*;
use crate::agents::hook_service::StopProcessAction;
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn fixture(body: impl FnOnce(&AgentxDb, &RequestContext)) {
    let directory = tempfile::Builder::new().prefix("agentx-system-cleanup-").tempdir().unwrap();
    let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
    let context = RequestContext {
        caller: CallerContext { request_id: RequestId("stable-cleanup".into()), invocation: InvocationContext { cwd: directory.path().to_path_buf(), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid: std::process::id(), incarnation: Some("authenticated-fixture-peer".into()) },
        daemon_instance: "cleanup-fixture".into(),
        database_generation: db.database_generation().unwrap(),
    };
    body(&db, &context);
}

fn row(db: &AgentxDb, name: &str, session: &str, created: f64) -> InstanceRow {
    db.save_instance_named(
        name,
        json!({"tool":"qoder","created_at":created,"session_id":session,"status":"listening","status_context":"ready","status_time":crate::shared::time::now_epoch_i64(),"last_stop":crate::shared::time::now_epoch_i64(),"directory":"/origin"})
            .as_object()
            .unwrap(),
    )
    .unwrap();
    db.set_session_binding(session, name).unwrap();
    db.set_process_binding(&format!("binding-{name}"), session, name).unwrap();
    db.get_instance_full(name).unwrap().unwrap()
}

fn gone(db: &AgentxDb, name: &str) {
    // The process is observable and alive, but it is positively a different
    // incarnation from the one this durable participant used to own.
    assert!(crate::sys::process::identity(std::process::id()).is_some());
    db.update_instance_pid_with_identity(name, std::process::id(), Some("previous-process-incarnation")).unwrap();
}

fn seed_tree(db: &AgentxDb) -> ParticipantRef {
    let root = row(db, "root", "root-session", 1.0);
    row(db, "named-child", "named-session", 2.0);
    row(db, "session-child", "child-session", 3.0);
    db.update_instance_fields("named-child", json!({"parent_name":"root"}).as_object().unwrap()).unwrap();
    db.update_instance_fields("session-child", json!({"parent_session_id":"root-session"}).as_object().unwrap()).unwrap();
    gone(db, "root");
    participant(&root)
}

#[test]
fn system_reap_returns_child_first_exact_plan_and_lost_reply_recovers_it() {
    fixture(|db, context| {
        seed_tree(db);
        let first = cleanup_system(db, context, SystemCleanup::DeadProcesses).unwrap();
        assert_eq!(first.retired, 1);
        assert_eq!(first.effects.iter().map(|effect| effect.record.name.as_str()).collect::<Vec<_>>(), vec!["named-child", "session-child", "root"]);
        assert!(first.effects.iter().all(|effect| matches!(effect.process_action, StopProcessAction::KeepRunning)));
        assert_eq!(first.effects.last().unwrap().process_identity.as_deref(), Some("previous-process-incarnation"));
        assert!(first.effects.iter().all(|effect| effect.scope == context.caller.invocation.cwd));
        assert!(db.iter_instances_full().unwrap().is_empty());
        assert!(!context.caller.invocation.cwd.join(".tmp/pids").exists(), "daemon retirement cannot create host pidtrack files");
        let events = db.get_last_event_id();
        row(db, "later", "later-session", 4.0);
        gone(db, "later");
        let replay = cleanup_system(db, context, SystemCleanup::DeadProcesses).unwrap();
        assert_eq!(serde_json::to_value(first).unwrap(), serde_json::to_value(replay).unwrap());
        assert!(db.get_instance_full("later").unwrap().is_some(), "same request cannot sweep a newly arrived participant");
        assert_eq!(db.get_last_event_id(), events);
        row(db, "root", "successor-session", 99.0);
        assert_eq!(cleanup_system(db, context, SystemCleanup::DeadProcesses).unwrap_err(), ServiceError::StaleFence);
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().created_at, 99.0);
    });
}

#[test]
fn exact_dead_parent_defers_whole_tree_while_child_owns_a_process() {
    fixture(|db, context| {
        let fence = seed_tree(db);
        db.update_instance_pid("named-child", std::process::id()).unwrap();
        let before = db.get_last_event_id();
        let outcome = cleanup_system(db, context, SystemCleanup::Resume { participant: fence.clone() }).unwrap();
        assert_eq!(outcome.retired, 0);
        assert_eq!(outcome.deferred, vec![fence]);
        assert!(outcome.effects.is_empty());
        assert_eq!(db.iter_instances_full().unwrap().len(), 3);
        assert_eq!(db.get_process_binding("binding-root").unwrap().as_deref(), Some("root"));
        assert_eq!(db.get_last_event_id(), before);
    });
}

#[test]
fn unavailable_identity_remote_and_rebound_rows_grant_no_reap_authority() {
    fixture(|db, context| {
        let mut untrusted = context.clone();
        untrusted.process.pid = 0;
        assert!(matches!(cleanup_system(db, &untrusted, SystemCleanup::DeadProcesses), Err(ServiceError::Forbidden(_))));
        let original = row(db, "source", "source-session", 1.0);
        gone(db, "source");
        let inspected = db.get_instance_full("source").unwrap().unwrap();
        db.update_instance_pid("source", std::process::id()).unwrap();
        let error = db.with_write_scope(|| crate::agents::hook_store::cleanup_stop(db, context, &inspected, "process_exit", false, true).map_err(anyhow::Error::new)).unwrap_err();
        assert_eq!(error.downcast_ref::<ServiceError>(), Some(&ServiceError::StaleFence));
        row(db, "unknown", "unknown-session", 2.0);
        db.update_instance_pid_with_identity("unknown", 0, Some("unavailable")).unwrap();
        assert_eq!(tracked_process_state(&db.get_instance_full("unknown").unwrap().unwrap()), TrackedProcessState::Unverifiable);
        row(db, "remote", "remote-session", 3.0);
        gone(db, "remote");
        db.update_instance_fields("remote", json!({"origin_device_id":"other-device"}).as_object().unwrap()).unwrap();
        let outcome = cleanup_system(db, context, SystemCleanup::DeadProcesses).unwrap();
        assert_eq!(outcome.retired, 0);
        assert!(outcome.effects.is_empty());
        assert_eq!(db.iter_instances_full().unwrap().len(), 3);
        let mut fresh = context.clone();
        fresh.caller.request_id = RequestId("stale-exact-target".into());
        db.update_instance_fields("source", json!({"created_at":99.0}).as_object().unwrap()).unwrap();
        assert_eq!(cleanup_system(db, &fresh, SystemCleanup::Resume { participant: participant(&original) }).unwrap_err(), ServiceError::StaleFence);
    });
}

#[test]
fn retained_plan_failure_rolls_back_all_rows_events_and_bindings_then_retries() {
    fixture(|db, context| {
        seed_tree(db);
        let before = db.get_last_event_id();
        db.conn().execute_batch("CREATE TRIGGER fail_cleanup_plan BEFORE INSERT ON kv WHEN NEW.key LIKE 'participant_request:%' BEGIN SELECT RAISE(ABORT,'injected cleanup outcome failure'); END;").unwrap();
        assert!(cleanup_system(db, context, SystemCleanup::DeadProcesses).is_err());
        assert_eq!(db.iter_instances_full().unwrap().len(), 3);
        assert_eq!(db.get_session_binding("root-session").unwrap().as_deref(), Some("root"));
        assert_eq!(db.get_process_binding("binding-named-child").unwrap().as_deref(), Some("named-child"));
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().status, "listening");
        assert_eq!(db.get_last_event_id(), before);
        assert!(db.kv_get(SWEEP_KEY).unwrap().is_none());
        db.conn().execute_batch("DROP TRIGGER fail_cleanup_plan").unwrap();
        assert_eq!(cleanup_system(db, context, SystemCleanup::DeadProcesses).unwrap().effects.len(), 3);
    });
}

#[test]
fn placeholder_expiry_preserves_marker_snapshot_and_fresh_launch() {
    fixture(|db, context| {
        let now = crate::shared::time::now_epoch_f64();
        row(db, "launcher", "launch-parent", now - 150.0);
        for (name, age) in [("expired", 130.0), ("fresh", 110.0)] {
            db.save_instance_named(
                name,
                json!({"tool":"qoder","created_at":now-age,"status":"pending","status_context":"new","parent_session_id":"launch-parent","last_event_id":0,"pid":std::process::id(),"launch_context":json!({"pid_identity":"provisional"}).to_string()})
                    .as_object()
                    .unwrap(),
            )
            .unwrap();
        }
        row(db, "bound", "bound-session", now - 130.0);
        let outcome = cleanup_system(db, context, SystemCleanup::LaunchPlaceholders).unwrap();
        assert_eq!(outcome.retired, 1);
        assert_eq!(outcome.effects[0].record.name, "expired");
        assert!(db.get_instance_full("fresh").unwrap().is_some());
        assert!(db.get_instance_full("bound").unwrap().is_some());
        let event: String = db.conn().query_row("SELECT data FROM events WHERE type='life' AND instance='expired' ORDER BY id DESC LIMIT 1", [], |row| row.get(0)).unwrap();
        let event: serde_json::Value = serde_json::from_str(&event).unwrap();
        assert_eq!(event["placeholder"], true);
        assert_eq!(event["snapshot"]["parent_session_id"], "launch-parent");
        assert_eq!(event["snapshot"]["last_event_id"], 0);
    });
}

#[test]
fn shared_wake_grace_defers_clock_cleanup_but_not_positive_process_death() {
    let _guard = crate::agents::lifecycle::wake_test_guard();
    fixture(|db, context| {
        row(db, "stale", "stale-session", 1.0);
        db.update_instance_fields("stale", json!({"status":"inactive","status_context":"exit:closed","status_time":crate::shared::time::now_epoch_i64()-4000}).as_object().unwrap()).unwrap();
        row(db, "dead", "dead-session", 2.0);
        gone(db, "dead");
        db.kv_set("_wake_grace_until", Some(&(crate::shared::time::now_epoch_f64() + 60.0).to_string())).unwrap();
        let result = cleanup_system(db, context, SystemCleanup::Listing).unwrap();
        assert_eq!(result.retired, 1);
        assert_eq!(result.effects[0].record.name, "dead");
        assert!(db.get_instance_full("stale").unwrap().is_some());
    });
}

fn switched_fixture(db: &AgentxDb, context: &RequestContext) -> RequestContext {
    for _ in 0..4 {
        db.log_event("life", "_fixture", &json!({"action":"fixture"})).unwrap();
    }
    row(db, "lobo", "old-session", 1.0);
    row(db, "nava", "new-session", 2.0);
    row(db, "ancestor", "grandparent", 0.5);
    db.update_instance_pid("lobo", std::process::id()).unwrap();
    db.update_instance_fields("lobo", json!({"last_event_id":3,"parent_session_id":"grandparent"}).as_object().unwrap()).unwrap();
    db.update_instance_fields("nava", json!({"last_event_id":1}).as_object().unwrap()).unwrap();
    crate::db::subscriptions::create_filter_subscription(db, &std::collections::HashMap::from([("action".into(), vec!["created".into()])]), &[], "lobo", false, None).unwrap_or_else(|error| panic!("{error}"));
    let mut context = context.clone();
    context.caller.invocation.process_binding_id = Some("binding-lobo".into());
    context
}

#[test]
fn session_switch_keeps_cursor_and_saved_subscriptions_but_moves_process_authority() {
    fixture(|db, context| {
        let context = switched_fixture(db, context);
        let actor = crate::agents::invocation_binding::bind_session(db, &context, "new-session", "qoder", false).unwrap().unwrap();
        assert_eq!(actor.record.name, "nava");
        let old = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(old.pid, None);
        assert_eq!(old.last_event_id, 3);
        assert_eq!(old.status_context, "exit:session_switch");
        assert_eq!(old.pid_identity(), None);
        assert_eq!(actor.record.pid, Some(i64::from(std::process::id())));
        assert_eq!(db.get_process_binding("binding-lobo").unwrap().as_deref(), Some("nava"));
        assert!(db.get_session_binding("old-session").unwrap().is_none());
        assert!(!crate::db::subscriptions::subscriptions_to_save(db, "lobo", old.created_at).unwrap().is_empty(), "soft retirement retains the stopped snapshot's subscriptions");
        let snapshot: String = db.conn().query_row("SELECT json_extract(data,'$.snapshot') FROM events WHERE type='life' AND instance='lobo' AND json_extract(data,'$.action')='stopped' ORDER BY id DESC LIMIT 1", [], |row| row.get(0)).unwrap();
        let snapshot: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
        assert_eq!(snapshot["last_event_id"], 3);
        assert_eq!(snapshot["parent_session_id"], "grandparent");
        assert!(!snapshot["subscriptions"].as_array().unwrap().is_empty());
        assert!(!context.caller.invocation.cwd.join(".tmp/pids").exists());
    });
}

#[test]
fn failed_switch_stopped_event_restores_process_cursor_and_subscriptions() {
    fixture(|db, context| {
        let context = switched_fixture(db, context);
        let original = db.get_instance_full("lobo").unwrap().unwrap();
        let saved = crate::db::subscriptions::subscriptions_to_save(db, "lobo", original.created_at).unwrap();
        let before = db.get_last_event_id();
        db.conn()
            .execute_batch("CREATE TRIGGER fail_switch_event BEFORE INSERT ON events WHEN NEW.type='life' AND NEW.instance='lobo' AND json_extract(NEW.data,'$.action')='stopped' BEGIN SELECT RAISE(ABORT,'injected switch snapshot failure'); END;")
            .unwrap();
        assert!(crate::agents::invocation_binding::bind_session(db, &context, "new-session", "qoder", false).is_err());
        let old = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(old.pid, original.pid);
        assert_eq!(old.pid_identity(), original.pid_identity());
        assert_eq!(old.last_event_id, 3);
        assert_eq!(old.status, "listening");
        assert_eq!(db.get_instance_full("nava").unwrap().unwrap().pid, None);
        assert_eq!(db.get_process_binding("binding-lobo").unwrap().as_deref(), Some("lobo"));
        assert_eq!(db.get_session_binding("old-session").unwrap().as_deref(), Some("lobo"));
        assert_eq!(crate::db::subscriptions::subscriptions_to_save(db, "lobo", old.created_at).unwrap(), saved);
        assert_eq!(db.get_last_event_id(), before);
    });
}

#[test]
fn stored_launch_diagnosis_never_captures_host_output_and_required_event_rolls_back() {
    fixture(|db, context| {
        let log = context.caller.invocation.cwd.join("background.log");
        std::fs::write(&log, "private-host-output\n").unwrap();
        db.save_instance_named("launch", json!({"tool":"qoder","created_at":crate::shared::time::now_epoch_f64()-40.0,"status":"pending","status_context":"new","background_log_file":log.to_string_lossy()}).as_object().unwrap()).unwrap();
        let row = db.get_instance_full("launch").unwrap().unwrap();
        let status = crate::agents::lifecycle::get_instance_status_stored(&row, db);
        assert!(!status.description.contains("private-host-output"));
        assert_eq!(db.get_instance_full("launch").unwrap().unwrap().status, "pending");
        assert!(crate::agents::lifecycle::capture_launch_failure_evidence(&row).unwrap().contains("private-host-output"));
        db.conn().execute_batch("CREATE TRIGGER fail_launch_diagnosis BEFORE INSERT ON events WHEN NEW.type='status' AND NEW.instance='launch' BEGIN SELECT RAISE(ABORT,'injected diagnosis event failure'); END;").unwrap();
        assert!(crate::agents::lifecycle::try_finalize_launch_failure_detail_stored(db, &row, None).is_err());
        assert_eq!(db.get_instance_full("launch").unwrap().unwrap().status, "pending");
        db.conn().execute_batch("DROP TRIGGER fail_launch_diagnosis").unwrap();
        let detail = crate::agents::lifecycle::try_finalize_launch_failure_detail_stored(db, &row, None).unwrap().unwrap();
        assert!(!detail.contains("private-host-output"));
        assert_eq!(db.get_instance_full("launch").unwrap().unwrap().status_context, "launch_failed");
    });
}
