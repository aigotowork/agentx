//! Daemon-owned bounded ledger. No filesystem, Provider reads or host dispatch under SQLite.
use super::{analysis, contract::*, recovery};
use crate::db::AgentxDb;
use crate::management::contract::{MessageInput, Request as ManagementRequest, Response as ManagementResponse};
use crate::service::{RequestContext, ServiceError, ServiceResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const MAX_OBSERVATIONS: usize = 256;
const MAX_JOBS: usize = 32;
const MAX_INPUTS: usize = 16;
const MAX_ATTEMPTS: usize = 64;
const MAX_LEDGER_BYTES: usize = 2 * 1024 * 1024;
const MAX_COLLECTED: usize = 100;
const COLLECTION_GAP_SECONDS: i64 = 120;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Ledger {
    settings: Settings,
    coverage: Coverage,
    observation_revision: u64,
    observations: Vec<Observation>,
    jobs: Vec<AnalysisJob>,
    attempts: Vec<RecoveryAttempt>,
    dropped_attempts: u64,
}
#[derive(Deserialize, Serialize)]
struct FrozenAnalysis {
    job: AnalysisJob,
    inputs: Vec<Observation>,
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn invalid(reason: &str) -> ServiceError {
    ServiceError::InvalidInput(reason.into())
}
fn conflict(reason: &str) -> ServiceError {
    ServiceError::Conflict(reason.into())
}
fn key(context: &RequestContext, suffix: &str) -> String {
    format!("inspection:{}:{suffix}", context.database_generation)
}
fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}
fn get<T: serde::de::DeserializeOwned>(db: &AgentxDb, key: &str) -> ServiceResult<Option<T>> {
    db.kv_get(key).map_err(internal)?.map(|value| serde_json::from_str(&value).map_err(internal)).transpose()
}
fn put<T: Serialize>(db: &AgentxDb, key: &str, value: &T) -> ServiceResult<()> {
    db.kv_set(key, Some(&serde_json::to_string(value).map_err(internal)?)).map_err(internal)
}
fn load(db: &AgentxDb, context: &RequestContext) -> ServiceResult<Ledger> {
    Ok(get(db, &key(context, "ledger"))?.unwrap_or_else(|| Ledger {
        settings: Settings { enabled: false, collected_from: None, revision: 0, recovery: RecoveryPolicy::default() },
        coverage: Coverage { observed_at: None, retained_from: None, discovery_complete: false, dropped_observations: 0, gaps: vec![], basis: "sampled_current_native_state_no_backfill".into() },
        observation_revision: 0,
        observations: vec![],
        jobs: vec![],
        attempts: vec![],
        dropped_attempts: 0,
    }))
}
fn gap(ledger: &mut Ledger, from: i64, through: Option<i64>, reason: &str) {
    ledger.coverage.gaps.push(Gap { from, through, reason: reason.into() });
    if ledger.coverage.gaps.len() > 64 {
        // Merge old gaps conservatively; retain an explicit statement that their detail was compacted.
        let first = ledger.coverage.gaps.remove(0);
        let next = &mut ledger.coverage.gaps[0];
        next.from = first.from.min(next.from);
        next.reason = "compacted_prior_gaps".into();
    }
}
fn save(db: &AgentxDb, context: &RequestContext, ledger: &mut Ledger) -> ServiceResult<()> {
    // Account serialized observations once; avoid repeatedly serializing the entire ledger
    // while holding the daemon write lock. Reserve 512 KiB for job/audit/coverage metadata.
    let sizes: Vec<usize> = ledger.observations.iter().map(|o| serde_json::to_vec(o).map(|v| v.len() + 1).map_err(internal)).collect::<ServiceResult<_>>()?;
    let mut bytes: usize = sizes.iter().sum();
    let mut removed = 0;
    while ledger.observations.len() - removed > MAX_OBSERVATIONS || bytes > MAX_LEDGER_BYTES * 3 / 4 {
        bytes = bytes.saturating_sub(sizes[removed]);
        removed += 1;
    }
    let evicted = removed > 0;
    ledger.observations.drain(..removed);
    ledger.coverage.dropped_observations = ledger.coverage.dropped_observations.saturating_add(removed as u64);
    ledger.coverage.retained_from = ledger.observations.first().map(|o| o.at);
    if evicted {
        let from = ledger.settings.collected_from.unwrap_or_default();
        let through = ledger.coverage.retained_from.or(ledger.coverage.observed_at);
        if let Some(previous) = ledger.coverage.gaps.iter_mut().find(|g| g.reason == "retention_eviction") {
            previous.through = through;
        } else {
            gap(ledger, from, through, "retention_eviction");
        }
    }
    if serde_json::to_vec(ledger).map_err(internal)?.len() > MAX_LEDGER_BYTES {
        return Err(conflict("inspection ledger capacity reached"));
    }
    put(db, &key(context, "ledger"), ledger)
}
fn snapshot(context: &RequestContext, mut ledger: Ledger) -> Response {
    let now = crate::shared::time::now_epoch_i64();
    if ledger.settings.enabled
        && let Some(last) = ledger.coverage.observed_at.or(ledger.settings.collected_from)
    {
        if now.saturating_sub(last) > COLLECTION_GAP_SECONDS {
            ledger.coverage.discovery_complete = false;
            gap(&mut ledger, last, None, "collector_not_observed_recently");
        }
    }
    Response::Snapshot(Snapshot {
        generation: context.database_generation.clone(),
        settings: ledger.settings,
        coverage: ledger.coverage,
        observations: ledger.observations,
        jobs: ledger.jobs,
        attempts: ledger.attempts,
        dropped_attempts: ledger.dropped_attempts,
        observation_revision: ledger.observation_revision,
    })
}
fn fence(context: &RequestContext, generation: &str) -> ServiceResult<()> {
    if generation != context.database_generation { Err(ServiceError::StaleFence) } else { Ok(()) }
}

/// Shared application entry; caller context supplies the daemon's authoritative generation.
pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    let generation = match &request {
        Request::Read { generation } | Request::Collect { generation, .. } | Request::RunAnalysis { generation, .. } => generation,
        Request::Configure(input) => &input.generation,
        Request::Analyze(input) => &input.generation,
        Request::CancelAnalysis(input) => &input.generation,
        Request::RequestContinuation(input) => &input.generation,
    };
    fence(context, generation)?;
    transaction(db, || {
        let mut ledger = load(db, context)?;
        let now = crate::shared::time::now_epoch_i64();
        let result = match request {
            Request::Read { .. } => return Ok(snapshot(context, ledger)),
            Request::Configure(ConfigureInput { expected_revision, enabled, recovery, .. }) => {
                if expected_revision != ledger.settings.revision {
                    return Err(ServiceError::StaleFence);
                }
                if !(1..=604_800).contains(&recovery.error_duration_seconds) {
                    return Err(invalid("error duration must be 1..604800 seconds"));
                }
                if recovery.enabled && !enabled {
                    return Err(invalid("recovery requires enabled collection"));
                }
                if ledger.settings.enabled && !enabled {
                    gap(&mut ledger, now, None, "collection_disabled");
                }
                if !ledger.settings.enabled && enabled {
                    ledger.settings.collected_from.get_or_insert(now);
                    for gap in &mut ledger.coverage.gaps {
                        if gap.through.is_none() && gap.reason == "collection_disabled" {
                            gap.through = Some(now);
                        }
                    }
                    ledger.coverage.discovery_complete = false;
                }
                ledger.settings.enabled = enabled;
                ledger.settings.recovery = recovery;
                ledger.settings.revision = ledger.settings.revision.checked_add(1).ok_or_else(|| conflict("revision exhausted"))?;
                None
            }
            Request::Collect { expected_revision, samples, discovery_complete, .. } => {
                if !ledger.settings.enabled || expected_revision != ledger.settings.revision {
                    return Err(ServiceError::StaleFence);
                }
                collect(db, context, &mut ledger, samples, discovery_complete, now)?;
                None
            }
            Request::Analyze(AnalyzeInput { request_id, observation_ids, .. }) => Some(Response::Job(submit_analysis(db, context, &mut ledger, request_id, observation_ids, now)?)),
            Request::RunAnalysis { id, .. } => Some(Response::Job(finish_analysis(db, context, &mut ledger, &id, false)?)),
            Request::CancelAnalysis(CancelAnalysisInput { id, .. }) => Some(Response::Job(finish_analysis(db, context, &mut ledger, &id, true)?)),
            Request::RequestContinuation(RequestContinuationInput { episode, .. }) => Some(Response::Attempt(continuation(db, context, &mut ledger, episode, now)?)),
        };
        save(db, context, &mut ledger)?;
        Ok(result.unwrap_or_else(|| snapshot(context, ledger)))
    })
}

fn collect(db: &AgentxDb, context: &RequestContext, ledger: &mut Ledger, samples: Vec<UsageSample>, discovery_complete: bool, now: i64) -> ServiceResult<()> {
    if samples.len() > MAX_COLLECTED {
        return Err(invalid("at most 100 host usage samples per collection"));
    }
    let mut supplied = BTreeSet::new();
    for sample in &samples {
        crate::management::store::validate_reference(&sample.agent)?;
        if !supplied.insert(serde_json::to_string(&sample.agent).map_err(internal)?) {
            return Err(invalid("duplicate sample for Agent"));
        }
        if sample.source_key.len() != 64 || !sample.source_key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid("source key must be opaque sha256"));
        }
        if !matches!(sample.availability.as_str(), "available" | "missing" | "read_failed") || (sample.availability != "available" && sample.usage.is_some()) {
            return Err(invalid("invalid usage availability"));
        }
        if let Some(usage) = &sample.usage {
            let counts = [&usage.input, &usage.output, &usage.cache_read, &usage.cache_write];
            let optional = [&usage.reasoning, &usage.context_tokens, &usage.context_window];
            if counts.into_iter().chain(optional.into_iter().filter_map(Option::as_ref)).any(|v| v.parse::<u64>().is_err() || v.starts_with('+')) {
                return Err(invalid("token quantities must be decimal u64 strings"));
            }
            if usage.unit != "tokens" || usage.basis != "parser_normalized_session_cumulative" || usage.model.as_ref().is_some_and(|m| m.len() > 1024) || usage.context_usage_ratio.is_some_and(|v| !v.is_finite() || v < 0.0) {
                return Err(invalid("invalid native usage projection"));
            }
        }
    }
    if let Some(previous) = ledger.coverage.observed_at {
        if now.saturating_sub(previous) > COLLECTION_GAP_SECONDS {
            gap(ledger, previous, Some(now), "collector_observation_gap");
        }
        if now < previous {
            gap(ledger, now, Some(previous), "clock_regression");
        }
    }
    // Bounded native enumeration is independent of host transcript success. No stopped history backfill.
    let names: Vec<String> = {
        let mut stmt = db.conn().prepare("SELECT name FROM instances WHERE COALESCE(origin_device_id,'')='' ORDER BY name LIMIT 101").map_err(internal)?;

        stmt.query_map([], |row| row.get(0)).map_err(internal)?.collect::<Result<_, _>>().map_err(internal)?
    };
    ledger.coverage.discovery_complete = discovery_complete && names.len() <= MAX_COLLECTED;
    if !ledger.coverage.discovery_complete {
        gap(ledger, now, Some(now), "partial_discovery");
    }
    for name in names.into_iter().take(MAX_COLLECTED) {
        let Some(row) = db.get_instance_full(&name).map_err(internal)? else {
            continue;
        };
        let reference = crate::management::contract::AgentRef { name, incarnation: Some(row.created_at.to_bits().to_string()), stopped_event: None };
        let selected = crate::transcript::store::select(db, &row.name, Some(&crate::agents::attachments::ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }))?;
        let sample = samples.iter().find(|sample| sample.agent == reference && sample.session_id == row.session_id && selected.source.as_ref().is_some_and(|source| super::collection::source_key(source) == sample.source_key)).cloned();
        let status = crate::agents::lifecycle::get_instance_status_stored(&row, db).status;
        ledger.observations.push(Observation { id: uuid::Uuid::new_v4().to_string(), at: now, agent: reference, session_id: row.session_id.clone(), status, model_request: row.model_request.as_deref().cloned(), sample });
        if ledger.settings.recovery.enabled
            && let Some(episode) = recovery::episode(&row)
        {
            if recovery::eligible(db, &row, &ledger.settings.recovery, &episode, now).is_ok() {
                // Admission errors propagate: do not conceal storage/send failures as successful collection.
                continuation(db, context, ledger, episode, now)?;
            }
        }
        if row.model_request.as_ref().is_some_and(|s| s.phase == crate::agents::model_request::RequestPhase::Completed && s.error.is_none()) {
            for attempt in &mut ledger.attempts {
                if attempt.episode.agent.name == row.name
                    && attempt.episode.agent.incarnation.as_deref() == Some(row.created_at.to_bits().to_string().as_str())
                    && row.session_id.as_deref() == Some(attempt.episode.session_id.as_str())
                    && attempt.status == "queued"
                {
                    attempt.status = "observed_error_cleared".into();
                    attempt.reason = Some("Later native successful completion observed; causation by continuation is not established".into());
                    put(db, &attempt_key(context, &attempt.episode), attempt)?;
                }
            }
        }
    }
    ledger.coverage.observed_at = Some(now);
    ledger.observation_revision = ledger.observation_revision.checked_add(1).ok_or_else(|| conflict("observation revision exhausted"))?;
    Ok(())
}

fn analysis_key(context: &RequestContext, id: &str) -> String {
    key(context, &format!("analysis:{id}"))
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 128 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
fn submit_analysis(db: &AgentxDb, context: &RequestContext, ledger: &mut Ledger, id: String, ids: Vec<String>, now: i64) -> ServiceResult<AnalysisJob> {
    if !valid_id(&id) || ids.is_empty() || ids.len() > MAX_INPUTS {
        return Err(invalid("analysis needs request_id and 1..16 exact observation IDs"));
    }
    if let Some(existing) = get::<FrozenAnalysis>(db, &analysis_key(context, &id))? {
        if existing.job.observation_ids != ids {
            return Err(conflict("analysis request ID already binds different inputs"));
        }
        return Ok(existing.job);
    }
    if ledger.jobs.len() >= MAX_JOBS {
        return Err(conflict("retained analysis capacity reached (32 jobs)"));
    }
    let mut unique = BTreeSet::new();
    let mut inputs = Vec::new();
    for id in &ids {
        if !unique.insert(id) {
            return Err(invalid("duplicate analysis observation ID"));
        }
        inputs.push(ledger.observations.iter().find(|o| &o.id == id).cloned().ok_or(ServiceError::StaleFence)?);
    }
    let job = AnalysisJob { id, input_revision: ledger.observation_revision, input_coverage: ledger.coverage.clone(), created_at: now, status: "queued".into(), observation_ids: ids, report: None };
    put(db, &analysis_key(context, &job.id), &FrozenAnalysis { job: job.clone(), inputs })?;
    ledger.jobs.push(job.clone());
    Ok(job)
}
fn finish_analysis(db: &AgentxDb, context: &RequestContext, ledger: &mut Ledger, id: &str, cancel: bool) -> ServiceResult<AnalysisJob> {
    if !valid_id(id) {
        return Err(invalid("invalid analysis ID"));
    }
    let mut record: FrozenAnalysis = get(db, &analysis_key(context, id))?.ok_or_else(|| ServiceError::NotFound("analysis job".into()))?;
    if record.job.status == "queued" {
        if cancel {
            record.job.status = "cancelled".into();
        } else {
            record.job.report = Some(analysis::summarize(&record.inputs));
            record.job.status = "succeeded".into();
        }
        put(db, &analysis_key(context, id), &record)?;
        if let Some(job) = ledger.jobs.iter_mut().find(|j| j.id == id) {
            *job = record.job.clone();
        }
    }
    Ok(record.job)
}

fn attempt_key(context: &RequestContext, episode: &Episode) -> String {
    let digest = Sha256::digest(recovery::barrier(episode).as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    key(context, &format!("episode:{hex}"))
}
fn continuation(db: &AgentxDb, context: &RequestContext, ledger: &mut Ledger, episode: Episode, now: i64) -> ServiceResult<RecoveryAttempt> {
    crate::management::store::validate_reference(&episode.agent)?;
    if let Some(attempt) = get(db, &attempt_key(context, &episode))? {
        return Ok(attempt);
    }
    if !ledger.settings.enabled {
        return Err(conflict("collection is disabled"));
    }
    let row = db.get_instance_full(&episode.agent.name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    recovery::eligible(db, &row, &ledger.settings.recovery, &episode, now)?;
    let mut attempt = RecoveryAttempt { id: uuid::Uuid::new_v4().to_string(), episode, at: now, status: "reserved".into(), message_id: None, reason: None };
    // Reservation, final native admission and audit are one transaction: a lost IPC reply is replayable
    // by episode without repeating effects. No external execution or uncertain host claim is involved.
    put(db, &attempt_key(context, &attempt.episode), &attempt)?;
    let mut message_context = context.clone();
    message_context.caller.request_id = crate::service::RequestId(format!("inspection-continuation-{}", attempt.id));
    let result = crate::management::store::handle(
        db,
        &message_context,
        ManagementRequest::Send {
            agent: attempt.episode.agent.clone(),
            message: MessageInput {
                attachments: vec![],
                text: "A sustained native model request error was observed. Please continue the current task when possible. This queued request does not confirm recovery.".into(),
                intent: Some("inform".into()),
                reply_to: None,
                thread: None,
            },
        },
    )?;
    let ManagementResponse::Sent(sent) = result else {
        return Err(internal("unexpected continuation mailbox result"));
    };
    attempt.status = "queued".into();
    attempt.message_id = Some(sent.message_id);
    put(db, &attempt_key(context, &attempt.episode), &attempt)?;
    ledger.attempts.push(attempt.clone());
    if ledger.attempts.len() > MAX_ATTEMPTS {
        ledger.attempts.remove(0);
        ledger.dropped_attempts = ledger.dropped_attempts.saturating_add(1);
    }
    Ok(attempt)
}

#[cfg(test)]
mod tests;
