//! Originating collection host. Provider reads occur outside daemon storage;
//! native admission rechecks generation, collection revision and source identity.
use crate::inspection::{
    collection,
    contract::{Request, Response, Snapshot, UsageSample},
};
use crate::management::contract::{Request as ManagementRequest, Response as ManagementResponse};
use crate::service::{InvocationContext, ServiceError, ServiceResult, client::Client};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

fn call(client: &Client, request: Request) -> ServiceResult<Response> {
    match client.call(crate::service::Request::Inspection(request))? {
        crate::service::Response::Inspection(result) => Ok(result),
        _ => Err(ServiceError::Internal("unexpected inspection collector response".into())),
    }
}

/// One bounded host loop belongs to the daemon's management runtime, independent
/// of browser lifetime. Collection and auto continuation start disabled.
pub(crate) fn run(scope: PathBuf, invocation: InvocationContext, stop: Arc<AtomicBool>) {
    let mut offset = 0;
    while !stop.load(Ordering::Acquire) {
        let _ = tick(&scope, &invocation, &mut offset, &stop);
        let next = Instant::now() + Duration::from_secs(10);
        while !stop.load(Ordering::Acquire) && Instant::now() < next {
            std::thread::park_timeout(Duration::from_millis(100));
        }
    }
}

fn tick(scope: &std::path::Path, invocation: &InvocationContext, offset: &mut usize, stop: &AtomicBool) -> ServiceResult<()> {
    let client = Client::connect_existing(scope, invocation.clone())?;
    let generation = client.info().database_generation.clone();
    let Response::Snapshot(snapshot) = call(&client, Request::Read { generation: generation.clone() })? else {
        return Err(ServiceError::Internal("inspection snapshot required".into()));
    };
    // Deterministic report execution has no host effects; retained queued jobs
    // safely resume after browser closure or daemon restart.
    for job in snapshot.jobs.iter().filter(|job| job.status == "queued").take(4) {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        call(&client, Request::RunAnalysis { generation: generation.clone(), id: job.id.clone() })?;
    }
    if !snapshot.settings.enabled || stop.load(Ordering::Acquire) {
        return Ok(());
    }
    collect(&client, generation, snapshot, offset, stop)
}

fn collect(client: &Client, generation: String, snapshot: Snapshot, offset: &mut usize, stop: &AtomicBool) -> ServiceResult<()> {
    let page = match client.call(crate::service::Request::Management(ManagementRequest::List { after: None, limit: 100 }))? {
        crate::service::Response::Management(ManagementResponse::Agents(page)) => page,
        _ => return Err(ServiceError::Internal("inspection Agent page required".into())),
    };
    let current: Vec<_> = page.agents.into_iter().filter(|agent| agent.agent.incarnation.is_some()).collect();
    let mut samples = Vec::new();
    let budget = current.len().min(16);
    for index in 0..budget {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        let view = &current[(*offset + index) % current.len()];
        let selected = match client.call(crate::service::Request::Management(ManagementRequest::TranscriptSource(view.agent.clone()))) {
            Ok(crate::service::Response::Management(ManagementResponse::TranscriptSource(selected))) => selected,
            _ => continue,
        };
        let Some(source) = selected.source else {
            continue;
        };
        // Avoid repeatedly parsing very large Provider histories. Missing or
        // oversized sources remain unavailable samples, never zero usage.
        let readable = std::fs::metadata(&source.path).is_ok_and(|meta| meta.is_file() && meta.len() <= 32 * 1024 * 1024);
        let loaded = readable.then(|| crate::transcript::client::read_source(&source, 1, false, false));
        let sample = match loaded {
            Some(Ok(transcript)) => collection::usage_sample(view.agent.clone(), &source, &transcript),
            _ => UsageSample { agent: view.agent.clone(), source_key: collection::source_key(&source), session_id: source.session_id.clone(), usage: None, availability: "read_failed".into() },
        };
        samples.push(sample);
    }
    if !current.is_empty() {
        *offset = (*offset + budget) % current.len();
    }
    let complete = page.next.is_none() && current.len() <= budget;
    call(client, Request::Collect { generation, expected_revision: snapshot.settings.revision, samples, discovery_complete: complete })?;
    Ok(())
}
