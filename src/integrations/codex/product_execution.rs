//! Private pipe for the trusted dedicated runner. The daemon continues to
//! authenticate this process and own participant registration and inbox effects.

use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::agents::attachments::{AttachmentOwnerId, ExecutionId};
use crate::agents::service::RemoteAttachments;
use crate::messaging::activation::AttemptId;
use crate::messaging::inbox::{ClaimPhase, Confirmation, DeliveryEvidence, DeliveryMode, DeliveryReceipt, PendingInbox, ProductBatch, ProductInput};
use crate::messaging::service::RemoteInbox;
use crate::runtime::hosts::{HostRegistration, HostService, NativeRootReservation, NativeRootState, Request, Response};
use crate::runtime::notify::NotifyServer;
use crate::service::{InvocationContext, client::Client};

use super::mail_evidence::{client_id, item_matches};
use super::protocol::RawObject;
use super::roots::{CodexRootBindings, NativeThreadKey, RootBindings, RootIntent};

#[derive(Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Bind { thread: String },
    Renew,
    Prepare,
    Issue,
    Record { thread: String, turn: String, item: Box<serde_json::value::RawValue> },
    Finish,
    Close,
}

struct Runner {
    client: Client,
    inbox: RemoteInbox,
    notify: NotifyServer,
    root: NativeRootReservation,
    role_selector: String,
    role: crate::config::roles::ResolvedRole,
    thread: Option<String>,
    host: Option<HostRegistration>,
    flight: Option<(ProductBatch, ClaimPhase)>,
}

impl Runner {
    fn command(&mut self, command: Command) -> Result<Value> {
        match command {
            Command::Bind { thread } => {
                ensure!(self.host.is_none() && uuid::Uuid::parse_str(&thread).is_ok(), "exact fresh native thread required");
                let Response::Reserved(participant) = HostService::call(&self.client, Request::ReserveProductRoot { root: self.root.clone(), role: self.role_selector.clone(), workspace: self.root.cwd.clone() })? else {
                    bail!("unexpected reservation response")
                };
                let mut attachments = RemoteAttachments::new(self.client.clone());
                let native = NativeThreadKey { canonical_home: self.root.canonical_home.to_string_lossy().into_owned(), thread_id: thread.clone() };
                let binding = CodexRootBindings::new(&mut attachments).attach(&self.root.execution, &self.root.owner, RootIntent::Create { provisional: participant.clone() }, native)?;
                self.inbox.select_delivery_mode(&binding.attachment.target, DeliveryMode::DedicatedProduct)?;
                let Response::Registered(host) = HostService::call(&self.client, Request::RegisterNativeRoot { target: binding.attachment.target, owner: self.root.owner.clone(), wake_port: self.notify.port(), state: NativeRootState::Idle })? else {
                    bail!("unexpected host registration response")
                };
                self.host = Some(host);
                self.thread = Some(thread);
                Ok(json!({
                    "execution": {"nativeScope": self.client.scope_root(), "generation": self.client.generation()?,
                        "name": participant.canonical_name, "incarnation": participant.incarnation_bits.to_string(),
                        "workspace": self.root.cwd, "executionId": self.root.execution},
                    "role": {"name": self.role.name, "path": self.role.path, "namespace": self.role.namespace, "profile": self.role.profile},
                }))
            }
            Command::Renew => {
                let host = self.host.clone().context("no native root")?;
                ensure!(matches!(HostService::call(&self.client, Request::RenewNativeRoot(host))?, Response::Renewed));
                Ok(Value::Null)
            }
            Command::Prepare => {
                let target = &self.host.as_ref().context("no native root")?.target;
                ensure!(self.flight.is_none(), "a product turn is still open");
                if let Some(source) = self.inbox.prepare_product(target, AttemptId(uuid::Uuid::new_v4().to_string()))? {
                    let response = json!({"batchId": source.batch.id, "requestId": source.request_id, "messageId": source.message_id,
                        "content": source.batch.formatted_context, "clientId": client_id(&source.batch)});
                    self.flight = Some((source, ClaimPhase::Prepared));
                    Ok(response)
                } else {
                    Ok(Value::Null)
                }
            }
            Command::Issue => {
                let target = &self.host.as_ref().context("no native root")?.target;
                let (source, phase) = self.flight.as_mut().context("no prepared flight")?;
                ensure!(*phase == ClaimPhase::Prepared, "input already issued");
                self.inbox.mark_issuing(target, &source.batch.id)?;
                *phase = ClaimPhase::Issuing;
                Ok(Value::Null)
            }
            Command::Record { thread, turn, item } => {
                let target = &self.host.as_ref().context("no native root")?.target;
                let (source, phase) = self.flight.as_mut().context("no issued flight")?;
                ensure!(self.thread.as_deref() == Some(&thread) && !turn.is_empty() && matches!(phase, ClaimPhase::Issuing | ClaimPhase::Admitted), "wrong native input fence");
                let item = RawObject::parse(item.get())?;
                ensure!(item_matches(&item, &client_id(&source.batch), &source.batch.formatted_context), "native input does not match frozen claim");
                let native_item: String = serde_json::from_str(item.get("id").context("no native input ID")?.get())?;
                let payload_digest = ProductInput::digest(&source.batch.formatted_context);
                let proof = ProductInput { request_id: source.request_id.clone(), message_id: source.message_id.clone(), thread_id: thread, turn_id: turn, item_id: native_item, payload_digest };
                let confirmation = self.inbox.confirm(DeliveryReceipt { target: target.clone(), batch: source.batch.id.clone(), through: source.batch.window.through, evidence: DeliveryEvidence::ProductInputRecorded(proof.clone()) })?;
                ensure!(matches!(confirmation, Confirmation::Advanced | Confirmation::AlreadyConfirmed), "mailbox proof was not accepted");
                *phase = ClaimPhase::Admitted;
                Ok(serde_json::to_value(proof)?)
            }
            Command::Finish => {
                ensure!(matches!(self.flight.as_ref(), Some((_, ClaimPhase::Admitted))), "only a recorded turn can settle");
                self.flight = None;
                Ok(Value::Null)
            }
            Command::Close => {
                self.release()?;
                Ok(Value::Null)
            }
        }
    }

    fn release(&mut self) -> Result<()> {
        if let Some(host) = self.host.take() {
            ensure!(matches!(HostService::call(&self.client, Request::ReleaseNativeRoot(host))?, Response::Released));
        }
        Ok(())
    }
}

pub(crate) fn serve(args: &[String]) -> Result<()> {
    let [scope, workspace, home, name, role] = args else { bail!("expected scope workspace native-home name role") };
    let scope_arg = Path::new(scope);
    ensure!(scope_arg.is_absolute() && Path::new(workspace).is_absolute() && Path::new(home).is_absolute(), "require explicit absolute paths");
    let scope = scope_arg.canonicalize()?;
    ensure!(scope.join("product-execution.enabled").is_file(), "this scope is not dedicated to product execution");
    let workspace = Path::new(workspace).canonicalize()?;
    let home = Path::new(home).canonicalize()?;
    ensure!(scope != home && scope != workspace, "dedicated workspace, scope and native home required");
    let selected_role = crate::config::roles::resolve_role(&scope, Some(&workspace), role)?;
    let profile = crate::config::profiles::ProfileCatalog::load(&scope)?;
    ensure!(
        profile.profiles.get(&selected_role.profile).is_some_and(|entry| entry.tool == "codex" && entry.args.is_empty()),
        "dedicated execution requires an argument-free Codex profile; configure the approved provider in the isolated native launcher"
    );
    let invocation = InvocationContext { cwd: workspace.clone(), ..Default::default() };
    let client = Client::connect_existing(&scope, invocation)?;
    let mut runner = Runner {
        inbox: RemoteInbox::new(client.clone()),
        client,
        notify: NotifyServer::new()?,
        role_selector: role.clone(),
        role: selected_role,
        root: NativeRootReservation {
            explicit_name: Some(name.clone()),
            initial_cursor: None,
            launch_args: vec!["app-server".into()],
            launch_profile: None,
            cwd: workspace,
            environment: Default::default(),
            canonical_home: home,
            execution: ExecutionId(uuid::Uuid::new_v4().to_string()),
            owner: AttachmentOwnerId(uuid::Uuid::new_v4().to_string()),
        },
        thread: None,
        host: None,
        flight: None,
    };
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{}", json!({"ok": true, "value": {"instructions": runner.role.prompt}}))?;
    stdout.flush()?;
    let run = (|| -> Result<()> {
        for line in stdin.lock().lines() {
            let line = line?;
            ensure!(line.len() <= 2 * 1024 * 1024, "oversized product pipe request");
            let command: Command = serde_json::from_str(&line)?;
            let stop = matches!(command, Command::Close);
            let response = match runner.command(command) {
                Ok(value) => json!({"ok": true, "value": value}),
                Err(error) => json!({"ok": false, "error": error.to_string()}),
            };
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
            if stop {
                break;
            }
        }
        Ok(())
    })();
    let cleanup = runner.release();
    run.and(cleanup)
}
