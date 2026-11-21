//! Private immutable FIFO evidence store. Epoch comparison and allocation share
//! one lock; reads clone a bounded same-era batch and never call Application.

// Match the public configuration leaf's accepted inline era/report errors.
// Exact requested/current identities remain intact without allocating on the
// nonblocking admission failure path; their serialized values are bounded.
#![allow(clippy::result_large_err)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

use super::configuration::*;
use super::context::{DaemonRef, EvidenceOrigin, EvidenceStamp, EvidenceValue, ObservationClock, ObservationEpoch, ObservationTime};
use super::events::{ObservationInput, ObservationPayload, ObservationSink};

#[cfg(test)]
#[path = "configuration_store_tests.rs"]
mod tests;

#[derive(Clone)]
pub(crate) struct ConfigurationStore {
    inner: Arc<Inner>,
}
struct Inner {
    daemon: DaemonRef,
    clock: ObservationClock,
    state: Mutex<State>,
}
struct Entry {
    snapshot: ConfigSnapshot,
    bytes: usize,
}
struct State {
    epoch: Option<ObservationEpoch>,
    available: bool,
    exhausted: bool,
    snapshots: VecDeque<Entry>,
    bytes: usize,
    last_issued: u64,
    retired_through: u64,
    skipped_pressure: u64,
    skipped_oversize: u64,
    skipped_other: u64,
    limits: ConfigurationLimits,
    sink: Option<Arc<dyn ObservationSink>>,
}

impl Default for ConfigurationLimits {
    fn default() -> Self {
        Self {
            snapshots: 1024,
            payload_bytes: 8 * 1024 * 1024,
            max_age_ms: 900_000,
            snapshot_bytes: MAX_SNAPSHOT_BYTES as u32,
            fields_per_snapshot: MAX_FIELDS as u16,
            steps_per_field: MAX_STEPS as u8,
            source_reads_per_snapshot: MAX_SOURCE_READS as u8,
        }
    }
}

impl ConfigurationStore {
    /// Available for wiring before generation is known; dataset reads remain
    /// GenerationUnavailable until composition publishes a committed epoch.
    pub(crate) fn new(daemon: DaemonRef, clock: ObservationClock, sink: Option<Arc<dyn ObservationSink>>) -> Self {
        Self {
            inner: Arc::new(Inner {
                daemon,
                clock,
                state: Mutex::new(State {
                    epoch: None,
                    available: false,
                    exhausted: false,
                    snapshots: VecDeque::new(),
                    bytes: 0,
                    last_issued: 0,
                    retired_through: 0,
                    skipped_pressure: 0,
                    skipped_oversize: 0,
                    skipped_other: 0,
                    limits: ConfigurationLimits::default(),
                    sink,
                }),
            }),
        }
    }

    /// Lifecycle-only transition. Same-epoch replacement preserves the entire
    /// allocator/frontier; only a new actual epoch permits sequence reuse.
    pub(crate) fn rotate_epoch(&self, epoch: ObservationEpoch) -> Result<(), ConfigurationQueryError> {
        epoch.validate().map_err(|_| ConfigurationQueryError::InvalidReference)?;
        if epoch.daemon != self.inner.daemon {
            return Err(ConfigurationQueryError::InvalidReference);
        }
        let mut state = self.inner.state.lock().map_err(|_| ConfigurationQueryError::Unavailable)?;
        if state.epoch.as_ref() == Some(&epoch) {
            if state.exhausted {
                return Err(ConfigurationQueryError::Unavailable);
            }
            state.available = true;
            return Ok(());
        }
        state.snapshots.clear();
        state.bytes = 0;
        state.last_issued = 0;
        state.retired_through = 0;
        state.skipped_pressure = 0;
        state.skipped_oversize = 0;
        state.skipped_other = 0;
        state.exhausted = false;
        state.epoch = Some(epoch);
        state.available = true;
        Ok(())
    }

    pub(crate) fn invalidate_generation(&self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.available = false;
        }
    }

    /// Replace only the epoch-fixed event producer handle after journal rotation.
    /// Late cloned handles remain fenced by their input epoch at the journal.
    pub(crate) fn set_sink(&self, sink: Option<Arc<dyn ObservationSink>>) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.sink = sink;
        }
    }

    pub(crate) fn handle(&self, peer: &ConfigPeerBinding, request: ConfigurationRequest) -> Result<ConfigurationResponse, ConfigurationError> {
        if peer.connection.daemon != self.inner.daemon {
            return Err(ConfigurationError::DaemonChanged { expected: peer.connection.daemon.clone(), current: self.inner.daemon.clone() });
        }
        match request {
            ConfigurationRequest::Head { expected } => self.head(expected).map(ConfigurationResponse::Head).map_err(ConfigurationError::Query),
            ConfigurationRequest::Lookup(reference) => self.lookup(reference).map(ConfigurationResponse::Snapshot).map_err(ConfigurationError::Query),
            ConfigurationRequest::Compare(input) => self.compare(input).map(ConfigurationResponse::Comparison).map_err(ConfigurationError::Query),
            ConfigurationRequest::ReportSafeCapture { expected_epoch, report } => self.record_report(expected_epoch, peer, report).map(ConfigurationResponse::Reported).map_err(ConfigurationError::Report),
        }
    }

    fn state(&self) -> Result<MutexGuard<'_, State>, ConfigurationQueryError> {
        self.inner.state.try_lock().map_err(|_| ConfigurationQueryError::Busy)
    }
    fn now(&self) -> Result<ObservationTime, ConfigurationQueryError> {
        self.inner.clock.now().map_err(|_| ConfigurationQueryError::Unavailable)
    }

    fn insert(&self, expected: ObservationEpoch, body: ConfigSnapshotBody, provenance: ConfigProvenance) -> Result<ConfigSnapshotRef, ConfigReportError> {
        validate_body(&body)?;
        expected.validate().map_err(|_| ConfigReportError::InvalidFields)?;
        let at = self.inner.clock.now().map_err(|_| ConfigReportError::Unavailable)?;
        let mut state = match self.inner.state.try_lock() {
            Ok(state) => state,
            Err(_) => return Err(ConfigReportError::Pressure),
        };
        let epoch = state.current(&self.inner.daemon).map_err(query_to_report)?.clone();
        if expected != epoch {
            return Err(ConfigReportError::WrongEpoch { requested: expected, current: epoch });
        }
        validate_subject_scope(&body, &epoch)?;
        if state.exhausted {
            return Err(ConfigReportError::Unavailable);
        }
        let sequence = state.last_issued.checked_add(1).ok_or_else(|| {
            state.exhausted = true;
            ConfigReportError::Unavailable
        })?;
        let snapshot = ConfigSnapshot { reference: ConfigSnapshotRef { epoch: epoch.clone(), sequence }, body, provenance, retained_at: at.clone() };
        let bytes = encoded_size(&snapshot, state.limits.snapshot_bytes as usize).map_err(|_| ConfigReportError::Oversize)?;
        let mut removed = state.expire(&at);
        if let ConfigProvenance::ClientReported { peer, .. } = &snapshot.provenance {
            let (count, used) = state.snapshots.iter().filter(|e| same_report_peer(&e.snapshot.provenance, peer)).fold((0usize, 0usize), |(n, bytes), e| (n + 1, bytes + e.bytes));
            if count >= 64 || used.checked_add(bytes).is_none_or(|n| n > 512 * 1024) {
                let sink = state.sink.clone();
                drop(state);
                emit_retired(&sink, &epoch, &at, removed);
                return Err(ConfigReportError::PeerQuota);
            }
        }
        while state.snapshots.len() >= state.limits.snapshots as usize || state.bytes.checked_add(bytes).is_none_or(|n| n > state.limits.payload_bytes as usize) {
            let reason = if state.snapshots.len() >= state.limits.snapshots as usize { RetentionEnd::CountPressure } else { RetentionEnd::BytePressure };
            if let Some(retired) = state.retire_one(reason) {
                removed.push(retired);
            } else {
                return Err(ConfigReportError::Oversize);
            }
        }
        state.last_issued = sequence;
        state.bytes += bytes;
        let event = ConfigObservationEvent::SnapshotPublished {
            snapshot: snapshot.reference.clone(),
            invocation: snapshot.body.correlation.invocation.clone(),
            attempt: snapshot.body.correlation.attempt,
            stage: snapshot.body.stage.clone(),
            origin: origin(&snapshot.provenance),
            captured_at: snapshot.body.captured_at.clone(),
        };
        state.snapshots.push_back(Entry { snapshot, bytes });
        let sink = state.sink.clone();
        drop(state);
        emit_retired(&sink, &epoch, &at, removed);
        emit(&sink, epoch.clone(), event);
        Ok(ConfigSnapshotRef { epoch, sequence })
    }

    fn note_skipped(&self, expected: &ObservationEpoch, correlation: ConfigCorrelation, stage: ConfigStage, reason: CaptureSkipReason) {
        let Ok(mut state) = self.inner.state.try_lock() else {
            return;
        };
        // Counters describe the current era only. A failed try-lock can leave
        // them incomplete; the caller still receives the explicit skip result.
        if !state.available || state.epoch.as_ref() != Some(expected) {
            return;
        }
        let counter = match reason {
            CaptureSkipReason::Oversize => &mut state.skipped_oversize,
            CaptureSkipReason::Pressure => &mut state.skipped_pressure,
            _ => &mut state.skipped_other,
        };
        if let Some(next) = counter.checked_add(1) {
            *counter = next;
        } else {
            state.exhausted = true;
        }
        let sink = state.sink.clone();
        drop(state);
        let Ok(at) = self.inner.clock.now() else {
            return;
        };
        emit_at(&sink, expected.clone(), EvidenceOrigin::DaemonBoundary, at, ConfigObservationEvent::CaptureSkipped { invocation: correlation.invocation, attempt: correlation.attempt, stage, reason });
    }
}

impl State {
    fn current<'a>(&'a self, daemon: &DaemonRef) -> Result<&'a ObservationEpoch, ConfigurationQueryError> {
        if !self.available {
            return Err(ConfigurationQueryError::GenerationUnavailable(daemon.clone()));
        }
        self.epoch.as_ref().ok_or_else(|| ConfigurationQueryError::GenerationUnavailable(daemon.clone()))
    }
    fn check(&self, daemon: &DaemonRef, expected: &ObservationEpoch) -> Result<ObservationEpoch, ConfigurationQueryError> {
        expected.validate().map_err(|_| ConfigurationQueryError::InvalidReference)?;
        let current = self.current(daemon)?;
        if current != expected {
            return Err(ConfigurationQueryError::EpochChanged { requested: expected.clone(), current: current.clone() });
        }
        Ok(current.clone())
    }
    fn retire_one(&mut self, reason: RetentionEnd) -> Option<(u64, RetentionEnd)> {
        let entry = self.snapshots.pop_front()?;
        self.bytes -= entry.bytes;
        self.retired_through = entry.snapshot.reference.sequence;
        Some((self.retired_through, reason))
    }
    fn expire(&mut self, at: &ObservationTime) -> Vec<(u64, RetentionEnd)> {
        let mut removed = Vec::new();
        while self.snapshots.front().is_some_and(|e| at.elapsed_since(&e.snapshot.retained_at).is_ok_and(|age| age.0 >= self.limits.max_age_ms)) {
            if let Some(entry) = self.retire_one(RetentionEnd::Age) {
                removed.push(entry);
            }
        }
        removed
    }
    fn lookup(&self, reference: ConfigSnapshotRef) -> SnapshotLookup {
        if let Some(entry) = self.snapshots.iter().find(|e| e.snapshot.reference == reference) {
            return SnapshotLookup::Found(entry.snapshot.clone());
        }
        let reason = if reference.sequence <= self.retired_through { ConfigMissing::NoLongerRetained { retired_through: self.retired_through } } else { ConfigMissing::NeverIssued };
        SnapshotLookup::Missing { reference, reason }
    }
}

impl ConfigurationRecorder for ConfigurationStore {
    fn record_resolved(&self, capture: OwnedConfigCapture) -> ConfigCaptureResult {
        let provenance = match capture.provenance {
            OwnedConfigOrigin::DaemonCaptured => ConfigProvenance::DaemonCaptured,
            OwnedConfigOrigin::ApplicationResolved => ConfigProvenance::ApplicationResolved,
            OwnedConfigOrigin::StoredDomainProjection => ConfigProvenance::StoredDomainProjection,
        };
        let permitted = matches!(
            (&provenance, &capture.body.stage),
            (ConfigProvenance::DaemonCaptured, ConfigStage::StartupInputs | ConfigStage::ScopedDeclarations)
                | (ConfigProvenance::ApplicationResolved, ConfigStage::ScopedEffective)
                | (ConfigProvenance::StoredDomainProjection, ConfigStage::ParticipantState)
        );
        let correlation = capture.body.correlation.clone();
        let stage = capture.body.stage.clone();
        let result = if permitted { self.insert(capture.expected_epoch.clone(), capture.body, provenance) } else { Err(ConfigReportError::InvalidSubject) };
        match result {
            Ok(reference) => ConfigCaptureResult::Recorded(reference),
            Err(error) => {
                let reason = match error {
                    ConfigReportError::Pressure | ConfigReportError::PeerQuota => CaptureSkipReason::Pressure,
                    ConfigReportError::Oversize => CaptureSkipReason::Oversize,
                    ConfigReportError::WrongEpoch { .. } => CaptureSkipReason::StaleEpoch,
                    ConfigReportError::GenerationUnavailable(_) => CaptureSkipReason::GenerationUnavailable,
                    ConfigReportError::UnsupportedRevision => CaptureSkipReason::Unsupported,
                    _ => CaptureSkipReason::ObservationFailed,
                };
                self.note_skipped(&capture.expected_epoch, correlation, stage, reason.clone());
                ConfigCaptureResult::Skipped(reason)
            }
        }
    }

    fn record_report(&self, expected: ObservationEpoch, peer: &ConfigPeerBinding, report: ConfigReport) -> Result<ConfigSnapshotRef, ConfigReportError> {
        validate_peer(peer, &expected)?;
        let source = report.source;
        if source.reference.sequence == 0 || source.body.correlation.invocation.as_ref() != Some(&source.reference.capture) || !matches!(source.body.subject, ConfigSubject::Invocation { .. } | ConfigSubject::HostPlan { .. }) {
            return Err(ConfigReportError::InvalidSubject);
        }
        let provenance = ConfigProvenance::ClientReported { source: source.reference, peer: peer.clone() };
        self.insert(expected, source.body, provenance)
    }

    fn record_host_report(&self, expected: ObservationEpoch, peer: &ConfigPeerBinding, report: HostConfigReport) -> Result<ConfigSnapshotRef, ConfigReportError> {
        validate_peer(peer, &expected)?;
        if !matches!(report.body.subject, ConfigSubject::Host { .. }) {
            return Err(ConfigReportError::InvalidSubject);
        }
        self.insert(expected, report.body, ConfigProvenance::HostReported { peer: peer.clone() })
    }
}

impl ConfigurationReader for ConfigurationStore {
    fn head(&self, expected: Option<ObservationEpoch>) -> Result<ConfigurationHead, ConfigurationQueryError> {
        let at = self.now()?;
        let mut state = self.state()?;
        let epoch = match expected {
            Some(ref value) => state.check(&self.inner.daemon, value)?,
            None => state.current(&self.inner.daemon)?.clone(),
        };
        let removed = state.expire(&at);
        let result = ConfigurationHead {
            epoch: epoch.clone(),
            observed_at: at.clone(),
            oldest_retained: state.snapshots.front().map(|e| e.snapshot.reference.sequence),
            last_issued: state.last_issued,
            retired_through: state.retired_through,
            retained_count: state.snapshots.len() as u32,
            retained_payload_bytes: state.bytes as u64,
            skipped_pressure: state.skipped_pressure,
            skipped_oversize: state.skipped_oversize,
            skipped_other: state.skipped_other,
            limits: state.limits.clone(),
        };
        let sink = state.sink.clone();
        drop(state);
        emit_retired(&sink, &epoch, &at, removed);
        Ok(result)
    }

    fn lookup(&self, reference: ConfigSnapshotRef) -> Result<ConfigurationRead<SnapshotLookup>, ConfigurationQueryError> {
        if reference.sequence == 0 {
            return Err(ConfigurationQueryError::InvalidReference);
        }
        let at = self.now()?;
        let mut state = self.state()?;
        let epoch = state.check(&self.inner.daemon, &reference.epoch)?;
        let removed = state.expire(&at);
        let result = state.lookup(reference);
        let sink = state.sink.clone();
        drop(state);
        emit_retired(&sink, &epoch, &at, removed);
        Ok(ConfigurationRead { epoch, observed_at: at, result })
    }

    fn compare(&self, input: CompareConfig) -> Result<ConfigurationRead<CompareConfigResult>, ConfigurationQueryError> {
        validate_selection(&input.fields)?;
        if input.left.sequence == 0 || input.right.sequence == 0 || input.left.epoch.validate().is_err() || input.right.epoch.validate().is_err() {
            return Err(ConfigurationQueryError::InvalidReference);
        }
        let at = self.now()?;
        let mut state = self.state()?;
        let epoch = state.current(&self.inner.daemon)?.clone();
        let boundary = if input.left.epoch.daemon.scope != input.right.epoch.daemon.scope {
            Some(ComparisonBoundary::DifferentScope)
        } else if input.left.epoch != input.right.epoch {
            Some(ComparisonBoundary::DifferentEpoch)
        } else {
            None
        };
        if let Some(boundary) = boundary {
            return Ok(ConfigurationRead { epoch, observed_at: at, result: CompareConfigResult::Incomparable(boundary) });
        }
        state.check(&self.inner.daemon, &input.left.epoch)?;
        let removed = state.expire(&at);
        let left = state.lookup(input.left);
        let right = state.lookup(input.right);
        let sink = state.sink.clone();
        drop(state);
        emit_retired(&sink, &epoch, &at, removed);
        let result = match (left, right) {
            (SnapshotLookup::Found(left), SnapshotLookup::Found(right)) => compare_snapshots(left, right, &input.fields)?,
            (left, right) => CompareConfigResult::Unavailable { left, right },
        };
        encoded_size(&result, 128 * 1024).map_err(|_| ConfigurationQueryError::Oversize)?;
        Ok(ConfigurationRead { epoch, observed_at: at, result })
    }
}

fn validate_peer(peer: &ConfigPeerBinding, expected: &ObservationEpoch) -> Result<(), ConfigReportError> {
    if peer.connection.daemon != expected.daemon || peer.connection.id.0 == 0 || peer.process.pid == 0 || peer.process.incarnation.as_ref().is_some_and(|v| v.is_empty() || v.len() > 256) {
        return Err(ConfigReportError::InvalidSubject);
    }
    peer.observed_at.clock.validate().map_err(|_| ConfigReportError::InvalidFields)
}
fn validate_subject_scope(body: &ConfigSnapshotBody, epoch: &ObservationEpoch) -> Result<(), ConfigReportError> {
    if let EvidenceValue::Present(scope) = &body.resolution_scope
        && scope != &epoch.daemon.scope
    {
        return Err(ConfigReportError::InvalidSubject);
    }
    match &body.subject {
        ConfigSubject::DaemonStartup { daemon } if daemon != &epoch.daemon => Err(ConfigReportError::InvalidSubject),
        ConfigSubject::ScopeDeclarations { scope } if scope != &epoch.daemon.scope => Err(ConfigReportError::InvalidSubject),
        _ => Ok(()),
    }
}
fn same_report_peer(provenance: &ConfigProvenance, peer: &ConfigPeerBinding) -> bool {
    let ConfigProvenance::ClientReported { peer: other, .. } = provenance else {
        return false;
    };
    if peer.connection.daemon != other.connection.daemon {
        return false;
    }
    match (&peer.process.incarnation, &other.process.incarnation) {
        (Some(a), Some(b)) => peer.process.pid == other.process.pid && a == b,
        _ => peer.connection == other.connection,
    }
}
fn query_to_report(error: ConfigurationQueryError) -> ConfigReportError {
    match error {
        ConfigurationQueryError::GenerationUnavailable(daemon) => ConfigReportError::GenerationUnavailable(daemon),
        _ => ConfigReportError::Unavailable,
    }
}
fn origin(provenance: &ConfigProvenance) -> EvidenceOrigin {
    match provenance {
        ConfigProvenance::DaemonCaptured => EvidenceOrigin::DaemonBoundary,
        ConfigProvenance::ApplicationResolved => EvidenceOrigin::ApplicationResolved,
        ConfigProvenance::StoredDomainProjection => EvidenceOrigin::StoredDomainFact,
        ConfigProvenance::ClientReported { .. } => EvidenceOrigin::ClientReported,
        ConfigProvenance::HostReported { .. } => EvidenceOrigin::HostReported,
    }
}
fn emit(sink: &Option<Arc<dyn ObservationSink>>, epoch: ObservationEpoch, event: ConfigObservationEvent) {
    if let ConfigObservationEvent::SnapshotPublished { origin, ref captured_at, .. } = event {
        let at = captured_at.clone();
        emit_at(sink, epoch, origin, at, event);
    }
}
fn emit_at(sink: &Option<Arc<dyn ObservationSink>>, epoch: ObservationEpoch, origin: EvidenceOrigin, at: ObservationTime, event: ConfigObservationEvent) {
    if let Some(sink) = sink {
        let _ = sink.record(ObservationInput { epoch, source: EvidenceStamp { origin, at }, payload: ObservationPayload::Configuration(event) });
    }
}
fn emit_retired(sink: &Option<Arc<dyn ObservationSink>>, epoch: &ObservationEpoch, at: &ObservationTime, removed: Vec<(u64, RetentionEnd)>) {
    let mut current: Option<(u64, u32, RetentionEnd)> = None;
    for (through, reason) in removed {
        if let Some((last, count, previous)) = current.as_mut()
            && *previous == reason
        {
            *last = through;
            *count += 1;
            continue;
        }
        if let Some((through_sequence, count, reason)) = current.take() {
            emit_at(sink, epoch.clone(), EvidenceOrigin::DaemonBoundary, at.clone(), ConfigObservationEvent::SnapshotsRetired { through_sequence, count, reason });
        }
        current = Some((through, 1, reason));
    }
    if let Some((through_sequence, count, reason)) = current {
        emit_at(sink, epoch.clone(), EvidenceOrigin::DaemonBoundary, at.clone(), ConfigObservationEvent::SnapshotsRetired { through_sequence, count, reason });
    }
}
