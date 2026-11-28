//! Bounded Linux IPC for the daemon's scope messaging role. The service never
//! accepts network addresses, HTTP paths, commands or credentials from callers.
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::daemon::{DaemonScope, DatabaseGeneration, ScopeLease, frame, platform};

use super::{
    Actor, Http, HttpError,
    ownership::{self, Record},
    port::{Command, Selection, checked_descriptor, heartbeat},
    qualified,
};

const VERSION: u32 = 1;
const REQUEST_BYTES: usize = 256 * 1024;
const RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_WORKERS: usize = 16;
const CONNECTION: Duration = Duration::from_secs(2);
const CALL: Duration = Duration::from_secs(21);

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Binding {
    pub local_generation: String,
    pub cloud_generation: String,
    pub installation_id: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Info {
    pub binding: Binding,
    pub host: String,
    pub daemon_instance: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    scope: PathBuf,
    binding: Option<Binding>,
    command: Command,
}

#[derive(Deserialize, Serialize)]
struct Failure {
    http: Option<u16>,
    code: Option<String>,
    message: String,
}

#[derive(Clone)]
pub(super) struct Client {
    scope: DaemonScope,
    pub info: Info,
}

impl Client {
    pub(super) fn connect(root: &Path, acquire: bool) -> Result<Self> {
        let scope = if acquire {
            let scope = DaemonScope::prepare(root)?;
            crate::daemon::acquire(&scope)?;
            scope
        } else {
            DaemonScope::existing(root, &std::env::current_dir()?)?
        };
        let deadline = Instant::now() + CONNECTION;
        let info = loop {
            match platform::connect_cloud(&scope, deadline) {
                Ok(stream) => break exchange(stream, &scope, None, Command::Info)?,
                Err(error) if acquire && Instant::now() < deadline && matches!(error, crate::daemon::DaemonError::Io { os_code: Some(libc::ENOENT | libc::ECONNREFUSED), .. }) => {
                    thread::sleep(Duration::from_millis(30));
                }
                Err(error) => return Err(error).context("cloud daemon IPC unavailable; check `agentx daemon status` and restart after enrollment"),
            }
        };
        Ok(Self { scope, info })
    }

    pub(super) fn root(&self) -> &Path {
        self.scope.root()
    }

    pub(super) fn call<T: DeserializeOwned>(&self, command: Command) -> Result<T> {
        let stream = platform::connect_cloud(&self.scope, Instant::now() + CONNECTION).context("connect to local cloud daemon IPC")?;
        exchange(stream, &self.scope, Some(self.info.binding.clone()), command)
    }
}

fn exchange<T: DeserializeOwned>(mut stream: platform::Stream, scope: &DaemonScope, binding: Option<Binding>, command: Command) -> Result<T> {
    // Session frames go only to a same-user kernel-authenticated local peer.
    let (_, identity) = platform::peer(&stream)?;
    ensure!(identity.is_some(), "cloud daemon peer has no process incarnation");
    let request = Request { version: VERSION, scope: scope.root().to_owned(), binding, command };
    frame::write_limited(&mut stream, &request, Instant::now() + CONNECTION, REQUEST_BYTES)?;
    match frame::read_limited::<Result<Value, Failure>>(&mut stream, Instant::now() + CALL, RESPONSE_BYTES)? {
        Ok(value) => Ok(serde_json::from_value(value)?),
        Err(Failure { http: Some(status), code, message }) => Err(HttpError { status, code: code.unwrap_or_else(|| "http_error".into()), message }.into()),
        Err(Failure { message, .. }) => bail!("{message}"),
    }
}

pub(crate) struct IpcServer {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl IpcServer {
    pub(crate) fn start(lease: &ScopeLease, config: &crate::cloud::connection::Config, host: &str, installation_id: &str) -> Result<Self> {
        let url = config.url.trim_end_matches('/').to_owned();
        let http = Http::new(&format!("{url}/api/v1"), &config.token)?;
        let listener = platform::Listener::bind_cloud(lease)?;
        let state =
            Arc::new(Server { root: lease.scope().root().to_owned(), daemon_instance: lease.instance().to_owned(), local_generation: lease.database_generation(), http, url, host: host.to_owned(), installation_id: installation_id.to_owned() });
        let stop = Arc::new(AtomicBool::new(false));
        let cancel = stop.clone();
        let thread = thread::Builder::new().name("cloud-local-ipc".into()).spawn(move || {
            let mut workers = Vec::<JoinHandle<()>>::new();
            while !cancel.load(Ordering::Acquire) {
                workers.retain_mut(|worker| !worker.is_finished());
                match listener.wait_ready(Instant::now() + Duration::from_millis(100)).and_then(|ready| if ready { listener.accept() } else { Ok(None) }) {
                    Ok(Some(stream)) if workers.len() < MAX_WORKERS => {
                        let state = state.clone();
                        let cancel = cancel.clone();
                        if let Ok(worker) = thread::Builder::new().name("cloud-local-call".into()).spawn(move || state.connection(stream, &cancel)) {
                            workers.push(worker);
                        }
                    }
                    Ok(Some(mut stream)) => {
                        let reply: Result<Value, Failure> = Err(Failure { http: Some(503), code: Some("busy".into()), message: "local cloud IPC is busy".into() });
                        let _ = frame::write_limited(&mut stream, &reply, Instant::now() + Duration::from_millis(10), RESPONSE_BYTES);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("Cloud IPC admission: {error}");
                        break;
                    }
                }
            }
            drop(listener);
            for worker in workers {
                let _ = worker.join();
            }
        })?;
        Ok(Self { stop, thread: Some(thread) })
    }

    pub(crate) fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        self.request_stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Server {
    root: PathBuf,
    daemon_instance: String,
    local_generation: DatabaseGeneration,
    http: Http,
    url: String,
    host: String,
    installation_id: String,
}

impl Server {
    fn connection(&self, mut stream: platform::Stream, cancel: &AtomicBool) {
        let response = (|| {
            let (pid, identity) = platform::peer(&stream)?;
            let identity = identity.context("local IPC peer process incarnation is unavailable")?;
            let request = frame::read_limited(&mut stream, Instant::now() + CONNECTION, REQUEST_BYTES)?;
            ensure!(!cancel.load(Ordering::Acquire), "cloud IPC daemon is stopping");
            self.dispatch(request, pid, &identity)
        })()
        .map_err(|error| {
            if let Some(http) = error.downcast_ref::<HttpError>() { Failure { http: Some(http.status), code: Some(http.code.clone()), message: http.message.clone() } } else { Failure { http: None, code: None, message: format!("{error:#}") } }
        });
        let _ = frame::write_limited(&mut stream, &response, Instant::now() + CONNECTION, RESPONSE_BYTES);
    }

    fn dispatch(&self, mut request: Request, pid: u32, identity: &str) -> Result<Value> {
        ensure!(request.version == VERSION && request.scope == self.root, "cloud IPC protocol or canonical scope mismatch");
        let local_generation = self.local_generation.current();
        ensure!(!local_generation.is_empty(), "native scope generation unavailable");
        if matches!(&request.command, Command::Info) {
            ensure!(request.binding.is_none(), "cloud IPC discovery cannot carry a binding");
            let cloud_generation = checked_descriptor(&self.http, &self.url)?.generation;
            return Ok(serde_json::to_value(Info { binding: Binding { local_generation, cloud_generation, installation_id: self.installation_id.clone() }, host: self.host.clone(), daemon_instance: self.daemon_instance.clone() })?);
        }
        let binding = request.binding.context("missing cloud IPC scope binding")?;
        ensure!(binding.local_generation == local_generation && binding.installation_id == self.installation_id, "stale native cloud IPC scope generation or installation");
        match &mut request.command {
            Command::Resolve(selection) => return Ok(serde_json::to_value(self.resolve(&binding, selection, pid, identity)?)?),
            Command::Claim(body) => {
                ensure!(body.limit == Some(1) && body.wait_ms.is_none_or(|ms| ms <= 1500), "cloud IPC permits one mailbox item and a bounded wait");
            }
            Command::Recover(body) => ensure!(!body.release, "cloud IPC does not release unresolved claims"),
            Command::Confirm(body) => ensure!(body.message_ids.len() == 1, "cloud IPC confirms one mailbox item at a time"),
            Command::Receipt { actor, .. } => ensure!(actor.is_some(), "cloud IPC receipt requires the exact sender"),
            _ => {}
        }
        if let Some((generation, installation, actor)) = request.command.actor() {
            ensure!(generation == binding.cloud_generation && installation == self.installation_id && actor.host == self.host, "stale cloud IPC actor generation or installation");
            let exact = ownership::record(&self.root, &actor.incarnation)?.context("cloud Agent has no exact local supervisor")?;
            ensure!(&exact.actor.agent == actor, "cloud IPC actor differs from supervisor identity");
            self.check_record(&binding, &exact)?;
            if matches!(request.command, Command::Register { .. } | Command::Retire(_)) {
                ensure!(exact.owns(pid, identity), "only the exact supervisor can change its cloud registration");
            } else if matches!(request.command, Command::PublishTranscript(_)) {
                ensure!(exact.session.is_some() && exact.owns(pid, identity), "only the ready exact supervisor can publish Agent transcript");
            } else {
                ensure!(exact.session.is_some(), "cloud Agent supervisor has not reported readiness");
                self.authorize_actor(&exact, pid, identity)?;
            }
        }
        ensure!(self.local_generation.current() == binding.local_generation, "native scope generation changed during cloud IPC admission");
        if let Command::Register { actor, daemon_instance } = &request.command {
            ensure!(daemon_instance.as_ref().is_none_or(|instance| *instance == self.daemon_instance), "daemon instance changed during cloud launch admission");
            // Messaging registration can be re-established after an outage.
            // This ordinary heartbeat cannot enable the management lease.
            heartbeat(&self.http, actor)?;
            ensure!(self.local_generation.current() == binding.local_generation, "native scope generation changed during registration");
        }
        request.command.upstream(&self.http)
    }

    fn check_record(&self, binding: &Binding, record: &Record) -> Result<()> {
        ensure!(
            record.live() && record.local_generation == binding.local_generation && record.actor.generation == binding.cloud_generation && record.actor.installation_id == self.installation_id && record.actor.agent.host == self.host,
            "local cloud supervisor is stopped or its scope binding changed"
        );
        Ok(())
    }

    fn resolve(&self, binding: &Binding, selection: &Selection, pid: u32, identity: &str) -> Result<Actor> {
        let exact = match selection {
            Selection::Bound(incarnation) => {
                let record = ownership::record(&self.root, incarnation)?.context("cloud tool has no exact local supervisor")?;
                let lineage = ownership::lineage(pid, identity)?;
                ensure!(lineage.iter().any(|(pid, id)| record.owns(*pid, id)), "cloud tool is not a descendant of its exact supervisor");
                record
            }
            Selection::Operator(name) => {
                let lineage = ownership::lineage(pid, identity)?;
                let records = ownership::records(&self.root)?;
                let found = if let Some(owner) = records.iter().find(|record| lineage.iter().any(|(pid, id)| record.owns(*pid, id))) {
                    ensure!(name.as_ref().is_none_or(|name| *name == owner.actor.agent.name || *name == qualified(&owner.actor.agent)), "cannot select another Agent from a hosted tool");
                    Some(owner)
                } else {
                    let name = name.as_deref().context("cloud send and listen require a launched Agent or --iam NAME for a live Agent on this host")?;
                    records.iter().find(|r| self.check_record(binding, r).is_ok() && (r.actor.agent.name == name || qualified(&r.actor.agent) == name))
                };
                found.cloned().context("no exact live cloud Agent for --iam on this host")?
            }
        };
        self.check_record(binding, &exact)?;
        ensure!(exact.session.is_some(), "cloud Agent supervisor has not reported readiness");
        Ok(exact.actor)
    }

    fn authorize_actor(&self, exact: &Record, pid: u32, identity: &str) -> Result<()> {
        let lineage = ownership::lineage(pid, identity)?;
        if lineage.iter().any(|(pid, identity)| exact.owns(*pid, identity)) {
            return Ok(());
        }
        // A same-user operator can select a live local Agent. A different
        // hosted Agent may not spoof operator selection by deleting its env.
        ensure!(!ownership::records(&self.root)?.iter().any(|owner| lineage.iter().any(|(pid, id)| owner.owns(*pid, id))), "cloud tool cannot act for another Agent");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::communication::types::{AgentView, RecoverInbox};
    use uuid::Uuid;

    fn endpoint(root: PathBuf) -> (Server, Binding) {
        let local_generation = DatabaseGeneration::default();
        let generation = Uuid::new_v4().to_string();
        local_generation.publish(generation.clone()).unwrap();
        let installation_id = Uuid::new_v4().to_string();
        let url = "http://127.0.0.1:1/scopes/test".to_owned();
        let http = Http::new(&format!("{url}/api/v1"), "test-bearer").unwrap();
        let binding = Binding { local_generation: generation, cloud_generation: Uuid::new_v4().to_string(), installation_id: installation_id.clone() };
        let server = Server { root, local_generation, daemon_instance: Uuid::new_v4().to_string(), http, url, host: "alpha".into(), installation_id };
        (server, binding)
    }

    fn dispatch(server: &Server, binding: &Binding, command: Command) -> Result<Value> {
        let pid = std::process::id();
        let identity = crate::sys::process::identity(pid).unwrap();
        server.dispatch(Request { version: VERSION, scope: server.root.clone(), binding: Some(binding.clone()), command }, pid, &identity)
    }

    #[test]
    fn bounded_vocabulary_and_generation_deny_invalid_work_before_upstream() {
        assert!(serde_json::from_value::<Command>(serde_json::json!({"op":"proxy","args":{"path":"/hosts/alpha/control/poll"}})).is_err());
        let root = tempfile::tempdir().unwrap();
        let (server, binding) = endpoint(root.path().to_owned());
        let mut stale = binding.clone();
        stale.local_generation = Uuid::new_v4().to_string();
        assert!(dispatch(&server, &stale, Command::Agents(None)).unwrap_err().to_string().contains("stale native"));

        let command = Command::Recover(RecoverInbox {
            generation: binding.cloud_generation.clone(),
            installation_id: binding.installation_id.clone(),
            agent: AgentView { name: "sender".into(), host: "alpha".into(), incarnation: Uuid::new_v4().to_string() },
            claim_id: Uuid::new_v4().to_string(),
            release: true,
        });
        assert!(dispatch(&server, &binding, command).unwrap_err().to_string().contains("unresolved claims"));
    }

    #[test]
    fn hosted_process_cannot_spoof_operator_selection() {
        let root = tempfile::tempdir().unwrap();
        let (server, binding) = endpoint(root.path().to_owned());
        let agent = AgentView { name: "sender".into(), host: server.host.clone(), incarnation: Uuid::new_v4().to_string() };
        let actor = Actor { generation: binding.cloud_generation.clone(), installation_id: binding.installation_id.clone(), agent: agent.clone() };
        let mut process = Record::start(actor, binding.local_generation.clone()).unwrap();
        process.session = Some("test-session".into());
        process.persist(root.path()).unwrap();

        let exact = dispatch(&server, &binding, Command::Resolve(Selection::Bound(agent.incarnation))).unwrap();
        assert_eq!(exact["agent"]["name"], "sender");
        let response = dispatch(&server, &binding, Command::Resolve(Selection::Operator(Some("another".into()))));
        assert!(response.unwrap_err().to_string().contains("cannot select another Agent"));
    }
}
