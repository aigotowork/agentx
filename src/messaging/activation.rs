//! Required activation port owned by pending-mail policy, not by a provider.
//! Native RPC, PTY input and concrete service construction stay in adapters.

use crate::agents::attachments::ActivationTarget;
use crate::messaging::inbox::PreparedBatch;
use crate::messaging::inbox::{ClaimPhase, InboxConsumer, InboxFailure, NoInputEvidence, PendingInbox, PrepareFailure};

/// Live-runtime coalescing token, never a provider idempotency guarantee.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct AttemptId(pub String);

/// Advance actual frozen mail. This API never chooses a fresh batch by name.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActivationRequest {
    pub target: ActivationTarget,
    pub attempt: AttemptId,
    pub batch: PreparedBatch,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum DeferReason {
    NotInitialized,
    NoBinding,
    AlreadyRunning,
    AwaitingApproval,
    AwaitingUserInput,
    AnotherAttemptPending,
    ExternalMutationPending,
    UserDraftOrActivity,
    OutputBackpressure,
    StateNotSynchronized,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ActivationRejection {
    StaleTarget,
    NoPendingMail,
    AttemptConflict,
    UnsupportedBackend,
    RuntimeClosed,
}

/// Accepted transfers attempt ownership, not proof of native input or delivery.
/// Deferred/rejected guarantee this offer issued no external activation.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Admission {
    Accepted { attempt: AttemptId },
    Deferred(DeferReason),
    Rejected(ActivationRejection),
}

/// Composition supplies this capability to message policy. Reoffering an
/// accepted attempt returns current admission without a second external effect.
/// The adapter must revalidate the target and durably mark the batch Issuing
/// before any possibly partial provider write. Progress never commits a cursor.
pub(crate) trait ActivationDriver {
    fn offer(&mut self, request: ActivationRequest) -> Admission;
}

/// Optional because a PTY may observe only hook output. Provider ids remain opaque.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TurnEvidence {
    pub opaque_turn_id: Option<String>,
}

/// Input/cancellation outcomes remain fenced. Cancellation or turn completion
/// does not prove input absent. On connection close retire the driver, retain
/// uncertain claims and do not cancel unrelated shared-backend work.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum DriverEvent {
    Ready { target: ActivationTarget },
    Deferred { target: ActivationTarget, reason: DeferReason },
    Issued { target: ActivationTarget, attempt: AttemptId },
    Admitted { target: ActivationTarget, attempt: AttemptId, turn: TurnEvidence },
    ProvenNotAdmitted { target: ActivationTarget, attempt: AttemptId, detail: String },
    OutcomeUnknown { target: ActivationTarget, attempt: AttemptId, detail: String },
    Settled { target: ActivationTarget, turn: TurnEvidence },
    Retired { target: ActivationTarget },
}

/// Provider-neutral progression of one fenced mailbox. The host supplies its
/// notifications/scan loop and driver; policy owns claim selection and never
/// interprets RPC success or turn completion as delivery.
#[derive(Default)]
pub(crate) struct ActivationPolicy {
    offered: std::collections::HashMap<ActivationTarget, AttemptId>,
}

impl ActivationPolicy {
    /// Sequential seam for an actor whose driver also owns the same injected
    /// inbox: prepare, offer, then finish without aliasing mutable ports.
    pub(crate) fn prepare_request(&mut self, inbox: &mut dyn PendingInbox, target: &ActivationTarget) -> Result<Option<ActivationRequest>, InboxFailure> {
        let (attempt, batch) = match inbox.outstanding(target)? {
            Some(claim) => {
                if claim.target != *target || claim.phase != ClaimPhase::Prepared || claim.consumer != InboxConsumer::NativeInput {
                    return Ok(None);
                }
                if self.offered.get(target) == Some(&claim.attempt) {
                    return Ok(None);
                }
                (claim.attempt, claim.batch)
            }
            None => {
                self.offered.remove(target);
                let Some(snapshot) = inbox.snapshot(&target.participant)? else {
                    return Ok(None);
                };
                let attempt = AttemptId(uuid::Uuid::new_v4().to_string());
                let batch = match inbox.prepare(target, &attempt, &snapshot, InboxConsumer::NativeInput) {
                    Ok(batch) => batch,
                    Err(PrepareFailure::Store(error)) => return Err(error),
                    Err(_) => return Ok(None),
                };
                (attempt, batch)
            }
        };
        Ok(Some(ActivationRequest { target: target.clone(), attempt, batch }))
    }

    pub(crate) fn finish_admission(&mut self, inbox: &mut dyn PendingInbox, request: &ActivationRequest, admission: &Admission) -> Result<(), InboxFailure> {
        match admission {
            Admission::Accepted { attempt: accepted } if *accepted == request.attempt => {
                self.offered.insert(request.target.clone(), request.attempt.clone());
            }
            Admission::Accepted { .. } => return Err(InboxFailure::InvalidEvidence),
            Admission::Deferred(_) | Admission::Rejected(_) => {
                inbox.release_proven_unrecorded(&request.target, &request.batch.id, NoInputEvidence::NeverIssued)?;
                self.offered.remove(&request.target);
            }
        }
        Ok(())
    }

    /// Late events are checked against the current outstanding claim. A closed
    /// or superseded driver cannot alter its successor's mailbox.
    pub(crate) fn observe(&mut self, inbox: &mut dyn PendingInbox, event: DriverEvent) -> Result<(), InboxFailure> {
        let (target, attempt, phase) = match event {
            DriverEvent::Issued { target, attempt } => (target, attempt, ClaimPhase::Issuing),
            DriverEvent::Admitted { target, attempt, .. } => (target, attempt, ClaimPhase::Admitted),
            DriverEvent::OutcomeUnknown { target, attempt, .. } => (target, attempt, ClaimPhase::Uncertain),
            DriverEvent::ProvenNotAdmitted { target, attempt, .. } => {
                let claim = match inbox.outstanding(&target) {
                    Ok(Some(claim)) if claim.target == target && claim.attempt == attempt => claim,
                    Ok(_) | Err(InboxFailure::StaleParticipant) => return Ok(()),
                    Err(error) => return Err(error),
                };
                inbox.record_phase(&target, &claim.batch.id, ClaimPhase::Blocked)?;
                inbox.release_proven_unrecorded(&target, &claim.batch.id, NoInputEvidence::ProviderRejectedBeforeInput { attempt })?;
                self.offered.remove(&target);
                return Ok(());
            }
            DriverEvent::Retired { target } => {
                self.offered.remove(&target);
                return Ok(());
            }
            DriverEvent::Ready { .. } | DriverEvent::Deferred { .. } | DriverEvent::Settled { .. } => return Ok(()),
        };
        let claim = match inbox.outstanding(&target) {
            Ok(Some(claim)) if claim.target == target && claim.attempt == attempt => claim,
            Ok(_) | Err(InboxFailure::StaleParticipant) => return Ok(()),
            Err(error) => return Err(error),
        };
        if phase == ClaimPhase::Issuing && claim.phase != ClaimPhase::Prepared {
            return Ok(());
        }
        inbox.record_phase(&target, &claim.batch.id, phase)
    }
}
