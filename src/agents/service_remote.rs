//! Typed client adapter for the existing domain port; no local SQLite fallback.

use super::attachments::*;
use super::service::*;
use crate::service::{self, client::Client};

#[derive(Clone)]
pub(crate) struct RemoteAttachments {
    client: Client,
}

impl RemoteAttachments {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }
    fn call(&self, operation: AttachmentOperation) -> Result<AttachmentOutcome, AttachmentFailure> {
        match self.client.call(service::Request::Participants(Request::Attachments(operation))) {
            Ok(service::Response::Participants(Response::Attachment(outcome))) => Ok(outcome),
            _ => Err(AttachmentFailure::StoreUnavailable),
        }
    }
}

impl AttachmentStore for RemoteAttachments {
    fn participant(&self, name: &str) -> Result<ParticipantRef, AttachmentFailure> {
        match self.call(AttachmentOperation::Participant { name: name.into() }) {
            Ok(AttachmentOutcome::Participant(result)) => result,
            _ => Err(AttachmentFailure::StoreUnavailable),
        }
    }
    fn canonical_participant(&self, session: &ProviderSessionKey) -> Result<Option<ParticipantRef>, AttachmentFailure> {
        match self.call(AttachmentOperation::CanonicalParticipant { session: session.clone() }) {
            Ok(AttachmentOutcome::CanonicalParticipant(result)) => result,
            _ => Err(AttachmentFailure::StoreUnavailable),
        }
    }
    fn attach(&mut self, request: AttachmentRequest) -> Result<Attachment, AttachmentFailure> {
        match self.call(AttachmentOperation::Attach(request)) {
            Ok(AttachmentOutcome::Attached(result)) => result,
            _ => Err(AttachmentFailure::StoreUnavailable),
        }
    }
    fn lookup(&self, execution: &ExecutionId, session: &ProviderSessionKey) -> Result<Option<Attachment>, AttachmentFailure> {
        match self.call(AttachmentOperation::Lookup { execution: execution.clone(), session: session.clone() }) {
            Ok(AttachmentOutcome::Attachment(result)) => result,
            _ => Err(AttachmentFailure::StoreUnavailable),
        }
    }
    fn current_for_participant(&self, participant: &ParticipantRef) -> Result<Option<Attachment>, AttachmentFailure> {
        match self.call(AttachmentOperation::Current { participant: participant.clone() }) {
            Ok(AttachmentOutcome::Attachment(result)) => result,
            _ => Err(AttachmentFailure::StoreUnavailable),
        }
    }
    fn owned(&self, owner: &AttachmentOwnerId) -> Result<Vec<Attachment>, AttachmentFailure> {
        match self.call(AttachmentOperation::Owned { owner: owner.clone() }) {
            Ok(AttachmentOutcome::Attachments(result)) => result,
            _ => Err(AttachmentFailure::StoreUnavailable),
        }
    }
    fn validate(&self, target: &ActivationTarget) -> Result<bool, AttachmentFailure> {
        match self.call(AttachmentOperation::Validate { target: target.clone() }) {
            Ok(AttachmentOutcome::Valid(result)) => result,
            _ => Err(AttachmentFailure::StoreUnavailable),
        }
    }
    fn retire(&mut self, target: &ActivationTarget) -> Result<(), AttachmentFailure> {
        match self.call(AttachmentOperation::Retire { target: target.clone() }) {
            Ok(AttachmentOutcome::Done(result)) => result,
            _ => Err(AttachmentFailure::StoreUnavailable),
        }
    }
    fn retire_owner(&mut self, owner: &AttachmentOwnerId) -> Result<(), AttachmentFailure> {
        match self.call(AttachmentOperation::RetireOwner { owner: owner.clone() }) {
            Ok(AttachmentOutcome::Done(result)) => result,
            _ => Err(AttachmentFailure::StoreUnavailable),
        }
    }
}
