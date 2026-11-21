//! Durable transcript correlation; file contents remain owned by Provider readers.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct Source {
    pub name: String,
    pub path: String,
    pub tool: String,
    pub session_id: Option<String>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct Selection {
    pub canonical_name: String,
    pub source: Option<Source>,
    pub missing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_lookup: Option<CodexLookup>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct CodexLookup {
    pub home: std::path::PathBuf,
    pub session: String,
}

/// Stored transcript references in live-first, then latest-stop order. Provider
/// content, timestamp interpretation and filesystem search remain host-owned.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct TrackedSource {
    pub source: Source,
    pub stopped_at: Option<String>,
}

pub(crate) fn correlation_key(path: &str, session: Option<&str>) -> String {
    format!("{path}\u{0}{}", session.unwrap_or(""))
}
