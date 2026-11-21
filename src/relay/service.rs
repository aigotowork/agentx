//! Native relay persistence operations. MQTT remains outside the application
//! dispatcher; payload records retain the existing authenticated peer protocol.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) use super::control_service::{ControlReply, PeerCapabilities, ReceiveResult};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ExportPage {
    pub state: Value,
    pub events: Vec<Value>,
    pub previous_cursor: i64,
    pub has_more: bool,
    pub sealed_payload_budget: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Envelope {
    pub relay_id: String,
    pub own_device: String,
    pub topic: String,
    pub payload: Vec<u8>,
    pub psk: [u8; 32],
}

/// Read-only local queue and remote-peer projection. Each short mapping is
/// retained so peer notification does not lose historical aliases.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct RelaySnapshot {
    pub queued: i64,
    pub peers: Vec<Peer>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Peer {
    pub device_uuid: String,
    pub short: String,
    pub sync_time: f64,
    /// Base names in the existing namespaced-name query order.
    pub agents: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Request {
    Export { device_uuid: String },
    CommitPush { previous_cursor: i64, through: i64 },
    Receive(Envelope),
    BeginControl { effect_id: String, token: String },
    CompleteControl { effect_id: String, token: String, reply: ControlReply },
    Capabilities { short_id: String },
    Status { configured: bool, enabled: bool, broker: Option<String> },
    Snapshot { own_device: String },
    ClearDeviceState,
    WorkerHeartbeat,
    WorkerStopped,
    ClearWorkerRuntime,
    Backfill { device_uuid: String },
    BackfillPublished { short_id: String, request_id: String, params: Value },
    WorkerStatus { status: String, error: Option<String> },
    BrokerPacketLimit { max_bytes: Option<u32> },
    NotifyPort { port: Option<u16> },
    WorkerPort,
    ShortId { device_uuid: String },
    RpcResult { request_id: String, consume: bool },
    ExpectRpc { request_id: String, short_id: String, action: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Response {
    Export(ExportPage),
    Changed(bool),
    WorkerPort(Option<u16>),
    ShortId(String),
    RpcResult(Option<Value>),
    Backfill { requests: Vec<BackfillRequest>, wake_ports: Vec<u16> },
    Received(ReceiveResult),
    ControlGranted(bool),
    Capabilities(PeerCapabilities),
    Status(super::RelayStatus),
    Snapshot(RelaySnapshot),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct BackfillRequest {
    pub short_id: String,
    pub request_id: String,
    pub params: Value,
}
