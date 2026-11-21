//! Native scenarios derived from classification/write/lifecycle/query invariants.
use super::*;
use crate::agents::labels::{LabelPredicate, parse_assignments, parse_selector, validate_value};
use crate::{
    agents::launch_service as launch,
    service::{CallerContext, CallerProcess, InvocationContext, RequestId},
};
fn context(db: &AgentxDb) -> RequestContext {
    RequestContext {
        caller: CallerContext {
            request_id: RequestId(uuid::Uuid::new_v4().to_string()),
            invocation: InvocationContext { cwd: db.path().parent().unwrap().to_owned(), ..Default::default() },
            deadline_unix_ms: None,
            expected_database_generation: None,
        },
        process: CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
        daemon_instance: "labels-fixture".into(),
        database_generation: db.database_generation().unwrap(),
    }
}
fn labels(pairs: &[(&str, &str)]) -> LabelSet {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}
fn reserve(db: &AgentxDb, name: &str, labels: LabelSet) -> ParticipantRef {
    let input = launch::Reserve {
        name: Some(name.into()),
        tool: "qoder".into(),
        title: None,
        labels,
        directory: db.path().parent().unwrap().to_owned(),
        process_binding: uuid::Uuid::new_v4().to_string(),
        prior_session: None,
        resume_cursor: None,
        background: true,
        launch_args: vec![],
        launch_profile: None,
        collaboration: None,
    };
    let launch::Response::Reserved(result) = crate::agents::launch_store::handle(db, &context(db), launch::Request::Reserve(input)).unwrap() else { panic!("reservation") };
    result.participant
}
fn apply(db: &AgentxDb, ctx: &RequestContext, target: &ParticipantRef, set: LabelSet, remove: Vec<String>, revision: Option<&str>) -> ServiceResult<LabelOutcome> {
    let Response::Patched(outcome) = handle(db, ctx, Request::Patch(PatchLabels { participant: target.clone(), set, remove, if_revision: revision.map(str::to_owned) }))? else { panic!("patch") };
    Ok(outcome)
}
fn stop(db: &AgentxDb, target: &ParticipantRef) {
    db.delete_instance(&target.canonical_name).unwrap();
}

#[test]
fn labels_validation_preserves_scalar_values_and_refuses_ambiguous_input() {
    for key in ["project", "review/state", "acme.review/state", "1", "a_b-c.d"] {
        validate_key(key).unwrap();
    }
    for key in ["", "A", "a/", "/a", "a/b/c", "a!", "agentx/state", "-a", "a_", "中文"] {
        assert!(validate_key(key).is_err(), "{key}");
    }
    for value in ["false", "0", "中文 内部空格", "a=b!=c", "frontend,backend", "<script>"] {
        validate_value(value).unwrap();
    }
    for value in ["", " x", "x ", "x\n", "\u{85}"] {
        assert!(validate_value(value).is_err());
    }
    assert!(validate_value(&"中".repeat(86)).is_err());
    assert!(parse_assignments(&["a=x".into(), "a=y".into()]).is_err());
    assert_eq!(parse_selector("note=a!=b=c").unwrap(), LabelPredicate::Eq { key: "note".into(), value: "a!=b=c".into() });
    assert!(parse_selector("a=").is_err());
    assert!(normalize_selectors(&vec![LabelPredicate::Exists { key: "a".into() }; 17]).is_err());
    let raw = r#"{"participant":{"canonical_name":"luna","incarnation_bits":1},"set":{"a":"x","a":"y"},"if_revision":null}"#;
    assert!(serde_json::from_str::<PatchLabels>(raw).is_err());
    for raw_value in ["null", "true", "1", "[]", "{}"] {
        let raw = format!(r#"{{"participant":{{"canonical_name":"luna","incarnation_bits":1}},"set":{{"a":{raw_value}}},"if_revision":null}}"#);
        assert!(serde_json::from_str::<PatchLabels>(&raw).is_err());
    }
}
#[test]
fn labels_patch_revision_replay_partial_keys_and_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let target = reserve(&db, "luna", labels(&[("project", "atlas"), ("feature", "search")]));
    let initial = get(&db, &context(&db), &target).unwrap();
    assert_eq!(initial.revision, "1");
    let request = context(&db);
    let patch = labels(&[("review/state", "pending")]);
    let first = apply(&db, &request, &target, patch.clone(), vec![], Some("1")).unwrap();
    assert_eq!(first.snapshot.labels["project"], "atlas");
    assert_eq!(apply(&db, &request, &target, patch, vec![], Some("1")).unwrap(), first);
    assert!(matches!(apply(&db, &request, &target, labels(&[("review/state", "approved")]), vec![], None), Err(ServiceError::Conflict(_))));
    assert!(!apply(&db, &context(&db), &target, labels(&[("project", "atlas")]), vec!["missing".into()], None).unwrap().changed);
    let second = apply(&db, &context(&db), &target, labels(&[("plugin-a/state", "x")]), vec![], None).unwrap();
    let third = apply(&db, &context(&db), &target, labels(&[("plugin-b/state", "y")]), vec![], None).unwrap();
    assert_eq!(third.snapshot.labels["plugin-a/state"], "x");
    assert!(matches!(apply(&db, &context(&db), &target, labels(&[("project", "comet")]), vec![], Some(&second.snapshot.revision)), Err(ServiceError::Conflict(_))));
    let before = get(&db, &context(&db), &target).unwrap();
    assert!(apply(&db, &context(&db), &target, labels(&[("project", "changed")]), vec!["project".into()], None).is_err());
    assert!(apply(&db, &context(&db), &target, labels(&[("project", "changed"), ("bad", " ")]), vec![], None).is_err());
    db.conn().execute_batch("CREATE TRIGGER labels_fail_receipt BEFORE INSERT ON kv WHEN NEW.key LIKE 'participant_request:%' BEGIN SELECT RAISE(ABORT,'receipt unavailable'); END;").unwrap();
    assert!(apply(&db, &context(&db), &target, labels(&[("project", "changed")]), vec![], None).is_err());
    assert_eq!(get(&db, &context(&db), &target).unwrap(), before);
    db.conn().execute_batch("DROP TRIGGER labels_fail_receipt;").unwrap();
    let removed = apply(&db, &context(&db), &target, LabelSet::new(), before.labels.keys().cloned().collect(), None).unwrap();
    assert!(removed.snapshot.labels.is_empty());
    assert_ne!(removed.snapshot.revision, "0");
    assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM agent_label_sets", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
    let event_count = db.conn().query_row("SELECT COUNT(*) FROM events WHERE type='labels_changed'", [], |r| r.get::<_, i64>(0)).unwrap();
    assert_eq!(event_count, 5); // initial + 3 changed patches + removal; replay/no-op/rollback add none.
}
#[test]
fn labels_bounds_generation_exact_target_and_legacy_read() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let target = reserve(&db, "luna", LabelSet::new());
    let full: LabelSet = (0..64).map(|i| (format!("key-{i}"), "value".into())).collect();
    apply(&db, &context(&db), &target, full, vec![], None).unwrap();
    assert!(apply(&db, &context(&db), &target, labels(&[("overflow", "x")]), vec![], None).is_err());
    let large: LabelSet = (0..64).map(|i| (format!("key-{i}"), "x".repeat(256))).collect();
    assert!(apply(&db, &context(&db), &target, large, vec![], None).is_err());
    let mut wrong = context(&db);
    wrong.caller.expected_database_generation = Some("another".into());
    assert!(matches!(handle(&db, &wrong, Request::Get(target.clone())), Err(ServiceError::StaleFence)));
    let stale = ParticipantRef { incarnation_bits: target.incarnation_bits + 1, ..target.clone() };
    assert!(matches!(get(&db, &context(&db), &stale), Err(ServiceError::StaleFence)));
    db.save_instance_named("legacy", json!({"created_at":42.25,"tool":"qoder"}).as_object().unwrap()).unwrap();
    let legacy = ParticipantRef { canonical_name: "legacy".into(), incarnation_bits: 42.25f64.to_bits() };
    let before = db.conn().query_row("SELECT COUNT(*) FROM agent_creations", [], |r| r.get::<_, i64>(0)).unwrap();
    assert_eq!(get(&db, &context(&db), &legacy).unwrap().revision, "0");
    assert_eq!(before, db.conn().query_row("SELECT COUNT(*) FROM agent_creations", [], |r| r.get::<_, i64>(0)).unwrap());
    apply(&db, &context(&db), &legacy, labels(&[("project", "legacy")]), vec![], None).unwrap();
    let id = owner(&db, &legacy).unwrap().unwrap();
    assert_eq!(db.conn().query_row("SELECT origin FROM agent_creations WHERE id=?", [id], |r| r.get::<_, String>(0)).unwrap(), "unknown");
}
#[test]
fn labels_multiple_resume_aliases_fork_namesake_and_reset() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let original = reserve(&db, "luna", labels(&[("review/state", "approved")]));
    let id = owner(&db, &original).unwrap().unwrap();
    let snapshot = get(&db, &context(&db), &original).unwrap();
    let mut current_ref = original.clone();
    for bits in [100.25f64.to_bits(), 200.5f64.to_bits(), 300.75f64.to_bits()] {
        stop(&db, &current_ref);
        db.save_instance_named("luna", json!({"created_at":f64::from_bits(bits),"tool":"qoder"}).as_object().unwrap()).unwrap();
        let resumed = ParticipantRef { canonical_name: "luna".into(), incarnation_bits: bits };
        db.with_write_scope(|| super::super::launch_relations::inherit(&db, &resumed, &current_ref)).unwrap();
        assert_eq!(owner(&db, &resumed).unwrap().as_deref(), Some(id.as_str()));
        let now = get(&db, &context(&db), &resumed).unwrap();
        assert_eq!(now.labels, snapshot.labels);
        assert_eq!(now.revision, snapshot.revision);
        current_ref = resumed;
    }
    let fork = reserve(&db, "nova", LabelSet::new());
    assert!(get(&db, &context(&db), &fork).unwrap().labels.is_empty());
    stop(&db, &current_ref);
    let successor = reserve(&db, "luna", LabelSet::new());
    assert_ne!(owner(&db, &successor).unwrap().unwrap(), id);
    assert!(get(&db, &context(&db), &successor).unwrap().labels.is_empty());
    assert!(matches!(get(&db, &context(&db), &original), Err(ServiceError::StaleFence)));
    assert!(select_current(&db, &context(&db), LabelQuery { selectors: vec![parse_selector("review/state=approved").unwrap()], limit: 100, ..Default::default() }).unwrap().records.is_empty());
    db.with_write_scope(|| db.clear_managed_state_for_reset()).unwrap();
    for table in ["agent_labels", "agent_label_sets", "agent_creation_instances", "agent_creations"] {
        assert_eq!(db.conn().query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    }
}
#[test]
fn labels_selection_before_page_missing_negative_cursor_and_budget() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    for (name, set) in [("aaaa", LabelSet::new()), ("bbbb", labels(&[("project", "comet")])), ("cccc", labels(&[("project", "atlas")])), ("dddd", labels(&[("project", "atlas"), ("note", "a=b")])), ("eeee", LabelSet::new())] {
        reserve(&db, name, set);
    }
    let query = LabelQuery { selectors: vec![parse_selector("project=atlas").unwrap()], limit: 1, ..Default::default() };
    let first = select_current(&db, &context(&db), query.clone()).unwrap();
    assert_eq!(first.records[0].name, "cccc");
    let next = first.next.unwrap();
    let second = select_current(&db, &context(&db), LabelQuery { after: Some(next.clone()), ..query.clone() }).unwrap();
    assert_eq!(second.records[0].name, "dddd");
    assert!(second.next.is_none());
    assert!(select_current(&db, &context(&db), LabelQuery { after: Some(next), selectors: vec![parse_selector("!project").unwrap()], ..query.clone() }).is_err());
    for (selector, names) in [("!project", vec!["aaaa", "eeee"]), ("project!=atlas", vec!["aaaa", "bbbb", "eeee"]), ("project", vec!["bbbb", "cccc", "dddd"])] {
        let result = select_current(&db, &context(&db), LabelQuery { selectors: vec![parse_selector(selector).unwrap()], limit: 100, ..Default::default() }).unwrap();
        assert_eq!(result.records.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), names);
    }
    assert!(select_current(&db, &context(&db), LabelQuery { after: Some("1:00000000000000000000".into()), ..query.clone() }).is_err());
    assert!(hydrate(&db, &context(&db), &vec![ParticipantRef { canonical_name: "aaaa".into(), incarnation_bits: 0 }; 101]).is_err());
    let mut expired = context(&db);
    expired.caller.deadline_unix_ms = Some(0);
    assert!(matches!(select_current(&db, &expired, query), Err(ServiceError::Unavailable(_))));
}

#[test]
fn labels_independent_connections_preserve_disjoint_concurrent_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agentx.db");
    let db = AgentxDb::open_at(&path).unwrap();
    let target = reserve(&db, "luna", labels(&[("project", "atlas")]));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let threads: Vec<_> = ["plugin-a/state", "plugin-b/state"]
        .into_iter()
        .map(|key| {
            let barrier = barrier.clone();
            let path = path.clone();
            let target = target.clone();
            std::thread::spawn(move || {
                let db = AgentxDb::open_at(&path).unwrap();
                let ctx = context(&db);
                barrier.wait();
                apply(&db, &ctx, &target, labels(&[(key, "complete")]), vec![], None).unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let snapshot = get(&db, &context(&db), &target).unwrap();
    assert_eq!(snapshot.labels, labels(&[("project", "atlas"), ("plugin-a/state", "complete"), ("plugin-b/state", "complete")]));
    assert_eq!(snapshot.revision, "3");
}
#[test]
fn labels_reservation_failure_is_atomic_and_revision_never_wraps() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    db.conn().execute_batch("CREATE TRIGGER initial_labels_fail BEFORE INSERT ON agent_labels BEGIN SELECT RAISE(ABORT,'label failure'); END;").unwrap();
    let input = launch::Reserve {
        name: Some("luna".into()),
        tool: "qoder".into(),
        title: None,
        labels: labels(&[("project", "atlas")]),
        directory: dir.path().to_owned(),
        process_binding: "atomic-reservation".into(),
        prior_session: None,
        resume_cursor: None,
        background: true,
        launch_args: vec![],
        launch_profile: None,
        collaboration: None,
    };
    assert!(crate::agents::launch_store::handle(&db, &context(&db), launch::Request::Reserve(input)).is_err());
    assert!(db.get_instance_full("luna").unwrap().is_none());
    assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM agent_creations", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    assert!(db.get_process_binding("atomic-reservation").unwrap().is_none());
    db.conn().execute_batch("DROP TRIGGER initial_labels_fail;").unwrap();
    let target = reserve(&db, "luna", labels(&[("project", "atlas")]));
    db.conn().execute("UPDATE agent_label_sets SET revision=?", [i64::MAX]).unwrap();
    assert!(matches!(apply(&db, &context(&db), &target, labels(&[("project", "comet")]), vec![], None), Err(ServiceError::Conflict(_))));
    let snapshot = get(&db, &context(&db), &target).unwrap();
    assert_eq!(snapshot.revision, i64::MAX.to_string());
    assert_eq!(snapshot.labels["project"], "atlas");
}

/// Actual native SQL and bundled SQLite, with finite synthetic current rows.
/// Prints a single machine-readable evidence record for the independent runner.
#[test]
fn labels_native_query_plans_and_bounded_timings() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    db.with_write_scope(|| {
        db.conn().execute_batch("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<19999) INSERT INTO instances(name,created_at,tool,directory,launch_context) SELECT printf('agent_%05d',x),1700000000.0+x/1000.0,'qoder','/fixture',json_object('collaboration',json_object('workspace',CASE WHEN x%10=0 THEN '/scope/small' ELSE '/scope/other' END,'role',NULL,'endpoint',json('false'))) FROM n;
        INSERT INTO agent_creations(id,name,incarnation,origin) SELECT 'owner-'||name,name,agentx_incarnation(created_at),'unknown' FROM instances;
        INSERT INTO agent_creation_instances(name,incarnation,creation) SELECT name,agentx_incarnation(created_at),'owner-'||name FROM instances;
        INSERT INTO agent_label_sets(owner_creation,revision) SELECT id,1 FROM agent_creations;
        INSERT INTO agent_labels SELECT id,'project',CASE WHEN CAST(substr(name,7) AS INTEGER)%1000=0 THEN 'sparse' ELSE 'common' END FROM agent_creations;
        INSERT INTO agent_labels SELECT id,'note','value=a!=b' FROM agent_creations;
        INSERT INTO agent_labels SELECT id,'review/state','pending' FROM agent_creations WHERE CAST(substr(name,7) AS INTEGER)%2=0;
        WITH RECURSIVE r(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM r WHERE x<50) INSERT INTO agent_creation_instances(name,incarnation,creation) SELECT c.name,agentx_incarnation(1600000000.0+r.x),'owner-'||c.name FROM agent_creations c CROSS JOIN r WHERE CAST(substr(c.name,7) AS INTEGER)<100;
        ANALYZE;")?;Ok(())
    }).unwrap();
    let ctx = context(&db);
    let mut evidence = Vec::new();
    for (name, selector, workspace, after) in [
        ("scope_first", "!absent", Some("/scope/small"), ""),
        ("selective_label_first", "project=sparse", None, ""),
        ("common_scope", "project=common", Some("/scope/small"), ""),
        ("negative_missing", "project!=sparse", None, ""),
        ("empty", "project=unknown", None, ""),
        ("deep_keyset", "!absent", None, "agent_19000"),
    ] {
        let query = LabelQuery { selectors: vec![parse_selector(selector).unwrap()], workspace: workspace.map(str::to_owned), limit: 100, ..Default::default() };
        let (sql, parameters) = crate::db::labels::selection_sql(&query, after);
        assert!(!sql.contains("COUNT(") && !sql.contains("OFFSET"));
        let plans = db.conn().prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap().query_map(rusqlite::params_from_iter(parameters.clone()), |r| r.get::<_, String>(3)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
        let start = std::time::Instant::now();
        let mut records = db.select_labeled_instances(&query, after).unwrap();
        let selection_us = start.elapsed().as_micros();
        let has_more = records.len() > 100;
        records.truncate(100);
        let participants: Vec<_> = records.iter().map(|row| ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }).collect();
        let start = std::time::Instant::now();
        let snapshots = hydrate(&db, &ctx, &participants).unwrap();
        let hydration_us = start.elapsed().as_micros();
        assert_eq!(snapshots.len(), records.len());
        let hydrated_rows = snapshots.values().map(|s| s.labels.len()).sum::<usize>();
        assert!(hydrated_rows <= 6400);
        if name == "scope_first" {
            assert!(plans.iter().any(|line| line.contains("instances_by_adopted_workspace_name")), "{plans:?}");
        }
        if name == "selective_label_first" {
            assert!(plans.iter().any(|line| line.contains("agent_labels_by_key_value_owner")), "{plans:?}");
            assert!(plans.iter().any(|line| line.contains("agent_creation_instances_by_creation")), "{plans:?}");
            assert_eq!(records.len(), 20);
        }
        if name == "empty" {
            assert!(records.is_empty());
        }
        if name == "deep_keyset" {
            assert_eq!(records[0].name, "agent_19001");
        }
        evidence.push(json!({"case":name,"sql":sql,"parameters":parameters.iter().map(|value|format!("{value:?}")).collect::<Vec<_>>(),"plan":plans,"rows":records.len(),"has_more":has_more,"selection_us":selection_us,"hydration_us":hydration_us,"hydrated_label_rows":hydrated_rows,"hydration_statements":1,"automatic_count":false}));
    }
    // Public consumer path includes normalization/cursor/exact refs and hydration.
    let start = std::time::Instant::now();
    let page = select_current(&db, &ctx, LabelQuery { selectors: vec![parse_selector("project=sparse").unwrap()], limit: 10, ..Default::default() }).unwrap();
    let native_page_us = start.elapsed().as_micros();
    assert_eq!(page.records.len(), 10);
    assert_eq!(page.snapshots.len(), 10);
    assert!(page.next.is_some());
    println!("LABELS_NATIVE_QUERY_EVIDENCE={}", json!({"sqlite":rusqlite::version(),"current_agents":20000,"historical_aliases":5000,"scope_size":2000,"label_rows":50000,"fixture_sync":"test OFF","native_page_us":native_page_us,"cases":evidence}));
}
