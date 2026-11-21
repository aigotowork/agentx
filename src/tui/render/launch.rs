use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::integrations::tool::Tool;
use crate::tui::app::App;
use crate::tui::control_mouse::{ControlTarget, LaunchAction};
use crate::tui::model::*;
use crate::tui::theme::{Theme, palette};

const LABEL_W: usize = 9;

pub fn render_launch_inline(frame: &mut Frame, area: Rect, app: &App) {
    let settings = &app.ui.launch;
    let w = area.width as usize;
    let mut lines = vec![header(area, app)];
    if let Some(error) = &settings.quick.error {
        lines.push(Line::from(Span::styled(super::truncate_display(error, w), Theme::error())));
    }
    if settings.quick.picker.is_some() {
        lines.push(Line::from(format!("  Search ❯ {}", settings.quick.query)));
        let choices = settings.choices();
        let visible = usize::from(area.height).saturating_sub(lines.len()).min(4);
        let start = (settings.quick.index + 1).saturating_sub(visible);
        for (index, (name, detail)) in choices.iter().enumerate().skip(start).take(visible) {
            let text = match name {
                Some(name) => format!("  {name}  {detail}"),
                None if settings.quick.picker == Some(LaunchField::Profile) && settings.quick.profile.is_none() => format!("  Auto  {} {}", settings.quick.profile_label, settings.quick.source_label),
                None => format!("  {detail}"),
            };
            super::add_control(app, area, 0, lines.len(), w, ControlTarget::LaunchChoice(index));
            let style = if index == settings.quick.index { Style::default().bg(palette::SELECTION) } else { Style::default() };
            lines.push(Line::from(text).style(style));
        }
        if choices.is_empty() {
            lines.push(Line::styled("  No matches", Theme::dim()));
        }
    } else if settings.quick.advanced {
        let headless = if settings.tool.or(settings.resolved_tool) == Some(Tool::Claude) {
            match (settings.headless, settings.headless_pty) {
                (false, _) => " ",
                (true, false) => "print",
                (true, true) => "pty",
            }
        } else if settings.headless {
            "✓"
        } else {
            " "
        };
        let values = [
            ("Tool", settings.tool.map_or("Auto", |t| t.as_str()).to_owned(), LaunchField::Tool),
            ("Count", settings.count.to_string(), LaunchField::Count),
            ("Headless", format!("[{headless}]"), LaunchField::Headless),
            ("Terminal", settings.terminal_presets.get(settings.terminal).cloned().unwrap_or_else(|| "auto".into()), LaunchField::Terminal),
        ];
        for (label, value, field) in &values {
            lines.push(field_row(area, app, lines.len(), label, value, *field));
        }
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn header(area: Rect, app: &App) -> Line<'static> {
    let launch = &app.ui.launch;
    let w = usize::from(area.width);
    let role = launch.quick.role.as_deref().unwrap_or("—");
    let profile = launch.quick.profile.as_deref().unwrap_or_else(|| if launch.quick.profile_label.is_empty() { "Auto" } else { &launch.quick.profile_label });
    let mut summary = Vec::new();
    if let Some(tool) = launch.tool {
        summary.push(tool.as_str().to_owned());
    }
    if launch.count > 1 {
        summary.push(format!("×{}", launch.count));
    }
    if launch.headless {
        summary.push("Headless".into());
    }
    if !launch.terminal_override().is_empty() {
        summary.push(launch.terminal_override().into());
    }
    let role_text = format!("Role {role} ");
    let mut profile_text = format!("Profile {profile} ");
    if w >= 58 && !launch.quick.source_label.is_empty() {
        profile_text.push_str(&format!("({}) ", launch.quick.source_label));
    }
    let available = w.saturating_sub(3);
    let role_w = role_text.width().min(available / 2);
    let mut spans = Vec::new();
    let mut x = 0;
    for (field, text, budget) in [(LaunchField::Role, role_text, role_w), (LaunchField::Profile, profile_text, available - role_w), (LaunchField::More, format!("··· {}", summary.join(" · ")), w)] {
        let label = super::truncate_display(&text, budget.min(w.saturating_sub(x)));
        let width = label.width();
        super::add_control(app, area, x, 0, width, ControlTarget::OpenLaunch(field));
        let style = if launch.options_cursor == Some(field) { Style::default().bg(palette::SELECTION) } else { Theme::launch_active() };
        spans.push(Span::styled(label, style));
        x += width;
    }
    Line::from(spans)
}

fn field_row(area: Rect, app: &App, row: usize, label: &str, value: &str, field: LaunchField) -> Line<'static> {
    let width = area.width as usize;
    let selected = app.ui.launch.options_cursor == Some(field);
    let style = if selected { Theme::launch_active().bg(palette::SELECTION) } else { Style::default().fg(palette::FG) };
    let label_style = if selected { style } else { Theme::dim() };
    let arrow_style = if selected { Theme::launch_arrow() } else { Style::default().fg(palette::FG_DIM) };
    let headless = field == LaunchField::Headless;

    let prefix = if width >= 21 { "    " } else { "" };
    let prefix_w = UnicodeWidthStr::width(prefix);
    let label_w = LABEL_W.min(width.saturating_sub(prefix_w + 3));
    let label = super::truncate_display(label, label_w);
    let label_pad = label_w.saturating_sub(UnicodeWidthStr::width(label.as_str()));
    let content_start = prefix_w + label_w;
    let mut spans = vec![Span::styled(prefix, Theme::cursor()), Span::styled(format!("{label}{}", " ".repeat(label_pad)), label_style)];
    let register = |start, len, action| super::add_control(app, area, start, row, len, ControlTarget::LaunchField(field, action));

    if content_start > prefix_w {
        register(prefix_w.saturating_sub(if selected { 2 } else { 0 }), label_w + if selected { prefix_w.min(2) } else { 0 }, if headless { LaunchAction::Toggle } else { LaunchAction::Focus });
    }
    if headless {
        let value = super::truncate_display(value, width.saturating_sub(content_start));
        register(content_start, UnicodeWidthStr::width(value.as_str()), LaunchAction::Toggle);
        spans.push(Span::styled(value, style));
    } else if width.saturating_sub(content_start) >= 3 {
        let (left, right) = if width >= 21 { ("◂ ", " ▸") } else { ("◂", "▸") };
        let left_w = UnicodeWidthStr::width(left);
        let right_w = UnicodeWidthStr::width(right);
        let value = super::truncate_display(value, width.saturating_sub(content_start + left_w + right_w));
        let value_w = UnicodeWidthStr::width(value.as_str());
        register(content_start, left_w, LaunchAction::Previous);
        register(content_start + left_w, value_w, LaunchAction::Focus);
        register(content_start + left_w + value_w, right_w, LaunchAction::Next);
        spans.extend([Span::styled(left, arrow_style), Span::styled(value, style), Span::styled(right, arrow_style)]);
    } else {
        let value = super::truncate_display(value, width.saturating_sub(content_start));
        register(content_start, UnicodeWidthStr::width(value.as_str()), LaunchAction::Next);
        spans.push(Span::styled(value, style));
    }
    Line::from(spans)
}
