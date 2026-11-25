//! Typed daemon-backed TUI data source; preserves the last snapshot on transport errors.
use crate::paths;
use crate::service::queries::{self, AgentFilter, SnapshotOptions};
use crate::service::{InvocationContext, Request, Response, ServiceError, client::Client};
use crate::tui::app::DataState;
use crate::tui::data::DataSource;
use crate::tui::model::{Agent, Event, Message};

pub struct DbDataSource {
    client: Option<Client>,
    cached: Option<DataState>,
    last_error: Option<String>,
    timeline_limit: usize,
    filter: AgentFilter,
    filter_label: Option<String>,
}
impl Default for DbDataSource {
    fn default() -> Self {
        Self::new()
    }
}
impl DbDataSource {
    pub fn new() -> Self {
        Self::with_filter(AgentFilter::default())
    }
    pub(crate) fn with_filter(filter: AgentFilter) -> Self {
        let timeline_limit = std::env::var("AGENTX_TUI_TIMELINE_LIMIT").ok().and_then(|v| v.parse().ok()).filter(|v| *v > 0).unwrap_or(200);
        let filter_label = (!filter.is_unfiltered()).then(|| {
            let labels = if filter.labels.is_empty() { "" } else { " · labels" };
            format!("{}{}", filter.workspace.display(), labels)
        });
        Self { client: None, cached: None, last_error: None, timeline_limit, filter, filter_label }
    }
    fn query(&mut self, request: queries::Request) -> Option<queries::Response> {
        let result = (|| {
            if self.client.is_none() {
                let invocation = InvocationContext::capture()?;
                self.client = Some(Client::connect(paths::agentx_dir(), invocation)?);
            }
            let client = self.client.as_ref().unwrap();
            match client.call(Request::Queries(request))? {
                Response::Queries(response) => Ok(response),
                _ => Err(ServiceError::Internal("unexpected snapshot response".into())),
            }
        })();
        self.accept_query_result(result)
    }
    fn accept_query_result(&mut self, result: crate::service::ServiceResult<queries::Response>) -> Option<queries::Response> {
        match result {
            Ok(response) => {
                self.last_error = None;
                Some(response)
            }
            Err(error) => {
                // Observation is read-only; next poll may explicitly reconnect to
                // a new generation. Never retry a mutation or open SQLite here.
                self.client = None;
                self.last_error = Some(error.to_string());
                None
            }
        }
    }
}
impl DataSource for DbDataSource {
    fn agent_filter_label(&self) -> Option<&str> {
        self.filter_label.as_deref()
    }
    fn set_timeline_limit(&mut self, limit: usize) {
        self.timeline_limit = limit;
    }
    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }
    fn load_all_stopped(&mut self) -> Vec<Agent> {
        if !self.filter.labels.is_empty() {
            return Vec::new();
        }
        let request = if self.filter.is_unfiltered() { queries::Request::Stopped } else { queries::Request::FilteredStopped(self.filter.workspace.clone()) };
        match self.query(request) {
            Some(queries::Response::Stopped(agents)) => agents,
            _ => vec![],
        }
    }
    fn load_if_changed(&mut self) -> Option<DataState> {
        let (relay_configured, relay_enabled) = read_relay_config_flags();
        let options = SnapshotOptions { timeline_limit: self.timeline_limit, relay_configured, relay_enabled };
        let request = if self.filter.is_unfiltered() { queries::Request::Snapshot(options) } else { queries::Request::FilteredSnapshot { options, filter: self.filter.clone() } };
        match self.query(request) {
            Some(queries::Response::Snapshot(data)) => {
                self.cached = Some(data.clone());
                Some(data)
            }
            _ => None,
        }
    }
    fn load(&mut self) -> DataState {
        self.load_if_changed().or_else(|| self.cached.clone()).unwrap_or_else(DataState::empty)
    }
    fn search_timeline(&mut self, query: &str, limit: usize) -> (Vec<Message>, Vec<Event>) {
        match self.query(queries::Request::SearchTimeline { query: query.into(), limit }) {
            Some(queries::Response::Timeline { messages, events }) => (messages, events),
            _ => (vec![], vec![]),
        }
    }
}

fn read_relay_config_flags() -> (bool, bool) {
    let Some(table) = read_config_toml() else {
        return (false, false);
    };
    let Some(relay) = table.get("relay").and_then(|v| v.as_table()) else {
        return (false, false);
    };
    let Some(id) = relay.get("id").and_then(|v| v.as_str()) else {
        return (false, false);
    };
    if id.is_empty() {
        return (false, false);
    }
    let enabled = relay.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
    (true, enabled)
}

// ── Launch defaults from config.toml ─────────────────────────────

pub struct LaunchDefaults {
    pub terminal: String,
}

impl Default for LaunchDefaults {
    fn default() -> Self {
        Self { terminal: "default".into() }
    }
}

fn read_config_toml() -> Option<toml::Table> {
    let content = std::fs::read_to_string(paths::config_toml_path()).ok()?;
    content.parse().ok()
}

pub fn read_launch_defaults() -> LaunchDefaults {
    let table = match read_config_toml() {
        Some(t) => t,
        None => return LaunchDefaults::default(),
    };

    let terminal = table.get("terminal").and_then(|v| v.as_table()).and_then(|t| t.get("active")).and_then(|v| v.as_str()).unwrap_or("default").to_string();

    LaunchDefaults { terminal }
}

// ── Dynamic terminal preset detection ────────────────────────────

/// Get terminal presets available on this system.
/// Always starts with "default", then lists detected presets.
/// Also includes user-defined presets from config.toml.
pub fn get_available_presets() -> Vec<String> {
    crate::runtime::terminal::get_available_presets().into_iter().filter(|(name, available)| *available && name != "custom").map(|(name, _)| name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transport_failure_retains_last_snapshot_and_reports_error() {
        let mut source = DbDataSource::new();
        let mut cached = DataState::empty();
        cached.relay_enabled = true;
        source.cached = Some(cached);
        assert!(source.accept_query_result(Err(ServiceError::Unavailable("fixture endpoint stopped".into()))).is_none());
        assert!(source.cached.as_ref().unwrap().relay_enabled);
        assert!(source.last_error().unwrap().contains("fixture endpoint stopped"));
        assert!(source.client.is_none());
        let response = queries::Response::Stopped(vec![]);
        assert!(source.accept_query_result(Ok(response)).is_some());
        assert!(source.last_error().is_none());
    }
}
