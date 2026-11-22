//! SQLite database access for agentx
//!
//! Three loosely-coupled state planes live in a single DB:
//! - `instances`: live per-agent state (TUI display, gating, delivery cursors)
//! - `events`: append-only history / message log / relay replication source
//! - `process_bindings`, `session_bindings`, `notify_endpoints`, `kv`: routing
//!   and control-plane state
//!
//! Callers typically write an event, advance per-instance cursors separately,
//! and touch bindings/endpoints/kv for delivery, identity resolution, relay
//! cursors, request-watch bookkeeping, and other control-plane state.
//!
//! Includes:
//! - Reading unread messages from `events`
//! - Updating cursor position (instances.last_event_id)
//! - Reading instance status
//! - Registering notify endpoints

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::shared::time::now_epoch_f64;

mod claude_actors;
mod conversation_routes;
mod human_mailbox;
mod mailbox_routes;
pub(crate) use human_mailbox::HUMAN_ADDRESSED;
mod collaboration;
mod events;
mod instances;
mod kv;
pub(crate) mod labels;
mod launch_names;
mod launch_relations;
mod message_acceptance;
mod notify;
mod post_commit;
pub(crate) mod reqwatch_policy;
mod reset;
mod sessions;
mod shared_runtime;
mod subscription_index;
pub(crate) mod subscriptions;

pub(crate) use collaboration::StoredWorkCommand;
pub use events::Message;
pub use instances::InstanceRow;
#[allow(unused_imports)]
pub use instances::InstanceStatus;

/// Storage-owned current head; migration consumers assert this after upgrades.
/// Bump on any schema change.
pub(crate) const SCHEMA_VERSION: i32 = 29;
// Tables introduced by replayable additive migrations. A version stamp alone
// cannot prove these migrations ran; detection and repair share this inventory.
const ADDITIVE_TABLES: &[(i32, &str)] = &[
    (21, "participant_sessions"),
    (21, "participant_attachments"),
    (21, "inbox_claims"),
    (22, "message_acceptances"),
    (23, "collaboration_work"),
    (23, "collaboration_commands"),
    (26, "agent_creations"),
    (26, "agent_creation_instances"),
    (28, "agent_label_sets"),
    (28, "agent_labels"),
    (29, "agent_creation_forks"),
];
pub const DEV_ROOT_KV_KEY: &str = "config:dev_root";
const MIGRATIONS: &[(i32, &str)] = &[
    (
        17,
        "ALTER TABLE instances ADD COLUMN terminal_preset_requested TEXT DEFAULT '';
         ALTER TABLE instances ADD COLUMN terminal_preset_effective TEXT DEFAULT '';
         UPDATE instances
         SET terminal_preset_effective = json_extract(launch_context, '$.terminal_preset')
         WHERE launch_context != '' AND json_valid(launch_context) AND json_extract(launch_context, '$.terminal_preset') IS NOT NULL;",
    ),
    (
        18,
        "ALTER TABLE instances ADD COLUMN last_seen INTEGER DEFAULT 0;
         CREATE TABLE IF NOT EXISTS claude_actor_capabilities (
             token TEXT PRIMARY KEY,
             session_id TEXT NOT NULL,
             tool_use_id TEXT NOT NULL,
             agent_id TEXT NOT NULL DEFAULT '',
             instance_name TEXT NOT NULL,
             created_at INTEGER NOT NULL,
             expires_at INTEGER NOT NULL,
             last_seen INTEGER NOT NULL,
             UNIQUE(session_id, tool_use_id, agent_id)
         );
         CREATE INDEX IF NOT EXISTS idx_claude_actor_expiry
             ON claude_actor_capabilities(expires_at);
         CREATE INDEX IF NOT EXISTS idx_claude_actor_session
             ON claude_actor_capabilities(session_id);",
    ),
    (
        19,
        "CREATE TRIGGER IF NOT EXISTS events_fts_delete
         AFTER DELETE ON events BEGIN
             DELETE FROM events_fts WHERE rowid = old.id;
         END;
         DELETE FROM events_fts WHERE rowid NOT IN (SELECT id FROM events);",
    ),
    (
        20,
        "UPDATE instances
         SET terminal_preset_effective = json_extract(launch_context, '$.terminal_preset_effective')
         WHERE COALESCE(terminal_preset_effective, '') = ''
           AND CASE WHEN json_valid(launch_context) THEN
               json_type(launch_context, '$.terminal_preset_effective') = 'text'
               AND json_extract(launch_context, '$.terminal_preset_effective') != ''
               ELSE 0 END;",
    ),
    (21, shared_runtime::SCHEMA),
    (22, message_acceptance::SCHEMA),
    (23, collaboration::SCHEMA),
    (24, "ALTER TABLE instances ADD COLUMN launch_profile TEXT DEFAULT '';"),
    (25, "ALTER TABLE instances ADD COLUMN model_request TEXT DEFAULT '';"),
    (26, launch_relations::SCHEMA),
    (27, ""), // Canonical instance-column projection runs inside the migration transaction.
    (28, labels::SCHEMA),
    (29, launch_relations::FORK_SCHEMA),
];

/// Identify columns outside the current native instance storage contract.
/// The internal running-task snapshot is retained even though row APIs omit it.
pub(crate) fn obsolete_instance_columns(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let retained: std::collections::HashSet<&str> = instances::INSTANCE_COLUMNS.split(',').map(str::trim).chain(["running_tasks"]).collect();
    let mut statement = conn.prepare("PRAGMA table_info(instances)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(columns.into_iter().filter(|column| !retained.contains(column.as_str())).collect())
}

/// Identify a failed projection within the preserved schema-upgrade error chain.
#[derive(Debug)]
struct PreserveInstanceStorage;

impl std::fmt::Display for PreserveInstanceStorage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Instance storage projection failed; existing database preserved")
    }
}

/// Drop retired metadata in place, preserving rows, rowids, constraints and all
/// cross-table identities. An error rolls back the whole schema transaction.
fn project_instance_columns(conn: &Connection) -> Result<()> {
    let columns = obsolete_instance_columns(conn)?;
    for column in &columns {
        // Bound JSON paths avoid SQL interpolation. JSON1 preserves the retained
        // numeric text, including exact incarnation values, without materializing history.
        let member = serde_json::to_string(column)?;
        let root = format!("$.{member}");
        let snapshot = format!("$.snapshot.{member}");
        conn.execute("UPDATE events SET data=json_remove(data,?1,?2) WHERE type='life' AND CASE WHEN json_valid(data) THEN json_type(data,?1) IS NOT NULL OR json_type(data,?2) IS NOT NULL ELSE 0 END", rusqlite::params![root, snapshot])?;
        conn.execute("UPDATE participant_sessions SET snapshot=json_remove(snapshot,?1) WHERE CASE WHEN json_valid(snapshot) THEN json_type(snapshot,?1) IS NOT NULL ELSE 0 END", [&root])?;
    }
    for column in columns {
        let quoted = column.replace('"', "\"\"");
        conn.execute_batch(&format!("ALTER TABLE instances DROP COLUMN \"{quoted}\""))?;
    }
    Ok(())
}

/// Schema compatibility check result
enum SchemaCompat {
    /// Schema is compatible (or fresh DB) — proceed with init_db
    Ok,
    /// Schema is incompatible — archive, reconnect, reinit
    NeedsArchive(String, Option<i32>),
    /// DB is newer than code — stale process, work with existing schema
    StaleProcess,
}

/// Database handle for agentx operations
pub struct AgentxDb {
    conn: Connection,
    db_path: std::path::PathBuf,
    db_inode: u64,
    post_commit: std::cell::RefCell<post_commit::Effects>,
    hook_mailbox_routes: std::cell::Cell<bool>,
    observer_hints: Option<crate::runtime::notify::observer_hints::ObserverHints>,
    indexed_subscriptions: bool,
    indexed_ordinary_subscriptions: bool,
    read_budget: std::cell::Cell<bool>,
    pub(crate) poll_progress: crate::runtime::poll_progress::PollProgress,
}

fn get_inode(path: &std::path::Path) -> u64 {
    crate::sys::fs::file_id(path)
}

/// Reject filesystem-backed unit-test databases that are not disposable state.
/// This is a last-resort tripwire for code paths that bypass Config entirely.
///
/// A path is disposable if a fixture registered its root, or if it sits under
/// the system temp tree — the backstop for ad-hoc `tempfile` DBs opened by
/// explicit path. Unlike the Config redirect, this stays lenient about temp
/// geography because tests only ever hand `open_raw` their own throwaway paths;
/// the inherited-real-DB threat flows through Config, which is registry-gated.
#[cfg(test)]
fn assert_isolated_db_path(db_path: &std::path::Path) {
    if db_path == std::path::Path::new(":memory:") {
        return;
    }

    if crate::paths::test_roots::is_registered(db_path) {
        return;
    }

    assert!(
        crate::paths::is_test_temp_path(db_path),
        "test refused to open a DB at {} (not a registered or temp-tree path).\n\
         This path is not disposable test state, so open_raw fails closed.\n\
         Tests must install an isolated environment first:\n    \
         let (_dir, _agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();",
        db_path.display(),
    );
}

impl AgentxDb {
    /// Open a hardened connection: secure the directory and database files to
    /// owner-only modes (see `paths::ensure_private_db`), then open with the
    /// standard agentx PRAGMAs. The single write path for opening the DB.
    fn open_connection(db_path: &std::path::Path) -> Result<Connection> {
        crate::paths::ensure_private_db(db_path).with_context(|| format!("Failed to secure database: {}", db_path.display()))?;

        let conn = Connection::open(db_path).with_context(|| format!("Failed to open database: {}", db_path.display()))?;
        // Every test opens a fresh tempdir DB, and the fsyncs of WAL conversion
        // and the schema commit cost ~170ms per open on Windows — most of the
        // unit suite's runtime, serialized behind the test env lock. Tests
        // never exercise crash durability, so skip the syncs. Integration
        // fixtures opt their agentx processes in via AGENTX_TEST_DB_NOSYNC.
        if cfg!(test) || std::env::var_os("AGENTX_TEST_DB_NOSYNC").is_some() {
            conn.execute_batch("PRAGMA synchronous=OFF;")?;
        }
        // busy_timeout first: converting a fresh db to WAL takes a brief
        // exclusive lock, so with a 0 timeout a concurrent first-open (or heavy
        // load) fails instantly with SQLITE_BUSY. Setting the timeout up front
        // makes the WAL conversion retry instead.
        conn.execute_batch("PRAGMA busy_timeout=5000; PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")?;
        Self::register_query_functions(&conn)?;

        Ok(conn)
    }

    fn register_query_functions(conn: &Connection) -> rusqlite::Result<()> {
        use rusqlite::functions::FunctionFlags;
        labels::register_functions(conn)?;
        conn.create_scalar_function("agentx_adopted_workspace", 1, FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_INNOCUOUS, |context| {
            let snapshot: Option<String> = context.get(0)?;
            Ok(crate::agents::workspaces::adopted_workspace_key(snapshot.as_deref()))
        })
    }

    /// A daemon-owned read connection; it cannot migrate or write the store.
    pub(crate) fn open_reader(db_path: &std::path::Path) -> Result<Self> {
        #[cfg(test)]
        assert_isolated_db_path(db_path);
        let conn = Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        conn.execute_batch("PRAGMA busy_timeout=100; PRAGMA query_only=ON;")?;
        Self::register_query_functions(&conn)?;
        let reader = Self::from_connection(conn, db_path);
        // The writer owns setup; readers can use the projection once its
        // durable coverage row exists and falls back before that point.
        reader.hook_mailbox_routes.set(true);
        Ok(reader)
    }

    /// Access the underlying SQLite connection (for direct queries).
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// SQLite has one progress handler per connection. Nested queries keep
    /// the enclosing budget, including its remaining instruction count.
    pub(crate) fn with_read_budget<T>(&self, mut expired: impl FnMut() -> bool + Send + 'static, read: impl FnOnce() -> T) -> rusqlite::Result<T> {
        if self.read_budget.get() {
            return Ok(read());
        }
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let interrupted = Arc::new(AtomicBool::new(false));
        let flag = interrupted.clone();
        let mut ticks = 0u32;
        self.conn.progress_handler(
            1_000,
            Some(move || {
                ticks += 1;
                let stop = ticks > 20_000 || expired();
                if stop {
                    flag.store(true, Ordering::Relaxed);
                }
                stop
            }),
        )?;
        self.read_budget.set(true);
        struct Reset<'a>(&'a AgentxDb);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                let _ = self.0.conn.progress_handler(0, None::<fn() -> bool>);
                self.0.read_budget.set(false);
            }
        }
        let _reset = Reset(self);
        let result = read();
        if interrupted.load(Ordering::Relaxed) {
            return Err(rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERRUPT), None));
        }
        Ok(result)
    }

    /// Run `f` inside a `BEGIN IMMEDIATE` transaction and commit on success.
    ///
    /// The immediate write lock makes read-modify-write sequences atomic
    /// across the separate database connections used by hook processes.
    /// Queries inside `f` must use the provided transaction.
    pub fn with_immediate_transaction<T>(&self, f: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
        let checkpoint = self.post_commit.borrow().len();
        let txn = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
        let result = f(&txn).and_then(|value| {
            crate::observability::post_commit::measure(crate::observability::post_commit::Phase::Commit, || txn.commit())?;
            Ok(value)
        });
        if result.is_err() {
            self.post_commit.borrow_mut().truncate(checkpoint);
        } else {
            self.flush_post_commit();
        }
        result
    }

    /// Run `f` atomically on this connection: in its own `BEGIN IMMEDIATE`
    /// transaction when none is open, or in a savepoint nested inside the
    /// caller's transaction. Any error rolls back every write made by `f`.
    pub fn with_write_scope<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        let own_transaction = self.conn.is_autocommit();
        let (begin, commit, rollback) = if own_transaction { ("BEGIN IMMEDIATE", "COMMIT", "ROLLBACK") } else { ("SAVEPOINT agentx_write_scope", "RELEASE agentx_write_scope", "ROLLBACK TO agentx_write_scope; RELEASE agentx_write_scope") };
        self.conn.execute_batch(begin)?;
        let checkpoint = self.post_commit.borrow().len();
        let result = f().and_then(|value| {
            if own_transaction {
                crate::observability::post_commit::measure(crate::observability::post_commit::Phase::Commit, || self.conn.execute_batch(commit))?;
            } else {
                self.conn.execute_batch(commit)?;
            }
            Ok(value)
        });
        if result.is_err() {
            let _ = self.conn.execute_batch(rollback);
            self.post_commit.borrow_mut().truncate(checkpoint);
        } else if own_transaction {
            self.flush_post_commit();
        }
        result
    }

    /// Access the filesystem path backing this DB handle.
    pub fn path(&self) -> &std::path::Path {
        &self.db_path
    }

    /// Open the agentx database at ~/.agentx/agentx.db with schema migration/compat.
    pub fn open() -> Result<Self> {
        let agentx_dir = crate::paths::agentx_dir();
        crate::paths::ensure_private_directory(&agentx_dir).with_context(|| format!("Failed to secure agentx directory: {}", agentx_dir.display()))?;
        Self::open_at(&agentx_dir.join("agentx.db"))
    }

    /// Open the agentx database at a specific path with schema migration/compat.
    pub fn open_at(db_path: &std::path::Path) -> Result<Self> {
        let mut db = Self::open_raw(db_path)?;
        db.ensure_schema()?;
        Ok(db)
    }

    /// Open DB connection without schema checks (for testing only).
    pub fn open_raw(db_path: &std::path::Path) -> Result<Self> {
        #[cfg(test)]
        assert_isolated_db_path(db_path);
        let conn = Self::open_connection(db_path)?;

        Ok(Self::from_connection(conn, db_path))
    }

    fn from_connection(conn: Connection, db_path: &std::path::Path) -> Self {
        Self {
            conn,
            db_path: db_path.to_path_buf(),
            db_inode: get_inode(db_path),
            post_commit: Default::default(),
            hook_mailbox_routes: Default::default(),
            observer_hints: None,
            indexed_subscriptions: false,
            indexed_ordinary_subscriptions: false,
            read_budget: Default::default(),
            poll_progress: Default::default(),
        }
    }

    /// Reconnect if the DB file was replaced (e.g., by agentx reset / schema bump).
    /// Long-lived threads (PTY delivery, listeners) hold an open connection to the
    /// old inode; this moves them onto the new DB file.
    /// Returns true if reconnection happened.
    pub fn reconnect_if_stale(&mut self) -> bool {
        let current_inode = get_inode(&self.db_path);
        if current_inode == 0 || current_inode == self.db_inode {
            return false;
        }
        // DB file replaced — reconnect
        use crate::log::{log_error, log_info};
        // Best-effort re-harden: the replacement was written by another agentx
        // process (reset/archive) that already secured it, so failing here must
        // not wedge a live delivery/listener loop — log and continue.
        if let Err(e) = crate::paths::ensure_private_db(&self.db_path) {
            use crate::log::log_warn;
            log_warn("native", "db.secure_fail", &format!("Failed to re-secure DB after replacement: {}", e));
        }
        match Connection::open(&self.db_path) {
            Ok(new_conn) => {
                if let Err(e) = new_conn.execute_batch("PRAGMA busy_timeout=5000; PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;") {
                    use crate::log::log_warn;
                    log_warn("native", "db.pragma_fail", &format!("PRAGMA setup failed after reconnect: {}", e));
                }
                if let Err(e) = Self::register_query_functions(&new_conn) {
                    log_error("native", "db.query_function_fail", &format!("Failed to register native query functions: {}", e));
                    return false;
                }
                log_info("native", "db.reconnect", &format!("DB file replaced (inode {} -> {}), reconnected", self.db_inode, current_inode));
                self.conn = new_conn;
                self.observer_hints = None;
                self.indexed_subscriptions = false;
                self.indexed_ordinary_subscriptions = false;
                self.poll_progress = Default::default();
                self.db_inode = current_inode;
                true
            }
            Err(e) => {
                log_error("native", "db.reconnect_fail", &format!("Failed to reconnect: {}", e));
                false
            }
        }
    }

    /// Initialize database schema. Idempotent (IF NOT EXISTS).
    /// Creates all tables, indexes, events_v view, FTS5 virtual table + trigger,
    /// and sets PRAGMA user_version.
    pub fn init_db(&self) -> Result<()> {
        // Skip if already at current version (avoids DROP VIEW race with concurrent readers)
        let current: i32 = self.conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if current == SCHEMA_VERSION {
            return Ok(());
        }

        anyhow::ensure!(current == 0, "Schema bootstrap cannot replace version {current}; use ensure_schema for migration");

        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
        // A concurrent initializer may have finished while we acquired the
        // write lock. Avoid rebuilding its view after it published the schema.
        let current: i32 = tx.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if current == SCHEMA_VERSION {
            return Ok(());
        }
        anyhow::ensure!(current == 0, "Schema bootstrap cannot replace version {current}; use ensure_schema for migration");

        tx.execute_batch(
            "
            -- Events table
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                type TEXT NOT NULL,
                instance TEXT NOT NULL,
                data TEXT NOT NULL
            );

            -- Notify endpoints
            CREATE TABLE IF NOT EXISTS notify_endpoints (
                instance TEXT NOT NULL,
                kind TEXT NOT NULL,
                port INTEGER NOT NULL,
                updated_at REAL NOT NULL,
                PRIMARY KEY (instance, kind)
            );
            CREATE INDEX IF NOT EXISTS idx_notify_endpoints_instance ON notify_endpoints(instance);
            CREATE INDEX IF NOT EXISTS idx_notify_endpoints_port ON notify_endpoints(port);

            -- Process bindings
            CREATE TABLE IF NOT EXISTS process_bindings (
                process_id TEXT PRIMARY KEY,
                session_id TEXT,
                instance_name TEXT,
                updated_at REAL NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_process_bindings_instance ON process_bindings(instance_name);
            CREATE INDEX IF NOT EXISTS idx_process_bindings_session ON process_bindings(session_id);

            -- Session bindings
            CREATE TABLE IF NOT EXISTS session_bindings (
                session_id TEXT PRIMARY KEY,
                instance_name TEXT NOT NULL,
                created_at REAL NOT NULL,
                FOREIGN KEY (instance_name) REFERENCES instances(name) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_session_bindings_instance ON session_bindings(instance_name);

            -- Instances table
            CREATE TABLE IF NOT EXISTS instances (
                name TEXT PRIMARY KEY,
                session_id TEXT UNIQUE,
                parent_session_id TEXT,
                parent_name TEXT,
                last_event_id INTEGER DEFAULT 0,
                status TEXT DEFAULT 'active',
                status_time INTEGER DEFAULT 0,
                last_seen INTEGER DEFAULT 0,
                status_context TEXT DEFAULT '',
                status_detail TEXT DEFAULT '',
                last_stop INTEGER DEFAULT 0,
                directory TEXT,
                created_at REAL NOT NULL,
                transcript_path TEXT DEFAULT '',
                tcp_mode INTEGER DEFAULT 0,
                wait_timeout INTEGER,
                background INTEGER DEFAULT 0,
                background_log_file TEXT DEFAULT '',
                name_announced INTEGER DEFAULT 0,
                agent_id TEXT UNIQUE,
                running_tasks TEXT DEFAULT '',
                origin_device_id TEXT DEFAULT '',
                hints TEXT DEFAULT '',
                subagent_timeout INTEGER,
                tool TEXT DEFAULT 'claude',
                launch_args TEXT DEFAULT '',
                launch_profile TEXT DEFAULT '',
                model_request TEXT DEFAULT '',
                terminal_preset_requested TEXT DEFAULT '',
                terminal_preset_effective TEXT DEFAULT '',
                idle_since TEXT DEFAULT '',
                pid INTEGER DEFAULT NULL,
                launch_context TEXT DEFAULT '',
                FOREIGN KEY (parent_session_id) REFERENCES instances(session_id) ON DELETE SET NULL
            );

            -- Claude shell actor capabilities
            CREATE TABLE IF NOT EXISTS claude_actor_capabilities (
                token TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                tool_use_id TEXT NOT NULL,
                agent_id TEXT NOT NULL DEFAULT '',
                instance_name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                last_seen INTEGER NOT NULL,
                UNIQUE(session_id, tool_use_id, agent_id)
            );
            CREATE INDEX IF NOT EXISTS idx_claude_actor_expiry ON claude_actor_capabilities(expires_at);
            CREATE INDEX IF NOT EXISTS idx_claude_actor_session ON claude_actor_capabilities(session_id);

            -- KV table
            CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT);

            -- Event indexes
            CREATE INDEX IF NOT EXISTS idx_timestamp ON events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_type ON events(type);
            CREATE INDEX IF NOT EXISTS idx_instance ON events(instance);
            CREATE INDEX IF NOT EXISTS idx_type_instance ON events(type, instance);

            -- Instance indexes
            CREATE INDEX IF NOT EXISTS idx_session_id ON instances(session_id);
            CREATE INDEX IF NOT EXISTS idx_parent_session_id ON instances(parent_session_id);
            CREATE INDEX IF NOT EXISTS idx_parent_name ON instances(parent_name);
            CREATE INDEX IF NOT EXISTS idx_created_at ON instances(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_status ON instances(status);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_id_unique ON instances(agent_id) WHERE agent_id IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_instances_origin ON instances(origin_device_id);

            -- Flattened events view (DROP first to apply schema changes)
            DROP VIEW IF EXISTS events_v;
            CREATE VIEW IF NOT EXISTS events_v AS
            SELECT
                id, timestamp, type, instance, data,
                json_extract(data, '$.from') as msg_from,
                json_extract(data, '$.text') as msg_text,
                json_extract(data, '$.scope') as msg_scope,
                json_extract(data, '$.sender_kind') as msg_sender_kind,
                json_extract(data, '$.delivered_to') as msg_delivered_to,
                json_extract(data, '$.mentions') as msg_mentions,
                json_extract(data, '$.intent') as msg_intent,
                json_extract(data, '$.thread') as msg_thread,
                json_extract(data, '$.reply_to') as msg_reply_to,
                json_extract(data, '$.reply_to_local') as msg_reply_to_local,
                json_extract(data, '$.bundle_id') as bundle_id,
                json_extract(data, '$.title') as bundle_title,
                json_extract(data, '$.description') as bundle_description,
                json_extract(data, '$.extends') as bundle_extends,
                json_extract(data, '$.refs.events') as bundle_events,
                json_extract(data, '$.refs.files') as bundle_files,
                json_extract(data, '$.refs.transcript') as bundle_transcript,
                json_extract(data, '$.created_by') as bundle_created_by,
                json_extract(data, '$.status') as status_val,
                json_extract(data, '$.context') as status_context,
                json_extract(data, '$.detail') as status_detail,
                json_extract(data, '$.action') as life_action,
                json_extract(data, '$.by') as life_by,
                json_extract(data, '$.batch_id') as life_batch_id,
                json_extract(data, '$.reason') as life_reason
            FROM events;

            -- FTS5 full-text search index
            CREATE VIRTUAL TABLE IF NOT EXISTS events_fts USING fts5(
                searchable,
                tokenize='unicode61'
            );
            CREATE TRIGGER IF NOT EXISTS events_fts_insert
            AFTER INSERT ON events BEGIN
                INSERT INTO events_fts(rowid, searchable) VALUES (
                    new.id,
                    COALESCE(json_extract(new.data, '$.text'), '') || ' ' ||
                    COALESCE(json_extract(new.data, '$.from'), '') || ' ' ||
                    COALESCE(new.instance, '') || ' ' ||
                    COALESCE(json_extract(new.data, '$.context'), '') || ' ' ||
                    COALESCE(json_extract(new.data, '$.detail'), '') || ' ' ||
                    COALESCE(json_extract(new.data, '$.action'), '') || ' ' ||
                    COALESCE(json_extract(new.data, '$.reason'), '')
                );
            END;
            CREATE TRIGGER IF NOT EXISTS events_fts_delete
            AFTER DELETE ON events BEGIN
                DELETE FROM events_fts WHERE rowid = old.id;
            END;
            ",
        )?;

        tx.execute_batch(shared_runtime::SCHEMA)?;
        tx.execute_batch(message_acceptance::SCHEMA)?;
        tx.execute_batch(collaboration::SCHEMA)?;
        tx.execute_batch(launch_relations::SCHEMA)?;
        tx.execute_batch(labels::SCHEMA)?;
        tx.execute_batch(launch_relations::FORK_SCHEMA)?;

        project_instance_columns(&tx).context(PreserveInstanceStorage)?;

        // Set schema version
        tx.execute_batch(&format!("PRAGMA user_version = {}", SCHEMA_VERSION))?;
        tx.commit()?;

        Ok(())
    }

    /// Bootstrap or migrate storage, preserving all recognized migration failures.
    ///
    /// Validate structure as well as version, repairing known migrations atomically.
    /// Migration failures preserve the database; only unsupported legacy layouts archive.
    pub fn ensure_schema(&mut self) -> Result<()> {
        match self.check_schema_compat()? {
            SchemaCompat::Ok => {
                self.init_db()?;
                Ok(())
            }
            SchemaCompat::NeedsArchive(reason, old_version) => {
                if let Some(version) = old_version {
                    // A DB can be stamped at some version yet be missing columns
                    // an earlier migration should have added ("stamped without
                    // migration"). The stamp alone can't tell us how far back to
                    // start, so key off the tables and columns actually present.
                    let migrate_from = self.repair_migrate_from(version);
                    match self.try_apply_migrations(migrate_from) {
                        Ok(true) => return Ok(()),
                        Ok(false) => {}
                        Err(e) => {
                            return Err(e).context("In-place schema upgrade failed; existing database preserved");
                        }
                    }
                }
                eprintln!("agentx: {}, archiving...", reason);

                // Snapshot running instances to pidtrack before archive so orphan
                // recovery can re-register them into the fresh DB.
                self.snapshot_running_to_pidtrack();

                // Release our handle to the old DB file before archiving. Windows
                // refuses to delete a file that still has an open handle; Unix
                // unlinks an open file fine, so this is a no-op there.
                //
                // This only releases *our own* connection. If any other agentx
                // process — another agent instance, a relay worker, a hook
                // invocation — has the same DB file open at this moment, the
                // `remove_file` inside `archive_db_at` below can still fail on
                // Windows; see the doc comment there for why closing our own
                // handle isn't sufficient in general.
                self.conn = Connection::open_in_memory()?;

                // Archive the old DB
                let archive_path = Self::archive_db_at(&self.db_path)?;
                if let Some(ref path) = archive_path {
                    eprintln!("agentx: Archived to {}", path);
                    eprintln!("       Query with: agentx archive 1");
                }

                // Reconnect to fresh DB file
                let new_conn = Self::open_connection(&self.db_path).with_context(|| format!("Failed to reopen DB after archive: {}", self.db_path.display()))?;
                self.conn = new_conn;
                self.db_inode = get_inode(&self.db_path);

                // Init fresh schema
                self.init_db()?;

                // Log reset event to fresh DB
                self.log_reset_event()?;

                Ok(())
            }
            SchemaCompat::StaleProcess => {
                // DB is newer than our code — work with it, don't archive
                Ok(())
            }
        }
    }

    /// Internal: check schema compatibility without taking action.
    fn check_schema_compat(&self) -> Result<SchemaCompat> {
        let version: i32 = self.conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap_or(0);

        // Check what tables exist
        let mut stmt = self.conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
        let tables: std::collections::HashSet<String> = stmt.query_map([], |row| row.get::<_, String>(0))?.filter_map(|r| r.ok()).collect();

        let required: std::collections::HashSet<&str> = ["events", "instances", "kv", "notify_endpoints", "session_bindings", "claude_actor_capabilities"].into_iter().collect();

        if version == 0 {
            // Race handling: another process may be initializing
            if !tables.is_empty() && required.iter().any(|t| tables.contains(*t)) {
                let mut resolved_version = 0i32;
                for _ in 0..20 {
                    let v2: i32 = self.conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap_or(0);
                    if v2 != 0 {
                        resolved_version = v2;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                if resolved_version == SCHEMA_VERSION {
                    return Ok(SchemaCompat::Ok);
                }
                if resolved_version > SCHEMA_VERSION {
                    crate::log::log_warn("db", "schema.stale_process", &format!("DB v{} > code v{}, working with newer schema", resolved_version, SCHEMA_VERSION));
                    return Ok(SchemaCompat::StaleProcess);
                }
                // Timeout exhausted — another process is still initializing.
                // Return Ok rather than falling through to NeedsArchive which
                // would incorrectly archive a valid in-progress DB.
                if resolved_version == 0 {
                    crate::log::log_warn("db", "schema.init_timeout", "Concurrent init poll timed out, assuming OK");
                    return Ok(SchemaCompat::Ok);
                }
            }
            // Fresh DB (no tables) - safe to initialize
            if tables.is_empty() {
                return Ok(SchemaCompat::Ok);
            }
            // Pre-versioned DB with our tables - needs archive
            if required.iter().any(|t| tables.contains(*t)) {
                return Ok(SchemaCompat::NeedsArchive("Pre-versioned DB found".to_string(), None));
            }
            // Has tables but not ours - fresh enough
            return Ok(SchemaCompat::Ok);
        }

        if version != SCHEMA_VERSION {
            if version > SCHEMA_VERSION {
                // DB newer than code - stale process, work with it
                crate::log::log_warn("db", "schema.stale_process", &format!("DB v{} > code v{}, working with newer schema", version, SCHEMA_VERSION));
                return Ok(SchemaCompat::StaleProcess);
            }
            // DB older - needs archive
            return Ok(SchemaCompat::NeedsArchive(format!("DB version mismatch (DB v{}, code v{})", version, SCHEMA_VERSION), Some(version)));
        }

        // Verify required tables exist
        let have_all = required.iter().all(|t| tables.contains(*t));
        if !have_all {
            let missing: Vec<&&str> = required.iter().filter(|t| !tables.contains(**t)).collect();
            return Ok(SchemaCompat::NeedsArchive(format!("DB missing tables {:?}", missing), None));
        }

        // Column guard: verify all expected columns exist (catches partial schema from
        // version bump before migration was written)
        let missing_col: Option<String> = self
            .conn
            .prepare("PRAGMA table_info(instances)")
            .and_then(|mut s| {
                let cols: Vec<String> = s.query_map([], |row| row.get::<_, String>(1))?.filter_map(|r| r.ok()).collect();
                let required = ["tool", "terminal_preset_requested", "terminal_preset_effective", "last_seen", "launch_profile", "model_request"];
                Ok(required.iter().find(|c| !cols.contains(&c.to_string())).map(|s| s.to_string()))
            })
            .unwrap_or(None);
        if let Some(col) = missing_col {
            // No migration introduces the base tool column; retain the existing
            // unsupported-layout archive path instead of attempting a repair.
            let repair_version = if col == "tool" { None } else { Some(version) };
            return Ok(SchemaCompat::NeedsArchive(format!("DB schema missing instances.{}", col), repair_version));
        }

        let missing: Vec<&str> = ADDITIVE_TABLES.iter().filter(|(_, table)| !tables.contains(*table)).map(|(_, table)| *table).collect();
        if !missing.is_empty() {
            return Ok(SchemaCompat::NeedsArchive(format!("DB missing additive tables {missing:?}"), Some(version)));
        }

        if !obsolete_instance_columns(&self.conn)?.is_empty() {
            return Ok(SchemaCompat::NeedsArchive("DB instance storage contains retired columns".into(), Some(version)));
        }

        Ok(SchemaCompat::Ok)
    }

    /// Decide the version to migrate *from* when repairing a DB that may have
    /// been stamped without actually running its migrations.
    ///
    /// Inspect additive tables and columns independently of the recorded stamp.
    /// Walk back to the migration that adds the earliest missing object, but
    /// never skip migrations after the recorded version. A genuinely up-to-date
    /// database just runs the remaining steps.
    fn repair_migrate_from(&self, version: i32) -> i32 {
        let columns: std::collections::HashSet<String> = self.conn.prepare("PRAGMA table_info(instances)").and_then(|mut s| Ok(s.query_map([], |row| row.get::<_, String>(1))?.filter_map(|r| r.ok()).collect())).unwrap_or_default();

        // (migration version, a column that migration introduces)
        const COLUMN_MIGRATIONS: &[(i32, &str)] = &[(17, "terminal_preset_requested"), (18, "last_seen"), (24, "launch_profile"), (25, "model_request")];
        let mut migrate_from = if version >= SCHEMA_VERSION { SCHEMA_VERSION - 1 } else { version };
        for (migration, column) in COLUMN_MIGRATIONS {
            if !columns.contains(*column) {
                migrate_from = migrate_from.min(migration - 1);
            }
        }
        let tables: std::collections::HashSet<String> = self.conn.prepare("SELECT name FROM sqlite_master WHERE type='table'").and_then(|mut s| s.query_map([], |row| row.get(0))?.collect::<rusqlite::Result<_>>()).unwrap_or_default();
        for (migration, table) in ADDITIVE_TABLES {
            if !tables.contains(*table) {
                migrate_from = migrate_from.min(migration - 1);
            }
        }

        // Additive successors must still repair prematurely stamped projection.
        if obsolete_instance_columns(&self.conn).is_ok_and(|columns| !columns.is_empty()) {
            migrate_from = migrate_from.min(26);
        }

        // Never advance beyond the recorded version or an absent prerequisite.
        migrate_from
    }

    /// Try in-place migration for consecutive schema versions.
    ///
    /// Returns `Ok(false)` for an unsupported legacy version. Known migrations
    /// roll back and preserve the database on failure.
    fn try_apply_migrations(&self, old_version: i32) -> Result<bool> {
        if old_version <= 0 || old_version >= SCHEMA_VERSION {
            return Ok(false);
        }
        let tx = self.conn.unchecked_transaction()?;
        for next_version in (old_version + 1)..=SCHEMA_VERSION {
            if next_version == 17 {
                let has_launch_context = tx.prepare("PRAGMA table_info(instances)")?.query_map([], |row| row.get::<_, String>(1))?.filter_map(|r| r.ok()).any(|col| col == "launch_context");
                if !has_launch_context {
                    tx.execute("ALTER TABLE instances ADD COLUMN launch_context TEXT DEFAULT ''", [])?;
                }
            }
            let Some((_, sql)) = MIGRATIONS.iter().find(|(v, _)| *v == next_version) else {
                return Ok(false);
            };
            let has_status_time = if next_version == 18 {
                let mut statement = tx.prepare("PRAGMA table_info(instances)")?;
                let columns: Vec<String> = statement.query_map([], |row| row.get::<_, String>(1))?.filter_map(|row| row.ok()).collect();
                columns.iter().any(|column| column == "status_time")
            } else {
                false
            };
            let has_launch_profile = next_version == 24 && tx.prepare("PRAGMA table_info(instances)")?.query_map([], |row| row.get::<_, String>(1))?.collect::<rusqlite::Result<Vec<_>>>()?.iter().any(|column| column == "launch_profile");
            let has_model_request = next_version == 25 && tx.prepare("PRAGMA table_info(instances)")?.query_map([], |row| row.get::<_, String>(1))?.collect::<rusqlite::Result<Vec<_>>>()?.iter().any(|column| column == "model_request");
            if !has_launch_profile && !has_model_request {
                tx.execute_batch(sql)?;
            }
            if next_version == 18 && has_status_time {
                tx.execute("UPDATE instances SET last_seen = status_time WHERE last_seen = 0", [])?;
            }
            if next_version == 27 {
                project_instance_columns(&tx).context(PreserveInstanceStorage)?;
            }
            tx.execute_batch(&format!("PRAGMA user_version = {}", next_version))?;
        }
        // IF NOT EXISTS can silently accept an existing view with the same
        // name. Verify the promised tables before committing a repaired stamp.
        let tables: std::collections::HashSet<String> = tx.prepare("SELECT name FROM sqlite_master WHERE type='table'")?.query_map([], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
        for (_, table) in ADDITIVE_TABLES {
            anyhow::ensure!(tables.contains(*table), "Schema migration did not create required table {table}");
        }
        tx.commit()?;
        Ok(true)
    }

    /// Archive current database at a given path.
    /// WAL checkpoint, copy to archive dir (sibling archive/ directory), delete original.
    ///
    /// Known, deliberately deferred limitation on Windows: the `remove_file`
    /// below can fail even though the caller already released its own
    /// connection (see `ensure_schema`). Windows only allows deleting a file
    /// while other handles remain open if *every* one of those handles was
    /// opened with `FILE_SHARE_DELETE` — and SQLite's Windows VFS (and thus
    /// rusqlite's default `Connection::open`) does not request that flag.
    /// Unix has no equivalent restriction; `unlink` on an open file always
    /// succeeds there, which is why this asymmetry doesn't show up in the
    /// Unix path at all.
    ///
    /// In practice this only bites when a schema-version mismatch forces an
    /// archive-and-reset (rare) while some other agentx process — another agent
    /// instance, a relay worker, a hook invocation — still has the same DB
    /// file open anywhere on the machine. When that happens, this call
    /// returns a real, un-recoverable-in-place `Err`; there is no retry that
    /// helps within this function. A proper fix would need a different
    /// strategy entirely — e.g. copying the live file's contents into a fresh
    /// DB and resetting schema in place, rather than deleting the original —
    /// so no cross-process handle-closing coordination is required. That is a
    /// larger change than this narrow Windows-support pass and is deferred
    /// given how rare schema mismatches are in practice.
    fn archive_db_at(db_path: &std::path::Path) -> Result<Option<String>> {
        if !db_path.exists() {
            return Ok(None);
        }

        let db_wal = db_path.with_extension("db-wal");
        let db_shm = db_path.with_extension("db-shm");

        // WAL checkpoint before archive
        if let Ok(temp_conn) = Connection::open(db_path) {
            let _ = temp_conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE)");
        }

        // Create archive directory next to the DB file
        let parent = db_path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let timestamp = Utc::now().format("%Y-%m-%d_%H%M%S").to_string();
        let archive_dir = parent.join("archive").join(format!("session-{}", timestamp));
        std::fs::create_dir_all(&archive_dir)?;

        // Copy DB files to archive
        let db_name = db_path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("agentx.db"));
        std::fs::copy(db_path, archive_dir.join(db_name))?;
        if db_wal.exists() {
            let wal_name = format!("{}-wal", db_name.to_string_lossy());
            let _ = std::fs::copy(&db_wal, archive_dir.join(wal_name));
        }
        if db_shm.exists() {
            let shm_name = format!("{}-shm", db_name.to_string_lossy());
            let _ = std::fs::copy(&db_shm, archive_dir.join(shm_name));
        }

        // Delete original
        std::fs::remove_file(db_path)?;
        let _ = std::fs::remove_file(&db_wal);
        let _ = std::fs::remove_file(&db_shm);

        Ok(Some(archive_dir.to_string_lossy().to_string()))
    }

    /// Snapshot running instances to pidtrack before DB archive.
    ///
    /// Writes live instances (with their PIDs) to ~/.agentx/.tmp/launched_pids.json
    /// so orphan recovery can re-register them into the fresh DB after schema bump.
    fn snapshot_running_to_pidtrack(&self) {
        let Ok(mut stmt) = self.conn.prepare(
            "SELECT i.name, i.pid, i.tool, i.directory, i.session_id, p.process_id, \
                    n_pty.port AS notify_port, n_inj.port AS inject_port \
             FROM instances i \
             LEFT JOIN process_bindings p ON i.name = p.instance_name \
             LEFT JOIN notify_endpoints n_pty ON i.name = n_pty.instance AND n_pty.kind = 'pty' \
             LEFT JOIN notify_endpoints n_inj ON i.name = n_inj.instance AND n_inj.kind = 'inject' \
             WHERE i.pid IS NOT NULL",
        ) else {
            return;
        };

        let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,         // name
                row.get::<_, i64>(1)?,            // pid
                row.get::<_, Option<String>>(2)?, // tool
                row.get::<_, Option<String>>(3)?, // directory
                row.get::<_, Option<String>>(4)?, // session_id
                row.get::<_, Option<String>>(5)?, // process_id
                row.get::<_, Option<i64>>(6)?,    // notify_port
                row.get::<_, Option<i64>>(7)?,    // inject_port
            ))
        }) else {
            return;
        };

        let pidfile_path = self.db_path.parent().unwrap_or_else(|| std::path::Path::new(".")).join(".tmp").join("launched_pids.json");

        // Read existing pidfile
        let mut piddata: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(&pidfile_path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();

        for row in rows.flatten() {
            let (name, pid, tool, directory, session_id, process_id, notify_port, inject_port) = row;
            let alive = crate::runtime::process_tracking::is_alive(pid as u32);
            if !alive {
                continue;
            }

            piddata.insert(
                pid.to_string(),
                serde_json::json!({
                    "tool": tool.unwrap_or_else(|| "claude".to_string()),
                    "names": [name],
                    "directory": directory.unwrap_or_default(),
                    "process_id": process_id.unwrap_or_default(),
                    "session_id": session_id.unwrap_or_default(),
                    "notify_port": notify_port.unwrap_or(0),
                    "inject_port": inject_port.unwrap_or(0),
                    "launched_at": now_epoch_f64(),
                }),
            );
        }

        if let Ok(json) = serde_json::to_string(&piddata) {
            let _ = std::fs::write(&pidfile_path, json);
        }
    }

    /// Log _device reset event + set relay timestamp. Call after any DB archive/reset.
    pub fn log_reset_event(&self) -> Result<()> {
        // Derive agentx_dir from db_path (db is at agentx_dir/agentx.db)
        let agentx_dir = self.db_path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let device_id = std::fs::read_to_string(agentx_dir.join(".tmp").join("device_uuid")).unwrap_or_else(|_| "unknown".to_string()).trim().to_string();

        self.log_event("life", "_device", &serde_json::json!({"action": "reset", "device": device_id}))?;

        self.kv_set("relay_local_reset_ts", Some(&now_epoch_f64().to_string()))?;

        Ok(())
    }

    /// Remove all event subscriptions owned by an instance.
    ///
    /// Subscriptions are stored as kv entries with key 'events_sub:sub-{hash}'
    /// and a JSON value containing a "caller" field.
    pub fn cleanup_subscriptions(&self, name: &str) -> Result<u32> {
        // Delegates to db::subscriptions; events_sub: kv ownership lives there.
        subscriptions::cleanup_subscriptions(self, name)
    }

    /// Remove delivery-only thread memberships for an instance.
    ///
    /// This is used when a stopped name is being reused by a fresh instance:
    /// normal stop/resume should preserve memberships, but identity replacement
    /// must not inherit old thread state.
    pub fn cleanup_thread_memberships_for_name_reuse(&self, name: &str) -> Result<u32> {
        // Delegates to db::subscriptions; events_sub: kv ownership lives there.
        subscriptions::cleanup_thread_memberships_for_name_reuse(self, name)
    }

    /// Return active members of a thread in join order.
    pub fn get_thread_members(&self, thread: &str) -> Vec<String> {
        // Delegates to db::subscriptions; events_sub: kv ownership lives there.
        subscriptions::get_thread_members(self, thread)
    }

    /// Upsert memberships for recipients of a thread message.
    pub fn add_thread_memberships(&self, thread: &str, sender: Option<&str>, recipients: &[String]) {
        // Delegates to db::subscriptions; events_sub: kv ownership lives there.
        subscriptions::add_thread_memberships(self, thread, sender, recipients);
    }

    /// Send a system notification message (simplified inline version).
    /// Parses @mentions, computes scope, inserts message event.
    pub fn send_system_message(&self, sender_name: &str, message: &str) -> Result<Vec<String>> {
        // Delegates to db::subscriptions; events_sub: kv ownership lives there.
        subscriptions::send_system_message(self, sender_name, message)
    }

    /// Like `send_system_message` but lets the caller specify `sender_kind`
    /// ("instance" | "external" | "system"). Used by subscription on-hit to
    /// preserve the sub caller's real identity on the event.
    pub fn send_message_as(&self, sender_name: &str, sender_kind: &str, message: &str) -> Result<Vec<String>> {
        // Delegates to db::subscriptions; events_sub: kv ownership lives there.
        subscriptions::send_message_as(self, sender_name, sender_kind, message)
    }
}

/// Generate ISO timestamp for current time.
pub(super) fn chrono_now_iso() -> String {
    crate::shared::time::now_iso()
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use rusqlite::{Connection, params};
    use std::path::PathBuf;

    /// Clean up test database
    pub(super) fn cleanup_test_db(path: PathBuf) {
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(PathBuf::from(format!("{}-wal", path.display())));
        let _ = std::fs::remove_file(PathBuf::from(format!("{}-shm", path.display())));
    }

    #[cfg(unix)]
    fn mode(path: &std::path::Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[cfg(unix)]
    #[test]
    fn open_raw_creates_private_database_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("agentx.db");

        let db = AgentxDb::open_raw(&db_path).unwrap();
        db.conn().execute("CREATE TABLE permission_probe (id INTEGER)", []).unwrap();
        db.conn().execute("INSERT INTO permission_probe VALUES (1)", []).unwrap();

        assert_eq!(mode(&db_path), 0o600);
        assert_eq!(mode(&crate::paths::sidecar_path(&db_path, "-wal")), 0o600);
        assert_eq!(mode(&crate::paths::sidecar_path(&db_path, "-shm")), 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn open_raw_restricts_existing_database_files() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("agentx.db");
        let first = AgentxDb::open_raw(&db_path).unwrap();
        first.conn().execute("CREATE TABLE permission_probe (id INTEGER)", []).unwrap();
        first.conn().execute("INSERT INTO permission_probe VALUES (1)", []).unwrap();

        for path in [db_path.clone(), crate::paths::sidecar_path(&db_path, "-wal"), crate::paths::sidecar_path(&db_path, "-shm")] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
        }

        let _second = AgentxDb::open_raw(&db_path).unwrap();

        assert_eq!(mode(&db_path), 0o600);
        assert_eq!(mode(&crate::paths::sidecar_path(&db_path, "-wal")), 0o600);
        assert_eq!(mode(&crate::paths::sidecar_path(&db_path, "-shm")), 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn open_raw_restricts_sidecars_for_non_db_filename() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("state.sqlite");

        let db = AgentxDb::open_raw(&db_path).unwrap();
        db.conn().execute("CREATE TABLE permission_probe (id INTEGER)", []).unwrap();
        db.conn().execute("INSERT INTO permission_probe VALUES (1)", []).unwrap();

        let wal_path = tmp.path().join("state.sqlite-wal");
        let shm_path = tmp.path().join("state.sqlite-shm");
        std::fs::set_permissions(&wal_path, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::set_permissions(&shm_path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let _second = AgentxDb::open_raw(&db_path).unwrap();

        assert_eq!(mode(&wal_path), 0o600);
        assert_eq!(mode(&shm_path), 0o600);
    }

    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn open_restricts_the_configured_agentx_directory_and_database() {
        use std::os::unix::fs::PermissionsExt;

        let (_tmp, agentx_dir, _home, _guard) = crate::integrations::hooks::test_helpers::isolated_test_env();
        std::fs::set_permissions(&agentx_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

        let db = AgentxDb::open().unwrap();

        assert_eq!(mode(&agentx_dir), 0o700);
        assert_eq!(mode(db.path()), 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn reconnect_if_stale_resecures_replaced_database() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("agentx.db");
        let mut db = AgentxDb::open_raw(&db_path).unwrap();

        // Simulate another process replacing the DB with a broad-mode file
        // (new inode), as reset/schema-archive does.
        std::fs::remove_file(&db_path).unwrap();
        let _ = std::fs::remove_file(crate::paths::sidecar_path(&db_path, "-wal"));
        let _ = std::fs::remove_file(crate::paths::sidecar_path(&db_path, "-shm"));
        drop(AgentxDb::open_raw(&db_path).unwrap());
        std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o644)).unwrap();

        assert!(db.reconnect_if_stale());
        assert_eq!(mode(&db_path), 0o600);
    }

    #[test]
    fn test_open_raw_rejects_non_temp_path() {
        crate::paths::test_roots::with_temp_boundary("db::tests::test_open_raw_rejects_non_temp_path", |_, outside| {
            assert_rejected_without_database(&outside.join("agentx.db"));
        });
    }

    fn assert_rejected_without_database(path: &std::path::Path) {
        let panic = match std::panic::catch_unwind(|| AgentxDb::open_raw(path)) {
            Err(panic) => panic,
            Ok(_) => panic!("outside path must panic before open"),
        };
        let message = panic.downcast_ref::<String>().map(String::as_str).or_else(|| panic.downcast_ref::<&str>().copied()).unwrap_or("");
        assert!(message.contains("not a registered or temp-tree path"), "unexpected panic: {message}");
        for file in [path.to_path_buf(), crate::paths::sidecar_path(path, "-wal"), crate::paths::sidecar_path(path, "-shm")] {
            assert!(!file.exists(), "path guard created {}", file.display());
        }
    }

    #[test]
    fn test_open_raw_allows_temp_path() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("allowed.db");

        let db = AgentxDb::open_raw(&db_path).unwrap();

        assert_eq!(db.path(), db_path);
    }

    #[cfg(unix)]
    #[test]
    fn test_open_raw_rejects_temp_symlink_to_non_temp_path() {
        crate::paths::test_roots::with_temp_boundary("db::tests::test_open_raw_rejects_temp_symlink_to_non_temp_path", |inside, outside| {
            let link = inside.join("escape");
            std::os::unix::fs::symlink(outside, &link).unwrap();
            assert_rejected_without_database(&link.join("agentx.db"));
            assert_eq!(std::fs::read_dir(outside).unwrap().count(), 0);
        });
    }

    #[test]
    fn test_all_methods_return_ok_none_when_not_found() {
        let (db, db_path) = setup_full_test_db();

        // All these should return Ok(None) for non-existent data
        assert!(db.get_instance_status("nonexistent").unwrap().is_none());
        assert!(db.get_status("nonexistent").unwrap().is_none());
        assert!(db.get_process_binding("nonexistent").unwrap().is_none());
        assert!(db.get_transcript_path("nonexistent").unwrap().is_none());
        assert!(db.get_instance_snapshot("nonexistent").unwrap().is_none());

        cleanup_test_db(db_path);
    }

    /// Create a test DB with full init_db() schema
    pub(super) fn setup_full_test_db() -> (AgentxDb, PathBuf) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let temp_dir = std::env::temp_dir();
        let test_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let db_path = temp_dir.join(format!("test_agentx_full_{}_{}.db", std::process::id(), test_id));

        let db = AgentxDb::open_at(&db_path).unwrap();
        (db, db_path)
    }

    #[test]
    fn schema_initialization_failure_rolls_back() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_raw(&dir.path().join("broken.db")).unwrap();
        db.conn().execute_batch("CREATE TABLE instances (name TEXT PRIMARY KEY);").unwrap();
        assert!(db.init_db().is_err());
        let mut statement = db.conn().prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap();
        let tables: Vec<String> = statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect();
        assert_eq!(tables, vec!["instances"]);
        let version: i32 = db.conn().query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, 0);
        db.conn().execute_batch("DROP TABLE instances;").unwrap();
        db.init_db().unwrap();
    }

    #[test]
    fn concurrent_schema_initializers_succeed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("concurrent.db");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        // Open connections before spawning: a failed open must fail the test,
        // rather than leave other workers stuck at a barrier forever.
        let connections: Vec<_> = (0..4).map(|_| AgentxDb::open_raw(&path).unwrap()).collect();
        let workers: Vec<_> = connections
            .into_iter()
            .map(|db| {
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    db.init_db().unwrap();
                    db.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES ('now','test','probe','{}')", []).unwrap();
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        let db = AgentxDb::open_raw(&path).unwrap();
        let count: i64 = db.conn().query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 4);
    }

    #[test]
    fn test_init_db_creates_all_tables() {
        let (db, db_path) = setup_full_test_db();

        let tables: Vec<String> = db.conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").unwrap().query_map([], |row| row.get(0)).unwrap().filter_map(|r| r.ok()).collect();

        assert!(tables.contains(&"events".to_string()));
        assert!(tables.contains(&"instances".to_string()));
        assert!(tables.contains(&"kv".to_string()));
        assert!(tables.contains(&"notify_endpoints".to_string()));
        assert!(tables.contains(&"process_bindings".to_string()));
        assert!(tables.contains(&"session_bindings".to_string()));

        cleanup_test_db(db_path);
    }

    #[test]
    fn test_init_db_sets_schema_version() {
        let (db, db_path) = setup_full_test_db();

        let version: i32 = db.conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        cleanup_test_db(db_path);
    }

    #[test]
    fn test_init_db_idempotent() {
        let (db, db_path) = setup_full_test_db();

        // Call init_db again - should be no-op
        db.init_db().unwrap();

        let version: i32 = db.conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        cleanup_test_db(db_path);
    }

    #[test]
    fn test_init_db_creates_events_v_view() {
        let (db, db_path) = setup_full_test_db();

        // Check view exists
        let count: i64 = db.conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='view' AND name='events_v'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 1);

        cleanup_test_db(db_path);
    }

    #[test]
    fn test_init_db_creates_fts5_table() {
        let (db, db_path) = setup_full_test_db();

        // FTS5 tables show up as 'table' in sqlite_master
        let count: i64 = db.conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE name='events_fts'", [], |row| row.get(0)).unwrap();
        assert!(count > 0, "events_fts should exist");

        cleanup_test_db(db_path);
    }

    #[test]
    fn test_init_db_fts_trigger_indexes_on_insert() {
        let (db, db_path) = setup_full_test_db();

        // Insert an event
        db.conn.execute("INSERT INTO events (timestamp, type, instance, data) VALUES ('2026-01-01T00:00:00Z', 'message', 'luna', ?)", params![serde_json::json!({"from": "luna", "text": "hello world"}).to_string()]).unwrap();

        // Search FTS
        let count: i64 = db.conn.query_row("SELECT COUNT(*) FROM events_fts WHERE searchable MATCH 'hello'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 1);

        cleanup_test_db(db_path);
    }

    #[test]
    fn test_v19_migration_removes_orphans_and_tracks_deletes_in_place() {
        let (mut db, db_path) = setup_full_test_db();
        db.conn.execute_batch("DROP TRIGGER events_fts_delete; PRAGMA user_version = 18;").unwrap();
        db.log_event("message", "sender", &serde_json::json!({"text": "purgedword"})).unwrap();
        db.conn.execute("DELETE FROM events", []).unwrap();
        db.log_event("message", "sender", &serde_json::json!({"text": "retainedword"})).unwrap();
        db.kv_set("migration_sentinel", Some("preserved")).unwrap();
        let inode = get_inode(&db_path);
        db.ensure_schema().unwrap();
        assert_eq!(get_inode(&db_path), inode);
        assert_eq!(db.kv_get("migration_sentinel").unwrap().as_deref(), Some("preserved"));
        let count = |word: &str| db.conn.query_row("SELECT count(*) FROM events_fts WHERE events_fts MATCH ?", [word], |r| r.get::<_, i64>(0)).unwrap();
        assert_eq!(count("purgedword"), 0);
        assert_eq!(count("retainedword"), 1);
        db.conn.execute("DELETE FROM events", []).unwrap();
        assert_eq!(count("retainedword"), 0);
        cleanup_test_db(db_path);
    }

    #[test]
    fn test_check_schema_compat_fresh_db() {
        let (db, db_path) = setup_full_test_db();
        match db.check_schema_compat().unwrap() {
            SchemaCompat::Ok => {} // expected
            other => panic!(
                "Expected SchemaCompat::Ok, got {:?}",
                match other {
                    SchemaCompat::NeedsArchive(r, v) => format!("NeedsArchive({}, {:?})", r, v),
                    SchemaCompat::StaleProcess => "StaleProcess".to_string(),
                    SchemaCompat::Ok => unreachable!(),
                }
            ),
        }
        cleanup_test_db(db_path);
    }

    #[test]
    fn test_ensure_schema_fresh_db() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1000);

        let temp_dir = std::env::temp_dir();
        let test_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let db_path = temp_dir.join(format!("test_agentx_ensure_{}_{}.db", std::process::id(), test_id));

        let mut db = AgentxDb::open_raw(&db_path).unwrap();
        db.ensure_schema().unwrap();

        // Should have full schema
        let version: i32 = db.conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        cleanup_test_db(db_path);
    }

    #[test]
    fn test_ensure_schema_archives_old_version() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(2000);

        let temp_dir = std::env::temp_dir();
        let test_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let db_path = temp_dir.join(format!("test_agentx_archive_{}_{}.db", std::process::id(), test_id));

        // Create a DB with old schema version
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE events (id INTEGER PRIMARY KEY, timestamp TEXT, type TEXT, instance TEXT, data TEXT);
                 CREATE TABLE instances (name TEXT PRIMARY KEY, created_at REAL NOT NULL);
                 CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT);
                 CREATE TABLE notify_endpoints (instance TEXT, kind TEXT, port INTEGER, updated_at REAL, PRIMARY KEY(instance, kind));
                 CREATE TABLE session_bindings (session_id TEXT PRIMARY KEY, instance_name TEXT NOT NULL, created_at REAL NOT NULL);
                 PRAGMA user_version = 5;",
            )
            .unwrap();
        }

        let mut db = AgentxDb::open_raw(&db_path).unwrap();
        db.ensure_schema().unwrap();

        // Should have been archived and recreated at current version
        let version: i32 = db.conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        // Archive directory should exist
        let archive_dir = temp_dir.join("archive");
        if archive_dir.exists() {
            let _ = std::fs::remove_dir_all(&archive_dir);
        }

        cleanup_test_db(db_path);
    }

    #[test]
    fn test_ensure_schema_migrates_v16_to_v18_in_place_without_status_time() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(2500);

        let temp_dir = std::env::temp_dir();
        let test_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let db_path = temp_dir.join(format!("test_agentx_migrate_{}_{}.db", std::process::id(), test_id));

        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE events (id INTEGER PRIMARY KEY, timestamp TEXT, type TEXT, instance TEXT, data TEXT);
                 CREATE TABLE instances (
                     name TEXT PRIMARY KEY,
                     tool TEXT DEFAULT 'claude',
                     origin_device_id TEXT DEFAULT '',
                     created_at REAL NOT NULL,
                     launch_context TEXT DEFAULT ''
                 );
                 CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT);
                 CREATE TABLE notify_endpoints (instance TEXT, kind TEXT, port INTEGER, updated_at REAL, PRIMARY KEY(instance, kind));
                 CREATE TABLE session_bindings (session_id TEXT PRIMARY KEY, instance_name TEXT NOT NULL, created_at REAL NOT NULL);
                 CREATE VIRTUAL TABLE events_fts USING fts5(searchable);
                 PRAGMA user_version = 16;",
            )
            .unwrap();
            conn.execute("INSERT INTO instances (name, tool, created_at, launch_context) VALUES (?1, ?2, ?3, ?4)", rusqlite::params!["luna", "claude", 1.0f64, r#"{"terminal_preset":"ghostty-tab"}"#]).unwrap();
        }

        let mut db = AgentxDb::open_raw(&db_path).unwrap();
        db.ensure_schema().unwrap();

        let version: i32 = db.conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        let preset: String = db.conn.query_row("SELECT terminal_preset_effective FROM instances WHERE name = ?", params!["luna"], |row| row.get(0)).unwrap();
        assert_eq!(preset, "ghostty-tab");
        let launch_context: String = db.conn.query_row("SELECT launch_context FROM instances WHERE name = ?", params!["luna"], |row| row.get(0)).unwrap();
        assert_eq!(launch_context, r#"{"terminal_preset":"ghostty-tab"}"#);
        let last_seen: i64 = db.conn.query_row("SELECT last_seen FROM instances WHERE name = ?", params!["luna"], |row| row.get(0)).unwrap();
        assert_eq!(last_seen, 0);

        cleanup_test_db(db_path);
    }

    #[test]
    fn test_ensure_schema_migrates_v17_to_v18_using_status_time() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(2750);

        let temp_dir = std::env::temp_dir();
        let test_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let db_path = temp_dir.join(format!("test_agentx_migrate_status_time_{}_{}.db", std::process::id(), test_id));

        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE events (id INTEGER PRIMARY KEY, timestamp TEXT, type TEXT, instance TEXT, data TEXT);
                 CREATE TABLE instances (
                     name TEXT PRIMARY KEY,
                     tool TEXT DEFAULT 'claude',
                     origin_device_id TEXT DEFAULT '',
                     status_time INTEGER DEFAULT 0,
                     created_at REAL NOT NULL,
                     launch_context TEXT DEFAULT '',
                     terminal_preset_requested TEXT DEFAULT '',
                     terminal_preset_effective TEXT DEFAULT ''
                 );
                 CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT);
                 CREATE TABLE notify_endpoints (instance TEXT, kind TEXT, port INTEGER, updated_at REAL, PRIMARY KEY(instance, kind));
                 CREATE TABLE session_bindings (session_id TEXT PRIMARY KEY, instance_name TEXT NOT NULL, created_at REAL NOT NULL);
                 CREATE VIRTUAL TABLE events_fts USING fts5(searchable);
                 PRAGMA user_version = 17;",
            )
            .unwrap();
            conn.execute("INSERT INTO instances (name, tool, status_time, created_at) VALUES (?1, ?2, ?3, ?4)", rusqlite::params!["luna", "claude", 123i64, 1.0f64]).unwrap();
        }

        let mut db = AgentxDb::open_raw(&db_path).unwrap();
        db.ensure_schema().unwrap();

        let last_seen: i64 = db.conn.query_row("SELECT last_seen FROM instances WHERE name = ?", params!["luna"], |row| row.get(0)).unwrap();
        assert_eq!(last_seen, 123);

        cleanup_test_db(db_path);
    }

    #[test]
    fn test_ensure_schema_column_guard() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(3000);

        let temp_dir = std::env::temp_dir();
        let test_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let db_path = temp_dir.join(format!("test_agentx_colguard_{}_{}.db", std::process::id(), test_id));

        // Create a DB at current version but missing 'tool' column
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(&format!(
                "CREATE TABLE events (id INTEGER PRIMARY KEY, timestamp TEXT, type TEXT, instance TEXT, data TEXT);
                 CREATE TABLE instances (name TEXT PRIMARY KEY, created_at REAL NOT NULL);
                 CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT);
                 CREATE TABLE notify_endpoints (instance TEXT, kind TEXT, port INTEGER, updated_at REAL, PRIMARY KEY(instance, kind));
                 CREATE TABLE session_bindings (session_id TEXT PRIMARY KEY, instance_name TEXT NOT NULL, created_at REAL NOT NULL);
                 CREATE TABLE claude_actor_capabilities (
                     token TEXT PRIMARY KEY,
                     session_id TEXT NOT NULL,
                     tool_use_id TEXT NOT NULL,
                     agent_id TEXT NOT NULL DEFAULT '',
                     instance_name TEXT NOT NULL,
                     created_at INTEGER NOT NULL,
                     expires_at INTEGER NOT NULL,
                     last_seen INTEGER NOT NULL,
                     UNIQUE(session_id, tool_use_id, agent_id)
                 );
                 PRAGMA user_version = {};",
                SCHEMA_VERSION
            ))
            .unwrap();
        }

        let mut db = AgentxDb::open_raw(&db_path).unwrap();

        // check_schema_compat should detect missing column
        match db.check_schema_compat().unwrap() {
            SchemaCompat::NeedsArchive(reason, _) => {
                assert!(reason.contains("instances.tool"), "reason: {}", reason);
            }
            _ => panic!("Expected NeedsArchive for missing tool column"),
        }

        // ensure_schema should fix it
        db.ensure_schema().unwrap();

        let version: i32 = db.conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        cleanup_test_db(db_path);
    }

    /// Regression test for issue #16: init_db() stamped user_version=17 without
    /// actually adding the terminal_preset_* columns. ensure_schema must repair
    /// this via migration instead of archiving (which would lose data).
    #[test]
    fn test_ensure_schema_repairs_stamped_but_not_migrated_db() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(4000);

        let temp_dir = std::env::temp_dir();
        let test_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let db_path = temp_dir.join(format!("test_agentx_repair_{}_{}.db", std::process::id(), test_id));

        // Simulate the bug: create a v16-style DB but stamp it as v17
        // (this is what init_db() did — CREATE IF NOT EXISTS is a no-op on
        // existing tables, then it unconditionally set user_version = 17)
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE events (id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp TEXT NOT NULL, type TEXT NOT NULL, instance TEXT NOT NULL, data TEXT NOT NULL);
                 CREATE TABLE instances (
                     name TEXT PRIMARY KEY,
                     session_id TEXT UNIQUE,
                     parent_session_id TEXT,
                     parent_name TEXT,
                     last_event_id INTEGER DEFAULT 0,
                     status TEXT DEFAULT 'active',
                     status_time INTEGER DEFAULT 0,
                     status_context TEXT DEFAULT '',
                     status_detail TEXT DEFAULT '',
                     last_stop INTEGER DEFAULT 0,
                     directory TEXT,
                     created_at REAL NOT NULL,
                     transcript_path TEXT DEFAULT '',
                     tcp_mode INTEGER DEFAULT 0,
                     wait_timeout INTEGER DEFAULT 86400,
                     background INTEGER DEFAULT 0,
                     background_log_file TEXT DEFAULT '',
                     name_announced INTEGER DEFAULT 0,
                     agent_id TEXT UNIQUE,
                     running_tasks TEXT DEFAULT '',
                     origin_device_id TEXT DEFAULT '',
                     hints TEXT DEFAULT '',
                     subagent_timeout INTEGER,
                     tool TEXT DEFAULT 'claude',
                     launch_args TEXT DEFAULT '',
                     idle_since TEXT DEFAULT '',
                     pid INTEGER DEFAULT NULL,
                     launch_context TEXT DEFAULT ''
                 );
                 CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT);
                 CREATE TABLE notify_endpoints (instance TEXT NOT NULL, kind TEXT NOT NULL, port INTEGER NOT NULL, updated_at REAL NOT NULL, PRIMARY KEY(instance, kind));
                 CREATE TABLE session_bindings (session_id TEXT PRIMARY KEY, instance_name TEXT NOT NULL, created_at REAL NOT NULL);
                 CREATE TABLE process_bindings (process_id TEXT PRIMARY KEY, session_id TEXT, instance_name TEXT, updated_at REAL NOT NULL);
                 CREATE VIRTUAL TABLE events_fts USING fts5(searchable);
                 PRAGMA user_version = 17;",
            )
            .unwrap();
            // Insert test data that should survive the repair
            conn.execute("INSERT INTO instances (name, tool, created_at) VALUES ('luna', 'claude', 1.0)", []).unwrap();
        }

        // Verify columns are missing before repair
        {
            let conn = Connection::open(&db_path).unwrap();
            let cols: Vec<String> = conn.prepare("PRAGMA table_info(instances)").unwrap().query_map([], |row| row.get::<_, String>(1)).unwrap().filter_map(|r| r.ok()).collect();
            assert!(!cols.contains(&"terminal_preset_requested".to_string()), "column should be missing before repair");
        }

        let mut db = AgentxDb::open_raw(&db_path).unwrap();
        db.ensure_schema().unwrap();

        // Should be at current version
        let version: i32 = db.conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        // Columns should now exist
        let cols: Vec<String> = db.conn.prepare("PRAGMA table_info(instances)").unwrap().query_map([], |row| row.get::<_, String>(1)).unwrap().filter_map(|r| r.ok()).collect();
        assert!(cols.contains(&"terminal_preset_requested".to_string()), "terminal_preset_requested column should exist after repair");
        assert!(cols.contains(&"terminal_preset_effective".to_string()), "terminal_preset_effective column should exist after repair");

        // Test data should have survived (not archived)
        let name: String = db.conn.query_row("SELECT name FROM instances WHERE name = 'luna'", [], |row| row.get(0)).unwrap();
        assert_eq!(name, "luna");

        cleanup_test_db(db_path);
    }
    #[test]
    fn test_ensure_schema_profile_addition_and_stamp_repair_preserve_data() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("agentx.db");
        let mut db = AgentxDb::open_at(&path).unwrap();
        db.kv_set("profile-schema-marker", Some("keep")).unwrap();
        db.conn().execute_batch("ALTER TABLE instances DROP COLUMN launch_profile; PRAGMA user_version=23;").unwrap();
        db.ensure_schema().unwrap();
        assert_eq!(db.kv_get("profile-schema-marker").unwrap().as_deref(), Some("keep"));
        db.conn().execute_batch("ALTER TABLE instances DROP COLUMN launch_profile;").unwrap();
        db.ensure_schema().unwrap();
        assert_eq!(db.kv_get("profile-schema-marker").unwrap().as_deref(), Some("keep"));
        let columns: Vec<String> = db.conn().prepare("PRAGMA table_info(instances)").unwrap().query_map([], |row| row.get(1)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert!(columns.iter().any(|column| column == "launch_profile"));
    }

    #[test]
    fn instance_storage_upgrade_preserves_native_state_and_repairs_stamps() {
        for version in [26, SCHEMA_VERSION] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("agentx.db");
            let mut db = AgentxDb::open_at(&path).unwrap();
            let created_at = f64::from_bits(1_700_000_000.123_456_f64.to_bits() + 1);
            db.conn.execute("INSERT INTO instances(name,session_id,created_at,last_event_id,running_tasks) VALUES('luna','root-session',?1,17,'retained-tasks')", [created_at]).unwrap();
            db.conn
                .execute_batch(
                    "INSERT INTO instances(name,session_id,parent_name,parent_session_id,created_at) VALUES('child','child-session','luna','root-session',2);
                 INSERT INTO participant_sessions(id,provider,scope,session_id,participant_name,incarnation,delivered_through,snapshot) VALUES(1,'codex','scope','root-session','luna','exact-incarnation',17,'frozen-snapshot');
                 INSERT INTO participant_attachments(binding_id,lineage,target,participant_name,incarnation,execution,owner,owner_pid,active) VALUES('binding',1,'frozen-target','luna','exact-incarnation','execution','owner',42,1);
                 INSERT INTO inbox_claims(batch_id,lineage,claim,messages,resolution) VALUES('batch',1,'frozen-claim','frozen-messages','open');
                 ALTER TABLE instances ADD COLUMN retired_metadata TEXT;",
                )
                .unwrap();
            db.set_session_binding("root-session", "luna").unwrap();
            db.set_process_binding("process", "root-session", "luna").unwrap();
            db.upsert_notify_endpoint("luna", "plugin", 12345).unwrap();
            db.kv_set("service:database_generation", Some("same-generation")).unwrap();
            db.log_event("message", "luna", &serde_json::json!({"text":"pending message"})).unwrap();
            let retained_snapshot = serde_json::json!({"name":"luna","created_at":created_at,"last_event_id":17,"parent_name":null});
            let mut old_snapshot = retained_snapshot.clone();
            old_snapshot["retired_metadata"] = serde_json::Value::Null;
            db.conn
                .execute("INSERT INTO participant_sessions(id,provider,scope,session_id,participant_name,incarnation,delivered_through,snapshot) VALUES(2,'codex','scope','other-session','other','other-incarnation',17,?1)", [old_snapshot.to_string()])
                .unwrap();
            let lifecycle = serde_json::json!({"action":"stopped","retired_metadata":"old","snapshot":old_snapshot});
            db.log_event("life", "luna", &lifecycle).unwrap();
            let event_id = db.get_last_event_id();
            let event_timestamp: String = db.conn.query_row("SELECT timestamp FROM events WHERE id=?1", [event_id], |row| row.get(0)).unwrap();
            let parent = db.get_instance("luna").unwrap();
            let child = db.get_instance("child").unwrap();
            let rowid: i64 = db.conn.query_row("SELECT rowid FROM instances WHERE name='luna'", [], |row| row.get(0)).unwrap();
            let inode = get_inode(&path);
            db.conn.execute_batch(&format!("PRAGMA user_version = {version}")).unwrap();

            db.ensure_schema().unwrap();

            assert_eq!(get_inode(&path), inode);
            assert_eq!(db.get_instance("luna").unwrap(), parent);
            assert_eq!(db.get_instance("child").unwrap(), child);
            assert_eq!(db.get_instance_full("luna").unwrap().unwrap().created_at.to_bits(), created_at.to_bits());
            assert_eq!(db.conn.query_row("SELECT rowid FROM instances WHERE name='luna'", [], |row| row.get::<_, i64>(0)).unwrap(), rowid);
            assert_eq!(db.conn.query_row("SELECT running_tasks FROM instances WHERE name='luna'", [], |row| row.get::<_, String>(0)).unwrap(), "retained-tasks");
            assert_eq!(db.get_session_binding("root-session").unwrap().as_deref(), Some("luna"));
            assert_eq!(db.get_process_binding("process").unwrap().as_deref(), Some("luna"));
            assert!(db.has_notify_endpoint_kind("luna", "plugin"));
            assert_eq!(db.kv_get("service:database_generation").unwrap().as_deref(), Some("same-generation"));
            assert_eq!(db.get_last_event_id(), event_id);
            let (timestamp, raw): (String, String) = db.conn.query_row("SELECT timestamp,data FROM events WHERE id=?1", [event_id], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
            assert_eq!(timestamp, event_timestamp);
            let projected: serde_json::Value = serde_json::from_str(&raw).unwrap();
            assert_eq!(projected, serde_json::json!({"action":"stopped","snapshot":retained_snapshot}));
            assert_eq!(projected["snapshot"]["created_at"].as_f64().unwrap().to_bits(), created_at.to_bits());
            let raw: String = db.conn.query_row("SELECT snapshot FROM participant_sessions WHERE id=2", [], |row| row.get(0)).unwrap();
            assert_eq!(serde_json::from_str::<serde_json::Value>(&raw).unwrap(), retained_snapshot);
            let mailbox: (String, String, String, String, String, i64) = db
                .conn
                .query_row("SELECT s.snapshot,a.target,c.claim,c.messages,c.resolution,s.delivered_through FROM participant_sessions s JOIN participant_attachments a ON a.lineage=s.id JOIN inbox_claims c ON c.lineage=s.id", [], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
                })
                .unwrap();
            assert_eq!(mailbox, ("frozen-snapshot".into(), "frozen-target".into(), "frozen-claim".into(), "frozen-messages".into(), "open".into(), 17));
            assert!(!db.conn.prepare("PRAGMA foreign_key_check").unwrap().exists([]).unwrap());
            assert!(obsolete_instance_columns(&db.conn).unwrap().is_empty());
            assert_eq!(db.conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i32>(0)).unwrap(), SCHEMA_VERSION);
            assert!(!directory.path().join("archive").exists());
        }
    }

    #[test]
    fn instance_storage_upgrade_failure_rolls_back_without_archiving() {
        for repair_additive in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("agentx.db");
            let mut db = AgentxDb::open_at(&path).unwrap();
            db.conn
                .execute_batch(
                    "INSERT INTO instances(name,created_at) VALUES('luna',1);
             ALTER TABLE instances ADD COLUMN retired_first TEXT;
             ALTER TABLE instances ADD COLUMN retired_second TEXT;
             CREATE INDEX blocking_projection ON instances(retired_second);
             PRAGMA user_version = 26;",
                )
                .unwrap();
            if repair_additive {
                db.conn.execute_batch("ALTER TABLE instances DROP COLUMN launch_profile").unwrap();
            }
            let original = serde_json::json!({"action":"stopped","retired_first":"first","snapshot":{"name":"luna","created_at":1.0,"retired_second":"second"}}).to_string();
            db.conn.execute("INSERT INTO events(timestamp,type,instance,data) VALUES('unchanged','life','luna',?1)", [&original]).unwrap();
            db.conn.execute("INSERT INTO participant_sessions(id,provider,scope,session_id,participant_name,incarnation,delivered_through,snapshot) VALUES(1,'codex','scope','session','luna','exact',17,?1)", [&original]).unwrap();
            let error = db.ensure_schema().unwrap_err();
            assert!(error.to_string().contains("existing database preserved"));
            assert_eq!(db.conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i32>(0)).unwrap(), 26);
            assert_eq!(obsolete_instance_columns(&db.conn).unwrap(), ["retired_first", "retired_second"]);
            let has_profile: bool = db.conn.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('instances') WHERE name='launch_profile')", [], |row| row.get(0)).unwrap();
            assert_eq!(has_profile, !repair_additive);
            assert_eq!(db.conn.query_row("SELECT COUNT(*) FROM instances WHERE name='luna'", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
            assert_eq!(db.conn.query_row("SELECT data FROM events", [], |row| row.get::<_, String>(0)).unwrap(), original);
            assert_eq!(db.conn.query_row("SELECT snapshot FROM participant_sessions", [], |row| row.get::<_, String>(0)).unwrap(), original);
            assert!(!directory.path().join("archive").exists());
        }
    }

    /// Reproduce the home database: a v26 stamp with v21-v23 tables absent,
    /// including the interaction with a later missing column and v27 projection.
    #[test]
    fn schema_repair_missing_intermediate_tables_preserves_state() {
        for version in [26, SCHEMA_VERSION] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("agentx.db");
            let mut db = AgentxDb::open_at(&path).unwrap();
            let created = f64::from_bits(1_700_000_000.123_456_f64.to_bits() + 1);
            db.conn.execute("INSERT INTO instances(name,session_id,created_at) VALUES('luna','session',?1)", [created]).unwrap();
            db.kv_set("service:database_generation", Some("retained-generation")).unwrap();
            db.log_event("message", "luna", &serde_json::json!({"text":"retained"})).unwrap();
            let event = db.get_last_event_id();
            let inode = get_inode(&path);
            db.conn
                .execute_batch(&format!(
                    "DROP TABLE inbox_claims; DROP TABLE participant_attachments; DROP TABLE participant_sessions;
                 DROP TABLE message_acceptances; DROP TABLE collaboration_commands; DROP TABLE collaboration_work;
                 ALTER TABLE instances ADD COLUMN tag TEXT; ALTER TABLE instances DROP COLUMN launch_profile;
                 PRAGMA user_version={version};"
                ))
                .unwrap();

            db.ensure_schema().unwrap();

            assert_eq!(get_inode(&path), inode);
            assert_eq!(db.get_instance_full("luna").unwrap().unwrap().created_at.to_bits(), created.to_bits());
            assert_eq!(db.get_instance_full("luna").unwrap().unwrap().session_id.as_deref(), Some("session"));
            assert_eq!(db.kv_get("service:database_generation").unwrap().as_deref(), Some("retained-generation"));
            assert_eq!(db.get_last_event_id(), event);
            assert!(obsolete_instance_columns(&db.conn).unwrap().is_empty());
            assert!(matches!(db.check_schema_compat().unwrap(), SchemaCompat::Ok));
            assert!(!directory.path().join("archive").exists());
        }
    }

    #[test]
    fn schema_repair_each_additive_table_at_current_stamp() {
        for (_, table) in ADDITIVE_TABLES {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("agentx.db");
            let mut db = AgentxDb::open_at(&path).unwrap();
            db.conn.execute_batch(&format!("DROP TABLE {table}")).unwrap();
            db.kv_set("retained", Some(table)).unwrap();
            db.ensure_schema().unwrap();
            assert!(db.conn.prepare(&format!("SELECT * FROM {table}")).is_ok(), "{table}");
            assert!(matches!(db.check_schema_compat().unwrap(), SchemaCompat::Ok));
            assert_eq!(db.kv_get("retained").unwrap().as_deref(), Some(*table));
            assert!(!directory.path().join("archive").exists());
        }
    }

    #[test]
    fn schema_repair_table_creation_failure_preserves_database() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("agentx.db");
        let mut db = AgentxDb::open_at(&path).unwrap();
        db.conn
            .execute_batch(
                "DROP TABLE message_acceptances;
             CREATE VIEW message_acceptances AS SELECT 1;
             INSERT INTO instances(name,created_at) VALUES('luna',1);
             PRAGMA user_version=26;",
            )
            .unwrap();
        let inode = get_inode(&path);
        let error = db.ensure_schema().unwrap_err();
        let diagnostic = format!("{error:#}");
        assert!(diagnostic.contains("existing database preserved"), "{diagnostic}");
        assert!(diagnostic.contains("message_acceptances"), "{diagnostic}");
        assert_eq!(get_inode(&path), inode);
        assert_eq!(db.conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0)).unwrap(), 26);
        assert!(db.get_instance("luna").unwrap().is_some());
        assert!(!directory.path().join("archive").exists());
    }

    #[test]
    fn schema_repair_projection_failure_rolls_back_missing_tables() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("agentx.db");
        let mut db = AgentxDb::open_at(&path).unwrap();
        db.conn
            .execute_batch(
                "DROP TABLE message_acceptances;
             ALTER TABLE instances ADD COLUMN retired TEXT;
             CREATE INDEX blocking_projection ON instances(retired);
             PRAGMA user_version=26;",
            )
            .unwrap();
        let error = db.ensure_schema().unwrap_err();
        assert!(format!("{error:#}").contains("blocking_projection"));
        assert!(db.conn.prepare("SELECT * FROM message_acceptances").is_err());
        assert_eq!(obsolete_instance_columns(&db.conn).unwrap(), ["retired"]);
        assert_eq!(db.conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0)).unwrap(), 26);
        assert!(!directory.path().join("archive").exists());
    }

    #[test]
    fn schema_bootstrap_cannot_promote_or_downgrade_versioned_databases() {
        for version in [26, SCHEMA_VERSION + 1] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("agentx.db");
            let mut db = AgentxDb::open_at(&path).unwrap();
            db.conn.execute_batch(&format!("DROP TABLE message_acceptances; PRAGMA user_version={version}")).unwrap();
            assert!(db.init_db().unwrap_err().to_string().contains("use ensure_schema"));
            assert_eq!(db.conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0)).unwrap(), version);
            assert!(db.conn.prepare("SELECT * FROM message_acceptances").is_err());
            if version > SCHEMA_VERSION {
                db.ensure_schema().unwrap();
                assert!(db.conn.prepare("SELECT * FROM message_acceptances").is_err());
            }
        }
    }
}
