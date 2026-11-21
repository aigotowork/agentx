//! Canonical message values for frozen inbox formatting. Presentation adapters
//! can reuse this mapping without the daemon depending on hook configuration.
use crate::db::Message;
use serde_json::{Value, json};

pub(crate) fn message_to_value(message: &Message) -> Value {
    let text = match &message.requested_directory {
        Some(directory) => format!("[Requested directory: {directory}]\n{}", message.text),
        None => message.text.clone(),
    };
    let mut value = json!({"from":message.from,"message":text});
    let object = value.as_object_mut().unwrap();
    for (key, field) in [("intent", &message.intent), ("thread", &message.thread), ("timestamp", &message.timestamp), ("bundle_id", &message.bundle_id), ("requested_directory", &message.requested_directory)] {
        if let Some(field) = field {
            object.insert(key.into(), json!(field));
        }
    }
    if let Some(id) = message.event_id {
        object.insert("event_id".into(), json!(id));
    }
    if let Some(recipients) = &message.delivered_to {
        object.insert("delivered_to".into(), json!(recipients));
    }
    value
}
