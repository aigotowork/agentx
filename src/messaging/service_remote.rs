//! Typed client adapter for the existing domain port; no local SQLite fallback.

use super::activation::AttemptId;
use super::inbox::*;
use super::service::*;
use crate::agents::attachments::{ActivationTarget, ParticipantRef};
use crate::service::{self, client::Client};

#[derive(Clone)]
pub(crate) struct RemoteInbox {
    client: Client,
}

impl RemoteInbox {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }
    pub(crate) fn prepare_product(&self, target: &ActivationTarget, attempt: AttemptId) -> Result<Option<ProductBatch>, PrepareFailure> {
        match self.call(InboxOperation::PrepareProduct { target: target.clone(), attempt }) {
            Ok(InboxOutcome::ProductPrepared(result)) => result,
            _ => Err(PrepareFailure::Store(InboxFailure::Unavailable)),
        }
    }
    fn call(&self, operation: InboxOperation) -> Result<InboxOutcome, InboxFailure> {
        match self.client.call(service::Request::Messages(Request::Inbox(operation))) {
            Ok(service::Response::Messages(Response::Inbox(outcome))) => Ok(outcome),
            _ => Err(InboxFailure::Unavailable),
        }
    }
}

impl PendingInbox for RemoteInbox {
    fn snapshot(&self, participant: &ParticipantRef) -> Result<Option<PendingSnapshot>, InboxFailure> {
        match self.call(InboxOperation::Snapshot { participant: participant.clone() }) {
            Ok(InboxOutcome::Snapshot(result)) => result,
            _ => Err(InboxFailure::Unavailable),
        }
    }
    fn outstanding(&self, target: &ActivationTarget) -> Result<Option<OutstandingClaim>, InboxFailure> {
        match self.call(InboxOperation::Outstanding { target: target.clone() }) {
            Ok(InboxOutcome::Outstanding(result)) => result,
            _ => Err(InboxFailure::Unavailable),
        }
    }
    fn delivery_mode(&self, target: &ActivationTarget) -> Result<DeliveryMode, InboxFailure> {
        match self.call(InboxOperation::DeliveryMode { target: target.clone() }) {
            Ok(InboxOutcome::DeliveryMode(result)) => result,
            _ => Err(InboxFailure::Unavailable),
        }
    }
    fn select_delivery_mode(&mut self, target: &ActivationTarget, mode: DeliveryMode) -> Result<(), InboxFailure> {
        match self.call(InboxOperation::SelectDeliveryMode { target: target.clone(), mode }) {
            Ok(InboxOutcome::Done(result)) => result,
            _ => Err(InboxFailure::Unavailable),
        }
    }
    fn prepare(&mut self, target: &ActivationTarget, attempt: &AttemptId, snapshot: &PendingSnapshot, consumer: InboxConsumer) -> Result<PreparedBatch, PrepareFailure> {
        match self.call(InboxOperation::Prepare { target: target.clone(), attempt: attempt.clone(), snapshot: snapshot.clone(), consumer }) {
            Ok(InboxOutcome::Prepared(result)) => result,
            _ => Err(PrepareFailure::Store(InboxFailure::Unavailable)),
        }
    }
    fn mark_issuing(&mut self, target: &ActivationTarget, batch: &BatchId) -> Result<(), InboxFailure> {
        match self.call(InboxOperation::MarkIssuing { target: target.clone(), batch: batch.clone() }) {
            Ok(InboxOutcome::Done(result)) => result,
            _ => Err(InboxFailure::Unavailable),
        }
    }
    fn record_phase(&mut self, target: &ActivationTarget, batch: &BatchId, phase: ClaimPhase) -> Result<(), InboxFailure> {
        match self.call(InboxOperation::RecordPhase { target: target.clone(), batch: batch.clone(), phase }) {
            Ok(InboxOutcome::Done(result)) => result,
            _ => Err(InboxFailure::Unavailable),
        }
    }
    fn release_proven_unrecorded(&mut self, target: &ActivationTarget, batch: &BatchId, proof: NoInputEvidence) -> Result<(), InboxFailure> {
        match self.call(InboxOperation::ReleaseProvenUnrecorded { target: target.clone(), batch: batch.clone(), proof }) {
            Ok(InboxOutcome::Done(result)) => result,
            _ => Err(InboxFailure::Unavailable),
        }
    }
    fn confirm(&mut self, receipt: DeliveryReceipt) -> Result<Confirmation, InboxFailure> {
        match self.call(InboxOperation::Confirm(receipt)) {
            Ok(InboxOutcome::Confirmed(result)) => result,
            _ => Err(InboxFailure::Unavailable),
        }
    }
    fn reconcile_predecessor(&mut self, predecessor: &BatchId, successor: &ActivationTarget, proof: ReconciliationEvidence) -> Result<RecoveryOutcome, InboxFailure> {
        match self.call(InboxOperation::ReconcilePredecessor { predecessor: predecessor.clone(), successor: successor.clone(), proof }) {
            Ok(InboxOutcome::Recovered(result)) => result,
            _ => Err(InboxFailure::Unavailable),
        }
    }
    fn delivered_through(&self, participant: &ParticipantRef) -> Result<EventId, InboxFailure> {
        match self.call(InboxOperation::DeliveredThrough { participant: participant.clone() }) {
            Ok(InboxOutcome::DeliveredThrough(result)) => result,
            _ => Err(InboxFailure::Unavailable),
        }
    }
}
