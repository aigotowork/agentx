//! Durable local evidence of the control effect boundary. Only the scoped
//! uplink owns this database. A cloud retry cannot create a second effect.

use std::path::Path;

use anyhow::{Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::cloud::communication::types::AgentView;

pub(super) struct Journal(Connection);

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Entry {
    Started(Recovery),
    Finished(Outcome),
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(super) enum Recovery {
    Launch { incarnation: String, name: Option<String> },
    Stop { agent: AgentView },
    NoEffect,
}

pub(super) struct Pending {
    pub id: String,
    pub claim: String,
    pub local_generation: String,
    pub entry: Entry,
}

/// Only safe categories and the cloud participant identity leave the machine.
/// The operation record contains no credential, initial prompt or native PID.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(super) enum Outcome {
    Succeeded(AgentView),
    Failed(Failure),
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(super) enum Failure {
    CapabilityChanged,
    LaunchRejected,
    StopRejected,
    LocalGenerationChanged,
    LeaseNotCurrent,
    WorkerUnavailable,
}

impl Failure {
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::CapabilityChanged => "capability_changed",
            Self::LaunchRejected => "launch_rejected",
            Self::StopRejected => "stop_rejected",
            Self::LocalGenerationChanged => "local_generation_changed",
            Self::LeaseNotCurrent => "lease_not_current",
            Self::WorkerUnavailable => "local_worker_unavailable",
        }
    }
}

impl Journal {
    pub(super) fn open(directory: &Path) -> Result<Self> {
        let db = Connection::open(directory.join("cloud-control.sqlite3"))?;
        db.execute_batch(
            "PRAGMA synchronous = FULL;
             CREATE TABLE IF NOT EXISTS operation (
                 namespace TEXT NOT NULL,
                 id TEXT NOT NULL,
                 fingerprint TEXT NOT NULL,
                 claim TEXT NOT NULL,
                 local_generation TEXT NOT NULL,
                 recovery TEXT NOT NULL,
                 outcome TEXT,
                 reported INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (namespace, id)
             );
             CREATE INDEX IF NOT EXISTS unreported ON operation(namespace) WHERE reported = 0;
             CREATE TABLE IF NOT EXISTS latest_lease (
                 namespace TEXT PRIMARY KEY,
                 lease TEXT NOT NULL
             );",
        )?;
        Ok(Self(db))
    }

    pub(super) fn seen(&self, namespace: &str, id: &str, fingerprint: &str, claim: &str) -> Result<bool> {
        let row: Option<(String, String)> = self.0.query_row("SELECT fingerprint, claim FROM operation WHERE namespace = ?1 AND id = ?2", params![namespace, id], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
        let Some((stored_fingerprint, stored_claim)) = row else { return Ok(false) };
        ensure!(stored_fingerprint == fingerprint && stored_claim == claim, "cloud operation identity reused with a different claim or effect");
        Ok(true)
    }

    /// Commit before calling any code that could start or stop a process.
    pub(super) fn start(&self, namespace: &str, id: &str, fingerprint: &str, claim: &str, local_generation: &str, recovery: &Recovery) -> Result<()> {
        self.0.execute("INSERT INTO operation (namespace, id, fingerprint, claim, local_generation, recovery) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![namespace, id, fingerprint, claim, local_generation, serde_json::to_string(recovery)?])?;
        Ok(())
    }

    /// A terminal outcome stays final even if the upstream report is lost.
    pub(super) fn finish(&self, namespace: &str, id: &str, outcome: &Outcome) -> Result<()> {
        ensure!(
            self.0.execute("UPDATE operation SET outcome = ?3 WHERE namespace = ?1 AND id = ?2 AND (outcome IS NULL OR outcome = ?3)", params![namespace, id, serde_json::to_string(outcome)?])? == 1,
            "cloud operation journal outcome conflicts or its entry is missing"
        );
        Ok(())
    }

    pub(super) fn pending(&self, namespace: &str) -> Result<Option<Pending>> {
        let row = self
            .0
            .query_row("SELECT id, claim, local_generation, recovery, outcome FROM operation WHERE namespace = ?1 AND reported = 0 ORDER BY rowid LIMIT 1", params![namespace], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get::<_, String>(3)?, row.get::<_, Option<String>>(4)?))
            })
            .optional()?;
        row.map(|(id, claim, local_generation, recovery, outcome)| {
            let entry = match outcome {
                Some(outcome) => Entry::Finished(serde_json::from_str(&outcome)?),
                None => Entry::Started(serde_json::from_str(&recovery)?),
            };
            Ok(Pending { id, claim, local_generation, entry })
        })
        .transpose()
    }

    pub(super) fn mark_reported(&self, namespace: &str, id: &str) -> Result<()> {
        self.0.execute("UPDATE operation SET reported = 1 WHERE namespace = ?1 AND id = ?2 AND outcome IS NOT NULL", params![namespace, id])?;
        Ok(())
    }

    pub(super) fn replace_lease(&self, namespace: &str, lease: &str) -> Result<Option<String>> {
        let tx = self.0.unchecked_transaction()?;
        let previous = tx.query_row("SELECT lease FROM latest_lease WHERE namespace = ?1", params![namespace], |row| row.get(0)).optional()?;
        tx.execute("INSERT INTO latest_lease VALUES (?1, ?2) ON CONFLICT(namespace) DO UPDATE SET lease = excluded.lease", params![namespace, lease])?;
        tx.commit()?;
        Ok(previous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_journal_reopens_started_and_terminal_entries() {
        let temp = tempfile::tempdir().unwrap();
        let journal = Journal::open(temp.path()).unwrap();
        let recovery = Recovery::Launch { incarnation: "exact-launch".into(), name: Some("agent".into()) };
        journal.start("scope-one", "op", "hash-one", "claim-one", "native-era", &recovery).unwrap();
        assert!(journal.start("scope-one", "op", "hash-one", "claim-one", "native-era", &recovery).is_err());
        assert_eq!(journal.replace_lease("scope-one", "old-lease").unwrap(), None);
        drop(journal);

        let journal = Journal::open(temp.path()).unwrap();
        let pending = journal.pending("scope-one").unwrap().unwrap();
        assert_eq!(pending.local_generation, "native-era");
        assert_eq!(pending.claim, "claim-one");
        assert_eq!(pending.entry, Entry::Started(recovery));
        assert!(journal.seen("scope-one", "op", "other-hash", "claim-one").is_err());
        assert!(journal.seen("scope-one", "op", "hash-one", "other-claim").is_err());
        assert!(!journal.seen("scope-two", "op", "hash-one", "claim-one").unwrap());
        assert_eq!(journal.replace_lease("scope-one", "new-lease").unwrap().as_deref(), Some("old-lease"));
        journal.finish("scope-one", "op", &Outcome::Failed(Failure::LaunchRejected)).unwrap();
        journal.finish("scope-one", "op", &Outcome::Failed(Failure::LaunchRejected)).unwrap();
        assert!(journal.finish("scope-one", "op", &Outcome::Unknown).is_err());
        drop(journal);

        let journal = Journal::open(temp.path()).unwrap();
        assert_eq!(journal.pending("scope-one").unwrap().unwrap().entry, Entry::Finished(Outcome::Failed(Failure::LaunchRejected)));
        journal.mark_reported("scope-one", "op").unwrap();
        assert!(journal.pending("scope-one").unwrap().is_none());
        assert!(journal.seen("scope-one", "op", "hash-one", "claim-one").unwrap());
    }
}
