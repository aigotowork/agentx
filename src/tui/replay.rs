//! Test-only trace of the production renderer and input handlers. The synthetic
//! DataSource and RPC sink keep the replay independent of daemon/terminal I/O.
use std::{cell::RefCell, env, fmt::Write as _, fs, path::PathBuf, rc::Rc};

use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Terminal, TerminalOptions, Viewport,
    backend::{Backend, TestBackend},
    buffer::Buffer,
    layout::{Position, Rect},
};
use serde::Serialize;
use unicode_width::UnicodeWidthStr;

use super::agent_mouse::{AgentTarget, ItemKey};
use super::app::{App, DataState};
use super::control_mouse::{ControlTarget, Field, ShortcutScope};
use super::data::DataSource;
use super::info::Worker;
use super::info_data::{Link, Page, Row, Subject};
use super::model::{Agent, AgentStatus, CursorTarget, FIXED_EPOCH, InputMode, Message, SenderKind, ViewMode};
use super::mouse::Target;
use super::rpc::Response;
use super::rpc::client::{RpcOp, RpcResult};

const NOW: f64 = 1_760_010_000.0;
const INLINE_TOP: u16 = 11;

/// Keep age displays and participant identity stable without changing clock
/// behavior for other tests running in parallel.
struct ReplayClock(Option<f64>);

impl Drop for ReplayClock {
    fn drop(&mut self) {
        FIXED_EPOCH.set(self.0);
    }
}

#[derive(Clone)]
struct Fixture(Rc<RefCell<DataState>>);

impl DataSource for Fixture {
    fn load(&mut self) -> DataState {
        self.0.borrow().clone()
    }

    fn load_all_stopped(&mut self) -> Vec<Agent> {
        self.0.borrow().stopped_agents.clone()
    }
}

#[derive(Serialize)]
struct Trace {
    scenario: &'static str,
    title: &'static str,
    steps: Vec<Step>,
}

#[derive(Serialize)]
struct Step {
    label: &'static str,
    input: String,
    pointer: Option<[u16; 2]>,
    hit: Option<Region>,
    effects: Vec<String>,
    frame: Screen,
}

#[derive(Serialize)]
struct Region {
    target: String,
    rect: [u16; 4],
}

#[derive(Serialize)]
struct Screen {
    origin: [u16; 2],
    columns: u16,
    lines: u16,
    cursor: Option<[u16; 2]>,
    rows: Vec<Vec<Run>>,
}

#[derive(Serialize)]
struct Run {
    text: String,
    columns: u16,
    fg: String,
    bg: String,
    attributes: Vec<&'static str>,
}

enum Input {
    Key(KeyCode),
    Ctrl(char),
    Paste(&'static str),
    Pointer(MouseEvent),
    Resize(u16, u16),
    Snapshot(Box<DataState>),
    TranscriptReply,
    InfoReply(uuid::Uuid, Result<Vec<Row>, String>),
}

type InfoRequest = (uuid::Uuid, Subject, Page);

struct Replay {
    app: App,
    source: Fixture,
    terminal: Terminal<TestBackend>,
    is_inline: bool,
    trace: Trace,
    info_requests: Vec<InfoRequest>,
    _clock: ReplayClock,
}

impl Replay {
    fn new(scenario: &'static str, title: &'static str, layout: ViewMode) -> Self {
        let clock = ReplayClock(FIXED_EPOCH.replace(Some(NOW)));
        let mut data = DataState::empty();
        data.agents = ["luna", "nova", "ravo"].iter().map(|name| super::test_helpers::make_test_agent(name, 70.0)).collect();
        data.messages = (1..=36)
            .map(|i| Message {
                event_id: i,
                sender: "luna".into(),
                recipients: vec!["nova".into()],
                body: format!("Sample note {i:02} · check the rendered message feed"),
                time: 1_760_000_000.0 + i as f64 * 60.0,
                sender_kind: SenderKind::Instance,
                intent: None,
                reply_to: None,
            })
            .collect();

        let source = Fixture(Rc::new(RefCell::new(data)));
        let mut app = App::for_replay(Box::new(source.clone()));
        app.info_worker = Worker::record();
        match layout {
            ViewMode::Inline => super::prepare_inline_viewport(&mut app),
            ViewMode::Vertical => super::prepare_vertical_viewport(&mut app),
        }
        let is_inline = layout == ViewMode::Inline;
        let mut replay = Self { app, source, terminal: Self::terminal(100, 24, is_inline), is_inline, trace: Trace { scenario, title, steps: Vec::new() }, info_requests: Vec::new(), _clock: clock };
        let frame = replay.draw();
        replay.trace.steps.push(Step { label: "Initial render", input: "Start".into(), frame, pointer: None, hit: None, effects: vec![] });
        replay
    }

    fn terminal(columns: u16, lines: u16, inline: bool) -> Terminal<TestBackend> {
        let mut backend = TestBackend::new(columns, lines);
        if inline {
            let top = INLINE_TOP.min(lines.saturating_sub(super::DEFAULT_INLINE_HEIGHT));
            backend.set_cursor_position(Position::new(0, top)).unwrap();
            Terminal::with_options(backend, TerminalOptions { viewport: Viewport::Inline(super::DEFAULT_INLINE_HEIGHT) }).unwrap()
        } else {
            Terminal::new(backend).unwrap()
        }
    }

    fn draw(&mut self) -> Screen {
        if !self.app.reader_open() {
            self.app.ui.msg_scroll = self.app.ui.msg_scroll.min(self.app.ui.scroll_max);
        }
        let drawn = self.terminal.draw(|frame| super::render::render(frame, &mut self.app, super::DEFAULT_INLINE_HEIGHT)).unwrap();
        let mut screen = Screen::from_buffer(drawn.buffer);
        let backend = self.terminal.backend_mut();
        if backend.cursor_visible() {
            let position = backend.get_cursor_position().unwrap();
            screen.cursor = Some([position.x, position.y]);
        }
        screen
    }

    fn step(&mut self, label: &'static str, input: Input) {
        let before = self.app.recorded_rpc.as_ref().unwrap().len();
        let mut pointer = None;
        let mut hit = None;
        let input = match input {
            Input::Key(code) => {
                self.app.handle_key(code, KeyModifiers::NONE);
                format!("Key {code:?}")
            }
            Input::Ctrl(c) => {
                self.app.handle_key(KeyCode::Char(c), KeyModifiers::CONTROL);
                format!("Key Ctrl+{c}")
            }
            Input::Paste(text) => {
                self.app.mouse.cancel_press();
                self.app.handle_paste(text);
                format!("Paste {text:?}")
            }
            Input::Pointer(event) => {
                let position = Position::new(event.column, event.row);
                pointer = Some([position.x, position.y]);
                hit = self.hit(position).map(|(rect, target)| Region::new(rect, &target));
                self.app.handle_mouse(event);
                format!("Mouse {:?} · {}, {}", event.kind, position.x, position.y)
            }
            Input::Resize(columns, lines) => {
                self.app.mouse.invalidate();
                // The host recreates the inline viewport to avoid Ratatui
                // autoresize clearing terminal scrollback on width changes.
                if self.is_inline {
                    self.terminal = Self::terminal(columns, lines, true);
                } else {
                    self.terminal.backend_mut().resize(columns, lines);
                }
                format!("Resize {columns} × {lines}")
            }
            Input::Snapshot(snapshot) => {
                self.source.0.replace(*snapshot);
                self.app.reload_data_force();
                "Refresh the in-memory data source".into()
            }
            Input::TranscriptReply => {
                let [RpcOp::ReadTranscript { id, target }] = self.app.recorded_rpc.as_deref().unwrap() else { panic!("expected one recorded transcript request") };
                let stdout = (1..=48).map(|number| format!("USER: Checkpoint {number:02}\n")).collect();
                self.app.apply_rpc_result(RpcResult { op: RpcOp::ReadTranscript { id: *id, target: target.clone() }, result: Ok(Response { exit_code: 0, stdout, stderr: String::new() }) });
                "Fixture replies with 48 transcript lines".into()
            }
            Input::InfoReply(id, result) => {
                self.app.finish_info(id, result);
                "Complete the recorded Info request".into()
            }
        };
        // App::run reports this boundary to the terminal host, which swaps
        // buffers; the offline equivalent swaps Ratatui backends in memory.
        if self.app.reader_viewport_changed(self.is_inline) {
            let size = self.terminal.size().unwrap();
            self.app.mouse.invalidate();
            self.is_inline = !self.app.reader_open();
            self.terminal = Self::terminal(size.width, size.height, self.is_inline);
        }
        let frame = self.draw();
        let mut effects: Vec<_> = self.app.recorded_rpc.as_ref().unwrap()[before..]
            .iter()
            .map(|op| match op {
                RpcOp::Send { recipients, body, .. } => format!("Send to {}: {body}", recipients.join(", ")),
                RpcOp::ReadTranscript { .. } => "Read transcript".into(),
                RpcOp::KillAgent { name } => format!("Kill @{name}"),
                _ => "RPC request recorded".into(),
            })
            .collect();
        let requests = self.app.info_worker.take_requests();
        effects.extend(requests.iter().map(|(_, subject, page)| {
            let (device, name, created) = subject.identity();
            let device = device.map_or(String::new(), |device| format!("@{device}"));
            format!("Read Info {page:?} for @{name}{device} ({:.3})", f64::from_bits(created))
        }));
        self.info_requests.extend(requests);
        self.trace.steps.push(Step { label, input, frame, pointer, hit, effects });
    }

    fn mouse(&mut self, label: &'static str, kind: MouseEventKind, at: Position) {
        self.step(label, Input::Pointer(MouseEvent { kind, column: at.x, row: at.y, modifiers: KeyModifiers::NONE }));
    }

    fn click(&mut self, label: &'static str, at: Position) {
        self.mouse("Press left button", MouseEventKind::Down(MouseButton::Left), at);
        self.mouse(label, MouseEventKind::Up(MouseButton::Left), at);
    }

    fn hit(&self, position: Position) -> Option<(Rect, Target)> {
        self.app.mouse.regions().into_iter().rev().find(|(rect, _)| rect.contains(position))
    }

    fn bounds(&self, matches: impl Fn(&Target) -> bool) -> Rect {
        self.app.mouse.regions().into_iter().find(|(_, target)| matches(target)).map(|(rect, _)| rect).expect("target must be painted")
    }

    fn agent_position(&self, name: &str) -> Position {
        let bounds = self.bounds(|target| matches!(target, Target::Agent(AgentTarget::Focus(ItemKey::Agent { name: painted, .. })) if painted == name));
        Position::new(bounds.x + bounds.width / 2, bounds.y)
    }

    fn target_position(&self, target: Target) -> Position {
        let bounds = self.bounds(|painted| *painted == target);
        Position::new(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2)
    }

    fn shows(&self, needle: &str) -> bool {
        self.trace.steps.last().unwrap().frame.text().any(|row| row.contains(needle))
    }

    fn request(&mut self, expected_name: &str, expected_page: impl FnOnce(&Page) -> bool) -> InfoRequest {
        let [request] = std::mem::take(&mut self.info_requests).try_into().unwrap_or_else(|requests: Vec<_>| panic!("expected a single Info request, got {requests:?}"));
        assert_eq!(request.1.name, expected_name);
        assert!(expected_page(&request.2), "unexpected Info request: {request:?}");
        request
    }

    fn click_info_row(&mut self, label: &'static str, value: &str) {
        let rows = self.app.ui.info.as_ref().unwrap().rows();
        let index = rows.iter().position(|row| row.link.is_some() && row.value == value).unwrap_or_else(|| panic!("no Info link for {value}"));
        let position = self.target_position(Target::InfoRow(index));
        assert_eq!(self.hit(position).unwrap().1, Target::InfoRow(index));
        self.click(label, position);
    }

    fn info_reply(&mut self, label: &'static str, request: InfoRequest, rows: Vec<Row>) {
        self.step(label, Input::InfoReply(request.0, Ok(rows)));
        assert!(!self.app.ui.info.as_ref().unwrap().loading());
    }

    fn finish_info_replay(self, expected_transcript_reads: usize) {
        assert!(self.info_requests.is_empty(), "unanswered Info requests: {:?}", self.info_requests);
        let recorded = self.app.recorded_rpc.as_deref().unwrap();
        assert_eq!(recorded.len(), expected_transcript_reads);
        assert!(recorded.iter().all(|op| matches!(op, RpcOp::ReadTranscript { .. })));
        self.trace.publish();
    }
}

impl Region {
    fn new(rect: Rect, target: &Target) -> Self {
        let target = match target {
            Target::Agent(AgentTarget::Focus(ItemKey::Agent { name, .. })) => format!("Focus @{name}"),
            Target::Agent(AgentTarget::Select(ItemKey::Agent { name, .. })) => format!("Select @{name}"),
            Target::Control(control) => format!("{control:?}"),
            _ => format!("{target:?}"),
        };
        Self { target, rect: [rect.x, rect.y, rect.width, rect.height] }
    }
}

impl Screen {
    fn from_buffer(buffer: &Buffer) -> Self {
        use ratatui::style::Modifier;
        const ATTRIBUTES: [(Modifier, &str); 7] =
            [(Modifier::BOLD, "bold"), (Modifier::DIM, "dim"), (Modifier::ITALIC, "italic"), (Modifier::UNDERLINED, "underline"), (Modifier::REVERSED, "reverse"), (Modifier::HIDDEN, "hidden"), (Modifier::CROSSED_OUT, "strike")];

        let columns = buffer.area.width;
        let rows = buffer
            .content
            .chunks(usize::from(columns))
            .map(|cells| {
                let mut runs: Vec<Run> = vec![];
                let mut x = 0;
                while x < cells.len() {
                    let cell = &cells[x];
                    let symbol = cell.symbol();
                    let width = symbol.width().clamp(1, cells.len() - x);
                    x += width;
                    let fg = cell.fg.to_string();
                    let bg = cell.bg.to_string();
                    let attributes: Vec<_> = ATTRIBUTES.iter().filter(|(flag, _)| cell.modifier.contains(*flag)).map(|(_, name)| *name).collect();

                    // A terminal's glyph width can differ from the browser font's.
                    // Anchor Unicode glyphs individually to the native column grid.
                    if let Some(run) = runs.last_mut()
                        && symbol.is_ascii()
                        && run.text.is_ascii()
                        && run.fg == fg
                        && run.bg == bg
                        && run.attributes == attributes
                    {
                        run.text.push_str(symbol);
                        run.columns += width as u16;
                    } else {
                        runs.push(Run { text: symbol.into(), columns: width as u16, fg, bg, attributes });
                    }
                }
                runs
            })
            .collect();

        Self { origin: [buffer.area.x, buffer.area.y], columns, lines: buffer.area.height, cursor: None, rows }
    }

    fn text(&self) -> impl Iterator<Item = String> + '_ {
        self.rows.iter().map(|runs| runs.iter().map(|run| run.text.as_str()).collect())
    }
}

impl Trace {
    /// Keep ordinary cargo tests artifact-free; only the replay recipe opts in.
    fn publish(self) {
        let Some(root) = env::var_os("AGENTX_TUI_REPLAY_OUTPUT") else { return };
        let output = PathBuf::from(root).join(self.scenario);
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join("trace.json"), serde_json::to_vec(&self).unwrap()).unwrap();
        fs::write(output.join("steps.txt"), self.text()).unwrap();
    }

    fn text(&self) -> String {
        let mut text = format!("{} · {}\n", self.title, self.scenario);
        for (index, step) in self.steps.iter().enumerate() {
            let Screen { origin, columns, lines, cursor, .. } = &step.frame;
            writeln!(text, "\n--- {index:02} · {} · {columns} × {lines} · origin {}, {} ---", step.label, origin[0], origin[1]).unwrap();
            writeln!(text, "Input: {}", step.input).unwrap();
            if step.pointer.is_some() {
                match &step.hit {
                    Some(hit) => writeln!(text, "Before target: {} {:?}", hit.target, hit.rect).unwrap(),
                    None => writeln!(text, "Before target: none").unwrap(),
                }
            }
            if let Some([x, y]) = cursor {
                writeln!(text, "After text cursor: {x}, {y}").unwrap();
            }
            for effect in &step.effects {
                writeln!(text, "Recorded: {effect}").unwrap();
            }
            for line in step.frame.text() {
                writeln!(text, "{}", line.trim_end()).unwrap();
            }
        }
        text
    }
}

#[test]
fn mixed() {
    let mut replay = Replay::new("mixed", "Compose, read and resume the draft", ViewMode::Vertical);
    let nova = replay.agent_position("nova");
    replay.click("Focus nova", nova);
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(1));

    replay.step("Compose to focused agent", Input::Key(KeyCode::Char('m')));
    assert_eq!(replay.app.ui.mode, InputMode::Compose);
    assert_eq!(replay.app.ui.input, "@nova ");
    assert!(replay.trace.steps.last().unwrap().frame.cursor.is_some());

    let body = "Replay <fixture> · 你好";
    replay.step("Paste message", Input::Paste(body));
    let draft = format!("@nova {body}");
    assert!(replay.shows(&draft));

    let open_reader = replay.target_position(Target::Control(ControlTarget::OpenReader));
    replay.click("Open the focused transcript from compose", open_reader);
    assert!(replay.shows("Transcript · @nova"));
    assert!(replay.shows("Loading transcript"));
    assert_eq!(replay.app.ui.input, draft);

    replay.step("Deliver the fixture transcript", Input::TranscriptReply);
    assert!(replay.shows("Checkpoint 48"));
    let reader = replay.target_position(Target::Reader);
    replay.mouse("Scroll older transcript lines", MouseEventKind::ScrollUp, reader);
    assert!(replay.shows("Checkpoint 45"));
    assert!(!replay.shows("Checkpoint 48"));
    assert_eq!(replay.app.ui.msg_scroll, 0);

    // Moving the parallel agent focus does not retarget the pane or the draft.
    let luna = replay.agent_position("luna");
    replay.click("Move agent focus while the reader stays open", luna);
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(0));
    assert!(replay.shows("Transcript · @nova"));
    let draft_target = replay.target_position(Target::Control(ControlTarget::Text(Field::Compose, draft.len())));
    replay.click("Resume the draft at its last cell", draft_target);
    assert!(replay.app.ui.transcript.is_none());
    assert_eq!(replay.app.ui.mode, InputMode::Compose);
    assert_eq!(replay.app.ui.input, draft);
    assert_eq!(replay.app.ui.input_cursor, draft.len());
    assert!(replay.trace.steps.last().unwrap().frame.cursor.is_some());

    replay.step("Submit message", Input::Key(KeyCode::Enter));
    let recorded = replay.app.recorded_rpc.as_deref().unwrap();
    assert!(matches!(recorded, [RpcOp::ReadTranscript { .. }, RpcOp::Send { recipients, body: sent, intent: None, reply_to: None }]
        if recipients == &["nova"] && sent == body));
    assert_eq!(replay.app.ui.mode, InputMode::Navigate);
    assert!(replay.app.ui.input.is_empty());
    replay.trace.publish();
}

#[test]
fn geometry() {
    let mut replay = Replay::new("geometry", "Resize and resolve fresh targets", ViewMode::Vertical);
    let wide_feed = replay.bounds(|target| *target == Target::Messages);
    let old_nova = replay.agent_position("nova");
    replay.mouse("Press nova in wide layout", MouseEventKind::Down(MouseButton::Left), old_nova);
    replay.step("Narrow the viewport", Input::Resize(62, 17));
    let narrow_feed = replay.bounds(|target| *target == Target::Messages);
    assert_ne!(wide_feed.x, narrow_feed.x);
    // Make sure cancellation, rather than a missed hit, prevents activation.
    assert!(matches!(replay.hit(old_nova), Some((_, Target::Agent(AgentTarget::Focus(ItemKey::Agent { name, .. })))) if name == "nova"));

    replay.mouse("Release the cancelled press", MouseEventKind::Up(MouseButton::Left), old_nova);
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(0));
    let new_nova = replay.agent_position("nova");
    replay.click("Focus nova in narrow layout", new_nova);
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(1));

    let feed = Position::new(narrow_feed.x + 2, narrow_feed.y + 1);
    replay.mouse("Scroll the resized message feed", MouseEventKind::ScrollUp, feed);
    assert!(replay.app.ui.msg_scroll > 0);

    let old_luna = replay.agent_position("luna");
    replay.mouse("Press luna before the data refresh", MouseEventKind::Down(MouseButton::Left), old_luna);
    let mut snapshot = replay.app.data.clone();
    snapshot.agents.rotate_right(1);
    replay.step("Refresh with agents in a different order", Input::Snapshot(Box::new(snapshot)));
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(2));
    assert_eq!(replay.app.data.agents[2].name, "nova");
    assert!(matches!(replay.hit(old_luna), Some((_, Target::Agent(AgentTarget::Focus(ItemKey::Agent { name, .. })))) if name == "ravo"));

    replay.mouse("Release over the replacement row", MouseEventKind::Up(MouseButton::Left), old_luna);
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(2));
    assert_eq!(replay.app.data.agents[2].name, "nova");
    assert!(replay.app.recorded_rpc.as_ref().unwrap().is_empty());
    replay.trace.publish();
}

#[test]
fn modal() {
    let mut replay = Replay::new("modal", "Modal masking and confirmation", ViewMode::Vertical);
    let nova = replay.agent_position("nova");
    replay.step("Open help with the keyboard", Input::Key(KeyCode::Char('?')));
    assert!(replay.app.ui.help_open);
    assert!(!matches!(replay.hit(nova), Some((_, Target::Agent(_)))));
    replay.click("Try the underlying agent row", nova);
    assert!(replay.app.ui.help_open);
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(0));

    replay.step("Dismiss help with Escape", Input::Key(KeyCode::Esc));
    assert!(!replay.app.ui.help_open);
    replay.click("Focus nova after dismissal", nova);
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(1));

    replay.step("Open the kill confirmation", Input::Key(KeyCode::Char('k')));
    assert!(replay.app.ui.confirm.is_some());
    let cancel = replay.target_position(Target::Control(ControlTarget::ConfirmChoice(false)));
    replay.click("Cancel with No", cancel);
    assert!(replay.app.ui.confirm.is_none());
    assert!(replay.app.recorded_rpc.as_ref().unwrap().is_empty());

    replay.step("Reopen the confirmation", Input::Key(KeyCode::Char('k')));
    let confirm = replay.target_position(Target::Control(ControlTarget::ConfirmChoice(true)));
    replay.click("Confirm with Yes", confirm);
    assert!(replay.app.ui.confirm.is_none());
    assert!(matches!(replay.app.recorded_rpc.as_deref(), Some([RpcOp::KillAgent { name }]) if name == "nova"));
    assert_eq!(replay.app.data.agents.len(), 3);
    replay.trace.publish();
}

#[test]
fn inline() {
    let mut replay = Replay::new("inline", "Inline dashboard and transcript detail", ViewMode::Inline);
    assert_eq!(replay.trace.steps[0].frame.origin, [0, INLINE_TOP]);
    let nova = replay.agent_position("nova");
    assert!(nova.y >= INLINE_TOP);
    replay.click("Focus nova at its absolute inline position", nova);
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(1));

    replay.step("Compose to nova in the inline viewport", Input::Key(KeyCode::Char('m')));
    assert_eq!(replay.app.ui.input, "@nova ");
    replay.step("Open transcript with Ctrl+T", Input::Ctrl('t'));
    assert_eq!(replay.trace.steps.last().unwrap().frame.origin, [0, 0]);
    replay.step("Deliver the fixture transcript", Input::TranscriptReply);
    assert!(replay.shows("Transcript · @nova"));
    assert!(replay.shows("Checkpoint 48"));
    assert!(replay.agent_position("nova").y >= replay.bounds(|target| *target == Target::Reader).bottom());

    let close = replay.target_position(Target::Control(ControlTarget::Shortcut(ShortcutScope::Reader, KeyCode::Esc, KeyModifiers::NONE)));
    replay.click("Close detail and return to the inline viewport", close);
    assert_eq!(replay.trace.steps.last().unwrap().frame.origin, [0, INLINE_TOP]);
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(1));
    assert_eq!(replay.app.ui.input, "@nova ");
    assert!(replay.app.ui.transcript.is_none());
    assert!(matches!(replay.app.recorded_rpc.as_deref(), Some([RpcOp::ReadTranscript { .. }])));
    replay.trace.publish();
}

fn info_row(label: &str, value: &str, link: Option<Link>) -> Row {
    Row { label: label.into(), value: value.into(), link, status: None }
}

fn page_link(label: &str, value: &str, page: Page) -> Row {
    info_row(label, value, Some(Link::Page(page)))
}

fn assert_info_subject(request: &InfoRequest, agent: &Agent) {
    assert_eq!(request.1.identity(), (agent.device_name.as_deref(), agent.name.as_str(), agent.created_at.to_bits()));
}

#[test]
fn info_links() {
    const DELIVERY: &str = "/fixture/delivery.md";
    const TASK: &str = "/fixture/replay.task.md";

    let mut replay = Replay::new("info_links", "Read an adopted profile and a Delivery's declared Task", ViewMode::Vertical);
    replay.step("Open Info with i", Input::Key(KeyCode::Char('i')));
    assert!(replay.app.ui.info.as_ref().unwrap().loading());
    let root = replay.request("luna", |page| matches!(page, Page::Agent));
    replay.info_reply("Load the Agent's links", root, vec![page_link("Role", "replay engineer", Page::Role), page_link("Delivery", "Release verification", Page::Task(DELIVERY.into()))]);

    replay.step("Follow the focused Role with Enter", Input::Key(KeyCode::Enter));
    let role = replay.request("luna", |page| matches!(page, Page::Role));
    replay.info_reply("Load the adopted Role", role, vec![page_link("Profile", "offline reader", Page::Profile), info_row("Role", "Verify native input with synthetic fixtures.", None)]);
    assert!(replay.shows("Verify native input"));
    replay.click_info_row("Open the adopted Profile", "offline reader");
    let profile = replay.request("luna", |page| matches!(page, Page::Profile));
    replay.info_reply("Load the adopted Profile", profile, vec![info_row("Model", "fixture model snapshot", None)]);
    assert!(replay.shows("fixture model snapshot"));
    let back = replay.target_position(Target::InfoBack);
    replay.click("Use the painted Back control", back);
    assert!(replay.shows("Verify native input"));
    assert!(!replay.shows("fixture model snapshot"));
    replay.step("Escape back to Agent Info", Input::Key(KeyCode::Esc));
    assert!(replay.info_requests.is_empty(), "Back should use the loaded pages");

    replay.click_info_row("Open the Delivery", "Release verification");
    let delivery = replay.request("luna", |page| matches!(page, Page::Task(path) if path == DELIVERY));
    replay.info_reply("Load Delivery metadata and its declared Task", delivery, vec![page_link("Body", "Read the Delivery", Page::TaskBody(DELIVERY.into())), page_link("Task", "Run replay checks", Page::Task(TASK.into()))]);
    replay.click_info_row("Open the Delivery body", "Read the Delivery");
    let delivery_body = replay.request("luna", |page| matches!(page, Page::TaskBody(path) if path == DELIVERY));
    replay.info_reply("Read the Delivery text", delivery_body, vec![info_row("", "Delivery scope: offline verification.", None)]);
    assert!(replay.shows("Delivery scope: offline verification"));
    replay.step("Return to the Delivery links", Input::Key(KeyCode::Esc));

    replay.click_info_row("Follow the declared Task", "Run replay checks");
    let task = replay.request("luna", |page| matches!(page, Page::Task(path) if path == TASK));
    replay.info_reply("Load the declared Task metadata", task, vec![page_link("Body", "Read the Task", Page::TaskBody(TASK.into()))]);
    replay.click_info_row("Open the Task body", "Read the Task");
    let task_body = replay.request("luna", |page| matches!(page, Page::TaskBody(path) if path == TASK));
    let body = (1..=48).map(|line| format!("Task checkpoint {line:02} · verify the rendered body")).collect::<Vec<_>>().join("\n");
    replay.info_reply("Read the Task text without touching the filesystem", task_body, vec![info_row("", &body, None)]);
    assert!(replay.shows("Task checkpoint 01"));
    assert!(!replay.shows("Task checkpoint 48"));
    replay.step("Jump to the end of the long Task body", Input::Key(KeyCode::End));
    assert!(replay.shows("Task checkpoint 48"));
    assert!(!replay.shows("Task checkpoint 01"));
    replay.finish_info_replay(0);
}

#[test]
fn info_identity() {
    let mut replay = Replay::new("info_identity", "Follow historical launch relations by exact instance", ViewMode::Vertical);
    let mut history = replay.app.data.clone();
    let historical = |name, age| {
        let mut agent = super::test_helpers::make_test_agent(name, age);
        agent.status = AgentStatus::Inactive;
        agent
    };
    let parent = historical("luna", 800.0);
    let sibling = historical("ravo", 400.0);
    history.stopped_agents = vec![parent.clone(), sibling.clone()];
    replay.step("Add stopped records alongside active namesakes", Input::Snapshot(Box::new(history)));

    let original_nova = replay.app.data.agents[1].clone();
    let check_nova = replay.target_position(Target::Agent(AgentTarget::Select(super::agent_mouse::Item::Live(&original_nova).key())));
    replay.click("Check the original nova with the mouse", check_nova);
    let luna = replay.agent_position("luna");
    replay.click("Move focus away from the checked child", luna);
    let mut replacement = replay.app.data.clone();
    let mut stopped_nova = original_nova.clone();
    stopped_nova.status = AgentStatus::Inactive;
    replacement.stopped_agents.push(stopped_nova.clone());
    replacement.agents[1] = super::test_helpers::make_test_agent("nova", 4.0);
    replay.step("Stop the checked nova and replace its live name", Input::Snapshot(Box::new(replacement)));

    replay.step("Open Info for the sole checked Agent", Input::Key(KeyCode::Char('i')));
    let child = replay.request("nova", |page| matches!(page, Page::Agent));
    assert_info_subject(&child, &original_nova);
    assert_ne!(child.1.identity().2, replay.app.data.agents[1].created_at.to_bits());
    replay.info_reply("Load nova's exact parent link", child, vec![info_row("From", "@luna", Some(Link::Agent(Box::new(Subject::from_agent(&parent)))))]);
    replay.click_info_row("Follow the historical parent", "@luna");
    let parent_read = replay.request("luna", |page| matches!(page, Page::Agent));
    assert_info_subject(&parent_read, &parent);
    assert_ne!(parent_read.1.identity().2, replay.app.data.agents[0].created_at.to_bits());
    replay.info_reply(
        "Load the parent's exact child links",
        parent_read,
        vec![info_row("Child", "@nova", Some(Link::Agent(Box::new(Subject::from_agent(&stopped_nova))))), info_row("Child", "@ravo", Some(Link::Agent(Box::new(Subject::from_agent(&sibling)))))],
    );

    let mut replacement = replay.app.data.clone();
    replacement.agents[2] = super::test_helpers::make_test_agent("ravo", 5.0);
    replay.step("Replace the current ravo while the parent's link stays open", Input::Snapshot(Box::new(replacement)));
    replay.click_info_row("Follow the historical sibling, not the new ravo", "@ravo");
    let sibling_read = replay.request("ravo", |page| matches!(page, Page::Agent));
    assert_info_subject(&sibling_read, &sibling);
    assert_ne!(sibling_read.1.identity().2, replay.app.data.agents[2].created_at.to_bits());
    replay.info_reply("Show a marker from the stopped sibling", sibling_read, vec![info_row("Status", "historical sibling record", None)]);
    assert!(replay.shows("historical sibling record"));
    replay.step("Close the entire Info stack with i", Input::Key(KeyCode::Char('i')));
    assert!(replay.app.ui.info.is_none());
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(0));
    assert_eq!(replay.app.ui.selected.iter().map(String::as_str).collect::<Vec<_>>(), ["nova"]);

    let ravo = replay.agent_position("ravo");
    replay.click("Focus ravo for a second check", ravo);
    replay.step("Check a second Agent", Input::Key(KeyCode::Enter));
    assert_eq!(replay.app.ui.selected.len(), 2);
    replay.step("Try Info with two checked Agents", Input::Key(KeyCode::Char('i')));
    assert!(replay.app.ui.info.is_none());
    replay.finish_info_replay(0);
}

#[test]
fn info_context() {
    let mut replay = Replay::new("info_context", "Preserve the draft, transcript, and message position", ViewMode::Vertical);
    let feed = replay.target_position(Target::Messages);
    replay.mouse("Scroll the message feed", MouseEventKind::ScrollUp, feed);
    let feed_scroll = replay.app.ui.msg_scroll;
    assert!(feed_scroll > 0);

    let nova = replay.agent_position("nova");
    replay.click("Focus nova to compose", nova);
    replay.step("Compose to nova", Input::Key(KeyCode::Char('m')));
    replay.step("Type a literal i in the draft", Input::Key(KeyCode::Char('i')));
    assert_eq!(replay.app.ui.input, "@nova i");
    assert!(replay.app.ui.info.is_none());
    replay.step("Add draft text", Input::Paste("nitial reply · 你好"));
    replay.step("Move the caret inside the draft", Input::Key(KeyCode::Left));
    let draft = replay.app.ui.input.clone();
    let caret = replay.app.ui.input_cursor;

    replay.step("Open nova's transcript from compose", Input::Ctrl('t'));
    replay.step("Load the transcript fixture", Input::TranscriptReply);
    let transcript = replay.target_position(Target::Reader);
    replay.mouse("Scroll the transcript away from its latest line", MouseEventKind::ScrollUp, transcript);
    assert!(replay.shows("Checkpoint 45"));
    assert!(!replay.shows("Checkpoint 48"));
    let luna = replay.agent_position("luna");
    replay.click("Focus a different Agent while leaving the transcript pinned", luna);
    assert_eq!(replay.app.cursor_target(), CursorTarget::Agent(0));
    let open = replay.target_position(Target::InfoOpen);
    replay.click("Click the transcript's Info shortcut", open);
    let info = replay.request("nova", |page| matches!(page, Page::Agent));
    assert_info_subject(&info, &replay.app.data.agents[1]);
    replay.info_reply("Load Info for the pinned transcript Agent", info, vec![info_row("Status", "pinned nova details", None)]);
    assert!(replay.shows("pinned nova details"));
    assert_eq!(replay.app.ui.input, draft);
    assert_eq!(replay.app.ui.input_cursor, caret);
    assert_eq!(replay.app.ui.msg_scroll, feed_scroll);
    replay.step("Ignore pasted text while reading Info", Input::Paste("unexpected paste"));
    assert_eq!(replay.app.ui.input, draft);

    replay.step("Close Info with i and reveal the same transcript position", Input::Key(KeyCode::Char('i')));
    assert!(replay.app.ui.info.is_none());
    assert!(replay.shows("Transcript · @nova"));
    assert!(replay.shows("Checkpoint 45"));
    assert!(!replay.shows("Checkpoint 48"));
    replay.step("Escape the transcript and resume composing", Input::Key(KeyCode::Esc));
    assert!(replay.app.ui.transcript.is_none());
    assert_eq!(replay.app.ui.mode, InputMode::Compose);
    assert_eq!(replay.app.ui.input, "@nova initial reply · 你好");
    assert_eq!(replay.app.ui.input_cursor, caret);
    assert_eq!(replay.app.ui.msg_scroll, feed_scroll);
    assert!(replay.trace.steps.last().unwrap().frame.cursor.is_some());
    replay.finish_info_replay(1);
}

#[test]
fn info_resize() {
    let mut replay = Replay::new("info_resize", "Keep compact Info usable through resize and late reads", ViewMode::Inline);
    replay.step("Open search", Input::Key(KeyCode::Char('/')));
    replay.step("Type i as search text", Input::Key(KeyCode::Char('i')));
    assert_eq!(replay.app.active_search_query(), Some("i"));
    assert!(replay.app.ui.info.is_none());
    replay.step("Cancel search", Input::Key(KeyCode::Esc));

    replay.step("Open Info above the Agent strip", Input::Key(KeyCode::Char('i')));
    assert_eq!(replay.trace.steps.last().unwrap().frame.origin, [0, 0]);
    let root = replay.request("luna", |page| matches!(page, Page::Agent));
    let rows = (1..=12).map(|line| info_row("Detail", &format!("Launch detail {line:02} · synthetic text wraps when the inline viewport narrows"), None)).chain([page_link("Role", "compact reader", Page::Role)]).collect();
    replay.info_reply("Load enough Agent details to scroll", root, rows);
    replay.step("Resize the open inline reader", Input::Resize(62, 17));
    let resized = &replay.trace.steps.last().unwrap().frame;
    assert_eq!(resized.columns, 62);
    assert_eq!(resized.origin, [0, 0]);
    let pane_bounds = replay.bounds(|target| *target == Target::InfoPane);
    let strip = resized.text().position(|line| line.contains("nova")).unwrap();
    assert!(pane_bounds.height >= resized.lines / 2 && usize::from(pane_bounds.bottom()) <= strip);
    assert!(replay.shows("nova") && replay.shows("ravo"));
    let pane = replay.target_position(Target::InfoPane);
    replay.mouse("Wheel down inside the resized Info pane", MouseEventKind::ScrollDown, pane);
    assert!(replay.app.ui.info.as_ref().unwrap().scroll > 0);
    replay.step("Scroll the compact Info pane to the Role", Input::Key(KeyCode::End));
    let before_role = replay.app.ui.info.as_ref().unwrap();
    let saved_scroll = before_role.scroll;
    let saved_cursor = before_role.cursor;
    assert!(saved_scroll > 0);
    replay.click_info_row("Open the Role and leave the request unresolved", "compact reader");
    let stale = replay.request("luna", |page| matches!(page, Page::Role));
    replay.step("Back out before the Role arrives", Input::Key(KeyCode::Esc));
    let restored = replay.app.ui.info.as_ref().unwrap();
    assert_eq!(restored.scroll, saved_scroll);
    assert_eq!(restored.cursor, saved_cursor);
    replay.click_info_row("Reopen the same Role", "compact reader");
    let current = replay.request("luna", |page| matches!(page, Page::Role));
    assert_ne!(stale.0, current.0);
    replay.step("Deliver the old failure while the new visit is loading", Input::InfoReply(stale.0, Err("stale role error".into())));
    assert!(replay.app.ui.info.as_ref().unwrap().loading());
    assert!(!replay.shows("stale role error"));
    replay.info_reply("Complete the current Role read", current, vec![info_row("Role", "compact role ready", None)]);
    assert!(replay.shows("compact role ready"));
    assert!(replay.shows("nova") && replay.shows("ravo"));

    replay.step("Close Info with i", Input::Key(KeyCode::Char('i')));
    replay.step("Open the launch form", Input::Key(KeyCode::Tab));
    replay.step("Type i as the launch prompt", Input::Key(KeyCode::Char('i')));
    assert_eq!(replay.app.ui.mode, InputMode::Launch);
    assert_eq!(replay.app.ui.input, "i");
    assert!(replay.app.ui.info.is_none());
    replay.finish_info_replay(0);
}

#[test]
fn quick_launch() {
    use super::{launch::Catalog, model::LaunchField};
    use std::sync::Arc;
    let fixture = tempfile::tempdir().unwrap();
    let scope = fixture.path();
    fs::create_dir(scope.join("roles")).unwrap();
    fs::write(scope.join("roles/coder.toml"), "profile='deep'\nprompt='Review the implementation.'").unwrap();
    fs::write(scope.join("config.toml"), "[launch]\ndefault_profile='daily'\n[launch.profiles.daily]\ntool='claude'\n[launch.profiles.deep]\ntool='codex'\n").unwrap();
    let catalog = Arc::new(Catalog {
        scope: scope.into(),
        cwd: scope.into(),
        environment: [("HOME".into(), scope.to_string_lossy().into_owned()), ("AGENTX_WORKSPACE".into(), "global".into())].into(),
        roles: vec![("coder".into(), "Code review".into())],
        profiles: vec![("deep".into(), "codex".into()), ("daily".into(), "claude".into())],
    });
    for (layout, scenario) in [(ViewMode::Inline, "quick_launch"), (ViewMode::Vertical, "quick_launch_vertical")] {
        let mut r = Replay::new(scenario, "Launch with a Role and an optional Profile override", layout);
        r.app.ui.launch.quick.catalog = Some(catalog.clone());
        r.app.ui.launch.refresh_preview();
        r.step("Open quick launch", Input::Key(KeyCode::Tab));
        assert!(r.shows("daily"));
        r.step("Write the task", Input::Paste("Review the changes"));
        let role = r.target_position(Target::Control(ControlTarget::OpenLaunch(LaunchField::Role)));
        r.click("Choose a Role", role);
        r.step("Filter roles", Input::Paste("coder"));
        let choice = r.target_position(Target::Control(ControlTarget::LaunchChoice(0)));
        r.click("Use coder and its Profile", choice);
        assert!(r.shows("deep (role)"), "{:?}", r.app.ui.launch.quick.error);
        r.step("Use a narrow terminal", Input::Resize(40, 18));
        r.step("Focus Role", Input::Key(KeyCode::Up));
        r.step("Focus Profile", Input::Key(KeyCode::Right));
        r.step("Open profiles", Input::Key(KeyCode::Enter));
        r.step("Search profiles", Input::Paste("daily"));
        r.step("Cancel without changing the draft", Input::Key(KeyCode::Esc));
        assert_eq!(r.app.ui.input, "Review the changes");
        assert!(r.app.ui.launch.quick.profile.is_none());
        let profile = r.target_position(Target::Control(ControlTarget::OpenLaunch(LaunchField::Profile)));
        r.click("Open Profile again", profile);
        r.step("Find daily", Input::Paste("daily"));
        r.step("Override the role Profile", Input::Key(KeyCode::Enter));
        assert_eq!(r.app.ui.launch.quick.profile.as_deref(), Some("daily"));
        // A changed Role definition is authoritative on the next preview.
        fs::write(scope.join("roles/coder.toml"), "profile='missing'\nprompt='Review the implementation.'").unwrap();
        r.click("Inspect Profile selection", r.target_position(Target::Control(ControlTarget::OpenLaunch(LaunchField::Profile))));
        r.step("Choose automatic Profile", Input::Key(KeyCode::Up));
        r.step("Move to Auto", Input::Key(KeyCode::Up));
        r.step("Show missing inherited Profile", Input::Key(KeyCode::Enter));
        assert!(r.app.ui.launch.quick.error.as_deref().unwrap().contains("missing"));
        r.click("Correct the missing Profile", r.target_position(Target::Control(ControlTarget::OpenLaunch(LaunchField::Profile))));
        r.step("Select a valid Profile", Input::Paste("daily"));
        r.step("Apply correction", Input::Key(KeyCode::Enter));
        assert!(r.app.ui.launch.quick.error.is_none());
        fs::write(scope.join("roles/coder.toml"), "profile='deep'\nprompt='Review the implementation.'").unwrap();
        r.step("Launch from the prompt", Input::Key(KeyCode::Enter));
        let [RpcOp::Launch { settings, prompt }] = r.app.recorded_rpc.as_deref().unwrap() else { panic!("expected one launch") };
        assert_eq!(super::rpc::launch_argv(settings, prompt), ["launch", "--role", "coder", "--profile", "daily", "--count", "1", "--no-run-here", "--go", "--agentx-prompt", "Review the changes"]);
        r.trace.publish();
    }
}
