//! Renewable cloud control admission and serial local effects. HTTPS, the
//! effect journal, and the Provider startup thread have separate lifetimes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, Scope, ScopedJoinHandle};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::super::client::http::{Http, HttpError};
use super::super::client::{IpcServer, LaunchRejected, exited_process, installation_id, launch_headless, owned_process, stop_exact};
use super::super::communication::types::{AgentView, Descriptor};
use super::super::connection::{Config, publish_status};
use super::super::control::types::{ControlCommand, ControlOutcome, ControlPoll, ControlReceipt, ControlStatus, HostCapabilities, PollControlOperation, RenewControlLease, ReportControlOperation};
use super::super::directory::{DirectoryError, WorkingDirectory};
use super::capabilities::Catalogue;
use super::journal::{Entry, Failure, Journal, Outcome, Recovery};
use crate::daemon::ScopeLease;

const RENEW_EVERY: Duration = Duration::from_secs(5);
const LEASE_TTL: Duration = Duration::from_secs(30);
const EFFECT_TICK: Duration = Duration::from_millis(125);
const POLL_WAIT_MS: u32 = 5_000;
const POLL_PACE: Duration = Duration::from_millis(250);
const MAX_RETRY: Duration = Duration::from_secs(5);

pub(super) fn run(lease: &ScopeLease, stop: &AtomicBool, config: &Config) {
    let root = lease.scope().root();
    let mut status = Status { lease, last: None };
    status.set("connecting", None);
    let host = config.host_name();
    let Ok(installation) = installation_id(root) else {
        status.set("unavailable", Some("installation_identity_unavailable"));
        return;
    };
    // IPC also serves hosted processes while the cloud is temporarily offline.
    let Ok(ipc) = IpcServer::start(lease, config, &host, &installation) else {
        status.set("unavailable", Some("cloud_ipc_unavailable"));
        return;
    };
    let Ok(wire) = Wire::new(config, host, installation) else {
        status.set("unavailable", Some("transport_configuration_invalid"));
        return;
    };
    let journal = loop {
        match Journal::open(lease.scope().state()) {
            Ok(journal) => break journal,
            Err(_) => status.set("unavailable", Some("operation_journal_unavailable")),
        }
        if wait_for(stop, MAX_RETRY, || false) {
            return;
        }
    };
    let mut worker = Worker { lease, stop, config, wire, journal, status, session: None };
    thread::scope(|effects| {
        let mut active = None;
        let mut completed = None;
        let mut retry = Duration::from_secs(1);
        while !stop.load(Ordering::Acquire) {
            // Observe local completion even when the HTTPS service is offline.
            if active.as_ref().is_some_and(Active::is_finished) {
                completed = active.take().map(Active::complete);
            }
            let cycle = persist_completed(&worker.journal, &mut completed).and_then(|()| worker.cycle(effects, &mut active));
            let pause = match cycle {
                Ok(pause) => {
                    retry = Duration::from_secs(1);
                    pause
                }
                Err(error) => {
                    worker.note_issue(&error);
                    let delay = retry;
                    retry = (retry * 2).min(MAX_RETRY);
                    delay
                }
            };
            wait_for(stop, pause, || active.as_ref().is_some_and(Active::is_finished));
        }
        ipc.request_stop();
        worker.status.set("offline", Some("daemon_stopping"));
        // Do not discard an effect that was already in flight on shutdown.
        // IPC admission drains concurrently with the bounded in-flight effect.
        if let Some(active) = active.take() {
            completed = Some(active.complete());
        }
        let _ = persist_completed(&worker.journal, &mut completed);
    });
}

fn wait_for(stop: &AtomicBool, delay: Duration, interrupted: impl Fn() -> bool) -> bool {
    let until = Instant::now() + delay;
    while !stop.load(Ordering::Acquire) && Instant::now() < until && !interrupted() {
        thread::park_timeout(until.saturating_duration_since(Instant::now()).min(EFFECT_TICK));
    }
    stop.load(Ordering::Acquire)
}

struct Status<'a> {
    lease: &'a ScopeLease,
    last: Option<(&'static str, Option<&'static str>)>,
}

impl Status<'_> {
    fn set(&mut self, state: &'static str, reason: Option<&'static str>) {
        let next = (state, reason);
        if self.last != Some(next) && publish_status(self.lease.scope().root(), self.lease.instance(), state, reason).is_ok() {
            self.last = Some(next);
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct Fault(&'static str);

struct Worker<'a> {
    lease: &'a ScopeLease,
    stop: &'a AtomicBool,
    config: &'a Config,
    wire: Wire,
    journal: Journal,
    status: Status<'a>,
    session: Option<Session>,
}

impl<'a> Worker<'a> {
    fn cycle<'effect>(&mut self, effects: &'effect Scope<'effect, 'a>, active: &mut Option<Active<'effect>>) -> Result<Duration> {
        self.lease.ensure_ownership().context(Fault("local_lease_unavailable"))?;
        let native_generation = self.lease.database_generation().current();
        ensure!(!native_generation.is_empty(), Fault("local_lease_unavailable"));
        if self.session.as_ref().is_none_or(|session| session.native_generation != native_generation) {
            let cloud_generation = self.wire.descriptor()?.generation;
            self.session = Some(Session::new(&self.wire, &self.journal, native_generation, cloud_generation).context(Fault("operation_journal_unavailable"))?);
        }
        let session = self.session.as_mut().expect("control session constructed");
        if session.catalogue.is_none() || Instant::now() >= session.renew_at {
            let catalogue = Catalogue::discover(self.lease.scope().root(), &self.config.workspaces);
            let started = Instant::now();
            self.wire.renew(session, &catalogue)?;
            session.catalogue = Some(catalogue);
            session.renew_at = started + RENEW_EVERY;
            // The service's expiration clock has whole-second resolution.
            session.fresh_until = started + LEASE_TTL - Duration::from_secs(1);
            session.previous = None;
            self.status.set("online", None);
        }
        if self.stop.load(Ordering::Acquire) {
            return Ok(Duration::ZERO);
        }
        if active.is_some() {
            return Ok(EFFECT_TICK);
        }

        if let Some(pending) = self.journal.pending(&session.namespace).context(Fault("operation_journal_unavailable"))? {
            let outcome = match pending.entry {
                Entry::Finished(outcome) => outcome,
                Entry::Started(recovery) => {
                    let outcome = recover(self.lease, &self.wire, session, &pending.local_generation, recovery);
                    self.journal.finish(&session.namespace, &pending.id, &outcome).context(Fault("operation_journal_unavailable"))?;
                    outcome
                }
            };
            if let Err(error) = self.wire.report(session, &pending.id, &pending.claim, outcome)
                && !error.downcast_ref::<HttpError>().is_some_and(|http| http.status == 409 && http.code == "control_outcome_conflict")
            {
                return Err(error);
            }
            self.journal.mark_reported(&session.namespace, &pending.id).context(Fault("operation_journal_unavailable"))?;
            self.status.set("online", None);
            return Ok(Duration::ZERO);
        }

        if let Some(operation) = self.wire.poll(session)? {
            let context = EffectContext { lease: self.lease, stop: self.stop, fresh_until: session.fresh_until };
            *active = prepare_effect(context, effects, &self.journal, session, &self.wire, &self.config.workspaces, operation)?;
        }
        self.status.set("online", None);
        Ok(POLL_PACE)
    }

    fn note_issue(&mut self, error: &anyhow::Error) {
        let mut reason = error.downcast_ref::<Fault>().map_or("transport_retry", |fault| fault.0);
        if let Some(http) = error.downcast_ref::<HttpError>() {
            reason = match http.code.as_str() {
                "stale_lease" => {
                    // Before first acceptance, retry the same candidate (the
                    // previous lease may still be expiring). After expiry or
                    // replacement, an already used UUID cannot acquire again.
                    if self.session.as_ref().is_some_and(|s| s.catalogue.is_some() || s.created.elapsed() >= LEASE_TTL) {
                        self.session = None;
                    }
                    "lease_retry"
                }
                "stale_generation" => {
                    self.session = None;
                    "scope_generation_changed"
                }
                "host_conflict" => "installation_conflict",
                _ if matches!(http.status, 401 | 403) => "authorization_rejected",
                _ => "cloud_request_rejected",
            };
        }
        self.status.set("reconnecting", Some(reason));
    }
}

struct Session {
    namespace: String,
    cloud_generation: String,
    native_generation: String,
    id: String,
    previous: Option<String>,
    created: Instant,
    renew_at: Instant,
    fresh_until: Instant,
    catalogue: Option<Catalogue>,
}

impl Session {
    fn new(wire: &Wire, journal: &Journal, native_generation: String, cloud_generation: String) -> Result<Self> {
        let namespace = digest(&(&wire.base, &cloud_generation, &wire.installation, &wire.host))?;
        let id = Uuid::new_v4().to_string();
        let previous = journal.replace_lease(&namespace, &id)?;
        let created = Instant::now();
        Ok(Self { namespace, cloud_generation, native_generation, id, previous, created, renew_at: created, fresh_until: created, catalogue: None })
    }
}

struct Wire {
    http: Http,
    base: String,
    host_path: String,
    host: String,
    installation: String,
}

impl Wire {
    fn new(config: &Config, host: String, installation: String) -> Result<Self> {
        let base = config.url.trim_end_matches('/').to_owned();
        let http = Http::new(&format!("{base}/api/v1"), &config.token)?;
        let host_path = format!("/hosts/{}/control", segment(&host));
        Ok(Self { http, base, host_path, host, installation })
    }

    fn descriptor(&self) -> Result<Descriptor> {
        let descriptor: Descriptor = self.http.request("GET", "/descriptor", None)?;
        ensure!(descriptor.protocol_version == 1 && self.base.rsplit_once('/').is_some_and(|(_, id)| id == descriptor.scope_id), Fault("scope_descriptor_mismatch"));
        Ok(descriptor)
    }

    fn renew(&self, session: &Session, catalogue: &Catalogue) -> Result<()> {
        let body = RenewControlLease { generation: session.cloud_generation.clone(), installation_id: self.installation.clone(), lease_id: session.id.clone(), previous_lease_id: session.previous.clone(), workspaces: catalogue.workspaces.clone() };
        // Older deployments ignore the query, preserving compatibility with ordinary launch.
        let result: HostCapabilities = self.request("PUT", "/lease?directory_launch=true", &body)?;
        ensure!(self.matches(session, &result.generation, &result.host, &result.installation_id) && result.online && result.observed_at.is_some() && result.workspaces == catalogue.workspaces, Fault("lease_receipt_mismatch"));
        Ok(())
    }

    fn poll(&self, session: &Session) -> Result<Option<ControlReceipt>> {
        let body = PollControlOperation { generation: session.cloud_generation.clone(), installation_id: self.installation.clone(), lease_id: session.id.clone(), wait_ms: Some(POLL_WAIT_MS) };
        let result: ControlPoll = self.request("POST", "/poll", &body)?;
        if let Some(receipt) = &result.operation {
            ensure!(
                self.matches(session, &receipt.generation, &receipt.host, &receipt.installation_id) && receipt.status == ControlStatus::Running && receipt.claim_id.as_ref().is_some_and(|claim| !claim.is_empty()) && receipt.outcome.is_none(),
                Fault("claim_receipt_mismatch")
            );
        }
        Ok(result.operation)
    }

    fn report(&self, session: &Session, id: &str, claim: &str, outcome: Outcome) -> Result<()> {
        let outcome = match outcome {
            Outcome::Succeeded(agent) => ControlOutcome::Succeeded { agent },
            Outcome::Failed(failure) => ControlOutcome::Failed { code: failure.code().into() },
            Outcome::Unknown => ControlOutcome::Uncertain { code: "local_execution_unknown".into() },
        };
        let body = ReportControlOperation { generation: session.cloud_generation.clone(), installation_id: self.installation.clone(), lease_id: session.id.clone(), claim_id: claim.into(), outcome };
        let receipt: ControlReceipt = self.request("POST", &format!("/operations/{}/report", segment(id)), &body)?;
        ensure!(
            self.matches(session, &receipt.generation, &receipt.host, &receipt.installation_id) && receipt.request_id == id && receipt.claim_id.as_deref() == Some(claim) && receipt.outcome.as_ref() == Some(&body.outcome),
            Fault("report_receipt_mismatch")
        );
        Ok(())
    }

    fn request<T: DeserializeOwned>(&self, method: &str, suffix: &str, body: &impl Serialize) -> Result<T> {
        self.http.request(method, &format!("{}{suffix}", self.host_path), Some(&serde_json::to_value(body)?))
    }

    fn matches(&self, session: &Session, generation: &str, host: &str, installation: &str) -> bool {
        generation == session.cloud_generation && host == self.host && installation == self.installation
    }
}

fn digest(value: &impl Serialize) -> Result<String> {
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(serde_json::to_vec(value)?)))
}

fn segment(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC).to_string()
}

struct Active<'a> {
    namespace: String,
    id: String,
    effect: ScopedJoinHandle<'a, Outcome>,
}

impl Active<'_> {
    fn is_finished(&self) -> bool {
        self.effect.is_finished()
    }

    fn complete(self) -> Completed {
        let outcome = self.effect.join().unwrap_or(Outcome::Unknown);
        Completed { namespace: self.namespace, id: self.id, outcome }
    }
}

struct Completed {
    namespace: String,
    id: String,
    outcome: Outcome,
}

fn persist_completed(journal: &Journal, completed: &mut Option<Completed>) -> Result<()> {
    if let Some(result) = completed.as_ref() {
        journal.finish(&result.namespace, &result.id, &result.outcome).context(Fault("operation_journal_unavailable"))?;
        *completed = None;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct EffectContext<'a> {
    lease: &'a ScopeLease,
    stop: &'a AtomicBool,
    fresh_until: Instant,
}

impl EffectContext<'_> {
    fn refusal(&self, native_generation: &str) -> Option<Failure> {
        if self.stop.load(Ordering::Acquire) || self.lease.ensure_ownership().is_err() || Instant::now() >= self.fresh_until {
            return Some(Failure::LeaseNotCurrent);
        }
        if self.lease.database_generation().current() != native_generation {
            return Some(Failure::LocalGenerationChanged);
        }
        None
    }
}

fn prepare_effect<'effect, 'env>(
    context: EffectContext<'env>,
    threads: &'effect Scope<'effect, 'env>,
    journal: &Journal,
    session: &Session,
    wire: &Wire,
    exports: &std::collections::BTreeMap<String, std::path::PathBuf>,
    operation: ControlReceipt,
) -> Result<Option<Active<'effect>>> {
    let id = operation.request_id;
    let claim = operation.claim_id.context(Fault("claim_receipt_mismatch"))?;
    let fingerprint = digest(&operation.command)?;
    let ns = &session.namespace;
    if journal.seen(ns, &id, &fingerprint, &claim).context(Fault("operation_journal_unavailable"))? {
        return Ok(None);
    }
    let native_generation = &session.native_generation;
    let record = |recovery: &Recovery| journal.start(ns, &id, &fingerprint, &claim, native_generation, recovery).context(Fault("operation_journal_unavailable"));
    let finish = |outcome: &Outcome| journal.finish(ns, &id, outcome).context(Fault("operation_journal_unavailable"));
    let reject = |failure| {
        record(&Recovery::NoEffect)?;
        finish(&Outcome::Failed(failure))?;
        Ok(None)
    };
    if let Some(failure) = context.refusal(native_generation) {
        return reject(failure);
    }
    let root = context.lease.scope().root();
    let effect = match operation.command {
        ControlCommand::Launch { workspace, role, name, title, initial_prompt, working_directory, create_directory } => {
            let Some(workspace) = session.catalogue.as_ref().and_then(|catalogue| catalogue.recheck(root, exports, &workspace, role.as_deref())) else {
                return reject(Failure::CapabilityChanged);
            };
            let incarnation = Uuid::new_v4().to_string();
            record(&Recovery::Launch { incarnation: incarnation.clone(), name: name.clone() })?;
            Effect::Launch {
                workspace,
                working_directory,
                create_directory: create_directory.unwrap_or_default(),
                role,
                name,
                title,
                initial_prompt,
                incarnation,
                host: wire.host.clone(),
                generation: session.cloud_generation.clone(),
                installation: wire.installation.clone(),
            }
        }
        ControlCommand::Stop { name, incarnation } => {
            let agent = AgentView { name, incarnation, host: wire.host.clone() };
            let exact = owned_process(root, &agent.incarnation).ok().flatten().is_some_and(|process| process.agent == agent);
            if !exact {
                return reject(Failure::StopRejected);
            }
            record(&Recovery::Stop { agent: agent.clone() })?;
            Effect::Stop(agent)
        }
    };
    let native_generation = native_generation.clone();
    let spawned = thread::Builder::new().name("agentx-cloud-effect".into()).spawn_scoped(threads, move || {
        if let Some(failure) = context.refusal(&native_generation) {
            return Outcome::Failed(failure);
        }
        effect.execute(root, context.lease.instance(), &native_generation, context.stop)
    });
    match spawned {
        Ok(effect) => Ok(Some(Active { namespace: ns.clone(), id, effect })),
        Err(_) => {
            finish(&Outcome::Failed(Failure::WorkerUnavailable))?;
            Ok(None)
        }
    }
}

enum Effect {
    Launch {
        workspace: std::path::PathBuf,
        working_directory: Option<String>,
        create_directory: bool,
        role: Option<String>,
        name: Option<String>,
        title: Option<String>,
        initial_prompt: Option<String>,
        incarnation: String,
        host: String,
        generation: String,
        installation: String,
    },
    Stop(AgentView),
}
impl Effect {
    fn execute(self, root: &std::path::Path, daemon_instance: &str, native_generation: &str, stop: &AtomicBool) -> Outcome {
        match self {
            Self::Launch { workspace, working_directory, create_directory, role, name, title, initial_prompt, incarnation, host, generation, installation } => {
                let directory = if working_directory.is_some() || create_directory {
                    match WorkingDirectory::open(&workspace, working_directory.as_deref().unwrap_or_default(), create_directory) {
                        Ok(directory) => Some(directory),
                        Err(error) => {
                            return Outcome::Failed(match error {
                                DirectoryError::Invalid => Failure::DirectoryInvalid,
                                DirectoryError::Missing => Failure::DirectoryMissing,
                                DirectoryError::Unsafe => Failure::DirectoryUnsafe,
                                DirectoryError::Unavailable => Failure::DirectoryUnavailable,
                                DirectoryError::CreateFailed => Failure::DirectoryCreateFailed,
                            });
                        }
                    }
                } else {
                    None
                };
                let requested_name = name.clone();
                match launch_headless(root, &workspace, directory.as_ref(), role.as_deref(), name, initial_prompt, title, &incarnation, Some((&generation, &installation, daemon_instance, native_generation)), stop) {
                    Ok(process) if process.agent.host == host && process.agent.incarnation == incarnation && requested_name.is_none_or(|name| name == process.agent.name) => Outcome::Succeeded(process.agent),
                    Err(error) if error.is::<LaunchRejected>() => Outcome::Failed(Failure::LaunchRejected),
                    _ => Outcome::Unknown,
                }
            }
            Self::Stop(agent) => match stop_exact(root, &agent) {
                Ok(true) => Outcome::Succeeded(agent),
                Ok(false) => Outcome::Failed(Failure::StopRejected),
                Err(_) => Outcome::Unknown,
            },
        }
    }
}

// After restart the journal proves an effect was authorized, but not that it
// ran. Reconcile exact existing evidence only; never run an effect from here.
fn recover(lease: &ScopeLease, wire: &Wire, session: &Session, native_generation: &str, recovery: Recovery) -> Outcome {
    if lease.database_generation().current() != native_generation {
        return Outcome::Unknown;
    }
    let root = lease.scope().root();
    match recovery {
        Recovery::Launch { incarnation, name } => match owned_process(root, &incarnation) {
            Ok(Some(process)) if process.agent.host == wire.host && process.agent.incarnation == incarnation && name.is_none_or(|name| name == process.agent.name) => Outcome::Succeeded(process.agent),
            _ => Outcome::Unknown,
        },
        // Completion after restart requires positive kernel exit evidence
        // for the same supervisor record. Missing evidence stays uncertain.
        Recovery::Stop { agent } => match exited_process(root, &agent, (native_generation, &session.cloud_generation, &wire.installation)) {
            Ok(true) if agent.host == wire.host => Outcome::Succeeded(agent),
            _ => Outcome::Unknown,
        },
        Recovery::NoEffect => Outcome::Unknown,
    }
}
