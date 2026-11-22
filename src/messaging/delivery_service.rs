//! Durable policy for a caller-owned PTY delivery loop. Screen observation,
//! timing and terminal/provider I/O remain with the host.
use crate::agents::service::ResolvedActor;
use crate::db::Message;
use crate::runtime::hosts::ProcessRegistration;
use crate::service::ServiceResult;

/// Listening transitions also refresh last_stop. Only reassert the heartbeat
/// early when no TCP presence has been registered or the wall clock regressed.
pub(crate) fn heartbeat_due(actor: &crate::db::InstanceRow, now: i64) -> bool {
    actor.tcp_mode != 1 || !(0..15).contains(&now.saturating_sub(actor.last_stop))
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct DeliverySnapshot {
    pub actor: ResolvedActor,
    pub pending: Vec<Message>,
    pub endpoints: Vec<(String, u16)>,
    pub has_session: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<crate::agents::recovery::Ticket>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum LaunchObservation {
    Ready { cleared_block: bool },
    Blocked { detail: String },
    Failed { reason: String, detail: String },
}

/// Gates can change only the same pending window and may clear only their own
/// context. RecoveredIdle additionally compares the previous status context.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum GateObservation {
    Blocked { cursor: i64, context: String, detail: String },
    Cleared { cursor: i64, context: String },
    RecoveredIdle { previous_context: String },
    CodexTurnCompleted { session: String, turn: String, phase: crate::agents::model_request::RequestPhase, progress_at: i64, status_time: i64, cursor: i64, completed_at_ms: i64 },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) enum Request {
    /// The current explicit process/session binding may have changed. Return a
    /// fresh participant; the host re-registers its child before further writes.
    Poll,
    Recovery {
        registration: ProcessRegistration,
        ticket: crate::agents::recovery::Ticket,
        action: crate::agents::recovery::HostAction,
    },
    ObserveLaunch {
        registration: ProcessRegistration,
        observation: LaunchObservation,
    },
    ObserveGate {
        registration: ProcessRegistration,
        observation: GateObservation,
    },
    Heartbeat {
        registration: ProcessRegistration,
    },
    BindSession {
        registration: ProcessRegistration,
        session: String,
        cwd: std::path::PathBuf,
    },
    Activity {
        registration: ProcessRegistration,
        status: String,
        context: String,
        detail: String,
    },
    Finish {
        registration: ProcessRegistration,
        killed: bool,
        exit_code: Option<i32>,
        output: Option<String>,
    },
    FinalizeLaunchExit {
        registration: ProcessRegistration,
        never_ready: bool,
        elapsed_secs: u64,
        exit_code: i32,
        output: Option<String>,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[expect(clippy::large_enum_variant, reason = "Each delivery call returns one participant snapshot or a change flag as a direct native result")]
pub(crate) enum Response {
    Snapshot(DeliverySnapshot),
    Changed(bool),
}

/// Narrow typed policy client; no database-shaped fallback or mutable cache.
#[derive(Clone)]
pub(crate) struct Client(pub(crate) crate::service::client::Client);

impl Client {
    pub(crate) fn call(&self, request: Request) -> ServiceResult<Response> {
        match self.0.call(crate::service::Request::Messages(super::service::Request::Delivery(request)))? {
            crate::service::Response::Messages(super::service::Response::Delivery(response)) => Ok(response),
            _ => Err(crate::service::ServiceError::Internal("unexpected delivery response".into())),
        }
    }
}
