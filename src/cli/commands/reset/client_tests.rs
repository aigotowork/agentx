//! Actual reset host cases: native IPC owns storage and generation, while the
//! originating host preserves file scope and never retries a failed transport.
use super::*;
use crate::agents::service::{Request as Participants, Response as ParticipantResult};
use crate::service::{InvocationContext, client::Client};

fn fixture(fail_clear: bool) -> (tempfile::TempDir, crate::daemon::test_support::TestDaemon, Session) {
    let root = tempfile::tempdir().unwrap();
    let database = root.path().join("agentx.db");
    let db = AgentxDb::open_at(&database).unwrap();
    db.save_instance_named("luna", serde_json::json!({"tool":"adhoc","created_at":1.0,"status":"inactive"}).as_object().unwrap()).unwrap();
    db.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES('2026-11-08T00:00:00Z','message','luna','{\"content\":\"before reset\"}')", []).unwrap();
    if fail_clear {
        db.conn().execute_batch("CREATE TRIGGER refuse_reset BEFORE DELETE ON events WHEN OLD.type='message' BEGIN SELECT RAISE(ABORT,'reset clear rejected'); END;").unwrap();
    }
    drop(db);
    let daemon = crate::daemon::test_support::TestDaemon::start(root.path(), &database);
    let invocation = InvocationContext { explicit_name: Some("luna".into()), cwd: root.path().into(), ..Default::default() };
    let client = Client::connect_existing(root.path(), invocation.clone()).unwrap();
    let Response::Participants(ParticipantResult::Actor(actor)) = client.call(Request::Participants(Participants::Resolve)).unwrap() else { panic!("actor response") };
    let session = Session { client, invocation, actor: Some(actor), command: crate::shared::CommandContext { explicit_name: Some("luna".into()), identity: None, go: true, identity_warning: None } };
    (root, daemon, session)
}

#[test]
fn native_reset_host_archives_rotates_generation_and_leaves_original_client_stale() {
    let (root, _daemon, session) = fixture(false);
    let old_generation = session.client.generation().unwrap().to_owned();
    assert_eq!(cmd_reset_client(&session, &ResetArgs { target: None }), 0);
    assert!(matches!(summary(&session.client), Err(ServiceError::StaleGeneration)));
    assert!(root.path().join("agentx.db").exists());
    let next = Client::connect_existing(root.path(), session.invocation.clone()).unwrap();
    let current = summary(&next).unwrap();
    assert_ne!(current.database_generation, old_generation);
    assert!(current.local_participants.is_empty());
    let Response::Maintenance(M::Response::ArchivedEvents(events)) = next.call(Request::Maintenance(M::Request::ArchivedEvents { selector: "1".into(), here: false, predicate: Some("type='message'".into()), limit: 10 })).unwrap() else {
        panic!("archive response")
    };
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["data"]["content"], "before reset");
    // The retired actor never causes a post-reset mailbox read through the
    // stale generation when this result returns to the actual router.
    assert!(!session.deliver_pending(false).unwrap());
}

#[test]
fn native_reset_all_preview_preserves_participants_generation_and_host_files() {
    let (root, _daemon, mut session) = fixture(false);
    let config = root.path().join("config.toml");
    std::fs::write(&config, "timeout = 123\n").unwrap();
    session.invocation.environment.insert("AGENTX_LAUNCHED".into(), "1".into());
    session.client = session.client.with_invocation(session.invocation.clone());
    session.command.go = false;
    let before = summary(&session.client).unwrap();
    assert_eq!(cmd_reset_client(&session, &ResetArgs { target: Some(ResetTarget::All) }), 0);
    let after = summary(&session.client).unwrap();
    assert_eq!(after.local_participants, before.local_participants);
    assert_eq!(after.database_generation, before.database_generation);
    assert_eq!(after.event_count, before.event_count);
    assert_eq!(std::fs::read_to_string(config).unwrap(), "timeout = 123\n");
    assert!(!root.path().join("archive").exists());
}

#[test]
fn native_reset_transport_failure_does_not_recreate_storage_or_clear_full_reset_files() {
    let (root, daemon, session) = fixture(false);
    let config = root.path().join("config.toml");
    std::fs::write(&config, "timeout = 123\n").unwrap();
    let artifact = root.path().join("integrations").join("fixture");
    std::fs::create_dir_all(artifact.parent().unwrap()).unwrap();
    std::fs::write(&artifact, "retain").unwrap();
    drop(daemon);
    std::fs::remove_file(root.path().join("agentx.db")).unwrap();
    assert_eq!(cmd_reset_client(&session, &ResetArgs { target: Some(ResetTarget::All) }), 1);
    assert!(!root.path().join("agentx.db").exists());
    assert!(artifact.exists());
    assert!(config.exists());
}

#[test]
fn native_reset_clear_failure_preserves_generation_events_and_full_reset_files() {
    let (root, _daemon, session) = fixture(true);
    let config = root.path().join("config.toml");
    std::fs::write(&config, "timeout = 123\n").unwrap();
    let before = summary(&session.client).unwrap();
    assert_eq!(cmd_reset_client(&session, &ResetArgs { target: Some(ResetTarget::All) }), 1);
    let after = summary(&session.client).unwrap();
    assert_eq!(before.database_generation, after.database_generation);
    assert!(after.event_count >= before.event_count);
    assert_eq!(std::fs::read_to_string(config).unwrap(), "timeout = 123\n");
}

#[test]
fn reset_hooks_preview_is_host_only_with_no_database() {
    let root = tempfile::tempdir().unwrap();
    let invocation = InvocationContext { cwd: root.path().into(), environment: std::collections::HashMap::from([("AGENTX_DIR".into(), root.path().to_string_lossy().into()), ("AGENTX_LAUNCHED".into(), "1".into())]), ..Default::default() };
    assert_eq!(hooks_for_invocation(&invocation, false), 0);
    assert!(!root.path().join("agentx.db").exists());
    assert!(!root.path().join(".tmp").exists());
}

#[test]
fn full_reset_file_cleanup_uses_only_the_selected_scope() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("first");
    let second = root.path().join("second");
    for scope in [&first, &second] {
        for name in [".tmp/launched_pids.json", ".tmp/device_id", ".tmp/flags/instance_count", "integrations/fixture"] {
            let file = scope.join(name);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, "fixture").unwrap();
        }
    }
    operations::clear_full_reset_artifacts_for_scope(&first).unwrap();
    for name in [".tmp/launched_pids.json", ".tmp/device_id", ".tmp/flags/instance_count", "integrations/fixture"] {
        assert!(!first.join(name).exists());
        assert!(second.join(name).exists());
    }
}
