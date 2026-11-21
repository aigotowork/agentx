//! Versioned, reproducible evidence counts. No AI, external effects or task-success inference.
use super::contract::{AnalysisReport, Observation};
use std::collections::BTreeSet;

pub(crate) fn summarize(observations: &[Observation]) -> AnalysisReport {
    let agents: BTreeSet<_> = observations.iter().map(|o| (&o.agent.name, &o.agent.incarnation, &o.agent.stopped_event)).collect();
    let sources: BTreeSet<_> = observations.iter().filter_map(|o| o.sample.as_ref().filter(|s| s.usage.is_some()).map(|s| &s.source_key)).collect();
    AnalysisReport {
        algorithm: "native-evidence-v1".into(),
        observation_ids: observations.iter().map(|o| o.id.clone()).collect(),
        observation_count: observations.len() as u32,
        native_error_count: observations.iter().filter(|o| o.model_request.as_ref().is_some_and(|s| s.error.is_some())).count() as u32,
        distinct_agents: agents.len() as u32,
        usage_sources: sources.len() as u32,
        unknown_usage_count: observations.iter().filter(|o| o.sample.as_ref().and_then(|s| s.usage.as_ref()).is_none()).count() as u32,
        caveats: vec![
            "Deterministic sampled evidence summary, not AI analysis or task completion".into(),
            "Counts refer to observations, not distinct failures, requests, turns or completed tasks".into(),
            "Collection begins at enable time; sampled/retained inputs do not prove complete history".into(),
            "Usage is a parser-normalized cumulative source checkpoint; repeated snapshots must not be summed".into(),
        ],
    }
}
