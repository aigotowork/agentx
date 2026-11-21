//! Local claim journal shared by the provider host and explicit CLI listeners.
//! A cloud claim does not expire into replay after either local sink takes it.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Actor, Scope, read_json_file};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "phase", content = "message_id", rename_all = "snake_case")]
pub(super) enum Phase {
    Reserved,
    ProviderIssuing(String),
    ListenIssuing(String),
    Confirming(String),
}

#[derive(Deserialize, Serialize)]
pub(super) struct Claim {
    pub(super) id: String,
    pub(super) phase: Phase,
}

pub(super) struct Mailbox {
    file: PathBuf,
    _lock: File,
}

impl Mailbox {
    pub(super) fn try_open(scope: &Scope, actor: &Actor) -> Result<Option<Self>> {
        // The incarnation is generated locally and never reused. A replacement Agent
        // cannot consume or overwrite its predecessor's unresolved claim journal.
        let incarnation = Uuid::parse_str(&actor.agent.incarnation).context("invalid local cloud Agent incarnation")?;
        let prefix = scope.state.join(format!("{}-{incarnation}", actor.agent.name));
        let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(prefix.with_extension("claim-lock"))?;
        match lock.try_lock() {
            Ok(()) => Ok(Some(Self { file: prefix.with_extension("claim.json"), _lock: lock })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }

    pub(super) fn read(&self) -> Result<Option<Claim>> {
        match fs::metadata(&self.file) {
            Ok(_) => Ok(Some(read_json_file(&self.file)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn begin(&self) -> Result<Claim> {
        let claim = Claim { id: Uuid::new_v4().to_string(), phase: Phase::Reserved };
        self.persist(&claim)?;
        Ok(claim)
    }

    pub(super) fn transition(&self, claim: &mut Claim, phase: Phase) -> Result<()> {
        claim.phase = phase;
        self.persist(claim)
    }

    fn persist(&self, claim: &Claim) -> Result<()> {
        let mut temp = tempfile::NamedTempFile::new_in(self.file.parent().context("claim journal has no parent")?)?;
        serde_json::to_writer(&mut temp, claim)?;
        temp.as_file().sync_all()?;
        temp.persist(&self.file)?;
        #[cfg(unix)]
        File::open(self.file.parent().context("claim journal has no parent")?)?.sync_all()?;
        Ok(())
    }

    pub(super) fn clear(&self) -> Result<()> {
        fs::remove_file(&self.file)?;
        Ok(())
    }

    pub(super) fn complete(&self, scope: &Scope, actor: &Actor, claim: &Claim) -> Result<bool> {
        match &claim.phase {
            Phase::Confirming(message_id) => {
                // Only this phase proves a local sink accepted the message. Repeat the
                // idempotent receipt after a lost response, never the handoff itself.
                scope.confirm(actor, &claim.id, message_id)?;
            }
            _ => return Ok(false),
        }
        self.clear()?;
        Ok(true)
    }
}
