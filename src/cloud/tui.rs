//! Keyboard-driven deployment administration; the cloud HTTP service owns all writes.

mod control;

use std::io::{IsTerminal, stdin, stdout};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::client::http::{Http, HttpError};
use super::communication::{AgentView, HostView};
use crate::tui::theme::{Theme, palette};

const ADMIN: &str = "/api/v1/admin";
const PAGE_SIZE: usize = 50;

pub(super) fn run(url: &str) -> Result<()> {
    if !stdin().is_terminal() || !stdout().is_terminal() {
        bail!("Deployment administration requires an interactive terminal");
    }
    let token = std::env::var("AGENTX_ADMIN_TOKEN").context("Set AGENTX_ADMIN_TOKEN to the deployment admin token")?;
    let http = Arc::new(Http::new(url, &token)?);
    enable_raw_mode()?;
    let _restore = RestoreTerminal;
    crossterm::execute!(stdout(), EnterAlternateScreen, EnableBracketedPaste, crossterm::cursor::Hide)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    session(terminal, http, url)
}

struct RestoreTerminal;

impl Drop for RestoreTerminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(stdout(), DisableBracketedPaste, crossterm::style::SetAttribute(crossterm::style::Attribute::Reset), crossterm::style::ResetColor, crossterm::cursor::Show, LeaveAlternateScreen);
    }
}

#[derive(Deserialize)]
struct Page<T> {
    items: Vec<T>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct Overview {
    deployment_id: String,
    protocol_version: u32,
    scopes: usize,
    hosts: usize,
    online_hosts: usize,
    agents: usize,
    unavailable: usize,
}

#[derive(Deserialize)]
struct Scope {
    id: String,
    scope_url: String,
    credential_version: u32,
    generation: String,
    hosts: usize,
    online_hosts: usize,
    agents: usize,
    available: bool,
}

#[derive(Clone, Default)]
enum Inventory {
    #[default]
    Hosts,
    Agents(Option<String>),
}

enum Rows {
    Hosts(Page<HostView>),
    Agents(Page<AgentView>),
}

struct Detail {
    scope: Scope,
    kind: Inventory,
    rows: Option<Rows>,
    cursor: ListState,
}

#[derive(Clone)]
struct Change {
    id: String,
    request_id: Uuid,
    token: String,
    expected_version: Option<u32>,
}

enum Phase {
    Draft,
    Rejected(String),
    Unknown,
    Saved(u32),
}

struct Credential {
    change: Change,
    visible: bool,
    revealed: bool,
    phase: Phase,
    error: Option<String>,
}

enum Job {
    Refresh,
    Scopes(String),
    Inspect(String, Inventory),
    Inventory(String, Inventory, Option<String>),
    Write(Change),
    Control(Box<control::Job>),
}

enum Update {
    Snapshot(Overview, Page<Scope>),
    Scopes(Page<Scope>),
    Detail(Scope, Inventory, Result<Rows>),
    Inventory(Inventory, Rows, bool),
    Written(std::result::Result<u32, Phase>),
    Control(control::Update),
}

#[derive(Default)]
struct App {
    endpoint: String,
    overview: Option<Overview>,
    scopes: Page<Scope>,
    cursor: ListState,
    detail: Option<Detail>,
    credential: Option<Credential>,
    control: Option<control::Dialog>,
    notice: Option<String>,
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self { items: Vec::new(), next: None }
    }
}

impl<T> Page<T> {
    fn append(&mut self, mut more: Self) {
        self.items.append(&mut more.items);
        self.next = more.next;
    }
}

impl Inventory {
    fn label(&self) -> &'static str {
        match self {
            Self::Hosts => "Hosts",
            Self::Agents(_) => "Agents",
        }
    }

    fn other(&self) -> Self {
        match self {
            Self::Hosts => Self::Agents(None),
            Self::Agents(_) => Self::Hosts,
        }
    }

    fn host(&self) -> Option<&str> {
        match self {
            Self::Agents(Some(host)) => Some(host),
            _ => None,
        }
    }
}

impl Rows {
    fn next(&self) -> Option<&str> {
        match self {
            Self::Hosts(page) => page.next.as_deref(),
            Self::Agents(page) => page.next.as_deref(),
        }
    }

    fn append(&mut self, more: Self) {
        match (self, more) {
            (Self::Hosts(current), Self::Hosts(more)) => current.append(more),
            (Self::Agents(current), Self::Agents(more)) => current.append(more),
            _ => unreachable!("one inventory request runs at a time"),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Hosts(page) => page.items.len(),
            Self::Agents(page) => page.items.len(),
        }
    }
}

impl Job {
    fn execute(self, http: &Http) -> Result<Update> {
        let get_scopes = |after: Option<&str>| http.request("GET", &page_path(&format!("{ADMIN}/scopes"), after), None);
        let get_inventory = |id: &str, kind: &Inventory, after: Option<&str>| {
            let mut path = page_path(&format!("{}/{}", scope_path(id), kind.label().to_ascii_lowercase()), after);
            if let Some(host) = kind.host() {
                path.push_str("&host=");
                path.push_str(&encode(host));
            }
            match kind {
                Inventory::Hosts => http.request("GET", &path, None).map(Rows::Hosts),
                Inventory::Agents(_) => http.request("GET", &path, None).map(Rows::Agents),
            }
        };
        Ok(match self {
            Self::Refresh => Update::Snapshot(http.request("GET", &format!("{ADMIN}/overview"), None)?, get_scopes(None)?),
            Self::Scopes(after) => Update::Scopes(get_scopes(Some(&after))?),
            Self::Inspect(id, kind) => {
                let scope = http.request("GET", &scope_path(&id), None)?;
                let rows = get_inventory(&id, &kind, None);
                Update::Detail(scope, kind, rows)
            }
            Self::Inventory(id, kind, after) => {
                let rows = get_inventory(&id, &kind, after.as_deref())?;
                Update::Inventory(kind, rows, after.is_some())
            }
            Self::Write(change) => Update::Written(change.submit(http)),
            Self::Control(job) => Update::Control(job.execute(http)),
        })
    }
}

fn scope_path(id: &str) -> String {
    format!("{ADMIN}/scopes/{}", encode(id))
}

fn encode(value: &str) -> String {
    const COMPONENT: &percent_encoding::AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.').remove(b'~');
    utf8_percent_encode(value, COMPONENT).to_string()
}

fn page_path(path: &str, after: Option<&str>) -> String {
    let mut url = format!("{path}?limit={PAGE_SIZE}");
    if let Some(cursor) = after {
        url.push_str("&after=");
        url.push_str(&encode(cursor));
    }
    url
}

// Display status and recovery guidance without echoing an upstream response body.
fn http_error(error: &anyhow::Error) -> String {
    match error.downcast_ref::<HttpError>().map(|error| error.status) {
        Some(401 | 403) => "Access denied. Check AGENTX_ADMIN_TOKEN.".into(),
        Some(404) => "Resource not found. Refresh the deployment.".into(),
        Some(409 | 412) => "Conflict. Refresh before trying again.".into(),
        Some(503) => "Service unavailable. Try again when it recovers.".into(),
        Some(status) => format!("The deployment returned HTTP {status}."),
        None => "No usable reply from the deployment. Check the URL and connection.".into(),
    }
}

impl Change {
    fn submit(self, http: &Http) -> std::result::Result<u32, Phase> {
        #[derive(Deserialize)]
        struct Receipt {
            id: String,
            credential_version: u32,
        }

        let mut path = scope_path(&self.id);
        let mut payload = json!({"request_id": self.request_id.to_string(), "token": self.token});
        let method = if let Some(version) = self.expected_version {
            path.push_str("/token-rotations");
            payload["expected_version"] = json!(version);
            "POST"
        } else {
            "PUT"
        };
        match http.request::<Receipt>(method, &path, Some(&payload)) {
            Ok(receipt) if receipt.id == self.id => Ok(receipt.credential_version),
            Ok(_) => Err(Phase::Unknown),
            Err(error) => {
                let reason = http_error(&error);
                match error.downcast_ref::<HttpError>().map(|error| error.status) {
                    Some(status @ 400..=499) if !matches!(status, 408 | 425 | 429) => Err(Phase::Rejected(reason)),
                    _ => Err(Phase::Unknown),
                }
            }
        }
    }
}

fn id_char(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'
}

impl Credential {
    fn new(scope: Option<&Scope>) -> Self {
        let mut entropy = [0; 24];
        rand::fill(&mut entropy);
        let change = Change { id: scope.map_or_else(String::new, |scope| scope.id.clone()), request_id: Uuid::new_v4(), token: URL_SAFE_NO_PAD.encode(entropy), expected_version: scope.map(|scope| scope.credential_version) };
        Self { change, visible: false, revealed: false, phase: Phase::Draft, error: None }
    }

    fn accepts_id(&self) -> bool {
        matches!(self.phase, Phase::Draft) && self.change.expected_version.is_none()
    }

    fn input_id(&mut self, text: &str) {
        if self.accepts_id() {
            if self.change.id.len() + text.len() <= 63 && text.chars().all(id_char) {
                self.change.id.push_str(text);
                self.error = None;
            } else {
                self.error = Some("Use up to 63 lowercase letters, digits or hyphens.".into());
            }
        }
    }

    fn submit(&mut self) -> Option<Job> {
        if matches!(self.phase, Phase::Unknown) {
            self.visible = false;
            return Some(Job::Write(self.change.clone()));
        }
        if !matches!(self.phase, Phase::Draft) {
            return None;
        }
        if super::validate_id(&self.change.id).is_err() {
            self.error = Some("Use a scope ID that starts and ends with a letter or digit.".into());
        } else if !self.revealed {
            self.error = Some("Reveal and save the token before submitting.".into());
        } else {
            self.error = None;
            self.visible = false;
            return Some(Job::Write(self.change.clone()));
        }
        None
    }
}

impl App {
    fn apply(&mut self, update: Result<Update>) {
        match update {
            Err(error) => self.notice = Some(http_error(&error)),
            Ok(Update::Snapshot(overview, scopes)) => {
                let selected = self.selected_scope().map(|scope| scope.id.as_str());
                let next = scopes.items.iter().position(|scope| Some(scope.id.as_str()) == selected);
                self.cursor = ListState::default().with_selected(next.or_else(|| first(scopes.items.len())));
                self.overview = Some(overview);
                self.scopes = scopes;
            }
            Ok(Update::Scopes(more)) => {
                self.scopes.append(more);
                self.cursor.select(self.cursor.selected().or_else(|| first(self.scopes.items.len())));
            }
            Ok(Update::Detail(scope, kind, rows)) => {
                let rows = rows.map_err(|error| self.notice = Some(http_error(&error))).ok();
                let cursor = ListState::default().with_selected(first(rows.as_ref().map_or(0, Rows::len)));
                self.detail = Some(Detail { scope, kind, rows, cursor });
            }
            Ok(Update::Inventory(kind, rows, append)) => {
                if let Some(detail) = &mut self.detail {
                    if let Some(current) = &mut detail.rows
                        && append
                    {
                        current.append(rows);
                    } else {
                        let previous_host = match (&rows, detail.kind.host()) {
                            (Rows::Hosts(page), Some(host)) => page.items.iter().position(|row| row.name == host),
                            _ => None,
                        };
                        detail.cursor = ListState::default().with_selected(previous_host.or_else(|| first(rows.len())));
                        detail.rows = Some(rows);
                    }
                    detail.kind = kind;
                }
            }
            Ok(Update::Written(outcome)) => {
                if let Some(dialog) = &mut self.credential {
                    dialog.phase = match outcome {
                        Ok(version) => Phase::Saved(version),
                        Err(_) if matches!(dialog.phase, Phase::Unknown) => Phase::Unknown,
                        Err(phase) => phase,
                    };
                }
            }
            Ok(Update::Control(update)) => {
                if let Some(dialog) = &mut self.control {
                    dialog.apply(update);
                }
            }
        }
    }

    fn selected_scope(&self) -> Option<&Scope> {
        self.scopes.items.get(self.cursor.selected()?)
    }

    fn target(&self) -> Option<&Scope> {
        self.detail.as_ref().map(|detail| &detail.scope).or_else(|| self.selected_scope())
    }

    fn key(&mut self, key: KeyEvent) -> Option<Job> {
        if let Some(dialog) = &mut self.control {
            if key.code == KeyCode::Esc {
                self.control = None;
                return None;
            }
            return dialog.key(key).map(|job| Job::Control(Box::new(job)));
        }
        if let Some(dialog) = &mut self.credential {
            return match key.code {
                KeyCode::F(2) | KeyCode::Tab => {
                    dialog.visible = !dialog.visible;
                    dialog.revealed |= dialog.visible;
                    dialog.error = None;
                    None
                }
                KeyCode::Enter if matches!(dialog.phase, Phase::Saved(_)) => self.close_dialog(),
                KeyCode::Enter => dialog.submit(),
                KeyCode::Esc => self.close_dialog(),
                KeyCode::Backspace if dialog.accepts_id() => {
                    dialog.change.id.pop();
                    dialog.error = None;
                    None
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) && dialog.accepts_id() => {
                    dialog.change.id.clear();
                    dialog.error = None;
                    None
                }
                KeyCode::Char(ch) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                    dialog.input_id(ch.encode_utf8(&mut [0; 4]));
                    None
                }
                _ => None,
            };
        }
        if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
            return None;
        }
        match key.code {
            KeyCode::Char('l') | KeyCode::Char('s') | KeyCode::Char('o') => return self.open_control(key.code),
            KeyCode::Char('n') if self.detail.is_none() => self.credential = Some(Credential::new(None)),
            KeyCode::Char('r') => {
                if let Some(scope) = self.target() {
                    self.credential = Some(Credential::new(Some(scope)));
                }
            }
            KeyCode::Char('f') => return Some(self.detail.as_ref().map_or(Job::Refresh, |detail| Job::Inspect(detail.scope.id.clone(), detail.kind.clone()))),
            KeyCode::Enter | KeyCode::Right if self.detail.is_none() => return self.selected_scope().map(|scope| Job::Inspect(scope.id.clone(), Inventory::Hosts)),
            KeyCode::Enter => {
                let detail = self.detail.as_ref()?;
                if let Rows::Hosts(hosts) = detail.rows.as_ref()? {
                    let host = hosts.items.get(detail.cursor.selected()?)?;
                    return Some(Job::Inventory(detail.scope.id.clone(), Inventory::Agents(Some(host.name.clone())), None));
                }
            }
            KeyCode::Esc | KeyCode::Left if self.detail.is_some() => {
                self.detail = None;
                self.notice = None;
            }
            KeyCode::Tab | KeyCode::Right if self.detail.is_some() => {
                let detail = self.detail.as_ref()?;
                return Some(Job::Inventory(detail.scope.id.clone(), detail.kind.other(), None));
            }
            KeyCode::Char(']') => {
                return match &self.detail {
                    Some(detail) => Some(Job::Inventory(detail.scope.id.clone(), detail.kind.clone(), Some(detail.rows.as_ref()?.next()?.into()))),
                    None => self.scopes.next.clone().map(Job::Scopes),
                };
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown => {
                if let Some(detail) = &mut self.detail {
                    move_cursor(&mut detail.cursor, detail.rows.as_ref().map_or(0, Rows::len), key.code);
                } else {
                    move_cursor(&mut self.cursor, self.scopes.items.len(), key.code);
                }
            }
            _ => {}
        }
        None
    }

    fn open_control(&mut self, key: KeyCode) -> Option<Job> {
        let scope = self.target()?;
        let target = control::ScopeTarget { id: scope.id.clone(), generation: scope.generation.clone() };
        let dialog = if key == KeyCode::Char('o') {
            control::Dialog::lookup(target)
        } else {
            let detail = self.detail.as_ref()?;
            let index = detail.cursor.selected()?;
            match (key, detail.rows.as_ref()?) {
                (KeyCode::Char('l'), Rows::Hosts(page)) => control::Dialog::launch(target, page.items.get(index)?.name.clone()),
                (KeyCode::Char('s'), Rows::Agents(page)) => control::Dialog::stop(target, page.items.get(index)?.clone()),
                _ => return None,
            }
        };
        let load = dialog.load().map(|job| Job::Control(Box::new(job)));
        self.control = Some(dialog);
        load
    }

    fn close_dialog(&mut self) -> Option<Job> {
        let dialog = self.credential.take()?;
        if matches!(dialog.phase, Phase::Saved(_) | Phase::Unknown) {
            // Re-enter from a fresh scope snapshot before offering another rotation.
            self.detail = None;
            return Some(Job::Refresh);
        }
        None
    }
}

fn first(len: usize) -> Option<usize> {
    (len > 0).then_some(0)
}

fn move_cursor(cursor: &mut ListState, len: usize, key: KeyCode) {
    let Some(last) = len.checked_sub(1) else {
        return;
    };
    let current = cursor.selected().unwrap_or_default();
    cursor.select(Some(match key {
        KeyCode::Home => 0,
        KeyCode::End => last,
        KeyCode::Up => current.saturating_sub(1),
        KeyCode::Down => current.saturating_add(1).min(last),
        KeyCode::PageUp => current.saturating_sub(10),
        KeyCode::PageDown => current.saturating_add(10).min(last),
        _ => current,
    }));
}

fn start(http: &Arc<Http>, job: Job) -> mpsc::Receiver<Result<Update>> {
    let (send, receive) = mpsc::sync_channel(1);
    let http = Arc::clone(http);
    std::thread::spawn(move || {
        let _ = send.send(job.execute(&http));
    });
    receive
}

fn session(mut terminal: Terminal<CrosstermBackend<std::io::Stdout>>, http: Arc<Http>, url: &str) -> Result<()> {
    let mut app = App { endpoint: url.to_owned(), ..App::default() };
    let mut pending = Some(start(&http, Job::Refresh));
    loop {
        if let Some(receive) = &pending {
            match receive.try_recv() {
                Ok(result) => {
                    pending = None;
                    app.apply(result);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    pending = None;
                    // A worker interrupted after writing may have applied the mutation.
                    if let Some(dialog) = &mut app.control {
                        dialog.worker_interrupted();
                    } else {
                        app.apply(if app.credential.is_some() { Ok(Update::Written(Err(Phase::Unknown))) } else { Err(anyhow::anyhow!("deployment request worker stopped")) });
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let mut too_small = false;
        terminal.draw(|frame| {
            let area = frame.area();
            too_small = area.width < 36 || area.height < 14;
            if too_small {
                frame.render_widget(Paragraph::new("Resize to 36 × 14. Ctrl+C exits.").wrap(Wrap { trim: true }), area);
            } else {
                render(frame, &mut app, pending.is_some());
            }
        })?;
        if pending.is_some() && !event::poll(Duration::from_millis(60))? {
            continue;
        }
        let event = event::read()?;
        if pending.is_some() {
            continue;
        }
        match event {
            Event::Paste(text) if !too_small => {
                if let Some(dialog) = &mut app.credential {
                    dialog.input_id(&text);
                } else if let Some(dialog) = &mut app.control {
                    dialog.input(&text);
                }
            }
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) || app.credential.is_none() && app.control.is_none() && app.detail.is_none() && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                    return Ok(());
                }
                if !too_small && let Some(job) = app.key(key) {
                    app.notice = None;
                    pending = Some(start(&http, job));
                }
            }
            _ => {}
        }
    }
}
const SELECTION: Style = Style::new().fg(palette::FG).bg(palette::SELECTION).add_modifier(Modifier::BOLD);

fn render(frame: &mut Frame<'_>, app: &mut App, pending: bool) {
    let area = frame.area();
    let [title, overview, workspace, notice, keys] = Layout::vertical([Constraint::Length(1), Constraint::Length(2), Constraint::Min(4), Constraint::Length(2), Constraint::Length(2)]).areas(area);
    let heading = app.overview.as_ref().map_or_else(|| " AgentX / Deployment".to_owned(), |data| format!(" AgentX / Deployment · API v{}", data.protocol_version));
    frame.render_widget(Paragraph::new(format!("{heading}  {}", app.endpoint)).style(Theme::title()), title);
    if let Some(data) = &app.overview {
        let scope_label = if data.scopes == 1 { "scope" } else { "scopes" };
        let agent_label = if data.agents == 1 { "agent" } else { "agents" };
        let mut text = format!(" {} {scope_label} · {} unavailable   {}/{} hosts online   {} {agent_label}", data.scopes, data.unavailable, data.online_hosts, data.hosts, data.agents);
        if overview.width >= 80 {
            text.push_str(&format!("\n Deployment ID: {}", display(&data.deployment_id)));
        }
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), overview);
    }

    if let Some(detail) = &mut app.detail {
        if workspace.width >= 90 {
            let [scopes, selected] = Layout::horizontal([Constraint::Percentage(44), Constraint::Percentage(56)]).areas(workspace);
            render_scopes(frame, scopes, &app.scopes, &mut app.cursor, false, app.overview.is_some());
            render_detail(frame, selected, detail);
        } else {
            render_detail(frame, workspace, detail);
        }
    } else {
        render_scopes(frame, workspace, &app.scopes, &mut app.cursor, true, app.overview.is_some());
    }

    if pending {
        frame.render_widget(Paragraph::new(" Working…").style(Theme::title()), notice);
    } else if let Some(message) = &app.notice {
        frame.render_widget(Paragraph::new(message.as_str()).style(Theme::error()).wrap(Wrap { trim: true }), notice);
    }
    let (keys_primary, keys_secondary) = match &app.detail {
        Some(detail) if matches!(detail.kind, Inventory::Hosts) => ("Esc Back  Tab Agents  Enter Filter", "l Launch  o Receipt  r Rotate  f Refresh"),
        Some(_) => ("Esc Back  Tab Hosts  ↑↓ Browse", "s Stop  o Receipt  r Rotate  f Refresh"),
        None => ("↑↓ Select   Enter Open   q Quit", "n Create  o Receipt  r Rotate  f Refresh"),
    };
    let more = match &app.detail {
        Some(detail) => detail.rows.as_ref().and_then(Rows::next).is_some(),
        None => app.scopes.next.is_some(),
    };
    let shortcuts = if more && keys.width < 48 && app.detail.is_none() { format!("↑↓ Select   Enter Open   ] More\n{keys_secondary}") } else { format!("{keys_primary}{}\n{keys_secondary}", if more { "   ] More" } else { "" }) };
    if !pending {
        frame.render_widget(Paragraph::new(shortcuts), keys);
    }

    if let Some(dialog) = &app.credential {
        render_credential(frame, area, dialog, pending);
    }
    if let Some(dialog) = &app.control {
        dialog.render(frame, area, pending);
    }
}

fn render_scopes(frame: &mut Frame<'_>, area: Rect, page: &Page<Scope>, cursor: &mut ListState, focused: bool, loaded: bool) {
    let block = Block::bordered().title(if focused { " Scopes › " } else { " Scopes " });
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if page.items.is_empty() {
        let text = if loaded { "No scopes. Press n to create one." } else { "Scope list not loaded." };
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), inner);
        return;
    }

    let [columns, rows] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    frame.render_widget(Paragraph::new("   Hosts  Agents  Scope").style(Style::new().add_modifier(Modifier::BOLD)), columns);
    let items = page.items.iter().map(|scope| {
        let counts = if scope.available { format!("{:>3}/{:<3} {:>5}    ", scope.online_hosts, scope.hosts, scope.agents) } else { "      —     —  ! ".to_owned() };
        let text = format!("{counts}{}", display(&scope.id));
        ListItem::new(text)
    });
    let list = List::new(items).highlight_symbol("› ").highlight_style(if focused { SELECTION } else { Style::new().add_modifier(Modifier::BOLD) });
    frame.render_stateful_widget(list, rows, cursor);
}

fn render_detail(frame: &mut Frame<'_>, area: Rect, detail: &mut Detail) {
    let scope = &detail.scope;
    let block = Block::bordered().title(format!(" Scope › {} ", display(&scope.id)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let info_height = if inner.width < 68 && inner.height >= 8 { 5 } else { 3 };
    let [info, tabs, rows] = Layout::vertical([Constraint::Length(info_height), Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    let status = if scope.available { "Available" } else { "Unavailable" };
    let metadata = format!("{status} · token v{}\nGeneration: {}\n{}", scope.credential_version, display(&scope.generation), display(&scope.scope_url));
    frame.render_widget(Paragraph::new(metadata).wrap(Wrap { trim: true }), info);

    let tabs_text = match &detail.kind {
        Inventory::Hosts => "[Hosts]   Agents".to_owned(),
        Inventory::Agents(None) => " Hosts   [Agents]".to_owned(),
        Inventory::Agents(Some(host)) => format!(" Hosts   [Agents on {}]", display(host)),
    };
    frame.render_widget(Paragraph::new(tabs_text).style(Theme::title()), tabs);
    let items = match &detail.rows {
        Some(Rows::Hosts(page)) => page
            .items
            .iter()
            .map(|host| {
                let status = if host.online { "Online " } else { "Offline" };
                let seen = chrono::DateTime::from_timestamp(host.last_seen, 0).map(|time| time.format("%Y-%m-%d %H:%M UTC").to_string()).unwrap_or_else(|| host.last_seen.to_string());
                ListItem::new(format!("{status}  {}  · seen {seen}", display(&host.name)))
            })
            .collect::<Vec<_>>(),
        Some(Rows::Agents(page)) => page.items.iter().map(|agent| ListItem::new(format!("@{}/{}  · {}", display(&agent.host), display(&agent.name), display(&agent.incarnation)))).collect(),
        None => {
            frame.render_widget(Paragraph::new("Inventory unavailable."), rows);
            return;
        }
    };
    if items.is_empty() {
        frame.render_widget(Paragraph::new(format!("No {} reported.", detail.kind.label().to_ascii_lowercase())), rows);
    } else {
        frame.render_stateful_widget(List::new(items).highlight_symbol("› ").highlight_style(SELECTION), rows, &mut detail.cursor);
    }
}

fn render_credential(frame: &mut Frame<'_>, area: Rect, dialog: &Credential, pending: bool) {
    let width = area.width.min(76);
    let height = (area.height.saturating_sub(2)).min(13);
    let modal = Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height);
    let change = &dialog.change;
    let title = if change.expected_version.is_some() { " Rotate scope token " } else { " Create scope " };
    let block = Block::bordered().title(Line::styled(title, Theme::title()));
    let inner = block.inner(modal);
    frame.render_widget(Clear, modal);
    frame.render_widget(block, modal);

    let [target, token, result, keys] = Layout::vertical([Constraint::Length(3), Constraint::Length(2), Constraint::Min(2), Constraint::Length(2)]).areas(inner);
    let target_text = match change.expected_version {
        Some(version) => format!("Scope: {}\nToken to replace: v{version}", display(&change.id)),
        None if dialog.accepts_id() => format!("Scope ID: {}▏", change.id),
        None => format!("Scope: {}", change.id),
    };
    frame.render_widget(Paragraph::new(target_text).wrap(Wrap { trim: true }), target);
    let secret = if dialog.visible { change.token.as_str() } else { "••••••••••••••••••••••••••••••••" };
    frame.render_widget(Paragraph::new(format!("New communication token\n{secret}")), token);

    let message = if pending {
        "Submitting…".into()
    } else if let Some(error) = &dialog.error {
        error.clone()
    } else {
        match &dialog.phase {
            Phase::Draft if dialog.revealed => "Save the token before submitting.".into(),
            Phase::Draft => "Reveal and save the token.".into(),
            Phase::Rejected(reason) => reason.clone(),
            Phase::Unknown => format!("Outcome unknown. Request:\n{}", change.request_id),
            Phase::Saved(version) => format!("Saved · credential v{version}"),
        }
    };
    let is_error = !pending && (dialog.error.is_some() || matches!(dialog.phase, Phase::Rejected(_) | Phase::Unknown));
    let style = if is_error { Theme::error() } else { Style::default() };
    frame.render_widget(Paragraph::new(message).style(style).wrap(Wrap { trim: true }), result);

    let toggle = if dialog.visible { "Tab Hide" } else { "Tab Reveal" };
    let action = match dialog.phase {
        Phase::Draft if dialog.revealed => "Enter Submit   Esc Cancel",
        Phase::Draft => "Esc Cancel",
        Phase::Rejected(_) => "Esc Close",
        Phase::Unknown => "Enter Retry   Esc Close",
        Phase::Saved(_) => "Enter Done",
    };
    if !pending {
        frame.render_widget(Paragraph::new(format!("{toggle}\n{action}")).wrap(Wrap { trim: true }), keys);
    }
}

fn display(value: &str) -> String {
    value.chars().map(|ch| if ch.is_control() { '�' } else { ch }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::{Json, Router, routing::put};
    use ratatui::backend::TestBackend;
    use serde_json::Value;
    use std::sync::Mutex;

    fn press(app: &mut App, key: KeyCode) -> Option<Job> {
        app.key(KeyEvent::new(key, KeyModifiers::NONE))
    }

    fn screen(app: &mut App, pending: bool) -> String {
        let mut terminal = Terminal::new(TestBackend::new(38, 14)).unwrap();
        terminal.draw(|frame| render(frame, app, pending)).unwrap();
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height).map(|y| (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn lost_create_result_retries_the_original_http_request_and_keeps_token_masked() {
        const ADMIN_BEARER: &str = "synthetic-deployment-admin-for-test";
        type Captured = Arc<Mutex<Vec<Value>>>;

        async fn handler(State(captured): State<Captured>, headers: HeaderMap, Json(body): Json<Value>) -> (StatusCode, Json<Value>) {
            if headers.get("authorization").and_then(|value| value.to_str().ok()) != Some(&format!("Bearer {ADMIN_BEARER}")) {
                return (StatusCode::UNAUTHORIZED, Json(json!({"code":"unauthorized","message":"invalid bearer"})));
            }
            let mut rows = captured.lock().unwrap();
            rows.push(body);
            match rows.len() {
                1 => (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"code":"unavailable","message":"lost reply"}))),
                2 => (StatusCode::CONFLICT, Json(json!({"code":"conflict","message":"cannot verify original write"}))),
                _ => (StatusCode::OK, Json(json!({"id":"lab","credential_version":1,"generation":"fixture","scope_url":"http://127.0.0.1/scopes/lab/api/v1"}))),
            }
        }

        let captured = Captured::default();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let listener = runtime.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
        let http = Http::new(&format!("http://{}", listener.local_addr().unwrap()), ADMIN_BEARER).unwrap();
        let router = Router::new().route("/api/v1/admin/scopes/lab", put(handler)).with_state(Arc::clone(&captured));
        runtime.spawn(async move { axum::serve(listener, router).await.unwrap() });

        let mut app = App::default();
        assert!(press(&mut app, KeyCode::Char('n')).is_none());
        let dialog = app.credential.as_mut().unwrap();
        dialog.input_id("lab");
        let token = dialog.change.token.clone();
        assert!(!screen(&mut app, false).contains(&token));
        assert!(press(&mut app, KeyCode::Enter).is_none());
        assert!(captured.lock().unwrap().is_empty());

        press(&mut app, KeyCode::Tab);
        assert!(screen(&mut app, false).contains(&token));
        for reply in 1..=3 {
            let job = press(&mut app, KeyCode::Enter).unwrap();
            assert!(!screen(&mut app, true).contains(&token));
            app.apply(job.execute(&http));
            let shown = screen(&mut app, false);
            assert!(!shown.contains(&token));
            assert!(shown.contains(if reply < 3 { "Outcome unknown" } else { "Saved · credential v1" }));
        }

        let rows = captured.lock().unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.windows(2).all(|pair| pair[0] == pair[1]));
        assert_eq!(rows[0]["token"], token);
        assert_eq!(rows[0]["request_id"], app.credential.as_ref().unwrap().change.request_id.to_string());
        assert!(rows[0].get("expected_version").is_none());
        press(&mut app, KeyCode::Tab);
        assert!(screen(&mut app, false).contains(&token));
        assert!(matches!(press(&mut app, KeyCode::Enter), Some(Job::Refresh)));
    }

    #[test]
    fn refresh_keeps_scope_identity_and_rotation_uses_its_latest_observed_version() {
        fn snapshot(ids: &[(&str, u32)]) -> Result<Update> {
            let items = ids
                .iter()
                .map(|&(id, version)| Scope { id: id.to_string(), scope_url: format!("https://cloud.example/scopes/{id}/api/v1"), credential_version: version, generation: "fixture".into(), hosts: 0, online_hosts: 0, agents: 0, available: true })
                .collect();
            Ok(Update::Snapshot(Overview { deployment_id: "fixture".into(), protocol_version: 1, scopes: ids.len(), unavailable: 0, hosts: 0, online_hosts: 0, agents: 0 }, Page { items, next: None }))
        }

        let mut app = App::default();
        app.apply(snapshot(&[("a", 1), ("lab", 5)]));
        press(&mut app, KeyCode::Down);
        app.apply(snapshot(&[("lab", 6), ("a", 1)]));
        press(&mut app, KeyCode::Char('r'));
        let shown = screen(&mut app, false);
        assert!(shown.contains("Scope: lab"));
        assert!(shown.contains("Token to replace: v6"));
        press(&mut app, KeyCode::Tab);
        let Some(Job::Write(change)) = press(&mut app, KeyCode::Enter) else { panic!("expected rotation") };
        assert_eq!((change.id.as_str(), change.expected_version), ("lab", Some(6)));
    }
}
