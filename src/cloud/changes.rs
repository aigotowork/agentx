//! Bounded long polling for the shared Space. Revisions are invalidations, never authorization or data cursors.
use super::{
    ScopeContext,
    communication::{self, store::Error},
};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};
use utoipa::{IntoParams, ToSchema};

static WAITERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(16);

#[derive(Clone, Serialize, ToSchema)]
pub(super) struct SpaceChanges {
    epoch: String,
    pub(super) roster: u32,
    pub(super) messages: u32,
    pub(super) transcript: u32,
}
impl SpaceChanges {
    pub(super) fn new() -> Self {
        Self { epoch: uuid::Uuid::new_v4().to_string(), roster: 0, messages: 0, transcript: 0 }
    }

    fn cursor(&self) -> String {
        format!("{}.{}.{}.{}", self.epoch, self.roster, self.messages, self.transcript)
    }
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in=Query)]
#[serde(deny_unknown_fields)]
pub(super) struct ChangeQuery {
    generation: String,
    cursor: Option<String>,
}

pub(super) async fn wait(scope: ScopeContext, query: ChangeQuery, member: bool) -> Result<SpaceChanges, Error> {
    communication::fence(&scope, &query.generation)?;
    let mut revisions = scope.space_changed.subscribe();
    let mut credential = scope.current.subscribe();
    let deadline = Instant::now() + Duration::from_secs(25);
    let mut slot = None;
    loop {
        if member && !scope.valid() {
            return Err(Error::Credential);
        }
        let snapshot = revisions.borrow_and_update().clone();
        if query.cursor.as_deref() != Some(&snapshot.cursor()) || Instant::now() >= deadline {
            return Ok(snapshot);
        }
        if slot.is_none() {
            slot = Some(WAITERS.try_acquire().map_err(|_| Error::WaitCapacity)?);
        }
        tokio::select! {
            _ = revisions.changed() => {},
            _ = credential.changed(), if member => {},
            _ = tokio::time::sleep_until(deadline) => {},
        }
    }
}
