//! Independent A2/A3/A7/A8 bridge cases. Only native frame I/O is scripted;
//! identity attachments, claims, phases and cursor effects use real SQLite.

use super::*;
use crate::agents::attachments::{AttachmentIntent, AttachmentRequest, ProviderSessionKey, SqliteAttachmentStore};
use crate::db::AgentxDb;
use crate::integrations::codex::roots::NativeThreadKey;
use crate::integrations::codex::service::{ConnectionFailure, Receive, ServiceCompatibility};
use crate::messaging::inbox::{ClaimPhase, EventId, SqlitePendingInbox};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// In-process daemon boundary: real persistence with authenticated fixture-host
/// evidence. The bridge sees only the offered HostService capability.
pub(super) struct FixtureHosts<'a> {
    db: &'a AgentxDb,
    context: crate::service::RequestContext,
}

impl<'a> FixtureHosts<'a> {
    pub(super) fn new(db: &'a AgentxDb) -> Self {
        use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestContext, RequestId};
        let pid = std::process::id();
        Self {
            db,
            context: RequestContext {
                caller: CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: None },
                process: CallerProcess { pid, incarnation: crate::sys::process::identity(pid) },
                daemon_instance: "bridge-fixture-daemon".into(),
                database_generation: "bridge-fixture-generation".into(),
            },
        }
    }
}

impl HostService for FixtureHosts<'_> {
    fn call(&self, request: crate::runtime::hosts::Request) -> crate::service::ServiceResult<crate::runtime::hosts::Response> {
        use crate::runtime::hosts::{Request, Response};
        let mut context = self.context.clone();
        context.caller.request_id = crate::service::RequestId(uuid::Uuid::new_v4().to_string());
        match request {
            Request::ReserveNativeRoot(input) => crate::agents::service::reserve_native_root(self.db, &context, input).map(Response::Reserved),
            request => crate::runtime::hosts_store::handle(self.db, &context, request),
        }
    }
}

#[derive(Default)]
struct Wire {
    sent: Vec<String>,
    fail_next_write: bool,
    closed: bool,
}

struct ScriptedConnection(Arc<Mutex<Wire>>);

impl NativeConnection for ScriptedConnection {
    fn send(&mut self, frame: &str) -> Result<(), ConnectionFailure> {
        let mut wire = self.0.lock().unwrap();
        wire.sent.push(frame.to_owned());
        if wire.fail_next_write {
            wire.fail_next_write = false;
            return Err(ConnectionFailure::Io("fixture lost connection after possible partial write".into()));
        }
        Ok(())
    }

    fn receive(&mut self, _timeout: Duration) -> Result<Receive, ConnectionFailure> {
        Ok(Receive::TimedOut)
    }

    fn close(&mut self) -> Result<(), ConnectionFailure> {
        self.0.lock().unwrap().closed = true;
        Ok(())
    }
}

fn sent(wire: &Arc<Mutex<Wire>>) -> Vec<Value> {
    wire.lock().unwrap().sent.iter().map(|frame| serde_json::from_str(frame).unwrap()).collect()
}

fn inputs(wire: &Arc<Mutex<Wire>>) -> Vec<Value> {
    sent(wire).into_iter().filter(|frame| matches!(frame["method"].as_str(), Some("turn/start" | "turn/steer"))).collect()
}

fn queue(db: &AgentxDb) -> i64 {
    let message = json!({"from":"child","text":"frozen batch alpha","scope":"mentions","mentions":["lobo"],"delivered_to":["lobo"],"sender_kind":"instance","intent":"inform"});
    db.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES('2026-11-07T15:00:00','message','child',?)", [message.to_string()]).unwrap();
    db.conn().last_insert_rowid()
}

fn cursor(db: &AgentxDb) -> i64 {
    db.conn().query_row("SELECT last_event_id FROM instances WHERE name='lobo'", [], |row| row.get(0)).unwrap()
}

fn with_bridge(state: ThreadState, body: impl FnOnce(&mut NativeBridge<'_>, &AgentxDb, Arc<Mutex<Wire>>)) {
    with_observed_home(state, false, body);
}

fn with_observed_home(state: ThreadState, mismatch: bool, body: impl FnOnce(&mut NativeBridge<'_>, &AgentxDb, Arc<Mutex<Wire>>)) {
    let (_dir, agentx_dir, home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
    let db = AgentxDb::open_at(&agentx_dir.join("agentx.db")).unwrap();
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
    let launch = RootLaunch { explicit_name: None, initial_cursor: Some(0), launch_args: args, launch_profile: None };
    let mut attachments = SqliteAttachmentStore::new(&db);
    let mut inbox = SqlitePendingInbox::new(&db);
    let wire = Arc::new(Mutex::new(Wire::default()));
    let mut transport = ScriptedConnection(wire.clone());
    let hosts = FixtureHosts::new(&db);
    let wake = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let wake_port = wake.local_addr().unwrap().port();
    let ports = BridgePorts { attachments: &mut attachments, inbox: &mut inbox, transport: &mut transport, hosts: &hosts };
    let mut bridge = NativeBridge::new(configuration, launch, identity.clone(), ports, wake_port);
    db.save_instance_named("lobo", json!({"created_at":10.0,"tool":"codex","status":"active","last_event_id":0}).as_object().unwrap()).unwrap();
    let participant = bridge.ports.attachments.participant("lobo").unwrap();
    let attachment = bridge
        .ports
        .attachments
        .attach(AttachmentRequest {
            execution: identity.execution,
            owner: bridge.owner.clone(),
            session: ProviderSessionKey { provider: "codex".into(), scope: native_home.to_string_lossy().into_owned(), session_id: "root-a".into() },
            intent: AttachmentIntent::Create { provisional: participant },
        })
        .unwrap();
    bridge.ports.inbox.select_delivery_mode(&attachment.target, crate::messaging::inbox::DeliveryMode::NativeInput).unwrap();
    let crate::runtime::hosts::Response::Registered(host) = hosts
        .call(crate::runtime::hosts::Request::RegisterNativeRoot {
            target: attachment.target.clone(),
            owner: bridge.owner.clone(),
            wake_port,
            state: if state == ThreadState::Idle { crate::runtime::hosts::NativeRootState::Idle } else { crate::runtime::hosts::NativeRootState::Busy },
        })
        .unwrap()
    else {
        panic!("fixture host registration response");
    };
    bridge.roots.insert(
        "root-a".into(),
        BoundRoot {
            binding: RootBinding { attachment, native_thread: NativeThreadKey { canonical_home: native_home.to_string_lossy().into_owned(), thread_id: "root-a".into() } },
            state,
            turn_id: (state == ThreadState::Active).then(|| "active-turn-a".to_owned()),
            suspended: false,
            host,
        },
    );
    bridge.handle_outer(r#"{"id":"initialize","method":"initialize","params":{"clientInfo":{"name":"independent-fixture","version":"1"}}}"#).unwrap();
    let initialize = sent(&wire).pop().unwrap();
    let observed_home = if mismatch { home.join("unexpected-native-home") } else { native_home };
    bridge.handle_native(&json!({"id":initialize["id"],"result":{"codexHome":observed_home}}).to_string()).unwrap();
    bridge.handle_outer(r#"{"method":"initialized","params":{}}"#).unwrap();
    wire.lock().unwrap().sent.clear();
    body(&mut bridge, &db, wire);
}

fn matching_record(input: &Value) -> Value {
    json!({"method":"item/completed","params":{"threadId":"root-a","turnId":"fixture-turn","item":{"type":"userMessage","clientId":input["params"]["clientUserMessageId"],"content":input["params"]["input"]}}})
}

#[test]
#[serial_test::serial]
fn acceptance_a4_resume_owned_by_sibling_never_submits_config_or_native_request() {
    with_bridge(ThreadState::Idle, |bridge, db, wire| {
        let attachment = bridge.roots["root-a"].binding.attachment.clone();
        bridge.ports.attachments.retire(&attachment.target).unwrap();
        let sibling =
            bridge.ports.attachments.attach(AttachmentRequest { execution: attachment.target.execution, owner: AttachmentOwnerId("live-sibling".into()), session: attachment.session, intent: AttachmentIntent::Resume { provisional: None } }).unwrap();
        let before: i64 = db.conn().query_row("SELECT count(*) FROM instances", [], |row| row.get(0)).unwrap();
        let result = bridge.handle_outer(r#"{"id":"conflicting-resume","method":"thread/resume","params":{"threadId":"root-a"}}"#);
        assert!(result.is_err(), "sibling ownership must reject before native configuration changes");
        assert!(wire.lock().unwrap().sent.is_empty(), "resume/config was sent before rejecting ownership");
        assert!(bridge.ports.attachments.validate(&sibling.target).unwrap());
        assert_eq!(db.conn().query_row::<i64, _, _>("SELECT count(*) FROM instances", [], |row| row.get(0)).unwrap(), before);
    });
}

#[test]
#[serial_test::serial]
fn acceptance_a9_connected_home_mismatch_forwards_traffic_but_disables_autonomous_input() {
    with_observed_home(ThreadState::Idle, true, |bridge, db, wire| {
        queue(db);
        bridge.drive_pending().unwrap();
        assert!(inputs(&wire).is_empty());
        assert_eq!(cursor(db), 0);
        bridge.handle_outer(r#"{"id":"opaque-client","method":"future/read","params":{"nested":{"id":1},"text":"literal"}}"#).unwrap();
        let request = sent(&wire).pop().unwrap();
        assert_eq!(request["method"], "future/read");
        assert_eq!(request["params"]["nested"]["id"], 1);
        let result = bridge.handle_native(&json!({"id":request["id"],"result":{"opaque":true}}).to_string()).unwrap().unwrap();
        assert_eq!(serde_json::from_str::<Value>(&result).unwrap(), json!({"id":"opaque-client","result":{"opaque":true}}));
        let notification = r#"{"method":"future/nativeEvent","params":{"id":"opaque-event"}}"#;
        assert_eq!(bridge.handle_native(notification).unwrap(), Some(notification.into()));
        bridge.drive_pending().unwrap();
        assert!(inputs(&wire).is_empty());
        assert_eq!(cursor(db), 0);
    });
}

#[test]
#[serial_test::serial]
fn acceptance_a2_stale_expected_turn_proof_requires_state_refresh_before_reoffering() {
    with_bridge(ThreadState::Active, |bridge, db, wire| {
        // Proven rejection releases the claim; keep one-time guidance fixed
        // while asserting exact message content across the new claim.
        db.kv_set("tip:lobo:recv:intent:inform", Some("1")).unwrap();
        queue(db);
        bridge.drive_pending().unwrap();
        let first = inputs(&wire).pop().unwrap();
        let rejection = json!({"id":first["id"],"error":{"code":-32600,"message":"expected active turn id `active-turn-a` but found `none`"}});
        assert!(bridge.handle_native(&rejection.to_string()).unwrap().is_none());
        bridge.drive_pending().unwrap();
        assert_eq!(inputs(&wire).len(), 1, "known rejection retried without synchronizing native state");
        assert_eq!(cursor(db), 0);
        let refresh = sent(&wire).into_iter().find(|frame| frame["method"] == "thread/read").unwrap();
        bridge.handle_native(&json!({"id":refresh["id"],"result":{"thread":{"id":"root-a","status":{"type":"idle"},"turns":[]}}}).to_string()).unwrap();
        bridge.drive_pending().unwrap();
        let messages = inputs(&wire);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1]["method"], "turn/start");
        assert_eq!(messages[1]["params"]["input"], first["params"]["input"]);
        assert_eq!(cursor(db), 0);
    });
}

#[test]
#[serial_test::serial]
fn acceptance_a3_archive_before_mail_retires_binding_without_reopening_on_late_events() {
    with_bridge(ThreadState::Idle, |bridge, db, wire| {
        bridge.handle_outer(r#"{"id":"archive","method":"thread/archive","params":{"threadId":"root-a"}}"#).unwrap();
        let archive = sent(&wire).pop().unwrap();
        queue(db);
        bridge.drive_pending().unwrap();
        assert!(inputs(&wire).is_empty());
        bridge.handle_native(&json!({"id":archive["id"],"result":{}}).to_string()).unwrap();
        for event in [json!({"method":"turn/started","params":{"threadId":"root-a","turn":{"id":"late"}}}), json!({"method":"thread/status/changed","params":{"threadId":"root-a","status":{"type":"idle"}}})] {
            bridge.handle_native(&event.to_string()).unwrap();
        }
        bridge.drive_pending().unwrap();
        assert!(inputs(&wire).is_empty(), "retired root reopened by late native traffic");
        assert!(!bridge.ports.attachments.validate(&bridge.roots["root-a"].binding.attachment.target).unwrap());
        assert_eq!(cursor(db), 0);
    });
}

#[test]
#[serial_test::serial]
fn acceptance_a5_native_internal_thread_events_never_register_or_rebind_participants() {
    with_bridge(ThreadState::Idle, |bridge, db, _wire| {
        let original = bridge.roots["root-a"].binding.attachment.clone();
        for event in [
            json!({"method":"thread/started","params":{"thread":{"id":"internal-memory","source":"memoryConsolidation"}}}),
            json!({"method":"thread/status/changed","params":{"threadId":"internal-subagent","status":{"type":"active","activeFlags":[]}}}),
            json!({"method":"turn/started","params":{"threadId":"internal-subagent","turn":{"id":"internal-turn"}}}),
        ] {
            bridge.handle_native(&event.to_string()).unwrap();
        }
        assert_eq!(bridge.roots.len(), 1);
        assert_eq!(bridge.roots["root-a"].binding.attachment, original);
        assert_eq!(db.conn().query_row::<i64, _, _>("SELECT count(*) FROM instances", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(db.conn().query_row::<i64, _, _>("SELECT count(*) FROM participant_sessions", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(cursor(db), 0);
    });
}

#[test]
#[serial_test::serial]
fn acceptance_a2_active_delivery_uses_frozen_text_and_expected_turn() {
    with_bridge(ThreadState::Active, |bridge, db, wire| {
        let mail = queue(db);
        bridge.drive_pending().unwrap();
        let messages = inputs(&wire);
        assert_eq!(messages.len(), 1, "{messages:?}");
        let input = &messages[0];
        assert_eq!(input["method"], "turn/steer");
        assert_eq!(input["params"]["expectedTurnId"], "active-turn-a");
        assert!(input["params"]["input"].to_string().contains("frozen batch alpha"));
        assert_eq!(cursor(db), 0, "native write became delivery evidence");
        bridge.handle_native(&matching_record(input).to_string()).unwrap();
        assert_eq!(cursor(db), mail);
    });
}

#[test]
#[serial_test::serial]
fn acceptance_a2_pending_server_controls_delay_mail_but_answers_and_cancel_forward() {
    for method in ["item/commandExecution/requestApproval", "item/tool/requestUserInput", "future/nativeControl"] {
        with_bridge(ThreadState::Idle, |bridge, db, wire| {
            queue(db);
            let control = json!({"id":"control-a","method":method,"params":{"threadId":"root-a"}}).to_string();
            assert_eq!(bridge.handle_native(&control).unwrap(), Some(control));
            bridge.drive_pending().unwrap();
            assert!(inputs(&wire).is_empty(), "mail issued while {method} needed a response");
            let answer = r#"{"id":"control-a","result":{"fixture":"answer"}}"#;
            bridge.handle_outer(answer).unwrap();
            assert!(wire.lock().unwrap().sent.iter().any(|frame| frame == answer));
            bridge.handle_outer(r#"{"id":"cancel","method":"turn/interrupt","params":{"threadId":"root-a","turnId":"active-turn-a"}}"#).unwrap();
            assert!(sent(&wire).iter().any(|frame| frame["method"] == "turn/interrupt"));
            assert_eq!(cursor(db), 0);
        });
    }
}

#[test]
#[serial_test::serial]
fn acceptance_a3_arrived_user_input_is_preserved_and_not_overwritten_by_mail() {
    with_bridge(ThreadState::Idle, |bridge, db, wire| {
        queue(db);
        let user = json!({"id":"user-input","method":"turn/start","params":{"threadId":"root-a","input":[{"type":"text","text":"original user request"}],"futureField":{"id":1}}});
        bridge.handle_outer(&user.to_string()).unwrap();
        bridge.drive_pending().unwrap();
        let writes = inputs(&wire);
        assert_eq!(writes.len(), 1, "internal input bypassed pending user mutation");
        assert_eq!(writes[0]["method"], "turn/start");
        assert_eq!(writes[0]["params"], user["params"]);
        let response = json!({"id":writes[0]["id"],"result":{"turn":{"id":"user-turn","status":"inProgress"}}});
        let visible: Value = serde_json::from_str(&bridge.handle_native(&response.to_string()).unwrap().unwrap()).unwrap();
        assert_eq!(visible["id"], user["id"]);
        assert_eq!(cursor(db), 0);
    });
}

#[test]
#[serial_test::serial]
fn acceptance_a7_rpc_success_wrong_records_and_empty_completion_never_commit_mail() {
    let fixtures: Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/codex_shared_vectors.json"))).unwrap();
    for vector in fixtures["receiptCounterexamples"].as_array().unwrap().iter().filter(|vector| vector["commits"] == false) {
        with_bridge(ThreadState::Idle, |bridge, db, wire| {
            queue(db);
            bridge.drive_pending().unwrap();
            let input = inputs(&wire).pop().expect("no native input issued");
            let mut event = vector["message"].clone();
            let case = vector["case"].as_str().unwrap();
            if case == "rpc-success-only" {
                event["id"] = input["id"].clone();
            } else if let Some(item) = event["params"].get_mut("item") {
                if case != "wrong-client-id" {
                    item["clientId"] = input["params"]["clientUserMessageId"].clone();
                }
                if case != "wrong-content" {
                    item["content"] = input["params"]["input"].clone();
                }
            }
            bridge.handle_native(&event.to_string()).unwrap();
            assert_eq!(cursor(db), 0, "{case} was treated as delivery evidence");
        });
    }
}

#[test]
#[serial_test::serial]
fn acceptance_a7_exact_record_before_rpc_response_commits_once_and_late_reply_stays_internal() {
    with_bridge(ThreadState::Idle, |bridge, db, wire| {
        let mail = queue(db);
        bridge.drive_pending().unwrap();
        let input = inputs(&wire).pop().unwrap();
        let record = matching_record(&input).to_string();
        assert!(bridge.handle_native(&record).unwrap().is_some(), "real native event was hidden");
        assert_eq!(cursor(db), mail);
        bridge.handle_native(&record).unwrap();
        assert_eq!(cursor(db), mail);
        let response = json!({"id":input["id"],"result":{"turn":{"id":"fixture-turn","status":"inProgress"}}}).to_string();
        assert_eq!(bridge.handle_native(&response).unwrap(), None, "late internal RPC response leaked");
        bridge.drive_pending().unwrap();
        assert_eq!(inputs(&wire).len(), 1, "recorded batch was issued twice");
    });
}

#[test]
#[serial_test::serial]
fn acceptance_a7_blocked_hook_and_empty_turn_do_not_form_an_automatic_retry_loop() {
    with_bridge(ThreadState::Idle, |bridge, db, wire| {
        queue(db);
        bridge.drive_pending().unwrap();
        let input = inputs(&wire).pop().unwrap();
        bridge.handle_native(&json!({"id":input["id"],"result":{"turn":{"id":"fixture-turn","status":"inProgress"}}}).to_string()).unwrap();
        bridge.handle_native(r#"{"method":"hook/completed","params":{"threadId":"root-a","run":{"eventName":"userPromptSubmit","status":"blocked"}}}"#).unwrap();
        bridge.handle_native(r#"{"method":"turn/completed","params":{"threadId":"root-a","turn":{"id":"fixture-turn","items":[],"status":"completed"}}}"#).unwrap();
        for _ in 0..3 {
            bridge.drive_pending().unwrap();
        }
        assert_eq!(inputs(&wire).len(), 1, "blocked input retried without any changed condition");
        assert_eq!(cursor(db), 0);
    });
}

#[test]
#[serial_test::serial]
fn acceptance_a8_possible_partial_write_retires_as_uncertain_and_never_replays() {
    with_bridge(ThreadState::Idle, |bridge, db, wire| {
        queue(db);
        wire.lock().unwrap().fail_next_write = true;
        let _ = bridge.drive_pending();
        assert_eq!(inputs(&wire).len(), 1);
        assert_eq!(cursor(db), 0);
        let target = bridge.roots["root-a"].binding.attachment.target.clone();
        let claim = bridge.ports.inbox.outstanding(&target).unwrap().expect("possible native effect lost its claim");
        assert!(matches!(claim.phase, ClaimPhase::Issuing | ClaimPhase::Uncertain));
        bridge.close().unwrap();
        assert_eq!(cursor(db), 0);
        let resolution: String = db.conn().query_row("SELECT resolution FROM inbox_claims WHERE batch_id=?", [&claim.batch.id.0], |row| row.get(0)).unwrap();
        assert_eq!(resolution, "open");
        assert_eq!(inputs(&wire).len(), 1);
        assert_ne!(bridge.ports.inbox.delivered_through(&target.participant).unwrap_or(EventId(0)), claim.batch.window.through);
    });
}

#[test]
fn acceptance_a7_receipt_counterexamples_require_exact_completed_native_record() {
    let fixtures: Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/codex_shared_vectors.json"))).unwrap();
    for vector in fixtures["receiptCounterexamples"].as_array().unwrap() {
        assert_eq!(recorded_input_matches(&vector["message"].to_string(), "root-a", "batch-a", "frozen batch α"), vector["commits"].as_bool().unwrap(), "{}", vector["case"]);
    }
    let extra_content = json!({"method":"item/completed","params":{"threadId":"root-a","item":{"type":"userMessage","clientId":"batch-a","content":[{"type":"text","text":"frozen batch α"},{"type":"text","text":"unexpected appended input"}]}}});
    assert!(!recorded_input_matches(&extra_content.to_string(), "root-a", "batch-a", "frozen batch α"));
    assert!(!recorded_input_matches("not json", "root-a", "batch-a", "frozen batch α"));
}

#[test]
#[serial_test::serial]
fn acceptance_a8_failed_or_empty_history_never_proves_an_issued_input_absent() {
    for result in [json!({"error":{"code":-32000,"message":"history unavailable"}}), json!({"result":{"thread":{"id":"root-a","status":{"type":"idle"},"turns":[]}}})] {
        with_bridge(ThreadState::Idle, |bridge, db, wire| {
            queue(db);
            bridge.drive_pending().unwrap();
            let input = inputs(&wire).pop().unwrap();
            bridge.handle_native(&json!({"id":input["id"],"error":{"code":-32000,"message":"unknown side effect"}}).to_string()).unwrap();
            let target = bridge.roots["root-a"].binding.attachment.target.clone();
            let claim = bridge.ports.inbox.outstanding(&target).unwrap().unwrap();
            assert_eq!(claim.phase, ClaimPhase::Uncertain);
            bridge.request_state("root-a", Some(claim.clone())).unwrap();
            let history_read = sent(&wire).into_iter().rev().find(|frame| frame["method"] == "thread/read").unwrap();
            let mut reply = result.clone();
            reply["id"] = history_read["id"].clone();
            assert_eq!(bridge.handle_native(&reply.to_string()).unwrap(), None);
            bridge.drive_pending().unwrap();
            assert_eq!(inputs(&wire).len(), 1, "an incomplete or failed history read unlocked replay");
            assert_eq!(cursor(db), 0);
            assert_eq!(bridge.ports.inbox.outstanding(&target).unwrap().unwrap().batch.id, claim.batch.id);
        });
    }
}
