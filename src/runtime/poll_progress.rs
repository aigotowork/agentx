//! Recent authenticated PTY reads, shared by the daemon's reader and writer.
//! The durable process registration remains the authority for every lookup.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
pub(crate) struct PollProgress(Arc<Mutex<HashMap<String, (Instant, i64)>>>);

const MAX_HOSTS: usize = 4096;
const RETAIN: Duration = Duration::from_secs(300);

impl PollProgress {
    pub(crate) fn record(&self, registration_token: String) {
        let Ok(mut entries) = self.0.lock() else { return };
        if !entries.contains_key(&registration_token) && entries.len() >= MAX_HOSTS {
            entries.retain(|_, (at, _)| at.elapsed() < RETAIN);
        }
        if entries.len() < MAX_HOSTS || entries.contains_key(&registration_token) {
            entries.insert(registration_token, (Instant::now(), crate::shared::time::now_epoch_i64()));
        }
    }

    pub(crate) fn last(&self, registration_token: &str) -> Option<i64> {
        self.0.lock().ok()?.get(registration_token).filter(|(at, _)| at.elapsed() < RETAIN).map(|(_, stamp)| *stamp)
    }
}
