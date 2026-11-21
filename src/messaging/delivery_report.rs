//! Read-only delivery facts for one accepted event. A mailbox confirmation is
//! not evidence of a model turn or completed work; names never select successors.
use crate::agents::attachments::ParticipantRef;
use crate::db::AgentxDb;
use crate::service::{ServiceError, ServiceResult};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct DeliveryReport {
    pub event_id: i64,
    pub recipients: Vec<RecipientDelivery>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct RecipientDelivery {
    pub name: String,
    pub participant: Option<ParticipantRef>,
    pub mailbox: MailboxProgress,
    pub host: Option<crate::runtime::hosts::DeliveryHostProgress>,
    pub status: Option<String>,
    pub context: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MailboxProgress {
    Pending,
    Confirmed,
    TargetRetired,
    Unknown,
}

pub(crate) fn report(db: &AgentxDb, event: i64) -> ServiceResult<DeliveryReport> {
    use rusqlite::OptionalExtension;
    let raw: String = db.conn().query_row("SELECT data FROM events WHERE id=? AND type='message'", [event], |row| row.get(0)).optional().map_err(internal)?.ok_or_else(|| ServiceError::InvalidInput("accepted message event not found".into()))?;
    let data: serde_json::Value = serde_json::from_str(&raw).map_err(internal)?;
    let frozen: Vec<ParticipantRef> = data.get("scene_addressing").and_then(|value| value.get("recipients")).cloned().map(serde_json::from_value).transpose().map_err(internal)?.unwrap_or_default();
    let mut recipients = Vec::new();
    for name in data["delivered_to"].as_array().into_iter().flatten().filter_map(|value| value.as_str()) {
        let participant = frozen.iter().find(|target| target.canonical_name == name).cloned();
        let mut recipient = RecipientDelivery { name: name.into(), participant: participant.clone(), mailbox: MailboxProgress::Unknown, host: None, status: None, context: None };
        if let Some(participant) = participant {
            match db.get_instance_full(name).map_err(internal)? {
                Some(row) if row.created_at.to_bits() == participant.incarnation_bits => {
                    recipient.mailbox = if row.last_event_id >= event { MailboxProgress::Confirmed } else { MailboxProgress::Pending };
                    recipient.host = crate::runtime::hosts_store::delivery_progress(db, &participant)?;
                    recipient.status = Some(row.status);
                    recipient.context = Some(row.status_context);
                }
                _ => recipient.mailbox = MailboxProgress::TargetRetired,
            }
        }
        recipients.push(recipient);
    }
    Ok(DeliveryReport { event_id: event, recipients })
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
