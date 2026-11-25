//! Single database owner composition. SQLite is the generation authority;
//! the admission mirror follows commit and fails closed on publication loss.

#[cfg(test)]
mod acceptance_tests;

use super::{Application, Request, RequestContext, Response, ServiceError, ServiceResult};
use crate::daemon::DatabaseGeneration;
use crate::db::AgentxDb;

pub(crate) struct SqliteApplication {
    db: AgentxDb,
    reminders: crate::events::reminders::Schedule,
    mailbox_backfill: Option<std::time::Instant>,
    human_mailbox_backfill: Option<std::time::Instant>,
    conversation_backfill: Option<std::time::Instant>,
    generation: DatabaseGeneration,
}

impl SqliteApplication {
    pub(crate) fn new(mut db: AgentxDb, generation: DatabaseGeneration) -> ServiceResult<Self> {
        super::maintenance_store::publish_dev_root(&db)?;
        let committed = db.database_generation().map_err(|error| ServiceError::Internal(error.to_string()))?;
        generation.publish(committed).map_err(|error| ServiceError::Unavailable(error.to_string()))?;
        if std::env::var_os("AGENTX_ASYNC_EVENT_OBSERVERS").as_deref() == Some(std::ffi::OsStr::new("1")) && db.enable_observer_hints(generation.clone()).is_err() {
            crate::log::log_warn("events", "observer.hints_unavailable", "Keeping synchronous observer wakes");
        }
        let use_subscription_indexes = std::env::var_os("AGENTX_INDEX_EVENT_SUBSCRIPTIONS").as_deref().is_none_or(|value| value == std::ffi::OsStr::new("1"));
        if use_subscription_indexes && db.enable_subscription_indexes().is_err() {
            crate::log::log_warn("events", "subscription.index_unavailable", "Keeping legacy subscription selection");
        }
        if db.enable_launch_name_index().is_err() {
            crate::log::log_warn("db", "launch.name_index_unavailable", "Keeping legacy stopped-name selection");
        }
        let mailbox_backfill = match db.start_mailbox_routes() {
            Ok(true) => Some(std::time::Instant::now()),
            Ok(false) => None,
            Err(_) => {
                crate::log::log_warn("db", "hook_mailbox.index_unavailable", "Using event journal for Hook mailbox reads");
                None
            }
        };
        let human_mailbox_backfill = match db.start_human_mailbox_index() {
            Ok(true) => Some(std::time::Instant::now()),
            Ok(false) => None,
            Err(_) => {
                crate::log::log_warn("db", "human_mailbox.index_unavailable", "Using event journal for human mailbox reads");
                None
            }
        };
        let conversation_backfill = match db.start_conversation_routes() {
            Ok(true) => Some(std::time::Instant::now()),
            Ok(false) => None,
            Err(_) => {
                crate::log::log_warn("db", "conversation.index_unavailable", "Using event journal for exact conversation reads");
                None
            }
        };
        let mut reminders = crate::events::reminders::Schedule::new(&db);
        reminders.check(&db, || true);
        Ok(Self { db, reminders, generation, mailbox_backfill, human_mailbox_backfill, conversation_backfill })
    }
}

impl Application for SqliteApplication {
    fn read_lane(&self) -> Option<super::ReadLane> {
        Some(super::read_application::lane(self.db.path(), self.generation.clone(), self.db.poll_progress.clone()))
    }

    fn after_request(&mut self) {
        self.reminders.check(&self.db, || !self.generation.current().is_empty());
    }

    fn idle(&mut self) {
        self.after_request();
        let Some((due, backfill)) = [
            (&mut self.mailbox_backfill, AgentxDb::backfill_mailbox_routes as fn(&AgentxDb) -> anyhow::Result<bool>),
            (&mut self.human_mailbox_backfill, AgentxDb::backfill_human_mailbox),
            (&mut self.conversation_backfill, AgentxDb::backfill_conversation_routes),
        ]
        .into_iter()
        .filter(|(due, _)| due.is_some_and(|at| at <= std::time::Instant::now()))
        .min_by_key(|(due, _)| **due) else {
            return;
        };
        *due = match backfill(&self.db) {
            Ok(true) => Some(std::time::Instant::now() + std::time::Duration::from_millis(25)),
            Ok(false) => None,
            Err(_) => Some(std::time::Instant::now() + std::time::Duration::from_secs(1)),
        };
    }

    fn handle(&mut self, context: &RequestContext, request: Request) -> ServiceResult<Response> {
        let committed = match self.db.kv_get("service:database_generation") {
            Ok(Some(value)) if !value.is_empty() => value,
            _ => {
                self.generation.invalidate();
                return Err(ServiceError::Unavailable("database generation unavailable".into()));
            }
        };
        let published = self.generation.current();
        if published.is_empty() || committed != published {
            self.generation.invalidate();
            return Err(ServiceError::Unavailable("database generation mirror is inconsistent".into()));
        }
        if committed != context.database_generation || context.caller.expected_database_generation.as_ref().is_some_and(|expected| expected != &committed) {
            return Err(ServiceError::StaleGeneration);
        }
        let reminders_changed = matches!(&request, Request::Events(crate::events::service::Request::CreateReminder { .. })) || matches!(&request, Request::Events(crate::events::service::Request::Unsubscribe { id }) if id.starts_with("rem-"));
        let result = match request {
            Request::Inspection(request) => crate::inspection::store::handle(&self.db, context, request).map(Response::Inspection),
            Request::Management(request) => crate::management::store::handle(&self.db, context, request).map(Response::Management),
            Request::Collaboration(request) => crate::collaboration::service::handle(&self.db, context, request).map(Response::Collaboration),
            Request::Participants(request) => crate::agents::service::handle(&self.db, context, request).map(Response::Participants),
            Request::CodexHook(event) => super::codex_hook::run(&self.db, context, event).map(Response::CodexHook),
            Request::Messages(request) => crate::messaging::service::handle(&self.db, context, request).map(Response::Messages),
            Request::Hosts(request) => crate::runtime::hosts_store::handle(&self.db, context, request).map(Response::Hosts),
            Request::Events(request) => crate::events::store::handle(&self.db, context, request).map(Response::Events),
            Request::Queries(request) => super::queries_store::handle(&self.db, context, request).map(Response::Queries),
            Request::Maintenance(request) => super::maintenance_store::handle(&self.db, context, request).map(Response::Maintenance),
            Request::Relay(request) => crate::relay::service_store::handle(&self.db, context, request).map(Response::Relay),
        };
        // A handler can commit before response construction fails. Reconcile
        // the key on both Ok and Err before admitting another request.
        let after = match self.db.kv_get("service:database_generation") {
            Ok(Some(after)) if !after.is_empty() => after,
            other => {
                self.generation.invalidate();
                return Err(ServiceError::Unavailable(format!("database generation unavailable after dispatch: {other:?}")));
            }
        };
        if after != committed {
            if let Err(error) = self.generation.publish(after) {
                self.generation.invalidate();
                return Err(ServiceError::Unavailable(error.to_string()));
            }
            self.reminders.after_reset();
            super::maintenance_store::publish_dev_root(&self.db).map_err(|error| ServiceError::OutcomeUnknown(format!("data changed but its local dev-root selector could not be refreshed: {error}")))?;
        } else if reminders_changed && result.is_ok() {
            self.reminders.invalidate();
        }
        result
    }
}
