//! One outer native protocol connection, qualified roots and actual-mail input.
//! Concrete services and generic stores are supplied by launcher composition.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::agents::attachments::{ActivationTarget, AttachmentOwnerId, AttachmentStore, ParticipantRef};
use crate::messaging::activation::{ActivationPolicy, ActivationRequest, DriverEvent};
use crate::messaging::inbox::{BatchId, OutstandingClaim, PendingInbox};
use crate::runtime::hosts::{HostRegistration, HostService};

use super::configuration::SessionConfiguration;
use super::protocol::{ConnectionHandshake, ProtocolRouter, RpcId};
use super::roots::{RootBinding, RootIntent};
use super::service::{NativeConnection, ServiceIdentity};

mod driver;
mod session;
mod stdio;
#[cfg(target_os = "linux")]
mod terminal;

#[cfg(test)]
mod acceptance_tests;

#[cfg(test)]
pub(crate) use driver::recorded_input_matches;
pub(crate) use stdio::run_with_client;

#[derive(Clone, Debug)]
pub(crate) struct RootLaunch {
    pub explicit_name: Option<String>,
    pub initial_cursor: Option<i64>,
    pub launch_args: Vec<String>,
    pub launch_profile: Option<crate::runtime::launch_profile::LaunchProvenance>,
}

pub(crate) struct BridgePorts<'a> {
    pub attachments: &'a mut dyn AttachmentStore,
    pub inbox: &'a mut dyn PendingInbox,
    pub transport: &'a mut dyn NativeConnection,
    pub hosts: &'a dyn HostService,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThreadState {
    Unknown,
    Idle,
    Active,
    AwaitingApproval,
    AwaitingInput,
    Retired,
}

struct BoundRoot {
    binding: RootBinding,
    state: ThreadState,
    turn_id: Option<String>,
    suspended: bool,
    host: HostRegistration,
}

struct PendingRoot {
    intent: RootIntent,
    participant: ParticipantRef,
}

struct ExternalOperation {
    method: String,
    thread_id: Option<String>,
    mutation: bool,
    root: Option<PendingRoot>,
}

struct NativeAttempt {
    request: ActivationRequest,
    client_id: String,
    steer: bool,
    turn_id: Option<String>,
    blocked: bool,
}

struct StateRead {
    target: ActivationTarget,
    claim: Option<OutstandingClaim>,
    history: bool,
    pages: u16,
}

/// Injectable actor for one outer connection. The caller drains user frames
/// before `drive_pending`, and forwards only frames returned by `handle_native`.
/// It is the sole writer to its supplied native connection.
pub(crate) struct NativeBridge<'a> {
    configuration: SessionConfiguration,
    launch: RootLaunch,
    identity: ServiceIdentity,
    ports: BridgePorts<'a>,
    owner: AttachmentOwnerId,
    router: ProtocolRouter,
    handshake: ConnectionHandshake,
    roots: HashMap<String, BoundRoot>,
    external: HashMap<RpcId, ExternalOperation>,
    controls: HashMap<RpcId, (Option<String>, bool)>,
    attempts: HashMap<String, NativeAttempt>,
    state_reads: HashMap<RpcId, StateRead>,
    recovery_scanned: HashSet<BatchId>,
    policy: ActivationPolicy,
    events: VecDeque<DriverEvent>,
    notify_port: u16,
    supported: bool,
    closed: bool,
    fatal: Option<String>,
}
