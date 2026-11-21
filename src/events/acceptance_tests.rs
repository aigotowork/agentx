//! Independent bounded forward-history cases. Reads never acknowledge durable
//! participant delivery; a page boundary must leave its first unread row visible.

use super::service::{EventPage, EventQuery, MAX_PAGE_BYTES, MAX_PAGE_EVENTS, Request, Response};
use crate::db::AgentxDb;
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestContext, RequestId, ServiceError};

fn context() -> RequestContext {
    RequestContext {
        caller: CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
        daemon_instance: "independent-event-fixture".into(),
        database_generation: "event-fixture-generation".into(),
    }
}

fn with_db(body: impl FnOnce(&AgentxDb)) {
    let directory = tempfile::Builder::new().prefix("agentx-page-budget-").tempdir().unwrap();
    let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
    body(&db);
}

fn insert(db: &AgentxDb, payload: &str) -> i64 {
    db.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES ('2026-11-07T00:00:00Z','message','lobo',?1)", [serde_json::json!({"text": payload}).to_string()]).unwrap();
    db.conn().last_insert_rowid()
}

fn page(db: &AgentxDb, after: i64, limit: usize) -> EventPage {
    let Response::Page(page) = super::store::handle(db, &context(), Request::Query(EventQuery { after: Some(after), limit, oldest_first: true, ..Default::default() })).unwrap() else {
        panic!("event page response");
    };
    page
}

#[test]
fn huge_requested_page_is_clamped_and_forward_resume_has_no_gap() {
    with_db(|db| {
        db.with_write_scope(|| {
            for _ in 0..MAX_PAGE_EVENTS + 3 {
                insert(db, "small");
            }
            Ok(())
        })
        .unwrap();
        let first = page(db, 0, usize::MAX);
        assert_eq!(first.events.len(), MAX_PAGE_EVENTS);
        assert!(first.has_more);
        assert_eq!(first.next_after, first.events.last().unwrap()["id"].as_i64().unwrap());
        let rest = page(db, first.next_after, usize::MAX);
        assert_eq!(rest.events.len(), 3);
        assert!(!rest.has_more);
        assert_eq!(rest.events[0]["id"].as_i64(), Some(first.next_after + 1));
        assert_eq!(rest.next_after, db.get_last_event_id());
    });
}

#[test]
fn byte_budget_keeps_overflow_row_for_the_next_page() {
    with_db(|db| {
        let payload = "x".repeat(MAX_PAGE_BYTES / 2 + 1024);
        let one = insert(db, &payload);
        let two = insert(db, &payload);
        let three = insert(db, "tail");
        let first = page(db, 0, 100);
        assert_eq!(first.events.len(), 1);
        assert_eq!(first.next_after, one);
        assert!(first.has_more);
        assert!(first.events.iter().map(|event| serde_json::to_vec(event).unwrap().len()).sum::<usize>() <= MAX_PAGE_BYTES);
        let next = page(db, first.next_after, 100);
        assert_eq!(next.events.iter().filter_map(|event| event["id"].as_i64()).collect::<Vec<_>>(), vec![two, three]);
        assert_eq!(next.next_after, three);
        assert!(!next.has_more);
    });
}

#[test]
fn oversized_event_fails_explicitly_instead_of_skipping_it() {
    with_db(|db| {
        let id = insert(db, &"x".repeat(MAX_PAGE_BYTES + 1));
        let result = super::store::handle(db, &context(), Request::Query(EventQuery { limit: usize::MAX, oldest_first: true, ..Default::default() }));
        assert!(matches!(result, Err(ServiceError::InvalidInput(message)) if message.contains(&format!("Event {id}")) && message.contains("budget")));
    });
}

#[test]
fn abandoned_page_does_not_advance_participant_delivery_cursor() {
    with_db(|db| {
        db.save_instance_named("lobo", serde_json::json!({"created_at":10.0,"tool":"codex","last_event_id":0}).as_object().unwrap()).unwrap();
        let event = insert(db, "unacknowledged");
        drop(page(db, 0, 1));
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().last_event_id, 0);
        assert_eq!(page(db, 0, 1).events[0]["id"].as_i64(), Some(event));
    });
}
