//! Rebuildable exact conversation membership. Message contents and time bounds
//! remain in the event journal; Hook delivery eligibility is a different index.

use rusqlite::params;

use super::AgentxDb;

// The Events participant predicate includes the routing instance and actual
// delivered recipient values, not display sender, mentions or broadcast intent.
// Keep malformed legacy payloads from poisoning public event writes/backfill.
const PARTICIPANTS: &str = "
    SELECT instance,id FROM incoming
    UNION ALL
    SELECT recipient.value,i.id FROM incoming i, json_each(
        CASE WHEN json_valid(i.data) THEN
            CASE WHEN json_valid(json_extract(i.data,'$.delivered_to')) THEN json_extract(i.data,'$.delivered_to') END
        END
    ) recipient WHERE recipient.type='text'";

const BACKFILL_BATCH: i64 = 512;

impl AgentxDb {
    /// Install plain SQL triggers so every writer, including an older binary,
    /// maintains the projection in the same transaction as the event.
    pub(crate) fn start_conversation_routes(&self) -> anyhow::Result<bool> {
        self.with_write_scope(|| {
            self.conn().execute_batch(
                "CREATE TABLE IF NOT EXISTS conversation_routes (
                    participant TEXT NOT NULL, event_id INTEGER NOT NULL,
                    PRIMARY KEY(participant,event_id)
                ) WITHOUT ROWID;
                CREATE INDEX IF NOT EXISTS idx_conversation_route_event ON conversation_routes(event_id);
                CREATE TABLE IF NOT EXISTS conversation_route_coverage (
                    singleton INTEGER PRIMARY KEY CHECK(singleton=1), through_id INTEGER NOT NULL, ceiling_id INTEGER NOT NULL
                );
                INSERT OR IGNORE INTO conversation_route_coverage SELECT 1,0,COALESCE(MAX(id),0) FROM events;
                CREATE TRIGGER IF NOT EXISTS conversation_route_delete AFTER DELETE ON events WHEN old.type='message' BEGIN
                    DELETE FROM conversation_routes WHERE event_id=old.id;
                END;",
            )?;
            for (trigger, operation, condition, remove) in
                [("conversation_route_insert", "INSERT", "new.type='message'", "new.id"), ("conversation_route_update", "UPDATE OF id,type,instance,data", "old.type='message' OR new.type='message'", "old.id")]
            {
                self.conn().execute_batch(&format!(
                    "CREATE TRIGGER IF NOT EXISTS {trigger} AFTER {operation} ON events WHEN {condition} BEGIN
                        DELETE FROM conversation_routes WHERE event_id={remove};
                        INSERT OR IGNORE INTO conversation_routes(participant,event_id) SELECT * FROM (
                            WITH incoming AS (SELECT new.id AS id,new.instance AS instance,new.data AS data WHERE new.type='message')
                            {PARTICIPANTS}
                        ); END;"
                ))?;
            }
            let (through, ceiling) = self.conversation_route_coverage()?;
            Ok(through < ceiling)
        })
    }

    fn conversation_route_coverage(&self) -> rusqlite::Result<(i64, i64)> {
        self.conn().prepare_cached("SELECT through_id,ceiling_id FROM conversation_route_coverage WHERE singleton=1")?.query_row([], |row| Ok((row.get(0)?, row.get(1)?)))
    }

    pub(crate) fn conversation_routes_cover(&self, after: i64, through: i64) -> bool {
        self.conversation_route_coverage().is_ok_and(|(covered, ceiling)| covered >= ceiling || after >= ceiling || through <= covered)
    }

    /// A small idle transaction advances coverage and inserts its membership
    /// together. The reader never treats an unfinished range as empty history.
    pub(crate) fn backfill_conversation_routes(&self) -> anyhow::Result<bool> {
        let pending = self.with_event_backfill(|| {
            let (through, ceiling) = self.conversation_route_coverage()?;
            if through >= ceiling {
                return Ok(false);
            }
            let upper = self.conn().query_row("SELECT COALESCE(MAX(id),?2) FROM (SELECT id FROM events WHERE id>?1 AND id<=?2 ORDER BY id LIMIT ?3)", params![through, ceiling, BACKFILL_BATCH], |row| row.get::<_, i64>(0))?;
            self.conn().execute(
                &format!(
                    "INSERT OR IGNORE INTO conversation_routes(participant,event_id)
                    WITH incoming AS MATERIALIZED (SELECT id,instance,data FROM events NOT INDEXED WHERE id>?1 AND id<=?2 AND type='message')
                    {PARTICIPANTS}"
                ),
                params![through, upper],
            )?;
            self.conn().execute("UPDATE conversation_route_coverage SET through_id=?1 WHERE singleton=1", [upper])?;
            Ok(upper < ceiling)
        })?;
        if !pending && let Some(scope) = self.path().parent() {
            crate::log::log_in_scope(scope, "INFO", "db", "conversation.index_ready", "Exact conversation history index backfill completed", &[]);
        }
        Ok(pending)
    }
}
