//! One private Unix WebSocket frontend for the existing native bridge actor.

use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};

use crate::runtime::notify::NotifyServer;

use super::super::service::{NativeConnection, Receive, UnixConnection};
use super::{NativeBridge, stdio::log_events};

struct Frontend {
    listener: UnixListener,
    path: PathBuf,
    file: (u64, u64),
}

impl Frontend {
    fn bind(path: &Path) -> Result<Self> {
        let parent = path.parent().filter(|_| path.is_absolute()).context("Codex TUI socket requires an absolute path")?;
        let parent = parent.canonicalize().context("Could not resolve the Codex TUI socket directory")?;
        let info = parent.metadata()?;
        // SAFETY: geteuid takes no arguments and does not access Rust memory.
        if !info.is_dir() || info.uid() != unsafe { libc::geteuid() } || info.mode() & 0o077 != 0 {
            bail!("Codex TUI socket directory must be owned by this user and private (0700)");
        }
        let path = parent.join(path.file_name().context("Missing Codex TUI socket filename")?);
        let listener = UnixListener::bind(&path).with_context(|| format!("Could not bind Codex TUI socket {}", path.display()))?;
        let info = path.symlink_metadata()?;
        let frontend = Self { listener, path, file: (info.dev(), info.ino()) };
        frontend.listener.set_nonblocking(true)?;
        Ok(frontend)
    }

    fn accept(&self, stop: &AtomicBool) -> Result<Option<UnixConnection>> {
        let deadline = Instant::now() + Duration::from_secs(45);
        while !stop.load(Ordering::Relaxed) {
            match self.listener.accept() {
                Ok((stream, _)) => return Ok(Some(UnixConnection::accept(stream)?)),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => bail!("Timed out waiting for the Codex TUI connection"),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(None)
    }
}

impl Drop for Frontend {
    fn drop(&mut self) {
        if let Ok(info) = self.path.symlink_metadata()
            && info.file_type().is_socket()
            && (info.dev(), info.ino()) == self.file
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(super) fn serve(path: &Path, bridge: &mut NativeBridge<'_>, notify: &NotifyServer) -> Result<i32> {
    let stop = Arc::new(AtomicBool::new(false));
    let signals = [SIGHUP, SIGINT, SIGTERM].into_iter().map(|signal| signal_hook::flag::register(signal, stop.clone())).collect::<io::Result<Vec<_>>>()?;
    let result = (|| -> Result<i32> {
        let frontend = Frontend::bind(path)?;
        if let Some(mut outer) = frontend.accept(&stop)? {
            pump(&mut outer, bridge, notify, &stop)?;
        }
        Ok(0)
    })();
    let cleanup = bridge.close();
    log_events(bridge);
    for signal in signals {
        signal_hook::low_level::unregister(signal);
    }
    result.and(cleanup.map(|()| 0))
}

fn pump(outer: &mut dyn NativeConnection, bridge: &mut NativeBridge<'_>, notify: &NotifyServer, stop: &AtomicBool) -> Result<()> {
    let mut next_scan = Instant::now();
    let mut next_heartbeat = next_scan;
    while !stop.load(Ordering::Relaxed) {
        log_events(bridge);
        let mut user_frames = false;
        for _ in 0..64 {
            match outer.receive(Duration::ZERO)? {
                Receive::Frame(frame) => {
                    bridge.handle_outer(&frame)?;
                    user_frames = true;
                }
                Receive::Closed => return Ok(()),
                Receive::TimedOut => break,
            }
        }
        if Instant::now() >= next_heartbeat {
            bridge.heartbeat()?;
            next_heartbeat = Instant::now() + Duration::from_secs(5);
        }
        match bridge.ports.transport.receive(if user_frames { Duration::ZERO } else { Duration::from_millis(25) })? {
            Receive::Frame(frame) => {
                if let Some(frame) = bridge.handle_native(&frame)? {
                    outer.send(&frame)?;
                }
            }
            Receive::Closed => return Ok(()),
            Receive::TimedOut if !user_frames && (notify.wait(Duration::ZERO) || Instant::now() >= next_scan) => match outer.receive(Duration::ZERO)? {
                Receive::Frame(frame) => bridge.handle_outer(&frame)?,
                Receive::Closed => return Ok(()),
                Receive::TimedOut => {
                    bridge.drive_pending()?;
                    next_scan = Instant::now() + Duration::from_secs(1);
                }
            },
            Receive::TimedOut => {}
        }
        if let Some(error) = bridge.fatal.take() {
            bail!("Native input write outcome is unknown: {error}");
        }
    }
    Ok(())
}
