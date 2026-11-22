//! Optional selection indexes over existing subscription KV rows. Indexing
//! changes which rows are loaded, never the authoritative predicate or cursor.

use std::{collections::HashSet, sync::LazyLock};

use rusqlite::ToSql;

use super::AgentxDb;

// The legacy prefix query uses SQLite's ASCII case-insensitive LIKE. The
// equivalent deterministic expression keeps even noncanonical historical keys
// in the same index without requiring a second scan of the KV table.
const SUBS: &str = "lower(key) GLOB 'events?sub:*'";
const WATCHES: &str = "lower(key) GLOB 'events?sub:reqwatch-*'";
const THREAD_MEMBER: &str = "CASE WHEN json_valid(value) THEN json_extract(value,'$.auto_thread_member')=1 ELSE 0 END";
const THREAD_NAME: &str = "CASE WHEN json_valid(value) THEN json_extract(value,'$.thread_name') END";
const KIND: &str = "CASE WHEN key GLOB 'events_sub:*' AND json_valid(value) THEN CASE json_extract(value,'$.dispatch_kind_v1') WHEN 'message' THEN 'message' WHEN 'status' THEN 'status' WHEN 'life' THEN 'life' ELSE '' END ELSE '' END";
const TARGET: &str = "CASE WHEN json_valid(value) THEN CASE json_type(value,'$.filters.target') WHEN 'text' THEN json_extract(value,'$.filters.target') END END";
const GRACE: &str =
    "CASE WHEN json_valid(value) THEN CASE WHEN json_extract(value,'$.filters.target_tool')='antigravity' AND json_type(value,'$.idle_grace_until') IN ('integer','real') THEN CAST(json_extract(value,'$.idle_grace_until') AS REAL) END END";
const MATCHABLE: &str = "CASE WHEN json_valid(value) THEN COALESCE(json_extract(value,'$.delivery_only'),0) != 1 AND COALESCE(json_extract(value,'$.delivery_only'),'false') != 'true' ELSE 0 END";
// A JSON value cannot yield a BLOB here. Include this sentinel in cleanup
// seeks so malformed subscription rows retain the original SQL error behavior.
const OWNER: &str = "CASE WHEN json_valid(value) THEN json_extract(value,'$.caller') ELSE X'' END";
const CLEANUP: &str = "key LIKE 'events_sub:%' AND json_extract(value,'$.caller')=?1";
const ORDINARY_OWNER: &str = "COALESCE(json_extract(value,'$.delivery_only'),0)!=1";
const MEMBERSHIP_OWNER: &str = "json_extract(value,'$.auto_thread_member')=1 AND COALESCE(json_extract(value,'$.delivery_only'),0)=1";
static OWNER_SETUP: LazyLock<String> = LazyLock::new(|| format!("CREATE INDEX IF NOT EXISTS agentx_sub_owner_v1 ON kv(({OWNER})) WHERE {SUBS}"));
static ORDINARY_CLEANUP: LazyLock<OwnerCleanup> = LazyLock::new(|| OwnerCleanup::new(ORDINARY_OWNER));
static MEMBERSHIP_CLEANUP: LazyLock<OwnerCleanup> = LazyLock::new(|| OwnerCleanup::new(MEMBERSHIP_OWNER));

struct OwnerCleanup {
    indexed: String,
    legacy: String,
}

impl OwnerCleanup {
    fn new(predicate: &str) -> Self {
        Self { indexed: format!("DELETE FROM kv INDEXED BY agentx_sub_owner_v1 WHERE {SUBS} AND ({OWNER}) IN (?1,X'') AND {CLEANUP} AND {predicate}"), legacy: format!("DELETE FROM kv WHERE {CLEANUP} AND {predicate}") }
    }

    fn prepare<'a>(&self, db: &'a AgentxDb) -> rusqlite::Result<rusqlite::CachedStatement<'a>> {
        db.conn.prepare_cached(&self.indexed).or_else(|_| db.conn.prepare_cached(&self.legacy))
    }
}

pub(super) fn cleanup_owner(db: &AgentxDb, name: &str, memberships: bool) -> rusqlite::Result<usize> {
    let cleanup = if memberships { &*MEMBERSHIP_CLEANUP } else { &*ORDINARY_CLEANUP };
    cleanup.prepare(db)?.execute([name])
}

static SETUP: LazyLock<String> = LazyLock::new(|| {
    format!(
        "CREATE INDEX IF NOT EXISTS agentx_sub_kind_v1 ON kv(({KIND})) WHERE {SUBS};
     CREATE INDEX IF NOT EXISTS agentx_watch_target_v1 ON kv(({TARGET})) WHERE {WATCHES};
     CREATE INDEX IF NOT EXISTS agentx_watch_grace_v1 ON kv(({GRACE})) WHERE {WATCHES};
     CREATE INDEX IF NOT EXISTS agentx_thread_member_v1 ON kv(({THREAD_NAME}),json_extract(value,'$.created'),key) WHERE {SUBS} AND {THREAD_MEMBER};"
    )
});
static LEGACY: LazyLock<String> = LazyLock::new(|| format!("SELECT key,value FROM kv NOT INDEXED WHERE key LIKE 'events_sub:%' AND ({MATCHABLE}) ORDER BY rowid"));
static FOR_KIND: LazyLock<String> = LazyLock::new(|| format!("SELECT key,value FROM kv INDEXED BY agentx_sub_kind_v1 WHERE {SUBS} AND ({KIND}) IN (?1,'') AND ({MATCHABLE}) ORDER BY rowid"));
// Non-file events still perform the old SQL upgrade. Include those rows in
// the partial index until the evaluator has upgraded them once.
static ORDINARY: LazyLock<String> = LazyLock::new(|| {
    let [first, second] = super::subscriptions::LEGACY_COLLISION_PATTERNS.map(|pattern| pattern.replace('\'', "''"));
    format!(
        "({MATCHABLE}) AND CASE WHEN json_valid(value) THEN
        json_type(value,'$.filters.collision') IS NULL
        OR instr(COALESCE(json_extract(value,'$.sql'),''),'{first}')>0
        OR instr(COALESCE(json_extract(value,'$.sql'),''),'{second}')>0 ELSE 0 END"
    )
});
static FOR_ORDINARY_KIND: LazyLock<String> = LazyLock::new(|| format!("SELECT key,value FROM kv INDEXED BY agentx_sub_ordinary_kind_v1 WHERE {SUBS} AND ({KIND}) IN (?1,'') AND {} ORDER BY rowid", &*ORDINARY));
static ALL_WATCH: &str = "SELECT key,value FROM kv NOT INDEXED WHERE key LIKE 'events_sub:reqwatch-%' ORDER BY rowid";
// Replay already owns the eligible keys. Drive the join from that small set
// and only then sort the fetched rows into the live evaluator's row order.
static REPLAY_MATCHES: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT kv.key,kv.value FROM (SELECT value AS replay_key FROM json_each(?1)) AS restored
     CROSS JOIN kv ON kv.key=restored.replay_key
     WHERE kv.key LIKE 'events_sub:%' AND ({MATCHABLE}) ORDER BY kv.rowid"
    )
});
const REPLAY_WATCHES: &str = "SELECT kv.key,kv.value FROM json_each(?1) CROSS JOIN kv ON kv.key=json_each.value
     WHERE kv.key LIKE 'events_sub:reqwatch-%' ORDER BY kv.rowid";
static BY_TARGET: LazyLock<String> = LazyLock::new(|| watch_sql("agentx_watch_target_v1", TARGET, "= ?1"));
static BY_GRACE: LazyLock<String> = LazyLock::new(|| watch_sql("agentx_watch_grace_v1", GRACE, "<= ?1"));
static THREAD_LEGACY: LazyLock<String> = LazyLock::new(|| thread_sql("", "key LIKE 'events_sub:%'"));
static THREAD_INDEXED: LazyLock<String> = LazyLock::new(|| thread_sql("INDEXED BY agentx_thread_member_v1", SUBS));

fn thread_sql(index: &str, prefix: &str) -> String {
    format!(
        "SELECT i.name FROM kv {index} CROSS JOIN instances i ON i.name=CASE WHEN json_valid(value) THEN CASE WHEN json_type(value,'$.caller')='text' THEN json_extract(value,'$.caller') END END
             WHERE {prefix} AND {THREAD_MEMBER} AND ({THREAD_NAME})=?1 ORDER BY json_extract(value,'$.created'),key"
    )
}

pub(super) fn thread_members(db: &AgentxDb, thread: &str) -> rusqlite::Result<Vec<String>> {
    let read = |sql: &str| -> rusqlite::Result<Vec<String>> { db.conn().prepare_cached(sql)?.query_map([thread], |row| row.get(0))?.collect() };
    if db.indexed_subscriptions
        && let Ok(names) = read(&THREAD_INDEXED)
    {
        return Ok(names);
    }
    read(&THREAD_LEGACY)
}

fn watch_sql(index: &str, expr: &str, compare: &str) -> String {
    format!("SELECT key,value FROM kv INDEXED BY {index} WHERE {WATCHES} AND ({expr}) {compare} ORDER BY rowid")
}

impl AgentxDb {
    pub(crate) fn enable_subscription_indexes(&mut self) -> anyhow::Result<()> {
        self.with_write_scope(|| {
            self.conn().execute_batch(&SETUP)?;
            Ok(())
        })?;
        self.indexed_subscriptions = true;
        self.indexed_ordinary_subscriptions = self
            .with_write_scope(|| {
                self.conn().execute_batch(&format!("CREATE INDEX IF NOT EXISTS agentx_sub_ordinary_kind_v1 ON kv(({KIND})) WHERE {SUBS} AND {};", &*ORDINARY))?;
                Ok(())
            })
            .is_ok();
        if !self.indexed_ordinary_subscriptions {
            crate::log::log_warn("events", "subscription.ordinary_index_unavailable", "Keeping the event-kind selector for ordinary events");
        }
        if self.conn.execute_batch(&OWNER_SETUP).is_err() {
            crate::log::log_warn("events", "subscription.owner_index_unavailable", "Keeping legacy subscription cleanup selection");
        }
        Ok(())
    }
}

pub(super) fn matching(db: &AgentxDb, event_type: &str, replay_keys: Option<&HashSet<String>>, can_collide: bool) -> rusqlite::Result<Vec<(String, String)>> {
    if let Some(keys) = replay_keys {
        return replay(db, &REPLAY_MATCHES, keys);
    }
    if db.indexed_ordinary_subscriptions
        && !can_collide
        && !event_type.is_empty()
        && let Ok(rows) = query(db, &FOR_ORDINARY_KIND, &[&event_type])
    {
        return Ok(rows);
    }
    if db.indexed_subscriptions
        && !event_type.is_empty()
        && let Ok(rows) = query(db, &FOR_KIND, &[&event_type])
    {
        return Ok(rows);
    }
    query(db, &LEGACY, &[])
}

#[derive(Clone, Copy)]
pub(super) enum Watches<'a> {
    Target(&'a str),
    Due(f64),
}

pub(super) fn watches(db: &AgentxDb, selection: Watches<'_>, replay_keys: Option<&HashSet<String>>) -> rusqlite::Result<Vec<(String, String)>> {
    if let Some(keys) = replay_keys {
        return replay(db, REPLAY_WATCHES, keys);
    }
    if db.indexed_subscriptions {
        let rows = match selection {
            Watches::Target(target) if !target.is_empty() => query(db, &BY_TARGET, &[&target]),
            Watches::Due(now) if now.is_finite() => query(db, &BY_GRACE, &[&now]),
            _ => return query(db, ALL_WATCH, &[]),
        };
        if rows.is_ok() {
            return rows;
        }
    }
    query(db, ALL_WATCH, &[])
}

fn replay(db: &AgentxDb, sql: &str, keys: &HashSet<String>) -> rusqlite::Result<Vec<(String, String)>> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }
    // One bound JSON array avoids the SQL parameter limit on large snapshots.
    let keys = serde_json::to_string(keys).map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    query(db, sql, &[&keys])
}

fn query(db: &AgentxDb, sql: &str, args: &[&dyn ToSql]) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt = db.conn().prepare_cached(sql)?;
    stmt.query_map(args, |row| Ok((row.get(0)?, row.get(1)?)))?.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::subscriptions;
    use crate::observability::post_commit::Dispatch;
    use serde_json::json;

    #[test]
    fn launch_subscription_cleanup_seeks_owner_and_preserves_delivery_memberships() {
        let root = tempfile::tempdir().unwrap();
        let mut db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
        db.conn.execute_batch("WITH RECURSIVE ids(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM ids WHERE i<4000) INSERT INTO kv SELECT 'history:' || i, '{}' FROM ids;").unwrap();
        for (key, value) in [
            ("events_sub:ordinary", json!({"caller":"luna"})),
            ("EVENTS-SUB:legacy", json!({"caller":"luna"})),
            ("events_sub:member", json!({"caller":"luna","delivery_only":1,"auto_thread_member":1})),
            ("events_sub:pinned", json!({"caller":"luna","delivery_only":1})),
            ("events_sub:unrelated", json!({"caller":"nava"})),
        ] {
            db.kv_set(key, Some(&value.to_string())).unwrap();
        }
        let work = |db: &AgentxDb, cleanup: &OwnerCleanup, count| {
            db.conn.execute_batch("SAVEPOINT cleanup_probe").unwrap();
            let mut stmt = cleanup.prepare(db).unwrap();
            assert_eq!(stmt.execute(["luna"]).unwrap(), count);
            let steps = stmt.get_status(rusqlite::StatementStatus::VmStep);
            db.conn.execute_batch("ROLLBACK TO cleanup_probe; RELEASE cleanup_probe").unwrap();
            steps
        };
        let baseline = [work(&db, &ORDINARY_CLEANUP, 2), work(&db, &MEMBERSHIP_CLEANUP, 1)];
        db.enable_subscription_indexes().unwrap();
        let indexed = [work(&db, &ORDINARY_CLEANUP, 2), work(&db, &MEMBERSHIP_CLEANUP, 1)];
        assert!(indexed.iter().zip(baseline).all(|(actual, baseline)| *actual < baseline / 20), "{indexed:?} vs {baseline:?} VM steps");

        db.kv_set("events_sub:broken", Some("not-json")).unwrap();
        assert!(subscriptions::cleanup_subscriptions(&db, "luna").is_err());
        assert!(subscriptions::cleanup_thread_memberships_for_name_reuse(&db, "luna").is_err());
        db.kv_set("events_sub:broken", None).unwrap();
        assert_eq!(subscriptions::cleanup_subscriptions(&db, "luna").unwrap(), 2);
        assert_eq!(subscriptions::cleanup_thread_memberships_for_name_reuse(&db, "luna").unwrap(), 1);
        assert!(db.kv_get("events_sub:pinned").unwrap().is_some());
        assert!(db.kv_get("events_sub:unrelated").unwrap().is_some());
    }

    #[test]
    fn indexed_thread_members_seek_and_preserve_live_join_order() {
        let root = tempfile::tempdir().unwrap();
        let mut db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
        for name in ["lobo", "nava"] {
            db.save_instance_named(name, json!({"created_at":1.0,"tool":"codex"}).as_object().unwrap()).unwrap();
        }
        for (key, member, created, thread) in [("z", "nava", 1, "a"), ("a", "lobo", 1, "a"), ("b", "gone", 0, "a"), ("c", "lobo", 2, "a"), ("d", "lobo", 0, "b")] {
            db.kv_set(&format!("EVENTS_SUB:{key}"), Some(&json!({"thread_name":thread,"caller":member,"created":created,"auto_thread_member":true}).to_string())).unwrap();
        }
        db.kv_set("events_sub:malformed", Some("invalid-json")).unwrap();
        let expected = ["lobo", "nava"];
        assert_eq!(subscriptions::try_get_thread_members(&db, "a").unwrap(), expected);
        db.enable_subscription_indexes().unwrap();
        assert_eq!(subscriptions::try_get_thread_members(&db, "a").unwrap(), expected);
        let plan = db.conn().prepare(&format!("EXPLAIN QUERY PLAN {}", &*THREAD_INDEXED)).unwrap().query_map(["a"], |row| row.get::<_, String>(3)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
        assert!(plan.iter().any(|step| step.contains("SEARCH") && step.contains("agentx_thread_member_v1")), "{plan:?}");
        assert!(!plan.iter().any(|step| step.contains("TEMP B-TREE")), "{plan:?}");
        db.conn().execute_batch("DROP INDEX agentx_thread_member_v1").unwrap();
        assert_eq!(subscriptions::try_get_thread_members(&db, "a").unwrap(), expected);
    }

    #[test]
    fn postcommit_indexes_preserve_order_and_effects_and_skip_unrelated_work() {
        fn observed(indexed: bool) -> (Vec<String>, u32, u32) {
            let root = tempfile::tempdir().unwrap();
            let mut db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
            if indexed {
                db.enable_subscription_indexes().unwrap();
            }
            db.save_instance_named("luna", json!({"tool":"codex","created_at":1.0}).as_object().unwrap()).unwrap();
            for (kind, text) in [("status", "first"), ("message", "wrong-a"), ("status", "second"), ("life", "wrong-b")] {
                let filter = std::collections::HashMap::from([("type".to_owned(), vec![kind.to_owned()])]);
                subscriptions::create_filter_subscription(&db, &filter, &[], "luna", true, Some(&format!("@luna {text}"))).unwrap();
            }
            subscriptions::build_and_insert_sql_subscription(&db, &["type='status' AND instance='luna'".to_owned()], "luna", true, Some("@luna generic")).unwrap();
            // Invalid JSON is safe for index writes and cannot mask valid rows.
            db.kv_set("events_sub:broken", Some("invalid-json")).unwrap();
            let capture = Dispatch::begin();
            db.log_event("status", "luna", &json!({"status":"active"})).unwrap();
            let messages = db.conn().prepare("SELECT json_extract(data,'$.text') FROM events WHERE type='message' ORDER BY id").unwrap().query_map([], |row| row.get(0)).unwrap().collect::<rusqlite::Result<Vec<String>>>().unwrap();
            let work = capture.work().unwrap();
            (messages, work.subscription_candidates, work.subscription_predicates)
        }
        let (legacy, legacy_candidates, legacy_predicates) = observed(false);
        let (indexed, indexed_candidates, indexed_predicates) = observed(true);
        assert_eq!(indexed, legacy);
        assert!(indexed.iter().any(|text| text.contains("first")) && indexed.iter().any(|text| text.contains("second")) && indexed.iter().any(|text| text.contains("generic")));
        assert!(indexed_candidates < legacy_candidates && indexed_predicates < legacy_predicates, "{indexed_candidates}/{legacy_candidates} rows, {indexed_predicates}/{legacy_predicates} predicates");
    }

    #[test]
    fn ordinary_subscription_selector_skips_current_collision_rules_with_less_vm_work() {
        let root = tempfile::tempdir().unwrap();
        let mut db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
        db.enable_subscription_indexes().unwrap();

        let save = |key: &str, value: serde_json::Value| db.kv_set(&format!("events_sub:{key}"), Some(&value.to_string())).unwrap();
        save("ordinary", json!({"sql":"type='status'","dispatch_kind_v1":"status"}));
        for n in 0..77 {
            save(&format!("collision-{n}"), json!({"filters":{"collision":["true"]},"sql":"SELECT 1 WHERE +e.type = 'status'"}));
        }
        save("legacy", json!({"sql":"EXISTS (SELECT 1 FROM events_v e WHERE e.type = 'status')","filters":{"collision":["true"]}}));
        save("delivery", json!({"delivery_only":true}));

        let plan = db.conn().prepare(&format!("EXPLAIN QUERY PLAN {}", &*FOR_ORDINARY_KIND)).unwrap().query_map(["status"], |row| row.get::<_, String>(3)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
        assert!(plan.iter().any(|step| step.contains("SEARCH") && step.contains("agentx_sub_ordinary_kind_v1")), "{plan:?}");
        let run = |sql: &str| {
            let mut stmt = db.conn().prepare(sql).unwrap();
            let keys = stmt.query_map(["status"], |row| row.get::<_, String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
            (keys, stmt.get_status(rusqlite::StatementStatus::VmStep))
        };
        let (ordinary, ordinary_work) = run(&FOR_ORDINARY_KIND);
        let (all, all_work) = run(&FOR_KIND);
        assert_eq!(ordinary, ["events_sub:ordinary", "events_sub:legacy"]);
        assert_eq!(all.len(), 79);
        assert!(ordinary_work < all_work / 4, "{ordinary_work} vs {all_work} VM steps");
        let keys = |replay, can_collide| matching(&db, "status", replay, can_collide).unwrap().into_iter().map(|(key, _)| key).collect::<Vec<_>>();
        assert_eq!(keys(None, false), ordinary);
        assert_eq!(keys(None, true), all);
        let restored = all.iter().cloned().collect();
        assert_eq!(keys(Some(&restored), false), all);

        // Non-file status events still upgrade old rules, after which those
        // rules no longer enter the ordinary-event candidate set.
        db.log_status_event("nova", "active", "tool:Bash", Some("pwd"), None).unwrap();
        assert_eq!(keys(None, false), ["events_sub:ordinary"]);
        db.conn().execute_batch("DROP INDEX agentx_sub_ordinary_kind_v1").unwrap();
        assert_eq!(keys(None, false), all);
    }

    #[test]
    fn postcommit_indexes_seek_by_type_target_and_grace_and_fall_back_before_effects() {
        let root = tempfile::tempdir().unwrap();
        let mut db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
        db.enable_subscription_indexes().unwrap();
        for (sql, param, index) in [(&*FOR_KIND, &"status" as &dyn ToSql, "agentx_sub_kind_v1"), (&*BY_TARGET, &"luna" as &dyn ToSql, "agentx_watch_target_v1"), (&*BY_GRACE, &7.0 as &dyn ToSql, "agentx_watch_grace_v1")] {
            let explain = format!("EXPLAIN QUERY PLAN {sql}");
            let steps = db.conn().prepare(&explain).unwrap().query_map([param], |row| row.get::<_, String>(3)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
            assert!(steps.iter().any(|step| step.contains("SEARCH") && step.contains(index)), "{steps:?}");
        }
        for (key, target, time) in [("first", "luna", json!(6)), ("later", "luna", json!(8)), ("elsewhere", "nova", json!(5)), ("text", "luna", json!("5"))] {
            db.kv_set(&format!("events_sub:reqwatch-{key}"), Some(&json!({"filters":{"target":target,"target_tool":"antigravity"},"idle_grace_until":time}).to_string())).unwrap();
        }
        let names = |selection| watches(&db, selection, None).unwrap().into_iter().map(|(key, _)| key).collect::<Vec<_>>();
        assert_eq!(names(Watches::Target("luna")), ["events_sub:reqwatch-first", "events_sub:reqwatch-later", "events_sub:reqwatch-text"]);
        assert_eq!(names(Watches::Due(7.0)), ["events_sub:reqwatch-first", "events_sub:reqwatch-elsewhere"]);
        db.conn().execute_batch("DROP INDEX agentx_sub_kind_v1").unwrap();
        let legacy = matching(&db, "status", None, true).unwrap();
        let restored = legacy.iter().map(|(key, _)| key.clone()).collect();
        assert_eq!(legacy, matching(&db, "status", Some(&restored), true).unwrap());
    }

    #[test]
    fn replay_seeks_only_restored_keys_without_scanning_unrelated_history() {
        let root = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
        db.conn()
            .execute_batch(
                "WITH RECURSIVE history(id) AS (SELECT 1 UNION ALL SELECT id+1 FROM history WHERE id<20000)
             INSERT INTO kv(key,value) SELECT 'history:'||id,'{}' FROM history;
             INSERT INTO kv(key,value) VALUES ('events_sub:z','{}'),('events_sub:reqwatch-a','{}'),
                ('events_sub:unrelated','{}'),('events_sub:delivery','{\"delivery_only\":true}'),
                ('events_sub:invalid','oops');",
            )
            .unwrap();
        let keys = ["events_sub:reqwatch-a", "events_sub:z", "events_sub:delivery", "events_sub:invalid", "events_sub:missing"].map(str::to_string).into_iter().collect::<HashSet<_>>();
        let steps = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let measured = steps.clone();
        use std::sync::atomic::Ordering;
        db.conn()
            .progress_handler(
                50,
                Some(move || {
                    measured.fetch_add(50, Ordering::Relaxed);
                    false
                }),
            )
            .unwrap();
        let matches = matching(&db, "life", Some(&keys), false).unwrap();
        let watches = watches(&db, Watches::Target("any"), Some(&keys)).unwrap();
        assert_eq!(matches, [("events_sub:z".into(), "{}".into()), ("events_sub:reqwatch-a".into(), "{}".into())]);
        assert_eq!(watches, [("events_sub:reqwatch-a".into(), "{}".into())]);
        let steps = steps.load(Ordering::Relaxed);
        assert!((50..1_000).contains(&steps), "replay must use keyed reads for both selectors: {steps} VM steps");
    }
}
