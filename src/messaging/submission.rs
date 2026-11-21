//! Message routing and persistence use case. Callers own the write scope and
//! run event notifications only after commit; no provider I/O occurs here.

use crate::db::AgentxDb;
use crate::db::subscriptions::{try_add_thread_memberships, try_create_request_watches, try_get_thread_members};
use crate::messaging::routing::{InstanceInfo, MessageEnvelope, MessageScope, compute_scope, should_deliver_message, validate_message};
use crate::shared::{SenderIdentity, SenderKind};

pub(crate) struct PersistedMessage {
    pub event_id: i64,
    pub recipients: Vec<String>,
    pub routing_instance: String,
    pub data: serde_json::Value,
}

pub(crate) struct ResolvedDelivery {
    pub(crate) original_scope: MessageScope,
    pub(crate) effective_scope: MessageScope,
    pub(crate) effective_mentions: Vec<String>,
    pub(crate) delivered_to: Vec<String>,
    pub(crate) is_thread_resolved: bool,
}

fn deliverable_instances(db: &AgentxDb) -> Result<Vec<InstanceInfo>, String> {
    let rows = db
        .conn()
        .prepare(
            "SELECT name FROM instances
             WHERE status != 'stopped'
               AND status_context != 'launch_failed'
               AND NOT (status = 'inactive' AND status_context LIKE 'exit:%')",
        )
        .map_err(|e| format!("DB error: {e}"))?
        .query_map([], |row| Ok(InstanceInfo { name: row.get::<_, String>(0)? }))
        .map_err(|e| format!("DB error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("DB error: {error}"))?;
    Ok(rows)
}

pub(crate) fn resolve_delivery(db: &AgentxDb, identity: &SenderIdentity, message: &str, envelope: Option<&MessageEnvelope>, explicit_targets: Option<&[String]>) -> Result<ResolvedDelivery, String> {
    // Deliverable agents: exclude session-stopped (exit:*) and launch_failed placeholders.
    // Adhoc instances use inactive:tool:* between commands — still @mentionable.
    let rows = deliverable_instances(db)?;

    // Compute scope and routing. Thread-only sends keep their original message
    // semantics; membership only affects the delivery target set.
    let scope_result = compute_scope(message, &rows, explicit_targets)?;
    let thread_delivery_members = if let Some(thread) = envelope.and_then(|env| env.thread.as_deref()) {
        if scope_result.scope == MessageScope::Broadcast {
            let members = try_get_thread_members(db, thread).map_err(|error| error.to_string())?;
            if members.is_empty() {
                return Err(format!("Thread '{thread}' has no members. Seed it with @mentions first."));
            }
            members
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let is_thread_resolved = !thread_delivery_members.is_empty();
    let effective_scope = if thread_delivery_members.is_empty() { scope_result.scope } else { MessageScope::Mentions };
    let effective_mentions = if thread_delivery_members.is_empty() { scope_result.mentions.clone() } else { thread_delivery_members.clone() };

    let scope_data = build_scope_data(identity, effective_scope, &effective_mentions);
    let delivered_to = rows.iter().filter(|inst| should_deliver_message(&scope_data, &inst.name, &identity.name).unwrap_or(false)).map(|inst| inst.name.clone()).collect();

    Ok(ResolvedDelivery { original_scope: scope_result.scope, effective_scope, effective_mentions, delivered_to, is_thread_resolved })
}

fn build_scope_data(identity: &SenderIdentity, scope: MessageScope, mentions: &[String]) -> serde_json::Value {
    let mut scope_data = serde_json::json!({
        "scope": scope.as_str(),
    });
    if !mentions.is_empty() {
        scope_data["mentions"] = serde_json::json!(mentions);
    }
    if let Some(gid) = identity.group_id() {
        scope_data["group_id"] = serde_json::json!(gid);
    }
    scope_data
}

#[cfg(test)]
pub(crate) fn persist_message(db: &AgentxDb, identity: &SenderIdentity, message: &str, envelope: Option<&MessageEnvelope>, explicit_targets: Option<&[String]>) -> Result<PersistedMessage, String> {
    let delivery = resolve_delivery(db, identity, message, envelope, explicit_targets)?;
    persist_routed_message(db, identity, message, envelope, delivery)
}

/// Persist the recipients resolved and checked in this write scope. Generated
/// display text (such as a bundle summary) cannot change those recipients.
pub(crate) fn persist_routed_message(db: &AgentxDb, identity: &SenderIdentity, message: &str, envelope: Option<&MessageEnvelope>, delivery: ResolvedDelivery) -> Result<PersistedMessage, String> {
    use rusqlite::OptionalExtension;
    validate_message(message)?;
    let scope_str = delivery.effective_scope.as_str();

    // Build event data
    let mut data = serde_json::json!({
        "from": identity.name,
        "sender_kind": match identity.kind {
            SenderKind::External => "external",
            SenderKind::Instance => "instance",
            SenderKind::System => "system",
        },
        "scope": scope_str,
        "text": message,
        "delivered_to": delivery.delivered_to.clone(),
    });

    // Retain exact local instances for scene observation. Legacy events without
    // these facts remain unanimated; later name reuse must never rebind a flight.
    let mut exact_row = db.conn().prepare_cached("SELECT created_at FROM instances WHERE name=? AND COALESCE(origin_device_id,'')=''").map_err(|error| error.to_string())?;
    let mut exact = |name: &str| -> Result<Option<crate::agents::attachments::ParticipantRef>, String> {
        let created = exact_row.query_row([name], |row| row.get::<_, f64>(0)).optional().map_err(|error| error.to_string())?;
        Ok(created.map(|created| crate::agents::attachments::ParticipantRef { canonical_name: name.into(), incarnation_bits: created.to_bits() }))
    };
    let sender = if matches!(identity.kind, SenderKind::Instance) { exact(&identity.name)? } else { None };
    let recipients = delivery.delivered_to.iter().map(|name| exact(name)).collect::<Result<Vec<_>, _>>()?.into_iter().flatten().collect::<Vec<_>>();
    data["scene_addressing"] = serde_json::json!({"sender":sender, "recipients":recipients, "human_sender": matches!(identity.kind, SenderKind::External) && identity.name == crate::shared::SENDER, "human_recipient": delivery.effective_scope == MessageScope::Mentions && delivery.effective_mentions.iter().any(|name| name == crate::shared::SENDER)});

    // Add scope extra data (mentions)
    if !delivery.effective_mentions.is_empty() {
        data["mentions"] = serde_json::json!(delivery.effective_mentions);
    }

    if let Some(env) = envelope {
        if let Some(directory) = &env.requested_directory {
            data["requested_directory"] = serde_json::json!(directory);
        }
        if let Some(intent) = &env.intent {
            data["intent"] = serde_json::json!(intent.as_str());
        }
        if let Some(reply_to) = &env.reply_to {
            data["reply_to"] = serde_json::json!(reply_to);
            // Resolve to local event ID
            if let Some(local_id) = resolve_reply_to_local(db, reply_to) {
                data["reply_to_local"] = serde_json::json!(local_id);

                // Ack-on-ack loop prevention
                if env.intent.as_ref().map(|i| i.as_str()) == Some("ack")
                    && let Some(parent_intent) = get_intent_from_event(db, local_id)
                {
                    if parent_intent == "ack" {
                        return Err("Ack-on-ack loop detected. Message blocked.".to_string());
                    }
                    if parent_intent == "inform" {
                        return Err("Cannot ack an inform - informational messages don't need acknowledgment.".to_string());
                    }
                }
            }
        }
        if let Some(thread) = &env.thread {
            data["thread"] = serde_json::json!(thread);
        }
        if let Some(bundle_id) = &env.bundle_id {
            data["bundle_id"] = serde_json::json!(bundle_id);
        }
    }

    // Determine routing instance (namespace isolation)
    let routing_instance = match identity.kind {
        SenderKind::External => format!("ext_{}", identity.name),
        SenderKind::System => format!("sys_{}", identity.name),
        SenderKind::Instance => identity.name.clone(),
    };

    // Log event to DB
    let event_id = db.insert_event_row("message", &routing_instance, &data, None).map_err(|e| format!("Failed to write message to database: {e}"))?;

    // Auto-create request-watch subscriptions for targeted requests
    if let Some(env) = envelope {
        if let Some(thread) = env.thread.as_deref() {
            try_add_thread_memberships(db, thread, matches!(identity.kind, SenderKind::Instance).then_some(identity.name.as_str()), &delivery.delivered_to).map_err(|error| error.to_string())?;
        }

        if env.intent.as_ref().map(|i| i.as_str()) == Some("request") && matches!(identity.kind, SenderKind::Instance) && delivery.effective_scope == MessageScope::Mentions && !delivery.is_thread_resolved {
            try_create_request_watches(db, &identity.name, event_id, &delivery.delivered_to).map_err(|error| error.to_string())?;
        }
    }

    Ok(PersistedMessage { event_id, recipients: delivery.delivered_to, routing_instance, data })
}

/// Resolve reply_to to local event ID. Returns None if not found or ambiguous.
pub(crate) fn resolve_reply_to_local(db: &AgentxDb, reply_to: &str) -> Option<i64> {
    try_resolve_reply_to_local(db, reply_to).ok().flatten()
}

/// Preserve storage failure as failure, rather than claiming the reply is absent.
pub(crate) fn try_resolve_reply_to_local(db: &AgentxDb, reply_to: &str) -> anyhow::Result<Option<i64>> {
    use rusqlite::OptionalExtension;
    let reply_to = reply_to.trim();
    if let Some((origin, device)) = crate::relay::control::split_device_suffix(reply_to) {
        let Ok(origin) = origin.parse::<i64>() else {
            return Ok(None);
        };
        let mut statement = db.conn().prepare("SELECT id FROM events WHERE type='message' AND json_type(data,'$._relay.id')='integer' AND json_extract(data,'$._relay.id')=? AND json_extract(data,'$._relay.short')=? LIMIT 2")?;
        let ids = statement.query_map(rusqlite::params![origin, device], |row| row.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        return Ok((ids.len() == 1).then(|| ids[0]));
    }
    let Ok(id) = reply_to.parse::<i64>() else {
        return Ok(None);
    };
    Ok(db.conn().query_row("SELECT id FROM events WHERE type='message' AND id=?", [id], |row| row.get(0)).optional()?)
}

/// Get thread from an event (for --reply-to thread inheritance).
#[cfg(test)]
pub(crate) fn get_thread_from_event(db: &AgentxDb, event_id: i64) -> Option<String> {
    try_get_thread_from_event(db, event_id).ok().flatten()
}

pub(crate) fn try_get_thread_from_event(db: &AgentxDb, event_id: i64) -> anyhow::Result<Option<String>> {
    use rusqlite::OptionalExtension;
    Ok(db.conn().query_row("SELECT json_extract(data,'$.thread') FROM events WHERE id=?", [event_id], |row| row.get::<_, Option<String>>(0)).optional()?.flatten())
}

/// Get intent from an event (for ack-on-ack prevention).
fn get_intent_from_event(db: &AgentxDb, event_id: i64) -> Option<String> {
    db.conn().query_row("SELECT json_extract(data, '$.intent') FROM events WHERE id = ?", rusqlite::params![event_id], |row| row.get::<_, Option<String>>(0)).ok().flatten()
}
