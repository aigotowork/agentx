//! Safe immutable evidence about configuration; never an alternative settings resolver.
//! Native authority for .docs/observability/configuration.md. Existing loader policy
//! remains in crate::config; producers publish its actual same-pass outcomes.

// Era errors deliberately preserve both exact native scopes/daemon identities.
// Keeping this leaf's finite error contract inline avoids allocation on failure;
// admission bounds the values, and transport separately caps complete responses.
#![allow(clippy::result_large_err)]

use super::context::{DaemonRef, EvidenceOrigin, EvidenceValue, ObservationEpoch, ObservationTime, ScopeRef, UnknownReason};
use super::runtime::{ClientAttemptId, ClientInvocationId};
use crate::config::{AgentxConfig, FIELDS};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU32, AtomicU64, Ordering},
};

pub(crate) use super::configuration_store::ConfigurationStore;

pub(crate) const MAX_SNAPSHOT_BYTES: usize = 32 * 1024;
pub(crate) const MAX_FIELDS: usize = 64;
pub(crate) const MAX_STEPS: usize = 12;
pub(crate) const MAX_SOURCE_READS: usize = 16;

#[cfg(test)]
#[path = "configuration_tests.rs"]
mod tests;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) struct InvocationCaptureRef {
    pub invocation: ClientInvocationId,
    pub revision: u64,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) struct LocalConfigSnapshotRef {
    pub capture: InvocationCaptureRef,
    pub sequence: u32,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) struct ConfigSnapshotRef {
    pub epoch: ObservationEpoch,
    pub sequence: u64,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[allow(dead_code, reason = "Accepted local/retained selector contract for the downstream diagnostic adapter; no native consumer yet")]
pub(crate) enum ConfigEvidenceRef {
    Local(LocalConfigSnapshotRef),
    Retained(ConfigSnapshotRef),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) enum ConfigStage {
    StartupInputs,
    InvocationInputs,
    ScopedDeclarations,
    ScopedEffective,
    LaunchPlanned,
    HostReported,
    ParticipantState,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum CaptureSkipReason {
    Pressure,
    Oversize,
    Unsupported,
    ObservationFailed,
    StaleEpoch,
    GenerationUnavailable,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) enum ConfigCaptureResult {
    Recorded(ConfigSnapshotRef),
    Skipped(CaptureSkipReason),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum RetentionEnd {
    Age,
    CountPressure,
    BytePressure,
    Reset,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) enum ConfigObservationEvent {
    SnapshotPublished { snapshot: ConfigSnapshotRef, invocation: Option<InvocationCaptureRef>, attempt: Option<ClientAttemptId>, stage: ConfigStage, origin: EvidenceOrigin, captured_at: ObservationTime },
    CaptureSkipped { invocation: Option<InvocationCaptureRef>, attempt: Option<ClientAttemptId>, stage: ConfigStage, reason: CaptureSkipReason },
    SnapshotsRetired { through_sequence: u64, count: u32, reason: RetentionEnd },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema)]
#[serde(try_from = "String")]
pub(crate) struct ConfigSettingId(String); // validated constructor; current FIELDS only
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) enum ContextFieldId {
    Cwd,
    RequestedScope,
    CanonicalScope,
    StdinIsTty,
    StdoutIsTty,
    ExplicitNameSelection,
    ProviderHome,
    Tool,
    LaunchRegime,
    LaunchBackend,
    EffectiveWorkingDirectory,
    EffectiveDevRoot,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) enum ParticipantFieldId {
    Timeout,
    SubagentTimeout,
    Hints,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) enum ConfigFieldId {
    Setting(ConfigSettingId),
    Context(ContextFieldId),
    Participant(ParticipantFieldId),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum ConfigValue {
    Text(String),
    Boolean(bool),
    Seconds(u64),
    #[schema(value_type = String)]
    Path(std::path::PathBuf),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct RulesRevision {
    pub native_build_id: String,
    pub field_catalog_revision: u32,
    pub resolution_revision: u32,
    pub disclosure_revision: u32,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub(crate) struct SourceReadId(pub u16); // local to one snapshot; positive
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum SourceResource {
    ConfigToml,
    LaunchEnvironmentFile,
    TerminalPresets,
    PersistentDevRoot,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum SourceReadState {
    Read,
    Missing,
    Unreadable,
    SyntaxInvalid,
    NotObserved(UnknownReason),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct SourceRead {
    pub id: SourceReadId,
    pub resource: SourceResource,
    pub read_revision: String,
    pub at: ObservationTime,
    pub state: SourceReadState,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum ConfigSourceKind {
    BuiltInDefault,
    InvocationEnvironment,
    ScopeFile,
    ParsedOption,
    CapturedParent,
    CleanShell,
    LaunchEnvironmentFile,
    ExplicitLaunchEnvironment,
    DomainRecord,
    ToolResolution,
    ReservedIdentityInjection,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) struct ConfigSourceRef {
    pub kind: ConfigSourceKind,
    pub read: Option<SourceReadId>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum ConfigIssueCode {
    WrongScalarType,
    InvalidInteger,
    InvalidBoolean,
    OutOfRange,
    InvalidEnum,
    InvalidSyntax,
    UnsafeTerminal,
    UnsupportedTerminal,
    SourceUnreadable,
    SourceSyntaxInvalid,
    ValidationNotObserved,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum CandidateValidation {
    Valid,
    Invalid(ConfigIssueCode),
    NotEvaluated,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum CandidateDisposition {
    Selected,
    Shadowed,
    IgnoredEmpty,
    IgnoredByFieldPolicy,
    Rejected,
    NotConsulted,
    NotObserved(UnknownReason),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) struct ResolutionStep {
    pub source: ConfigSourceRef,
    pub disposition: CandidateDisposition,
    pub validation: CandidateValidation,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum FieldResolution {
    InputOnly,
    Effective { source: ConfigSourceRef, normalized: bool },
    Defaulted { rejected: ConfigSourceRef, issue: ConfigIssueCode },
    Absent,
    Unknown(UnknownReason),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct FieldEvidence {
    pub field: ConfigFieldId,
    pub value: EvidenceValue<ConfigValue>,
    pub resolution: EvidenceValue<FieldResolution>,
    pub steps: Vec<ResolutionStep>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) enum ConfigSubject {
    DaemonStartup { daemon: DaemonRef },
    Invocation { capture: InvocationCaptureRef },
    ScopeDeclarations { scope: ScopeRef },
    Participant { participant: crate::agents::attachments::ParticipantRef },
    HostPlan { capture: InvocationCaptureRef },
    Host { participant: crate::agents::attachments::ParticipantRef, execution: Option<crate::agents::attachments::ExecutionId> },
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum ConfigPurpose {
    StartupCapture,
    InputCapture,
    DeclarationRead,
    ActualResolution,
    LaunchPreparation,
    ValidatedHostReport,
    DomainProjection,
    DiagnosticReevaluation,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum ReadConsistency {
    SingleSourceRead,
    MultipleNonAtomicReads,
    InputsOnly,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct ConfigCorrelation {
    pub invocation: Option<InvocationCaptureRef>,
    pub attempt: Option<ClientAttemptId>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct ConfigSnapshotBody {
    pub subject: ConfigSubject,
    pub resolution_scope: EvidenceValue<ScopeRef>,
    pub stage: ConfigStage,
    pub purpose: ConfigPurpose,
    pub correlation: ConfigCorrelation,
    pub rules: RulesRevision,
    pub captured_at: ObservationTime,
    pub consistency: ReadConsistency,
    pub source_reads: Vec<SourceRead>,
    pub fields: Vec<FieldEvidence>,
    pub consumed_fields: Vec<ConfigFieldId>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LocalConfigSnapshot {
    pub reference: LocalConfigSnapshotRef,
    pub body: ConfigSnapshotBody,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct ConfigSnapshot {
    pub reference: ConfigSnapshotRef,
    pub body: ConfigSnapshotBody,
    pub provenance: ConfigProvenance,
    pub retained_at: ObservationTime,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) enum ConfigProvenance {
    DaemonCaptured,
    ApplicationResolved,
    StoredDomainProjection,
    ClientReported { source: LocalConfigSnapshotRef, peer: ConfigPeerBinding },
    HostReported { peer: ConfigPeerBinding },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct ConfigurationLimits {
    pub snapshots: u32,
    pub payload_bytes: u64,
    pub max_age_ms: u64,
    pub snapshot_bytes: u32,
    pub fields_per_snapshot: u16,
    pub steps_per_field: u8,
    pub source_reads_per_snapshot: u8,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct ConfigurationHead {
    pub epoch: ObservationEpoch,
    pub observed_at: ObservationTime,
    pub oldest_retained: Option<u64>,
    pub last_issued: u64,
    pub retired_through: u64,
    pub retained_count: u32,
    pub retained_payload_bytes: u64,
    pub skipped_pressure: u64,
    pub skipped_oversize: u64,
    pub skipped_other: u64,
    pub limits: ConfigurationLimits,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) enum ConfigMissing {
    NeverIssued,
    NoLongerRetained { retired_through: u64 },
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::large_enum_variant, reason = "One bounded immutable snapshot is carried inline by the accepted native read contract")]
pub(crate) enum SnapshotLookup {
    Found(ConfigSnapshot),
    Missing { reference: ConfigSnapshotRef, reason: ConfigMissing },
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct ConfigurationRead<T> {
    pub epoch: ObservationEpoch,
    pub observed_at: ObservationTime,
    pub result: T,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) enum ConfigFieldSelection {
    All,
    Only(Vec<ConfigFieldId>),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct CompareConfig {
    pub left: ConfigSnapshotRef,
    pub right: ConfigSnapshotRef,
    pub fields: ConfigFieldSelection,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum ComparisonBoundary {
    DifferentScope,
    ScopeUnestablished,
    DifferentEpoch,
    DifferentStage,
    DifferentMeaningRevision,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum ComparisonUnknown {
    Withheld,
    Unobserved,
    CaptureUnavailable,
    InvalidValue,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum ValueRelation {
    Equal,
    Different,
    Unknown(ComparisonUnknown),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) enum ProvenanceRelation {
    SameClass,
    DifferentClass,
    Unknown,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct FieldComparison {
    pub field: ConfigFieldId,
    pub left: Option<FieldEvidence>,
    pub right: Option<FieldEvidence>,
    pub value: ValueRelation,
    pub provenance: ProvenanceRelation,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct ComparisonBasis {
    pub left: ConfigSnapshotRef,
    pub right: ConfigSnapshotRef,
    pub left_subject: ConfigSubject,
    pub right_subject: ConfigSubject,
    pub left_resolution_scope: EvidenceValue<ScopeRef>,
    pub right_resolution_scope: EvidenceValue<ScopeRef>,
    pub stage: ConfigStage,
    pub left_purpose: ConfigPurpose,
    pub right_purpose: ConfigPurpose,
    pub left_captured_at: ObservationTime,
    pub right_captured_at: ObservationTime,
    pub left_provenance: ConfigProvenance,
    pub right_provenance: ConfigProvenance,
    pub left_rules: RulesRevision,
    pub right_rules: RulesRevision,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::large_enum_variant, reason = "The native comparison owns at most two bounded snapshots and preserves exact unavailable-side evidence")]
pub(crate) enum CompareConfigResult {
    Compared { basis: ComparisonBasis, fields: Vec<FieldComparison> },
    Unavailable { left: SnapshotLookup, right: SnapshotLookup },
    Incomparable(ComparisonBoundary),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) enum ConfigurationQueryError {
    GenerationUnavailable(DaemonRef),
    EpochChanged { requested: ObservationEpoch, current: ObservationEpoch },
    InvalidReference,
    UnknownField,
    InvalidSelection,
    Oversize,
    Busy,
    Unavailable,
}
pub(crate) trait ConfigurationReader: Send + Sync {
    fn head(&self, expected: Option<ObservationEpoch>) -> Result<ConfigurationHead, ConfigurationQueryError>;
    fn lookup(&self, reference: ConfigSnapshotRef) -> Result<ConfigurationRead<SnapshotLookup>, ConfigurationQueryError>;
    fn compare(&self, input: CompareConfig) -> Result<ConfigurationRead<CompareConfigResult>, ConfigurationQueryError>;
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(clippy::large_enum_variant, reason = "Finite native request keeps its bounded safe report inline without changing the shared facade")]
pub(crate) enum ConfigurationRequest {
    Head { expected: Option<ObservationEpoch> },
    Lookup(ConfigSnapshotRef),
    Compare(CompareConfig),
    ReportSafeCapture { expected_epoch: ObservationEpoch, report: ConfigReport },
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(clippy::large_enum_variant, reason = "Finite native response owns bounded read evidence; complete encoded output is capped independently")]
pub(crate) enum ConfigurationResponse {
    Head(ConfigurationHead),
    Snapshot(ConfigurationRead<SnapshotLookup>),
    Comparison(ConfigurationRead<CompareConfigResult>),
    Reported(ConfigSnapshotRef),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) enum ConfigurationError {
    Query(ConfigurationQueryError),
    Report(ConfigReportError),
    DaemonChanged { expected: DaemonRef, current: DaemonRef },
}

/// Transport-created evidence; callers never deserialize this as report input.
pub(crate) type ConfigPeerBinding = super::runtime::AcceptedPeer;

pub(crate) struct OwnedConfigCapture {
    pub expected_epoch: ObservationEpoch,
    pub body: ConfigSnapshotBody,
    pub provenance: OwnedConfigOrigin,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum OwnedConfigOrigin {
    DaemonCaptured,
    ApplicationResolved,
    StoredDomainProjection,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ConfigReport {
    pub source: LocalConfigSnapshot,
}
pub(crate) struct HostConfigReport {
    pub body: ConfigSnapshotBody,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) enum ConfigReportError {
    UnsupportedRevision,
    InvalidSubject,
    InvalidFields,
    Oversize,
    WrongEpoch { requested: ObservationEpoch, current: ObservationEpoch },
    GenerationUnavailable(DaemonRef),
    PeerQuota,
    Pressure,
    Unavailable,
}
pub(crate) trait ConfigurationRecorder: Send + Sync {
    fn record_resolved(&self, capture: OwnedConfigCapture) -> ConfigCaptureResult;
    fn record_report(&self, expected: ObservationEpoch, peer: &ConfigPeerBinding, report: ConfigReport) -> Result<ConfigSnapshotRef, ConfigReportError>;
    fn record_host_report(&self, expected: ObservationEpoch, peer: &ConfigPeerBinding, report: HostConfigReport) -> Result<ConfigSnapshotRef, ConfigReportError>;
}

impl ConfigSettingId {
    pub(crate) fn parse(name: &str) -> Result<Self, ConfigurationQueryError> {
        FIELDS.iter().find(|f| f.name == name).map(|f| Self(f.name.to_owned())).ok_or(ConfigurationQueryError::UnknownField)
    }
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ConfigSettingId {
    type Error = &'static str;
    fn try_from(name: String) -> Result<Self, Self::Error> {
        Self::parse(&name).map_err(|_| "unrecognized configuration setting")
    }
}
impl ConfigFieldId {
    pub(crate) fn setting(name: &str) -> Result<Self, ConfigurationQueryError> {
        ConfigSettingId::parse(name).map(Self::Setting)
    }
}
impl RulesRevision {
    /// The composition supplies its actual loaded build cohort, never client input.
    pub(crate) fn current(native_build_id: impl Into<String>) -> Self {
        Self { native_build_id: native_build_id.into(), field_catalog_revision: 1, resolution_revision: 1, disclosure_revision: 1 }
    }
}
impl SourceRead {
    pub(crate) fn new(id: u16, resource: SourceResource, at: ObservationTime, state: SourceReadState) -> Self {
        Self { id: SourceReadId(id), resource, read_revision: uuid::Uuid::new_v4().to_string(), at, state }
    }
}

/// Metadata supplied by the real loader's single selection/validation pass.
/// No candidate text or secret is accepted by this contract.
#[derive(Clone, Debug)]
pub(crate) struct ConfigFieldTrace {
    pub field: ConfigSettingId,
    pub resolution: FieldResolution,
    pub steps: Vec<ResolutionStep>,
}
#[derive(Clone, Debug)]
pub(crate) struct ConfigResolutionTrace {
    pub source_reads: Vec<SourceRead>,
    pub fields: Vec<ConfigFieldTrace>,
    /// The source owner establishes this while resolving presets; no reread here.
    pub terminal_is_builtin: bool,
}

/// Required originating-side port, implemented at the existing scoped loader.
pub(crate) trait ConfigResolutionSource {
    fn resolve(&self, scope: &Path, environment: &HashMap<String, String>) -> (AgentxConfig, ConfigResolutionTrace);
}

impl ConfigSnapshotBody {
    pub(crate) fn invocation(capture: InvocationCaptureRef, at: ObservationTime, rules: RulesRevision) -> Self {
        Self {
            subject: ConfigSubject::Invocation { capture: capture.clone() },
            resolution_scope: EvidenceValue::Unknown(UnknownReason::NotCaptured),
            stage: ConfigStage::InvocationInputs,
            purpose: ConfigPurpose::InputCapture,
            correlation: ConfigCorrelation { invocation: Some(capture), attempt: None },
            rules,
            captured_at: at,
            consistency: ReadConsistency::InputsOnly,
            source_reads: Vec::new(),
            fields: Vec::new(),
            consumed_fields: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Disclosure {
    Seconds,
    Boolean,
    Enum,
    Terminal,
    Path,
    Hidden,
    Concealed,
}

fn disclosure(field: &ConfigFieldId) -> Result<Disclosure, ConfigReportError> {
    use ContextFieldId as C;
    use ParticipantFieldId as P;
    Ok(match field {
        ConfigFieldId::Setting(id) => {
            let native = FIELDS.iter().find(|f| f.name == id.0).ok_or(ConfigReportError::InvalidFields)?;
            match id.as_str() {
                "timeout" | "subagent_timeout" => Disclosure::Seconds,
                "auto_approve" | "auto_trust_workspace" | "relay_enabled" => Disclosure::Boolean,
                "title_mode" | "claude_delivery" | "copilot_delivery" | "bootstrap_exclude" => Disclosure::Enum,
                "terminal" => Disclosure::Terminal,
                "headless_terminal" => Disclosure::Enum,
                "tmux_socket" | "tmux_config" => Disclosure::Path,
                "relay" | "relay_id" | "relay_token" | "relay_psk" => Disclosure::Concealed,
                "hints" | "notes" | "auto_subscribe" | "gemini_system_prompt" | "codex_system_prompt" | "name_export" => Disclosure::Hidden,
                _ if native.tool.is_some() => Disclosure::Hidden,
                _ => Disclosure::Concealed,
            }
        }
        ConfigFieldId::Participant(P::Timeout | P::SubagentTimeout) => Disclosure::Seconds,
        ConfigFieldId::Participant(P::Hints) => Disclosure::Hidden,
        ConfigFieldId::Context(C::StdinIsTty | C::StdoutIsTty) => Disclosure::Boolean,
        ConfigFieldId::Context(C::ExplicitNameSelection) => Disclosure::Concealed,
        ConfigFieldId::Context(C::Cwd | C::RequestedScope | C::CanonicalScope | C::ProviderHome | C::EffectiveWorkingDirectory | C::EffectiveDevRoot) => Disclosure::Path,
        ConfigFieldId::Context(C::Tool | C::LaunchRegime | C::LaunchBackend) => Disclosure::Enum,
    })
}

fn enum_allowed(field: &ConfigFieldId, value: &str) -> bool {
    match field {
        ConfigFieldId::Setting(id) => match id.as_str() {
            "title_mode" => crate::shared::VALID_TITLE_MODES.contains(&value),
            "claude_delivery" | "copilot_delivery" => crate::config::DELIVERY_MODES.contains(&value),
            "bootstrap_exclude" => crate::runtime::bootstrap::parse_excluded_parts(value).is_ok(),
            "headless_terminal" => matches!(value, "pty" | "tmux"),
            "terminal" => matches!(value, "default" | "print" | "here") || (value == value.to_ascii_lowercase() && crate::config::is_known_terminal_preset_pub(value)),
            _ => false,
        },
        ConfigFieldId::Context(ContextFieldId::Tool) => crate::integrations::registry::ALL.iter().any(|s| s.name == value),
        ConfigFieldId::Context(ContextFieldId::LaunchRegime) => matches!(value, "human_shell" | "contaminated_parent" | "run_here"),
        ConfigFieldId::Context(ContextFieldId::LaunchBackend) => matches!(value, "pty" | "stdio" | "shared_app_server" | "terminal" | "headless"),
        _ => false,
    }
}

fn safe_scalar(field: &ConfigFieldId, raw: &str) -> EvidenceValue<ConfigValue> {
    if raw.len() > 1024 {
        return EvidenceValue::Unknown(UnknownReason::Pressure);
    }
    let value = match disclosure(field) {
        Ok(Disclosure::Seconds) => raw.trim().parse::<u64>().ok().filter(|v| (1..=86400).contains(v)).map(ConfigValue::Seconds),
        Ok(Disclosure::Boolean) => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(ConfigValue::Boolean(true)),
            "0" | "false" | "no" | "off" | "" => Some(ConfigValue::Boolean(false)),
            _ => None,
        },
        Ok(Disclosure::Enum | Disclosure::Terminal) if enum_allowed(field, raw) => Some(ConfigValue::Text(raw.to_owned())),
        Ok(Disclosure::Hidden | Disclosure::Concealed | Disclosure::Terminal) => return EvidenceValue::Withheld,
        _ => None,
    };
    value.map(EvidenceValue::Present).unwrap_or(EvidenceValue::Unknown(UnknownReason::ObservationFailed))
}

/// Apply field policy to already resolved values. This never loads a file or
/// recreates source selection: absent trace stays unknown instead of inferred.
pub(crate) fn project_effective_config(config: &AgentxConfig, mut body: ConfigSnapshotBody, trace: ConfigResolutionTrace) -> Result<ConfigSnapshotBody, CaptureSkipReason> {
    if FIELDS.len() > MAX_FIELDS || trace.fields.len() > MAX_FIELDS || trace.source_reads.len() > MAX_SOURCE_READS || trace.fields.iter().any(|field| field.steps.len() > MAX_STEPS) {
        return Err(CaptureSkipReason::Oversize);
    }
    let mut seen = std::collections::HashSet::new();
    if trace.fields.iter().any(|field| !seen.insert(&field.field) || !FIELDS.iter().any(|native| native.name == field.field.as_str())) {
        return Err(CaptureSkipReason::ObservationFailed);
    }
    body.consistency = if trace.source_reads.len() > 1 { ReadConsistency::MultipleNonAtomicReads } else { ReadConsistency::SingleSourceRead };
    body.source_reads = trace.source_reads;
    body.fields.clear();
    for native in FIELDS.iter() {
        let field = ConfigFieldId::Setting(ConfigSettingId(native.name.to_owned()));
        let policy = disclosure(&field).map_err(|_| CaptureSkipReason::ObservationFailed)?;
        let entry = trace.fields.iter().find(|f| f.field.as_str() == native.name);
        let (resolution, steps) = match entry {
            Some(e) => (EvidenceValue::Present(e.resolution.clone()), e.steps.clone()),
            None => (EvidenceValue::Unknown(UnknownReason::NotCaptured), Vec::new()),
        };
        let value = match policy {
            Disclosure::Hidden | Disclosure::Concealed => EvidenceValue::Withheld,
            Disclosure::Terminal if !trace.terminal_is_builtin => EvidenceValue::Withheld,
            _ => config.get_field(native.name).map(|raw| safe_scalar(&field, &raw)).unwrap_or(EvidenceValue::Unknown(UnknownReason::NotCaptured)),
        };
        body.fields.push(FieldEvidence { field, value, resolution: if policy == Disclosure::Concealed { EvidenceValue::Withheld } else { resolution }, steps: if policy == Disclosure::Concealed { Vec::new() } else { steps } });
    }
    validate_body(&body).map_err(report_skip)?;
    Ok(body)
}

/// Source adapters use this for fixed context/path/categorical facts only.
pub(crate) fn projected_field(field: ConfigFieldId, value: EvidenceValue<ConfigValue>, resolution: FieldResolution) -> Result<FieldEvidence, ConfigReportError> {
    let policy = disclosure(&field)?;
    let result = FieldEvidence {
        field,
        value: if matches!(policy, Disclosure::Hidden | Disclosure::Concealed) { EvidenceValue::Withheld } else { value },
        resolution: if policy == Disclosure::Concealed { EvidenceValue::Withheld } else { EvidenceValue::Present(resolution) },
        steps: Vec::new(),
    };
    validate_field(&result)?;
    Ok(result)
}

fn report_skip(error: ConfigReportError) -> CaptureSkipReason {
    match error {
        ConfigReportError::Oversize => CaptureSkipReason::Oversize,
        ConfigReportError::Pressure | ConfigReportError::PeerQuota => CaptureSkipReason::Pressure,
        ConfigReportError::UnsupportedRevision => CaptureSkipReason::Unsupported,
        _ => CaptureSkipReason::ObservationFailed,
    }
}

fn valid_scope(value: &EvidenceValue<ScopeRef>) -> bool {
    match value {
        EvidenceValue::Present(scope) => scope.canonical_scope.is_absolute() && path_bounded(&scope.canonical_scope),
        EvidenceValue::Unknown(_) | EvidenceValue::Withheld => true,
        EvidenceValue::Absent => false,
    }
}
fn path_bounded(path: &Path) -> bool {
    path.to_str().is_some_and(|s| s.len() <= 4096)
}
fn projected_path(path: &Path) -> EvidenceValue<ConfigValue> {
    if path_bounded(path) { EvidenceValue::Present(ConfigValue::Path(path.to_path_buf())) } else { EvidenceValue::Unknown(UnknownReason::Pressure) }
}
fn validate_field(field: &FieldEvidence) -> Result<(), ConfigReportError> {
    let policy = disclosure(&field.field)?;
    if field.steps.len() > MAX_STEPS {
        return Err(ConfigReportError::Oversize);
    }
    if matches!(policy, Disclosure::Hidden | Disclosure::Concealed) && !matches!(field.value, EvidenceValue::Withheld) {
        return Err(ConfigReportError::InvalidFields);
    }
    if policy == Disclosure::Concealed && (!matches!(field.resolution, EvidenceValue::Withheld) || !field.steps.is_empty()) {
        return Err(ConfigReportError::InvalidFields);
    }
    if let EvidenceValue::Present(value) = &field.value {
        let valid = match (policy, value) {
            (Disclosure::Seconds, ConfigValue::Seconds(n)) => {
                if matches!(field.field, ConfigFieldId::Setting(_)) {
                    (1..=86400).contains(n)
                } else {
                    *n > 0
                }
            }
            (Disclosure::Boolean, ConfigValue::Boolean(_)) => true,
            (Disclosure::Enum | Disclosure::Terminal, ConfigValue::Text(v)) => v.len() <= 1024 && enum_allowed(&field.field, v),
            (Disclosure::Path, ConfigValue::Path(p)) => path_bounded(p),
            _ => false,
        };
        if !valid {
            return Err(ConfigReportError::InvalidFields);
        }
    }
    Ok(())
}

pub(crate) fn validate_body(body: &ConfigSnapshotBody) -> Result<usize, ConfigReportError> {
    if body.rules.field_catalog_revision != 1 || body.rules.resolution_revision != 1 || body.rules.disclosure_revision != 1 {
        return Err(ConfigReportError::UnsupportedRevision);
    }
    if body.rules.native_build_id.len() > 128 || body.rules.native_build_id.is_empty() || !valid_scope(&body.resolution_scope) {
        return Err(ConfigReportError::InvalidFields);
    }
    body.captured_at.clock.validate().map_err(|_| ConfigReportError::InvalidFields)?;
    if body.correlation.invocation.as_ref().is_some_and(|capture| !capture.invocation.validate()) || body.correlation.attempt.as_ref().is_some_and(|attempt| !attempt.validate()) {
        return Err(ConfigReportError::InvalidSubject);
    }
    if body.fields.len() > MAX_FIELDS || body.source_reads.len() > MAX_SOURCE_READS || body.consumed_fields.len() > MAX_FIELDS {
        return Err(ConfigReportError::Oversize);
    }
    let mut seen = std::collections::HashSet::new();
    for field in &body.fields {
        if !seen.insert(&field.field) {
            return Err(ConfigReportError::InvalidFields);
        }
        validate_field(field)?;
    }
    let mut reads = std::collections::HashSet::new();
    for read in &body.source_reads {
        let valid_receipt = read.read_revision.len() == 36 && uuid::Uuid::parse_str(&read.read_revision).is_ok_and(|id| id.get_version_num() == 4 && id.get_variant() == uuid::Variant::RFC4122);
        if read.id.0 == 0 || !reads.insert(read.id.0) || !valid_receipt {
            return Err(ConfigReportError::InvalidFields);
        }
        read.at.clock.validate().map_err(|_| ConfigReportError::InvalidFields)?;
    }
    let validate_source = |source: &ConfigSourceRef| {
        let receipt = source.read.as_ref().and_then(|id| body.source_reads.iter().find(|read| read.id == *id));
        match (&source.kind, &source.read, receipt) {
            (ConfigSourceKind::ScopeFile, Some(_), Some(read)) => matches!(read.resource, SourceResource::ConfigToml | SourceResource::TerminalPresets),
            (ConfigSourceKind::LaunchEnvironmentFile, Some(_), Some(read)) => read.resource == SourceResource::LaunchEnvironmentFile,
            (ConfigSourceKind::DomainRecord, Some(_), Some(read)) => read.resource == SourceResource::PersistentDevRoot,
            (ConfigSourceKind::ScopeFile | ConfigSourceKind::LaunchEnvironmentFile, _, _) => false,
            (_, None, _) => true,
            _ => false,
        }
    };
    for field in &body.fields {
        let valid_for_field = |source: &ConfigSourceRef| validate_source(source) && !(field.field == ConfigFieldId::Context(ContextFieldId::EffectiveDevRoot) && source.kind == ConfigSourceKind::DomainRecord && source.read.is_none());
        for step in &field.steps {
            if !valid_for_field(&step.source) {
                return Err(ConfigReportError::InvalidFields);
            }
        }
        if let EvidenceValue::Present(ref resolution) = field.resolution {
            match resolution {
                FieldResolution::Effective { source, .. } if !valid_for_field(source) => return Err(ConfigReportError::InvalidFields),
                FieldResolution::Defaulted { rejected, .. } if !valid_for_field(rejected) => return Err(ConfigReportError::InvalidFields),
                _ => {}
            }
        }
    }
    let mut consumed = std::collections::HashSet::new();
    for field in &body.consumed_fields {
        if !seen.contains(field) || !consumed.insert(field) {
            return Err(ConfigReportError::InvalidFields);
        }
    }
    if body.purpose == ConfigPurpose::DiagnosticReevaluation && !body.consumed_fields.is_empty() {
        return Err(ConfigReportError::InvalidFields);
    }
    match (&body.subject, &body.stage, &body.purpose) {
        (ConfigSubject::Invocation { capture }, ConfigStage::InvocationInputs, ConfigPurpose::InputCapture)
        | (ConfigSubject::Invocation { capture }, ConfigStage::ScopedEffective, ConfigPurpose::ActualResolution | ConfigPurpose::DiagnosticReevaluation)
        | (ConfigSubject::HostPlan { capture }, ConfigStage::LaunchPlanned, ConfigPurpose::LaunchPreparation) => {
            if body.correlation.invocation.as_ref() != Some(capture) {
                return Err(ConfigReportError::InvalidSubject);
            }
        }
        (ConfigSubject::DaemonStartup { .. }, ConfigStage::StartupInputs, ConfigPurpose::StartupCapture)
        | (ConfigSubject::ScopeDeclarations { .. }, ConfigStage::ScopedDeclarations, ConfigPurpose::DeclarationRead)
        | (ConfigSubject::Participant { .. }, ConfigStage::ParticipantState, ConfigPurpose::DomainProjection)
        | (ConfigSubject::Host { .. }, ConfigStage::HostReported, ConfigPurpose::ValidatedHostReport) => {}
        _ => return Err(ConfigReportError::InvalidSubject),
    }
    encoded_size(body, MAX_SNAPSHOT_BYTES).map_err(|_| ConfigReportError::Oversize)
}

/// Count bounded native serde output without allocating a second full buffer.
pub(crate) fn encoded_size<T: serde::Serialize>(value: &T, limit: usize) -> Result<usize, ()> {
    struct Counter {
        used: usize,
        limit: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let next = self.used.checked_add(bytes.len()).filter(|n| *n <= self.limit).ok_or_else(|| std::io::Error::other("configuration evidence size limit"))?;
            self.used = next;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { used: 0, limit };
    serde_json::to_writer(&mut counter, value).map_err(|_| ())?;
    Ok(counter.used)
}

pub(crate) struct CaptureInputs<'a> {
    pub invocation: &'a crate::service::InvocationContext,
    pub at: ObservationTime,
    pub rules: RulesRevision,
}
pub(crate) struct CaptureCurrent<'a> {
    pub inputs: CaptureInputs<'a>,
    pub explicitly_selected_scope: &'a Path,
    /// Supplied by the actual bound/passive scope resolver; never synthesized.
    pub resolution_scope: EvidenceValue<ScopeRef>,
    pub source: &'a dyn ConfigResolutionSource,
}

/// Clones share allocation and the total local retention budget. Distinct
/// `with_invocation` branches allocate distinct revisions from this one family.
#[derive(Clone, Debug)]
pub(crate) struct InvocationCaptureHandle {
    reference: InvocationCaptureRef,
    family: Arc<InvocationCaptureFamily>,
    next_snapshot: Arc<AtomicU32>,
}
#[derive(Debug)]
struct InvocationCaptureFamily {
    next_revision: AtomicU64,
    retained: Mutex<LocalCaptureRetention>,
}
#[derive(Debug)]
struct LocalCaptureRetention {
    snapshots: VecDeque<(LocalConfigSnapshot, usize)>,
    encoded_bytes: usize,
}

impl InvocationCaptureHandle {
    pub(crate) fn begin(invocation: ClientInvocationId) -> Self {
        Self {
            reference: InvocationCaptureRef { invocation, revision: 0 },
            family: Arc::new(InvocationCaptureFamily { next_revision: AtomicU64::new(1), retained: Mutex::new(LocalCaptureRetention { snapshots: VecDeque::new(), encoded_bytes: 0 }) }),
            next_snapshot: Arc::new(AtomicU32::new(1)),
        }
    }

    #[allow(deprecated, reason = "Atomic fetch_update supports MSRV 1.89; its try_update replacement is newer")]
    pub(crate) fn revise(&self) -> Result<Self, CaptureSkipReason> {
        let revision = self.family.next_revision.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| next.checked_add(1)).map_err(|_| CaptureSkipReason::Unsupported)?;
        Ok(Self { reference: InvocationCaptureRef { invocation: self.reference.invocation, revision }, family: Arc::clone(&self.family), next_snapshot: Arc::new(AtomicU32::new(1)) })
    }

    pub(crate) fn reference(&self) -> InvocationCaptureRef {
        self.reference.clone()
    }

    pub(crate) fn capture_invocation_inputs(&self, input: CaptureInputs<'_>) -> Result<LocalConfigSnapshot, CaptureSkipReason> {
        if FIELDS.len() > MAX_FIELDS - 5 {
            return Err(CaptureSkipReason::Oversize);
        }
        let mut body = ConfigSnapshotBody::invocation(self.reference(), input.at, input.rules);
        // Bound the existing lexical resolver's inputs before it clones a path.
        // This is capture pressure only; normal path selection remains unchanged.
        let requested = if path_bounded(&input.invocation.cwd) && ["AGENTX_DIR", "HOME", "USERPROFILE"].iter().all(|key| input.invocation.environment.get(*key).is_none_or(|value| value.len() <= 4096)) {
            let (path, _) = crate::paths::resolve_agentx_dir_from_env(&input.invocation.environment, &input.invocation.cwd);
            projected_path(&path)
        } else {
            EvidenceValue::Unknown(UnknownReason::Pressure)
        };
        for (field, value) in [
            (ContextFieldId::Cwd, projected_path(&input.invocation.cwd)),
            (ContextFieldId::RequestedScope, requested),
            (ContextFieldId::StdinIsTty, EvidenceValue::Present(ConfigValue::Boolean(input.invocation.stdin_is_tty))),
            (ContextFieldId::StdoutIsTty, EvidenceValue::Present(ConfigValue::Boolean(input.invocation.stdout_is_tty))),
        ] {
            body.fields.push(projected_field(ConfigFieldId::Context(field), value, FieldResolution::InputOnly).map_err(report_skip)?);
        }
        body.fields.push(projected_field(ConfigFieldId::Context(ContextFieldId::ExplicitNameSelection), EvidenceValue::Withheld, FieldResolution::InputOnly).map_err(report_skip)?);
        for native in FIELDS.iter() {
            let field = ConfigFieldId::Setting(ConfigSettingId(native.name.to_owned()));
            let policy = disclosure(&field).map_err(report_skip)?;
            let value = match policy {
                Disclosure::Hidden | Disclosure::Concealed | Disclosure::Terminal => EvidenceValue::Withheld,
                _ if native.is_relay() => EvidenceValue::Unknown(UnknownReason::NotCaptured),
                _ => input.invocation.environment.get(native.env).map(|s| safe_scalar(&field, s)).unwrap_or(EvidenceValue::Absent),
            };
            body.fields.push(FieldEvidence { field, value, resolution: if policy == Disclosure::Concealed { EvidenceValue::Withheld } else { EvidenceValue::Present(FieldResolution::InputOnly) }, steps: Vec::new() });
        }
        self.capture_resolved(body)
    }

    /// Explicit new source-side evaluation; the actual loader is injected and
    /// owns all file reads, selection and validation. Never used by inspection.
    pub(crate) fn capture_current_resolution(&self, input: CaptureCurrent<'_>) -> Result<LocalConfigSnapshot, CaptureSkipReason> {
        let (config, trace) = input.source.resolve(input.explicitly_selected_scope, &input.inputs.invocation.environment);
        let mut body = ConfigSnapshotBody::invocation(self.reference(), input.inputs.at, input.inputs.rules);
        body.stage = ConfigStage::ScopedEffective;
        body.purpose = ConfigPurpose::DiagnosticReevaluation;
        body.resolution_scope = input.resolution_scope;
        self.capture_resolved(project_effective_config(&config, body, trace)?)
    }

    pub(crate) fn capture_resolved(&self, body: ConfigSnapshotBody) -> Result<LocalConfigSnapshot, CaptureSkipReason> {
        if body.correlation.invocation.as_ref() != Some(&self.reference)
            || !matches!(&body.subject, ConfigSubject::Invocation { capture } | ConfigSubject::HostPlan { capture }
                if capture == &self.reference)
        {
            return Err(CaptureSkipReason::ObservationFailed);
        }
        validate_body(&body).map_err(report_skip)?;
        let mut retained = self.family.retained.try_lock().map_err(|_| CaptureSkipReason::Pressure)?;
        let sequence = self.next_snapshot.load(Ordering::Relaxed);
        let next = sequence.checked_add(1).ok_or(CaptureSkipReason::Unsupported)?;
        let snapshot = LocalConfigSnapshot { reference: LocalConfigSnapshotRef { capture: self.reference(), sequence }, body };
        let size = encoded_size(&snapshot, MAX_SNAPSHOT_BYTES).map_err(|_| CaptureSkipReason::Oversize)?;
        while retained.snapshots.len() >= 8 || retained.encoded_bytes.checked_add(size).is_none_or(|n| n > 256 * 1024) {
            if let Some((_, bytes)) = retained.snapshots.pop_front() {
                retained.encoded_bytes -= bytes;
            } else {
                return Err(CaptureSkipReason::Oversize);
            }
        }
        self.next_snapshot.store(next, Ordering::Relaxed);
        retained.encoded_bytes += size;
        retained.snapshots.push_back((snapshot.clone(), size));
        Ok(snapshot)
    }

    #[cfg_attr(not(test), allow(dead_code, reason = "Local historical lookup port is exercised by owner cases and awaits the diagnostic adapter"))]
    pub(crate) fn retained(&self, reference: &LocalConfigSnapshotRef) -> Result<Option<LocalConfigSnapshot>, CaptureSkipReason> {
        let retained = self.family.retained.try_lock().map_err(|_| CaptureSkipReason::Pressure)?;
        Ok(retained.snapshots.iter().find(|(s, _)| &s.reference == reference).map(|(s, _)| s.clone()))
    }
}

pub(crate) fn validate_selection(selection: &ConfigFieldSelection) -> Result<(), ConfigurationQueryError> {
    if let ConfigFieldSelection::Only(fields) = selection {
        if fields.is_empty() || fields.len() > MAX_FIELDS {
            return Err(ConfigurationQueryError::InvalidSelection);
        }
        let mut seen = std::collections::HashSet::new();
        for field in fields {
            disclosure(field).map_err(|_| ConfigurationQueryError::UnknownField)?;
            if !seen.insert(field) {
                return Err(ConfigurationQueryError::InvalidSelection);
            }
        }
    }
    Ok(())
}

fn scoped_stage(stage: &ConfigStage) -> bool {
    !matches!(stage, ConfigStage::InvocationInputs | ConfigStage::StartupInputs)
}

fn comparison_boundary(left: &ConfigSnapshotBody, right: &ConfigSnapshotBody) -> Option<ComparisonBoundary> {
    if left.stage != right.stage {
        return Some(ComparisonBoundary::DifferentStage);
    }
    if left.rules.field_catalog_revision != right.rules.field_catalog_revision || left.rules.resolution_revision != right.rules.resolution_revision {
        return Some(ComparisonBoundary::DifferentMeaningRevision);
    }
    if scoped_stage(&left.stage) {
        match (&left.resolution_scope, &right.resolution_scope) {
            (EvidenceValue::Present(a), EvidenceValue::Present(b)) if a == b => {}
            (EvidenceValue::Present(_), EvidenceValue::Present(_)) => return Some(ComparisonBoundary::DifferentScope),
            _ => return Some(ComparisonBoundary::ScopeUnestablished),
        }
    }
    None
}

fn relation(left: Option<&FieldEvidence>, right: Option<&FieldEvidence>) -> ValueRelation {
    use EvidenceValue as E;
    match (left.map(|f| &f.value), right.map(|f| &f.value)) {
        (Some(E::Withheld), _) | (_, Some(E::Withheld)) => ValueRelation::Unknown(ComparisonUnknown::Withheld),
        (None, _) | (_, None) => ValueRelation::Unknown(ComparisonUnknown::Unobserved),
        (Some(E::Unknown(_)), _) | (_, Some(E::Unknown(_))) => ValueRelation::Unknown(ComparisonUnknown::CaptureUnavailable),
        (Some(a), Some(b)) if a == b => ValueRelation::Equal,
        _ => ValueRelation::Different,
    }
}

fn effective_source(field: Option<&FieldEvidence>) -> Option<ConfigSourceKind> {
    match &field?.resolution {
        EvidenceValue::Present(FieldResolution::Effective { source, .. }) => Some(source.kind.clone()),
        EvidenceValue::Present(FieldResolution::Defaulted { .. }) => Some(ConfigSourceKind::BuiltInDefault),
        _ => None,
    }
}

pub(crate) fn compare_field_evidence(left: &ConfigSnapshotBody, right: &ConfigSnapshotBody, selection: &ConfigFieldSelection) -> Result<Vec<FieldComparison>, ConfigurationQueryError> {
    validate_selection(selection)?;
    let fields = match selection {
        ConfigFieldSelection::Only(fields) => fields.clone(),
        ConfigFieldSelection::All => {
            let mut ids = Vec::new();
            for field in left.fields.iter().chain(&right.fields) {
                if !ids.contains(&field.field) {
                    ids.push(field.field.clone());
                }
            }
            if ids.len() > MAX_FIELDS {
                return Err(ConfigurationQueryError::Oversize);
            }
            ids
        }
    };
    Ok(fields
        .into_iter()
        .map(|field| {
            let left = left.fields.iter().find(|e| e.field == field);
            let right = right.fields.iter().find(|e| e.field == field);
            let provenance = match (effective_source(left), effective_source(right)) {
                (Some(a), Some(b)) if a == b => ProvenanceRelation::SameClass,
                (Some(_), Some(_)) => ProvenanceRelation::DifferentClass,
                _ => ProvenanceRelation::Unknown,
            };
            let value = if matches!(disclosure(&field), Ok(Disclosure::Hidden | Disclosure::Concealed)) { ValueRelation::Unknown(ComparisonUnknown::Withheld) } else { relation(left, right) };
            FieldComparison { field, left: left.cloned(), right: right.cloned(), value, provenance }
        })
        .collect())
}

pub(crate) fn compare_snapshots(left: ConfigSnapshot, right: ConfigSnapshot, selection: &ConfigFieldSelection) -> Result<CompareConfigResult, ConfigurationQueryError> {
    validate_selection(selection)?;
    if left.reference.sequence == 0 || right.reference.sequence == 0 || left.reference.epoch.validate().is_err() || right.reference.epoch.validate().is_err() {
        return Err(ConfigurationQueryError::InvalidReference);
    }
    if left.reference.epoch.daemon.scope != right.reference.epoch.daemon.scope {
        return Ok(CompareConfigResult::Incomparable(ComparisonBoundary::DifferentScope));
    }
    if left.reference.epoch != right.reference.epoch {
        return Ok(CompareConfigResult::Incomparable(ComparisonBoundary::DifferentEpoch));
    }
    if let Some(boundary) = comparison_boundary(&left.body, &right.body) {
        return Ok(CompareConfigResult::Incomparable(boundary));
    }
    let fields = compare_field_evidence(&left.body, &right.body, selection)?;
    let a = &left.body;
    let b = &right.body;
    let basis = ComparisonBasis {
        left: left.reference,
        right: right.reference,
        left_subject: a.subject.clone(),
        right_subject: b.subject.clone(),
        left_resolution_scope: a.resolution_scope.clone(),
        right_resolution_scope: b.resolution_scope.clone(),
        stage: a.stage.clone(),
        left_purpose: a.purpose.clone(),
        right_purpose: b.purpose.clone(),
        left_captured_at: a.captured_at.clone(),
        right_captured_at: b.captured_at.clone(),
        left_provenance: left.provenance,
        right_provenance: right.provenance,
        left_rules: a.rules.clone(),
        right_rules: b.rules.clone(),
    };
    Ok(CompareConfigResult::Compared { basis, fields })
}

#[derive(Clone, Debug, serde::Serialize)]
#[cfg_attr(not(test), allow(dead_code, reason = "Origin-side comparison is exercised by owner cases and awaits the diagnostic adapter"))]
#[allow(clippy::large_enum_variant, reason = "Local comparison retains at most two bounded immutable bodies under the 128 KiB encoded result cap")]
pub(crate) enum LocalConfigComparison {
    Compared { left: LocalConfigSnapshotRef, right: LocalConfigSnapshotRef, left_body: ConfigSnapshotBody, right_body: ConfigSnapshotBody, fields: Vec<FieldComparison> },
    Incomparable(ComparisonBoundary),
}
#[cfg_attr(not(test), allow(dead_code, reason = "Origin-side comparison is exercised by owner cases and awaits the diagnostic adapter"))]
pub(crate) fn compare_local_snapshots(left: &LocalConfigSnapshot, right: &LocalConfigSnapshot, selection: &ConfigFieldSelection) -> Result<LocalConfigComparison, ConfigurationQueryError> {
    validate_selection(selection)?;
    if let Some(boundary) = comparison_boundary(&left.body, &right.body) {
        return Ok(LocalConfigComparison::Incomparable(boundary));
    }
    let fields = compare_field_evidence(&left.body, &right.body, selection)?;
    let result = LocalConfigComparison::Compared { left: left.reference.clone(), right: right.reference.clone(), left_body: left.body.clone(), right_body: right.body.clone(), fields };
    encoded_size(&result, 128 * 1024).map_err(|_| ConfigurationQueryError::Oversize)?;
    Ok(result)
}
