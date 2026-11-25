//! Typed local IPC binding. This module has no database construction or fallback.
//! Each call carries the originating invocation; reconnect never changes its
//! retained database generation or silently replays an operation.

use std::path::Path;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};

use crate::daemon::{self, DaemonError, DaemonInfo, DaemonScope};

use super::observation::{self, AttemptGuard};
use super::{CallerContext, InvocationContext, Request, RequestId, Response, ServiceError, ServiceResult};
use crate::observability::{
    configuration::{CaptureInputs, InvocationCaptureHandle, RulesRevision},
    context::ScopeRef,
    runtime::{ClientAttemptKind, ClientInvocationId, ClientStage},
};

#[derive(Clone)]
pub(crate) struct Client {
    scope: DaemonScope,
    invocation: InvocationContext,
    info: Arc<OnceLock<DaemonInfo>>,
    pinned: Option<String>,
    allow_start: Arc<AtomicBool>,
    capture: InvocationCaptureHandle,
    deadline_unix_ms: Option<u64>,
}

impl Client {
    pub(crate) fn connect(scope: impl AsRef<Path>, invocation: InvocationContext) -> ServiceResult<Self> {
        Self::bind(scope.as_ref(), invocation, true, None)
    }

    pub(crate) fn connect_existing(scope: impl AsRef<Path>, invocation: InvocationContext) -> ServiceResult<Self> {
        Self::bind(scope.as_ref(), invocation, false, None)
    }

    pub(crate) fn connect_existing_fenced(scope: impl AsRef<Path>, invocation: InvocationContext, generation: &str) -> ServiceResult<Self> {
        Self::bind(scope.as_ref(), invocation, false, Some(generation.to_owned()))
    }

    fn bind(path: &Path, invocation: InvocationContext, start: bool, pinned: Option<String>) -> ServiceResult<Self> {
        let capture = capture_inputs(&invocation, None);
        let scope = if start { DaemonScope::prepare(path).map_err(service_error)? } else { DaemonScope::existing(path, &invocation.cwd).map_err(DaemonError::from).map_err(service_error)? };
        let pinned = pinned.or_else(|| {
            let expected = invocation.environment.get("AGENTX_LAUNCH_GENERATION").filter(|value| !value.is_empty())?;
            let launched = invocation.environment.get("AGENTX_DIR")?;
            let scoped = DaemonScope::existing(Path::new(launched), &invocation.cwd).ok()?;
            (scoped.root() == scope.root()).then(|| expected.clone())
        });
        Ok(Self { scope, invocation, info: Arc::new(OnceLock::new()), pinned, allow_start: Arc::new(AtomicBool::new(start)), capture, deadline_unix_ms: None })
    }

    pub(crate) fn call_codex_hook(scope: impl AsRef<Path>, invocation: InvocationContext, event: super::codex_hook::HookEvent) -> ServiceResult<Option<(Self, super::codex_hook::HookOutcome)>> {
        use std::time::{SystemTime, UNIX_EPOCH};
        // Leave room within the configured five-second model hook budget, and
        // reserve the last part of this overall deadline for output confirmation.
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| ServiceError::Unavailable("system clock before Unix epoch".into()))?.as_millis() as u64;
        let request_id = RequestId(uuid::Uuid::new_v4().to_string());
        let mut attempt = AttemptGuard::begin(ClientAttemptKind::Call, scope.as_ref().to_path_buf(), None, Some(request_id.clone()));
        if let Some(attempt) = &mut attempt {
            attempt.name_operation_label(crate::observability::runtime::OperationLabel::of_codex_hook(&event));
        }
        let capture = capture_inputs(&invocation, None);
        observation::invocation(capture.reference());
        let scope = observation::stage(ClientStage::ScopeLookup, || DaemonScope::existing(scope.as_ref(), &invocation.cwd).map_err(DaemonError::from)).map_err(service_error)?;
        if let Ok(reference) = ScopeRef::from_canonical_scope(scope.root().to_path_buf()) {
            observation::resolved_scope(reference);
        }
        let expected_database_generation = invocation.environment.get("AGENTX_LAUNCH_GENERATION").filter(|value| !value.is_empty()).cloned();
        let mut caller = CallerContext { request_id, invocation: invocation.clone(), deadline_unix_ms: Some(now.saturating_add(3_000)), expected_database_generation };
        caller.invocation.observation_origin = attempt.as_ref().and_then(|attempt| attempt.origin());
        let (info, result) = match daemon::call_bound(&scope, false, caller, Request::CodexHook(event)) {
            Err(DaemonError::Unsupported(_)) => return Ok(None),
            result => result.map_err(service_error)?,
        };
        match result? {
            Response::CodexHook(outcome) => {
                Ok(Some((Self { scope, invocation, info: Arc::new(OnceLock::from(info)), pinned: None, allow_start: Arc::new(AtomicBool::new(false)), capture, deadline_unix_ms: Some(now.saturating_add(4_000)) }, outcome)))
            }
            _ => Err(ServiceError::Internal("unexpected Codex hook response".into())),
        }
    }

    /// Explicit status, or the description already returned by a business call.
    pub(crate) fn info(&self) -> ServiceResult<&DaemonInfo> {
        if let Some(info) = self.info.get() {
            return Ok(info);
        }
        let start = self.take_start_permission();
        let _attempt = AttemptGuard::begin(if start { ClientAttemptKind::Acquire } else { ClientAttemptKind::Probe }, self.scope.root().to_path_buf(), None, None);
        observation::invocation(self.capture.reference());
        if let Ok(reference) = ScopeRef::from_canonical_scope(self.scope.root().to_path_buf()) {
            observation::resolved_scope(reference);
        }
        let deadline = self.deadline_unix_ms.map(|value| daemon::request_deadline(Some(value))).transpose().map_err(service_error)?;
        let info = daemon::describe(&self.scope, start, self.pinned.as_deref(), deadline).map_err(service_error)?;
        self.retain_info(info)
    }

    pub(crate) fn generation(&self) -> ServiceResult<&str> {
        if let Some(generation) = self.retained_generation() {
            return Ok(generation);
        }
        Ok(&self.info()?.database_generation)
    }

    /// Auto-start belongs to the first acquisition only. A retained caller,
    /// including one whose first reply was lost, must observe daemon loss.
    fn take_start_permission(&self) -> bool {
        self.allow_start.swap(false, Ordering::AcqRel)
    }

    fn retained_generation(&self) -> Option<&str> {
        self.pinned.as_deref().or_else(|| self.info.get().map(|info| info.database_generation.as_str()))
    }

    fn retain_info(&self, info: DaemonInfo) -> ServiceResult<&DaemonInfo> {
        let generation = info.database_generation.clone();
        let _ = self.info.set(info);
        let retained = self.info.get().expect("scope binding initialized");
        if retained.database_generation != generation {
            return Err(ServiceError::StaleGeneration);
        }
        Ok(retained)
    }

    /// Canonical data scope retained by this connection binding.
    pub(crate) fn scope_root(&self) -> &Path {
        self.scope.root()
    }

    /// Captured caller configuration; these claims do not prove authority.
    pub(crate) fn invocation(&self) -> &InvocationContext {
        &self.invocation
    }

    /// Carry resolved invocation claims without reacquiring the scope or
    /// changing the retained daemon identity/database generation fence.
    pub(crate) fn with_invocation(&self, invocation: InvocationContext) -> Self {
        let capture = capture_inputs(&invocation, Some(&self.capture));
        Self { invocation, capture, ..self.clone() }
    }

    /// Preserve the result of the actual originating resolver without rereading
    /// its file. Retention remains local; it is never an automatic report.
    pub(crate) fn capture_effective_configuration(&self, config: &crate::config::AgentxConfig, trace: crate::observability::configuration::ConfigResolutionTrace) {
        use crate::observability::{configuration::*, context::EvidenceValue};
        let Ok(at) = super::context::originating_clock().now() else {
            return;
        };
        let mut body = ConfigSnapshotBody::invocation(self.capture.reference(), at, RulesRevision::current(env!("AGENTX_NATIVE_BUILD_ID")));
        body.stage = ConfigStage::ScopedEffective;
        body.purpose = ConfigPurpose::ActualResolution;
        body.resolution_scope = ScopeRef::from_canonical_scope(self.scope.root().to_path_buf()).map(EvidenceValue::Present).unwrap_or(EvidenceValue::Unknown(crate::observability::context::UnknownReason::NotCaptured));
        if let Ok(body) = project_effective_config(config, body, trace) {
            let _ = self.capture.capture_resolved(body);
        }
    }

    pub(crate) fn capture_launch_plan(&self, tool: &str, backend: &str, regime: &str, working_directory: &Path, provider_home: Option<&Path>, environment_read: Option<crate::observability::configuration::SourceRead>) {
        use crate::observability::{configuration::*, context::EvidenceValue};
        let Ok(at) = super::context::originating_clock().now() else {
            return;
        };
        let mut body = ConfigSnapshotBody::invocation(self.capture.reference(), at, RulesRevision::current(env!("AGENTX_NATIVE_BUILD_ID")));
        body.subject = ConfigSubject::HostPlan { capture: self.capture.reference() };
        body.stage = ConfigStage::LaunchPlanned;
        body.purpose = ConfigPurpose::LaunchPreparation;
        body.source_reads.extend(environment_read);
        body.consistency = ReadConsistency::MultipleNonAtomicReads;
        body.resolution_scope = EvidenceValue::Present(ScopeRef { canonical_scope: self.scope.root().to_path_buf() });
        let values = [
            (ContextFieldId::Tool, ConfigValue::Text(tool.into())),
            (ContextFieldId::LaunchBackend, ConfigValue::Text(backend.into())),
            (ContextFieldId::LaunchRegime, ConfigValue::Text(regime.into())),
            (ContextFieldId::EffectiveWorkingDirectory, ConfigValue::Path(working_directory.into())),
        ];
        for (field, value) in values.into_iter().chain(provider_home.map(|home| (ContextFieldId::ProviderHome, ConfigValue::Path(home.into())))) {
            let field = ConfigFieldId::Context(field);
            let resolution = FieldResolution::Effective { source: ConfigSourceRef { kind: ConfigSourceKind::ToolResolution, read: None }, normalized: false };
            if let Ok(evidence) = projected_field(field.clone(), EvidenceValue::Present(value), resolution) {
                body.fields.push(evidence);
                body.consumed_fields.push(field);
            }
        }
        let _ = self.capture.capture_resolved(body);
    }

    pub(crate) fn call(&self, request: Request) -> ServiceResult<Response> {
        let caller = CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: self.invocation.clone(), deadline_unix_ms: self.deadline_unix_ms, expected_database_generation: None };
        self.call_with_context(caller, request)
    }

    /// Stable IDs are passed unchanged; only the domain may promise duplicate
    /// reconciliation. No automatic retry follows a write or missing reply.
    pub(crate) fn call_with_context(&self, mut caller: CallerContext, request: Request) -> ServiceResult<Response> {
        let resetting = matches!(request, Request::Maintenance(super::maintenance::Request::ResetConversation));
        if let Some(retained) = self.retained_generation() {
            if caller.expected_database_generation.as_deref().is_some_and(|expected| expected != retained) {
                return Err(ServiceError::StaleGeneration);
            }
            caller.expected_database_generation = Some(retained.to_owned());
        } else if caller.expected_database_generation.is_none() && request.requires_generation() {
            caller.expected_database_generation = Some(self.generation()?.to_owned());
        }
        // A caller may supply a context differing from the retained binding;
        // freeze this exact immutable revision before the possible write.
        let mut attempt = AttemptGuard::begin(ClientAttemptKind::Call, self.scope.root().to_path_buf(), None, Some(caller.request_id.clone()));
        if let Some(attempt) = &mut attempt {
            attempt.name_operation(&request);
        }
        let capture = capture_inputs(&caller.invocation, Some(&self.capture));
        observation::invocation(capture.reference());
        if let Ok(reference) = ScopeRef::from_canonical_scope(self.scope.root().to_path_buf()) {
            observation::resolved_scope(reference);
        }
        caller.invocation.observation_origin = attempt.as_ref().and_then(|attempt| attempt.origin());
        let (info, result) = daemon::call_bound(&self.scope, self.take_start_permission(), caller, request).map_err(service_error)?;
        if !resetting {
            self.retain_info(info)?;
        }
        result
    }
}

fn capture_inputs(invocation: &InvocationContext, previous: Option<&InvocationCaptureHandle>) -> InvocationCaptureHandle {
    let capture = previous.and_then(|capture| capture.revise().ok()).or_else(crate::observability::trace::invocation_capture).unwrap_or_else(|| InvocationCaptureHandle::begin(ClientInvocationId::new()));
    if let Ok(at) = super::context::originating_clock().now() {
        let _ = capture.capture_invocation_inputs(CaptureInputs { invocation, at, rules: RulesRevision::current(env!("AGENTX_NATIVE_BUILD_ID")) });
    }
    capture
}

fn service_error(error: DaemonError) -> ServiceError {
    match error {
        DaemonError::StaleGeneration => ServiceError::StaleGeneration,
        DaemonError::InvalidInput(detail) => ServiceError::InvalidInput(detail),
        DaemonError::OutcomeUnknown(detail) => ServiceError::OutcomeUnknown(detail),
        DaemonError::Unsupported(detail) => ServiceError::Unsupported(detail),
        other => ServiceError::Unavailable(other.to_string()),
    }
}
