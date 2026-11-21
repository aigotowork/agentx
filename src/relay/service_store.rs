//! Daemon-owned relay adapter. Network publication never runs in this handler.
use super::service::*;
use crate::db::AgentxDb;
use crate::service::{RequestContext, ServiceError, ServiceResult};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Replay windows are isolated by database path, relay group and database generation.
type ReplayScopeKey = (std::path::PathBuf, String, String);
type ReplayWindows = HashMap<ReplayScopeKey, super::replay::ReplayGuard>;

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    match request {
        Request::Export { device_uuid } => {
            let previous_cursor = super::safe_kv_get(db, "relay_last_push_id").and_then(|s| s.parse().ok()).unwrap_or(0);
            let (state, events, _, has_more) = super::push::build_push_payload(db, &device_uuid);
            Ok(Response::Export(ExportPage { state, events, previous_cursor, has_more, sealed_payload_budget: super::push::sealed_payload_budget(db) }))
        }
        Request::CommitPush { previous_cursor, through } => {
            if through < previous_cursor {
                return Err(ServiceError::InvalidInput("relay cursor cannot regress".into()));
            }
            db.with_write_scope(|| {
                let current = db.kv_get("relay_last_push_id")?.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                anyhow::ensure!(current == previous_cursor, "relay push cursor changed before publication acknowledgement");
                let now = crate::shared::time::now_epoch_f64().to_string();
                db.kv_set("relay_last_push_id", Some(&through.to_string()))?;
                db.kv_set("relay_last_push", Some(&now))?;
                db.kv_set("relay_last_sync", Some(&now))?;
                Ok(())
            })
            .map_err(|e| ServiceError::Conflict(e.to_string()))?;
            super::set_relay_status(db, "ok", None, true);
            Ok(Response::Changed(true))
        }
        Request::Receive(envelope) => receive(db, context, envelope).map(Response::Received),
        Request::BeginControl { effect_id, token } => super::control_store::begin(db, context, &effect_id, &token).map(Response::ControlGranted),
        Request::CompleteControl { effect_id, token, reply } => super::control_store::complete(db, context, &effect_id, &token, reply).map(Response::Changed),
        Request::Capabilities { short_id } => super::control::read_remote_capabilities(db, &short_id).map(Response::Capabilities).map_err(ServiceError::InvalidInput),
        Request::ExpectRpc { request_id, short_id, action } => {
            super::control_store::expect_rpc(db, context, &request_id, &short_id, &action)?;
            Ok(Response::Changed(true))
        }
        Request::Status { configured, enabled, broker } => {
            let observation = super::RelayObservation {
                configured,
                enabled,
                broker,
                raw_status: db.kv_get("relay_status").map_err(internal)?,
                raw_error: db.kv_get("relay_last_error").map_err(internal)?,
                heartbeat_age_s: super::worker_heartbeat_age(db),
                pidfile: super::worker::observe_pid_file_in(db.path().parent().ok_or_else(|| ServiceError::Internal("relay database has no data scope".into()))?),
                last_push: db.kv_get("relay_last_push").map_err(internal)?.and_then(|value| value.parse().ok()).unwrap_or(0.0),
            };
            let health = super::derive_relay_health(&observation);
            Ok(Response::Status(super::RelayStatus {
                configured,
                enabled,
                status: observation.raw_status,
                error: observation.raw_error,
                last_push: observation.last_push,
                broker: observation.broker,
                heartbeat_age: observation.heartbeat_age_s,
                pidfile_pid: observation.pidfile.map(|(pid, _)| pid),
                health,
            }))
        }
        Request::Snapshot { own_device } => snapshot(db, &own_device).map(Response::Snapshot),
        Request::ClearDeviceState => {
            super::clear_relay_device_state(db);
            Ok(Response::Changed(true))
        }
        Request::ClearWorkerRuntime => {
            super::clear_runtime_relay_kv(db).map_err(internal)?;
            Ok(Response::Changed(true))
        }
        Request::Backfill { device_uuid } => {
            let short_id = super::device_short_id_for_db(db, &device_uuid);
            let mut requests = Vec::new();
            let summary = super::backfill::tick(db, &short_id, crate::shared::time::now_epoch_f64(), &mut |short_id, request_id, params| {
                requests.push(BackfillRequest { short_id: short_id.into(), request_id: request_id.into(), params: params.clone() });
                false // Publication stays client-owned; durable pending state follows enqueue.
            });
            Ok(Response::Backfill { requests, wake_ports: normalize_wake_ports(summary.wake_ports) })
        }
        Request::BackfillPublished { short_id, request_id, params } => {
            for device in super::backfill::devices_with_gaps(db) {
                let mut gaps = super::backfill::load_gaps(db, &device);
                let Some(gap) = gaps.first_mut() else {
                    continue;
                };
                if gap.short_id == short_id && gap.request_id.is_none() && super::backfill::request_params(gap) == params {
                    gap.request_id = Some(request_id);
                    gap.sent_at = crate::shared::time::now_epoch_f64();
                    gap.attempts += 1;
                    db.with_write_scope(|| super::backfill::save_gaps_checked(db, &device, &gaps)).map_err(internal)?;
                    return Ok(Response::Changed(true));
                }
            }
            Err(ServiceError::StaleFence)
        }
        Request::WorkerHeartbeat => {
            super::control_store::reconcile_abandoned(db)?;
            db.kv_set(super::HEARTBEAT_KEY, Some(&crate::shared::time::now_epoch_f64().to_string())).map_err(internal)?;
            Ok(Response::Changed(true))
        }
        Request::WorkerStopped => {
            db.with_write_scope(|| {
                db.kv_set(super::HEARTBEAT_KEY, None)?;
                db.kv_set("relay_daemon_port", None)?;
                db.kv_set("relay_status", Some("disconnected"))?;
                db.kv_set("relay_last_error", None)?;
                Ok(())
            })
            .map_err(internal)?;
            Ok(Response::Changed(true))
        }
        Request::WorkerStatus { status, error } => {
            if !matches!(status.as_str(), "ok" | "error" | "disconnected" | "connecting") {
                return Err(ServiceError::InvalidInput("invalid relay worker status".into()));
            }
            super::set_relay_status(db, &status, error.as_deref(), true);
            Ok(Response::Changed(true))
        }
        Request::BrokerPacketLimit { max_bytes } => {
            db.kv_set(super::BROKER_MAX_PACKET_KEY, max_bytes.map(|n| n.to_string()).as_deref()).map_err(internal)?;
            Ok(Response::Changed(true))
        }
        Request::NotifyPort { port } => {
            db.kv_set("relay_daemon_port", port.map(|n| n.to_string()).as_deref()).map_err(internal)?;
            Ok(Response::Changed(true))
        }
        Request::WorkerPort => Ok(Response::WorkerPort(db.kv_get("relay_daemon_port").map_err(internal)?.and_then(|s| s.parse().ok()))),
        Request::ShortId { device_uuid } => Ok(Response::ShortId(super::device_short_id_for_db(db, &device_uuid))),
        Request::RpcResult { request_id, consume } => super::control_store::rpc_result(db, context, &request_id, consume).map(Response::RpcResult),
    }
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

/// Preserve the CLI's existing queue and peer predicates without exposing a KV
/// or SQL capability. Every database/row failure aborts the complete projection.
fn snapshot(db: &AgentxDb, own_device: &str) -> ServiceResult<RelaySnapshot> {
    if own_device.len() > 256 {
        return Err(ServiceError::InvalidInput("relay own-device identity exceeds 256 bytes".into()));
    }
    let cursor = db.kv_get("relay_last_push_id").map_err(internal)?.and_then(|value| value.parse::<i64>().ok()).unwrap_or(0);
    let queued = db.conn().query_row("SELECT COUNT(*) FROM events WHERE id > ?1 AND instance NOT LIKE '%:%' AND instance != '_device' AND json_extract(data, '$._relay') IS NULL", [cursor], |row| row.get(0)).map_err(internal)?;
    let mappings = {
        let mut statement = db
            .conn()
            .prepare(
                "SELECT peer.key, peer.value, sync.value FROM kv AS peer LEFT JOIN kv AS sync ON sync.key = 'relay_sync_time_' || peer.value \
             WHERE peer.key LIKE ?1 ESCAPE '\\' AND peer.value IS NOT NULL AND peer.value != ?2 ORDER BY peer.key",
            )
            .map_err(internal)?;
        let rows = statement.query_map(rusqlite::params![r"relay\_short\_%", own_device], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))).map_err(internal)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)?
    };
    let mut agent_names = HashMap::<String, Vec<String>>::new();
    let mut statement = db.conn().prepare("SELECT origin_device_id, name FROM instances WHERE origin_device_id IS NOT NULL AND origin_device_id != '' ORDER BY name").map_err(internal)?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(internal)?;
    for row in rows {
        let (device, name) = row.map_err(internal)?;
        agent_names.entry(device).or_default().push(name.split(':').next().unwrap_or(&name).to_string());
    }
    let peers = mappings
        .into_iter()
        .map(|(key, device_uuid, sync)| Peer {
            short: key.strip_prefix(super::RELAY_SHORT_PREFIX).unwrap_or(&key).to_string(),
            sync_time: sync.and_then(|value| value.parse::<f64>().ok()).filter(|value| value.is_finite()).unwrap_or(0.0),
            agents: agent_names.get(&device_uuid).cloned().unwrap_or_default(),
            device_uuid,
        })
        .collect();
    Ok(RelaySnapshot { queued, peers })
}

fn normalize_wake_ports(mut ports: Vec<u16>) -> Vec<u16> {
    ports.retain(|port| *port > 0);
    ports.sort_unstable();
    ports.dedup();
    ports
}

/// Keep authenticated nonce windows across requests, separately per canonical
/// data scope and relay group. This is transport replay state, not caller identity.
fn receive(db: &AgentxDb, request_context: &RequestContext, envelope: Envelope) -> ServiceResult<ReceiveResult> {
    if envelope.payload.len() > super::MQTT_MAX_PACKET_BYTES || envelope.relay_id.len() > 256 || envelope.own_device.len() > 256 || envelope.topic.len() > 1024 {
        return Err(ServiceError::InvalidInput("relay envelope exceeds its metadata or packet budget".into()));
    }
    static GUARDS: OnceLock<Mutex<ReplayWindows>> = OnceLock::new();
    let prefix = format!("{}/", envelope.relay_id);
    let Some(suffix) = envelope.topic.strip_prefix(&prefix) else {
        return Ok(ReceiveResult::default());
    };
    if envelope.payload.is_empty() || suffix.is_empty() || suffix == envelope.own_device {
        return Ok(ReceiveResult::default());
    }
    let mut guards = GUARDS.get_or_init(Default::default).lock().map_err(|_| ServiceError::Internal("relay replay state poisoned".into()))?;
    guards.retain(|(path, _, generation), _| path != db.path() || generation == &request_context.database_generation);
    let key = (db.path().to_path_buf(), envelope.relay_id.clone(), request_context.database_generation.clone());
    if !guards.contains_key(&key) && guards.len() >= 64 {
        return Err(ServiceError::Unavailable("relay replay scope capacity is full".into()));
    }
    let guard = guards.entry(key).or_default();
    let mut context = super::pull::InboundContext { psk: &envelope.psk, relay_id: &envelope.relay_id, topic: &envelope.topic, replay_guard: guard };
    let mut result = ReceiveResult::default();
    let mut failure = None;
    let mut control = |db: &AgentxDb, events: &[serde_json::Value], own_short: &str, source: &str| match super::control_store::accept(db, request_context, &envelope.relay_id, events, own_short, source, &envelope.own_device) {
        Ok(received) => {
            let changed = received.changed;
            result.effects.extend(received.effects);
            changed
        }
        Err(error) => {
            failure = Some(error);
            false
        }
    };
    let mut wake_ports = Vec::new();
    let changed = if suffix == "control" {
        super::pull::handle_control_message_with(db, &envelope.payload, &envelope.own_device, &mut context, &mut control)
    } else {
        super::pull::handle_state_message_with(db, suffix, &envelope.payload, &envelope.own_device, &mut context, &mut control, &mut wake_ports)
    };
    if let Some(error) = failure {
        return Err(error);
    }
    result.changed = changed;
    result.wake_ports = normalize_wake_ports(wake_ports);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> (tempfile::TempDir, AgentxDb, RequestContext) {
        let directory = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_raw(&directory.path().join("agentx.db")).unwrap();
        db.init_db().unwrap();
        (directory, db, super::super::control_store::test_context())
    }
    fn envelope(db: &AgentxDb, params: serde_json::Value, at: f64) -> Envelope {
        let own = super::super::device_short_id_for_db(db, "fixture-own");
        let plaintext =
            serde_json::to_vec(&json!({"from_device":"fixture-peer","events":[{"type":"control","ts":at,"data":{"action":"launch","target_device":own,"request_id":"wire-launch-a","from":"_:ABCD","from_device":"fixture-peer","params":params}}]}))
                .unwrap();
        let topic = "fixture-relay/control";
        let payload = super::super::crypto::seal(&[0x35; 32], "fixture-relay", topic, &plaintext, at as u64).unwrap();
        Envelope { relay_id: "fixture-relay".into(), own_device: "fixture-own".into(), topic: topic.into(), payload, psk: [0x35; 32] }
    }
    #[test]
    fn authenticated_receive_returns_plan_and_nonce_replay_never_executes_it() {
        let (_directory, db, context) = fixture();
        let at = crate::shared::time::now_epoch_f64();
        let incoming = envelope(&db, json!({"tool":"codex","cwd":"/fixture/not-run"}), at);
        let Response::Received(received) = handle(&db, &context, Request::Receive(incoming.clone())).unwrap() else {
            panic!("received response");
        };
        assert_eq!(received.effects.len(), 1);
        assert!(!received.changed);
        let Response::Received(duplicate) = handle(&db, &context, Request::Receive(incoming)).unwrap() else {
            panic!("received response");
        };
        assert!(duplicate.effects.is_empty());
        assert!(!duplicate.changed);
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM instances", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
    }
    #[test]
    fn tampered_or_different_topic_envelopes_do_not_admit_control() {
        let (_directory, db, context) = fixture();
        let mut incoming = envelope(&db, json!({"tool":"codex"}), crate::shared::time::now_epoch_f64());
        let mut tampered = incoming.clone();
        tampered.payload[super::super::crypto::HEADER_LEN] ^= 1;
        let Response::Received(received) = handle(&db, &context, Request::Receive(tampered)).unwrap() else {
            panic!("received response");
        };
        assert!(received.effects.is_empty());
        incoming.topic = "different-group/control".into();
        let Response::Received(received) = handle(&db, &context, Request::Receive(incoming)).unwrap() else {
            panic!("received response");
        };
        assert!(received.effects.is_empty());
        assert!(db.kv_prefix("relay_control_receipt:").unwrap().is_empty());
    }
    #[test]
    fn fresh_nonce_cannot_change_an_already_accepted_wire_request_intent() {
        let (_directory, db, context) = fixture();
        let at = crate::shared::time::now_epoch_f64();
        handle(&db, &context, Request::Receive(envelope(&db, json!({"tool":"codex"}), at))).unwrap();
        let result = handle(&db, &context, Request::Receive(envelope(&db, json!({"tool":"claude"}), at + 0.001)));
        assert!(matches!(result, Err(ServiceError::Conflict(_))));
        assert_eq!(db.kv_prefix("relay_control_active:").unwrap().len(), 1);
    }
    #[test]
    fn export_without_enqueue_or_failed_cursor_commit_never_advances_watermark() {
        let (_directory, db, context) = fixture();
        db.kv_set("relay_last_push_id", Some("7")).unwrap();
        let Response::Export(page) = handle(&db, &context, Request::Export { device_uuid: "fixture-own".into() }).unwrap() else {
            panic!("export response");
        };
        assert_eq!(page.previous_cursor, 7);
        assert_eq!(db.kv_get("relay_last_push_id").unwrap().as_deref(), Some("7"));
        db.conn().execute_batch("CREATE TRIGGER refuse_push_cursor BEFORE INSERT ON kv WHEN NEW.key='relay_last_push_id' BEGIN SELECT RAISE(ABORT,'fixture refusal'); END;").unwrap();
        assert!(handle(&db, &context, Request::CommitPush { previous_cursor: 7, through: 10 }).is_err());
        assert_eq!(db.kv_get("relay_last_push_id").unwrap().as_deref(), Some("7"));
        assert!(db.kv_get("relay_last_push").unwrap().is_none());
        assert!(handle(&db, &context, Request::CommitPush { previous_cursor: 6, through: 10 }).is_err());
    }

    #[test]
    fn worker_runtime_cleanup_failure_rolls_back_and_never_confirms_a_partial_clear() {
        let (_directory, db, context) = fixture();
        for key in super::super::RUNTIME_HEALTH_KV_KEYS {
            db.kv_set(key, Some("original")).unwrap();
        }
        db.kv_set("relay_last_push_id", Some("17")).unwrap();
        db.conn().execute_batch("CREATE TRIGGER refuse_relay_port_clear BEFORE DELETE ON kv WHEN OLD.key='relay_daemon_port' BEGIN SELECT RAISE(ABORT,'fixture refusal'); END;").unwrap();
        assert!(handle(&db, &context, Request::ClearWorkerRuntime).is_err());
        for key in super::super::RUNTIME_HEALTH_KV_KEYS {
            assert_eq!(db.kv_get(key).unwrap().as_deref(), Some("original"));
        }
        db.conn().execute_batch("DROP TRIGGER refuse_relay_port_clear;").unwrap();
        assert!(matches!(handle(&db, &context, Request::ClearWorkerRuntime), Ok(Response::Changed(true))));
        for key in super::super::RUNTIME_HEALTH_KV_KEYS {
            assert!(db.kv_get(key).unwrap().is_none());
        }
        assert_eq!(db.kv_get("relay_last_push_id").unwrap().as_deref(), Some("17"));
    }

    #[test]
    fn worker_stopped_failure_rolls_back_heartbeat_port_and_status_together() {
        let (_directory, db, context) = fixture();
        for (key, value) in [(super::super::HEARTBEAT_KEY, "heartbeat"), ("relay_daemon_port", "32123"), ("relay_status", "ok"), ("relay_last_error", "old error")] {
            db.kv_set(key, Some(value)).unwrap();
        }
        db.conn().execute_batch("CREATE TRIGGER refuse_worker_stop BEFORE DELETE ON kv WHEN OLD.key='relay_daemon_port' BEGIN SELECT RAISE(ABORT,'fixture refusal'); END;").unwrap();
        assert!(handle(&db, &context, Request::WorkerStopped).is_err());
        for (key, expected) in [(super::super::HEARTBEAT_KEY, "heartbeat"), ("relay_daemon_port", "32123"), ("relay_status", "ok"), ("relay_last_error", "old error")] {
            assert_eq!(db.kv_get(key).unwrap().as_deref(), Some(expected));
        }
        db.conn().execute_batch("DROP TRIGGER refuse_worker_stop;").unwrap();
        assert!(matches!(handle(&db, &context, Request::WorkerStopped), Ok(Response::Changed(true))));
        assert!(db.kv_get(super::super::HEARTBEAT_KEY).unwrap().is_none());
        assert!(db.kv_get("relay_daemon_port").unwrap().is_none());
        assert_eq!(db.kv_get("relay_status").unwrap().as_deref(), Some("disconnected"));
        assert!(db.kv_get("relay_last_error").unwrap().is_none());
    }

    #[test]
    fn authenticated_departure_freezes_allowed_wake_ports_without_connecting_in_dispatch() {
        let (_directory, db, context) = fixture();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        db.upsert_notify_endpoint("observer", "listen", port).unwrap();
        db.upsert_notify_endpoint("rpc-only", "inject", port.wrapping_add(1).max(1)).unwrap();
        db.upsert_notify_endpoint("new-session", "session_start", port.wrapping_add(2).max(1)).unwrap();
        let topic = "fixture-relay/fixture-peer";
        let payload = super::super::crypto::seal(&[0x35; 32], "fixture-relay", topic, br#"{"state":null,"events":[]}"#, crate::shared::time::now_epoch_f64() as u64).unwrap();
        let incoming = Envelope { relay_id: "fixture-relay".into(), own_device: "fixture-own".into(), topic: topic.into(), payload, psk: [0x35; 32] };
        let Response::Received(received) = handle(&db, &context, Request::Receive(incoming)).unwrap() else {
            panic!("received response");
        };
        assert_eq!(received.wake_ports, vec![port]);
        assert!(matches!(listener.accept(),Err(error) if error.kind()==std::io::ErrorKind::WouldBlock));
    }

    #[test]
    fn backfill_publication_failure_does_not_claim_durable_correlation_or_spend_an_attempt() {
        let (_directory, db, context) = fixture();
        super::super::backfill::record_gap(&db, "fixture-peer", "ABCD", 10, 20);
        let params = super::super::backfill::request_params(&super::super::backfill::load_gaps(&db, "fixture-peer")[0]);
        let before = db.kv_get("relay_gap_fixture-peer").unwrap();
        db.conn().execute_batch("CREATE TRIGGER refuse_backfill_pending BEFORE INSERT ON kv WHEN NEW.key='relay_gap_fixture-peer' BEGIN SELECT RAISE(ABORT,'fixture refusal'); END;").unwrap();
        let request = || Request::BackfillPublished { short_id: "ABCD".into(), request_id: "published-request".into(), params: params.clone() };
        assert!(matches!(handle(&db, &context, request()), Err(ServiceError::Internal(_))));
        assert_eq!(db.kv_get("relay_gap_fixture-peer").unwrap(), before);
        let gap = super::super::backfill::load_gaps(&db, "fixture-peer").remove(0);
        assert!(gap.request_id.is_none());
        assert_eq!(gap.attempts, 0);
        db.conn().execute_batch("DROP TRIGGER refuse_backfill_pending;").unwrap();
        assert!(matches!(handle(&db, &context, Request::BackfillPublished { short_id: "ABCD".into(), request_id: "changed-request".into(), params: json!({"last":1}) }), Err(ServiceError::StaleFence)));
        assert!(matches!(handle(&db, &context, request()), Ok(Response::Changed(true))));
        let gap = super::super::backfill::load_gaps(&db, "fixture-peer").remove(0);
        assert_eq!(gap.request_id.as_deref(), Some("published-request"));
        assert_eq!(gap.attempts, 1);
    }

    #[test]
    fn backfill_import_freezes_allowed_wake_ports_without_connecting_in_dispatch() {
        let (_directory, db, context) = fixture();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        db.upsert_notify_endpoint("observer", "listen", port).unwrap();
        db.upsert_notify_endpoint("rpc-only", "inject", port.wrapping_add(1).max(1)).unwrap();
        db.upsert_notify_endpoint("new-session", "session_start", port.wrapping_add(2).max(1)).unwrap();
        db.kv_set("relay_short_ABCD", Some("fixture-peer")).unwrap();
        db.kv_set("relay_sync_time_fixture-peer", Some(&crate::shared::time::now_epoch_f64().to_string())).unwrap();
        db.kv_set("relay_caps_fixture-peer", Some(r#"["events"]"#)).unwrap();
        super::super::backfill::record_gap(&db, "fixture-peer", "ABCD", 10, 20);
        let Response::Backfill { mut requests, wake_ports } = handle(&db, &context, Request::Backfill { device_uuid: "fixture-own".into() }).unwrap() else {
            panic!("backfill response");
        };
        assert!(wake_ports.is_empty());
        assert_eq!(requests.len(), 1);
        assert!(super::super::backfill::load_gaps(&db, "fixture-peer")[0].request_id.is_none());
        let request = requests.remove(0);
        handle(&db, &context, Request::BackfillPublished { short_id: request.short_id, request_id: request.request_id.clone(), params: request.params }).unwrap();
        db.insert_event_row("rpc_result","_rpc",&json!({"request_id":request.request_id,"action":"events","ok":true,"_relay":{"device":"fixture-peer"},"result":{"events":[{"id":15,"ts":crate::shared::time::now_epoch_f64(),"type":"message","instance":"origin","data":{"from":"origin","text":"repaired","mentions":[]}}]}}),None).unwrap();
        let Response::Backfill { requests, wake_ports } = handle(&db, &context, Request::Backfill { device_uuid: "fixture-own".into() }).unwrap() else {
            panic!("backfill response");
        };
        assert!(requests.is_empty());
        assert_eq!(wake_ports, vec![port]);
        assert!(super::super::backfill::load_gaps(&db, "fixture-peer").is_empty());
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM events WHERE json_extract(data,'$._relay.device')='fixture-peer' AND json_extract(data,'$._relay.id')=15", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
        assert!(matches!(listener.accept(),Err(error) if error.kind()==std::io::ErrorKind::WouldBlock));
    }

    #[test]
    fn snapshot_preserves_queue_filters_peer_aliases_and_ordered_base_agent_names() {
        let (_directory, db, context) = fixture();
        let cursor = db.insert_event_row("message", "older", &json!({}), None).unwrap();
        db.kv_set("relay_last_push_id", Some(&cursor.to_string())).unwrap();
        for (kind, instance, data) in [
            ("message", "local", json!({})),
            ("control", "_control", json!({})),
            ("rpc_result", "_rpc", json!({})),
            ("message", "remote:ABCD", json!({})),
            ("life", "_device", json!({})),
            ("message", "imported", json!({"_relay":{"device":"peer-a"}})),
        ] {
            db.insert_event_row(kind, instance, &data, None).unwrap();
        }
        for (key, value) in [
            ("relay_short_OWN", "fixture-own"),
            ("relay_short_ABCD", "peer-a"),
            ("relay_short_AXYZ", "peer-a"),
            ("relay_short_ZZZZ", "peer-b"),
            ("relayXshort_TRAP", "not-a-peer-mapping"),
            ("relay_sync_time_peer-a", "123.5"),
            ("relay_sync_time_peer-b", "invalid-time"),
        ] {
            db.kv_set(key, Some(value)).unwrap();
        }
        for (name, device) in [("zeta:ABCD", "peer-a"), ("alpha:ABCD", "peer-a"), ("own:OWN", "fixture-own"), ("hidden:XX", "unmapped-peer")] {
            db.conn().execute("INSERT INTO instances(name,created_at,origin_device_id) VALUES (?1,1.0,?2)", rusqlite::params![name, device]).unwrap();
        }
        let before_events = db.get_last_event_id();
        let Response::Snapshot(snapshot) = handle(&db, &context, Request::Snapshot { own_device: "fixture-own".into() }).unwrap() else {
            panic!("snapshot response");
        };
        assert_eq!(snapshot.queued, 3);
        assert_eq!(
            snapshot.peers,
            vec![
                Peer { device_uuid: "peer-a".into(), short: "ABCD".into(), sync_time: 123.5, agents: vec!["alpha".into(), "zeta".into()] },
                Peer { device_uuid: "peer-a".into(), short: "AXYZ".into(), sync_time: 123.5, agents: vec!["alpha".into(), "zeta".into()] },
                Peer { device_uuid: "peer-b".into(), short: "ZZZZ".into(), sync_time: 0.0, agents: vec![] },
            ]
        );
        assert_eq!(db.get_last_event_id(), before_events);
        assert_eq!(db.kv_get("relay_last_push_id").unwrap(), Some(cursor.to_string()));
    }

    #[test]
    fn snapshot_query_or_peer_decode_failure_never_reports_empty_success() {
        let (_directory, db, context) = fixture();
        db.conn().execute_batch("ALTER TABLE events RENAME TO unavailable_events;").unwrap();
        assert!(matches!(handle(&db, &context, Request::Snapshot { own_device: "fixture-own".into() }), Err(ServiceError::Internal(_))));
        let (_directory, db, context) = fixture();
        db.conn().execute("INSERT INTO kv(key,value) VALUES ('relay_short_BAD',X'FF')", []).unwrap();
        assert!(matches!(handle(&db, &context, Request::Snapshot { own_device: "fixture-own".into() }), Err(ServiceError::Internal(_))));
    }
}
