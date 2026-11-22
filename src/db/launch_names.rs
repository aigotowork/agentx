//! Narrow name reservation projections over the live rows and event journal.

use super::AgentxDb;
use anyhow::{Result, ensure};
use std::{collections::HashSet, sync::LazyLock};

const STOPPED: &str = "json_extract(data,'$.action')='stopped' AND COALESCE(json_extract(data,'$.placeholder'),0)!=1";
// A corrupt life row is a candidate with a false validity flag, so the
// allocator can fail closed without making the optional index unwritable.
const CANDIDATE: &str = "type='life' AND CASE WHEN json_valid(data) THEN json_extract(data,'$.action')='stopped' AND COALESCE(json_extract(data,'$.placeholder'),0)!=1 ELSE 1 END";
static SETUP: LazyLock<String> = LazyLock::new(|| format!("CREATE INDEX IF NOT EXISTS agentx_stopped_name_v1 ON events(instance,json_valid(data)) WHERE {CANDIDATE}"));
static INDEXED: LazyLock<String> = LazyLock::new(|| format!("SELECT DISTINCT instance,json_valid(data) FROM events INDEXED BY agentx_stopped_name_v1 WHERE {CANDIDATE}"));
static LEGACY: LazyLock<String> = LazyLock::new(|| format!("SELECT DISTINCT instance,1 FROM events WHERE type='life' AND {STOPPED}"));

impl AgentxDb {
    pub(crate) fn enable_launch_name_index(&self) -> Result<()> {
        self.conn.execute_batch(&SETUP)?;
        Ok(())
    }

    pub(crate) fn current_launch_names(&self) -> Result<HashSet<String>> {
        let mut stmt = self.conn.prepare_cached("SELECT name FROM instances")?;
        Ok(stmt.query_map([], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?)
    }

    pub(crate) fn stopped_launch_names(&self) -> Result<HashSet<String>> {
        let mut stmt = self.conn.prepare_cached(&INDEXED).or_else(|_| self.conn.prepare_cached(&LEGACY))?;
        let mut rows = stmt.query([])?;
        let mut names = HashSet::new();
        while let Some(row) = rows.next()? {
            ensure!(row.get::<_, bool>(1)?, "cannot establish reserved names from an invalid life event");
            names.insert(row.get(0)?);
        }
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::StatementStatus;

    #[test]
    fn launch_name_lookup_reads_only_stopped_history_and_keeps_reservations() {
        let root = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
        db.conn
            .execute_batch(
                "WITH RECURSIVE ids(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM ids WHERE i<4000)
             INSERT INTO events(timestamp,type,instance,data) SELECT '', 'life', 'luna', '{\"action\":\"created\"}' FROM ids;
             INSERT INTO events(timestamp,type,instance,data) VALUES
                ('','life','luna','{\"action\":\"stopped\"}'),
                ('','life','luna','{\"action\":\"stopped\"}'),
                ('','life','nava','{\"action\":\"stopped\",\"placeholder\":1}'),
                ('','status','telu','{\"action\":\"stopped\"}');",
            )
            .unwrap();
        db.save_instance_named("nava", serde_json::json!({"tool":"codex","created_at":1.0}).as_object().unwrap()).unwrap();
        let before = crate::agents::names::collect_taken_names(&db).unwrap();
        db.enable_launch_name_index().unwrap();
        assert_eq!(crate::agents::names::collect_taken_names(&db).unwrap(), before);
        assert_eq!(before, (HashSet::from(["nava".to_owned()]), HashSet::from(["luna".to_owned(), "nava".to_owned()])));

        let work = |sql: &str| {
            let mut stmt = db.conn.prepare(sql).unwrap();
            let names: Vec<String> = stmt.query_map([], |row| row.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
            assert_eq!(names, ["luna"]);
            stmt.get_status(StatementStatus::VmStep)
        };
        let (legacy, indexed) = (work(&LEGACY), work(&INDEXED));
        assert!(indexed < legacy / 20, "{indexed} vs {legacy} VM steps");

        // Older journals can contain malformed rows even though the current
        // event append path supplies JSON and updates the full-text index.
        db.conn.execute("UPDATE events SET data='not-json' WHERE id=1", []).unwrap();
        assert!(db.stopped_launch_names().is_err());
    }
}
