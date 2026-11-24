//! Actual native IPC host cases, authored for the integration-owned run.
use super::*;
use crate::agents::service::{Request as Participants, Response as ParticipantResult};
use crate::db::AgentxDb;
use crate::service::InvocationContext;
use clap::Parser;

fn fixture() -> (tempfile::TempDir, crate::daemon::test_support::TestDaemon, Session) {
    let root = tempfile::tempdir().unwrap();
    let database = root.path().join("agentx.db");
    let db = AgentxDb::open_at(&database).unwrap();
    db.save_instance_named("luna", json!({"tool":"adhoc","created_at":1.0,"status":"inactive","last_event_id":0}).as_object().unwrap()).unwrap();
    drop(db);
    let daemon = crate::daemon::test_support::TestDaemon::start(root.path(), &database);
    let invocation = InvocationContext { explicit_name: Some("luna".into()), cwd: root.path().into(), ..Default::default() };
    let client = Client::connect_existing(root.path(), invocation.clone()).unwrap();
    let Response::Participants(ParticipantResult::Actor(actor)) = client.call(Request::Participants(Participants::Resolve)).unwrap() else { panic!("participant") };
    let session = Session { client, invocation, actor: Some(actor), command: crate::shared::CommandContext { explicit_name: Some("luna".into()), identity: None, go: true, identity_warning: None } };
    (root, daemon, session)
}
fn args() -> EventsArgs {
    EventsArgs::parse_from(["events"])
}
fn query() -> EventQuery {
    EventQuery { predicate: Some("type='status' AND json_extract(data,'$.context')='observer-trigger'".into()), ..Default::default() }
}
fn trigger(session: &Session) {
    session.client.call(Request::Participants(Participants::Status { participant: session.actor.as_ref().unwrap().participant.clone(), status: "active".into(), context: "observer-trigger".into(), detail: String::new() })).unwrap();
}
fn endpoint_count(root: &std::path::Path) -> i64 {
    let db = AgentxDb::open_at(&root.join("agentx.db")).unwrap();
    db.conn().query_row("SELECT COUNT(*) FROM notify_endpoints WHERE kind='events_wait' AND instance LIKE 'event-observer:%'", [], |row| row.get(0)).unwrap()
}

fn with_watchdog<T>(shutdown: &AtomicBool, body: impl FnOnce() -> T) -> T {
    std::thread::scope(|scope| {
        let (done, receive) = std::sync::mpsc::channel();
        scope.spawn(move || {
            if receive.recv_timeout(Duration::from_secs(10)).is_err() {
                shutdown.store(true, Ordering::Release);
            }
        });
        let result = body();
        let _ = done.send(());
        result
    })
}

#[test]
fn host_observer_wakes_through_real_ipc_before_the_five_second_fallback() {
    let (root, _daemon, session) = fixture();
    let mut observer = Observer::begin(&session.client, Some(&session.actor.as_ref().unwrap().participant)).unwrap();
    assert_eq!(endpoint_count(root.path()), 1);
    let start = Instant::now();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(100));
            trigger(&session);
        });
        observer.pause(Duration::from_secs(60), &AtomicBool::new(false), &|| false);
    });
    assert!(start.elapsed() < Duration::from_secs(3), "notification was lost; only the five-second fallback returned");
    observer.finish().unwrap();
    assert_eq!(endpoint_count(root.path()), 0);
}

#[test]
fn maximum_wait_timeout_can_match_and_clean_its_endpoint_without_overflow() {
    let (root, _daemon, session) = fixture();
    let mut output = Vec::new();
    let shutdown = AtomicBool::new(false);
    with_watchdog(&shutdown, || {
        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(100));
                trigger(&session);
            });
            assert_eq!(wait_to(&session, &args(), query(), u64::MAX, &mut output, &shutdown, &|| false).unwrap(), 0);
        })
    });
    let event: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(event["data"]["context"], "observer-trigger");
    assert_eq!(endpoint_count(root.path()), 0);
    assert_eq!(remaining(u64::MAX, Duration::from_nanos(1)), Duration::from_secs(u64::MAX) - Duration::from_nanos(1));
    assert!(remaining(1, Duration::from_secs(2)).is_zero());
}

#[test]
fn wait_timeout_interrupt_invalid_predicate_and_closed_reader_all_clean_endpoints() {
    let (root, _daemon, session) = fixture();
    let mut output = Vec::new();
    assert_eq!(wait_to(&session, &args(), query(), 0, &mut output, &AtomicBool::new(false), &|| false).unwrap(), 1);
    assert_eq!(serde_json::from_slice::<Value>(&output).unwrap(), json!({"timed_out":true}));
    output.clear();
    assert_eq!(wait_to(&session, &args(), query(), u64::MAX, &mut output, &AtomicBool::new(true), &|| false).unwrap(), 130);
    assert!(output.is_empty());
    let invalid = EventQuery { predicate: Some("EXISTS(SELECT 1 FROM kv)".into()), ..Default::default() };
    assert!(matches!(wait_to(&session, &args(), invalid, 60, &mut output, &AtomicBool::new(false), &|| false), Err(ServiceError::InvalidInput(_))));
    assert_eq!(wait_to(&session, &args(), query(), 60, &mut output, &AtomicBool::new(false), &|| true).unwrap(), 0);
    assert_eq!(endpoint_count(root.path()), 0);
}

struct Closed;
impl Write for Closed {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
}

#[test]
fn matched_wait_and_follow_output_failure_release_observers_without_mailbox_ack() {
    let (root, _daemon, session) = fixture();
    trigger(&session);
    assert_eq!(wait_to(&session, &args(), query(), 60, &mut Closed, &AtomicBool::new(false), &|| false).unwrap(), 0);
    let mut follow_args = args();
    follow_args.follow = true;
    follow_args.after_id = Some(0);
    assert_eq!(follow_to(&session, &follow_args, query(), &mut Closed, &AtomicBool::new(false), &|| false).unwrap(), 0);
    assert_eq!(endpoint_count(root.path()), 0);
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    assert_eq!(db.get_instance_full("luna").unwrap().unwrap().last_event_id, 0);
}

#[test]
fn real_cli_recent_continues_archives_past_one_native_page_and_releases_snapshot() {
    let (root, _daemon, session) = fixture();
    let archive = root.path().join("archive/session-many");
    std::fs::create_dir_all(&archive).unwrap();
    let archived = AgentxDb::open_at(&archive.join("agentx.db")).unwrap();
    let live = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    for (name, db) in [("archived", &archived), ("live", &live)] {
        db.with_write_scope(|| {
            for _ in 0..MAX_PAGE_EVENTS + 7 {
                db.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES('2026-11-07T00:00:00Z','message',?1,'{}')", [name])?;
            }
            Ok(())
        })
        .unwrap();
    }
    let (events, _) = recent(&session.client, EventQuery { include_archives: true, ..Default::default() }, 2 * (MAX_PAGE_EVENTS + 7)).unwrap();
    assert_eq!(events.len(), 2 * (MAX_PAGE_EVENTS + 7));
    assert!(events.windows(2).all(|pair| {
        let a = (&pair[0]["ts"], &pair[0]["source"], pair[0]["id"].as_i64().unwrap());
        let b = (&pair[1]["ts"], &pair[1]["source"], pair[1]["id"].as_i64().unwrap());
        (a.0.as_str(), a.1.as_str(), a.2) < (b.0.as_str(), b.1.as_str(), b.2)
    }));
    assert!(live.kv_prefix("event_history:").unwrap().is_empty());
}

#[test]
fn observer_and_wait_ipc_failure_never_reacquire_or_recreate_managed_storage() {
    let (root, daemon, session) = fixture();
    let mut observer = Observer::begin(&session.client, Some(&session.actor.as_ref().unwrap().participant)).unwrap();
    drop(daemon);
    std::fs::remove_file(root.path().join("agentx.db")).unwrap();
    assert!(matches!(observer.check(), Err(ServiceError::Unavailable(_))));
    assert!(matches!(observer.finish(), Err(ServiceError::Unavailable(_))));
    assert!(matches!(wait_to(&session, &args(), query(), 60, &mut Vec::new(), &AtomicBool::new(false), &|| false), Err(ServiceError::Unavailable(_))));
    assert!(!root.path().join("agentx.db").exists());
}

struct ResetOutput<'a> {
    bytes: Vec<u8>,
    shutdown: &'a AtomicBool,
}
impl Write for ResetOutput<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if String::from_utf8_lossy(&self.bytes).contains("db_reset") {
            self.shutdown.store(true, Ordering::Release);
        }
        Ok(())
    }
}

#[test]
fn follower_reset_rebinds_generation_and_never_registers_a_namesake_participant() {
    let (root, _daemon, session) = fixture();
    let shutdown = AtomicBool::new(false);
    let mut output = ResetOutput { bytes: Vec::new(), shutdown: &shutdown };
    with_watchdog(&shutdown, || {
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let start = Instant::now();
                while endpoint_count(root.path()) == 0 {
                    assert!(start.elapsed() < Duration::from_secs(5));
                    std::thread::sleep(Duration::from_millis(10));
                }
                session.client.call(Request::Maintenance(crate::service::maintenance::Request::ResetConversation)).unwrap();
            });
            let mut args = args();
            args.follow = true;
            assert_eq!(follow_to(&session, &args, query(), &mut output, &shutdown, &|| false).unwrap(), 130);
        })
    });
    assert!(String::from_utf8(output.bytes).unwrap().contains("\"db_reset\":true"));
    assert_eq!(endpoint_count(root.path()), 0);
}

#[cfg(unix)]
#[test]
fn reminder_create_reconciles_a_committed_call_with_a_lost_reply() {
    use crate::daemon::{DaemonScope, frame};
    use crate::service::CallerContext;
    use std::os::unix::net::UnixListener;

    let (root, _daemon, session) = fixture();
    let proxy_scope = DaemonScope::prepare(root.path().join("drop-reply")).unwrap();
    let proxy = UnixListener::bind(proxy_scope.socket_path().unwrap()).unwrap();
    proxy.set_nonblocking(true).unwrap();
    let target = session.actor.as_ref().unwrap().participant.clone();
    let due = chrono::Utc::now().timestamp_millis() + 30_000;

    std::thread::scope(|threads| {
        let actual = session.client.clone();
        let mut ready = actual.info().clone();
        ready.canonical_scope = proxy_scope.root().into();
        threads.spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(6);
            let mut first_request_id = None;
            for connection in 0..3 {
                let mut stream = loop {
                    match proxy.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline => std::thread::park_timeout(Duration::from_millis(5)),
                        Err(error) => panic!("proxy accept failed: {error}"),
                    }
                };
                stream.set_nonblocking(true).unwrap();
                let hello: Value = frame::read(&mut stream, deadline).unwrap();
                assert_eq!(hello["Hello"]["protocol_version"], 4);
                frame::write(&mut stream, &json!({"Ready": ready}), deadline).unwrap();
                if connection == 0 {
                    assert!(frame::read_optional::<Value>(&mut stream, deadline).unwrap().is_none());
                    continue;
                }
                let wire: Value = frame::read(&mut stream, deadline).unwrap();
                let caller: CallerContext = serde_json::from_value(wire["Call"]["caller"].clone()).unwrap();
                let request: Request = serde_json::from_value(wire["Call"]["request"].clone()).unwrap();
                if let Some(id) = &first_request_id {
                    assert_eq!(id, &caller.request_id);
                } else {
                    first_request_id = Some(caller.request_id.clone());
                }
                let response = actual.call_with_context(caller, request).unwrap();
                let Response::Events(EventResult::ReminderCreated(receipt)) = &response else {
                    panic!("expected real creation result");
                };
                assert_eq!(receipt.already_exists, connection == 2);
                if connection == 2 {
                    frame::write(&mut stream, &json!({"Result":{"result": {"Ok": response}, "database_generation": ready.database_generation}}), deadline).unwrap();
                }
            }
        });

        let client = Client::connect_existing(proxy_scope.root(), session.invocation.clone()).unwrap();
        let EventResult::ReminderCreated(receipt) = reconciled_call(&client, Events::CreateReminder { target, due_at_ms: due, message: "lost reply".into() }).unwrap() else {
            panic!("expected reconciled creation result");
        };
        assert!(receipt.already_exists);
        let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM reminders", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
    });
}
