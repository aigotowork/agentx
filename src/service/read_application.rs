//! Selected native projections and PTY delivery snapshots share a daemon-owned WAL reader.
mod bindings;
mod views;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{Application, ReadLane, Request, RequestContext, Response, ServiceError, ServiceResult};
use crate::daemon::DatabaseGeneration;
use crate::db::AgentxDb;
use crate::file_bindings::Assignment;
use crate::management::contract::Request as Management;
use crate::messaging::{delivery_service as delivery, service as messages};
use crate::runtime::poll_progress::PollProgress;

pub(super) fn lane(path: &Path, generation: DatabaseGeneration, poll_progress: PollProgress) -> ReadLane {
    ReadLane { accepts, application: Box::new(Reader { path: path.to_owned(), generation, db: None, file_bindings: bindings::Bindings::start(), views: views::ViewCache::default(), poll_progress }) }
}

fn accepts(request: &Request) -> bool {
    matches!(
        request,
        Request::Management(
            Management::List { .. }
                | Management::ListWorkspaceMembers { .. }
                | Management::SelectLabels { .. }
                | Management::Agent(_)
                | Management::AdoptedWorkspaces
                | Management::HumanMailbox { .. }
                | Management::OfficeActivity { .. }
                | Management::Messages { .. }
                | Management::MessageHistory { .. }
                | Management::LaunchRelations { .. }
        ) | Request::Management(Management::Operation { .. })
            | Request::Messages(messages::Request::Delivery(delivery::Request::Poll))
            | Request::Queries(super::queries::Request::FileBindings(_) | super::queries::Request::AgentNames(_) | super::queries::Request::AgentInfo { .. })
    )
}

struct Reader {
    path: PathBuf,
    generation: DatabaseGeneration,
    db: Option<AgentxDb>,
    file_bindings: Option<bindings::Bindings>,
    views: views::ViewCache,
    poll_progress: PollProgress,
}

fn unavailable(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Unavailable(error.to_string())
}

fn check_generation(db: &AgentxDb, context: &RequestContext, published: &DatabaseGeneration) -> ServiceResult<()> {
    let durable = db.read_database_generation().map_err(unavailable)?;
    if durable != context.database_generation || published.current() != durable || context.caller.expected_database_generation.as_ref().is_some_and(|expected| expected != &durable) {
        return Err(ServiceError::StaleGeneration);
    }
    Ok(())
}

/// Bound query work and release the SQLite snapshot before any file reads.
fn in_snapshot<T>(db: &AgentxDb, context: &RequestContext, generation: &DatabaseGeneration, read: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let client_deadline = context.caller.deadline_unix_ms.and_then(|millis| UNIX_EPOCH.checked_add(Duration::from_millis(millis)));
    db.with_read_budget(
        move || Instant::now() >= deadline || client_deadline.is_some_and(|end| SystemTime::now() >= end),
        || {
            let _snapshot = db.conn().unchecked_transaction().map_err(unavailable)?;
            check_generation(db, context, generation)?;
            read()
        },
    )
    .map_err(unavailable)?
}

impl Application for Reader {
    fn handle(&mut self, context: &RequestContext, request: Request) -> ServiceResult<Response> {
        if !accepts(&request) {
            return Err(ServiceError::Unsupported("operation cannot run on the read lane".into()));
        }
        let db = match &self.db {
            Some(db) => db,
            None => self.db.insert(AgentxDb::open_reader(&self.path).map_err(unavailable)?),
        };
        let scope = self.path.parent().expect("scoped database path");
        let response = match request {
            Request::Messages(messages::Request::Delivery(delivery::Request::Poll)) => {
                let (snapshot, token) = in_snapshot(db, context, &self.generation, || {
                    let snapshot = crate::messaging::delivery_store::snapshot(db, context)?;
                    let token = crate::runtime::hosts_store::poll_registration_token(db, context, &snapshot.actor.participant)?;
                    Ok((snapshot, token))
                })?;
                check_generation(db, context, &self.generation)?;
                if let Some(token) = token {
                    self.poll_progress.record(token);
                }
                return Ok(Response::Messages(messages::Response::Delivery(delivery::Response::Snapshot(snapshot))));
            }
            Request::Management(Management::Operation { request_id }) => {
                let record = in_snapshot(db, context, &self.generation, || crate::management::store::operation_record(db, context, &request_id))?;
                Response::Management(crate::management::contract::Response::Operation(record.observe(context)))
            }
            Request::Management(request) => {
                let key = views::Key::for_request(&context.database_generation, &request);
                // Sample outside the snapshot. If the writer commits between
                // this sample and the read, the next poll will invalidate it.
                let version = key.as_ref().and_then(|_| db.conn().pragma_query_value(None, "data_version", |row| row.get::<_, i64>(0)).ok());
                let mut response = in_snapshot(db, context, &self.generation, || {
                    if let Some(key) = key {
                        return self.views.read(version, key, || crate::management::store::handle(db, context, request));
                    }
                    crate::management::store::handle(db, context, request)
                })?;
                if let Some(bindings) = self.file_bindings.as_mut() {
                    bindings.enrich(scope, &context.database_generation, &mut response);
                }
                Response::Management(response)
            }
            Request::Queries(super::queries::Request::FileBindings(query)) => {
                let assignment = in_snapshot(db, context, &self.generation, || {
                    let Some(name) = &query.agent else { return Ok(None) };
                    let row = db.get_instance_full(name).map_err(unavailable)?.ok_or_else(|| ServiceError::NotFound(format!("Agent {name}")))?;
                    Ok(Some(Assignment {
                        scope: scope.to_string_lossy().into_owned(),
                        generation: context.database_generation.clone(),
                        participant: crate::agents::attachments::ParticipantRef { canonical_name: row.name, incarnation_bits: row.created_at.to_bits() },
                    }))
                })?;
                let bindings = self.file_bindings.as_ref().ok_or_else(|| unavailable("file binding worker unavailable"))?;
                let mut projection = bindings.query(query, &context.caller.invocation.cwd)?;
                if let Some(assignment) = &assignment {
                    projection.files.retain(|entry| entry.document.as_ref().is_some_and(|doc| doc.assignments.contains(assignment)));
                }
                Response::Queries(super::queries::Response::FileBindings(projection))
            }
            Request::Queries(super::queries::Request::AgentInfo { target, section: super::queries::info::Section::Bindings }) => {
                let (participant, mut roots) = in_snapshot(db, context, &self.generation, || super::queries_store::info_store::binding_input(db, &target))?;
                let files = if let Some(participant) = participant {
                    let bindings = self.file_bindings.as_ref().ok_or_else(|| unavailable("file binding worker unavailable"))?;
                    roots.push(scope.to_owned());
                    let assignment = Assignment { scope: scope.to_string_lossy().into_owned(), generation: context.database_generation.clone(), participant };
                    bindings.for_assignment(assignment, roots)?
                } else {
                    Vec::new()
                };
                Response::Queries(super::queries::Response::AgentInfo(super::queries::info::Projection::Bindings(files)))
            }
            Request::Queries(request @ (super::queries::Request::AgentNames(_) | super::queries::Request::AgentInfo { .. })) => {
                in_snapshot(db, context, &self.generation, || super::queries_store::handle(db, context, request)).map(Response::Queries)?
            }
            _ => unreachable!("only selected read requests reach the reader"),
        };
        // The writer may have reset while the reader fetched or enriched the old view.
        check_generation(db, context, &self.generation)?;
        Ok(response)
    }
}
