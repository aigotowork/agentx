//! Same-pass resolver receipts. This adapter records decisions already made by
//! config.rs; it never chooses, coerces or validates a configuration candidate.

use crate::observability::configuration::*;
use std::cell::RefCell;

thread_local! { static TRACE: RefCell<Option<ConfigResolutionTrace>> = const { RefCell::new(None) }; }

pub(super) struct ResolutionCapture {
    previous: Option<ConfigResolutionTrace>,
    finished: bool,
}

impl ResolutionCapture {
    pub(super) fn begin() -> Self {
        let fields = super::FIELDS
            .iter()
            .filter_map(|field| {
                Some(ConfigFieldTrace {
                    field: ConfigSettingId::parse(field.name).ok()?,
                    resolution: FieldResolution::Effective { source: source(ConfigSourceKind::BuiltInDefault, None), normalized: false },
                    steps: vec![ResolutionStep { source: source(ConfigSourceKind::BuiltInDefault, None), disposition: CandidateDisposition::Selected, validation: CandidateValidation::Valid }],
                })
            })
            .collect();
        let current = ConfigResolutionTrace { source_reads: Vec::new(), fields, terminal_is_builtin: true };
        let previous = TRACE.with(|slot| slot.replace(Some(current)));
        Self { previous, finished: false }
    }

    pub(super) fn finish(mut self) -> ConfigResolutionTrace {
        let trace = TRACE.with(|slot| slot.replace(self.previous.take())).unwrap_or(ConfigResolutionTrace { source_reads: Vec::new(), fields: Vec::new(), terminal_is_builtin: false });
        self.finished = true;
        trace
    }
}

impl Drop for ResolutionCapture {
    fn drop(&mut self) {
        if !self.finished {
            TRACE.with(|slot| {
                slot.replace(self.previous.take());
            });
        }
    }
}

fn source(kind: ConfigSourceKind, read: Option<SourceReadId>) -> ConfigSourceRef {
    ConfigSourceRef { kind, read }
}

pub(super) fn source_read(resource: SourceResource, state: SourceReadState) {
    TRACE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(trace) = slot.as_mut() else {
            return;
        };
        // Keep one excess receipt to force the owner's oversize outcome.
        if trace.source_reads.len() > MAX_SOURCE_READS {
            return;
        }
        let Some(at) = crate::service::context::observation_time() else {
            trace.fields.clear();
            return;
        };
        trace.source_reads.push(SourceRead::new(trace.source_reads.len() as u16 + 1, resource, at, state));
    });
}

pub(super) fn parse_errors(errors: &super::ConfigErrors) {
    TRACE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(trace) = slot.as_mut() else {
            return;
        };
        let read = trace.source_reads.iter().find(|r| r.resource == SourceResource::ConfigToml).map(|r| r.id.clone());
        for row in &mut trace.fields {
            if errors.contains_key(row.field.as_str()) {
                let issue = if row.field.as_str() == "terminal" { ConfigIssueCode::UnsafeTerminal } else { ConfigIssueCode::WrongScalarType };
                let rejected = source(ConfigSourceKind::ScopeFile, read.clone());
                row.resolution = FieldResolution::Defaulted { rejected: rejected.clone(), issue: issue.clone() };
                row.steps.push(ResolutionStep { source: rejected, disposition: CandidateDisposition::Rejected, validation: CandidateValidation::Invalid(issue) });
            }
        }
    });
}

/// A declaration projection describes this exact parsed file read only. It
/// never applies environment precedence, coerces a string or validates again.
pub(super) fn declarations(path: &std::path::Path, values: &std::collections::HashMap<String, super::TomlFieldValue>, errors: &super::ConfigErrors) {
    use crate::observability::context::{EvidenceValue, UnknownReason};
    if crate::service::context::active_observation().is_none() {
        return;
    }
    let read = TRACE.with(|slot| slot.borrow().as_ref()?.source_reads.iter().rev().find(|read| read.resource == SourceResource::ConfigToml).cloned());
    let Some(read) = read else {
        return;
    };
    let mut fields = Vec::new();
    for native in super::FIELDS.iter() {
        let Ok(field) = ConfigFieldId::setting(native.name) else {
            continue;
        };
        let value = if native.name == "terminal" {
            EvidenceValue::Withheld
        } else {
            match values.get(native.name) {
                Some(super::TomlFieldValue::Int(value)) if matches!(native.kind, super::FieldKind::Int) => {
                    u64::try_from(*value).ok().filter(|value| (1..=86400).contains(value)).map(|value| EvidenceValue::Present(ConfigValue::Seconds(value))).unwrap_or(EvidenceValue::Unknown(UnknownReason::ObservationFailed))
                }
                Some(super::TomlFieldValue::Bool(value)) => EvidenceValue::Present(ConfigValue::Boolean(*value)),
                Some(super::TomlFieldValue::Str(value)) if native.kind == super::FieldKind::Str => EvidenceValue::Present(ConfigValue::Text(value.clone())),
                Some(_) => EvidenceValue::Unknown(UnknownReason::NotCaptured),
                None if errors.contains_key(native.name) || read.state != SourceReadState::Read => EvidenceValue::Unknown(UnknownReason::ObservationFailed),
                None => EvidenceValue::Absent,
            }
        };
        let Ok(mut evidence) = projected_field(field, value, FieldResolution::InputOnly) else {
            continue;
        };
        if !matches!(evidence.resolution, EvidenceValue::Withheld) {
            evidence.steps.push(ResolutionStep { source: source(ConfigSourceKind::ScopeFile, Some(read.id.clone())), disposition: CandidateDisposition::NotConsulted, validation: CandidateValidation::NotEvaluated });
        }
        fields.push(evidence);
    }
    if let Some(scope) = path.parent() {
        crate::service::context::scoped_declarations(scope, read, fields);
    }
}

pub(super) fn selected(field: &super::ConfigField, from_environment: bool, file_present: bool) {
    TRACE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(trace) = slot.as_mut() else {
            return;
        };
        let read = trace.source_reads.iter().find(|r| r.resource == SourceResource::ConfigToml).map(|r| r.id.clone());
        let Some(row) = trace.fields.iter_mut().find(|row| row.field.as_str() == field.name) else {
            return;
        };
        row.steps[0].disposition = CandidateDisposition::Shadowed;
        if from_environment {
            for step in row.steps.iter_mut().filter(|s| s.source.kind == ConfigSourceKind::ScopeFile) {
                step.disposition = CandidateDisposition::Shadowed;
            }
            if file_present && !row.steps.iter().any(|s| s.source.kind == ConfigSourceKind::ScopeFile) {
                row.steps.push(ResolutionStep { source: source(ConfigSourceKind::ScopeFile, read.clone()), disposition: CandidateDisposition::Shadowed, validation: CandidateValidation::NotEvaluated });
            }
        }
        let selected = source(if from_environment { ConfigSourceKind::InvocationEnvironment } else { ConfigSourceKind::ScopeFile }, if from_environment { None } else { read });
        row.steps.push(ResolutionStep { source: selected.clone(), disposition: CandidateDisposition::Selected, validation: CandidateValidation::Valid });
        row.resolution = FieldResolution::Effective { source: selected, normalized: false };
    });
}

pub(super) fn ignored_empty(name: &str) {
    update_field(name, |row| {
        if let Some(step) = row.steps.last_mut() {
            step.disposition = CandidateDisposition::IgnoredEmpty;
        }
        row.steps[0].disposition = CandidateDisposition::Selected;
        row.resolution = FieldResolution::Effective { source: source(ConfigSourceKind::BuiltInDefault, None), normalized: false };
    });
}

pub(super) fn rejected(name: &str, issue: ConfigIssueCode) {
    update_field(name, |row| {
        if let Some(step) = row.steps.last_mut() {
            step.disposition = CandidateDisposition::Rejected;
            step.validation = CandidateValidation::Invalid(issue.clone());
            row.resolution = FieldResolution::Defaulted { rejected: step.source.clone(), issue };
        }
        row.steps[0].disposition = CandidateDisposition::Selected;
    });
}

pub(super) fn normalized(name: &str, changed: bool) {
    update_field(name, |row| {
        if let FieldResolution::Effective { normalized, .. } = &mut row.resolution {
            *normalized |= changed;
        }
    });
}

pub(super) fn terminal_builtin(value: bool) {
    TRACE.with(|slot| {
        if let Some(trace) = slot.borrow_mut().as_mut() {
            trace.terminal_is_builtin = value;
        }
    });
}

fn update_field(name: &str, update: impl FnOnce(&mut ConfigFieldTrace)) {
    TRACE.with(|slot| {
        if let Some(trace) = slot.borrow_mut().as_mut()
            && let Some(row) = trace.fields.iter_mut().find(|row| row.field.as_str() == name)
        {
            update(row);
        }
    });
}

/// Explicit diagnostic reevaluation uses the actual scoped loader once.
pub(crate) struct NativeConfigResolutionSource;
impl ConfigResolutionSource for NativeConfigResolutionSource {
    fn resolve(&self, scope: &std::path::Path, environment: &std::collections::HashMap<String, String>) -> (super::AgentxConfig, ConfigResolutionTrace) {
        let (config, _, trace) = super::AgentxConfig::load_for_scope_observed(scope, environment);
        (config, trace)
    }
}
