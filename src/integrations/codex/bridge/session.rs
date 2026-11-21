//! Root preparation, outer/native traffic and native lifecycle observation.

#[cfg(test)]
#[path = "session/acceptance_tests.rs"]
mod acceptance_tests;

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;

use crate::agents::attachments::{AttachmentFailure, AttachmentOwnerId, ParticipantRef};
use crate::messaging::activation::DriverEvent;
use crate::messaging::inbox::DeliveryMode;
use crate::runtime::hosts::{NativeRootReservation, NativeRootState, Request as HostRequest, Response as HostResponse};

use super::super::configuration::SessionConfiguration;
use super::super::protocol::{FrameKind, NativeFrame, NativeRoute, OuterRoute, ProtocolRouter, RawObject, RpcId};
use super::super::roots::{CodexRootBindings, NativeThreadKey, RootBindings, RootIntent};
use super::super::service::ServiceIdentity;
use super::{BoundRoot, BridgePorts, ExternalOperation, NativeBridge, PendingRoot, RootLaunch, ThreadState};

pub(super) fn field<T: DeserializeOwned>(object: &RawObject, name: &str) -> Option<T> {
    object.get(name).and_then(|raw| serde_json::from_str(raw.get()).ok())
}

pub(super) fn child(object: &RawObject, name: &str) -> Option<RawObject> {
    object.get(name).and_then(|raw| RawObject::parse(raw.get()).ok())
}

fn state_from_thread(thread: &RawObject) -> ThreadState {
    child(thread, "status").map(|status| state_from_status(&status)).unwrap_or(ThreadState::Unknown)
}

fn state_from_status(status: &RawObject) -> ThreadState {
    match field::<String>(status, "type").as_deref() {
        Some("idle") => ThreadState::Idle,
        Some("active") => {
            let flags = field::<Vec<String>>(status, "activeFlags").unwrap_or_default();
            if flags.iter().any(|flag| flag == "waitingOnApproval") {
                ThreadState::AwaitingApproval
            } else if flags.iter().any(|flag| flag == "waitingOnUserInput") {
                ThreadState::AwaitingInput
            } else if flags.is_empty() {
                ThreadState::Active
            } else {
                ThreadState::Unknown
            }
        }
        Some("notLoaded") => ThreadState::Retired,
        _ => ThreadState::Unknown,
    }
}

fn is_read_only(method: &str) -> bool {
    matches!(
        method,
        "thread/read"
            | "thread/list"
            | "thread/loaded/list"
            | "thread/turns/list"
            | "thread/items/list"
            | "model/list"
            | "config/read"
            | "account/read"
            | "account/rateLimits/read"
            | "skills/list"
            | "hooks/list"
            | "mcpServerStatus/list"
            | "collaborationMode/list"
            | "experimentalFeature/list"
    )
}

impl<'a> NativeBridge<'a> {
    pub(crate) fn new(configuration: SessionConfiguration, launch: RootLaunch, identity: ServiceIdentity, ports: BridgePorts<'a>, notify_port: u16) -> Self {
        let supported = super::super::service::supports_shared_native(&identity.native_version);
        if !supported {
            eprintln!("[agentx] native forwarding available; autonomous input has no verified profile for {}", identity.native_version);
        }
        Self {
            configuration,
            launch,
            identity,
            ports,
            owner: AttachmentOwnerId(uuid::Uuid::new_v4().to_string()),
            router: ProtocolRouter::new(4096),
            handshake: Default::default(),
            roots: HashMap::new(),
            external: HashMap::new(),
            controls: HashMap::new(),
            attempts: HashMap::new(),
            state_reads: HashMap::new(),
            recovery_scanned: Default::default(),
            policy: Default::default(),
            events: Default::default(),
            notify_port,
            supported,
            closed: false,
            fatal: None,
        }
    }

    fn reserve_participant(&mut self) -> Result<ParticipantRef> {
        let environment = self.configuration.env.iter().filter(|(key, _)| key.starts_with("AGENTX_") || key.as_str() == "CODEX_HOME").map(|(key, value)| (key.clone(), value.clone())).collect();
        let request = NativeRootReservation {
            explicit_name: self.launch.explicit_name.take(),
            initial_cursor: self.launch.initial_cursor,
            launch_args: self.launch.launch_args.clone(),
            launch_profile: self.launch.launch_profile.clone(),
            cwd: std::path::PathBuf::from(&self.configuration.cwd),
            environment,
            canonical_home: self.identity.canonical_home.clone(),
            execution: self.identity.execution.clone(),
            owner: self.owner.clone(),
        };
        match self.ports.hosts.call(HostRequest::ReserveNativeRoot(request))? {
            HostResponse::Reserved(participant) => Ok(participant),
            _ => bail!("Unexpected native root reservation response"),
        }
    }

    fn prepare_root(&mut self, method: &str, message: &NativeFrame) -> Result<Option<PendingRoot>> {
        if !matches!(method, "thread/start" | "thread/resume" | "thread/fork") || !self.supported {
            return Ok(None);
        }
        let params = message.params()?.unwrap_or_else(RawObject::empty);
        if field::<String>(&params, "path").is_some_and(|path| !path.is_empty()) || field::<bool>(&params, "experimentalPredictionMode") == Some(true) {
            eprintln!("[agentx] this native root variant is forwarded without AgentX attachment; path/prediction configuration has no verified binding profile");
            return Ok(None);
        }
        let key = || NativeThreadKey { canonical_home: self.identity.canonical_home.to_string_lossy().into_owned(), thread_id: field::<String>(&params, "threadId").unwrap_or_default() };
        let pending = match method {
            "thread/start" => {
                let participant = self.reserve_participant()?;
                PendingRoot { intent: RootIntent::Create { provisional: participant.clone() }, participant }
            }
            "thread/resume" => {
                let native_thread = key();
                if native_thread.thread_id.is_empty() {
                    return Ok(None);
                }
                let bindings = CodexRootBindings::new(self.ports.attachments);
                if bindings.lookup(&self.identity.execution, &native_thread)?.is_some_and(|binding| binding.attachment.owner != self.owner) {
                    return Err(AttachmentFailure::OwnedByLiveController.into());
                }
                // Atomic attach still arbitrates races after the native result;
                // reject a known sibling owner before changing thread config.
                let canonical = bindings.canonical_participant(&native_thread)?;
                let provisional = if canonical.is_none() { Some(self.reserve_participant()?) } else { None };
                let participant = canonical.or_else(|| provisional.clone()).context("No resume participant")?;
                PendingRoot { intent: RootIntent::Resume { native_thread, provisional }, participant }
            }
            "thread/fork" => {
                let parent = key();
                if parent.thread_id.is_empty() {
                    return Ok(None);
                }
                let participant = self.reserve_participant()?;
                PendingRoot { intent: RootIntent::Fork { parent, provisional: participant.clone() }, participant }
            }
            _ => unreachable!(),
        };
        Ok(Some(pending))
    }

    /// Forward outer traffic without rewriting user methods or native server
    /// response IDs. Only qualified lifecycle requests prepare identities.
    pub(crate) fn handle_outer(&mut self, raw: &str) -> Result<()> {
        if self.closed {
            bail!("Native bridge is closed");
        }
        let message = NativeFrame::parse(raw)?;
        let mut pending_root = None;
        let mut prepared = raw.to_owned();
        if let FrameKind::Request { method, .. } = &message.kind {
            pending_root = self.prepare_root(method, &message)?;
            if let Some(root) = &pending_root {
                prepared = self.configuration.prepare_root(raw, &root.participant)?;
            }
        }
        match self.router.route_outer(&prepared)? {
            OuterRoute::Request { downstream_id, method, mut frame } => {
                let params = message.params()?.unwrap_or_else(RawObject::empty);
                let thread_id = field::<String>(&params, "threadId");
                if method == "initialize" {
                    frame = self.handshake.prepare_initialize(downstream_id.clone(), &frame)?;
                }
                let mutation = method != "initialize" && !is_read_only(&method);
                if mutation {
                    for (id, root) in &mut self.roots {
                        if thread_id.as_ref().is_none_or(|thread| thread == id) {
                            root.state = ThreadState::Unknown;
                            if method == "turn/interrupt" {
                                root.suspended = true;
                            }
                            if method == "turn/start" {
                                root.suspended = false;
                            }
                        }
                    }
                }
                self.external.insert(downstream_id, ExternalOperation { method, thread_id, mutation, root: pending_root });
                self.ports.transport.send(&frame)?;
            }
            OuterRoute::Notification { method, frame } => {
                self.ports.transport.send(&frame)?;
                if method == "initialized" {
                    self.handshake.observe_initialized();
                }
            }
            OuterRoute::ServerResponse { native_id, frame } => {
                self.ports.transport.send(&frame)?;
                if let Some((thread, _)) = self.controls.remove(&native_id) {
                    self.refresh_after_control(thread.as_deref())?;
                }
            }
        }
        Ok(())
    }

    /// Internal replies are consumed by the actor. Everything else returns its
    /// native frame in order, restoring only outer downstream request IDs.
    pub(crate) fn handle_native(&mut self, raw: &str) -> Result<Option<String>> {
        match self.router.route_native(raw)? {
            NativeRoute::OuterResponse { downstream_id, frame, .. } => {
                self.handshake.observe_response(&downstream_id, raw)?;
                if let Some(operation) = self.external.remove(&downstream_id) {
                    self.observe_outer_result(operation, raw)?;
                }
                Ok(Some(frame))
            }
            NativeRoute::InternalResponse { downstream_id, owner, frame } => {
                self.observe_internal_result(&downstream_id, owner, &frame)?;
                Ok(None)
            }
            NativeRoute::ServerRequest { native_id, method, frame } => {
                let message = NativeFrame::parse(raw)?;
                let thread = message.params()?.and_then(|params| field::<String>(&params, "threadId"));
                self.controls.insert(native_id, (thread, method == "item/tool/requestUserInput"));
                Ok(Some(frame))
            }
            NativeRoute::Notification { method, frame } => {
                self.observe_notification(&method, &NativeFrame::parse(raw)?)?;
                Ok(self.handshake.forwards_notification(&method).then_some(frame))
            }
        }
    }

    fn observe_outer_result(&mut self, operation: ExternalOperation, raw: &str) -> Result<()> {
        let message = NativeFrame::parse(raw)?;
        let success = message.object.get("error").is_none();
        let result = child(&message.object, "result");
        if operation.method == "initialize" && success {
            if let Some(home) = result.as_ref().and_then(|result| field::<String>(result, "codexHome"))
                && std::path::Path::new(&home) != self.identity.canonical_home
            {
                self.supported = false;
                eprintln!("[agentx] initialize returned an unexpected native home; autonomous input disabled");
            }
            return Ok(());
        }
        if let Some(pending) = operation.root {
            if success {
                let thread = result.as_ref().and_then(|result| child(result, "thread")).context("Native root result has no thread")?;
                let thread_id = field::<String>(&thread, "id").filter(|id| !id.is_empty()).context("Native root result has no thread id")?;
                let key = NativeThreadKey { canonical_home: self.identity.canonical_home.to_string_lossy().into_owned(), thread_id: thread_id.clone() };
                let binding = CodexRootBindings::new(self.ports.attachments).attach(&self.identity.execution, &self.owner, pending.intent, key)?;
                self.ports.inbox.select_delivery_mode(&binding.attachment.target, DeliveryMode::NativeInput)?;
                let state = state_from_thread(&thread);
                let host = match self.ports.hosts.call(HostRequest::RegisterNativeRoot {
                    target: binding.attachment.target.clone(),
                    owner: self.owner.clone(),
                    wake_port: self.notify_port,
                    state: if state == ThreadState::Idle { NativeRootState::Idle } else { NativeRootState::Busy },
                })? {
                    HostResponse::Registered(registration) => registration,
                    _ => bail!("Unexpected native host registration response"),
                };
                self.events.push_back(DriverEvent::Ready { target: binding.attachment.target.clone() });
                self.roots.insert(thread_id.clone(), BoundRoot { binding, state, turn_id: None, suspended: false, host });
                if state != ThreadState::Idle {
                    self.request_state(&thread_id, None)?;
                }
            }
            return Ok(());
        }
        if let Some(thread) = operation.thread_id.as_deref() {
            if success && matches!(operation.method.as_str(), "thread/archive" | "thread/delete" | "thread/unsubscribe") {
                self.retire_thread(thread)?;
            } else if matches!(operation.method.as_str(), "turn/start" | "turn/steer") && success {
                if let Some(root) = self.roots.get_mut(thread) {
                    root.state = ThreadState::Active;
                    root.turn_id = result.as_ref().and_then(|result| child(result, "turn")).and_then(|turn| field(&turn, "id")).or_else(|| result.as_ref().and_then(|result| field(result, "turnId")));
                }
            } else if operation.mutation || operation.method == "thread/read" {
                if let Some(thread_data) = result.as_ref().and_then(|result| child(result, "thread")) {
                    self.observe_thread(thread, &thread_data);
                } else if self.roots.contains_key(thread) {
                    self.request_state(thread, None)?;
                }
            }
        } else if operation.mutation {
            let threads: Vec<String> = self.roots.keys().cloned().collect();
            for thread in threads {
                self.request_state(&thread, None)?;
            }
        }
        Ok(())
    }

    pub(super) fn observe_thread(&mut self, thread_id: &str, thread: &RawObject) {
        if let Some(root) = self.roots.get_mut(thread_id) {
            root.state = state_from_thread(thread);
            if root.state == ThreadState::Idle {
                root.turn_id = None;
            }
            if let Some(turns) = field::<Vec<Box<serde_json::value::RawValue>>>(thread, "turns") {
                for raw in turns {
                    if let Ok(turn) = RawObject::parse(raw.get())
                        && field::<String>(&turn, "status").as_deref() == Some("inProgress")
                    {
                        root.turn_id = field(&turn, "id");
                    }
                }
            }
        }
    }

    fn observe_notification(&mut self, method: &str, message: &NativeFrame) -> Result<()> {
        let Some(params) = message.params()? else {
            return Ok(());
        };
        if method == "serverRequest/resolved" {
            if let Some(raw_id) = params.get("requestId")
                && let Some((thread, _)) = self.controls.remove(&RpcId::from_raw(raw_id)?)
            {
                self.refresh_after_control(thread.as_deref())?;
            }
            return Ok(());
        }
        let Some(thread_id) = field::<String>(&params, "threadId") else {
            return Ok(());
        };
        if !self.roots.contains_key(&thread_id) {
            return Ok(());
        }
        match method {
            "thread/status/changed" => {
                if let Some(status) = child(&params, "status") {
                    let state = state_from_status(&status);
                    if state == ThreadState::Retired {
                        self.retire_thread(&thread_id)?;
                    } else if let Some(root) = self.roots.get_mut(&thread_id) {
                        root.state = state;
                        if state == ThreadState::Idle {
                            root.turn_id = None;
                        }
                    }
                }
            }
            "turn/started" => {
                if let Some(root) = self.roots.get_mut(&thread_id) {
                    root.state = ThreadState::Active;
                    root.turn_id = child(&params, "turn").and_then(|turn| field(&turn, "id"));
                }
            }
            "turn/completed" => {
                let turn = child(&params, "turn");
                let turn_id: Option<String> = turn.as_ref().and_then(|turn| field(turn, "id"));
                if let Some(root) = self.roots.get_mut(&thread_id)
                    && (root.turn_id.is_none() || root.turn_id == turn_id)
                {
                    root.state = ThreadState::Idle;
                    root.turn_id = None;
                }
                self.observe_turn_finished(&thread_id, turn_id)?;
            }
            "item/completed" => {
                if let Some(item) = child(&params, "item") {
                    self.observe_recorded_item(&thread_id, field(&params, "turnId"), &item)?;
                }
            }
            "hook/completed" => {
                if let Some(run) = child(&params, "run")
                    && field::<String>(&run, "eventName").as_deref() == Some("userPromptSubmit")
                    && matches!(field::<String>(&run, "status").as_deref(), Some("blocked" | "failed" | "stopped"))
                {
                    self.observe_blocked_input(&thread_id)?;
                }
            }
            "thread/closed" | "thread/archived" | "thread/deleted" => self.retire_thread(&thread_id)?,
            _ => {}
        }
        Ok(())
    }

    fn refresh_after_control(&mut self, thread_id: Option<&str>) -> Result<()> {
        let threads: Vec<String> = self.roots.keys().filter(|id| thread_id.is_none_or(|thread| thread == id.as_str())).cloned().collect();
        for thread in threads {
            self.request_state(&thread, None)?;
        }
        Ok(())
    }

    fn retire_thread(&mut self, thread_id: &str) -> Result<()> {
        if let Some(root) = self.roots.get_mut(thread_id) {
            root.state = ThreadState::Retired;
            root.suspended = true;
            self.ports.hosts.call(HostRequest::ReleaseNativeRoot(root.host.clone()))?;
            CodexRootBindings::new(self.ports.attachments).retire(&root.binding.attachment.target)?;
            self.events.push_back(DriverEvent::Retired { target: root.binding.attachment.target.clone() });
        }
        Ok(())
    }

    pub(crate) fn heartbeat(&mut self) -> Result<()> {
        for root in self.roots.values() {
            if self.ports.attachments.validate(&root.binding.attachment.target)? {
                self.ports.hosts.call(HostRequest::RenewNativeRoot(root.host.clone()))?;
            }
        }
        Ok(())
    }

    pub(crate) fn take_events(&mut self) -> Vec<DriverEvent> {
        self.events.drain(..).collect()
    }

    pub(crate) fn close(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        self.retain_unknown_attempts();
        let mut failure = None;
        for root in self.roots.values() {
            if let Err(error) = self.ports.hosts.call(HostRequest::ReleaseNativeRoot(root.host.clone()))
                && failure.is_none()
            {
                failure = Some(anyhow::Error::from(error));
            }
        }
        if let Err(error) = self.ports.attachments.retire_owner(&self.owner)
            && failure.is_none()
        {
            failure = Some(anyhow::Error::from(error));
        }
        self.router.close();
        if let Err(error) = self.ports.transport.close()
            && failure.is_none()
        {
            failure = Some(error.into());
        }
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(())
    }
}
