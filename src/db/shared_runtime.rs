//! Additive persistence for provider session identity, attachment fences and mail claims.
//! These records deliberately have no foreign key to the disposable live instance row.

use anyhow::Result;
use rusqlite::params;

use super::AgentxDb;

impl AgentxDb {
    /// Called only inside the successful instance-delete transaction. Explicit
    /// row removal revokes its attachments but retains cursor, identity and
    /// uncertain claims so a live proxy cannot prevent a legitimate resume.
    pub(super) fn retain_managed_instance(&self, snapshot: &serde_json::Value) -> Result<()> {
        let name = snapshot["name"].as_str().ok_or_else(|| anyhow::anyhow!("instance snapshot has no name"))?;
        let created = snapshot["created_at"].as_f64().ok_or_else(|| anyhow::anyhow!("instance snapshot has no incarnation"))?;
        let cursor = snapshot["last_event_id"].as_i64().ok_or_else(|| anyhow::anyhow!("instance snapshot has no cursor"))?;
        let bits = created.to_bits().to_string();
        self.conn().execute("UPDATE participant_sessions SET snapshot = ?, delivered_through = MAX(delivered_through, ?) WHERE participant_name = ? AND incarnation = ?", params![snapshot.to_string(), cursor, name, bits])?;
        self.conn().execute("UPDATE participant_attachments SET active = 0 WHERE participant_name = ? AND incarnation = ?", params![name, bits])?;
        Ok(())
    }
}

pub(super) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS participant_sessions (
    id INTEGER PRIMARY KEY,
    provider TEXT NOT NULL,
    scope TEXT NOT NULL,
    session_id TEXT NOT NULL,
    participant_name TEXT NOT NULL,
    incarnation TEXT NOT NULL,
    delivered_through INTEGER NOT NULL,
    snapshot TEXT NOT NULL,
    UNIQUE(provider, scope, session_id)
);
CREATE TABLE IF NOT EXISTS participant_attachments (
    binding_id TEXT PRIMARY KEY,
    lineage INTEGER NOT NULL REFERENCES participant_sessions(id),
    target TEXT NOT NULL,
    participant_name TEXT NOT NULL,
    incarnation TEXT NOT NULL,
    execution TEXT NOT NULL,
    owner TEXT NOT NULL,
    owner_pid INTEGER NOT NULL,
    owner_identity TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    delivery_mode TEXT NOT NULL DEFAULT 'hook'
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_attachment_live_session
    ON participant_attachments(lineage) WHERE active = 1;
CREATE UNIQUE INDEX IF NOT EXISTS idx_attachment_live_participant
    ON participant_attachments(participant_name) WHERE active = 1;
CREATE INDEX IF NOT EXISTS idx_attachment_owner ON participant_attachments(owner, active);
CREATE TABLE IF NOT EXISTS inbox_claims (
    batch_id TEXT PRIMARY KEY,
    lineage INTEGER NOT NULL REFERENCES participant_sessions(id),
    claim TEXT NOT NULL,
    messages TEXT NOT NULL,
    resolution TEXT NOT NULL DEFAULT 'open',
    receipt TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_inbox_claim_open
    ON inbox_claims(lineage) WHERE resolution = 'open';
";
