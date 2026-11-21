//! Native evidence shared by the two AgentX mailbox adapters.

use crate::messaging::inbox::PreparedBatch;

use super::protocol::RawObject;

pub(super) fn client_id(batch: &PreparedBatch) -> String {
    format!("agentx:batch:{}", batch.id.0)
}

pub(crate) fn item_matches(item: &RawObject, correlation: &str, context: &str) -> bool {
    let string = |object: &RawObject, key| object.get(key).and_then(|value| serde_json::from_str::<String>(value.get()).ok());
    if string(item, "type").as_deref() != Some("userMessage") || string(item, "clientId").as_deref() != Some(correlation) {
        return false;
    }
    let Some(content) = item.get("content").and_then(|value| serde_json::from_str::<Vec<Box<serde_json::value::RawValue>>>(value.get()).ok()) else {
        return false;
    };
    let [text] = content.as_slice() else { return false };
    let Ok(text) = RawObject::parse(text.get()) else { return false };
    string(&text, "type").as_deref() == Some("text") && string(&text, "text").as_deref() == Some(context)
}
