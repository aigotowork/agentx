//! Observer endpoint ownership and postcommit wake cases; integration executes.
use super::*;
use crate::events::service::{Request, Response};
use crate::runtime::notify::NotifyServer;
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};
use serde_json::json;
use std::time::Duration;

fn fixture(body: impl FnOnce(&mut AgentxDb, &RequestContext, ObserverSpec, &NotifyServer)) {
    let root = tempfile::tempdir().unwrap();
    let mut db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    db.save_instance_named("luna", json!({"created_at":100.0,"tool":"adhoc","status":"active","status_context":"tool:Bash","wait_timeout":12345,"tcp_mode":1,"last_stop":42,"last_event_id":0}).as_object().unwrap()).unwrap();
    let context = RequestContext {
        caller: CallerContext { request_id: RequestId("observer-test".into()), invocation: InvocationContext { explicit_name: Some("luna".into()), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
        daemon_instance: "observer-fixture".into(),
        database_generation: db.database_generation().unwrap(),
    };
    let server = NotifyServer::new().unwrap();
    let spec = ObserverSpec { participant: crate::agents::attachments::ParticipantRef { canonical_name: "luna".into(), incarnation_bits: 100.0f64.to_bits() }, id: format!("observe-{}", uuid::Uuid::new_v4()), port: server.port() };
    body(&mut db, &context, spec, &server);
}

#[test]
fn postcommit_async_observer_waits_for_commit_without_holding_dispatch_and_revokes_ended_hint() {
    use crate::observability::post_commit::Dispatch;
    use crate::runtime::notify::observer_hints::ObserverHints;
    fixture(|db, context, original, _| {
        let generation = crate::daemon::DatabaseGeneration::default();
        generation.publish(context.database_generation.clone()).unwrap();
        let (entered, waiting) = std::sync::mpsc::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let (sent, sent_hint) = std::sync::mpsc::channel();
        let first_port = original.port;
        db.install_observer_hints(generation, |generation, revision| {
            ObserverHints::with_sender(
                generation,
                revision,
                move |port| {
                    if port == first_port {
                        entered.send(()).unwrap();
                        let _ = gate.recv_timeout(Duration::from_secs(5));
                    }
                },
                move |port| {
                    let _ = sent.send(port);
                },
            )
        })
        .unwrap();
        begin(db, context, original.clone()).unwrap();
        let capture = Dispatch::begin();
        let rollback: anyhow::Result<()> = db.with_write_scope(|| {
            db.log_event("status", "luna", &json!({"status":"active"}))?;
            anyhow::bail!("rollback event")
        });
        assert!(rollback.is_err());
        assert_eq!(capture.work().map_or(0, |work| work.observer_hints_queued), 0);
        db.with_write_scope(|| {
            db.log_event("status", "luna", &json!({"status":"active"}))?;
            db.log_event("status", "luna", &json!({"status":"active"}))?;
            Ok(())
        })
        .unwrap();
        waiting.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(capture.work().unwrap().observer_hints_queued, 1);
        assert!(sent_hint.try_recv().is_err());
        end(db, context, &original).unwrap();
        let another = NotifyServer::new().unwrap();
        let successor = ObserverSpec { id: format!("observe-{}", uuid::Uuid::new_v4()), port: another.port(), ..original };
        begin(db, context, successor.clone()).unwrap();
        db.log_event("status", "luna", &json!({"status":"active"})).unwrap();
        release.send(()).unwrap();
        assert_eq!(sent_hint.recv_timeout(Duration::from_secs(2)).unwrap(), successor.port);
    });
}
fn dispatch(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    super::super::store::handle(db, context, request)
}
fn registered(db: &AgentxDb, spec: &ObserverSpec) -> bool {
    db.get_notify_endpoint_port(&endpoint(&spec.id), "events_wait") == Some(spec.port)
}

#[test]
fn observer_lifetime_is_idempotent_and_never_changes_participant_or_mailbox_state() {
    fixture(|db, context, spec, _| {
        let before = serde_json::to_value(db.get_instance_full("luna").unwrap().unwrap()).unwrap();
        assert!(matches!(dispatch(db, context, Request::BeginObserver(spec.clone())).unwrap(), Response::ObserverReady));
        dispatch(db, context, Request::BeginObserver(spec.clone())).unwrap();
        dispatch(db, context, Request::CheckObserver(spec.clone())).unwrap();
        assert!(registered(db, &spec));
        dispatch(db, context, Request::EndObserver(spec.clone())).unwrap();
        dispatch(db, context, Request::EndObserver(spec.clone())).unwrap();
        assert!(!registered(db, &spec));
        assert_eq!(dispatch(db, context, Request::BeginObserver(spec)).unwrap_err(), ServiceError::StaleFence);
        assert_eq!(serde_json::to_value(db.get_instance_full("luna").unwrap().unwrap()).unwrap(), before);
        assert!(db.kv_prefix("events_sub:").unwrap().is_empty());
    });
}

#[test]
fn only_committed_events_wake_an_observer_including_unrelated_status_events() {
    fixture(|db, context, spec, server| {
        begin(db, context, spec).unwrap();
        let rollback: Result<(), anyhow::Error> = db.with_write_scope(|| {
            db.log_event("status", "other", &json!({"status":"active"}))?;
            anyhow::bail!("rollback event");
        });
        assert!(rollback.is_err());
        assert!(!server.wait(Duration::from_millis(25)), "rollback must emit no observer wake");
        db.with_write_scope(|| {
            db.log_event("status", "other", &json!({"status":"active"}))?;
            assert!(!server.wait(Duration::from_millis(25)), "no wake while transaction is open");
            Ok(())
        })
        .unwrap();
        assert!(server.wait(Duration::from_secs(1)), "committed unrelated status must wake Events");
    });
}

#[test]
fn observer_rejects_other_peer_port_generation_and_namesake_successor() {
    fixture(|db, context, spec, _| {
        begin(db, context, spec.clone()).unwrap();
        let mut other = context.clone();
        other.process.incarnation = Some("other peer".into());
        assert_eq!(end(db, &other, &spec).unwrap_err(), ServiceError::StaleFence);
        let changed = ObserverSpec { port: if spec.port == 65535 { 65534 } else { spec.port + 1 }, ..spec.clone() };
        assert_eq!(end(db, context, &changed).unwrap_err(), ServiceError::StaleFence);
        other = context.clone();
        other.database_generation = "other generation".into();
        assert_eq!(check(db, &other, &spec).unwrap_err(), ServiceError::StaleFence);
        db.update_instance_fields("luna", json!({"created_at":200.0}).as_object().unwrap()).unwrap();
        assert_eq!(check(db, context, &spec).unwrap_err(), ServiceError::StaleFence);
        assert_eq!(end(db, context, &spec).unwrap_err(), ServiceError::StaleFence);
        assert!(registered(db, &spec));
    });
}

#[test]
fn concurrent_observers_keep_separate_endpoints_even_when_ports_are_reused() {
    fixture(|db, context, older, _| {
        let newer = ObserverSpec { id: format!("observe-{}", uuid::Uuid::new_v4()), ..older.clone() };
        begin(db, context, older.clone()).unwrap();
        begin(db, context, newer.clone()).unwrap();
        end(db, context, &older).unwrap();
        assert!(!registered(db, &older));
        assert!(registered(db, &newer));
        check(db, context, &newer).unwrap();
    });
}

#[test]
fn required_observer_setup_and_cleanup_failures_roll_back_the_whole_lifetime() {
    fixture(|db, context, spec, _| {
        db.conn().execute_batch("CREATE TRIGGER fail_observer_insert BEFORE INSERT ON kv WHEN NEW.key LIKE 'event_observer:%' BEGIN SELECT RAISE(ABORT,'observer save'); END;").unwrap();
        assert!(begin(db, context, spec.clone()).is_err());
        assert!(!registered(db, &spec));
        db.conn().execute_batch("DROP TRIGGER fail_observer_insert;").unwrap();
        begin(db, context, spec.clone()).unwrap();
        db.conn().execute_batch("CREATE TRIGGER fail_observer_delete BEFORE DELETE ON notify_endpoints WHEN OLD.kind='events_wait' BEGIN SELECT RAISE(ABORT,'observer delete'); END;").unwrap();
        assert!(end(db, context, &spec).is_err());
        check(db, context, &spec).unwrap();
        db.conn().execute_batch("DROP TRIGGER fail_observer_delete;").unwrap();
        end(db, context, &spec).unwrap();
        assert!(!registered(db, &spec));
    });
}

#[test]
fn postcommit_wake_prunes_retired_or_reused_process_owners_without_connecting() {
    fixture(|db, context, spec, server| {
        begin(db, context, spec.clone()).unwrap();
        let mut authority = load(db, context, &spec).unwrap();
        authority.incarnation = "previous OS incarnation".into();
        save(db, &context.database_generation, &authority).unwrap();
        db.log_event("status", "other", &json!({})).unwrap();
        assert!(!registered(db, &spec));
        assert!(!server.wait(Duration::from_millis(25)));
        assert_eq!(begin(db, context, spec).unwrap_err(), ServiceError::StaleFence);
    });
}
