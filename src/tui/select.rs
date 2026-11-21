//! Searchable CLI choices with optional multi-selection and terminal cleanup.

use anyhow::{Result, bail};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::{Frame, Terminal};
use std::collections::HashSet;
use std::io::{IsTerminal, stderr, stdin};

pub(crate) struct Choice {
    pub value: String,
    /// Optional display text; selection always returns the exact value.
    pub label: Option<String>,
    pub description: String,
}

/// Return the exact chosen value, or None when the user cancels.
pub(crate) fn select(title: &str, choices: &[Choice]) -> Result<Option<String>> {
    Ok(choose(title, choices, None)?.and_then(|indices| indices.first().map(|index| choices[*index].value.clone())))
}

/// Review every selected target before returning the exact values for an effect.
pub(crate) fn select_multiple(title: &str, choices: &[Choice], action: &str) -> Result<Option<Vec<String>>> {
    Ok(choose(title, choices, Some(action))?.map(|indices| indices.into_iter().map(|index| choices[index].value.clone()).collect()))
}

fn choose(title: &str, choices: &[Choice], action: Option<&str>) -> Result<Option<Vec<usize>>> {
    if !stdin().is_terminal() || !stderr().is_terminal() {
        bail!("Interactive selection requires a terminal; pass an explicit value");
    }
    if choices.is_empty() {
        bail!("No available choices for {title}");
    }
    enable_raw_mode()?;
    let _restore = RestoreTerminal;
    // Terminal::clear snapshots the cursor through Crossterm's stdout DSR.
    // A fresh fullscreen buffer needs only a clear on the actual stderr writer.
    crossterm::execute!(stderr(), EnterAlternateScreen, EnableMouseCapture, crossterm::terminal::Clear(crossterm::terminal::ClearType::All), crossterm::cursor::MoveTo(0, 0), crossterm::cursor::Hide)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stderr()))?;
    let mut state = Selection::new(choices.len(), action.is_some());
    loop {
        let mut hits = SelectorHits::default();
        terminal.draw(|frame| hits = render(frame, title, choices, action, &mut state))?;
        // Crossterm enables all-motion reporting; hover has no visual state to redraw.
        let event = loop {
            let event = event::read()?;
            if !matches!(event, Event::Mouse(MouseEvent { kind: MouseEventKind::Moved, .. })) {
                break event;
            }
        };
        match state.handle_event(event, choices, &hits) {
            Action::Continue => {}
            Action::Pick(indices) => return Ok(Some(indices)),
            Action::Cancel => return Ok(None),
        }
    }
}

struct RestoreTerminal;
impl Drop for RestoreTerminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(stderr(), DisableMouseCapture, crossterm::style::SetAttribute(crossterm::style::Attribute::Reset), crossterm::style::ResetColor, crossterm::cursor::Show, LeaveAlternateScreen);
    }
}

// The catalog is immutable for the whole picker lifetime. Indices therefore
// retain the original business value even when filtering changes visible rows.
struct Selection {
    query: String,
    matches: Vec<usize>,
    cursor: usize,
    offset: usize,
    selected: HashSet<usize>,
    multiple: bool,
    reviewing: bool,
    pressed: Option<(Rect, Hit)>,
}

#[derive(Debug, PartialEq)]
enum Action {
    Continue,
    Pick(Vec<usize>),
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hit {
    Row { position: usize, choice: usize },
    Submit,
    Edit,
    Cancel,
}

/// The most recently rendered viewport; a list cell resolves to the exact catalog entry.
#[derive(Default)]
struct SelectorHits {
    frame: Rect,
    list: Rect,
    offset: usize,
    rows: Vec<usize>,
    buttons: Vec<(Rect, Hit)>,
}

impl SelectorHits {
    fn at(&self, point: Position) -> Option<Hit> {
        if self.list.contains(point) {
            let row = usize::from(point.y - self.list.y);
            return self.rows.get(row).map(|&choice| Hit::Row { position: self.offset + row, choice });
        }
        self.buttons.iter().find_map(|(area, hit)| area.contains(point).then_some(*hit))
    }
}

impl Selection {
    fn new(count: usize, multiple: bool) -> Self {
        Self { query: String::new(), matches: (0..count).collect(), cursor: 0, offset: 0, selected: HashSet::new(), multiple, reviewing: false, pressed: None }
    }

    fn selected_indices(&self) -> Vec<usize> {
        let mut indices = self.selected.iter().copied().collect::<Vec<_>>();
        indices.sort_unstable();
        indices
    }

    fn reset_viewport(&mut self) {
        self.cursor = 0;
        self.offset = 0;
    }

    fn row_count(&self) -> usize {
        if self.reviewing { self.selected.len() } else { self.matches.len() }
    }

    fn submit(&mut self) -> Action {
        if self.reviewing {
            return Action::Pick(self.selected_indices());
        }
        if !self.multiple {
            return self.matches.get(self.cursor).map_or(Action::Continue, |&index| Action::Pick(vec![index]));
        }
        if !self.selected.is_empty() {
            self.reviewing = true;
            self.reset_viewport();
        }
        Action::Continue
    }

    fn edit(&mut self) {
        self.reviewing = false;
        self.reset_viewport();
    }

    fn toggle(&mut self, index: usize) {
        if !self.selected.remove(&index) {
            self.selected.insert(index);
        }
    }

    fn handle_event(&mut self, event: Event, choices: &[Choice], hits: &SelectorHits) -> Action {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                self.pressed = None;
                self.handle(key, choices)
            }
            Event::Mouse(mouse) => self.handle_mouse(mouse, hits),
            Event::Resize(..) | Event::FocusLost => {
                self.pressed = None;
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, hits: &SelectorHits) -> Action {
        let point = Position::new(mouse.column, mouse.row);
        let hit = hits.at(point).map(|hit| (hits.frame, hit));
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => self.pressed = hit,
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some((_, pressed)) = self.pressed.take().filter(|pressed| Some(*pressed) == hit) {
                    return self.activate(pressed);
                }
            }
            MouseEventKind::Drag(MouseButton::Left) if self.pressed != hit => self.pressed = None,
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                self.pressed = None;
                if hits.list.contains(point) {
                    self.scroll(hits.list.height, mouse.kind == MouseEventKind::ScrollDown);
                }
            }
            _ => {}
        }
        Action::Continue
    }

    fn activate(&mut self, hit: Hit) -> Action {
        match hit {
            Hit::Submit => return self.submit(),
            Hit::Edit => self.edit(),
            Hit::Cancel => return Action::Cancel,
            Hit::Row { position, choice } => {
                self.cursor = position;
                if !self.reviewing {
                    if !self.multiple {
                        return Action::Pick(vec![choice]);
                    }
                    self.toggle(choice);
                }
            }
        }
        Action::Continue
    }

    fn scroll(&mut self, height: u16, down: bool) {
        let count = self.row_count();
        let height = usize::from(height);
        if height == 0 || count <= height {
            return;
        }
        let max_offset = count - height;
        self.offset = if down { (self.offset + 3).min(max_offset) } else { self.offset.saturating_sub(3) };
        self.cursor = self.cursor.clamp(self.offset, self.offset + height - 1);
    }

    fn handle(&mut self, key: KeyEvent, choices: &[Choice]) -> Action {
        if key.kind == KeyEventKind::Release {
            return Action::Continue;
        }
        if key.code == KeyCode::Esc || (matches!(key.code, KeyCode::Char('c' | 'd')) && key.modifiers.contains(KeyModifiers::CONTROL)) {
            return Action::Cancel;
        }
        let last = self.row_count().saturating_sub(1);
        match key.code {
            KeyCode::Enter => return self.submit(),
            KeyCode::Up => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Down => self.cursor = (self.cursor + 1).min(last),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = last,
            KeyCode::PageUp => self.cursor = self.cursor.saturating_sub(8),
            KeyCode::PageDown => self.cursor = (self.cursor + 8).min(last),
            KeyCode::Backspace if self.reviewing => self.edit(),
            _ if self.reviewing => {}
            KeyCode::Char(' ') if self.multiple => {
                if let Some(&index) = self.matches.get(self.cursor) {
                    self.toggle(index);
                }
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.filter(choices);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.query.clear();
                self.filter(choices);
            }
            KeyCode::Char(character) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                self.query.push(character);
                self.filter(choices);
            }
            _ => {}
        }
        Action::Continue
    }

    fn filter(&mut self, choices: &[Choice]) {
        let query = self.query.to_lowercase();
        self.matches = choices.iter().enumerate().filter(|(_, choice)| format!("{} {} {}", choice.value, choice.label.as_deref().unwrap_or_default(), choice.description).to_lowercase().contains(&query)).map(|(index, _)| index).collect();
        self.reset_viewport();
    }
}

fn render(frame: &mut Frame, title: &str, choices: &[Choice], action: Option<&str>, state: &mut Selection) -> SelectorHits {
    let area = Rect { height: frame.area().height.min(18), ..frame.area() };
    let [heading, search, list, footer] = Layout::vertical([Constraint::Length(1), Constraint::Length(2), Constraint::Min(1), Constraint::Length(2)]).areas(area);
    let mut hits = SelectorHits { frame: frame.area(), list, ..SelectorHits::default() };
    let title = if state.reviewing { format!("Review {} targets — {}", state.selected.len(), action.unwrap_or("confirm")) } else { title.to_owned() };
    frame.render_widget(Paragraph::new(title).style(Style::default().add_modifier(Modifier::BOLD)), heading);
    let search_text = if state.reviewing { format!("{} selected targets", state.selected.len()) } else { format!("Search: {}", state.query) };
    frame.render_widget(Paragraph::new(search_text), search);
    let review;
    let visible = if state.reviewing {
        review = state.selected_indices();
        &review
    } else {
        &state.matches
    };
    if visible.is_empty() {
        frame.render_widget(Paragraph::new("No matches — edit the search or press Esc"), list);
    } else {
        let items = visible.iter().map(|index| {
            let choice = &choices[*index];
            let marker = if !state.multiple || state.reviewing {
                ""
            } else if state.selected.contains(index) {
                "[x] "
            } else {
                "[ ] "
            };
            let label = choice.label.as_deref().unwrap_or(&choice.value);
            ListItem::new(Line::from(vec![Span::raw(format!("{marker}{label:<24} ")), Span::styled(choice.description.clone(), Style::default().add_modifier(Modifier::DIM))]))
        });
        let widget = List::new(items).highlight_symbol("› ").highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        let height = usize::from(list.height);
        let offset = state.offset.min(visible.len().saturating_sub(height));
        let mut list_state = ListState::default().with_offset(offset).with_selected(Some(state.cursor));
        frame.render_stateful_widget(widget, list, &mut list_state);
        if !list.is_empty() {
            state.offset = list_state.offset();
            hits.offset = state.offset;
            hits.rows.extend(visible.iter().skip(state.offset).take(height).copied());
        }
    }
    render_footer(frame, footer, action, state, choices.len(), &mut hits);
    hits
}

/// Draw each button and register exactly its occupied cells; skip labels that cannot fit.
fn render_footer(frame: &mut Frame, area: Rect, action: Option<&str>, state: &Selection, count: usize, hits: &mut SelectorHits) {
    if area.is_empty() {
        return;
    }
    let (primary, primary_short, enabled) = if state.reviewing {
        let action = action.unwrap_or("confirm");
        (format!("Enter {action}"), action, true)
    } else if state.multiple {
        ("Enter review".into(), "Review", !state.selected.is_empty())
    } else {
        ("Enter select".into(), "Select", !state.matches.is_empty())
    };
    let mut buttons = vec![(primary.as_str(), primary_short, Hit::Submit, enabled)];
    if state.reviewing {
        buttons.push(("Backspace edit", "Edit", Hit::Edit, true));
    }
    buttons.push(("Esc cancel", "Cancel", Hit::Cancel, true));
    let full_width = buttons.iter().map(|(label, ..)| Span::raw(*label).width() + 2).sum::<usize>() + buttons.len() - 1;
    let mut x = area.x;
    for (label, short, hit, enabled) in buttons {
        let label = if full_width <= usize::from(area.width) { label } else { short };
        let style = if enabled { Style::default().add_modifier(Modifier::BOLD) } else { Style::default().add_modifier(Modifier::DIM) };
        let span = Span::styled(format!("[{label}]"), style);
        let Ok(width) = u16::try_from(span.width()) else { continue };
        if width > area.right().saturating_sub(x) {
            continue;
        }
        let button = Rect::new(x, area.y, width, 1);
        frame.render_widget(Paragraph::new(span), button);
        if enabled {
            hits.buttons.push((button, hit));
        }
        x = button.right().saturating_add(1);
    }
    if area.height > 1 {
        let help = if state.reviewing {
            if full_width > usize::from(area.width) { "Backspace edit · ↑↓ scroll".into() } else { "↑↓ scroll · Ctrl+C cancel".into() }
        } else if state.multiple {
            let hidden = state.selected.iter().filter(|index| !state.matches.contains(index)).count();
            format!("{} selected ({} hidden) · Space toggle · ↑↓ move · type to filter", state.selected.len(), hidden)
        } else {
            format!("{} / {count} choices · ↑↓ move · type to filter", state.matches.len())
        };
        frame.render_widget(Paragraph::new(help).style(Style::default().add_modifier(Modifier::DIM)), Rect::new(area.x, area.y + 1, area.width, 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn draw(terminal: &mut Terminal<TestBackend>, choices: &[Choice], state: &mut Selection) -> SelectorHits {
        let mut hits = SelectorHits::default();
        terminal.draw(|frame| hits = render(frame, "Choose", choices, Some("kill"), state)).unwrap();
        hits
    }

    fn mouse(state: &mut Selection, choices: &[Choice], hits: &SelectorHits, kind: MouseEventKind, point: Position) -> Action {
        state.handle_event(Event::Mouse(MouseEvent { kind, column: point.x, row: point.y, modifiers: KeyModifiers::NONE }), choices, hits)
    }

    fn click(state: &mut Selection, choices: &[Choice], hits: &SelectorHits, point: Position) -> Action {
        assert_eq!(mouse(state, choices, hits, MouseEventKind::Down(MouseButton::Left), point), Action::Continue);
        mouse(state, choices, hits, MouseEventKind::Up(MouseButton::Left), point)
    }

    fn rendered_position(terminal: &Terminal<TestBackend>, text: &str) -> Position {
        let buffer = terminal.backend().buffer();
        for y in 0..buffer.area.height {
            let row: String = (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect();
            if let Some(byte_index) = row.find(text) {
                return Position::new(Span::raw(&row[..byte_index]).width() as u16, y);
            }
        }
        panic!("{text:?} is missing from rendered selector");
    }

    fn choices() -> Vec<Choice> {
        vec![Choice { value: "coder".into(), label: None, description: "Build features".into() }, Choice { value: "reviewer".into(), label: None, description: "检查 changes".into() }]
    }

    #[test]
    fn selection_filters_descriptions_and_returns_exact_value() {
        let choices = choices();
        let mut state = Selection::new(choices.len(), false);
        state.handle(KeyEvent::new(KeyCode::Char('检'), KeyModifiers::NONE), &choices);
        assert_eq!(state.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &choices), Action::Pick(vec![1]));
        state.handle(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE), &choices);
        assert_eq!(state.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &choices), Action::Continue);
        state.handle(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL), &choices);
        state.handle(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &choices);
        assert_eq!(state.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &choices), Action::Pick(vec![1]));
        assert_eq!(state.handle(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &choices), Action::Cancel);
        assert_eq!(state.handle(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), &choices), Action::Cancel);
    }

    #[test]
    fn selection_multi_keeps_hidden_targets_and_requires_final_review() {
        let choices = choices();
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        let mut state = Selection::new(choices.len(), true);
        assert_eq!(state.handle(key(KeyCode::Enter), &choices), Action::Continue);
        assert!(!state.reviewing);
        state.handle(key(KeyCode::Char(' ')), &choices);
        state.handle(key(KeyCode::Char('检')), &choices);
        state.handle(key(KeyCode::Char(' ')), &choices);
        state.handle(key(KeyCode::Enter), &choices);
        assert!(state.reviewing);
        assert_eq!(state.selected_indices(), vec![0, 1]);
        state.handle(key(KeyCode::Backspace), &choices);
        assert!(!state.reviewing);
        state.handle(key(KeyCode::Enter), &choices);
        assert_eq!(state.handle(key(KeyCode::Enter), &choices), Action::Pick(vec![0, 1]));
        assert_eq!(state.handle(key(KeyCode::Esc), &choices), Action::Cancel);
    }

    #[test]
    fn wheel_and_keyboard_keep_rendered_rows_and_click_identity_aligned() {
        let choices = (0..20).map(|index| Choice { value: format!("id-{index}"), label: Some(format!("Choice {index:02}")), description: String::new() }).collect::<Vec<_>>();
        let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
        let mut state = Selection::new(choices.len(), false);
        let hits = draw(&mut terminal, &choices, &mut state);
        let first = rendered_position(&terminal, "Choice 00");
        mouse(&mut state, &choices, &hits, MouseEventKind::ScrollDown, first);
        let hits = draw(&mut terminal, &choices, &mut state);
        assert_eq!(rendered_position(&terminal, "Choice 03").y, first.y);

        for code in [KeyCode::End, KeyCode::Up] {
            state.handle_event(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)), &choices, &hits);
            draw(&mut terminal, &choices, &mut state);
        }
        assert_eq!(rendered_position(&terminal, "Choice 15").y, first.y);
        let hits = draw(&mut terminal, &choices, &mut state);
        mouse(&mut state, &choices, &hits, MouseEventKind::ScrollUp, first);
        let hits = draw(&mut terminal, &choices, &mut state);
        assert_eq!(rendered_position(&terminal, "Choice 12").y, first.y);
        assert_eq!(click(&mut state, &choices, &hits, rendered_position(&terminal, "Choice 13")), Action::Pick(vec![13]));
    }

    #[test]
    fn multi_mouse_edits_filtered_choices_then_confirms_only_from_review_footer() {
        let choices = choices();
        let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
        let mut state = Selection::new(choices.len(), true);
        let hits = draw(&mut terminal, &choices, &mut state);
        assert_eq!(click(&mut state, &choices, &hits, rendered_position(&terminal, "[Enter review]")), Action::Continue);
        assert!(!state.reviewing);
        click(&mut state, &choices, &hits, rendered_position(&terminal, "coder"));
        state.handle_event(Event::Key(KeyEvent::new(KeyCode::Char('检'), KeyModifiers::NONE)), &choices, &hits);
        let hits = draw(&mut terminal, &choices, &mut state);
        assert_eq!(click(&mut state, &choices, &hits, rendered_position(&terminal, "reviewer")), Action::Continue);

        let hits = draw(&mut terminal, &choices, &mut state);
        assert_eq!(click(&mut state, &choices, &hits, rendered_position(&terminal, "[Enter review]")), Action::Continue);
        let hits = draw(&mut terminal, &choices, &mut state);
        assert_eq!(click(&mut state, &choices, &hits, rendered_position(&terminal, "coder")), Action::Continue);
        assert_eq!(click(&mut state, &choices, &hits, rendered_position(&terminal, "[Backspace edit]")), Action::Continue);

        let hits = draw(&mut terminal, &choices, &mut state);
        rendered_position(&terminal, "[x] reviewer");
        rendered_position(&terminal, "1 hidden");
        click(&mut state, &choices, &hits, rendered_position(&terminal, "[Enter review]"));
        let hits = draw(&mut terminal, &choices, &mut state);
        assert_eq!(click(&mut state, &choices, &hits, rendered_position(&terminal, "[Enter kill]")), Action::Pick(vec![0, 1]));
    }

    #[test]
    fn interrupted_or_outside_mouse_gestures_do_not_pick() {
        use MouseEventKind::{Down, Drag, Up};

        let choices = choices();
        let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
        let mut state = Selection::new(choices.len(), false);
        let hits = draw(&mut terminal, &choices, &mut state);
        let first = rendered_position(&terminal, "coder");
        let other = rendered_position(&terminal, "reviewer");
        let blank = Position::new(other.x, other.y + 1);
        let outside = Position::new(0, 0);
        let left = MouseButton::Left;
        for (press, release) in [(outside, first), (first, outside), (first, other), (blank, blank)] {
            mouse(&mut state, &choices, &hits, Down(left), press);
            assert_eq!(mouse(&mut state, &choices, &hits, Up(left), release), Action::Continue);
        }
        for kind in [Down(MouseButton::Right), Up(MouseButton::Right)] {
            assert_eq!(mouse(&mut state, &choices, &hits, kind, first), Action::Continue);
        }
        mouse(&mut state, &choices, &hits, Down(left), first);
        mouse(&mut state, &choices, &hits, Drag(left), outside);
        assert_eq!(mouse(&mut state, &choices, &hits, Up(left), first), Action::Continue);

        mouse(&mut state, &choices, &hits, Down(left), first);
        state.handle_event(Event::Resize(60, 12), &choices, &hits);
        assert_eq!(mouse(&mut state, &choices, &hits, Up(left), first), Action::Continue);
        mouse(&mut state, &choices, &hits, Down(left), first);
        terminal.backend_mut().resize(20, 12);
        let hits = draw(&mut terminal, &choices, &mut state);
        assert_eq!(mouse(&mut state, &choices, &hits, Up(left), first), Action::Continue);
        assert_eq!(click(&mut state, &choices, &hits, rendered_position(&terminal, "[Cancel]")), Action::Cancel);
    }

    #[test]
    fn single_submit_picks_the_cursor_and_is_inactive_for_empty_results() {
        let choices = choices();
        let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
        let mut state = Selection::new(choices.len(), false);
        let hits = draw(&mut terminal, &choices, &mut state);
        state.handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)), &choices, &hits);
        let hits = draw(&mut terminal, &choices, &mut state);
        assert_eq!(click(&mut state, &choices, &hits, rendered_position(&terminal, "[Enter select]")), Action::Pick(vec![1]));
        state.handle_event(Event::Key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE)), &choices, &hits);
        let hits = draw(&mut terminal, &choices, &mut state);
        for text in ["No matches", "[Enter select]"] {
            assert_eq!(click(&mut state, &choices, &hits, rendered_position(&terminal, text)), Action::Continue);
        }
    }
}
