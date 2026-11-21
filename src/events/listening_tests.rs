//! Exact native listener invariants, authored for independent integration.
use super::*;
use crate::events::service::{Request, Response};
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};
use serde_json::json;

fn fixture(body: impl FnOnce(&AgentxDb, &RequestContext)) {
    let directory = tempfile::Builder::new().prefix("agentx-native-listen-").tempdir().unwrap();
    let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
    db.save_instance_named("luna", json!({"tool":"adhoc","created_at":100.0,"status":"inactive","status_context":"tool:send","wait_timeout":86400,"tcp_mode":0,"last_stop":0}).as_object().unwrap()).unwrap();
    let context = RequestContext {
        caller: CallerContext {
            request_id: RequestId("listen-fixture".into()),
            invocation: InvocationContext { cwd: directory.path().into(), explicit_name: Some("luna".into()), ..Default::default() },
            deadline_unix_ms: None,
            expected_database_generation: None,
        },
        process: CallerProcess { pid: std::process::id(), incarnation: Some("authenticated-listener".into()) },
        daemon_instance: "fixture".into(),
        database_generation: db.database_generation().unwrap(),
    };
    body(&db, &context);
}
fn spec() -> ListenSpec {
    ListenSpec { participant: ParticipantRef { canonical_name: "luna".into(), incarnation_bits: 100.0f64.to_bits() }, id: format!("listen-{}", uuid::Uuid::new_v4()), port: Some(24567), filters: Default::default(), predicate: None }
}
fn filtered() -> ListenSpec {
    ListenSpec { predicate: Some("type='fixture-listen'".into()), ..spec() }
}
fn dispatch(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    super::super::store::handle(db, context, request)
}

#[test]
fn listener_heartbeat_and_timeout_preserve_persistent_wait_and_tcp_mode() {
    fixture(|db, context| {
        for mut listen in [spec(), filtered()] {
            let ready = begin(db, context, listen.clone()).unwrap();
            assert!(ready.recent.is_none());
            assert!(pulse(db, context, &listen.participant, &listen.id).unwrap());
            let row = db.get_instance_full("luna").unwrap().unwrap();
            assert_eq!(row.wait_timeout, Some(86400));
            assert_eq!(row.tcp_mode, 0);
            assert!(row.last_stop > 0);
            if ready.filtered {
                let subscription: serde_json::Value = serde_json::from_str(&db.kv_get(&format!("events_sub:{}", listen.id)).unwrap().unwrap()).unwrap();
                let last_status: i64 = db.conn().query_row("SELECT MAX(id) FROM events WHERE type='status' AND instance='luna'", [], |row| row.get(0)).unwrap();
                assert!(subscription["last_id"].as_i64().unwrap() >= last_status, "own listening status must precede the subscription cursor");
            }
            end(db, context, &listen.participant, &listen.id, Some(ListenEnd::Timeout)).unwrap();
            let row = db.get_instance_full("luna").unwrap().unwrap();
            assert_eq!((row.status.as_str(), row.status_context.as_str()), ("inactive", "listen timeout"));
            assert_eq!(row.wait_timeout, Some(86400));
            assert_eq!(row.tcp_mode, 0);
            // Explicitly test that NULL keeps the caller's configuration default.
            listen.id = format!("listen-{}", uuid::Uuid::new_v4());
            db.update_instance_fields("luna", json!({"wait_timeout":null}).as_object().unwrap()).unwrap();
            begin(db, context, listen.clone()).unwrap();
            end(db, context, &listen.participant, &listen.id, Some(ListenEnd::Timeout)).unwrap();
            assert_eq!(db.get_instance_full("luna").unwrap().unwrap().wait_timeout, None);
            db.update_instance_fields("luna", json!({"wait_timeout":86400}).as_object().unwrap()).unwrap();
        }
    });
}

#[test]
fn hooked_listener_never_overwrites_mid_turn_status() {
    fixture(|db, context| {
        db.update_instance_fields("luna", json!({"tool":"claude","status":"active","status_context":"tool:Bash"}).as_object().unwrap()).unwrap();
        for listen in [spec(), filtered()] {
            begin(db, context, listen.clone()).unwrap();
            pulse(db, context, &listen.participant, &listen.id).unwrap();
            end(db, context, &listen.participant, &listen.id, Some(ListenEnd::Message)).unwrap();
        }
        let row = db.get_instance_full("luna").unwrap().unwrap();
        assert_eq!((row.status.as_str(), row.status_context.as_str()), ("active", "tool:Bash"));
        let statuses: i64 = db.conn().query_row("SELECT COUNT(*) FROM events WHERE type='status'", [], |row| row.get(0)).unwrap();
        assert_eq!(statuses, 0);
    });
}

#[test]
fn retained_listener_begin_never_recreates_fired_subscription() {
    fixture(|db, context| {
        let listen = filtered();
        let ready = begin(db, context, listen.clone()).unwrap();
        db.log_event("fixture-listen", "luna", &json!({"action":"match"})).unwrap();
        let sub_key = format!("events_sub:{}", listen.id);
        assert!(db.kv_get(&sub_key).unwrap().is_none(), "once subscription should have fired");
        let before = db.get_last_event_id();
        assert_eq!(serde_json::to_value(begin(db, context, listen.clone()).unwrap()).unwrap(), serde_json::to_value(ready).unwrap());
        assert!(db.kv_get(&sub_key).unwrap().is_none());
        assert_eq!(db.get_last_event_id(), before);
        let changed = ListenSpec { predicate: Some("type='changed'".into()), ..listen.clone() };
        assert!(matches!(begin(db, context, changed), Err(ServiceError::Conflict(_))));
        let mut impostor = context.clone();
        impostor.process.incarnation = Some("replacement-peer".into());
        assert_eq!(pulse(db, &impostor, &listen.participant, &listen.id).unwrap_err(), ServiceError::StaleFence);
    });
}

#[test]
fn listener_setup_required_failure_rolls_back_status_endpoint_and_subscription() {
    fixture(|db, context| {
        let listen = filtered();
        let before = db.get_last_event_id();
        db.conn().execute_batch("CREATE TRIGGER fail_listen_authority BEFORE INSERT ON kv WHEN NEW.key LIKE 'event_listener:%' BEGIN SELECT RAISE(ABORT,'injected listener retention failure'); END;").unwrap();
        assert!(begin(db, context, listen.clone()).is_err());
        assert!(db.kv_get(&format!("events_sub:{}", listen.id)).unwrap().is_none());
        assert!(db.get_notify_endpoint_port("luna", "listen_filter").is_none());
        assert_eq!(db.get_instance_full("luna").unwrap().unwrap().status, "inactive");
        assert_eq!(db.get_last_event_id(), before);
        db.conn().execute_batch("DROP TRIGGER fail_listen_authority").unwrap();
        begin(db, context, listen).unwrap();
    });
}

#[test]
fn older_listener_cleanup_preserves_new_endpoint_status_and_successor() {
    fixture(|db, context| {
        let older = spec();
        // A newer lifetime may reuse an OS port. Matching port alone cannot
        // authorize an older listener to remove that registration.
        let newer = ListenSpec { port: older.port, ..spec() };
        begin(db, context, older.clone()).unwrap();
        begin(db, context, newer.clone()).unwrap();
        end(db, context, &older.participant, &older.id, Some(ListenEnd::Timeout)).unwrap();
        assert_eq!(db.get_notify_endpoint_port("luna", "listen"), newer.port);
        assert_eq!(db.get_instance_full("luna").unwrap().unwrap().status, "listening");
        assert_eq!(pulse(db, context, &older.participant, &older.id).unwrap_err(), ServiceError::StaleFence);
        db.update_instance_fields("luna", json!({"created_at":200.0}).as_object().unwrap()).unwrap();
        assert_eq!(end(db, context, &newer.participant, &newer.id, Some(ListenEnd::Timeout)).unwrap_err(), ServiceError::StaleFence);
        assert_eq!(db.get_notify_endpoint_port("luna", "listen"), newer.port);
    });
}

#[test]
fn listener_cleanup_is_retained_and_refuses_persistent_subscription() {
    fixture(|db, context| {
        let listen = filtered();
        begin(db, context, listen.clone()).unwrap();
        let sub_key = format!("events_sub:{}", listen.id);
        db.kv_set(&sub_key, Some(&json!({"id":listen.id,"caller":"luna","sql":"type='status'","once":true}).to_string())).unwrap();
        assert!(end(db, context, &listen.participant, &listen.id, Some(ListenEnd::Timeout)).is_err());
        assert!(db.kv_get(&sub_key).unwrap().is_some());
        assert_eq!(db.get_notify_endpoint_port("luna", "listen_filter"), listen.port);
        db.kv_set(&sub_key, None).unwrap();
        end(db, context, &listen.participant, &listen.id, Some(ListenEnd::Timeout)).unwrap();
        db.update_instance_fields("luna", json!({"status":"active","status_context":"new activity"}).as_object().unwrap()).unwrap();
        end(db, context, &listen.participant, &listen.id, Some(ListenEnd::Timeout)).unwrap();
        assert_eq!(db.get_instance_full("luna").unwrap().unwrap().status_context, "new activity");
    });
}

#[test]
fn recent_listener_match_and_predicate_authority_use_native_dispatch() {
    fixture(|db, context| {
        let listen = filtered();
        let event = db.log_event("fixture-listen", "luna", &json!({"action":"recent"})).unwrap();
        let Response::ListenReady(ready) = dispatch(db, context, Request::BeginListen(listen)).unwrap() else {
            panic!("listener response");
        };
        assert_eq!(ready.recent.unwrap()["id"], event);
        assert_eq!(db.get_instance_full("luna").unwrap().unwrap().status, "inactive");
        let invalid = ListenSpec { predicate: Some("EXISTS(SELECT 1 FROM kv)".into()), ..spec() };
        assert!(matches!(dispatch(db, context, Request::BeginListen(invalid)), Err(ServiceError::InvalidInput(_))));
    });
}
