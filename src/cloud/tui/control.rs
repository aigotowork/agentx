//! Cloud administration dialogs. The server owns admission and effects; the dialog retains exact input.
use super::{ADMIN, AgentView, Http, HttpError, KeyCode, KeyEvent, KeyModifiers, Theme, display, encode, http_error, namespace_path};
use crate::cloud::control::types::{ControlCommand, ControlOutcome, ControlReceipt, ControlStatus, CreateControlOperation, HostCapabilities};
use crate::cloud::identifiers::{random_id, valid_workspace_id};
use anyhow::{Result, ensure};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Clear, Paragraph, Wrap},
};

#[derive(Clone)]
pub(super) struct NamespaceTarget {
    pub id: String,
    pub generation: String,
}

pub(super) enum Job {
    Catalogue(NamespaceTarget, String),
    Submit(NamespaceTarget, CreateControlOperation),
    Lookup(NamespaceTarget, String, Option<CreateControlOperation>),
}

pub(super) enum Update {
    Catalogue(Result<HostCapabilities>),
    Submitted(Result<ControlReceipt>),
    LookedUp(Result<ControlReceipt>),
}

fn check_receipt(receipt: ControlReceipt, namespace: &NamespaceTarget, id: &str, original: Option<&CreateControlOperation>) -> Result<ControlReceipt> {
    ensure!(receipt.generation == namespace.generation && receipt.request_id == id, "operation receipt identity mismatch");
    if let Some(input) = original {
        ensure!(receipt.host == input.host && receipt.installation_id == input.installation_id && receipt.command == input.command, "operation receipt differs from the original request");
    }
    let valid_state = match (&receipt.status, &receipt.outcome) {
        (ControlStatus::Queued, None) => receipt.claim_id.is_none(),
        (ControlStatus::Running, None) => receipt.claim_id.is_some(),
        (ControlStatus::Succeeded, Some(ControlOutcome::Succeeded { agent })) => {
            agent.host == receipt.host
                && match &receipt.command {
                    ControlCommand::Stop { name, incarnation } => &agent.name == name && &agent.incarnation == incarnation,
                    ControlCommand::Launch { name, workspace_id, .. } => {
                        name.as_ref().is_none_or(|name| name == &agent.name) && matches!(&agent.workspace, crate::cloud::communication::types::AgentWorkspace::Project { workspace_id: observed } if observed == workspace_id)
                    }
                    ControlCommand::AddWorkspace { .. } => false,
                }
        }
        (ControlStatus::Succeeded, Some(ControlOutcome::WorkspaceAdded { workspace_id })) => receipt.claim_id.is_some() && valid_workspace_id(workspace_id) && matches!(&receipt.command, ControlCommand::AddWorkspace { .. }),
        (ControlStatus::Failed, Some(ControlOutcome::Failed { .. })) | (ControlStatus::Uncertain, Some(ControlOutcome::Uncertain { .. })) => true,
        _ => false,
    };
    ensure!(valid_state, "operation receipt state mismatch");
    Ok(receipt)
}

impl Job {
    pub(super) fn execute(self, http: &Http) -> Update {
        match self {
            Self::Catalogue(namespace, host) => Update::Catalogue((|| {
                let path = format!("{}/hosts/{}/capabilities", namespace_path(&namespace.id), encode(&host));
                let data: HostCapabilities = http.request("GET", &path, None)?;
                ensure!(data.generation == namespace.generation && data.host == host, "host catalogue identity mismatch");
                Ok(data)
            })()),
            Self::Submit(namespace, input) => Update::Submitted((|| {
                let path = format!("{}/operations", namespace_path(&namespace.id));
                let receipt = http.request("POST", &path, Some(&serde_json::to_value(&input)?))?;
                check_receipt(receipt, &namespace, &input.request_id, Some(&input))
            })()),
            Self::Lookup(namespace, id, original) => Update::LookedUp((|| {
                let path = format!("{ADMIN}/namespaces/{}/operations/{}", encode(&namespace.id), encode(&id));
                let receipt = http.request("GET", &path, None)?;
                check_receipt(receipt, &namespace, &id, original.as_ref())
            })()),
        }
    }
}

enum Target {
    Launch(String),
    Stop(AgentView),
    Lookup,
}

impl Target {
    fn host(&self) -> Option<&str> {
        match self {
            Self::Launch(host) => Some(host),
            Self::Stop(agent) => Some(&agent.host),
            Self::Lookup => None,
        }
    }
}

enum Phase {
    Draft,
    Sending(CreateControlOperation, bool),
    Unknown(CreateControlOperation, bool),
    Known(Box<ControlReceipt>, Option<CreateControlOperation>),
    Rejected,
}

struct Draft {
    workspace: Option<usize>,
    role: Option<usize>,
    field: usize,
    name: String,
    title: String,
    task: String,
    request: String,
}

impl Default for Draft {
    fn default() -> Self {
        Self { workspace: None, role: Some(0), field: 0, name: String::new(), title: String::new(), task: String::new(), request: String::new() }
    }
}

pub(super) struct Dialog {
    namespace: NamespaceTarget,
    target: Target,
    catalogue: Option<HostCapabilities>,
    draft: Draft,
    phase: Phase,
    notice: Option<String>,
}

fn provided(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn trustworthy_status(error: &anyhow::Error) -> Option<u16> {
    error.downcast_ref::<HttpError>().filter(|e| e.code != "http_error").map(|e| e.status)
}

impl Dialog {
    fn new(namespace: NamespaceTarget, target: Target) -> Self {
        Self { namespace, target, catalogue: None, draft: Draft::default(), phase: Phase::Draft, notice: None }
    }

    pub(super) fn launch(namespace: NamespaceTarget, host: String) -> Self {
        Self::new(namespace, Target::Launch(host))
    }

    pub(super) fn stop(namespace: NamespaceTarget, agent: AgentView) -> Self {
        Self::new(namespace, Target::Stop(agent))
    }

    pub(super) fn lookup(namespace: NamespaceTarget) -> Self {
        Self::new(namespace, Target::Lookup)
    }

    pub(super) fn load(&self) -> Option<Job> {
        Some(Job::Catalogue(self.namespace.clone(), self.target.host()?.to_owned()))
    }

    pub(super) fn worker_interrupted(&mut self) {
        if let Phase::Sending(input, _) = &self.phase {
            self.phase = Phase::Unknown(input.clone(), false);
        }
        self.notice = Some("Deployment request worker stopped.".into());
    }

    pub(super) fn apply(&mut self, update: Update) {
        self.notice = None;
        match update {
            Update::Catalogue(result) => match result {
                Ok(value) => self.apply_catalogue(value),
                Err(error) => {
                    self.catalogue = None;
                    self.notice = Some(http_error(&error));
                }
            },
            Update::Submitted(result) => {
                let Phase::Sending(input, replaying) = &self.phase else { return };
                self.phase = match result {
                    Ok(receipt) => Phase::Known(Box::new(receipt), Some(input.clone())),
                    Err(error) if !replaying && matches!(trustworthy_status(&error), Some(status @ 400..=499) if !matches!(status, 408 | 425 | 429)) => {
                        self.notice = Some(http_error(&error));
                        Phase::Rejected
                    }
                    Err(_) => Phase::Unknown(input.clone(), false),
                };
            }
            Update::LookedUp(result) => match result {
                Ok(receipt) => {
                    let intent = match &self.phase {
                        Phase::Unknown(input, _) => Some(input.clone()),
                        Phase::Known(_, input) => input.clone(),
                        _ => None,
                    };
                    self.phase = Phase::Known(Box::new(receipt), intent);
                }
                Err(error) => {
                    if let Phase::Unknown(_, can_replay) = &mut self.phase {
                        *can_replay = trustworthy_status(&error) == Some(404);
                        if *can_replay {
                            self.notice = Some("No receipt found. F6 resends original input.".into());
                            return;
                        }
                    }
                    self.notice = Some(http_error(&error));
                }
            },
        }
    }

    fn apply_catalogue(&mut self, updated: HostCapabilities) {
        if let Some(current) = &self.catalogue {
            if let Some(workspace) = self.draft.workspace.and_then(|index| current.workspaces.get(index)) {
                self.draft.workspace = updated.workspaces.iter().position(|item| item.workspace_id == workspace.workspace_id);
                if let Some(role) = self.draft.role.filter(|index| *index > 0).and_then(|index| workspace.roles.get(index - 1)) {
                    self.draft.role = self.draft.workspace.and_then(|index| updated.workspaces[index].roles.iter().position(|item| item.name == role.name)).map(|index| index + 1);
                }
            }
        } else if !updated.workspaces.is_empty() {
            self.draft.workspace = Some(0);
            self.draft.role = Some(0);
        }
        self.catalogue = Some(updated);
    }

    fn field(&mut self) -> Option<(&mut String, usize, bool)> {
        if !matches!(self.phase, Phase::Draft) {
            return None;
        }
        match (&self.target, self.draft.field) {
            (Target::Lookup, _) => Some((&mut self.draft.request, 128, false)),
            (Target::Launch(_), 2) => Some((&mut self.draft.name, 64, false)),
            (Target::Launch(_), 3) => Some((&mut self.draft.title, crate::agents::launch_title::MAX_CHARS * 4, false)),
            (Target::Launch(_), 4) => Some((&mut self.draft.task, 16 * 1024, true)),
            _ => None,
        }
    }

    pub(super) fn input(&mut self, text: &str) {
        let Some((field, limit, multiline)) = self.field() else { return };
        let allowed = text.chars().all(|ch| !ch.is_control() || multiline && matches!(ch, '\n' | '\t'));
        if allowed && field.len() + text.len() <= limit {
            field.push_str(text);
            self.notice = None;
        } else {
            self.notice = Some("Text is too long or contains unsupported characters.".into());
        }
    }

    fn make_input(&self) -> Result<CreateControlOperation, &'static str> {
        let capabilities = self.catalogue.as_ref().ok_or("Load the host catalogue before submitting.")?;
        if !capabilities.online || capabilities.installation_id.is_empty() {
            return Err("Host management is offline.");
        }
        let command = match &self.target {
            Target::Launch(_) => {
                let workspace = self.draft.workspace.and_then(|index| capabilities.workspaces.get(index)).ok_or("Select an exported workspace.")?;
                let choice = self.draft.role.ok_or("Select an available role.")?;
                let role = if choice == 0 {
                    if !workspace.default_available {
                        return Err("Default Codex is unavailable. Select an available role.");
                    }
                    None
                } else {
                    let role = workspace.roles.get(choice - 1).filter(|role| role.available).ok_or("Select an available role.")?;
                    Some(role.name.clone())
                };
                let name = provided(&self.draft.name);
                if name.as_deref().is_some_and(|name| !crate::agents::identity::is_valid_name(name)) {
                    return Err("Use lowercase letters, digits or underscore for the Agent name.");
                }
                let title = crate::agents::launch_title::normalize(provided(&self.draft.title).as_deref()).map_err(|_| "Use a short single-line title.")?;
                ControlCommand::Launch { workspace_id: workspace.workspace_id.clone(), role, name, title, initial_prompt: provided(&self.draft.task), working_directory: None, create_directory: None }
            }
            Target::Stop(agent) => ControlCommand::Stop { name: agent.name.clone(), incarnation: agent.incarnation.clone() },
            Target::Lookup => unreachable!(),
        };
        Ok(CreateControlOperation { generation: self.namespace.generation.clone(), request_id: random_id(), host: capabilities.host.clone(), installation_id: capabilities.installation_id.clone(), command })
    }

    fn submit(&mut self) -> Option<Job> {
        match self.make_input() {
            Ok(input) => {
                self.phase = Phase::Sending(input.clone(), false);
                Some(Job::Submit(self.namespace.clone(), input))
            }
            Err(message) => {
                self.notice = Some(message.into());
                None
            }
        }
    }

    pub(super) fn key(&mut self, key: KeyEvent) -> Option<Job> {
        match (&self.phase, key.code) {
            (Phase::Sending(_, _), _) => None,
            (Phase::Unknown(input, _) | Phase::Known(_, Some(input)), KeyCode::Enter | KeyCode::F(5)) => Some(Job::Lookup(self.namespace.clone(), input.request_id.clone(), Some(input.clone()))),
            (Phase::Known(receipt, None), KeyCode::Enter | KeyCode::F(5)) => Some(Job::Lookup(self.namespace.clone(), receipt.request_id.clone(), None)),
            (Phase::Unknown(input, true), KeyCode::F(6)) => {
                let input = input.clone();
                self.phase = Phase::Sending(input.clone(), true);
                Some(Job::Submit(self.namespace.clone(), input))
            }
            (Phase::Draft, KeyCode::F(5)) => self.load(),
            (Phase::Draft, KeyCode::Enter) if matches!(self.target, Target::Lookup) => {
                let id = self.draft.request.trim();
                if !id.is_empty() && !matches!(id, "." | "..") && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(&byte)) {
                    Some(Job::Lookup(self.namespace.clone(), id.to_owned(), None))
                } else {
                    self.notice = Some("Enter the original request ID.".into());
                    None
                }
            }
            (Phase::Draft, KeyCode::Enter) => self.submit(),
            (Phase::Draft, KeyCode::Tab | KeyCode::BackTab) if matches!(self.target, Target::Launch(_)) => {
                self.draft.field = (self.draft.field + if key.code == KeyCode::Tab { 1 } else { 4 }) % 5;
                None
            }
            (Phase::Draft, KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down) if matches!(self.target, Target::Launch(_)) => {
                let data = self.catalogue.as_ref()?;
                let prev = matches!(key.code, KeyCode::Left | KeyCode::Up);
                let step = |value: Option<usize>, length: usize| {
                    if length == 0 { None } else { Some(value.map_or(if prev { length - 1 } else { 0 }, |index| (index + if prev { length - 1 } else { 1 }) % length)) }
                };
                if self.draft.field == 0 {
                    self.draft.workspace = step(self.draft.workspace, data.workspaces.len());
                    self.draft.role = Some(0);
                } else if self.draft.field == 1 {
                    self.draft.role = step(self.draft.role, data.workspaces.get(self.draft.workspace?)?.roles.len() + 1);
                }
                self.notice = None;
                None
            }
            (Phase::Draft, KeyCode::Backspace) => {
                if let Some((field, _, _)) = self.field() {
                    field.pop();
                }
                self.notice = None;
                None
            }
            (Phase::Draft, KeyCode::Char('u')) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some((field, _, _)) = self.field() {
                    field.clear();
                }
                self.notice = None;
                None
            }
            (Phase::Draft, KeyCode::Char(ch)) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                self.input(ch.encode_utf8(&mut [0; 4]));
                None
            }
            _ => None,
        }
    }

    fn draft_lines(&self, width: usize) -> Vec<String> {
        let Some(data) = &self.catalogue else {
            return match &self.target {
                Target::Lookup => vec![format!("Request ID: {}", tail(&self.draft.request, width.saturating_sub(13)))],
                _ => vec!["Host catalogue unavailable.".into()],
            };
        };
        let observed = data.observed_at.and_then(|value| chrono::DateTime::from_timestamp(value, 0));
        let mut lines = vec![format!("Management {}{}", if data.online { "online" } else { "offline" }, observed.map_or_else(String::new, |time| format!(" · {}", time.format("%m-%d %H:%MZ"))))];
        lines.push(format!("Install: {}", display(&data.installation_id)));
        let workspace = self.draft.workspace.and_then(|index| data.workspaces.get(index));
        if matches!(self.target, Target::Launch(_)) {
            let mut editors = vec![
                ("Workspace", workspace.map_or_else(|| if data.workspaces.is_empty() { "No exported workspace".into() } else { "Select a workspace".into() }, |row| display(&row.name))),
                (
                    "Role",
                    match (workspace, self.draft.role) {
                        (Some(row), Some(0)) => availability("Default Codex", row.default_available, row.default_reason.as_deref()),
                        (Some(row), Some(index)) => row.roles.get(index - 1).map_or_else(|| "—".into(), |role| availability(&role.name, role.available, role.reason.as_deref())),
                        _ => "—".to_owned(),
                    },
                ),
                ("Name", tail(&self.draft.name, width.saturating_sub(14))),
                ("Title", tail(&self.draft.title, width.saturating_sub(14))),
                ("Task", tail(&self.draft.task, width.saturating_sub(14))),
            ];
            lines.extend(editors.drain(..).enumerate().map(|(index, (label, value))| format!("{} {label}: {value}", if index == self.draft.field { "›" } else { " " })));
        } else if data.online {
            lines.push("Stop this exact Agent instance?".into());
        }
        lines
    }

    pub(super) fn render(&self, frame: &mut Frame<'_>, area: Rect, pending: bool) {
        let title = match self.target {
            Target::Launch(_) => " Launch Agent ",
            Target::Stop(_) => " Stop Agent ",
            Target::Lookup => " Operation receipt ",
        };
        let width = area.width.min(78);
        let height = area.height.min(18);
        let modal = Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height);
        let block = Block::bordered().title(Line::styled(title, Theme::title()));
        let inner = block.inner(modal);
        frame.render_widget(Clear, modal);
        frame.render_widget(block, modal);
        let target_height = if matches!(self.target, Target::Stop(_)) { 2 } else { 1 };
        let status_height = if self.notice.is_some() || pending { 2 } else { 0 };
        let [target_area, body, status, keys] = Layout::vertical([Constraint::Length(target_height), Constraint::Min(3), Constraint::Length(status_height), Constraint::Length(2)]).areas(inner);
        let namespace = display(&self.namespace.id);
        let target = match &self.target {
            Target::Launch(host) => format!("{namespace} / {}", display(host)),
            Target::Stop(agent) => format!("{namespace} / {}\n@{} · {}", display(&agent.host), display(&agent.name), display(&agent.incarnation)),
            Target::Lookup => format!("Namespace: {namespace}"),
        };
        frame.render_widget(Paragraph::new(target), target_area);
        let lines = match &self.phase {
            Phase::Draft => self.draft_lines(body.width as usize),
            Phase::Sending(input, _) | Phase::Unknown(input, _) => vec!["Submission not confirmed. Request ID:".into(), display(&input.request_id)],
            Phase::Known(receipt, _) => receipt_lines(receipt),
            Phase::Rejected => vec!["The deployment rejected the request.".into()],
        };
        let should_wrap = !matches!(self.phase, Phase::Draft) || matches!(self.target, Target::Lookup);
        let mut text = Paragraph::new(lines.join("\n"));
        if should_wrap {
            text = text.wrap(Wrap { trim: true });
        }
        frame.render_widget(text, body);
        if pending {
            frame.render_widget(Paragraph::new(" Working…").style(Theme::title()), status);
            return;
        }
        if let Some(notice) = &self.notice {
            frame.render_widget(Paragraph::new(notice.as_str()).style(Theme::error()).wrap(Wrap { trim: true }), status);
        }
        let hints = match &self.phase {
            Phase::Draft if matches!(self.target, Target::Launch(_)) => "Tab Field  ←→ Choose  F5 Reload\nEnter Submit  Esc Cancel",
            Phase::Draft if matches!(self.target, Target::Lookup) => "Enter Find  Esc Close",
            Phase::Draft => "Enter Confirm  F5 Reload  Esc Close",
            Phase::Unknown(_, true) => "Enter Check  F6 Replay  Esc Close",
            Phase::Unknown(_, false) | Phase::Known(_, _) => "Enter Check receipt  Esc Close",
            Phase::Rejected => "Esc Close",
            Phase::Sending(_, _) => "",
        };
        frame.render_widget(Paragraph::new(hints), keys);
    }
}

fn availability(name: &str, available: bool, reason: Option<&str>) -> String {
    if available { display(name) } else { format!("{} · {}", display(name), display(reason.unwrap_or("unavailable"))) }
}

fn tail(value: &str, limit: usize) -> String {
    let count = value.chars().count();
    let mut text = String::new();
    if count > limit {
        text.push('…');
    }
    text.extend(value.chars().skip(count.saturating_sub(limit)).map(|ch| {
        if ch == '\n' {
            '↵'
        } else if ch.is_control() {
            ' '
        } else {
            ch
        }
    }));
    text
}

fn receipt_lines(receipt: &ControlReceipt) -> Vec<String> {
    let status = match receipt.status {
        ControlStatus::Queued => "Queued · waiting for host",
        ControlStatus::Running => "Running · waiting for result",
        ControlStatus::Succeeded => "Succeeded",
        ControlStatus::Failed => "Failed",
        ControlStatus::Uncertain => "Execution uncertain",
    };
    let mut lines = vec![status.into(), format!("Request: {}", display(&receipt.request_id))];
    match &receipt.command {
        ControlCommand::Launch { workspace_id, role, .. } => lines.push(format!("Launch: {} / {}", display(workspace_id), display(role.as_deref().unwrap_or("Default Codex")))),
        ControlCommand::Stop { name, incarnation } => lines.push(format!("Stop: @{} · {}", display(name), display(incarnation))),
        ControlCommand::AddWorkspace { parent, directory, name, .. } => lines.push(format!("Add workspace: {} / {} as {}", display(parent), display(directory), display(name))),
    }
    if let Some(outcome) = &receipt.outcome {
        match outcome {
            ControlOutcome::Succeeded { agent } => lines.push(format!("Agent: @{} · {}", display(&agent.name), display(&agent.incarnation))),
            ControlOutcome::WorkspaceAdded { workspace_id } => lines.push(format!("Workspace added: {}", display(workspace_id))),
            ControlOutcome::Failed { code } | ControlOutcome::Uncertain { code } => lines.push(format!("Code: {}", display(code))),
        }
    }
    lines.push(format!("{} · {}", display(&receipt.host), display(&receipt.installation_id)));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_receipt_displays_registration_and_rejects_a_missing_identity() {
        let namespace = NamespaceTarget { id: "office".into(), generation: "generation".into() };
        let mut receipt = ControlReceipt {
            generation: namespace.generation.clone(),
            request_id: "operation-1".into(),
            host: "worker".into(),
            installation_id: "installation-1".into(),
            command: ControlCommand::AddWorkspace { parent: "root".into(), directory: "projects/test".into(), name: "test".into(), create_directory: false },
            status: ControlStatus::Succeeded,
            claim_id: Some("claim-1".into()),
            outcome: Some(ControlOutcome::WorkspaceAdded { workspace_id: "WS0000000001".into() }),
        };
        let verified = check_receipt(receipt.clone(), &namespace, "operation-1", None).unwrap();
        assert!(receipt_lines(&verified).iter().any(|line| line == "Workspace added: WS0000000001"));
        receipt.outcome = Some(ControlOutcome::WorkspaceAdded { workspace_id: String::new() });
        assert!(check_receipt(receipt, &namespace, "operation-1", None).is_err());
    }
}
