//! The native listener owns the lifecycle of the scope's outbound connection.
//! Network work and local Provider startup never run on a native dispatch lane.

#[cfg(target_os = "linux")]
mod capabilities;
#[cfg(target_os = "linux")]
mod journal;
#[cfg(target_os = "linux")]
mod worker;
#[cfg(target_os = "linux")]
mod workspaces;

use std::sync::{Arc, atomic::AtomicBool, mpsc};
use std::thread::{Scope, ScopedJoinHandle};

use crate::daemon::ScopeLease;

use super::connection::{Config, publish_status};

pub(crate) fn spawn<'scope, 'env>(threads: &'scope Scope<'scope, 'env>, lease: &'scope ScopeLease, stop: Arc<AtomicBool>, listening: mpsc::Receiver<()>) -> Option<ScopedJoinHandle<'scope, ()>> {
    let root = lease.scope().root();
    let config = match Config::load(root) {
        Ok(Some(config)) => config,
        Ok(None) => return None,
        Err(_) => {
            let _ = publish_status(root, lease.instance(), "unavailable", Some("configuration_invalid"));
            return None;
        }
    };

    #[cfg(target_os = "linux")]
    {
        match std::thread::Builder::new().name("agentx-cloud-uplink".into()).spawn_scoped(threads, move || {
            if listening.recv().is_ok() && !stop.load(std::sync::atomic::Ordering::Acquire) {
                worker::run(lease, &stop, &config);
            }
        }) {
            Ok(thread) => Some(thread),
            Err(_) => {
                let _ = publish_status(root, lease.instance(), "unavailable", Some("worker_start_failed"));
                None
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (threads, stop, listening, config);
        let _ = publish_status(root, lease.instance(), "unavailable", Some("platform_unsupported"));
        None
    }
}
