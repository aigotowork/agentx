//! Native cloud commands. The deployment owns registration and mailbox state;
//! only the local host owns Codex, its credentials, and its working directory.

#[cfg(target_os = "linux")]
mod host;
#[cfg(target_os = "linux")]
mod ipc;
#[cfg(target_os = "linux")]
mod ownership;
mod port;
#[cfg(target_os = "linux")]
pub(in crate::cloud) use host::{LaunchRejected, check_headless, launch_headless};
#[cfg(target_os = "linux")]
pub(crate) use ipc::IpcServer;
#[cfg(target_os = "linux")]
pub(in crate::cloud) use ownership::{OwnedProcess, exited_process, owned_process, stop_exact};
pub(super) mod http;
mod mailbox;

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{Write, stdout};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use clap::{CommandFactory, Parser};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::communication::types::{AgentView, ClaimInbox, ConfirmInbox, HostView, InboxBatch, InboxConfirmation, Intent, MessageOperation, MessageView, RecoverInbox, SendMessage};
use crate::cli::commands::{
    list::ListArgs,
    listen::ListenArgs,
    send::{self, SendArgs},
};
use crate::cli::router::{canonical_command, extract_global_flags_full};
use crate::messaging::routing::validate_message;
use http::{Http, HttpError};
use mailbox::{Mailbox, Phase};
use port::{Command, Port, Selection, checked_descriptor};

const SCOPE_URL: &str = "AGENTX_SCOPE_URL";
const SCOPE_TOKEN: &str = "AGENTX_SCOPE_TOKEN";
const HOST_NAME: &str = "AGENTX_HOST_NAME";
const CONTEXT_FILE: &str = "AGENTX_CLOUD_CONTEXT";
const IPC_CONTEXT: &str = "AGENTX_CLOUD_IPC_CONTEXT";
const HOST_ENTRY: &str = "__cloud-host";
const POLL: Duration = Duration::from_millis(350);

#[derive(Clone, Deserialize, Serialize)]
struct ManagedContext {
    root: PathBuf,
    incarnation: String,
}

enum Mode {
    Direct,
    Managed { root: PathBuf, context: Option<String> },
}

impl Mode {
    fn detect() -> Result<Option<Self>> {
        if let Some(path) = std::env::var_os(IPC_CONTEXT) {
            let context: ManagedContext = read_json_file(&PathBuf::from(path)).context("read local cloud IPC context")?;
            ensure!(context.root.is_absolute(), "cloud IPC context is missing an absolute scope path");
            return Ok(Some(Self::Managed { root: context.root, context: Some(context.incarnation) }));
        }
        if std::env::var_os(CONTEXT_FILE).is_some() {
            return Ok(Some(Self::Direct));
        }
        let env = std::env::vars().collect();
        let (root, _) = crate::paths::resolve_agentx_dir_from_env(&env, &std::env::current_dir()?);
        match fs::symlink_metadata(root.join("cloud.toml")) {
            Ok(_) => return Ok(Some(Self::Managed { root, context: None })),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("inspect scope cloud enrollment"),
        }
        Ok([SCOPE_URL, SCOPE_TOKEN].iter().any(|key| std::env::var_os(key).is_some()).then_some(Self::Direct))
    }

    fn scope(&self) -> Result<Scope> {
        match self {
            Self::Direct => Credentials::load()?.api(),
            #[cfg(target_os = "linux")]
            Self::Managed { root, context } => Scope::managed(root, context.is_none()),
            #[cfg(not(target_os = "linux"))]
            Self::Managed { .. } => bail!("cloud daemon IPC for local Codex hosting is supported on Linux"),
        }
    }

    fn resolve_actor(&self, explicit: Option<&str>, scope: &Scope) -> Result<Actor> {
        match self {
            Self::Direct => Credentials::load()?.resolve_actor(explicit, scope),
            Self::Managed { context, .. } => {
                let explicit = explicit.map(|name| name.trim_start_matches('@').to_owned());
                let selection = context.clone().map(Selection::Bound).unwrap_or_else(|| Selection::Operator(explicit.clone()));
                let actor: Actor = scope.port.call(Command::Resolve(selection))?;
                ensure!(explicit.is_none_or(|name| name == actor.agent.name || name == qualified(&actor.agent)), "--iam conflicts with the cloud Agent bound to this process");
                Ok(actor)
            }
        }
    }
}

pub(super) fn dispatch(argv: &[String]) -> Option<Result<()>> {
    if argv.first().is_some_and(|value| value == HOST_ENTRY) {
        #[cfg(target_os = "linux")]
        return Some(host::supervise());
        #[cfg(not(target_os = "linux"))]
        return Some(Err(anyhow::anyhow!("local Codex cloud hosting is supported on Linux")));
    }
    let (argv, flags, help) = extract_global_flags_full(argv);
    let (command, args) = argv.split_first()?;
    let canonical = canonical_command(command);
    if matches!(canonical, "daemon" | "help" | "--version" | "-v") {
        return None;
    }
    let mode = match Mode::detect() {
        Ok(mode) => mode?,
        Err(error) => return Some(Err(error)),
    };
    match canonical {
        "launch" => {
            #[cfg(target_os = "linux")]
            return Some(host::launch(args, help, flags.go, mode));
            #[cfg(not(target_os = "linux"))]
            return Some(Err(anyhow::anyhow!("local Codex cloud hosting is supported on Linux")));
        }
        "send" => Some(send_command(args, help, flags.name.as_deref(), mode)),
        "list" => Some(list_command(args, help, flags.name.as_deref(), mode)),
        "listen" => Some(listen_command(args, help, flags.name.as_deref(), mode)),
        _ if help => None,
        _ => Some(Err(anyhow::anyhow!("{command} is not available in a cloud scope; use launch, send, list or listen"))),
    }
}

#[derive(Serialize, Deserialize)]
struct Credentials {
    url: String,
    token: String,
    host: String,
    root: PathBuf,
    actor: Option<Actor>,
}

impl Credentials {
    fn load() -> Result<Self> {
        if let Some(path) = std::env::var_os(CONTEXT_FILE) {
            return read_json_file(&PathBuf::from(path)).context("read local cloud Agent context");
        }
        let url = std::env::var(SCOPE_URL).context("set AGENTX_SCOPE_URL for the cloud scope")?;
        let url = url.trim_end_matches('/').trim_end_matches("/api/v1").to_owned();
        let token = std::env::var(SCOPE_TOKEN).context("set AGENTX_SCOPE_TOKEN for the cloud scope")?;
        let environment = std::env::vars().collect();
        let (root, _) = crate::paths::resolve_agentx_dir_from_env(&environment, &std::env::current_dir()?);
        let host = std::env::var(HOST_NAME).ok().filter(|name| !name.is_empty()).unwrap_or_else(default_host);
        ensure!(
            !host.is_empty() && host.len() <= 253 && host.as_bytes()[0].is_ascii_alphanumeric() && host.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_.".contains(&byte)),
            "invalid cloud host name; use lowercase letters, digits, dots, hyphens or underscores, starting with a letter or digit"
        );
        let root = std::path::absolute(root)?.join("cloud-client");
        Ok(Self { url, token, host, root, actor: None })
    }

    fn api(&self) -> Result<Scope> {
        let api = Http::new(&format!("{}/api/v1", self.url), &self.token)?;
        let descriptor = checked_descriptor(&api, &self.url)?;
        Ok(Scope { port: Port::Direct(api), generation: descriptor.generation, state: self.state_dir() })
    }

    fn state_dir(&self) -> PathBuf {
        let key = URL_SAFE_NO_PAD.encode(Sha256::digest(format!("{}\0{}", self.url, self.host)));
        self.root.join(key)
    }

    fn resolve_actor(&self, explicit: Option<&str>, scope: &Scope) -> Result<Actor> {
        let explicit = explicit.map(|name| name.trim_start_matches('@'));
        if let Some(actor) = &self.actor {
            ensure!(explicit.is_none_or(|name| name == actor.agent.name || name == qualified(&actor.agent)), "--iam conflicts with the cloud Agent bound to this process");
            ensure!(actor.generation == scope.generation, "local cloud Agent belongs to another scope generation");
            return Ok(actor.clone());
        }
        let requested = explicit.context("cloud send and listen require a launched Agent or --iam NAME for a live Agent on this host")?;
        let name = if let Some((host, name)) = requested.split_once('/') {
            ensure!(host == self.host, "--iam must select a local cloud Agent");
            name
        } else {
            requested
        };
        valid_name(name)?;
        let lock = File::open(scope.state.join(format!("{name}.lock"))).context("no local cloud Agent with that name")?;
        match lock.try_lock() {
            Ok(()) => bail!("cloud Agent @{name} is not running on this host"),
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
        let local: LocalRun = read_json_file(&scope.actor_path(name)).context("read local cloud Agent registration")?;
        let actor = local.actor;
        ensure!(actor.generation == scope.generation && actor.agent.host == self.host && actor.agent.name == name, "local cloud Agent registration no longer matches this scope");
        Ok(actor)
    }
}

pub(super) fn default_host() -> String {
    let name = fs::read_to_string("/etc/hostname").ok().or_else(|| std::env::var("COMPUTERNAME").ok()).or_else(|| std::env::var("HOSTNAME").ok()).unwrap_or_else(|| "localhost".into());
    let sanitized: String = name.trim().to_ascii_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' }).take(63).collect();
    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() { "localhost".into() } else { sanitized.to_owned() }
}

fn valid_name(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    ensure!(!bytes.is_empty() && bytes.len() <= 64 && bytes.iter().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_".contains(byte)), "cloud Agent name must be 1–64 lowercase letters, digits, hyphens or underscores");
    Ok(())
}

fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_reader(File::open(path)?)?)
}

fn private_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn path_segment(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC).to_string()
}

#[derive(Clone, Serialize, Deserialize)]
struct Actor {
    generation: String,
    installation_id: String,
    agent: AgentView,
}

#[derive(Serialize, Deserialize)]
struct LocalRun {
    actor: Actor,
    session: Option<LocalSession>,
    private_dir: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
struct LocalSession {
    thread: String,
    cwd: PathBuf,
}

#[derive(Deserialize)]
struct Page<T> {
    items: Vec<T>,
    next: Option<String>,
}

struct Scope {
    port: Port,
    generation: String,
    state: PathBuf,
}

#[cfg(target_os = "linux")]
#[derive(Debug, thiserror::Error)]
#[error("cloud Agent @{0} is already reserved")]
struct NameTaken(String);

impl Scope {
    #[cfg(target_os = "linux")]
    fn managed(root: &Path, acquire: bool) -> Result<Self> {
        let client = ipc::Client::connect(root, acquire)?;
        let binding = &client.info.binding;
        let generation = binding.cloud_generation.clone();
        let state = client.root().join("cloud-client").join("managed").join(URL_SAFE_NO_PAD.encode(Sha256::digest(format!("{}\0{}\0{}\0{}", binding.cloud_generation, binding.installation_id, binding.local_generation, client.info.host))));
        Ok(Self { port: Port::Managed(client), generation, state })
    }

    fn pages<T: DeserializeOwned>(&self, agents: bool) -> Result<Vec<T>> {
        let mut cursor = None::<String>;
        let mut rows = Vec::new();
        loop {
            let command = if agents { Command::Agents(cursor.clone()) } else { Command::Hosts(cursor.clone()) };
            let page: Page<T> = self.port.call(command)?;
            rows.extend(page.items);
            let Some(next) = page.next else { return Ok(rows) };
            ensure!(cursor.as_ref().is_none_or(|after| after < &next), "cloud directory cursor did not advance");
            cursor = Some(next);
        }
    }

    fn actor_path(&self, name: &str) -> PathBuf {
        self.state.join(format!("{name}.json"))
    }

    #[cfg(target_os = "linux")]
    fn allocate_local(&self, origin: &LaunchOrigin) -> Result<Registration<'_>> {
        use std::collections::HashSet;

        let alive: HashSet<_> = self.pages::<AgentView>(true)?.into_iter().map(|agent| agent.name).collect();
        let mut collisions = HashSet::new();
        for _ in 0..8 {
            let name = crate::agents::names::allocate_name(&|name| alive.contains(name) || collisions.contains(name) || self.actor_path(name).exists(), &alive, 200, 1200, 30.0)?;
            match self.register_local(origin, &name, false) {
                Err(error) if error.downcast_ref::<NameTaken>().is_some() => {
                    collisions.insert(name);
                }
                result => return result,
            }
        }
        bail!("could not reserve an automatic cloud Agent name after concurrent launches; retry launch")
    }

    #[cfg(target_os = "linux")]
    fn register_local(&self, origin: &LaunchOrigin, name: &str, recover: bool) -> Result<Registration<'_>> {
        private_directory(&self.state)?;
        let path = self.state.join(format!("{name}.lock"));
        let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(NameTaken(name.into()).into()),
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
        let installation_id = installation_id(&origin.root)?;
        let local = self.actor_path(name);
        let previous = match read_json_file::<LocalRun>(&local) {
            Ok(previous) => Some(previous),
            Err(error) if error.downcast_ref::<std::io::Error>().is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) => None,
            Err(error) => return Err(error).context("read local cloud registration recovery record"),
        };
        let run = match previous {
            Some(previous) if previous.actor.generation == self.generation => {
                ensure!(recover, "cloud Agent @{name} retains its local incarnation; use `agentx launch --name {name} --recover`");
                ensure!(previous.actor.installation_id == installation_id && previous.actor.agent.host == origin.host && previous.actor.agent.name == name, "saved cloud Agent identity differs from this installation");
                previous
            }
            _ if recover => bail!("no recoverable local cloud Agent @{name} in this scope generation"),
            _ => LocalRun {
                actor: Actor { generation: self.generation.clone(), installation_id, agent: AgentView { name: name.into(), host: origin.host.clone(), incarnation: origin.incarnation.clone().unwrap_or_else(|| Uuid::new_v4().to_string()) } },
                session: None,
                private_dir: None,
            },
        };
        let mut guard = Registration { scope: self, run, ownership: None, retire_on_drop: false, _lock: lock };
        if let Port::Managed(client) = &self.port {
            let ownership = ownership::Record::start(guard.run.actor.clone(), client.info.binding.local_generation.clone())?;
            ownership.persist(client.root())?;
            guard.ownership = Some(ownership);
        } else {
            self.heartbeat(&guard.run.actor)?;
        }
        // Capture the exact attempted identity before registration so a lost
        // response cannot reserve a second incarnation.
        if !recover {
            guard.persist()?;
        }
        let register = || self.port.call::<AgentView>(Command::Register { actor: guard.run.actor.clone(), daemon_instance: origin.daemon_instance.clone() });
        let row = match retry_unknown(register) {
            Ok(row) => row,
            Err(error) => {
                if !recover && error.downcast_ref::<HttpError>().is_some_and(|error| error.status < 500) {
                    let _ = fs::remove_file(local);
                }
                if !recover && error.downcast_ref::<HttpError>().is_some_and(|error| error.code == "conflict") {
                    return Err(NameTaken(name.into()).into());
                }
                return Err(error).context("register cloud Agent");
            }
        };
        ensure!(row == guard.run.actor.agent, "cloud registration returned another Agent identity");
        guard.retire_on_drop = !recover;
        Ok(guard)
    }

    #[cfg(target_os = "linux")]
    fn heartbeat(&self, actor: &Actor) -> Result<()> {
        match &self.port {
            Port::Direct(http) => port::heartbeat(http, actor),
            Port::Managed(_) => Ok(()),
        }
    }

    fn claim(&self, actor: &Actor, id: &str, wait_ms: u32) -> Result<InboxBatch> {
        let command = || Command::Claim(ClaimInbox { generation: actor.generation.clone(), installation_id: actor.installation_id.clone(), agent: actor.agent.clone(), claim_id: id.into(), limit: Some(1), wait_ms: Some(wait_ms) });
        // Retrying the same claim ID cannot hand a different batch to the caller.
        let batch: InboxBatch = retry_unknown(|| self.port.call(command()))?;
        ensure!(batch.claim_id == id && batch.items.iter().all(|message| message.to == actor.agent), "cloud claim did not match the requested Agent and claim ID");
        Ok(batch)
    }

    fn recover(&self, actor: &Actor, claim_id: &str) -> Result<InboxBatch> {
        let batch: InboxBatch = self.port.call(Command::Recover(RecoverInbox { generation: actor.generation.clone(), installation_id: actor.installation_id.clone(), agent: actor.agent.clone(), claim_id: claim_id.into(), release: false }))?;
        ensure!(batch.claim_id == claim_id && batch.items.iter().all(|message| message.to == actor.agent), "cloud recovery did not match the requested Agent and claim ID");
        Ok(batch)
    }

    fn confirm(&self, actor: &Actor, claim_id: &str, message_id: &str) -> Result<()> {
        let result: InboxConfirmation = retry_unknown(|| {
            self.port.call(Command::Confirm(ConfirmInbox { generation: actor.generation.clone(), installation_id: actor.installation_id.clone(), agent: actor.agent.clone(), claim_id: claim_id.into(), message_ids: vec![message_id.into()] }))
        })?;
        ensure!(result.confirmed.iter().any(|id| id == message_id), "cloud confirmation omitted requested message {message_id}");
        Ok(())
    }
}

#[cfg(target_os = "linux")]
struct LaunchOrigin {
    root: PathBuf,
    host: String,
    incarnation: Option<String>,
    daemon_instance: Option<String>,
}

#[cfg(target_os = "linux")]
pub(crate) fn installation_id(root: &Path) -> Result<String> {
    let root = root.join("cloud-client");
    private_directory(&root)?;
    let path = root.join("installation-id");
    let id = match fs::read_to_string(&path) {
        Ok(id) => id,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let id = Uuid::new_v4().to_string();
            let mut pending = tempfile::NamedTempFile::new_in(&root)?;
            pending.write_all(id.as_bytes())?;
            pending.as_file().sync_all()?;
            match pending.persist_noclobber(&path) {
                Ok(_) => {
                    File::open(&root)?.sync_all()?;
                    id
                }
                Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => fs::read_to_string(&path)?,
                Err(error) => return Err(error.into()),
            }
        }
        Err(error) => return Err(error.into()),
    };
    Uuid::parse_str(&id).context("invalid local cloud installation ID")?;
    Ok(id)
}

#[cfg(target_os = "linux")]
struct Registration<'a> {
    scope: &'a Scope,
    run: LocalRun,
    ownership: Option<ownership::Record>,
    retire_on_drop: bool,
    _lock: File,
}

#[cfg(target_os = "linux")]
impl Registration<'_> {
    fn publish_session(&mut self, session_id: &str, ready: bool) -> Result<()> {
        if let (Some(record), Port::Managed(client)) = (&mut self.ownership, &self.scope.port) {
            record.session = Some(session_id.to_owned());
            record.ready = ready;
            record.persist(client.root())?;
        }
        Ok(())
    }

    fn persist(&self) -> Result<()> {
        let mut record = tempfile::NamedTempFile::new_in(&self.scope.state)?;
        serde_json::to_writer(&mut record, &self.run)?;
        record.as_file().sync_all()?;
        record.persist(self.scope.actor_path(&self.run.actor.agent.name))?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Drop for Registration<'_> {
    fn drop(&mut self) {
        if !self.retire_on_drop {
            return;
        }
        let actor = &self.run.actor;
        match retry_unknown(|| self.scope.port.call::<AgentView>(Command::Retire(actor.clone()))) {
            Ok(retired) if retired == actor.agent => {
                let _ = fs::remove_file(self.scope.actor_path(&actor.agent.name));
            }
            Ok(_) => eprintln!("Retire @{}: unexpected Agent identity", actor.agent.name),
            Err(error) => eprintln!("Retire @{}: {error:#}", actor.agent.name),
        }
    }
}

fn retry_unknown<T>(operation: impl Fn() -> Result<T>) -> Result<T> {
    operation().or_else(|error| if error.downcast_ref::<HttpError>().is_some_and(|error| error.status < 500) { Err(error) } else { operation() })
}

fn qualified(agent: &AgentView) -> String {
    format!("{}/{}", agent.host, agent.name)
}

fn message_text(message: &MessageView) -> String {
    let mut envelope = format!("{} #{}", message.intent.as_str(), message.id);
    if let Some(reply) = &message.reply_to {
        envelope.push_str(&format!(" reply-to #{reply}"));
    }
    if let Some(thread) = &message.thread {
        envelope.push_str(&format!(" thread {thread}"));
    }
    format!("[{envelope}] @{} -> @{}: {}", qualified(&message.from), qualified(&message.to), message.body)
}

fn print_json(value: &impl Serialize) -> Result<()> {
    let mut writer = stdout().lock();
    serde_json::to_writer(&mut writer, value)?;
    writeln!(writer)?;
    writer.flush()?;
    Ok(())
}

fn print_help<T: CommandFactory>() -> Result<()> {
    T::command().print_help()?;
    writeln!(stdout())?;
    Ok(())
}

#[derive(Parser)]
#[command(name = "send", about = "Send to an exact cloud Agent (@name or @host/name)")]
struct CloudSend {
    /// Stable ID for one recipient; reuse the ID after an unknown send result.
    #[arg(long, conflicts_with = "operation")]
    request_id: Option<String>,
    /// Look up a cloud send receipt by its request ID.
    #[arg(long)]
    operation: Option<String>,
    #[command(flatten)]
    options: SendArgs,
}

fn send_command(argv: &[String], help: bool, explicit: Option<&str>, mode: Mode) -> Result<()> {
    if help {
        return print_help::<CloudSend>();
    }
    let mut command = CloudSend::try_parse_from(std::iter::once("send").chain(argv.iter().map(String::as_str)))?;
    let args = &mut command.options;
    args.had_separator = argv.iter().any(|arg| arg == "--");
    ensure!(
        args.sender_name().is_none()
            && args.delivery_status.is_none()
            && args.dir.is_none()
            && args.workspace.is_none()
            && !args.dry_run
            && args.title.is_none()
            && args.description.is_none()
            && args.events.is_none()
            && args.files.is_none()
            && args.transcript.is_none()
            && args.extends.is_none(),
        "cloud send supports exact targets and registered senders; directory, relay, bundle and local delivery options are unavailable"
    );
    let scope = mode.scope()?;
    if let Some(request) = command.operation {
        ensure!(args.positionals.is_empty() && !args.had_separator && !args.stdin && args.file.is_none() && args.base64.is_none() && args.intent.is_none() && args.reply_to.is_none() && args.thread.is_none(), "--operation only reads a send receipt");
        let actor = if matches!(mode, Mode::Direct) { None } else { Some(mode.resolve_actor(explicit, &scope)?) };
        let receipt: MessageOperation = scope.port.call(Command::Receipt { request_id: request, actor })?;
        return print_json(&receipt);
    }
    let actor = mode.resolve_actor(explicit, &scope)?;
    let (targets, inline) = send::process_positionals(&args.positionals);
    let targets: Vec<_> = targets.into_iter().map(|name| format!("@{name}")).collect();
    ensure!(!targets.is_empty(), "cloud send requires an exact @name or @host/name target");
    ensure!(command.request_id.is_none() || targets.len() == 1, "--request-id requires one cloud recipient");
    ensure!(!args.had_separator || args.positionals.iter().all(|target| target.starts_with('@')), "only cloud targets may appear before --");
    let body = if inline.is_some() && !args.had_separator && !args.stdin && args.file.is_none() && args.base64.is_none() { inline.unwrap() } else { send::resolve_message(args).map_err(anyhow::Error::msg)? };
    validate_message(&body).map_err(anyhow::Error::msg)?;
    let intent = match args.intent.as_deref().unwrap_or("inform") {
        "request" => Intent::Request,
        "inform" => Intent::Inform,
        "ack" => Intent::Ack,
        _ => bail!("cloud message intent must be request, inform or ack"),
    };
    let mut receipts = Vec::new();
    for to in targets {
        let request_id = command.request_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        let send = || {
            Command::Send(SendMessage {
                generation: actor.generation.clone(),
                installation_id: actor.installation_id.clone(),
                request_id: request_id.clone(),
                from: actor.agent.clone(),
                to: to.clone(),
                body: body.clone(),
                intent,
                reply_to: args.reply_to.clone(),
                thread: args.thread.clone(),
            })
        };
        let sent: MessageView = match retry_unknown(|| scope.port.call(send())) {
            Ok(message) => message,
            Err(error) => {
                if args.json && !receipts.is_empty() {
                    print_json(&receipts)?;
                }
                return Err(error).with_context(|| format!("cloud send to {to}; request ID {request_id} (inspect with `agentx send --operation {request_id}`)"));
            }
        };
        if !args.json && !args.quiet {
            println!("Sent to {to} (#{}; request {request_id})", sent.id);
        }
        receipts.push(json!({"request_id":request_id,"message":sent}));
    }
    if args.json {
        if receipts.len() == 1 {
            print_json(&receipts[0])?;
        } else {
            print_json(&receipts)?;
        }
    }
    Ok(())
}

fn list_command(argv: &[String], help: bool, explicit: Option<&str>, mode: Mode) -> Result<()> {
    if help {
        return print_help::<ListArgs>();
    }
    let args = ListArgs::try_parse_from(std::iter::once("list").chain(argv.iter().map(String::as_str)))?;
    ensure!(
        !args.filters.has_workspace_selector() && args.filters.labels.is_empty() && !args.stopped && !args.sh && !args.verbose && args.format.is_none() && args.last.is_none(),
        "cloud list supports scope-wide directory, NAME, self, --names and --json"
    );
    let scope = mode.scope()?;
    let agents = scope.pages::<AgentView>(true)?;
    if let Some(name) = args.name.as_deref() {
        let exact_actor = if name == "self" { Some(mode.resolve_actor(explicit, &scope)?.agent) } else { None };
        let name = name.trim_start_matches('@');
        let row = agents.iter().find(|row| exact_actor.as_ref().map_or_else(|| row.name == name || qualified(row) == name, |actor| *row == actor)).context("cloud Agent not found")?;
        if let Some(field) = args.field.as_deref() {
            let value = match field {
                "name" => &row.name,
                "host" => &row.host,
                "incarnation" => &row.incarnation,
                _ => bail!("unknown cloud Agent field {field}"),
            };
            if args.json {
                print_json(value)?;
            } else {
                println!("{value}");
            }
        } else if args.json {
            print_json(row)?;
        } else if args.names {
            println!("{}", row.name);
        } else {
            println!("@{}", qualified(row));
        }
        return Ok(());
    }
    if args.json {
        print_json(&agents)?;
    } else if args.names {
        for row in agents {
            println!("{}", row.name);
        }
    } else {
        let hosts = scope.pages::<HostView>(false)?;
        for row in agents {
            let online = hosts.iter().any(|host| host.name == row.host && host.online);
            println!("@{}\t{}", qualified(&row), if online { "host online" } else { "host offline" });
        }
    }
    Ok(())
}

fn listen_command(argv: &[String], help: bool, explicit: Option<&str>, mode: Mode) -> Result<()> {
    if help {
        return print_help::<ListenArgs>();
    }
    let args = ListenArgs::try_parse_from(std::iter::once("listen").chain(argv.iter().map(String::as_str)))?;
    ensure!(args.sql.is_none() && args.filters.to_filter_map().is_empty(), "cloud listen selects only the exact Agent mailbox; local event filters are unavailable");
    let scope = mode.scope()?;
    let actor = mode.resolve_actor(explicit, &scope)?;
    let timeout = Duration::from_secs(args.timeout.or(args.timeout_positional).unwrap_or(86_400));
    let start = Instant::now();
    loop {
        if let Some(mailbox) = Mailbox::try_open(&scope, &actor)?
            && listen_once(&scope, &actor, &mailbox, args.json)?
        {
            return Ok(());
        }
        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            return Ok(());
        }
        std::thread::sleep(POLL.min(remaining));
    }
}

fn listen_once(scope: &Scope, actor: &Actor, mailbox: &Mailbox, json: bool) -> Result<bool> {
    let mut claim = match mailbox.read()? {
        Some(claim) => claim,
        None => mailbox.begin()?,
    };
    if mailbox.complete(scope, actor, &claim)? {
        return Ok(false);
    }
    match &claim.phase {
        Phase::ProviderIssuing(_) => return Ok(false),
        Phase::ListenIssuing(_) => bail!("output for cloud claim {} is unresolved; it will not be printed again", claim.id),
        Phase::Reserved => {}
        _ => unreachable!("completed phases returned above"),
    }
    let batch = scope.claim(actor, &claim.id, 0).with_context(|| format!("cloud claim {}", claim.id))?;
    let Some(message) = batch.items.first() else { return Ok(false) };
    mailbox.transition(&mut claim, Phase::ListenIssuing(message.id.clone()))?;
    if json {
        print_json(message)?;
    } else {
        let mut out = stdout().lock();
        writeln!(out, "{}", message_text(message))?;
        out.flush()?;
    }
    mailbox.transition(&mut claim, Phase::Confirming(message.id.clone()))?;
    mailbox.complete(scope, actor, &claim).with_context(|| format!("output written; cloud claim {} needs confirmation", claim.id))?;
    Ok(true)
}
