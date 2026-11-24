//! Real IPC compatibility cases; all scopes are disposable and no Provider runs.
use super::{DaemonError, DaemonInfo, DaemonScope, frame, platform, test_support::TestDaemon, transport, wire::*};
use crate::service::{CallerContext, InvocationContext, Request, RequestId};
use std::time::{Duration, Instant};

#[test]
fn compatible_build_server_accepts_native_calls_and_preserves_fences() {
    let directory = tempfile::tempdir().unwrap();
    let _daemon = TestDaemon::start(directory.path(), &directory.path().join("agentx.db"));
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let original = transport::probe(&scope).unwrap();
    assert_eq!(original.protocol_version, 4);
    for (protocol, path, accepted) in
        [(PROTOCOL_VERSION, scope.root().to_path_buf(), true), (3, scope.root().to_path_buf(), false), (PROTOCOL_VERSION + 1, scope.root().to_path_buf(), false), (PROTOCOL_VERSION, directory.path().join("wrong-scope"), false)]
    {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut stream = platform::connect(&scope, deadline).unwrap();
        frame::write(&mut stream, &ClientFrame::Hello { protocol_version: protocol, build_version: "different-package-version".into(), build_id: "different-build".into(), canonical_scope: path }, deadline).unwrap();
        let reply: ServerFrame = frame::read(&mut stream, deadline).unwrap();
        if !accepted {
            assert!(matches!(reply, ServerFrame::Failure(DaemonError::Incompatible(_))));
            continue;
        }
        let ServerFrame::Ready(info) = reply else { panic!("compatible build rejected") };
        assert_eq!(info.daemon_instance, original.daemon_instance);
        let caller = CallerContext { request_id: RequestId("compatible-build".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some(info.database_generation.clone()) };
        frame::write(&mut stream, &ClientFrame::Call { daemon_instance: info.daemon_instance, caller, request: Request::Events(crate::events::service::Request::Watermark) }, deadline).unwrap();
        assert!(matches!(frame::read::<ServerFrame>(&mut stream, deadline).unwrap(), ServerFrame::Result { result: Ok(_), .. }));
    }
    assert_eq!(transport::probe(&scope).unwrap().daemon_instance, original.daemon_instance);
}

#[test]
fn compatible_build_client_accepts_ready_but_rejects_invalid_identity() {
    use std::os::unix::net::UnixStream;
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let baseline = DaemonInfo {
        protocol_version: PROTOCOL_VERSION,
        build_version: "another-version".into(),
        build_id: "another-build".into(),
        canonical_scope: scope.root().to_path_buf(),
        daemon_instance: "instance".into(),
        database_generation: "generation".into(),
        pid: std::process::id(),
        codex_hook: false,
        agent_names: false,
        workspace_members: false,
        message_history: false,
    };
    for case in 0..7 {
        let mut info = baseline.clone();
        match case {
            1 => info.protocol_version = 3,
            2 => info.protocol_version += 1,
            3 => info.canonical_scope = directory.path().join("wrong"),
            4 => info.pid += 1,
            5 => info.daemon_instance.clear(),
            6 => info.database_generation.clear(),
            _ => {}
        }
        let (mut client, mut server) = UnixStream::pair().unwrap();
        client.set_nonblocking(true).unwrap();
        server.set_nonblocking(true).unwrap();
        std::thread::scope(|threads| {
            let deadline = Instant::now() + Duration::from_secs(3);
            threads.spawn(move || {
                assert!(matches!(frame::read::<ClientFrame>(&mut server, deadline).unwrap(), ClientFrame::Hello { protocol_version: 4, .. }));
                frame::write(&mut server, &ServerFrame::Ready(info), deadline).unwrap();
            });
            let result = transport::hello(&scope, &mut client, deadline);
            if case == 0 {
                assert_eq!(result.unwrap().build_id, "another-build");
            } else if case == 1 {
                assert!(matches!(result, Err(DaemonError::Incompatible(_))), "epoch 4 must reject epoch 3 readiness: {result:?}");
            } else {
                assert!(matches!(result, Err(DaemonError::Incompatible(_) | DaemonError::Protocol(_))));
            }
        });
    }
}

#[test]
fn agent_names_capability_is_checked_on_the_current_call_connection() {
    use crate::service::{CallerContext, InvocationContext, RequestId, queries};
    use std::os::unix::net::UnixListener;

    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let listener = UnixListener::bind(scope.socket_path().unwrap()).unwrap();
    for agent_names in [false, true] {
        let info = DaemonInfo {
            protocol_version: PROTOCOL_VERSION,
            build_version: "version".into(),
            build_id: "build".into(),
            canonical_scope: scope.root().to_path_buf(),
            daemon_instance: "instance".into(),
            database_generation: "generation".into(),
            pid: std::process::id(),
            codex_hook: false,
            agent_names,
            workspace_members: false,
            message_history: false,
        };
        std::thread::scope(|threads| {
            let listener = &listener;
            threads.spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_nonblocking(true).unwrap();
                let deadline = Instant::now() + Duration::from_secs(3);
                assert!(matches!(frame::read::<ClientFrame>(&mut stream, deadline).unwrap(), ClientFrame::Hello { .. }));
                frame::write(&mut stream, &ServerFrame::Ready(info), deadline).unwrap();
                let call = frame::read_optional::<ClientFrame>(&mut stream, deadline).unwrap();
                if agent_names {
                    assert!(matches!(call, Some(ClientFrame::Call { request: Request::Queries(queries::Request::AgentNames(_)), .. })));
                    frame::write(&mut stream, &ServerFrame::Result { result: Ok(crate::service::Response::Queries(queries::Response::AgentNames(vec!["lobo".into()]))), database_generation: "generation".into() }, deadline).unwrap();
                } else {
                    assert!(call.is_none(), "old daemon must not receive an unknown operation");
                }
            });
            let caller = CallerContext { request_id: RequestId("names".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some("generation".into()) };
            let result = transport::call(&scope, caller, Request::Queries(queries::Request::AgentNames(Default::default())));
            if agent_names {
                assert!(matches!(result, Ok(Ok(crate::service::Response::Queries(queries::Response::AgentNames(names)))) if names == ["lobo"]));
            } else {
                assert!(matches!(result, Err(DaemonError::Unsupported(_))));
            }
        });
    }
}

#[test]
fn native_completeness_old_daemon_does_not_receive_the_workspace_members_call() {
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let listener = std::os::unix::net::UnixListener::bind(scope.socket_path().unwrap()).unwrap();
    let info = DaemonInfo {
        protocol_version: PROTOCOL_VERSION,
        build_version: "previous-version".into(),
        build_id: "previous-build".into(),
        canonical_scope: scope.root().to_path_buf(),
        daemon_instance: "instance".into(),
        database_generation: "generation".into(),
        pid: std::process::id(),
        codex_hook: true,
        agent_names: true,
        workspace_members: false,
        message_history: false,
    };
    std::thread::scope(|threads| {
        threads.spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            assert!(matches!(frame::read::<ClientFrame>(&mut stream, deadline).unwrap(), ClientFrame::Hello { .. }));
            frame::write(&mut stream, &ServerFrame::Ready(info), deadline).unwrap();
            assert!(frame::read_optional::<ClientFrame>(&mut stream, deadline).unwrap().is_none());
        });
        let caller = CallerContext { request_id: RequestId("workspace-members".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some("generation".into()) };
        let request = Request::Management(crate::management::contract::Request::ListWorkspaceMembers { workspace: "global".into(), current_only: false, after: None, limit: 1 });
        assert!(matches!(transport::call(&scope, caller, request), Err(DaemonError::Unsupported(_))));
    });
}

#[test]
fn message_history_is_never_sent_to_an_old_daemon() {
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let listener = std::os::unix::net::UnixListener::bind(scope.socket_path().unwrap()).unwrap();
    let info = DaemonInfo {
        protocol_version: PROTOCOL_VERSION,
        build_version: "previous-version".into(),
        build_id: "previous-build".into(),
        canonical_scope: scope.root().to_path_buf(),
        daemon_instance: "instance".into(),
        database_generation: "generation".into(),
        pid: std::process::id(),
        codex_hook: true,
        agent_names: true,
        workspace_members: true,
        message_history: false,
    };
    std::thread::scope(|threads| {
        threads.spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            assert!(matches!(frame::read::<ClientFrame>(&mut stream, deadline).unwrap(), ClientFrame::Hello { .. }));
            frame::write(&mut stream, &ServerFrame::Ready(info), deadline).unwrap();
            assert!(frame::read_optional::<ClientFrame>(&mut stream, deadline).unwrap().is_none());
        });
        let caller = CallerContext { request_id: RequestId("message-history".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some("generation".into()) };
        let agent = crate::management::contract::AgentRef { name: "target".into(), incarnation: Some("42".into()), stopped_event: None };
        let request = Request::Management(crate::management::contract::Request::MessageHistory { agent, before: None, limit: 20 });
        assert!(matches!(transport::call(&scope, caller, request), Err(DaemonError::Unsupported(_))));
    });
}

#[test]
fn codex_hook_reuses_ready_connection_and_checks_capability_before_call() {
    use crate::service::codex_hook::{Activity, HookEvent, HookOutcome};
    use std::os::unix::net::UnixListener;
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let socket = scope.socket_path().unwrap();
    let listener = UnixListener::bind(&socket).unwrap();
    let baseline = DaemonInfo {
        protocol_version: PROTOCOL_VERSION,
        build_version: "version".into(),
        build_id: "build".into(),
        canonical_scope: scope.root().to_path_buf(),
        daemon_instance: "instance".into(),
        database_generation: "generation".into(),
        pid: std::process::id(),
        codex_hook: false,
        agent_names: false,
        workspace_members: false,
        message_history: false,
    };
    for (supports_hook, claims_current) in [(false, true), (true, false), (true, true)] {
        let mut info = baseline.clone();
        info.codex_hook = supports_hook;
        std::thread::scope(|threads| {
            let listener = &listener;
            threads.spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_nonblocking(true).unwrap();
                let deadline = Instant::now() + Duration::from_secs(3);
                assert!(matches!(frame::read::<ClientFrame>(&mut stream, deadline).unwrap(), ClientFrame::Hello { .. }));
                frame::write(&mut stream, &ServerFrame::Ready(info), deadline).unwrap();
                let first = frame::read_optional::<ClientFrame>(&mut stream, deadline).unwrap();
                if supports_hook && claims_current {
                    let Some(ClientFrame::Call { caller, request: Request::CodexHook(_), .. }) = first else { panic!("first socket did not carry the call") };
                    assert_eq!(caller.expected_database_generation.as_deref(), Some("generation"));
                    frame::write(&mut stream, &ServerFrame::Result { result: Ok(crate::service::Response::CodexHook(HookOutcome::NoOutput)), database_generation: "generation".into() }, deadline).unwrap();
                } else {
                    assert!(first.is_none());
                }
            });
            let caller = CallerContext { request_id: RequestId("codex-hook".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some(if claims_current { "generation" } else { "stale" }.into()) };
            let event = HookEvent { session_id: None, transcript_path: None, directory: scope.root().into(), activity: Activity::PostTool { tool_name: "Bash".into() } };
            let result = transport::call_codex_hook(&scope, caller, event);
            if !supports_hook {
                assert!(matches!(result, Ok(None)));
            } else if !claims_current {
                assert!(matches!(result, Err(DaemonError::StaleGeneration)));
            } else {
                assert!(matches!(result, Ok(Some((_, Ok(crate::service::Response::CodexHook(HookOutcome::NoOutput)))))));
            }
        });
    }
    std::fs::remove_file(socket).unwrap();
}
