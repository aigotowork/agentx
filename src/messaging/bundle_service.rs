//! Context bundle storage contracts. Files and transcript contents are read by
//! the originating host; these operations expose only owned event projections.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub(crate) enum Category {
    MessagesTo,
    MessagesFrom,
    FileOperations,
    Lifecycle,
}

/// Pages use one frozen event watermark. `next` is an exclusive ID: descending
/// for List/Context, ascending for ReferencedEvents. Limits never expand a range.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Page {
    pub events: Vec<Value>,
    pub through: i64,
    pub next: Option<i64>,
    pub has_more: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Created {
    pub bundle_id: String,
    pub event_id: i64,
    pub warnings: Vec<String>,
    pub replayed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Request {
    Lookup { id: String },
    List { before: Option<i64>, through: Option<i64>, limit: usize },
    Context { agent: String, category: Category, before: Option<i64>, through: Option<i64>, limit: usize },
    ReferencedEvents { refs: Vec<Value>, after: Option<i64>, through: Option<i64>, limit: usize },
    Create { bundle: Value, actor: Option<crate::agents::attachments::ParticipantRef> },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Response {
    Lookup(Option<Value>),
    Page(Page),
    Created(Created),
}
