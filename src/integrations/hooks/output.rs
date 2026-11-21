//! Host-side exact output preparation/confirmation over native mailbox ports.
//! Preparation is distinct from writing and flushing Provider output. A failed
//! or lost write never confirms the retained token or clears its uncertainty.

use anyhow::{Result, bail};

use crate::agents::attachments::ParticipantRef;
use crate::messaging::inbox::{Confirmation, DeliveryEvidence, DeliveryReceipt, InboxConsumer, OutputClaim};
use crate::messaging::service::{InboxOperation, InboxOutcome, OutputConfirmation, OutputToken, PreparedOutput, Request as MessageRequest, Response as MessageResponse};
use crate::service::client::Client;
use crate::service::{Request, Response};

use super::DeliveryAck;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(crate) enum OutputReceipt {
    Unmanaged(OutputToken),
    Claimed(DeliveryReceipt),
}

pub(crate) struct Prepared {
    pub formatted: String,
    pub messages: Vec<serde_json::Value>,
    pub ack: DeliveryAck,
    pub receipt: OutputReceipt,
}

fn inbox(client: &Client, operation: InboxOperation) -> Result<InboxOutcome> {
    match client.call(Request::Messages(MessageRequest::Inbox(operation)))? {
        Response::Messages(MessageResponse::Inbox(outcome)) => Ok(outcome),
        _ => bail!("unexpected output mailbox response"),
    }
}

/// Freeze the precise mailbox window before exposing any output to a Provider.
/// Native attached mailboxes also retain the complete formatted batch text.
pub(crate) fn prepare(client: &Client, participant: &ParticipantRef) -> Result<Option<Prepared>> {
    let prepared = match inbox(client, InboxOperation::PrepareOutput { participant: participant.clone(), consumer: InboxConsumer::Hook })? {
        InboxOutcome::Output(output) => output?,
        _ => bail!("unexpected prepared output response"),
    };
    Ok(from_prepared(participant, prepared))
}

/// Convert an already-frozen native output without a second mailbox read.
/// Provider-specific ownership gates must prepare inside their native scope.
pub(super) fn from_prepared(participant: &ParticipantRef, prepared: PreparedOutput) -> Option<Prepared> {
    let (messages, receipt, formatted) = match prepared {
        PreparedOutput::Deferred => return None,
        PreparedOutput::Unmanaged { messages, receipt, formatted_context, .. } => (messages, OutputReceipt::Unmanaged(receipt), formatted_context),
        PreparedOutput::Claimed { target, batch, messages, .. } => {
            let receipt = DeliveryReceipt { target, batch: batch.id, through: batch.window.through, evidence: DeliveryEvidence::HookOutputWritten };
            (messages, OutputReceipt::Claimed(receipt), batch.formatted_context)
        }
    };
    if messages.is_empty() {
        return None;
    }
    let messages: Vec<_> = messages.iter().map(crate::messaging::formatting::message_to_value).collect();
    let last = messages.last();
    let last_event_id = last.and_then(|message| message.get("event_id")).and_then(serde_json::Value::as_i64).unwrap_or(0);
    let sender = messages.first().and_then(|message| message.get("from")).and_then(serde_json::Value::as_str).unwrap_or("unknown");
    let ack = DeliveryAck {
        claim: match &receipt {
            OutputReceipt::Claimed(claim) => Some(OutputClaim { target: claim.target.clone(), batch: claim.batch.clone(), through: claim.through }),
            _ => None,
        },
        instance_name: participant.canonical_name.clone(),
        created_at: Some(f64::from_bits(participant.incarnation_bits)),
        last_event_id,
        status_context: format!("deliver:{sender}"),
        msg_ts: last.and_then(|message| message.get("timestamp")).and_then(serde_json::Value::as_str).unwrap_or("").to_owned(),
        mark_announced: false,
    };
    Some(Prepared { formatted, messages, ack, receipt })
}

/// Call only after exact host output flush or an owning Provider adapter's
/// positive input correlation. A cursor-only response cannot replace receipt.
pub(crate) fn confirm(client: &Client, receipt: &OutputReceipt, announce: bool, evidence: DeliveryEvidence) -> Result<()> {
    let outcome = match receipt {
        OutputReceipt::Unmanaged(receipt) => inbox(client, InboxOperation::ConfirmOutput { receipt: receipt.clone(), announce })?,
        OutputReceipt::Claimed(claim) => inbox(client, InboxOperation::Confirm(DeliveryReceipt { target: claim.target.clone(), batch: claim.batch.clone(), through: claim.through, evidence }))?,
    };
    match outcome {
        InboxOutcome::Done(result) => result.map_err(Into::into),
        InboxOutcome::Confirmed(Ok(Confirmation::Advanced | Confirmation::AlreadyConfirmed)) => Ok(()),
        _ => bail!("output receipt rejected: {outcome:?}"),
    }
}

/// Confirm exact hook output and its domain-owned delivery status in one write.
/// The store derives delivery context and timestamp from the retained batch.
pub(crate) fn confirm_hook(client: &Client, receipt: &OutputReceipt) -> Result<()> {
    let receipt = match receipt {
        OutputReceipt::Unmanaged(token) => OutputConfirmation::Unmanaged(token.clone()),
        OutputReceipt::Claimed(receipt) => OutputConfirmation::Claimed(DeliveryReceipt { evidence: DeliveryEvidence::HookOutputWritten, ..receipt.clone() }),
    };
    confirm_hook_output(client, receipt)
}

pub(crate) fn confirm_hook_output(client: &Client, receipt: OutputConfirmation) -> Result<()> {
    match inbox(client, InboxOperation::ConfirmOutputAndStatus { receipt, announce: false })? {
        InboxOutcome::Done(result) => result.map_err(Into::into),
        outcome => bail!("hook output receipt rejected: {outcome:?}"),
    }
}
