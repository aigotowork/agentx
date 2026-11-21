//! Bounded outer I/O queues and connection-only stdio/signal cleanup.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::{self, TryRecvError, TrySendError};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

use crate::agents::service::RemoteAttachments;
use crate::messaging::activation::DriverEvent;
use crate::messaging::service::RemoteInbox;
use crate::runtime::context::AgentxContext;
use crate::runtime::notify::NotifyServer;
use crate::service::client::Client;

use super::super::configuration::{SessionConfiguration, thread_overrides};
use super::super::service::{CodexService, ManagedCodexService, Receive, ServiceRequest};
use super::{BridgePorts, NativeBridge, RootLaunch};

enum OuterInput {
    Frame(String),
    Closed,
    Failed(String),
}

/// The driver owns policy effects; this drain never reapplies events.
pub(super) fn log_events(bridge: &mut NativeBridge<'_>) {
    for event in bridge.take_events() {
        let level = if matches!(&event, DriverEvent::OutcomeUnknown { .. }) { "WARN" } else { "INFO" };
        if let Ok(detail) = serde_json::to_string(&event) {
            crate::log::log(level, "codex", "native.driver", &detail);
        }
    }
}

pub(crate) fn run_with_client(args: &[String], env: &HashMap<String, String>, cwd: &Path, launch: RootLaunch, auto_approve: bool, client: &Client) -> Result<i32> {
    thread_overrides(args)?;
    let mut env = env.clone();
    // Only the originating bridge may listen. Never export this socket choice
    // to a native thread's tools, or a delegated child could try to bind it.
    let terminal_socket = env.remove("AGENTX_CODEX_TUI_SOCKET").filter(|path| !path.is_empty());
    #[cfg(not(target_os = "linux"))]
    if terminal_socket.is_some() {
        bail!("Shared Codex TUI transport is supported on Linux only");
    }
    crate::runtime::codex_stdio::pin()?;
    let command = crate::runtime::codex_stdio::pinned_command().context("Native Codex was not pinned")?;
    let canonical_home = PathBuf::from(env.get("CODEX_HOME").context("Shared native launch requires resolved CODEX_HOME")?).canonicalize()?;
    let request = ServiceRequest {
        program: PathBuf::from(command.get_program()),
        prefix_args: command.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect(),
        args: vec!["app-server".to_owned()],
        env: env.clone(),
        canonical_home,
        agentx_dir: crate::paths::agentx_dir().canonicalize()?,
        required_version: None,
    };
    let context = AgentxContext::from_env(&env, cwd.to_path_buf()).with_tty(false, false);
    if client.scope_root() != context.agentx_dir.canonicalize()?.as_path() {
        bail!("bridge client scope differs from context");
    }
    let client = client.with_invocation(crate::runtime::host_client::invocation(&context));
    let mut service = ManagedCodexService;
    let mut connection = service.connect(&request)?;
    let configuration = SessionConfiguration::new(args, env, cwd, &connection.identity, auto_approve)?;
    let notify = NotifyServer::new()?;
    let mut attachments = RemoteAttachments::new(client.clone());
    let mut inbox = RemoteInbox::new(client.clone());
    let ports = BridgePorts { attachments: &mut attachments, inbox: &mut inbox, transport: connection.transport.as_mut(), hosts: &client };
    let mut bridge = NativeBridge::new(configuration, launch, connection.identity.clone(), ports, notify.port());

    #[cfg(target_os = "linux")]
    if let Some(path) = terminal_socket {
        return super::terminal::serve(Path::new(&path), &mut bridge, &notify);
    }

    let stopped = Arc::new(AtomicI32::new(0));
    #[cfg(unix)]
    let signal_handle = {
        use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
        let mut signals = signal_hook::iterator::Signals::new([SIGINT, SIGTERM, SIGHUP, SIGQUIT])?;
        let handle = signals.handle();
        let stopped = stopped.clone();
        std::thread::spawn(move || {
            for signal in signals.forever() {
                stopped.store(128 + signal, Ordering::SeqCst);
            }
        });
        handle
    };

    let (input_tx, input_rx) = mpsc::sync_channel(32);
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        loop {
            let mut line = String::new();
            let input = match stdin.read_line(&mut line) {
                Ok(0) => OuterInput::Closed,
                Ok(_) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    OuterInput::Frame(line.trim_end_matches(['\r', '\n']).to_owned())
                }
                Err(error) => OuterInput::Failed(error.to_string()),
            };
            let finished = !matches!(&input, OuterInput::Frame(_));
            if input_tx.send(input).is_err() || finished {
                break;
            }
        }
    });
    let (output_tx, output_rx) = mpsc::sync_channel::<String>(32);
    let (output_done_tx, output_done_rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = (|| -> std::io::Result<()> {
            let mut stdout = std::io::stdout().lock();
            for frame in output_rx {
                stdout.write_all(frame.as_bytes())?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
            }
            Ok(())
        })();
        let _ = output_done_tx.send(result);
    });

    let mut pending_output = None;
    let mut next_scan = Instant::now();
    let mut next_heartbeat = Instant::now();
    let result = (|| -> Result<i32> {
        loop {
            log_events(&mut bridge);
            let signal = stopped.load(Ordering::SeqCst);
            if signal != 0 {
                return Ok(signal);
            }
            if let Ok(result) = output_done_rx.try_recv() {
                result.context("Native client stdout failed")?;
                return Ok(0);
            }
            let mut user_frames = false;
            for _ in 0..64 {
                match input_rx.try_recv() {
                    Ok(OuterInput::Frame(frame)) => {
                        bridge.handle_outer(&frame)?;
                        user_frames = true;
                    }
                    Ok(OuterInput::Closed) | Err(TryRecvError::Disconnected) => return Ok(0),
                    Ok(OuterInput::Failed(error)) => bail!("Native client stdin failed: {error}"),
                    Err(TryRecvError::Empty) => break,
                }
            }
            if Instant::now() >= next_heartbeat {
                bridge.heartbeat()?;
                next_heartbeat = Instant::now() + Duration::from_secs(5);
            }
            if let Some(frame) = pending_output.take() {
                match output_tx.try_send(frame) {
                    Ok(()) => {}
                    Err(TrySendError::Full(frame)) => pending_output = Some(frame),
                    Err(TrySendError::Disconnected(_)) => bail!("Native client stdout closed"),
                }
            }
            if pending_output.is_some() {
                // Keep outer controls responsive, but stop consuming native
                // frames or admitting new mail while output is backpressured.
                match input_rx.recv_timeout(Duration::from_millis(25)) {
                    Ok(OuterInput::Frame(frame)) => bridge.handle_outer(&frame)?,
                    Ok(OuterInput::Closed) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(0),
                    Ok(OuterInput::Failed(error)) => bail!("Native client stdin failed: {error}"),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                continue;
            }
            let native_quiet = match bridge.ports.transport.receive(if user_frames { Duration::ZERO } else { Duration::from_millis(25) })? {
                Receive::Frame(frame) => {
                    if let Some(frame) = bridge.handle_native(&frame)? {
                        match output_tx.try_send(frame) {
                            Ok(()) => {}
                            Err(TrySendError::Full(frame)) => pending_output = Some(frame),
                            Err(TrySendError::Disconnected(_)) => bail!("Native client stdout closed"),
                        }
                    }
                    false
                }
                Receive::TimedOut => true,
                Receive::Closed => return Ok(0),
            };
            let notified = notify.wait(Duration::ZERO);
            if native_quiet && pending_output.is_none() && (notified || Instant::now() >= next_scan) {
                // A frame already queued by the user always wins over a new
                // internal offer. Preserve the original native method/body.
                match input_rx.try_recv() {
                    Ok(OuterInput::Frame(frame)) => {
                        bridge.handle_outer(&frame)?;
                        continue;
                    }
                    Ok(OuterInput::Closed) | Err(TryRecvError::Disconnected) => return Ok(0),
                    Ok(OuterInput::Failed(error)) => bail!("Native client stdin failed: {error}"),
                    Err(TryRecvError::Empty) => {}
                }
                bridge.drive_pending()?;
                next_scan = Instant::now() + Duration::from_secs(1);
            }
            if let Some(error) = bridge.fatal.take() {
                bail!("Native input write outcome is unknown: {error}");
            }
        }
    })();

    let cleanup = bridge.close();
    log_events(&mut bridge);
    if let Err(error) = cleanup {
        eprintln!("[agentx] native connection cleanup failed: {error}");
    }
    if let Some(frame) = pending_output {
        let mut pending = frame;
        while stopped.load(Ordering::SeqCst) == 0 {
            match output_tx.try_send(pending) {
                Ok(()) => break,
                Err(TrySendError::Full(frame)) => {
                    pending = frame;
                    if output_done_rx.recv_timeout(Duration::from_millis(25)).is_ok() {
                        break;
                    }
                }
                Err(TrySendError::Disconnected(_)) => break,
            }
        }
    }
    drop(output_tx);
    while stopped.load(Ordering::SeqCst) == 0 {
        match output_done_rx.recv_timeout(Duration::from_millis(25)) {
            Ok(Ok(())) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Ok(Err(error)) => {
                eprintln!("[agentx] native stdout flush failed: {error}");
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    #[cfg(unix)]
    signal_handle.close();
    result
}
