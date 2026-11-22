//! Semantic mouse actions for native controls. The renderer owns all cell geometry.
use crossterm::event::{KeyCode, KeyModifiers};

use super::{
    app::App,
    model::{InputMode, LaunchField, OverlayKind, RELAY_ACTIONS},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverflowSlot {
    Input(usize),
    Footer(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Field {
    Compose,
    LaunchPrompt,
    Search,
    Command,
    RelayToken,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShortcutScope {
    Navigate,
    Compose,
    Launch,
    Search,
    Command,
    Reader,
    Relay,
    Output,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LaunchAction {
    Focus,
    Previous,
    Next,
    Toggle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlTarget {
    Shortcut(ShortcutScope, KeyCode, KeyModifiers),
    OpenReader,
    CloseHelp,
    HelpScroll { max: u16 },
    ConfirmChoice(bool),
    CancelConfirm,
    LaunchField(LaunchField, LaunchAction),
    OpenLaunch(LaunchField),
    LaunchChoice(usize),
    SubmitLaunch,
    RelayRow(u8),
    RelayScroll,
    PaletteRow(String),
    PaletteScroll,
    Text(Field, usize),
    More(OverflowSlot),
    CloseMore,
    MoreScroll { max: usize },
    MoreItem(Box<ControlTarget>, usize),
}

impl ShortcutScope {
    pub(super) fn active(self, app: &App) -> bool {
        if app.ui.transcript.is_some() {
            return self == Self::Reader;
        }
        if app.ui.confirm.is_some() {
            return false;
        }
        if app.ui.help_open {
            return false;
        }
        let active = match app.ui.mode {
            InputMode::Navigate => match app.ui.overlay.as_ref().map(|overlay| overlay.kind) {
                None => Self::Navigate,
                Some(OverlayKind::Search) => Self::Search,
                Some(OverlayKind::Command) => Self::Command,
            },
            InputMode::Compose => Self::Compose,
            InputMode::Launch => Self::Launch,
            InputMode::Relay => Self::Relay,
            InputMode::CommandOutput => Self::Output,
        };
        self == active
    }
}

pub fn activate(app: &mut App, target: ControlTarget) {
    use ControlTarget::*;
    if !matches!(target, More(_) | MoreScroll { .. } | HelpScroll { .. } | PaletteScroll | RelayScroll) {
        app.control_overflow = None;
    }

    match target {
        Shortcut(scope, key, modifiers) if scope.active(app) => app.handle_key(key, modifiers),
        OpenReader if ShortcutScope::Compose.active(app) => app.handle_key(KeyCode::Char('t'), KeyModifiers::CONTROL),
        CloseHelp if app.ui.help_open => app.handle_key(KeyCode::Esc, KeyModifiers::NONE),
        CancelConfirm if app.ui.confirm.is_some() => app.handle_key(KeyCode::Esc, KeyModifiers::NONE),
        ConfirmChoice(yes) => {
            let Some(confirm) = app.ui.confirm.as_mut() else { return };
            if confirm.is_expired() {
                app.ui.confirm = None;
            } else {
                confirm.selected = yes;
                let key = if confirm.is_inline_agent_action() && !yes { KeyCode::Esc } else { KeyCode::Enter };
                app.handle_key(key, KeyModifiers::NONE);
            }
        }
        LaunchField(field, action) if ShortcutScope::Launch.active(app) => {
            let launch = &mut app.ui.launch;
            let old_focus = launch.options_cursor.replace(field);
            match action {
                LaunchAction::Focus => {}
                LaunchAction::Previous => launch.adjust_left(),
                LaunchAction::Next => launch.adjust_right(),
                LaunchAction::Toggle => launch.toggle_or_select(),
            }
            if action != LaunchAction::Focus {
                launch.options_cursor = old_focus;
            }
        }
        OpenLaunch(field) if ShortcutScope::Launch.active(app) => app.ui.launch.open_field(field),
        LaunchChoice(index) if ShortcutScope::Launch.active(app) => app.ui.launch.choose(index),
        SubmitLaunch if ShortcutScope::Launch.active(app) => {
            app.ui.launch.quick.picker = None;
            app.ui.launch.options_cursor = None;
            app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        }
        RelayRow(row) if ShortcutScope::Relay.active(app) => {
            if let Some(relay) = app.ui.relay_popup.as_mut().filter(|relay| !relay.editing_token)
                && row as usize <= RELAY_ACTIONS.len()
            {
                relay.cursor = row;
                app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
            }
        }
        PaletteRow(command) if ShortcutScope::Command.active(app) => {
            if let Some(palette) = app.ui.overlay.as_mut().and_then(|overlay| overlay.palette.as_mut()) {
                palette.cursor = palette.filtered.iter().position(|&index| palette.all[index].command == command);
            }
        }
        Text(field, offset) => position_cursor(app, field, offset),
        More(slot) => app.control_overflow = Some(slot),
        MoreItem(target, _) => activate(app, *target),
        _ => {}
    }
}

fn position_cursor(app: &mut App, field: Field, offset: usize) {
    match field {
        Field::Compose if app.ui.mode == InputMode::Compose && app.ui.input.is_char_boundary(offset) => {
            app.ui.transcript = None;
            app.ui.input_cursor = offset;
        }
        Field::LaunchPrompt if ShortcutScope::Launch.active(app) && app.ui.input.is_char_boundary(offset) => {
            app.ui.launch.quick.picker = None;
            app.ui.launch.options_cursor = None;
            app.ui.input_cursor = offset;
        }
        Field::Search if ShortcutScope::Search.active(app) => {
            if let Some(overlay) = app.ui.overlay.as_mut().filter(|overlay| overlay.input.is_char_boundary(offset)) {
                overlay.cursor = offset;
            }
        }
        Field::Command if ShortcutScope::Command.active(app) => {
            if let Some(overlay) = app.ui.overlay.as_mut().filter(|overlay| overlay.input.is_char_boundary(offset)) {
                overlay.cursor = offset;
            }
        }
        Field::RelayToken if ShortcutScope::Relay.active(app) => {
            if let Some(relay) = app.ui.relay_popup.as_mut().filter(|relay| relay.editing_token && relay.token_input.is_char_boundary(offset)) {
                relay.token_cursor = offset;
            }
        }
        _ => {}
    }
}

pub fn scroll(app: &mut App, target: ControlTarget, down: bool) {
    use ControlTarget::*;
    let key = if down { KeyCode::Down } else { KeyCode::Up };
    match target {
        HelpScroll { max } if app.ui.help_open => {
            let current = app.ui.help_scroll.min(max);
            app.ui.help_scroll = if down { current.saturating_add(3) } else { current.saturating_sub(3) };
            app.ui.help_scroll = app.ui.help_scroll.min(max);
        }
        LaunchChoice(_) if ShortcutScope::Launch.active(app) => app.handle_key(key, KeyModifiers::NONE),
        PaletteScroll | PaletteRow(_) if ShortcutScope::Command.active(app) => app.handle_key(key, KeyModifiers::NONE),
        RelayScroll | RelayRow(_) if ShortcutScope::Relay.active(app) => app.handle_key(key, KeyModifiers::NONE),
        MoreScroll { max } | MoreItem(_, max) => {
            app.control_overflow = app.control_overflow.map(|slot| {
                let step = |offset: usize| if down { offset.saturating_add(1).min(max) } else { offset.saturating_sub(1) };
                match slot {
                    OverflowSlot::Input(offset) => OverflowSlot::Input(step(offset)),
                    OverflowSlot::Footer(offset) => OverflowSlot::Footer(step(offset)),
                }
            });
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{DEFAULT_INLINE_HEIGHT, model::ViewMode, mouse::Target, render, test_helpers::offline_app};
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::{
        Terminal, TerminalOptions, Viewport,
        backend::TestBackend,
        layout::{Position, Rect},
    };

    fn screen(width: u16, height: u16) -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(width, height)).unwrap()
    }

    fn draw(terminal: &mut Terminal<TestBackend>, app: &mut App) {
        terminal.draw(|frame| render::render(frame, app, DEFAULT_INLINE_HEIGHT)).unwrap();
    }

    fn target_at(app: &App, predicate: impl Fn(&ControlTarget) -> bool) -> Position {
        let (rect, _) = app.mouse.regions().into_iter().find(|(_, target)| matches!(target, Target::Control(control) if predicate(control))).expect("visible control");
        Position::new(rect.x, rect.y)
    }

    fn click(app: &mut App, at: Position) {
        mouse(app, MouseEventKind::Down(MouseButton::Left), at);
        assert!(mouse(app, MouseEventKind::Up(MouseButton::Left), at), "click at {at:?} did not activate a target");
    }

    fn mouse(app: &mut App, kind: MouseEventKind, at: Position) -> bool {
        app.handle_mouse(MouseEvent { kind, column: at.x, row: at.y, modifiers: KeyModifiers::NONE })
    }

    fn click_target(app: &mut App, predicate: impl Fn(&ControlTarget) -> bool) {
        click(app, target_at(app, predicate));
    }

    #[test]
    fn controls_launch_adjusts_visible_arrows_on_compact_terminal() {
        let mut app = offline_app();
        let mut terminal = screen(16, 14);
        draw(&mut terminal, &mut app);
        click_target(&mut app, |target| matches!(target, ControlTarget::More(OverflowSlot::Footer(_))));
        draw(&mut terminal, &mut app);
        click_target(&mut app, |target| matches!(target, ControlTarget::MoreItem(item, _) if matches!(**item, ControlTarget::Shortcut(ShortcutScope::Navigate, KeyCode::Tab, _))));
        assert_eq!(app.ui.mode, InputMode::Launch);

        draw(&mut terminal, &mut app);
        click_target(&mut app, |target| matches!(target, ControlTarget::OpenLaunch(LaunchField::More)));
        draw(&mut terminal, &mut app);
        let next = target_at(&app, |target| matches!(target, ControlTarget::LaunchField(LaunchField::Count, LaunchAction::Next)));
        assert_eq!(terminal.backend().buffer()[next].symbol(), "▸");
        mouse(&mut app, MouseEventKind::ScrollDown, next);
        assert_eq!(app.ui.launch.count, 1);
        click(&mut app, next);
        assert_eq!(app.ui.launch.count, 2);
        assert_eq!(app.ui.launch.options_cursor, Some(LaunchField::Tool));
        draw(&mut terminal, &mut app);
        click_target(&mut app, |target| matches!(target, ControlTarget::LaunchField(LaunchField::Headless, LaunchAction::Toggle)));
        assert!(app.ui.launch.headless);
    }

    #[test]
    fn controls_palette_uses_explicit_run_and_masks_under_confirm() {
        let mut app = offline_app();
        app.ui.view_mode = ViewMode::Vertical;
        let mut terminal = screen(68, 22);
        draw(&mut terminal, &mut app);
        click_target(&mut app, |target| matches!(target, ControlTarget::Shortcut(ShortcutScope::Navigate, KeyCode::Char('!'), _)));
        app.handle_paste("reset");
        draw(&mut terminal, &mut app);
        click_target(&mut app, |target| matches!(target, ControlTarget::PaletteRow(command) if command == "reset"));
        assert!(app.ui.confirm.is_none());
        click_target(&mut app, |target| matches!(target, ControlTarget::Shortcut(ShortcutScope::Command, KeyCode::Enter, _)));
        assert!(app.ui.confirm.is_some());
        draw(&mut terminal, &mut app);
        assert!(app.mouse.regions().iter().all(|(_, target)| matches!(target, Target::Control(ControlTarget::ConfirmChoice(_) | ControlTarget::CancelConfirm))));
        click_target(&mut app, |target| *target == ControlTarget::ConfirmChoice(false));
        assert!(app.ui.confirm.is_none());
    }

    #[test]
    fn controls_compose_hits_full_wide_grapheme_at_real_inline_origin() {
        let mut app = offline_app();
        let mut terminal = Terminal::with_options(TestBackend::new(40, 20), TerminalOptions { viewport: Viewport::Fixed(Rect::new(5, 4, 24, 13)) }).unwrap();
        app.handle_paste(&format!("{}漢e\u{301}{}", "a".repeat(20), "b".repeat(16)));
        draw(&mut terminal, &mut app);
        let wide = target_at(&app, |target| *target == ControlTarget::Text(Field::Compose, 20));
        assert_eq!(wide.x, 5);
        assert!(wide.y > 4);
        assert_eq!(terminal.backend().buffer()[wide].symbol(), "漢");
        click(&mut app, Position::new(wide.x + 1, wide.y));
        assert_eq!(app.ui.input_cursor, 20);
        draw(&mut terminal, &mut app);
        let combining = target_at(&app, |target| *target == ControlTarget::Text(Field::Compose, 23));
        assert_eq!(terminal.backend().buffer()[combining].symbol(), "e\u{301}");
        click(&mut app, combining);
        assert_eq!(app.ui.input_cursor, 23);
    }

    #[test]
    fn controls_single_line_scrolled_prompt_hits_visible_tail() {
        let mut app = offline_app();
        let mut terminal = screen(24, 15);
        app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
        app.handle_paste("invisible beginning · visible end!");
        draw(&mut terminal, &mut app);
        let offset = app.ui.input.len() - 1;
        let pos = target_at(&app, |target| *target == ControlTarget::Text(Field::LaunchPrompt, offset));
        assert_eq!(terminal.backend().buffer()[pos].symbol(), "!");
        click(&mut app, pos);
        assert_eq!(app.ui.input_cursor, offset);
        draw(&mut terminal, &mut app);
        assert_eq!(terminal.get_cursor_position().unwrap(), pos);
    }

    #[test]
    fn controls_more_scrolls_even_over_action_rows_and_masks_base() {
        let mut app = offline_app();
        let mut terminal = screen(11, 4);
        draw(&mut terminal, &mut app);
        click_target(&mut app, |target| matches!(target, ControlTarget::More(OverflowSlot::Footer(0))));
        draw(&mut terminal, &mut app);
        assert!(app.mouse.regions().iter().all(|(_, target)| matches!(target, Target::Control(ControlTarget::MoreItem(..) | ControlTarget::MoreScroll { .. } | ControlTarget::CloseMore))));
        let item = target_at(&app, |target| matches!(target, ControlTarget::MoreItem(..)));
        mouse(&mut app, MouseEventKind::ScrollDown, item);
        assert_eq!(app.control_overflow, Some(OverflowSlot::Footer(1)));
        draw(&mut terminal, &mut app);
        click_target(&mut app, |target| matches!(target, ControlTarget::MoreItem(item, _) if matches!(**item, ControlTarget::Shortcut(ShortcutScope::Navigate, KeyCode::Char('?'), _))));
        assert!(app.ui.help_open);
        draw(&mut terminal, &mut app);
        click_target(&mut app, |target| *target == ControlTarget::CloseHelp);
        assert!(!app.ui.help_open);
    }

    #[test]
    fn controls_relay_token_clicks_move_caret_without_rendering_secret() {
        let mut app = offline_app();
        let mut terminal = screen(60, 18);
        draw(&mut terminal, &mut app);
        click_target(&mut app, |target| matches!(target, ControlTarget::Shortcut(ShortcutScope::Navigate, KeyCode::Char('r'), KeyModifiers::CONTROL)));
        draw(&mut terminal, &mut app);
        click_target(&mut app, |target| *target == ControlTarget::RelayRow(3));
        app.handle_paste("a🦀z");
        draw(&mut terminal, &mut app);
        let pos = target_at(&app, |target| *target == ControlTarget::Text(Field::RelayToken, 5));
        assert_eq!(terminal.backend().buffer()[pos].symbol(), "•");
        assert!(!terminal.backend().buffer().content.iter().any(|cell| cell.symbol().contains('🦀')));
        click(&mut app, pos);
        assert_eq!(app.ui.relay_popup.as_ref().unwrap().token_cursor, 5);
    }
}
