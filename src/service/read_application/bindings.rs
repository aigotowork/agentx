//! Files own assignment state. A bounded worker publishes a disposable
//! snapshot so routine Agent reads never wait for file discovery or parsing.
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, mpsc};
use std::time::{Duration, Instant};

use crate::file_bindings::{
    Assignment, Index, Query,
    index::{Observation, Projection, project_root},
};
use crate::management::contract::{AgentView, FileBindingView, Response as View};
use crate::service::{ServiceError, ServiceResult};

type Views = HashMap<Assignment, Vec<Arc<FileBindingView>>>;
type Snapshot = Arc<RwLock<Arc<Views>>>;
const EXPLICIT_BUDGET: Duration = Duration::from_secs(2);
const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

enum Job {
    Refresh(BTreeSet<PathBuf>),
    Query { query: Query, cwd: PathBuf, deadline: Instant, reply: mpsc::SyncSender<ServiceResult<Projection>> },
    Assignment { assignment: Assignment, roots: Vec<PathBuf>, deadline: Instant, reply: mpsc::SyncSender<ServiceResult<Vec<FileBindingView>>> },
}

pub(super) struct Bindings {
    sender: mpsc::SyncSender<Job>,
    views: Snapshot,
    refresh_due: Instant,
    seen_directories: BTreeSet<PathBuf>,
}

impl Bindings {
    pub(super) fn start() -> Option<Self> {
        let (sender, receiver) = mpsc::sync_channel(2);
        let views = Snapshot::default();
        let published = views.clone();
        std::thread::Builder::new().name("agentx-binding-views".into()).spawn(move || run(receiver, published)).ok()?;
        Some(Self { sender, views, refresh_due: Instant::now(), seen_directories: BTreeSet::new() })
    }

    pub(super) fn enrich(&mut self, scope: &Path, generation: &str, response: &mut View) {
        let agents = match response {
            View::Agent(agent) => std::slice::from_mut(agent),
            View::Agents(page) => page.agents.as_mut_slice(),
            _ => return,
        };
        let mut paths = BTreeSet::from([scope.to_owned()]);
        paths.extend(agents.iter().filter(|view| view.agent.incarnation.is_some()).map(|view| PathBuf::from(&view.directory)).filter(|path| path.is_absolute()).take(63));
        let due = Instant::now() >= self.refresh_due;
        let discovered = paths.difference(&self.seen_directories).take(64usize.saturating_sub(self.seen_directories.len())).cloned().collect::<Vec<_>>();
        if (due || !discovered.is_empty()) && self.sender.try_send(Job::Refresh(paths)).is_ok() {
            self.seen_directories.extend(discovered);
            if due {
                self.refresh_due = Instant::now() + REFRESH_INTERVAL;
            }
        }
        let Ok(views) = self.views.read().map(|views| views.clone()) else { return };
        let scope = scope.to_string_lossy();
        for view in agents {
            if let Some(assignment) = assignment(&scope, generation, view)
                && let Some(files) = views.get(&assignment)
            {
                view.file_bindings = files.iter().map(|file| (**file).clone()).collect();
            }
        }
    }

    pub(super) fn query(&self, query: Query, cwd: &Path) -> ServiceResult<Projection> {
        let (reply, result) = mpsc::sync_channel(1);
        let deadline = Instant::now() + EXPLICIT_BUDGET;
        self.sender.try_send(Job::Query { query, cwd: cwd.to_owned(), deadline, reply }).map_err(|_| unavailable())?;
        result.recv_timeout(deadline.saturating_duration_since(Instant::now())).map_err(|_| unavailable())?
    }

    /// Discover the subject's project and return only its scoped exact assignments.
    pub(super) fn for_assignment(&self, assignment: Assignment, roots: Vec<PathBuf>) -> ServiceResult<Vec<FileBindingView>> {
        let (reply, result) = mpsc::sync_channel(1);
        let deadline = Instant::now() + EXPLICIT_BUDGET;
        self.sender.try_send(Job::Assignment { assignment, roots, deadline, reply }).map_err(|_| unavailable())?;
        result.recv_timeout(deadline.saturating_duration_since(Instant::now())).map_err(|_| unavailable())?
    }
}

fn assignment(scope: &str, generation: &str, view: &AgentView) -> Option<Assignment> {
    let incarnation_bits = view.agent.incarnation.as_ref()?.parse().ok()?;
    Some(Assignment { scope: scope.into(), generation: generation.into(), participant: crate::agents::attachments::ParticipantRef { canonical_name: view.agent.name.clone(), incarnation_bits } })
}

fn unavailable() -> ServiceError {
    ServiceError::Unavailable("file binding refresh unavailable; retry the read".into())
}

fn run(jobs: mpsc::Receiver<Job>, snapshot: Snapshot) {
    let mut index = Index::default();
    for job in jobs {
        match job {
            Job::Refresh(paths) => {
                index.refresh_known();
                let roots: BTreeSet<_> = paths.iter().map(|path| project_root(path)).collect();
                for root in roots {
                    let _ = index.observe_project(&root);
                }
                publish(&index, &snapshot);
            }
            Job::Query { query, cwd, deadline, reply } if Instant::now() < deadline => {
                if !query.refresh {
                    index.refresh_known();
                }
                let result = index.query(&query, &cwd).map_err(|error| ServiceError::InvalidInput(error.to_string()));
                publish(&index, &snapshot);
                let _ = reply.send(result);
            }
            Job::Assignment { assignment, roots, deadline, reply } if Instant::now() < deadline => {
                let result = (|| {
                    for root in roots.iter().map(|path| project_root(path)).collect::<BTreeSet<_>>() {
                        if root.is_dir() {
                            index.observe_project(&root).map_err(|error| ServiceError::Unavailable(error.to_string()))?;
                        }
                    }
                    index.refresh_known();
                    Ok(index.observations().filter(|entry| entry.document.as_ref().is_some_and(|doc| doc.assignments.contains(&assignment))).map(view).collect())
                })();
                let _ = reply.send(result);
            }
            Job::Query { .. } | Job::Assignment { .. } => {}
        }
    }
}

fn publish(index: &Index, snapshot: &Snapshot) {
    let mut next = Views::new();
    for observation in index.observations() {
        let Some(document) = &observation.document else { continue };
        if !document.assignments.is_empty() {
            let file = Arc::new(view(observation));
            for assignment in &document.assignments {
                next.entry(assignment.clone()).or_default().push(file.clone());
            }
        }
    }
    if let Ok(mut snapshot) = snapshot.write() {
        let previous = std::mem::replace(&mut *snapshot, Arc::new(next));
        drop(snapshot);
        drop(previous);
    }
}

fn view(observation: &Observation) -> FileBindingView {
    let document = observation.document.as_ref();
    FileBindingView {
        path: observation.path.to_string_lossy().into_owned(),
        schema: document.map(|doc| doc.schema.clone()),
        description: document.and_then(|doc| doc.description.clone()),
        raw_status: document.and_then(|doc| doc.raw_status.clone()),
        status: document.map(|doc| doc.status.as_str().to_owned()),
        diagnostic: observation.diagnostic.clone(),
        checked_at: observation.checked_at,
        last_success_at: observation.last_success_at,
    }
}
