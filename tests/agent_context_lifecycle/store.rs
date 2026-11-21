//! Production context store against an in-memory KV and pre-resolved actor seam.
//! These cases establish store decisions, not real authentication or SQLite atomicity.
#![allow(dead_code)]
mod agents {
    pub mod attachments {
        #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        pub struct ParticipantRef { pub canonical_name: String, pub incarnation_bits: u64 }
    }
}
#[path = "../../src/runtime/agent_context.rs"]
mod subject;
mod runtime { pub(crate) use crate::subject as agent_context; }
#[path = "../../src/agents/context_service.rs"]
mod context_service;
#[path = "../../src/agents/context_store.rs"]
mod context_store;

mod sys { pub mod process { pub fn identity(_: u32) -> Option<String> { Some("process-a".into()) } pub fn is_alive(_: u32) -> bool { true } } }
mod db {
    use std::{cell::RefCell, collections::BTreeMap};
    #[derive(Default)]
    pub struct AgentxDb { pub values: RefCell<BTreeMap<String, String>> }
    impl AgentxDb {
        pub fn kv_get(&self, key: &str) -> anyhow::Result<Option<String>> { Ok(self.values.borrow().get(key).cloned()) }
        pub fn kv_set(&self, key: &str, value: Option<&str>) -> anyhow::Result<()> {
            match value {
                Some(value) => { self.values.borrow_mut().insert(key.into(), value.into()); }
                None => { self.values.borrow_mut().remove(key); }
            }
            Ok(())
        }
        pub fn with_write_scope<T>(&self, operation: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
            let before = self.values.borrow().clone();
            let result = operation();
            if result.is_err() { *self.values.borrow_mut() = before; }
            result
        }
    }
}
mod service {
    use crate::{agents::attachments::ParticipantRef, db::AgentxDb};
    #[derive(Clone, Debug)]
    pub enum ServiceError { Internal(String), InvalidInput(String), StaleFence }
    impl std::fmt::Display for ServiceError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
    }
    impl std::error::Error for ServiceError {}
    pub type ServiceResult<T> = Result<T, ServiceError>;
    #[derive(Clone)]
    pub struct Process { pub pid: u32, pub incarnation: Option<String> }
    #[derive(Clone)]
    pub struct Record { pub session_id: Option<String>, pub tool: String }
    #[derive(Clone)]
    pub struct Actor { pub participant: ParticipantRef, pub record: Record }
    #[derive(Clone)]
    pub struct RequestContext {
        pub database_generation: String,
        pub process: Process,
        pub actor: Actor,
        pub authenticated: bool,
    }
    pub fn resolve_actor(_: &AgentxDb, context: &RequestContext) -> ServiceResult<Actor> {
        if !context.authenticated { return Err(ServiceError::StaleFence); }
        Ok(context.actor.clone())
    }
}
use agents::attachments;
use agents::attachments::ParticipantRef;
use context_service::{Request, Response};
use db::AgentxDb;
use service::*;
use subject::{FrozenInstructions, MAX_CONTEXT_BYTES};

fn context(name: &str, incarnation: u64, session: Option<&str>) -> RequestContext {
    RequestContext {
        database_generation: "generation-a".into(),
        process: Process { pid: 100, incarnation: Some("process-a".into()) },
        actor: Actor {
            participant: ParticipantRef { canonical_name: name.into(), incarnation_bits: incarnation },
            record: Record { session_id: session.map(str::to_owned), tool: "codex".into() },
        },
        authenticated: true,
    }
}
fn read(db: &AgentxDb, ctx: &RequestContext) -> Option<String> {
    match context_store::handle(db, ctx, Request::ReadInstructions { participant: ctx.actor.participant.clone() }).unwrap() {
        Response::Instructions(value) => value.map(|value| value.text),
        other => panic!("unexpected response: {other:?}"),
    }
}
fn freeze(db: &AgentxDb, ctx: &RequestContext, value: &str) -> ServiceResult<Response> {
    context_store::handle(db, ctx, Request::FreezeInstructions { participant: ctx.actor.participant.clone(), instructions: FrozenInstructions { text: value.into() } })
}
fn prepare(db: &AgentxDb, ctx: &RequestContext, session: &str) -> ServiceResult<Option<String>> {
    match context_store::handle(db, ctx, Request::PrepareInitial { participant: ctx.actor.participant.clone(), session_id: session.into() })? {
        Response::InitialPrepared { receipt } => Ok(receipt),
        other => panic!("unexpected response: {other:?}"),
    }
}
fn confirm(db: &AgentxDb, ctx: &RequestContext, receipt: &str) -> ServiceResult<Response> {
    context_store::handle(db, ctx, Request::ConfirmInitial { participant: ctx.actor.participant.clone(), receipt: receipt.into() })
}

#[test]
fn frozen_value_and_empty_value_are_write_once() {
    for text in ["fixed instructions", ""] {
        let db = AgentxDb::default();
        let ctx = context("a", 1, None);
        assert_eq!(read(&db, &ctx), None);
        freeze(&db, &ctx, text).unwrap();
        freeze(&db, &ctx, "changed instructions").unwrap();
        assert_eq!(read(&db, &ctx).as_deref(), Some(text));
        let mut attached = ctx.clone();
        attached.actor.record.session_id = Some("session-a".into());
        assert_eq!(read(&db, &attached).as_deref(), Some(text));
        let resumed = context("a", 2, Some("session-a"));
        assert_eq!(read(&db, &resumed).as_deref(), Some(text));
    }
}

#[test]
fn fork_tool_session_generation_and_actor_fences_isolate_snapshots() {
    let db = AgentxDb::default();
    let ctx = context("a", 1, Some("session-a"));
    freeze(&db, &ctx, "identity-a").unwrap();
    for mut other in [context("b", 2, Some("session-a")), context("a", 2, Some("session-b")), context("a", 2, Some("session-a"))] {
        if other.actor.participant.incarnation_bits == 2 && other.actor.record.session_id.as_deref() == Some("session-a") && other.actor.participant.canonical_name == "a" {
            other.actor.record.tool = "claude".into();
        }
        assert_eq!(read(&db, &other), None);
    }
    let mut generation = ctx.clone();
    generation.database_generation = "generation-b".into();
    assert_eq!(read(&db, &generation), None);
    let before = db.values.borrow().clone();
    let foreign = Request::ReadInstructions { participant: context("b", 2, None).actor.participant };
    assert!(matches!(context_store::handle(&db, &ctx, foreign), Err(ServiceError::StaleFence)));
    assert_eq!(*db.values.borrow(), before);
}

#[test]
fn size_limit_rejects_without_partial_writes() {
    let db = AgentxDb::default();
    let ctx = context("a", 1, Some("session-a"));
    assert!(matches!(freeze(&db, &ctx, &"x".repeat(MAX_CONTEXT_BYTES + 1)), Err(ServiceError::InvalidInput(_))));
    assert!(db.values.borrow().is_empty());
    freeze(&db, &ctx, &"é".repeat(MAX_CONTEXT_BYTES / 2)).unwrap();
    assert_eq!(read(&db, &ctx).unwrap().len(), MAX_CONTEXT_BYTES);
}

#[test]
fn receipt_fences_and_sequential_delivery_ignore_legacy_bootstrap_flags() {
    let db = AgentxDb::default();
    let ctx = context("a", 1, Some("session-a"));
    db.kv_set("name_announced", Some("true")).unwrap();
    db.kv_set("bootstrap", Some("delivered")).unwrap();
    let receipt = prepare(&db, &ctx, "session-a").unwrap().unwrap();
    let mut bad_process = ctx.clone();
    bad_process.process.pid += 1;
    let mut bad_incarnation = ctx.clone();
    bad_incarnation.process.incarnation = Some("process-b".into());
    let mut unauthenticated = ctx.clone();
    unauthenticated.authenticated = false;
    for other in [bad_process, bad_incarnation, unauthenticated, context("a", 2, Some("session-a")), context("a", 1, None), context("a", 1, Some("foreign"))] {
        let before = db.values.borrow().clone();
        assert!(matches!(confirm(&db, &other, &receipt), Err(ServiceError::StaleFence)));
        assert_eq!(*db.values.borrow(), before);
    }
    for session in ["", "foreign"] {
        assert!(matches!(prepare(&db, &ctx, session), Err(ServiceError::StaleFence)));
    }
    confirm(&db, &ctx, &receipt).unwrap();
    assert!(prepare(&db, &ctx, "session-a").unwrap().is_none());
    let new_session = context("a", 1, Some("session-b"));
    assert!(prepare(&db, &new_session, "session-b").unwrap().is_some());
}

#[test]
fn initial_delivery_does_not_issue_two_outstanding_grants() {
    let db = AgentxDb::default();
    let ctx = context("a", 1, Some("session-a"));
    assert!(prepare(&db, &ctx, "session-a").unwrap().is_some());
    assert!(prepare(&db, &ctx, "session-a").unwrap().is_none(), "second callback can deliver while first receipt remains outstanding");
}
