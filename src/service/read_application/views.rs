use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::management::contract::{Request as Management, Response};
use crate::service::ServiceResult;

const TTL: Duration = Duration::from_secs(8);
const MAX_ENTRIES: usize = 12;
const MAX_VIEW_BYTES: usize = 256 * 1024;

#[derive(Hash, PartialEq, Eq)]
pub(super) enum Key {
    List(String),
    Agent(String),
    Mailbox(String),
    Activity(String),
}

impl Key {
    pub(super) fn for_request(generation: &str, request: &Management) -> Option<Self> {
        let suffix = |value: &str| format!("{generation}:{value}");
        match request {
            Management::List { .. } | Management::SelectLabels { .. } => Some(Self::List(suffix(&serde_json::to_string(request).ok()?))),
            Management::Agent(_) => Some(Self::Agent(suffix(&serde_json::to_string(request).ok()?))),
            Management::HumanMailbox { before, pending, limit } => Some(Self::Mailbox(suffix(&format!("{pending}:{limit}:{before:?}")))),
            Management::OfficeActivity { after, limit } => Some(Self::Activity(suffix(&format!("{limit}:{after:?}")))),
            _ => None,
        }
    }
}

#[derive(Default)]
pub(super) struct ViewCache {
    version: Option<i64>,
    entries: HashMap<Key, (Instant, Response)>,
}

impl ViewCache {
    pub(super) fn read(&mut self, version: Option<i64>, key: Key, load: impl FnOnce() -> ServiceResult<Response>) -> ServiceResult<Response> {
        if self.version != version {
            self.entries.clear();
            self.version = version;
        }
        if version.is_none() {
            return load();
        }
        if let Some((at, response)) = self.entries.get(&key)
            && at.elapsed() < TTL
        {
            return Ok(response.clone());
        }
        let response = load()?;
        if crate::observability::runtime::encoded_size(&response, MAX_VIEW_BYTES).is_some() {
            self.entries.remove(&key);
            if self.entries.len() >= MAX_ENTRIES {
                self.entries.retain(|_, (at, _)| at.elapsed() < TTL);
            }
            if self.entries.len() < MAX_ENTRIES {
                self.entries.insert(key, (Instant::now(), response.clone()));
            }
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::AgentxDb;
    use crate::management::contract::HumanMailbox;

    #[test]
    fn management_view_cache_skips_repeat_load_and_observes_commit() {
        let dir = tempfile::tempdir().unwrap();
        let writer = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let reader = AgentxDb::open_reader(writer.path()).unwrap();
        let version = || reader.conn().pragma_query_value(None, "data_version", |row| row.get(0)).ok();
        let key = || Key::Mailbox("scope:page".into());
        let load = || {
            let unhandled_count = reader.kv_get("fixture:count").unwrap().map_or(0, |count| count.parse().unwrap());
            Ok(Response::HumanMailbox(HumanMailbox { generation: "scope".into(), messages: vec![], next_before: None, unhandled_count }))
        };
        let mut cache = ViewCache::default();
        assert!(matches!(cache.read(version(), key(), load).unwrap(), Response::HumanMailbox(page) if page.unhandled_count == 0));
        cache.read(version(), key(), || panic!("unchanged query must not load twice")).unwrap();
        writer.kv_set("fixture:count", Some("1")).unwrap();
        assert!(matches!(cache.read(version(), key(), load).unwrap(), Response::HumanMailbox(page) if page.unhandled_count == 1));
        cache.entries.get_mut(&key()).unwrap().0 = Instant::now() - TTL;
        let mut refreshed = false;
        cache
            .read(version(), key(), || {
                refreshed = true;
                load()
            })
            .unwrap();
        assert!(refreshed, "time-dependent views need a finite lifetime");
    }
}
