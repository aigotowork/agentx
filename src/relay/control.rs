//! Control events — remote RPC over the MQTT control topic.
//!
//! Control messages are published to `{relay_id}/control` (non-retained) and
//! currently carry request/response style remote actions.

use rumqttc::QoS;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

use crate::config::AgentxConfig;
use crate::db::AgentxDb;
use crate::log;
#[cfg(test)]
use crate::runtime::launcher;

#[cfg(test)]
use super::safe_kv_set;
use super::{control_topic, crypto, is_relay_enabled, load_psk, safe_kv_get};

/// Build a sealed RPC control envelope ready to publish. Returns the topic and
/// the AEAD-sealed bytes; the underlying JSON layout is unchanged from the
/// pre-encryption format so callers don't have to know about the cipher.
fn build_control_payload(client: &crate::service::client::Client, config: &AgentxConfig, action: &str, target_device_short_id: &str, request_id: Option<&str>, params: &serde_json::Value) -> Option<(String, Vec<u8>)> {
    let device_id = super::read_device_uuid_in(client.scope_root())?;
    let super::service::Response::ShortId(short_id) = super::persistence::call(client, super::service::Request::ShortId { device_uuid: device_id.clone() }).ok()? else {
        return None;
    };
    build_control_payload_for_device(config, action, target_device_short_id, request_id, params, &device_id, &short_id)
}

/// Seal a peer protocol operation after resolving local identity through the daemon.
pub(crate) fn build_control_payload_for_device(config: &AgentxConfig, action: &str, target_device_short_id: &str, request_id: Option<&str>, params: &Value, device_id: &str, short_id: &str) -> Option<(String, Vec<u8>)> {
    if !is_relay_enabled(config) {
        return None;
    }
    let relay_id = &config.relay_id;
    let psk = load_psk(config).ok()?;
    let now = crate::shared::time::now_epoch_f64();
    let mut control_data = json!({
        "action": action,
        "target_device": target_device_short_id,
        "from": format!("_:{}", short_id),
        "from_device": device_id,
        "params": params,
    });
    if let Some(request_id) = request_id {
        control_data["request_id"] = Value::String(request_id.to_string());
    }

    let control_payload = json!({
        "from_device": device_id,
        "events": [{
            "ts": now,
            "type": "control",
            "instance": "_control",
            "data": control_data,
        }],
    });

    let topic = control_topic(relay_id);
    let plaintext = serde_json::to_vec(&control_payload).ok()?;
    let sealed = crypto::seal(&psk, relay_id, &topic, &plaintext, now as u64).ok()?;
    Some((topic, sealed))
}

fn send_control_via_ephemeral(storage: &crate::service::client::Client, config: &AgentxConfig, client: &super::client::EphemeralClient, action: &str, target_device_short_id: &str, request_id: Option<&str>, params: &serde_json::Value) -> bool {
    let (topic, payload_bytes) = match build_control_payload(storage, config, action, target_device_short_id, request_id, params) {
        Some(v) => v,
        None => return false,
    };

    let result = client.publish_and_wait(&topic, QoS::AtLeastOnce, false, payload_bytes, Duration::from_secs(5));

    if result {
        if let Some(request_id) = request_id {
            log::log_with_fields("INFO", "relay", "relay.control", "", &[("action", action), ("target", target_device_short_id), ("request_id", request_id)]);
        } else {
            log::log_with_fields("INFO", "relay", "relay.control", "", &[("action", action), ("target", target_device_short_id)]);
        }
    } else {
        log::log_warn("relay", "relay.network", "control: PUBACK timeout");
    }

    result
}

/// Send an RPC control command using an ephemeral client.
pub(crate) fn send_rpc_control_ephemeral(client: &crate::service::client::Client, config: &AgentxConfig, action: &str, target_device_short_id: &str, request_id: &str, params: &serde_json::Value) -> bool {
    let ephemeral = match super::client::create_ephemeral_client(config) {
        Some(c) => c,
        None => return false,
    };

    let result = send_control_via_ephemeral(client, config, &ephemeral, action, target_device_short_id, Some(request_id), params);

    ephemeral.disconnect();
    result
}

pub(crate) fn send_one_way_control_ephemeral(client: &crate::service::client::Client, config: &AgentxConfig, action: &str, target_device_short_id: &str, params: &serde_json::Value) -> bool {
    let ephemeral = match super::client::create_ephemeral_client(config) {
        Some(c) => c,
        None => return false,
    };

    let result = send_control_via_ephemeral(client, config, &ephemeral, action, target_device_short_id, None, params);

    ephemeral.disconnect();
    result
}

/// Host-side disable: network and config file effects execute at the caller;
/// managed worker state is changed only through the retained generation.
pub(crate) fn disable_local_relay(config: &AgentxConfig, client: &crate::service::client::Client) -> Result<bool, String> {
    let device = super::read_device_uuid_in(client.scope_root()).ok_or("failed to resolve relay device identity")?;
    let cleared = config.relay_enabled && super::client::clear_retained_state_for_device(config, &device);
    crate::config::file::set_for_scope(client.scope_root(), "relay_enabled", "false")?;
    super::persistence::call(client, super::service::Request::ClearWorkerRuntime).map_err(|error| error.to_string())?;
    Ok(cleared)
}

#[cfg(test)]
pub fn get_rpc_result(db: &AgentxDb, request_id: &str) -> Option<Value> {
    let data: String = db
        .conn()
        .query_row(
            "SELECT data FROM events
             WHERE type = 'rpc_result'
             AND json_extract(data, '$.request_id') = ?
             ORDER BY id DESC LIMIT 1",
            rusqlite::params![request_id],
            |row| row.get(0),
        )
        .ok()?;
    serde_json::from_str(&data).ok()
}

/// Backfill has its own durable gap correlation and requires the exact peer
/// and events action; an unrelated imported answer cannot consume that gap.
pub(crate) fn take_rpc_result_from(db: &AgentxDb, request_id: &str, source: &str, action: &str) -> Option<Value> {
    use rusqlite::OptionalExtension;
    let row = db
        .conn()
        .query_row(
            "SELECT id,data FROM events WHERE type='rpc_result' AND json_extract(data,'$.request_id')=? AND json_extract(data,'$.action')=? AND json_extract(data,'$._relay.device')=? ORDER BY id DESC LIMIT 1",
            rusqlite::params![request_id, action, source],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .ok()??;
    let value = serde_json::from_str(&row.1).ok()?;
    db.conn().execute("DELETE FROM events WHERE id=?", [row.0]).ok()?;
    Some(value)
}

pub(crate) fn wait_for_rpc_result(client: &crate::service::client::Client, request_id: &str, timeout: Duration) -> Result<Value, String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let super::service::Response::RpcResult(Some(result)) = super::persistence::call(client, super::service::Request::RpcResult { request_id: request_id.into(), consume: true }).map_err(|error| error.to_string())? {
            return Ok(result);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!("timed out waiting for rpc_result {request_id}; remote outcome is unknown, do not retry a mutating action"))
}

/// Send a peer request and wait through one retained-generation local client.
/// Only read-only screen requests have automatic network retry attempts.
pub(crate) fn send_rpc_request_and_wait(client: &crate::service::client::Client, config: &AgentxConfig, action: &str, target_device_short_id: &str, target_name: Option<&str>, params: &serde_json::Value, timeout: Duration) -> Result<Value, String> {
    if !super::worker::ensure_worker() {
        if crate::sys::process::is_sandboxed() {
            return Err("relay worker not running and can't be started from inside a sandbox - rerun with approval/escalation".to_string());
        }
        return Err("relay worker not running - start with: agentx relay on".to_string());
    }
    ensure_remote_action_supported(client, target_device_short_id, action, target_name)?;

    let max_attempts = if action == rpc_action::TERM_SCREEN { 5 } else { 1 };
    for attempt in 0..max_attempts {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(500));
        }

        let request_id = uuid::Uuid::new_v4().to_string();
        super::persistence::call(client, super::service::Request::ExpectRpc { request_id: request_id.clone(), short_id: target_device_short_id.into(), action: action.into() }).map_err(|error| error.to_string())?;
        if !send_rpc_control_ephemeral(client, config, action, target_device_short_id, &request_id, params) {
            if attempt + 1 == max_attempts {
                return Err(format!("publication acknowledgement missing for {action}; remote outcome is unknown, reconcile before retry"));
            }
            continue;
        }

        match wait_for_rpc_result(client, &request_id, timeout) {
            Ok(result) => return Ok(result),
            Err(e) if action == rpc_action::TERM_SCREEN && attempt + 1 < max_attempts && e.starts_with("timed out waiting for rpc_result") => {
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Err(format!("failed to send {} request", action))
}

pub fn require_successful_rpc_result(response: Value) -> Result<Value, String> {
    if response.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Ok(response);
    }

    let action = response.get("action").and_then(|v| v.as_str()).unwrap_or("remote command");
    let result = response.get("result").unwrap_or(&Value::Null);
    let detail = result.get("error").and_then(|v| v.as_str()).map(ToString::to_string).filter(|s| !s.is_empty()).unwrap_or_else(|| {
        let rendered = serde_json::to_string(result).unwrap_or_default();
        if rendered.is_empty() || rendered == "null" { "unknown remote error".to_string() } else { rendered }
    });
    Err(format!("{action} failed: {detail}"))
}

/// Send an RPC request and return the raw response envelope
/// (`{ok, action, result}`). Loads config internally.
///
/// Prefer [`dispatch_remote`] unless the caller needs to inspect custom
/// fields on a failed response (e.g. kill's `permission_denied` path).
pub(crate) fn dispatch_remote_raw(client: &crate::service::client::Client, device_short_id: &str, target_name: Option<&str>, action: &str, params: &Value, timeout: Duration) -> Result<Value, String> {
    let config = AgentxConfig::load_for_scope(client.scope_root(), &client.invocation().environment);
    send_rpc_request_and_wait(client, &config, action, device_short_id, target_name, params, timeout)
}

/// Send an RPC request, require success, and return the unwrapped inner
/// `result` field. This is the default for CLI callers: on success you get
/// the handler's output value; on failure you get a user-facing error string.
pub(crate) fn dispatch_remote(client: &crate::service::client::Client, device_short_id: &str, target_name: Option<&str>, action: &str, params: &Value, timeout: Duration) -> Result<Value, String> {
    let response = dispatch_remote_raw(client, device_short_id, target_name, action, params, timeout)?;
    let response = require_successful_rpc_result(response)?;
    let mut result = response.get("result").cloned().unwrap_or(Value::Null);
    // The relay cut this answer to fit a publish after the peer built it. Keep
    // its marker on the result so callers can say so instead of passing cut
    // fields along as whole.
    if let Some(marker) = response.get(super::push::SHRINK_MARKER)
        && let Some(obj) = result.as_object_mut()
    {
        obj.insert(super::push::SHRINK_MARKER.into(), marker.clone());
    }
    Ok(result)
}

/// Dispatch a remote RPC and print the result field to stdout.
/// Returns the CLI exit code (0 on success, 1 on error).
/// `result_key` is the JSON key in the result to extract (e.g. "content", "message").
#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_remote_and_print(client: &crate::service::client::Client, device_short_id: &str, target_name: Option<&str>, action: &str, params: &Value, timeout: Duration, result_key: &str, fallback_msg: &str) -> i32 {
    match dispatch_remote(client, device_short_id, target_name, action, params, timeout) {
        Ok(inner) => {
            println!("{}", inner[result_key].as_str().unwrap_or(fallback_msg));
            0
        }
        Err(e) => {
            eprintln!("Remote {action} failed: {e}");
            1
        }
    }
}

/// Default timeout for remote RPC commands (kill, resume, term, transcript, config).
pub const RPC_DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Longer timeout for remote launch, which may need to pull a model and start a process.
pub const RPC_LAUNCH_TIMEOUT: Duration = Duration::from_secs(15);

/// Remote resume waits on a real relaunch on the target device, so it can
/// legitimately take longer than the generic term/config/kill RPC budget.
pub const RPC_RESUME_TIMEOUT: Duration = Duration::from_secs(30);

/// Split a target name like "luna:ABCD" into ("luna", "ABCD").
/// Only returns Some when the suffix is exactly 4 uppercase alphanumeric characters —
/// the format used for device short IDs. This prevents plain colons in agent names
/// with non-device suffixes from being misidentified as remote targets.
pub fn split_device_suffix(name: &str) -> Option<(&str, &str)> {
    let (base, suffix) = name.rsplit_once(':')?;
    if suffix.len() == 4 && suffix.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) { Some((base, suffix)) } else { None }
}

/// RPC action name constants — use these instead of raw string literals.
pub mod rpc_action {
    pub const LAUNCH: &str = "launch";
    pub const LAUNCH_PROFILE: &str = "launch-profile";
    pub const LAUNCH_PREVIEW: &str = "launch-preview";
    pub const KILL: &str = "kill";
    pub const RESUME: &str = "resume";
    pub const CONFIG_GET: &str = "config_get";
    pub const CONFIG_SET: &str = "config_set";
    pub const RELAY_OFF: &str = "relay_off";
    pub const TERM_SCREEN: &str = "term_screen";
    pub const TERM_INJECT: &str = "term_inject";
    pub const TRANSCRIPT: &str = "transcript";
    pub const EVENTS: &str = "events";
    pub const SUB_CREATE: &str = "sub_create";
    pub const SUB_LIST: &str = "sub_list";
    pub const SUB_UNSUB: &str = "sub_unsub";
}

pub fn advertised_remote_capabilities() -> Vec<&'static str> {
    vec![
        rpc_action::LAUNCH,
        rpc_action::LAUNCH_PROFILE,
        rpc_action::LAUNCH_PREVIEW,
        rpc_action::KILL,
        rpc_action::RESUME,
        rpc_action::CONFIG_GET,
        rpc_action::CONFIG_SET,
        rpc_action::RELAY_OFF,
        rpc_action::TERM_SCREEN,
        rpc_action::TERM_INJECT,
        rpc_action::TRANSCRIPT,
        rpc_action::EVENTS,
        rpc_action::SUB_CREATE,
        rpc_action::SUB_LIST,
        rpc_action::SUB_UNSUB,
    ]
}

/// Peers whose last relay sync is older than this are considered offline.
/// Must match REMOTE_DEVICE_STALE_THRESHOLD in instance_lifecycle.rs.
const PEER_STALE_THRESHOLD_SECS: f64 = 90.0;

/// Three distinct states a remote peer's capability advertisement can be in.
/// Keeping these separate is load-bearing for mixed-version relays:
/// a peer that predates the capability field (`Legacy`) must NOT be
/// hard-blocked like one that explicitly advertises `[]`.
use super::control_service::PeerCapabilities as CachedCapabilities;

pub(super) fn read_remote_capabilities(db: &AgentxDb, target_device_short_id: &str) -> Result<CachedCapabilities, String> {
    let Some(device_id) = safe_kv_get(db, &format!("relay_short_{}", target_device_short_id)) else {
        return Ok(CachedCapabilities::NotSynced);
    };

    // Check freshness before trusting cached capabilities.
    let sync_time = safe_kv_get(db, &format!("relay_sync_time_{}", device_id)).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    if sync_time > 0.0 {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
        let age = now - sync_time;
        if age > PEER_STALE_THRESHOLD_SECS {
            return Ok(CachedCapabilities::Stale(age as u64));
        }
    }

    let Some(raw) = safe_kv_get(db, &format!("relay_caps_{}", device_id)) else {
        return Ok(CachedCapabilities::NotSynced);
    };
    if raw == "null" {
        return Ok(CachedCapabilities::Legacy);
    }
    let parsed = serde_json::from_str::<Vec<String>>(&raw).map_err(|e| format!("failed to parse remote capabilities for {target_device_short_id}: {e}"))?;
    Ok(CachedCapabilities::Advertised(parsed))
}

#[cfg(test)]
fn check_remote_action_for_db(db: &AgentxDb, target_device_short_id: &str, action: &str, target_name: Option<&str>) -> Result<(), String> {
    check_remote_action(read_remote_capabilities(db, target_device_short_id)?, target_device_short_id, action, target_name)
}
fn check_remote_action(capabilities: CachedCapabilities, target_device_short_id: &str, action: &str, target_name: Option<&str>) -> Result<(), String> {
    let detail = target_name.map(|n| format!(" (target {})", n)).unwrap_or_default();
    match capabilities {
        CachedCapabilities::Advertised(capabilities) => {
            if capabilities.iter().any(|cap| cap == action) {
                Ok(())
            } else {
                Err(format!(
                    "device {target_device_short_id}{detail} does not advertise remote action '{action}' — peer may be running an older binary, or its relay worker may need a restart to pick up newly-installed capabilities (agentx relay off && agentx relay on on the peer)."
                ))
            }
        }
        // Pre-capability peer: forward the request optimistically. If the
        // peer can't handle it, the RPC wait will time out and surface a
        // clear "timed out waiting for rpc_result" error.
        CachedCapabilities::Legacy => Ok(()),
        CachedCapabilities::Stale(age_secs) => {
            let age_str = crate::shared::time::format_age(age_secs as i64);
            Err(format!("device {target_device_short_id}{detail} is offline (last seen {age_str} ago) — is the remote relay worker running?"))
        }
        CachedCapabilities::NotSynced => Err(format!("device {target_device_short_id}{detail} has not yet synced remote capabilities — try again in a few seconds")),
    }
}

/// Whether a peer can take `action` right now, without waiting: `Some(true)`
/// yes, `Some(false)` never (it advertises capabilities without the action),
/// `None` not yet (offline, or its capabilities have not synced).
pub(crate) fn peer_accepts_action(db: &AgentxDb, target_device_short_id: &str, action: &str) -> Option<bool> {
    match read_remote_capabilities(db, target_device_short_id) {
        Ok(CachedCapabilities::Advertised(capabilities)) => Some(capabilities.iter().any(|cap| cap == action)),
        Ok(CachedCapabilities::Legacy) => Some(true),
        Ok(CachedCapabilities::Stale(_)) | Ok(CachedCapabilities::NotSynced) | Err(_) => None,
    }
}

fn ensure_remote_action_supported(client: &crate::service::client::Client, target_device_short_id: &str, action: &str, target_name: Option<&str>) -> Result<(), String> {
    for attempt in 0..3 {
        let super::service::Response::Capabilities(capabilities) = super::persistence::call(client, super::service::Request::Capabilities { short_id: target_device_short_id.into() }).map_err(|error| error.to_string())? else {
            return Err("unexpected relay capability response".into());
        };
        if capabilities == CachedCapabilities::NotSynced && attempt < 2 {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        return check_remote_action(capabilities, target_device_short_id, action, target_name);
    }
    unreachable!()
}

#[cfg(test)]
use super::control_service::{RemoteLaunch as RemoteLaunchRequest, RemoteResume as RemoteResumeRequest};
#[cfg(test)]
fn handle_control_events(db: &AgentxDb, events: &[Value], own_short: &str, source: &str) -> bool {
    super::control_store::accept(db, &super::control_store::test_context(), "fixture-relay", events, own_short, source, "fixture-device").unwrap().changed
}
#[cfg(test)]
fn resolve_remote_cwd(requested: Option<&str>) -> Result<String, String> {
    super::control_host::resolve_remote_cwd(requested)
}
#[cfg(test)]
struct PreparedRemoteLaunch {
    args: Vec<String>,
    background: bool,
}
#[cfg(test)]
fn prepare_remote_launch(request: &RemoteLaunchRequest, config: &AgentxConfig) -> PreparedRemoteLaunch {
    let tool = launcher::LaunchTool::from_str(request.tool.as_deref().unwrap()).unwrap();
    let (args, background) = launcher::prepare_launch_execution(&tool, &request.args, config, request.background);
    PreparedRemoteLaunch { args, background }
}
#[cfg(test)]
fn handle_remote_config_get(_db: &AgentxDb, params: &Value, _initiator: &str, _config: &AgentxConfig) -> Result<Value, String> {
    super::control_parse::parse(rpc_action::CONFIG_GET, params)?;
    unreachable!("these retained cases only check secret rejection")
}
#[cfg(test)]
fn handle_remote_config_set(_db: &AgentxDb, params: &Value, _initiator: &str, _config: &AgentxConfig) -> Result<Value, String> {
    super::control_parse::parse(rpc_action::CONFIG_SET, params)?;
    unreachable!("these retained cases only check secret rejection")
}
#[cfg(test)]
fn handle_remote_events(db: &AgentxDb, params: &Value, _initiator: &str, _config: &AgentxConfig) -> Result<Value, String> {
    super::control_store::test_action(db, params, rpc_action::EVENTS)
}
#[cfg(test)]
fn remote_events_byte_cap(db: &AgentxDb) -> usize {
    super::control_store::remote_events_byte_cap(db, "fixture-device")
}
#[cfg(test)]
const CONTROL_DEDUP_WINDOW_SECS: f64 = 300.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::hooks::test_helpers::isolated_test_env;
    use serde_json::json;

    /// A live peer on an older agentx holds the publish budget at 112 KiB.
    fn with_legacy_peer(db: &AgentxDb) {
        crate::relay::safe_kv_set(db, "relay_sync_time_legacy-peer", Some(&crate::shared::time::now_epoch_f64().to_string()));
    }

    fn test_db() -> AgentxDb {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = AgentxDb::open_raw(&db_path).unwrap();
        db.init_db().unwrap();
        std::mem::forget(dir);
        db
    }

    fn latest_rpc_result(db: &AgentxDb) -> serde_json::Value {
        let payload: String = db.conn().query_row("SELECT data FROM events WHERE type = 'rpc_result' ORDER BY id DESC LIMIT 1", [], |row| row.get(0)).unwrap();
        serde_json::from_str(&payload).unwrap()
    }

    #[test]
    fn test_split_device_suffix_valid() {
        assert_eq!(split_device_suffix("luna:ABCD"), Some(("luna", "ABCD")));
        assert_eq!(split_device_suffix("myagent:XY12"), Some(("myagent", "XY12")));
        assert_eq!(split_device_suffix("a:B3C4"), Some(("a", "B3C4")));
        // All digits
        assert_eq!(split_device_suffix("foo:1234"), Some(("foo", "1234")));
    }

    #[test]
    fn test_split_device_suffix_invalid() {
        // Lowercase in suffix
        assert_eq!(split_device_suffix("luna:abcd"), None);
        // Mixed case
        assert_eq!(split_device_suffix("luna:ABCd"), None);
        // Too short
        assert_eq!(split_device_suffix("luna:ABC"), None);
        // Too long
        assert_eq!(split_device_suffix("luna:ABCDE"), None);
        // No colon
        assert_eq!(split_device_suffix("luna"), None);
        assert_eq!(split_device_suffix("agent:something"), None);
        // Hyphen in suffix
        assert_eq!(split_device_suffix("foo:AB-D"), None);
        // Empty base
        assert_eq!(split_device_suffix(":ABCD"), Some(("", "ABCD")));
    }

    #[test]
    fn test_handle_control_events_filters_by_target() {
        // Control events targeting a different device should be ignored
        let events = vec![json!({
            "type": "control",
            "ts": crate::shared::time::now_epoch_f64(),
            "data": {
                "action": "kill",
                "target_device": "ABCD",
                "from": "_:EFGH",
                "request_id": "req-other-device",
                "params": {
                    "target": "luna",
                }
            }
        })];

        // own_short_id is "WXYZ" — event targets "ABCD", so nothing should happen
        let db = test_db();
        assert!(!handle_control_events(&db, &events, "WXYZ", "device-123"));

        // No crash, no panic — event was filtered
    }

    #[test]
    fn test_resolve_remote_cwd_rejects_missing_requested_directory() {
        let err = resolve_remote_cwd(Some("/definitely/missing")).unwrap_err();
        assert!(err.contains("requested cwd does not exist or is not a directory"));
    }

    #[test]
    fn test_resolve_remote_cwd_rejects_empty() {
        let err = resolve_remote_cwd(None).unwrap_err();
        assert!(err.contains("--dir"));
        let err = resolve_remote_cwd(Some("")).unwrap_err();
        assert!(err.contains("--dir"));
    }

    #[test]
    #[serial_test::serial]
    fn test_build_rpc_control_payload_includes_request_id_and_params() {
        let (_dir, _agentx_dir, _home, _guard) = isolated_test_env();
        let psk = [0x55u8; 32];
        let config = AgentxConfig { relay_id: "relay-1".to_string(), relay_psk: super::super::encode_psk(&psk), ..Default::default() };
        let (topic, sealed) = build_control_payload_for_device(&config, "launch", "WXYZ", Some("req-launch"), &json!({"tool": "claude", "count": 1}), "fixture-device", "ABCD").expect("payload");
        // Build path now produces a sealed envelope; opening with the same PSK
        // returns the original JSON.
        let plaintext = super::super::crypto::open(&psk, &config.relay_id, &topic, &sealed).expect("open");
        let parsed: Value = serde_json::from_slice(&plaintext).unwrap();
        let data = &parsed["events"][0]["data"];
        assert_eq!(data["action"], "launch");
        assert_eq!(data["target_device"], "WXYZ");
        assert_eq!(data["request_id"], "req-launch");
        assert_eq!(data["params"]["tool"], "claude");
    }

    #[test]
    fn test_remote_launch_request_from_params_defaults_optional_fields() {
        let request = RemoteLaunchRequest::from_params(&json!({"tool": "claude", "count": 2})).unwrap();
        assert_eq!(request.tool.as_deref(), Some("claude"));
        assert_eq!(request.count, 2);
        assert!(request.args.is_empty());
        assert_eq!(request.launcher, None);
        assert_eq!(request.system_prompt, None);
        assert_eq!(request.initial_prompt, None);
        assert!(!request.background);
        assert_eq!(request.terminal, None);
        assert_eq!(request.cwd, None);
    }

    #[test]
    fn remote_launch_rejects_unknown_parameters() {
        let error = RemoteLaunchRequest::from_params(&json!({"tool": "claude", "unexpected": "value"})).unwrap_err();
        assert!(error.contains("unknown launch parameter 'unexpected'"));
    }

    #[test]
    fn test_remote_launch_request_from_params_collects_terminal() {
        let request = RemoteLaunchRequest::from_params(&json!({
            "tool": "codex",
            "count": 1,
            "terminal": "kitty-tab",
            "launcher": "rega",
        }))
        .unwrap();
        assert_eq!(request.terminal.as_deref(), Some("kitty-tab"));
        assert_eq!(request.launcher.as_deref(), Some("rega"));
    }

    #[test]
    fn test_prepare_remote_launch_supports_interactive() {
        let request = RemoteLaunchRequest::from_params(&json!({
            "tool": "codex",
            "count": 1,
            "args": ["--model", "gpt-5.4"],
            "background": false,
        }))
        .unwrap();
        let prepared = prepare_remote_launch(&request, &AgentxConfig::default());
        assert!(!prepared.background);
        assert_eq!(prepared.args, vec!["--model", "gpt-5.4"]);
    }

    #[test]
    fn test_prepare_remote_launch_keeps_background_detection() {
        let request = RemoteLaunchRequest::from_params(&json!({
            "tool": "claude",
            "count": 1,
            "args": ["-p"],
            "background": false,
        }))
        .unwrap();
        let prepared = prepare_remote_launch(&request, &AgentxConfig::default());
        assert!(prepared.background);
    }

    #[test]
    fn test_prepare_remote_launch_claude_print_normalizes() {
        // Remote `claude -p` (explicit print mode) must go through the same
        // print-mode normalization as the local path.
        let request = RemoteLaunchRequest::from_params(&json!({
            "tool": "claude",
            "count": 1,
            "args": ["-p"],
            "background": true,
            "initial_prompt": "say hi in agentx",
        }))
        .unwrap();
        let prepared = prepare_remote_launch(&request, &AgentxConfig::default());
        assert!(prepared.background);
        assert!(prepared.args.iter().any(|arg| arg == "-p"));
        assert!(prepared.args.windows(2).any(|w| w == ["--output-format", "stream-json"]));
        assert!(prepared.args.iter().any(|arg| arg == "--verbose"));
    }

    #[test]
    fn test_prepare_remote_launch_claude_headless_stays_pty() {
        // Bare remote `claude --headless` (no -p) is the live PTY session — no -p
        // is injected, no print-mode defaults, matching the local path.
        let request = RemoteLaunchRequest::from_params(&json!({
            "tool": "claude",
            "count": 1,
            "args": [],
            "background": true,
            "initial_prompt": "say hi in agentx",
        }))
        .unwrap();
        let prepared = prepare_remote_launch(&request, &AgentxConfig::default());
        assert!(prepared.background);
        assert!(!prepared.args.iter().any(|arg| matches!(arg.as_str(), "-p" | "--print")));
    }

    #[test]
    fn test_remote_resume_request_from_params_collects_extra_args() {
        let request = RemoteResumeRequest::from_params(&json!({
            "target": "luna",
            "fork": true,
            "extra_args": ["--terminal", "kitty", "--model", "opus"],
            "launcher": "rega",
        }))
        .unwrap();
        assert_eq!(request.target, "luna");
        assert!(request.fork);
        assert_eq!(request.launcher.as_deref(), Some("rega"));
        assert_eq!(request.extra_args, vec!["--terminal", "kitty", "--model", "opus"]);
    }

    #[test]
    fn test_handle_control_events_kill_emits_rpc_result() {
        let db = test_db();
        let events = vec![json!({
            "type": "control",
            "ts": crate::shared::time::now_epoch_f64(),
            "data": {
                "action": "kill",
                "target_device": "WXYZ",
                "from": "_:EFGH",
                "request_id": "req-kill",
                "params": {
                    "target": "missing-agent",
                }
            }
        })];

        assert!(handle_control_events(&db, &events, "WXYZ", "device-123"));
        let result = latest_rpc_result(&db);
        assert_eq!(result["request_id"], "req-kill");
        assert_eq!(result["action"], "kill");
        assert_eq!(result["ok"], false);
    }

    fn kill_event(ts: f64, target_device: &str, request_id: &str) -> Value {
        json!({
            "type": "control",
            "ts": ts,
            "data": {
                "action": "kill",
                "target_device": target_device,
                "from": "_:EFGH",
                "request_id": request_id,
                "params": { "target": "missing-agent" },
            }
        })
    }

    fn rpc_result_count(db: &AgentxDb) -> i64 {
        db.conn().query_row("SELECT COUNT(*) FROM events WHERE type = 'rpc_result'", [], |row| row.get(0)).unwrap()
    }

    #[test]
    fn test_handle_control_events_accepts_out_of_order_arrivals() {
        let db = test_db();
        let now = crate::shared::time::now_epoch_f64();
        // A later event for another device, then a later one for us, arrive first.
        let first = [kill_event(now, "ABCD", "req-a"), kill_event(now - 1.0, "WXYZ", "req-b")];
        assert!(handle_control_events(&db, &first, "WXYZ", "device-123"));
        let late = [kill_event(now - 2.0, "WXYZ", "req-c")];
        assert!(handle_control_events(&db, &late, "WXYZ", "device-123"));
        assert_eq!(rpc_result_count(&db), 2);
    }

    #[test]
    fn test_handle_control_events_dedup_survives_persisting_a_fractional_ts() {
        let db = test_db();
        let now = crate::shared::time::now_epoch_f64().floor();
        // Parsed from wire text, as a received envelope would be.
        let wire = format!(r#"[{{"type":"control","ts":{now}.0000022,"data":{{"action":"kill","target_device":"WXYZ","from":"_:EFGH","request_id":"req-f","params":{{"target":"missing-agent"}}}}}}]"#);
        let events: Vec<Value> = serde_json::from_str(&wire).unwrap();
        assert!(handle_control_events(&db, &events, "WXYZ", "device-123"));
        let again: Vec<Value> = serde_json::from_str(&wire).unwrap();
        assert!(!handle_control_events(&db, &again, "WXYZ", "device-123"));
        assert_eq!(rpc_result_count(&db), 1);
    }

    #[test]
    fn test_handle_control_events_drops_redelivery_and_stale_events() {
        let db = test_db();
        let now = crate::shared::time::now_epoch_f64();
        let event = [kill_event(now, "WXYZ", "req-once")];
        assert!(handle_control_events(&db, &event, "WXYZ", "device-123"));
        assert!(!handle_control_events(&db, &event, "WXYZ", "device-123"));
        let stale = [kill_event(now - CONTROL_DEDUP_WINDOW_SECS - 1.0, "WXYZ", "req-old")];
        assert!(!handle_control_events(&db, &stale, "WXYZ", "device-123"));
        assert_eq!(rpc_result_count(&db), 1);
    }

    #[test]
    fn test_handle_control_events_ignores_unknown_actions() {
        let db = test_db();
        let events = vec![json!({
            "type": "control",
            "ts": crate::shared::time::now_epoch_f64(),
            "data": {
                "action": "mystery",
                "target_device": "WXYZ",
                "from": "_:EFGH",
                "request_id": "req-unknown",
                "params": {},
            }
        })];

        assert!(!handle_control_events(&db, &events, "WXYZ", "device-123"));
        let count: i64 = db.conn().query_row("SELECT COUNT(*) FROM events WHERE type = 'rpc_result'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn relay_off_is_a_host_plan_without_daemon_file_or_network_effects() {
        let parsed = super::super::control_parse::parse(rpc_action::RELAY_OFF, &json!({})).unwrap();
        assert!(matches!(parsed, super::super::control_parse::Action::Host(super::super::control_service::HostAction::RelayOff)));
    }

    #[test]
    fn test_require_successful_rpc_result_returns_error_for_failed_response() {
        let err = require_successful_rpc_result(json!({
            "action": "resume",
            "ok": false,
            "result": { "error": "boom" }
        }))
        .unwrap_err();
        assert_eq!(err, "resume failed: boom");
    }

    #[test]
    fn test_check_remote_action_for_db_accepts_advertised_action() {
        let db = test_db();
        safe_kv_set(&db, "relay_short_WXYZ", Some("device-123"));
        safe_kv_set(&db, "relay_caps_device-123", Some(r#"["launch","resume","config_get"]"#));

        assert!(check_remote_action_for_db(&db, "WXYZ", "resume", None).is_ok());
    }

    #[test]
    fn test_check_remote_action_for_db_rejects_unadvertised_action() {
        let db = test_db();
        safe_kv_set(&db, "relay_short_WXYZ", Some("device-123"));
        safe_kv_set(&db, "relay_caps_device-123", Some(r#"["launch"]"#));

        let err = check_remote_action_for_db(&db, "WXYZ", "resume", None).unwrap_err();
        assert!(err.starts_with("device WXYZ does not advertise remote action 'resume'"), "unexpected err: {err}");
        assert!(err.contains("agentx relay off"), "missing restart hint: {err}");
    }

    #[test]
    fn test_check_remote_action_for_db_rejects_unadvertised_action_with_target_name() {
        let db = test_db();
        safe_kv_set(&db, "relay_short_WXYZ", Some("device-123"));
        safe_kv_set(&db, "relay_caps_device-123", Some(r#"["launch"]"#));

        let err = check_remote_action_for_db(&db, "WXYZ", "kill", Some("luna:WXYZ")).unwrap_err();
        assert!(err.starts_with("device WXYZ (target luna:WXYZ) does not advertise remote action 'kill'"), "unexpected err: {err}");
    }

    #[test]
    fn test_check_remote_action_for_db_rejects_missing_capabilities() {
        let db = test_db();
        safe_kv_set(&db, "relay_short_WXYZ", Some("device-123"));

        let err = check_remote_action_for_db(&db, "WXYZ", "resume", None).unwrap_err();
        assert!(err.contains("has not yet synced remote capabilities"), "unexpected error: {err}");
    }

    #[test]
    fn test_check_remote_action_for_db_accepts_legacy_peer() {
        // Pre-capability peers (pull.rs stores "null" when the state message
        // carries no `capabilities` field) must be allowed through. Blocking
        // them here would break rolling upgrades: the user would see
        // "does not advertise remote action 'launch'" against a peer that
        // simply predates the capability vocabulary.
        let db = test_db();
        safe_kv_set(&db, "relay_short_WXYZ", Some("device-123"));
        safe_kv_set(&db, "relay_caps_device-123", Some("null"));

        assert_eq!(read_remote_capabilities(&db, "WXYZ").unwrap(), CachedCapabilities::Legacy);
        assert!(check_remote_action_for_db(&db, "WXYZ", "launch", None).is_ok());
        assert!(check_remote_action_for_db(&db, "WXYZ", "kill", None).is_ok());
    }

    #[test]
    fn test_check_remote_action_for_db_rejects_explicit_empty_capabilities() {
        // A modern peer that explicitly advertises an empty list should still
        // be hard-blocked. The legacy carve-out only applies when the field is
        // MISSING.
        let db = test_db();
        safe_kv_set(&db, "relay_short_WXYZ", Some("device-123"));
        safe_kv_set(&db, "relay_caps_device-123", Some("[]"));

        assert_eq!(read_remote_capabilities(&db, "WXYZ").unwrap(), CachedCapabilities::Advertised(Vec::new()));
        let err = check_remote_action_for_db(&db, "WXYZ", "launch", None).unwrap_err();
        assert!(err.contains("does not advertise remote action"), "unexpected error: {err}");
    }

    #[test]
    fn test_advertised_remote_capabilities_lists_all_handlers() {
        assert_eq!(
            advertised_remote_capabilities(),
            vec!["launch", "launch-profile", "launch-preview", "kill", "resume", "config_get", "config_set", "relay_off", "term_screen", "term_inject", "transcript", "events", "sub_create", "sub_list", "sub_unsub",]
        );
    }

    #[test]
    fn test_handle_remote_config_get_blocks_relay_psk() {
        let db = test_db();
        let err = handle_remote_config_get(&db, &json!({"fields": ["relay_psk"]}), "initiator", &AgentxConfig::default()).unwrap_err();
        assert_eq!(err, "relay_psk is not remotely queryable");
    }

    #[test]
    fn test_handle_remote_config_get_blocks_relay_psk_for_instance_mode() {
        let db = test_db();
        let err = handle_remote_config_get(&db, &json!({"instance": "luna", "field": "relay_psk"}), "initiator", &AgentxConfig::default()).unwrap_err();
        assert_eq!(err, "relay_psk is not remotely queryable");
    }

    #[test]
    fn test_handle_remote_config_set_blocks_relay_psk() {
        let db = test_db();
        let err = handle_remote_config_set(&db, &json!({"field": "relay_psk", "value": "secret"}), "initiator", &AgentxConfig::default()).unwrap_err();
        assert_eq!(err, "relay_psk is not remotely queryable");
    }

    #[test]
    fn test_handle_remote_config_set_blocks_relay_psk_for_instance_mode() {
        let db = test_db();
        let err = handle_remote_config_set(&db, &json!({"instance": "luna", "field": "relay_psk", "value": "secret"}), "initiator", &AgentxConfig::default()).unwrap_err();
        assert_eq!(err, "relay_psk is not remotely queryable");
    }

    fn seed_events(db: &AgentxDb, count: usize) {
        for i in 0..count {
            let etype = if i % 2 == 0 { "message" } else { "status" };
            db.log_event(etype, "luna", &json!({"i": i})).unwrap();
        }
    }

    #[test]
    fn test_handle_remote_events_empty_filters_returns_all() {
        let db = test_db();
        seed_events(&db, 5);
        let out = handle_remote_events(&db, &json!({}), "initiator", &AgentxConfig::default()).unwrap();
        assert_eq!(out["count"].as_u64().unwrap(), 5);
        let events = out["events"].as_array().unwrap();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn test_handle_remote_events_type_message_filter() {
        let db = test_db();
        seed_events(&db, 6);
        let out = handle_remote_events(&db, &json!({"filters": {"type": ["message"]}, "last": 50}), "initiator", &AgentxConfig::default()).unwrap();
        let events = out["events"].as_array().unwrap();
        assert_eq!(events.len(), 3);
        for e in events {
            assert_eq!(e["type"].as_str().unwrap(), "message");
        }
    }

    #[test]
    fn test_handle_remote_events_hard_cap() {
        let db = test_db();
        seed_events(&db, 10);
        let out = handle_remote_events(&db, &json!({"last": 9999}), "initiator", &AgentxConfig::default()).unwrap();
        assert_eq!(out["count"].as_u64().unwrap(), 10);
        let out = handle_remote_events(&db, &json!({"last": 3}), "initiator", &AgentxConfig::default()).unwrap();
        assert_eq!(out["count"].as_u64().unwrap(), 3);
    }

    #[test]
    fn test_handle_remote_events_missing_filters_ok() {
        let db = test_db();
        seed_events(&db, 2);
        let out = handle_remote_events(&db, &json!({"last": 10}), "initiator", &AgentxConfig::default()).unwrap();
        assert_eq!(out["count"].as_u64().unwrap(), 2);
    }

    #[test]
    fn test_handle_remote_events_truncates_when_envelope_exceeds_cap() {
        let db = test_db();
        with_legacy_peer(&db);
        // Big payload per event so a few rows blow past the answer budget.
        let big = "x".repeat(8_000);
        for _ in 0..20 {
            db.log_event("message", "luna", &json!({"blob": big})).unwrap();
        }
        let input_count = 20usize;
        let out = handle_remote_events(&db, &json!({"last": input_count}), "initiator", &AgentxConfig::default()).unwrap();
        assert_eq!(out["truncated"].as_bool(), Some(true));
        let returned = out["events"].as_array().unwrap().len();
        assert!(returned < input_count, "expected truncation, got {returned}");
        let envelope_len = serde_json::to_string(&out).unwrap().len();
        assert!(envelope_len <= remote_events_byte_cap(&db), "envelope {envelope_len} bytes exceeds cap");
    }

    #[test]
    fn test_handle_remote_events_honours_a_smaller_byte_budget() {
        let db = test_db();
        with_legacy_peer(&db);
        let big = "x".repeat(8_000);
        for _ in 0..20 {
            db.log_event("message", "luna", &json!({"blob": big})).unwrap();
        }
        let out = handle_remote_events(&db, &json!({"last": 20, "max_bytes": 32 * 1024}), "initiator", &AgentxConfig::default()).unwrap();
        assert_eq!(out["truncated"].as_bool(), Some(true));
        let envelope_len = serde_json::to_string(&out).unwrap().len();
        assert!(envelope_len <= 32 * 1024, "envelope {envelope_len} bytes");
        // Newest first, oldest dropped: the survivors are the newest rows.
        let ids: Vec<i64> = out["events"].as_array().unwrap().iter().map(|e| e["id"].as_i64().unwrap()).collect();
        assert!(ids.windows(2).all(|w| w[0] > w[1]), "{ids:?}");

        // A budget above what fits a publish is clamped to it.
        let out = handle_remote_events(&db, &json!({"last": 20, "max_bytes": 10_000_000}), "initiator", &AgentxConfig::default()).unwrap();
        assert!(serde_json::to_string(&out).unwrap().len() <= remote_events_byte_cap(&db));
    }

    #[test]
    fn test_handle_remote_events_sends_a_cut_copy_of_one_event_larger_than_the_budget() {
        // ~150 KB: over the widest answer a 112 KiB publish leaves room for.
        let db = test_db();
        with_legacy_peer(&db);
        db.log_event("message", "luna", &json!({"from": "luna", "text": "x".repeat(150_000)})).unwrap();
        let out = handle_remote_events(&db, &json!({"last": 1, "max_bytes": remote_events_byte_cap(&db)}), "initiator", &AgentxConfig::default()).unwrap();
        let events = out["events"].as_array().unwrap();
        assert_eq!(events.len(), 1, "a cut copy, not an empty answer");
        assert_eq!(out["truncated"].as_bool(), Some(true));
        assert_eq!(events[0]["data"]["from"], "luna");
        assert_eq!(events[0]["data"]["_relay_truncated"]["level"], 1);
        assert!(serde_json::to_string(&out).unwrap().len() <= remote_events_byte_cap(&db));
    }

    #[test]
    fn test_handle_remote_events_below_the_widest_budget_never_cuts_an_event() {
        // 50 KB: over the 32 KiB normal backfill budget, well under the widest one. A cut
        // copy here would be imported and the intact event never fetched; the empty,
        // truncated answer is what sends backfill to the wide request.
        let db = test_db();
        with_legacy_peer(&db);
        db.log_event("message", "luna", &json!({"from": "luna", "text": "y".repeat(50_000)})).unwrap();
        let narrow = handle_remote_events(&db, &json!({"last": 1, "max_bytes": 32 * 1024}), "initiator", &AgentxConfig::default()).unwrap();
        assert_eq!(narrow["events"].as_array().unwrap().len(), 0);
        assert_eq!(narrow["truncated"].as_bool(), Some(true));
        let wide = handle_remote_events(&db, &json!({"last": 1, "max_bytes": remote_events_byte_cap(&db)}), "initiator", &AgentxConfig::default()).unwrap();
        let events = wide["events"].as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0]["data"].get("_relay_truncated").is_none(), "intact");
        assert_eq!(events[0]["data"]["text"].as_str().unwrap().len(), 50_000);
    }

    #[test]
    fn test_handle_remote_events_invalid_sql_returns_err() {
        let db = test_db();
        seed_events(&db, 2);
        let err = handle_remote_events(&db, &json!({"sql": "not a real column = 1"}), "initiator", &AgentxConfig::default()).unwrap_err();
        assert!(err.contains("sql error"), "unexpected err: {err}");
    }
}
