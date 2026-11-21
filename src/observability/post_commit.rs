//! Fixed per-dispatch attribution. Nested reactions pause the parent phase,
//! so the same time does not count twice. No labels from events or SQL.

use std::cell::RefCell;
use std::time::Instant;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct PostCommitWork {
    pub commit_us: u64,
    pub observers_us: u64,
    pub subscription_candidates_us: u64,
    pub subscription_match_us: u64,
    pub request_watch_us: u64,
    pub subscription_candidates: u32,
    pub subscription_predicates: u32,
    pub observer_hints_queued: u32,
    pub observer_hints_coalesced: u32,
    pub observer_hints_dropped: u32,
}

#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Commit,
    Observers,
    Candidates,
    Match,
    RequestWatch,
}

#[derive(Default)]
struct Capture {
    work: PostCommitWork,
    active: Option<(Phase, Instant)>,
}

thread_local! {
    static CAPTURE: RefCell<Option<Capture>> = const { RefCell::new(None) };
}

pub(crate) struct Dispatch(Option<Capture>);

impl Dispatch {
    pub(crate) fn begin() -> Self {
        Self(CAPTURE.with(|c| c.replace(Some(Capture::default()))))
    }
    pub(crate) fn work(&self) -> Option<PostCommitWork> {
        CAPTURE.with(|c| c.borrow().as_ref().map(|c| c.work.clone()).filter(|w| w != &PostCommitWork::default()))
    }
}

impl Drop for Dispatch {
    fn drop(&mut self) {
        CAPTURE.with(|c| c.replace(self.0.take()));
    }
}

pub(crate) fn count(add: impl FnOnce(&mut PostCommitWork)) {
    CAPTURE.with(|c| {
        if let Some(c) = c.borrow_mut().as_mut() {
            add(&mut c.work);
        }
    });
}

fn switch(next: Option<Phase>) -> Option<Phase> {
    CAPTURE.with(|c| {
        let mut c = c.borrow_mut();
        let c = c.as_mut()?;
        let now = Instant::now();
        let previous = c.active.take().map(|(phase, started)| {
            let elapsed = now.saturating_duration_since(started).as_micros().min(u64::MAX.into()) as u64;
            let total = match phase {
                Phase::Commit => &mut c.work.commit_us,
                Phase::Observers => &mut c.work.observers_us,
                Phase::Candidates => &mut c.work.subscription_candidates_us,
                Phase::Match => &mut c.work.subscription_match_us,
                Phase::RequestWatch => &mut c.work.request_watch_us,
            };
            *total = total.saturating_add(elapsed);
            phase
        });
        c.active = next.map(|phase| (phase, now));
        previous
    })
}

pub(crate) fn measure<T>(phase: Phase, work: impl FnOnce() -> T) -> T {
    struct Restore(Option<Phase>);
    impl Drop for Restore {
        fn drop(&mut self) {
            switch(self.0);
        }
    }
    let _phase = Restore(switch(Some(phase)));
    work()
}
