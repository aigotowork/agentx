//! Read-only dashboard transcript reader. Its target and scrolling are independent of the event feed.
use crossterm::event::KeyCode;
use uuid::Uuid;

use super::{app::App, commands, model::CursorTarget, rpc::client::RpcOp};
use crate::agents::attachments::ParticipantRef;
use crate::transcript::{
    client,
    service::{Selection, Source},
};

#[derive(Clone)]
pub struct Target(TargetKind);

#[derive(Clone)]
enum TargetKind {
    Live(ParticipantRef),
    Stopped(Selection),
    Remote(String),
}

impl Target {
    pub(super) fn read(&self) -> Result<String, String> {
        let opts = client::RenderOptions { full_mode: true, ..Default::default() };
        match &self.0 {
            TargetKind::Live(participant) => client::render_participant(&crate::cli::client::connect()?, participant, &opts),
            TargetKind::Stopped(selection) => client::render_selection(selection.clone(), &selection.canonical_name, None, &opts),
            TargetKind::Remote(name) => {
                let response = commands::run_native(&["transcript".into(), format!("@{name}"), "--last".into(), opts.last_n.to_string(), "--full".into()])?;
                if response.ok() { Ok(response.stdout) } else { Err(response.error_message()) }
            }
        }
    }
}

pub(super) struct Reader {
    pub name: String,
    pub(super) subject: super::info_data::Subject,
    target: Target,
    request: Uuid,
    result: Option<Result<String, String>>,
    wrapped: Option<(u16, Vec<String>)>,
    offset: usize,
    max_offset: usize,
    page: usize,
}

impl Reader {
    pub(super) fn scroll_up(&mut self, lines: usize) {
        self.offset = self.offset.saturating_add(lines).min(self.max_offset);
    }

    pub(super) fn scroll_down(&mut self, lines: usize) {
        self.offset = self.offset.saturating_sub(lines);
    }

    /// Wrap once per terminal width and keep a bottom-relative position across redraws.
    pub(super) fn viewport(&mut self, width: u16, height: usize, wrap: impl Fn(&str, u16) -> Vec<String>) -> (&[String], usize, usize) {
        if self.wrapped.as_ref().is_none_or(|(previous, _)| *previous != width) {
            let content = match &self.result {
                None => "Loading transcript…",
                Some(Ok(text)) if text.trim().is_empty() => "No exchanges found.",
                Some(Ok(text) | Err(text)) => text,
            };
            self.wrapped = Some((width, wrap(content, width)));
        }
        let (_, lines) = self.wrapped.as_ref().unwrap();
        self.page = height.saturating_sub(1).max(1);
        self.max_offset = lines.len().saturating_sub(height);
        self.offset = self.offset.min(self.max_offset);
        let start = self.max_offset - self.offset;
        (&lines[start..(start + height).min(lines.len())], start, lines.len())
    }

    pub(super) fn failed(&self) -> bool {
        matches!(self.result, Some(Err(_)))
    }
}

impl App {
    pub(super) fn has_transcript_target(&self) -> bool {
        matches!(self.cursor_target(), CursorTarget::Agent(_) | CursorTarget::StoppedAgent(_) | CursorTarget::RemoteAgent(_))
    }

    pub(super) fn open_transcript(&mut self) {
        let (agent, target) = match self.cursor_target() {
            CursorTarget::Agent(i) => {
                let agent = &self.data.agents[i];
                (agent, TargetKind::Live(ParticipantRef { canonical_name: agent.name.clone(), incarnation_bits: agent.created_at.to_bits() }))
            }
            CursorTarget::StoppedAgent(i) => {
                let agent = &self.data.stopped_agents[i];
                let source = agent.transcript_path.as_ref().map(|path| Source { name: agent.name.clone(), path: path.clone(), tool: agent.tool.name().into(), session_id: agent.session_id.clone() });
                (agent, TargetKind::Stopped(Selection { canonical_name: agent.name.clone(), source, missing: None, codex_lookup: None }))
            }
            CursorTarget::RemoteAgent(i) => {
                let name = self.data.remote_agents[i].action_name();
                (&self.data.remote_agents[i], TargetKind::Remote(name))
            }
            _ => return,
        };
        self.ui.transcript = Some(Reader { name: agent.display_name(), subject: super::info_data::Subject::from_agent(agent), target: Target(target), request: Uuid::nil(), result: None, wrapped: None, offset: 0, max_offset: 0, page: 1 });
        self.load_transcript();
    }

    fn load_transcript(&mut self) {
        let Some(reader) = self.ui.transcript.as_mut() else { return };
        let id = Uuid::new_v4();
        reader.request = id;
        reader.result = None;
        reader.wrapped = None;
        let target = reader.target.clone();
        if let Err(error) = self.enqueue_rpc(RpcOp::ReadTranscript { id, target }) {
            self.finish_transcript(id, Err(error));
        }
    }

    pub(super) fn finish_transcript(&mut self, id: Uuid, result: Result<String, String>) {
        if let Some(reader) = self.ui.transcript.as_mut()
            && reader.request == id
        {
            reader.result = Some(result);
            reader.wrapped = None;
            reader.offset = 0;
        }
    }

    pub(super) fn handle_transcript_key(&mut self, code: KeyCode) {
        let Some(reader) = self.ui.transcript.as_mut() else { return };
        match code {
            KeyCode::Esc | KeyCode::Char('t' | 'T') => self.ui.transcript = None,
            KeyCode::Char('\\') => self.ui.switch_viewport = true,
            KeyCode::Char('r') if reader.result.is_some() => self.load_transcript(),
            KeyCode::Up => reader.scroll_up(1),
            KeyCode::Down => reader.scroll_down(1),
            KeyCode::PageUp => reader.scroll_up(reader.page),
            KeyCode::PageDown => reader.scroll_down(reader.page),
            KeyCode::Home => reader.offset = reader.max_offset,
            KeyCode::End => reader.offset = 0,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{
        DEFAULT_INLINE_HEIGHT,
        model::{Message, SenderKind, ViewMode},
        render,
    };
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn transcript_reader_ignores_closed_requests_and_preserves_dashboard_context() {
        crate::config::Config::init();
        let mut app = crate::tui::test_helpers::offline_app();
        app.data.agents = ["nova", "luna"].map(|name| crate::tui::test_helpers::make_test_agent(name, 0.0)).into();
        app.ui.selected.insert("nova".into());
        app.ui.search_filter = Some("handoff".into());
        app.ui.input = "unsent draft".into();
        app.ui.msg_scroll = 7;

        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        let closed = app.ui.transcript.as_ref().unwrap().request;
        app.handle_key(KeyCode::Char('T'), KeyModifiers::SHIFT);
        assert!(app.ui.transcript.is_none());
        app.handle_key(KeyCode::Down, KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        let current = app.ui.transcript.as_ref().unwrap().request;
        app.finish_transcript(current, Ok("USER: How is the handoff?\nASSISTANT: It is ready.".into()));
        app.finish_transcript(closed, Ok("stale nova reply".into()));
        app.handle_paste("ignored");

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| render::render(frame, &mut app, DEFAULT_INLINE_HEIGHT)).unwrap();
        let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("Transcript · @luna"));
        assert!(text.contains("ASSISTANT: It is ready."));
        assert!(!text.contains("stale nova reply"));

        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.ui.transcript.is_none());
        assert_eq!(app.ui.cursor, 1);
        assert!(app.ui.selected.contains("nova"));
        assert_eq!(app.ui.search_filter.as_deref(), Some("handoff"));
        assert_eq!(app.ui.msg_scroll, 7);
        assert_eq!(app.ui.input, "unsent draft");
    }

    #[test]
    fn transcript_replaces_only_the_event_area_in_both_layouts() {
        crate::config::Config::init();
        let mut app = crate::tui::test_helpers::offline_app();
        app.data.agents = ["luna", "nova"].map(|name| crate::tui::test_helpers::make_test_agent(name, 0.0)).into();
        app.data.messages.push(Message { event_id: 1, sender: "bigboss".into(), recipients: vec!["luna".into()], body: "event feed marker".into(), time: 0.0, sender_kind: SenderKind::External, intent: None, reply_to: None });
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        let id = app.ui.transcript.as_ref().unwrap().request;
        app.finish_transcript(id, Ok("ASSISTANT: It is ready.".into()));

        // Use a short terminal for the stacked layout to exercise its compact Agent strip.
        let mut stacked = Terminal::new(TestBackend::new(80, 10)).unwrap();
        stacked.draw(|frame| render::render(frame, &mut app, DEFAULT_INLINE_HEIGHT)).unwrap();
        let rows: Vec<String> = stacked.backend().buffer().content.chunks(80).map(|row| row.iter().map(|cell| cell.symbol()).collect()).collect();
        assert!(rows[0].starts_with("Transcript · @luna"));
        assert!(rows[..3].iter().any(|row| row.contains("ASSISTANT: It is ready.")));
        assert!(rows[3..].iter().any(|row| row.contains("nova")));

        app.ui.view_mode = ViewMode::Vertical;
        let mut side_by_side = Terminal::new(TestBackend::new(80, 24)).unwrap();
        side_by_side.draw(|frame| render::render(frame, &mut app, DEFAULT_INLINE_HEIGHT)).unwrap();
        let rows = side_by_side.backend().buffer().content.chunks(80);
        assert!(rows.clone().any(|row| row[..26].iter().map(|cell| cell.symbol()).collect::<String>().contains("nova")));
        assert!(rows.clone().any(|row| row[27..].iter().map(|cell| cell.symbol()).collect::<String>().starts_with("Transcript · @luna")));
        assert!(rows.clone().any(|row| row[27..].iter().map(|cell| cell.symbol()).collect::<String>().contains("ASSISTANT: It is ready.")));
        assert!(!rows.clone().any(|row| row.iter().map(|cell| cell.symbol()).collect::<String>().contains("event feed marker")));

        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        side_by_side.draw(|frame| render::render(frame, &mut app, DEFAULT_INLINE_HEIGHT)).unwrap();
        let text: String = side_by_side.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("event feed marker"));
        assert!(!text.contains("Transcript · @luna"));
    }
}
