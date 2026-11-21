//! Native Hermes chat-registry use cases. Provider files, subprocesses,
//! bootstrap rendering and wake-listener ownership stay in the runtime host.
use super::attachments::ParticipantRef;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) use super::hermes_store::stored_bridges;
#[cfg(test)]
pub(crate) use super::hermes_store::stored_stale_gateways;

/// Persisted legacy registry representation; tokens remain stable across
/// gateway restarts and session rotation, while participant incarnations do not.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bridge {
    pub name: String,
    pub token: String,
    pub home: String,
    pub session_key: String,
    pub platform: String,
    #[serde(default)]
    pub chat: String,
    #[serde(default)]
    pub origin: serde_json::Value,
    #[serde(default)]
    pub visibility: Visibility,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Off,
    #[default]
    Preview,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Add {
    /// Canonical absolute profile path captured by the originating host.
    pub home: PathBuf,
    pub session_key: String,
    pub platform: String,
    pub chat: String,
    pub origin: serde_json::Value,
    pub name: Option<String>,
    pub session_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RegistryEntry {
    pub bridge: Bridge,
    pub participant: Option<ParticipantRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ActiveBridge {
    pub bridge: Bridge,
    pub participant: ParticipantRef,
    pub recreated: bool,
}
/// The host records ownership before enabling injection, and forgets it only
/// after a successful Provider unset. A user-owned grant is never recorded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum GrantOperation {
    Read,
    RecordOwned,
    ClearOwned,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Request {
    Add(Add),
    Sync { home: PathBuf },
    BindSession { participant: ParticipantRef, session_id: String },
    Remove { expected: RegistryEntry },
    Visibility { participant: ParticipantRef, visibility: Visibility },
    List { home: Option<PathBuf> },
    ForgetHome { home: PathBuf },
    Grant { home: PathBuf, operation: GrantOperation },
    GatewayVersion { home: PathBuf, current: bool },
    StaleGateways,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Response {
    Active(ActiveBridge),
    Synced(Vec<ActiveBridge>),
    Registry(Vec<RegistryEntry>),
    Removed { bridge: Bridge, effects: Vec<super::hook_service::RetiredParticipant> },
    Forgotten { bridges: Vec<Bridge>, effects: Vec<super::hook_service::RetiredParticipant> },
    GrantOwned(bool),
    StaleGateways(Vec<String>),
    Changed,
}
