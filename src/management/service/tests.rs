//! Native IPC composition, with finite journal grants instead of a Provider/worker.
//! Broader real HTTP/subprocess acceptance is owned by the pilot acceptance Task.
use super::*;
use crate::{daemon::test_support::TestDaemon, db::AgentxDb};

#[cfg(unix)]
fn diagnostic_worker(scope: &std::path::Path) -> WorkerInput {
    WorkerInput {
        scope: scope.to_string_lossy().into_owned(),
        generation: "verification-generation".into(),
        request_id: "verification-operation".into(),
        ticket: "private-ticket-not-for-logs".into(),
        invocation: InvocationContext { cwd: scope.to_path_buf(), environment: [("API_TOKEN".into(), "credential-fixture-sensitive".into())].into(), ..Default::default() },
    }
}

#[cfg(unix)]
fn diagnostic_script(scope: &std::path::Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let script = scope.join("disposable-worker");
    std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    script
}

#[cfg(unix)]
#[test]
fn runtime_read_worker_missing_cwd_has_stage_executable_and_directory() {
    let scope = tempfile::tempdir().unwrap();
    let mut input = diagnostic_worker(scope.path());
    input.invocation.cwd = scope.path().join("missing-directory");
    let executable = diagnostic_script(scope.path(), "/bin/cat >/dev/null\nexit 0");
    let error = supervision::supervise(&input, &executable, Duration::from_secs(2)).unwrap_err();
    assert!(error.contains("spawn"), "{error}");
    assert!(error.contains(&executable.to_string_lossy().to_string()), "{error}");
    assert!(error.contains(&input.invocation.cwd.to_string_lossy().to_string()), "{error}");
    assert!(!error.contains("credential-fixture-sensitive"));
    assert!(!error.contains(&input.ticket));
}

#[cfg(unix)]
#[test]
fn runtime_read_worker_drains_large_stderr_and_retains_bounded_private_diagnostic() {
    let scope = tempfile::tempdir().unwrap();
    let input = diagnostic_worker(scope.path());
    let executable = diagnostic_script(
        scope.path(),
        "/bin/cat >/dev/null\nprintf 'discarded-prefix\\n' >&2\ni=0\nwhile [ \"$i\" -lt 2048 ]; do printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\\n' >&2; i=$((i+1)); done\nprintf 'retained-final-marker %s\\n' \"$API_TOKEN\" >&2\nexit 7",
    );
    let started = std::time::Instant::now();
    let error = supervision::supervise(&input, &executable, Duration::from_secs(3)).unwrap_err();
    assert!(started.elapsed() < Duration::from_secs(4));
    assert!(error.contains("exited"), "{error}");
    assert!(!error.contains("retained-final-marker"));
    let logs = crate::log::get_recent_logs_in_scope(scope.path(), 1.0, &["ERROR"], 20);
    let diagnostic = logs.iter().find(|row| row["event"] == "worker.stderr").expect("private child diagnostic retained");
    assert_eq!(diagnostic["generation"], input.generation);
    assert_eq!(diagnostic["operation_id"], input.request_id);
    let message = diagnostic["msg"].as_str().unwrap();
    assert!(message.len() <= 16 * 1024);
    assert!(message.contains("retained-final-marker"), "last diagnostic bytes were lost");
    assert!(!message.contains("discarded-prefix"));
    let persisted = std::fs::read_to_string(scope.path().join(".tmp/logs/agentx.log")).unwrap();
    assert!(!persisted.contains("credential-fixture-sensitive"));
    assert!(!persisted.contains(&input.ticket));
    assert!(persisted.contains("[redacted]"));
}

#[cfg(target_os = "linux")]
#[test]
fn runtime_read_worker_inherited_stderr_does_not_extend_supervision_deadline() {
    let scope = tempfile::tempdir().unwrap();
    let input = diagnostic_worker(scope.path());
    // The finite orphan keeps inherited pipe handles open after its parent exits.
    let executable = diagnostic_script(scope.path(), "/bin/cat >/dev/null\nexec /usr/bin/python3 -c 'import os,select; pid=os.fork(); select.select([],[],[],1.5) if pid==0 else None; os._exit(0)'");
    let started = std::time::Instant::now();
    supervision::supervise(&input, &executable, Duration::from_millis(500)).unwrap();
    assert!(started.elapsed() < Duration::from_secs(1), "descendant pipe ownership blocked supervisor completion");
}

#[cfg(target_os = "linux")]
#[test]
fn runtime_read_worker_timeout_releases_writer_with_inherited_unread_stdin() {
    let scope = tempfile::tempdir().unwrap();
    let mut input = diagnostic_worker(scope.path());
    // Large private input fills stdin while a finite descendant retains its read end.
    input.ticket = "x".repeat(256 * 1024);
    let before: std::collections::HashSet<_> = std::fs::read_dir("/proc/self/task").unwrap().map(|entry| entry.unwrap().file_name()).collect();
    let executable = diagnostic_script(scope.path(), "exec /usr/bin/python3 -c 'import os,select; pid=os.fork(); select.select([],[],[],1.5 if pid==0 else 10); os._exit(0)'");
    let error = supervision::supervise(&input, &executable, Duration::from_millis(300)).unwrap_err();
    assert!(error.contains("timeout"), "{error}");
    let leaked: Vec<_> = std::fs::read_dir("/proc/self/task")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if before.contains(&entry.file_name()) {
                return None;
            }
            let name = std::fs::read_to_string(entry.path().join("comm")).ok()?;
            name.starts_with("management-inpu").then_some((entry.file_name(), name))
        })
        .collect();
    assert!(leaked.is_empty(), "worker timeout returned with a blocked input writer: {leaked:?}");
}

#[cfg(target_os = "linux")]
#[test]
fn runtime_read_deleted_executable_helper() {
    let Some(scope) = std::env::var_os("AGENTX_VERIFICATION_DELETED_IMAGE") else { return };
    let scope = std::path::Path::new(&scope);
    if std::env::var_os("AGENTX_VERIFICATION_RETAINED_IMAGE").is_some() {
        assert!(std::env::current_exe().unwrap().is_file());
        std::fs::write(scope.join("same-loaded-image-ran"), b"ok").unwrap();
        return;
    }
    let executable = std::env::current_exe().unwrap();
    std::fs::remove_file(&executable).unwrap();
    std::fs::write(&executable, b"replacement pathname must never execute").unwrap();
    let retained = supervision::self_executable(scope).unwrap();
    assert_ne!(retained, executable);
    assert!(retained.is_file());
    assert_eq!(supervision::self_executable(scope).unwrap(), retained);
    let output =
        std::process::Command::new(retained).args(["--exact", &format!("{}::runtime_read_deleted_executable_helper", module_path!().split_once("::").unwrap().1), "--nocapture"]).env("AGENTX_VERIFICATION_RETAINED_IMAGE", "1").output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(std::fs::read(scope.join("same-loaded-image-ran")).unwrap(), b"ok");
}

#[cfg(target_os = "linux")]
#[test]
fn runtime_read_worker_reexecutes_loaded_image_after_pathname_replacement() {
    let scope = tempfile::tempdir().unwrap();
    let copy = scope.path().join("disposable-test-image");
    std::fs::copy(std::env::current_exe().unwrap(), &copy).unwrap();
    let output =
        std::process::Command::new(&copy).args(["--exact", &format!("{}::runtime_read_deleted_executable_helper", module_path!().split_once("::").unwrap().1), "--nocapture"]).env("AGENTX_VERIFICATION_DELETED_IMAGE", scope.path()).output().unwrap();
    assert!(output.status.success(), "stdout={} stderr={}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert_eq!(std::fs::read(scope.path().join("same-loaded-image-ran")).unwrap(), b"ok");
}

#[test]
fn runtime_read_http_503_retains_class_duration_and_request_id_without_query() {
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicBool, Ordering};
    let scope = tempfile::tempdir().unwrap();
    std::fs::create_dir(scope.path().join(".daemon")).unwrap();
    crate::management::web::configure(scope.path(), scope.path(), Some("127.0.0.1".parse().unwrap()), Some(0)).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let thread_scope = scope.path().to_path_buf();
    let thread_stop = stop.clone();
    let thread = std::thread::spawn(move || crate::management::web::run(thread_scope, "disposable-http-verification".into(), thread_stop));
    struct StopListener(Arc<AtomicBool>, Option<std::thread::JoinHandle<()>>);
    impl Drop for StopListener {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
            self.1.take().unwrap().join().unwrap();
        }
    }
    let _listener = StopListener(stop, Some(thread));
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let endpoint = loop {
        if let Some(endpoint) = crate::management::web::endpoint(scope.path()) {
            break endpoint;
        }
        assert!(std::time::Instant::now() < deadline, "disposable HTTP listener did not bind");
        std::thread::park_timeout(Duration::from_millis(10));
    };
    // No native daemon exists in this disposable scope: one normal read yields 503.
    let mut stream = std::net::TcpStream::connect(endpoint.url.strip_prefix("http://").unwrap().trim_end_matches('/')).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    stream.write_all(b"GET /api/v1/management?generation=private-query-fixture HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 503"), "{response}");
    let request_id = response.lines().find_map(|line| line.split_once(':').filter(|(key, _)| key.eq_ignore_ascii_case("x-request-id")).map(|(_, value)| value.trim())).unwrap();
    uuid::Uuid::parse_str(request_id).unwrap();
    let logs = crate::log::get_recent_logs_in_scope(scope.path(), 1.0, &["ERROR"], 100);
    let diagnostic = logs.iter().find(|row| row["event"] == "http.failed").expect("actual 503 must be diagnosable");
    assert_eq!(diagnostic["http_request_id"], request_id);
    assert_eq!(diagnostic["route"], "/api/v1/management");
    assert_eq!(diagnostic["status"], "503");
    assert_eq!(diagnostic["msg"], "native_unavailable");
    assert!(diagnostic["duration_ms"].as_str().unwrap().parse::<u64>().is_ok());
    assert!(!diagnostic["error_code"].as_str().unwrap().is_empty());
    let persisted = std::fs::read_to_string(scope.path().join(".tmp/logs/agentx.log")).unwrap();
    assert!(!persisted.contains("private-query-fixture"));
    assert!(!scope.path().join("agentx.db").exists(), "read must never acquire/start a daemon");
}

/// Caller inputs come from describe -> list -> exact get; the original submission
/// is retained for replay, and observation retains the original generation/ID.
fn caller_flow(service: &ManagementService, input: SubmitMessage) -> ServiceResult<(Operation, Operation)> {
    let described = service.describe()?;
    if described.generation != input.generation {
        return Err(ServiceError::StaleFence);
    }
    let mut page = service.list_agents(ListAgentsInput { generation: described.generation.clone(), after: None, limit: None, current_only: true, labels: vec![] })?;
    loop {
        if page.agents.iter().any(|view| view.agent == input.agent) {
            break;
        }
        let after = page.next.ok_or_else(|| ServiceError::NotFound("selected exact Agent".into()))?;
        page = service.list_agents(ListAgentsInput { generation: described.generation.clone(), after: Some(after), limit: None, current_only: true, labels: vec![] })?;
    }
    let view = service.get_agent(GetAgentInput { generation: described.generation, agent: input.agent.clone() })?;
    if !view.can_send {
        return Err(ServiceError::Conflict("selected Agent cannot receive messages".into()));
    }
    let admitted = service.submit_message(input.clone())?;
    let observed = service.get_operation(GetOperationInput { generation: input.generation, request_id: input.request_id })?;
    Ok((admitted, observed))
}

#[test]
fn list_defaults_bounds_and_current_selector_authority() {
    let input: ListAgentsInput = serde_json::from_value(serde_json::json!({"generation":"g"})).unwrap();
    assert!(matches!(list_request(&input).unwrap(), Request::List { after: None, limit: 50 }));
    for limit in [0, MAX_PAGE + 1] {
        assert!(matches!(list_request(&ListAgentsInput { limit: Some(limit), ..input.clone() }), Err(ServiceError::InvalidInput(_))));
    }
    let filtered = ListAgentsInput { labels: vec!["topic=contracts".into(), "!retired".into()], after: Some("selector-bound-cursor".into()), limit: Some(MAX_PAGE), ..input };
    assert!(matches!(list_request(&filtered), Err(ServiceError::InvalidInput(_))));
    let Request::SelectLabels { after, limit, selectors } = list_request(&ListAgentsInput { current_only: true, ..filtered.clone() }).unwrap() else { panic!("current list must use native selector owner") };
    assert_eq!(selectors, filtered.labels);
    assert_eq!(after, filtered.after);
    assert_eq!(limit, MAX_PAGE);
}

#[test]
fn five_methods_compose_and_busy_replay_preserves_identity_through_native_ipc() {
    let root = tempfile::Builder::new().prefix("agentx-management-service-").tempdir().unwrap();
    let database = root.path().join("agentx.db");
    let agent = AgentRef { name: "toku".into(), incarnation: Some(10.0_f64.to_bits().to_string()), stopped_event: None };
    {
        let db = AgentxDb::open_at(&database).unwrap();
        db.save_instance_named("toku", serde_json::json!({"created_at":10.0,"tool":"codex","status":"active","pid":std::process::id()}).as_object().unwrap()).unwrap();
    }
    let daemon = TestDaemon::start(root.path(), &database);
    let service = ManagementService::new(root.path().into(), root.path().into(), InvocationContext { cwd: root.path().into(), ..Default::default() }, Arc::new(Semaphore::new(0)));
    let generation = service.describe().unwrap().generation;
    let input = SubmitMessage {
        generation: generation.clone(),
        request_id: uuid::Uuid::new_v4().to_string(),
        agent: agent.clone(),
        message: MessageInput { text: "native consumer flow".into(), attachments: vec![], intent: Some("inform".into()), reply_to: Some("42".into()), thread: Some("contracts".into()) },
    };
    // Zero worker permits refuse fresh work without writing a receipt.
    assert!(matches!(service.submit_message(input.clone()), Err(ServiceError::Unavailable(_))));
    let lookup = GetOperationInput { generation: generation.clone(), request_id: input.request_id.clone() };
    assert!(matches!(service.get_operation(lookup.clone()), Err(ServiceError::NotFound(_))));
    // A native accepted-but-not-dispatched grant models response loss. This does
    // not execute a worker; every public replay must retrieve this exact grant.
    let Response::Grant(grant) = service.call(&generation, Request::Submit { request_id: input.request_id.clone(), command: Command::Send { agent: agent.clone(), message: input.message.clone() }, admit: true }).unwrap() else {
        panic!("expected original native admission")
    };
    let (accepted, observed) = caller_flow(&service, input.clone()).unwrap();
    assert_eq!(accepted.request_id, input.request_id);
    assert_eq!(observed.generation, generation);
    assert_eq!(observed.status, OperationStatus::Accepted);
    let changed = SubmitMessage { message: MessageInput { text: "changed intent".into(), ..input.message.clone() }, ..input.clone() };
    assert!(matches!(service.submit_message(changed), Err(ServiceError::Conflict(_))));
    let stale = AgentRef { incarnation: Some(11.0_f64.to_bits().to_string()), ..agent };
    assert!(matches!(service.get_agent(GetAgentInput { generation: generation.clone(), agent: stale }), Err(ServiceError::StaleFence)));
    let mut wrong_generation = input.clone();
    wrong_generation.generation = "retired-generation".into();
    assert!(matches!(service.submit_message(wrong_generation), Err(ServiceError::StaleFence)));
    assert!(matches!(service.get_operation(GetOperationInput { generation: "retired-generation".into(), ..lookup.clone() }), Err(ServiceError::StaleFence)));
    assert_eq!(service.workers.available_permits(), 0);
    assert!(grant.ticket.is_some());
    // An unclaimed grant becomes Unknown after daemon replacement. Public replay
    // retains it instead of manufacturing Failed, Sent or a new worker ticket.
    drop(daemon);
    let _restarted = TestDaemon::start(root.path(), &database);
    let unknown = service.get_operation(lookup).unwrap();
    assert_eq!(unknown.status, OperationStatus::Unknown);
    assert!(unknown.result.is_none());
    assert_eq!(service.submit_message(input).unwrap().status, OperationStatus::Unknown);
}

/// Full native host composition: labels have inline receipts, attachments have
/// separate bytes, and Inspector has its own revision/job/attempt result model.
/// No Provider or child operation worker is needed for these finite cases.
#[test]
fn resources_and_inspector_preserve_native_boundaries_through_ipc() {
    let root = tempfile::Builder::new().prefix("agentx-management-full-service-").tempdir().unwrap();
    let database = root.path().join("agentx.db");
    let agent = AgentRef { name: "toku".into(), incarnation: Some(10.0_f64.to_bits().to_string()), stopped_event: None };
    {
        let db = AgentxDb::open_at(&database).unwrap();
        db.save_instance_named("toku", serde_json::json!({"created_at":10.0,"tool":"codex","status":"active","pid":std::process::id()}).as_object().unwrap()).unwrap();
    }
    let _daemon = TestDaemon::start(root.path(), &database);
    let service = ManagementService::new(root.path().into(), root.path().into(), InvocationContext { cwd: root.path().into(), ..Default::default() }, Arc::new(Semaphore::new(0)));
    let generation = service.describe().unwrap().generation;
    let reference = GetAgentInput { generation: generation.clone(), agent: agent.clone() };
    let labels = service.get_labels(reference.clone()).unwrap();
    let patch = SubmitLabels {
        generation: generation.clone(),
        request_id: uuid::Uuid::new_v4().to_string(),
        agent: agent.clone(),
        set: std::collections::BTreeMap::from([("topic".into(), "contract".into())]),
        remove: vec![],
        if_revision: Some(labels.revision),
    };
    let applied = service.patch_labels(patch.clone()).unwrap();
    assert_eq!(applied.status, OperationStatus::Succeeded);
    assert!(matches!(applied.result, Some(CommandResult::LabelsPatched(_))));
    assert_eq!(service.patch_labels(patch).unwrap().request_id, applied.request_id);
    assert_eq!(service.get_labels(reference).unwrap().labels["topic"], "contract");
    assert!(matches!(service.list_messages(AgentPageInput { generation: generation.clone(), agent, after: None, limit: Some(0) }), Err(ServiceError::InvalidInput(_))));
    assert!(service.list_human_mailbox(ListHumanMailboxInput { generation: generation.clone(), before: None, pending: false, limit: None }).unwrap().messages.is_empty());
    assert!(matches!(service.handle_human_messages(HandleHumanMessages { generation: generation.clone(), ids: vec![] }), Err(ServiceError::InvalidInput(_))));
    let bytes = b"\x00\xfffull contract\r\n";
    let attachment = service.upload_attachment(UploadAttachmentInput { generation: generation.clone(), name: "native.bin".into(), media_type: "application/octet-stream".into() }, bytes).unwrap();
    let downloaded = service.download_attachment(GetAttachmentInput { generation: generation.clone(), id: attachment.id.clone() }).unwrap();
    assert_eq!(downloaded.bytes, bytes);
    assert_eq!(downloaded.attachment, attachment);
    let initial = service.inspection_snapshot(GenerationInput { generation: generation.clone() }).unwrap();
    let configure = crate::inspection::contract::ConfigureInput {
        generation: generation.clone(),
        expected_revision: initial.settings.revision,
        enabled: true,
        recovery: crate::inspection::contract::RecoveryPolicy { enabled: false, error_duration_seconds: 600 },
    };
    let configured = service.configure_inspection(configure.clone()).unwrap();
    assert!(configured.settings.enabled);
    assert!(configured.settings.revision > initial.settings.revision);
    assert!(matches!(service.configure_inspection(configure), Err(ServiceError::StaleFence)));
    assert!(matches!(service.analyze_inspection(crate::inspection::contract::AnalyzeInput { generation: generation.clone(), request_id: "missing-observation".into(), observation_ids: vec!["missing".into()] }), Err(ServiceError::StaleFence)));
    assert!(matches!(service.download_attachment(GetAttachmentInput { generation: "old-generation".into(), id: attachment.id }), Err(ServiceError::StaleFence)));
    assert!(service.list_workspaces(GenerationInput { generation }).unwrap().coverage.directories_examined > 0);
}

#[test]
fn inspector_public_tags_round_trip_native_payloads_and_refuse_internal_variants() {
    use crate::inspection::contract::{PublicMutation, Request as InspectRequest};
    let inputs = [
        serde_json::json!({"Configure":{"generation":"g","expected_revision":u64::MAX,"enabled":false,"recovery":{"enabled":false,"error_duration_seconds":600}}}),
        serde_json::json!({"Analyze":{"generation":"g","request_id":"original","observation_ids":["one"]}}),
        serde_json::json!({"CancelAnalysis":{"generation":"g","id":"one"}}),
        serde_json::json!({"RequestContinuation":{"generation":"g","episode":{"agent":{"name":"toku","incarnation":"18446744073709551615","stopped_event":null},"session_id":"s","source":"model_request","failing_turn":"t","error_since":i64::MIN}}}),
    ];
    for wire in inputs {
        let public: PublicMutation = serde_json::from_value(wire.clone()).unwrap();
        let native: InspectRequest = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(public).unwrap(), wire);
        assert_eq!(serde_json::to_value(native).unwrap(), wire);
    }
    for private in [serde_json::json!({"Read":{"generation":"g"}}), serde_json::json!({"Collect":{"generation":"g","expected_revision":0,"samples":[],"discovery_complete":true}}), serde_json::json!({"RunAnalysis":{"generation":"g","id":"one"}})] {
        assert!(serde_json::from_value::<PublicMutation>(private).is_err());
    }
    assert!(serde_json::from_value::<PublicMutation>(serde_json::json!({"Configure":{"generation":"g","expected_revision":0,"enabled":false,"recovery":{"enabled":false,"error_duration_seconds":600},"admit":true}})).is_err());
}

#[test]
fn message_history_service_advertises_capability_and_fences_exact_reads() {
    let root = tempfile::tempdir().unwrap();
    let database = root.path().join("agentx.db");
    let agent = AgentRef { name: "toku".into(), incarnation: Some(10.0_f64.to_bits().to_string()), stopped_event: None };
    let events = {
        let db = AgentxDb::open_at(&database).unwrap();
        db.save_instance_named("toku", serde_json::json!({"created_at":10.0,"tool":"codex"}).as_object().unwrap()).unwrap();
        (0..22).map(|index| {
            db.insert_event_row("message", "toku", &serde_json::json!({"text":format!("item {index}")}), Some("1970-01-01T00:00:11.000000+00:00")).unwrap().to_string()
        }).collect::<Vec<_>>()
    };
    let _daemon = TestDaemon::start(root.path(), &database);
    let service = ManagementService::new(root.path().into(), root.path().into(), InvocationContext { cwd: root.path().into(), ..Default::default() }, Arc::new(Semaphore::new(0)));
    let descriptor = service.describe().unwrap();
    assert!(descriptor.supports_message_history);
    let input = MessageHistoryInput { generation: descriptor.generation, agent, before: None, limit: None };
    let page = service.message_history(input.clone()).unwrap();
    assert_eq!(page.messages.into_iter().map(|message| message.id).collect::<Vec<_>>(), events[2..]);
    assert_eq!(page.next_before, Some(events[2].clone()));
    assert_eq!(page.live_after.as_ref(), events.last());
    let older = service.message_history(MessageHistoryInput { before: page.next_before, ..input.clone() }).unwrap();
    assert_eq!(older.messages.into_iter().map(|message| message.id).collect::<Vec<_>>(), events[..2]);
    assert!(older.next_before.is_none());
    assert!(older.live_after.is_none());

    assert!(matches!(service.message_history(MessageHistoryInput { generation: "retired-generation".into(), ..input.clone() }), Err(ServiceError::StaleFence)));
    let stale_agent = AgentRef { incarnation: Some(9.0_f64.to_bits().to_string()), ..input.agent.clone() };
    assert!(matches!(service.message_history(MessageHistoryInput { agent: stale_agent, ..input.clone() }), Err(ServiceError::StaleFence)));
    assert!(matches!(service.message_history(MessageHistoryInput { limit: Some(101), ..input.clone() }), Err(ServiceError::InvalidInput(_))));
    assert!(matches!(service.message_history(MessageHistoryInput { before: Some("0".into()), ..input }), Err(ServiceError::InvalidInput(_))));
}
