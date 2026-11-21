//! Direct terminal continuation owns only its operator ticket and native input.

use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use serde_json::json;

use crate::agents::recovery::{NativeAction, NativeOutcome, PROMPT, Ticket};
use crate::runtime::hosts::{HostService, Request, Response};

use super::Mailbox;

pub(super) struct Continuation {
    ticket: Ticket,
    correlation: String,
    deadline: Instant,
}

impl Mailbox {
    fn recovery(&self, action: NativeAction) -> Result<NativeOutcome> {
        match HostService::call(&self.client, Request::NativeRecovery { registration: self.registration.clone(), action })? {
            Response::NativeRecovery(result) => Ok(result),
            _ => bail!("unexpected native recovery response"),
        }
    }

    /// A pending or uncertain continuation never drains the mailbox. Native
    /// request success alone is not evidence the continuation was recorded.
    pub(super) fn poll_recovery(&mut self) -> Result<bool> {
        if let Some(turn) = self.native.failed_turn.clone() {
            self.recovery(NativeAction::FailedTurnEnded { turn })?;
            self.native.failed_turn = None;
        }
        let thread = &self.attachment.session.session_id;
        if let Some(pending) = &self.continuation {
            let found = self.native.find_input(thread, &pending.correlation, PROMPT);
            let recorded = matches!(found, Ok(Some(_)));
            if recorded || Instant::now() >= pending.deadline {
                self.recovery(NativeAction::Finish { ticket: pending.ticket.clone(), recorded })?;
                self.continuation = None;
            } else {
                found?;
            }
            return Ok(true);
        }
        let NativeOutcome::Pending { ticket, model_error } = self.recovery(NativeAction::Poll)? else { bail!("unexpected native recovery poll response") };
        let Some(ticket) = ticket else { return Ok(model_error) };
        // A completed failing Codex turn leaves the thread in systemError,
        // not idle. The daemon ticket already proves a matching model failure
        // and turn end; the native read excludes active or unloaded threads.
        let read = self.native.call("thread/read", json!({"threadId": thread}))?;
        let bound = &read["thread"];
        let available = bound["id"] == *thread && matches!(bound["status"]["type"].as_str(), Some("idle" | "systemError"));
        if !available || !matches!(self.recovery(NativeAction::Begin(ticket.clone()))?, NativeOutcome::Changed(true)) {
            return Ok(true);
        }
        let correlation = format!("agentx:recover:{}", ticket.id);
        self.continuation = Some(Continuation { ticket, correlation: correlation.clone(), deadline: Instant::now() + Duration::from_secs(10) });
        self.native.start_input(thread, PROMPT, &correlation)?;
        Ok(true)
    }
}
