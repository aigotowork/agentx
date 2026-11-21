use std::ops::Range;

use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use unicode_width::UnicodeWidthStr;

use crate::tui::{
    app::App,
    info_data::{Link, Row},
    model::AgentStatus,
    mouse::Target,
    theme::{Theme, palette},
};

use super::{WrapLayout, fit_spans, truncate_display};

/// A visit's visual lines at one width. Navigation and data updates clear the cache.
#[derive(Default)]
pub(in crate::tui) struct Cache {
    width: u16,
    rows: Vec<CachedRow>,
    starts: Vec<usize>,
    total: usize,
}

impl Cache {
    fn new(rows: &[Row], width: u16, subject_is_request_error: bool) -> Self {
        let mut cache = Self { width, ..Self::default() };
        for row in rows {
            cache.starts.push(cache.total);
            let row = CachedRow::new(row, width - row_gutter(width), subject_is_request_error);
            cache.total += row.lines.len();
            cache.rows.push(row);
        }
        cache
    }
}

pub(super) fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(info) = app.ui.info.as_mut() else { return };
    let [header, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    app.mouse.add(area, Target::InfoPane);
    if !header.is_empty() {
        let back_width = render_header(frame, header, &info.title());
        app.mouse.add(Rect::new(header.x, header.y, back_width, 1), Target::InfoBack);
    }
    if body.is_empty() {
        return;
    }

    // Reserve the scroll rail before measuring, so appearing scrollbars do not reflow rows.
    let width = body.width - u16::from(body.width >= 4);
    let height = usize::from(body.height);
    if info.render_cache.width != width {
        info.render_cache = Cache::new(info.rows(), width, info.subject_is_request_error());
    }
    let total = info.render_cache.total;
    info.layout(info.render_cache.starts.clone(), total, height);
    let scroll = info.scroll.min(total.saturating_sub(height));

    for (offset, line_index) in (scroll..total.min(scroll + height)).enumerate() {
        let cache = &info.render_cache;
        let index = cache.starts.partition_point(|start| *start <= line_index) - 1;
        let row_line = line_index - cache.starts[index];
        let focused = index == info.cursor && info.rows()[index].link.is_some();
        let line = cache.rows[index].line(row_line, width, focused);
        let area = Rect::new(body.x, body.y + offset as u16, width, 1);
        frame.render_widget(Paragraph::new(line), area);
        app.mouse.add(area, Target::InfoRow(index));
    }
    if total > height && width < body.width {
        let mut state = ScrollbarState::new(total - height + 1).viewport_content_length(height).position(scroll);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight).begin_symbol(None).end_symbol(None).track_symbol(Some("│")).track_style(Theme::separator()).thumb_style(Theme::dim());
        frame.render_stateful_widget(scrollbar, body, &mut state);
    }
}

fn render_header(frame: &mut Frame, area: Rect, title: &str) -> u16 {
    let (back, gap) = if area.width >= 16 { ("‹ Back", "  ") } else { ("‹", " ") };
    let back_width = back.width();
    let prefix = if area.width >= 24 { "Info · " } else { "" };
    let title_width = usize::from(area.width).saturating_sub(back_width + gap.width() + prefix.width());
    let title = breadcrumb(&clean_text(title, false), title_width);
    let spans = vec![Span::styled(back, Theme::cursor()), Span::raw(gap), Span::styled(format!("{prefix}{title}"), Theme::title())];
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
    back_width as u16
}

fn breadcrumb(title: &str, width: usize) -> String {
    if title.width() <= width {
        return title.to_owned();
    }
    let continuation = "… › ";
    let delimiter = " › ";
    let mut short = None;
    // Keep the current visit and as many of its nearest ancestors as fit.
    for (start, _) in title.rmatch_indices(delimiter) {
        let suffix = &title[start + delimiter.len()..];
        if continuation.width() + suffix.width() > width {
            break;
        }
        short = Some(suffix);
    }
    short.map_or_else(|| truncate_display(title.rsplit(delimiter).next().unwrap_or(title), width), |suffix| format!("{continuation}{suffix}"))
}

pub(super) fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    if area.is_empty() {
        return;
    }
    let exit = if area.width >= 6 { "i Exit" } else { "i" };
    let mut spans = vec![Span::styled(exit, Theme::cursor()), Span::styled(" · ", Theme::separator()), Span::styled("Esc back · ↑↓ move", Theme::agent_context())];
    if app.ui.info.as_ref().is_some_and(|info| info.rows().iter().any(|row| row.link.is_some())) {
        spans.push(Span::styled(" · Enter open", Theme::agent_context()));
    }
    frame.render_widget(Paragraph::new(Line::from(fit_spans(spans, usize::from(area.width)))), area);
    app.mouse.add(Rect::new(area.x, area.y, exit.width() as u16, 1), Target::InfoOpen);
}

fn clean_text(text: &str, multiline: bool) -> String {
    let mut clean = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\t' => clean.push_str("    "),
            '\r' if multiline => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                clean.push('\n');
            }
            '\n' if multiline => clean.push(ch),
            ch if !ch.is_control() => clean.push(ch),
            _ => {}
        }
    }
    clean
}

fn row_gutter(width: u16) -> u16 {
    if width > 3 { 2 } else { 0 }
}

struct CachedRow {
    text: String,
    value_start: usize,
    lines: Vec<Range<usize>>,
    value_style: Style,
}

impl CachedRow {
    fn new(row: &Row, width: u16, subject_is_request_error: bool) -> Self {
        let linked = row.link.is_some();
        let mut text = if linked { String::from("› ") } else { String::new() };
        text.push_str(&clean_text(&row.label, false));
        if !row.label.is_empty() && !row.value.is_empty() {
            text.push(':');
            text.push(if text.width() < usize::from(width) / 2 { ' ' } else { '\n' });
        }
        let value_start = text.len();
        if let Some(status) = row.status {
            text.push_str(status.icon());
            if !row.value.is_empty() {
                text.push(' ');
            }
        }
        text.push_str(&clean_text(&row.value, true));

        let mut value_style = match row.status {
            Some(AgentStatus::Error)
                if match &row.link {
                    Some(Link::Agent(subject)) => subject.is_request_error(),
                    None => subject_is_request_error,
                    _ => false,
                } =>
            {
                Theme::launching()
            }
            Some(status) => status.style(),
            None => Style::default().fg(palette::FG),
        };
        if linked {
            value_style = value_style.add_modifier(Modifier::UNDERLINED);
        }
        let lines = wrap_ranges(&text, width);
        Self { text, value_start, lines, value_style }
    }

    fn line(&self, index: usize, width: u16, focused: bool) -> Line<'_> {
        let mut spans = Vec::with_capacity(3);
        let gutter = row_gutter(width);
        if gutter > 0 {
            let marker = if !focused {
                "  "
            } else if index == 0 {
                "❯ "
            } else {
                "▏ "
            };
            spans.push(Span::styled(marker, Theme::cursor()));
        }
        let range = &self.lines[index];
        let split = self.value_start.clamp(range.start, range.end);
        if range.start < split {
            spans.push(clipped_span(&self.text[range.start..split], width - gutter, Theme::agent_context()));
        }
        if split < range.end {
            spans.push(clipped_span(&self.text[split..range.end], width - gutter, self.value_style));
        }
        let style = if focused { Style::default().bg(palette::SELECTION) } else { Style::default() };
        Line::from(spans).style(style)
    }
}

fn clipped_span(text: &str, width: u16, style: Style) -> Span<'_> {
    if text.width() > usize::from(width) { Span::styled(truncate_display(text, usize::from(width)), style) } else { Span::styled(text, style) }
}

/// Store byte ranges instead of allocating styled lines for an entire document.
fn wrap_ranges(text: &str, width: u16) -> Vec<Range<usize>> {
    let layout = WrapLayout::new(width, 0);
    let mut lines = Vec::new();
    let mut base = 0;
    for line in text.split('\n') {
        let mut start = base;
        let mut visual_row = 0;
        for pos in layout.walk(line) {
            if pos.line != visual_row {
                let end = base + pos.offset;
                lines.push(start..end);
                start = end;
                visual_row = pos.line;
            }
        }
        let end = base + line.len();
        lines.push(start..end);
        base = end + 1;
    }
    lines
}
