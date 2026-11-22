//! Read the local hook summary independently of daemon availability; opt in
//! to one bounded passive join when the user selects a trace.

use std::io::{self, Write};
use std::path::PathBuf;

use crate::daemon::observation::{self, InspectionRequest, InspectionResponse};
use crate::observability::{
    context::ObservationTime,
    runtime::{ClientInvocationId, ExecutionObservation, ExistingScopeInput, RequestObservation, RuntimeList, RuntimeRequest, RuntimeResponse, ScopeLookup},
};

pub(super) fn read(requested: Option<PathBuf>, id: Option<ClientInvocationId>, local: bool, limit: u16, json: bool, out: &mut impl Write) -> io::Result<()> {
    let cwd = std::env::current_dir()?;
    let requested = requested.unwrap_or_else(crate::paths::agentx_dir);
    let ScopeLookup::Existing(scope) = observation::resolve_existing_scope(ExistingScopeInput { requested, cwd }) else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "trace needs an existing readable scope directory"));
    };
    let trace_id = id.map(|id| id.0.to_string());
    let logs = crate::log::recent_hook_traces(&scope.canonical_scope, trace_id.as_deref(), usize::from(limit));
    let native = id.filter(|_| !local).map(|invocation| observation::inspect_existing(&scope, None, InspectionRequest::Runtime(RuntimeRequest::InvocationRequests { invocation, page: RuntimeList { before: None, limit } })));

    if json {
        serde_json::to_writer(&mut *out, &serde_json::json!({ "scope": scope, "trace_id": trace_id, "logs": logs, "native": native }))?;
        return writeln!(out);
    }

    for row in &logs {
        if let Some(summary) = row.get("trace") {
            let text = |key| summary.get(key).and_then(serde_json::Value::as_str).unwrap_or("?");
            let ms = |key| summary.get(key).and_then(serde_json::Value::as_u64).unwrap_or(0) as f64 / 1000.0;
            writeln!(out, "{} {} {}  {:.1} ms  {}", row["ts"].as_str().unwrap_or("?"), text("trace_id"), text("hook"), ms("duration_us"), text("selection"))?;
            if id.is_some() {
                if let Some(phases) = summary.get("phases").and_then(serde_json::Value::as_array) {
                    for phase in phases {
                        writeln!(out, "  {:<19} {:>8.2} ms", phase["phase"].as_str().unwrap_or("?"), phase["duration_us"].as_u64().unwrap_or(0) as f64 / 1000.0)?;
                    }
                }
                if let Some(attempts) = summary.get("attempts").and_then(serde_json::Value::as_array) {
                    for attempt in attempts {
                        let name = attempt.pointer("/operation/variant").and_then(serde_json::Value::as_str).or_else(|| attempt["kind"].as_str()).unwrap_or("?");
                        writeln!(out, "  IPC {name:<16} {:>8.2} ms  {}", attempt["duration_us"].as_u64().unwrap_or(0) as f64 / 1000.0, attempt["request_id"].as_str().unwrap_or(""))?;
                    }
                }
            }
        } else {
            writeln!(out, "{} {} {} app={} ms queue={} ms", row["ts"], row["event"], row["native_request_id"], row["duration_ms"], row["queue_wait_ms"])?;
        }
    }
    if logs.is_empty() {
        writeln!(out, "No matching retained file evidence.")?;
    }
    if let Some(result) = native {
        match result {
            Ok(result) => match result.response {
                InspectionResponse::Runtime(Ok(RuntimeResponse::Requests(page))) => {
                    if page.rows.is_empty() {
                        writeln!(out, "No matching retained requests in the current daemon.")?;
                    }
                    for row in page.rows {
                        write_row(out, &row)?;
                    }
                    if page.next_before.is_some() {
                        writeln!(out, "Daemon results limited; use a larger --limit (up to 32).")?;
                    }
                }
                InspectionResponse::Runtime(Err(error)) => writeln!(out, "Daemon evidence unavailable: {error:?}")?,
                _ => writeln!(out, "Daemon evidence unavailable: unexpected inspection response.")?,
            },
            Err(error) => writeln!(out, "Daemon evidence unavailable: {:?} at {:?}.", error.kind, error.stage)?,
        }
    }
    Ok(())
}

fn write_row(out: &mut impl Write, row: &RequestObservation) -> io::Result<()> {
    let (application, state) = match &row.execution {
        ExecutionObservation::Returned { started_at, ended_at, result } => (elapsed_ms(started_at, ended_at), format!("{result:?}")),
        ExecutionObservation::NotRun { reason, .. } => (None, format!("not run: {reason:?}")),
        ExecutionObservation::Decoded => (None, "decoded".into()),
        ExecutionObservation::Queued { .. } => (None, "queued".into()),
        ExecutionObservation::Running { .. } => (None, "running".into()),
        ExecutionObservation::Interrupted { .. } => (None, "interrupted".into()),
    };
    let queue = row.enqueued_at.as_ref().zip(row.dequeued_at.as_ref()).and_then(|(start, end)| elapsed_ms(start, end));
    let display = |value: Option<u64>| value.map_or("?".into(), |value| value.to_string());
    writeln!(out, "  daemon {:<20} queue={} ms app={} ms  {state}  {}", row.operation.variant, display(queue), display(application), row.request_id.0)
}

fn elapsed_ms(start: &ObservationTime, end: &ObservationTime) -> Option<u64> {
    (start.clock == end.clock).then(|| end.monotonic_ms.0.checked_sub(start.monotonic_ms.0)).flatten()
}
