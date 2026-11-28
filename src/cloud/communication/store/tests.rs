use super::*;

const GEN: &str = "scope-generation";

fn apply<T>(root: &Path, action: impl FnOnce(&Connection, i64) -> Result<T>) -> Result<T> {
    transaction(root, GEN, true, || true, action)
}

fn host(host: &str) -> HostRegistration {
    HostRegistration { generation: GEN.into(), installation_id: format!("{host}-installation") }
}

fn agent(host: &str, incarnation: &str) -> AgentRegistration {
    AgentRegistration { generation: GEN.into(), installation_id: format!("{host}-installation"), host: host.into(), incarnation: incarnation.into() }
}

fn send_input(sender: &AgentView, target: &str, request: &str) -> SendMessage {
    SendMessage {
        generation: GEN.into(),
        installation_id: format!("{}-installation", sender.host),
        request_id: request.into(),
        from: sender.clone(),
        to: target.into(),
        body: "A durable note".into(),
        intent: Intent::Request,
        reply_to: None,
        thread: Some("coordination".into()),
    }
}

fn claim_input(agent: &AgentView, id: &str) -> ClaimInbox {
    ClaimInbox { generation: GEN.into(), installation_id: format!("{}-installation", agent.host), agent: agent.clone(), claim_id: id.into(), limit: None, wait_ms: None }
}

fn confirm_input(agent: &AgentView, claim_id: &str, message_ids: &[&str]) -> ConfirmInbox {
    ConfirmInbox { generation: GEN.into(), installation_id: format!("{}-installation", agent.host), agent: agent.clone(), claim_id: claim_id.into(), message_ids: message_ids.iter().map(|id| (*id).to_owned()).collect() }
}

fn recover_input(agent: &AgentView, claim_id: &str, release: bool) -> RecoverInbox {
    RecoverInbox { generation: GEN.into(), installation_id: format!("{}-installation", agent.host), agent: agent.clone(), claim_id: claim_id.into(), release }
}

fn scenario() -> (tempfile::TempDir, AgentView, AgentView) {
    let root = tempfile::tempdir().unwrap();
    initialize(root.path(), GEN).unwrap();
    let (luna, nova) = apply(root.path(), |db, now| {
        renew_host(db, now, "laptop".into(), host("laptop"))?;
        renew_host(db, now, "desktop".into(), host("desktop"))?;
        Ok((register_agent(db, now, "luna".into(), agent("laptop", "luna-one"))?, register_agent(db, now, "nova".into(), agent("desktop", "nova-one"))?))
    })
    .unwrap();
    (root, luna, nova)
}

#[test]
fn natural_reply_replays_or_defers_to_the_exact_explicit_answer() {
    let (root, luna, nova) = scenario();
    let root = root.path();
    let ask = |id: &str| apply(root, |db, now| send_human_message(db, now, SendHumanMessage { generation: GEN.into(), request_id: id.into(), to: luna.clone(), body: "Answer me".into() })).unwrap();
    let send_reply = |actor: &AgentView, request_id: &str, reference: &str, body: &str| {
        let mut reply = send_input(actor, "@~space/operator", request_id);
        reply.intent = Intent::Ack;
        reply.reply_to = Some(reference.into());
        reply.body = body.into();
        apply(root, |db, now| send(db, now, reply))
    };

    let first = ask("first");
    let explicit = send_reply(&luna, "reply-tool", &first.id, "Explicit result").unwrap();
    assert_eq!(send_reply(&luna, "space-final:first", &first.id, "Generated result").unwrap(), explicit);
    assert!(matches!(send_reply(&nova, "space-final:wrong-agent", &first.id, "Wrong sender"), Err(Error::NotFound(_))));

    let second = ask("second");
    let natural = send_reply(&luna, "space-final:second", &second.id, "Generated result").unwrap();
    assert_eq!(send_reply(&luna, "space-final:second", &second.id, "Generated result").unwrap(), natural);
    assert!(matches!(send_reply(&luna, "space-final:second", &second.id, "Changed replay"), Err(Error::Conflict(_))));
    let history = apply(root, |db, _| list_messages(db, MessageHistoryQuery { generation: GEN.into(), host: luna.host.clone(), name: luna.name.clone(), incarnation: luna.incarnation.clone(), after: None, before: None, limit: None })).unwrap();
    assert_eq!(history.items, [natural, second, explicit, first]);
}

#[test]
fn offline_target_keeps_its_name_and_mailbox_then_replies_from_its_exact_host() {
    let (root, luna, nova) = scenario();
    let root = root.path();
    apply(root, |db, now| {
        db.execute("UPDATE hosts SET last_seen=?1 WHERE name='desktop'", [now - LEASE_SECONDS])?;
        assert!(matches!(renew_host(db, now, "desktop".into(), host("intruder")), Err(Error::Installation)));
        assert!(matches!(register_agent(db, now, "nova".into(), agent("laptop", "nova-two")), Err(Error::Conflict(_))));
        assert!(matches!(send(db, now, send_input(&luna, "@laptop/nova", "wrong-host")), Err(Error::NotFound(_))));
        Ok(())
    })
    .unwrap();
    assert_eq!(super::super::summary(root, GEN).unwrap(), (2, 1, 2));
    let page = super::super::host_page(root, GEN, None, 1).unwrap();
    assert_eq!(page.items.iter().map(|h| (h.name.as_str(), h.online)).collect::<Vec<_>>(), [("desktop", false)]);
    assert_eq!(page.next.as_deref(), Some("desktop"));
    let page = super::super::host_page(root, GEN, page.next, 1).unwrap();
    assert_eq!(page.items.iter().map(|h| (h.name.as_str(), h.online)).collect::<Vec<_>>(), [("laptop", true)]);
    assert!(page.next.is_none());
    let page = super::super::agent_page(root, GEN, None, 1, Some("desktop".into())).unwrap();
    assert_eq!(page.items.as_slice(), std::slice::from_ref(&nova));
    assert!(page.next.is_none());

    let message = apply(root, |db, now| send(db, now, send_input(&luna, "@desktop/nova", "send-1"))).unwrap();
    let replay = apply(root, |db, now| send(db, now, send_input(&luna, "@desktop/nova", "send-1"))).unwrap();
    assert_eq!(replay, message);
    assert!(matches!(apply(root, |db, now| send(db, now, send_input(&luna, "@nova", "send-1"))), Err(Error::Conflict(_))));
    assert_eq!(message.to, nova);
    assert!(matches!(apply(root, |db, now| claim(db, now, &claim_input(&nova, "claim-1"))), Err(Error::Offline)));
    apply(root, |db, now| renew_host(db, now, "desktop".into(), host("desktop"))).unwrap();

    let (received, open) = apply(root, |db, now| claim(db, now, &claim_input(&nova, "claim-1"))).unwrap();
    assert!(!open);
    assert_eq!(received.items.as_slice(), std::slice::from_ref(&message));
    assert_eq!(apply(root, |db, _| operation(db, "send-1".into())).unwrap().status, OperationStatus::Claimed);
    apply(root, |db, now| confirm(db, now, confirm_input(&nova, "claim-1", &[&message.id]))).unwrap();
    assert_eq!(apply(root, |db, _| operation(db, "send-1".into())).unwrap().status, OperationStatus::Confirmed);

    let mut reply = send_input(&nova, "@laptop/luna", "reply-1");
    reply.intent = Intent::Ack;
    reply.reply_to = Some(message.id);
    let reply = apply(root, |db, now| send(db, now, reply)).unwrap();
    let (inbox, _) = apply(root, |db, now| claim(db, now, &claim_input(&luna, "claim-2"))).unwrap();
    assert_eq!(inbox.items, [reply]);
}

#[test]
fn partial_confirmation_recovers_without_redelivery_and_release_is_explicit() {
    let (root, luna, nova) = scenario();
    let root = root.path();
    let first = apply(root, |db, now| send(db, now, send_input(&luna, "@nova", "first"))).unwrap();
    let second = apply(root, |db, now| send(db, now, send_input(&luna, "@nova", "second"))).unwrap();
    let (claimed, _) = apply(root, |db, now| claim(db, now, &claim_input(&nova, "claim"))).unwrap();
    assert_eq!(claimed.items, [first.clone(), second.clone()]);
    assert!(matches!(apply(root, |db, now| recover(db, now, recover_input(&luna, "claim", false))), Err(Error::Conflict(_))));
    assert!(matches!(apply(root, |db, now| claim(db, now, &claim_input(&nova, "fresh"))), Err(Error::ClaimOutstanding)));

    let invalid = apply(root, |db, now| confirm(db, now, confirm_input(&nova, "claim", &[&first.id, "999"])));
    assert!(matches!(invalid, Err(Error::Conflict(_))));
    let recovered = apply(root, |db, now| recover(db, now, recover_input(&nova, "claim", false))).unwrap();
    assert_eq!(recovered.items, [first.clone(), second.clone()]);

    for _ in 0..2 {
        let confirmed = apply(root, |db, now| confirm(db, now, confirm_input(&nova, "claim", &[&first.id]))).unwrap();
        assert_eq!(confirmed.confirmed.as_slice(), std::slice::from_ref(&first.id));
    }
    let remaining = apply(root, |db, now| recover(db, now, recover_input(&nova, "claim", false))).unwrap();
    assert_eq!(remaining.items.as_slice(), std::slice::from_ref(&second));
    apply(root, |db, now| recover(db, now, recover_input(&nova, "claim", true))).unwrap();
    let (after_release, _) = apply(root, |db, now| claim(db, now, &claim_input(&nova, "fresh"))).unwrap();
    assert_eq!(after_release.items, [second]);
    let (old, reusable) = apply(root, |db, now| claim(db, now, &claim_input(&nova, "claim"))).unwrap();
    assert!(old.items.is_empty());
    assert!(!reusable);
}

#[test]
fn retirement_fences_claims_and_replayed_send_never_changes_targets() {
    let (root, luna, nova) = scenario();
    let root = root.path();
    let sent = apply(root, |db, now| send(db, now, send_input(&luna, "@nova", "frozen"))).unwrap();
    apply(root, |db, now| claim(db, now, &claim_input(&nova, "old-claim"))).unwrap();
    apply(root, |db, now| retire_agent(db, now, "nova".into(), agent("desktop", "nova-one"))).unwrap();
    assert!(matches!(apply(root, |db, now| register_agent(db, now, "nova".into(), agent("desktop", "nova-one"))), Err(Error::Conflict(_))));
    let successor = apply(root, |db, now| register_agent(db, now, "nova".into(), agent("desktop", "nova-two"))).unwrap();

    apply(root, |db, now| retire_agent(db, now, "nova".into(), agent("desktop", "nova-one"))).unwrap();
    assert_eq!(super::super::agent_page(root, GEN, None, 50, None).unwrap().items, [luna.clone(), successor.clone()]);
    let (batch, reusable) = apply(root, |db, now| claim(db, now, &claim_input(&successor, "new-claim"))).unwrap();
    assert!(batch.items.is_empty());
    assert!(reusable);
    assert!(matches!(apply(root, |db, now| recover(db, now, recover_input(&nova, "old-claim", false))), Err(Error::Agent)));
    assert_eq!(apply(root, |db, now| send(db, now, send_input(&luna, "@nova", "frozen"))).unwrap(), sent);
    assert_eq!(apply(root, |db, _| operation(db, "frozen".into())).unwrap().status, OperationStatus::Undeliverable);
}

#[test]
fn storage_generation_and_mid_transaction_credential_are_fenced() {
    let root = tempfile::tempdir().unwrap();
    initialize(root.path(), GEN).unwrap();
    assert!(initialize(root.path(), "stale").is_err());
    assert!(matches!(transaction(root.path(), "stale", true, || true, |_, _| Ok(())), Err(Error::Generation)));

    let valid = std::cell::Cell::new(true);
    let change = transaction(
        root.path(),
        GEN,
        true,
        || valid.get(),
        |db, now| {
            renew_host(db, now, "revoked".into(), host("revoked"))?;
            valid.set(false);
            Ok(())
        },
    );
    assert!(matches!(change, Err(Error::Credential)));
    assert_eq!(super::super::summary(root.path(), GEN).unwrap(), (0, 0, 0));
}

#[test]
fn batch_body_budget_leaves_later_messages_available() {
    let (root, luna, nova) = scenario();
    let root = root.path();
    let mut inserted = Vec::new();
    for index in 0..=MAX_BATCH_BODY_BYTES / MAX_BODY_BYTES {
        let mut input = send_input(&luna, "@nova", &format!("budget-{index}"));
        input.body = "x".repeat(MAX_BODY_BYTES);
        inserted.push(apply(root, |db, now| send(db, now, input)).unwrap().id);
    }
    let (claim_batch, _) = apply(root, |db, now| claim(db, now, &claim_input(&nova, "first"))).unwrap();
    let claimed: Vec<_> = claim_batch.items.iter().map(|message| message.id.as_str()).collect();
    assert_eq!(claimed.as_slice(), &inserted[..inserted.len() - 1]);
    apply(root, |db, now| confirm(db, now, confirm_input(&nova, "first", &claimed))).unwrap();
    let (remaining, _) = apply(root, |db, now| claim(db, now, &claim_input(&nova, "second"))).unwrap();
    assert_eq!(remaining.items.len(), 1);
    assert_eq!(&remaining.items[0].id, inserted.last().unwrap());
}

#[test]
fn terminal_control_characters_do_not_create_a_send_receipt_or_mail() {
    let (root, luna, nova) = scenario();
    let root = root.path();
    let request_id = "native-text-safety";
    for body in ["before\x1b[2Jafter", "before\u{009B}2Jafter"] {
        let mut input = send_input(&luna, "@nova", request_id);
        input.body = body.into();
        let rejected = apply(root, |db, now| send(db, now, input));
        assert!(matches!(rejected, Err(Error::InvalidMessage(_))));
    }
    let mut oversized = send_input(&luna, "@nova", request_id);
    oversized.body = "x".repeat(MAX_BODY_BYTES + 1);
    assert!(matches!(apply(root, |db, now| send(db, now, oversized)), Err(Error::Invalid(_))));
    assert!(matches!(apply(root, |db, _| operation(db, request_id.into())), Err(Error::NotFound(_))));
    let (empty, _) = apply(root, |db, now| claim(db, now, &claim_input(&nova, "safe-claim"))).unwrap();
    assert!(empty.items.is_empty());

    let mut input = send_input(&luna, "@nova", request_id);
    input.body = "first line\n\t雪".into();
    let accepted = apply(root, |db, now| send(db, now, input)).unwrap();
    let (batch, _) = apply(root, |db, now| claim(db, now, &claim_input(&nova, "safe-claim"))).unwrap();
    assert_eq!(batch.items, [accepted]);
}
