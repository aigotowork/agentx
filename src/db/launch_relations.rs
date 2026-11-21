//! Additive creation provenance storage; no foreign key to live instances.
pub(super) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS agent_creations (
 id TEXT PRIMARY KEY, name TEXT NOT NULL, incarnation TEXT NOT NULL,
 origin TEXT NOT NULL CHECK(origin IN ('agent','external','unknown')),
 creator_name TEXT, creator_incarnation TEXT, creator_creation TEXT
);
CREATE INDEX IF NOT EXISTS agent_creations_downstream ON agent_creations(creator_creation,id);
CREATE TABLE IF NOT EXISTS agent_creation_instances (
 name TEXT NOT NULL, incarnation TEXT NOT NULL, creation TEXT NOT NULL,
 stopped_event INTEGER, launch_failure TEXT, PRIMARY KEY(name,incarnation)
);
";

/// Fork input is immutable and independent of the actor who launched the child.
pub(super) const FORK_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS agent_creation_forks (
 creation TEXT PRIMARY KEY NOT NULL REFERENCES agent_creations(id) ON DELETE CASCADE,
 source_name TEXT NOT NULL, source_incarnation TEXT, source_stopped_event INTEGER,
 CHECK ((source_incarnation IS NULL) != (source_stopped_event IS NULL))
);
";

impl super::AgentxDb {
    /// Stop snapshots supply the exact old incarnation; missing legacy facts are untouched.
    pub(crate) fn retain_creation_stop(&self, name: &str, event: i64, data: &serde_json::Value) -> anyhow::Result<()> {
        if data.get("action").and_then(serde_json::Value::as_str) == Some("stopped")
            && let Some(created) = data.pointer("/snapshot/created_at").and_then(serde_json::Value::as_f64)
        {
            self.conn().execute("UPDATE agent_creation_instances SET stopped_event=?1 WHERE name=?2 AND incarnation=?3", rusqlite::params![event, name, created.to_bits().to_string()])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::{AgentxDb, SCHEMA_VERSION};

    #[test]
    fn fork_provenance_migrates_v28_without_inventing_a_source() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn().execute_batch(
            "DROP TABLE agent_creation_forks;
             INSERT INTO agent_creations(id,name,incarnation,origin) VALUES('legacy','luna','1','external');
             PRAGMA user_version=28;"
        ).unwrap();
        db.ensure_schema().unwrap();
        let version = db.conn().query_row("PRAGMA user_version", [], |row| row.get::<_, i32>(0)).unwrap();
        let legacy = db.conn().query_row("SELECT origin FROM agent_creations WHERE id='legacy'", [], |row| row.get::<_, String>(0)).unwrap();
        let sources = db.conn().query_row("SELECT count(*) FROM agent_creation_forks", [], |row| row.get::<_, i64>(0)).unwrap();
        assert_eq!((version, legacy.as_str(), sources), (SCHEMA_VERSION, "external", 0));
    }
}
