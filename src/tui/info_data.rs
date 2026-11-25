//! Exact Agent Info subjects and read-only row projections for the navigation worker.

use super::model::{Agent, AgentStatus};

#[derive(Clone, Debug)]
pub(super) struct Subject {
    pub name: String,
    agent: Agent,
}

#[derive(Clone, Debug)]
pub(super) enum Page {
    Agent,
    Role,
    Profile,
    Task(String),
    Bootstrap,
    Runtime,
    Instructions,
    TaskBody(String),
    Children,
    ProfileArgs,
    Bindings,
    ChildrenAfter(String),
}

#[derive(Clone, Debug)]
pub(super) enum Link {
    Page(Page),
    Agent(Box<Subject>),
}

#[derive(Clone, Debug)]
pub(super) struct Row {
    pub label: String,
    pub value: String,
    pub link: Option<Link>,
    pub status: Option<AgentStatus>,
}

use crate::{
    file_bindings,
    management::contract::{FileBindingView, LaunchRelations, RelatedAgent},
    service::{
        InvocationContext, Request, Response, ServiceError, ServiceResult,
        client::Client,
        queries::{
            self,
            info::{Facts, Projection, Reference, Section, Target},
        },
    },
};
use chrono::{Local, TimeZone};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

const INLINE_CHILDREN: u32 = 3;
const CHILDREN_PAGE: u32 = 16;

impl Subject {
    pub(super) fn from_agent(agent: &Agent) -> Self {
        Self { name: agent.display_name(), agent: agent.clone() }
    }

    /// Fixture-facing identity; native reads use the fenced reference instead.
    #[cfg(test)]
    pub(super) fn identity(&self) -> (Option<&str>, &str, u64) {
        (self.agent.device_name.as_deref(), &self.agent.name, self.agent.created_at.to_bits())
    }

    pub(super) fn is_request_error(&self) -> bool {
        self.agent.status == AgentStatus::Error && self.agent.status_context == crate::agents::model_request::ERROR_CONTEXT
    }

    fn related(&self, relation: RelatedAgent) -> Option<Self> {
        let view = relation.agent?;
        let mut reference = self.agent.info_reference.clone()?;
        let bits = relation.incarnation.parse().ok()?;
        let status = status(&view.status)?;
        if view.agent.name != relation.name {
            return None;
        }
        reference.target = if let Some(event) = view.agent.stopped_event.as_deref() {
            Target::Stopped { name: relation.name.clone(), event: event.parse::<i64>().ok().filter(|id| *id > 0)?, bits: Some(bits) }
        } else {
            if view.agent.incarnation.as_deref()?.parse::<u64>().ok()? != bits {
                return None;
            }
            Target::Incarnation { name: relation.name.clone(), bits }
        };
        let agent = Agent {
            info_reference: Some(reference),
            name: relation.name,
            tool: super::model::AgentTool::parse(&view.tool),
            status,
            status_context: view.status_context,
            status_detail: view.detail,
            created_at: f64::from_bits(bits),
            status_time: 0.0,
            last_heartbeat: 0.0,
            has_tcp: false,
            directory: view.directory,
            unread: 0,
            last_event_id: None,
            device_name: None,
            sync_age: None,
            headless: false,
            session_id: view.session_id,
            transcript_path: None,
            pid: None,
            terminal_preset: None,
        };
        Some(Self::from_agent(&agent))
    }

    fn assignment(&self) -> Option<file_bindings::Assignment> {
        let reference = self.agent.info_reference.as_ref()?;
        Some(file_bindings::Assignment {
            scope: reference.scope.to_string_lossy().into_owned(),
            generation: reference.generation.clone(),
            participant: crate::agents::attachments::ParticipantRef { canonical_name: reference.target.name().to_owned(), incarnation_bits: reference.target.bits()? },
        })
    }
}

impl Page {
    pub(super) fn caption(&self) -> String {
        match self {
            Self::Agent => "Agent",
            Self::Role => "Role",
            Self::Profile => "Profile",
            Self::Task(_) => "Task / Delivery",
            Self::Bootstrap => "Bootstrap",
            Self::Runtime => "Runtime",
            Self::Instructions => "Instructions",
            Self::TaskBody(_) => "Body",
            Self::Children | Self::ChildrenAfter(_) => "Children",
            Self::ProfileArgs => "Arguments",
            Self::Bindings => "Bindings",
        }
        .into()
    }
}

impl Row {
    fn text(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self { label: label.into(), value: value.into(), link: None, status: None }
    }

    fn page(label: impl Into<String>, value: impl Into<String>, page: Page) -> Self {
        Self { link: Some(Link::Page(page)), ..Self::text(label, value) }
    }

    fn agent(label: &str, name: &str, status: AgentStatus, link: Option<Link>) -> Self {
        Self { label: label.into(), value: name.to_owned(), link, status: Some(status) }
    }
}

fn status(value: &str) -> Option<AgentStatus> {
    Some(match value {
        "error" => AgentStatus::Error,
        "active" => AgentStatus::Active,
        "listening" => AgentStatus::Listening,
        "blocked" => AgentStatus::Blocked,
        "launching" => AgentStatus::Launching,
        "inactive" | "stopped" => AgentStatus::Inactive,
        _ => return None,
    })
}

fn add(rows: &mut Vec<Row>, label: &str, value: Option<impl Into<String>>) {
    if let Some(value) = value.map(Into::into).filter(|value| !value.is_empty()) {
        rows.push(Row::text(label, value));
    }
}

fn local_time(time: f64) -> Option<String> {
    if !time.is_finite() || time <= 0.0 {
        return None;
    }
    Some(Local.timestamp_opt(time as i64, 0).single()?.format("%b %-d, %Y %H:%M").to_string())
}

fn summary_rows(subject: &Subject, title: Option<&str>, creation: Option<f64>, provider: Option<&str>, status: AgentStatus) -> Vec<Row> {
    let name = title.map_or_else(|| subject.name.clone(), |title| format!("{} · {title}", subject.name));
    let mut rows = vec![Row::agent("Agent", &name, status, None)];
    add(&mut rows, "Created", creation.and_then(local_time));
    add(&mut rows, "Provider", provider);
    rows
}

pub(super) fn summary(subject: &Subject) -> Vec<Row> {
    let (creation, provider) = match subject.agent.info_reference.as_ref().map(|reference| &reference.target) {
        Some(Target::Stopped { bits, .. }) => (bits.map(f64::from_bits), None),
        None if !subject.agent.is_remote() => (None, None),
        _ => (Some(subject.agent.created_at), Some(subject.agent.tool.name())),
    };
    summary_rows(subject, None, creation, provider, subject.agent.status)
}

/// Bind every detail call to the scope and generation that supplied the row.
fn connect(reference: &Reference) -> ServiceResult<Client> {
    let client = Client::connect_existing_fenced(&reference.scope, InvocationContext::capture()?, &reference.generation)?;
    if client.scope_root() != reference.scope {
        return Err(ServiceError::StaleFence);
    }
    Ok(client)
}

struct Reader<'a> {
    client: &'a Client,
    reference: &'a Reference,
}

impl Reader<'_> {
    fn query(&self, section: Section) -> ServiceResult<Projection> {
        match self.client.call(Request::Queries(queries::Request::AgentInfo { target: self.reference.target.clone(), section }))? {
            Response::Queries(queries::Response::AgentInfo(projection)) => Ok(projection),
            _ => Err(unexpected("Agent Info")),
        }
    }

    fn facts(&self) -> ServiceResult<Box<Facts>> {
        let Projection::Facts(facts) = self.query(Section::Facts)? else { return Err(unexpected("Agent Info facts")) };
        Ok(facts)
    }

    fn bindings(&self) -> ServiceResult<Vec<FileBindingView>> {
        let Projection::Bindings(files) = self.query(Section::Bindings)? else { return Err(unexpected("bindings")) };
        Ok(files)
    }

    fn relations(&self, after: Option<String>, limit: u32) -> ServiceResult<(u64, LaunchRelations)> {
        let Projection::Relations { page, total_children } = self.query(Section::Relations { after, limit })? else { return Err(unexpected("launch relations")) };
        Ok((total_children, page))
    }
}

fn unexpected(section: &str) -> ServiceError {
    ServiceError::Internal(format!("Unexpected {section} reply"))
}

fn missing(error: &ServiceError) -> bool {
    matches!(error, ServiceError::StaleFence | ServiceError::NotFound(_) | ServiceError::Unsupported(_))
}

fn available_rows(subject: &Subject, page: &Page, message: &str) -> Vec<Row> {
    match page {
        Page::Agent => {
            let mut rows = summary(subject);
            rows.push(Row::page("Runtime", "", Page::Runtime));
            rows.push(Row::text("", message));
            rows
        }
        Page::Runtime => runtime_rows(Facts {
            directory: Some(subject.agent.directory.clone()),
            terminal_preset: subject.agent.terminal_preset.clone(),
            headless: subject.agent.headless.then_some(true),
            pid: subject.agent.pid,
            session_id: subject.agent.session_id.clone(),
            transcript_path: subject.agent.transcript_path.clone(),
            ..Facts::default()
        }),
        _ => vec![Row::text("", message)],
    }
}

pub(super) fn read(subject: &Subject, page: &Page) -> Result<Vec<Row>, String> {
    if subject.agent.is_remote() {
        return Ok(available_rows(subject, page, "Only fields synced from the remote device are available."));
    }
    let unavailable = || Ok(available_rows(subject, page, "No exact saved record is available for this visit."));
    let Some(reference) = subject.agent.info_reference.as_ref() else {
        return Ok(available_rows(subject, page, "Exact Agent Info requires a current AgentX daemon."));
    };
    let client = match connect(reference) {
        Ok(client) => client,
        Err(error) if missing(&error) => return unavailable(),
        Err(error) => return Err(error.to_string()),
    };
    if let Page::Task(path) | Page::TaskBody(path) = page {
        return task_rows(subject, Path::new(path), matches!(page, Page::TaskBody(_)));
    }
    let reader = Reader { client: &client, reference };
    match read_page(subject, page, &reader) {
        Ok(rows) => Ok(rows),
        Err(error) if missing(&error) => unavailable(),
        Err(error) => Err(error.to_string()),
    }
}

fn read_page(subject: &Subject, page: &Page, reader: &Reader<'_>) -> ServiceResult<Vec<Row>> {
    match page {
        Page::Agent | Page::Role | Page::Profile | Page::Runtime => {
            let facts = reader.facts()?;
            Ok(match page {
                Page::Agent => agent_rows(subject, reader, &facts),
                Page::Role => role_rows(&facts),
                Page::Profile => profile_rows(&facts),
                Page::Runtime => runtime_rows(*facts),
                _ => unreachable!(),
            })
        }
        Page::Bootstrap | Page::Instructions => {
            let section = if matches!(page, Page::Bootstrap) { Section::Bootstrap } else { Section::Instructions };
            let Projection::Text(text) = reader.query(section)? else { return Err(unexpected("instructions")) };
            Ok(text_rows(text))
        }
        Page::ProfileArgs => {
            let Projection::ProfileArguments(profile) = reader.query(Section::ProfileArguments)? else { return Err(unexpected("saved arguments")) };
            let Some(profile) = profile else { return Ok(vec![Row::text("Profile", "No saved launch profile")]) };
            Ok(vec![Row::text("Merge", if profile.replace_args { "Replace base arguments" } else { "Override base arguments" }), Row::text("Base", arguments(&profile.definition.args)), Row::text("Overrides", arguments(&profile.overrides))])
        }
        Page::Children | Page::ChildrenAfter(_) => {
            let after = if let Page::ChildrenAfter(cursor) = page { Some(cursor.clone()) } else { None };
            let (_, page) = reader.relations(after, CHILDREN_PAGE)?;
            let mut rows = page.downstream.into_iter().map(|child| relation_row(subject, "Child", child)).collect::<Vec<_>>();
            if let Some(next) = page.next {
                rows.push(Row::page("More", "", Page::ChildrenAfter(next)));
            } else if rows.is_empty() {
                rows.push(Row::text("Children", "No recorded child Agents"));
            }
            Ok(rows)
        }
        Page::Bindings => {
            let files = reader.bindings()?;
            Ok(if files.is_empty() { vec![Row::text("Bindings", "No recorded bindings")] } else { files.into_iter().map(binding_row).collect() })
        }
        Page::Task(_) | Page::TaskBody(_) => unreachable!("file pages are read before native projections"),
    }
}

fn agent_rows(subject: &Subject, reader: &Reader<'_>, facts: &Facts) -> Vec<Row> {
    let mut rows = summary_rows(subject, facts.title.as_deref(), facts.created_at, facts.tool.as_deref(), facts.status.as_deref().and_then(status).unwrap_or(subject.agent.status));
    if let Some(role) = &facts.role {
        rows.push(Row::page("Role", &role.name, Page::Role));
    }
    let profile = facts.profile.as_ref().map(|profile| &profile.name).or_else(|| facts.role.as_ref().map(|role| &role.profile));
    if let Some(name) = profile {
        rows.push(Row::page("Profile", name, Page::Profile));
    }
    match reader.bindings() {
        Ok(mut files) if files.len() == 1 => rows.push(binding_row(files.remove(0))),
        Ok(files) if !files.is_empty() => rows.push(Row::page("Bindings", files.len().to_string(), Page::Bindings)),
        Err(error) if !missing(&error) => rows.push(Row::page("Bindings · Retry", error.to_string(), Page::Bindings)),
        _ => {}
    }
    match reader.relations(None, INLINE_CHILDREN) {
        Ok((total_children, page)) => {
            rows.push(if let Some(upstream) = page.upstream { relation_row(subject, "From", upstream) } else { Row::text("From", if page.origin == "external" { "External launch" } else { "Unknown origin" }) });
            if page.next.is_some() {
                rows.push(Row::page("Children", total_children.to_string(), Page::Children));
            } else {
                rows.extend(page.downstream.into_iter().map(|child| relation_row(subject, "Child", child)));
            }
        }
        Err(error) if missing(&error) => rows.push(Row::text("From", "No saved launch relation is available")),
        Err(error) => rows.push(Row::page("Relations · Retry", error.to_string(), Page::Agent)),
    }
    rows.push(Row::page("Bootstrap", "", Page::Bootstrap));
    rows.push(Row::page("Runtime", "", Page::Runtime));
    rows
}

fn relation_row(subject: &Subject, label: &str, relation: RelatedAgent) -> Row {
    let name = relation.name.clone();
    if let Some(related) = subject.related(relation) { Row::agent(label, &name, related.agent.status, Some(Link::Agent(Box::new(related)))) } else { Row::text(label, format!("{name} · Record unavailable")) }
}

fn role_rows(facts: &Facts) -> Vec<Row> {
    let Some(role) = &facts.role else { return vec![Row::text("Role", "No saved role")] };
    let mut rows = vec![Row::text("Role", &role.name)];
    add(&mut rows, "Description", role.description.as_deref());
    add(&mut rows, "Namespace", Some(role.namespace.as_str()));
    if role.default {
        rows.push(Row::text("Default", "Directory role"));
    }
    add(&mut rows, "Source", Some(role.path.to_string_lossy().into_owned()));
    let profile = facts.profile.as_ref().map_or(role.profile.as_str(), |profile| profile.name.as_str());
    if !profile.is_empty() {
        rows.push(Row::page("Profile", profile, Page::Profile));
    }
    rows.push(Row::page("Instructions", "", Page::Instructions));
    rows
}

fn profile_rows(facts: &Facts) -> Vec<Row> {
    let Some(profile) = &facts.profile else { return vec![Row::text("Profile", "No saved launch profile")] };
    let mut rows = vec![Row::text("Profile", &profile.name)];
    add(&mut rows, "Description", profile.description.as_deref());
    add(&mut rows, "Provider", Some(profile.tool.as_str()));
    let source = match profile.selected_by.as_str() {
        "explicit_profile" => "--profile",
        "role_profile" => "Role configuration",
        "environment_profile" => "AGENTX_PROFILE",
        "scope_default" => "Scope default",
        "tool_default" => "Provider default",
        other => other,
    };
    add(&mut rows, "Selected by", Some(source));
    rows.push(Row::page("Arguments", "", Page::ProfileArgs));
    rows
}

fn runtime_rows(facts: Facts) -> Vec<Row> {
    let mut rows = Vec::new();
    add(&mut rows, "Working directory", facts.directory);
    add(&mut rows, "Workspace", facts.workspace);
    add(&mut rows, "Terminal preset", facts.terminal_preset);
    add(&mut rows, "Headless", facts.headless.map(|headless| if headless { "Yes" } else { "No" }));
    add(&mut rows, "PID", facts.pid.map(|pid| pid.to_string()));
    add(&mut rows, "Session", facts.session_id);
    add(&mut rows, "Transcript", facts.transcript_path);
    if rows.is_empty() {
        rows.push(Row::text("", "No recorded runtime fields"));
    }
    rows
}

fn arguments(args: &[String]) -> String {
    if args.is_empty() { "(none)".into() } else { args.iter().map(|arg| serde_json::to_string(arg).expect("serializing a string")).collect::<Vec<_>>().join("\n") }
}

fn text_rows(text: Option<String>) -> Vec<Row> {
    vec![Row::text("", text.filter(|text| !text.trim().is_empty()).unwrap_or_else(|| "No saved text".into()))]
}

fn binding_row(binding: FileBindingView) -> Row {
    let kind = if binding.schema.as_deref() == Some("delivery") { "Delivery" } else { "Task" };
    let name = binding.description.as_deref().filter(|text| !text.trim().is_empty()).map_or_else(|| basename(Path::new(&binding.path)), str::to_owned);
    let value = if binding.diagnostic.is_some() { format!("{name} · Source unavailable") } else { name };
    Row::page(kind, value, Page::Task(binding.path))
}

fn basename(path: &Path) -> String {
    path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().into_owned()
}

fn task_rows(subject: &Subject, path: &Path, body_only: bool) -> Result<Vec<Row>, String> {
    let (document, body) = file_bindings::read_document(path, body_only).map_err(|error| format!("{}: {error}", basename(path)))?;
    if body_only {
        return Ok(text_rows(body));
    }
    let kind = if document.schema == "delivery" { "Delivery" } else { "Task" };
    let dir = Path::new(&subject.agent.directory);
    let project = if dir.is_absolute() { file_bindings::index::project_root(dir) } else { file_bindings::index::project_root(path.parent().unwrap_or(path)) };
    let visible_path = path.strip_prefix(&project).unwrap_or(path);
    let mut rows = vec![Row::text("Type", kind), Row::text("File", visible_path.to_string_lossy().into_owned())];
    add(&mut rows, "Description", document.description.as_deref());
    rows.push(Row::text("Status", document.raw_status.as_deref().unwrap_or("Undeclared")));
    let owner = subject.assignment();
    for assignment in &document.assignments {
        let suffix = if owner.as_ref() == Some(assignment) { "this instance".into() } else { assignment.participant.incarnation_bits.to_string() };
        rows.push(Row::text("Bound to", format!("@{} · {suffix}", assignment.participant.canonical_name)));
    }
    rows.push(Row::page("Body", "", Page::TaskBody(path.to_string_lossy().into_owned())));
    if kind == "Delivery" {
        rows.extend(declared_tasks(&document, path).into_iter().map(|task| Row::page("Task", basename(&task), Page::Task(task.to_string_lossy().into_owned()))));
    }
    Ok(rows)
}

/// Interpret only declared Task paths; arbitrary Markdown links and legacy
/// names never acquire navigation or assignment semantics.
fn declared_tasks(document: &file_bindings::Document, path: &Path) -> Vec<PathBuf> {
    let parent = path.parent().unwrap_or(path);
    let stages = document.relations.get("stages").and_then(serde_yaml_ng::Value::as_mapping);
    let lists = document.relations.get("tasks").into_iter().chain(stages.into_iter().flat_map(|stages| stages.values().filter_map(|stage| stage.get("tasks"))));
    let mut seen = HashSet::new();
    lists
        .filter_map(serde_yaml_ng::Value::as_sequence)
        .flatten()
        .filter_map(|task| task.as_str().or_else(|| task.get("path")?.as_str()))
        .filter(|task| !task.is_empty())
        .map(|task| parent.join(task))
        .filter(|task| task != path && seen.insert(task.clone()))
        .collect()
}
