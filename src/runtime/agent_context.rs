//! Shared, caller-owned prompt composition for startup and context recovery.
//! Domain generators supply current facts; provider adapters own delivery.
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub(crate) const MAX_CONTEXT_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContextReason {
    Startup,
    Resume,
    Compact,
}

/// A selected source. Literal text and files are never template-expanded or executed.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContextSource {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builtin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
}

/// Current resolved facts, not authority supplied by configurable prompt text.
pub(crate) struct ContextRequest<'a> {
    pub reason: ContextReason,
    pub participant: &'a str,
    pub tool: &'a str,
    pub cwd: &'a Path,
}

/// Internal module hook. Register hooks at composition, never through a shell command.
/// Implementations obtain additional domain projections using their own typed ports.
pub(crate) trait ContextGenerator {
    fn id(&self) -> &str;
    fn generate(&self, request: &ContextRequest<'_>) -> Result<String>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ContextBlock {
    pub source: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PreparedContext {
    pub blocks: Vec<ContextBlock>,
}
impl PreparedContext {
    pub fn text(&self) -> String {
        self.blocks.iter().map(|block| block.text.as_str()).collect::<Vec<_>>().join("\n\n")
    }
}

pub(crate) fn parse_sources(raw: &str) -> Result<Vec<ContextSource>> {
    let sources: Vec<ContextSource> =
        if raw.trim().is_empty() { vec![ContextSource { id: "agentx".into(), builtin: Some("agentx".into()), text: None, file: None }] } else { serde_json::from_str(raw).context("context_sources must be a JSON array of source objects")? };
    if sources.len() > 64 {
        bail!("context_sources supports at most 64 sources");
    }
    let mut ids = HashSet::new();
    for source in &sources {
        if source.id.trim().is_empty() || !ids.insert(&source.id) {
            bail!("context source IDs must be nonempty and unique");
        }
        if usize::from(source.builtin.is_some()) + usize::from(source.text.is_some()) + usize::from(source.file.is_some()) != 1 {
            bail!("context source {:?} must select exactly one of builtin, text or file", source.id);
        }
        if source.file.as_ref().is_some_and(|path| path.as_os_str().is_empty()) || source.builtin.as_ref().is_some_and(|name| name.is_empty()) {
            bail!("context source {:?} has an empty file or builtin", source.id);
        }
    }
    Ok(sources)
}

/// Generate once per real boundary. No cached startup data or once-per-session gate.
/// Failure returns no partial context; each source and the complete result are bounded.
pub(crate) fn compose(request: &ContextRequest<'_>, sources: &[ContextSource], generators: &[&dyn ContextGenerator]) -> Result<PreparedContext> {
    let mut blocks = Vec::new();
    let mut bytes = 0usize;
    for source in sources {
        let text = if let Some(name) = &source.builtin {
            let generator = generators.iter().find(|generator| generator.id() == name).with_context(|| format!("unknown context generator {name:?} for source {:?}", source.id))?;
            generator.generate(request).with_context(|| format!("context source {:?} failed for {} ({}, {:?})", source.id, request.participant, request.tool, request.reason))?
        } else if let Some(text) = &source.text {
            text.clone()
        } else if let Some(file) = &source.file {
            let path = if file.is_absolute() { file.clone() } else { request.cwd.join(file) };
            let mut data = Vec::new();
            std::fs::File::open(&path).with_context(|| format!("context source {:?}: cannot open {}", source.id, path.display()))?.take(MAX_CONTEXT_BYTES as u64 + 1).read_to_end(&mut data)?;
            if data.len() > MAX_CONTEXT_BYTES {
                bail!("context source {:?} exceeds {MAX_CONTEXT_BYTES} bytes", source.id);
            }
            String::from_utf8(data).with_context(|| format!("context source {:?} is not UTF-8", source.id))?
        } else {
            bail!("context source {:?} has no content", source.id);
        };
        if text.is_empty() {
            continue;
        }
        bytes = bytes.saturating_add(text.len()).saturating_add(if blocks.is_empty() { 0 } else { 2 });
        if bytes > MAX_CONTEXT_BYTES {
            bail!("context source {:?} makes the context exceed {MAX_CONTEXT_BYTES} bytes", source.id);
        }
        blocks.push(ContextBlock { source: source.id.clone(), text });
    }
    Ok(PreparedContext { blocks })
}

/// Stable Agent identity resolved by native lifecycle, not by prompt content.
pub(crate) struct InstructionRequest<'a> {
    pub participant: &'a crate::agents::attachments::ParticipantRef,
    pub tool: &'a str,
    pub cwd: &'a Path,
}

/// A full current-state projection at one native session boundary.
pub(crate) struct SnapshotRequest<'a> {
    pub identity: InstructionRequest<'a>,
    pub session_id: &'a str,
    pub reason: SnapshotReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SnapshotReason {
    Initial,
    AfterCompact,
}

pub(crate) trait InstructionContributor {
    fn id(&self) -> &str;
    fn instructions(&self, request: &InstructionRequest<'_>) -> Result<String>;
}
pub(crate) trait ContextContributor {
    fn id(&self) -> &str;
    fn snapshot(&self, request: &SnapshotRequest<'_>) -> Result<String>;
}

/// Stored verbatim. An empty value is a deliberate frozen absence, not a miss.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FrozenInstructions {
    pub text: String,
}

/// Compatibility: old builtins become fixed; old literals/files remain dynamic.
/// An explicit instruction selector opts into independent source lists.
pub(crate) fn select_sources(instructions: Option<&str>, context: &str) -> Result<(Vec<ContextSource>, Vec<ContextSource>)> {
    if let Some(instructions) = instructions {
        return Ok((parse_sources(instructions)?, if context.trim().is_empty() { Vec::new() } else { parse_sources(context)? }));
    }
    let (fixed, dynamic) = parse_sources(context)?.into_iter().partition(|source| source.builtin.is_some());
    Ok((fixed, dynamic))
}

pub(crate) fn prepare_instructions(request: &InstructionRequest<'_>, sources: &[ContextSource], contributors: &[&dyn InstructionContributor]) -> Result<FrozenInstructions> {
    struct Adapter<'a, 'b> {
        contributor: &'a dyn InstructionContributor,
        request: &'a InstructionRequest<'b>,
    }
    impl ContextGenerator for Adapter<'_, '_> {
        fn id(&self) -> &str {
            self.contributor.id()
        }
        fn generate(&self, _: &ContextRequest<'_>) -> Result<String> {
            self.contributor.instructions(self.request)
        }
    }
    let adapters: Vec<_> = contributors.iter().map(|contributor| Adapter { contributor: *contributor, request }).collect();
    let generators: Vec<&dyn ContextGenerator> = adapters.iter().map(|adapter| adapter as &dyn ContextGenerator).collect();
    let request = ContextRequest { reason: ContextReason::Startup, participant: &request.participant.canonical_name, tool: request.tool, cwd: request.cwd };
    Ok(FrozenInstructions { text: compose(&request, sources, &generators)?.text() })
}

fn prepare_snapshot(request: &SnapshotRequest<'_>, sources: &[ContextSource], contributors: &[&dyn ContextContributor]) -> Result<PreparedContext> {
    struct Adapter<'a, 'b> {
        contributor: &'a dyn ContextContributor,
        request: &'a SnapshotRequest<'b>,
    }
    impl ContextGenerator for Adapter<'_, '_> {
        fn id(&self) -> &str {
            self.contributor.id()
        }
        fn generate(&self, _: &ContextRequest<'_>) -> Result<String> {
            self.contributor.snapshot(self.request)
        }
    }
    let adapters: Vec<_> = contributors.iter().map(|contributor| Adapter { contributor: *contributor, request }).collect();
    let generators: Vec<&dyn ContextGenerator> = adapters.iter().map(|adapter| adapter as &dyn ContextGenerator).collect();
    let context = ContextRequest {
        reason: if request.reason == SnapshotReason::Initial { ContextReason::Startup } else { ContextReason::Compact },
        participant: &request.identity.participant.canonical_name,
        tool: request.identity.tool,
        cwd: request.identity.cwd,
    };
    let _session = request.session_id;
    compose(&context, sources, &generators)
}

pub(crate) fn prepare_initial_context(request: &SnapshotRequest<'_>, sources: &[ContextSource], contributors: &[&dyn ContextContributor]) -> Result<PreparedContext> {
    if request.reason != SnapshotReason::Initial {
        bail!("initial context requires the initial boundary");
    }
    prepare_snapshot(request, sources, contributors)
}
pub(crate) fn prepare_compact_context(request: &SnapshotRequest<'_>, sources: &[ContextSource], contributors: &[&dyn ContextContributor]) -> Result<PreparedContext> {
    if request.reason != SnapshotReason::AfterCompact {
        bail!("compact context requires the compact boundary");
    }
    prepare_snapshot(request, sources, contributors)
}

impl PreparedContext {
    /// Every snapshot explicitly supersedes previous dynamic state, including empty state.
    pub fn snapshot_text(&self) -> String {
        let text = self.text();
        format!(
            "<agentx_runtime_context>\nCurrent AgentX runtime snapshot. This supersedes all earlier AgentX runtime snapshots; omitted assignments are no longer current.\n{}\n</agentx_runtime_context>",
            if text.is_empty() { "No current runtime context." } else { &text }
        )
    }
}
