//! Native event history and subscription operations. SQL is an existing event
//! predicate grammar, compiled only here against the owned event read model.

use crate::agents::attachments::ParticipantRef;
use crate::events::filters::FilterMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const MAX_PAGE_EVENTS: usize = 1000;
pub(crate) const MAX_PAGE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub(crate) struct EventQuery {
    pub filters: FilterMap,
    pub predicate: Option<String>,
    pub after: Option<i64>,
    pub through: Option<i64>,
    /// Clamped to MAX_PAGE_EVENTS. Live forward reads use next_after; merged
    /// archive/live history continues with the returned history cursor.
    pub limit: usize,
    pub oldest_first: bool,
    pub include_archives: bool,
    /// Ordinary one-shot CLI diagnosis only; never repeated by continuation.
    pub warn_unknown_agents: bool,
    pub history: Option<HistoryCursor>,
}

/// Position in a fixed archive/live window. Source names are labels, never paths.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct HistoryPosition {
    pub timestamp: String,
    pub source: String,
    pub id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct HistoryCursor {
    pub snapshot: String,
    pub position: HistoryPosition,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct EventPage {
    pub events: Vec<Value>,
    pub resolved_filters: FilterMap,
    pub newest_id: i64,
    pub next_after: i64,
    pub has_more: bool,
    pub database_generation: String,
    pub warnings: Vec<String>,
    pub history: Option<HistoryCursor>,
}

/// Temporary observation endpoint. Unlike Listen this never changes participant
/// status, subscriptions, heartbeat or delivery cursors.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ObserverSpec {
    pub participant: ParticipantRef,
    pub id: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct SubscriptionCreated {
    pub id: String,
    pub already_exists: bool,
    pub predicate: Option<String>,
    pub historical_matches: Option<i64>,
    pub latest_match: Option<Value>,
}

/// Originating host lifetime for a mailbox/filter wait. The id is correlation;
/// the daemon binds it to the authenticated peer and exact participant.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ListenSpec {
    pub participant: ParticipantRef,
    pub id: String,
    pub port: Option<u16>,
    pub filters: FilterMap,
    pub predicate: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ListenReady {
    pub recent: Option<Value>,
    pub filtered: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum ListenEnd {
    Message,
    Match,
    Timeout,
    DeliveryFailed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Request {
    Query(EventQuery),
    Watermark,
    Subscriptions,
    Subscribe { filters: FilterMap, predicate: Option<String>, owner: String, once: bool, on_hit: Option<String> },
    Unsubscribe { id: String },
    BeginListen(ListenSpec),
    PulseListen { participant: ParticipantRef, id: String },
    EndListen { participant: ParticipantRef, id: String, reason: Option<ListenEnd> },
    BeginObserver(ObserverSpec),
    CheckObserver(ObserverSpec),
    EndObserver(ObserverSpec),
    EndHistory { snapshot: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Response {
    Page(EventPage),
    Watermark { newest_id: i64, database_generation: String },
    Subscriptions(Vec<Value>),
    Subscribed(SubscriptionCreated),
    Unsubscribed(bool),
    ListenReady(ListenReady),
    ListenState { connected: bool },
    ListenEnded,
    ObserverReady,
    ObserverEnded,
    HistoryEnded,
}
