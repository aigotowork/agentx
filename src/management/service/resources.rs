//! Typed resource reads and originating filesystem supply. No HTTP headers or DB access.
use super::*;

impl ManagementService {
    pub(crate) fn list_workspaces(&self, input: GenerationInput) -> ServiceResult<WorkspaceCatalog> {
        let client = self.client(&input.generation)?;
        super::super::offices::catalog(&client, &self.project)
    }

    pub(crate) fn list_roles(&self, input: ListRolesInput) -> ServiceResult<RoleCatalog> {
        let client = self.client(&input.generation)?;
        let result = super::super::offices::roles(&client, &input.workspace)?;
        self.verify_generation(&input.generation)?;
        Ok(result)
    }

    /// Preview uses native endpoint inspection only, never launch/adopt/reserve.
    pub(crate) fn inspect_directory(&self, input: InspectDirectoryInput) -> ServiceResult<DirectoryEntry> {
        let client = self.client(&input.generation)?;
        super::super::offices::entry(&client, &input.directory)
    }

    pub(crate) fn get_labels(&self, input: GetAgentInput) -> ServiceResult<LabelsSnapshot> {
        match self.call(&input.generation, Request::Labels(input.agent))? {
            Response::Labels(snapshot) => Ok(snapshot),
            _ => Err(ServiceError::Internal("unexpected labels response".into())),
        }
    }

    pub(crate) fn list_messages(&self, input: AgentPageInput) -> ServiceResult<MessagePage> {
        match self.call(&input.generation, Request::Messages { agent: input.agent, after: input.after, limit: page_limit(input.limit)? })? {
            Response::Messages(page) => Ok(page),
            _ => Err(ServiceError::Internal("unexpected messages response".into())),
        }
    }

    pub(crate) fn message_history(&self, input: MessageHistoryInput) -> ServiceResult<MessageHistoryPage> {
        let limit = page_limit(Some(input.limit.unwrap_or(DEFAULT_MESSAGE_HISTORY_PAGE)))?;
        match self.call(&input.generation, Request::MessageHistory { agent: input.agent, before: input.before, limit })? {
            Response::MessageHistory(page) => Ok(page),
            _ => Err(ServiceError::Internal("unexpected message history response".into())),
        }
    }

    pub(crate) fn list_launch_relations(&self, input: AgentPageInput) -> ServiceResult<LaunchRelations> {
        match self.call(&input.generation, Request::LaunchRelations { agent: input.agent, after: input.after, limit: page_limit(input.limit)? })? {
            Response::LaunchRelations(page) => Ok(page),
            _ => Err(ServiceError::Internal("unexpected launch relations response".into())),
        }
    }

    pub(crate) fn list_office_activity(&self, input: PageInput) -> ServiceResult<OfficeActivity> {
        match self.call(&input.generation, Request::OfficeActivity { after: input.after, limit: page_limit(input.limit)? })? {
            Response::OfficeActivity(page) => Ok(page),
            _ => Err(ServiceError::Internal("unexpected office activity response".into())),
        }
    }

    pub(crate) fn list_human_mailbox(&self, input: ListHumanMailboxInput) -> ServiceResult<HumanMailbox> {
        match self.call(&input.generation, Request::HumanMailbox { before: input.before, pending: input.pending, limit: page_limit(input.limit)? })? {
            Response::HumanMailbox(page) => Ok(page),
            _ => Err(ServiceError::Internal("unexpected human mailbox response".into())),
        }
    }

    pub(crate) fn handle_human_messages(&self, input: HandleHumanMessages) -> ServiceResult<HumanMailbox> {
        match self.call(&input.generation, Request::HandleHumanMessages { ids: input.ids })? {
            Response::HumanMailbox(page) => Ok(page),
            _ => Err(ServiceError::Internal("unexpected human handling response".into())),
        }
    }

    /// Persist bounded bytes outside dispatch, then register through native IPC.
    /// This retains existing upload semantics; it does not add a replay journal.
    pub(crate) fn upload_attachment(&self, input: UploadAttachmentInput, bytes: &[u8]) -> ServiceResult<Attachment> {
        self.verify_generation(&input.generation)?;
        let stored = super::super::uploads::persist(&self.scope, &input.generation, &input.name, &input.media_type, bytes)?;
        let Response::Attachment(stored) = self.call(&input.generation, Request::RegisterAttachment(stored))? else {
            return Err(ServiceError::Internal("unexpected upload response".into()));
        };
        Ok(stored.attachment)
    }

    pub(crate) fn download_attachment(&self, input: GetAttachmentInput) -> ServiceResult<AttachmentBytes> {
        let Response::Attachment(stored) = self.call(&input.generation, Request::Attachment(input.id))? else {
            return Err(ServiceError::Internal("unexpected attachment response".into()));
        };
        super::super::uploads::check(&stored)?;
        let bytes = std::fs::read(&stored.path).map_err(|error| ServiceError::Internal(error.to_string()))?;
        Ok(AttachmentBytes { attachment: stored.attachment, bytes })
    }
}

pub(super) fn page_limit(limit: Option<u32>) -> ServiceResult<u32> {
    let limit = limit.unwrap_or(50);
    if limit == 0 || limit > MAX_PAGE {
        return Err(ServiceError::InvalidInput("invalid page size".into()));
    }
    Ok(limit)
}
