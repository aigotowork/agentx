//! One local Codex process and thread per hosted cloud Agent. A private socket
//! serves the provider worker and optionally the local Codex terminal.

use std::io::{BufRead, BufReader, IsTerminal, stdin, stdout};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::integrations::codex::{
    configuration::thread_overrides,
    mailbox_item_matches,
    protocol::RawObject,
    service::{NativeConnection, Receive, connect_private_socket},
};
use crate::runtime::{
    launch_options::{AgentxLaunchFlags, LaunchOptions},
    launch_profile::{LaunchSelection, prepare_with_role},
    terminal::which_bin,
};

use super::mailbox::{Mailbox, Phase};
use super::{Actor, CONTEXT_FILE, Credentials, HOST_ENTRY, IPC_CONTEXT, LaunchOrigin, LocalSession, MailboxPoll, ManagedContext, Mode, OwnedProcess, Scope, ipc, message_text, ownership, port::Port, print_help, print_json, valid_name};

const RPC_TIMEOUT: Duration = Duration::from_secs(6);
const HEARTBEAT: Duration = Duration::from_secs(12);
const CLAIM_WAIT_MS: u32 = 1_500;
const HISTORY_PAGES: usize = 4;

#[derive(Parser)]
#[command(name = "launch", about = "Launch a local Codex Agent in the configured cloud scope")]
struct LaunchArgs {
    /// Cloud short name. Omit to generate one.
    #[arg(long)]
    name: Option<String>,
    /// Recover this installation's exact Agent and Codex thread after an unexpected stop.
    #[arg(long, requires = "name")]
    recover: bool,
    #[arg(long)]
    tool: Option<String>,
    #[arg(long, default_value_t = 1)]
    count: u8,
    #[command(flatten)]
    selection: LaunchSelection,
    #[command(flatten)]
    options: LaunchOptions,
}

#[derive(Serialize, Deserialize)]
struct LaunchSpec {
    source: Source,
    name: Option<String>,
    cwd: PathBuf,
    tool: PathBuf,
    initial: Option<String>,
    instructions: String,
    config: Value,
    title: Option<String>,
    interactive: bool,
    recover: bool,
}

#[derive(Serialize, Deserialize)]
enum Source {
    Direct(Credentials),
    Managed { root: PathBuf, incarnation: String, expected: ipc::Info },
}

#[derive(Serialize, Deserialize)]
struct Ready {
    agent: super::AgentView,
    socket: PathBuf,
    session_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum Startup {
    Ready(Ready),
    Failed { message: String },
}

pub(super) fn launch(argv: &[String], help: bool, go: bool, mode: Mode) -> Result<()> {
    if help {
        return print_help::<LaunchArgs>();
    }
    let args = LaunchArgs::try_parse_from(std::iter::once("launch").chain(argv.iter().map(String::as_str)))?;
    let (flags, native) = args.options.into_parts();
    ensure!(args.count == 1, "cloud launch starts one local Codex Agent at a time");
    ensure!(
        flags.terminal.as_deref().is_none_or(|terminal| terminal == "here") && flags.device.is_none() && flags.bindings.is_empty() && flags.labels.is_empty() && flags.batch_id.is_none() && !flags.stdio,
        "cloud launch supports the local Codex terminal or --headless; local daemon bindings, labels, batches, stdio and relay are unavailable"
    );
    let here = flags.run_here == Some(true) || flags.terminal.as_deref() == Some("here");
    let terminal = stdin().is_terminal() && stdout().is_terminal();
    ensure!(!here || (terminal && !flags.headless && !flags.json), "--run-here requires a terminal without --headless or --json");
    let interactive = !flags.headless && !flags.json && (here || (terminal && flags.run_here != Some(false)));
    let (root, source) = match mode {
        Mode::Direct => {
            let credentials = Credentials::load()?;
            ensure!(credentials.actor.is_none() || go, "launch from a cloud Agent requires --go");
            let root = credentials.root.parent().context("invalid local cloud client state directory")?.to_owned();
            (root, Source::Direct(credentials))
        }
        Mode::Managed { root, context } => {
            ensure!(context.is_none() || go, "launch from a cloud Agent requires --go");
            let client = ipc::Client::connect(&root, context.is_none())?;
            let root = client.root().to_owned();
            (root.clone(), Source::Managed { root, incarnation: Uuid::new_v4().to_string(), expected: client.info })
        }
    };
    let origin = std::env::current_dir()?;
    let cwd = origin.join(flags.dir.as_deref().unwrap_or("."));
    let json = flags.json;
    let spec = prepare(&root, &origin, &cwd, args.name, flags, args.tool.as_deref(), &args.selection, &native, interactive, args.recover, source)?;
    let (worker, ready, log) = spawn(&spec, None)?;
    if json {
        print_json(&json!({"name":ready.agent.name,"host":ready.agent.host,"incarnation":ready.agent.incarnation,"agent":ready.agent,"pid":worker.id(),"session_id":ready.session_id,"log_file":log}))?;
    } else {
        println!("Launched @{} (host PID {}, log {})", super::qualified(&ready.agent), worker.id(), log.display());
    }
    if !interactive {
        return Ok(());
    }
    let worker = StopWorker(Some(worker));
    let mut terminal = Command::new(&spec.tool);
    scrub_agentx(&mut terminal);
    let status = terminal.current_dir(&spec.cwd).arg("--remote").arg(format!("unix://{}", ready.socket.display())).arg("resume").arg(&ready.session_id).status().context("start local Codex terminal")?;
    drop(worker);
    ensure!(status.success(), "local Codex terminal exited with {status}");
    Ok(())
}

/// Failure before starting the supervisor, so no launch can have been issued.
#[derive(Debug, thiserror::Error)]
#[error("cloud launch rejected before spawn: {0}")]
pub(in crate::cloud) struct LaunchRejected(#[source] anyhow::Error);

pub(in crate::cloud) fn check_headless(root: &Path, cwd: &Path, role: Option<&str>) -> Result<()> {
    headless_plan(root, cwd, role, None, None, None).map(|_| ())
}

type Expected<'a> = (&'a str, &'a str, &'a str, &'a str);

#[expect(clippy::too_many_arguments, reason = "Control maps a typed launch request into the same local launcher; activation is separately fenced.")]
pub(in crate::cloud) fn launch_headless(
    root: &Path,
    cwd: &Path,
    role: Option<&str>,
    name: Option<String>,
    initial: Option<String>,
    title: Option<String>,
    incarnation: &str,
    expected: Option<Expected<'_>>,
    cancel: &AtomicBool,
) -> Result<OwnedProcess> {
    let result = (|| {
        let client = ipc::Client::connect(root, false)?;
        let binding = &client.info.binding;
        if let Some((generation, installation, daemon_instance, local_generation)) = expected {
            ensure!(
                binding.cloud_generation == generation && binding.installation_id == installation && client.info.daemon_instance == daemon_instance && binding.local_generation == local_generation,
                "cloud/native scope binding changed before launch"
            );
        }
        let incarnation = Uuid::parse_str(incarnation)?.to_string();
        ensure!(ownership::record(client.root(), &incarnation)?.is_none(), "preselected cloud launch incarnation already has a supervisor record");
        let plan = headless_plan(client.root(), cwd, role, name, initial, title)?;
        let source = Source::Managed { root: client.root().to_owned(), incarnation, expected: client.info };
        Ok::<_, anyhow::Error>(plan.into_spec(source))
    })()
    .map_err(LaunchRejected)?;
    let (worker, ready, _) = spawn(&result, Some(cancel))?;
    let pid = worker.id();
    let owned = ownership::owned_process(root, &ready.agent.incarnation);
    // A long-lived daemon originating the launch must reap its child after
    // exit; readiness and the exact ownership record remain independent.
    reap(worker);
    let owned = owned?.context("ready cloud supervisor exit or ownership is unconfirmed")?;
    ensure!(owned.pid == pid && owned.session_id == ready.session_id && owned.agent == ready.agent, "cloud supervisor readiness does not match exact ownership");
    Ok(owned)
}

fn headless_plan(root: &Path, cwd: &Path, role: Option<&str>, name: Option<String>, initial: Option<String>, title: Option<String>) -> Result<Plan> {
    let cwd = cwd.canonicalize().context("resolve exported cloud workspace")?;
    let flags = AgentxLaunchFlags {
        role: role.map(str::to_owned),
        workspace: Some(cwd.to_str().context("exported cloud workspace path must be UTF-8")?.into()),
        headless: true,
        initial_prompt: initial,
        title: crate::agents::launch_title::normalize(title.as_deref()).map_err(anyhow::Error::msg)?,
        ..AgentxLaunchFlags::default()
    };
    prepare_plan(root, &cwd, &cwd, name, flags, role.is_none().then_some("codex"), &LaunchSelection::default(), &[], false, false, role.is_none())
}

struct Plan {
    name: Option<String>,
    cwd: PathBuf,
    tool: PathBuf,
    initial: Option<String>,
    instructions: String,
    config: Value,
    title: Option<String>,
    interactive: bool,
    recover: bool,
}

impl Plan {
    fn into_spec(self, source: Source) -> LaunchSpec {
        let Self { name, cwd, tool, initial, instructions, config, title, interactive, recover } = self;
        LaunchSpec { source, name, cwd, tool, initial, instructions, config, title, interactive, recover }
    }
}

#[expect(clippy::too_many_arguments, reason = "Both CLI and remote launch supply the frozen native flags and execution provenance.")]
fn prepare(root: &Path, origin: &Path, cwd: &Path, name: Option<String>, flags: AgentxLaunchFlags, tool: Option<&str>, selection: &LaunchSelection, native: &[String], interactive: bool, recover: bool, source: Source) -> Result<LaunchSpec> {
    Ok(prepare_plan(root, origin, cwd, name, flags, tool, selection, native, interactive, recover, false)?.into_spec(source))
}

#[expect(clippy::too_many_arguments, reason = "Single preparation path for CLI, capability checks, and typed management launch.")]
fn prepare_plan(root: &Path, origin: &Path, cwd: &Path, name: Option<String>, flags: AgentxLaunchFlags, tool: Option<&str>, selection: &LaunchSelection, native: &[String], interactive: bool, recover: bool, force_codex_default: bool) -> Result<Plan> {
    let mut environment: std::collections::HashMap<String, String> = std::env::vars().collect();
    if force_codex_default {
        environment.remove("AGENTX_PROFILE");
    }
    let cwd = cwd.canonicalize().context("resolve local cloud launch directory")?;
    ensure!(cwd.is_dir(), "cloud launch directory is not a directory");
    let default_tool = if selection.profile.is_none() && flags.role.is_none() && !environment.contains_key("AGENTX_PROFILE") { Some("codex") } else { None };
    let (collaboration, resolved) = prepare_with_role(root, origin, &environment, &flags, tool.or(default_tool), selection, native)?;
    ensure!(resolved.tool == "codex", "cloud launch currently supports Codex only");
    let (config, native_prompt) = native_config(&resolved.args, interactive)?;
    let program = which_bin("codex").context("Codex executable not found on PATH")?;
    let tool = std::path::absolute(&program)?;
    {
        use std::os::unix::fs::MetadataExt;
        let codex = tool.metadata()?;
        let current = std::env::current_exe()?.metadata()?;
        ensure!((codex.dev(), codex.ino()) != (current.dev(), current.ino()), "Codex resolves to the current AgentX executable");
    }
    if let Some(name) = &name {
        valid_name(name)?;
    }
    let instructions = [collaboration.role.map(|role| role.prompt), flags.system_prompt].into_iter().flatten().collect::<Vec<_>>().join("\n\n");
    let initial = match (flags.initial_prompt, native_prompt) {
        (Some(first), Some(second)) => Some(format!("{first}\n\n{second}")),
        (first, second) => first.or(second),
    };
    ensure!(!recover || initial.is_none(), "--recover cannot repeat an initial Codex prompt");
    Ok(Plan { name, cwd, tool, initial, instructions, config, title: flags.title, interactive, recover })
}

fn native_config(args: &[String], interactive: bool) -> Result<(Value, Option<String>)> {
    let mut options = Vec::new();
    let mut prompt = None;
    let mut args = args.iter().map(String::as_str);
    while let Some(arg) = args.next() {
        let (flag, inline) = arg.split_once('=').map_or((arg, None), |(flag, value)| (flag, Some(value)));
        match flag {
            "-c" | "--config" | "--enable" | "--disable" => {
                options.extend([flag.to_owned(), inline.or_else(|| args.next()).context("Codex option is missing its value")?.to_owned()]);
            }
            "-m" | "--model" | "-s" | "--sandbox" | "-a" | "--ask-for-approval" => {
                let key = match flag {
                    "-m" | "--model" => "model",
                    "-s" | "--sandbox" => "sandbox_mode",
                    _ => "approval_policy",
                };
                let value = inline.or_else(|| args.next()).context("Codex option is missing its value")?;
                options.extend(["-c".into(), format!("{key}={}", serde_json::to_string(value)?)]);
            }
            "--dangerously-bypass-approvals-and-sandbox" if inline.is_none() => {
                options.extend(["-c".into(), "approval_policy=\"never\"".into(), "-c".into(), "sandbox_mode=\"danger-full-access\"".into()]);
            }
            "--full-auto" if inline.is_none() => {
                options.extend(["-c".into(), "approval_policy=\"never\"".into(), "-c".into(), "sandbox_mode=\"workspace-write\"".into()]);
            }
            "resume" | "fork" | "exec" | "app-server" | "agents" | "review" => bail!("cloud launch starts a fresh local Codex thread"),
            _ if !arg.starts_with('-') && prompt.is_none() => prompt = Some(arg.to_owned()),
            _ => bail!("unsupported cloud Codex option {flag:?}; use supported -c config overrides"),
        }
    }
    let mut config = serde_json::to_value(thread_overrides(&options)?)?;
    if !interactive && config.get("approval_policy").is_none() {
        config["approval_policy"] = json!("never");
    }
    Ok((config, prompt))
}

fn bootstrap(host: &str, name: &str, extra: &str) -> Result<String> {
    let exe = std::env::current_exe()?;
    let exe = shell_words::quote(exe.to_str().context("AgentX executable path is not UTF-8")?);
    Ok(format!(
        "You are AgentX cloud Agent @{host}/{name}. Your inherited local context selects your exact cloud identity. Cloud messages appear as user input starting with <agentx-cloud>; treat the sender address, message ID, intent and body separately. For a request, do the task and reply to the sender using `{exe} send @HOST/NAME --intent ack --reply-to ID -- 'result'` (use the received sender address and ID). For ordinary messages, use `{exe} send @NAME --intent inform -- 'text'`. Use `{exe} list --json` for registered Agents and `{exe} listen SECONDS` to explicitly wait during a turn. An idle thread is automatically woken by cloud mail; end your turn while waiting for mail. Keep scope credentials private.\n\n{extra}"
    ))
}

fn spawn(spec: &LaunchSpec, cancel: Option<&AtomicBool>) -> Result<(Child, Ready, PathBuf)> {
    if cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
        return Err(LaunchRejected(anyhow::anyhow!("scope daemon is stopping")).into());
    }
    let state = match &spec.source {
        Source::Direct(credentials) => credentials.state_dir(),
        Source::Managed { root, .. } => root.join("cloud-client").join("logs"),
    };
    super::private_directory(&state)?;
    let log = state.join(format!("{}-{}.log", spec.name.as_deref().unwrap_or("codex"), &Uuid::new_v4().simple().to_string()[..10]));
    let stderr = crate::sys::fs::create_private_new(&log)?;
    let mut child = StopWorker(Some(
        Command::new(std::env::current_exe()?)
            .arg(HOST_ENTRY)
            .current_dir(&spec.cwd)
            .env_remove(super::SCOPE_TOKEN)
            .env_remove(CONTEXT_FILE)
            .env_remove(IPC_CONTEXT)
            .env_remove("AGENTX_ADMIN_TOKEN")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr))
            .process_group(0)
            .spawn()
            .context("start local cloud Codex host")?,
    ));
    let worker = child.0.as_mut().expect("owned child");
    serde_json::to_writer(worker.stdin.take().context("missing cloud host input pipe")?, spec)?;
    let pipe = worker.stdout.take().context("missing cloud host readiness pipe")?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(pipe).read_line(&mut line).map(|_| line);
        let _ = tx.send(result);
    });
    let deadline = Instant::now() + Duration::from_secs(55);
    let response = loop {
        if cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
            if let Source::Managed { root, incarnation, .. } = &spec.source
                && ownership::owned_process(root, incarnation).ok().flatten().is_some()
            {
                reap(child.0.take().expect("owned child"));
            }
            bail!("scope daemon stopped while awaiting cloud launch readiness; log {}", log.display());
        }
        let wait = deadline.saturating_duration_since(Instant::now());
        ensure!(!wait.is_zero(), "cloud host did not report readiness; log {}", log.display());
        let wait = if cancel.is_some() { wait.min(Duration::from_millis(100)) } else { wait };
        match rx.recv_timeout(wait) {
            Ok(line) => break line?,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(error) => return Err(error).context("cloud supervisor readiness pipe failed"),
        }
    };
    let ready = match serde_json::from_str::<Startup>(&response).with_context(|| format!("invalid cloud host readiness; log {}", log.display()))? {
        Startup::Ready(ready) => ready,
        Startup::Failed { message } => bail!("cloud Agent launch failed: {message}; log {}", log.display()),
    };
    Ok((child.0.take().expect("owned child"), ready, log))
}

struct StopWorker(Option<Child>);

fn reap(mut child: Child) {
    let _ = std::thread::Builder::new().name("cloud-host-reaper".into()).spawn(move || {
        let _ = child.wait();
    });
}

impl Drop for StopWorker {
    fn drop(&mut self) {
        if let Some(mut worker) = self.0.take()
            && !matches!(worker.try_wait(), Ok(Some(_)))
        {
            // Only the still-unreaped process launched by this command receives TERM.
            let _ = nix::sys::signal::kill(nix::unistd::Pid::from_raw(worker.id() as i32), nix::sys::signal::Signal::SIGTERM);
            reap(worker);
        }
    }
}

pub(super) fn supervise() -> Result<()> {
    let mut started = false;
    let result = (|| {
        let spec: LaunchSpec = serde_json::from_reader(stdin().lock()).context("read local cloud launch configuration")?;
        run(spec, &mut started)
    })();
    if let Err(error) = &result
        && !started
    {
        let _ = print_json(&Startup::Failed { message: format!("{error:#}") });
    }
    result
}

fn run(mut spec: LaunchSpec, started: &mut bool) -> Result<()> {
    let stop = Arc::new(AtomicBool::new(false));
    for signal in [signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT] {
        signal_hook::flag::register(signal, stop.clone())?;
    }
    let (scope, origin) = match &spec.source {
        Source::Direct(credentials) => {
            let root = credentials.root.parent().context("invalid local cloud client state directory")?.to_owned();
            (credentials.api()?, LaunchOrigin { root, host: credentials.host.clone(), incarnation: None, daemon_instance: None })
        }
        Source::Managed { root, incarnation, expected } => {
            let scope = Scope::managed(root, false)?;
            let Port::Managed(client) = &scope.port else { unreachable!("managed scope has an IPC port") };
            ensure!(
                client.info.daemon_instance == expected.daemon_instance
                    && client.info.binding.local_generation == expected.binding.local_generation
                    && client.info.binding.cloud_generation == expected.binding.cloud_generation
                    && client.info.binding.installation_id == expected.binding.installation_id
                    && client.info.host == expected.host,
                "scope or daemon instance changed during cloud launch"
            );
            let origin = LaunchOrigin { root: client.root().to_owned(), host: expected.host.clone(), incarnation: Some(incarnation.clone()), daemon_instance: Some(expected.daemon_instance.clone()) };
            (scope, origin)
        }
    };
    let mut registration = match &spec.name {
        Some(name) => scope.register_local(&origin, name, spec.recover)?,
        None => scope.allocate_local(&origin)?,
    };
    let agent = &registration.run.actor.agent;
    spec.instructions = bootstrap(&agent.host, &agent.name, &spec.instructions)?;
    let title = spec.title.get_or_insert_with(|| format!("@{}", super::qualified(agent)));
    ensure!(!title.is_empty(), "missing Codex thread title");
    if let Some(previous) = registration.run.private_dir.take() {
        clean_private_dir(&previous);
    }
    if let Some(session) = &registration.run.session {
        ensure!(spec.cwd == session.cwd, "recovery must use the original Codex working directory {}", session.cwd.display());
    }
    let local = tempfile::Builder::new().prefix("ax-cloud-").permissions(std::fs::Permissions::from_mode(0o700)).tempdir()?;
    registration.run.private_dir = Some(local.path().to_owned());
    registration.persist()?;
    let socket = local.path().join("codex.sock");
    let context = local.path().join("agent.json");
    let record = crate::sys::fs::create_private_new(&context)?;
    let context_key = match &mut spec.source {
        Source::Direct(credentials) => {
            credentials.actor = Some(registration.run.actor.clone());
            serde_json::to_writer(record, credentials)?;
            CONTEXT_FILE
        }
        Source::Managed { root, .. } => {
            serde_json::to_writer(record, &ManagedContext { root: root.clone(), incarnation: registration.run.actor.agent.incarnation.clone() })?;
            IPC_CONTEXT
        }
    };
    let thread = registration.run.session.as_ref().map(|session| session.thread.as_str());
    let (mut native, mut provider) = Codex::start(&spec, &socket, context_key, &context, &stop, thread)?;
    registration.run.session = Some(LocalSession { thread: native.thread.clone(), cwd: spec.cwd.clone() });
    registration.persist()?;
    registration.publish_session(&native.thread, false)?;
    let mut next_heartbeat = Instant::now() + HEARTBEAT;
    if let Some(prompt) = &spec.initial {
        native.start_turn(prompt, None).context("start initial Codex turn")?;
    }
    registration.publish_session(&native.thread, true)?;
    print_json(&Startup::Ready(Ready { agent: registration.run.actor.agent.clone(), socket, session_id: native.thread.clone() }))?;
    *started = true;
    registration.retire_on_drop = false;
    let mut next_mailbox = Instant::now();
    let mut mailbox_delay = super::POLL;
    let mut last_error = None;
    while !stop.load(Ordering::Relaxed) {
        if Instant::now() >= next_heartbeat {
            if let Err(error) = scope.heartbeat(&registration.run.actor) {
                let permanent = error.downcast_ref::<super::HttpError>().is_some_and(|error| matches!(error.status, 401 | 403 | 404 | 409));
                if permanent {
                    return Err(error).context("cloud host lease rejected");
                }
                eprintln!("Cloud host heartbeat: {error:#}");
                next_heartbeat = Instant::now() + Duration::from_secs(2);
            } else {
                next_heartbeat = Instant::now() + HEARTBEAT;
            }
        }
        if Instant::now() >= next_mailbox {
            let observation = match pump(&scope, &registration.run.actor, &mut native) {
                Ok(observation) => {
                    last_error = None;
                    observation
                }
                Err(error) => {
                    let message = format!("{error:#}");
                    if last_error.as_ref() != Some(&message) {
                        eprintln!("Cloud mailbox: {message}");
                    }
                    last_error = Some(message);
                    MailboxPoll::Pending
                }
            };
            mailbox_delay = observation.next_delay(mailbox_delay);
            next_mailbox = Instant::now() + mailbox_delay;
        }
        if provider.0.try_wait()?.is_some() {
            bail!("private Codex app-server exited");
        }
        native.step(next_mailbox.saturating_duration_since(Instant::now()).min(super::POLL))?;
    }
    registration.retire_on_drop = true;
    Ok(())
}

fn clean_private_dir(dir: &Path) {
    // Delete only known files of a previously recorded private runtime, under
    // the current installation's system temp directory. Never recurse.
    if dir.parent() == Some(std::env::temp_dir().as_path()) && dir.file_name().is_some_and(|name| name.as_encoded_bytes().starts_with(b"ax-cloud-")) && dir.symlink_metadata().is_ok_and(|metadata| metadata.is_dir()) {
        let _ = std::fs::remove_file(dir.join("agent.json"));
        let _ = std::fs::remove_file(dir.join("codex.sock"));
        let _ = std::fs::remove_dir(dir);
    }
}

fn pump(scope: &Scope, actor: &Actor, native: &mut Codex) -> Result<MailboxPoll> {
    let Some(mailbox) = Mailbox::try_open(scope, actor)? else { return Ok(MailboxPoll::Pending) };
    let previous = mailbox.read()?;
    if let Some(claim) = &previous
        && mailbox.complete(scope, actor, claim)?
    {
        return Ok(MailboxPoll::Pending);
    }
    let idle = native.idle;
    let mut claim = match previous {
        Some(claim) => claim,
        None if idle => mailbox.begin()?,
        None => return Ok(MailboxPoll::Pending),
    };
    match &claim.phase {
        Phase::ListenIssuing(_) => bail!("explicit listener output is unresolved for claim {}; it will not be replayed", claim.id),
        Phase::ProviderIssuing(id) => {
            let batch = scope.recover(actor, &claim.id)?;
            let message = batch.items.iter().find(|message| &message.id == id).context("provider claim did not contain its outstanding message")?;
            settle_provider(scope, actor, native, &mailbox, &mut claim, message)?;
        }
        Phase::Reserved if idle => {
            let batch = scope.claim(actor, &claim.id, CLAIM_WAIT_MS)?;
            let Some(message) = batch.items.first() else { return Ok(MailboxPoll::Empty) };
            let text = delivery_text(message);
            mailbox.transition(&mut claim, Phase::ProviderIssuing(message.id.clone()))?;
            match native.start_turn(&text, Some(&correlation(&claim.id))) {
                Ok(()) => settle_provider(scope, actor, native, &mailbox, &mut claim, message)?,
                Err(error) => return Err(error).with_context(|| format!("Codex acceptance of claim {} is unknown", claim.id)),
            }
        }
        _ => return Ok(MailboxPoll::Pending),
    }
    Ok(MailboxPoll::Delivered)
}

fn settle_provider(scope: &Scope, actor: &Actor, native: &mut Codex, mailbox: &Mailbox, claim: &mut super::mailbox::Claim, message: &super::MessageView) -> Result<()> {
    // A turn ID only acknowledges the request. Exact native user-input evidence
    // is required before persisting permission to confirm the cloud delivery.
    ensure!(native.find_input(&correlation(&claim.id), &delivery_text(message))?, "waiting for exact Codex user input for cloud claim {}", claim.id);
    mailbox.transition(claim, Phase::Confirming(message.id.clone()))?;
    mailbox.complete(scope, actor, claim)?;
    Ok(())
}

fn correlation(claim: &str) -> String {
    format!("agentx-cloud:{claim}")
}
fn delivery_text(message: &super::MessageView) -> String {
    format!("<agentx-cloud>\n{}\n</agentx-cloud>", message_text(message))
}

struct Provider(Child);
impl Drop for Provider {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let group = nix::unistd::Pid::from_raw(self.0.id() as i32);
            let _ = nix::sys::signal::killpg(group, nix::sys::signal::Signal::SIGTERM);
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                if self.0.try_wait().ok().flatten().is_some() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            let _ = nix::sys::signal::killpg(group, nix::sys::signal::Signal::SIGKILL);
        }
        let _ = self.0.wait();
    }
}

fn scrub_agentx(command: &mut Command) {
    for key in std::env::vars_os().map(|(key, _)| key) {
        if key.to_string_lossy().starts_with("AGENTX_") || matches!(key.to_str(), Some("CODEX_SESSION_ID" | "CODEX_THREAD_ID")) {
            command.env_remove(key);
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Codex {method} rejected: {message}")]
struct NativeRejection {
    method: String,
    message: String,
}

struct Codex {
    conn: Box<dyn NativeConnection>,
    thread: String,
    idle: bool,
}

impl Codex {
    fn start(spec: &LaunchSpec, socket: &Path, context_key: &str, context: &Path, stop: &AtomicBool, prior_thread: Option<&str>) -> Result<(Self, Provider)> {
        let mut command = Command::new(&spec.tool);
        command.current_dir(&spec.cwd).stdin(Stdio::null()).stdout(Stdio::null()).process_group(0).arg("app-server").arg("--listen").arg(format!("unix://{}", socket.display()));
        scrub_agentx(&mut command);
        command.env(context_key, context);
        let supervisor = std::process::id() as libc::pid_t;
        // SAFETY: only async-signal-safe Linux syscalls run between fork and exec.
        // If the host itself dies, the private app-server must not outlive it.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 || libc::getppid() != supervisor {
                    return Err(std::io::Error::other("could not bind private Codex to its supervisor"));
                }
                Ok(())
            });
        }
        let mut child = Provider(command.spawn().context("start private Codex app-server")?);
        let began = Instant::now();
        let conn = loop {
            ensure!(!stop.load(Ordering::Relaxed), "cloud host stopped during startup");
            if let Some(status) = child.0.try_wait()? {
                bail!("private Codex app-server exited during startup: {status}");
            }
            if socket.exists() {
                match connect_private_socket(socket) {
                    Ok(conn) => break conn,
                    Err(error) if began.elapsed() > RPC_TIMEOUT => return Err(error.into()),
                    Err(_) => {}
                }
            }
            ensure!(began.elapsed() < RPC_TIMEOUT, "private Codex app-server socket did not become ready");
            std::thread::sleep(Duration::from_millis(50));
        };
        let mut client = Self { conn, thread: String::new(), idle: false };
        client.call("initialize", json!({"clientInfo":{"name":"agentx-cloud-host","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}}))?;
        client.send(&json!({"method":"initialized"}))?;
        let mut config = spec.config.clone();
        if !config.get("features").is_some_and(Value::is_object) {
            config["features"] = json!({});
        }
        config["features"]["hooks"] = json!(false);
        if !config.get("shell_environment_policy").is_some_and(Value::is_object) {
            config["shell_environment_policy"] = json!({});
        }
        let set = &mut config["shell_environment_policy"]["set"];
        if !set.is_object() {
            *set = json!({});
        }
        set.as_object_mut().expect("object just established").retain(|key, _| !key.starts_with("AGENTX_"));
        set[context_key] = json!(context);
        let own_bin = std::env::current_exe()?;
        let parent = own_bin.parent().context("AgentX executable has no directory")?;
        let path = std::env::var_os("PATH").unwrap_or_default();
        let path = std::env::join_paths(std::iter::once(parent.to_owned()).chain(std::env::split_paths(&path)))?;
        set["PATH"] = json!(path.to_string_lossy());
        let mut params = json!({"cwd":spec.cwd,"developerInstructions":spec.instructions,"config":config,"historyMode":"paginated"});
        let method = match prior_thread {
            Some(thread) => {
                params["threadId"] = json!(thread);
                "thread/resume"
            }
            None => {
                params["threadSource"] = json!("user");
                "thread/start"
            }
        };
        let result = client.call(method, params)?;
        client.thread = result["thread"]["id"].as_str().filter(|id| !id.is_empty()).context("Codex returned no thread ID")?.to_owned();
        ensure!(prior_thread.is_none_or(|expected| expected == client.thread), "Codex resumed a different thread");
        client.idle = result["thread"]["status"]["type"] == "idle";
        client.call("thread/name/set", json!({"threadId":client.thread,"name":spec.title.as_deref().context("missing Codex thread title")?}))?;
        Ok((client, child))
    }

    fn send(&mut self, message: &Value) -> Result<()> {
        self.conn.send(&message.to_string()).map_err(Into::into)
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        self.call_until(method, params, Instant::now() + RPC_TIMEOUT)
    }

    fn call_until(&mut self, method: &str, params: Value, deadline: Instant) -> Result<Value> {
        let id = Uuid::new_v4().to_string();
        self.send(&json!({"id":id,"method":method,"params":params}))?;
        while let Some(frame) = self.frame(deadline.saturating_duration_since(Instant::now()))? {
            if frame.get("method").is_none() && frame["id"] == id {
                if let Some(result) = frame.get("result") {
                    return Ok(result.clone());
                }
                if let Some(error) = frame.get("error") {
                    return Err(NativeRejection { method: method.into(), message: error["message"].as_str().unwrap_or("request rejected").into() }.into());
                }
                bail!("invalid Codex {method} reply");
            }
            self.observe(&frame)?;
            if Instant::now() >= deadline {
                break;
            }
        }
        bail!("Codex {method} timed out; outcome may be unknown")
    }

    fn start_turn(&mut self, text: &str, client_id: Option<&str>) -> Result<()> {
        self.idle = false;
        let mut params = json!({"threadId":self.thread,"input":[{"type":"text","text":text}]});
        if let Some(id) = client_id {
            params["clientUserMessageId"] = json!(id);
        }
        let outcome = self.call("turn/start", params);
        if outcome.as_ref().is_err_and(|error| error.downcast_ref::<NativeRejection>().is_some())
            && let Ok(result) = self.call("thread/read", json!({"threadId":self.thread}))
        {
            self.idle = result["thread"]["status"]["type"] == "idle";
        }
        let result = outcome?;
        ensure!(result["turn"]["id"].as_str().is_some(), "invalid Codex turn receipt");
        Ok(())
    }

    fn find_input(&mut self, client_id: &str, text: &str) -> Result<bool> {
        let deadline = Instant::now() + RPC_TIMEOUT;
        let mut cursor = None::<String>;
        for _ in 0..HISTORY_PAGES {
            if Instant::now() >= deadline {
                break;
            }
            let result = self.call_until("thread/items/list", json!({"threadId":self.thread,"limit":100,"sortDirection":"desc","cursor":cursor}), deadline)?;
            for row in result["data"].as_array().context("invalid Codex history response")? {
                if row["item"]["type"] == "userMessage" && mailbox_item_matches(&RawObject::parse(&row["item"].to_string())?, client_id, text) {
                    return Ok(true);
                }
            }
            let Some(next) = result["nextCursor"].as_str() else { break };
            ensure!(cursor.as_deref() != Some(next), "Codex history cursor did not advance");
            cursor = Some(next.to_owned());
        }
        Ok(false)
    }

    fn frame(&mut self, timeout: Duration) -> Result<Option<Value>> {
        match self.conn.receive(timeout)? {
            Receive::Frame(frame) => Ok(Some(serde_json::from_str(&frame)?)),
            Receive::TimedOut => Ok(None),
            Receive::Closed => bail!("private Codex connection closed"),
        }
    }

    fn step(&mut self, timeout: Duration) -> Result<()> {
        if let Some(frame) = self.frame(timeout)? {
            self.observe(&frame)?;
        }
        Ok(())
    }

    fn observe(&mut self, frame: &Value) -> Result<()> {
        let params = &frame["params"];
        if params["threadId"] == self.thread {
            match frame["method"].as_str() {
                Some("thread/status/changed") => self.idle = params["status"]["type"] == "idle",
                Some("turn/started") => self.idle = false,
                Some("turn/completed") => self.idle = true,
                _ => {}
            }
        }
        // A failure reply settles provider requests that this background client cannot
        // fulfill (for example, an interactive approval when no UI owns the turn).
        if let (Some(method), Some(id)) = (frame["method"].as_str(), frame.get("id")) {
            eprintln!("Codex requested local interaction ({method}) from the cloud host");
            self.send(&json!({"id":id,"error":{"code":-32601,"message":"This connection does not handle interactive requests"}}))?;
        }
        Ok(())
    }
}
