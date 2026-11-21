//! Originating-host transcript projection. No managed database or Provider calls here.
use super::contract::{Usage, UsageSample};
use crate::management::contract::AgentRef;
use crate::transcript::{Transcript, service::Source};
use sha2::{Digest, Sha256};

/// Native source identity deduplicates resumed/shared sources without exposing paths.
pub(crate) fn source_key(source: &Source) -> String {
    let mut hash = Sha256::new();
    hash.update(source.tool.as_bytes());
    hash.update([0]);
    hash.update(crate::transcript::service::correlation_key(&source.path, source.session_id.as_deref()).as_bytes());
    hash.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Project parser-normalized session totals; never include price or add exchange totals again.
pub(crate) fn usage_sample(agent: AgentRef, source: &Source, transcript: &Transcript) -> UsageSample {
    let usage = transcript.usage.as_ref().map(|session| Usage {
        input: session.tokens.input.to_string(),
        output: session.tokens.output.to_string(),
        cache_read: session.tokens.cache_read.to_string(),
        cache_write: session.tokens.cache_write.to_string(),
        reasoning: session.tokens.reasoning.map(|v| v.to_string()),
        context_tokens: session.context_tokens.map(|v| v.to_string()),
        context_window: session.context_window.map(|v| v.to_string()),
        context_usage_ratio: session.context_usage_ratio,
        model: session.model.clone(),
        unit: "tokens".into(),
        basis: "parser_normalized_session_cumulative".into(),
    });
    UsageSample { agent, source_key: source_key(source), session_id: source.session_id.clone(), usage, availability: "available".into() }
}
