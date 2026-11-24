//! Derived human recipient membership. Events remain the source of truth;
//! handling receipts and Agent mail routing are separate from this index.

use rusqlite::params;

use super::AgentxDb;

// Match the historical json_each(json_extract(...)) predicate, including
// object and encoded JSON values. Invalid address documents cannot poison
// event append or backfill. CASE guards SQLite's JSON functions explicitly.
pub(crate) const HUMAN_ADDRESSED: &str = "e.type='message' AND CASE WHEN json_valid(e.data) THEN
    CASE WHEN json_extract(e.data,'$.scope')='mentions' AND json_valid(json_extract(e.data,'$.mentions')) THEN
        EXISTS (SELECT 1 FROM json_each(json_extract(e.data,'$.mentions')) WHERE value='bigboss')
    ELSE 0 END ELSE 0 END";

impl AgentxDb {
    pub(crate) fn start_human_mailbox_index(&self) -> anyhow::Result<bool> {
        self.with_write_scope(|| {
            self.conn().execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS human_mailbox_events (event_id INTEGER PRIMARY KEY);
                 CREATE TABLE IF NOT EXISTS human_mailbox_coverage (
                     singleton INTEGER PRIMARY KEY CHECK(singleton=1), through_id INTEGER NOT NULL, ceiling_id INTEGER NOT NULL
                 );
                 INSERT OR IGNORE INTO human_mailbox_coverage SELECT 1,0,COALESCE(MAX(id),0) FROM events;
                 CREATE TRIGGER IF NOT EXISTS human_mailbox_insert AFTER INSERT ON events WHEN new.type='message' BEGIN
                     INSERT OR IGNORE INTO human_mailbox_events SELECT e.id FROM events e WHERE e.id=new.id AND {HUMAN_ADDRESSED};
                 END;
                 CREATE TRIGGER IF NOT EXISTS human_mailbox_update AFTER UPDATE OF id,type,data ON events WHEN old.type='message' OR new.type='message' BEGIN
                     DELETE FROM human_mailbox_events WHERE event_id=old.id;
                     INSERT OR IGNORE INTO human_mailbox_events SELECT e.id FROM events e WHERE e.id=new.id AND {HUMAN_ADDRESSED};
                 END;
                 CREATE TRIGGER IF NOT EXISTS human_mailbox_delete AFTER DELETE ON events WHEN old.type='message' BEGIN
                     DELETE FROM human_mailbox_events WHERE event_id=old.id;
                 END;"
            ))?;
            let (through, ceiling) = self.human_mailbox_coverage()?;
            Ok(through < ceiling)
        })
    }

    pub(crate) fn backfill_human_mailbox(&self) -> anyhow::Result<bool> {
        self.with_background_write_scope(|| {
            let (through, ceiling) = self.human_mailbox_coverage()?;
            if through >= ceiling {
                return Ok(false);
            }
            let upper = self.next_event_batch(through, ceiling)?;
            self.conn().execute(&format!("INSERT OR IGNORE INTO human_mailbox_events SELECT e.id FROM events e NOT INDEXED WHERE e.id>?1 AND e.id<=?2 AND {HUMAN_ADDRESSED}"), params![through, upper])?;
            self.conn().execute("UPDATE human_mailbox_coverage SET through_id=?1 WHERE singleton=1", [upper])?;
            Ok(upper < ceiling)
        })
    }

    fn human_mailbox_coverage(&self) -> rusqlite::Result<(i64, i64)> {
        self.conn().prepare_cached("SELECT through_id,ceiling_id FROM human_mailbox_coverage WHERE singleton=1")?.query_row([], |row| Ok((row.get(0)?, row.get(1)?)))
    }

    pub(crate) fn human_mailbox_covered(&self) -> bool {
        self.human_mailbox_coverage().is_ok_and(|(through, ceiling)| through >= ceiling)
    }
}
