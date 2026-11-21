//! Daemon-owned aggregate/receipt persistence only. Domain rules stay in collaboration.
use super::AgentxDb;
use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, params};

pub(super) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS collaboration_work (
    generation TEXT NOT NULL, id TEXT NOT NULL, project TEXT NOT NULL,
    revision INTEGER NOT NULL, payload TEXT NOT NULL,
    PRIMARY KEY(generation,id)
);
CREATE INDEX IF NOT EXISTS collaboration_work_project ON collaboration_work(generation,project,id);
CREATE TABLE IF NOT EXISTS collaboration_commands (
    generation TEXT NOT NULL, command_id TEXT NOT NULL, project TEXT NOT NULL,
    work_id TEXT NOT NULL, actor TEXT NOT NULL, intent TEXT NOT NULL, outcome TEXT NOT NULL,
    PRIMARY KEY(generation,command_id)
);
CREATE INDEX IF NOT EXISTS collaboration_commands_work ON collaboration_commands(generation,work_id);
";

pub(crate) struct StoredWorkCommand {
    pub project: String,
    pub actor: String,
    pub intent: String,
    pub outcome: String,
}

fn stored_count(count: i64) -> Result<usize> {
    usize::try_from(count).context("collaboration stored count is outside the nonnegative usize range")
}

impl AgentxDb {
    pub(crate) fn collaboration_message_reference(&self, event: i64, work_id: &str) -> Result<bool> {
        Ok(self.conn().query_row("SELECT EXISTS(SELECT 1 FROM events WHERE id=? AND type='message' AND json_extract(data,'$.thread')=?)", params![event, format!("work-{work_id}")], |r| r.get(0))?)
    }
    pub(crate) fn collaboration_work(&self, generation: &str, id: &str) -> Result<Option<String>> {
        Ok(self.conn().query_row("SELECT payload FROM collaboration_work WHERE generation=? AND id=?", params![generation, id], |r| r.get(0)).optional()?)
    }
    pub(crate) fn collaboration_list(&self, generation: &str, project: &str, after: &str, limit: u32) -> Result<Vec<String>> {
        let mut stmt = self.conn().prepare("SELECT payload FROM collaboration_work WHERE generation=? AND project=? AND id>? ORDER BY id LIMIT ?")?;
        Ok(stmt.query_map(params![generation, project, after, limit], |r| r.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub(crate) fn collaboration_count(&self, generation: &str) -> Result<usize> {
        let count = self.conn().query_row("SELECT count(*) FROM collaboration_work WHERE generation=?", [generation], |r| r.get::<_, i64>(0))?;
        stored_count(count)
    }
    pub(crate) fn collaboration_save(&self, generation: &str, id: &str, project: &str, revision: u64, payload: &str) -> Result<()> {
        anyhow::ensure!(!self.conn().is_autocommit(), "work save requires outer write scope");
        let revision = i64::try_from(revision).context("collaboration revision exceeds SQLite signed INTEGER range")?;
        self.conn().execute(
            "INSERT INTO collaboration_work(generation,id,project,revision,payload) VALUES (?,?,?,?,?) ON CONFLICT(generation,id) DO UPDATE SET revision=excluded.revision,payload=excluded.payload",
            params![generation, id, project, revision, payload],
        )?;
        Ok(())
    }
    pub(crate) fn collaboration_command(&self, generation: &str, id: &str) -> Result<Option<StoredWorkCommand>> {
        Ok(self
            .conn()
            .query_row("SELECT project,actor,intent,outcome FROM collaboration_commands WHERE generation=? AND command_id=?", params![generation, id], |r| {
                Ok(StoredWorkCommand { project: r.get(0)?, actor: r.get(1)?, intent: r.get(2)?, outcome: r.get(3)? })
            })
            .optional()?)
    }
    pub(crate) fn collaboration_command_count(&self, generation: &str, work: &str) -> Result<usize> {
        let count = self.conn().query_row("SELECT count(*) FROM collaboration_commands WHERE generation=? AND work_id=?", params![generation, work], |r| r.get::<_, i64>(0))?;
        stored_count(count)
    }
    pub(crate) fn collaboration_retain(&self, generation: &str, id: &str, work: &str, command: &StoredWorkCommand) -> Result<()> {
        anyhow::ensure!(!self.conn().is_autocommit(), "receipt save requires outer write scope");
        self.conn().execute("INSERT INTO collaboration_commands(generation,command_id,project,work_id,actor,intent,outcome) VALUES (?,?,?,?,?,?,?)", params![generation, id, command.project, work, command.actor, command.intent, command.outcome])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::stored_count;

    #[test]
    fn stored_counts_reject_negative_values_without_wrapping() {
        assert_eq!(stored_count(0).unwrap(), 0);
        assert_eq!(stored_count(512).unwrap(), 512);
        assert!(stored_count(-1).unwrap_err().to_string().contains("nonnegative usize range"));
    }
}
