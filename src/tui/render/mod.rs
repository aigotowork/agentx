mod agents;
pub(super) mod info;
pub mod launch;
pub(crate) mod messages;
pub(crate) mod text;
mod transcript;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::tui::app::{App, Confirm, ConfirmAction};
use crate::tui::control_mouse::{ControlTarget, Field, OverflowSlot, ShortcutScope};
use crate::tui::inline::eject::filtered_counts;
use crate::tui::model::*;
use crate::tui::mouse::Target;
use crate::tui::theme::{Theme, palette};

/// Truncate a string to fit within `max_w` display columns, appending "\u{2026}" if truncated.
pub(crate) fn truncate_display(s: &str, max_w: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let w = UnicodeWidthStr::width(s);
    if w <= max_w {
        return s.to_string();
    }
    if max_w == 0 {
        return String::new();
    }
    let target = max_w.saturating_sub(1); // room for "\u{2026}"
    let mut result = String::new();
    let mut cur = 0;
    for ch in s.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if cur + cw > target {
            break;
        }
        result.push(ch);
        cur += cw;
    }
    result.push('\u{2026}');
    result
}

/// Truncate spans to fit within `max_w` display columns.
/// Preserves spans left-to-right (highest priority first), drops from the right.
pub(crate) fn fit_spans(spans: Vec<Span<'static>>, max_w: usize) -> Vec<Span<'static>> {
    use unicode_width::UnicodeWidthChar;
    let total: usize = spans.iter().map(|s| s.width()).sum();
    if total <= max_w {
        return spans;
    }
    let mut result: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    for span in spans {
        let w = span.width();
        if used + w <= max_w {
            result.push(span);
            used += w;
        } else {
            let remaining = max_w.saturating_sub(used);
            if remaining > 1 {
                let text = span.content.to_string();
                let target = remaining - 1; // room for …
                let mut truncated = String::new();
                let mut tw = 0;
                for ch in text.chars() {
                    let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
                    if tw + cw > target {
                        break;
                    }
                    truncated.push(ch);
                    tw += cw;
                }
                if !truncated.is_empty() {
                    result.push(Span::styled(truncated, span.style));
                }
                result.push(Span::styled("\u{2026}", span.style));
            } else if remaining == 1 {
                result.push(Span::styled("\u{2026}", span.style));
            }
            break;
        }
    }
    result
}

/// Register painted cells relative to a geometry rectangle; clipped controls never receive input.
fn add_control(app: &App, area: Rect, x: usize, row: usize, width: usize, target: ControlTarget) {
    if row < area.height as usize && x < area.width as usize {
        let width = width.min(area.width as usize - x) as u16;
        app.mouse.add(Rect::new(area.x + x as u16, area.y + row as u16, width, 1), Target::Control(target));
    }
}

fn mode_input_bg(app: &App) -> ratatui::style::Color {
    if let Some(ref confirm) = app.ui.confirm
        && confirm.is_inline_agent_action()
    {
        return match &confirm.action {
            ConfirmAction::KillAgents(_) => palette::MODE_KILL,
            ConfirmAction::ForkAgents(_) => palette::MODE_FORK,
            ConfirmAction::ResumeAgents(_) => palette::MODE_RESUME,
            _ => palette::SELECTION,
        };
    }
    if let Some(ref overlay) = app.ui.overlay {
        return match overlay.kind {
            OverlayKind::Search => palette::MODE_SEARCH,
            OverlayKind::Command => palette::MODE_CMD,
        };
    }
    match app.ui.mode {
        InputMode::Compose => palette::MODE_COMPOSE,
        InputMode::CommandOutput => palette::MODE_CMD,
        _ => palette::SELECTION,
    }
}

pub fn render(frame: &mut Frame, app: &mut App, inline_height: u16) {
    app.mouse.clear_targets();
    render_main(frame, app, inline_height);

    if app.ui.info.is_some() {
        return;
    }
    if app.ui.transcript.is_none() {
        if let Some(relay) = app.ui.relay_popup.as_ref() {
            app.mouse.clear_targets();
            render_footer(frame, footer_rect(frame.area()), app);
            render_relay_popup(frame, app, relay);
        } else if let Some(confirm) = app.ui.confirm.as_ref().filter(|confirm| !confirm.is_inline_agent_action()) {
            app.mouse.clear_targets();
            render_confirm(frame, app, confirm);
            return;
        } else if app.ui.help_open {
            app.mouse.clear_targets();
            render_help(frame, app);
            return;
        } else {
            render_command_palette(frame, app);
        }
    }
    render_more(frame, app);
}

fn empty_dashboard(app: &App) -> bool {
    app.ui.transcript.is_none()
        && app.ui.info.is_none()
        && app.data.agents.is_empty()
        && app.data.remote_agents.is_empty()
        && app.data.stopped_agents.is_empty()
        && app.data.orphans.is_empty()
        && app.data.messages.is_empty()
        && app.data.events.is_empty()
        && app.ui.mode == InputMode::Navigate
        && app.ui.overlay.is_none()
        && app.ui.confirm.is_none()
}

fn render_main(frame: &mut Frame, app: &mut App, inline_height: u16) {
    let area = frame.area();
    app.ui.term_width = area.width;

    if empty_dashboard(app) {
        render_empty(frame, area, app);
        return;
    }

    match app.ui.view_mode {
        ViewMode::Inline => {
            if app.ui.info.is_some() {
                render_inline_info(frame, app, area);
            } else if app.ui.transcript.is_some() {
                let controls_height = inline_height.min(area.height.saturating_sub(3));
                let [reader, controls] = Layout::vertical([Constraint::Min(0), Constraint::Length(controls_height)]).areas(area);
                render_reader(frame, reader, app);
                render_inline(frame, app, controls);
            } else {
                render_inline(frame, app, area);
            }
        }
        ViewMode::Vertical => render_vertical(frame, app, area),
    }
}

fn extra_list_rows(app: &App) -> u16 {
    let mut n = 0u16;
    if !app.data.remote_agents.is_empty() {
        n += 1;
        if app.ui.remote_expanded {
            n += app.data.remote_agents.len() as u16;
        }
    }
    if app.ui.view_mode != ViewMode::Inline && !app.data.stopped_agents.is_empty() {
        n += 1;
        if app.ui.stopped_expanded {
            n += app.data.stopped_agents.len() as u16;
        }
    }
    if !app.data.orphans.is_empty() {
        n += 1;
        if app.ui.orphans_expanded {
            n += app.data.orphans.len() as u16;
        }
    }
    n
}

/// Grapheme position within wrapped text: (byte_offset, visual_line, visual_col).
/// Single source of truth for wrap logic — all wrap functions derive from this.
struct WrapLayout {
    first_avail: usize,
    cont_avail: usize,
}

impl WrapLayout {
    fn new(total_width: u16, prefix_w: usize) -> Self {
        let w = total_width as usize;
        Self { first_avail: w - prefix_w.min(w.saturating_sub(1)), cont_avail: w }
    }

    /// Iterate grapheme positions: yields (byte_offset, line, col, grapheme_str, grapheme_width).
    /// After iteration, final (line, col) is one past the last grapheme.
    fn walk<'a>(&self, text: &'a str) -> WrapWalker<'a> {
        WrapWalker { graphemes: text.graphemes(true), line: 0, col: 0, offset: 0, cur_avail: self.first_avail, cont_avail: self.cont_avail }
    }
}

struct WrapWalker<'a> {
    graphemes: unicode_segmentation::Graphemes<'a>,
    line: usize,
    col: usize,
    offset: usize,
    cur_avail: usize,
    cont_avail: usize,
}

struct WrapPos<'a> {
    offset: usize,
    line: usize,
    col: usize,
    grapheme: &'a str,
    width: usize,
}

impl<'a> Iterator for WrapWalker<'a> {
    type Item = WrapPos<'a>;

    fn next(&mut self) -> Option<WrapPos<'a>> {
        let g = self.graphemes.next()?;
        let gw = UnicodeWidthStr::width(g);
        if self.col + gw > self.cur_avail && (self.col > 0 || self.cur_avail < self.cont_avail && gw <= self.cont_avail) {
            self.line += 1;
            self.cur_avail = self.cont_avail;
            self.col = 0;
        }
        let pos = WrapPos { offset: self.offset, line: self.line, col: self.col, grapheme: g, width: gw };
        self.col += gw;
        self.offset += g.len();
        Some(pos)
    }
}

/// Number of visual wrapped lines for text with given available width.
/// Prefix width (e.g. "  ❯ " = 4) is subtracted from the first line's width.
/// Continuation lines use the full terminal width (ratatui wraps flush left).
fn wrap_line_count(text: &str, total_width: u16, prefix_w: usize) -> usize {
    if text.is_empty() {
        return 1;
    }
    let layout = WrapLayout::new(total_width, prefix_w);
    if layout.first_avail == 0 {
        return 1;
    }
    let mut last_line = 0;
    for pos in layout.walk(text) {
        last_line = pos.line;
    }
    last_line + 1
}

/// Compute dynamic input area height for compose mode.
/// Returns 3 (default) for non-compose modes, grows with wrapped line count.
fn compose_input_height(app: &App, is_inline: bool, width: u16) -> u16 {
    if app.ui.mode != InputMode::Compose || app.ui.input.is_empty() {
        return 3;
    }
    let wrapped = wrap_line_count(&app.ui.input, width, 4) as u16;
    let max_lines = if is_inline { 4 } else { 8 };
    wrapped.min(max_lines) + 2 // +2 for top/bottom padding rows
}

fn cursor_wrap_location(text: &str, cursor: usize, total_width: u16, prefix_w: usize) -> (usize, usize) {
    let mut end = (0, 0);
    for pos in WrapLayout::new(total_width, prefix_w).walk(text) {
        if cursor <= pos.offset {
            return (pos.line, pos.col);
        }
        end = (pos.line, pos.col + pos.width);
    }
    end
}

/// Break text into visual lines using character-level wrapping.
/// Uses WrapLayout for consistent wrap logic with all other wrap functions.
fn break_into_visual_lines(text: &str, total_width: u16, prefix_w: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let layout = WrapLayout::new(total_width, prefix_w);
    if layout.first_avail == 0 {
        return vec![text.to_string()];
    }

    let mut lines: Vec<String> = vec![String::new()];
    for pos in layout.walk(text) {
        while lines.len() <= pos.line {
            lines.push(String::new());
        }
        lines[pos.line].push_str(pos.grapheme);
    }
    lines
}

/// Move to the closest insertion point on the adjacent visual row.
fn cursor_wrap_move(text: &str, cursor: &mut usize, total_width: u16, prefix_w: usize, down: bool) {
    let (line, col) = cursor_wrap_location(text, *cursor, total_width, prefix_w);
    let target = if down { Some(line + 1) } else { line.checked_sub(1) };
    let Some(target) = target else { return };
    let mut best = None;
    let mut end = (0, 0);
    for pos in WrapLayout::new(total_width, prefix_w).walk(text) {
        if pos.line > target {
            break;
        }
        if pos.line == target && (best.is_none() || pos.col <= col) {
            best = Some(pos.offset);
        }
        end = (pos.line, pos.col + pos.width);
    }
    let (last_line, last_col) = cursor_wrap_location(text, text.len(), total_width, prefix_w);
    if last_line == target && last_col <= col && end.0 == target {
        best = Some(text.len());
    }
    if let Some(offset) = best {
        *cursor = offset;
    }
}

pub(crate) fn cursor_wrap_up(text: &str, cursor: &mut usize, total_width: u16, prefix_w: usize) {
    cursor_wrap_move(text, cursor, total_width, prefix_w, false);
}

pub(crate) fn cursor_wrap_down(text: &str, cursor: &mut usize, total_width: u16, prefix_w: usize) {
    cursor_wrap_move(text, cursor, total_width, prefix_w, true);
}

/// The upper reader uses all rows left by the Agent strip, footer and active draft.
fn render_inline_info(frame: &mut Frame, app: &mut App, area: Rect) {
    let footer = u16::from(area.height >= 3);
    let strip = u16::from(area.height >= 2);
    let spare = area.height.saturating_sub(footer + strip + 3);
    let input = if app.ui.mode == InputMode::Compose { compose_input_height(app, true, area.width).min(spare) } else { 0 };
    let divider = u16::from(spare > input);
    let [reader, rule, agents, draft, hints] = Layout::vertical([Constraint::Min(0), Constraint::Length(divider), Constraint::Length(strip), Constraint::Length(input), Constraint::Length(footer)]).areas(area);
    if !rule.is_empty() {
        render_separator(frame, rule, app, rule.width);
    }
    agents::render_agents_tabstrip(frame, agents, app);
    render_bottom_section(frame, app, None, draft, hints);
    render_active_info(frame, app, reader, hints);
}

fn render_inline_header(frame: &mut Frame, app: &App, rule: Rect, status: Rect, separator: Rect) {
    if !rule.is_empty() {
        let top_rule = "─".repeat(rule.width as usize);
        frame.render_widget(Paragraph::new(Line::from(Span::styled(top_rule, filter_separator_style(app)))), rule);
    }
    if !status.is_empty() {
        render_status_bar(frame, status, app);
    }
    if !separator.is_empty() {
        render_separator(frame, separator, app, separator.width);
    }
}

fn render_inline(frame: &mut Frame, app: &mut App, area: Rect) {
    if area.is_empty() {
        return;
    }
    let launch_height = if app.ui.mode == InputMode::Launch { app.ui.launch.panel_height() } else { 0 };

    // On short terminals give the Agent strip a row before padding the inactive input bar.
    let mut input_height = compose_input_height(app, true, area.width);
    if app.ui.transcript.is_some() {
        input_height = if app.ui.mode == InputMode::Compose {
            input_height.min(area.height.saturating_sub(5))
        } else if area.height < 8 {
            0
        } else {
            input_height
        };
    }

    // Layout: topline + status + sep + agents + [launch] + input + footer
    let chrome = 1 + 1 + 1 + input_height + 1 + launch_height;
    let agent_height = area.height.saturating_sub(chrome);

    let mut constraints = vec![
        Constraint::Length(1),            // top line
        Constraint::Length(1),            // status bar
        Constraint::Length(1),            // separator
        Constraint::Length(agent_height), // agents (fills remaining)
    ];
    if launch_height > 0 {
        constraints.push(Constraint::Length(launch_height));
    }
    constraints.push(Constraint::Length(input_height)); // input
    constraints.push(Constraint::Length(1)); // footer

    let layout = Layout::vertical(constraints).split(area);
    let n = layout.len();

    render_inline_header(frame, app, layout[0], layout[1], layout[2]);

    if !app.data.agents.is_empty() || extra_list_rows(app) > 0 {
        agents::render_agents_tabstrip(frame, layout[3], app);
    }

    if launch_height > 0 {
        let la = layout[n - 3];
        let input_area = layout[n - 2];
        frame.render_widget(Block::default().style(Style::default().bg(mode_input_bg(app))), Rect::new(area.x, la.y, area.width, input_area.y + input_area.height - la.y));
        launch::render_launch_inline(frame, la, app);
    } else {
        frame.render_widget(Block::default().style(Style::default().bg(mode_input_bg(app))), layout[n - 2]);
    }

    render_controls(frame, app, layout[n - 2], layout[n - 1]);
    if app.ui.transcript.is_none() {
        app.ui.scroll_max = 0;
    }
}

fn render_vertical(frame: &mut Frame, app: &mut App, area: Rect) {
    let launch_height = if app.ui.mode == InputMode::Launch { app.ui.launch.panel_height() } else { 0 };

    let mut outer = vec![
        Constraint::Length(1), // status bar
        Constraint::Length(1), // blank
        Constraint::Min(3),    // body (agents left | messages right)
    ];
    if launch_height > 0 {
        outer.push(Constraint::Length(launch_height));
    }
    let input_height = compose_input_height(app, false, area.width);
    outer.push(Constraint::Length(1)); // blank
    outer.push(Constraint::Length(input_height)); // input
    outer.push(Constraint::Length(1)); // footer

    let vlayout = Layout::vertical(outer).split(area);
    let n = vlayout.len();
    let launch_area = if launch_height > 0 { Some(vlayout[n - 4]) } else { None };

    render_status_bar(frame, vlayout[0], app);

    // Separator line after agentx header (brighter when filtered)
    let sep_line = "\u{2500}".repeat(area.width as usize);
    frame.render_widget(Paragraph::new(Line::from(Span::styled(sep_line, filter_separator_style(app)))), vlayout[1]);

    // Split body horizontally: agents left, separator, messages right
    let left_width = (area.width / 3).clamp(20, 36);
    let left_width = if app.ui.info.is_some() { left_width.min(area.width / 2) } else { left_width };
    let body = Layout::horizontal([Constraint::Length(left_width), Constraint::Length(1), Constraint::Min(10)]).split(vlayout[2]);

    if !app.data.agents.is_empty() || extra_list_rows(app) > 0 {
        agents::render_agents_compact(frame, body[0], app);
    }

    // Vertical separator bar
    let sep_lines: Vec<Line> = (0..body[1].height).map(|_| Line::from(Span::styled("\u{2502}", Theme::separator()))).collect();
    frame.render_widget(Paragraph::new(sep_lines), body[1]);

    if app.ui.info.is_some() {
        render_bottom_section(frame, app, launch_area, vlayout[n - 2], vlayout[n - 1]);
        render_active_info(frame, app, body[2], vlayout[n - 1]);
        return;
    } else if app.ui.transcript.is_some() {
        render_reader(frame, body[2], app);
    } else {
        let [heading, content] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(body[2]);
        render_messages_heading(frame, heading, app);
        let scroll = messages::render_messages(frame, content, app);
        app.mouse.add(content, Target::Messages);
        app.ui.scroll_max = scroll.max;
        app.ui.msg_scroll = scroll.offset;
        app.ui.last_msg_total = scroll.total;
    }
    render_bottom_section(frame, app, launch_area, vlayout[n - 2], vlayout[n - 1]);
}

/// Shared bottom section: launch panel bg, input bar, footer, cursor.
fn render_bottom_section(frame: &mut Frame, app: &mut App, launch_area: Option<Rect>, input_area: Rect, footer_area: Rect) {
    if let Some(la) = launch_area {
        let bg_top = la.y;
        let bg_bottom = input_area.y + input_area.height;
        let bg_rect = Rect::new(input_area.x, bg_top, input_area.width, bg_bottom.saturating_sub(bg_top));
        frame.render_widget(Block::default().style(Style::default().bg(mode_input_bg(app))), bg_rect);
        launch::render_launch_inline(frame, la, app);
    } else {
        frame.render_widget(Block::default().style(Style::default().bg(mode_input_bg(app))), input_area);
    }

    render_controls(frame, app, input_area, footer_area);
}

fn render_controls(frame: &mut Frame, app: &mut App, input: Rect, footer: Rect) {
    if app.ui.info.is_some() {
        if app.ui.mode == InputMode::Compose && !input_content(input).is_empty() {
            render_compose(frame, input_content(input), app, false);
        }
        return;
    }
    if app.ui.mode != InputMode::Compose {
        app.ui.input_scroll = 0;
    }
    if app.ui.transcript.is_none() && (app.ui.confirm.is_some() || app.ui.overlay.is_some()) {
        app.mouse.clear_targets();
    }
    if app.ui.transcript.is_none() || app.ui.mode == InputMode::Compose {
        render_input(frame, input, app);
    }
    render_footer(frame, footer, app);
}

fn render_active_info(frame: &mut Frame, app: &mut App, reader: Rect, footer: Rect) {
    app.mouse.clear_targets();
    info::render(frame, reader, app);
    info::render_footer(frame, footer, app);
}

fn render_reader(frame: &mut Frame, area: Rect, app: &mut App) {
    if let Some(reader) = app.ui.transcript.as_mut() {
        transcript::render(frame, area, reader);
    }
    let body = Rect::new(area.x, area.y.saturating_add(1), area.width, area.height.saturating_sub(1));
    app.mouse.add(body, Target::Reader);
    if area.is_empty() {
        return;
    }
    let header = Rect::new(area.x, area.y, area.width, 1);
    render_close(frame, header, app, shortcut(ShortcutScope::Reader, KeyCode::Esc));
}

fn dashboard_title(app: &App) -> Vec<Span<'static>> {
    let mut title = vec![Span::raw("  "), Span::styled("agentx", Theme::title())];
    if let Some(scope) = app.source.agent_filter_label() {
        title.push(Span::styled(format!("  Agents: {scope} | timeline: all"), Theme::dim()));
    }
    title
}

fn render_empty(frame: &mut Frame, area: Rect, app: &App) {
    let [title, content, footer] = Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).areas(area);
    frame.render_widget(Paragraph::new(Line::from(dashboard_title(app))), title);

    let mut lines = vec![Line::raw("")];
    if let Some(err) = app.backend_error() {
        lines.push(Line::from(Span::styled("  Backend unavailable", Style::default().fg(palette::RED))));
        lines.push(Line::from(Span::styled(format!("  {}", truncate_display(&err, area.width.saturating_sub(4) as usize)), Theme::dim())));
    } else {
        let hint = if app.source.agent_filter_label().is_some() { "  No agents match the selected filters" } else { "  No agents running" };
        lines.push(Line::from(Span::styled(hint, Style::default().fg(palette::FG_DIM))));
    }
    lines.push(Line::raw(""));
    let first_action = lines.len();
    let actions = [
        Hint::key(ShortcutScope::Navigate, KeyCode::Tab, "tab", "launch agents"),
        Hint::key(ShortcutScope::Navigate, KeyCode::Char('!'), "!", "run agentx command"),
        Hint::relay(),
        Hint::key(ShortcutScope::Navigate, KeyCode::Char('?'), "?", "keyboard shortcuts"),
    ];
    for (i, hint) in actions.iter().enumerate() {
        let spans = vec![Span::raw("  "), Span::styled(format!("{:<10}", hint.key), Style::default().fg(palette::FG)), Span::styled(hint.label, Theme::dim())];
        let spans = fit_spans(spans, content.width as usize);
        let width = spans.iter().map(Span::width).sum::<usize>();
        // Footer overflow also exposes these actions if the body is too short or narrow.
        if width > 2 && content.width > 3 {
            add_control(app, content, 2, first_action + i, width - 2, hint.target.clone());
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), content);
    render_footer(frame, footer, app);
}

fn render_messages_heading(frame: &mut Frame, area: Rect, app: &App) {
    let selected_names = || app.ui.selected.iter().cloned().collect::<Vec<_>>().join(", ");
    let label = if !app.ui.selected.is_empty() { selected_names() } else { "messages".to_string() };

    let count_str = messages::display_count_str(app);

    let has_filter = !app.ui.selected.is_empty() || app.ui.search_filter.is_some();
    let label_style = if has_filter { Style::default().fg(palette::BLUE) } else { Theme::dim() };

    let spans = vec![Span::raw("  "), Span::styled(label, label_style), Span::raw("  "), Span::styled(count_str, Style::default().fg(palette::FG_DARK))];
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let mut errors = 0u16;
    let mut active = 0u16;
    let mut listening = 0u16;
    let mut blocked = 0u16;
    let mut launching = 0u16;
    let mut inactive = 0u16;

    for agent in app.data.agents.iter().chain(app.data.remote_agents.iter()) {
        match agent.status {
            AgentStatus::Error => errors += 1,
            AgentStatus::Active => active += 1,
            AgentStatus::Listening => listening += 1,
            AgentStatus::Blocked => blocked += 1,
            AgentStatus::Launching => launching += 1,
            AgentStatus::Inactive => inactive += 1,
        }
    }

    let mut left = dashboard_title(app);

    // Filter/count info next to title
    let dim_info = Style::default().fg(palette::FG_DARK);
    if app.ui.view_mode == ViewMode::Inline {
        let has_agent_filter = app.ui.eject_filter.is_some();
        let has_text_filter = app.ui.search_filter.is_some();
        let (ev_count, msg_count) = filtered_counts(&app.data, &app.ui.eject_filter, &app.ui.search_filter);
        let matched = ev_count + msg_count;
        let loaded = app.data.events.len() + app.data.messages.len();

        left.push(Span::raw("  "));
        if has_text_filter {
            // FTS search: searched entire DB
            let mut parts: Vec<String> = Vec::new();
            if let Some(ref ef) = app.ui.eject_filter {
                parts.push(ef.iter().cloned().collect::<Vec<_>>().join(", "));
            }
            if let Some(ref sf) = app.ui.search_filter {
                parts.push(format!("/{}", sf));
            }
            parts.push(format!("{} found", matched));
            left.push(Span::styled(format!("[{}]", parts.join(" \u{00b7} ")), Style::default().fg(palette::YELLOW)));
        } else if has_agent_filter {
            // Agent filter: X matching out of loaded
            let names = app.ui.eject_filter.as_ref().unwrap().iter().cloned().collect::<Vec<_>>().join(", ");
            left.push(Span::styled(format!("[{} \u{00b7} {}/{}]", names, matched, loaded), Style::default().fg(palette::BLUE)));
        } else {
            left.push(Span::styled(format!("[last {}]", loaded), dim_info));
        }
    } else if let Some(ref filter) = app.ui.search_filter {
        // Vertical mode: show search filter
        left.push(Span::raw("  "));
        left.push(Span::styled(format!("/{}", filter), Style::default().fg(palette::CYAN)));
    }

    let mut right: Vec<Span> = Vec::new();

    // Relay indicator (right side, before status counts). Branches off the
    // canonical RelayHealth — same enum the CLI and JSON render from, so
    // the indicator can't disagree with `agentx relay` / `agentx status`.
    use crate::relay::{RelayErrorReason, RelayHealth};
    match &app.data.relay_health {
        RelayHealth::Connected => {
            let show_text = app.ui.relay_text_until.is_some_and(|t| std::time::Instant::now() < t);
            if show_text {
                right.push(Span::styled("relay connected ", Style::default().fg(palette::GREEN)));
            }
            right.push(Span::styled("\u{21c4}  ", Style::default().fg(palette::GREEN)));
        }
        RelayHealth::Error { reason, detail, .. } => {
            // Show short detail only for worker-reported errors — StalePidfile
            // and Ghost are operational anomalies the user can't act on inline.
            if matches!(reason, RelayErrorReason::Reported)
                && let Some(err) = detail
            {
                let truncated: String = err.chars().take(20).collect();
                right.push(Span::styled(format!("{} ", truncated), Style::default().fg(palette::RED)));
            }
            right.push(Span::styled("\u{21c4}  ", Style::default().fg(palette::RED)));
        }
        RelayHealth::Stale { .. } => {
            right.push(Span::styled("\u{21c4}  ", Style::default().fg(palette::RED)));
        }
        RelayHealth::Starting { .. } | RelayHealth::Waiting => {
            right.push(Span::styled("\u{21c4}  ", Style::default().fg(palette::FG_DIM)));
        }
        RelayHealth::Disabled => {
            // Disabled means relay_id is set but enabled=false — show a dim
            // arrow so the user knows the toggle exists. (Previously this was
            // gated on raw_status being present, which broke once disable
            // started clearing runtime KV.)
            right.push(Span::styled("\u{21c4}  ", Style::default().fg(palette::FG_DIM)));
        }
        RelayHealth::NotConfigured => {}
    }

    let relay_width: usize = right.iter().map(Span::width).sum();
    let counts = [(errors, AgentStatus::Error), (active, AgentStatus::Active), (listening, AgentStatus::Listening), (blocked, AgentStatus::Blocked), (launching, AgentStatus::Launching), (inactive, AgentStatus::Inactive)];
    for (count, status) in counts {
        if count > 0 {
            right.push(Span::styled(format!("{} ", status.icon()), status.style()));
            right.push(Span::styled(format!("{}  ", count), status.style()));
        }
    }

    let left_width: usize = left.iter().map(|s| s.width()).sum();
    let right_width: usize = right.iter().map(|s| s.width()).sum();
    let pad = (area.width as usize).saturating_sub(left_width + right_width);

    let mut spans = left;
    spans.push(Span::raw(" ".repeat(pad)));
    spans.extend(right);

    if relay_width > 0 && ShortcutScope::Navigate.active(app) {
        add_control(app, area, left_width + pad, 0, relay_width, Hint::relay().target);
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Whether any filter (selection, agent filter, search) is active.
fn has_active_filter(app: &App) -> bool {
    !app.ui.selected.is_empty() || app.ui.eject_filter.is_some() || app.ui.search_filter.is_some()
}

/// Separator style: brighter when a filter is active to visually frame the filtered state.
fn filter_separator_style(app: &App) -> Style {
    if has_active_filter(app) { Style::default().fg(palette::FG_DIM) } else { Theme::separator() }
}

fn render_separator(frame: &mut Frame, area: Rect, app: &App, width: u16) {
    let cmd_label: String;
    let flash_label: String;

    // Center label: command > flash (selection shown via tab styling, not here)
    let label_opt: Option<(&str, Style)> = if let Some(ref cr) = app.ui.command_result {
        cmd_label = format!("! {}", cr.label);
        Some((cmd_label.as_str(), Theme::dim()))
    } else if let Some(flash) = &app.ui.flash {
        flash_label = format!("\u{25cf} {}", flash.text);
        Some((flash_label.as_str(), flash.style))
    } else {
        None
    };

    // Right-side scroll position hint
    let right_hint_str: String;
    let right_hint = if app.ui.msg_scroll > 0 {
        let max = app.ui.scroll_max;
        let pos = max.saturating_sub(app.ui.msg_scroll);
        right_hint_str = format!(" \u{2191} {}/{} ", pos, max);
        right_hint_str.as_str()
    } else {
        ""
    };

    let avail = width as usize;
    let right_w = UnicodeWidthStr::width(right_hint);

    let effective_right = if right_w < avail { right_hint } else { "" };
    let effective_right_w = UnicodeWidthStr::width(effective_right);

    let sep_style = filter_separator_style(app);
    let mut spans: Vec<Span> = Vec::new();

    if let Some((label, style)) = label_opt {
        let label_padded = format!(" {} ", label);
        let label_w = UnicodeWidthStr::width(label_padded.as_str());
        let truncated: String;
        let label_display = if label_w + effective_right_w + 2 <= avail {
            label_padded.as_str()
        } else {
            let max_label = avail.saturating_sub(effective_right_w + 2);
            truncated = truncate_display(&label_padded, max_label);
            truncated.as_str()
        };
        let label_display_w = UnicodeWidthStr::width(label_display);
        let fill_total = avail.saturating_sub(label_display_w + effective_right_w);
        let fill_left = fill_total / 2;
        let fill_right = fill_total - fill_left;
        spans.push(Span::styled("\u{2500}".repeat(fill_left), sep_style));
        spans.push(Span::styled(label_display.to_string(), style));
        spans.push(Span::styled("\u{2500}".repeat(fill_right), sep_style));
    } else {
        let fill = avail.saturating_sub(effective_right_w);
        spans.push(Span::styled("\u{2500}".repeat(fill), sep_style));
    }

    if !effective_right.is_empty() {
        let hint_style = if app.active_search_query().is_some() { Style::default().fg(palette::CYAN) } else { Theme::dim() };
        spans.push(Span::styled(effective_right.to_string(), hint_style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_input(frame: &mut Frame, area: Rect, app: &mut App) {
    let content = input_content(area);
    if content.is_empty() {
        return;
    }
    let row = Rect::new(content.x, content.y, content.width, 1);
    let show_cursor = app.ui.transcript.is_none() && !app.ui.help_open && app.ui.confirm.is_none() && app.ui.relay_popup.is_none() && app.control_overflow.is_none();
    match app.ui.mode {
        InputMode::CommandOutput => {
            if let Some(ref result) = app.ui.command_result {
                let line = Line::from(vec![Span::raw("  "), Span::styled("! ", Theme::dim()), Span::styled(result.label.clone(), Theme::dim())]);
                frame.render_widget(Paragraph::new(line), row);
            }
        }
        InputMode::Launch => {
            let prefix = vec![Span::raw("  "), Span::styled("prompt ", Theme::dim()), Span::styled("❯ ", Theme::cursor())];
            render_text_field(
                frame,
                row,
                app,
                prefix,
                TextField { field: Field::LaunchPrompt, text: &app.ui.input, cursor: app.ui.input_cursor, placeholder: "initial task for agent…", show_cursor: show_cursor && app.ui.launch.options_cursor.is_none() },
            );
        }
        InputMode::Relay => {}
        InputMode::Compose => render_compose(frame, content, app, show_cursor),
        InputMode::Navigate => {
            if let Some(ref confirm) = app.ui.confirm
                && confirm.is_inline_agent_action()
            {
                frame.render_widget(Paragraph::new(render_inline_confirm_input(confirm)), row);
            } else if let Some(overlay) = app.ui.overlay.as_ref() {
                let (label, marker, accent, field) = match overlay.kind {
                    OverlayKind::Search => ("SEARCH ", "/ ", palette::YELLOW, Field::Search),
                    OverlayKind::Command => ("CMD ", "! ", palette::MAGENTA, Field::Command),
                };
                let prefix = vec![Span::raw("  "), Span::styled(label, Style::default().fg(accent).add_modifier(Modifier::BOLD)), Span::styled(marker, Theme::cursor())];
                render_text_field(frame, row, app, prefix, TextField { field, text: &overlay.input, cursor: overlay.cursor, placeholder: "", show_cursor });
            } else {
                render_hint_row(frame, row, app, OverflowSlot::Input(0), &navigate_hints(app));
            }
        }
    }
}

fn input_content(area: Rect) -> Rect {
    if area.height >= 3 { Rect::new(area.x, area.y + 1, area.width, area.height - 2) } else { area }
}

struct TextField<'a> {
    field: Field,
    text: &'a str,
    cursor: usize,
    placeholder: &'a str,
    show_cursor: bool,
}

/// The horizontal window and all insertion cells come from the same graphemes as the rendered text.
fn render_text_field(frame: &mut Frame, area: Rect, app: &App, prefix: Vec<Span<'static>>, field: TextField<'_>) {
    if area.is_empty() {
        return;
    }
    let prefix = fit_spans(prefix, area.width.saturating_sub(1) as usize);
    let prefix_width: usize = prefix.iter().map(Span::width).sum();
    let body_width = area.width as usize - prefix_width;
    let masked = field.field == Field::RelayToken;
    let width = |g| if masked { 1 } else { UnicodeWidthStr::width(g) };
    let graphemes: Vec<_> = field.text.grapheme_indices(true).collect();
    let cursor = graphemes.partition_point(|&(offset, _)| offset < field.cursor);
    let start_for = |end: usize| {
        let mut remaining = body_width - 1; // reserve the insertion cell
        let mut start = end;
        while start > 0 {
            let w = width(graphemes[start - 1].1);
            if w > remaining {
                break;
            }
            remaining -= w;
            start -= 1;
        }
        start
    };
    // Keep the rendered tail steady when clicking anywhere in it.
    let tail = start_for(graphemes.len());
    let start = if cursor >= tail { tail } else { start_for(cursor) };
    let caret_col: usize = graphemes[start..cursor].iter().map(|&(_, g)| width(g)).sum();

    let start_offset = graphemes.get(start).map_or(field.text.len(), |&(offset, _)| offset);
    add_control(app, area, 0, 0, prefix_width, ControlTarget::Text(field.field, start_offset));
    let mut spans = prefix;
    let mut text = String::new();
    let mut col = prefix_width;
    let mut end_offset = start_offset;
    for &(offset, grapheme) in &graphemes[start..] {
        let w = width(grapheme);
        if col + w > area.width as usize {
            break;
        }
        add_control(app, area, col, 0, w, ControlTarget::Text(field.field, offset));
        text.push_str(if masked { "•" } else { grapheme });
        col += w;
        end_offset = offset + grapheme.len();
    }
    add_control(app, area, col, 0, area.width as usize, ControlTarget::Text(field.field, end_offset));
    if field.text.is_empty() {
        spans.push(Span::styled(field.placeholder, Theme::dim()));
    } else {
        spans.push(Span::styled(text, Style::default().fg(palette::FG)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
    if field.show_cursor {
        frame.set_cursor_position(Position::new(area.x + (prefix_width + caret_col) as u16, area.y));
    }
}

const COMPOSE_PREFIX: usize = 4;

fn render_compose(frame: &mut Frame, area: Rect, app: &mut App, show_cursor: bool) {
    let layout = WrapLayout::new(area.width, COMPOSE_PREFIX);
    let (caret_line, caret_col) = cursor_wrap_location(&app.ui.input, app.ui.input_cursor, area.width, COMPOSE_PREFIX);
    let visual_lines = break_into_visual_lines(&app.ui.input, area.width, COMPOSE_PREFIX);
    let previous_scroll = if app.ui.transcript.is_some() { app.ui.input_scroll.min(caret_line) } else { app.ui.input_scroll };
    let scroll = previous_scroll.min(visual_lines.len().saturating_sub(area.height as usize)).clamp(caret_line.saturating_sub(area.height as usize - 1), caret_line);
    if app.ui.transcript.is_none() && app.ui.info.is_none() {
        app.ui.input_scroll = scroll;
    }
    let prefix = fit_spans(vec![Span::raw("  "), Span::styled("❯ ", Theme::cursor())], area.width.saturating_sub(1).min(COMPOSE_PREFIX as u16) as usize);
    let prefix_width = area.width as usize - layout.first_avail;
    let mut byte_offset = visual_lines[..scroll].iter().map(String::len).sum::<usize>();
    let mut lines = Vec::new();
    for (row, line) in visual_lines.iter().skip(scroll).take(area.height as usize).enumerate() {
        let first = row + scroll == 0;
        let mut col = if first { prefix_width } else { 0 };
        add_control(app, area, 0, row, col, ControlTarget::Text(Field::Compose, byte_offset));
        for grapheme in line.graphemes(true) {
            let w = UnicodeWidthStr::width(grapheme);
            add_control(app, area, col, row, w, ControlTarget::Text(Field::Compose, byte_offset));
            col += w;
            byte_offset += grapheme.len();
        }
        add_control(app, area, col, row, area.width as usize, ControlTarget::Text(Field::Compose, byte_offset));
        let mut spans = if first { prefix.clone() } else { Vec::new() };
        spans.extend(compose_input_spans(line));
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), area);

    if !app.ui.input.is_empty() && visual_lines.len() == 1 {
        let hint = format!("→ {} ", send_target_display(&app.ui.input));
        let hint_w = UnicodeWidthStr::width(hint.as_str());
        let input_w = prefix_width + UnicodeWidthStr::width(app.ui.input.as_str());
        if input_w + hint_w + 2 < area.width as usize {
            let hint_area = Rect::new(area.right() - hint_w as u16 - 1, area.y, hint_w as u16, 1);
            frame.render_widget(Paragraph::new(Span::styled(hint, Theme::dim())), hint_area);
        }
    }

    if show_cursor {
        let caret_x = caret_col + if caret_line == 0 { prefix_width } else { 0 };
        frame.set_cursor_position(Position::new(area.x + caret_x.min(area.width as usize - 1) as u16, area.y + (caret_line - scroll) as u16));
    }
}

fn compose_input_spans(text: &str) -> Vec<Span<'static>> {
    use crate::tui::input::compose::is_recipient_char;

    let mut spans = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current = String::new();

    while let Some(ch) = chars.next() {
        if ch == '@' {
            let mut mention = String::from("@");
            while let Some(&next) = chars.peek() {
                if is_recipient_char(next) {
                    mention.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            if mention.len() > 1 {
                if !current.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut current), Style::default().fg(palette::FG)));
                }
                spans.push(Span::styled(mention, Theme::mention()));
            } else {
                current.push('@');
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        spans.push(Span::styled(current, Style::default().fg(palette::FG)));
    }

    spans
}

fn render_inline_confirm_input(confirm: &Confirm) -> Line<'static> {
    let (label, accent) = match &confirm.action {
        ConfirmAction::KillAgents(_) => ("KILL", palette::RED),
        ConfirmAction::ForkAgents(_) => ("FORK", palette::BLUE),
        ConfirmAction::ResumeAgents(_) => ("RESUME", palette::GREEN),
        _ => ("CONFIRM", palette::YELLOW),
    };
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{} ", label), Style::default().fg(accent).add_modifier(Modifier::BOLD)),
        Span::styled("\u{276f} ", Style::default().fg(accent)),
        Span::styled(confirm.text.clone(), Style::default().fg(palette::FG)),
    ])
}

fn send_target_display(input: &str) -> String {
    use crate::tui::input::compose::is_recipient_char;
    let mut recipients = Vec::new();
    for token in input.split_whitespace() {
        if token.starts_with('@') && token.len() > 1 {
            let name: String = token[1..].chars().take_while(|c| is_recipient_char(*c)).collect();
            if !name.is_empty() && !recipients.contains(&name) {
                recipients.push(name);
            }
        }
    }
    if recipients.is_empty() { "all".to_string() } else { recipients.join(", ") }
}

fn shortcut(scope: ShortcutScope, key: KeyCode) -> ControlTarget {
    ControlTarget::Shortcut(scope, key, KeyModifiers::NONE)
}

struct Hint {
    target: ControlTarget,
    key: &'static str,
    label: &'static str,
}

impl Hint {
    fn new(target: ControlTarget, key: &'static str, label: &'static str) -> Self {
        Self { target, key, label }
    }

    fn key(scope: ShortcutScope, code: KeyCode, key: &'static str, label: &'static str) -> Self {
        Self::new(shortcut(scope, code), key, label)
    }

    fn relay() -> Self {
        Self::new(ControlTarget::Shortcut(ShortcutScope::Navigate, KeyCode::Char('r'), KeyModifiers::CONTROL), "^r", "relay settings")
    }

    fn width(&self) -> usize {
        UnicodeWidthStr::width(self.key) + usize::from(!self.key.is_empty()) + UnicodeWidthStr::width(self.label)
    }

    fn spans(&self) -> [Span<'static>; 3] {
        [Span::styled(self.key, Style::default().fg(palette::FG_MID)), Span::raw(if self.key.is_empty() { "" } else { " " }), Span::styled(self.label, Theme::dim())]
    }
}

fn navigate_hints(app: &App) -> Vec<Hint> {
    use ShortcutScope::Navigate;
    let make = |code, key, label| Hint::key(Navigate, code, key, label);
    let mut hints = Vec::new();
    let orphan = matches!(app.cursor_target(), CursorTarget::Orphan(_));
    if orphan {
        hints.push(make(KeyCode::Enter, "enter", "kill/recover"));
    } else {
        if app.ui.search_filter.is_some() {
            hints.push(make(KeyCode::Esc, "esc", "clear search"));
        } else if !app.ui.selected.is_empty() || app.ui.eject_filter.is_some() {
            hints.push(make(KeyCode::Esc, "esc", "clear selection"));
        }
        hints.push(make(KeyCode::Char('m'), "m", "message"));
        let actions = app.cursor_action_availability();
        if actions.resume {
            hints.push(make(KeyCode::Char('r'), "r", "resume"));
        }
        if actions.fork {
            hints.push(make(KeyCode::Char('f'), "f", "fork"));
        }
        if actions.kill {
            hints.push(make(KeyCode::Char('k'), "k", "kill"));
        }
    }
    if app.has_info_target() {
        hints.push(make(KeyCode::Char('i'), "i", "Info"));
    }
    hints.extend([make(KeyCode::Char('/'), "/", "search"), make(KeyCode::Char('!'), "!", "command")]);
    if orphan || app.ui.selected.is_empty() {
        hints.push(make(KeyCode::Tab, "tab", "launch"));
    }
    hints
}

fn footer_hints(app: &App) -> Vec<Hint> {
    use ShortcutScope::*;
    let key = Hint::key;
    let view = |scope| key(scope, KeyCode::Char('\\'), "\\", "view");
    if app.ui.transcript.is_some() {
        let mut hints = vec![key(Reader, KeyCode::Esc, "t/Esc", "events")];
        if app.has_info_target() {
            hints.push(key(Reader, KeyCode::Char('i'), "i", "Info"));
        }
        hints.extend([key(Reader, KeyCode::Up, "↑", "older"), key(Reader, KeyCode::Down, "↓", "newer"), key(Reader, KeyCode::Char('r'), "r", "refresh"), view(Reader)]);
        return hints;
    }

    if app.ui.confirm.as_ref().is_some_and(|confirm| confirm.is_inline_agent_action()) {
        return vec![Hint::new(ControlTarget::ConfirmChoice(true), "enter", "confirm"), Hint::new(ControlTarget::ConfirmChoice(false), "esc", "cancel")];
    }

    match app.ui.mode {
        InputMode::Relay => {
            if app.ui.relay_popup.as_ref().is_some_and(|relay| relay.editing_token) {
                vec![key(Relay, KeyCode::Enter, "enter", "connect"), key(Relay, KeyCode::Esc, "esc", "cancel")]
            } else {
                vec![key(Relay, KeyCode::Enter, "enter", "select"), key(Relay, KeyCode::Esc, "esc", "close"), key(Relay, KeyCode::Up, "↑", "previous"), key(Relay, KeyCode::Down, "↓", "next")]
            }
        }
        InputMode::CommandOutput => vec![key(Output, KeyCode::Esc, "esc", "back")],
        InputMode::Launch => {
            if app.ui.launch.quick.picker.is_some() {
                vec![key(Launch, KeyCode::Enter, "enter", "select"), key(Launch, KeyCode::Esc, "esc", "cancel")]
            } else if let Some(field) = app.ui.launch.options_cursor {
                let mut hints = vec![key(Launch, KeyCode::Esc, "esc", "prompt")];
                if matches!(field, LaunchField::Role | LaunchField::Profile | LaunchField::More) {
                    hints.extend([key(Launch, KeyCode::Enter, "enter", "open"), key(Launch, KeyCode::Left, "←", "previous"), key(Launch, KeyCode::Right, "→", "next")]);
                } else {
                    hints.extend([key(Launch, KeyCode::Up, "↑", "previous field"), key(Launch, KeyCode::Down, "↓", "next field")]);
                    if field == LaunchField::Headless {
                        hints.push(key(Launch, KeyCode::Enter, "enter", "toggle"));
                    } else {
                        hints.extend([key(Launch, KeyCode::Left, "←", "decrease"), key(Launch, KeyCode::Right, "→", "increase")]);
                    }
                }
                hints
            } else {
                vec![Hint::new(ControlTarget::SubmitLaunch, "enter", "launch"), key(Launch, KeyCode::Up, "↑", "options"), key(Launch, KeyCode::Tab, "tab", "close")]
            }
        }
        InputMode::Compose => {
            let mut hints = vec![key(Compose, KeyCode::Enter, "enter", "send"), key(Compose, KeyCode::Esc, "esc", "cancel")];
            if app.has_transcript_target() {
                hints.push(Hint::new(ControlTarget::OpenReader, "^t", "transcript"));
            }
            hints
        }
        InputMode::Navigate => {
            if let Some(overlay) = app.ui.overlay.as_ref() {
                let (scope, verb) = if overlay.kind == OverlayKind::Command { (Command, "run") } else { (Search, "keep") };
                let mut hints = vec![key(scope, KeyCode::Enter, "enter", verb), key(scope, KeyCode::Esc, "esc", "cancel")];
                if overlay.palette.is_some() {
                    hints.extend([key(scope, KeyCode::Tab, "tab", "fill"), key(scope, KeyCode::Up, "↑", "previous"), key(scope, KeyCode::Down, "↓", "next")]);
                }
                hints
            } else {
                let mut hints = Vec::new();
                if empty_dashboard(app) {
                    hints.extend([key(Navigate, KeyCode::Tab, "tab", "launch"), key(Navigate, KeyCode::Char('!'), "!", "command")]);
                } else if app.has_transcript_target() {
                    hints.push(key(Navigate, KeyCode::Char('t'), "t", "transcript"));
                }
                hints.extend([key(Navigate, KeyCode::Char('?'), "?", "help"), view(Navigate), Hint::relay()]);
                if !app.data.agents.is_empty() {
                    hints.push(key(Navigate, KeyCode::Char('a'), "a", "select all"));
                }
                hints.push(key(Navigate, KeyCode::Char('b'), "b", "broadcast"));
                let stopped_label = if app.ui.show_all_stopped { "recent stopped" } else { "all stopped" };
                hints.push(Hint::new(ControlTarget::Shortcut(Navigate, KeyCode::Char('s'), KeyModifiers::CONTROL), "^s", stopped_label));
                hints.push(Hint::new(ControlTarget::Shortcut(Navigate, KeyCode::Char('d'), KeyModifiers::CONTROL), "^d", "quit"));
                hints
            }
        }
    }
}

fn footer_rect(area: Rect) -> Rect {
    let height = area.height.min(1);
    Rect::new(area.x, area.bottom().saturating_sub(height), area.width, height)
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    render_hint_row(frame, area, app, OverflowSlot::Footer(0), &footer_hints(app));
}

const HINT_SEPARATOR: &str = " · ";

/// Pack whole controls; once a control overflows, every following one is reachable in More.
fn hint_row_plan(width: usize, hints: &[Hint]) -> (usize, usize, &'static str) {
    let inset = if width >= 10 { 2 } else { 0 };
    let separator = UnicodeWidthStr::width(HINT_SEPARATOR);
    let total = hints.iter().map(Hint::width).sum::<usize>() + separator * hints.len().saturating_sub(1);
    if inset + total <= width {
        return (inset, hints.len(), "");
    }
    let more = if width >= 8 { "… more" } else { "…" };
    let available = width.saturating_sub(inset + UnicodeWidthStr::width(more));
    let mut used = 0;
    let mut count = 0;
    for hint in hints {
        let next = hint.width() + separator;
        if used + next > available {
            break;
        }
        used += next;
        count += 1;
    }
    (inset, count, more)
}

fn render_hint_row(frame: &mut Frame, area: Rect, app: &App, slot: OverflowSlot, hints: &[Hint]) {
    if area.is_empty() {
        return;
    }
    let (mut x, visible, more) = hint_row_plan(area.width as usize, hints);
    let mut spans = vec![Span::raw(" ".repeat(x))];
    for (index, hint) in hints.iter().take(visible).enumerate() {
        if index > 0 {
            spans.push(Span::styled(HINT_SEPARATOR, Theme::dim()));
            x += UnicodeWidthStr::width(HINT_SEPARATOR);
        }
        if matches!(hint.target, ControlTarget::Shortcut(ShortcutScope::Navigate | ShortcutScope::Reader, KeyCode::Char('i'), KeyModifiers::NONE)) {
            app.mouse.add(Rect::new(area.x + x as u16, area.y, hint.width() as u16, 1), Target::InfoOpen);
        } else {
            add_control(app, area, x, 0, hint.width(), hint.target.clone());
        }
        spans.extend(hint.spans());
        x += hint.width();
    }
    if !more.is_empty() {
        if visible > 0 {
            spans.push(Span::styled(HINT_SEPARATOR, Theme::dim()));
            x += UnicodeWidthStr::width(HINT_SEPARATOR);
        }
        add_control(app, area, x, 0, UnicodeWidthStr::width(more), ControlTarget::More(slot));
        spans.push(Span::styled(more, Theme::title()));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_more(frame: &mut Frame, app: &mut App) {
    let Some(slot) = app.control_overflow else { return };
    let area = frame.area();
    if area.is_empty() {
        return;
    }
    let (hints, offset) = match slot {
        OverflowSlot::Input(offset) if ShortcutScope::Navigate.active(app) && !empty_dashboard(app) => (navigate_hints(app), offset),
        OverflowSlot::Input(_) => {
            app.control_overflow = None;
            return;
        }
        OverflowSlot::Footer(offset) => (footer_hints(app), offset),
    };
    let (_, visible, _) = hint_row_plan(area.width as usize, &hints);
    let hidden = &hints[visible..];
    if hidden.is_empty() {
        app.control_overflow = None;
        return;
    }
    let header_h = u16::from(area.height > 1);
    let rows = hidden.len().min(area.height.saturating_sub(header_h) as usize);
    let max = hidden.len().saturating_sub(rows);
    let offset = offset.min(max);
    app.control_overflow = Some(match slot {
        OverflowSlot::Input(_) => OverflowSlot::Input(offset),
        OverflowSlot::Footer(_) => OverflowSlot::Footer(offset),
    });
    let w = area.width.min(34);
    let h = header_h + rows as u16;
    let popup = Rect::new(area.right() - w, area.bottom() - h, w, h);
    app.mouse.clear_targets();
    app.mouse.add(popup, Target::Control(ControlTarget::MoreScroll { max }));
    frame.render_widget(Clear, popup);
    frame.render_widget(Block::default().style(Style::default().bg(palette::SELECTION)), popup);
    if header_h > 0 {
        let header = Rect::new(popup.x, popup.y, popup.width, header_h);
        let title = if max > 0 { format!(" More · {}–{} / {}", offset + 1, offset + rows, hidden.len()) } else { " More".into() };
        frame.render_widget(Paragraph::new(Span::styled(title, Theme::title())), header);
        render_close(frame, header, app, ControlTarget::CloseMore);
    }
    for (row, hint) in hidden.iter().skip(offset).take(rows).enumerate() {
        let y = header_h as usize + row;
        let row_area = Rect::new(popup.x, popup.y + y as u16, popup.width, 1);
        let inset = usize::from(popup.width >= 8);
        let mut spans = vec![Span::raw(" ".repeat(inset))];
        // The action name leads here, so labels remain readable at narrow widths.
        spans.extend([Span::styled(hint.label, Style::default().fg(palette::FG)), Span::styled(format!("  {}", hint.key), Theme::dim())]);
        let spans = fit_spans(spans, popup.width as usize);
        let visible_w: usize = spans.iter().map(Span::width).sum();
        add_control(app, row_area, inset, 0, visible_w.saturating_sub(inset), ControlTarget::MoreItem(Box::new(hint.target.clone()), max));
        frame.render_widget(Paragraph::new(Line::from(spans)), row_area);
    }
}

fn render_close(frame: &mut Frame, area: Rect, app: &App, target: ControlTarget) {
    if area.is_empty() {
        return;
    }
    let label = if area.width >= 5 { " [×]" } else { "×" };
    let width = UnicodeWidthStr::width(label);
    let x = area.width as usize - width;
    let rect = Rect::new(area.x + x as u16, area.y, width as u16, 1);
    frame.render_widget(Paragraph::new(Span::styled(label, Theme::dim())), rect);
    add_control(app, area, x + usize::from(label.starts_with(' ')), 0, width, target);
}

fn render_command_palette(frame: &mut Frame, app: &App) {
    let overlay = match app.ui.overlay.as_ref() {
        Some(o) if o.kind == OverlayKind::Command => o,
        _ => return,
    };
    let pal = match overlay.palette.as_ref() {
        Some(p) if !p.filtered.is_empty() => p,
        _ => return,
    };

    let area = frame.area();

    // Cap visible items to terminal height and list size
    let max_visible = 10usize.min(pal.filtered.len()).min(area.height.saturating_sub(7) as usize);
    if max_visible == 0 {
        return;
    }

    let popup_h = max_visible as u16 + 2; // +2 for border
    let popup_w = 52u16.min(area.width.saturating_sub(2));

    // Anchor above input bar (3 rows) + footer (1 row) = 4 from bottom
    let popup_y = area.height.saturating_sub(4 + popup_h);
    let popup_x = 1u16.min(area.width.saturating_sub(popup_w));
    let popup = Rect::new(area.x + popup_x, area.y + popup_y, popup_w, popup_h);
    if popup.width < 3 {
        return;
    }
    app.mouse.add(popup, Target::Control(ControlTarget::PaletteScroll));
    frame.render_widget(Clear, popup);

    // Scroll window: keep cursor visible
    let scroll_offset = match pal.cursor {
        Some(c) if c >= max_visible => c - max_visible + 1,
        _ => 0,
    };

    let inner_w = popup_w.saturating_sub(2) as usize; // inside border
    let marker_w = 2.min(inner_w.saturating_sub(1));
    let cmd_col = 26usize.min(inner_w.saturating_sub(marker_w));
    let desc_col = inner_w.saturating_sub(marker_w + cmd_col + 2);

    let mut lines: Vec<Line> = Vec::with_capacity(max_visible);
    for (vi, &fi) in pal.filtered.iter().skip(scroll_offset).take(max_visible).enumerate() {
        let sg = &pal.all[fi];
        let idx = scroll_offset + vi;
        let is_sel = pal.cursor == Some(idx);

        let marker = truncate_display(if is_sel { "❯ " } else { "  " }, marker_w);
        let marker_style = if is_sel { Style::default().fg(palette::CYAN) } else { Style::default() };
        let cmd_style = if is_sel { Style::default().fg(palette::FG).add_modifier(Modifier::BOLD) } else { Style::default().fg(palette::FG_MID) };
        let desc_style = Style::default().fg(palette::FG_DIM);

        let cmd_text = truncate_display(&sg.command, cmd_col);
        let pad = cmd_col.saturating_sub(UnicodeWidthStr::width(cmd_text.as_str()));
        let desc_text = truncate_display(sg.description, desc_col);

        let width = marker_w + UnicodeWidthStr::width(cmd_text.as_str()) + if desc_text.is_empty() { 0 } else { pad + 2 + UnicodeWidthStr::width(desc_text.as_str()) };
        let inner = Rect::new(popup.x + 1, popup.y + 1, popup.width - 2, popup.height - 2);
        add_control(app, inner, 0, vi, width, ControlTarget::PaletteRow(sg.command.clone()));
        lines.push(Line::from(vec![Span::styled(marker, marker_style), Span::styled(cmd_text, cmd_style), Span::raw(" ".repeat(pad + 2)), Span::styled(desc_text, desc_style)]));
    }

    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(palette::MAGENTA));

    frame.render_widget(Paragraph::new(lines).block(block), popup);
}

fn render_confirm(frame: &mut Frame, app: &App, confirm: &Confirm) {
    let area = frame.area();
    let w = (UnicodeWidthStr::width(confirm.text.as_str()) + 8).clamp(24, 50).min(area.width as usize) as u16;
    let popup = centered_popup(area, w, 7);
    if popup.is_empty() {
        return;
    }
    frame.render_widget(Clear, popup);
    let is_orphan = matches!(confirm.action, ConfirmAction::OrphanAction(_));
    let destructive = matches!(confirm.action, ConfirmAction::KillAgents(_) | ConfirmAction::KillOrphan(_) | ConfirmAction::Reset(_));
    let accent = if destructive { palette::RED } else { palette::BLUE };

    let title = match confirm.action {
        ConfirmAction::KillAgents(ref names) if names.len() > 1 => " Batch Kill ",
        ConfirmAction::KillAgents(_) | ConfirmAction::KillOrphan(_) => " Kill ",
        ConfirmAction::ForkAgents(ref names) if names.len() > 1 => " Batch Fork ",
        ConfirmAction::ForkAgents(_) => " Fork ",
        ConfirmAction::ResumeAgents(ref names) if names.len() > 1 => " Batch Resume ",
        ConfirmAction::ResumeAgents(_) => " Resume ",
        ConfirmAction::OrphanAction(_) => " Orphan ",
        ConfirmAction::Reset(_) => " Reset ",
    };
    let bordered = popup.width >= 4 && popup.height >= 3;
    let block = popup_block(title, accent, bordered);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    if bordered {
        let header = Rect::new(inner.x, popup.y, inner.width, 1);
        render_close(frame, header, app, ControlTarget::CancelConfirm);
    }
    let [details, choices] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);
    if !details.is_empty() {
        let label = format!("  {}", confirm.text);
        let details_row = Rect::new(details.x, details.y + u16::from(details.height > 1), details.width, 1);
        frame.render_widget(Paragraph::new(Span::styled(truncate_display(&label, details.width as usize), Style::default().fg(palette::FG))), details_row);
    }

    let (left_label, right_label, left_accent, right_accent) = if is_orphan { ("Kill", "Recover", palette::RED, palette::GREEN) } else { ("No", "Yes", palette::FG, accent) };
    let [left, right] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(choices);
    for (choice, label, selected_accent, rect) in [(false, left_label, left_accent, left), (true, right_label, right_accent, right)] {
        let selected = confirm.selected == choice;
        let style = if selected { Style::default().fg(selected_accent).add_modifier(Modifier::BOLD) } else { Theme::dim() };
        let label = if rect.width > UnicodeWidthStr::width(label) as u16 + 2 { format!("{}{}", if selected { "❯ " } else { "  " }, label) } else { label.to_string() };
        let label = truncate_display(&label, rect.width as usize);
        let w = UnicodeWidthStr::width(label.as_str());
        let offset = (rect.width as usize - w) / 2;
        add_control(app, rect, offset, 0, w, ControlTarget::ConfirmChoice(choice));
        frame.render_widget(Paragraph::new(Span::styled(label, style)).centered(), rect);
    }
}

fn centered_popup(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height)
}

fn popup_block(title: &'static str, accent: Color, bordered: bool) -> Block<'static> {
    if bordered { Block::default().title(Span::styled(title, Style::default().fg(accent))).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(accent)) } else { Block::default() }
}

fn render_relay_popup(frame: &mut Frame, app: &App, relay: &RelayPopupState) {
    let mut area = frame.area();
    area.height = area.height.saturating_sub(1);
    let popup = centered_popup(area, 34, if relay.editing_token { 5 } else { 8 });
    if popup.is_empty() {
        return;
    }
    app.mouse.add(popup, Target::Control(ControlTarget::RelayScroll));
    frame.render_widget(Clear, popup);
    let bordered = popup.width >= 4 && popup.height >= 3;
    let block = popup_block(" relay ", palette::CYAN, bordered);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    if bordered {
        let header = Rect::new(inner.x, popup.y, inner.width, 1);
        render_close(frame, header, app, shortcut(ShortcutScope::Relay, KeyCode::Esc));
    }

    if relay.editing_token {
        let row = Rect::new(inner.x, inner.y + u16::from(inner.height > 1), inner.width, inner.height.min(1));
        let prefix = vec![Span::raw("  "), Span::styled("❯ ", Theme::cursor())];
        render_text_field(frame, row, app, prefix, TextField { field: Field::RelayToken, text: &relay.token_input, cursor: relay.token_cursor, placeholder: "paste token…", show_cursor: app.control_overflow.is_none() });
        return;
    }

    let (toggle_label, toggle_icon, toggle_color) = if relay.toggling {
        ("...", "○", palette::YELLOW)
    } else if relay.enabled {
        ("enabled", "●", palette::GREEN)
    } else {
        ("disabled", "○", palette::FG_DIM)
    };
    let toggle = format!("{toggle_icon} {toggle_label}");
    let actions = std::iter::once(toggle.as_str()).chain(RELAY_ACTIONS.iter().copied());
    let count = RELAY_ACTIONS.len() + 1;
    let visible = count.min(inner.height as usize);
    let offset = (relay.cursor as usize).saturating_sub(visible.saturating_sub(1)).min(count - visible);
    let pad_y = usize::from(inner.height as usize > count);
    for (index, action) in actions.enumerate().skip(offset).take(visible) {
        let selected = relay.cursor as usize == index;
        let color = if index == 0 {
            toggle_color
        } else if selected {
            palette::CYAN
        } else {
            palette::FG
        };
        let style = if selected { Style::default().fg(color).add_modifier(Modifier::BOLD) } else { Style::default().fg(color) };
        let marker = if inner.width >= 8 { if selected { "  ❯ " } else { "    " } } else { "" };
        let spans = fit_spans(vec![Span::styled(marker, Theme::cursor()), Span::styled(action.to_owned(), style)], inner.width as usize);
        let width = spans.iter().map(Span::width).sum::<usize>();
        let y = index - offset + pad_y;
        if index != 0 || !relay.toggling {
            add_control(app, inner, 0, y, width, ControlTarget::RelayRow(index as u8));
        }
        let row = Rect::new(inner.x, inner.y + y as u16, inner.width, 1);
        frame.render_widget(Paragraph::new(Line::from(spans)), row);
    }
}

fn render_help(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let key = |k: &str| Span::styled(format!("  {:<12}", k), Style::default().fg(palette::FG));
    let desc = |d: &str| Span::styled(d.to_string(), Theme::dim());
    let section = |title: &str| Line::from(Span::styled(format!("  {}", title), Style::default().fg(palette::FG_MID)));
    let help_lines = vec![
        Line::raw(""),
        section("Navigate"),
        Line::from(vec![key("\u{2191}\u{2193} / \u{2190}\u{2192}"), desc("move cursor")]),
        Line::from(vec![key("enter/space"), desc("select + filter scrollback")]),
        Line::from(vec![key("t"), desc("open or close transcript")]),
        Line::from(vec![key("i"), desc("open or close Agent Info")]),
        Line::from(vec![key("a"), desc("select all")]),
        Line::from(vec![key("b"), desc("broadcast to all")]),
        Line::from(vec![key("ctrl+r"), desc("relay settings")]),
        Line::from(vec![key("ctrl+s"), desc("all stopped agents")]),
        Line::from(vec![key("ctrl+d"), desc("quit")]),
        Line::raw(""),
        section("Compose"),
        Line::from(vec![key("ctrl+t"), desc("read while keeping draft")]),
        Line::from(vec![key("enter"), desc("send message")]),
        Line::from(vec![key("esc"), desc("cancel")]),
        Line::raw(""),
        section("Mouse"),
        Line::from(Span::styled("  Hold Shift for native selection/copy.", Theme::dim())),
        Line::from(Span::styled("  Some terminals use another modifier.", Theme::dim())),
        Line::raw(""),
    ];

    let total = help_lines.len() as u16;
    // Size popup to content (+ 2 for border), clamped to terminal
    let w = 44u16.min(area.width);
    let h = (total + 2).min(area.height);
    if w == 0 || h == 0 {
        return;
    }
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect::new(x, y, w, h);

    frame.render_widget(Clear, popup);

    let inner_h = h.saturating_sub(2);
    let max_scroll = total.saturating_sub(inner_h);
    let scroll_pos = app.ui.help_scroll.min(max_scroll);
    app.mouse.add(popup, Target::Control(ControlTarget::HelpScroll { max: max_scroll }));

    let block = Block::default().title(Span::styled(" Keyboard ", Theme::title())).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Theme::separator());

    let para = Paragraph::new(help_lines).block(block).scroll((scroll_pos, 0));
    frame.render_widget(para, popup);
    render_close(frame, Rect::new(popup.x, popup.y, popup.width.saturating_sub(1).max(1), 1), app, ControlTarget::CloseHelp);

    if inner_h > 0 && total > inner_h {
        // Render scrollbar inside the border
        let sb_area = Rect::new(popup.x + popup.width - 1, popup.y + 1, 1, inner_h);
        let mut state = ScrollbarState::new(max_scroll as usize).position(scroll_pos as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight).begin_symbol(None).end_symbol(None).track_symbol(Some("\u{2502}")).track_style(Theme::separator()).thumb_style(Style::default().fg(palette::FG_DIM)),
            sb_area,
            &mut state,
        );
    }
}
