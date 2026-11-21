//! Frozen actual-message activation, input receipts and conservative recovery.

use anyhow::Result;
use serde_json::json;

use crate::agents::attachments::ActivationTarget;
use crate::messaging::activation::{ActivationDriver, ActivationRejection, ActivationRequest, Admission, DeferReason, DriverEvent, TurnEvidence};
use crate::messaging::inbox::{ClaimPhase, Confirmation, DeliveryEvidence, DeliveryReceipt, OutstandingClaim, ReconciliationEvidence};

#[cfg(test)]
use super::super::protocol::FrameKind;
use super::super::protocol::{ClientRequestOwner, NativeFrame, RawObject, RpcId};
use super::super::mail_evidence::{client_id, item_matches};
use super::session::{child, field};
use super::{NativeAttempt, NativeBridge, StateRead, ThreadState};

/// A pure receipt seam for independent counterexamples. Successful admission,
/// turn completion, item/started and mismatched thread/content never qualify.
#[cfg(test)]
pub(crate) fn recorded_input_matches(raw: &str, thread_id: &str, correlation: &str, context: &str) -> bool {
    let Ok(message) = NativeFrame::parse(raw) else {
        return false;
    };
    if !matches!(&message.kind, FrameKind::Notification { method } if method == "item/completed") {
        return false;
    }
    let Ok(Some(params)) = message.params() else {
        return false;
    };
    field::<String>(&params, "threadId").as_deref() == Some(thread_id) && child(&params, "item").is_some_and(|item| item_matches(&item, correlation, context))
}

impl NativeBridge<'_> {
    fn eligibility(&self, thread_id: &str) -> Result<(), DeferReason> {
        if !self.handshake.ready() {
            return Err(DeferReason::NotInitialized);
        }
        let Some(root) = self.roots.get(thread_id) else {
            return Err(DeferReason::NoBinding);
        };
        if root.suspended {
            return Err(DeferReason::UserDraftOrActivity);
        }
        if self.external.values().any(|operation| operation.mutation && operation.thread_id.as_ref().is_none_or(|thread| thread == thread_id)) {
            return Err(DeferReason::ExternalMutationPending);
        }
        for (thread, input) in self.controls.values() {
            if thread.as_ref().is_none_or(|thread| thread == thread_id) {
                return Err(if *input { DeferReason::AwaitingUserInput } else { DeferReason::AwaitingApproval });
            }
        }
        match root.state {
            ThreadState::Idle => Ok(()),
            ThreadState::Active if root.turn_id.is_some() => Ok(()),
            ThreadState::AwaitingApproval => Err(DeferReason::AwaitingApproval),
            ThreadState::AwaitingInput => Err(DeferReason::AwaitingUserInput),
            ThreadState::Retired => Err(DeferReason::NoBinding),
            _ => Err(DeferReason::StateNotSynchronized),
        }
    }

    fn emit(&mut self, event: DriverEvent) -> Result<()> {
        self.policy.observe(self.ports.inbox, event.clone())?;
        self.events.push_back(event);
        Ok(())
    }

    /// Generic policy selects/freezes the actual batch. The bridge contributes
    /// native eligibility and fenced effects, not its own mailbox query policy.
    pub(crate) fn drive_pending(&mut self) -> Result<()> {
        if self.closed || !self.supported || !self.handshake.ready() {
            return Ok(());
        }
        let threads: Vec<String> = self.roots.keys().cloned().collect();
        let mut policy = std::mem::take(&mut self.policy);
        let result = (|| {
            for thread in threads {
                let target = self.roots[&thread].binding.attachment.target.clone();
                if !self.ports.attachments.validate(&target)? || self.eligibility(&thread).is_err() || self.attempts.contains_key(&thread) {
                    continue;
                }
                if let Some(claim) = self.ports.inbox.outstanding(&target)?
                    && (claim.target != target || claim.phase != ClaimPhase::Prepared)
                {
                    if claim.phase != ClaimPhase::Blocked && self.recovery_scanned.insert(claim.batch.id.clone()) {
                        self.request_state(&thread, Some(claim))?;
                    }
                    continue;
                }
                let Some(request) = policy.prepare_request(self.ports.inbox, &target)? else {
                    continue;
                };
                let admission = self.offer(request.clone());
                policy.finish_admission(self.ports.inbox, &request, &admission)?;
                if self.fatal.is_some() {
                    break;
                }
            }
            Ok(())
        })();
        self.policy = policy;
        result
    }

    pub(super) fn request_state(&mut self, thread_id: &str, claim: Option<OutstandingClaim>) -> Result<()> {
        let Some(root) = self.roots.get(thread_id) else {
            return Ok(());
        };
        if root.state == ThreadState::Retired || self.state_reads.values().any(|read| read.target == root.binding.attachment.target) {
            return Ok(());
        }
        let target = root.binding.attachment.target.clone();
        let params = json!({"threadId": thread_id, "includeTurns": true});
        self.send_state_read("thread/read", params, StateRead { target, claim, history: false, pages: 0 })
    }

    fn send_state_read(&mut self, method: &str, params: serde_json::Value, read: StateRead) -> Result<()> {
        let id = self.router.reserve_internal(ClientRequestOwner::Reconciliation { target: read.target.clone() })?;
        let raw = NativeFrame::parse(&json!({"id": 0, "method": method, "params": params}).to_string())?.with_id(&id)?;
        self.state_reads.insert(id, read);
        self.ports.transport.send(&raw)?;
        Ok(())
    }

    pub(super) fn observe_internal_result(&mut self, id: &RpcId, owner: ClientRequestOwner, raw: &str) -> Result<()> {
        let message = NativeFrame::parse(raw)?;
        if let Some(read) = self.state_reads.remove(id) {
            if !self.ports.attachments.validate(&read.target)? {
                return Ok(());
            }
            let Some(result) = child(&message.object, "result") else {
                return Ok(());
            };
            let thread_id = self.roots.iter().find(|(_, root)| root.binding.attachment.target == read.target).map(|(thread, _)| thread.clone());
            let Some(thread_id) = thread_id else {
                return Ok(());
            };
            let mut found = false;
            if !read.history {
                if let Some(thread) = child(&result, "thread") {
                    if field::<String>(&thread, "id").as_deref() != Some(thread_id.as_str()) {
                        return Ok(());
                    }
                    self.observe_thread(&thread_id, &thread);
                    if let Some(claim) = &read.claim {
                        for turn in field::<Vec<Box<serde_json::value::RawValue>>>(&thread, "turns").unwrap_or_default() {
                            if let Ok(turn) = RawObject::parse(turn.get()) {
                                for item in field::<Vec<Box<serde_json::value::RawValue>>>(&turn, "items").unwrap_or_default() {
                                    if let Ok(item) = RawObject::parse(item.get())
                                        && item_matches(&item, &client_id(&claim.batch), &claim.batch.formatted_context)
                                    {
                                        self.confirm_recovered(&read.target, claim, field(&turn, "id"))?;
                                        found = true;
                                        break;
                                    }
                                }
                            }
                            if found {
                                break;
                            }
                        }
                    }
                }
            } else if let Some(claim) = &read.claim {
                for entry in field::<Vec<Box<serde_json::value::RawValue>>>(&result, "data").unwrap_or_default() {
                    if let Ok(entry) = RawObject::parse(entry.get())
                        && let Some(item) = child(&entry, "item")
                        && item_matches(&item, &client_id(&claim.batch), &claim.batch.formatted_context)
                    {
                        self.confirm_recovered(&read.target, claim, field(&entry, "turnId"))?;
                        found = true;
                        break;
                    }
                }
            }
            if !found && read.claim.is_some() && read.pages < 100 {
                let cursor: Option<String> = if read.history { field(&result, "nextCursor") } else { None };
                if !read.history || cursor.is_some() {
                    let params = json!({"threadId": thread_id, "limit": 100, "sortDirection": "desc", "cursor": cursor});
                    self.send_state_read("thread/items/list", params, StateRead { target: read.target, claim: read.claim, history: true, pages: read.pages + 1 })?;
                }
            }
            // Even a complete empty page is not proof that an old execution
            // cannot still record input. Recovery here only proves positives.
            return Ok(());
        }
        let ClientRequestOwner::Activation { target, attempt } = owner else {
            return Ok(());
        };
        let thread_id = self.attempts.iter().find(|(_, pending)| pending.request.target == target && pending.request.attempt == attempt).map(|(thread, _)| thread.clone());
        let Some(thread_id) = thread_id else {
            return Ok(());
        };
        if let Some(error) = child(&message.object, "error") {
            let detail = field::<String>(&error, "message").unwrap_or_else(|| "native rejected input with no absence proof".to_owned());
            let stale_steer = self.attempts[&thread_id].steer && field::<i64>(&error, "code") == Some(-32600) && detail.starts_with("expected active turn id `") && detail.contains("` but found `");
            if stale_steer {
                self.emit(DriverEvent::ProvenNotAdmitted { target, attempt, detail })?;
                self.attempts.remove(&thread_id);
                if let Some(root) = self.roots.get_mut(&thread_id) {
                    root.state = ThreadState::Unknown;
                }
                self.request_state(&thread_id, None)?;
            } else {
                self.emit(DriverEvent::OutcomeUnknown { target, attempt, detail })?;
            }
            return Ok(());
        }
        let result = child(&message.object, "result");
        let turn_id = result.as_ref().and_then(|result| child(result, "turn")).and_then(|turn| field::<String>(&turn, "id")).or_else(|| result.as_ref().and_then(|result| field(result, "turnId")));
        if let Some(pending) = self.attempts.get_mut(&thread_id) {
            pending.turn_id = turn_id.clone();
            if pending.blocked {
                return Ok(());
            }
        }
        self.emit(DriverEvent::Admitted { target, attempt, turn: TurnEvidence { opaque_turn_id: turn_id } })?;
        Ok(())
    }

    fn confirm_recovered(&mut self, successor: &ActivationTarget, claim: &OutstandingClaim, turn_id: Option<String>) -> Result<()> {
        let receipt = DeliveryReceipt { target: claim.target.clone(), batch: claim.batch.id.clone(), through: claim.batch.window.through, evidence: DeliveryEvidence::ProviderInputRecorded { turn: TurnEvidence { opaque_turn_id: turn_id } } };
        if claim.target == *successor {
            self.ports.inbox.confirm(receipt)?;
        } else {
            self.ports.inbox.reconcile_predecessor(&claim.batch.id, successor, ReconciliationEvidence::PreviouslyRecorded(receipt))?;
        }
        Ok(())
    }

    pub(super) fn observe_recorded_item(&mut self, thread_id: &str, turn_id: Option<String>, item: &RawObject) -> Result<()> {
        if let Some(pending) = self.attempts.get(thread_id) {
            if !item_matches(item, &pending.client_id, &pending.request.batch.formatted_context) {
                return Ok(());
            }
            let receipt = DeliveryReceipt {
                target: pending.request.target.clone(),
                batch: pending.request.batch.id.clone(),
                through: pending.request.batch.window.through,
                evidence: DeliveryEvidence::ProviderInputRecorded { turn: TurnEvidence { opaque_turn_id: turn_id } },
            };
            if matches!(self.ports.inbox.confirm(receipt)?, Confirmation::Advanced | Confirmation::AlreadyConfirmed) {
                self.attempts.remove(thread_id);
            }
        } else if let Some(root) = self.roots.get(thread_id) {
            let target = root.binding.attachment.target.clone();
            if let Some(claim) = self.ports.inbox.outstanding(&target)?
                && claim.phase != ClaimPhase::Prepared
                && item_matches(item, &client_id(&claim.batch), &claim.batch.formatted_context)
            {
                self.confirm_recovered(&target, &claim, turn_id)?;
            }
        }
        Ok(())
    }

    pub(super) fn observe_blocked_input(&mut self, thread_id: &str) -> Result<()> {
        if let Some(pending) = self.attempts.get_mut(thread_id) {
            pending.blocked = true;
            self.ports.inbox.record_phase(&pending.request.target, &pending.request.batch.id, ClaimPhase::Blocked)?;
        }
        Ok(())
    }

    pub(super) fn observe_turn_finished(&mut self, thread_id: &str, turn_id: Option<String>) -> Result<()> {
        let mut reconcile = None;
        if let Some(pending) = self.attempts.get(thread_id)
            && (pending.turn_id.is_none() || pending.turn_id == turn_id)
            && !pending.blocked
        {
            self.emit(DriverEvent::OutcomeUnknown { target: pending.request.target.clone(), attempt: pending.request.attempt.clone(), detail: "turn completed without the matching recorded user input".to_owned() })?;
            if let Some(root) = self.roots.get(thread_id) {
                reconcile = self.ports.inbox.outstanding(&root.binding.attachment.target)?;
            }
        }
        if let Some(claim) = reconcile {
            self.request_state(thread_id, Some(claim))?;
        }
        if let Some(root) = self.roots.get(thread_id) {
            self.emit(DriverEvent::Settled { target: root.binding.attachment.target.clone(), turn: TurnEvidence { opaque_turn_id: turn_id } })?;
        }
        Ok(())
    }

    pub(super) fn retain_unknown_attempts(&mut self) {
        for pending in self.attempts.values() {
            if !pending.blocked {
                let _ = self.ports.inbox.record_phase(&pending.request.target, &pending.request.batch.id, ClaimPhase::Uncertain);
            }
        }
    }
}

impl ActivationDriver for NativeBridge<'_> {
    fn offer(&mut self, request: ActivationRequest) -> Admission {
        if self.closed || self.fatal.is_some() {
            return Admission::Rejected(ActivationRejection::RuntimeClosed);
        }
        if !self.supported {
            return Admission::Rejected(ActivationRejection::UnsupportedBackend);
        }
        let thread_id = self.roots.iter().find(|(_, root)| root.binding.attachment.target == request.target).map(|(thread, _)| thread.clone());
        let Some(thread_id) = thread_id else {
            return Admission::Rejected(ActivationRejection::StaleTarget);
        };
        if let Some(pending) = self.attempts.get(&thread_id) {
            return if pending.request == request { Admission::Accepted { attempt: request.attempt } } else { Admission::Deferred(DeferReason::AnotherAttemptPending) };
        }
        if let Err(reason) = self.eligibility(&thread_id) {
            return Admission::Deferred(reason);
        }
        match self.ports.attachments.validate(&request.target) {
            Ok(true) => {}
            Ok(false) => return Admission::Rejected(ActivationRejection::StaleTarget),
            Err(_) => return Admission::Deferred(DeferReason::StateNotSynchronized),
        }
        match self.ports.inbox.outstanding(&request.target) {
            Ok(Some(claim)) if claim.target == request.target && claim.attempt == request.attempt && claim.batch == request.batch && claim.phase == ClaimPhase::Prepared => {}
            Ok(_) => return Admission::Rejected(ActivationRejection::AttemptConflict),
            Err(_) => return Admission::Deferred(DeferReason::StateNotSynchronized),
        }
        let steer = self.roots[&thread_id].state == ThreadState::Active;
        let correlation = client_id(&request.batch);
        let params = if steer {
            json!({"threadId": thread_id, "expectedTurnId": self.roots[&thread_id].turn_id, "input": [{"type": "text", "text": request.batch.formatted_context}], "clientUserMessageId": correlation})
        } else {
            json!({"threadId": thread_id, "input": [{"type": "text", "text": request.batch.formatted_context}], "clientUserMessageId": correlation})
        };
        let result = (|| -> Result<()> {
            let id = self.router.reserve_internal(ClientRequestOwner::Activation { target: request.target.clone(), attempt: request.attempt.clone() })?;
            let raw = NativeFrame::parse(&json!({"id": 0, "method": if steer {"turn/steer"} else {"turn/start"}, "params": params}).to_string())?.with_id(&id)?;
            if let Err(error) = self.ports.inbox.mark_issuing(&request.target, &request.batch.id) {
                self.router.abandon_before_send(&id)?;
                return Err(error.into());
            }
            self.attempts.insert(thread_id.clone(), NativeAttempt { request: request.clone(), client_id: correlation, steer, turn_id: None, blocked: false });
            self.events.push_back(DriverEvent::Issued { target: request.target.clone(), attempt: request.attempt.clone() });
            self.ports.transport.send(&raw)?;
            Ok(())
        })();
        if let Err(error) = result {
            if self.attempts.contains_key(&thread_id) {
                let _ = self.ports.inbox.record_phase(&request.target, &request.batch.id, ClaimPhase::Uncertain);
                self.fatal = Some(error.to_string());
                return Admission::Accepted { attempt: request.attempt };
            }
            return Admission::Deferred(DeferReason::StateNotSynchronized);
        }
        Admission::Accepted { attempt: request.attempt }
    }
}
