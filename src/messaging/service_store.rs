//! Daemon message admission and mailbox adapters. A retained acceptance binds
//! actor and normalized intent to one committed event and frozen routing result.

#[cfg(test)]
mod acceptance_tests;

use super::inbox::*;
use super::routing::{MessageIntent, MessageScope};
use super::service::*;
use super::submission;
use crate::agents::attachments::{AttachmentStore, ParticipantRef, SqliteAttachmentStore};
use crate::db::AgentxDb;
use crate::service::{CallerProcess, RequestContext, ServiceError, ServiceResult};
use crate::shared::{SenderIdentity, SenderKind};
use serde_json::{Value, json};

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

fn sender(db: &AgentxDb, context: &RequestContext, external: Option<&str>) -> ServiceResult<(SenderIdentity, Value)> {
    if let Some(name) = external {
        if name.is_empty() || name.len() > 50 || name.contains(['@', '|', '&', ';', '<', '>', '`', '$', '\'', '"', '\\', '\n', '\r']) {
            return Err(ServiceError::InvalidInput("invalid external sender".into()));
        }
        // A malformed/stale claimed actor must not fall back to anonymous
        // human authority merely because external presentation was requested.
        let actor = match crate::agents::service::resolve_actor(db, context) {
            Ok(actor) => Some(actor),
            Err(ServiceError::IdentityRequired(_))
                if context.caller.invocation.explicit_name.is_none()
                    && context.caller.invocation.process_binding_id.is_none()
                    && context.caller.invocation.activation_target.is_none()
                    && context.caller.invocation.actor_capability.is_none()
                    && context.caller.invocation.provider_session.is_none() =>
            {
                None
            }
            Err(error) => return Err(error),
        };
        if actor.as_ref().is_some_and(|actor| actor.record.parent_name.as_deref().is_some_and(|parent| !parent.is_empty())) {
            return Err(ServiceError::Forbidden("subagents cannot select an external sender".into()));
        }
        let authority = json!({"external":name,"participant":actor.map(|actor| actor.participant)});
        return Ok((SenderIdentity { kind: SenderKind::External, name: name.into(), instance_data: None, session_id: None }, authority));
    }
    let actor = crate::agents::service::resolve_actor(db, context)?;
    let data = Some(json!({"parent_session_id": actor.record.parent_session_id, "session_id": actor.record.session_id}));
    let authority = json!({"participant":actor.participant,"target":context.caller.invocation.activation_target});
    Ok((SenderIdentity { kind: SenderKind::Instance, name: actor.record.name, instance_data: data, session_id: actor.record.session_id }, authority))
}

/// Normalization never allocates IDs or resolves a mutable recipient set.
/// It is safe before acceptance lookup and deterministic across lost replies.
fn normalize(mut input: Submission) -> ServiceResult<Submission> {
    super::routing::validate_message(&input.text).map_err(ServiceError::InvalidInput)?;
    for recipient in &mut input.recipients {
        *recipient = recipient.trim().trim_start_matches('@').to_owned();
    }
    if input.recipients.iter().any(|recipient| recipient.is_empty()) {
        return Err(ServiceError::InvalidInput("empty recipient".into()));
    }
    input.recipients.sort();
    input.recipients.dedup();
    if let Some(reply) = input.envelope.reply_to.as_mut() {
        *reply = reply.trim().to_owned();
        if reply.is_empty() {
            return Err(ServiceError::InvalidInput("empty reply reference".into()));
        }
    }
    if input.envelope.intent == Some(MessageIntent::Ack) && input.envelope.reply_to.is_none() {
        return Err(ServiceError::InvalidInput("intent 'ack' requires a reply reference".into()));
    }
    if let Some(thread) = &input.envelope.thread
        && (thread.is_empty() || thread.len() > 64 || !thread.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'))
    {
        return Err(ServiceError::InvalidInput("invalid thread name".into()));
    }
    if let Some(directory) = &input.envelope.requested_directory
        && (directory.is_empty() || directory.len() > 4096 || directory.chars().any(char::is_control))
    {
        return Err(ServiceError::InvalidInput("invalid requested directory context".into()));
    }
    if let Some(bundle) = input.inline_bundle.as_mut() {
        if input.envelope.bundle_id.is_some() {
            return Err(ServiceError::InvalidInput("inline bundle and bundle ID are mutually exclusive".into()));
        }
        super::bundles::validate_bundle(bundle).map_err(ServiceError::InvalidInput)?;
    }
    Ok(input)
}

fn inherit_reply(db: &AgentxDb, input: &mut Submission) -> ServiceResult<()> {
    if let Some(reply) = &input.envelope.reply_to {
        let local = submission::try_resolve_reply_to_local(db, reply).map_err(internal)?.ok_or_else(|| ServiceError::InvalidInput("reply event not found or ambiguous".into()))?;
        if input.envelope.thread.is_none() {
            input.envelope.thread = submission::try_get_thread_from_event(db, local).map_err(internal)?;
        }
        if input.envelope.intent == Some(MessageIntent::Ack) {
            let intent: Option<String> = db.conn().query_row("SELECT json_extract(data,'$.intent') FROM events WHERE id=?", [local], |row| row.get(0)).map_err(internal)?;
            if matches!(intent.as_deref(), Some("ack" | "inform")) {
                return Err(ServiceError::InvalidInput("cannot acknowledge an ack or informational message".into()));
            }
        }
    }
    Ok(())
}

fn delivery(db: &AgentxDb, identity: &SenderIdentity, input: &Submission) -> ServiceResult<submission::ResolvedDelivery> {
    let explicit_targets = (!input.recipients.is_empty()).then_some(input.recipients.as_slice());
    let routed = submission::resolve_delivery(db, identity, &input.text, Some(&input.envelope), explicit_targets).map_err(ServiceError::InvalidInput)?;
    // Test the same canonical names that routing uses, including bigboss:DEVICE.
    if explicit_targets.is_some() && !routed.effective_mentions.is_empty() && routed.effective_mentions.iter().all(|target| target == &identity.name) {
        return Err(ServiceError::InvalidInput("agents do not receive their own messages".into()));
    }
    Ok(routed)
}

fn bundle_summary(bundle: &Value, id: &str) -> String {
    let refs = &bundle["refs"];
    let list = |field: &str| refs[field].as_array().map(|values| values.iter().map(|value| value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string())).collect::<Vec<_>>().join(", ")).unwrap_or_default();
    let transcript = refs["transcript"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .map(|value| match value.as_object() {
                    Some(_) => format!("{}:{}", value["range"].as_str().unwrap_or(""), value["detail"].as_str().unwrap_or("")),
                    None => value.as_str().unwrap_or("").to_owned(),
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let mut lines = vec![
        format!("[Bundle {id}]"),
        format!("Title: {}", bundle["title"].as_str().unwrap_or("")),
        format!("Description: {}", bundle["description"].as_str().unwrap_or("")),
        "Refs:".into(),
        format!("  events: {}", list("events")),
        format!("  files: {}", list("files")),
        format!("  transcript: {transcript}"),
    ];
    if let Some(extends) = bundle["extends"].as_str() {
        lines.push(format!("Extends: {extends}"));
    }
    lines.extend([String::new(), "View bundle:".into(), format!("  agentx bundle cat {id}")]);
    lines.join("\n")
}

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    match request {
        Request::Bundle(request) => super::bundle_store::handle(db, context, request).map(Response::Bundle),
        Request::Delivery(request) => super::delivery_store::handle(db, context, request).map(Response::Delivery),
        Request::DeliveryReport { event } => transaction(db, || super::delivery_report::report(db, event.0)).map(Response::DeliveryReport),
        Request::Submit(input) => {
            let input = normalize(input)?;
            if context.caller.request_id.0.is_empty() {
                return Err(ServiceError::InvalidInput("stable request ID required".into()));
            }
            let mut effects = Vec::new();
            let acceptance = transaction(db, || {
                let (identity, authority) = sender(db, context, input.external_sender.as_deref())?;
                let actor = authority.to_string();
                let normalized = serde_json::to_string(&input).map_err(internal)?;
                if let Some(stored) = db.message_acceptance(&context.database_generation, &context.caller.request_id.0).map_err(internal)? {
                    if stored.actor != actor || stored.input != normalized {
                        return Err(ServiceError::Conflict("request ID reused with a different message or actor".into()));
                    }
                    let mut outcome: Acceptance = serde_json::from_str(&stored.outcome).map_err(internal)?;
                    outcome.replayed = true;
                    return Ok(outcome);
                }
                let mut input = input.clone();
                inherit_reply(db, &mut input)?;
                let routed = delivery(db, &identity, &input)?;
                if matches!(identity.kind, SenderKind::Instance) && !input.broadcast_confirmed && routed.original_scope == MessageScope::Broadcast && !routed.is_thread_resolved && routed.delivered_to.len() > 3 {
                    return Err(ServiceError::InvalidInput("broadcast requires explicit confirmation; preview recipients first".into()));
                }
                if let Some(mut bundle) = input.inline_bundle.take() {
                    let id = bundle["bundle_id"].as_str().map(str::to_owned).unwrap_or_else(super::bundles::generate_bundle_id);
                    bundle["bundle_id"] = json!(id);
                    bundle["created_by"] = json!(identity.name);
                    let instance = super::validation::get_bundle_instance_name(&identity);
                    let event = db.insert_event_row("bundle", &instance, &bundle, None).map_err(internal)?;
                    effects.push((event, "bundle", instance, bundle.clone()));
                    input.envelope.bundle_id = Some(id.clone());
                    input.text = format!("{}\n\n{}", input.text.trim_end(), bundle_summary(&bundle, &id));
                }
                let persisted = submission::persist_routed_message(db, &identity, &input.text, Some(&input.envelope), routed).map_err(ServiceError::InvalidInput)?;
                let outcome = Acceptance { event: EventId(persisted.event_id), recipients: persisted.recipients, replayed: false };
                db.retain_message_acceptance(&context.database_generation, &context.caller.request_id.0, &actor, &normalized, &serde_json::to_string(&outcome).map_err(internal)?, outcome.event.0).map_err(internal)?;
                effects.push((persisted.event_id, "message", persisted.routing_instance, persisted.data));
                Ok(outcome)
            })?;
            if let Some(active) = crate::service::context::active_observation() {
                use crate::observability::events::{DomainCommitFact, DurableEventRef, MessageAcceptanceRef};
                let scope = active.epoch.daemon.scope;
                let generation = active.epoch.database_generation;
                db.observe_domain(DomainCommitFact::MessageAccepted {
                    acceptance: MessageAcceptanceRef { scope: scope.clone(), database_generation: generation.clone(), request_id: context.caller.request_id.clone() },
                    event: DurableEventRef { scope, database_generation: generation, event: acceptance.event },
                    replayed: acceptance.replayed,
                });
            }
            for (id, kind, instance, data) in effects {
                db.after_event_logged(id, kind, &instance, &data);
            }
            if !acceptance.replayed {
                for recipient in &acceptance.recipients {
                    db.wake_after_commit(recipient, crate::runtime::notify::WakeKind::INSTANCE_LOOPS);
                }
            }
            Ok(Response::Accepted(acceptance))
        }
        Request::Preview(input) => {
            let mut input = normalize(input)?;
            let (identity, _) = sender(db, context, input.external_sender.as_deref())?;
            inherit_reply(db, &mut input)?;
            let routed = delivery(db, &identity, &input)?;
            Ok(Response::Preview(Preview { recipients: routed.delivered_to, broadcast: routed.original_scope == MessageScope::Broadcast, thread_resolved: routed.is_thread_resolved, normalized_envelope: input.envelope }))
        }
        Request::Acceptance { request_id } => {
            let accepted = db
                .message_acceptance(&context.database_generation, &request_id.0)
                .map_err(internal)?
                .map(|stored| {
                    let authority: Value = serde_json::from_str(&stored.actor).map_err(internal)?;
                    let (_, current) = sender(db, context, authority["external"].as_str())?;
                    if current != authority {
                        return Err(ServiceError::Forbidden("acceptance belongs to another actor".into()));
                    }
                    let mut result: Acceptance = serde_json::from_str(&stored.outcome).map_err(internal)?;
                    result.replayed = true;
                    Ok(result)
                })
                .transpose()?;
            Ok(Response::Acceptance(accepted))
        }
        Request::Inbox(operation) => inbox(db, context, operation).map(Response::Inbox),
    }
}

fn validate_participant(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<()> {
    if SqliteAttachmentStore::new(db).participant(&participant.canonical_name).map_err(internal)? != *participant {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}

fn inbox(db: &AgentxDb, context: &RequestContext, operation: InboxOperation) -> ServiceResult<InboxOutcome> {
    let hints = if matches!(&operation, InboxOperation::Prepare { .. } | InboxOperation::PrepareOutput { .. } | InboxOperation::PrepareProduct { .. }) { output_hints(db, context) } else { String::new() };
    let mut inbox = SqlitePendingInbox::with_hints(db, hints);
    // Native input/evidence is authority of the authenticated persistent host,
    // never of a caller that merely read an attachment's visible owner token.
    match &operation {
        InboxOperation::Prepare { consumer: InboxConsumer::ProductInput, .. } => return Err(ServiceError::Forbidden("use dedicated product preparation".into())),
        InboxOperation::Prepare { target, consumer: InboxConsumer::NativeInput, .. } | InboxOperation::SelectDeliveryMode { target, .. } | InboxOperation::PrepareProduct { target, .. } => authorize_host(db, context, target)?,
        InboxOperation::MarkIssuing { target, .. } | InboxOperation::RecordPhase { target, .. } => {
            if inbox.outstanding(target).map_err(internal)?.is_some_and(|claim| matches!(claim.consumer, InboxConsumer::NativeInput | InboxConsumer::ProductInput)) {
                authorize_host(db, context, target)?;
            }
        }
        InboxOperation::Confirm(receipt) if matches!(receipt.evidence, DeliveryEvidence::ProviderInputRecorded { .. } | DeliveryEvidence::ProductInputRecorded(_)) => authorize_host(db, context, &receipt.target)?,
        InboxOperation::ReleaseProvenUnrecorded { target, .. } if inbox.delivery_mode(target).is_ok_and(|mode| mode == super::inbox::DeliveryMode::DedicatedProduct) => authorize_host(db, context, target)?,
        InboxOperation::ReleaseProvenUnrecorded { target, proof, .. } if !matches!(proof, NoInputEvidence::NeverIssued) => {
            authorize_host(db, context, target)?;
            if !inbox.outstanding(target).map_err(internal)?.is_some_and(|claim| claim.consumer == InboxConsumer::NativeInput) {
                return Err(ServiceError::Forbidden("provider no-input evidence cannot release output claims".into()));
            }
        }
        InboxOperation::ReconcilePredecessor { successor, .. } => authorize_host(db, context, successor)?,
        _ => {}
    }
    Ok(match operation {
        InboxOperation::Snapshot { participant } => InboxOutcome::Snapshot(inbox.snapshot(&participant)),
        InboxOperation::HasPendingAfter { participant, after } => {
            let result = transaction(db, || {
                validate_participant(db, &participant)?;
                Ok(db.try_get_unread_messages(&participant.canonical_name).map_err(internal)?.iter().any(|message| message.event_id.is_some_and(|id| id > after.0)))
            });
            InboxOutcome::HasPendingAfter(result.map_err(|error| if error == ServiceError::StaleFence { InboxFailure::StaleParticipant } else { InboxFailure::Unavailable }))
        }
        InboxOperation::Outstanding { target } => InboxOutcome::Outstanding(inbox.outstanding(&target)),
        InboxOperation::DeliveryMode { target } => InboxOutcome::DeliveryMode(inbox.delivery_mode(&target)),
        InboxOperation::SelectDeliveryMode { target, mode } => InboxOutcome::Done(inbox.select_delivery_mode(&target, mode)),
        InboxOperation::Prepare { target, attempt, snapshot, consumer } => InboxOutcome::Prepared(inbox.prepare(&target, &attempt, &snapshot, consumer)),
        InboxOperation::PrepareProduct { target, attempt } => InboxOutcome::ProductPrepared(inbox.prepare_product(&target, &attempt, &context.database_generation)),
        InboxOperation::MarkIssuing { target, batch } => InboxOutcome::Done(inbox.mark_issuing(&target, &batch)),
        InboxOperation::RecordPhase { target, batch, phase } => InboxOutcome::Done(inbox.record_phase(&target, &batch, phase)),
        InboxOperation::ReleaseProvenUnrecorded { target, batch, proof } => InboxOutcome::Done(inbox.release_proven_unrecorded(&target, &batch, proof)),
        InboxOperation::Confirm(receipt) => InboxOutcome::Confirmed(inbox.confirm(receipt)),
        InboxOperation::ReconcilePredecessor { predecessor, successor, proof } => InboxOutcome::Recovered(inbox.reconcile_predecessor(&predecessor, &successor, proof)),
        InboxOperation::DeliveredThrough { participant } => InboxOutcome::DeliveredThrough(inbox.delivered_through(&participant)),
        InboxOperation::ConfirmOutput { receipt, announce } => {
            let result = transaction(db, || {
                validate_participant(db, &receipt.participant)?;
                let key = format!("output_receipt:{}", receipt.id);
                let stored = db.kv_get(&key).map_err(internal)?.ok_or_else(|| ServiceError::Forbidden("unknown output receipt".into()))?;
                let expected: OutputToken = serde_json::from_str(&stored).map_err(internal)?;
                if expected != receipt {
                    return Err(ServiceError::StaleFence);
                }
                let advanced = db.ack_hook_delivery(&receipt.participant.canonical_name, receipt.through.0, announce, Some(f64::from_bits(receipt.participant.incarnation_bits))).map_err(internal)?;
                if !advanced {
                    return Err(ServiceError::StaleFence);
                }
                release_legacy_codex_output(db, &receipt)?;
                observe_output(db, &receipt.participant, receipt.through);
                Ok(())
            });
            InboxOutcome::Done(result.map_err(|error| if error == ServiceError::StaleFence { InboxFailure::StaleParticipant } else { InboxFailure::InvalidEvidence }))
        }
        InboxOperation::ConfirmOutputAndStatus { receipt, announce } => {
            let result = confirm_output_and_status(db, &mut inbox, receipt, announce);
            InboxOutcome::Done(result.map_err(|error| if error == ServiceError::StaleFence { InboxFailure::StaleParticipant } else { InboxFailure::InvalidEvidence }))
        }
        InboxOperation::PrepareOutput { participant, consumer } => InboxOutcome::Output(prepare_output(db, &mut inbox, &participant, consumer, &context.process, true)),
    })
}

fn output_hints(db: &AgentxDb, context: &RequestContext) -> String {
    crate::config::AgentxConfig::load_for_scope(db.path().parent().unwrap_or(std::path::Path::new(".")), &context.caller.invocation.environment).hints
}

/// Hook callers only need a bounded delivery window, not exact total backlog.
/// An empty or concurrently drained mailbox never creates a receipt.
pub(crate) fn prepare_hook_output(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef) -> ServiceResult<Option<HookOutput>> {
    validate_participant(db, participant)?;
    if db.unread_messages_up_to(&participant.canonical_name, 1).map_err(internal)?.is_empty() {
        return Ok(None);
    }
    let mut inbox = SqlitePendingInbox::with_hints(db, output_hints(db, context));
    match prepare_output(db, &mut inbox, participant, InboxConsumer::Hook, &context.process, false) {
        Ok(PreparedOutput::Deferred) => Ok(None),
        Ok(PreparedOutput::Unmanaged { receipt, formatted_context, .. }) => Ok(Some(HookOutput { formatted_context, confirmation: OutputConfirmation::Unmanaged(receipt) })),
        Ok(PreparedOutput::Claimed { target, batch, .. }) => {
            Ok(Some(HookOutput { formatted_context: batch.formatted_context, confirmation: OutputConfirmation::Claimed(DeliveryReceipt { target, batch: batch.id, through: batch.window.through, evidence: DeliveryEvidence::HookOutputWritten }) }))
        }
        Err(InboxFailure::StaleParticipant) => Err(ServiceError::StaleFence),
        Err(error) => Err(ServiceError::Unavailable(error.to_string())),
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct OutputStatusMeta {
    sender_display: String,
    msg_ts: String,
}

fn store_output_status_meta(db: &AgentxDb, id: &str, messages: &[crate::db::Message]) -> Result<(), InboxFailure> {
    let Some(first) = messages.first() else {
        return Ok(());
    };
    let sender_display = first.from.clone();
    let meta = OutputStatusMeta { sender_display, msg_ts: messages.last().and_then(|message| message.timestamp.clone()).unwrap_or_default() };
    let stored = serde_json::to_string(&meta).map_err(|_| InboxFailure::Unavailable)?;
    db.kv_set(&format!("output_status_meta:{id}"), Some(&stored)).map_err(|_| InboxFailure::Unavailable)
}

/// Output evidence and its delivery status commit together. A replay validates
/// the same receipt but cannot overwrite status from a later activity callback.
fn confirm_output_and_status(db: &AgentxDb, inbox: &mut SqlitePendingInbox<'_>, receipt: OutputConfirmation, announce: bool) -> ServiceResult<()> {
    transaction(db, || {
        let (participant, through, id) = match &receipt {
            OutputConfirmation::Unmanaged(token) => {
                validate_participant(db, &token.participant)?;
                let stored = db.kv_get(&format!("output_receipt:{}", token.id)).map_err(internal)?.ok_or_else(|| ServiceError::Forbidden("unknown output receipt".into()))?;
                let expected: OutputToken = serde_json::from_str(&stored).map_err(internal)?;
                if expected != *token {
                    return Err(ServiceError::StaleFence);
                }
                (&token.participant, token.through, &token.id)
            }
            OutputConfirmation::Claimed(claim) => {
                if !matches!(claim.evidence, DeliveryEvidence::HookOutputWritten | DeliveryEvidence::ToolOutputWritten) {
                    return Err(ServiceError::Forbidden("output flush evidence required".into()));
                }
                match inbox.confirm(claim.clone()).map_err(internal)? {
                    Confirmation::Advanced | Confirmation::AlreadyConfirmed => {}
                    _ => return Err(ServiceError::StaleFence),
                }
                (&claim.target.participant, claim.through, &claim.batch.0)
            }
        };
        let name = &participant.canonical_name;
        let before = db.get_instance_full(name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        if !db.ack_hook_delivery(name, through.0, announce, Some(f64::from_bits(participant.incarnation_bits))).map_err(internal)? {
            return Err(ServiceError::StaleFence);
        }
        if let OutputConfirmation::Unmanaged(token) = &receipt {
            release_legacy_codex_output(db, token)?;
        }
        let key = format!("output_status_applied:{id}");
        if db.kv_get(&key).map_err(internal)?.is_some() {
            if matches!(&receipt, OutputConfirmation::Unmanaged(_)) {
                observe_output(db, participant, through);
            }
            return Ok(());
        }
        let meta = db.kv_get(&format!("output_status_meta:{id}")).map_err(internal)?.ok_or_else(|| ServiceError::Forbidden("output status metadata unavailable".into()))?;
        let meta: OutputStatusMeta = serde_json::from_str(&meta).map_err(internal)?;
        if before.last_event_id <= through.0 && before.status_context != "exit:killed" {
            crate::agents::lifecycle::try_set_status(db, name, "active", &format!("deliver:{}", meta.sender_display), crate::agents::lifecycle::StatusUpdate { msg_ts: &meta.msg_ts, ..Default::default() }).map_err(internal)?;
        }
        db.kv_set(&key, Some("1")).map_err(internal)?;
        if matches!(&receipt, OutputConfirmation::Unmanaged(_)) {
            observe_output(db, participant, through);
        }
        Ok(())
    })
}

fn observe_output(db: &AgentxDb, participant: &ParticipantRef, through: EventId) {
    if let Some(active) = crate::service::context::active_observation() {
        use crate::observability::events::{DomainCommitFact, OutputEvidenceRef, OutputObservationId};
        db.observe_domain(DomainCommitFact::OutputConfirmationStored {
            output: OutputEvidenceRef { scope: active.epoch.daemon.scope, database_generation: active.epoch.database_generation, participant: participant.clone(), observation: OutputObservationId::new() },
            through,
        });
    }
}

fn authorize_host(db: &AgentxDb, context: &RequestContext, target: &crate::agents::attachments::ActivationTarget) -> ServiceResult<()> {
    let attachment = SqliteAttachmentStore::new(db).current_for_participant(&target.participant).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    if attachment.target != *target {
        return Err(ServiceError::StaleFence);
    }
    if !crate::agents::service::attachment_owned_by_process(db, target, &attachment.owner, &context.process)? {
        return Err(ServiceError::Forbidden("native input belongs to another host process".into()));
    }
    Ok(())
}

/// Legacy Codex hooks and explicit reads flush from the same process that
/// prepares output. Hold one receipt per mailbox until it confirms or that
/// process exits. Recovery retries unread mail; process exit does not ack it.
#[derive(serde::Serialize, serde::Deserialize)]
struct LegacyCodexOutput {
    receipt: OutputToken,
    pid: u32,
    incarnation: Option<String>,
}

impl LegacyCodexOutput {
    fn key(participant: &ParticipantRef) -> String {
        format!("legacy_codex_output:{}", participant.canonical_name)
    }

    fn load(db: &AgentxDb, key: &str) -> anyhow::Result<Option<Self>> {
        db.kv_get(key)?.map(|stored| serde_json::from_str(&stored).map_err(Into::into)).transpose()
    }

    fn owner_ended(&self) -> bool {
        crate::sys::process::has_exited(self.pid) || self.incarnation.as_ref().zip(crate::sys::process::identity(self.pid).as_ref()).is_some_and(|(expected, current)| expected != current)
    }
}

fn release_legacy_codex_output(db: &AgentxDb, receipt: &OutputToken) -> ServiceResult<()> {
    let key = LegacyCodexOutput::key(&receipt.participant);
    if LegacyCodexOutput::load(db, &key).map_err(internal)?.is_some_and(|reservation| reservation.receipt == *receipt) {
        db.kv_set(&key, None).map_err(internal)?;
    }
    Ok(())
}

/// Freeze the output window, receipt and any pagination metadata together.
fn prepare_output(db: &AgentxDb, inbox: &mut SqlitePendingInbox<'_>, participant: &ParticipantRef, consumer: InboxConsumer, process: &CallerProcess, exact_backlog: bool) -> Result<PreparedOutput, InboxFailure> {
    db.with_write_scope(|| {
        let prepare = || -> Result<PreparedOutput, InboxFailure> {
            let row = db.get_instance_full(&participant.canonical_name).map_err(|_| InboxFailure::Unavailable)?.ok_or(InboxFailure::StaleParticipant)?;
            if row.created_at.to_bits() != participant.incarnation_bits {
                return Err(InboxFailure::StaleParticipant);
            }
            let mut unread = if exact_backlog { db.try_get_unread_messages(&participant.canonical_name).map_err(|_| InboxFailure::Unavailable)? } else { Vec::new() };
            let backlog_end = unread.last().and_then(|message| message.event_id).map(EventId);
            let unread_count = unread.len();
            let reserves_legacy_output = row.tool == "codex" && matches!(consumer, InboxConsumer::Hook | InboxConsumer::ExplicitListen);
            match inbox.prepare_output(&participant.canonical_name, consumer)? {
                OutputPreparation::Unmanaged => {
                    let reservation_key = reserves_legacy_output.then(|| LegacyCodexOutput::key(participant));
                    if let Some(key) = &reservation_key
                        && LegacyCodexOutput::load(db, key).map_err(|_| InboxFailure::Unavailable)?.is_some_and(|held| held.receipt.participant == *participant && held.receipt.through.0 > row.last_event_id && !held.owner_ended())
                    {
                        return Ok(PreparedOutput::Deferred);
                    }
                    if exact_backlog {
                        unread.truncate(crate::shared::MAX_MESSAGES_PER_DELIVERY);
                    } else {
                        unread = db.unread_messages_up_to(&participant.canonical_name, crate::shared::MAX_MESSAGES_PER_DELIVERY).map_err(|_| InboxFailure::Unavailable)?;
                        if unread.is_empty() {
                            return Ok(PreparedOutput::Deferred);
                        }
                        // Bound transcript output while retaining a full first
                        // message. The receipt covers only the emitted prefix.
                        let mut body_bytes = 0usize;
                        let count = unread
                            .iter()
                            .enumerate()
                            .take_while(|(index, message)| {
                                body_bytes = body_bytes.saturating_add(message.text.len());
                                *index == 0 || body_bytes <= crate::shared::MAX_MESSAGE_SIZE
                            })
                            .count();
                        unread.truncate(count);
                    }
                    let remaining = unread_count.saturating_sub(unread.len());
                    let through = unread.last().and_then(|message| message.event_id).unwrap_or(inbox.delivered_through(participant)?.0);
                    let receipt = OutputToken { id: uuid::Uuid::new_v4().to_string(), participant: participant.clone(), through: EventId(through) };
                    let stored = serde_json::to_string(&receipt).map_err(|_| InboxFailure::Unavailable)?;
                    db.kv_set(&format!("output_receipt:{}", receipt.id), Some(&stored)).map_err(|_| InboxFailure::Unavailable)?;
                    let formatted_context = inbox.format_output(participant, &unread)?;
                    if !exact_backlog {
                        check_hook_output_size(&formatted_context)?;
                    }
                    store_output_status_meta(db, &receipt.id, &unread)?;
                    if let Some(key) = reservation_key.filter(|_| !unread.is_empty()) {
                        let reservation = LegacyCodexOutput { receipt: receipt.clone(), pid: process.pid, incarnation: process.incarnation.clone() };
                        let stored = serde_json::to_string(&reservation).map_err(|_| InboxFailure::Unavailable)?;
                        db.kv_set(&key, Some(&stored)).map_err(|_| InboxFailure::Unavailable)?;
                    }
                    Ok(PreparedOutput::Unmanaged { messages: unread, receipt, remaining, backlog_end, formatted_context })
                }
                OutputPreparation::Deferred => Ok(PreparedOutput::Deferred),
                OutputPreparation::Claimed { claim, messages, .. } => {
                    let remaining = unread_count.saturating_sub(messages.len());
                    let batch = inbox.outstanding(&claim.target)?.ok_or(InboxFailure::InvalidEvidence)?.batch;
                    if !exact_backlog {
                        check_hook_output_size(&batch.formatted_context)?;
                    }
                    inbox.mark_issuing(&claim.target, &claim.batch)?;
                    store_output_status_meta(db, &claim.batch.0, &messages)?;
                    Ok(PreparedOutput::Claimed { target: claim.target, batch, messages, remaining, backlog_end })
                }
            }
        };
        prepare().map_err(anyhow::Error::new)
    })
    .map_err(|error| error.downcast_ref::<InboxFailure>().cloned().unwrap_or(InboxFailure::Unavailable))
}

fn check_hook_output_size(text: &str) -> Result<(), InboxFailure> {
    // Leave room for JSON escaping (up to six bytes per input byte) and the
    // small receipt/envelope. Abort preparation before committing an unsendable claim.
    if text.len() > crate::shared::MAX_MESSAGE_SIZE + 64 * 1024 {
        return Err(InboxFailure::Unavailable);
    }
    Ok(())
}
