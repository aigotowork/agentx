//! Durable mailbox transactions. External effects occur between issuing and
//! confirmation, never inside a SQLite lock; uncertain effects remain claimed.

use std::num::NonZeroUsize;

use rusqlite::{OptionalExtension, params};

use super::activation::AttemptId;
use super::inbox::*;
use crate::agents::attachments::{ActivationTarget, AttachmentStore, ParticipantRef, SqliteAttachmentStore};
use crate::db::{AgentxDb, Message};

pub(crate) struct SqlitePendingInbox<'a> {
    db: &'a AgentxDb,
    hints: String,
}

/// Prospective output token; callers create evidence only after output flush.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutputClaim {
    pub target: ActivationTarget,
    pub batch: BatchId,
    pub through: EventId,
}

pub(crate) enum OutputPreparation {
    Unmanaged,
    Deferred,
    Claimed {
        claim: OutputClaim,
        #[cfg(test)]
        formatted_context: String,
        messages: Vec<Message>,
    },
}

struct StoredClaim {
    lineage: i64,
    claim: OutstandingClaim,
    resolution: String,
    receipt: Option<DeliveryReceipt>,
}

fn unavailable(error: impl std::fmt::Display) -> InboxFailure {
    crate::log::log_error("inbox", "store", &error.to_string());
    InboxFailure::Unavailable
}

fn observe_claim_phase(db: &AgentxDb, target: &ActivationTarget, batch: &BatchId, phase: ClaimPhase) {
    if let Some(active) = crate::service::context::active_observation() {
        use crate::observability::events::{DomainCommitFact, InboxClaimRef};
        db.observe_domain(DomainCommitFact::InboxPhaseStored { claim: InboxClaimRef { scope: active.epoch.daemon.scope, database_generation: active.epoch.database_generation, target: target.clone(), batch: batch.clone() }, phase });
    }
}

impl<'a> SqlitePendingInbox<'a> {
    #[cfg(test)]
    pub(crate) fn new(db: &'a AgentxDb) -> Self {
        Self::with_hints(db, String::new())
    }

    /// Effective per-invocation configuration; never read daemon environment.
    pub(crate) fn with_hints(db: &'a AgentxDb, hints: String) -> Self {
        Self { db, hints }
    }

    /// Format against the same database snapshot as the frozen message window.
    /// Only the tip actually emitted is marked, on the caller's connection.
    pub(crate) fn format_output(&self, participant: &ParticipantRef, messages: &[Message]) -> Result<String, InboxFailure> {
        let mut records = std::collections::HashMap::new();
        for name in std::iter::once(participant.canonical_name.as_str()).chain(messages.iter().map(|message| message.from.as_str())) {
            if !records.contains_key(name) {
                records.insert(name.to_owned(), self.db.get_instance(name).map_err(unavailable)?);
            }
        }
        let mut seen = std::collections::HashSet::new();
        for message in messages {
            let keys = message.thread.as_ref().map(|thread| format!("recv:thread:{thread}")).into_iter().chain(message.intent.as_ref().map(|intent| format!("recv:intent:{intent}")));
            for tip in keys {
                let key = format!("tip:{}:{tip}", participant.canonical_name);
                if self.db.kv_get(&key).map_err(unavailable)?.is_some() {
                    seen.insert(key);
                }
            }
        }
        let marked = std::cell::RefCell::new(Vec::new());
        let checker = |name: &str, tip: &str| {
            let key = format!("tip:{name}:{tip}");
            let was_seen = seen.contains(&key);
            let marked = &marked;
            let mark: Box<dyn Fn() + '_> = Box::new(move || marked.borrow_mut().push(key.clone()));
            (was_seen, mark)
        };
        let values: Vec<_> = messages.iter().map(super::formatting::message_to_value).collect();
        let output = super::routing::format_messages_json(&values, &participant.canonical_name, &|name| records.get(name).cloned().flatten(), &|| self.hints.clone(), Some(&checker));
        for key in marked.into_inner() {
            self.db.kv_set(&key, Some("1")).map_err(unavailable)?;
        }
        Ok(output)
    }

    /// Shared hook/listen preparation. Managed-but-retired and storage failures
    /// propagate; contention or a different autonomous medium merely defer output.
    pub(crate) fn prepare_output(&mut self, name: &str, consumer: InboxConsumer) -> Result<OutputPreparation, InboxFailure> {
        let attachment = match SqliteAttachmentStore::new(self.db).for_delivery(name) {
            Ok(Some(attachment)) => attachment,
            Ok(None) => return Ok(OutputPreparation::Unmanaged),
            Err(crate::agents::attachments::AttachmentFailure::StaleParticipant | crate::agents::attachments::AttachmentFailure::StaleExecution) => return Err(InboxFailure::StaleParticipant),
            Err(error) => return Err(unavailable(error)),
        };
        let Some(snapshot) = self.snapshot(&attachment.target.participant)? else {
            return Ok(OutputPreparation::Deferred);
        };
        let attempt = AttemptId(uuid::Uuid::new_v4().to_string());
        let batch = match self.prepare(&attachment.target, &attempt, &snapshot, consumer) {
            Ok(batch) => batch,
            Err(PrepareFailure::Store(error)) => return Err(error),
            Err(_) => return Ok(OutputPreparation::Deferred),
        };
        let messages = self.frozen_messages(&batch.id)?;
        Ok(OutputPreparation::Claimed {
            claim: OutputClaim { target: attachment.target, batch: batch.id, through: batch.window.through },
            #[cfg(test)]
            formatted_context: batch.formatted_context,
            messages,
        })
    }

    fn transaction<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<T, E>
    where
        E: std::error::Error + Send + Sync + Clone + From<InboxFailure> + 'static,
    {
        self.db.with_write_scope(|| f().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<E>().cloned().unwrap_or_else(|| unavailable(error).into()))
    }

    fn validate_participant(&self, participant: &ParticipantRef) -> Result<(), InboxFailure> {
        match SqliteAttachmentStore::new(self.db).participant(&participant.canonical_name) {
            Ok(current) if current == *participant => Ok(()),
            Ok(_) | Err(crate::agents::attachments::AttachmentFailure::StaleParticipant) => Err(InboxFailure::StaleParticipant),
            Err(error) => Err(unavailable(error)),
        }
    }

    fn validate(&self, target: &ActivationTarget) -> Result<(), InboxFailure> {
        match SqliteAttachmentStore::new(self.db).validate(target) {
            Ok(true) => Ok(()),
            Ok(false) => Err(InboxFailure::StaleParticipant),
            Err(error) => Err(unavailable(error)),
        }
    }

    fn lineage(&self, target: &ActivationTarget) -> Result<i64, InboxFailure> {
        self.validate(target)?;
        self.db.conn().query_row("SELECT lineage FROM participant_attachments WHERE binding_id = ? AND active = 1", [&target.binding.0], |row| row.get(0)).map_err(unavailable)
    }

    fn read_batch(&self, participant: &ParticipantRef) -> Result<Option<(PendingSnapshot, Vec<Message>)>, InboxFailure> {
        self.validate_participant(participant)?;
        let row = self.db.get_instance_full(&participant.canonical_name).map_err(unavailable)?.ok_or(InboxFailure::StaleParticipant)?;
        let messages = self.db.unread_messages_up_to(&participant.canonical_name, crate::shared::MAX_MESSAGES_PER_DELIVERY).map_err(unavailable)?;
        let Some(count) = NonZeroUsize::new(messages.len()) else {
            return Ok(None);
        };
        let through = messages.last().and_then(|message| message.event_id).ok_or(InboxFailure::Unavailable)?;
        Ok(Some((PendingSnapshot { after: EventId(row.last_event_id), through: EventId(through), count }, messages)))
    }

    fn stored(&self, clause: &str, values: &[&dyn rusqlite::ToSql]) -> Result<Option<StoredClaim>, InboxFailure> {
        let sql = format!("SELECT lineage, claim, resolution, receipt FROM inbox_claims WHERE {clause}");
        let row: Option<(i64, String, String, Option<String>)> = self.db.conn().query_row(&sql, values, |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).optional().map_err(unavailable)?;
        row.map(|(lineage, claim, resolution, receipt)| {
            Ok(StoredClaim { lineage, claim: serde_json::from_str(&claim).map_err(unavailable)?, resolution, receipt: receipt.map(|receipt| serde_json::from_str(&receipt)).transpose().map_err(unavailable)? })
        })
        .transpose()
    }

    fn exact_claim(&self, target: &ActivationTarget, batch: &BatchId) -> Result<StoredClaim, InboxFailure> {
        self.validate(target)?;
        let stored = self.stored("batch_id = ?", &[&batch.0])?.ok_or(InboxFailure::InvalidEvidence)?;
        if stored.claim.target != *target || stored.resolution != "open" {
            return Err(InboxFailure::InvalidEvidence);
        }
        Ok(stored)
    }

    fn write_phase(&self, mut stored: StoredClaim, phase: ClaimPhase) -> Result<(), InboxFailure> {
        stored.claim.phase = phase;
        self.db.conn().execute("UPDATE inbox_claims SET claim = ? WHERE batch_id = ? AND resolution = 'open'", params![serde_json::to_string(&stored.claim).map_err(unavailable)?, stored.claim.batch.id.0]).map_err(unavailable)?;
        Ok(())
    }

    fn valid_receipt(claim: &OutstandingClaim, receipt: &DeliveryReceipt) -> bool {
        let medium = matches!(
            (&claim.consumer, &receipt.evidence),
            (InboxConsumer::Hook, DeliveryEvidence::HookOutputWritten)
                | (InboxConsumer::ExplicitListen, DeliveryEvidence::ToolOutputWritten)
                | (InboxConsumer::NativeInput, DeliveryEvidence::ProviderInputRecorded { .. })
                | (InboxConsumer::ProductInput, DeliveryEvidence::ProductInputRecorded(_))
        );
        medium && claim.target == receipt.target && claim.batch.id == receipt.batch && claim.batch.window.through == receipt.through && claim.phase != ClaimPhase::Prepared
    }

    fn valid_release(claim: &OutstandingClaim, proof: &NoInputEvidence) -> bool {
        match proof {
            NoInputEvidence::NeverIssued => claim.phase == ClaimPhase::Prepared,
            NoInputEvidence::ProviderRejectedBeforeInput { attempt } => *attempt == claim.attempt && matches!(claim.phase, ClaimPhase::Issuing | ClaimPhase::Blocked),
            NoInputEvidence::OldExecutionEndedAndHistoryComplete { snapshot_reference } => !snapshot_reference.trim().is_empty(),
        }
    }

    fn advance(&self, lineage: i64, target: &ActivationTarget, through: EventId) -> Result<(), InboxFailure> {
        self.validate(target)?;
        let changed = self
            .db
            .conn()
            .execute("UPDATE instances SET last_event_id = MAX(last_event_id, ?) WHERE name = ? AND created_at = ?", params![through.0, target.participant.canonical_name, f64::from_bits(target.participant.incarnation_bits)])
            .map_err(unavailable)?;
        if changed != 1 {
            return Err(InboxFailure::StaleParticipant);
        }
        let snapshot = self.db.get_instance(&target.participant.canonical_name).map_err(unavailable)?.ok_or(InboxFailure::StaleParticipant)?;
        self.db.conn().execute("UPDATE participant_sessions SET delivered_through = MAX(delivered_through, ?), snapshot = ? WHERE id = ?", params![through.0, snapshot.to_string(), lineage]).map_err(unavailable)?;
        Ok(())
    }

    /// Frozen raw messages for explicit output formats. These are the exact
    /// messages claimed in prepare, not a second read of the mutable mailbox.
    pub(crate) fn frozen_messages(&self, batch: &BatchId) -> Result<Vec<Message>, InboxFailure> {
        let json: String = self.db.conn().query_row("SELECT messages FROM inbox_claims WHERE batch_id = ?", [&batch.0], |row| row.get(0)).map_err(unavailable)?;
        serde_json::from_str(&json).map_err(unavailable)
    }

    fn product_source(&self, target: &ActivationTarget, batch: &PreparedBatch, generation: &str) -> Result<ProductBatch, InboxFailure> {
        let messages = self.frozen_messages(&batch.id)?;
        let [message] = messages.as_slice() else {
            return Err(InboxFailure::InvalidEvidence);
        };
        let member = message.event_id.ok_or(InboxFailure::InvalidEvidence)?;
        let raw = self.db.kv_get(&ProductSend::key(generation, member)).map_err(unavailable)?.ok_or(InboxFailure::InvalidEvidence)?;
        let source: ProductSend = serde_json::from_str(&raw).map_err(unavailable)?;
        if source.participant != target.participant || source.execution != target.execution || member != batch.window.through.0 {
            return Err(InboxFailure::InvalidEvidence);
        }
        Ok(ProductBatch { batch: batch.clone(), request_id: source.request_id, message_id: member.to_string() })
    }

    pub(crate) fn prepare_product(&mut self, target: &ActivationTarget, attempt: &AttemptId, generation: &str) -> Result<Option<ProductBatch>, PrepareFailure> {
        // Freeze, read exact members and validate their origin in the same
        // outer transaction. Foreign/mixed input leaves no claim or footer.
        let db = self.db;
        db.with_write_scope(|| {
            self.validate(target)?;
            let Some(snapshot) = self.snapshot(&target.participant)? else {
                return Ok(None);
            };
            let batch = self.prepare(target, attempt, &snapshot, InboxConsumer::ProductInput)?;
            Ok(Some(self.product_source(target, &batch, generation).map_err(PrepareFailure::Store)?))
        })
        .map_err(|error| error.downcast_ref::<PrepareFailure>().cloned().or_else(|| error.downcast_ref::<InboxFailure>().cloned().map(PrepareFailure::Store)).unwrap_or_else(|| PrepareFailure::Store(unavailable(error))))
    }

    fn validate_product_input(&self, receipt: &DeliveryReceipt, proof: &ProductInput, batch: &PreparedBatch) -> Result<(), InboxFailure> {
        let generation = self.db.read_database_generation().map_err(unavailable)?;
        let source = self.product_source(&receipt.target, batch, &generation)?;
        let attachment = SqliteAttachmentStore::new(self.db).current_for_participant(&receipt.target.participant).map_err(unavailable)?.ok_or(InboxFailure::StaleParticipant)?;
        if attachment.target != receipt.target
            || attachment.session.session_id != proof.thread_id
            || proof.turn_id.is_empty()
            || proof.item_id.is_empty()
            || proof.request_id != source.request_id
            || proof.message_id != source.message_id
            || proof.payload_digest != ProductInput::digest(&source.batch.formatted_context)
        {
            return Err(InboxFailure::InvalidEvidence);
        }
        Ok(())
    }
}

impl From<InboxFailure> for PrepareFailure {
    fn from(error: InboxFailure) -> Self {
        Self::Store(error)
    }
}

impl PendingInbox for SqlitePendingInbox<'_> {
    fn snapshot(&self, participant: &ParticipantRef) -> Result<Option<PendingSnapshot>, InboxFailure> {
        self.transaction(|| Ok(self.read_batch(participant)?.map(|(snapshot, _)| snapshot)))
    }

    fn outstanding(&self, target: &ActivationTarget) -> Result<Option<OutstandingClaim>, InboxFailure> {
        self.transaction(|| {
            let lineage = self.lineage(target)?;
            Ok(self.stored("lineage = ? AND resolution = 'open'", &[&lineage])?.map(|stored| stored.claim))
        })
    }

    fn delivery_mode(&self, target: &ActivationTarget) -> Result<DeliveryMode, InboxFailure> {
        self.validate(target)?;
        let mode: String = self.db.conn().query_row("SELECT delivery_mode FROM participant_attachments WHERE binding_id = ? AND active = 1", [&target.binding.0], |row| row.get(0)).map_err(unavailable)?;
        match mode.as_str() {
            "hook" => Ok(DeliveryMode::HookContext),
            "native" => Ok(DeliveryMode::NativeInput),
            "product" => Ok(DeliveryMode::DedicatedProduct),
            _ => Err(InboxFailure::Unavailable),
        }
    }

    fn select_delivery_mode(&mut self, target: &ActivationTarget, mode: DeliveryMode) -> Result<(), InboxFailure> {
        self.transaction(|| {
            self.validate(target)?;
            let row = self.db.get_instance_full(&target.participant.canonical_name).map_err(unavailable)?.ok_or(InboxFailure::StaleParticipant)?;
            let product = ProductSend::execution(&row).map_err(unavailable)?;
            if product.is_some() != (mode == DeliveryMode::DedicatedProduct) || product.is_some_and(|execution| execution != target.execution) {
                return Err(InboxFailure::InvalidEvidence);
            }
            let mode = match mode {
                DeliveryMode::HookContext => "hook",
                DeliveryMode::NativeInput => "native",
                DeliveryMode::DedicatedProduct => "product",
            };
            self.db.conn().execute("UPDATE participant_attachments SET delivery_mode = ? WHERE binding_id = ? AND active = 1", params![mode, target.binding.0]).map_err(unavailable)?;
            Ok(())
        })
    }

    fn prepare(&mut self, target: &ActivationTarget, attempt: &AttemptId, snapshot: &PendingSnapshot, consumer: InboxConsumer) -> Result<PreparedBatch, PrepareFailure> {
        let result = self.transaction(|| {
            let lineage = self.lineage(target)?;
            let mode = self.delivery_mode(target)?;
            let row = self.db.get_instance_full(&target.participant.canonical_name).map_err(unavailable)?.ok_or(InboxFailure::StaleParticipant)?;
            let product = ProductSend::execution(&row).map_err(unavailable)?.is_some();
            if product != (consumer == InboxConsumer::ProductInput)
                || (product && mode != DeliveryMode::DedicatedProduct)
                || matches!((&consumer, mode), (InboxConsumer::Hook, DeliveryMode::NativeInput) | (InboxConsumer::NativeInput, DeliveryMode::HookContext))
            {
                return Err(PrepareFailure::WrongDeliveryMode);
            }
            if let Some(stored) = self.stored("lineage = ? AND resolution = 'open'", &[&lineage])? {
                if stored.claim.target != *target {
                    return Err(PrepareFailure::UnresolvedPredecessor);
                }
                if stored.claim.attempt == *attempt && stored.claim.consumer == consumer && stored.claim.batch.window == *snapshot {
                    return Ok(stored.claim.batch);
                }
                return Err(PrepareFailure::ClaimedByAnotherConsumer);
            }
            let Some((current, messages)) = self.read_batch(&target.participant)? else {
                return Err(PrepareFailure::NoPendingMail);
            };
            if current != *snapshot {
                return Err(PrepareFailure::SnapshotChanged);
            }
            let formatted_context = self.format_output(&target.participant, &messages)?;
            let batch = PreparedBatch { id: BatchId(uuid::Uuid::new_v4().to_string()), window: current, formatted_context };
            let claim = OutstandingClaim { target: target.clone(), attempt: attempt.clone(), batch: batch.clone(), consumer, phase: ClaimPhase::Prepared };
            self.db
                .conn()
                .execute("INSERT INTO inbox_claims(batch_id, lineage, claim, messages) VALUES (?, ?, ?, ?)", params![batch.id.0, lineage, serde_json::to_string(&claim).map_err(unavailable)?, serde_json::to_string(&messages).map_err(unavailable)?])
                .map_err(unavailable)?;
            Ok(batch)
        });
        if let Ok(batch) = &result {
            observe_claim_phase(self.db, target, &batch.id, ClaimPhase::Prepared);
        }
        result
    }

    fn mark_issuing(&mut self, target: &ActivationTarget, batch: &BatchId) -> Result<(), InboxFailure> {
        self.record_phase(target, batch, ClaimPhase::Issuing)
    }

    fn record_phase(&mut self, target: &ActivationTarget, batch: &BatchId, phase: ClaimPhase) -> Result<(), InboxFailure> {
        let result = self.transaction(|| {
            let stored = self.exact_claim(target, batch)?;
            let valid = stored.claim.phase == phase
                || matches!(
                    (&stored.claim.phase, &phase),
                    (ClaimPhase::Prepared, ClaimPhase::Issuing)
                        | (ClaimPhase::Issuing, ClaimPhase::Admitted | ClaimPhase::Uncertain | ClaimPhase::Blocked)
                        | (ClaimPhase::Admitted, ClaimPhase::Uncertain | ClaimPhase::Blocked)
                        | (ClaimPhase::Uncertain, ClaimPhase::Admitted | ClaimPhase::Blocked)
                );
            if !valid {
                return Err(InboxFailure::InvalidEvidence);
            }
            self.write_phase(stored, phase.clone())
        });
        if result.is_ok() {
            observe_claim_phase(self.db, target, batch, phase);
        }
        result
    }

    fn release_proven_unrecorded(&mut self, target: &ActivationTarget, batch: &BatchId, proof: NoInputEvidence) -> Result<(), InboxFailure> {
        self.transaction(|| {
            let stored = self.exact_claim(target, batch)?;
            if !Self::valid_release(&stored.claim, &proof) {
                return Err(InboxFailure::InvalidEvidence);
            }
            self.db.conn().execute("UPDATE inbox_claims SET resolution = 'released' WHERE batch_id = ?", [&batch.0]).map_err(unavailable)?;
            Ok(())
        })
    }

    fn confirm(&mut self, receipt: DeliveryReceipt) -> Result<Confirmation, InboxFailure> {
        let result = self.transaction(|| {
            match self.validate(&receipt.target) {
                Err(InboxFailure::StaleParticipant) => return Ok(Confirmation::StaleFence),
                other => other?,
            }
            let Some(stored) = self.stored("batch_id = ?", &[&receipt.batch.0])? else {
                return Ok(Confirmation::WrongBatch);
            };
            if stored.claim.target != receipt.target || stored.claim.batch.window.through != receipt.through {
                return Ok(Confirmation::WrongBatch);
            }
            if !Self::valid_receipt(&stored.claim, &receipt) {
                return Err(InboxFailure::InvalidEvidence);
            }
            if let DeliveryEvidence::ProductInputRecorded(proof) = &receipt.evidence {
                self.validate_product_input(&receipt, proof, &stored.claim.batch)?;
            }
            if stored.resolution == "confirmed" {
                return if stored.receipt.as_ref() == Some(&receipt) { Ok(Confirmation::AlreadyConfirmed) } else { Err(InboxFailure::InvalidEvidence) };
            }
            if stored.resolution != "open" {
                return Ok(Confirmation::WrongBatch);
            }
            self.advance(stored.lineage, &receipt.target, receipt.through)?;
            self.db.conn().execute("UPDATE inbox_claims SET resolution = 'confirmed', receipt = ? WHERE batch_id = ?", params![serde_json::to_string(&receipt).map_err(unavailable)?, receipt.batch.0]).map_err(unavailable)?;
            Ok(Confirmation::Advanced)
        });
        if let Ok(confirmation @ (Confirmation::Advanced | Confirmation::AlreadyConfirmed)) = &result
            && let Some(active) = crate::service::context::active_observation()
        {
            use crate::observability::events::{ConfirmationDisposition, DeliveryBoundary, DomainCommitFact, InboxClaimRef};
            let boundary = match &receipt.evidence {
                DeliveryEvidence::HookOutputWritten => DeliveryBoundary::HookOutputWritten,
                DeliveryEvidence::ToolOutputWritten => DeliveryBoundary::ToolOutputWritten,
                DeliveryEvidence::ProviderInputRecorded { .. } | DeliveryEvidence::ProductInputRecorded(_) => DeliveryBoundary::ProviderInputRecorded,
            };
            self.db.observe_domain(DomainCommitFact::DeliveryConfirmationStored {
                claim: InboxClaimRef { scope: active.epoch.daemon.scope, database_generation: active.epoch.database_generation, target: receipt.target.clone(), batch: receipt.batch.clone() },
                boundary,
                confirmation: if *confirmation == Confirmation::Advanced { ConfirmationDisposition::Advanced } else { ConfirmationDisposition::AlreadyConfirmed },
            });
        }
        result
    }

    fn reconcile_predecessor(&mut self, predecessor: &BatchId, successor: &ActivationTarget, proof: ReconciliationEvidence) -> Result<RecoveryOutcome, InboxFailure> {
        self.transaction(|| {
            let lineage = self.lineage(successor)?;
            let stored = self.stored("batch_id = ?", &[&predecessor.0])?.ok_or(InboxFailure::InvalidEvidence)?;
            if stored.lineage != lineage || stored.claim.target == *successor {
                return Err(InboxFailure::InvalidEvidence);
            }
            if stored.claim.consumer == InboxConsumer::ProductInput {
                return Err(InboxFailure::InvalidEvidence);
            }
            match proof {
                ReconciliationEvidence::PreviouslyRecorded(receipt) => {
                    if !Self::valid_receipt(&stored.claim, &receipt) || stored.resolution == "released" {
                        return Err(InboxFailure::InvalidEvidence);
                    }
                    if stored.resolution == "confirmed" && stored.receipt.as_ref() != Some(&receipt) {
                        return Err(InboxFailure::InvalidEvidence);
                    }
                    self.advance(lineage, successor, receipt.through)?;
                    self.db.conn().execute("UPDATE inbox_claims SET resolution = 'confirmed', receipt = ? WHERE batch_id = ?", params![serde_json::to_string(&receipt).map_err(unavailable)?, predecessor.0]).map_err(unavailable)?;
                    Ok(RecoveryOutcome::RecoveredDelivery)
                }
                ReconciliationEvidence::ProvenUnrecorded(proof) => {
                    if !Self::valid_release(&stored.claim, &proof) || stored.resolution == "confirmed" {
                        return Err(InboxFailure::InvalidEvidence);
                    }
                    self.db.conn().execute("UPDATE inbox_claims SET resolution = 'released' WHERE batch_id = ?", [&predecessor.0]).map_err(unavailable)?;
                    Ok(RecoveryOutcome::ReleasedForSuccessor)
                }
            }
        })
    }

    fn delivered_through(&self, participant: &ParticipantRef) -> Result<EventId, InboxFailure> {
        self.validate_participant(participant)?;
        self.db.get_instance_full(&participant.canonical_name).map_err(unavailable)?.map(|row| EventId(row.last_event_id)).ok_or(InboxFailure::StaleParticipant)
    }
}
