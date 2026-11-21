//! Daemon-only Copilot offer/queue policy. One transaction excludes ordinary
//! hook output from an extension-owned batch and retains its exact receipt.
//! SDK calls, hook stdout and endpoint transport stay in the originating host.
use serde::{Deserialize, Serialize};
use serde_json::json;

#[cfg(test)]
mod acceptance_tests;
use super::attachments::ParticipantRef;
use super::copilot_service::*;
use crate::db::{AgentxDb, InstanceRow};
use crate::messaging::inbox::{DeliveryEvidence, DeliveryReceipt, EventId, InboxConsumer};
use crate::messaging::service::{InboxOperation, InboxOutcome, OutputConfirmation, PreparedOutput};
use crate::service::{RequestContext, ServiceError, ServiceResult};

const OFFER_LEASE: i64 = 60;
const QUEUED_LEASE: i64 = 90;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ExtensionOutput {
    offer: Offer,
    receipt: OutputConfirmation,
    offered_at: i64,
    queued_at: Option<i64>,
    confirmed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum OwnedOutput {
    Extension(ExtensionOutput),
    Hook { participant: ParticipantRef, output: PreparedOutput, prepared_at: i64, session_id: Option<String> },
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn participant(row: &InstanceRow) -> ParticipantRef {
    ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }
}
fn key(participant: &ParticipantRef) -> String {
    format!("copilot_delivery:{}:{}", participant.canonical_name, participant.incarnation_bits)
}
fn load(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<Option<OwnedOutput>> {
    db.kv_get(&key(participant)).map_err(internal)?.map(|raw| serde_json::from_str(&raw).map_err(internal)).transpose()
}
fn save(db: &AgentxDb, participant: &ParticipantRef, owned: &OwnedOutput) -> ServiceResult<()> {
    db.kv_set(&key(participant), Some(&serde_json::to_string(owned).map_err(internal)?)).map_err(internal)
}
fn clear(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<()> {
    db.kv_set(&key(participant), None).map_err(internal)
}

/// Copilot is process-bound, even before its lazily created first session.
/// Reuse the canonical observation validator for managed attachment fences.
fn actor(db: &AgentxDb, context: &RequestContext, expected: &ParticipantRef) -> ServiceResult<InstanceRow> {
    if context.process.pid == 0 || context.process.incarnation.as_deref().is_none_or(str::is_empty) {
        return Err(ServiceError::Forbidden("authenticated Copilot process required".into()));
    }
    let binding = context.caller.invocation.process_binding_id.as_deref().filter(|binding| !binding.is_empty()).ok_or_else(|| ServiceError::IdentityRequired("Copilot process binding required".into()))?;
    if db.get_process_binding(binding).map_err(internal)?.as_deref() != Some(expected.canonical_name.as_str()) {
        return Err(ServiceError::StaleFence);
    }
    let row = super::hook_store::observation_actor(db, context, expected, None)?;
    if row.tool != "copilot" || row.origin_device_id.is_some() {
        return Err(ServiceError::StaleFence);
    }
    Ok(row)
}

fn foreign_session(row: &InstanceRow, incoming: Option<&str>) -> bool {
    matches!((row.session_id.as_deref(), incoming), (Some(bound), Some(incoming)) if !incoming.is_empty() && bound != incoming)
}

/// Runtime endpoint registration shares the same process/session admission.
/// A foreground offer pins its extension even before lazy SessionStart binds.
pub(crate) fn validate_endpoint(db: &AgentxDb, context: &RequestContext, expected: &ParticipantRef, session: &str) -> ServiceResult<()> {
    let row = actor(db, context, expected)?;
    if session.is_empty() || foreign_session(&row, Some(session)) {
        return Err(ServiceError::StaleFence);
    }
    if let Some(OwnedOutput::Extension(extension)) = active_owned(db, &row)?
        && extension.offer.receipt.session_id != session
    {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}

fn through(output: &PreparedOutput) -> Option<EventId> {
    match output {
        PreparedOutput::Unmanaged { messages, receipt, .. } if !messages.is_empty() => Some(receipt.through),
        PreparedOutput::Claimed { batch, messages, .. } if !messages.is_empty() => Some(batch.window.through),
        _ => None,
    }
}
fn managed(output: &PreparedOutput) -> bool {
    matches!(output, PreparedOutput::Claimed { .. })
}
fn output_id(output: &PreparedOutput) -> Option<&str> {
    match output {
        PreparedOutput::Unmanaged { receipt, .. } => Some(&receipt.id),
        PreparedOutput::Claimed { batch, .. } => Some(&batch.id.0),
        PreparedOutput::Deferred => None,
    }
}
fn output_confirmation(output: &PreparedOutput) -> Option<OutputConfirmation> {
    match output {
        PreparedOutput::Unmanaged { receipt, .. } => Some(OutputConfirmation::Unmanaged(receipt.clone())),
        PreparedOutput::Claimed { target, batch, .. } => Some(OutputConfirmation::Claimed(DeliveryReceipt { target: target.clone(), batch: batch.id.clone(), through: batch.window.through, evidence: DeliveryEvidence::HookOutputWritten })),
        PreparedOutput::Deferred => None,
    }
}
fn inbox(db: &AgentxDb, context: &RequestContext, operation: InboxOperation) -> ServiceResult<InboxOutcome> {
    match crate::messaging::service::handle(db, context, crate::messaging::service::Request::Inbox(operation))? {
        crate::messaging::service::Response::Inbox(outcome) => Ok(outcome),
        _ => Err(ServiceError::Internal("unexpected Copilot mailbox response".into())),
    }
}
fn prepare(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef) -> ServiceResult<PreparedOutput> {
    match inbox(db, context, InboxOperation::PrepareOutput { participant: participant.clone(), consumer: InboxConsumer::Hook })? {
        InboxOutcome::Output(output) => output.map_err(internal),
        _ => Err(ServiceError::Internal("unexpected Copilot prepared output".into())),
    }
}

/// An unmanaged lease retains the existing best-effort redelivery policy.
/// Managed output was already Issuing: neither time nor a send report releases
/// it. Its canonical ledger and full receipt remain available for correlation.
fn active_owned(db: &AgentxDb, row: &InstanceRow) -> ServiceResult<Option<OwnedOutput>> {
    let participant = participant(row);
    let Some(owned) = load(db, &participant)? else {
        return Ok(None);
    };
    let now = crate::shared::time::now_epoch_i64();
    let active = match &owned {
        OwnedOutput::Extension(extension) => {
            if extension.offer.receipt.participant != participant {
                return Err(ServiceError::StaleFence);
            }
            extension.offer.receipt.through.0 > row.last_event_id
                && !extension.confirmed
                && (matches!(&extension.receipt, OutputConfirmation::Claimed(_))
                    || (!foreign_session(row, Some(&extension.offer.receipt.session_id))
                        && match extension.queued_at {
                            Some(queued) => now.saturating_sub(queued) < QUEUED_LEASE,
                            None => now.saturating_sub(extension.offered_at) < OFFER_LEASE,
                        }))
        }
        OwnedOutput::Hook { participant: owner, output, prepared_at, session_id } => {
            if owner != &participant {
                return Err(ServiceError::StaleFence);
            }
            through(output).is_some_and(|through| through.0 > row.last_event_id) && (managed(output) || (!foreign_session(row, session_id.as_deref()) && now.saturating_sub(*prepared_at) < OFFER_LEASE))
        }
    };
    Ok(active.then_some(owned))
}

/// Pre-cutover records have no exact output authority. Respect their remaining
/// lease without accepting their cursor as evidence. Any issued managed claim
/// is independently preserved by the canonical inbox, including after expiry.
fn legacy_inflight(db: &AgentxDb, row: &InstanceRow) -> ServiceResult<Option<EventId>> {
    #[derive(Deserialize)]
    struct Legacy {
        last_event_id: i64,
        sent_at: i64,
        confirmed_at: Option<i64>,
        session_id: Option<String>,
    }
    let Some(raw) = db.kv_get(&format!("copilot_ext_inflight:{}", row.name)).map_err(internal)? else {
        return Ok(None);
    };
    let legacy: Legacy = serde_json::from_str(&raw).map_err(internal)?;
    let now = crate::shared::time::now_epoch_i64();
    let live = legacy.last_event_id > row.last_event_id
        && !foreign_session(row, legacy.session_id.as_deref())
        && match legacy.confirmed_at {
            Some(queued) => now.saturating_sub(queued) < QUEUED_LEASE,
            None => now.saturating_sub(legacy.sent_at) < OFFER_LEASE,
        };
    Ok(live.then_some(EventId(legacy.last_event_id)))
}

fn poll(db: &AgentxDb, context: &RequestContext, row: &InstanceRow, session_id: &str, idle: bool) -> ServiceResult<Response> {
    if session_id.is_empty() {
        return Err(ServiceError::InvalidInput("Copilot extension session required".into()));
    }
    let participant = participant(row);
    let mut result = PollResult { participant: participant.clone(), offer: None, inflight: None, session_mismatch: foreign_session(row, Some(session_id)) };
    if result.session_mismatch {
        return Ok(Response::Poll(result));
    }
    let owned = active_owned(db, row)?;
    if let Some(OwnedOutput::Extension(extension)) = &owned
        && extension.offer.receipt.session_id != session_id
    {
        result.session_mismatch = true;
        return Ok(Response::Poll(result));
    }
    if idle && row.status != crate::shared::ST_LISTENING {
        super::lifecycle::try_set_status(db, &row.name, crate::shared::ST_LISTENING, "", Default::default()).map_err(internal)?;
    }
    if !idle && row.status != crate::shared::ST_LISTENING {
        return Ok(Response::Poll(result));
    }
    if let Some(owned) = owned {
        result.inflight = match owned {
            OwnedOutput::Extension(extension) => Some(extension.offer.receipt.through),
            OwnedOutput::Hook { output, .. } => through(&output),
        };
        return Ok(Response::Poll(result));
    }
    if let Some(inflight) = legacy_inflight(db, row)? {
        result.inflight = Some(inflight);
        return Ok(Response::Poll(result));
    }
    let output = prepare(db, context, &participant)?;
    let Some(through) = through(&output) else {
        return Ok(Response::Poll(result));
    };
    let (text, count) = match &output {
        PreparedOutput::Unmanaged { formatted_context, messages, .. } => (formatted_context.clone(), messages.len()),
        PreparedOutput::Claimed { batch, messages, .. } => (batch.formatted_context.clone(), messages.len()),
        PreparedOutput::Deferred => unreachable!(),
    };
    let offer = Offer { receipt: OfferToken { id: uuid::Uuid::new_v4().to_string(), participant: participant.clone(), session_id: session_id.to_owned(), through }, text, count };
    let extension = ExtensionOutput {
        offer: offer.clone(),
        receipt: output_confirmation(&output).ok_or_else(|| ServiceError::Internal("Copilot output has no receipt".into()))?,
        offered_at: crate::shared::time::now_epoch_i64(),
        queued_at: None,
        confirmed: false,
    };
    save(db, &participant, &OwnedOutput::Extension(extension))?;
    result.offer = Some(offer);
    Ok(Response::Poll(result))
}

fn sent(db: &AgentxDb, row: &InstanceRow, token: &OfferToken, observation: SendObservation) -> ServiceResult<Response> {
    if foreign_session(row, Some(&token.session_id)) {
        return Err(ServiceError::StaleFence);
    }
    let Some(OwnedOutput::Extension(mut extension)) = load(db, &token.participant)? else {
        return Err(ServiceError::StaleFence);
    };
    if extension.offer.receipt != *token {
        return Err(ServiceError::StaleFence);
    }
    if extension.confirmed || token.through.0 <= row.last_event_id {
        return Ok(Response::Sent);
    }
    match observation {
        SendObservation::Accepted => extension.queued_at = Some(crate::shared::time::now_epoch_i64()),
        SendObservation::Failed if matches!(&extension.receipt, OutputConfirmation::Unmanaged(_)) => {
            clear(db, &token.participant)?;
            return Ok(Response::Sent);
        }
        SendObservation::Failed | SendObservation::Unknown => {}
    }
    save(db, &token.participant, &OwnedOutput::Extension(extension))?;
    Ok(Response::Sent)
}

fn observe_prompt(db: &AgentxDb, context: &RequestContext, row: &InstanceRow, session: Option<&str>, prompt: &str) -> ServiceResult<Response> {
    if foreign_session(row, session) {
        return Ok(Response::PromptObserved { matched: false, ignored: true });
    }
    let participant = participant(row);
    if let Some(OwnedOutput::Extension(mut extension)) = load(db, &participant)? {
        // Prompt observation does not use the lease: a late exact prompt is
        // still positive evidence. Its actual Provider session must match.
        if session == Some(extension.offer.receipt.session_id.as_str()) && extension.offer.text.trim() == prompt.trim() {
            if !extension.confirmed {
                match inbox(db, context, InboxOperation::ConfirmOutputAndStatus { receipt: extension.receipt.clone(), announce: false })? {
                    InboxOutcome::Done(result) => result.map_err(internal)?,
                    _ => return Err(ServiceError::Internal("unexpected Copilot prompt confirmation".into())),
                }
                extension.confirmed = true;
                save(db, &participant, &OwnedOutput::Extension(extension))?;
            }
            return Ok(Response::PromptObserved { matched: true, ignored: false });
        }
    }
    let context = if prompt.trim() == "<agentx>" { "trigger" } else { "prompt" };
    super::lifecycle::try_set_status(db, &row.name, crate::shared::ST_ACTIVE, context, Default::default()).map_err(internal)?;
    Ok(Response::PromptObserved { matched: false, ignored: false })
}

fn pending(db: &AgentxDb, context: &RequestContext, row: &InstanceRow, session: Option<&str>, hook: PendingHook) -> ServiceResult<Response> {
    if foreign_session(row, session) {
        return Ok(Response::Pending(PreparedOutput::Deferred));
    }
    let participant = participant(row);
    let output = match active_owned(db, row)? {
        Some(OwnedOutput::Extension(_)) => PreparedOutput::Deferred,
        Some(OwnedOutput::Hook { output, .. }) => {
            if managed(&output) {
                PreparedOutput::Deferred
            } else {
                output
            }
        }
        None if legacy_inflight(db, row)?.is_some() => PreparedOutput::Deferred,
        None => {
            let output = prepare(db, context, &participant)?;
            if through(&output).is_some() {
                save(db, &participant, &OwnedOutput::Hook { participant: participant.clone(), output: output.clone(), prepared_at: crate::shared::time::now_epoch_i64(), session_id: row.session_id.clone() })?;
            }
            output
        }
    };
    if matches!(hook, PendingHook::Stop) && through(&output).is_none() {
        super::lifecycle::try_set_status(db, &row.name, crate::shared::ST_LISTENING, "", Default::default()).map_err(internal)?;
        db.wake_after_commit(&row.name, crate::runtime::notify::WakeKind::INSTANCE_LOOPS);
    }
    Ok(Response::Pending(output))
}

/// Native declarations and this handler are released together with the actual
/// hook/extension host; only the agents aggregate publishes module dispatch.
pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    db.with_write_scope(|| {
        let expected = match &request {
            Request::Poll { participant, .. } | Request::ObservePrompt { participant, .. } | Request::Pending { participant, .. } => participant,
            Request::Sent { receipt, .. } => &receipt.participant,
        };
        let row = actor(db, context, expected)?;
        if let Request::Sent { receipt, .. } = &request
            && (receipt.id.is_empty() || receipt.session_id.is_empty() || foreign_session(&row, Some(&receipt.session_id)))
        {
            return Err(ServiceError::StaleFence.into());
        }
        let response = super::service_store::retained(db, context, &json!({"copilot": request}), || match &request {
            Request::Poll { session_id, idle, .. } => poll(db, context, &row, session_id, *idle),
            Request::Sent { receipt, observation } => sent(db, &row, receipt, *observation),
            Request::ObservePrompt { session_id, prompt, .. } => observe_prompt(db, context, &row, session_id.as_deref(), prompt),
            Request::Pending { session_id, hook, .. } => pending(db, context, &row, session_id.as_deref(), *hook),
        })?;
        // A retained old response never reoffers text after its owner changed
        // or after a later hook already confirmed that delivery frontier.
        let response = match response {
            Response::Poll(mut result) => {
                if let Request::Poll { session_id, .. } = &request
                    && foreign_session(&row, Some(session_id))
                {
                    result.session_mismatch = true;
                    result.offer = None;
                    result.inflight = None;
                }
                if let Some(offer) = &result.offer {
                    let Some(OwnedOutput::Extension(extension)) = load(db, expected)? else {
                        return Err(ServiceError::StaleFence.into());
                    };
                    if extension.offer.receipt != offer.receipt {
                        return Err(ServiceError::StaleFence.into());
                    }
                    if extension.confirmed || extension.queued_at.is_some() || offer.receipt.through.0 <= row.last_event_id {
                        result.offer = None;
                    }
                }
                Response::Poll(result)
            }
            Response::Pending(_) if matches!(&request, Request::Pending {session_id,..} if foreign_session(&row,session_id.as_deref())) => Response::Pending(PreparedOutput::Deferred),
            Response::Pending(output) if through(&output).is_some() => {
                if through(&output).is_some_and(|through| through.0 <= row.last_event_id) {
                    Response::Pending(PreparedOutput::Deferred)
                } else if let Some(OwnedOutput::Hook { output: current, .. }) = load(db, expected)? {
                    if output_id(&current) != output_id(&output) {
                        return Err(ServiceError::StaleFence.into());
                    }
                    Response::Pending(output)
                } else {
                    return Err(ServiceError::StaleFence.into());
                }
            }
            response => response,
        };
        Ok(response)
    })
    .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}
