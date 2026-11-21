//! Independent A6/A8 cases: real production envelope/ledgers, adversarial peers.
//! Case meaning and evidence limits: `.docs/codex-activation/acceptance.md`.

use super::*;
use crate::agents::attachments::{ActivationTarget, BindingId, ExecutionId, ParticipantRef};
use serde_json::{Value, json};
use std::collections::HashSet;

fn target() -> ActivationTarget {
    ActivationTarget { participant: ParticipantRef { canonical_name: "lobo".into(), incarnation_bits: 12.5_f64.to_bits() }, execution: ExecutionId("backend-generation-a".into()), binding: BindingId("binding-a".into()) }
}

fn vectors() -> Value {
    serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/codex_shared_vectors.json"))).unwrap()
}

fn id_json(id: &RpcId) -> Value {
    match id {
        RpcId::Integer(value) => json!(value),
        RpcId::Text(value) => json!(value),
    }
}

fn request(router: &mut ProtocolRouter, raw: &str) -> (RpcId, String) {
    match router.route_outer(raw).unwrap() {
        OuterRoute::Request { downstream_id, frame, .. } => (downstream_id, frame),
        other => panic!("expected outer request, got {other:?}"),
    }
}

#[test]
fn acceptance_a6_reordered_typed_ids_share_allocator_with_internal_requests() {
    let mut router = ProtocolRouter::new(64);
    let fixtures = vectors();
    let mut downstream_ids = HashSet::new();
    let mut pending = Vec::new();
    let mut internal = Vec::new();
    for input in fixtures["clientRequests"].as_array().unwrap() {
        let (downstream, frame) = request(&mut router, &input.to_string());
        assert!(downstream_ids.insert(downstream.clone()));
        let forwarded: Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(forwarded["params"], input["params"]);
        assert_eq!(forwarded["method"], input["method"]);
        pending.push((downstream, input["id"].clone()));
        let owner = ClientRequestOwner::Reconciliation { target: target() };
        let id = router.reserve_internal(owner.clone()).unwrap();
        assert!(downstream_ids.insert(id.clone()), "internal and outer request collided");
        internal.push((id, owner));
    }
    // An event can be observed before the internal response. It must remain a
    // real event, while the following internal response stays inside AgentX.
    let event = r#"{"method":"turn/completed","params":{"threadId":"root-a","turn":{"items":[]}}}"#;
    assert!(matches!(router.route_native(event).unwrap(), NativeRoute::Notification { frame, .. } if frame == event));
    for (id, expected_owner) in internal.into_iter().rev() {
        let response = json!({"id": id_json(&id), "result": {"opaque": "internal-only"}}).to_string();
        match router.route_native(&response).unwrap() {
            NativeRoute::InternalResponse { downstream_id, owner, frame } => {
                assert_eq!(downstream_id, id);
                assert_eq!(owner, expected_owner);
                assert_eq!(frame, response);
            }
            other => panic!("internal response leaked as {other:?}"),
        }
    }
    for (id, original_id) in pending.into_iter().rev() {
        let response = json!({"id": id_json(&id), "error": {"code": -32601, "message": "future method", "data": {"requestId": 1}}}).to_string();
        match router.route_native(&response).unwrap() {
            NativeRoute::OuterResponse { frame, .. } => {
                let restored: Value = serde_json::from_str(&frame).unwrap();
                assert_eq!(restored["id"], original_id);
                assert_eq!(restored["error"]["data"]["requestId"], 1);
            }
            other => panic!("outer response misrouted as {other:?}"),
        }
    }
    assert!(router.close().is_empty());
}

#[test]
fn acceptance_a6_native_server_ids_are_independent_of_client_ids() {
    let mut router = ProtocolRouter::new(16);
    let (client_id, _) = request(&mut router, r#"{"id":1,"method":"thread/read","params":{}}"#);
    let fixtures = vectors();
    for message in fixtures["serverRequests"].as_array().unwrap() {
        let raw = message.to_string();
        match router.route_native(&raw).unwrap() {
            NativeRoute::ServerRequest { frame, .. } => assert_eq!(frame, raw),
            other => panic!("server request misrouted as {other:?}"),
        }
    }
    for message in fixtures["resolvedNotifications"].as_array().unwrap() {
        let raw = message.to_string();
        assert!(matches!(router.route_native(&raw).unwrap(), NativeRoute::Notification { frame, .. } if frame == raw));
    }
    // Resolved control replies can arrive late and still go to native. They
    // must not consume an equal outstanding client id in the other direction.
    for message in fixtures["serverRequests"].as_array().unwrap() {
        let reply = json!({"id": message["id"], "result": {"fixture": "answer"}}).to_string();
        assert!(matches!(router.route_outer(&reply).unwrap(), OuterRoute::ServerResponse { frame, .. } if frame == reply));
    }
    let response = json!({"id": id_json(&client_id), "result": {"thread": "client-result"}}).to_string();
    assert!(matches!(router.route_native(&response).unwrap(), NativeRoute::OuterResponse { original_id: RpcId::Integer(1), .. }));
}

#[test]
fn acceptance_a6_opaque_values_survive_id_rewrite_without_numeric_rounding() {
    let raw = r#"{"id":"outer","method":"future/method","params": { "huge": 1234567890123456789012345678901234567890, "exponent":1e400, "nested":{"id":9223372036854775808}, "text":"\u4e2d\\\"" },"extension": [true,null,1.2300e-7]}"#;
    let mut router = ProtocolRouter::new(2);
    let (id, forwarded) = request(&mut router, raw);
    let before = RawObject::parse(raw).unwrap();
    let after = RawObject::parse(&forwarded).unwrap();
    for field in ["params", "extension", "method"] {
        assert_eq!(before.get(field).unwrap().get(), after.get(field).unwrap().get(), "opaque field {field} changed");
    }
    let response = format!(r#"{{"id":{},"result":{{"future":1e400,"embeddedId":9223372036854775808}},"extension":1.2300e-7}}"#, id_json(&id));
    let expected = RawObject::parse(&response).unwrap();
    match router.route_native(&response).unwrap() {
        NativeRoute::OuterResponse { frame, .. } => {
            let restored = RawObject::parse(&frame).unwrap();
            assert_eq!(restored.get("id").unwrap().get(), r#""outer""#);
            assert_eq!(restored.get("result").unwrap().get(), expected.get("result").unwrap().get());
            assert_eq!(restored.get("extension").unwrap().get(), "1.2300e-7");
        }
        other => panic!("unexpected route {other:?}"),
    }
}

#[test]
fn acceptance_a6_long_unknown_notifications_remain_exact_and_do_not_claim_rpc_capacity() {
    let mut router = ProtocolRouter::new(1);
    let _ = request(&mut router, r#"{"id":1,"method":"thread/read"}"#);
    let raw = json!({"method": "future/notification", "params": {"literal": "中文😀\\\"\n".repeat(40000)}}).to_string();
    assert!(raw.len() > 256 * 1024);
    assert!(matches!(router.route_outer(&raw).unwrap(), OuterRoute::Notification { frame, .. } if frame == raw));
    assert!(matches!(router.route_native(&raw).unwrap(), NativeRoute::Notification { frame, .. } if frame == raw));
    // Server direction has its own capacity even with the client side full.
    let server = r#"{"id":1,"method":"item/tool/call","params":{"future":true}}"#;
    assert!(matches!(router.route_native(server).unwrap(), NativeRoute::ServerRequest { frame, .. } if frame == server));
}

#[test]
fn acceptance_a6_duplicate_outer_id_rejected_but_typed_id_and_reuse_are_safe() {
    let mut router = ProtocolRouter::new(4);
    let (old_id, _) = request(&mut router, r#"{"id":1,"method":"thread/read"}"#);
    assert!(matches!(router.route_outer(r#"{"id":1,"method":"thread/list"}"#), Err(ProtocolFailure::Route(RouteFailure::DuplicateOutstandingId))));
    let _ = request(&mut router, r#"{"id":"1","method":"thread/read"}"#);
    let response = json!({"id": id_json(&old_id), "result": {}}).to_string();
    router.route_native(&response).unwrap();
    let (new_id, _) = request(&mut router, r#"{"id":1,"method":"thread/read"}"#);
    assert_ne!(old_id, new_id, "late response could take over reused downstream id");
    assert!(matches!(router.route_native(&response), Err(ProtocolFailure::Route(RouteFailure::UnknownResponseId))));
    let new_response = json!({"id": id_json(&new_id), "result": {"fresh":true}}).to_string();
    assert!(matches!(router.route_native(&new_response).unwrap(), NativeRoute::OuterResponse { original_id: RpcId::Integer(1), .. }));
}

#[test]
fn acceptance_a8_close_returns_fenced_internal_work_and_rejects_new_writes() {
    let mut router = ProtocolRouter::new(2);
    let owner = ClientRequestOwner::Activation { target: target(), attempt: crate::messaging::activation::AttemptId("attempt-a".into()) };
    let internal_id = router.reserve_internal(owner.clone()).unwrap();
    let (outer_id, _) = request(&mut router, r#"{"id":"cancel","method":"turn/interrupt"}"#);
    assert!(matches!(router.reserve_internal(owner.clone()), Err(RouteFailure::Capacity)));
    let pending = router.close();
    assert_eq!(pending.len(), 2);
    assert!(pending.contains(&(internal_id.clone(), owner.clone())));
    assert!(pending.contains(&(outer_id, ClientRequestOwner::Outer { original_id: RpcId::Text("cancel".into()) })));
    assert!(matches!(router.reserve_internal(owner), Err(RouteFailure::Closed)));
    assert!(matches!(router.route_outer(r#"{"id":"new","method":"thread/start"}"#), Err(ProtocolFailure::Route(RouteFailure::Closed))));
    let late = json!({"id": id_json(&internal_id), "result": {}}).to_string();
    assert!(matches!(router.route_native(&late), Err(ProtocolFailure::Route(RouteFailure::Closed))));
}

#[test]
fn acceptance_a6_server_resolution_releases_only_exact_typed_control_id() {
    let mut ledger = PendingServerRequests::new(2);
    ledger.track(RpcId::Integer(1)).unwrap();
    ledger.track(RpcId::Text("1".into())).unwrap();
    assert_eq!(ledger.track(RpcId::Integer(1)), Err(RouteFailure::DuplicateOutstandingId));
    ledger.resolve(&RpcId::Integer(1)).unwrap();
    assert_eq!(ledger.resolve(&RpcId::Integer(1)), Err(RouteFailure::UnknownResponseId));
    assert_eq!(ledger.track(RpcId::Text("1".into())), Err(RouteFailure::DuplicateOutstandingId));
    ledger.track(RpcId::Integer(2)).unwrap();
    assert_eq!(ledger.track(RpcId::Integer(3)), Err(RouteFailure::Capacity));
}

#[test]
fn acceptance_a6_invalid_envelopes_fail_before_reserving_any_request() {
    let mut router = ProtocolRouter::new(1);
    for invalid in [
        r#"{"id":null,"method":"thread/read"}"#,
        r#"{"id":true,"method":"thread/read"}"#,
        r#"{"id":1.5,"method":"thread/read"}"#,
        r#"{"id":9223372036854775808,"method":"thread/read"}"#,
        r#"{"id":1,"id":2,"method":"thread/read"}"#,
        r#"{"id":1,"method":false}"#,
        r#"{"id":1,"result":{},"error":{}}"#,
        r#"{"id":1}"#,
        "[]",
    ] {
        assert!(router.route_outer(invalid).is_err(), "accepted malformed envelope {invalid}");
    }
    let _ = request(&mut router, r#"{"id":1,"method":"thread/read"}"#);
}

#[test]
fn acceptance_a6_connection_ledgers_do_not_share_original_id_or_owner() {
    let mut first = ProtocolRouter::new(1);
    let mut second = ProtocolRouter::new(1);
    let (first_id, _) = request(&mut first, r#"{"id":"same","method":"thread/read"}"#);
    let (second_id, _) = request(&mut second, r#"{"id":"same","method":"thread/list"}"#);
    first.close();
    let response = json!({"id": id_json(&second_id), "result": {"still": "alive"}}).to_string();
    assert!(matches!(second.route_native(&response).unwrap(), NativeRoute::OuterResponse { original_id: RpcId::Text(id), .. } if id == "same"));
    let stale = json!({"id": id_json(&first_id), "result": {}}).to_string();
    assert!(first.route_native(&stale).is_err());
}

#[test]
fn acceptance_a6_initialize_preserves_client_contract_and_virtualizes_only_observation_opt_outs() {
    let mut handshake = ConnectionHandshake::default();
    let initialize = r#"{"id":"init","method":"initialize","params":{"clientInfo":{"name":"unchanged-client","version":"original"},"capabilities":{"experimentalApi":false,"futureCapability":{"huge":1e400},"optOutNotificationMethods":["turn/completed","item/completed","future/progress"]},"extension":true}}"#;
    let native = handshake.prepare_initialize(RpcId::Integer(5), initialize).unwrap();
    let original = NativeFrame::parse(initialize).unwrap().params().unwrap().unwrap();
    let forwarded = NativeFrame::parse(&native).unwrap().params().unwrap().unwrap();
    assert_eq!(forwarded.get("clientInfo").unwrap().get(), original.get("clientInfo").unwrap().get());
    assert_eq!(forwarded.get("extension").unwrap().get(), "true");
    let capabilities = RawObject::parse(forwarded.get("capabilities").unwrap().get()).unwrap();
    assert_eq!(capabilities.get("experimentalApi").unwrap().get(), "false");
    assert_eq!(capabilities.get("futureCapability").unwrap().get(), r#"{"huge":1e400}"#);
    let native_opt_out: Vec<String> = serde_json::from_str(capabilities.get("optOutNotificationMethods").unwrap().get()).unwrap();
    assert_eq!(native_opt_out, ["future/progress"]);
    assert!(!handshake.ready());
    handshake.observe_initialized();
    assert!(!handshake.ready(), "early notification granted activation authority");
    handshake.observe_response(&RpcId::Integer(5), r#"{"id":5,"result":{"userAgent":"native"}}"#).unwrap();
    assert!(!handshake.ready(), "response alone granted activation authority");
    handshake.observe_initialized();
    assert!(handshake.ready());
    assert!(!handshake.forwards_notification("turn/completed"));
    assert!(!handshake.forwards_notification("item/completed"));
    assert!(!handshake.forwards_notification("future/progress"));
    assert!(handshake.forwards_notification("turn/started"));
}

#[test]
fn acceptance_a6_failed_duplicate_initialize_cannot_replace_outer_subscription() {
    let mut handshake = ConnectionHandshake::default();
    handshake.prepare_initialize(RpcId::Integer(1), r#"{"id":1,"method":"initialize","params":{"capabilities":{"optOutNotificationMethods":["item/completed"]}}}"#).unwrap();
    handshake.observe_response(&RpcId::Integer(1), r#"{"id":1,"result":{}}"#).unwrap();
    handshake.observe_initialized();
    handshake.prepare_initialize(RpcId::Integer(2), r#"{"id":2,"method":"initialize","params":{"capabilities":{"optOutNotificationMethods":["turn/started"]}}}"#).unwrap();
    handshake.observe_response(&RpcId::Integer(2), r#"{"id":2,"error":{"code":-32600,"message":"already initialized"}}"#).unwrap();
    assert!(handshake.ready());
    assert!(!handshake.forwards_notification("item/completed"));
    assert!(handshake.forwards_notification("turn/started"));
}

#[test]
fn acceptance_a6_rejected_initialization_and_missing_capabilities_do_not_enable_features() {
    let mut handshake = ConnectionHandshake::default();
    let rejected = r#"{"id":"bad-init","method":"initialize","params":{"clientInfo":{"name":"bad-client"}}}"#;
    assert_eq!(handshake.prepare_initialize(RpcId::Integer(1), rejected).unwrap(), rejected);
    handshake.observe_response(&RpcId::Integer(1), r#"{"id":1,"error":{"code":-32600,"message":"denied"}}"#).unwrap();
    handshake.observe_initialized();
    assert!(!handshake.ready());
    let retry = r#"{"id":"retry","method":"initialize","params":{"clientInfo":{"name":"plain"},"capabilities":null}}"#;
    assert_eq!(handshake.prepare_initialize(RpcId::Integer(2), retry).unwrap(), retry);
    handshake.observe_response(&RpcId::Integer(2), r#"{"id":2,"result":{}}"#).unwrap();
    handshake.observe_initialized();
    assert!(handshake.ready());
    assert!(handshake.forwards_notification("item/completed"));
}
