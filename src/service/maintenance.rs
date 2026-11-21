//! Native maintenance capabilities. Reset rotates durable database generation;
//! daemon restarts alone never rotate it. Local hook/config files stay client-owned.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct DatabaseSummary {
    pub local_participants: Vec<String>,
    pub event_count: i64,
    pub database_generation: String,
    pub database_path: std::path::PathBuf,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ResetResult {
    pub archive: Option<String>,
    pub database_generation: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Request {
    ReapDeadProcesses,
    Summary,
    DevRoot,
    SetDevRoot { value: Option<String> },
    TipSeen { participant: String, tip: String },
    MarkTipSeen { participant: String, tip: String },
    Archives { here: bool },
    ArchivedEvents { selector: String, here: bool, predicate: Option<String>, limit: usize },
    ArchivedParticipants { selector: String, here: bool, predicate: Option<String> },
    ResetConversation,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Response {
    Cleanup(crate::agents::service::CleanupOutcome),
    Summary(DatabaseSummary),
    DevRoot(Option<String>),
    Updated,
    TipSeen(bool),
    Archives(Vec<Value>),
    ArchivedEvents(Vec<Value>),
    ArchivedParticipants(Vec<Value>),
    Reset(ResetResult),
}
