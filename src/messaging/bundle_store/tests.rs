//! Bundle scenarios use the public nested Messages operation, not a copied SQL client.
use super::*;
use crate::service::{CallerContext, CallerProcess, RequestId};

fn fixture(test: impl FnOnce(&AgentxDb, RequestContext)) {
    let root = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    db.save_instance_named("lobo", json!({"created_at":10.0,"tool":"adhoc","status":"active"}).as_object().unwrap()).unwrap();
    let context = RequestContext {
        caller: CallerContext { request_id: RequestId("bundle-case".into()), invocation: InvocationContext { explicit_name: Some("lobo".into()), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid: std::process::id(), incarnation: Some("bundle-fixture-peer".into()) },
        daemon_instance: "bundle-test".into(),
        database_generation: db.database_generation().unwrap(),
    };
    test(&db, context);
}
fn payload() -> Value {
    json!({"title":"Context","description":"one original intent","created_by":"forged","refs":{"events":["1-9223372036854775807"],"files":["missing-at-daemon-cwd"],"transcript":["1:normal"]},"extends":"missing"})
}
fn dispatch(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    match crate::messaging::service::handle(db, context, crate::messaging::service::Request::Bundle(request))? {
        crate::messaging::service::Response::Bundle(response) => Ok(response),
        _ => panic!("bundle response"),
    }
}
fn created(db: &AgentxDb, context: &RequestContext, bundle: Value) -> ServiceResult<Created> {
    match dispatch(db, context, Request::Create { bundle, actor: context.caller.invocation.explicit_name.as_ref().map(|name| ParticipantRef { canonical_name: name.clone(), incarnation_bits: 10.0f64.to_bits() }) })? {
        Response::Created(result) => Ok(result),
        _ => panic!("create result"),
    }
}
fn count(db: &AgentxDb) -> i64 {
    db.conn().query_row("SELECT count(*) FROM events WHERE type='bundle'", [], |row| row.get(0)).unwrap()
}

#[test]
fn create_replays_frozen_event_and_rejects_intent_peer_and_namesake_changes() {
    fixture(|db, context| {
        let first = created(db, &context, payload()).unwrap();
        let replay = created(db, &context, payload()).unwrap();
        assert!(replay.replayed);
        assert_eq!(first.event_id, replay.event_id);
        assert_eq!(first.bundle_id, replay.bundle_id);
        assert_eq!(replay.warnings, vec!["Parent bundle not found: missing"]);
        let stored: String = db.conn().query_row("SELECT json_extract(data,'$.created_by') FROM events WHERE id=?1", [first.event_id], |row| row.get(0)).unwrap();
        assert_eq!(stored, "lobo");
        let mut changed = payload();
        changed["description"] = json!("other intent");
        assert!(matches!(created(db, &context, changed), Err(ServiceError::Conflict(_))));
        let mut other = context.clone();
        other.process.incarnation = Some("new-peer".into());
        assert!(matches!(created(db, &other, payload()), Err(ServiceError::Conflict(_))));
        db.update_instance_fields("lobo", json!({"created_at":11.0}).as_object().unwrap()).unwrap();
        assert!(matches!(created(db, &context, payload()), Err(ServiceError::StaleFence)));
        assert_eq!(count(db), 1);
    });
}

#[test]
fn original_actor_fence_rejects_replacement_before_first_create() {
    fixture(|db, context| {
        db.update_instance_fields("lobo", json!({"created_at":11.0}).as_object().unwrap()).unwrap();
        assert!(matches!(created(db, &context, payload()), Err(ServiceError::StaleFence)));
        assert_eq!(count(db), 0);
    });
}

#[test]
fn event_and_retained_result_failures_rollback_together_and_same_id_can_retry() {
    for trigger in [
        "CREATE TRIGGER fail_bundle BEFORE INSERT ON events WHEN NEW.type='bundle' BEGIN SELECT RAISE(ABORT,'bundle event failure'); END;",
        "CREATE TRIGGER fail_bundle BEFORE INSERT ON kv WHEN NEW.key LIKE 'bundle_create:%' BEGIN SELECT RAISE(ABORT,'bundle result failure'); END;",
    ] {
        fixture(|db, context| {
            db.conn().execute_batch(trigger).unwrap();
            assert!(created(db, &context, payload()).is_err());
            assert_eq!(count(db), 0);
            assert!(db.kv_get(&format!("bundle_create:{}:{}", context.database_generation, context.caller.request_id.0)).unwrap().is_none());
            db.conn().execute_batch("DROP TRIGGER fail_bundle").unwrap();
            assert!(!created(db, &context, payload()).unwrap().replayed);
            assert_eq!(count(db), 1);
        });
    }
}

#[test]
fn only_unclaimed_human_can_create_anonymous_bundle() {
    fixture(|db, mut context| {
        context.caller.invocation = InvocationContext::default();
        let result = created(db, &context, payload()).unwrap();
        let (instance, by): (String, Option<String>) = db.conn().query_row("SELECT instance,json_extract(data,'$.created_by') FROM events WHERE id=?1", [result.event_id], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(instance, "cli");
        assert!(by.is_none());
        context.caller.request_id.0 = "unbound-claimed".into();
        context.caller.invocation.process_binding_id = Some("expired-binding".into());
        assert!(matches!(created(db, &context, payload()), Err(ServiceError::IdentityRequired(_))));
        assert_eq!(count(db), 1);
    });
}

#[test]
fn literal_prefix_numeric_lookup_and_required_read_failure_are_distinct() {
    fixture(|db, context| {
        let mut input = payload();
        input["bundle_id"] = json!("bundle:a_b");
        let first = created(db, &context, input).unwrap();
        assert_eq!(lookup(db, &first.event_id.to_string()).unwrap().unwrap()["bundle_id"], "bundle:a_b");
        assert_eq!(lookup(db, "a_").unwrap().unwrap()["event_id"], first.event_id);
        assert!(lookup(db, "a%").unwrap().is_none());
        assert!(lookup(db, "missing").unwrap().is_none());
        db.conn().execute_batch("ALTER TABLE events RENAME TO fixture_events").unwrap();
        assert!(dispatch(db, &context, Request::Lookup { id: "missing".into() }).is_err());
    });
}

#[test]
fn referenced_ranges_page_without_expansion_or_cursor_loss_and_freeze_watermark() {
    fixture(|db, context| {
        let original = (0..1003).map(|n| db.log_event("status", "lobo", &json!({"status":"active","detail":n})).unwrap()).collect::<Vec<_>>();
        let refs = vec![json!("1-9223372036854775807"), json!(original[0]), json!("invalid")];
        let first = referenced(db, &refs, None, None, usize::MAX).unwrap();
        assert_eq!(first.events.len(), MAX_PAGE_EVENTS);
        assert!(first.has_more);
        assert_eq!(first.next, Some(original[999]));
        db.log_event("status", "lobo", &json!({"status":"active","detail":"after-watermark"})).unwrap();
        let next = referenced(db, &refs, first.next, Some(first.through), usize::MAX).unwrap();
        assert_eq!(next.events.iter().map(|event| event["id"].as_i64().unwrap()).collect::<Vec<_>>(), original[1000..]);
        assert!(!next.has_more);
        assert_eq!(context.database_generation, db.database_generation().unwrap());
    });
}

#[test]
fn byte_boundary_continuation_and_single_oversized_event_are_explicit() {
    fixture(|db, _| {
        let ids = (0..3).map(|_| db.log_event("status", "lobo", &json!({"text":"x".repeat(800_000)})).unwrap()).collect::<Vec<_>>();
        let refs = vec![json!("1-9223372036854775807")];
        let first = referenced(db, &refs, None, None, 1000).unwrap();
        assert_eq!(first.events.len(), 2);
        assert_eq!(first.next, Some(ids[1]));
        assert!(first.has_more);
        let second = referenced(db, &refs, first.next, Some(first.through), 1000).unwrap();
        assert_eq!(second.events.len(), 1);
        assert_eq!(second.events[0]["id"], ids[2]);
        db.log_event("status", "lobo", &json!({"text":"x".repeat(MAX_PAGE_BYTES+1)})).unwrap();
        assert!(matches!(referenced(db, &refs, Some(ids[2]), None, 1000), Err(ServiceError::InvalidInput(_))));
    });
}

#[test]
fn context_categories_preserve_local_membership_and_file_operation_predicates() {
    fixture(|db, _| {
        let to = db.log_event("message", "other", &json!({"from":"other","delivered_to":["lobo"]})).unwrap();
        let from = db.log_event("message", "lobo", &json!({"from":"lobo","delivered_to":["other"]})).unwrap();
        let file = db.log_event("status", "lobo", &json!({"status":"active","context":"tool:Write","detail":"/work/main.rs"})).unwrap();
        db.log_event("status", "other", &json!({"status":"active","context":"tool:Write","detail":"/work/other.rs"})).unwrap();
        let life = db.log_event("life", "other", &json!({"action":"stopped","by":"lobo"})).unwrap();
        for (category, expected) in [(Category::MessagesTo, to), (Category::MessagesFrom, from), (Category::FileOperations, file), (Category::Lifecycle, life)] {
            let page = context_events(db, "lobo", category, None, None, 10).unwrap();
            assert_eq!(page.events.len(), 1);
            assert_eq!(page.events[0]["id"], expected);
        }
    });
}
