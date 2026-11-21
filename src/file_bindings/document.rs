//! Source document values; parsing never changes work state or infers relations.
use crate::agents::attachments::ParticipantRef;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub(crate) struct Assignment {
    pub scope: String,
    pub generation: String,
    pub participant: ParticipantRef,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NormalizedStatus {
    NotStarted,
    Ongoing,
    Completed,
    Paused,
    Blocked,
    Failed,
    Cancelled,
    Unknown,
    Undeclared,
}
impl NormalizedStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Ongoing => "ongoing",
            Self::Completed => "completed",
            Self::Paused => "paused",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Unknown => "unknown",
            Self::Undeclared => "undeclared",
        }
    }
    pub(crate) fn from_raw(raw: Option<&str>) -> Self {
        let Some(raw) = raw else {
            return Self::Undeclared;
        };
        let word = raw.trim().to_lowercase().split(|c: char| c.is_whitespace() || c == '-' || c == '_').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("_");
        match word.as_str() {
            "planned" | "plan" | "pending" | "todo" | "to_do" | "not_started" | "not_yet_started" | "draft" | "queued" | "new" | "ready" | "open" | "backlog" => Self::NotStarted,
            "active" | "planning" | "in_progress" | "inprogress" | "ongoing" | "doing" | "started" | "running" | "working" | "implementing" | "implementation" | "in_review" | "reviewing" => Self::Ongoing,
            "completed" | "complete" | "done" | "finished" | "resolved" | "closed" | "accepted" | "delivered" | "success" | "succeeded" => Self::Completed,
            "paused" | "pause" | "on_hold" | "onhold" | "suspended" | "deferred" => Self::Paused,
            "blocked" | "blocking" | "stuck" | "waiting" | "waiting_on_dependency" => Self::Blocked,
            "failed" | "failure" | "error" | "errored" => Self::Failed,
            "cancelled" | "canceled" | "cancel" | "aborted" | "abandoned" => Self::Cancelled,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Document {
    pub schema: String,
    pub description: Option<String>,
    pub raw_status: Option<String>,
    pub status: NormalizedStatus,
    /// Original scalar/object legacy declaration, never resolved by name.
    pub agent: Option<Value>,
    pub agents: Vec<Value>,
    pub assignments: Vec<Assignment>,
    /// Declared relations only; child links do not cause binding or aggregation.
    pub relations: std::collections::BTreeMap<String, Value>,
}

pub(super) const MAX_BYTES: u64 = 1024 * 1024;

/// Byte offsets preserve BOM, CRLF, body, unknown fields and comments outside agents.
pub(super) fn frontmatter(text: &str) -> Result<std::ops::Range<usize>> {
    let start = if text.starts_with('\u{feff}') { 3 } else { 0 };
    let first = text[start..].find('\n').ok_or_else(|| anyhow::anyhow!("missing YAML frontmatter"))? + start + 1;
    if text[start..first].trim_end() != "---" {
        bail!("missing YAML frontmatter");
    }
    let mut offset = first;
    for line in text[first..].split_inclusive('\n') {
        if matches!(line.trim_end(), "---" | "...") {
            return Ok(first..offset);
        }
        offset += line.len();
    }
    bail!("unterminated YAML frontmatter")
}

pub(crate) fn parse(text: &str) -> Result<Document> {
    if text.len() as u64 > MAX_BYTES {
        bail!("document exceeds 1 MiB");
    }
    let range = frontmatter(text)?;
    let value: Value = serde_yaml_ng::from_str(&text[range])?;
    let map = value.as_mapping().ok_or_else(|| anyhow::anyhow!("frontmatter must be a mapping"))?;
    let field = |key: &str| map.get(Value::String(key.into()));
    let string = |key: &str| -> Result<Option<String>> {
        match field(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            _ => bail!("{key} must be a string"),
        }
    };
    let schema = string("schema")?.ok_or_else(|| anyhow::anyhow!("missing schema"))?;
    if !matches!(schema.as_str(), "task" | "delivery") {
        bail!("unsupported schema: {schema}");
    }
    let raw_status = string("status")?;
    let agents = match field("agents") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Sequence(entries)) => entries.clone(),
        _ => bail!("agents must be a sequence"),
    };
    let assignments = agents.iter().filter_map(|value| serde_yaml_ng::from_value::<Assignment>(value.clone()).ok()).collect();
    let relations = ["links", "tasks", "dependencies", "stages", "deliveries", "models"].into_iter().filter_map(|key| field(key).map(|value| (key.into(), value.clone()))).collect();
    Ok(Document { schema, description: string("description")?, status: NormalizedStatus::from_raw(raw_status.as_deref()), raw_status, agent: field("agent").cloned(), agents, assignments, relations })
}
