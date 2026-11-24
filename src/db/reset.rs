//! Known managed storage reset. The daemon serializes archive/reset/generation
//! changes; this adapter never discovers or mutates arbitrary user tables.

use anyhow::Result;

use super::AgentxDb;

pub(super) const GENERATION_KEY: &str = "service:database_generation";

impl AgentxDb {
    /// A read fence must never create a generation or acquire the write lock.
    pub(crate) fn read_database_generation(&self) -> Result<String> {
        self.kv_get(GENERATION_KEY)?.filter(|value| !value.is_empty()).ok_or_else(|| anyhow::anyhow!("database generation unavailable"))
    }

    /// The sole persistent generation authority, stable over daemon restarts.
    pub(crate) fn database_generation(&self) -> Result<String> {
        self.with_write_scope(|| {
            if let Some(generation) = self.kv_get(GENERATION_KEY)? {
                anyhow::ensure!(!generation.is_empty(), "database generation is empty");
                return Ok(generation);
            }
            let generation = uuid::Uuid::new_v4().to_string();
            self.kv_set(GENERATION_KEY, Some(&generation))?;
            Ok(generation)
        })
    }

    /// Clear all owned state inside an existing write transaction, preserving
    /// schema and event sequence. The maintenance use case supplies the new
    /// generation and reset event in that same write scope.
    pub(crate) fn clear_managed_state_for_reset(&self) -> Result<()> {
        anyhow::ensure!(!self.conn().is_autocommit(), "reset requires a write scope");
        self.conn().execute_batch(
            "DELETE FROM agent_labels;
             DELETE FROM agent_label_sets;
             DELETE FROM agent_creation_forks;
             DELETE FROM agent_creation_instances;
             DELETE FROM agent_creations;
             DELETE FROM collaboration_commands;
             DELETE FROM collaboration_work;
             DELETE FROM inbox_claims;
             DELETE FROM participant_attachments;
             DELETE FROM participant_sessions;
             DELETE FROM message_acceptances;
             DELETE FROM reminders;
             DELETE FROM claude_actor_capabilities;
             DELETE FROM session_bindings;
             DELETE FROM process_bindings;
             DELETE FROM notify_endpoints;
             DELETE FROM instances;
             DELETE FROM events;
             DELETE FROM kv;",
        )?;
        Ok(())
    }
}
