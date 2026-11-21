//! SQLite identity attachment transactions. Session lineage outlives live rows;
//! binding tokens fence callbacks and OS process identity fences owner takeover.

use rusqlite::{OptionalExtension, params};

use super::attachments::*;
use crate::db::AgentxDb;

pub(crate) struct SqliteAttachmentStore<'a> {
    db: &'a AgentxDb,
    owner_process: AttachmentOwnerProcess,
}

struct Session {
    id: i64,
    participant: ParticipantRef,
    cursor: i64,
    snapshot: String,
}

struct StoredAttachment {
    attachment: Attachment,
    pid: u32,
    process_identity: Option<String>,
}

fn unavailable(error: impl std::fmt::Display) -> AttachmentFailure {
    crate::log::log_error("attachments", "store", &error.to_string());
    AttachmentFailure::StoreUnavailable
}

impl<'a> SqliteAttachmentStore<'a> {
    pub(crate) fn new(db: &'a AgentxDb) -> Self {
        let pid = std::process::id();
        Self::for_owner_process(db, AttachmentOwnerProcess { pid, process_identity: crate::sys::process::identity(pid) })
    }

    /// Daemon attachment writes use the authenticated persistent host process.
    pub(crate) fn for_owner_process(db: &'a AgentxDb, owner_process: AttachmentOwnerProcess) -> Self {
        Self { db, owner_process }
    }

    /// Distinguish legacy participants from managed but currently retired roots.
    /// A retired managed root must never bypass its claims through legacy output.
    pub(crate) fn for_delivery(&self, name: &str) -> Result<Option<Attachment>, AttachmentFailure> {
        let participant = self.participant(name)?;
        if let Some(attachment) = self.current_for_participant(&participant)? {
            return Ok(Some(attachment));
        }
        let managed: bool =
            self.db.conn().query_row("SELECT EXISTS(SELECT 1 FROM participant_sessions WHERE participant_name = ? AND incarnation = ?)", params![name, participant.incarnation_bits.to_string()], |row| row.get(0)).map_err(unavailable)?;
        if managed {
            return Err(AttachmentFailure::StaleExecution);
        }
        Ok(None)
    }

    fn transaction<T>(&self, f: impl FnOnce() -> Result<T, AttachmentFailure>) -> Result<T, AttachmentFailure> {
        self.db.with_write_scope(|| f().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<AttachmentFailure>().cloned().unwrap_or_else(|| unavailable(error)))
    }

    fn session(&self, key: &ProviderSessionKey) -> Result<Option<Session>, AttachmentFailure> {
        let row: Option<(i64, String, String, i64, String)> = self
            .db
            .conn()
            .query_row(
                "SELECT id, participant_name, incarnation, delivered_through, snapshot
             FROM participant_sessions WHERE provider = ? AND scope = ? AND session_id = ?",
                params![key.provider, key.scope, key.session_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(unavailable)?;
        row.map(|(id, name, bits, cursor, snapshot)| Ok(Session { id, participant: ParticipantRef { canonical_name: name, incarnation_bits: bits.parse().map_err(unavailable)? }, cursor, snapshot })).transpose()
    }

    fn stored(&self, clause: &str, values: &[&dyn rusqlite::ToSql]) -> Result<Vec<StoredAttachment>, AttachmentFailure> {
        let sql = format!(
            "SELECT a.target, a.owner, a.owner_pid, a.owner_identity, s.provider, s.scope, s.session_id
             FROM participant_attachments a JOIN participant_sessions s ON s.id = a.lineage WHERE {clause}"
        );
        let mut statement = self.db.conn().prepare(&sql).map_err(unavailable)?;
        let rows = statement
            .query_map(values, |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, u32>(2)?, row.get::<_, Option<String>>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?, row.get::<_, String>(6)?)))
            .map_err(unavailable)?;
        rows.map(|row| {
            let (target, owner, pid, process_identity, provider, scope, session_id) = row.map_err(unavailable)?;
            Ok(StoredAttachment { attachment: Attachment { target: serde_json::from_str(&target).map_err(unavailable)?, owner: AttachmentOwnerId(owner), session: ProviderSessionKey { provider, scope, session_id } }, pid, process_identity })
        })
        .collect()
    }

    /// Positive OS evidence only: a missing/reused process cannot later issue
    /// effects. Missing observation or elapsed heartbeat is not such evidence.
    fn owner_ended(stored: &StoredAttachment) -> bool {
        if crate::sys::process::has_exited(stored.pid) {
            return true;
        }
        match (&stored.process_identity, crate::sys::process::identity(stored.pid)) {
            (Some(expected), Some(current)) if !expected.is_empty() && !current.is_empty() => expected != &current,
            _ => false,
        }
    }

    fn save_snapshot(&self, lineage: i64, participant: &ParticipantRef) -> Result<(), AttachmentFailure> {
        match self.participant(&participant.canonical_name) {
            Ok(current) if current == *participant => {}
            Ok(_) | Err(AttachmentFailure::StaleParticipant) => return Ok(()),
            Err(error) => return Err(error),
        }
        let snapshot = self.db.get_instance(&participant.canonical_name).map_err(unavailable)?.ok_or(AttachmentFailure::StaleParticipant)?;
        let cursor = snapshot["last_event_id"].as_i64().ok_or(AttachmentFailure::StoreUnavailable)?;
        self.db.conn().execute("UPDATE participant_sessions SET snapshot = ?, delivered_through = MAX(delivered_through, ?) WHERE id = ?", params![snapshot.to_string(), cursor, lineage]).map_err(unavailable)?;
        Ok(())
    }

    fn retire_inner(&self, target: &ActivationTarget) -> Result<(), AttachmentFailure> {
        let lineage: Option<i64> = self
            .db
            .conn()
            .query_row("SELECT lineage FROM participant_attachments WHERE binding_id = ? AND target = ?", params![target.binding.0, serde_json::to_string(target).map_err(unavailable)?], |row| row.get(0))
            .optional()
            .map_err(unavailable)?;
        if let Some(lineage) = lineage {
            self.save_snapshot(lineage, &target.participant)?;
            self.db.conn().execute("UPDATE participant_attachments SET active = 0 WHERE binding_id = ?", [&target.binding.0]).map_err(unavailable)?;
        }
        Ok(())
    }

    fn restore(&self, session: &Session) -> Result<ParticipantRef, AttachmentFailure> {
        match self.db.get_instance_full(&session.participant.canonical_name).map_err(unavailable)? {
            Some(row) if row.created_at.to_bits() == session.participant.incarnation_bits => return Ok(session.participant.clone()),
            Some(_) => return Err(AttachmentFailure::StaleParticipant),
            None => {}
        }
        let mut snapshot: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&session.snapshot).map_err(unavailable)?;
        AgentxDb::project_instance_snapshot(&mut snapshot);
        let old_created = f64::from_bits(session.participant.incarnation_bits);
        let now = crate::shared::time::now_epoch_f64();
        let created = if now.to_bits() == old_created.to_bits() { f64::from_bits(now.to_bits() + 1) } else { now };
        snapshot.insert("created_at".into(), serde_json::json!(created));
        snapshot.insert("last_event_id".into(), serde_json::json!(session.cursor));
        snapshot.insert("session_id".into(), serde_json::Value::Null);
        snapshot.insert("pid".into(), serde_json::Value::Null);
        snapshot.insert("launch_context".into(), serde_json::json!(""));
        snapshot.insert("status".into(), serde_json::json!("inactive"));
        snapshot.insert("status_context".into(), serde_json::json!("resume"));
        snapshot.insert("name_announced".into(), serde_json::json!(0));
        self.db.save_instance_named(&session.participant.canonical_name, &snapshot).map_err(unavailable)?;
        let participant = self.participant(&session.participant.canonical_name)?;
        super::launch_relations::inherit(self.db, &participant, &session.participant).map_err(unavailable)?;
        self.db
            .conn()
            .execute("UPDATE participant_sessions SET incarnation = ?, snapshot = ? WHERE id = ?", params![participant.incarnation_bits.to_string(), serde_json::to_string(&snapshot).map_err(unavailable)?, session.id])
            .map_err(unavailable)?;
        Ok(participant)
    }

    fn attach_inner(&self, request: AttachmentRequest) -> Result<Attachment, AttachmentFailure> {
        if self.owner_process.pid == 0 {
            return Err(AttachmentFailure::StaleExecution);
        }
        if request.execution.0.is_empty() || request.owner.0.is_empty() {
            return Err(AttachmentFailure::StaleExecution);
        }
        if request.session.provider.is_empty() || request.session.scope.is_empty() || request.session.session_id.is_empty() {
            return Err(AttachmentFailure::UnqualifiedSession);
        }
        let existing = self.session(&request.session)?;
        let participant = match request.intent {
            AttachmentIntent::Create { provisional } => {
                if existing.is_some() {
                    return Err(AttachmentFailure::UnqualifiedSession);
                }
                provisional
            }
            AttachmentIntent::Fork { parent, provisional } => {
                if parent == request.session || self.session(&parent)?.is_none() || existing.is_some() {
                    return Err(AttachmentFailure::UnqualifiedSession);
                }
                provisional
            }
            AttachmentIntent::Resume { provisional } => match existing.as_ref() {
                Some(session) => self.restore(session)?,
                None => provisional.ok_or(AttachmentFailure::UnqualifiedSession)?,
            },
        };
        if self.participant(&participant.canonical_name)? != participant {
            return Err(AttachmentFailure::StaleParticipant);
        }
        if existing.is_none() {
            let associated: bool = self
                .db
                .conn()
                .query_row("SELECT EXISTS(SELECT 1 FROM participant_sessions WHERE participant_name = ? AND incarnation = ?)", params![participant.canonical_name, participant.incarnation_bits.to_string()], |row| row.get(0))
                .map_err(unavailable)?;
            if associated {
                return Err(AttachmentFailure::UnqualifiedSession);
            }
        }

        // Serialize session and participant ownership with binding replacement.
        // Sharing an execution id never authorizes a second live controller.
        let candidates =
            self.stored("a.active = 1 AND (a.participant_name = ? OR (s.provider = ? AND s.scope = ? AND s.session_id = ?))", &[&participant.canonical_name, &request.session.provider, &request.session.scope, &request.session.session_id])?;
        for stored in candidates {
            let same_process =
                stored.attachment.owner == request.owner && stored.pid == self.owner_process.pid && stored.process_identity.as_deref().is_some_and(|identity| !identity.is_empty() && Some(identity) == self.owner_process.process_identity.as_deref());
            if !same_process && !Self::owner_ended(&stored) {
                return Err(AttachmentFailure::OwnedByLiveController);
            }
            if stored.attachment.session != request.session && stored.attachment.target.participant == participant {
                return Err(AttachmentFailure::UnqualifiedSession);
            }
            self.retire_inner(&stored.attachment.target)?;
        }

        let lineage = match existing {
            Some(session) => session.id,
            None => {
                let snapshot = self.db.get_instance(&participant.canonical_name).map_err(unavailable)?.ok_or(AttachmentFailure::StaleParticipant)?;
                let cursor = snapshot["last_event_id"].as_i64().ok_or(AttachmentFailure::StoreUnavailable)?;
                self.db
                    .conn()
                    .execute(
                        "INSERT INTO participant_sessions(provider, scope, session_id, participant_name, incarnation, delivered_through, snapshot) VALUES (?, ?, ?, ?, ?, ?, ?)",
                        params![request.session.provider, request.session.scope, request.session.session_id, participant.canonical_name, participant.incarnation_bits.to_string(), cursor, snapshot.to_string()],
                    )
                    .map_err(unavailable)?;
                self.db.conn().last_insert_rowid()
            }
        };
        let target = ActivationTarget { participant, execution: request.execution, binding: BindingId(uuid::Uuid::new_v4().to_string()) };
        let pid = self.owner_process.pid;
        self.db
            .conn()
            .execute(
                "INSERT INTO participant_attachments(binding_id, lineage, target, participant_name, incarnation, execution, owner, owner_pid, owner_identity) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    target.binding.0,
                    lineage,
                    serde_json::to_string(&target).map_err(unavailable)?,
                    target.participant.canonical_name,
                    target.participant.incarnation_bits.to_string(),
                    target.execution.0,
                    request.owner.0,
                    pid,
                    self.owner_process.process_identity
                ],
            )
            .map_err(unavailable)?;
        Ok(Attachment { target, owner: request.owner, session: request.session })
    }
}

impl AttachmentStore for SqliteAttachmentStore<'_> {
    fn participant(&self, name: &str) -> Result<ParticipantRef, AttachmentFailure> {
        let row = self.db.get_instance_full(name).map_err(unavailable)?.ok_or(AttachmentFailure::StaleParticipant)?;
        Ok(ParticipantRef { canonical_name: row.name, incarnation_bits: row.created_at.to_bits() })
    }

    fn canonical_participant(&self, session: &ProviderSessionKey) -> Result<Option<ParticipantRef>, AttachmentFailure> {
        self.transaction(|| self.session(session)?.map(|session| self.restore(&session)).transpose())
    }

    fn attach(&mut self, request: AttachmentRequest) -> Result<Attachment, AttachmentFailure> {
        self.transaction(|| self.attach_inner(request))
    }

    fn lookup(&self, execution: &ExecutionId, session: &ProviderSessionKey) -> Result<Option<Attachment>, AttachmentFailure> {
        let mut matches = self.stored("a.active = 1 AND s.provider = ? AND s.scope = ? AND s.session_id = ?", &[&session.provider, &session.scope, &session.session_id])?;
        let Some(stored) = matches.pop() else {
            return Ok(None);
        };
        if stored.attachment.target.execution != *execution {
            return Err(AttachmentFailure::StaleExecution);
        }
        if self.participant(&stored.attachment.target.participant.canonical_name)? != stored.attachment.target.participant {
            return Err(AttachmentFailure::StaleParticipant);
        }
        if Self::owner_ended(&stored) {
            return Ok(None);
        }
        Ok(Some(stored.attachment))
    }

    fn current_for_participant(&self, participant: &ParticipantRef) -> Result<Option<Attachment>, AttachmentFailure> {
        if self.participant(&participant.canonical_name)? != *participant {
            return Err(AttachmentFailure::StaleParticipant);
        }
        let matches = self.stored("a.active = 1 AND a.participant_name = ? AND a.incarnation = ?", &[&participant.canonical_name, &participant.incarnation_bits.to_string()])?;
        Ok(matches.into_iter().find(|stored| !Self::owner_ended(stored)).map(|stored| stored.attachment))
    }

    fn owned(&self, owner: &AttachmentOwnerId) -> Result<Vec<Attachment>, AttachmentFailure> {
        Ok(self.stored("a.active = 1 AND a.owner = ?", &[&owner.0])?.into_iter().map(|stored| stored.attachment).collect())
    }

    fn validate(&self, target: &ActivationTarget) -> Result<bool, AttachmentFailure> {
        match self.current_for_participant(&target.participant) {
            Ok(Some(attachment)) => Ok(attachment.target == *target),
            Ok(None) | Err(AttachmentFailure::StaleParticipant) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn retire(&mut self, target: &ActivationTarget) -> Result<(), AttachmentFailure> {
        self.transaction(|| self.retire_inner(target))
    }

    fn retire_owner(&mut self, owner: &AttachmentOwnerId) -> Result<(), AttachmentFailure> {
        self.transaction(|| {
            for attachment in self.owned(owner)? {
                self.retire_inner(&attachment.target)?;
            }
            Ok(())
        })
    }
}
