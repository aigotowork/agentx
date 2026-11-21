use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::tui::{
    theme::{Theme, palette},
    transcript::Reader,
};

pub(super) fn render(frame: &mut Frame, area: Rect, reader: &mut Reader) {
    let [header, mut body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let title = format!("Transcript · @{}", reader.name);
    frame.render_widget(Paragraph::new(title).style(Theme::title()), header);
    if body.is_empty() {
        return;
    }
    let scrollbar_area = body;
    body.width = body.width.saturating_sub(1).max(1);
    let style = if reader.failed() { Theme::error() } else { Style::default().fg(palette::FG) };
    let (lines, start, total) = reader.viewport(body.width, body.height as usize, wrap);
    frame.render_widget(Paragraph::new(lines.iter().map(|line| Line::raw(line.as_str())).collect::<Vec<_>>()).style(style), body);
    if total > body.height as usize && scrollbar_area.width > 1 {
        let mut state = ScrollbarState::new(total).viewport_content_length(body.height as usize).position(start);
        frame.render_stateful_widget(Scrollbar::new(ScrollbarOrientation::VerticalRight).begin_symbol(None).end_symbol(None).track_symbol(Some(" ")).thumb_style(Theme::dim()), scrollbar_area, &mut state);
    }
}

fn wrap(text: &str, width: u16) -> Vec<String> {
    text.lines()
        .flat_map(|line| {
            // Provider text is display data. Never emit terminal control characters.
            let clean: String = line.replace('\t', "    ").chars().filter(|c| !c.is_control()).collect();
            super::break_into_visual_lines(&clean, width, 0)
        })
        .collect()
}
