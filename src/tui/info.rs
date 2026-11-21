//! Per-view Agent Info navigation. A visit retains its subject, focus and visual
//! scroll independently of both the dashboard cursor and the message draft.
use std::sync::mpsc::{self, Receiver, Sender};

use crossterm::event::KeyCode;
use uuid::Uuid;

use super::app::App;
use super::info_data::{self, Link, Page, Row, Subject};
use super::model::{Agent, CursorTarget};

type Request = (Uuid, Subject, Page);
type Reply = (Uuid, Result<Vec<Row>, String>);

#[derive(Default)]
pub(super) enum Worker {
    #[default]
    Idle,
    Active(Sender<Request>, Receiver<Reply>),
    #[cfg(test)]
    Recording(Vec<Request>),
}

impl Worker {
    fn submit(&mut self, request: Request) -> Result<(), String> {
        if matches!(self, Self::Idle) {
            let (send, receive) = mpsc::channel::<Request>();
            let (reply, replies) = mpsc::channel();
            std::thread::Builder::new()
                .name("agent-info".into())
                .spawn(move || {
                    while let Ok(request) = receive.recv() {
                        // Only the most recently requested page can still be visible.
                        let (id, subject, page) = receive.try_iter().last().unwrap_or(request);
                        let result = std::panic::catch_unwind(|| info_data::read(&subject, &page)).unwrap_or_else(|_| Err("Info reader failed".into()));
                        if reply.send((id, result)).is_err() {
                            break;
                        }
                    }
                })
                .map_err(|e| e.to_string())?;
            *self = Self::Active(send, replies);
        }
        match self {
            Self::Active(send, _) => send.send(request).map_err(|_| "Info reader disconnected".into()),
            #[cfg(test)]
            Self::Recording(requests) => {
                requests.push(request);
                Ok(())
            }
            Self::Idle => unreachable!(),
        }
    }

    #[cfg(test)]
    pub(super) fn record() -> Self {
        Self::Recording(Vec::new())
    }

    #[cfg(test)]
    pub(super) fn take_requests(&mut self) -> Vec<Request> {
        match self {
            Self::Recording(requests) => std::mem::take(requests),
            _ => panic!("Info recorder is disabled"),
        }
    }
}

struct Visit {
    subject: Subject,
    page: Page,
    label: String,
    rows: Vec<Row>,
    request: Option<Uuid>,
    failed: bool,
}

/// The active visit and its back stack. Scroll coordinates are wrapped screen
/// lines; link focus is a logical row and survives terminal width changes.
pub(super) struct Info {
    pub(super) render_cache: super::render::info::Cache,
    visit: Visit,
    history: Vec<(Visit, usize, usize)>,
    pub cursor: usize,
    pub scroll: usize,
    starts: Vec<usize>,
    max_scroll: usize,
    height: usize,
    reveal: bool,
}

impl Visit {
    fn new(subject: Subject, page: Page, label: String) -> Self {
        let mut rows = if matches!(page, Page::Agent) { info_data::summary(&subject) } else { Vec::new() };
        rows.push(Row { label: String::new(), value: "Loading…".into(), link: None, status: None });
        Self { subject, page, label, rows, request: None, failed: false }
    }
}

impl Info {
    fn new(subject: Subject) -> Self {
        let label = subject.name.clone();
        Self { render_cache: Default::default(), visit: Visit::new(subject, Page::Agent, label), history: Vec::new(), cursor: 0, scroll: 0, starts: Vec::new(), max_scroll: 0, height: 1, reveal: false }
    }
    pub(super) fn rows(&self) -> &[Row] {
        &self.visit.rows
    }
    pub(super) fn title(&self) -> String {
        let labels = self.history.iter().map(|(v, _, _)| v.label.as_str()).chain([self.visit.label.as_str()]);
        labels.collect::<Vec<_>>().join(" › ")
    }
    pub(super) fn subject_is_request_error(&self) -> bool {
        self.visit.subject.is_request_error()
    }

    pub(super) fn loading(&self) -> bool {
        self.visit.request.is_some()
    }

    pub(super) fn layout(&mut self, row_starts: Vec<usize>, total_lines: usize, height: usize) {
        self.starts = row_starts;
        self.height = height.max(1);
        self.max_scroll = total_lines.saturating_sub(height);
        if self.reveal
            && let Some(start) = self.starts.get(self.cursor).copied()
        {
            if start < self.scroll {
                self.scroll = start;
            }
            if start >= self.scroll.saturating_add(self.height) {
                self.scroll = start.saturating_sub(self.height - 1);
            }
        }
        self.reveal = false;
        self.scroll = self.scroll.min(self.max_scroll);
        self.focus_visible();
    }

    fn step(&mut self, down: bool) {
        let next = if down { (self.cursor.saturating_add(1)..self.rows().len()).find(|&i| self.rows()[i].link.is_some()) } else { (0..self.cursor).rev().find(|&i| self.rows()[i].link.is_some()) };
        if let Some(next) = next {
            self.cursor = next;
            self.reveal = true;
        } else {
            self.pan(down, 1);
        }
    }

    pub(super) fn pan(&mut self, down: bool, count: usize) {
        self.scroll = if down { self.scroll.saturating_add(count).min(self.max_scroll) } else { self.scroll.saturating_sub(count) };
        self.reveal = false;
        self.focus_visible();
    }

    fn focus_visible(&mut self) {
        let visible = self.scroll..self.scroll.saturating_add(self.height);
        let Some(&start) = self.starts.get(self.cursor) else { return };
        let end = self.starts.get(self.cursor + 1).copied().unwrap_or(usize::MAX);
        if start < visible.end && end > visible.start {
            return;
        }
        let top = self.starts.partition_point(|&line| line <= visible.start).saturating_sub(1);
        self.cursor = (top..self.starts.len()).take_while(|&i| self.starts[i] < visible.end).find(|&i| self.rows().get(i).is_some_and(|r| r.link.is_some())).unwrap_or(top);
    }
}

impl App {
    fn checked_info_subject(&self) -> Option<&Subject> {
        let name = self.ui.selected.first()?;
        self.ui.checked_subjects.get(name)
    }

    pub(super) fn focused_info_agent(&self) -> Option<&Agent> {
        match self.cursor_target() {
            CursorTarget::Agent(i) => self.data.agents.get(i),
            CursorTarget::StoppedAgent(i) => self.data.stopped_agents.get(i),
            CursorTarget::RemoteAgent(i) => self.data.remote_agents.get(i),
            _ => None,
        }
    }

    pub(super) fn has_info_target(&self) -> bool {
        match self.ui.selected.len() {
            0 => self.ui.transcript.is_some() || self.focused_info_agent().is_some(),
            1 => self.checked_info_subject().is_some(),
            _ => false,
        }
    }

    pub(super) fn open_info(&mut self) {
        let subject = match self.ui.selected.len() {
            0 => self.ui.transcript.as_ref().map(|reader| reader.subject.clone()).or_else(|| self.focused_info_agent().map(Subject::from_agent)),
            1 => self.checked_info_subject().cloned(),
            _ => None,
        };
        let Some(subject) = subject else { return };
        self.ui.info = Some(Info::new(subject));
        self.control_overflow = None;
        self.request_info();
    }

    fn request_info(&mut self) {
        let Some(info) = self.ui.info.as_mut() else { return };
        let id = Uuid::new_v4();
        info.visit.request = Some(id);
        let result = self.info_worker.submit((id, info.visit.subject.clone(), info.visit.page.clone()));
        if let Err(error) = result {
            self.finish_info(id, Err(error));
        }
        self.mouse.invalidate();
    }

    pub(super) fn poll_info(&mut self) -> bool {
        let replies: Vec<_> = match &self.info_worker {
            Worker::Active(_, replies) => replies.try_iter().collect(),
            _ => return false,
        };
        let mut dirty = false;
        for (id, result) in replies {
            dirty |= self.finish_info(id, result);
        }
        dirty
    }

    pub(super) fn finish_info(&mut self, id: Uuid, result: Result<Vec<Row>, String>) -> bool {
        let Some(info) = self.ui.info.as_mut().filter(|i| i.visit.request == Some(id)) else { return false };
        info.visit.request = None;
        info.visit.failed = result.is_err();
        info.render_cache = Default::default();
        info.visit.rows = result.unwrap_or_else(|error| {
            let mut rows = if matches!(info.visit.page, Page::Agent) { info_data::summary(&info.visit.subject) } else { Vec::new() };
            rows.push(Row { label: "Retry".into(), value: error, link: Some(Link::Page(info.visit.page.clone())), status: None });
            rows
        });
        info.cursor = info.rows().iter().position(|r| r.link.is_some()).unwrap_or(0);
        self.mouse.invalidate();
        true
    }

    pub(super) fn info_activate(&mut self, index: usize) {
        let Some(info) = self.ui.info.as_mut() else { return };
        let Some(row) = info.rows().get(index) else { return };
        let Some(link) = row.link.clone() else { return };
        if info.visit.failed && index + 1 == info.rows().len() {
            self.request_info();
            return;
        }
        let (subject, page, label) = match link {
            Link::Agent(subject) => {
                let label = subject.name.clone();
                (subject, Page::Agent, label)
            }
            Link::Page(page) => {
                let label = match &page {
                    Page::Role | Page::Profile => row.value.clone(),
                    Page::Task(path) => std::path::Path::new(path).file_name().map_or_else(|| row.label.clone(), |s| s.to_string_lossy().into_owned()),
                    _ => page.caption(),
                };
                (info.visit.subject.clone(), page, label)
            }
        };
        info.render_cache = Default::default();
        let old = std::mem::replace(&mut info.visit, Visit::new(subject, page, label));
        info.history.push((old, index, info.scroll));
        info.cursor = 0;
        info.scroll = 0;
        info.starts.clear();
        self.request_info();
    }

    pub(super) fn info_back(&mut self) {
        if let Some(info) = self.ui.info.as_mut()
            && let Some((visit, cursor, scroll)) = info.history.pop()
        {
            info.render_cache = Default::default();
            info.visit = visit;
            info.cursor = cursor;
            info.scroll = scroll;
            info.reveal = false;
            info.starts.clear();
            if info.loading() {
                self.request_info();
            }
        } else {
            self.ui.info = None;
        }
        self.mouse.invalidate();
    }

    pub(super) fn handle_info_key(&mut self, code: KeyCode) {
        let Some(info) = self.ui.info.as_mut() else { return };
        match code {
            KeyCode::Char('i') => {
                self.ui.info = None;
                self.mouse.invalidate();
            }
            KeyCode::Esc | KeyCode::Left | KeyCode::Backspace => self.info_back(),
            KeyCode::Up | KeyCode::BackTab => info.step(false),
            KeyCode::Down | KeyCode::Tab => info.step(true),
            KeyCode::Home => info.pan(false, usize::MAX),
            KeyCode::End => info.pan(true, usize::MAX),
            KeyCode::PageUp => info.pan(false, info.height.saturating_sub(1).max(1)),
            KeyCode::PageDown => info.pan(true, info.height.saturating_sub(1).max(1)),
            KeyCode::Enter | KeyCode::Right => {
                let index = info.cursor;
                self.info_activate(index);
            }
            _ => {}
        }
    }
}
