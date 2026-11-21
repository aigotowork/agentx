//! Caller-owned hook effects around typed participant and mailbox operations.
//! Provider payload parsing/rendering stays in its adapter; only exact flushed
//! output can confirm bootstrap or message receipts.
use super::output;
use crate::agents::lifecycle_service as lifecycle;
use crate::agents::service::{Request as ParticipantRequest, ResolvedActor, Response as ParticipantResponse};
use crate::runtime::context::AgentxContext;
use crate::runtime::hosts::{HostService, Request as HostRequest};
use crate::service::client::Client;
use crate::service::{Request, Response};
use anyhow::{Result, bail};
use serde_json::Value;

pub(super) struct HookClient {
    pub client: Client,
    pub context: AgentxContext,
}

impl HookClient {
    pub fn connect(context: &AgentxContext, capture_launch: bool) -> Result<Self> {
        let mut invocation = crate::runtime::host_client::invocation(context);
        if capture_launch {
            invocation.launch_context = Some(Value::Object(crate::agents::binding::capture_context_snapshot(&context.agentx_dir, &context.cwd, &context.raw_env)));
        }
        Ok(Self { client: Client::connect(&context.agentx_dir, invocation)?, context: context.clone() })
    }
    pub fn participant(&self, request: ParticipantRequest) -> Result<ParticipantResponse> {
        match self.client.call(Request::Participants(request))? {
            Response::Participants(response) => Ok(response),
            _ => bail!("unexpected hook participant response"),
        }
    }
    pub fn resolve(&self, tool: &str) -> Result<ResolvedActor> {
        match self.participant(ParticipantRequest::Resolve)? {
            ParticipantResponse::Actor(actor) if actor.record.tool == tool => Ok(actor),
            _ => bail!("hook process belongs to another Provider"),
        }
    }
    pub fn session(&self, tool: &str, payload: &super::HookPayload, directory: Option<std::path::PathBuf>, create_if_unbound: bool) -> Result<ResolvedActor> {
        let session_id = payload.session_id.as_deref().filter(|id| !id.is_empty()).ok_or_else(|| anyhow::anyhow!("hook session required"))?;
        let request = crate::agents::service::HookSession { tool: tool.to_owned(), session_id: session_id.to_owned(), transcript_path: payload.transcript_path.clone(), directory, create_if_unbound };
        match self.participant(ParticipantRequest::HookSession(request))? {
            ParticipantResponse::Registered(actor) => Ok(actor),
            _ => bail!("unexpected hook session response"),
        }
    }
    pub fn status(&self, actor: &ResolvedActor, status: &str, context: &str, detail: &str) -> Result<()> {
        self.participant(ParticipantRequest::Status { participant: actor.participant.clone(), status: status.to_owned(), context: context.to_owned(), detail: detail.to_owned() })?;
        Ok(())
    }
    pub fn tool_status(&self, actor: &ResolvedActor, payload: &super::HookPayload) -> Result<()> {
        let detail = super::tool_status::extract_tool_detail(&actor.record.tool, &payload.tool_name, &payload.tool_input);
        self.status(actor, "active", &format!("tool:{}", payload.tool_name), &detail)
    }
    pub fn wake(&self, actor: &ResolvedActor) -> Result<()> {
        HostService::call(&self.client, HostRequest::WakeHook { participant: actor.participant.clone() })?;
        Ok(())
    }
    pub fn bootstrap(&self, actor: &ResolvedActor, force: bool) -> Result<Option<(String, String)>> {
        self.context_for(actor, force, crate::runtime::agent_context::ContextReason::Startup)
    }
    pub fn context_for(&self, actor: &ResolvedActor, force: bool, reason: crate::runtime::agent_context::ContextReason) -> Result<Option<(String, String)>> {
        if actor.record.tool == "codex" && !force {
            return self.initial_context(actor);
        }
        let receipt = match self.participant(ParticipantRequest::Lifecycle(lifecycle::Request::PrepareBootstrap { participant: actor.participant.clone(), force }))? {
            ParticipantResponse::Lifecycle(lifecycle::Response::BootstrapPrepared { receipt }) => receipt,
            _ => bail!("unexpected bootstrap preparation response"),
        };
        let Some(receipt) = receipt else {
            return Ok(None);
        };
        let text = if actor.record.tool == "codex" {
            // Bind the frozen snapshot to the actual native session for resume.
            let fixed = crate::runtime::bootstrap::get_instructions_client(&self.client, &self.context, &actor.participant.canonical_name, &actor.record.tool)?;
            let dynamic = crate::runtime::bootstrap::get_compact_context_client(&self.client, &self.context, &actor.participant.canonical_name, &actor.record.tool)?;
            if crate::integrations::codex::roots::is_shared_context(&self.context) { [fixed, dynamic].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join("\n\n") } else { dynamic }
        } else {
            crate::runtime::bootstrap::get_context_client(&self.client, &self.context, &actor.participant.canonical_name, &actor.record.tool, reason)?
        };
        Ok(Some((text, receipt)))
    }
    /// Independent from name_announced: fixed instructions do not consume this gate.
    pub fn initial_context(&self, actor: &ResolvedActor) -> Result<Option<(String, String)>> {
        use crate::agents::context_service::{Request, Response};
        let session_id = actor.record.session_id.clone().filter(|session| !session.is_empty()).ok_or_else(|| anyhow::anyhow!("initial context requires a native session"))?;
        let receipt = match crate::runtime::bootstrap::context_state(&self.client, Request::PrepareInitial { participant: actor.participant.clone(), session_id })? {
            Response::InitialPrepared { receipt } => receipt,
            _ => bail!("unexpected initial context receipt"),
        };
        let Some(receipt) = receipt else {
            return Ok(None);
        };
        let prepared = (|| {
            let fixed = crate::runtime::bootstrap::get_instructions_client(&self.client, &self.context, &actor.participant.canonical_name, &actor.record.tool)?;
            let dynamic = crate::runtime::bootstrap::get_initial_context_client(&self.client, &self.context, &actor.participant.canonical_name, &actor.record.tool)?;
            Ok(if crate::integrations::codex::roots::is_shared_context(&self.context) { [fixed, dynamic].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join("\n\n") } else { dynamic })
        })();
        match prepared {
            Ok(text) => Ok(Some((text, receipt))),
            Err(error) => {
                let _ = crate::runtime::bootstrap::context_state(&self.client, Request::ReleaseInitial { participant: actor.participant.clone(), receipt });
                Err(error)
            }
        }
    }
    /// Native boundaries without a compact callback can refresh current facts
    /// before the next turn without claiming mail or creating a user message.
    pub fn runtime_context(&self, actor: &ResolvedActor) -> Result<String> {
        crate::runtime::bootstrap::get_context_client(&self.client, &self.context, &actor.participant.canonical_name, &actor.record.tool, crate::runtime::agent_context::ContextReason::Compact)
    }
    pub fn pending(&self, actor: &ResolvedActor) -> Result<Option<output::Prepared>> {
        crate::observability::trace::time(crate::observability::trace::Phase::Inbox, || output::prepare(&self.client, &actor.participant))
    }
    pub fn flush_json(&self, actor: &ResolvedActor, output: &Value, mailbox: Option<&output::Prepared>, bootstrap: Option<&str>) -> Result<()> {
        if bootstrap.is_some() {
            let current = self.resolve(&actor.record.tool)?;
            if current.participant != actor.participant || current.record.session_id != actor.record.session_id {
                bail!("context attachment changed before output");
            }
        }
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer(&mut stdout, output)?;
        stdout.flush()?;
        self.confirm(actor, mailbox, bootstrap)
    }
    pub fn stop(&self, actor: &ResolvedActor, session: Option<&str>, reason: &str, preserve_live_pty: bool, soft: bool) -> Result<()> {
        let request = crate::agents::service::StopSession { participant: actor.participant.clone(), session_id: session.map(str::to_owned), reason: reason.to_owned(), preserve_live_pty, soft };
        let outcome = match self.participant(ParticipantRequest::StopSession(request))? {
            ParticipantResponse::SessionStopped(outcome) => outcome,
            _ => bail!("unexpected session stop response"),
        };
        for effect in outcome.effects {
            self.apply_stop_effect(effect);
        }
        Ok(())
    }
    fn apply_stop_effect(&self, effect: crate::agents::hook_service::RetiredParticipant) {
        crate::runtime::stop_effects::apply(effect);
    }
    /// The caller must write the complete Provider response and flush before
    /// entering here. Independent receipts remain independent on IPC failure.
    pub fn confirm(&self, actor: &ResolvedActor, mailbox: Option<&output::Prepared>, bootstrap: Option<&str>) -> Result<()> {
        if let Some(prepared) = mailbox {
            output::confirm_hook(&self.client, &prepared.receipt)?;
        }
        if let Some(receipt) = bootstrap {
            if receipt.starts_with("context:") {
                crate::runtime::bootstrap::context_state(&self.client, crate::agents::context_service::Request::ConfirmInitial { participant: actor.participant.clone(), receipt: receipt.to_owned() })?;
                return Ok(());
            }
            self.participant(ParticipantRequest::Lifecycle(lifecycle::Request::BootstrapFlushed { participant: actor.participant.clone(), receipt: receipt.to_owned() }))?;
        }
        Ok(())
    }
}
