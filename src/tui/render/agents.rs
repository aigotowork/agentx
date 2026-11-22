use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::tui::agent_mouse::{self, AgentTarget, AgentWindow, Item, ItemKey, Section};
use crate::tui::app::App;
use crate::tui::model::*;
use crate::tui::mouse::Target;
use crate::tui::theme::{Theme, palette};

/// Shorten a directory path by replacing the home directory prefix with `~`.
fn shorten_dir(path: &str) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        let home = home.to_string_lossy();
        if let Some(rest) = path.strip_prefix(home.as_ref()) {
            return format!("~{}", rest);
        }
    }
    path.to_string()
}

/// Indicator color for tab strip entries (blocked > unread > none).
fn tab_indicator_color(agent: &Agent) -> Option<ratatui::style::Color> {
    if agent.is_pty_blocked() {
        Some(palette::ORANGE)
    } else if agent.unread > 0 {
        Some(palette::YELLOW)
    } else {
        None
    }
}

const FOCUSED_CHECKED: Color = Color::Rgb(55, 70, 135);
const CHECKED: Color = Color::Rgb(45, 60, 120);

fn agent_background(focused: bool, checked: bool) -> Option<Color> {
    match (focused, checked) {
        (true, true) => Some(FOCUSED_CHECKED),
        (true, false) => Some(palette::SELECTION),
        (false, true) => Some(CHECKED),
        (false, false) => None,
    }
}

/// Agent name style: bold blue if selected, normal bold if not.
fn selected_name_style(is_selected: bool) -> Style {
    if is_selected { Style::default().fg(palette::BLUE).add_modifier(Modifier::BOLD) } else { Theme::agent_name() }
}

const SPINNER_FRAMES: &[&str] = &["\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}", "\u{2826}", "\u{2827}", "\u{2807}", "\u{280f}"];

/// Pad left spans and append right spans so the result fills `width`.
fn append_right_aligned(left: &mut Vec<Span<'static>>, right: Vec<Span<'static>>, width: u16) -> usize {
    let right = super::fit_spans(right, width as usize);
    let right_w: usize = right.iter().map(|s| s.width()).sum();
    *left = super::fit_spans(std::mem::take(left), (width as usize).saturating_sub(right_w));
    let left_w: usize = left.iter().map(|s| s.width()).sum();
    let pad = (width as usize).saturating_sub(left_w + right_w);
    left.push(Span::raw(" ".repeat(pad)));
    left.extend(right);
    left_w
}

/// Resolve status icon, handling Launching spinner and Adhoc neutral dot.
fn agent_icon(agent: &Agent, tick: u64) -> &'static str {
    if agent.status == AgentStatus::Launching {
        SPINNER_FRAMES[(tick as usize / 2) % SPINNER_FRAMES.len()]
    } else if agent.status != AgentStatus::Listening
        && let Some(icon) = agent.tool.spec().and_then(|spec| spec.adhoc_icon)
    {
        icon
    } else {
        agent.status.icon()
    }
}

/// Compact tool prefix for multi-tool display.
fn tool_prefix_str(tool: &AgentTool) -> String {
    match tool {
        AgentTool::Unknown(raw) => raw.chars().take(4).collect(),
        AgentTool::Known(tool) => tool.spec().tui_prefix.to_string(),
    }
}

#[derive(Clone)]
struct AgentRow {
    line: Line<'static>,
    target: AgentTarget,
    marker: Option<u16>,
}

impl AgentRow {
    fn new(line: Line<'static>, key: ItemKey) -> Self {
        Self { line, target: AgentTarget::Focus(key), marker: None }
    }

    fn selectable((line, left_width): (Line<'static>, usize), key: ItemKey, marker: u16) -> Self {
        Self { marker: (left_width > usize::from(marker)).then_some(marker), ..Self::new(line, key) }
    }
}

fn collect_agent_lines(app: &App, width: u16, height: usize) -> (Vec<AgentRow>, AgentWindow) {
    let all_agents = app.data.agents.iter().chain(&app.data.remote_agents).chain(&app.data.stopped_agents);
    let tools: std::collections::HashSet<&str> = all_agents.clone().map(|a| a.tool.name()).collect();
    let multi_tool = tools.len() > 1;
    let name_width = all_agents.map(|a| UnicodeWidthStr::width(a.display_name().as_str())).max().unwrap_or(4).max(4);

    let mut rows = Vec::new();
    let mut live_count = 0;
    for (i, item) in agent_mouse::items(app).enumerate() {
        let cursor = app.ui.cursor == i;
        let key = item.key();
        let row = match item {
            Item::Live(agent) => {
                let selected = app.ui.selected.contains(&agent.action_name());
                rows.push(AgentRow::selectable(compact_agent_line(agent, cursor, selected, width, app.ui.tick, multi_tool), key.clone(), LIVE_MARKER));
                live_count += 1;
                if cursor && selected {
                    live_count += 1;
                    AgentRow::new(detail_line_info(agent, width), key)
                } else {
                    continue;
                }
            }
            Item::Remote(agent) => {
                let selected = app.ui.selected.contains(&agent.action_name());
                AgentRow::selectable(remote_agent_line(agent, cursor, selected, width, app.ui.tick, multi_tool), key, NESTED_MARKER)
            }
            Item::Stopped(agent) => {
                let selected = app.ui.selected.contains(&agent.action_name());
                rows.push(AgentRow::selectable(stopped_agent_line(agent, cursor, selected, width, multi_tool, name_width), key.clone(), NESTED_MARKER));
                if cursor && selected {
                    AgentRow::new(stopped_detail_line(agent, width), key)
                } else {
                    continue;
                }
            }
            Item::Orphan(orphan) => AgentRow::new(orphan_line(orphan, cursor, width), key),
            Item::Header(section) => {
                let (count, label, expanded, accent) = match section {
                    Section::Remote => (app.data.remote_agents.len(), "remote", app.ui.remote_expanded, palette::CYAN),
                    Section::Stopped => (app.data.stopped_agents.len(), "stopped", app.ui.stopped_expanded, palette::FG_DIM),
                    Section::Orphans => (app.data.orphans.len(), "orphans", app.ui.orphans_expanded, palette::RED),
                };
                AgentRow::new(collapsible_header(count, label, expanded, cursor, width, accent), key)
            }
        };
        rows.push(row);
    }

    let max_start = list_max_start(rows.len(), height);
    let mut visible = Vec::new();
    let start = if let Some(manual_start) = app.mouse.agent_scroll.get() {
        let start = manual_start.min(max_start);
        append_list_window(&mut visible, &rows, height, 0, start);
        start
    } else {
        // Keyboard navigation keeps both live and other sections in view.
        let (live, sections) = rows.split_at(live_count);
        let section_focus = app.ui.cursor >= app.data.agents.len();
        let section_budget = if live.is_empty() { height } else { height / 2 + usize::from(height == 1 && section_focus) }.min(sections.len());
        let live_budget = height.saturating_sub(section_budget);
        let live_start = follow_cursor(live, live_budget);
        append_list_window(&mut visible, live, live_budget, 0, live_start);
        let section_height = height.saturating_sub(visible.len());
        let section_start = follow_cursor(sections, section_height);
        append_list_window(&mut visible, sections, section_height, live_count, section_start);
        if section_focus { live_count + section_start } else { live_start }
    }
    .min(max_start);

    let window = AgentWindow { prev: start.checked_sub(1), next: (start < max_start).then_some(start + 1) };
    (visible, window)
}

const LIVE_MARKER: u16 = 0;
const NESTED_MARKER: u16 = 4;

fn list_capacity(total: usize, height: usize) -> usize {
    if total <= height || height == 0 { height } else { height.saturating_sub(2).max(1) }
}

fn list_max_start(total: usize, height: usize) -> usize {
    total.saturating_sub(list_capacity(total, height))
}

fn follow_cursor(rows: &[AgentRow], height: usize) -> usize {
    let cursor = rows.iter().position(|row| matches!(row.line.style.bg, Some(palette::SELECTION | FOCUSED_CHECKED))).unwrap_or(0);
    let count = list_capacity(rows.len(), height);
    cursor.saturating_sub(count / 2).min(list_max_start(rows.len(), height))
}

fn append_list_window(visible: &mut Vec<AgentRow>, rows: &[AgentRow], height: usize, base: usize, start: usize) {
    if height == 0 {
        return;
    }
    let end = (start + list_capacity(rows.len(), height)).min(rows.len());
    let overflow = |line: String, start| AgentRow { line: Line::from(Span::styled(line, Theme::dim())), target: AgentTarget::ScrollTo(start), marker: None };
    let top = visible.len();
    if start > 0 && height > 1 {
        visible.push(overflow(format!("  ↑ {start} more"), base + start - 1));
    }
    visible.extend_from_slice(&rows[start..end]);
    if end < rows.len() && visible.len() - top < height {
        visible.push(overflow(format!("  ↓ {} more", rows.len() - end), base + start + 1));
    }
}

/// Truncate spans to fit within max_width, appending '…' if truncated. Pads to fill width.
fn finalize_detail_spans(parts: &mut Vec<Span<'static>>, width: u16) {
    let max = width as usize;
    let mut used: usize = 0;
    for i in 0..parts.len() {
        let sw = parts[i].width();
        if used + sw > max.saturating_sub(1) {
            // This span would overflow — truncate here, add ellipsis
            parts.truncate(i);
            parts.push(Span::styled("\u{2026}", Theme::dim()));
            break;
        }
        used += sw;
    }
    let total: usize = parts.iter().map(|s| s.width()).sum();
    let pad = max.saturating_sub(total);
    parts.push(Span::raw(" ".repeat(pad)));
}

fn detail_line_info(agent: &Agent, width: u16) -> Line<'static> {
    let mut parts: Vec<Span> = vec![
        Span::raw("    "),
        Span::styled("\u{251c} ", Theme::separator()),
        Span::styled(agent.tool.name().to_string(), Theme::dim()),
        Span::styled(" \u{00b7} ", Theme::separator()),
        Span::styled(shorten_dir(&agent.directory), Theme::dim()),
        Span::styled(" \u{00b7} ", Theme::separator()),
        Span::styled(format!("created {}", agent.created_display()), Theme::dim()),
    ];
    if agent.headless {
        parts.push(Span::styled(" \u{00b7} ", Theme::separator()));
        parts.push(Span::styled("[headless]", Theme::dim()));
    }
    if agent.unread > 0 {
        parts.push(Span::styled(" \u{00b7} ", Theme::separator()));
        parts.push(Span::styled(format!("{} unread", agent.unread), Style::default().fg(palette::YELLOW)));
    }
    if let Some(ref sid) = agent.session_id {
        parts.push(Span::styled(" \u{00b7} ", Theme::separator()));
        let short: String = sid.chars().take(6).collect();
        parts.push(Span::styled(format!("session:{}", short), Theme::dim()));
    }
    if let Some(pid) = agent.pid {
        parts.push(Span::styled(" \u{00b7} ", Theme::separator()));
        parts.push(Span::styled(format!("pid:{}", pid), Theme::dim()));
    }
    if agent.is_pty_blocked() {
        parts.push(Span::styled(" \u{00b7} ", Theme::separator()));
        parts.push(Span::styled("\u{2298} ", Theme::pty_block())); // ⊘
        parts.push(Span::styled(agent.context_display(), Theme::pty_block()));
    }
    finalize_detail_spans(&mut parts, width);
    Line::from(parts).style(Style::default().bg(palette::SELECTION))
}

/// Detail line for stopped agents: exit context, directory, age.
fn stopped_detail_line(agent: &Agent, width: u16) -> Line<'static> {
    let mut parts: Vec<Span> =
        vec![Span::raw("      "), Span::styled("\u{2570} ", Theme::separator()), Span::styled(agent.tool.name().to_string(), Theme::dim()), Span::styled(" \u{00b7} ", Theme::separator()), Span::styled(agent.directory.clone(), Theme::dim())];
    if !agent.status_context.is_empty() {
        parts.push(Span::styled(" \u{00b7} ", Theme::separator()));
        parts.push(Span::styled(agent.context_display(), Theme::dim()));
    }
    finalize_detail_spans(&mut parts, width);
    Line::from(parts).style(Style::default().bg(palette::SELECTION))
}

// ── Collapsible sections ─────────────────────────────────────────

fn collapsible_header(count: usize, label: &str, expanded: bool, is_cursor: bool, width: u16, accent: ratatui::style::Color) -> Line<'static> {
    let arrow = if expanded { "\u{25be}" } else { "\u{25b8}" }; // ▾ / ▸
    let cursor_str = if is_cursor { "\u{276f} " } else { "  " };
    let cursor_style = if is_cursor { Theme::cursor() } else { Style::default() };
    let text = format!("{} {} {}", arrow, count, label);

    let mut spans = vec![Span::raw("  "), Span::styled(cursor_str.to_string(), cursor_style), Span::styled(text, Style::default().fg(accent))];

    let used: usize = spans.iter().map(|s| s.width()).sum();
    let pad = (width as usize).saturating_sub(used);
    spans.push(Span::raw(" ".repeat(pad)));

    let mut line = Line::from(spans);
    if is_cursor {
        line = line.style(Style::default().bg(palette::SELECTION));
    }
    line
}

fn stopped_agent_line(agent: &Agent, is_cursor: bool, is_selected: bool, width: u16, multi_tool: bool, name_width: usize) -> (Line<'static>, usize) {
    let name_style = Style::default().fg(palette::FG_DIM);

    let dim = Style::default().fg(palette::FG_DARK);
    let display = agent.display_name();
    let display_w = UnicodeWidthStr::width(display.as_str());
    let pad_n = name_width + 2 - display_w.min(name_width + 2);
    let name_padded = format!("{}{}", display, " ".repeat(pad_n));
    let age = agent.age_display();

    let mut left: Vec<Span> = vec![Span::raw("    ")];
    left.push(Span::styled("\u{25cb} ", Style::default().fg(palette::FG_DIM)));
    if multi_tool {
        left.push(Span::styled(tool_prefix_str(&agent.tool), dim));
    }
    left.extend([Span::styled(name_padded, name_style), Span::styled(agent.context_display(), dim)]);

    // Right: age
    let right = vec![Span::styled(format!("{:>3}", age), dim), Span::raw("  ")];
    let left_width = append_right_aligned(&mut left, right, width);

    let mut line = Line::from(left);
    if let Some(bg) = agent_background(is_cursor, is_selected) {
        line = line.style(Style::default().bg(bg));
    }
    (line, left_width)
}

fn orphan_line(orphan: &OrphanProcess, is_cursor: bool, width: u16) -> Line<'static> {
    let cursor_str = if is_cursor { "\u{276f} " } else { "  " };
    let cursor_style = if is_cursor { Theme::cursor() } else { Style::default() };
    let age = orphan.age_display();

    let mut left: Vec<Span> = vec![
        Span::raw("    "), // extra indent under header
        Span::styled(cursor_str.to_string(), cursor_style),
        Span::styled("\u{2716} ", Style::default().fg(palette::RED)), // ✖
        Span::styled(tool_prefix_str(&orphan.tool), Theme::dim()),
        Span::styled(format!("{:<6}", orphan.pid), Style::default().fg(palette::FG_DIM)),
        Span::styled(orphan.names_display(), Style::default().fg(palette::FG_DARK)),
    ];

    let right: Vec<Span> = vec![Span::styled(format!("{:>3}", age), Style::default().fg(palette::FG_DARK)), Span::raw("  ")];

    append_right_aligned(&mut left, right, width);

    let mut line = Line::from(left);
    if is_cursor {
        line = line.style(Style::default().bg(palette::SELECTION));
    }
    line
}

fn remote_agent_line(ragent: &Agent, is_cursor: bool, is_selected: bool, width: u16, tick: u64, multi_tool: bool) -> (Line<'static>, usize) {
    let icon = agent_icon(ragent, tick);
    let name_style = Theme::agent_name();

    let display_name = ragent.display_name();
    let age = ragent.age_display();

    let mut left: Vec<Span> = vec![Span::raw("    ")];
    left.extend([Span::styled(icon, ragent.status_style()), Span::raw(" ")]);
    if multi_tool {
        left.push(Span::styled(tool_prefix_str(&ragent.tool), Theme::dim()));
    }
    left.extend([Span::styled(display_name, name_style), Span::raw("  "), Span::styled(ragent.context_display(), Theme::agent_context())]);

    // Right: sync age + agent age
    let sync = ragent.sync_age.as_deref().unwrap_or("");
    let right: Vec<Span> = vec![Span::styled(format!("{}  ", sync), Style::default().fg(palette::FG_DARK)), Span::styled(format!("{:>3}", age), Theme::agent_dim()), Span::raw("  ")];

    let left_width = append_right_aligned(&mut left, right, width);

    let mut line = Line::from(left);
    if let Some(bg) = agent_background(is_cursor, is_selected) {
        line = line.style(Style::default().bg(bg));
    }
    (line, left_width)
}

/// Compact agent list for vertical (side-by-side) view.
pub fn render_agents_compact(frame: &mut Frame, area: Rect, app: &App) {
    app.mouse.add(area, Target::AgentsArea);
    let (rows, window) = collect_agent_lines(app, area.width, area.height as usize);
    app.mouse.agent_window.set(window);

    let buf = frame.buffer_mut();
    for (i, entry) in rows.into_iter().enumerate() {
        let row = Rect::new(area.x, area.y + i as u16, area.width, 1);
        entry.line.render(row, buf);
        app.mouse.add(row, Target::Agent(entry.target.clone()));
        if let (Some(offset), AgentTarget::Focus(key)) = (entry.marker, entry.target) {
            app.mouse.add(Rect::new(row.x + offset, row.y, 1, 1), Target::Agent(AgentTarget::Select(key)));
        }
    }
}

/// Compact line for vertical view
fn compact_agent_line(agent: &Agent, is_cursor: bool, is_selected: bool, width: u16, tick: u64, multi_tool: bool) -> (Line<'static>, usize) {
    let icon = agent_icon(agent, tick);

    let name_style = Theme::agent_name();

    let age = agent.age_display();

    let mut spans = vec![Span::styled(icon, agent.status_style()), Span::raw(" ")];
    if multi_tool {
        spans.push(Span::styled(tool_prefix_str(&agent.tool), Theme::dim()));
    }
    spans.push(Span::styled(agent.display_name(), name_style));

    // Right-align unread count + age (fixed-width slots)
    let mut right_spans: Vec<Span> = Vec::new();
    if agent.unread > 0 {
        right_spans.push(Span::styled(format!("{:>2}", agent.unread), Style::default().fg(palette::YELLOW)));
    } else {
        right_spans.push(Span::raw("  "));
    }
    right_spans.push(Span::styled(format!(" {:>3}", age), Theme::agent_dim()));
    right_spans.push(Span::raw(" "));
    let left_width = append_right_aligned(&mut spans, right_spans, width);

    let mut line = Line::from(spans);
    if let Some(bg) = agent_background(is_cursor, is_selected) {
        line = line.style(Style::default().bg(bg));
    }
    (line, left_width)
}

// ══════════════════════════════════════════════════════════════════════
// Tab Strip layout — alternative inline rendering
// ══════════════════════════════════════════════════════════════════════

pub fn render_agents_tabstrip(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width < 10 {
        return;
    }

    let width = area.width;

    // Tab strip: 1 row
    app.mouse.add(area, Target::AgentsArea);
    let tab_area = Rect::new(area.x, area.y, width, 1);
    let tab_line = build_tab_strip(app, tab_area);
    frame.render_widget(Paragraph::new(tab_line), tab_area);

    // Thin separator: 1 row
    if area.height >= 3 {
        let sep = "\u{254c}".repeat(width as usize);
        let sep_area = Rect::new(area.x, area.y + 1, width, 1);
        frame.render_widget(Paragraph::new(Line::from(Span::styled(sep, Theme::separator()))), sep_area);
    }

    // Detail pane: remaining rows
    let detail_start = area.y + 2;
    let detail_height = area.height.saturating_sub(2);
    if detail_height == 0 {
        return;
    }
    let detail_area = Rect::new(area.x, detail_start, width, detail_height);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let ct = app.cursor_target();

    match ct {
        CursorTarget::Agent(idx) => {
            build_agent_detail(&app.data.agents[idx], app, &mut lines, width as usize);
        }
        CursorTarget::RemoteAgent(idx) => {
            build_agent_detail(&app.data.remote_agents[idx], app, &mut lines, width as usize);
        }
        CursorTarget::RemoteHeader => {
            lines.push(Line::from(Span::styled(format!("  {} remote agents", app.data.remote_agents.len()), Style::default().fg(palette::CYAN))));
        }
        CursorTarget::OrphanHeader => {
            lines.push(Line::from(Span::styled(format!("  {} orphan processes", app.data.orphans.len()), Style::default().fg(palette::RED))));
        }
        CursorTarget::Orphan(idx) => {
            build_orphan_detail(&app.data.orphans[idx], &mut lines, width as usize);
        }
        _ => {
            lines.push(Line::from(Span::styled("  No agent selected", Theme::dim())));
        }
    }

    // Pad to fill detail area
    while lines.len() < detail_height as usize {
        lines.push(Line::raw(""));
    }

    frame.render_widget(Paragraph::new(lines), detail_area);
}

struct TabEntry {
    text: String,
    icon: Option<&'static str>,
    style: Style,
    key: ItemKey,
    is_cursor: bool,
    is_selected: bool,
    selectable: bool,
    indicator: Option<Color>,
}

impl TabEntry {
    fn from_item(app: &App, item: Item<'_>, is_cursor: bool) -> Self {
        let mut tab = Self { text: String::new(), icon: None, style: Style::default(), key: item.key(), is_cursor, is_selected: false, selectable: false, indicator: None };
        match item {
            Item::Live(agent) | Item::Remote(agent) | Item::Stopped(agent) => {
                tab.icon = Some(agent_icon(agent, app.ui.tick));
                tab.text = agent.display_name();
                tab.style = agent.status_style();
                tab.is_selected = app.ui.selected.contains(&agent.action_name());
                tab.selectable = true;
                tab.indicator = tab_indicator_color(agent);
            }
            Item::Header(section) => {
                let (expanded, label, accent) = match section {
                    Section::Remote => (app.ui.remote_expanded, format!("{}r", app.data.remote_agents.len()), palette::CYAN),
                    Section::Orphans => (app.ui.orphans_expanded, format!("{} orphan", app.data.orphans.len()), palette::RED),
                    Section::Stopped => (app.ui.stopped_expanded, format!("{} stopped", app.data.stopped_agents.len()), palette::FG_DIM),
                };
                tab.text = format!("{} {label}", if expanded { "▾" } else { "▸" });
                tab.style = Style::default().fg(accent);
            }
            Item::Orphan(orphan) => {
                tab.text = format!("✖{}", orphan.pid);
                tab.style = Style::default().fg(palette::RED);
            }
        }
        tab
    }

    fn spans(&self) -> Vec<Span<'static>> {
        let bg = agent_background(self.is_cursor, self.is_selected).or(match self.indicator {
            Some(palette::ORANGE) => Some(Color::Rgb(48, 28, 8)),
            Some(_) => Some(Color::Rgb(46, 42, 14)),
            None => None,
        });
        let style = bg.map_or(self.style, |color| self.style.bg(color));
        if let Some(icon) = self.icon { vec![Span::styled(icon, style), Span::styled(format!(" {}", self.text), style)] } else { vec![Span::styled(format!(" {} ", self.text), style)] }
    }
}

const TAB_GAP: usize = 2;

fn build_tab_strip(app: &App, area: Rect) -> Line<'static> {
    let tabs: Vec<_> = agent_mouse::items(app).enumerate().map(|(i, item)| TabEntry::from_item(app, item, app.ui.cursor == i)).collect();
    if tabs.is_empty() {
        app.mouse.agent_window.set(AgentWindow::default());
        return Line::from(Span::styled("  No agents", Theme::dim()));
    }

    let width = usize::from(area.width);
    let widths: Vec<_> = tabs.iter().map(|tab| tab.text.width() + tab.icon.map_or(2, |icon| icon.width() + 1)).collect();
    let show_counts = 2 * (tabs.len().to_string().len() + 2) + 3 <= width;
    let cursor = app.ui.cursor.min(tabs.len() - 1);
    let (start, end) = compute_tab_scroll(&widths, width, cursor, app.mouse.agent_scroll.get(), show_counts);
    let window = AgentWindow { prev: start.checked_sub(1), next: (end < tabs.len()).then_some(start + 1) };
    app.mouse.agent_window.set(window);

    let left = tab_edge(start, false, show_counts);
    let right = tab_edge(tabs.len() - end, true, show_counts);
    let right_width = UnicodeWidthStr::width(right.as_str());
    let mut col = UnicodeWidthStr::width(left.as_str());
    let tab_rect = |col: usize, width: usize| Rect::new(area.x + col as u16, area.y, width as u16, 1);
    if let Some(prev) = window.prev {
        app.mouse.add(tab_rect(0, col), Target::Agent(AgentTarget::ScrollTo(prev)));
    }
    let mut spans = vec![Span::styled(left, Theme::dim())];

    for (i, tab) in tabs[start..end].iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" ".repeat(TAB_GAP)));
            col += TAB_GAP;
        }
        let fitted = super::fit_spans(tab.spans(), width.saturating_sub(col + right_width));
        let cells = fitted.iter().map(Span::width).sum::<usize>();
        spans.extend(fitted);
        app.mouse.add(tab_rect(col, cells), Target::Agent(AgentTarget::Focus(tab.key.clone())));
        if tab.selectable && cells > 0 {
            app.mouse.add(tab_rect(col, 1), Target::Agent(AgentTarget::Select(tab.key.clone())));
        }
        col += cells;
    }

    if let Some(next) = window.next {
        app.mouse.add(tab_rect(col, right_width), Target::Agent(AgentTarget::ScrollTo(next)));
        spans.push(Span::styled(right, Theme::dim()));
    }
    Line::from(spans)
}

/// Return exactly the overflow text that gets painted; its cell width is
/// reserved before allocating space for tabs, including overlong Unicode labels.
fn tab_edge(hidden: usize, right: bool, counts: bool) -> String {
    match (hidden, right, counts) {
        (0, true, _) => String::new(),
        (0, false, _) => " ".into(),
        (_, true, true) => format!(" {hidden}→"),
        (_, false, true) => format!("←{hidden} "),
        (_, true, false) => "→".into(),
        (_, false, false) => "←".into(),
    }
}

fn compute_tab_scroll(widths: &[usize], avail: usize, cursor: usize, manual: Option<usize>, counts: bool) -> (usize, usize) {
    let total = widths.len();
    let all_width = widths.iter().sum::<usize>() + total.saturating_sub(1) * TAB_GAP + 1;
    if all_width <= avail {
        return (0, total);
    }

    let mut start = manual.unwrap_or(cursor).min(total - 1);
    let mut end = start + 1;
    let mut used = widths[start];
    let fits = |start: usize, end: usize, used| {
        let pre = tab_edge(start, false, counts);
        let post = tab_edge(total - end, true, counts);
        used + UnicodeWidthStr::width(pre.as_str()) + UnicodeWidthStr::width(post.as_str()) <= avail
    };
    loop {
        if end < total && fits(start, end + 1, used + TAB_GAP + widths[end]) {
            used += TAB_GAP + widths[end];
            end += 1;
        } else if manual.is_none() && start > 0 && fits(start - 1, end, used + TAB_GAP + widths[start - 1]) {
            start -= 1;
            used += TAB_GAP + widths[start];
        } else {
            break;
        }
    }
    (start, end)
}

fn build_agent_detail(agent: &Agent, app: &App, lines: &mut Vec<Line<'static>>, width: usize) {
    let w = width as u16;
    let is_selected = app.ui.selected.contains(&agent.action_name());

    // Line 1: icon name · tool · age · context                 created Xm
    let icon = agent_icon(agent, app.ui.tick);
    let age = agent.age_display();
    let created = agent.created_display();
    let name_style = selected_name_style(is_selected);

    let mut left: Vec<Span<'static>> = vec![
        Span::raw("  "),
        Span::styled(icon.to_string(), agent.status_style()),
        Span::raw(" "),
        Span::styled(agent.display_name(), name_style),
        Span::styled(" \u{00b7} ", Theme::separator()),
        Span::styled(agent.tool.name().to_string(), Theme::dim()),
        Span::styled(" \u{00b7} ", Theme::separator()),
        Span::styled(age, Theme::agent_dim()),
    ];
    if let Some((context, fresh)) = app.usage.context(agent)
        && left.iter().map(Span::width).sum::<usize>() + 3 + UnicodeWidthStr::width(context.as_str()) + UnicodeWidthStr::width(format!("created {}  ", created).as_str()) <= width
    {
        left.push(Span::styled(" · ", Theme::separator()));
        // Darker until the refresh after selecting this agent lands.
        let style = if fresh { Theme::agent_dim() } else { Theme::separator() };
        left.push(Span::styled(context, style));
    }
    let created_label = format!("created {}", created);
    let right: Vec<Span<'static>> = vec![Span::styled(created_label, Theme::agent_dim()), Span::raw("  ")];
    let right_w: usize = right.iter().map(|s| s.width()).sum();

    // Truncate context to fit within available width; prefix ⊘ when PTY-blocked
    let ctx = agent.context_display();
    let blocked = agent.is_pty_blocked();
    if !ctx.is_empty() || blocked {
        left.push(Span::styled(" \u{00b7} ", Theme::separator()));
        if blocked {
            left.push(Span::styled("\u{2298} ", Theme::pty_block())); // ⊘
        }
        let ctx_style = if blocked { Theme::pty_block() } else { Theme::agent_context() };
        let left_w: usize = left.iter().map(|s| s.width()).sum();
        let avail = (w as usize).saturating_sub(left_w + right_w);
        let ctx_w = UnicodeWidthStr::width(ctx.as_str());
        if avail > 1 && ctx_w > avail {
            let mut truncated = String::new();
            let mut tw = 0;
            for c in ctx.chars() {
                let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                if tw + cw + 1 > avail {
                    break;
                }
                truncated.push(c);
                tw += cw;
            }
            truncated.push('\u{2026}');
            left.push(Span::styled(truncated, ctx_style));
        } else if avail > 0 && !ctx.is_empty() {
            left.push(Span::styled(ctx, ctx_style));
        }
    }

    append_right_aligned(&mut left, right, w);
    lines.push(Line::from(left));

    // Line 2: directory · [headless]/[terminal] · pid · session · unread
    let mut info: Vec<Span<'static>> = vec![Span::raw("  "), Span::styled(shorten_dir(&agent.directory), Theme::dim())];
    if agent.headless {
        info.push(Span::styled(" \u{00b7} ", Theme::separator()));
        info.push(Span::styled("[headless]", Theme::dim()));
    } else if let Some(ref preset) = agent.terminal_preset {
        info.push(Span::styled(" \u{00b7} ", Theme::separator()));
        info.push(Span::styled(format!("[{}]", preset), Theme::dim()));
    }
    if let Some(pid) = agent.pid {
        info.push(Span::styled(" \u{00b7} ", Theme::separator()));
        info.push(Span::styled(format!("pid:{}", pid), Theme::dim()));
    }
    if let Some(ref sid) = agent.session_id {
        info.push(Span::styled(" \u{00b7} ", Theme::separator()));
        let short: String = sid.chars().take(6).collect();
        info.push(Span::styled(format!("session:{}", short), Theme::dim()));
    }
    if agent.unread > 0 {
        info.push(Span::styled(" \u{00b7} ", Theme::separator()));
        info.push(Span::styled(format!("{} unread", agent.unread), Style::default().fg(palette::YELLOW)));
    }
    finalize_detail_spans(&mut info, w);
    lines.push(Line::from(info));

    // Separator line
    let dashes = "\u{254c}".repeat(w.saturating_sub(4) as usize);
    lines.push(Line::from(vec![Span::raw("  "), Span::styled(dashes, Theme::separator())]));
}

fn build_orphan_detail(orphan: &OrphanProcess, lines: &mut Vec<Line<'static>>, _width: usize) {
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("\u{2716} ", Style::default().fg(palette::RED)),
        Span::styled(format!("PID {} ", orphan.pid), Style::default().fg(palette::FG)),
        Span::styled("\u{00b7} ", Theme::separator()),
        Span::styled(orphan.tool.name().to_string(), Theme::dim()),
        Span::styled(" \u{00b7} ", Theme::separator()),
        Span::styled(orphan.names_display(), Theme::dim()),
    ]));

    lines.push(Line::from(vec![Span::raw("  "), Span::styled(orphan.directory.clone(), Theme::dim()), Span::styled(" \u{00b7} ", Theme::separator()), Span::styled(orphan.age_display(), Theme::dim())]));
}

#[cfg(test)]
mod usage_tests {
    use super::*;

    #[test]
    fn detail_selection_uses_action_identity() {
        crate::config::Config::init();
        let mut app = crate::tui::test_helpers::offline_app();
        let mut agent = crate::tui::test_helpers::make_test_agent("nova", 0.0);
        agent.device_name = Some("BOX".into());
        for (selected, expected) in [("nova", false), ("nova:BOX", true)] {
            app.ui.selected.clear();
            app.ui.selected.insert(selected.into());
            let mut lines = Vec::new();
            build_agent_detail(&agent, &app, &mut lines, 100);
            let name = lines[0].spans.iter().find(|span| span.content == "nova:BOX").unwrap();
            assert_eq!(name.style, selected_name_style(expected));
        }
    }

    #[test]
    fn right_badge_survives_long_unicode_middle_fields() {
        for width in 5..30 {
            let mut left = vec![Span::raw("  nova "), Span::raw("界".repeat(100))];
            append_right_aligned(&mut left, vec![Span::raw(" 12m ")], width);
            let line = Line::from(left);
            assert!(line.width() <= width as usize);
            assert!(line.to_string().ends_with(" 12m "));
        }
    }

    #[test]
    fn scrolling_keeps_every_cursor_visible_and_counts_hidden_rows() {
        for height in 1..12 {
            for cursor in 0..20 {
                let rows = (0..20)
                    .map(|i| {
                        let line = Line::raw(format!("agent {i}"));
                        let line = if i == cursor { line.style(Style::default().bg(palette::SELECTION)) } else { line };
                        AgentRow::new(line, ItemKey::Header(Section::Remote))
                    })
                    .collect::<Vec<_>>();
                let mut visible = Vec::new();
                append_list_window(&mut visible, &rows, height, 0, follow_cursor(&rows, height));
                let visible: Vec<_> = visible.into_iter().map(|row| row.line).collect();
                assert!(visible.len() <= height);
                assert!(visible.iter().any(|line| line.to_string() == format!("agent {cursor}")));
                if height >= 3 {
                    let content: Vec<usize> = visible.iter().filter_map(|line| line.to_string().strip_prefix("agent ").and_then(|s| s.parse().ok())).collect();
                    let first = *content.first().unwrap();
                    let last = *content.last().unwrap();
                    if first > 0 {
                        assert_eq!(visible[0].to_string(), format!("  ↑ {first} more"));
                    }
                    if last < 19 {
                        assert_eq!(visible.last().unwrap().to_string(), format!("  ↓ {} more", 19 - last));
                    }
                }
            }
        }
    }

    #[test]
    fn expanded_stopped_section_keeps_live_share_and_section_cursor_visible() {
        crate::config::Config::init();
        let mut app = crate::tui::test_helpers::offline_app();
        app.data.agents = (0..10).map(|i| crate::tui::test_helpers::make_test_agent(&format!("live{i}"), 0.0)).collect();
        app.data.stopped_agents = (0..100).map(|i| crate::tui::test_helpers::make_test_agent(&format!("stop{i}"), 0.0)).collect();
        app.ui.stopped_expanded = true;
        app.ui.view_mode = ViewMode::Vertical;
        for cursor in [0, 9, 10, 11, 110] {
            app.ui.cursor = cursor;
            let (rows, _) = collect_agent_lines(&app, 80, 12);
            let lines: Vec<_> = rows.into_iter().map(|row| row.line).collect();
            assert!(lines.len() <= 12);
            assert!(lines.iter().any(|line| line.style.bg == Some(palette::SELECTION)));
            assert!(lines.iter().filter(|line| line.to_string().contains("live")).count() >= 4);
        }
    }

    #[test]
    fn context_stays_in_heading_and_is_omitted_when_narrow() {
        crate::config::Config::init();
        let mut app = crate::tui::test_helpers::offline_app();
        let mut agent = crate::tui::test_helpers::make_test_agent("nahe", 3240.0);
        agent.tool = crate::integrations::tool::Tool::Qoder.into();
        agent.status_time = agent.created_at;
        agent.transcript_path = Some("/test/session.jsonl".into());
        for context in ["100k/200k", "100k"] {
            app.usage.set_context_for_test(&agent, context);
            let mut lines = Vec::new();
            build_agent_detail(&agent, &app, &mut lines, 120);
            assert_eq!(lines.len(), 3);
            let heading = lines[0].to_string();
            assert!(heading.contains(&format!("nahe · qoder · 54m · ctx:{context}")), "{heading}");
            assert!(!lines[1].to_string().contains(context));
            let style_of = |lines: &[Line]| lines[0].spans.iter().find(|s| s.content == format!("ctx:{context}")).unwrap().style;
            assert_eq!(style_of(&lines), Theme::agent_dim());
            app.usage.mark_stale_for_test();
            lines.clear();
            build_agent_detail(&agent, &app, &mut lines, 120);
            assert_eq!(style_of(&lines), Theme::separator());
            lines.clear();
            build_agent_detail(&agent, &app, &mut lines, 40);
            assert!(!lines[0].to_string().contains(context));
            assert!(lines[0].to_string().contains("nahe · qoder · 54m"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::backend::TestBackend;

    #[test]
    fn list_and_strip_wheel_leave_focus_selection_and_message_scroll_alone() {
        crate::config::Config::init();
        let renderers: [(ViewMode, fn(&mut Frame, Rect, &App)); 2] = [(ViewMode::Vertical, render_agents_compact), (ViewMode::Inline, render_agents_tabstrip)];
        for (mode, render) in renderers {
            let mut app = crate::tui::test_helpers::offline_app();
            app.ui.view_mode = mode;
            app.ui.msg_scroll = 17;
            app.ui.selected.insert("agent0".into());
            app.ui.eject_filter = Some(app.ui.selected.clone());
            app.data.agents = (0..10).map(|i| crate::tui::test_helpers::make_test_agent(&format!("agent{i}"), 0.0)).collect();
            let mut terminal = Terminal::new(TestBackend::new(48, 16)).unwrap();
            let area = Rect::new(4, 5, if mode == ViewMode::Inline { 11 } else { 24 }, 5);
            let draw = |terminal: &mut Terminal<TestBackend>, app: &App| {
                app.mouse.clear_targets();
                terminal.draw(|frame| render(frame, area, app)).unwrap();
            };
            draw(&mut terminal, &app);
            let first_tab_or_row = |app: &App| {
                app.mouse
                    .regions()
                    .into_iter()
                    .find_map(|(_, target)| match target {
                        Target::Agent(AgentTarget::Select(key)) => Some(key),
                        _ => None,
                    })
                    .unwrap()
            };
            let original = first_tab_or_row(&app);

            assert!(app.handle_mouse(MouseEvent { kind: MouseEventKind::ScrollDown, column: area.x, row: area.y, modifiers: KeyModifiers::NONE }));
            draw(&mut terminal, &app);
            assert_ne!(first_tab_or_row(&app), original);
            assert_eq!(app.ui.cursor, 0);
            assert_eq!(app.ui.msg_scroll, 17);
            assert_eq!(app.ui.eject_filter.as_ref(), Some(&app.ui.selected));
            assert_eq!(app.ui.selected.len(), 1);
        }
    }

    #[test]
    fn tab_clicks_use_unicode_width_and_nonzero_viewport_origin() {
        crate::config::Config::init();
        let mut app = crate::tui::test_helpers::offline_app();
        app.data.agents = ["界界", "luna"].map(|name| crate::tui::test_helpers::make_test_agent(name, 0.0)).to_vec();
        let mut terminal = Terminal::new(TestBackend::new(48, 16)).unwrap();
        let area = Rect::new(7, 9, 22, 1);
        terminal.draw(|frame| render_agents_tabstrip(frame, area, &app)).unwrap();

        let click = |app: &mut App, col| {
            for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
                app.handle_mouse(MouseEvent { kind, column: col, row: area.y, modifiers: KeyModifiers::NONE });
            }
        };
        // First tab occupies six cells (status, space, four name cells).
        let icon = area.x + 1 + 6 + TAB_GAP as u16;
        let symbol = AgentStatus::Active.icon();
        assert_eq!(terminal.backend().buffer()[(icon, area.y)].symbol(), symbol);
        click(&mut app, icon + 2);
        assert_eq!(app.ui.cursor, 1);
        assert!(app.ui.selected.is_empty());
        click(&mut app, icon);
        assert!(app.ui.selected.contains("luna"));
        terminal.draw(|frame| render_agents_tabstrip(frame, area, &app)).unwrap();
        let cell = &terminal.backend().buffer()[(icon, area.y)];
        assert_eq!(cell.symbol(), symbol);
        assert_eq!(cell.fg, AgentStatus::Active.style().fg.unwrap());
        assert_eq!(cell.bg, FOCUSED_CHECKED);
        click(&mut app, area.x + 3);
        terminal.draw(|frame| render_agents_tabstrip(frame, area, &app)).unwrap();
        assert_eq!(terminal.backend().buffer()[(icon, area.y)].bg, CHECKED);
    }

    #[test]
    fn session_preview_truncates_at_character_boundaries() {
        let mut agent = crate::tui::test_helpers::make_test_agent("nova", 60.0);
        agent.session_id = Some("é世🙂abcde".into());
        let line = detail_line_info(&agent, 200).to_string();
        assert!(line.contains("session:é世🙂abc"), "{line}");
        assert!(!line.contains("abcde"), "{line}");
    }
}
