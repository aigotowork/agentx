//! Real disposable IPC tests for direct calls and per-operation compatibility.
use super::{DaemonError, DaemonInfo, DaemonScope, frame, platform, test_support::TestDaemon, transport, wire::*};
use crate::service::{CallerContext, InvocationContext, Request, RequestId, ServiceError, client::Client};
use serde_json::json;
use std::os::unix::net::UnixListener;
use std::time::{Duration, Instant};

fn caller(generation: Option<&str>) -> CallerContext {
    CallerContext { request_id: RequestId("direct-fixture".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: generation.map(str::to_owned) }
}

#[test]
fn direct_server_classifies_operations_and_preserves_scope_instance_and_generation() {
    let directory = tempfile::tempdir().unwrap();
    let _daemon = TestDaemon::start(directory.path(), &directory.path().join("agentx.db"));
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let selected = transport::probe(&scope).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let request = |path, instance, generation, request| {
        let mut stream = platform::connect(&scope, deadline).unwrap();
        frame::write(&mut stream, &ClientFrame::Call { canonical_scope: path, daemon_instance: instance, caller: caller(generation), request }, deadline).unwrap();
        frame::read::<ServerFrame>(&mut stream, deadline).unwrap()
    };
    let root = scope.root().to_path_buf();
    let reply = request(root.clone(), None, None, json!({"Events":"Watermark"}));
    assert!(
        matches!(reply, ServerFrame::Result { result: Ok(_), info, executed_generation, request_id } if info.database_generation == selected.database_generation && executed_generation == selected.database_generation && request_id == caller(None).request_id)
    );
    for (input, operation) in
        [(json!({"FutureDomain":{"payload":1}}), "FutureDomain"), (json!({"Management":{"FutureOperation":{"limit":"invalid"}}}), "Management.FutureOperation"), (json!({"Messages":{"Delivery":{"FuturePoll":null}}}), "Messages.Delivery.FuturePoll")]
    {
        assert!(matches!(request(root.clone(), None, None, input), ServerFrame::Failure(DaemonError::Unsupported(name)) if name == operation));
    }
    assert!(matches!(request(root.clone(), None, Some(&selected.database_generation), json!({"Events":{"Query":false}})), ServerFrame::Failure(DaemonError::InvalidInput(_))));
    assert!(matches!(request(root.clone(), None, None, json!({"Maintenance":"ResetConversation"})), ServerFrame::Failure(DaemonError::InvalidInput(_))));
    assert!(matches!(request(root.clone(), Some("replaced".into()), None, json!({"Events":"Watermark"})), ServerFrame::Failure(DaemonError::Incompatible(_))));
    assert!(matches!(request(root, None, Some("replaced"), json!({"Events":"Watermark"})), ServerFrame::Failure(DaemonError::StaleGeneration)));
    assert!(matches!(request(directory.path().join("neighbor"), None, None, json!({"Events":"Watermark"})), ServerFrame::Failure(DaemonError::Incompatible(_))));
}

fn listen_once(listener: &UnixListener, deadline: Instant) -> std::os::unix::net::UnixStream {
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(true).unwrap();
                return stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline => std::thread::park_timeout(Duration::from_millis(5)),
            Err(error) => panic!("accept direct call: {error}"),
        }
    }
}

fn fake_info(scope: &DaemonScope) -> DaemonInfo {
    DaemonInfo {
        protocol_version: 99,
        build_version: "future-package".into(),
        build_id: "future-build".into(),
        canonical_scope: scope.root().into(),
        daemon_instance: "instance".into(),
        database_generation: "generation".into(),
        pid: std::process::id(),
        codex_hook: false,
        agent_names: false,
        workspace_members: false,
        message_history: false,
    }
}

#[test]
fn lazy_client_uses_one_business_exchange_and_retains_returned_generation() {
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let listener = UnixListener::bind(scope.socket_path().unwrap()).unwrap();
    listener.set_nonblocking(true).unwrap();
    let info = fake_info(&scope);
    let client = Client::connect_existing(scope.root(), InvocationContext::default()).unwrap();
    assert!(matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock));
    std::thread::scope(|threads| {
        threads.spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut stream = listen_once(&listener, deadline);
            let ClientFrame::Call { canonical_scope, daemon_instance, caller, request } = frame::read::<ClientFrame>(&mut stream, deadline).unwrap();
            assert_eq!(canonical_scope, info.canonical_scope);
            assert_eq!(daemon_instance, None);
            assert_eq!(caller.expected_database_generation, None);
            assert!(matches!(request, Request::Events(crate::events::service::Request::Watermark)));
            frame::write(&mut stream, &ServerFrame::Failure(DaemonError::Unsupported("Events.Watermark".into())), deadline).unwrap();
            drop(stream);
            let mut stream = listen_once(&listener, deadline);
            let ClientFrame::Call { caller, request, .. } = frame::read::<ClientFrame>(&mut stream, deadline).unwrap();
            assert!(caller.expected_database_generation.is_none());
            assert!(matches!(request, Request::Queries(crate::service::queries::Request::AgentNames(_))));
            let reply = ServerFrame::Result {
                result: Ok(crate::service::Response::Queries(crate::service::queries::Response::AgentNames(vec!["agent".into()]))),
                info: info.clone(),
                executed_generation: info.database_generation.clone(),
                request_id: caller.request_id,
            };
            frame::write(&mut stream, &reply, deadline).unwrap();
            let mut stream = listen_once(&listener, deadline);
            let ClientFrame::Call { caller, request, .. } = frame::read::<ClientFrame>(&mut stream, deadline).unwrap();
            assert_eq!(caller.expected_database_generation.as_deref(), Some("generation"));
            assert!(matches!(request, Request::Events(crate::events::service::Request::Watermark)));
            frame::write(&mut stream, &ServerFrame::Failure(DaemonError::StaleGeneration), deadline).unwrap();
        });
        let watermark = || Request::Events(crate::events::service::Request::Watermark);
        assert!(matches!(client.call(watermark()), Err(ServiceError::Unsupported(_))));
        assert!(client.call(Request::Queries(crate::service::queries::Request::AgentNames(Default::default()))).is_ok());
        assert_eq!(client.generation().unwrap(), "generation");
        assert_eq!(client.info().unwrap().protocol_version, 99);
        assert_eq!(client.call(watermark()).unwrap_err(), ServiceError::StaleGeneration);
    });
}

#[test]
fn result_identity_mismatch_preserves_the_write_request_id() {
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let listener = UnixListener::bind(scope.socket_path().unwrap()).unwrap();
    listener.set_nonblocking(true).unwrap();
    std::thread::scope(|threads| {
        let info = fake_info(&scope);
        threads.spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut stream = listen_once(&listener, deadline);
            let _: ClientFrame = frame::read(&mut stream, deadline).unwrap();
            frame::write(&mut stream, &ServerFrame::Result { result: Err(ServiceError::InvalidInput("test reply".into())), info, executed_generation: "generation".into(), request_id: RequestId("another-call".into()) }, deadline).unwrap();
        });
        let response = transport::call_bound(&scope, false, caller(Some("generation")), Request::Events(crate::events::service::Request::Watermark));
        assert!(matches!(response, Err(DaemonError::OutcomeUnknown(detail)) if detail.contains("direct-fixture")));
    });
}
