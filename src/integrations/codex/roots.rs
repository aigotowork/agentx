//! Codex root qualification and mapping onto provider-neutral attachments.
//! Only correlated outer create/resume/fork results qualify; arbitrary native
//! started events, native subagents and memory threads never register roots.

use crate::agents::attachments::{ActivationTarget, Attachment, AttachmentFailure, AttachmentIntent, AttachmentOwnerId, AttachmentRequest, AttachmentStore, ExecutionId, ParticipantRef, ProviderSessionKey};
use crate::runtime::context::AgentxContext;

/// Native thread ids are namespaced by the resolved canonical Codex home.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct NativeThreadKey {
    pub canonical_home: String,
    pub thread_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RootBinding {
    pub attachment: Attachment,
    pub native_thread: NativeThreadKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RootIntent {
    Create { provisional: ParticipantRef },
    Resume { native_thread: NativeThreadKey, provisional: Option<ParticipantRef> },
    Fork { parent: NativeThreadKey, provisional: ParticipantRef },
}

/// Codex mapping adapter composed with AttachmentStore. Attach and select the
/// intended inbox delivery medium before releasing the native root response.
/// Live owner conflicts remain failures even within one shared execution.
pub(crate) trait RootBindings {
    fn canonical_participant(&self, native_thread: &NativeThreadKey) -> Result<Option<ParticipantRef>, AttachmentFailure>;
    fn attach(&mut self, execution: &ExecutionId, owner: &AttachmentOwnerId, intent: RootIntent, native_thread: NativeThreadKey) -> Result<RootBinding, AttachmentFailure>;
    fn lookup(&self, execution: &ExecutionId, native_thread: &NativeThreadKey) -> Result<Option<RootBinding>, AttachmentFailure>;
    fn retire(&mut self, target: &ActivationTarget) -> Result<(), AttachmentFailure>;
}

impl NativeThreadKey {
    pub(crate) fn session(&self) -> ProviderSessionKey {
        ProviderSessionKey { provider: "codex".to_owned(), scope: self.canonical_home.clone(), session_id: self.thread_id.clone() }
    }
}

/// This adapter alone translates native home/thread identity to the generic
/// provider-session contract; generic identity and inbox stores stay opaque.
pub(crate) struct CodexRootBindings<'a> {
    store: &'a mut dyn AttachmentStore,
}

impl<'a> CodexRootBindings<'a> {
    pub(crate) fn new(store: &'a mut dyn AttachmentStore) -> Self {
        Self { store }
    }
}

impl RootBindings for CodexRootBindings<'_> {
    fn canonical_participant(&self, native_thread: &NativeThreadKey) -> Result<Option<ParticipantRef>, AttachmentFailure> {
        self.store.canonical_participant(&native_thread.session())
    }

    fn attach(&mut self, execution: &ExecutionId, owner: &AttachmentOwnerId, intent: RootIntent, native_thread: NativeThreadKey) -> Result<RootBinding, AttachmentFailure> {
        if native_thread.thread_id.is_empty() || !std::path::Path::new(&native_thread.canonical_home).is_absolute() {
            return Err(AttachmentFailure::UnqualifiedSession);
        }
        let intent = match intent {
            RootIntent::Create { provisional } => AttachmentIntent::Create { provisional },
            RootIntent::Resume { native_thread: requested, provisional } => {
                if requested != native_thread {
                    return Err(AttachmentFailure::UnqualifiedSession);
                }
                AttachmentIntent::Resume { provisional }
            }
            RootIntent::Fork { parent, provisional } => {
                if parent.canonical_home != native_thread.canonical_home || parent.thread_id == native_thread.thread_id {
                    return Err(AttachmentFailure::UnqualifiedSession);
                }
                AttachmentIntent::Fork { parent: parent.session(), provisional }
            }
        };
        let attachment = self.store.attach(AttachmentRequest { execution: execution.clone(), owner: owner.clone(), session: native_thread.session(), intent })?;
        Ok(RootBinding { attachment, native_thread })
    }

    fn lookup(&self, execution: &ExecutionId, native_thread: &NativeThreadKey) -> Result<Option<RootBinding>, AttachmentFailure> {
        Ok(self.store.lookup(execution, &native_thread.session())?.map(|attachment| RootBinding { attachment, native_thread: native_thread.clone() }))
    }

    fn retire(&mut self, target: &ActivationTarget) -> Result<(), AttachmentFailure> {
        self.store.retire(target)
    }
}

pub(crate) fn is_shared_context(ctx: &AgentxContext) -> bool {
    ctx.raw_env.get("AGENTX_CODEX_SHARED").is_some_and(|value| value == "1")
}

/// Call at Codex hook or CLI composition before legacy process fallback. A
/// recognized shared context never guesses another root when lookup is empty.
pub(crate) fn resolve_shared_attachment(store: &dyn AttachmentStore, ctx: &AgentxContext, thread_id: &str) -> Result<Option<Attachment>, AttachmentFailure> {
    if !is_shared_context(ctx) || thread_id.is_empty() {
        return Err(AttachmentFailure::UnqualifiedSession);
    }
    let home = ctx.raw_env.get("CODEX_HOME").filter(|home| std::path::Path::new(home).is_absolute()).ok_or(AttachmentFailure::UnqualifiedSession)?;
    let execution = ctx.process_id.as_ref().filter(|id| !id.is_empty()).ok_or(AttachmentFailure::StaleExecution)?;
    store.lookup(&ExecutionId(execution.clone()), &NativeThreadKey { canonical_home: home.clone(), thread_id: thread_id.to_owned() }.session())
}
