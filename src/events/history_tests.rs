//! Archive/live continuation cases authored for independent integration.
use super::*;
use crate::events::service::{Request, Response};
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn context(db: &AgentxDb) -> RequestContext {
    RequestContext {
        caller: CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
        daemon_instance: "history-test".into(),
        database_generation: db.database_generation().unwrap(),
    }
}
fn archive(root: &std::path::Path, name: &str) -> AgentxDb {
    let directory = root.join("archive").join(name);
    std::fs::create_dir_all(&directory).unwrap();
    AgentxDb::open_at(&directory.join("agentx.db")).unwrap()
}
fn insert(db: &AgentxDb, timestamp: &str, who: &str, text: &str) -> i64 {
    db.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES (?1,'message',?2,?3)", rusqlite::params![timestamp, who, json!({"text":text,"thread":"history"}).to_string()]).unwrap();
    db.conn().last_insert_rowid()
}
fn page(db: &AgentxDb, context: &RequestContext, query: EventQuery) -> ServiceResult<EventPage> {
    let Response::Page(page) = super::super::store::handle(db, context, Request::Query(query))? else { panic!("history page") };
    Ok(page)
}
fn collect(db: &AgentxDb, context: &RequestContext, mut query: EventQuery, first: Option<EventPage>) -> Vec<Value> {
    let mut result = first;
    let mut events = Vec::new();
    let mut snapshot = None;
    loop {
        let current = result.take().unwrap_or_else(|| page(db, context, query.clone()).unwrap());
        assert!(current.events.len() <= MAX_PAGE_EVENTS);
        assert!(current.events.iter().map(|event| serde_json::to_vec(event).unwrap().len()).sum::<usize>() <= MAX_PAGE_BYTES);
        if let Some(cursor) = &current.history {
            snapshot = Some(cursor.snapshot.clone());
        }
        events.extend(current.events);
        if !current.has_more {
            break;
        }
        query.history = current.history;
    }
    if let Some(snapshot) = snapshot {
        super::super::store::handle(db, context, Request::EndHistory { snapshot }).unwrap();
    }
    events
}

#[test]
fn archive_pages_merge_interleaved_sources_and_id_timestamp_ties_without_gaps() {
    let root = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    let a = archive(root.path(), "session-a");
    let b = archive(root.path(), "session-b");
    let mut expected = Vec::new();
    for (name, source) in [("current", &db), ("session-a", &a), ("session-b", &b)] {
        source
            .with_write_scope(|| {
                for i in 0..407 {
                    // Write order deliberately differs from time order (Relay).
                    let ts = format!("2026-11-07T00:{:02}:{:02}Z", (406 - i) / 60, (406 - i) % 60);
                    let id = insert(source, &ts, "luna", "same event ids in each source");
                    expected.push(json!({"id":id,"ts":ts,"source":name}));
                }
                Ok(())
            })
            .unwrap();
    }
    let query = EventQuery { include_archives: true, limit: usize::MAX, ..Default::default() };
    let events = collect(&db, &context(&db), query, None);
    expected.sort_by(|a, b| history_order(b, a));
    assert_eq!(events.len(), 1221);
    assert_eq!(events.iter().map(|e| (&e["ts"], &e["source"], &e["id"])).collect::<Vec<_>>(), expected.iter().map(|e| (&e["ts"], &e["source"], &e["id"])).collect::<Vec<_>>());
}

#[test]
fn archive_snapshot_freezes_sources_watermarks_and_native_filters() {
    let root = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    let a = archive(root.path(), "session-a");
    db.save_instance_named("luna", json!({"created_at":1.0,"tool":"adhoc"}).as_object().unwrap()).unwrap();
    for source in [&db, &a] {
        for _ in 0..4 {
            insert(source, "2026-11-07T00:00:00Z", "luna", "original");
        }
    }
    let context = context(&db);
    let query = EventQuery { include_archives: true, limit: 2, filters: std::collections::HashMap::from([("instance".into(), vec!["luna".into()])]), ..Default::default() };
    let first = page(&db, &context, query.clone()).unwrap();
    assert!(first.has_more);
    for source in [&db, &a] {
        insert(source, "2026-11-08T00:00:00Z", "luna", "late append");
    }
    let later = archive(root.path(), "session-later");
    insert(&later, "2026-11-09T00:00:00Z", "luna", "new source");
    let events = collect(&db, &context, query, Some(first));
    assert_eq!(events.len(), 8);
    assert!(events.iter().all(|event| event["data"]["text"] == "original"));
    assert!(db.kv_prefix("event_history:").unwrap().is_empty());
}

#[test]
fn archive_byte_boundary_and_ascending_continuation_preserve_the_overflow_row() {
    let root = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    let a = archive(root.path(), "session-a");
    let text = "x".repeat(MAX_PAGE_BYTES / 2 + 1024);
    for source in [&db, &a] {
        for _ in 0..3 {
            insert(source, "2026-11-07T00:00:00Z", "luna", &text);
        }
    }
    let events = collect(&db, &context(&db), EventQuery { include_archives: true, oldest_first: true, limit: 100, ..Default::default() }, None);
    assert_eq!(events.len(), 6);
    assert!(events.windows(2).all(|events| history_order(&events[0], &events[1]).is_lt()));
    assert_eq!(events.iter().map(|event| format!("{}:{}", event["source"], event["id"])).collect::<std::collections::HashSet<_>>().len(), 6);
}

#[test]
fn history_predicate_and_structured_filters_are_retained_across_pages() {
    let root = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    let a = archive(root.path(), "session-a");
    for source in [&db, &a] {
        for who in ["luna", "muse", "other"] {
            for _ in 0..3 {
                insert(source, "2026-11-07T00:00:00Z", who, "wanted");
            }
        }
    }
    let query = EventQuery {
        include_archives: true,
        limit: 2,
        filters: std::collections::HashMap::from([("instance".into(), vec!["luna".into(), "muse".into()]), ("thread".into(), vec!["history".into()])]),
        predicate: Some("json_extract(data,'$.text')='wanted' AND NOT EXISTS(SELECT 1 FROM events WHERE instance='late')".into()),
        ..Default::default()
    };
    let context = context(&db);
    let first = page(&db, &context, query.clone()).unwrap();
    for source in [&db, &a] {
        insert(source, "2026-11-07T00:00:00Z", "late", "new subquery witness");
    }
    let events = collect(&db, &context, query, Some(first));
    assert_eq!(events.len(), 12);
    assert!(events.iter().all(|event| event["instance"] == "luna" || event["instance"] == "muse"));
}

#[test]
fn history_continuation_rejects_changed_intent_peer_generation_and_source_identity() {
    let root = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    let a = archive(root.path(), "session-a");
    for source in [&db, &a] {
        for _ in 0..3 {
            insert(source, "2026-11-07T00:00:00Z", "luna", "original");
        }
    }
    let context = context(&db);
    let mut query = EventQuery { include_archives: true, limit: 1, ..Default::default() };
    query.history = page(&db, &context, query.clone()).unwrap().history;
    let mut changed = query.clone();
    changed.predicate = Some("1=1".into());
    assert!(matches!(page(&db, &context, changed), Err(ServiceError::Conflict(_))));
    let mut impostor = context.clone();
    impostor.process.incarnation = Some("replacement peer".into());
    assert_eq!(page(&db, &impostor, query.clone()).unwrap_err(), ServiceError::StaleFence);
    impostor = context.clone();
    impostor.database_generation = "replacement generation".into();
    assert_eq!(page(&db, &impostor, query.clone()).unwrap_err(), ServiceError::StaleFence);
    a.kv_set("service:database_generation", Some("replacement archive")).unwrap();
    assert_eq!(page(&db, &context, query.clone()).unwrap_err(), ServiceError::StaleFence);
    a.kv_set("service:database_generation", None).unwrap();
    drop(a);
    std::fs::rename(root.path().join("archive/session-a"), root.path().join("archive/session-retired")).unwrap();
    assert_eq!(page(&db, &context, query).unwrap_err(), ServiceError::StaleFence);
}

#[test]
fn ordinary_unknown_agent_warning_keeps_live_and_historical_name_parity() {
    let root = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    db.save_instance_named("luna", json!({"created_at":1.0,"tool":"adhoc"}).as_object().unwrap()).unwrap();
    insert(&db, "2026-11-07T00:00:00Z", "stopped", "known historical row");
    let mut query = EventQuery { limit: 20, warn_unknown_agents: true, filters: std::collections::HashMap::from([("instance".into(), vec!["luna".into(), "stopped".into(), "luno".into()])]), ..Default::default() };
    let result = page(&db, &context(&db), query.clone()).unwrap();
    assert_eq!(result.resolved_filters["instance"], ["luna", "stopped", "luno"]);
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("No agent named 'luno'"));
    assert!(result.warnings[0].contains("luna"));
    query.warn_unknown_agents = false;
    assert!(page(&db, &context(&db), query.clone()).unwrap().warnings.is_empty());
    query.warn_unknown_agents = true;
    query.include_archives = true;
    assert!(page(&db, &context(&db), query).unwrap().warnings.is_empty());
}

#[test]
fn historical_projection_uses_source_columns_without_rewriting_archive_or_message_payloads() {
    let root = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    let archived = archive(root.path(), "session-old");
    archived.conn().execute_batch("ALTER TABLE instances ADD COLUMN retired_metadata TEXT").unwrap();
    let created_at = f64::from_bits(1_700_000_000.123_456_f64.to_bits() + 1);
    let old = json!({"action":"stopped","retired_metadata":"old","snapshot":{"name":"luna","created_at":created_at,"last_event_id":17,"retired_metadata":"old"}});
    archived.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES('2026-11-07T00:00:00Z','life','luna',?1)", [old.to_string()]).unwrap();
    let life_id = archived.conn().last_insert_rowid();
    let message = json!({"text":"user payload","retired_metadata":"keep","snapshot":{"retired_metadata":"keep"}});
    archived.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES('2026-11-07T00:00:00Z','message','luna',?1)", [message.to_string()]).unwrap();
    let live = json!({"action":"stopped","snapshot":old["snapshot"]});
    db.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES('2026-11-07T00:00:00Z','life','luna',?1)", [live.to_string()]).unwrap();
    let expected = json!({"action":"stopped","snapshot":{"name":"luna","created_at":created_at,"last_event_id":17}});
    let live_page = page(&db, &context(&db), EventQuery { limit: 10, ..Default::default() }).unwrap();
    assert_eq!(live_page.events.len(), 1);
    assert_eq!(live_page.events[0]["data"], expected);
    assert_eq!(db.get_events_since(0, None, None).unwrap()[0]["data"], expected);
    assert_eq!(archived.get_events_since(0, Some("life"), None).unwrap()[0]["data"], expected);

    let archive_path = archived.path().to_path_buf();
    drop(archived);
    let archive_bytes = std::fs::read(&archive_path).unwrap();
    let events = collect(&db, &context(&db), EventQuery { include_archives: true, limit: 10, ..Default::default() }, None);
    assert_eq!(events.len(), 3);
    for event in &events {
        if event["type"] == "life" {
            assert_eq!(event["data"], expected);
            assert_eq!(event["data"]["snapshot"]["created_at"].as_f64().unwrap().to_bits(), created_at.to_bits());
        } else {
            assert_eq!(event["data"], message);
        }
    }
    assert!(events.iter().any(|event| event["source"] == "session-old" && event["id"] == life_id));
    assert_eq!(std::fs::read(&archive_path).unwrap(), archive_bytes);
    let raw = rusqlite::Connection::open_with_flags(&archive_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let original: String = raw.query_row("SELECT data FROM events WHERE id=?1", [life_id], |row| row.get(0)).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&original).unwrap(), old);
}
