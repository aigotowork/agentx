//! Frozen mailbox output through the typed native receipt contract.
//! Preparing a managed batch persists Issuing; only exact successful host output
//! permits confirmation. Lost output or acknowledgement never permits replay.

use crate::agents::attachments::ParticipantRef;
use crate::messaging::inbox::{ClaimPhase, Confirmation, DeliveryEvidence, DeliveryReceipt, EventId, InboxConsumer};
use crate::messaging::service::{InboxOperation, InboxOutcome, OutputConfirmation, PreparedOutput, Request as Messages, Response as MessageResult};
use crate::service::{Request, Response, ServiceError, ServiceResult, client::Client};

pub(crate) struct Batch {
    pub messages: Vec<crate::db::Message>,
    pub remaining: usize,
    pub backlog_end: Option<EventId>,
    authority: PreparedOutput,
}

fn call(client: &Client, operation: InboxOperation) -> ServiceResult<InboxOutcome> {
    match client.call(Request::Messages(Messages::Inbox(operation)))? {
        Response::Messages(MessageResult::Inbox(outcome)) => Ok(outcome),
        _ => Err(ServiceError::Internal("daemon returned a different mailbox response".into())),
    }
}

impl Batch {
    pub(crate) fn prepare(client: &Client, participant: &ParticipantRef) -> ServiceResult<Option<Self>> {
        let prepared = match call(client, InboxOperation::PrepareOutput { participant: participant.clone(), consumer: InboxConsumer::ExplicitListen })? {
            InboxOutcome::Output(result) => result.map_err(|error| ServiceError::Unavailable(error.to_string()))?,
            _ => return Err(ServiceError::Internal("daemon returned a different mailbox preparation".into())),
        };
        let (messages, remaining, backlog_end) = match &prepared {
            PreparedOutput::Unmanaged { messages, remaining, backlog_end, .. } | PreparedOutput::Claimed { messages, remaining, backlog_end, .. } => (messages.clone(), *remaining, *backlog_end),
            PreparedOutput::Deferred => return Ok(None),
        };
        if messages.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self { messages, remaining, backlog_end, authority: prepared }))
    }

    pub(crate) fn remaining_note(&self, name: &str) -> String {
        if self.remaining == 0 {
            return String::new();
        }
        format!("[+{} more unread — run: agentx listen --iam {name}]\n", self.remaining)
    }

    pub(crate) fn emit(&self, client: &Client, output: &str, set_delivery_status: bool) -> ServiceResult<()> {
        use std::io::Write;
        let written = {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(output.as_bytes()).and_then(|_| stdout.flush())
        };
        if let Err(error) = written {
            if let PreparedOutput::Claimed { target, batch, .. } = &self.authority {
                let _ = call(client, InboxOperation::RecordPhase { target: target.clone(), batch: batch.id.clone(), phase: ClaimPhase::Uncertain });
            }
            return Err(ServiceError::OutcomeUnknown(format!("incoming message output failed; unread messages retained: {error}")));
        }
        if set_delivery_status {
            let confirmation = match &self.authority {
                PreparedOutput::Unmanaged { receipt, .. } => OutputConfirmation::Unmanaged(receipt.clone()),
                PreparedOutput::Claimed { target, batch, .. } => OutputConfirmation::Claimed(DeliveryReceipt { target: target.clone(), batch: batch.id.clone(), through: batch.window.through, evidence: DeliveryEvidence::ToolOutputWritten }),
                PreparedOutput::Deferred => return Err(ServiceError::Internal("deferred mailbox output cannot be emitted".into())),
            };
            return match call(client, InboxOperation::ConfirmOutputAndStatus { receipt: confirmation, announce: false })? {
                InboxOutcome::Done(Ok(())) => Ok(()),
                other => Err(ServiceError::OutcomeUnknown(format!("receive acknowledgement rejected after output: {other:?}"))),
            };
        }
        match &self.authority {
            PreparedOutput::Unmanaged { receipt, .. } => match call(client, InboxOperation::ConfirmOutput { receipt: receipt.clone(), announce: false })? {
                InboxOutcome::Done(Ok(())) => Ok(()),
                other => Err(ServiceError::OutcomeUnknown(format!("receive acknowledgement rejected after output: {other:?}"))),
            },
            PreparedOutput::Claimed { target, batch, .. } => {
                let receipt = DeliveryReceipt { target: target.clone(), batch: batch.id.clone(), through: batch.window.through, evidence: DeliveryEvidence::ToolOutputWritten };
                match call(client, InboxOperation::Confirm(receipt))? {
                    InboxOutcome::Confirmed(Ok(Confirmation::Advanced | Confirmation::AlreadyConfirmed)) => Ok(()),
                    other => Err(ServiceError::OutcomeUnknown(format!("receive acknowledgement rejected after output: {other:?}"))),
                }
            }
            PreparedOutput::Deferred => Err(ServiceError::Internal("deferred mailbox output cannot be emitted".into())),
        }
    }

    pub(crate) fn arrived_since(&self, client: &Client, participant: &ParticipantRef) -> ServiceResult<bool> {
        if self.remaining > 0 {
            return Ok(false);
        }
        let after = self.backlog_end.unwrap_or(EventId(0));
        pending_after(client, participant, after)
    }
}

pub(crate) fn pending_after(client: &Client, participant: &ParticipantRef, after: EventId) -> ServiceResult<bool> {
    match call(client, InboxOperation::HasPendingAfter { participant: participant.clone(), after })? {
        InboxOutcome::HasPendingAfter(result) => result.map_err(|error| ServiceError::Unavailable(error.to_string())),
        _ => Err(ServiceError::Internal("daemon returned a different mailbox observation".into())),
    }
}

pub(crate) fn format_messages(messages: &[&crate::db::Message], receiver: &str, records: &[crate::db::InstanceRow]) -> String {
    format_message_group(messages, receiver, records, false, "→")
}

pub(crate) fn format_router_messages(messages: &[&crate::db::Message], receiver: &str, records: &[crate::db::InstanceRow]) -> String {
    format_message_group(messages, receiver, records, true, "→")
}

pub(crate) fn format_listen_messages(messages: &[&crate::db::Message], receiver: &str, records: &[crate::db::InstanceRow]) -> String {
    format_message_group(messages, receiver, records, false, "->")
}

fn format_message_group(messages: &[&crate::db::Message], receiver: &str, _records: &[crate::db::InstanceRow], show_other_recipients: bool, arrow: &str) -> String {
    let display = |name: &str| name.to_owned();
    let recipient = display(receiver);
    let parts: Vec<String> = messages
        .iter()
        .map(|message| {
            let prefix = crate::cli::context::format_envelope_prefix(message.intent.as_deref(), message.thread.as_deref(), message.reply_id().as_deref());
            let others = message.delivered_to.as_ref().map_or(0, |names| names.len().saturating_sub(1));
            let to = if !show_other_recipients || others == 0 {
                recipient.clone()
            } else if messages.len() == 1 {
                format!("{recipient} (+{others} other{})", if others == 1 { "" } else { "s" })
            } else {
                format!("{recipient} (+{others})")
            };
            let directory = message.requested_directory.as_ref().map(|directory| format!(" [directory: {directory}]")).unwrap_or_default();
            format!("{prefix} {} {arrow} {to}{directory}: {}", display(&message.from), message.text)
        })
        .collect();
    match parts.as_slice() {
        [only] => only.clone(),
        [] => String::new(),
        _ => format!("[{} new messages] | {}", parts.len(), parts.join(" | ")),
    }
}
