//! Consumer framing only; identity, clocks, disclosure and errors stay native.

use serde::Serialize;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::daemon::observation::{InspectionResponse, InspectionResult};
use crate::observability::configuration::{CaptureSkipReason, ConfigurationResponse, LocalConfigSnapshot};
use crate::observability::context::ObservationTime;
use crate::observability::events::{ObservationReadResult, ObservationResponse};
use crate::observability::runtime::RuntimeResponse;
use crate::observability::runtime::{ClientAttemptReport, LocalFailure, ScopeLookup};

#[derive(Clone, Debug, Serialize)]
pub(super) struct LocalEvidence {
    pub inputs: Option<Result<LocalConfigSnapshot, CaptureSkipReason>>,
    pub attempt: Option<ClientAttemptReport>,
}

#[derive(Clone, Debug, Serialize)]
#[allow(clippy::large_enum_variant)] // Keep unchanged native values in the consumer envelope.
pub(super) enum ObserveResult {
    Inspection(InspectionResult),
    LocalFailure(LocalFailure),
    ConfigurationCurrent(Result<LocalConfigSnapshot, CaptureSkipReason>),
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct ObserveOutput {
    pub command: &'static str,
    pub requested_scope: Option<PathBuf>,
    pub scope: Option<ScopeLookup>,
    pub local: LocalEvidence,
    pub result: ObserveResult,
}

impl ObserveOutput {
    pub fn exit_code(&self) -> i32 {
        match &self.result {
            ObserveResult::LocalFailure(_) | ObserveResult::ConfigurationCurrent(Err(_)) => 1,
            ObserveResult::ConfigurationCurrent(Ok(_)) => 0,
            ObserveResult::Inspection(value) => match &value.response {
                InspectionResponse::DaemonChanged { .. } | InspectionResponse::Runtime(Err(_)) | InspectionResponse::Configuration(Err(_)) | InspectionResponse::Events(Err(_)) => 1,
                _ => 0,
            },
        }
    }

    pub fn event_page(&self) -> Option<&crate::observability::events::ObservationPage> {
        match &self.result {
            ObserveResult::Inspection(InspectionResult { response: InspectionResponse::Events(Ok(ObservationResponse::Read(ObservationReadResult::Page(page)))), .. }) => Some(page),
            _ => None,
        }
    }

    /// Follow promises a continuing stream; a visible native discontinuity
    /// completes a one-shot diagnosis but ends that follow with exit 1.
    pub fn follow_exit_code(&self) -> Option<i32> {
        if self.exit_code() != 0 || self.event_page().is_none() { Some(1) } else { None }
    }
}

pub(super) fn write_record(writer: &mut impl Write, record: &ObserveOutput, json: bool) -> io::Result<()> {
    // Encode before touching stdout, so a serialization failure cannot mix an
    // error string or a partial second value into a machine-readable result.
    let mut bytes = if json {
        serde_json::to_vec(record).map_err(io::Error::other)?
    } else {
        let mut bytes = Vec::new();
        write_text(&mut bytes, record)?;
        bytes
    };
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    writer.write_all(&bytes)?;
    writer.flush()
}

fn native_line(writer: &mut impl Write, label: &str, value: &impl Serialize) -> io::Result<()> {
    write!(writer, "{label}: ")?;
    serde_json::to_writer(&mut *writer, value).map_err(io::Error::other)?;
    writeln!(writer)
}

fn time_line(writer: &mut impl Write, label: &str, value: &ObservationTime) -> io::Result<()> {
    writeln!(writer, "{label}: clock={} monotonic_ms={} unix_ms={}", value.clock.0, value.monotonic_ms.0, value.unix_ms.map(|at| at.0.to_string()).unwrap_or_else(|| "unknown".into()))
}

fn write_text(writer: &mut impl Write, record: &ObserveOutput) -> io::Result<()> {
    writeln!(writer, "Observation: {} (read only)", record.command)?;
    if let Some(path) = &record.requested_scope {
        writeln!(writer, "Requested scope: {}", path.display())?;
    }
    if let Some(scope) = &record.scope {
        native_line(writer, "Scope lookup", scope)?;
    }
    if let Some(attempt) = &record.local.attempt {
        writeln!(writer, "Local source: originating client; knowledge={:?}", attempt.knowledge)?;
        time_line(writer, "Attempt started", &attempt.started_at)?;
        time_line(writer, "Attempt ended", &attempt.ended_at)?;
    } else {
        writeln!(writer, "Local attempt chronology: unknown (source clock/capture unavailable)")?;
    }
    match &record.local.inputs {
        Some(Ok(snapshot)) => {
            native_line(writer, "Local input capture", &snapshot.reference)?;
            writeln!(writer, "Input capture source: originating environment/cwd, before diagnostic scope selection")?;
            native_line(writer, "Safe input evidence", &snapshot.body)?;
        }
        Some(Err(reason)) => writeln!(writer, "Local input capture unavailable: {reason:?}")?,
        None => writeln!(writer, "Local input capture: unknown")?,
    }
    match &record.result {
        ObserveResult::LocalFailure(failure) => {
            writeln!(writer, "Local failure: stage={:?} kind={:?} os_code={:?}", failure.stage, failure.kind, failure.os_code)?;
            if let Some(at) = &failure.at {
                time_line(writer, "Failure observed", at)?;
            } else {
                writeln!(writer, "Failure time: unknown; no ordering or age can be inferred")?;
            }
            writeln!(writer, "Daemon evidence unavailable for this attempt. Its running state and product outcomes remain unknown.")?;
        }
        ObserveResult::ConfigurationCurrent(result) => {
            writeln!(writer, "Configuration: explicit DiagnosticReevaluation of this invocation's selected scope")?;
            writeln!(writer, "This new local read does not replace historical actual-use evidence.")?;
            native_line(writer, "Safe local snapshot", result)?;
        }
        ObserveResult::Inspection(value) => {
            native_line(writer, "Daemon incarnation", &value.ready.daemon)?;
            native_line(writer, "Ready generation evidence", &value.ready.generation)?;
            time_line(writer, "Ready sampled (daemon clock)", &value.ready.sampled_at)?;
            time_line(writer, "Received (originating clock)", &value.received_at)?;
            writeln!(writer, "Freshness: read received in this invocation; producer capture/retention times remain distinct. Clock domains are not subtracted.")?;
            match &value.response {
                InspectionResponse::Runtime(Ok(RuntimeResponse::Snapshot(snapshot))) => {
                    writeln!(writer, "Runtime: lifecycle={:?} activity={:?}", snapshot.lifecycle, snapshot.activity)?;
                    writeln!(
                        writer,
                        "Admission: business_connections={} queued_jobs={} executing_jobs={} inspection_connections={}",
                        snapshot.gauges.business_connections, snapshot.gauges.queued_jobs, snapshot.gauges.executing_jobs, snapshot.gauges.inspection_connections
                    )?;
                    let counts = &snapshot.recent.counters;
                    writeln!(
                        writer,
                        "Recent: calls={} admitted={} busy={} application_errors={} reply_failures={} waits_abandoned={}",
                        counts.decoded_calls, counts.queue_admissions, counts.admission_busy, counts.application_error, counts.reply_write_failed, counts.reply_wait_abandoned
                    )?;
                    writeln!(writer, "Latency buckets (upper bound in ms): 1, 5, 20, 100, 500, 2000, 10000, +Inf")?;
                    writeln!(writer, "  Queue       {:?}", snapshot.recent.queue_wait_ms.counts)?;
                    writeln!(writer, "  Application {:?}", snapshot.recent.application_ms.counts)?;
                    writeln!(writer, "  Reply       {:?}", snapshot.recent.server_to_reply_ms.counts)?;
                    native_line(writer, "Coverage and loss", &snapshot.coverage)?;
                    writeln!(writer, "A responsive inspection does not establish application readiness or a full host census.")?;
                }
                InspectionResponse::Runtime(Ok(RuntimeResponse::RequestDetail(request))) => {
                    native_line(writer, "Exact request", &request.reference)?;
                    native_line(writer, "Execution observation", &request.execution)?;
                    native_line(writer, "Reply observation", &request.reply)?;
                    native_line(writer, "Configuration uses", &request.configuration)?;
                    if let Some(work) = &request.post_commit_work {
                        native_line(writer, "Commit and follow-up work (µs)", work)?;
                    }
                    writeln!(writer, "Product effects require the original domain identity and permitted recovery read; missing evidence never authorizes replay.")?;
                }
                InspectionResponse::Configuration(Ok(ConfigurationResponse::Snapshot(_))) => {
                    writeln!(writer, "Configuration: retained historical capture; no current settings were reread.")?;
                }
                InspectionResponse::Configuration(Ok(ConfigurationResponse::Comparison(_))) => {
                    writeln!(writer, "Comparison: exact retained operands; Withheld/Unknown and incomparable boundaries keep their native meaning.")?;
                }
                InspectionResponse::Events(Ok(ObservationResponse::Read(result))) => write_event_summary(writer, result)?,
                InspectionResponse::DaemonChanged { .. } => writeln!(writer, "Daemon incarnation changed. Retire old volatile joins and explicitly select fresh evidence.")?,
                _ => {}
            }
            // Full safe native results retain precise refs, stamps and branches
            // for text users too; JSON mode wraps the same values unchanged.
            native_line(writer, "Native result", &value.response)?;
        }
    }
    Ok(())
}

fn write_event_summary(writer: &mut impl Write, result: &ObservationReadResult) -> io::Result<()> {
    match result {
        ObservationReadResult::Page(page) => {
            writeln!(writer, "Events: Page records={} scanned_after={} through={} retained_oldest={} newest={}", page.records.len(), page.next.after.sequence, page.next.through, page.head.oldest, page.head.newest)?;
            native_line(writer, "Collection losses (independent of retained gaps)", &page.head.losses)?;
            writeln!(writer, "An empty page establishes no matching returned records in this scanned interval; loss counters remain relevant.")?;
        }
        ObservationReadResult::Gap { first_missing, last_missing, .. } => {
            writeln!(writer, "Events: Gap missing_sequences={first_missing}..={last_missing}; no automatic rebase. Preserve this gap before selecting a new window.")?;
        }
        ObservationReadResult::EpochChanged { .. } => writeln!(writer, "Events: EpochChanged; explicitly select a new window and retire old dataset joins.")?,
        ObservationReadResult::StreamChanged { .. } => writeln!(writer, "Events: StreamChanged; old collector continuity is unavailable, even within the same epoch.")?,
    }
    Ok(())
}
