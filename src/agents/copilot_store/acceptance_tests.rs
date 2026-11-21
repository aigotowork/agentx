//! Independent native Copilot ownership/receipt cases. Provider SDK calls and
//! actual stdout flushes belong to the separate product-host acceptance path.
use crate::agents::attachments::{ActivationTarget, AttachmentIntent, AttachmentOwnerId, AttachmentOwnerProcess, AttachmentRequest, AttachmentStore, ExecutionId, ParticipantRef, ProviderSessionKey, SqliteAttachmentStore};
use crate::agents::{copilot_service as native, service as participants};
use crate::db::{AgentxDb, InstanceRow};
use crate::messaging::inbox::{DeliveryEvidence, DeliveryReceipt, EventId};
use crate::messaging::service::{self as messages, InboxOperation, InboxOutcome, OutputConfirmation, PreparedOutput};
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestContext, RequestId, ServiceError, ServiceResult};
use serde_json::json;

struct Fixture {
    db: AgentxDb,
    context: RequestContext,
    participant: ParticipantRef,
    target: Option<ActivationTarget>,
    _root: tempfile::TempDir,
}

impl Fixture {
    fn new(managed: bool) -> Self {
        let root = tempfile::Builder::new().prefix("agentx-copilot-native-").tempdir().unwrap();
        let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
        for (name, tool, created, session) in [("lobo", "copilot", 10.0, "copilot-main"), ("nava", "codex", 11.0, "sender-session")] {
            db.save_instance_named(
                name,
                json!({
                    "tool":tool,"created_at":created,"session_id":session,
                    "status":"listening","last_event_id":0,"directory":root.path()
                })
                .as_object()
                .unwrap(),
            )
            .unwrap();
            db.set_session_binding(session, name).unwrap();
        }
        db.set_process_binding("copilot-process", "copilot-main", "lobo").unwrap();
        let pid = std::process::id();
        let mut context = RequestContext {
            caller: CallerContext {
                request_id: RequestId("fixture-unused".into()),
                invocation: InvocationContext { cwd: root.path().to_path_buf(), explicit_name: Some("lobo".into()), process_binding_id: Some("copilot-process".into()), ..Default::default() },
                deadline_unix_ms: None,
                expected_database_generation: None,
            },
            process: CallerProcess { pid, incarnation: crate::sys::process::identity(pid) },
            daemon_instance: "copilot-native-fixture".into(),
            database_generation: db.database_generation().unwrap(),
        };
        let participant = SqliteAttachmentStore::new(&db).participant("lobo").unwrap();
        let target = managed.then(|| {
            let mut store = SqliteAttachmentStore::for_owner_process(&db, AttachmentOwnerProcess { pid, process_identity: context.process.incarnation.clone() });
            let attachment = store
                .attach(AttachmentRequest {
                    execution: ExecutionId("copilot-fixture-execution".into()),
                    owner: AttachmentOwnerId("copilot-fixture-owner".into()),
                    session: ProviderSessionKey { provider: "copilot".into(), scope: root.path().display().to_string(), session_id: "copilot-main".into() },
                    intent: AttachmentIntent::Create { provisional: participant.clone() },
                })
                .unwrap();
            context.caller.invocation.activation_target = Some(attachment.target.clone());
            context.caller.invocation.provider_session = Some(attachment.session);
            attachment.target
        });
        Self { db, context, participant, target, _root: root }
    }

    fn context(&self, id: &str) -> RequestContext {
        let mut context = self.context.clone();
        context.caller.request_id = RequestId(id.into());
        context
    }

    fn call_at(&self, context: &RequestContext, request: native::Request) -> ServiceResult<native::Response> {
        match participants::handle(&self.db, context, participants::Request::Copilot(request))? {
            participants::Response::Copilot(response) => Ok(response),
            _ => panic!("native Copilot response required"),
        }
    }

    fn call(&self, id: &str, request: native::Request) -> ServiceResult<native::Response> {
        self.call_at(&self.context(id), request)
    }

    fn poll_request(&self, session: &str) -> native::Request {
        native::Request::Poll { participant: self.participant.clone(), session_id: session.into(), idle: true }
    }

    fn poll(&self, id: &str) -> native::PollResult {
        let native::Response::Poll(result) = self.call(id, self.poll_request("copilot-main")).unwrap() else {
            panic!("native poll response required");
        };
        result
    }

    fn pending_request(&self, session: &str, hook: native::PendingHook) -> native::Request {
        native::Request::Pending { participant: self.participant.clone(), session_id: Some(session.into()), hook }
    }

    fn pending(&self, id: &str) -> PreparedOutput {
        let native::Response::Pending(output) = self.call(id, self.pending_request("copilot-main", native::PendingHook::ToolResult)).unwrap() else {
            panic!("native pending output required");
        };
        output
    }

    fn message(&self, id: &str, text: &str) -> EventId {
        let mut context = self.context(id);
        context.caller.invocation = InvocationContext { cwd: context.caller.invocation.cwd.clone(), explicit_name: Some("nava".into()), ..Default::default() };
        let request = messages::Request::Submit(messages::Submission { recipients: vec!["lobo".into()], text: text.into(), envelope: Default::default(), external_sender: None, broadcast_confirmed: true, inline_bundle: None });
        let messages::Response::Accepted(result) = messages::handle(&self.db, &context, request).unwrap() else {
            panic!("native message acceptance required");
        };
        result.event
    }

    fn prompt_request(&self, session: &str, prompt: &str) -> native::Request {
        native::Request::ObservePrompt { participant: self.participant.clone(), session_id: Some(session.into()), prompt: prompt.into() }
    }

    fn row(&self) -> InstanceRow {
        self.db.get_instance_full("lobo").unwrap().unwrap()
    }
}

fn json_value(value: impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

fn confirmation(output: PreparedOutput) -> OutputConfirmation {
    match output {
        PreparedOutput::Unmanaged { receipt, .. } => OutputConfirmation::Unmanaged(receipt),
        PreparedOutput::Claimed { target, batch, .. } => OutputConfirmation::Claimed(DeliveryReceipt { target, batch: batch.id, through: batch.window.through, evidence: DeliveryEvidence::HookOutputWritten }),
        PreparedOutput::Deferred => panic!("a deferred output cannot be confirmed"),
    }
}

#[test]
fn lost_offer_reply_recovers_exact_text_but_changed_intent_or_peer_conflicts() {
    for managed in [false, true] {
        let fixture = Fixture::new(managed);
        fixture.message("first-message", "one frozen Copilot body");
        let first = fixture.poll("lost-poll").offer.unwrap();
        assert!(first.text.contains("one frozen Copilot body"));
        assert_eq!(json_value(fixture.poll("lost-poll").offer.unwrap()), json_value(&first));
        let changed = native::Request::Poll { participant: fixture.participant.clone(), session_id: "copilot-main".into(), idle: false };
        assert!(matches!(fixture.call("lost-poll", changed), Err(ServiceError::Conflict(_))));
        let mut peer = fixture.context("lost-poll");
        peer.process.incarnation = Some("another-authenticated-peer".into());
        assert!(matches!(fixture.call_at(&peer, fixture.poll_request("copilot-main")), Err(ServiceError::Conflict(_))));
        fixture.call("accepted", native::Request::Sent { receipt: first.receipt.clone(), observation: native::SendObservation::Accepted }).unwrap();
        assert!(fixture.poll("lost-poll").offer.is_none(), "accepted enqueue replay exposed old text");
        assert_eq!(fixture.poll("another-poll").inflight, Some(first.receipt.through));
        assert_eq!(fixture.row().last_event_id, 0, "enqueue is not a delivery receipt");
    }
}

#[test]
fn sdk_observations_and_lookalike_prompts_never_confirm_the_frozen_frontier() {
    for managed in [false, true] {
        let fixture = Fixture::new(managed);
        let first_event = fixture.message("first-message", "exact frozen body");
        let offer = fixture.poll("first-poll").offer.unwrap();
        let later = fixture.message("later-message", "later body must remain pending");
        for (id, observation) in [("sdk-unknown", native::SendObservation::Unknown), ("sdk-enqueue", native::SendObservation::Accepted)] {
            fixture.call(id, native::Request::Sent { receipt: offer.receipt.clone(), observation }).unwrap();
            assert_eq!(fixture.row().last_event_id, 0);
            assert!(matches!(fixture.pending(&format!("{id}-hook")), PreparedOutput::Deferred));
        }
        let lookalike = fixture.prompt_request("copilot-main", &format!("{} plus unrelated text", offer.text));
        assert!(matches!(fixture.call("lookalike", lookalike).unwrap(), native::Response::PromptObserved { matched: false, ignored: false }));
        assert!(matches!(fixture.call("wrong-session", fixture.prompt_request("historical-session", &offer.text)).unwrap(), native::Response::PromptObserved { matched: false, ignored: true }));
        assert_eq!(fixture.row().last_event_id, 0);
        let input = fixture.prompt_request("copilot-main", &offer.text);
        assert!(matches!(fixture.call("exact-prompt", input.clone()).unwrap(), native::Response::PromptObserved { matched: true, ignored: false }));
        assert_eq!(fixture.row().last_event_id, first_event.0);
        assert_eq!(fixture.row().status, "active");
        fixture.db.update_instance_fields("lobo", json!({"status":"blocked","status_context":"newer-approval"}).as_object().unwrap()).unwrap();
        let watermark = fixture.db.get_last_event_id();
        fixture.call("exact-prompt", input.clone()).unwrap();
        fixture.call("duplicate-exact-prompt", input).unwrap();
        assert_eq!(fixture.row().status, "blocked");
        assert_eq!(fixture.row().status_context, "newer-approval");
        assert_eq!(fixture.row().last_event_id, first_event.0);
        assert_eq!(fixture.db.get_last_event_id(), watermark);
        let next = fixture.poll("next-batch").offer.unwrap();
        assert_eq!(next.receipt.through, later);
        assert!(next.text.contains("later body must remain pending"));
        assert!(!next.text.contains("exact frozen body"));
    }
}

#[test]
fn ordinary_hook_and_extension_share_one_owner_and_exact_flush_receipt() {
    for managed in [false, true] {
        let fixture = Fixture::new(managed);
        let event = fixture.message("first-message", "hook-owned body");
        let output = fixture.pending("lost-hook-result");
        assert_eq!(json_value(fixture.pending("lost-hook-result")), json_value(&output));
        assert!(fixture.poll("extension-race").offer.is_none());
        assert_eq!(fixture.poll("extension-race").inflight, Some(event));
        if managed {
            assert!(matches!(fixture.pending("other-hook"), PreparedOutput::Deferred));
        }
        assert_eq!(fixture.row().last_event_id, 0);
        let result = messages::handle(&fixture.db, &fixture.context("actual-flush"), messages::Request::Inbox(InboxOperation::ConfirmOutputAndStatus { receipt: confirmation(output), announce: false })).unwrap();
        assert!(matches!(result, messages::Response::Inbox(InboxOutcome::Done(Ok(())))));
        assert_eq!(fixture.row().last_event_id, event.0);
        assert!(matches!(fixture.pending("lost-hook-result"), PreparedOutput::Deferred));
        assert!(fixture.poll("after-flush").offer.is_none());
    }
}

#[test]
fn expired_or_failed_sdk_offer_cannot_transfer_a_managed_issuing_claim() {
    for managed in [false, true] {
        for queued in [false, true] {
            let fixture = Fixture::new(managed);
            fixture.message("first-message", "lease-bound body");
            let first = fixture.poll("first-poll").offer.unwrap();
            if queued {
                fixture.call("queued", native::Request::Sent { receipt: first.receipt.clone(), observation: native::SendObservation::Accepted }).unwrap();
            }
            // Age persisted facts without sleeping or changing the product clock.
            let key = format!("copilot_delivery:lobo:{}", fixture.participant.incarnation_bits);
            let mut value: serde_json::Value = serde_json::from_str(&fixture.db.kv_get(&key).unwrap().unwrap()).unwrap();
            value["Extension"]["offered_at"] = json!(0);
            if queued {
                value["Extension"]["queued_at"] = json!(0);
            }
            fixture.db.kv_set(&key, Some(&value.to_string())).unwrap();
            let after_expiry = fixture.poll("after-expiry");
            if managed {
                assert!(after_expiry.offer.is_none());
                assert_eq!(after_expiry.inflight, Some(first.receipt.through));
                fixture.call("sdk-failed", native::Request::Sent { receipt: first.receipt.clone(), observation: native::SendObservation::Failed }).unwrap();
                assert!(fixture.poll("after-failure").offer.is_none());
                assert!(matches!(fixture.pending("ordinary-hook"), PreparedOutput::Deferred));
                assert!(fixture.target.is_some());
            } else {
                let replacement = after_expiry.offer.unwrap();
                assert_ne!(replacement.receipt.id, first.receipt.id);
                assert_eq!(replacement.receipt.through, first.receipt.through);
                fixture.call("sdk-failed", native::Request::Sent { receipt: replacement.receipt, observation: native::SendObservation::Failed }).unwrap();
                assert!(fixture.poll("after-failure").offer.is_some());
            }
            assert_eq!(fixture.row().last_event_id, 0);
        }
    }
}

#[test]
fn required_owner_and_retention_writes_roll_back_claims_before_same_id_retry() {
    for managed in [false, true] {
        for prefix in ["copilot_delivery:%", "participant_request:%"] {
            let fixture = Fixture::new(managed);
            fixture.message("first-message", "atomic offer body");
            let watermark = fixture.db.get_last_event_id();
            fixture.db.conn().execute_batch(&format!("CREATE TRIGGER copilot_offer_fault BEFORE INSERT ON kv WHEN NEW.key LIKE '{prefix}' BEGIN SELECT RAISE(ABORT,'fixture offer fault'); END;")).unwrap();
            assert!(fixture.call("atomic-poll", fixture.poll_request("copilot-main")).is_err());
            assert_eq!(fixture.row().last_event_id, 0);
            assert_eq!(fixture.db.get_last_event_id(), watermark);
            fixture.db.conn().execute_batch("DROP TRIGGER copilot_offer_fault").unwrap();
            assert!(fixture.poll("atomic-poll").offer.is_some(), "failed acceptance stranded an output claim");
        }
    }
}

#[test]
fn failed_prompt_owner_save_restores_cursor_status_and_can_retry_exact_evidence() {
    for managed in [false, true] {
        let fixture = Fixture::new(managed);
        let event = fixture.message("first-message", "confirmation rollback body");
        let offer = fixture.poll("first-poll").offer.unwrap();
        let input = fixture.prompt_request("copilot-main", &offer.text);
        let watermark = fixture.db.get_last_event_id();
        fixture.db.conn().execute_batch("CREATE TRIGGER copilot_confirm_fault BEFORE INSERT ON kv WHEN NEW.key LIKE 'copilot_delivery:%' BEGIN SELECT RAISE(ABORT,'fixture confirmation fault'); END;").unwrap();
        assert!(fixture.call("exact-prompt", input.clone()).is_err());
        assert_eq!(fixture.row().last_event_id, 0);
        assert_eq!(fixture.row().status, "listening");
        assert_eq!(fixture.db.get_last_event_id(), watermark);
        fixture.db.conn().execute_batch("DROP TRIGGER copilot_confirm_fault").unwrap();
        fixture.call("exact-prompt", input).unwrap();
        assert_eq!(fixture.row().last_event_id, event.0);
        assert_eq!(fixture.row().status, "active");
    }
}

#[test]
fn historical_session_cannot_claim_root_mail_or_replace_endpoint() {
    for managed in [false, true] {
        let fixture = Fixture::new(managed);
        fixture.message("first-message", "root mailbox body");
        for (id, hook) in [("historical-tool", native::PendingHook::ToolResult), ("historical-stop", native::PendingHook::Stop)] {
            assert!(matches!(fixture.call(id, fixture.pending_request("historical-session", hook)).unwrap(), native::Response::Pending(PreparedOutput::Deferred)));
        }
        assert_eq!(fixture.row().last_event_id, 0);
        assert!(fixture.poll("real-root").offer.is_some());
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let endpoint =
            |session: &str, port| crate::runtime::hosts_store::handle(&fixture.db, &fixture.context("endpoint"), crate::runtime::hosts::Request::SetCopilotEndpoint { participant: fixture.participant.clone(), session_id: session.into(), port });
        endpoint("copilot-main", port).unwrap();
        assert!(matches!(endpoint("historical-session", port), Err(ServiceError::StaleFence)));
        assert!(endpoint("copilot-main", 0).is_err());
        assert_eq!(fixture.db.get_notify_endpoint_port("lobo", "plugin"), Some(port));
        let mut missing_peer = fixture.context("missing-peer");
        missing_peer.process.incarnation = None;
        assert!(matches!(fixture.call_at(&missing_peer, fixture.poll_request("copilot-main")), Err(ServiceError::Forbidden(_))));
        let mut missing_binding = fixture.context("missing-binding");
        missing_binding.caller.invocation.process_binding_id = None;
        assert!(fixture.call_at(&missing_binding, fixture.poll_request("copilot-main")).is_err());
        fixture.db.update_instance_fields("lobo", json!({"created_at":99.0}).as_object().unwrap()).unwrap();
        assert!(matches!(fixture.call("successor", fixture.poll_request("copilot-main")), Err(ServiceError::StaleFence)));
        assert_eq!(fixture.row().last_event_id, 0);
    }
}

#[test]
fn lazy_first_session_is_pinned_to_offer_and_wrong_receipt_never_mutates_it() {
    let fixture = Fixture::new(false);
    fixture.db.update_instance_fields("lobo", json!({"session_id":null}).as_object().unwrap()).unwrap();
    fixture.message("first-message", "lazy session body");
    let offer = fixture.poll("lazy-offer").offer.unwrap();
    let native::Response::Poll(other) = fixture.call("other-extension", fixture.poll_request("other-session")).unwrap() else {
        panic!("poll result required");
    };
    assert!(other.session_mismatch && other.offer.is_none());
    let bad_endpoint =
        crate::runtime::hosts_store::handle(&fixture.db, &fixture.context("other-endpoint"), crate::runtime::hosts::Request::SetCopilotEndpoint { participant: fixture.participant.clone(), session_id: "other-session".into(), port: 12345 });
    assert!(matches!(bad_endpoint, Err(ServiceError::StaleFence)));
    for changed in ["id", "through", "session"] {
        let mut token = offer.receipt.clone();
        match changed {
            "id" => token.id.push_str("-forged"),
            "through" => token.through.0 += 1,
            _ => token.session_id = "other-session".into(),
        }
        assert!(matches!(fixture.call(changed, native::Request::Sent { receipt: token, observation: native::SendObservation::Accepted }), Err(ServiceError::StaleFence)));
    }
    assert_eq!(fixture.row().last_event_id, 0);
    assert_eq!(fixture.poll("valid-owner").inflight, Some(offer.receipt.through));
}
