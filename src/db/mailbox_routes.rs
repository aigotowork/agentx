//! Rebuildable routing keys for the bounded Hook mailbox reader. Event JSON,
//! participant cursors and receipt semantics remain authoritative.

use rusqlite::{OptionalExtension, params};

use super::AgentxDb;

const BATCH: i64 = 128;

// A broadcast occupies one row under the empty key (agent names are never
// empty). Non-string scope/from values have the legacy Rust defaults; only
// string elements of an actual mentions array can generate direct routes.
const ROUTE_SELECT: &str = "
    SELECT id, '', sender FROM incoming WHERE scope = 'broadcast'
    UNION ALL
    SELECT incoming.id, m.value, sender FROM incoming, json_each(doc, '$.mentions') m
    WHERE scope = 'mentions' AND json_type(doc, '$.mentions') = 'array' AND m.type = 'text' AND m.value != ''";

const ROUTE_FIELDS: &str = "SELECT id, doc,
    CASE json_type(doc, '$.scope') WHEN 'text' THEN json_extract(doc, '$.scope') ELSE 'broadcast' END AS scope,
    CASE json_type(doc, '$.from') WHEN 'text' THEN json_extract(doc, '$.from') ELSE '' END AS sender FROM valid";

impl AgentxDb {
    /// Add a derived table without rebuilding event history during readiness.
    /// The plain-SQL triggers also maintain it if an older daemon owns the DB.
    pub(crate) fn start_mailbox_routes(&self) -> anyhow::Result<bool> {
        let pending = self.with_write_scope(|| {
            self.conn().execute_batch(
                "CREATE TABLE IF NOT EXISTS hook_mailbox_routes (
                recipient TEXT NOT NULL, event_id INTEGER NOT NULL, sender TEXT NOT NULL,
                PRIMARY KEY(recipient,event_id)
            ) WITHOUT ROWID;
            CREATE INDEX IF NOT EXISTS idx_hook_mailbox_route_event ON hook_mailbox_routes(event_id);
            CREATE TABLE IF NOT EXISTS hook_mailbox_coverage (
                singleton INTEGER PRIMARY KEY CHECK(singleton=1), through_id INTEGER NOT NULL, ceiling_id INTEGER NOT NULL
            );
            INSERT OR IGNORE INTO hook_mailbox_coverage SELECT 1,0,COALESCE(MAX(id),0) FROM events;
            CREATE TRIGGER IF NOT EXISTS hook_mailbox_event_delete AFTER DELETE ON events WHEN old.type='message' BEGIN
                DELETE FROM hook_mailbox_routes WHERE event_id=old.id;
            END;",
            )?;
            // Inside trigger bodies SQLite supports WITH in an INSERT SELECT
            // via a subquery; keeping one route expression avoids two policies.
            for (trigger, operation, condition, preceding) in [
                ("hook_mailbox_event_insert", "INSERT", "new.type='message'", "DELETE FROM hook_mailbox_routes WHERE event_id=new.id;"),
                ("hook_mailbox_event_update", "UPDATE OF id,type,data", "old.type='message' OR new.type='message'", "DELETE FROM hook_mailbox_routes WHERE event_id=old.id;"),
            ] {
                self.conn().execute_batch(&format!(
                    "CREATE TRIGGER IF NOT EXISTS {trigger} AFTER {operation} ON events WHEN {condition} BEGIN
                    {preceding}
                    INSERT OR IGNORE INTO hook_mailbox_routes(event_id,recipient,sender) SELECT * FROM (
                        WITH valid AS (SELECT new.id AS id, CASE WHEN json_valid(new.data) THEN new.data END AS doc WHERE new.type='message'),
                        incoming AS ({ROUTE_FIELDS} WHERE doc IS NOT NULL)
                        {ROUTE_SELECT}
                    ); END;"
                ))?;
            }
            let (through, ceiling) = self.mailbox_route_coverage()?.expect("coverage initialized in the same transaction");
            Ok(through < ceiling)
        })?;
        self.hook_mailbox_routes.set(true);
        Ok(pending)
    }

    fn mailbox_route_coverage(&self) -> rusqlite::Result<Option<(i64, i64)>> {
        self.conn().query_row("SELECT through_id,ceiling_id FROM hook_mailbox_coverage WHERE singleton=1", [], |row| Ok((row.get(0)?, row.get(1)?))).optional()
    }

    /// Copy a fixed row range in one idle dispatch. Rollback retains the old
    /// coverage, so queries never mistake an interrupted copy for an empty box.
    pub(crate) fn backfill_mailbox_routes(&self) -> anyhow::Result<bool> {
        self.with_background_write_scope(|| {
            let Some((through, ceiling)) = self.mailbox_route_coverage()? else { return Ok(false) };
            if through >= ceiling {
                return Ok(false);
            }
            let upper = self.next_event_batch(through, ceiling)?;
            self.conn().execute(
                &format!(
                    "INSERT OR IGNORE INTO hook_mailbox_routes(event_id,recipient,sender)
                WITH valid AS (SELECT id,CASE WHEN json_valid(data) THEN data END AS doc FROM events WHERE id>?1 AND id<=?2 AND type='message'),
                incoming AS ({ROUTE_FIELDS} WHERE doc IS NOT NULL)
                {ROUTE_SELECT}"
                ),
                params![through, upper],
            )?;
            self.conn().execute("UPDATE hook_mailbox_coverage SET through_id=?1 WHERE singleton=1", [upper])?;
            Ok(upper < ceiling)
        })
    }

    pub(super) fn next_event_batch(&self, through: i64, ceiling: i64) -> rusqlite::Result<i64> {
        self.conn().query_row("SELECT COALESCE(MAX(id),?2) FROM (SELECT id FROM events WHERE id>?1 AND id<=?2 ORDER BY id LIMIT ?3)", params![through, ceiling, BATCH], |row| row.get(0))
    }

    pub(super) fn mailbox_routes_cover(&self, after: i64) -> bool {
        if !self.hook_mailbox_routes.get() {
            return false;
        }
        self.mailbox_route_coverage().ok().flatten().is_some_and(|(through, ceiling)| through >= ceiling || after >= ceiling)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn codex_hook_mailbox_index_covers_legacy_routes_and_commits() {
        let directory = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
        db.save_instance_named("luna", json!({"tool":"codex","created_at":1.0}).as_object().unwrap()).unwrap();
        let incoming = |data| db.insert_event_row("message", "fixture", &data, None).unwrap();
        let first = incoming(json!({"scope":"mentions","mentions":["luna","luna",10],"from":"nova"}));
        incoming(json!({"scope":"mentions","mentions":["nova"],"from":"belu"}));
        incoming(json!({"from":"luna"}));
        let broadcast = incoming(json!({"from":"belu","scope":null}));
        assert!(db.start_mailbox_routes().unwrap());
        assert!(!db.mailbox_routes_cover(0));
        let more = incoming(json!({"scope":"mentions","mentions":["luna"],"from":"rubo"}));
        let ids = |limit| db.unread_messages_up_to("luna", limit).unwrap().iter().map(|m| m.event_id.unwrap()).collect::<Vec<_>>();
        assert_eq!(ids(9), [first, broadcast, more]);
        assert!(!db.backfill_mailbox_routes().unwrap());
        assert!(db.mailbox_routes_cover(0));
        assert_eq!(ids(2), [first, broadcast]);
        assert_eq!(ids(9), ids(usize::MAX));
        assert!(
            db.with_write_scope(|| -> anyhow::Result<()> {
                incoming(json!({"from":"rollback"}));
                anyhow::bail!("abort")
            })
            .is_err()
        );
        db.conn().execute("DELETE FROM events WHERE id=?1", [first]).unwrap();
        assert_eq!(ids(9), [broadcast, more]);
        db.with_write_scope(|| db.clear_managed_state_for_reset()).unwrap();
        db.save_instance_named("luna", json!({"tool":"codex","created_at":2.0}).as_object().unwrap()).unwrap();
        let new_generation = incoming(json!({"from":"new"}));
        assert_eq!(ids(9), [new_generation]);
    }
}
