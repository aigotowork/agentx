//! Painted cell regions for the dashboard. Coordinates are absolute terminal cells,
//! including the real viewport origin when Ratatui is drawing inline.
use std::cell::{Cell, RefCell};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};

use super::{agent_mouse, app::App, control_mouse};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Target {
    Agent(agent_mouse::AgentTarget),
    AgentsArea,
    InfoOpen,
    InfoBack,
    InfoRow(usize),
    InfoPane,
    Control(control_mouse::ControlTarget),
    Reader,
    Messages,
}

#[derive(Default)]
pub(super) struct MouseState {
    regions: RefCell<Vec<(Rect, Target)>>,
    pressed: Option<(Target, Option<ActionContext>)>,
    pub agent_window: Cell<agent_mouse::AgentWindow>,
    pub agent_scroll: Cell<Option<usize>>,
}

// A footer action can retain its position while the participants it acts on change.
#[derive(PartialEq, Eq)]
struct ActionContext {
    focus: Option<agent_mouse::ItemKey>,
    selected: Vec<(String, u64)>,
    confirmation: Option<std::time::Instant>,
}

impl ActionContext {
    fn current(app: &App) -> Self {
        let identity = |agent: &super::model::Agent| (agent.action_name(), agent.created_at.to_bits());
        let focus = agent_mouse::items(app).nth(app.ui.cursor).map(|item| item.key());
        let mut selected: Vec<_> = app.data.agents.iter().chain(&app.data.remote_agents).chain(&app.data.stopped_agents).filter(|agent| app.ui.selected.contains(&agent.action_name())).map(identity).collect();
        selected.sort_unstable();
        Self { focus, selected, confirmation: app.ui.confirm.as_ref().map(|confirm| confirm.expires_at) }
    }
}

impl MouseState {
    pub(super) fn add(&self, area: Rect, target: Target) {
        if !area.is_empty() {
            self.regions.borrow_mut().push((area, target));
        }
    }

    pub(super) fn clear_targets(&self) {
        self.regions.borrow_mut().clear();
    }

    #[cfg(test)]
    pub(super) fn regions(&self) -> Vec<(Rect, Target)> {
        self.regions.borrow().clone()
    }

    fn hit(&self, position: Position) -> Option<Target> {
        self.regions.borrow().iter().rev().find(|(area, _)| area.contains(position)).map(|(_, target)| target.clone())
    }

    pub(super) fn cancel_press(&mut self) {
        self.pressed = None;
    }

    pub(super) fn invalidate(&mut self) {
        self.clear_targets();
        self.cancel_press();
    }
}

impl App {
    /// True only when the input could have changed the displayed state.
    pub(super) fn handle_mouse(&mut self, event: MouseEvent) -> bool {
        use MouseEventKind::*;
        if matches!(event.kind, Moved) {
            return false;
        }
        let target = self.mouse.hit(Position::new(event.column, event.row));
        match event.kind {
            Down(MouseButton::Left) => {
                self.mouse.pressed = target.map(|target| {
                    let context = matches!(target, Target::Control(_) | Target::InfoOpen).then(|| ActionContext::current(self));
                    (target, context)
                });
            }
            Drag(MouseButton::Left) => {
                if self.mouse.pressed.as_ref().map(|(pressed, _)| pressed) != target.as_ref() {
                    self.mouse.cancel_press();
                }
            }
            Up(MouseButton::Left) => {
                let Some((pressed, context)) = self.mouse.pressed.take() else { return false };
                if target.as_ref() != Some(&pressed) || context.is_some_and(|old| old != ActionContext::current(self)) {
                    return false;
                }
                match pressed {
                    Target::InfoOpen if self.ui.info.is_some() => self.handle_info_key(crossterm::event::KeyCode::Char('i')),
                    Target::InfoOpen => self.open_info(),
                    Target::InfoBack => self.info_back(),
                    Target::InfoRow(index) => self.info_activate(index),
                    Target::Agent(agent) => agent_mouse::activate(self, agent),
                    Target::Control(control) => control_mouse::activate(self, control),
                    _ => return false,
                }
                return true;
            }
            ScrollUp | ScrollDown | ScrollLeft | ScrollRight => {
                self.mouse.cancel_press();
                let down = matches!(event.kind, ScrollDown | ScrollRight);
                match target {
                    Some(Target::AgentsArea | Target::Agent(_)) => agent_mouse::scroll(self, down),
                    _ if matches!(event.kind, ScrollLeft | ScrollRight) => return false,
                    Some(Target::Control(control)) => control_mouse::scroll(self, control, down),
                    Some(Target::InfoRow(_) | Target::InfoPane | Target::InfoBack | Target::InfoOpen) => {
                        let Some(info) = self.ui.info.as_mut() else { return false };
                        info.pan(down, 3);
                    }
                    Some(Target::Reader) => {
                        let Some(reader) = self.ui.transcript.as_mut() else { return false };
                        if down { reader.scroll_down(3) } else { reader.scroll_up(3) }
                    }
                    Some(Target::Messages) => {
                        self.ui.msg_scroll = if down { self.ui.msg_scroll.saturating_sub(3) } else { self.ui.msg_scroll.saturating_add(3).min(self.ui.scroll_max) };
                    }
                    None => return false,
                }
                return true;
            }
            _ => self.mouse.cancel_press(),
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::test_helpers::{make_test_agent, offline_app};
    use crossterm::event::KeyModifiers;

    fn event(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent { kind, column, row, modifiers: KeyModifiers::NONE }
    }

    #[test]
    fn scroll_uses_the_visible_panel_and_absolute_inline_origin() {
        let mut app = offline_app();
        app.ui.scroll_max = 4;
        app.mouse.add(Rect::new(30, 12, 50, 8), Target::Messages);
        assert!(!app.handle_mouse(event(MouseEventKind::ScrollUp, 30, 11)));
        assert!(app.handle_mouse(event(MouseEventKind::ScrollUp, 30, 12)));
        assert_eq!(app.ui.msg_scroll, 3);
        app.handle_mouse(event(MouseEventKind::ScrollUp, 79, 19));
        assert_eq!(app.ui.msg_scroll, 4);
        assert!(!app.handle_mouse(event(MouseEventKind::ScrollDown, 80, 19)));
        app.mouse.clear_targets();
        assert!(!app.handle_mouse(event(MouseEventKind::ScrollDown, 30, 12)));
        assert_eq!(app.ui.msg_scroll, 4);
    }

    #[test]
    fn clearing_frame_geometry_and_canceling_an_inflight_press_are_distinct() {
        let mut app = offline_app();
        app.mouse.add(Rect::new(2, 3, 10, 1), Target::Messages);
        app.handle_mouse(event(MouseEventKind::Down(MouseButton::Left), 2, 3));
        app.mouse.clear_targets();
        assert!(app.mouse.pressed.is_some());
        app.mouse.invalidate();
        assert!(app.mouse.pressed.is_none());
    }

    #[test]
    fn data_refresh_cannot_retarget_a_pressed_action_to_another_incarnation() {
        let mut app = offline_app();
        app.data.agents = vec![make_test_agent("nova", 0.0)];
        let before = ActionContext::current(&app);
        app.data.agents[0].created_at += 1.0;
        assert!(before != ActionContext::current(&app));
    }
}
