//! Transaction-local external notifications. Savepoints retain only committed
//! effects; the owning outer transaction flushes after SQLite releases its lock.
use super::AgentxDb;
use crate::runtime::notify::WakeKind;

#[expect(
    clippy::large_enum_variant,
    reason = "The existing rollback list owns inline safe observation projections with an explicit shared 64-record/64-KiB preparation budget; avoid a heap allocation per projection inside SQLite solely for enum layout."
)]
pub(super) enum Effect {
    Event { id: i64, kind: String, instance: String, data: serde_json::Value },
    Wake { instance: String, kinds: Vec<WakeKind> },
    Observation { prepared: crate::service::context::PreparedDomainObservation, bytes: usize },
}

/// Counts are updated with the existing savepoint list, so admitting another
/// projection never scans an arbitrarily long business notification list.
#[derive(Default)]
pub(super) struct Effects {
    entries: Vec<Effect>,
    observation_count: usize,
    observation_bytes: usize,
}

impl Effects {
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
    #[cfg(test)]
    pub(super) fn iter(&self) -> std::slice::Iter<'_, Effect> {
        self.entries.iter()
    }
    fn push(&mut self, effect: Effect) {
        if let Effect::Observation { bytes, .. } = &effect {
            self.observation_count += 1;
            self.observation_bytes += bytes;
        }
        self.entries.push(effect);
    }
    pub(super) fn truncate(&mut self, checkpoint: usize) {
        for effect in self.entries.drain(checkpoint..) {
            if let Effect::Observation { bytes, .. } = effect {
                self.observation_count -= 1;
                self.observation_bytes -= bytes;
            }
        }
    }
}

impl AgentxDb {
    pub(crate) fn enable_observer_hints(&mut self, generation: crate::daemon::DatabaseGeneration) -> anyhow::Result<()> {
        self.install_observer_hints(generation, crate::runtime::notify::observer_hints::ObserverHints::start)
    }

    pub(crate) fn install_observer_hints(
        &mut self,
        generation: crate::daemon::DatabaseGeneration,
        start: impl FnOnce(crate::daemon::DatabaseGeneration, std::sync::Arc<std::sync::atomic::AtomicU64>) -> std::io::Result<crate::runtime::notify::observer_hints::ObserverHints>,
    ) -> anyhow::Result<()> {
        use rusqlite::functions::FunctionFlags;
        use std::sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        };
        let revision = Arc::new(AtomicU64::new(0));
        let invalidate = revision.clone();
        self.conn().create_scalar_function("agentx_revoke_observer_hints", 0, FunctionFlags::SQLITE_UTF8, move |_| {
            invalidate.fetch_add(1, Ordering::AcqRel);
            Ok(0)
        })?;
        // Scoped to the sole daemon writer's connection. A mutation revokes a
        // pending hint before it can point at a retired registration. Rollback
        // can conservatively lose a hint; it can never publish one.
        self.with_write_scope(|| {
            self.conn().execute_batch(
                "CREATE TEMP TRIGGER observer_fence_instance_insert AFTER INSERT ON main.instances BEGIN SELECT agentx_revoke_observer_hints(); END;
                 CREATE TEMP TRIGGER observer_fence_instance_delete AFTER DELETE ON main.instances BEGIN SELECT agentx_revoke_observer_hints(); END;
                 CREATE TEMP TRIGGER observer_fence_instance_update AFTER UPDATE OF name,created_at ON main.instances
                   WHEN old.name IS NOT new.name OR old.created_at IS NOT new.created_at BEGIN SELECT agentx_revoke_observer_hints(); END;
                 CREATE TEMP TRIGGER observer_fence_endpoint_insert AFTER INSERT ON main.notify_endpoints WHEN new.kind='events_wait' BEGIN SELECT agentx_revoke_observer_hints(); END;
                 CREATE TEMP TRIGGER observer_fence_endpoint_delete AFTER DELETE ON main.notify_endpoints WHEN old.kind='events_wait' BEGIN SELECT agentx_revoke_observer_hints(); END;
                 CREATE TEMP TRIGGER observer_fence_endpoint_update AFTER UPDATE ON main.notify_endpoints WHEN old.kind='events_wait' OR new.kind='events_wait' BEGIN SELECT agentx_revoke_observer_hints(); END;
                 CREATE TEMP TRIGGER observer_fence_control_insert AFTER INSERT ON main.kv WHEN new.key='service:database_generation' OR new.key GLOB 'event_observer:*' BEGIN SELECT agentx_revoke_observer_hints(); END;
                 CREATE TEMP TRIGGER observer_fence_control_delete AFTER DELETE ON main.kv WHEN old.key='service:database_generation' OR old.key GLOB 'event_observer:*' BEGIN SELECT agentx_revoke_observer_hints(); END;
                 CREATE TEMP TRIGGER observer_fence_control_update AFTER UPDATE ON main.kv
                   WHEN old.key='service:database_generation' OR new.key='service:database_generation' OR old.key GLOB 'event_observer:*' OR new.key GLOB 'event_observer:*'
                   BEGIN SELECT agentx_revoke_observer_hints(); END;",
            )?;
            Ok(())
        })?;
        self.observer_hints = Some(start(generation, revision)?);
        Ok(())
    }

    pub(crate) fn observer_hints(&self) -> Option<&crate::runtime::notify::observer_hints::ObserverHints> {
        self.observer_hints.as_ref()
    }

    /// Freeze only safe domain references and exact originating fences. This
    /// shares savepoint rollback with existing deferred effects; there is no
    /// collector lock, telemetry serialization buffer or ring call in SQLite.
    pub(crate) fn observe_domain(&self, fact: crate::observability::events::DomainCommitFact) {
        use crate::observability::events::{DropReason, MAX_POSTCOMMIT_BYTES, MAX_POSTCOMMIT_RECORDS};
        let Some(prepared) = crate::service::context::prepare_domain_observation(fact) else {
            return;
        };
        let Ok(bytes) = crate::observability::configuration::encoded_size(&prepared.observation, MAX_POSTCOMMIT_BYTES) else {
            prepared.sink.note_drop(DropReason::PreparationBudget, 1);
            return;
        };
        if self.conn().is_autocommit() {
            prepared.publish();
            return;
        }
        let mut effects = self.post_commit.borrow_mut();
        if effects.observation_count >= MAX_POSTCOMMIT_RECORDS || bytes > MAX_POSTCOMMIT_BYTES.saturating_sub(effects.observation_bytes) {
            prepared.sink.note_drop(DropReason::PreparationBudget, 1);
            return;
        }
        effects.push(Effect::Observation { prepared, bytes });
    }

    pub(super) fn defer_event(&self, id: i64, kind: &str, instance: &str, data: &serde_json::Value) -> bool {
        if self.conn().is_autocommit() {
            return false;
        }
        self.post_commit.borrow_mut().push(Effect::Event { id, kind: kind.into(), instance: instance.into(), data: data.clone() });
        true
    }

    pub(crate) fn wake_after_commit(&self, instance: &str, kinds: &[WakeKind]) {
        if self.conn().is_autocommit() {
            crate::runtime::notify::wake(self, instance, kinds);
        } else {
            self.post_commit.borrow_mut().push(Effect::Wake { instance: instance.into(), kinds: kinds.to_vec() });
        }
    }

    pub(super) fn flush_post_commit(&self) {
        debug_assert!(self.conn().is_autocommit());
        let effects = std::mem::take(&mut *self.post_commit.borrow_mut());
        let mut observers_woken = false;
        for effect in effects.entries {
            match effect {
                Effect::Event { id, kind, instance, data } => {
                    // One signal suffices for observers to read the full committed
                    // batch; business subscriptions still process every event.
                    self.publish_committed_event(id, &kind, &instance, &data, !observers_woken);
                    observers_woken = true;
                }
                Effect::Wake { instance, kinds } => crate::runtime::notify::wake(self, &instance, &kinds),
                Effect::Observation { prepared, .. } => prepared.publish(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::{context::*, events::*, runtime::*};

    fn fixture(root: &std::path::Path) -> (AgentxDb, ObservationJournal, crate::service::context::ObservationDispatchGuard) {
        fixture_with_scope(root, ScopeRef::from_canonical_scope(root.canonicalize().unwrap()).unwrap())
    }

    fn fixture_with_scope(root: &std::path::Path, projected_scope: ScopeRef) -> (AgentxDb, ObservationJournal, crate::service::context::ObservationDispatchGuard) {
        let db = AgentxDb::open_at(&root.join("observations.db")).unwrap();
        let daemon = DaemonRef::from_lease(projected_scope, "postcommit-fixture".into()).unwrap();
        let epoch = ObservationEpoch::from_committed_generation(daemon.clone(), db.database_generation().unwrap()).unwrap();
        let clock = ObservationClock::new();
        let journal = ObservationJournal::new(daemon.clone(), clock.clone()).unwrap();
        journal.publish_epoch(epoch.clone()).unwrap();
        let sink = journal.sink().unwrap();
        let runtime = RuntimeRegistry::new(daemon.clone(), clock.clone(), None).unwrap();
        let configuration = crate::observability::configuration::ConfigurationStore::new(daemon, clock.clone(), None);
        let guard = crate::service::context::ActiveObservation {
            epoch,
            clock,
            request: None,
            peer: None,
            accepted: None,
            origin: None,
            request_id: crate::service::RequestId("postcommit-request".into()),
            runtime: std::sync::Arc::new(runtime),
            configuration: std::sync::Arc::new(configuration),
            sink,
            pending_configuration: Default::default(),
        }
        .enter();
        (db, journal, guard)
    }

    #[test]
    fn outer_commit_publishes_only_surviving_savepoint_references() {
        let root = tempfile::tempdir().unwrap();
        let (db, journal, _guard) = fixture(root.path());
        let mut committed = 0;
        db.with_write_scope(|| {
            committed = db.log_event("status", "luna", &serde_json::json!({"secret":"never-observed"}))?;
            let rolled_back: anyhow::Result<()> = db.with_write_scope(|| {
                db.log_event("status", "luna", &serde_json::json!({}))?;
                anyhow::bail!("rollback nested savepoint");
            });
            assert!(rolled_back.is_err());
            assert_eq!(journal.describe().unwrap().newest, 0);
            Ok(())
        })
        .unwrap();
        let head = journal.describe().unwrap();
        assert_eq!(head.newest, 1);
        let result = journal.read(ObservationRead { window: ObservationWindow { after: ObservationCursor { stream: head.stream, sequence: 0 }, through: 1, filter: ObservationFilter::default() }, limit: 10 }).unwrap();
        let ObservationReadResult::Page(page) = result else {
            panic!("expected committed page");
        };
        assert!(matches!(&page.records[0].input.payload, ObservationPayload::DomainCommitted(DomainCommitObservation {
            fact: DomainCommitFact::EventAppended { event, .. }, .. }) if event.event.0 == committed));
        assert!(!serde_json::to_string(&page).unwrap().contains("never-observed"));
    }

    #[test]
    fn outer_rollback_publishes_no_successful_commit_fact() {
        let root = tempfile::tempdir().unwrap();
        let (db, journal, _guard) = fixture(root.path());
        let failed: anyhow::Result<()> = db.with_write_scope(|| {
            db.log_event("message", "luna", &serde_json::json!({"text":"private-body"}))?;
            anyhow::bail!("rollback outer transaction");
        });
        assert!(failed.is_err());
        assert_eq!(journal.describe().unwrap().newest, 0);
    }

    #[test]
    fn one_outer_transaction_has_a_bounded_observation_preparation_budget() {
        let root = tempfile::tempdir().unwrap();
        let (db, journal, _guard) = fixture(root.path());
        db.with_write_scope(|| {
            for _ in 0..(MAX_POSTCOMMIT_RECORDS + 3) {
                db.log_event("status", "luna", &serde_json::json!({}))?;
            }
            assert_eq!(db.post_commit.borrow().iter().filter(|effect| matches!(effect, Effect::Observation { .. })).count(), MAX_POSTCOMMIT_RECORDS);
            Ok(())
        })
        .unwrap();
        assert_eq!(journal.describe().unwrap().newest, MAX_POSTCOMMIT_RECORDS as u64);
    }

    #[test]
    fn rollback_restores_shared_preparation_budget_for_the_outer_commit() {
        let root = tempfile::tempdir().unwrap();
        let (db, journal, _guard) = fixture(root.path());
        db.with_write_scope(|| {
            for _ in 0..(MAX_POSTCOMMIT_RECORDS - 1) {
                db.log_event("status", "luna", &serde_json::json!({}))?;
            }
            let nested: anyhow::Result<()> = db.with_write_scope(|| {
                db.log_event("status", "luna", &serde_json::json!({}))?;
                anyhow::bail!("discard the final nested preparation slot");
            });
            assert!(nested.is_err());
            db.log_event("status", "luna", &serde_json::json!({}))?;
            Ok(())
        })
        .unwrap();
        assert_eq!(journal.describe().unwrap().newest, MAX_POSTCOMMIT_RECORDS as u64);
    }

    #[test]
    fn encoded_byte_budget_can_stop_preparation_before_the_record_count_limit() {
        let root = tempfile::tempdir().unwrap();
        // Collector identities are native values, not database-open paths.
        // Keep SQLite below its pathname limit while independently exercising
        // a long, absolute safe reference in each encoded domain projection.
        let mut scope = root.path().canonicalize().unwrap();
        for index in 0..7 {
            scope.push(format!("{index}{}", "s".repeat(170)));
        }
        let (db, journal, _guard) = fixture_with_scope(root.path(), ScopeRef::from_canonical_scope(scope).unwrap());
        let prepared_count = db
            .with_write_scope(|| {
                for _ in 0..MAX_POSTCOMMIT_RECORDS {
                    db.log_event("status", "luna", &serde_json::json!({}))?;
                }
                let effects = db.post_commit.borrow();
                assert!(effects.observation_count < MAX_POSTCOMMIT_RECORDS);
                assert!(effects.observation_count > 0);
                assert!(effects.observation_bytes <= MAX_POSTCOMMIT_BYTES);
                assert!(effects.observation_bytes > MAX_POSTCOMMIT_BYTES / 2);
                Ok(effects.observation_count)
            })
            .unwrap();
        assert!(journal.describe().unwrap().newest < MAX_POSTCOMMIT_RECORDS as u64);
        assert_eq!(journal.describe().unwrap().newest, prepared_count as u64);
    }

    #[test]
    fn reset_created_rows_are_not_published_under_the_admitted_old_epoch() {
        let root = tempfile::tempdir().unwrap();
        let (db, journal, _guard) = fixture(root.path());
        let old = db.database_generation().unwrap();
        let context = crate::service::RequestContext {
            caller: crate::service::CallerContext { request_id: crate::service::RequestId("reset-projection".into()), invocation: crate::service::InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some(old.clone()) },
            process: crate::service::CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
            daemon_instance: "postcommit-fixture".into(),
            database_generation: old.clone(),
        };
        crate::service::maintenance_store::handle(&db, &context, crate::service::maintenance::Request::ResetConversation).unwrap();
        assert_ne!(db.database_generation().unwrap(), old);
        assert!(db.get_last_event_id() > 0);
        assert_eq!(journal.describe().unwrap().newest, 0);
        assert_eq!(crate::service::context::active_observation().unwrap().epoch.database_generation, old);
    }
}
