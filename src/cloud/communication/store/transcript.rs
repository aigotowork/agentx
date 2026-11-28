//! Bounded, exact scope projection of visible Provider records and supervisor facts.

use super::*;

const RECORDS_PER_AGENT: i64 = 256;
const RECORDS_PER_SCOPE: i64 = 20_000;
const FRESH_SECONDS: i64 = 75;

pub(super) fn decode_facts(value: Option<(String, i64)>, now: i64, retired: bool) -> Result<Option<CloudAgentFacts>> {
    value.map(|(value, reported)| {
        let mut facts: CloudAgentFacts = serde_json::from_str(&value)?;
        if retired {
            facts.status = CloudAgentStatus::Stopped;
        } else if !fresh(now, reported) {
            facts.status = CloudAgentStatus::Unknown;
        }
        Ok(facts)
    }).transpose()
}

fn fresh(now: i64, reported: i64) -> bool {
    now.saturating_sub(reported) <= FRESH_SECONDS
}

fn validate(input: &PublishTranscript) -> Result<()> {
    let facts = &input.facts;
    if facts.status == CloudAgentStatus::Stopped || facts.cwd.is_empty()
        || [&facts.role, &facts.title].into_iter().flatten().chain([&facts.cwd]).any(|value| value.len() > TRANSCRIPT_FACT_BYTES || value.contains('\0'))
        || input.items.len() > TRANSCRIPT_BATCH {
        return Err(Error::Invalid("invalid Agent transcript facts or batch size"));
    }
    if input.items.iter().any(|item| item.key.len() != 64 || !item.key.bytes().all(|byte| byte.is_ascii_hexdigit())
        || item.text.len() > TRANSCRIPT_TEXT_BYTES || item.text.trim().is_empty() || item.text.contains('\0')) {
        return Err(Error::Invalid("invalid Agent transcript record"));
    }
    Ok(())
}

pub(in crate::cloud) fn publish(conn: &Connection, now: i64, input: PublishTranscript) -> Result<(bool, bool)> {
    validate(&input)?;
    current_agent(conn, now, &input.agent, &input.installation_id)?;
    let AgentView { name, incarnation, .. } = input.agent;
    let facts = serde_json::to_string(&input.facts)?;
    let previous: Option<(String, i64)> = conn.query_row(
        "SELECT facts,reported_at FROM agent_facts WHERE name=?1 AND incarnation=?2",
        params![name, incarnation], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional()?;
    let facts_changed = previous.is_none_or(|(saved, reported)| saved != facts || !fresh(now, reported));
    conn.execute(
        "INSERT INTO agent_facts VALUES (?1,?2,?3,?4) ON CONFLICT(name,incarnation) DO UPDATE SET facts=excluded.facts,reported_at=excluded.reported_at",
        params![name, incarnation, facts, now],
    )?;
    let mut inserted = false;
    for item in input.items {
        let role = i64::from(item.role == CloudTranscriptRole::Assistant);
        inserted |= conn.execute(
            "INSERT INTO agent_transcript(name,incarnation,item_key,role,text) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(name,incarnation,item_key) DO NOTHING",
            params![name, incarnation, item.key, role, item.text],
        )? > 0;
    }
    if inserted {
        conn.execute(
            "DELETE FROM agent_transcript WHERE name=?1 AND incarnation=?2 AND id <= (SELECT id FROM agent_transcript WHERE name=?1 AND incarnation=?2 ORDER BY id DESC LIMIT 1 OFFSET ?3)",
            params![name, incarnation, RECORDS_PER_AGENT],
        )?;
        // The primary-key cutoff keeps pruning indexed and also bounds retired history across the scope.
        conn.execute("DELETE FROM agent_transcript WHERE id <= (SELECT id FROM agent_transcript ORDER BY id DESC LIMIT 1 OFFSET ?1)", [RECORDS_PER_SCOPE])?;
    }
    Ok((facts_changed || inserted, facts_changed))
}

fn cursor(value: &str, allow_zero: bool) -> Result<i64> {
    let id = value.parse::<i64>().map_err(|_| Error::Invalid("invalid transcript cursor"))?;
    if id < i64::from(!allow_zero) || id.to_string() != value {
        return Err(Error::Invalid("invalid transcript cursor"));
    }
    Ok(id)
}

pub(in crate::cloud) fn history(conn: &Connection, now: i64, input: TranscriptHistoryQuery) -> Result<TranscriptHistoryPage> {
    name(&input.name)?;
    host_name(&input.host)?;
    opaque(&input.incarnation)?;
    if input.before.is_some() && input.after.is_some() {
        return Err(Error::Invalid("select one transcript cursor direction"));
    }
    let before = input.before.as_deref().map(|value| cursor(value, false)).transpose()?;
    let after = input.after.as_deref().map(|value| cursor(value, true)).transpose()?;
    let limit = page_limit(input.limit, 20, 50)?;
    let agent = AgentView { host: input.host, name: input.name, incarnation: input.incarnation };
    let exact: Option<(bool, Option<String>, Option<i64>)> = conn.query_row(
        "SELECT a.retired,f.facts,f.reported_at FROM agents a LEFT JOIN agent_facts f USING(name,incarnation) WHERE a.name=?1 AND a.incarnation=?2 AND a.host=?3",
        params![agent.name, agent.incarnation, agent.host], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional()?;
    let (retired, value, reported) = exact.ok_or(Error::NotFound("exact Agent registration does not exist"))?;
    let facts = decode_facts(value.zip(reported), now, retired)?;
    let latest: Option<i64> = conn.query_row("SELECT max(id) FROM agent_transcript WHERE name=?1 AND incarnation=?2", params![agent.name, agent.incarnation], |row| row.get(0))?;
    let mut stmt = conn.prepare(match (after, before) {
        (Some(_), _) => "SELECT id,role,text FROM agent_transcript WHERE name=?1 AND incarnation=?2 AND id>?3 ORDER BY id ASC LIMIT ?4",
        (_, Some(_)) => "SELECT id,role,text FROM agent_transcript WHERE name=?1 AND incarnation=?2 AND id<?3 ORDER BY id DESC LIMIT ?4",
        _ => "SELECT id,role,text FROM agent_transcript WHERE name=?1 AND incarnation=?2 AND id>=?3 ORDER BY id DESC LIMIT ?4",
    })?;
    let mut items = stmt.query_map(params![agent.name, agent.incarnation, after.or(before).unwrap_or(0), limit as i64 + 1], |row| {
        Ok(CloudTranscriptRecord { id: row.get::<_, i64>(0)?.to_string(), role: if row.get::<_, bool>(1)? { CloudTranscriptRole::Assistant } else { CloudTranscriptRole::User }, text: row.get(2)? })
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    let next = if items.len() > limit { items.pop(); items.last().map(|item| item.id.clone()) } else { None };
    Ok(TranscriptHistoryPage { agent, available: facts.is_some(), facts, items, next, latest: latest.map(|id| id.to_string()) })
}
