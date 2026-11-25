//! Caller presentation for accepted-event delivery observations. No retries of
//! Submit, terminal intervention, or receipt confirmation occur in this path.
use crate::cli::client::Session;
use crate::messaging::delivery_report::{DeliveryReport, MailboxProgress};
use crate::messaging::inbox::EventId;
use crate::messaging::service::{Request as Messages, Response as MessageResult};
use crate::service::{Request, Response};

pub(super) fn read(session: &Session, event: i64) -> Result<DeliveryReport, String> {
    match session.client.call(Request::Messages(Messages::DeliveryReport { event: EventId(event) })).map_err(|error| error.to_string())? {
        Response::Messages(MessageResult::DeliveryReport(report)) => Ok(report),
        _ => Err("unexpected delivery report response".into()),
    }
}

pub(super) fn render(report: &DeliveryReport) -> String {
    let now = crate::shared::time::now_epoch_i64();
    let mut lines = vec![format!("[delivery #{}] accepted; mailbox confirmation and model execution are separate.", report.event_id)];
    for recipient in &report.recipients {
        let state = match recipient.mailbox {
            MailboxProgress::Confirmed => "mailbox confirmed (execution not established)".to_owned(),
            MailboxProgress::TargetRetired => "original target retired; successor not credited".to_owned(),
            MailboxProgress::Unknown => "delivery unknown (no exact local target evidence)".to_owned(),
            MailboxProgress::Pending => match &recipient.host {
                Some(host) if host.process_alive == Some(false) => "pending; registered delivery host is gone; explicit session recovery required".to_owned(),
                Some(host) => match host.last_poll {
                    Some(at) if now >= at && now - at <= 65 => format!("pending; host {} polled {}s ago; provider {} ({})", host.pid, now - at, recipient.status.as_deref().unwrap_or("unknown"), recipient.context.as_deref().unwrap_or("")),
                    Some(at) => format!("pending; host {} snapshot progress stale (last={at}); inspect the host or reopen it if it predates the IPC upgrade", host.pid),
                    None => format!("pending; host {} snapshot progress unknown (legacy/unobserved); a live socket does not establish delivery; inspect the registered host", host.pid),
                },
                None => "pending; no registered PTY host evidence (other delivery channels may consume it)".to_owned(),
            },
        };
        lines.push(format!("  @{}: {state}", recipient.name));
    }
    if report.recipients.iter().any(|recipient| recipient.mailbox != MailboxProgress::Confirmed) {
        lines.push(format!("  Inspect without resending: agentx send --delivery-status {} --json", report.event_id));
    }
    lines.join("\n")
}
