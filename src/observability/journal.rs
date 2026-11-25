//! Bounded in-memory append frontier; never a domain audit log or a DB adapter.
//! Producer handles retain only a fence/counters and a weak journal reference,
//! so retired handles cannot pin old record buffers or redirect into a new era.

use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{
    Arc, Mutex, MutexGuard, TryLockError, Weak,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use serde::Serialize;

use super::context::{DaemonRef, ObservationClock, ObservationEpoch, ObservationTime};
use super::events::*;

#[derive(Clone)]
pub(crate) struct ObservationJournal {
    inner: Arc<Journal>,
}

struct Journal {
    daemon: DaemonRef,
    clock: ObservationClock,
    closed: AtomicBool,
    published: AtomicBool,
    state: Mutex<Option<Stream>>,
}

struct Stream {
    id: ObservationStreamId,
    newest: u64,
    bytes: usize,
    records: VecDeque<StoredRecord>,
    counters: Arc<Counters>,
    evicted: EvictedCounts,
}

struct StoredRecord {
    sequence: u64,
    collected_ms: u64,
    encoded: Box<[u8]>,
}

struct StreamSink {
    journal: Weak<Journal>,
    stream: ObservationStreamId,
    counters: Arc<Counters>,
}

#[derive(Default)]
struct Counters {
    dropped: [AtomicU64; 6],
    suppressed: AtomicU64,
    exhausted: AtomicBool,
}

/// Internal capped writer used for admission and exact response budgeting.
struct CappedWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for CappedWriter {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        if value.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("observation byte budget"));
        }
        self.bytes.extend_from_slice(value);
        Ok(value.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn encode_bounded(value: &impl Serialize, limit: usize) -> Option<Vec<u8>> {
    let mut writer = CappedWriter { bytes: Vec::new(), limit };
    serde_json::to_writer(&mut writer, value).ok()?;
    Some(writer.bytes)
}

impl ObservationJournal {
    /// Construct under the actual lease, before a committed generation exists.
    pub(crate) fn new(daemon: DaemonRef, clock: ObservationClock) -> Result<Self, ObservationReadError> {
        if daemon.validate().is_err() || encode_bounded(&daemon, 4096).is_none() {
            return Err(ObservationReadError::InvalidInput(ReadInputError::InvalidReference));
        }
        Ok(Self { inner: Arc::new(Journal { daemon, clock, closed: AtomicBool::new(false), published: AtomicBool::new(false), state: Mutex::new(None) }) })
    }

    /// Idempotent while an equal epoch is still published; otherwise create a new fence.
    pub(crate) fn publish_epoch(&self, epoch: ObservationEpoch) -> Result<ObservationStreamId, ObservationReadError> {
        if epoch.validate().is_err() || epoch.daemon != self.inner.daemon {
            return Err(ObservationReadError::InvalidInput(ReadInputError::InvalidReference));
        }
        if self.inner.closed.load(Ordering::Acquire) {
            return Err(ObservationReadError::Unavailable);
        }
        let mut state = self.inner.try_state()?;
        if self.inner.closed.load(Ordering::Acquire) {
            return Err(ObservationReadError::Unavailable);
        }
        if self.inner.published.load(Ordering::Acquire)
            && let Some(current) = state.as_ref().filter(|s| s.id.epoch == epoch && !s.counters.exhausted.load(Ordering::Acquire))
        {
            return Ok(current.id.clone());
        }
        let stream = Stream::new(epoch);
        let id = stream.id.clone();
        *state = Some(stream);
        self.inner.published.store(true, Ordering::Release);
        Ok(id)
    }

    /// Withdraw immediately before reset/mirror failure; never leave stale current reads.
    /// Busy means cleanup was deferred, not that old data remains published.
    pub(crate) fn unpublish_epoch(&self) -> Result<(), ObservationReadError> {
        self.inner.published.store(false, Ordering::Release);
        let mut state = self.inner.try_state()?;
        *state = None;
        Ok(())
    }

    /// Explicit same-dataset collector replacement, with a fresh nonce/counters.
    #[allow(dead_code, reason = "Reserved lifecycle recovery API for checked sequence/counter exhaustion; exercised by owner cases")]
    pub(crate) fn replace_stream(&self) -> Result<ObservationStreamId, ObservationReadError> {
        let mut state = self.inner.try_state()?;
        let current = self.inner.active_stream(state.as_mut())?;
        let stream = Stream::new(current.id.epoch.clone());
        let id = stream.id.clone();
        *state = Some(stream);
        Ok(id)
    }

    pub(crate) fn sink(&self) -> Result<Arc<dyn ObservationSink>, ObservationReadError> {
        let mut state = self.inner.try_state()?;
        let stream = self.inner.current(state.as_mut())?;
        Ok(Arc::new(StreamSink { journal: Arc::downgrade(&self.inner), stream: stream.id.clone(), counters: stream.counters.clone() }))
    }

    /// Called from the existing bounded lifecycle tick, including idle periods.
    pub(crate) fn prune(&self) -> Result<(), ObservationReadError> {
        let mut state = self.inner.try_state()?;
        let now = self.inner.clock.now().map_err(|_| ObservationReadError::Unavailable)?;
        self.inner.current(state.as_mut())?.expire(now.monotonic_ms.0)
    }

    pub(crate) fn close(&self) {
        self.inner.closed.store(true, Ordering::Release);
        self.inner.published.store(false, Ordering::Release);
        if let Ok(mut state) = self.inner.try_state() {
            *state = None;
        }
    }

    /// Best-effort suppression count; no active/available lock means no sample.
    /// Never produce a detail event or a recursive collection-loss event.
    #[allow(dead_code, reason = "Optional composition suppression accounting port; intentional self-traffic never becomes a detail event")]
    pub(crate) fn suppress(&self) -> RecordDisposition {
        if let Ok(mut state) = self.inner.try_state()
            && let Ok(stream) = self.inner.current(state.as_mut())
            && !checked_atomic_add(&stream.counters.suppressed, 1)
        {
            stream.counters.exhausted.store(true, Ordering::Release);
        }
        RecordDisposition::Suppressed
    }

    pub(crate) fn handle(&self, request: ObservationRequest, response_budget: usize) -> Result<ObservationResponse, ObservationReadError> {
        match request {
            ObservationRequest::Describe => self.describe().map(ObservationResponse::Head),
            ObservationRequest::Read(input) => self.read(input, response_budget).map(ObservationResponse::Read),
        }
    }

    fn describe_at(&self, at: Option<ObservationTime>) -> Result<ObservationHead, ObservationReadError> {
        let mut state = self.inner.try_state()?;
        let now = at.map(Ok).unwrap_or_else(|| self.inner.clock.now()).map_err(|_| ObservationReadError::Unavailable)?;
        let stream = self.inner.current(state.as_mut())?;
        stream.expire(now.monotonic_ms.0)?;
        Ok(stream.head(now))
    }

    fn read_at(&self, input: ObservationRead, at: Option<ObservationTime>, response_budget: usize) -> Result<ObservationReadResult, ObservationReadError> {
        if !(1..=MAX_PAGE_RECORDS).contains(&input.limit) {
            return Err(ObservationReadError::InvalidInput(ReadInputError::LimitOutOfRange));
        }
        input.window.filter.validate().map_err(ObservationReadError::InvalidInput)?;
        let requested = &input.window.after.stream;
        if requested.epoch.daemon.scope != self.inner.daemon.scope {
            return Err(ObservationReadError::InvalidInput(ReadInputError::WrongScope));
        }
        if requested.epoch.validate().is_err() || !valid_nonce(&requested.nonce) {
            return Err(ObservationReadError::InvalidInput(ReadInputError::InvalidReference));
        }
        let mut state = self.inner.try_state()?;
        let now = at.map(Ok).unwrap_or_else(|| self.inner.clock.now()).map_err(|_| ObservationReadError::Unavailable)?;
        let stream = self.inner.current(state.as_mut())?;
        stream.expire(now.monotonic_ms.0)?;
        let head = stream.head(now);
        if requested.epoch != stream.id.epoch {
            return Ok(ObservationReadResult::EpochChanged { requested: requested.epoch.clone(), head });
        }
        if requested != &stream.id {
            return Ok(ObservationReadResult::StreamChanged { requested: requested.clone(), head });
        }
        if input.window.after.sequence > input.window.through || input.window.through > head.newest {
            return Err(ObservationReadError::InvalidInput(ReadInputError::BoundaryOutOfRange));
        }
        if input.window.after.sequence < input.window.through && input.window.after.sequence + 1 < head.oldest {
            return Ok(ObservationReadResult::Gap { first_missing: input.window.after.sequence + 1, last_missing: input.window.through.min(head.oldest - 1), head });
        }
        stream.page(input, head, response_budget)
    }
}

impl ObservationReader for ObservationJournal {
    fn describe(&self) -> Result<ObservationHead, ObservationReadError> {
        self.describe_at(None)
    }

    fn read(&self, input: ObservationRead, response_budget: usize) -> Result<ObservationReadResult, ObservationReadError> {
        self.read_at(input, None, response_budget.min(MAX_PAGE_BYTES))
    }
}

impl Journal {
    fn try_state(&self) -> Result<MutexGuard<'_, Option<Stream>>, ObservationReadError> {
        self.state.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => ObservationReadError::Busy,
            TryLockError::Poisoned(_) => ObservationReadError::Unavailable,
        })
    }

    // Replacement must remain possible after sequence/counter exhaustion.
    fn active_stream<'a>(&self, stream: Option<&'a mut Stream>) -> Result<&'a mut Stream, ObservationReadError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(ObservationReadError::Unavailable);
        }
        if !self.published.load(Ordering::Acquire) {
            return Err(ObservationReadError::GenerationUnavailable { daemon: self.daemon.clone() });
        }
        stream.ok_or(ObservationReadError::Unavailable)
    }

    fn current<'a>(&self, stream: Option<&'a mut Stream>) -> Result<&'a mut Stream, ObservationReadError> {
        let stream = self.active_stream(stream)?;
        if stream.counters.exhausted.load(Ordering::Acquire) {
            return Err(ObservationReadError::Unavailable);
        }
        Ok(stream)
    }
}

impl Stream {
    fn new(epoch: ObservationEpoch) -> Self {
        Self { id: ObservationStreamId { epoch, nonce: uuid::Uuid::new_v4().to_string() }, newest: 0, bytes: 0, records: VecDeque::new(), counters: Arc::new(Counters::default()), evicted: EvictedCounts::default() }
    }

    fn head(&self, now: ObservationTime) -> ObservationHead {
        ObservationHead {
            stream: self.id.clone(),
            oldest: self.records.front().map_or(self.newest + 1, |r| r.sequence),
            newest: self.newest,
            observed_at: now.clone(),
            losses: CollectionLosses { sampled_at: now, dropped: self.counters.snapshot(), evicted: self.evicted.clone(), suppressed: self.counters.suppressed.load(Ordering::Acquire) },
        }
    }

    fn remove_first(&mut self, reason: Eviction) -> Result<(), ObservationReadError> {
        let counter = match reason {
            Eviction::Age => &mut self.evicted.age,
            Eviction::Count => &mut self.evicted.count_pressure,
            Eviction::Bytes => &mut self.evicted.byte_pressure,
        };
        let Some(next) = counter.checked_add(1) else {
            self.counters.exhausted.store(true, Ordering::Release);
            return Err(ObservationReadError::Unavailable);
        };
        if let Some(record) = self.records.pop_front() {
            self.bytes -= record.encoded.len();
            *counter = next;
        }
        Ok(())
    }

    fn expire(&mut self, now_ms: u64) -> Result<(), ObservationReadError> {
        while self.records.front().is_some_and(|r| now_ms.checked_sub(r.collected_ms).is_some_and(|age| age >= MAX_AGE_MS)) {
            self.remove_first(Eviction::Age)?;
        }
        Ok(())
    }

    fn page(&self, input: ObservationRead, head: ObservationHead, budget: usize) -> Result<ObservationReadResult, ObservationReadError> {
        let mut page = ObservationPage { records: Vec::new(), next: input.window, head };
        // Largest next frontier digit count; fixed inspection enum wrapper fits
        // in the 128-byte reserve. Count raw immutable record encodings once.
        let mut sizing = page.clone();
        sizing.next.after.sequence = sizing.next.through;
        let empty = ObservationResponse::Read(ObservationReadResult::Page(sizing));
        let overhead = encode_bounded(&empty, budget.saturating_sub(128)).ok_or(ObservationReadError::InvalidInput(ReadInputError::FilterTooLarge))?.len() + 128;
        let mut bytes = overhead;
        let after = page.next.after.sequence;
        let through = page.next.through;
        // Accepted sequences are contiguous, so index directly into the retained
        // prefix instead of traversing old records before this page's window.
        let first = self.records.front().map_or(after + 1, |r| r.sequence);
        let start = (after + 1).saturating_sub(first).min(self.records.len() as u64) as usize;
        for stored in self.records.range(start..).take(MAX_SCAN_RECORDS).take_while(|r| r.sequence <= through) {
            let record: ObservationRecord = serde_json::from_slice(&stored.encoded).map_err(|_| ObservationReadError::Unavailable)?;
            if page.next.filter.matches(&record.input) {
                let added = stored.encoded.len() + usize::from(!page.records.is_empty());
                if added > budget.saturating_sub(bytes) {
                    if page.records.is_empty() {
                        return Err(ObservationReadError::InvalidInput(ReadInputError::FilterTooLarge));
                    }
                    break;
                }
                bytes += added;
                page.records.push(record);
            }
            page.next.after.sequence = stored.sequence;
            if page.records.len() >= usize::from(input.limit) {
                break;
            }
        }
        Ok(ObservationReadResult::Page(page))
    }
}

enum Eviction {
    Age,
    Count,
    Bytes,
}

impl ObservationSink for StreamSink {
    fn record(&self, input: ObservationInput) -> RecordDisposition {
        let Some(journal) = self.journal.upgrade() else {
            return self.drop_record(DropReason::CollectorUnavailable);
        };
        self.record_at(&journal, input, None)
    }

    fn note_drop(&self, reason: DropReason, count: u64) {
        self.counters.increment(reason, count);
    }
}

impl StreamSink {
    fn drop_record(&self, reason: DropReason) -> RecordDisposition {
        self.counters.increment(reason, 1);
        RecordDisposition::Dropped(reason)
    }

    fn record_at(&self, journal: &Journal, input: ObservationInput, at: Option<ObservationTime>) -> RecordDisposition {
        if journal.closed.load(Ordering::Acquire) {
            return self.drop_record(DropReason::CollectorUnavailable);
        }
        if input.epoch != self.stream.epoch || !journal.published.load(Ordering::Acquire) {
            return self.drop_record(DropReason::RetiredEpoch);
        }
        if !input.validate() {
            return self.drop_record(DropReason::InvalidInput);
        }
        let mut state = match journal.try_state() {
            Ok(state) => state,
            Err(ObservationReadError::Busy) => return self.drop_record(DropReason::Contended),
            Err(_) => return self.drop_record(DropReason::CollectorUnavailable),
        };
        if journal.closed.load(Ordering::Acquire) {
            return self.drop_record(DropReason::CollectorUnavailable);
        }
        if !journal.published.load(Ordering::Acquire) {
            return self.drop_record(DropReason::RetiredEpoch);
        }
        let Some(stream) = state.as_mut().filter(|s| s.id == self.stream) else {
            return self.drop_record(DropReason::RetiredEpoch);
        };
        if stream.counters.exhausted.load(Ordering::Acquire) || stream.newest >= u64::MAX - 1 {
            stream.counters.exhausted.store(true, Ordering::Release);
            return self.drop_record(DropReason::CollectorUnavailable);
        }
        let now = match at.map(Ok).unwrap_or_else(|| journal.clock.now()) {
            Ok(now) => now,
            Err(_) => return self.drop_record(DropReason::CollectorUnavailable),
        };
        let cursor = ObservationCursor { stream: self.stream.clone(), sequence: stream.newest + 1 };
        let record = ObservationRecord { cursor: cursor.clone(), collected_at: now.clone(), input };
        let Some(encoded) = encode_bounded(&record, MAX_RECORD_BYTES) else {
            return self.drop_record(DropReason::Oversize);
        };
        if stream.expire(now.monotonic_ms.0).is_err() {
            return self.drop_record(DropReason::CollectorUnavailable);
        }
        while stream.records.len() >= MAX_RECORDS {
            if stream.remove_first(Eviction::Count).is_err() {
                return self.drop_record(DropReason::CollectorUnavailable);
            }
        }
        while stream.bytes + encoded.len() > MAX_RETAINED_BYTES {
            if stream.remove_first(Eviction::Bytes).is_err() {
                return self.drop_record(DropReason::CollectorUnavailable);
            }
        }
        stream.bytes += encoded.len();
        stream.newest = cursor.sequence;
        stream.records.push_back(StoredRecord { sequence: cursor.sequence, collected_ms: now.monotonic_ms.0, encoded: encoded.into_boxed_slice() });
        RecordDisposition::Recorded(cursor)
    }
}

impl Counters {
    fn increment(&self, reason: DropReason, count: u64) {
        if !checked_atomic_add(&self.dropped[reason as usize], count) {
            self.exhausted.store(true, Ordering::Release);
        }
    }

    fn snapshot(&self) -> DroppedCounts {
        let [contended, oversize, invalid_input, retired_epoch, collector_unavailable, preparation_budget] = std::array::from_fn(|i| self.dropped[i].load(Ordering::Acquire));
        DroppedCounts { contended, oversize, invalid_input, retired_epoch, collector_unavailable, preparation_budget }
    }
}

// fetch_update is available on the supported Rust 1.89 MSRV. New compilers
// deprecate its spelling in favor of try_update, which that MSRV lacks. Keep
// the standard checked atomic operation and its existing memory ordering.
#[allow(deprecated, reason = "AtomicU64::try_update is unavailable on the supported Rust 1.89 MSRV")]
fn checked_atomic_add(value: &AtomicU64, count: u64) -> bool {
    value.fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| current.checked_add(count)).is_ok()
}

fn valid_nonce(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|v| v.get_version_num() == 4 && v.get_variant() == uuid::Variant::RFC4122)
}

#[cfg(test)]
#[path = "journal_tests.rs"]
mod tests;
