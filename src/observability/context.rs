//! Shared observation identity, provenance and comparable process-clock values.
//!
//! Constructors and serde validate value shape only: the passive scope adapter
//! canonicalizes paths, the actual lease supplies daemon identity, and the
//! Application supplies committed generation. These values grant no authority.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, Serialize};

/// Canonical local data scope, supplied by an existing resolver or lease.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ScopeRef {
    #[serde(deserialize_with = "deserialize_scope")]
    #[schema(value_type = String)]
    pub canonical_scope: PathBuf,
}

/// One daemon incarnation within a canonical scope; independent of PID/build.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct DaemonRef {
    pub scope: ScopeRef,
    #[serde(deserialize_with = "deserialize_instance")]
    pub daemon_instance: String,
}

/// Dataset fence: daemon incarnation plus the exact committed DB generation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservationEpoch {
    pub daemon: DaemonRef,
    #[serde(deserialize_with = "deserialize_generation")]
    pub database_generation: String,
}

/// UUID v4 identifying one retained process-clock origin, never OS authority.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
pub(crate) struct ClockId(#[serde(deserialize_with = "deserialize_clock_id")] pub String);

/// Whole milliseconds since the Unix epoch; chronology, not causal ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
pub(crate) struct UnixMillis(pub u64);

/// Whole milliseconds from the origin identified by an accompanying ClockId.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
pub(crate) struct MonotonicMillis(pub u64);

/// Nonnegative elapsed whole milliseconds; zero is a valid observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct DurationMillis(pub u64);

/// A producer's clock-domain time; unavailable wall time remains None.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ObservationTime {
    pub clock: ClockId,
    pub monotonic_ms: MonotonicMillis,
    pub unix_ms: Option<UnixMillis>,
}

/// Where a fact was obtained; provenance never implies authentication.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum EvidenceOrigin {
    OsObserved,
    DaemonBoundary,
    ApplicationResolved,
    StoredDomainFact,
    ClientReported,
    HostReported,
    Derived,
}

/// A bounded observation cannot establish this value for the stated reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum UnknownReason {
    NotCaptured,
    NotSupported,
    ObservationFailed,
    Expired,
    Evicted,
    Pressure,
    GenerationUnavailable,
}

/// Original producer capture/attempt time, distinct from journal retention time.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct EvidenceStamp {
    pub origin: EvidenceOrigin,
    pub at: ObservationTime,
}

/// Absent requires a complete positive lookup; Unknown/Withheld prove no absence.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) enum EvidenceValue<T> {
    Present(T),
    Absent,
    Unknown(UnknownReason),
    Withheld,
}

/// A captured value with source provenance; freshness policy belongs to its owner.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct Evidence<T> {
    pub value: EvidenceValue<T>,
    pub stamp: EvidenceStamp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ObservationValueError {
    #[error("observation scope must be absolute")]
    ScopeNotAbsolute,
    #[error("daemon instance must be nonempty")]
    EmptyDaemonInstance,
    #[error("database generation must be a committed UUID")]
    InvalidDatabaseGeneration,
    #[error("clock identity must be a UUID v4")]
    InvalidClockId,
    #[error("observation times have different clock origins")]
    ClockMismatch,
    #[error("observation time is reversed or cannot fit whole milliseconds")]
    TimeOverflow,
}

impl ScopeRef {
    /// Validate an already canonicalized value without filesystem I/O.
    pub(crate) fn from_canonical_scope(canonical_scope: PathBuf) -> Result<Self, ObservationValueError> {
        let value = Self { canonical_scope };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn validate(&self) -> Result<(), ObservationValueError> {
        if self.canonical_scope.is_absolute() { Ok(()) } else { Err(ObservationValueError::ScopeNotAbsolute) }
    }
}

impl DaemonRef {
    /// Composition supplies the actual lease instance; this does not authenticate it.
    pub(crate) fn from_lease(scope: ScopeRef, daemon_instance: String) -> Result<Self, ObservationValueError> {
        let value = Self { scope, daemon_instance };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn validate(&self) -> Result<(), ObservationValueError> {
        self.scope.validate()?;
        if self.daemon_instance.is_empty() { Err(ObservationValueError::EmptyDaemonInstance) } else { Ok(()) }
    }
}

impl ObservationEpoch {
    /// Reject unavailable generation; never create or normalize a generation.
    pub(crate) fn from_committed_generation(daemon: DaemonRef, database_generation: String) -> Result<Self, ObservationValueError> {
        let value = Self { daemon, database_generation };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn validate(&self) -> Result<(), ObservationValueError> {
        self.daemon.validate()?;
        uuid::Uuid::parse_str(&self.database_generation).map(|_| ()).map_err(|_| ObservationValueError::InvalidDatabaseGeneration)
    }
}

impl ClockId {
    pub(crate) fn parse(value: String) -> Result<Self, ObservationValueError> {
        let value = Self(value);
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn validate(&self) -> Result<(), ObservationValueError> {
        match uuid::Uuid::parse_str(&self.0) {
            Ok(id) if id.get_version_num() == 4 && id.get_variant() == uuid::Variant::RFC4122 => Ok(()),
            _ => Err(ObservationValueError::InvalidClockId),
        }
    }
}

impl DurationMillis {
    pub(crate) fn from_duration(value: Duration) -> Result<Self, ObservationValueError> {
        u64::try_from(value.as_millis()).map(Self).map_err(|_| ObservationValueError::TimeOverflow)
    }
}

impl MonotonicMillis {
    // Retain the shared checked-deadline contract; current adapters use Instant
    // deadlines, so only the rollover cases exercise this operation today.
    #[allow(dead_code)]
    pub(crate) fn checked_add(self, duration: DurationMillis) -> Result<Self, ObservationValueError> {
        self.0.checked_add(duration.0).map(Self).ok_or(ObservationValueError::TimeOverflow)
    }
}

impl ObservationTime {
    pub(crate) fn elapsed_since(&self, earlier: &Self) -> Result<DurationMillis, ObservationValueError> {
        self.clock.validate()?;
        earlier.clock.validate()?;
        if self.clock != earlier.clock {
            return Err(ObservationValueError::ClockMismatch);
        }
        self.monotonic_ms.0.checked_sub(earlier.monotonic_ms.0).map(DurationMillis).ok_or(ObservationValueError::TimeOverflow)
    }
}

/// One retained process-clock origin, explicitly supplied by composition.
/// Clones share the origin/ID; constructing another clock creates a new domain.
#[derive(Clone, Debug)]
pub(crate) struct ObservationClock {
    id: ClockId,
    origin: Instant,
}

impl ObservationClock {
    pub(crate) fn new() -> Self {
        Self { id: ClockId(uuid::Uuid::new_v4().to_string()), origin: Instant::now() }
    }

    pub(crate) fn id(&self) -> &ClockId {
        &self.id
    }

    pub(crate) fn now(&self) -> Result<ObservationTime, ObservationValueError> {
        let mut value = self.at_instant(Instant::now())?;
        value.unix_ms = SystemTime::now().duration_since(UNIX_EPOCH).ok().and_then(|duration| u64::try_from(duration.as_millis()).ok()).map(UnixMillis);
        Ok(value)
    }

    /// Convert a retained local Instant, including a future deadline. Its wall
    /// time is not inferred; an Instant before this origin is rejected.
    pub(crate) fn at_instant(&self, at: Instant) -> Result<ObservationTime, ObservationValueError> {
        let elapsed = at.checked_duration_since(self.origin).ok_or(ObservationValueError::TimeOverflow)?;
        let millis = DurationMillis::from_duration(elapsed)?;
        Ok(ObservationTime { clock: self.id.clone(), monotonic_ms: MonotonicMillis(millis.0), unix_ms: None })
    }
}

impl Default for ObservationClock {
    fn default() -> Self {
        Self::new()
    }
}

fn deserialize_scope<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    let value = PathBuf::deserialize(deserializer)?;
    ScopeRef::from_canonical_scope(value).map(|scope| scope.canonical_scope).map_err(serde::de::Error::custom)
}

fn deserialize_instance<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value.is_empty() { Err(serde::de::Error::custom(ObservationValueError::EmptyDaemonInstance)) } else { Ok(value) }
}

fn deserialize_generation<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    uuid::Uuid::parse_str(&value).map_err(|_| serde::de::Error::custom(ObservationValueError::InvalidDatabaseGeneration))?;
    Ok(value)
}

fn deserialize_clock_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    ClockId::parse(String::deserialize(deserializer)?).map(|clock| clock.0).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const GENERATION: &str = "11111111-1111-4111-8111-111111111111";
    const OTHER_GENERATION: &str = "22222222-2222-4222-8222-222222222222";
    const CLOCK: &str = "33333333-3333-4333-8333-333333333333";
    const OTHER_CLOCK: &str = "44444444-4444-4444-8444-444444444444";

    fn canonical_root() -> PathBuf {
        if cfg!(windows) { PathBuf::from(r"C:\fixtures\observation") } else { PathBuf::from("/fixtures/observation") }
    }

    fn daemon(instance: &str) -> DaemonRef {
        DaemonRef::from_lease(ScopeRef::from_canonical_scope(canonical_root()).unwrap(), instance.to_owned()).unwrap()
    }

    fn time(clock: &str, monotonic_ms: u64, unix_ms: Option<u64>) -> ObservationTime {
        ObservationTime { clock: ClockId::parse(clock.to_owned()).unwrap(), monotonic_ms: MonotonicMillis(monotonic_ms), unix_ms: unix_ms.map(UnixMillis) }
    }

    #[test]
    fn scope_identity_uses_supplied_canonical_root_without_resolving_aliases() {
        // A real resolver supplies this same canonical value for two aliases.
        // Value construction intentionally performs no filesystem operation.
        let direct = ScopeRef::from_canonical_scope(canonical_root()).unwrap();
        let alias_resolved_by_adapter = ScopeRef::from_canonical_scope(canonical_root()).unwrap();
        assert_eq!(direct, alias_resolved_by_adapter);
        assert_ne!(direct, ScopeRef::from_canonical_scope(canonical_root().join("another-scope")).unwrap());
        assert_eq!(ScopeRef::from_canonical_scope(PathBuf::from("relative")), Err(ObservationValueError::ScopeNotAbsolute));
    }

    #[test]
    fn restart_and_reset_are_independent_identity_fences() {
        let first = ObservationEpoch::from_committed_generation(daemon("daemon-a"), GENERATION.to_owned()).unwrap();
        let restarted = ObservationEpoch::from_committed_generation(daemon("daemon-b"), GENERATION.to_owned()).unwrap();
        let reset = ObservationEpoch::from_committed_generation(daemon("daemon-a"), OTHER_GENERATION.to_owned()).unwrap();
        assert_ne!(first, restarted);
        assert_eq!(first.database_generation, restarted.database_generation);
        assert_ne!(first, reset);
        assert_eq!(first.daemon, reset.daemon);
    }

    #[test]
    fn missing_generation_cannot_construct_a_dataset_epoch() {
        let daemon = daemon("initializing");
        assert_eq!(ObservationEpoch::from_committed_generation(daemon.clone(), String::new()), Err(ObservationValueError::InvalidDatabaseGeneration));
        assert_eq!(ObservationEpoch::from_committed_generation(daemon.clone(), "not-a-generation".to_owned()), Err(ObservationValueError::InvalidDatabaseGeneration));
        let process_only = Evidence::<String> { value: EvidenceValue::Unknown(UnknownReason::GenerationUnavailable), stamp: EvidenceStamp { origin: EvidenceOrigin::DaemonBoundary, at: time(CLOCK, 0, None) } };
        assert_eq!(daemon.daemon_instance, "initializing");
        assert_ne!(process_only.value, EvidenceValue::Present(String::new()));
        assert_ne!(process_only.value, EvidenceValue::Absent);
    }

    #[test]
    fn serde_rejects_invalid_nested_references_at_the_boundary() {
        let valid = ObservationEpoch::from_committed_generation(daemon("daemon-a"), GENERATION.to_owned()).unwrap();
        let encoded = serde_json::to_value(&valid).unwrap();
        assert_eq!(serde_json::from_value::<ObservationEpoch>(encoded.clone()).unwrap(), valid);
        for invalid in [
            json!({"daemon": {"scope": {"canonical_scope": "relative"}, "daemon_instance": "a"}, "database_generation": GENERATION}),
            json!({"daemon": {"scope": {"canonical_scope": canonical_root()}, "daemon_instance": ""}, "database_generation": GENERATION}),
            json!({"daemon": encoded["daemon"], "database_generation": ""}),
            json!({"daemon": encoded["daemon"], "database_generation": "invalid"}),
        ] {
            assert!(serde_json::from_value::<ObservationEpoch>(invalid).is_err());
        }
        assert_eq!(DaemonRef::from_lease(ScopeRef::from_canonical_scope(canonical_root()).unwrap(), String::new()), Err(ObservationValueError::EmptyDaemonInstance));
    }

    #[test]
    fn clock_identity_requires_a_v4_uuid_in_native_and_serde_construction() {
        for invalid in ["", "not-a-clock", "00000000-0000-0000-0000-000000000000", "11111111-1111-1111-8111-111111111111"] {
            assert_eq!(ClockId::parse(invalid.to_owned()), Err(ObservationValueError::InvalidClockId));
            assert!(serde_json::from_value::<ClockId>(json!(invalid)).is_err());
        }
        let valid = ClockId::parse(CLOCK.to_owned()).unwrap();
        assert_eq!(serde_json::from_value::<ClockId>(json!(CLOCK)).unwrap(), valid);
        assert_eq!(serde_json::to_value(valid).unwrap(), json!(CLOCK));
    }

    #[test]
    fn elapsed_time_uses_one_clock_even_if_wall_time_moves_backwards() {
        let start = time(CLOCK, 10, Some(1000));
        let end = time(CLOCK, 15, Some(900));
        assert_eq!(end.elapsed_since(&start), Ok(DurationMillis(5)));
        assert_eq!(start.elapsed_since(&start), Ok(DurationMillis(0)));
        assert_eq!(start.elapsed_since(&end), Err(ObservationValueError::TimeOverflow));
        assert_eq!(time(OTHER_CLOCK, 15, Some(1000)).elapsed_since(&start), Err(ObservationValueError::ClockMismatch));
    }

    #[test]
    fn duration_and_deadline_arithmetic_reject_rollover_instead_of_wrapping() {
        assert_eq!(DurationMillis::from_duration(Duration::ZERO), Ok(DurationMillis(0)));
        assert_eq!(DurationMillis::from_duration(Duration::from_secs(u64::MAX)), Err(ObservationValueError::TimeOverflow));
        assert_eq!(MonotonicMillis(u64::MAX).checked_add(DurationMillis(1)), Err(ObservationValueError::TimeOverflow));
        assert_eq!(MonotonicMillis(u64::MAX).checked_add(DurationMillis(0)), Ok(MonotonicMillis(u64::MAX)));
    }

    #[test]
    fn retained_clock_clones_share_origin_but_new_clocks_do_not() {
        let clock = ObservationClock::new();
        let clone = clock.clone();
        let instant = clock.origin + Duration::from_millis(5);
        assert_eq!(clock.at_instant(instant), clone.at_instant(instant));
        assert_eq!(clock.at_instant(instant).unwrap().monotonic_ms, MonotonicMillis(5));
        assert_eq!(clock.at_instant(instant).unwrap().unix_ms, None);
        let before = clock.origin.checked_sub(Duration::from_millis(1)).unwrap();
        assert_eq!(clock.at_instant(before), Err(ObservationValueError::TimeOverflow));
        assert_eq!(clock.now().unwrap().clock, *clock.id());
        assert_ne!(clock.id(), ObservationClock::new().id());
    }

    #[test]
    fn provenance_and_absence_states_survive_serialization_without_conflation() {
        let stamp = EvidenceStamp { origin: EvidenceOrigin::ClientReported, at: time(CLOCK, 20, None) };
        for value in [EvidenceValue::Present(false), EvidenceValue::Absent, EvidenceValue::Unknown(UnknownReason::NotCaptured), EvidenceValue::Withheld] {
            let evidence = Evidence { value, stamp: stamp.clone() };
            assert_eq!(serde_json::from_value::<Evidence<bool>>(serde_json::to_value(&evidence).unwrap()).unwrap(), evidence);
        }
        assert_ne!(EvidenceValue::<bool>::Absent, EvidenceValue::Unknown(UnknownReason::NotCaptured));
        assert_ne!(EvidenceValue::<bool>::Withheld, EvidenceValue::Unknown(UnknownReason::NotCaptured));
    }
}
