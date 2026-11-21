//! Exact combined hook output confirmation. Mailbox, bootstrap and Task
//! history are separate facts, committed together after the same host flush.
use super::*;
use crate::messaging::inbox::{DeliveryEvidence, DeliveryReceipt, EventId, InboxConsumer};
use crate::messaging::service::{InboxOperation, InboxOutcome, OutputConfirmation, OutputToken, PreparedOutput};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize)]
struct BootstrapRecord {
    actor: Actor,
    pid: u32,
    incarnation: String,
    ready: Option<(String, String)>,
    confirmed: bool,
}
#[derive(Serialize, Deserialize)]
struct HistoryRecord {
    actor: Actor,
    token: OutputToken,
    pid: u32,
    incarnation: String,
    mailbox: Option<OutputConfirmation>,
}
#[derive(Serialize, Deserialize)]
struct MailboxRecord {
    actor: Actor,
    receipt: OutputConfirmation,
    pid: u32,
    incarnation: String,
}
fn mailbox_key(context: &RequestContext, receipt: &OutputConfirmation) -> String {
    let id = match receipt {
        OutputConfirmation::Unmanaged(token) => &token.id,
        OutputConfirmation::Claimed(receipt) => &receipt.batch.0,
    };
    format!("claude_mailbox:{}:{id}", context.database_generation)
}
fn bootstrap_key(context: &RequestContext, receipt: &str) -> String {
    format!("claude_bootstrap:{}:{receipt}", context.database_generation)
}
fn history_key(context: &RequestContext, token: &OutputToken) -> String {
    format!("claude_history_receipt:{}:{}", context.database_generation, token.id)
}
fn frontier_key(context: &RequestContext, actor: &Actor) -> String {
    format!("claude_history_frontier:{}:{}:{}", context.database_generation, actor.participant.canonical_name, actor.participant.incarnation_bits)
}
fn inbox(db: &AgentxDb, context: &RequestContext, operation: InboxOperation) -> ServiceResult<InboxOutcome> {
    match crate::messaging::service::handle(db, context, crate::messaging::service::Request::Inbox(operation))? {
        crate::messaging::service::Response::Inbox(outcome) => Ok(outcome),
        _ => Err(ServiceError::Internal("unexpected Claude mailbox response".into())),
    }
}
fn mailbox(db: &AgentxDb, context: &RequestContext, actor: &Actor) -> ServiceResult<PreparedOutput> {
    let output = match inbox(db, context, InboxOperation::PrepareOutput { participant: actor.participant.clone(), consumer: InboxConsumer::Hook })? {
        InboxOutcome::Output(output) => output.map_err(internal),
        _ => Err(ServiceError::Internal("unexpected Claude prepared output".into())),
    }?;
    if let Some(receipt) = confirmation(&output) {
        let record = MailboxRecord { actor: actor.clone(), receipt: receipt.clone(), pid: context.process.pid, incarnation: context.process.incarnation.clone().unwrap() };
        db.kv_set(&mailbox_key(context, &receipt), Some(&serde_json::to_string(&record).map_err(internal)?)).map_err(internal)?;
    }
    Ok(output)
}
fn messages(output: &PreparedOutput) -> &[crate::db::Message] {
    match output {
        PreparedOutput::Unmanaged { messages, .. } | PreparedOutput::Claimed { messages, .. } => messages,
        PreparedOutput::Deferred => &[],
    }
}
fn confirmation(output: &PreparedOutput) -> Option<OutputConfirmation> {
    if messages(output).is_empty() {
        return None;
    }
    match output {
        PreparedOutput::Unmanaged { receipt, .. } => Some(OutputConfirmation::Unmanaged(receipt.clone())),
        PreparedOutput::Claimed { target, batch, .. } => Some(OutputConfirmation::Claimed(DeliveryReceipt { target: target.clone(), batch: batch.id.clone(), through: batch.window.through, evidence: DeliveryEvidence::HookOutputWritten })),
        PreparedOutput::Deferred => None,
    }
}

pub(super) fn bootstrap(db: &AgentxDb, context: &RequestContext, input: &Actor, session_start: bool) -> ServiceResult<BootstrapOffer> {
    let row = current(db, &input.participant)?;
    let receipt = crate::agents::lifecycle_store::issue_bootstrap_receipt(db, context, &input.participant, context.caller.invocation.activation_target.as_ref())?;
    let env = &context.caller.invocation.environment;
    let ready = if session_start && input.agent_id.is_none() && env.get("AGENTX_LAUNCHED").map(String::as_str) == Some("1") && env.get("AGENTX_PTY_MODE").map(String::as_str) != Some("1") {
        env.get("AGENTX_LAUNCH_BATCH_ID").filter(|batch| !batch.is_empty()).map(|batch| (batch.clone(), env.get("AGENTX_LAUNCHED_BY").cloned().unwrap_or_else(|| "unknown".into())))
    } else {
        None
    };
    let record = BootstrapRecord { actor: input.clone(), pid: context.process.pid, incarnation: context.process.incarnation.clone().unwrap(), ready, confirmed: false };
    db.kv_set(&bootstrap_key(context, &receipt), Some(&serde_json::to_string(&record).map_err(internal)?)).map_err(internal)?;
    let kind = if input.agent_id.is_some() { crate::agents::lifecycle_service::Bootstrap::Subagent { parent: input.root.canonical_name.clone() } } else { crate::agents::lifecycle_service::Bootstrap::Root { tool: row.tool } };
    Ok(BootstrapOffer { receipt, kind })
}

pub(super) fn prepare(db: &AgentxDb, context: &RequestContext, input: &Actor, want_mailbox: bool, want_bootstrap: bool, claim: Option<&StopClaim>) -> ServiceResult<Output> {
    let (root, row) = actor(db, context, input)?;
    if root.status_context == "exit:killed" || row.status_context == "exit:killed" {
        return Ok(Output { mailbox: PreparedOutput::Deferred, bootstrap: None });
    }
    if let Some(claim) = claim
        && super::stop::validate_claim(db, context, claim)?.actor != *input
    {
        return Err(ServiceError::StaleFence);
    }
    let mailbox = if want_mailbox { mailbox(db, context, input)? } else { PreparedOutput::Deferred };
    let bootstrap = if want_bootstrap && row.name_announced == 0 && (input.agent_id.is_none() || !messages(&mailbox).is_empty()) { Some(bootstrap(db, context, input, false)?) } else { None };
    db.update_instance_fields(&row.name, json!({"last_seen":crate::shared::time::now_epoch_i64()}).as_object().unwrap()).map_err(internal)?;
    Ok(Output { mailbox, bootstrap })
}

pub(super) fn task(db: &AgentxDb, context: &RequestContext, input: &Actor, async_launched: bool) -> ServiceResult<Option<TaskOutput>> {
    let (_, row) = actor(db, context, input)?;
    if async_launched {
        return Ok(None);
    }
    if row.status_context == "exit:killed" {
        return Ok(None);
    }
    let mailbox = mailbox(db, context, input)?;
    let mail_ids: HashSet<i64> = messages(&mailbox).iter().filter_map(|message| message.event_id).collect();
    let history_frontier = db.kv_get(&frontier_key(context, input)).map_err(internal)?.map(|raw| raw.parse::<i64>().map_err(internal)).transpose()?.unwrap_or(row.last_event_id);
    let mut query = db.conn().prepare("SELECT name FROM instances WHERE parent_name=? ORDER BY name").map_err(internal)?;
    let names = query.query_map([&row.name], |row| row.get::<_, String>(0)).map_err(internal)?.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)?;
    let mut children = Vec::new();
    for name in names {
        let child = db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        children.push(ChildIdentity { participant: participant(&child), agent_id: child.agent_id });
    }
    let child_names: HashSet<&str> = children.iter().map(|child| child.participant.canonical_name.as_str()).collect();
    let history_limit = crate::shared::MAX_MESSAGES_PER_DELIVERY.saturating_sub(mail_ids.len());
    let mut history_count = 0;
    let mut history_through = history_frontier;
    let mut selected = Vec::new();
    // Root messages are selected only by their canonical frozen claim. Child
    // history is a bounded chronological prefix with an independent frontier.
    // It cannot consume a root window owned by native input or another hook.
    for event in db.get_events_since(row.last_event_id.min(history_frontier), Some("message"), None).map_err(internal)? {
        let Some(id) = event["id"].as_i64() else {
            return Err(ServiceError::Internal("message event without id".into()));
        };
        let data = &event["data"];
        let from = data["from"].as_str().unwrap_or("");
        let frozen_root = mail_ids.contains(&id);
        let for_root = crate::messaging::routing::should_deliver_message(data, &row.name, from).map_err(ServiceError::InvalidInput)?;
        let for_child = child_names.contains(from) || child_names.iter().try_fold(false, |found, name| crate::messaging::routing::should_deliver_message(data, name, from).map(|next| found || next)).map_err(ServiceError::InvalidInput)?;
        let history = id > history_frontier && !for_root && for_child && history_count < history_limit;
        if !frozen_root && !history {
            continue;
        }
        if history {
            history_count += 1;
            history_through = id;
        }
        selected.push(TaskMessage { event: EventId(id), from: from.to_owned(), text: data["text"].as_str().unwrap_or("").to_owned(), timestamp: event["timestamp"].as_str().unwrap_or("").to_owned() });
    }
    if selected.is_empty() {
        return Ok(Some(TaskOutput { mailbox, messages: selected, children, history_receipt: None }));
    }
    let token = OutputToken { id: uuid::Uuid::new_v4().to_string(), participant: input.participant.clone(), through: EventId(history_through) };
    let record = HistoryRecord { actor: input.clone(), token: token.clone(), pid: context.process.pid, incarnation: context.process.incarnation.clone().unwrap(), mailbox: confirmation(&mailbox) };
    db.kv_set(&history_key(context, &token), Some(&serde_json::to_string(&record).map_err(internal)?)).map_err(internal)?;
    Ok(Some(TaskOutput { mailbox, messages: selected, children, history_receipt: Some(token) }))
}

pub(super) fn confirm(db: &AgentxDb, context: &RequestContext, input: &Actor, mailbox: Option<&OutputConfirmation>, bootstrap_receipt: Option<&str>, history: Option<&OutputToken>) -> ServiceResult<()> {
    if mailbox.is_none() && bootstrap_receipt.is_none() && history.is_none() {
        return Err(ServiceError::InvalidInput("exact flushed output receipt required".into()));
    }
    // A SessionStart bootstrap may prove an owner while preserving a foreign
    // live worker binding. Its existing peer-bound receipt allows only that
    // bootstrap confirmation, never subsequent mailbox or actor mutations.
    if mailbox.is_some() || history.is_some() || input.agent_id.is_some() {
        actor(db, context, input)?;
    } else {
        let root = current(db, &input.root)?;
        if input.participant != input.root || root.session_id.as_deref() != Some(input.session_id.as_str()) {
            return Err(ServiceError::StaleFence);
        }
        root_claims(db, context, &root)?;
    }
    if let Some(token) = history {
        let raw = db.kv_get(&history_key(context, token)).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        let record: HistoryRecord = serde_json::from_str(&raw).map_err(internal)?;
        if record.actor != *input
            || record.token != *token
            || record.pid != context.process.pid
            || Some(record.incarnation.as_str()) != context.process.incarnation.as_deref()
            || serde_json::to_value(&record.mailbox).map_err(internal)? != serde_json::to_value(mailbox).map_err(internal)?
        {
            return Err(ServiceError::StaleFence);
        }
        let key = frontier_key(context, input);
        let previous = db.kv_get(&key).map_err(internal)?.map(|raw| raw.parse::<i64>().map_err(internal)).transpose()?.unwrap_or(0);
        db.kv_set(&key, Some(&previous.max(token.through.0).to_string())).map_err(internal)?;
    }
    if let Some(receipt) = mailbox {
        let raw = db.kv_get(&mailbox_key(context, receipt)).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        let record: MailboxRecord = serde_json::from_str(&raw).map_err(internal)?;
        if record.actor != *input
            || record.pid != context.process.pid
            || Some(record.incarnation.as_str()) != context.process.incarnation.as_deref()
            || serde_json::to_value(&record.receipt).map_err(internal)? != serde_json::to_value(receipt).map_err(internal)?
        {
            return Err(ServiceError::StaleFence);
        }
        let fence = match receipt {
            OutputConfirmation::Unmanaged(token) => &token.participant,
            OutputConfirmation::Claimed(receipt) => &receipt.target.participant,
        };
        if *fence != input.participant {
            return Err(ServiceError::StaleFence);
        }
        match inbox(db, context, InboxOperation::ConfirmOutputAndStatus { receipt: receipt.clone(), announce: false })? {
            InboxOutcome::Done(result) => result.map_err(internal)?,
            _ => return Err(ServiceError::Internal("unexpected Claude output confirmation".into())),
        }
    }
    if let Some(receipt) = bootstrap_receipt {
        let key = bootstrap_key(context, receipt);
        let raw = db.kv_get(&key).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        let mut record: BootstrapRecord = serde_json::from_str(&raw).map_err(internal)?;
        if record.actor != *input || record.pid != context.process.pid || Some(record.incarnation.as_str()) != context.process.incarnation.as_deref() {
            return Err(ServiceError::StaleFence);
        }
        crate::agents::lifecycle_store::handle(db, context, crate::agents::lifecycle_service::Request::BootstrapFlushed { participant: input.participant.clone(), receipt: receipt.to_owned() })?;
        if !record.confirmed {
            if let Some((batch, launcher)) = &record.ready {
                let row = current(db, &input.participant)?;
                let ready: bool = db
                    .conn()
                    .query_row("SELECT EXISTS(SELECT 1 FROM events WHERE type='life' AND instance=?1 AND json_extract(data,'$.action')='ready' AND json_extract(data,'$.batch_id')=?2)", params![row.name, batch], |row| row.get(0))
                    .map_err(internal)?;
                if !ready && row.status_context != "exit:killed" {
                    db.log_event("life", &row.name, &json!({"action":"ready","by":launcher,"status":"listening","context":"start","batch_id":batch})).map_err(internal)?;
                    if launcher != "unknown" {
                        db.check_batch_completion(launcher, batch).map_err(internal)?;
                    }
                }
            }
            record.confirmed = true;
            db.kv_set(&key, Some(&serde_json::to_string(&record).map_err(internal)?)).map_err(internal)?;
        }
    }
    Ok(())
}
