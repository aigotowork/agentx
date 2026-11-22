//! Bounded, lossy refresh hints for ordinary Events observers. SQLite remains
//! the authority. The worker never owns storage or acknowledges delivery.

use std::collections::VecDeque;
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::daemon::DatabaseGeneration;

const CAPACITY: usize = 256;
const TTL: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Admission {
    Queued,
    Coalesced,
    Dropped,
}

pub(crate) struct ObserverHints {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct Pending {
    stopping: bool,
    jobs: VecDeque<Hint>,
}

struct Shared {
    revision: Arc<AtomicU64>,
    generation: DatabaseGeneration,
    pending: Mutex<Pending>,
    ready: Condvar,
}

struct Hint {
    registration: uuid::Uuid,
    generation: String,
    revision: u64,
    pid: u32,
    incarnation: String,
    port: u16,
    queued: Instant,
}

impl ObserverHints {
    pub(crate) fn start(generation: DatabaseGeneration, revision: Arc<AtomicU64>) -> std::io::Result<Self> {
        Self::with_sender(
            generation,
            revision,
            |_| {},
            |port| {
                let target = SocketAddr::from(([127, 0, 0, 1], port));
                let _ = TcpStream::connect_timeout(&target, Duration::from_millis(super::wake::WAKE_FANOUT_MS));
            },
        )
    }

    pub(crate) fn with_sender(generation: DatabaseGeneration, revision: Arc<AtomicU64>, before: impl Fn(u16) + Send + 'static, send: impl Fn(u16) + Send + 'static) -> std::io::Result<Self> {
        let shared = Arc::new(Shared { revision, generation, pending: Mutex::new(Pending::default()), ready: Condvar::new() });
        let worker = {
            let shared = Arc::clone(&shared);
            std::thread::Builder::new().name("events-observer-hints".into()).spawn(move || {
                while let Some(hint) = shared.take() {
                    before(hint.port);
                    if !shared.valid(&hint) || crate::sys::process::identity(hint.pid).as_deref() != Some(hint.incarnation.as_str()) || !shared.valid(&hint) {
                        continue;
                    }
                    send(hint.port);
                }
            })?
        };
        Ok(Self { shared, worker: Some(worker) })
    }

    pub(crate) fn offer(&self, registration: uuid::Uuid, generation: &str, pid: u32, incarnation: &str, port: u16) -> Admission {
        let Ok(mut pending) = self.shared.pending.try_lock() else { return Admission::Dropped };
        if pending.stopping {
            return Admission::Dropped;
        }
        let revision = self.shared.revision.load(Ordering::Acquire);
        pending.jobs.retain(|hint| hint.revision == revision);
        let hint = Hint { registration, generation: generation.into(), revision, pid, incarnation: incarnation.into(), port, queued: Instant::now() };
        if let Some(existing) = pending.jobs.iter_mut().find(|other| other.registration == registration) {
            *existing = hint;
            return Admission::Coalesced;
        }
        if pending.jobs.len() == CAPACITY {
            return Admission::Dropped;
        }
        pending.jobs.push_back(hint);
        drop(pending);
        self.shared.ready.notify_one();
        Admission::Queued
    }
}

impl Shared {
    fn take(&self) -> Option<Hint> {
        let pending = self.pending.lock().ok()?;
        let mut pending = self.ready.wait_while(pending, |pending| !pending.stopping && pending.jobs.is_empty()).ok()?;
        if pending.stopping { None } else { pending.jobs.pop_front() }
    }

    fn valid(&self, hint: &Hint) -> bool {
        hint.queued.elapsed() < TTL && self.generation.current() == hint.generation && self.revision.load(Ordering::Acquire) == hint.revision
    }
}

impl Drop for ObserverHints {
    fn drop(&mut self) {
        self.shared.revision.fetch_add(1, Ordering::AcqRel);
        {
            let mut pending = self.shared.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            pending.stopping = true;
            pending.jobs.clear();
        }
        self.shared.ready.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcommit_hints_coalesce_and_reject_capacity_without_waiting_for_sender() {
        let generation = DatabaseGeneration::default();
        let key = uuid::Uuid::new_v4().to_string();
        generation.publish(key.clone()).unwrap();
        let (entered, waiting) = std::sync::mpsc::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let hints = ObserverHints::with_sender(
            generation,
            Arc::new(AtomicU64::new(0)),
            move |port| {
                if port == 1 {
                    entered.send(()).unwrap();
                    let _ = gate.recv_timeout(Duration::from_secs(5));
                }
            },
            |_| {},
        )
        .unwrap();
        let offer = |id, port| hints.offer(id, &key, std::process::id(), "not-an-authoritative-incarnation", port);
        assert_eq!(offer(uuid::Uuid::new_v4(), 1), Admission::Queued);
        waiting.recv_timeout(Duration::from_secs(2)).unwrap();
        let id = uuid::Uuid::new_v4();
        assert_eq!(offer(id, 2), Admission::Queued);
        assert_eq!(offer(id, 2), Admission::Coalesced);
        for _ in 1..CAPACITY {
            assert_eq!(offer(uuid::Uuid::new_v4(), 2), Admission::Queued);
        }
        assert_eq!(offer(uuid::Uuid::new_v4(), 2), Admission::Dropped);
        release.send(()).unwrap();
    }
}
