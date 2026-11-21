//! Independent durable acceptance cases; requests use the offered native types.

use super::*;
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn with_db(body: impl FnOnce(&AgentxDb, RequestContext)) {
    let root = tempfile::Builder::new().prefix("agentx-message-acceptance-").tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    for (index, name) in ["lobo", "nava"].into_iter().enumerate() {
        db.save_instance_named(name, serde_json::json!({"created_at":10.0 + index as f64,"tool":"codex","status":"active","last_event_id":0}).as_object().unwrap()).unwrap();
    }
    let pid = std::process::id();
    let context = RequestContext {
        caller: CallerContext { request_id: RequestId("fixed-submission-id".into()), invocation: InvocationContext { explicit_name: Some("lobo".into()), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid, incarnation: crate::sys::process::identity(pid) },
        daemon_instance: "acceptance-daemon".into(),
        database_generation: db.database_generation().unwrap(),
    };
    body(&db, context);
}

fn submission() -> Submission {
    Submission { recipients: Vec::new(), text: "one immutable broadcast".into(), envelope: Default::default(), external_sender: None, broadcast_confirmed: true, inline_bundle: None }
}

fn accepted(response: Response) -> Acceptance {
    let Response::Accepted(acceptance) = response else {
        panic!("expected durable acceptance");
    };
    acceptance
}

#[test]
fn replay_freezes_original_recipients_after_membership_changes() {
    with_db(|db, context| {
        let input = submission();
        let first = accepted(handle(db, &context, Request::Submit(input.clone())).unwrap());
        assert_eq!(first.recipients, vec!["nava"]);
        db.save_instance_named("miri", serde_json::json!({"created_at":12.0,"tool":"codex","status":"active","last_event_id":0}).as_object().unwrap()).unwrap();
        let preview = handle(db, &context, Request::Preview(input.clone())).unwrap();
        let Response::Preview(preview) = preview else {
            panic!("expected recipient preview");
        };
        assert!(preview.recipients.contains(&"miri".to_owned()));
        let replay = accepted(handle(db, &context, Request::Submit(input)).unwrap());
        assert!(replay.replayed);
        assert_eq!(replay.event, first.event);
        assert_eq!(replay.recipients, first.recipients);
        let count: i64 = db.conn().query_row("SELECT count(*) FROM events WHERE type='message'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 1);
        let Response::Acceptance(Some(lookup)) = handle(db, &context, Request::Acceptance { request_id: context.caller.request_id.clone() }).unwrap() else {
            panic!("acceptance lookup");
        };
        assert_eq!(lookup.event, first.event);
        assert_eq!(lookup.recipients, first.recipients);
    });
}

#[test]
fn generated_bundle_summary_does_not_change_accepted_message_recipients() {
    with_db(|db, context| {
        db.save_instance_named("miri", json!({"created_at":12.0,"tool":"codex","status":"active"}).as_object().unwrap()).unwrap();
        let input = Submission {
            text: "@nava review this".into(),
            inline_bundle: Some(json!({"title":"Note about @miri","description":"Attached context","refs":{"events":["1"],"files":["src/main.rs"],"transcript":["1:normal"]}})),
            ..submission()
        };
        let first = accepted(handle(db, &context, Request::Submit(input.clone())).unwrap());
        assert_eq!(first.recipients, ["nava"]);
        let event: String = db.conn().query_row("SELECT data FROM events WHERE id=?1", [first.event.0], |row| row.get(0)).unwrap();
        let data: serde_json::Value = serde_json::from_str(&event).unwrap();
        assert_eq!(data["mentions"], json!(["nava"]));
        assert_eq!(data["scene_addressing"]["recipients"], json!([{"canonical_name":"nava","incarnation_bits":11.0_f64.to_bits()}]));
        assert!(data["text"].as_str().unwrap().contains("Note about @miri"));
        assert_eq!(accepted(handle(db, &context, Request::Submit(input)).unwrap()).event, first.event);
    });
}

#[test]
fn reused_request_rejects_changed_input_actor_and_replaced_incarnation() {
    with_db(|db, context| {
        let input = submission();
        handle(db, &context, Request::Submit(input.clone())).unwrap();
        let mut changed = input.clone();
        changed.text = "different body".into();
        assert!(matches!(handle(db, &context, Request::Submit(changed)), Err(ServiceError::Conflict(_))));
        let mut other = context.clone();
        other.caller.invocation.explicit_name = Some("nava".into());
        assert!(matches!(handle(db, &other, Request::Submit(input.clone())), Err(ServiceError::Conflict(_))));
        assert!(matches!(handle(db, &other, Request::Acceptance { request_id: context.caller.request_id.clone() }), Err(ServiceError::Forbidden(_))));
        db.update_instance_fields("lobo", serde_json::json!({"created_at":99.0}).as_object().unwrap()).unwrap();
        assert!(matches!(handle(db, &context, Request::Submit(input)), Err(ServiceError::Conflict(_))));
        let count: i64 = db.conn().query_row("SELECT count(*) FROM events WHERE type='message'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 1);
    });
}

#[test]
fn failed_message_commit_retains_neither_event_nor_acceptance() {
    with_db(|db, context| {
        db.conn().execute_batch("CREATE TRIGGER fixture_fail_message BEFORE INSERT ON events WHEN NEW.type='message' BEGIN SELECT RAISE(ABORT, 'fixture message commit failure'); END;").unwrap();
        assert!(handle(db, &context, Request::Submit(submission())).is_err());
        let Response::Acceptance(value) = handle(db, &context, Request::Acceptance { request_id: context.caller.request_id.clone() }).unwrap() else {
            panic!("acceptance lookup");
        };
        assert!(value.is_none());
        assert_eq!(db.conn().query_row::<i64, _, _>("SELECT count(*) FROM events WHERE type='message'", [], |row| row.get(0)).unwrap(), 0);
        db.conn().execute_batch("DROP TRIGGER fixture_fail_message").unwrap();
        assert!(!accepted(handle(db, &context, Request::Submit(submission())).unwrap()).replayed);
    });
}

#[test]
fn nested_write_scopes_publish_wakes_only_after_outer_commit() {
    use std::io::ErrorKind;
    let root = tempfile::Builder::new().prefix("agentx-postcommit-").tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    db.upsert_notify_endpoint("lobo", "pty", listener.local_addr().unwrap().port()).unwrap();
    let absent = || assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
    db.with_write_scope(|| {
        db.with_write_scope(|| {
            db.wake_after_commit("lobo", &[crate::runtime::notify::WakeKind::Pty]);
            Ok(())
        })?;
        absent();
        Ok(())
    })
    .unwrap();
    listener.accept().expect("committed nested wake missing");
    absent();
    let failed: anyhow::Result<()> = db.with_write_scope(|| {
        db.with_write_scope(|| {
            db.wake_after_commit("lobo", &[crate::runtime::notify::WakeKind::Pty]);
            Ok(())
        })?;
        anyhow::bail!("outer rollback fixture")
    });
    assert!(failed.is_err());
    absent();
    db.with_write_scope(|| {
        let failed: anyhow::Result<()> = db.with_write_scope(|| {
            db.wake_after_commit("lobo", &[crate::runtime::notify::WakeKind::Pty]);
            anyhow::bail!("nested rollback fixture")
        });
        assert!(failed.is_err());
        Ok(())
    })
    .unwrap();
    absent();
}

#[test]
fn output_backlog_metadata_and_pending_after_preserve_bounded_receipt_frontier() {
    use crate::agents::attachments::{AttachmentIntent, AttachmentOwnerId, AttachmentOwnerProcess, AttachmentRequest, ExecutionId, ProviderSessionKey};
    for managed in [false, true] {
        with_db(|db, mut context| {
            let participant = SqliteAttachmentStore::new(db).participant("nava").unwrap();
            if managed {
                let mut store = SqliteAttachmentStore::for_owner_process(db, AttachmentOwnerProcess { pid: context.process.pid, process_identity: context.process.incarnation.clone() });
                store
                    .attach(AttachmentRequest {
                        execution: ExecutionId("output-backlog-fixture".into()),
                        owner: AttachmentOwnerId("output-fixture-owner".into()),
                        session: ProviderSessionKey { provider: "codex".into(), scope: "/fixture/output".into(), session_id: "session".into() },
                        intent: AttachmentIntent::Create { provisional: participant.clone() },
                    })
                    .unwrap();
            }
            let mut ids = Vec::new();
            for index in 0..crate::shared::MAX_MESSAGES_PER_DELIVERY + 2 {
                context.caller.request_id = crate::service::RequestId(format!("backlog-{index}"));
                let mut input = submission();
                input.recipients = vec!["nava".into()];
                ids.push(accepted(handle(db, &context, Request::Submit(input)).unwrap()).event);
            }
            // A later lifecycle watermark must not masquerade as message backlog.
            db.log_event("life", "lobo", &serde_json::json!({"action":"unrelated"})).unwrap();
            let Response::Inbox(InboxOutcome::Output(Ok(output))) = handle(db, &context, Request::Inbox(InboxOperation::PrepareOutput { participant: participant.clone(), consumer: InboxConsumer::Hook })).unwrap() else {
                panic!("prepared output response");
            };
            let (messages, through, remaining, backlog_end) = match output {
                PreparedOutput::Unmanaged { messages, receipt, remaining, backlog_end, .. } if !managed => (messages, receipt.through, remaining, backlog_end),
                PreparedOutput::Claimed { messages, batch, remaining, backlog_end, .. } if managed => (messages, batch.window.through, remaining, backlog_end),
                _ => panic!("wrong output management mode"),
            };
            assert_eq!(messages.len(), crate::shared::MAX_MESSAGES_PER_DELIVERY);
            assert_eq!(through, ids[crate::shared::MAX_MESSAGES_PER_DELIVERY - 1]);
            assert_eq!(remaining, 2);
            assert_eq!(backlog_end, ids.last().copied());
            assert_eq!(db.get_instance_full("nava").unwrap().unwrap().last_event_id, 0);
            for (after, expected) in [(through, true), (*ids.last().unwrap(), false)] {
                let Response::Inbox(InboxOutcome::HasPendingAfter(Ok(actual))) = handle(db, &context, Request::Inbox(InboxOperation::HasPendingAfter { participant: participant.clone(), after })).unwrap() else {
                    panic!("pending-after response");
                };
                assert_eq!(actual, expected);
            }
        });
    }
}

#[test]
#[cfg(unix)]
fn legacy_codex_output_recovers_unconfirmed_mail_after_the_owner_exits() {
    with_db(|db, context| {
        let mut input = submission();
        input.recipients = vec!["nava".into()];
        let event = accepted(handle(db, &context, Request::Submit(input)).unwrap()).event;
        let participant = SqliteAttachmentStore::new(db).participant("nava").unwrap();
        let prepare = |ctx: &RequestContext| handle(db, ctx, Request::Inbox(InboxOperation::PrepareOutput { participant: participant.clone(), consumer: InboxConsumer::ExplicitListen })).unwrap();
        let mut child = std::process::Command::new("cat").stdin(std::process::Stdio::piped()).spawn().unwrap();
        let mut owner = context.clone();
        owner.process = CallerProcess { pid: child.id(), incarnation: crate::sys::process::identity(child.id()) };
        let prepared = prepare(&owner);
        let competing = prepare(&context);
        // Closing stdin terminates the child before it can confirm the receipt.
        assert!(child.wait().unwrap().success());
        assert!(matches!(prepared, Response::Inbox(InboxOutcome::Output(Ok(PreparedOutput::Unmanaged { .. })))));
        assert!(matches!(competing, Response::Inbox(InboxOutcome::Output(Ok(PreparedOutput::Deferred)))));
        assert_eq!(db.get_instance_full("nava").unwrap().unwrap().last_event_id, 0);
        let Response::Inbox(InboxOutcome::Output(Ok(PreparedOutput::Unmanaged { receipt, .. }))) = prepare(&context) else { panic!("expected retry of the unread output") };
        assert_eq!(receipt.through, event);
        assert!(matches!(handle(db, &context, Request::Inbox(InboxOperation::ConfirmOutput { receipt, announce: false })).unwrap(), Response::Inbox(InboxOutcome::Done(Ok(())))));
        assert_eq!(db.get_instance_full("nava").unwrap().unwrap().last_event_id, event.0);
    });
}

#[test]
fn output_confirmation_rolls_back_status_failure_then_replays_without_clobbering_newer_activity() {
    with_db(|db, context| {
        let mut input = submission();
        input.recipients = vec!["nava".into()];
        let accepted = accepted(handle(db, &context, Request::Submit(input)).unwrap());
        let participant = SqliteAttachmentStore::new(db).participant("nava").unwrap();
        let Response::Inbox(InboxOutcome::Output(Ok(PreparedOutput::Unmanaged { messages, receipt, .. }))) =
            handle(db, &context, Request::Inbox(InboxOperation::PrepareOutput { participant: participant.clone(), consumer: InboxConsumer::ExplicitListen })).unwrap()
        else {
            panic!("prepared unmanaged output");
        };
        let timestamp = messages.last().unwrap().timestamp.clone().unwrap();
        let mut changed = receipt.clone();
        changed.through = EventId(receipt.through.0 + 1);
        assert!(matches!(handle(db, &context, Request::Inbox(InboxOperation::ConfirmOutputAndStatus { receipt: OutputConfirmation::Unmanaged(changed), announce: true })).unwrap(), Response::Inbox(InboxOutcome::Done(Err(_)))));
        let request = Request::Inbox(InboxOperation::ConfirmOutputAndStatus { receipt: OutputConfirmation::Unmanaged(receipt), announce: true });
        db.conn().execute_batch("CREATE TRIGGER fixture_delivery_status_failure BEFORE INSERT ON events WHEN NEW.type='status' BEGIN SELECT RAISE(ABORT, 'fixture status commit failure'); END;").unwrap();
        assert!(matches!(handle(db, &context, request.clone()).unwrap(), Response::Inbox(InboxOutcome::Done(Err(_)))));
        let row = db.get_instance_full("nava").unwrap().unwrap();
        assert_eq!(row.last_event_id, 0);
        assert_eq!(row.name_announced, 0);
        assert_eq!(row.status, "active");
        db.conn().execute_batch("DROP TRIGGER fixture_delivery_status_failure").unwrap();
        assert!(matches!(handle(db, &context, request.clone()).unwrap(), Response::Inbox(InboxOutcome::Done(Ok(())))));
        let row = db.get_instance_full("nava").unwrap().unwrap();
        assert_eq!(row.last_event_id, accepted.event.0);
        assert_eq!(row.name_announced, 1);
        assert_eq!(row.status_context, "deliver:lobo");
        let data: String = db.conn().query_row("SELECT data FROM events WHERE type='status' AND instance='nava' ORDER BY id DESC LIMIT 1", [], |row| row.get(0)).unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(&data).unwrap()["msg_ts"], timestamp);
        let watermark = db.get_last_event_id();
        db.update_instance_fields("nava", serde_json::json!({"status":"blocked","status_context":"hook:newer"}).as_object().unwrap()).unwrap();
        assert!(matches!(handle(db, &context, request).unwrap(), Response::Inbox(InboxOutcome::Done(Ok(())))));
        assert_eq!(db.get_last_event_id(), watermark);
        assert_eq!(db.get_instance_full("nava").unwrap().unwrap().status_context, "hook:newer");
    });
}

#[test]
fn message_delivery_report_keeps_acceptance_pending_and_fences_successors() {
    use crate::messaging::delivery_report::{MailboxProgress, report};
    with_db(|db, context| {
        let accepted = accepted(handle(db, &context, Request::Submit(submission())).unwrap());
        let first = report(db, accepted.event.0).unwrap();
        assert_eq!(first.recipients[0].mailbox, MailboxProgress::Pending);
        assert!(first.recipients[0].host.is_none());
        assert_eq!(db.get_instance_full("nava").unwrap().unwrap().last_event_id, 0);
        db.update_instance_fields("nava", serde_json::json!({"last_event_id":accepted.event.0}).as_object().unwrap()).unwrap();
        assert_eq!(report(db, accepted.event.0).unwrap().recipients[0].mailbox, MailboxProgress::Confirmed);
        db.update_instance_fields("nava", serde_json::json!({"created_at":99.0,"last_event_id":accepted.event.0+100}).as_object().unwrap()).unwrap();
        let retired = report(db, accepted.event.0).unwrap();
        assert_eq!(retired.recipients[0].mailbox, MailboxProgress::TargetRetired);
        assert_eq!(retired.recipients[0].participant, first.recipients[0].participant);
        assert!(report(db, accepted.event.0 + 999).is_err());
    });
}
