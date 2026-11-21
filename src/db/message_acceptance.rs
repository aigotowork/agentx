//! Durable submission outcomes. This ledger is retained with message history;
//! deleting a live participant never makes its accepted message replayable.

use anyhow::Result;
use rusqlite::{OptionalExtension, params};

use super::AgentxDb;

pub(crate) struct MessageAcceptanceRecord {
    pub actor: String,
    pub input: String,
    pub outcome: String,
}

impl AgentxDb {
    pub(crate) fn message_acceptance(&self, generation: &str, request_id: &str) -> Result<Option<MessageAcceptanceRecord>> {
        Ok(self
            .conn()
            .query_row("SELECT actor, input, outcome FROM message_acceptances WHERE generation = ? AND request_id = ?", params![generation, request_id], |row| {
                Ok(MessageAcceptanceRecord { actor: row.get(0)?, input: row.get(1)?, outcome: row.get(2)? })
            })
            .optional()?)
    }

    /// Caller owns the same write scope as message insertion.
    pub(crate) fn retain_message_acceptance(&self, generation: &str, request_id: &str, actor: &str, input: &str, outcome: &str, event_id: i64) -> Result<()> {
        anyhow::ensure!(!self.conn().is_autocommit(), "message acceptance requires a write scope");
        self.conn().execute("INSERT INTO message_acceptances(generation, request_id, actor, input, outcome, event_id) VALUES (?, ?, ?, ?, ?, ?)", params![generation, request_id, actor, input, outcome, event_id])?;
        Ok(())
    }
}

pub(super) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS message_acceptances (
    generation TEXT NOT NULL,
    request_id TEXT NOT NULL,
    actor TEXT NOT NULL,
    input TEXT NOT NULL,
    outcome TEXT NOT NULL,
    event_id INTEGER NOT NULL,
    PRIMARY KEY(generation, request_id)
);
";
