//! Provider confirmation remains a host action; a cloud claim alone never marks delivery.
use super::*;

fn batch(claim_id: &str, items: Vec<MessageView>) -> InboxBatch {
    InboxBatch { claim_id: claim_id.to_owned(), items, next: None }
}

fn owner(conn: &Connection, claim_id: &str, agent: &AgentView) -> Result<Option<bool>> {
    let claim = conn.query_row("SELECT agent_name,agent_incarnation,closed FROM claims WHERE claim_id=?1", [claim_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, bool>(2)?))).optional()?;
    match claim {
        Some((name, incarnation, closed)) if name == agent.name && incarnation == agent.incarnation => Ok(Some(closed)),
        Some(_) => Err(Error::Conflict("claim ID belongs to another agent instance")),
        None => Ok(None),
    }
}

fn remaining(conn: &Connection, claim_id: &str) -> Result<InboxBatch> {
    let mut stmt = conn.prepare(&format!("SELECT {MESSAGE_COLUMNS} FROM messages WHERE claim_id=?1 AND claim_id IS NOT NULL AND confirmed=0 ORDER BY id"))?;
    let items = stmt.query_map([claim_id], read_message)?.collect::<rusqlite::Result<_>>()?;
    Ok(batch(claim_id, items))
}

pub(super) fn claim(conn: &Connection, now: i64, input: &ClaimInbox) -> Result<(InboxBatch, bool)> {
    current_agent(conn, now, &input.agent, &input.installation_id)?;
    opaque(&input.claim_id)?;
    let limit = page_limit(input.limit, 20, BATCH_LIMIT)?;
    if let Some(closed) = owner(conn, &input.claim_id, &input.agent)? {
        let existing = if closed { batch(&input.claim_id, Vec::new()) } else { remaining(conn, &input.claim_id)? };
        return Ok((existing, false));
    }
    let unsettled: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM claims WHERE agent_name=?1 AND agent_incarnation=?2 AND closed=0)", params![input.agent.name, input.agent.incarnation], |row| row.get(0))?;
    if unsettled {
        return Err(Error::ClaimOutstanding);
    }
    let mut stmt = conn.prepare(&format!("SELECT {MESSAGE_COLUMNS} FROM messages WHERE to_name=?1 AND to_incarnation=?2 AND confirmed=0 AND claim_id IS NULL ORDER BY id LIMIT ?3",))?;
    let rows = stmt.query_map(params![input.agent.name, input.agent.incarnation, limit as i64], read_message)?;
    let mut items = Vec::new();
    let mut body_bytes = 0;
    for row in rows {
        let row = row?;
        body_bytes += row.body.len();
        if body_bytes > MAX_BATCH_BODY_BYTES {
            break;
        }
        items.push(row);
    }
    if !items.is_empty() {
        conn.execute("INSERT INTO claims(claim_id,agent_name,agent_incarnation) VALUES(?1,?2,?3)", params![input.claim_id, input.agent.name, input.agent.incarnation])?;
        for item in &items {
            conn.execute("UPDATE messages SET claim_id=?1 WHERE id=?2", params![input.claim_id, message_id(&item.id)?])?;
        }
    }
    let reusable = items.is_empty();
    Ok((batch(&input.claim_id, items), reusable))
}

pub(super) fn confirm(conn: &Connection, now: i64, input: ConfirmInbox) -> Result<InboxConfirmation> {
    current_agent(conn, now, &input.agent, &input.installation_id)?;
    opaque(&input.claim_id)?;
    owner(conn, &input.claim_id, &input.agent)?.ok_or(Error::NotFound("inbox claim was not found"))?;
    if input.message_ids.is_empty() || input.message_ids.len() > BATCH_LIMIT as usize {
        return Err(Error::Invalid("confirm requires 1–100 message IDs"));
    }
    let mut seen = std::collections::HashSet::new();
    for message in &input.message_ids {
        let id = message_id(message)?;
        if !seen.insert(id) {
            return Err(Error::Invalid("confirm message IDs must be unique"));
        }
        let count = conn.execute("UPDATE messages SET confirmed=1 WHERE id=?1 AND claim_id=?2", params![id, input.claim_id])?;
        if count != 1 {
            return Err(Error::Conflict("message does not belong to this claim"));
        }
    }
    conn.execute("UPDATE claims SET closed=1 WHERE claim_id=?1 AND NOT EXISTS(SELECT 1 FROM messages WHERE claim_id=?1 AND claim_id IS NOT NULL AND confirmed=0)", [&input.claim_id])?;
    Ok(InboxConfirmation { confirmed: input.message_ids })
}

pub(super) fn recover(conn: &Connection, now: i64, input: RecoverInbox) -> Result<InboxBatch> {
    current_agent(conn, now, &input.agent, &input.installation_id)?;
    opaque(&input.claim_id)?;
    let closed = owner(conn, &input.claim_id, &input.agent)?.ok_or(Error::NotFound("inbox claim was not found"))?;
    if closed || !input.release {
        return remaining(conn, &input.claim_id);
    }
    conn.execute("UPDATE messages SET claim_id=NULL WHERE claim_id=?1 AND confirmed=0", [&input.claim_id])?;
    conn.execute("UPDATE claims SET closed=1 WHERE claim_id=?1", [&input.claim_id])?;
    Ok(batch(&input.claim_id, Vec::new()))
}
