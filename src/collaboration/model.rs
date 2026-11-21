//! Pure work aggregate rules. Storage/effects are composed by store.rs.
use super::service::*;

pub(super) struct PendingMessage {
    pub id: String,
    pub kind: MessageKind,
    pub text: String,
    pub roles: Vec<Role>,
}

impl Work {
    pub(super) fn brief(&self) -> &Brief {
        self.briefs.last().expect("work always has a brief")
    }
    pub(super) fn assignment(&self, role: &Role) -> &Assignment {
        let id = if *role == Role::Implementer { &self.implementer } else { &self.checker };
        self.assignments.iter().find(|a| &a.id == id).expect("current assignment exists")
    }

    pub(super) fn create(reference: WorkRef, input: BriefInput, implementer: crate::agents::attachments::ParticipantRef, checker: crate::agents::attachments::ParticipantRef, at: String) -> Result<Self, Failure> {
        validate_brief(&input)?;
        if implementer == checker {
            return Err(invalid("checker", "must be independent from implementer"));
        }
        let implementer_id = format!("{}-implementer", reference.id);
        let checker_id = format!("{}-checker", reference.id);
        Ok(Self {
            reference,
            revision: 1,
            briefs: vec![Brief { revision: 1, input }],
            assignments: vec![
                Assignment { id: implementer_id.clone(), role: Role::Implementer, participant: implementer, dispatched_brief: None },
                Assignment { id: checker_id.clone(), role: Role::Checker, participant: checker, dispatched_brief: None },
            ],
            implementer: implementer_id,
            checker: checker_id,
            messages: vec![],
            progress: vec![],
            questions: vec![],
            results: vec![],
            check_requests: vec![],
            checks: vec![],
            decisions: vec![],
            created_at: at,
        })
    }

    pub(super) fn closed(&self) -> bool {
        self.decisions.iter().any(|d| matches!(d.kind, DecisionKind::Accepted))
    }

    /// Eligibility is derived from retained domain facts, never runtime activity.
    pub(super) fn eligibility(&self) -> Eligibility {
        let mut reasons = Vec::new();
        let closed = self.closed();
        if closed {
            reasons.push("Accepted work is closed".into());
        }
        if self.questions.iter().any(|q| q.answer.is_none()) {
            reasons.push("Open questions remain".into());
        }
        let Some(result) = self.results.last() else {
            reasons.push("No published result".into());
            return Eligibility { selection: None, reasons, closed };
        };
        if result.brief_revision != self.brief().revision || result.assignment_id != self.implementer {
            reasons.push("Result belongs to a previous brief or implementer".into());
        }
        if self.decisions.iter().any(|d| matches!(d.kind, DecisionKind::ChangesRequested) && d.result_id == result.id) {
            reasons.push("This result has outstanding requested changes".into());
        }
        let Some(request) = self.check_requests.last() else {
            reasons.push("Independent check not requested".into());
            return Eligibility { selection: None, reasons, closed };
        };
        if request.result_id != result.id || request.brief_revision != self.brief().revision || request.assignment_id != self.checker {
            reasons.push("Check request belongs to a previous result, brief or checker".into());
        }
        let Some(check) = self.checks.iter().rev().find(|c| c.input.request_id == request.id) else {
            reasons.push("Requested check not reported".into());
            return Eligibility { selection: None, reasons, closed };
        };
        if check.assignment_id != self.checker || check.input.result_id != result.id || check.brief_revision != self.brief().revision || check.input.verdict != Verdict::Passed {
            reasons.push("Current independent check has not passed".into());
        }
        if check.input.observed_fingerprint != result.input.source.revision {
            reasons.push("Observed source does not match the published result".into());
        }
        let publisher = self.assignments.iter().find(|a| a.id == result.assignment_id);
        if publisher.is_none_or(|a| a.participant == self.assignment(&Role::Checker).participant) {
            reasons.push("Checker is not independent from the publisher".into());
        }
        if self.brief().input.criteria.iter().filter(|c| c.required).any(|criterion| !check.input.criteria.iter().any(|c| c.criterion_id == criterion.id && c.verdict == Verdict::Passed)) {
            reasons.push("Required criteria are not completely covered".into());
        }
        let selection = if reasons.is_empty() { Some(ResultSelection { result_id: result.id.clone(), check_id: check.id.clone(), brief_revision: self.brief().revision }) } else { None };
        Eligibility { selection, reasons, closed }
    }

    pub(super) fn summary(&self) -> WorkSummary {
        WorkSummary {
            reference: self.reference.clone(),
            revision: self.revision,
            title: self.brief().input.title.clone(),
            brief_revision: self.brief().revision,
            implementer: self.assignment(&Role::Implementer).participant.clone(),
            checker: self.assignment(&Role::Checker).participant.clone(),
            open_questions: self.questions.iter().filter(|q| q.answer.is_none()).count() as u32,
            current_result: self.results.last().map(|r| r.id.clone()),
            closed: self.closed(),
        }
    }

    fn set_brief(&mut self, input: BriefInput) -> Result<(), Failure> {
        validate_brief(&input)?;
        self.briefs.push(Brief { revision: self.brief().revision + 1, input });
        Ok(())
    }
    fn dispatched(&mut self, role: &Role) {
        let revision = self.brief().revision;
        let id = self.assignment(role).id.clone();
        self.assignments.iter_mut().find(|a| a.id == id).expect("assignment exists").dispatched_brief = Some(revision);
    }
    pub(super) fn history_len(&self) -> usize {
        self.briefs.len() + self.assignments.len() + self.messages.len() + self.progress.len() + self.questions.len() + self.results.len() + self.check_requests.len() + self.checks.len() + self.decisions.len()
    }

    /// The store saves only after required message admission succeeds. A failed
    /// transition may modify this disposable copy but never persisted state.
    pub(super) fn apply_operator(&mut self, command: OperatorCommand, id: &str, at: &str) -> Result<(Effect, Option<PendingMessage>), Failure> {
        if self.closed() {
            return Err(Failure::Closed);
        }
        let message_id = format!("{id}-message");
        let message = |kind, text, roles| Some(PendingMessage { id: message_id.clone(), kind, text, roles });
        let result = match command {
            OperatorCommand::CreateWork { .. } => return Err(invalid("command", "create does not address existing work")),
            OperatorCommand::UpdateBrief(input) => {
                self.set_brief(input)?;
                (Effect::Saved, None)
            }
            OperatorCommand::BindParticipant { role, participant } => {
                let other = if role == Role::Implementer { Role::Checker } else { Role::Implementer };
                if self.assignment(&other).participant == participant {
                    return Err(invalid("participant", "roles must be independent"));
                }
                if self.assignment(&role).participant == participant {
                    return Err(invalid("participant", "already assigned"));
                }
                let assignment = format!("{id}-assignment");
                self.assignments.push(Assignment { id: assignment.clone(), role: role.clone(), participant, dispatched_brief: None });
                if role == Role::Implementer {
                    self.implementer = assignment;
                } else {
                    self.checker = assignment;
                }
                (Effect::Saved, None)
            }
            OperatorCommand::DispatchAssignment { role } => {
                self.dispatched(&role);
                let text = format!("Work assignment. Brief: {}", serde_json::to_string(self.brief()).map_err(|_| Failure::Unavailable)?);
                (Effect::MessageAccepted, message(MessageKind::Assignment, text, vec![role]))
            }
            OperatorCommand::SendNote { roles, text } => {
                text_valid("text", &text, MAX_TEXT)?;
                if roles.is_empty() || roles.len() > 2 || (roles.len() == 2 && roles[0] == roles[1]) {
                    return Err(invalid("roles", "choose one or two distinct bound roles"));
                }
                (Effect::MessageAccepted, message(MessageKind::Note, text, roles))
            }
            OperatorCommand::AnswerQuestion { question_id, text, amendment } => {
                text_valid("answer", &text, MAX_TEXT)?;
                let question = self.questions.iter().find(|q| q.id == question_id).ok_or(Failure::NotFound)?;
                if question.answer.is_some() {
                    return Err(invalid("question", "already answered"));
                }
                let role = self.assignments.iter().find(|a| a.id == question.assignment_id).ok_or(Failure::NotFound)?.role.clone();
                if let Some(input) = amendment {
                    self.set_brief(input)?;
                    self.dispatched(&role);
                }
                let brief_revision = self.brief().revision;
                self.questions.iter_mut().find(|q| q.id == question_id).expect("question exists").answer = Some(Answer { text: text.clone(), message_id: message_id.clone(), brief_revision });
                let body = format!("Answer to {question_id}: {text}\nCurrent brief: {}", serde_json::to_string(self.brief()).map_err(|_| Failure::Unavailable)?);
                (Effect::MessageAccepted, message(MessageKind::Answer, body, vec![role]))
            }
            OperatorCommand::RequestCheck { result_id, reason } => {
                text_valid("reason", &reason, MAX_TEXT)?;
                let result = self.results.last().filter(|r| r.id == result_id && r.brief_revision == self.brief().revision && r.assignment_id == self.implementer).ok_or_else(|| invalid("result", "choose current result and basis"))?.clone();
                let publisher = self.assignments.iter().find(|a| a.id == result.assignment_id).expect("publisher exists");
                if publisher.participant == self.assignment(&Role::Checker).participant {
                    return Err(Failure::Forbidden);
                }
                self.dispatched(&Role::Checker);
                let request_id = format!("{id}-check-request");
                self.check_requests.push(CheckRequest {
                    id: request_id.clone(),
                    assignment_id: self.checker.clone(),
                    result_id: result_id.clone(),
                    brief_revision: self.brief().revision,
                    reason: reason.clone(),
                    message_id: message_id.clone(),
                    at: at.into(),
                });
                let text = format!(
                    "Independent check {request_id} for result {result_id}. Reason: {reason}\nResult: {}\nBrief: {}",
                    serde_json::to_string(&result).map_err(|_| Failure::Unavailable)?,
                    serde_json::to_string(self.brief()).map_err(|_| Failure::Unavailable)?
                );
                (Effect::MessageAccepted, message(MessageKind::CheckRequest, text, vec![Role::Checker]))
            }
            OperatorCommand::AcceptResult(selection) => {
                let eligibility = self.eligibility();
                if eligibility.selection.as_ref() != Some(&selection) {
                    return Err(Failure::Ineligible { reasons: if eligibility.reasons.is_empty() { vec!["Selected result/check/basis is no longer current".into()] } else { eligibility.reasons } });
                }
                self.decisions.push(Decision {
                    id: id.into(),
                    kind: DecisionKind::Accepted,
                    result_id: selection.result_id,
                    check_id: Some(selection.check_id),
                    brief_revision: selection.brief_revision,
                    reason: "Explicit browser/local operator acceptance".into(),
                    message_id: None,
                    at: at.into(),
                });
                (Effect::Accepted, None)
            }
            OperatorCommand::RequestChanges { result_id, check_id, reason } => {
                text_valid("reason", &reason, MAX_TEXT)?;
                let result = self.results.last().filter(|r| r.id == result_id && r.brief_revision == self.brief().revision && r.assignment_id == self.implementer).ok_or_else(|| invalid("result", "choose current result and basis"))?;
                if check_id.as_ref().is_some_and(|id| !self.checks.iter().any(|c| &c.id == id && c.input.result_id == result.id)) {
                    return Err(invalid("check", "check does not belong to this result"));
                }
                self.decisions.push(Decision {
                    id: id.into(),
                    kind: DecisionKind::ChangesRequested,
                    result_id: result_id.clone(),
                    check_id,
                    brief_revision: self.brief().revision,
                    reason: reason.clone(),
                    message_id: Some(message_id.clone()),
                    at: at.into(),
                });
                self.dispatched(&Role::Implementer);
                (Effect::MessageAccepted, message(MessageKind::Changes, format!("Changes requested for {result_id}: {reason}"), vec![Role::Implementer]))
            }
        };
        self.revision += 1;
        Ok(result)
    }

    pub(super) fn report(&mut self, assignment_id: String, brief_revision: u64, actor: &crate::agents::attachments::ParticipantRef, report: AgentReport, id: &str, at: &str) -> Result<(), Failure> {
        if self.closed() {
            return Err(Failure::Closed);
        }
        let assignment = self.assignments.iter().find(|a| a.id == assignment_id).ok_or(Failure::Forbidden)?;
        if &assignment.participant != actor || self.assignment(&assignment.role).id != assignment_id || assignment.dispatched_brief != Some(brief_revision) || self.brief().revision != brief_revision {
            return Err(Failure::Forbidden);
        }
        match report {
            AgentReport::Progress { kind, summary } => {
                text_valid("summary", &summary, MAX_TEXT)?;
                self.progress.push(Progress { assignment_id, brief_revision, kind, summary, at: at.into() });
            }
            AgentReport::Question { prompt, suggestions } => {
                text_valid("prompt", &prompt, MAX_TEXT)?;
                if suggestions.len() > 8 {
                    return Err(invalid("suggestions", "at most eight suggestions"));
                }
                for suggestion in &suggestions {
                    text_valid("suggestion", suggestion, MAX_TEXT)?;
                }
                self.questions.push(Question { id: id.into(), assignment_id, brief_revision, prompt, suggestions, answer: None, at: at.into() });
            }
            AgentReport::Result(input) => {
                if assignment.role != Role::Implementer {
                    return Err(Failure::Forbidden);
                }
                validate_result(&input)?;
                self.results.push(WorkResult { id: id.into(), assignment_id, brief_revision, input, at: at.into() });
            }
            AgentReport::Check(input) => {
                if assignment.role != Role::Checker {
                    return Err(Failure::Forbidden);
                }
                let request = self.check_requests.last().filter(|r| r.id == input.request_id && r.result_id == input.result_id && r.assignment_id == assignment_id && r.brief_revision == brief_revision).ok_or(Failure::Forbidden)?;
                let result = self.results.last().filter(|r| r.id == input.result_id && r.brief_revision == brief_revision && r.assignment_id == self.implementer).ok_or(Failure::Forbidden)?;
                if self.checks.iter().any(|c| c.input.request_id == request.id) {
                    return Err(invalid("request", "already reported; request an explicit recheck"));
                }
                let publisher = self.assignments.iter().find(|a| a.id == result.assignment_id).expect("publisher exists");
                if publisher.participant == *actor {
                    return Err(Failure::Forbidden);
                }
                validate_check(&input, &self.brief().input, result)?;
                self.checks.push(CheckReport { id: id.into(), assignment_id, brief_revision, input, at: at.into() });
            }
        }
        self.revision += 1;
        Ok(())
    }
}

pub(super) fn invalid(field: &str, reason: &str) -> Failure {
    Failure::InvalidInput { field: field.into(), reason: reason.into() }
}
pub(super) fn text_valid(field: &str, text: &str, limit: usize) -> Result<(), Failure> {
    if text.trim().is_empty() || text.len() > limit || text.contains('\0') {
        return Err(invalid(field, "must be nonempty and within the native UTF-8 byte limit"));
    }
    Ok(())
}
pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 96 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
}
fn validate_source(source: &SourceBasis) -> Result<(), Failure> {
    text_valid("source.reference", &source.reference, MAX_SOURCE_REFERENCE)?;
    text_valid("source.revision", &source.revision, MAX_SOURCE_REVISION)
}
fn validate_brief(input: &BriefInput) -> Result<(), Failure> {
    text_valid("title", &input.title, MAX_TITLE)?;
    text_valid("goal", &input.goal, MAX_TEXT)?;
    if input.criteria.is_empty() || input.criteria.len() > MAX_CRITERIA || !input.criteria.iter().any(|c| c.required) {
        return Err(invalid("criteria", "one to 32 criteria including a required criterion"));
    }
    let mut ids = std::collections::HashSet::new();
    for c in &input.criteria {
        if !valid_id(&c.id) || !ids.insert(&c.id) {
            return Err(invalid("criteria.id", "unique stable criterion IDs required"));
        }
        text_valid("criteria.text", &c.text, MAX_TEXT)?;
    }
    if let Some(source) = &input.source {
        validate_source(source)?;
    }
    Ok(())
}
fn validate_relative(path: &str) -> Result<(), Failure> {
    use std::path::{Component, Path};
    text_valid("path", path, 2048)?;
    if path.contains('\\') || path.contains(':') || !Path::new(path).components().all(|p| matches!(p, Component::Normal(_))) {
        return Err(invalid("path", "project-relative path without parent traversal required"));
    }
    Ok(())
}
fn validate_result(input: &ResultInput) -> Result<(), Failure> {
    text_valid("summary", &input.summary, MAX_TEXT)?;
    validate_source(&input.source)?;
    if !immutable_fingerprint(&input.source.revision) {
        return Err(invalid("source.revision", "full immutable Git object ID or sha256 content fingerprint required"));
    }
    if input.artifacts.is_empty() || input.artifacts.len() > MAX_ARTIFACTS {
        return Err(invalid("artifacts", "one to 32 artifact references required"));
    }
    for artifact in &input.artifacts {
        match artifact {
            Artifact::Path { path, fingerprint } => {
                validate_relative(path)?;
                if !immutable_fingerprint(fingerprint) {
                    return Err(invalid("fingerprint", "immutable content fingerprint required"));
                }
            }
            Artifact::Git { revision, path } => {
                validate_relative(path)?;
                if !matches!(revision.len(), 40 | 64) || !revision.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(invalid("revision", "full immutable Git object ID required"));
                }
            }
            Artifact::Message { event } => {
                if *event <= 0 {
                    return Err(invalid("event", "positive retained message event required"));
                }
            }
            Artifact::External { reference } => {
                text_valid("reference", reference, 2048)?;
            }
        }
    }
    Ok(())
}
fn immutable_fingerprint(value: &str) -> bool {
    let hex = value.strip_prefix("sha256:").unwrap_or(value);
    matches!(hex.len(), 40 | 64) && hex.bytes().all(|b| b.is_ascii_hexdigit()) && (!value.starts_with("sha256:") || hex.len() == 64)
}
fn validate_check(input: &CheckInput, brief: &BriefInput, result: &WorkResult) -> Result<(), Failure> {
    text_valid("observed_fingerprint", &input.observed_fingerprint, 256)?;
    if input.criteria.len() > MAX_CRITERIA || input.evidence.is_empty() || input.evidence.len() > 32 {
        return Err(invalid("evidence", "bounded concrete check evidence required"));
    }
    for evidence in &input.evidence {
        text_valid("evidence", evidence, MAX_TEXT)?;
    }
    let mut ids = std::collections::HashSet::new();
    for check in &input.criteria {
        if !ids.insert(&check.criterion_id) || !brief.criteria.iter().any(|c| c.id == check.criterion_id) {
            return Err(invalid("criteria", "unique known criterion IDs required"));
        }
        text_valid("criteria.evidence", &check.evidence, MAX_TEXT)?;
    }
    if input.verdict == Verdict::Passed
        && (input.observed_fingerprint != result.input.source.revision
            || input.criteria.iter().any(|c| c.verdict != Verdict::Passed)
            || brief.criteria.iter().filter(|c| c.required).any(|c| !input.criteria.iter().any(|check| check.criterion_id == c.id && check.verdict == Verdict::Passed)))
    {
        return Err(invalid("verdict", "pass requires exact source and complete passing criterion evidence"));
    }
    Ok(())
}
