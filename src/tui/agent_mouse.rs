//! Agent pane interactions. Rendered targets carry identities rather than
//! cursor indices, which may change between painting and receiving a click.

use crossterm::event::{KeyCode, KeyModifiers};

use super::app::App;
use super::model::{Agent, InputMode, OrphanProcess, ViewMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Section {
    Remote,
    Stopped,
    Orphans,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ItemKey {
    Agent { name: String, device: Option<String>, created_at: u64 },
    Orphan { pid: u32, launched_at: u64 },
    Header(Section),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum AgentTarget {
    Focus(ItemKey),
    Select(ItemKey),
    ScrollTo(usize),
}

/// The adjacent viewport starts from the most recent paint. Scrolling changes
/// only the viewport; the keyboard cursor and the active filter stay put.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct AgentWindow {
    pub prev: Option<usize>,
    pub next: Option<usize>,
}

#[derive(Clone, Copy)]
pub(super) enum Item<'a> {
    Live(&'a Agent),
    Remote(&'a Agent),
    Stopped(&'a Agent),
    Orphan(&'a OrphanProcess),
    Header(Section),
}

impl Item<'_> {
    pub(super) fn key(self) -> ItemKey {
        match self {
            Self::Live(agent) | Self::Remote(agent) | Self::Stopped(agent) => ItemKey::Agent { name: agent.name.clone(), device: agent.device_name.clone(), created_at: agent.created_at.to_bits() },
            Self::Orphan(orphan) => ItemKey::Orphan { pid: orphan.pid, launched_at: orphan.launched_at.to_bits() },
            Self::Header(section) => ItemKey::Header(section),
        }
    }

    pub(super) fn matches(self, key: &ItemKey) -> bool {
        match (self, key) {
            (Self::Live(agent) | Self::Remote(agent) | Self::Stopped(agent), ItemKey::Agent { name, device, created_at }) => &agent.name == name && &agent.device_name == device && agent.created_at.to_bits() == *created_at,
            (Self::Orphan(orphan), ItemKey::Orphan { pid, launched_at }) => orphan.pid == *pid && orphan.launched_at.to_bits() == *launched_at,
            (Self::Header(section), ItemKey::Header(expected)) => section == *expected,
            _ => false,
        }
    }
}

fn section<'a, T>(header: Section, visible: bool, expanded: bool, rows: &'a [T], item: impl Fn(&'a T) -> Item<'a>) -> impl Iterator<Item = Item<'a>> {
    (visible && !rows.is_empty()).then_some(Item::Header(header)).into_iter().chain(rows.iter().take(if visible && expanded { rows.len() } else { 0 }).map(item))
}

/// Logical items in the same order as the keyboard cursor. Detail lines and
/// overflow controls are view decorations and do not consume cursor indices.
pub(super) fn items(app: &App) -> impl Iterator<Item = Item<'_>> {
    app.data
        .agents
        .iter()
        .map(Item::Live)
        .chain(section(Section::Remote, true, app.ui.remote_expanded, &app.data.remote_agents, Item::Remote))
        .chain(section(Section::Stopped, app.ui.view_mode != ViewMode::Inline, app.ui.stopped_expanded, &app.data.stopped_agents, Item::Stopped))
        .chain(section(Section::Orphans, true, app.ui.orphans_expanded, &app.data.orphans, Item::Orphan))
}

pub fn activate(app: &mut App, target: AgentTarget) {
    let (key, select) = match target {
        AgentTarget::Focus(key) => (key, false),
        AgentTarget::Select(key) => (key, true),
        AgentTarget::ScrollTo(start) => {
            app.mouse.agent_scroll.set(Some(start));
            return;
        }
    };

    let enter = select || matches!(key, ItemKey::Header(_));
    if enter && (app.ui.mode != InputMode::Navigate || app.ui.overlay.is_some() || app.ui.confirm.is_some() || app.ui.help_open || app.ui.transcript.is_some()) {
        return;
    }
    let Some(index) = items(app).position(|item| item.matches(&key)) else {
        return;
    };
    app.ui.cursor = index;
    app.mouse.agent_scroll.set(None);
    if enter {
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    }
}

pub fn scroll(app: &mut App, down: bool) {
    let window = app.mouse.agent_window.get();
    if let Some(start) = if down { window.next } else { window.prev } {
        activate(app, AgentTarget::ScrollTo(start));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::model::CursorTarget;
    use crate::tui::test_helpers::{make_test_agent, offline_app};

    #[test]
    fn focus_re_resolves_reordered_agents_and_rejects_replacements() {
        crate::config::Config::init();
        let mut app = offline_app();
        app.data.agents = vec![make_test_agent("nova", 0.0), make_test_agent("luna", 1.0)];
        let target = AgentTarget::Focus(Item::Live(&app.data.agents[0]).key());

        app.data.agents.swap(0, 1);
        activate(&mut app, target.clone());
        assert_eq!(app.cursor_target(), CursorTarget::Agent(1));
        assert!(app.ui.selected.is_empty());

        app.ui.cursor = 0;
        app.data.agents[1].created_at += 1.0;
        activate(&mut app, target);
        assert_eq!(app.cursor_target(), CursorTarget::Agent(0));
    }

    #[test]
    fn header_and_remote_marker_share_existing_selection_and_replay() {
        crate::config::Config::init();
        let mut app = offline_app();
        let mut remote = make_test_agent("nova", 1.0);
        remote.device_name = Some("box".into());
        app.data.agents.push(make_test_agent("nova", 0.0));
        app.data.stopped_agents.push(make_test_agent("luna", 2.0));
        app.data.remote_agents.push(remote);
        app.ui.view_mode = ViewMode::Inline;

        let target = AgentTarget::Select(Item::Remote(&app.data.remote_agents[0]).key());
        activate(&mut app, target.clone());
        assert_eq!(app.cursor_target(), CursorTarget::Agent(0));
        assert!(app.ui.selected.is_empty());

        activate(&mut app, AgentTarget::Focus(ItemKey::Header(Section::Remote)));
        assert!(app.ui.remote_expanded);
        assert_eq!(app.cursor_target(), CursorTarget::RemoteHeader);

        activate(&mut app, target.clone());
        assert_eq!(app.cursor_target(), CursorTarget::RemoteAgent(0));
        assert!(app.ui.selected.contains("nova:box"));
        assert!(!app.ui.selected.contains("nova"));
        assert_eq!(app.ui.eject_filter.as_ref(), Some(&app.ui.selected));

        activate(&mut app, target);
        assert!(app.ui.selected.is_empty());
        assert!(app.ui.eject_filter.is_none());
    }
}
