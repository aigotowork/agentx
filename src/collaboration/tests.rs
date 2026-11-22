//! Authored domain/transaction cases; independent acceptance owns execution.
use super::service::*;
use crate::{
    agents::attachments::ParticipantRef,
    db::AgentxDb,
    service::{CallerContext, CallerProcess, InvocationContext, RequestContext, RequestId},
};

fn participant(name: &str, created: f64) -> ParticipantRef {
    ParticipantRef { canonical_name: name.into(), incarnation_bits: created.to_bits() }
}
fn brief() -> BriefInput {
    BriefInput { title: "CSV export".into(), goal: "Refuse accidental overwrite".into(), criteria: vec![Criterion { id: "no-overwrite".into(), text: "Existing file is unchanged".into(), required: true }], source: None }
}
fn work() -> Work {
    Work::create(WorkRef { scope: "/scope".into(), generation: "generation".into(), project: "/project".into(), id: "work-id".into() }, brief(), participant("impl", 1.0), participant("check", 2.0), "now".into()).unwrap()
}
fn operator(work: &mut Work, command: OperatorCommand, id: &str) {
    work.apply_operator(command, id, "now").unwrap();
}
fn result_input() -> ResultInput {
    ResultInput { summary: "Implemented CSV policy".into(), source: SourceBasis { reference: "git".into(), revision: "a".repeat(40) }, artifacts: vec![Artifact::Git { revision: "a".repeat(40), path: "export.rs".into() }] }
}
fn result(work: &mut Work, id: &str) {
    operator(work, OperatorCommand::DispatchAssignment { role: Role::Implementer }, &format!("dispatch-{id}"));
    work.report(work.implementer.clone(), work.brief().revision, &participant("impl", 1.0), AgentReport::Result(result_input()), id, "now").unwrap();
}
fn request_check(work: &mut Work, result_id: &str, id: &str) {
    operator(work, OperatorCommand::RequestCheck { result_id: result_id.into(), reason: "Inspect immutable source".into() }, id);
}
fn check_input(work: &Work, verdict: Verdict) -> CheckInput {
    CheckInput {
        request_id: work.check_requests.last().unwrap().id.clone(),
        result_id: work.results.last().unwrap().id.clone(),
        observed_fingerprint: "a".repeat(40),
        verdict: verdict.clone(),
        criteria: vec![CriterionCheck { criterion_id: "no-overwrite".into(), verdict, evidence: "Ran overwrite check; original bytes retained".into() }],
        evidence: vec!["git revision and test output observed".into()],
    }
}
fn publish_check(work: &mut Work, input: CheckInput, id: &str) -> Result<(), Failure> {
    work.report(work.checker.clone(), work.brief().revision, &participant("check", 2.0), AgentReport::Check(input), id, "now")
}

#[test]
fn failed_check_changes_new_result_pass_and_exact_acceptance() {
    let mut w = work();
    result(&mut w, "r1");
    request_check(&mut w, "r1", "request1");
    let failed = check_input(&w, Verdict::Failed);
    publish_check(&mut w, failed, "c1").unwrap();
    assert!(w.eligibility().selection.is_none());
    operator(&mut w, OperatorCommand::RequestChanges { result_id: "r1".into(), check_id: Some("c1".into()), reason: "Fix overwrite".into() }, "changes");
    request_check(&mut w, "r1", "request2");
    let passed = check_input(&w, Verdict::Passed);
    publish_check(&mut w, passed, "c2").unwrap();
    assert!(w.eligibility().selection.is_none(), "same-result pass cannot erase requested changes");
    result(&mut w, "r2");
    request_check(&mut w, "r2", "request3");
    let passed = check_input(&w, Verdict::Passed);
    publish_check(&mut w, passed, "c3").unwrap();
    let selected = w.eligibility().selection.unwrap();
    operator(&mut w, OperatorCommand::AcceptResult(selected.clone()), "accept");
    assert!(w.closed());
    assert_eq!(w.results.len(), 2);
    assert_eq!(w.checks.len(), 3);
    assert!(matches!(w.apply_operator(OperatorCommand::SendNote { roles: vec![Role::Implementer], text: "late".into() }, "late", "now"), Err(Failure::Closed)));
    assert_eq!(w.decisions.last().unwrap().check_id.as_deref(), Some(selected.check_id.as_str()));
}

#[test]
fn same_result_recheck_open_question_and_brief_change_remove_eligibility() {
    let mut w = work();
    result(&mut w, "r1");
    request_check(&mut w, "r1", "request1");
    let passed = check_input(&w, Verdict::Passed);
    publish_check(&mut w, passed, "c1").unwrap();
    let previous = w.eligibility().selection.unwrap();
    request_check(&mut w, "r1", "request2");
    assert!(w.eligibility().selection.is_none());
    assert!(matches!(w.apply_operator(OperatorCommand::AcceptResult(previous), "stale-accept", "now"), Err(Failure::Ineligible { .. })));
    let passed = check_input(&w, Verdict::Passed);
    publish_check(&mut w, passed, "c2").unwrap();
    w.report(w.implementer.clone(), 1, &participant("impl", 1.0), AgentReport::Question { prompt: "Overwrite?".into(), suggestions: vec![] }, "q1", "now").unwrap();
    assert!(w.eligibility().selection.is_none());
    operator(&mut w, OperatorCommand::AnswerQuestion { question_id: "q1".into(), text: "Refuse".into(), amendment: None }, "answer");
    assert!(w.eligibility().selection.is_some());
    let mut amended = brief();
    amended.goal = "Additional requirement".into();
    operator(&mut w, OperatorCommand::UpdateBrief(amended), "brief");
    assert!(w.eligibility().selection.is_none());
    assert!(matches!(w.report(w.implementer.clone(), 1, &participant("impl", 1.0), AgentReport::Progress { kind: ProgressKind::Started, summary: "late".into() }, "late", "now"), Err(Failure::Forbidden)));
    assert_eq!(w.briefs.len(), 2);
    assert_eq!(w.checks.len(), 2);
}

#[test]
fn checker_requires_exact_actor_current_request_matching_source_and_full_coverage() {
    let mut w = work();
    result(&mut w, "r1");
    request_check(&mut w, "r1", "request1");
    let mut input = check_input(&w, Verdict::Passed);
    input.criteria.clear();
    assert!(matches!(publish_check(&mut w, input, "missing"), Err(Failure::InvalidInput { .. })));
    let mut input = check_input(&w, Verdict::Passed);
    input.observed_fingerprint = "b".repeat(40);
    assert!(matches!(publish_check(&mut w, input, "different-source"), Err(Failure::InvalidInput { .. })));
    let input = check_input(&w, Verdict::Passed);
    assert!(matches!(w.report(w.checker.clone(), 1, &participant("check", 3.0), AgentReport::Check(input), "reused-name", "now"), Err(Failure::Forbidden)));
    let mut input = check_input(&w, Verdict::Passed);
    input.request_id = "unrequested".into();
    assert!(matches!(publish_check(&mut w, input, "wrong-request"), Err(Failure::Forbidden)));
    assert!(w.checks.is_empty());
    assert!(matches!(w.apply_operator(OperatorCommand::BindParticipant { role: Role::Checker, participant: participant("impl", 1.0) }, "same-person", "now"), Err(Failure::InvalidInput { .. })));
}

#[test]
fn replacement_preserves_question_author_and_amendment_targets_current_assignment() {
    let mut w = work();
    operator(&mut w, OperatorCommand::DispatchAssignment { role: Role::Implementer }, "dispatch");
    let old = w.implementer.clone();
    w.report(old.clone(), 1, &participant("impl", 1.0), AgentReport::Question { prompt: "Policy?".into(), suggestions: vec![] }, "q", "now").unwrap();
    operator(&mut w, OperatorCommand::BindParticipant { role: Role::Implementer, participant: participant("replacement", 3.0) }, "replace");
    let (_, pending) = w.apply_operator(OperatorCommand::AnswerQuestion { question_id: "q".into(), text: "Refuse overwrite".into(), amendment: Some(brief()) }, "answer", "now").unwrap();
    assert_eq!(pending.unwrap().roles, vec![Role::Implementer]);
    assert_eq!(w.questions[0].assignment_id, old);
    assert_eq!(w.questions[0].answer.as_ref().unwrap().brief_revision, 2);
    assert_eq!(w.assignment(&Role::Implementer).participant.canonical_name, "replacement");
    assert_eq!(w.assignment(&Role::Implementer).dispatched_brief, Some(2));
    assert!(matches!(w.report(old, 2, &participant("impl", 1.0), AgentReport::Result(result_input()), "late", "now"), Err(Failure::Forbidden)));
}

#[test]
fn artifact_paths_and_mutable_git_branches_cannot_escape_reported_identity() {
    let mut w = work();
    operator(&mut w, OperatorCommand::DispatchAssignment { role: Role::Implementer }, "dispatch");
    for artifact in [Artifact::Path { path: "../private".into(), fingerprint: "hash".into() }, Artifact::Path { path: "/etc/passwd".into(), fingerprint: "hash".into() }, Artifact::Git { revision: "HEAD".into(), path: "export.rs".into() }] {
        let mut input = result_input();
        input.artifacts = vec![artifact];
        assert!(matches!(w.report(w.implementer.clone(), 1, &participant("impl", 1.0), AgentReport::Result(input), "bad", "now"), Err(Failure::InvalidInput { .. })));
    }
    let mut mutable = result_input();
    mutable.source.revision = "HEAD".into();
    assert!(matches!(w.report(w.implementer.clone(), 1, &participant("impl", 1.0), AgentReport::Result(mutable), "mutable-source", "now"), Err(Failure::InvalidInput { .. })));
}

struct Fixture {
    _root: tempfile::TempDir,
    db: AgentxDb,
    context: RequestContext,
    project: String,
    generation: String,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().canonicalize().unwrap().to_str().unwrap().to_owned();
        let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
        let generation = db.database_generation().unwrap();
        for (name, created) in [("impl", 1.0), ("check", 2.0)] {
            db.save_instance_named(name, serde_json::json!({"created_at":created,"directory":project,"status":"active","tool":"codex"}).as_object().unwrap()).unwrap();
        }
        let context = RequestContext {
            caller: CallerContext { request_id: RequestId("transport".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some(generation.clone()) },
            process: CallerProcess { pid: std::process::id(), incarnation: None },
            daemon_instance: "daemon".into(),
            database_generation: generation.clone(),
        };
        Self { _root: root, db, context, project, generation }
    }
    fn call(&self, call: CommandCall) -> crate::service::ServiceResult<Response> {
        handle(&self.db, &self.context, Request::Apply(call))
    }
    fn create_call(&self) -> CommandCall {
        CommandCall {
            command_id: "create".into(),
            generation: self.generation.clone(),
            project: self.project.clone(),
            work: None,
            expected_revision: None,
            command: Command::Operator(OperatorCommand::CreateWork { brief: brief(), implementer: participant("impl", 1.0), checker: participant("check", 2.0) }),
        }
    }
    fn create(&self) -> Change {
        let Response::Applied(change) = self.call(self.create_call()).unwrap() else { panic!("create rejected") };
        change
    }
    fn mutation(&self, change: &Change, id: &str, command: OperatorCommand) -> CommandCall {
        CommandCall { command_id: id.into(), generation: self.generation.clone(), project: self.project.clone(), work: Some(change.reference.clone()), expected_revision: Some(change.revision), command: Command::Operator(command) }
    }
}

#[test]
fn durable_replay_precedes_revision_and_rejects_changed_intent() {
    let f = Fixture::new();
    let created = f.create();
    let dispatch = f.mutation(&created, "dispatch", OperatorCommand::DispatchAssignment { role: Role::Implementer });
    let Response::Applied(first) = f.call(dispatch.clone()).unwrap() else { panic!("dispatch rejected") };
    assert_eq!(first.messages.len(), 1);
    let Response::Applied(replayed) = f.call(dispatch.clone()).unwrap() else { panic!("replay rejected") };
    assert!(replayed.replayed);
    assert_eq!(first.messages[0].event, replayed.messages[0].event);
    let Response::Applied(replayed_create) = f.call(f.create_call()).unwrap() else { panic!("create replay rejected") };
    assert_eq!(replayed_create.reference, created.reference);
    let mut changed = dispatch;
    changed.expected_revision = Some(first.revision);
    assert!(matches!(f.call(changed).unwrap(), Response::Rejected(Failure::CommandConflict)));
    let stale = f.mutation(&created, "another", OperatorCommand::SendNote { roles: vec![Role::Implementer], text: "note".into() });
    assert!(matches!(f.call(stale).unwrap(), Response::Rejected(Failure::RevisionConflict { .. })));
}

#[test]
fn failed_aggregate_write_rolls_back_message_acceptance_receipt_and_revision() {
    let f = Fixture::new();
    let created = f.create();
    let count = || f.db.conn().query_row("SELECT count(*) FROM events WHERE type='message'", [], |r| r.get::<_, i64>(0)).unwrap();
    let before = count();
    f.db.conn().execute_batch("CREATE TRIGGER fail_work BEFORE UPDATE ON collaboration_work BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    let dispatch = f.mutation(&created, "dispatch", OperatorCommand::DispatchAssignment { role: Role::Implementer });
    assert!(f.call(dispatch.clone()).is_err());
    assert_eq!(count(), before);
    assert!(f.db.collaboration_command(&f.generation, "dispatch").unwrap().is_none());
    assert!(f.db.message_acceptance(&f.generation, "work-message-dispatch").unwrap().is_none());
    let work: Work = serde_json::from_str(&f.db.collaboration_work(&f.generation, &created.reference.id).unwrap().unwrap()).unwrap();
    assert_eq!(work.revision, 1);
    assert!(work.messages.is_empty());
    f.db.conn().execute_batch("DROP TRIGGER fail_work").unwrap();
    assert!(matches!(f.call(dispatch).unwrap(), Response::Applied(_)));
    assert_eq!(count(), before + 1);
}

#[test]
fn exact_incarnation_generation_project_and_capacity_are_native_guards() {
    let f = Fixture::new();
    let created = f.create();
    let wrong = f.mutation(&created, "wrong-incarnation", OperatorCommand::BindParticipant { role: Role::Implementer, participant: participant("impl", 99.0) });
    assert!(matches!(f.call(wrong).unwrap(), Response::Rejected(Failure::StaleReference)));
    let mut stale = f.mutation(&created, "stale", OperatorCommand::DispatchAssignment { role: Role::Implementer });
    stale.generation = "replaced".into();
    assert!(matches!(f.call(stale).unwrap(), Response::Rejected(Failure::StaleReference)));
    let mut wrong_project = f.mutation(&created, "cross-project", OperatorCommand::DispatchAssignment { role: Role::Implementer });
    let elsewhere = f._root.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    wrong_project.project = elsewhere.canonicalize().unwrap().to_str().unwrap().to_owned();
    assert!(matches!(f.call(wrong_project).unwrap(), Response::Rejected(Failure::StaleReference)));
    let mut w: Work = serde_json::from_str(&f.db.collaboration_work(&f.generation, &created.reference.id).unwrap().unwrap()).unwrap();
    for _ in 0..MAX_HISTORY {
        w.progress.push(Progress { assignment_id: w.implementer.clone(), brief_revision: 1, kind: ProgressKind::Progress, summary: "retained".into(), at: "now".into() });
    }
    f.db.with_write_scope(|| f.db.collaboration_save(&f.generation, &w.reference.id, &f.project, w.revision, &serde_json::to_string(&w).unwrap())).unwrap();
    let dispatch = f.mutation(&created, "capacity", OperatorCommand::DispatchAssignment { role: Role::Implementer });
    assert!(matches!(f.call(dispatch).unwrap(), Response::Rejected(Failure::Capacity)));
    assert!(f.db.message_acceptance(&f.generation, "work-message-capacity").unwrap().is_none());
}

#[cfg(windows)]
#[test]
fn local_provider_cwd_matches_the_canonical_project_for_candidates_and_assignment() {
    let f = Fixture::new();
    let project = std::path::Path::new(&f.project);
    let provider_cwd = crate::shared::platform::child_process_path(project);
    assert_ne!(provider_cwd, project, "fixture must exercise distinct Windows spellings");
    let fields = serde_json::json!({"directory":provider_cwd});
    f.db.update_instance_fields("impl", fields.as_object().unwrap()).unwrap();
    f.db.save_instance_named("remote", serde_json::json!({"directory":provider_cwd,"origin_device_id":"another-host","created_at":3.0}).as_object().unwrap()).unwrap();
    let request = Request::Read { generation: f.generation.clone(), project: f.project.clone(), read: Read::Candidates(Page { after: None, limit: 10 }) };
    let Response::Read(ReadValue::Candidates { items, .. }) = handle(&f.db, &f.context, request).unwrap() else { panic!("expected candidates") };
    assert!(items.iter().any(|candidate| candidate.participant == participant("impl", 1.0)));
    assert!(!items.iter().any(|candidate| candidate.participant.canonical_name == "remote"));
    let created = f.create();
    assert_eq!(created.reference.project, f.project);
    assert_eq!(created.reference.generation, f.generation);
}

#[test]
fn managed_reset_removes_work_receipts_and_changes_generation_fence() {
    let f = Fixture::new();
    let created = f.create();
    f.db.with_write_scope(|| {
        f.db.clear_managed_state_for_reset()?;
        f.db.kv_set("service:database_generation", Some("new-generation"))
    })
    .unwrap();
    assert!(f.db.collaboration_work(&f.generation, &created.reference.id).unwrap().is_none());
    assert!(f.db.collaboration_command(&f.generation, "create").unwrap().is_none());
}

#[test]
fn native_report_resolves_exact_actor_and_thread_read_has_no_cross_work_leak() {
    let mut f = Fixture::new();
    let created = f.create();
    let dispatch = f.mutation(&created, "dispatch", OperatorCommand::DispatchAssignment { role: Role::Implementer });
    let Response::Applied(dispatched) = f.call(dispatch).unwrap() else { panic!("dispatch rejected") };
    let w: Work = serde_json::from_str(&f.db.collaboration_work(&f.generation, &created.reference.id).unwrap().unwrap()).unwrap();
    let report = CommandCall {
        command_id: "progress".into(),
        generation: f.generation.clone(),
        project: f.project.clone(),
        work: Some(created.reference.clone()),
        expected_revision: Some(dispatched.revision),
        command: Command::Report { assignment_id: w.implementer.clone(), brief_revision: 1, report: AgentReport::Progress { kind: ProgressKind::Acknowledged, summary: "Received".into() } },
    };
    assert!(matches!(f.call(report.clone()), Err(crate::service::ServiceError::IdentityRequired(_))));
    f.context.caller.invocation.explicit_name = Some("check".into());
    assert!(matches!(f.call(report.clone()).unwrap(), Response::Rejected(Failure::Forbidden)));
    f.context.caller.invocation.explicit_name = Some("impl".into());
    assert!(matches!(f.call(report).unwrap(), Response::Applied(_)));
    let read = Request::Read { generation: f.generation.clone(), project: f.project.clone(), read: Read::Thread { work: created.reference.clone(), page: ThreadPage { limit: 1, after: None } } };
    f.db.log_event("message", "impl", &serde_json::json!({"from":"impl","text":"real reply","thread":format!("work-{}",created.reference.id)})).unwrap();
    f.db.log_event("message", "other", &serde_json::json!({"from":"other","text":"private sibling","thread":"work-other"})).unwrap();
    let Response::Read(ReadValue::Thread { items, next }) = handle(&f.db, &f.context, read).unwrap() else { panic!("thread read rejected") };
    assert_eq!(items.len(), 1);
    assert!(next.is_some());
    let read = Request::Read { generation: f.generation.clone(), project: f.project.clone(), read: Read::Thread { work: created.reference, page: ThreadPage { limit: 64, after: next } } };
    let Response::Read(ReadValue::Thread { items, next }) = handle(&f.db, &f.context, read).unwrap() else { panic!("thread continuation rejected") };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].text, "real reply");
    assert!(next.is_none());
}

#[test]
fn sqlite_revision_range_rejects_overflow_without_changing_retained_work() {
    let f = Fixture::new();
    let created = f.create();
    let maximum = u64::try_from(i64::MAX).unwrap();
    let payload = f.db.collaboration_work(&f.generation, &created.reference.id).unwrap().unwrap();
    f.db.with_write_scope(|| f.db.collaboration_save(&f.generation, &created.reference.id, &f.project, maximum, &payload)).unwrap();
    let overflow = f.db.with_write_scope(|| f.db.collaboration_save(&f.generation, &created.reference.id, &f.project, maximum + 1, "must not replace retained payload")).unwrap_err();
    assert!(overflow.to_string().contains("revision exceeds SQLite signed INTEGER range"));
    let stored: (i64, String) = f.db.conn().query_row("SELECT revision,payload FROM collaboration_work WHERE generation=? AND id=?", rusqlite::params![f.generation, created.reference.id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    assert_eq!(stored.0, i64::MAX);
    assert_eq!(stored.1, payload);
    assert_eq!(f.db.collaboration_count(&f.generation).unwrap(), 1);
    assert_eq!(f.db.collaboration_command_count(&f.generation, &created.reference.id).unwrap(), 1);
}
