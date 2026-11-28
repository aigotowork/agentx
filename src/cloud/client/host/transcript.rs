//! Project only visible text from the owned Codex thread and publish it without blocking Provider IO.

use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, LazyLock, Mutex, mpsc},
    time::{Duration, Instant},
};

use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::cloud::communication::types::{
    CloudAgentFacts, CloudAgentStatus, CloudTranscriptRole, PublishTranscript, PublishTranscriptRecord,
    TRANSCRIPT_BATCH, TRANSCRIPT_FACT_BYTES, TRANSCRIPT_TEXT_BYTES,
};
use super::{Actor, Codex, LaunchSpec, Port, RPC_TIMEOUT, floor_char_boundary};
use super::super::port::Command;

const PENDING: usize = 96;
const SEEN: usize = 256;
const COALESCE: Duration = Duration::from_millis(250);
const RENEW: Duration = Duration::from_secs(30);
const RETRY: Duration = Duration::from_secs(2);
const TRUNCATED: &str = "\n[AgentX truncated this record]";

struct Pending {
    facts: CloudAgentFacts,
    items: VecDeque<PublishTranscriptRecord>,
}

pub(super) struct Projection {
    pending: Arc<Mutex<Pending>>,
    wake: mpsc::SyncSender<()>,
    secrets: Vec<String>,
    seen: HashSet<String>,
    seen_order: VecDeque<String>,
}

impl Projection {
    pub(super) fn start(port: Port, actor: Actor, spec: &LaunchSpec, status: CloudAgentStatus) -> Result<Self> {
        let mut secrets: Vec<_> = std::env::vars().filter(|(name, value)| credential_name(name) && value.len() >= 8).map(|(_, value)| value).collect();
        if let super::Source::Direct(credentials) = &spec.source {
            secrets.push(credentials.token.clone());
        }
        secrets.sort_by_key(|value| std::cmp::Reverse(value.len()));
        secrets.dedup();
        let facts = CloudAgentFacts {
            role: spec.role.as_deref().map(|value| safe_text(value, &secrets, TRANSCRIPT_FACT_BYTES)),
            title: spec.title.as_deref().map(|value| safe_text(value, &secrets, TRANSCRIPT_FACT_BYTES)),
            cwd: safe_text(&spec.cwd.to_string_lossy(), &secrets, TRANSCRIPT_FACT_BYTES),
            status,
        };
        let pending = Arc::new(Mutex::new(Pending { facts, items: VecDeque::new() }));
        let (wake, receiver) = mpsc::sync_channel(1);
        let shared = pending.clone();
        std::thread::Builder::new().name("cloud-transcript".into()).spawn(move || publish(port, actor, shared, receiver))?;
        Ok(Self { pending, wake, secrets, seen: HashSet::new(), seen_order: VecDeque::new() })
    }

    pub(super) fn status(&self, status: CloudAgentStatus) {
        let mut pending = self.pending.lock().expect("transcript state lock");
        if pending.facts.status != status {
            pending.facts.status = status;
            let _ = self.wake.try_send(());
        }
    }

    pub(super) fn title(&self, title: Option<&str>) {
        let title = title.map(|value| safe_text(value, &self.secrets, TRANSCRIPT_FACT_BYTES));
        let mut pending = self.pending.lock().expect("transcript state lock");
        if pending.facts.title != title {
            pending.facts.title = title;
            let _ = self.wake.try_send(());
        }
    }

    pub(super) fn item(&mut self, turn: &str, item: &Value) {
        let Some(item_id) = item["id"].as_str().filter(|id| !id.is_empty()) else { return };
        if turn.is_empty() { return; }
        let key: String = Sha256::digest(format!("{turn}\0{item_id}")).iter().map(|b| format!("{b:02x}")).collect();
        if self.seen.contains(&key) { return; }
        let Some((role, text)) = visible_item(item) else { return };
        let text = safe_text(&text, &self.secrets, TRANSCRIPT_TEXT_BYTES);
        if text.trim().is_empty() { return; }
        self.seen.insert(key.clone());
        self.seen_order.push_back(key.clone());
        if self.seen_order.len() > SEEN && let Some(old) = self.seen_order.pop_front() {
            self.seen.remove(&old);
        }
        let mut pending = self.pending.lock().expect("transcript state lock");
        if pending.items.len() >= PENDING { pending.items.pop_front(); }
        pending.items.push_back(PublishTranscriptRecord { key, role, text });
        let _ = self.wake.try_send(());
    }

    /// A recovered session gets one finite tail scan; ordinary updates use notifications.
    pub(super) fn recover(native: &mut Codex) -> Result<()> {
        let result = native.call_until("thread/items/list", serde_json::json!({"threadId":native.thread,"limit":80,"sortDirection":"desc"}), Instant::now() + RPC_TIMEOUT)?;
        if let (Some(rows), Some(projection)) = (result["data"].as_array(), &mut native.projection) {
            for row in rows.iter().rev() {
                if let Some(turn) = row["turnId"].as_str() {
                    projection.item(turn, &row["item"]);
                }
            }
        }
        Ok(())
    }
}

fn publish(port: Port, actor: Actor, shared: Arc<Mutex<Pending>>, wake: mpsc::Receiver<()>) {
    let mut reported = None;
    let mut deadline = Instant::now();
    let mut delay = RETRY;
    let mut warned = false;
    loop {
        match wake.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(()) => {
                // A second notification may arrive during the short coalescing window.
                std::thread::sleep(COALESCE);
                while wake.try_recv().is_ok() {}
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        let (facts, items) = {
            let pending = shared.lock().expect("transcript state lock");
            (pending.facts.clone(), pending.items.iter().take(TRANSCRIPT_BATCH).cloned().collect::<Vec<_>>())
        };
        let now = Instant::now();
        if now < deadline && (warned || items.is_empty() && reported.as_ref() == Some(&facts)) {
            continue;
        }
        let last = items.last().map(|item| item.key.clone());
        let command = Command::PublishTranscript(PublishTranscript {
            generation: actor.generation.clone(), installation_id: actor.installation_id.clone(), agent: actor.agent.clone(), facts: facts.clone(), items,
        });
        match port.call::<bool>(command) {
            Ok(_) => {
                let mut pending = shared.lock().expect("transcript state lock");
                if let Some(index) = last.and_then(|key| pending.items.iter().position(|item| item.key == key)) {
                    pending.items.drain(..=index);
                }
                deadline = if pending.items.is_empty() && pending.facts == facts { Instant::now() + RENEW } else { Instant::now() + COALESCE };
                reported = Some(facts);
                delay = RETRY;
                warned = false;
            }
            Err(_) => {
                if !warned {
                    // Never print transport details from an endpoint that saw transcript content.
                    eprintln!("Cloud transcript publishing unavailable; will retry");
                }
                warned = true;
                deadline = Instant::now() + delay;
                delay = (delay * 2).min(RENEW);
            }
        }
    }
}

fn visible_item(item: &Value) -> Option<(CloudTranscriptRole, String)> {
    if !(item["moderation"].is_null() || item["moderation"]["display"]["mode"] == "show") { return None; }
    match item["type"].as_str()? {
        "userMessage" => {
            let mut text = String::new();
            for part in item["content"].as_array()? {
                if part["type"] == "text" && let Some(part) = part["text"].as_str() {
                    if !text.is_empty() { text.push('\n'); }
                    text.push_str(part);
                }
            }
            Some((CloudTranscriptRole::User, text))
        }
        "agentMessage" if item["phase"] == "final_answer" && item["delivery"].is_null() => {
            Some((CloudTranscriptRole::Assistant, item["text"].as_str()?.to_owned()))
        }
        _ => None,
    }
}

fn credential_name(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    ["API_KEY", "APIKEY", "ACCESS_KEY", "TOKEN", "PASSWORD", "PASSWD", "SECRET", "PRIVATE_KEY", "CREDENTIAL"].iter().any(|part| name.contains(part))
}

static CREDENTIALS: LazyLock<Regex> = LazyLock::new(|| Regex::new(concat!(
    r#"(?is)-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?(?:-----END [A-Z0-9 ]*PRIVATE KEY-----|\z)"#,
    r#"|\b(?:bearer|basic)\s+[a-z0-9_./+=-]{8,}"#,
    r#"|\b(?:sk-[a-z0-9_-]{12,}|(?:gh[oprsu]_|github_pat_|xox[baprs]-)[a-z0-9_-]{12,}|(?:AKIA|ASIA)[A-Z0-9]{16}|eyJ[a-z0-9_-]+\.[a-z0-9_-]+\.[a-z0-9_-]+)"#,
    r#"|\b(?:[a-z0-9_-]*(?:api[_-]?key|access[_-]?key|token|password|passwd|secret|credential))["']?\s*[:=]\s*(?:"[^"]*"|'[^']*'|[^\s'",;\]}]+)"#,
    r#"|\b[a-z][a-z0-9+.-]*://[^\s/@]+:[^\s/@]+@"#,
)).expect("valid transcript credential patterns"));

fn safe_text(value: &str, secrets: &[String], limit: usize) -> String {
    let mut value = value.to_owned();
    for secret in secrets {
        if value.contains(secret) { value = value.replace(secret, "[redacted]"); }
    }
    let mut value = CREDENTIALS.replace_all(&value, "[redacted]").into_owned();
    value.retain(|character| !character.is_control() || matches!(character, '\r' | '\n' | '\t'));
    if value.len() > limit {
        value.truncate(floor_char_boundary(&value, limit - TRUNCATED.len()));
        value.push_str(TRUNCATED);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_visible_plain_text_is_projected_and_credentials_are_redacted() {
        let input = json!({"id":"one","type":"userMessage","content":[{"type":"text","text":"ask secret-value"},{"type":"image","url":"never"}]});
        let (role, text) = visible_item(&input).unwrap();
        assert_eq!(role, CloudTranscriptRole::User);
        assert_eq!(safe_text(&text, &["secret-value".into()], 100), "ask [redacted]");
        assert_eq!(safe_text("Bearer abcdefghijk and API_KEY=\"literal-key\"", &[], 100), "[redacted] and [redacted]");
        assert_eq!(safe_text("-----BEGIN PRIVATE KEY-----\npartial private data", &[], 100), "[redacted]");
        assert!(safe_text(&"雪".repeat(4000), &[], TRANSCRIPT_TEXT_BYTES).len() <= TRANSCRIPT_TEXT_BYTES);
        for item in [
            json!({"type":"reasoning","text":"never"}),
            json!({"type":"commandExecution","aggregatedOutput":"never"}),
            json!({"type":"agentMessage","phase":"commentary","text":"never"}),
            json!({"type":"agentMessage","phase":"final_answer","text":"never","moderation":{"display":{"mode":"hide"}}}),
        ] {
            assert!(visible_item(&item).is_none());
        }
        assert_eq!(visible_item(&json!({"type":"agentMessage","phase":"final_answer","text":"done"})), Some((CloudTranscriptRole::Assistant, "done".into())));
    }
}
