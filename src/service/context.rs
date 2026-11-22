//! Invocation configuration, claimed identity and observed caller authority.

use std::collections::HashMap;
use std::io::IsTerminal;
use std::path::PathBuf;

use crate::agents::attachments::{ActivationTarget, ProviderSessionKey};

#[cfg(test)]
#[path = "observation_context_tests.rs"]
mod observation_tests;

/// Stable caller-generated identifier. Reuse after a lost reply only where the
/// operation explicitly promises durable deduplication with the same input.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) struct RequestId(pub String);

/// Captured by the originating process. Environment values select configuration,
/// never prove identity, process ownership or authority over another attachment.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub(crate) struct InvocationContext {
    pub cwd: PathBuf,
    pub environment: HashMap<String, String>,
    /// Caller-selected dotenv variable names for local child composition.
    /// Values stay in environment and these names grant no authority.
    #[serde(default)]
    pub environment_file_keys: std::collections::BTreeSet<String>,
    /// Existing launch metadata captured by the originating host. This is a
    /// configuration claim, never process authority; the daemon must not run
    /// git/tty or consume terminal handoff files to reconstruct it.
    #[serde(default)]
    pub launch_context: Option<serde_json::Value>,
    pub explicit_name: Option<String>,
    pub process_binding_id: Option<String>,
    pub provider_session: Option<ProviderSessionKey>,
    pub activation_target: Option<ActivationTarget>,
    pub actor_capability: Option<String>,
    pub stdin_is_tty: bool,
    pub stdout_is_tty: bool,
    /// Optional originating correlation claim. This supplies no actor authority.
    #[serde(default)]
    pub observation_origin: Option<crate::observability::runtime::OriginRef>,
}

impl InvocationContext {
    /// Capture one originating launch/session initialization, including its
    /// one-shot terminal handoff. Ordinary queries must use `capture` instead.
    pub(crate) fn capture_for_launch() -> super::ServiceResult<Self> {
        let mut invocation = Self::capture()?;
        let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
        invocation.launch_context = Some(serde_json::Value::Object(crate::agents::binding::capture_context_snapshot(&scope, &invocation.cwd, &invocation.environment)));
        Ok(invocation)
    }

    /// Capture the originating client process once, before service acquisition.
    /// Callers apply parsed identity/session/launch overrides to this value;
    /// no environment variable is promoted to authenticated identity here.
    pub(crate) fn capture() -> super::ServiceResult<Self> {
        use crate::observability::runtime::{ClientAttemptKind, ClientStage};
        let _attempt = super::observation::AttemptGuard::begin(ClientAttemptKind::Probe, PathBuf::new(), None, None);
        super::observation::stage(ClientStage::Capture, || Self::capture_inputs().map_err(crate::daemon::DaemonError::from)).map_err(|error| super::ServiceError::InvalidInput(format!("cannot resolve caller working directory: {error}")))
    }

    /// Passive originating consumers receive the known input failure before
    /// service flattening, even when observation time itself is unavailable.
    pub(crate) fn capture_observed() -> Result<Self, crate::observability::runtime::LocalFailure> {
        Self::capture_inputs().map_err(|error| crate::daemon::observation::io_local_failure(error, crate::observability::runtime::ClientStage::Capture, originating_clock()))
    }

    fn capture_inputs() -> std::io::Result<Self> {
        let cwd = std::env::current_dir()?;
        let environment: HashMap<String, String> = std::env::vars().collect();
        let process_binding_id = environment.get("AGENTX_PROCESS_ID").filter(|value| !value.is_empty()).cloned();
        Ok(Self { cwd, environment, environment_file_keys: crate::runtime::environment::originating_keys(), process_binding_id, stdin_is_tty: std::io::stdin().is_terminal(), stdout_is_tty: std::io::stdout().is_terminal(), ..Self::default() })
    }
}

/// Caller-supplied request metadata. No PID supplied here becomes a trusted host.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct CallerContext {
    pub request_id: RequestId,
    pub invocation: InvocationContext,
    pub deadline_unix_ms: Option<u64>,
    pub expected_database_generation: Option<String>,
}

/// OS process evidence observed by the local transport/host registration.
/// This is deliberately not deserializable from the request body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CallerProcess {
    pub pid: u32,
    pub incarnation: Option<String>,
}

/// Trusted application context, constructed at the daemon boundary. A daemon
/// restart changes `daemon_instance`; only data replacement changes generation.
#[derive(Clone, Debug)]
pub(crate) struct RequestContext {
    pub caller: CallerContext,
    pub process: CallerProcess,
    pub daemon_instance: String,
    pub database_generation: String,
}

/// One retained origin shared by all originating adapters in this process.
pub(crate) fn originating_clock() -> &'static crate::observability::context::ObservationClock {
    static CLOCK: std::sync::OnceLock<crate::observability::context::ObservationClock> = std::sync::OnceLock::new();
    CLOCK.get_or_init(crate::observability::context::ObservationClock::new)
}

/// Instrumentation-only context for the sole Application dispatch. No business
/// resolver reads this context for identity, configuration or mutation authority.
#[derive(Clone)]
pub(crate) struct ActiveObservation {
    pub epoch: crate::observability::context::ObservationEpoch,
    pub clock: crate::observability::context::ObservationClock,
    pub request: Option<crate::observability::runtime::RequestObservationRef>,
    pub peer: Option<crate::observability::runtime::PeerObservationRef>,
    pub accepted: Option<crate::observability::runtime::AcceptedPeer>,
    pub origin: Option<crate::observability::runtime::OriginRef>,
    pub request_id: RequestId,
    pub runtime: std::sync::Arc<dyn crate::observability::runtime::RuntimeRecorder>,
    pub configuration: std::sync::Arc<dyn crate::observability::configuration::ConfigurationRecorder>,
    pub sink: std::sync::Arc<dyn crate::observability::events::ObservationSink>,
    pub pending_configuration: std::rc::Rc<std::cell::RefCell<Vec<crate::observability::configuration::OwnedConfigCapture>>>,
}

thread_local! {
    static ACTIVE_OBSERVATION: std::cell::RefCell<Option<ActiveObservation>> = const { std::cell::RefCell::new(None) };
}

pub(crate) struct ObservationDispatchGuard {
    previous: Option<ActiveObservation>,
}

impl ActiveObservation {
    pub(crate) fn enter(self) -> ObservationDispatchGuard {
        let previous = ACTIVE_OBSERVATION.with(|slot| slot.replace(Some(self)));
        ObservationDispatchGuard { previous }
    }
}

impl Drop for ObservationDispatchGuard {
    fn drop(&mut self) {
        use crate::observability::runtime::{ConfigurationUse, RequestUpdate};
        let active = ACTIVE_OBSERVATION.with(|slot| slot.replace(self.previous.take()));
        let Some(active) = active else {
            return;
        };
        // The domain handler has returned/unwound and its transaction guards
        // have already released SQLite. Publication failure changes no result.
        let captures = std::mem::take(&mut *active.pending_configuration.borrow_mut());
        for capture in captures {
            let stage = capture.body.stage.clone();
            let result = active.configuration.record_resolved(capture);
            if let Ok(at) = active.clock.now() {
                active.runtime.request_changed(active.request.as_ref(), RequestUpdate::ConfigurationUsed(ConfigurationUse { stage, capture: result }), at);
            }
        }
    }
}

pub(crate) fn active_observation() -> Option<ActiveObservation> {
    ACTIVE_OBSERVATION.with(|slot| slot.try_borrow().ok().and_then(|active| active.clone()))
}

/// Dataset replacement writes belong to a generation not published yet. Skip
/// those projections instead of assigning its new rows to the admitted era.
/// The outer dispatch still owns runtime chronology and later epoch rotation.
pub(crate) fn without_dataset_observation<T>(action: impl FnOnce() -> T) -> T {
    let previous = ACTIVE_OBSERVATION.with(|slot| slot.replace(None));
    let _restore = ObservationDispatchGuard { previous };
    action()
}

pub(crate) fn observation_time() -> Option<crate::observability::context::ObservationTime> {
    active_observation().map(|active| active.clock).unwrap_or_else(|| originating_clock().clone()).now().ok()
}

fn projection_body(
    active: &ActiveObservation,
    subject: crate::observability::configuration::ConfigSubject,
    stage: crate::observability::configuration::ConfigStage,
    purpose: crate::observability::configuration::ConfigPurpose,
) -> Option<crate::observability::configuration::ConfigSnapshotBody> {
    use crate::observability::{configuration::*, context::EvidenceValue};
    Some(ConfigSnapshotBody {
        subject,
        resolution_scope: EvidenceValue::Present(active.epoch.daemon.scope.clone()),
        stage,
        purpose,
        correlation: ConfigCorrelation { invocation: active.origin.as_ref().map(|origin| origin.invocation.clone()), attempt: active.origin.as_ref().map(|origin| origin.attempt) },
        rules: RulesRevision::current(env!("AGENTX_NATIVE_BUILD_ID")),
        captured_at: active.clock.now().ok()?,
        consistency: ReadConsistency::SingleSourceRead,
        source_reads: Vec::new(),
        fields: Vec::new(),
        consumed_fields: Vec::new(),
    })
}

pub(crate) fn buffer_configuration(body: crate::observability::configuration::ConfigSnapshotBody, provenance: crate::observability::configuration::OwnedConfigOrigin) {
    let Some(active) = active_observation() else {
        return;
    };
    let mut pending = active.pending_configuration.borrow_mut();
    if pending.len() < 8 {
        pending.push(crate::observability::configuration::OwnedConfigCapture { expected_epoch: active.epoch, body, provenance });
    }
}

pub(crate) fn scoped_declarations(scope: &std::path::Path, read: crate::observability::configuration::SourceRead, fields: Vec<crate::observability::configuration::FieldEvidence>) {
    use crate::observability::configuration::*;
    let Some(active) = active_observation() else {
        return;
    };
    if scope != active.epoch.daemon.scope.canonical_scope {
        return;
    }
    let Some(mut body) = projection_body(&active, ConfigSubject::ScopeDeclarations { scope: active.epoch.daemon.scope.clone() }, ConfigStage::ScopedDeclarations, ConfigPurpose::DeclarationRead) else {
        return;
    };
    body.captured_at = read.at.clone();
    body.source_reads.push(read);
    body.fields = fields;
    buffer_configuration(body, OwnedConfigOrigin::DaemonCaptured);
}

/// Use only the row already obtained by a successful native participant read.
/// No identity lookup, value defaulting or additional SQL is done for telemetry.
pub(crate) fn participant_configuration(row: &crate::db::InstanceRow) {
    use crate::observability::{
        configuration::*,
        context::{EvidenceValue, UnknownReason},
    };
    let Some(active) = active_observation() else {
        return;
    };
    let participant = crate::agents::attachments::ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() };
    let Some(mut body) = projection_body(&active, ConfigSubject::Participant { participant }, ConfigStage::ParticipantState, ConfigPurpose::DomainProjection) else {
        return;
    };
    for (field, value) in [(ParticipantFieldId::Timeout, row.wait_timeout), (ParticipantFieldId::SubagentTimeout, row.subagent_timeout)] {
        let value = match value {
            None => EvidenceValue::Absent,
            Some(value) => u64::try_from(value).ok().filter(|value| *value > 0).map(|value| EvidenceValue::Present(ConfigValue::Seconds(value))).unwrap_or(EvidenceValue::Unknown(UnknownReason::ObservationFailed)),
        };
        let resolution = FieldResolution::Effective { source: ConfigSourceRef { kind: ConfigSourceKind::DomainRecord, read: None }, normalized: false };
        if let Ok(field) = projected_field(ConfigFieldId::Participant(field), value, resolution) {
            body.fields.push(field);
        }
    }
    // These fields have fixed withholding; do not even copy row text into the projection.
    {
        let field = ParticipantFieldId::Hints;
        if let Ok(field) = projected_field(ConfigFieldId::Participant(field), EvidenceValue::Withheld, FieldResolution::InputOnly) {
            body.fields.push(field);
        }
    }
    buffer_configuration(body, OwnedConfigOrigin::StoredDomainProjection);
}

/// The caller supplies the result of its one real persistent dev-root read.
/// A missing key, failed read and explicit path remain different evidence.
pub(crate) fn persistent_dev_root(value: Result<Option<&str>, ()>) {
    use crate::observability::{
        configuration::*,
        context::{EvidenceValue, UnknownReason},
    };
    let Some(active) = active_observation() else {
        return;
    };
    let Some(origin) = &active.origin else {
        return;
    };
    let Some(mut body) = projection_body(&active, ConfigSubject::Invocation { capture: origin.invocation.clone() }, ConfigStage::ScopedEffective, ConfigPurpose::ActualResolution) else {
        return;
    };
    let (value, state, resolution) = match value {
        Ok(Some(path)) => {
            (EvidenceValue::Present(ConfigValue::Path(path.into())), SourceReadState::Read, FieldResolution::Effective { source: ConfigSourceRef { kind: ConfigSourceKind::DomainRecord, read: Some(SourceReadId(1)) }, normalized: false })
        }
        Ok(None) => (EvidenceValue::Absent, SourceReadState::Missing, FieldResolution::Absent),
        Err(()) => (EvidenceValue::Unknown(UnknownReason::ObservationFailed), SourceReadState::Unreadable, FieldResolution::Unknown(UnknownReason::ObservationFailed)),
    };
    body.source_reads.push(SourceRead::new(1, SourceResource::PersistentDevRoot, body.captured_at.clone(), state));
    let field = ConfigFieldId::Context(ContextFieldId::EffectiveDevRoot);
    if let Ok(evidence) = projected_field(field.clone(), value, resolution) {
        body.fields.push(evidence);
        body.consumed_fields.push(field);
    }
    buffer_configuration(body, OwnedConfigOrigin::ApplicationResolved);
}

pub(crate) fn configuration_loaded(scope: &std::path::Path, config: &crate::config::AgentxConfig, trace: crate::observability::configuration::ConfigResolutionTrace) {
    use crate::observability::configuration::*;
    use crate::observability::context::EvidenceValue;
    let Some(active) = active_observation() else {
        return;
    };
    let Some(origin) = &active.origin else {
        return;
    };
    let Ok(at) = active.clock.now() else {
        return;
    };
    let mut body = ConfigSnapshotBody::invocation(origin.invocation.clone(), at, RulesRevision::current(env!("AGENTX_NATIVE_BUILD_ID")));
    body.stage = ConfigStage::ScopedEffective;
    body.purpose = ConfigPurpose::ActualResolution;
    body.correlation.attempt = Some(origin.attempt);
    body.resolution_scope = if scope == active.epoch.daemon.scope.canonical_scope { EvidenceValue::Present(active.epoch.daemon.scope.clone()) } else { EvidenceValue::Unknown(crate::observability::context::UnknownReason::NotCaptured) };
    let Ok(body) = project_effective_config(config, body, trace) else {
        return;
    };
    buffer_configuration(body, OwnedConfigOrigin::ApplicationResolved);
}

/// A safe projection freezes its producer fence before entering the DB's
/// postcommit list. Later TLS changes cannot redirect or upgrade the fact.
pub(crate) struct PreparedDomainObservation {
    pub epoch: crate::observability::context::ObservationEpoch,
    pub clock: crate::observability::context::ObservationClock,
    pub sink: std::sync::Arc<dyn crate::observability::events::ObservationSink>,
    pub observation: crate::observability::events::DomainCommitObservation,
}

impl PreparedDomainObservation {
    pub(crate) fn publish(self) {
        use crate::observability::{
            context::{EvidenceOrigin, EvidenceStamp},
            events::*,
        };
        let Ok(at) = self.clock.now() else {
            self.sink.note_drop(DropReason::CollectorUnavailable, 1);
            return;
        };
        let _ = self.sink.record(ObservationInput { epoch: self.epoch, source: EvidenceStamp { origin: EvidenceOrigin::StoredDomainFact, at }, payload: ObservationPayload::DomainCommitted(self.observation) });
    }
}

pub(crate) fn prepare_domain_observation(fact: crate::observability::events::DomainCommitFact) -> Option<PreparedDomainObservation> {
    use crate::observability::events::{DomainCommitObservation, DomainCorrelation};
    let active = active_observation()?;
    Some(PreparedDomainObservation {
        epoch: active.epoch,
        clock: active.clock,
        sink: active.sink,
        observation: DomainCommitObservation { correlation: DomainCorrelation { request: active.request.map(|request| request.id), client_attempt: active.origin.map(|origin| origin.attempt), request_id: Some(active.request_id), actor: None }, fact },
    })
}

/// Invoke only after a successful native registration transaction returned.
/// Unknown/missing projection never rejects a host, and no token is retained.
pub(crate) fn host_validated(
    kind: crate::observability::runtime::HostObservationKind,
    participant: crate::agents::attachments::ParticipantRef,
    target: Option<ActivationTarget>,
    child: Option<crate::observability::runtime::ObservedProcess>,
    state: crate::observability::runtime::HostAssociationState,
) {
    let Some(active) = active_observation() else {
        return;
    };
    let (Some(peer), Some(request), Ok(at)) = (active.peer, active.request, active.clock.now()) else {
        return;
    };
    let _ = active.runtime.host_validated(crate::observability::runtime::ValidatedHostObservation { epoch: active.epoch, previous: None, kind, peer, participant, target, child, state, request }, at);
}

/// A successful host registration may report only policy-filtered originating
/// inputs from that exact request; native ownership and the OS peer are already
/// validated by the host service before this instrumentation callback.
pub(crate) fn host_configuration(invocation: &InvocationContext, participant: crate::agents::attachments::ParticipantRef, execution: Option<crate::agents::attachments::ExecutionId>) {
    use crate::observability::{configuration::*, context::EvidenceValue, runtime::ClientInvocationId};
    let Some(active) = active_observation() else {
        return;
    };
    let Some(peer) = active.accepted else {
        return;
    };
    let Ok(at) = active.clock.now() else {
        return;
    };
    let capture = InvocationCaptureHandle::begin(ClientInvocationId::new());
    let Ok(snapshot) = capture.capture_invocation_inputs(CaptureInputs { invocation, at, rules: RulesRevision::current(env!("AGENTX_NATIVE_BUILD_ID")) }) else {
        return;
    };
    let mut body = snapshot.body;
    body.subject = ConfigSubject::Host { participant, execution };
    body.stage = ConfigStage::HostReported;
    body.purpose = ConfigPurpose::ValidatedHostReport;
    body.correlation.invocation = active.origin.as_ref().map(|origin| origin.invocation.clone());
    body.correlation.attempt = active.origin.as_ref().map(|origin| origin.attempt);
    body.resolution_scope = EvidenceValue::Present(active.epoch.daemon.scope.clone());
    let _ = active.configuration.record_host_report(active.epoch, &peer, HostConfigReport { body });
}
