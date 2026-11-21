//! Independent root-reservation regression cases. These call the real root
//! preparation boundary with fresh SQLite state, before hook/native effects.

use super::*;
use crate::agents::attachments::SqliteAttachmentStore;
use crate::integrations::codex::service::{ConnectionFailure, NativeConnection, Receive, ServiceCompatibility};
use crate::messaging::inbox::SqlitePendingInbox;
use serde_json::json;
use std::time::Duration;

struct UnusedTransport;

impl NativeConnection for UnusedTransport {
    fn send(&mut self, _: &str) -> Result<(), ConnectionFailure> {
        panic!("reservation must precede native effects")
    }

    fn receive(&mut self, _: Duration) -> Result<Receive, ConnectionFailure> {
        panic!("reservation must not read native transport")
    }

    fn close(&mut self) -> Result<(), ConnectionFailure> {
        Ok(())
    }
}

fn with_empty_bridge(body: impl FnOnce(&mut NativeBridge<'_>, &crate::db::AgentxDb)) {
    let (_dir, agentx_dir, home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
    let db = crate::db::AgentxDb::open_at(&agentx_dir.join("agentx.db")).unwrap();
    for id in 1..=7 {
        db.conn().execute("INSERT INTO events(id,timestamp,type,instance,data) VALUES(?1,'2026-11-07T15:00:00','fixture','fixture','{}')", [id]).unwrap();
    }
    let native_home = home.join(".codex");
    std::fs::create_dir_all(&native_home).unwrap();
    let identity = ServiceIdentity {
        canonical_home: native_home.clone(),
        execution: crate::agents::attachments::ExecutionId("backend-a".into()),
        native_version: "0.167.0".into(),
        control_socket: native_home.join("fixture.sock"),
        compatibility: ServiceCompatibility { native_program: home.join("native-codex"), environment_fingerprint: "fixture-env".into(), configuration_fingerprint: "fixture-config".into(), agentx_dir: agentx_dir.clone() },
    };
    let env = HashMap::from_iter([("HOME".into(), home.to_string_lossy().into_owned()), ("AGENTX_DIR".into(), agentx_dir.to_string_lossy().into_owned())]);
    let args = vec!["app-server".to_owned()];
    let configuration = SessionConfiguration::new(&args, env, &home, &identity, false).unwrap();
    let launch = RootLaunch { explicit_name: None, initial_cursor: Some(7), launch_args: args, launch_profile: None };
    let mut attachments = SqliteAttachmentStore::new(&db);
    let mut inbox = SqlitePendingInbox::new(&db);
    let mut transport = UnusedTransport;
    let hosts = super::super::acceptance_tests::FixtureHosts::new(&db);
    let wake = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let ports = BridgePorts { attachments: &mut attachments, inbox: &mut inbox, transport: &mut transport, hosts: &hosts };
    let mut bridge = NativeBridge::new(configuration, launch, identity, ports, wake.local_addr().unwrap().port());
    body(&mut bridge, &db);
}

#[test]
#[serial_test::serial]
fn acceptance_a4_fresh_create_fork_and_unregistered_resume_promote_only_their_reserved_placeholder() {
    for method in ["thread/start", "thread/fork", "thread/resume"] {
        with_empty_bridge(|bridge, db| {
            let request = json!({"id":"fresh-root","method":method,"params":{"threadId":"native-source"}});
            let frame = NativeFrame::parse(&request.to_string()).unwrap();
            let root = bridge.prepare_root(method, &frame).unwrap().unwrap();
            let participant = &root.participant;
            let row = db.get_instance_full(&participant.canonical_name).unwrap().unwrap();
            assert_eq!(row.created_at.to_bits(), participant.incarnation_bits);
            assert_eq!(row.tool, "codex");
            assert_eq!(row.last_event_id, 7);
            assert_eq!(db.conn().query_row::<i64, _, _>("SELECT count(*) FROM instances", [], |r| r.get(0)).unwrap(), 1);
            assert_eq!(db.conn().query_row::<i64, _, _>("SELECT count(*) FROM participant_attachments", [], |r| r.get(0)).unwrap(), 0, "reservation alone qualified an attachment");
            match root.intent {
                RootIntent::Create { provisional } => assert_eq!(provisional, *participant),
                RootIntent::Fork { parent, provisional } => {
                    assert_eq!(provisional, *participant);
                    assert_eq!(parent.thread_id, "native-source");
                }
                RootIntent::Resume { provisional, .. } => assert_eq!(provisional.as_ref(), Some(participant)),
            }
        });
    }
}

#[test]
#[serial_test::serial]
fn acceptance_a4_explicit_live_name_collision_preserves_original_participant() {
    with_empty_bridge(|bridge, db| {
        db.save_instance_named("lobo", json!({"created_at":10.0,"tool":"claude","last_event_id":3,"status":"active"}).as_object().unwrap()).unwrap();
        bridge.launch.explicit_name = Some("lobo".into());
        let frame = NativeFrame::parse(r#"{"id":"collision","method":"thread/start","params":{}}"#).unwrap();
        assert!(bridge.prepare_root("thread/start", &frame).is_err());
        let row = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(row.created_at, 10.0);
        assert_eq!(row.tool, "claude");
        assert_eq!(row.last_event_id, 3);
        assert_eq!(db.conn().query_row::<i64, _, _>("SELECT count(*) FROM instances", [], |r| r.get(0)).unwrap(), 1);
    });
}
