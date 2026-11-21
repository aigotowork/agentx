//! Scalar classification facts, selectors and native operations. Labels never
//! provide addressing, membership, process state or authority.
use super::attachments::ParticipantRef;
use crate::service::{ServiceError, ServiceResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type LabelSet = BTreeMap<String, String>;
pub(crate) const MAX_LABELS: usize = 64;
pub(crate) const MAX_BYTES: usize = 16 * 1024;
pub(crate) const MAX_SELECTORS: usize = 16;

/// Used on every external map carrier so repeated JSON keys are rejected before
/// deserialization can silently discard one of the user's writes.
pub(crate) fn deserialize_unique<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<LabelSet, D::Error> {
    struct Unique;
    impl<'de> serde::de::Visitor<'de> for Unique {
        type Value = LabelSet;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a map of unique label keys to strings")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            let mut labels = LabelSet::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                if labels.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate label key"));
                }
                if labels.len() > MAX_LABELS {
                    return Err(serde::de::Error::custom("at most 64 labels"));
                }
            }
            Ok(labels)
        }
    }
    deserializer.deserialize_map(Unique)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "op", rename_all = "snake_case")]
pub(crate) enum LabelPredicate {
    Eq { key: String, value: String },
    NotEq { key: String, value: String },
    Exists { key: String },
    Missing { key: String },
}
impl LabelPredicate {
    pub(crate) fn key(&self) -> &str {
        match self {
            Self::Eq { key, .. } | Self::NotEq { key, .. } | Self::Exists { key } | Self::Missing { key } => key,
        }
    }

    pub(crate) fn matches(&self, labels: &LabelSet) -> bool {
        match self {
            Self::Eq { key, value } => labels.get(key) == Some(value),
            Self::NotEq { key, value } => labels.get(key) != Some(value),
            Self::Exists { key } => labels.contains_key(key),
            Self::Missing { key } => !labels.contains_key(key),
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct LabelSnapshot {
    pub participant: ParticipantRef,
    pub generation: String,
    pub labels: LabelSet,
    pub revision: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct PatchLabels {
    pub participant: ParticipantRef,
    #[serde(default, deserialize_with = "deserialize_unique")]
    pub set: LabelSet,
    #[serde(default)]
    pub remove: Vec<String>,
    pub if_revision: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct LabelOutcome {
    pub snapshot: LabelSnapshot,
    pub changed: bool,
}
/// Current local participants only. Workspace None selects all; "global"
/// selects unassigned participants. include_global preserves CLI visibility.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub(crate) struct LabelQuery {
    #[serde(default)]
    pub selectors: Vec<LabelPredicate>,
    pub workspace: Option<String>,
    #[serde(default)]
    pub include_global: bool,
    pub directory: Option<String>,
    pub after: Option<String>,
    pub limit: u32,
}

impl LabelQuery {
    pub(crate) fn visible_agents(selectors: &[LabelPredicate], workspace: Option<&str>, after: Option<String>) -> Self {
        Self { selectors: selectors.to_vec(), workspace: workspace.map(str::to_owned), include_global: true, after, limit: 100, ..Self::default() }
    }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct LabelPage {
    pub records: Vec<crate::db::InstanceRow>,
    pub snapshots: BTreeMap<String, LabelSnapshot>,
    pub next: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Request {
    Get(ParticipantRef),
    Patch(PatchLabels),
    Select(LabelQuery),
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Response {
    Snapshot(LabelSnapshot),
    Patched(LabelOutcome),
    Page(LabelPage),
}

fn invalid(message: &str) -> ServiceError {
    ServiceError::InvalidInput(message.into())
}
pub(crate) fn validate_key(key: &str) -> ServiceResult<()> {
    let parts: Vec<_> = key.split('/').collect();
    if parts.len() > 2
        || key.starts_with("agentx/")
        || parts.iter().any(|p| {
            p.is_empty()
                || p.len() > 63
                || !p.as_bytes()[0].is_ascii_lowercase() && !p.as_bytes()[0].is_ascii_digit()
                || !p.as_bytes()[p.len() - 1].is_ascii_lowercase() && !p.as_bytes()[p.len() - 1].is_ascii_digit()
                || !p.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
        })
    {
        return Err(invalid("invalid or reserved label key"));
    }
    Ok(())
}
pub(crate) fn validate_value(value: &str) -> ServiceResult<()> {
    if value.is_empty() || value.len() > 256 || value.trim() != value || value.chars().any(char::is_control) {
        return Err(invalid("label value requires 1–256 UTF-8 bytes, no edge whitespace or controls"));
    }
    Ok(())
}
pub(crate) fn validate_set(labels: &LabelSet) -> ServiceResult<()> {
    if labels.len() > MAX_LABELS || labels.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() > MAX_BYTES {
        return Err(invalid("labels exceed 64 keys or 16 KiB"));
    }
    for (key, value) in labels {
        validate_key(key)?;
        validate_value(value)?;
    }
    Ok(())
}
pub(crate) fn parse_assignments(values: &[String]) -> ServiceResult<LabelSet> {
    let mut labels = LabelSet::new();
    for value in values {
        let (key, value) = value.split_once('=').ok_or_else(|| invalid("label assignment must be key=value"))?;
        if labels.insert(key.into(), value.into()).is_some() {
            return Err(invalid("duplicate label key"));
        }
    }
    validate_set(&labels)?;
    Ok(labels)
}
pub(crate) fn parse_selector(text: &str) -> ServiceResult<LabelPredicate> {
    let result = if let Some(key) = text.strip_prefix('!') {
        LabelPredicate::Missing { key: key.into() }
    } else if let Some((key, value)) = text.split_once('=') {
        if let Some(key) = key.strip_suffix('!') { LabelPredicate::NotEq { key: key.into(), value: value.into() } } else { LabelPredicate::Eq { key: key.into(), value: value.into() } }
    } else {
        LabelPredicate::Exists { key: text.into() }
    };
    normalize_selectors(std::slice::from_ref(&result))?;
    Ok(result)
}
pub(crate) fn normalize_selectors(selectors: &[LabelPredicate]) -> ServiceResult<Vec<LabelPredicate>> {
    if selectors.len() > MAX_SELECTORS {
        return Err(invalid("at most 16 label selectors"));
    }
    for predicate in selectors {
        validate_key(predicate.key())?;
        if let LabelPredicate::Eq { value, .. } | LabelPredicate::NotEq { value, .. } = predicate {
            validate_value(value)?;
        }
    }
    Ok(selectors.iter().cloned().collect::<BTreeSet<_>>().into_iter().collect())
}

pub(crate) use super::labels_store::{get, handle, hydrate, select_current};
