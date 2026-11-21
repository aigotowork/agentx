//! Ordinary TUI connects to the native socket. This client creates one root
//! and owns only that root's AgentX mailbox; it never reads TUI traffic.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

use crate::agents::attachments::{Attachment, AttachmentOwnerId, AttachmentStore, ParticipantRef};
use crate::agents::service::RemoteAttachments;
use crate::messaging::activation::{ActivationPolicy, Admission, TurnEvidence};
use crate::messaging::inbox::{ClaimPhase, DeliveryEvidence, DeliveryMode, DeliveryReceipt, OutstandingClaim, PendingInbox};
use crate::messaging::service::RemoteInbox;
use crate::runtime::context::AgentxContext;
use crate::runtime::hosts::{HostRegistration, HostService, NativeRootState, Request, Response};
use crate::runtime::notify::NotifyServer;
use crate::service::client::Client;

use super::configuration::SessionConfiguration;
use super::roots::{CodexRootBindings, NativeThreadKey, RootBindings, RootIntent};
use super::service::{CodexService, ManagedCodexService, NativeConnection, Receive, ServiceRequest};

mod recovery;

const NATIVE_VERSION: &str = "0.170.0";
const RPC_TIMEOUT: Duration = Duration::from_secs(5);

/// Options consumed once by the runner; never inherited as launch authority.
pub(crate) const RUNNER_MODE: &str = "AGENTX_CODEX_DIRECT_ROOT";
pub(crate) const DIRECT_TERMINAL: &str = "AGENTX_CODEX_DIRECT_TERMINAL";

/// The native thread remains loaded through TUI attach and mailbox operation.
pub(crate) struct TerminalSession {
    pub(crate) argv: Vec<String>,
    pub(crate) home: PathBuf,
    pub(crate) ready: Arc<AtomicBool>,
    worker: Option<Mailbox>,
    stop: Option<(Sender<()>, u16, std::thread::JoinHandle<()>)>,
}

struct Mailbox {
    native: NativeClient,
    client: Client,
    attachment: Attachment,
    registration: HostRegistration,
    notify: NotifyServer,
    continuation: Option<recovery::Continuation>,
}

struct NativeClient {
    transport: Box<dyn NativeConnection>,
    thread: String,
    failed_turn: Option<String>,
}

impl NativeClient {
    fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = uuid::Uuid::new_v4().to_string();
        self.transport.send(&json!({"id": id, "method": method, "params": params}).to_string())?;
        let deadline = Instant::now() + RPC_TIMEOUT;
        while Instant::now() < deadline {
            match self.transport.receive(deadline.saturating_duration_since(Instant::now()))? {
                Receive::Frame(raw) => {
                    let mut frame: Value = serde_json::from_str(&raw)?;
                    if frame.get("id").and_then(Value::as_str) != Some(&id) || frame.get("method").is_some() {
                        self.observe(&frame);
                        continue;
                    }
                    if let Some(result) = frame.get_mut("result") {
                        return Ok(result.take());
                    }
                    bail!("native {method} rejected: {}", frame["error"]["message"]);
                }
                Receive::TimedOut => break,
                Receive::Closed => bail!("native socket closed during {method}"),
            }
        }
        bail!("native {method} timed out")
    }

    fn observe(&mut self, frame: &Value) {
        if frame["method"] == "turn/completed" && frame["params"]["threadId"] == self.thread && frame["params"]["turn"]["status"] == "failed" {
            self.failed_turn = frame["params"]["turn"]["id"].as_str().map(str::to_owned);
        }
    }

    fn idle(&mut self, thread: &str) -> Result<bool> {
        let result = self.call("thread/read", json!({"threadId": thread}))?;
        Ok(result["thread"]["id"] == thread && result["thread"]["status"]["type"] == "idle")
    }

    fn start_input(&mut self, thread: &str, text: &str, correlation: &str) -> Result<Value> {
        self.call("turn/start", json!({"threadId": thread, "input": [{"type": "text", "text": text}], "clientUserMessageId": correlation}))
    }

    fn find_input(&mut self, thread: &str, correlation: &str, text: &str) -> Result<Option<TurnEvidence>> {
        let result = self.call("thread/items/list", json!({"threadId": thread, "limit": 100, "sortDirection": "desc"}))?;
        for row in result["data"].as_array().context("native thread/items/list returned no entries")? {
            let item = super::protocol::RawObject::parse(&row["item"].to_string())?;
            if super::mail_evidence::item_matches(&item, correlation, text) {
                return Ok(Some(TurnEvidence { opaque_turn_id: row["turnId"].as_str().map(str::to_owned) }));
            }
        }
        Ok(None)
    }
}

/// Translate only the terminal launch options whose start semantics we know.
/// Positional prompt text still goes through the real TUI as user input.
fn launch_options(args: &[&str]) -> Result<(Vec<String>, Value, Vec<String>)> {
    let mut config = Vec::new();
    let mut params = json!({"threadSource": "user", "historyMode": "paginated"});
    let mut tui = Vec::new();
    let mut args = args.iter().copied();
    let mut prompt = None;
    while let Some(arg) = args.next() {
        let (flag, inline) = arg.split_once('=').map_or((arg, None), |(key, value)| (key, Some(value)));
        match flag {
            "-c" | "--config" | "--enable" | "--disable" => {
                config.extend([flag.to_owned(), inline.or_else(|| args.next()).context("missing Codex option value")?.to_owned()]);
            }
            "-m" | "--model" | "-s" | "--sandbox" | "-a" | "--ask-for-approval" => {
                let key = match flag {
                    "-m" | "--model" => "model",
                    "-s" | "--sandbox" => "sandbox",
                    _ => "approvalPolicy",
                };
                params[key] = json!(inline.or_else(|| args.next()).context("missing Codex option value")?);
            }
            "--no-alt-screen" if inline.is_none() => tui.push(arg.to_owned()),
            "--" => {
                if let Some(value) = args.next() {
                    ensure!(prompt.replace(value).is_none() && args.next().is_none(), "expected one Codex prompt");
                }
                break;
            }
            "resume" | "fork" | "exec" | "app-server" | "agents" | "review" => bail!("direct Codex supports fresh terminal launches only"),
            _ if !arg.starts_with('-') && prompt.is_none() => prompt = Some(arg),
            _ => bail!("unsupported direct Codex terminal argument {flag:?}"),
        }
    }
    if let Some(prompt) = prompt {
        tui.extend(["--".to_owned(), prompt.to_owned()]);
    }
    Ok((config, params, tui))
}

impl TerminalSession {
    pub(crate) fn prepare(program: &str, prefix: &[String], args: &[&str], title: &str, auto_approve: bool, mut context: AgentxContext, launch_context: Value) -> Result<Option<Self>> {
        let required = context.raw_env.get("AGENTX_CODEX_TERMINAL").is_some_and(|mode| mode == "shared");
        let (config, mut params, tui) = match launch_options(args) {
            Ok(options) => options,
            Err(_) if !required => return Ok(None),
            Err(error) => return Err(error),
        };
        let participant: ParticipantRef = serde_json::from_str(context.raw_env.get("AGENTX_LAUNCH_PARTICIPANT").context("direct Codex requires an AgentX reservation")?)?;
        let mut invocation = crate::runtime::host_client::invocation(&context);
        invocation.launch_context = Some(launch_context);
        let client = Client::connect(&context.agentx_dir, invocation)?;
        match HostService::call(&client, Request::DirectTerminalSupport) {
            Ok(Response::Renewed) => {}
            _ if !required => return Ok(None),
            _ => bail!("the running AgentX daemon does not support direct Codex terminals with recovery"),
        }
        ensure!(context.raw_env.get("AGENTX_LAUNCH_GENERATION") == Some(&client.info().database_generation), "direct Codex reservation belongs to another database generation");
        let actual = match client.call(crate::service::Request::Participants(crate::agents::service::Request::Resolve))? {
            crate::service::Response::Participants(crate::agents::service::Response::Actor(actor)) => actor.participant,
            _ => bail!("unexpected direct Codex participant response"),
        };
        ensure!(actual == participant, "direct Codex process binding differs from its reservation");
        let (home, _) = crate::integrations::launch_args::codex::resolve_codex_home_from_env(&context.raw_env, &context.cwd).context("missing CODEX_HOME")?;
        let home = match home.canonicalize() {
            Ok(home) => home,
            Err(error) if !required && error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).context("direct Codex requires an existing CODEX_HOME"),
        };
        context.raw_env.remove(RUNNER_MODE);
        let request = ServiceRequest {
            program: PathBuf::from(program),
            prefix_args: prefix.to_vec(),
            args: vec!["app-server".into()],
            env: context.raw_env.clone(),
            canonical_home: home.clone(),
            agentx_dir: client.scope_root().to_owned(),
            required_version: Some(NATIVE_VERSION),
        };
        let connection = match ManagedCodexService.connect(&request) {
            Ok(connection) => connection,
            Err(error) if !required => {
                crate::log::log_info("codex", "direct_terminal.fallback", &error.to_string());
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        let socket = connection.identity.control_socket.to_str().context("native control socket path is not UTF-8")?;
        let mut native = NativeClient { transport: connection.transport, thread: String::new(), failed_turn: None };
        let initialized = native.call(
            "initialize",
            json!({"clientInfo": {"name": "agentx-terminal", "version": env!("CARGO_PKG_VERSION")}, "capabilities": {
                "experimentalApi": true,
                "optOutNotificationMethods": ["item/agentMessage/delta", "item/reasoning/summaryTextDelta", "item/reasoning/textDelta", "item/commandExecution/outputDelta", "item/fileChange/outputDelta", "thread/tokenUsage/updated"],
            }}),
        )?;
        ensure!(initialized.get("codexHome").and_then(Value::as_str).map(Path::new) == Some(home.as_path()), "native initialize returned a different Codex home");
        native.transport.send(r#"{"method":"initialized"}"#)?;
        params["cwd"] = json!(context.cwd);
        context.raw_env.insert(DIRECT_TERMINAL.into(), "1".into());
        let composition = SessionConfiguration::new(&config, context.raw_env, &context.cwd, &connection.identity, auto_approve)?;
        let start = composition.prepare_root(&json!({"id": 0, "method": "thread/start", "params": params}).to_string(), &participant)?;
        let result = native.call("thread/start", serde_json::from_str::<Value>(&start)?["params"].take())?;
        let thread = result["thread"]["id"].as_str().filter(|id| !id.is_empty()).context("native thread/start returned no id")?;
        native.thread = thread.to_owned();
        native.call("thread/name/set", json!({"threadId": thread, "name": title}))?;

        let notify = NotifyServer::new()?;
        let mut attachments = RemoteAttachments::new(client.clone());
        let owner = AttachmentOwnerId(uuid::Uuid::new_v4().to_string());
        let key = NativeThreadKey { canonical_home: home.to_string_lossy().into_owned(), thread_id: thread.to_owned() };
        let attachment = CodexRootBindings::new(&mut attachments).attach(&connection.identity.execution, &owner, RootIntent::Create { provisional: participant }, key)?.attachment;
        let registration = (|| -> Result<HostRegistration> {
            RemoteInbox::new(client.clone()).select_delivery_mode(&attachment.target, DeliveryMode::NativeInput)?;
            match HostService::call(&client, Request::RegisterNativeRoot { target: attachment.target.clone(), owner, wake_port: notify.port(), state: NativeRootState::Idle })? {
                Response::Registered(registration) => Ok(registration),
                _ => bail!("unexpected direct Codex host registration response"),
            }
        })();
        let registration = match registration {
            Ok(registration) => registration,
            Err(error) => {
                let _ = attachments.retire(&attachment.target);
                return Err(error);
            }
        };
        let prompt_at = tui.iter().position(|arg| arg == "--").unwrap_or(tui.len());
        let mut argv = tui[..prompt_at].to_vec();
        argv.extend(["--remote".into(), format!("unix://{socket}"), "resume".into(), thread.to_owned()]);
        argv.extend_from_slice(&tui[prompt_at..]);
        Ok(Some(Self { argv, home, ready: Arc::new(AtomicBool::new(false)), worker: Some(Mailbox { native, client, attachment, registration, notify, continuation: None }), stop: None }))
    }

    pub(crate) fn run_mailbox(&mut self) {
        let Some(mailbox) = self.worker.take() else { return };
        let (stop, stopped) = mpsc::channel();
        let port = mailbox.notify.port();
        let ready = self.ready.clone();
        let worker = std::thread::spawn(move || mailbox.run(stopped, ready));
        self.stop = Some((stop, port, worker));
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if let Some((stop, port, worker)) = self.stop.take() {
            let _ = stop.send(());
            let _ = std::net::TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_millis(100));
            let deadline = Instant::now() + Duration::from_secs(2);
            while !worker.is_finished() && Instant::now() < deadline {
                std::thread::park_timeout(Duration::from_millis(20));
            }
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
        // If startup failed before the worker was transferred, dropping it
        // still releases the exact registration.
    }
}

impl Mailbox {
    fn run(mut self, stopped: mpsc::Receiver<()>, ready: Arc<AtomicBool>) {
        let mut policy = ActivationPolicy::default();
        let mut inbox = RemoteInbox::new(self.client.clone());
        let attachments = RemoteAttachments::new(self.client.clone());
        let mut heartbeat = Instant::now();
        let mut outage_reported = false;
        let mut ready_published = false;
        while matches!(stopped.try_recv(), Err(mpsc::TryRecvError::Empty)) {
            for _ in 0..256 {
                match self.native.transport.receive(Duration::ZERO) {
                    Ok(Receive::Frame(raw)) => {
                        if let Ok(frame) = serde_json::from_str(&raw) {
                            self.native.observe(&frame);
                        }
                    }
                    Ok(Receive::TimedOut) => break,
                    // A replaced native backend can never inherit this worker.
                    _ => return,
                }
            }
            let tick = (|| -> Result<bool> {
                if !attachments.validate(&self.attachment.target)? {
                    return Ok(false);
                }
                if heartbeat.elapsed() >= Duration::from_secs(15) {
                    HostService::call(&self.client, Request::RenewNativeRoot(self.registration.clone()))?;
                    heartbeat = Instant::now();
                }
                if !ready_published && ready.load(Ordering::Acquire) {
                    HostService::call(&self.client, Request::NativeRootReady(self.registration.clone()))?;
                    ready_published = true;
                }
                if ready_published {
                    let allow_mail = !self.poll_recovery()?;
                    self.poll_mail(&mut policy, &mut inbox, allow_mail)?;
                }
                Ok(true)
            })();
            match tick {
                Ok(false) => break,
                Ok(true) => outage_reported = false,
                Err(error) if !outage_reported => {
                    crate::log::log_warn("codex", "direct_terminal.mail", &error.to_string());
                    outage_reported = true;
                }
                Err(_) => {}
            }
            self.notify.wait(Duration::from_secs(if outage_reported { 5 } else { 1 }));
        }
    }

    fn poll_mail(&mut self, policy: &mut ActivationPolicy, inbox: &mut dyn PendingInbox, allow_send: bool) -> Result<()> {
        let target = self.attachment.target.clone();
        let thread = self.attachment.session.session_id.clone();
        if let Some(claim) = inbox.outstanding(&target)? {
            if claim.phase != ClaimPhase::Prepared || claim.target != target {
                return self.confirm_input(inbox, &claim);
            }
        } else if !allow_send || inbox.snapshot(&target.participant)?.is_none() {
            return Ok(());
        }
        if !allow_send || !self.native.idle(&thread)? {
            return Ok(());
        }
        let Some(request) = policy.prepare_request(inbox, &target)? else { return Ok(()) };
        // No provider-side idempotency: persist Issuing before any send; only
        // positive history evidence can resolve a lost or ambiguous response.
        inbox.mark_issuing(&target, &request.batch.id)?;
        let outcome = self.native.start_input(&thread, &request.batch.formatted_context, &super::mail_evidence::client_id(&request.batch));
        let phase = if outcome.is_ok() { ClaimPhase::Admitted } else { ClaimPhase::Uncertain };
        inbox.record_phase(&target, &request.batch.id, phase)?;
        policy.finish_admission(inbox, &request, &Admission::Accepted { attempt: request.attempt.clone() })?;
        outcome?;
        Ok(())
    }

    fn confirm_input(&mut self, inbox: &mut dyn PendingInbox, claim: &OutstandingClaim) -> Result<()> {
        if claim.phase == ClaimPhase::Prepared || claim.target != self.attachment.target {
            return Ok(());
        }
        let correlation = super::mail_evidence::client_id(&claim.batch);
        if let Some(turn) = self.native.find_input(&self.attachment.session.session_id, &correlation, &claim.batch.formatted_context)? {
            inbox.confirm(DeliveryReceipt { target: claim.target.clone(), batch: claim.batch.id.clone(), through: claim.batch.window.through, evidence: DeliveryEvidence::ProviderInputRecorded { turn } })?;
        }
        Ok(())
    }
}

impl Drop for Mailbox {
    fn drop(&mut self) {
        let _ = HostService::call(&self.client, Request::ReleaseNativeRoot(self.registration.clone()));
        let _ = RemoteAttachments::new(self.client.clone()).retire(&self.attachment.target);
    }
}
