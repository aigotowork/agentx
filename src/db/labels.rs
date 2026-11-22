//! Normalized creation-owned labels and exact current-alias SQL support.
pub(super) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS agent_label_sets (
 owner_creation TEXT NOT NULL COLLATE BINARY PRIMARY KEY REFERENCES agent_creations(id) ON DELETE CASCADE,
 revision INTEGER NOT NULL DEFAULT 0 CHECK(typeof(revision)='integer' AND revision>=0)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS agent_labels (
 owner_creation TEXT NOT NULL COLLATE BINARY REFERENCES agent_label_sets(owner_creation) ON DELETE CASCADE,
 label_key TEXT NOT NULL COLLATE BINARY CHECK(typeof(label_key)='text' AND length(CAST(label_key AS BLOB)) BETWEEN 1 AND 127),
 label_value TEXT NOT NULL COLLATE BINARY CHECK(typeof(label_value)='text' AND length(CAST(label_value AS BLOB)) BETWEEN 1 AND 256),
 PRIMARY KEY(owner_creation,label_key)
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS agent_labels_by_key_value_owner ON agent_labels(label_key,label_value,owner_creation);
CREATE INDEX IF NOT EXISTS agent_creation_instances_by_creation ON agent_creation_instances(creation,name,incarnation);
CREATE INDEX IF NOT EXISTS instances_by_adopted_workspace_name ON instances(
 CASE WHEN json_valid(launch_context) THEN json_extract(launch_context,'$.collaboration.workspace') END,name
) WHERE COALESCE(origin_device_id,'')='';
";

use crate::agents::labels::{LabelPredicate, LabelQuery};
use rusqlite::{Connection, types::Value};

/// The decimal representation here is of the integer bits, never of the float.
/// Register once per connection; scope/identity predicates use this before LIMIT.
pub(super) fn register_functions(conn: &Connection) -> rusqlite::Result<()> {
    use rusqlite::functions::FunctionFlags;
    conn.create_scalar_function("agentx_incarnation", 1, FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_INNOCUOUS, |context| Ok(context.get::<f64>(0)?.to_bits().to_string()))
}

/// The saved workspace is a string or unassigned, independent of launch cwd.
pub(crate) const WORKSPACE_MEMBER_SQL: &str = "CASE WHEN json_valid(i.launch_context) THEN CASE WHEN json_type(i.launch_context,'$.collaboration.workspace')='text' THEN json_extract(i.launch_context,'$.collaboration.workspace') END END";
// SQLite expression indexes match SQL expressions syntactically. Retain the
// original seek expression and apply the stricter type check as a residual.
const INDEXED_WORKSPACE_SQL: &str = "CASE WHEN json_valid(i.launch_context) THEN json_extract(i.launch_context,'$.collaboration.workspace') END";

/// Both SQL paths keep one row per current exact incarnation. Positive seed
/// joins let SQLite choose scope-first or the covering reverse index from its
/// actual statistics. Negative predicates remain owner/key anti-probes.
pub(crate) fn selection_sql(query: &LabelQuery, after: &str) -> (String, Vec<Value>) {
    selection_sql_with_columns(query, after, "i.*")
}

fn selection_sql_with_columns(query: &LabelQuery, after: &str, columns: &str) -> (String, Vec<Value>) {
    let mut sql = format!("SELECT {columns} FROM instances i LEFT JOIN agent_creation_instances a ON a.name=i.name AND a.incarnation=agentx_incarnation(i.created_at)");
    let mut parameters = Vec::new();
    let seed = query.selectors.iter().find(|p| matches!(p, LabelPredicate::Eq { .. })).or_else(|| query.selectors.iter().find(|p| matches!(p, LabelPredicate::Exists { .. })));
    if let Some(seed) = seed {
        sql = sql.replacen("LEFT JOIN agent_creation_instances", "JOIN agent_creation_instances", 1);
        sql.push_str(" JOIN agent_labels seed ON seed.owner_creation=a.creation AND seed.label_key=?");
        parameters.push(Value::Text(seed.key().into()));
        if let LabelPredicate::Eq { value, .. } = seed {
            sql.push_str(" AND seed.label_value=?");
            parameters.push(Value::Text(value.clone()));
        }
    }
    sql.push_str(" WHERE COALESCE(i.origin_device_id,'')='' AND i.name>?");
    parameters.push(Value::Text(after.into()));
    if let Some(workspace) = &query.workspace {
        // Legacy/invalid collaboration snapshots are unassigned, matching the
        // native adopted-membership projection rather than process cwd.
        let member = WORKSPACE_MEMBER_SQL;
        if workspace == "global" {
            sql.push_str(&format!(" AND {member} IS NULL"));
        } else {
            sql.push_str(&format!(" AND (({INDEXED_WORKSPACE_SQL}=? AND {member} IS NOT NULL)"));
            parameters.push(Value::Text(workspace.clone()));
            if query.include_global {
                sql.push_str(&format!(" OR {member} IS NULL"));
            }
            sql.push(')');
        }
    }
    if let Some(directory) = &query.directory {
        sql.push_str(" AND i.directory=?");
        parameters.push(Value::Text(directory.clone()));
    }
    for predicate in &query.selectors {
        if Some(predicate) == seed {
            continue;
        }
        let negative = matches!(predicate, LabelPredicate::Missing { .. } | LabelPredicate::NotEq { .. });
        sql.push_str(if negative { " AND NOT EXISTS (" } else { " AND EXISTS (" });
        sql.push_str("SELECT 1 FROM agent_labels p WHERE p.owner_creation=a.creation AND p.label_key=?");
        parameters.push(Value::Text(predicate.key().into()));
        if let LabelPredicate::Eq { value, .. } | LabelPredicate::NotEq { value, .. } = predicate {
            sql.push_str(" AND p.label_value=?");
            parameters.push(Value::Text(value.clone()));
        }
        sql.push(')');
    }
    sql.push_str(" ORDER BY i.name LIMIT ?");
    parameters.push(Value::Integer(i64::from(query.limit) + 1));
    (sql, parameters)
}

impl super::AgentxDb {
    pub(crate) fn select_labeled_names(&self, query: &LabelQuery, after: &str) -> rusqlite::Result<Vec<String>> {
        let (sql, parameters) = selection_sql_with_columns(query, after, "i.name");
        let mut statement = self.conn().prepare_cached(&sql)?;
        statement.query_map(rusqlite::params_from_iter(parameters), |row| row.get(0))?.collect()
    }

    pub(crate) fn select_labeled_instances(&self, query: &LabelQuery, after: &str) -> rusqlite::Result<Vec<super::InstanceRow>> {
        let (sql, parameters) = selection_sql(query, after);
        let mut statement = self.conn().prepare(&sql)?;
        statement.query_map(rusqlite::params_from_iter(parameters), super::InstanceRow::from_row)?.collect()
    }
}

#[cfg(test)]
mod tests {

    use crate::db::AgentxDb;
    fn downgrade(db: &AgentxDb) {
        db.conn().execute_batch("DROP TABLE agent_labels; DROP TABLE agent_label_sets; DROP INDEX agent_creation_instances_by_creation; DROP INDEX instances_by_adopted_workspace_name; PRAGMA user_version=27;").unwrap();
    }
    #[test]
    fn labels_migration_retains_prior_data_and_reset_constraints() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("agentx.db");
        let mut db = AgentxDb::open_at(&path).unwrap();
        downgrade(&db);
        db.conn().execute_batch("INSERT INTO instances(name,created_at,session_id) VALUES('luna',42.25,'session'); INSERT INTO agent_creations(id,name,incarnation,origin) VALUES('owner','luna','bits','unknown'); INSERT INTO agent_creation_instances(name,incarnation,creation) VALUES('luna','bits','owner'); INSERT INTO kv VALUES('retained','yes'); INSERT INTO events(timestamp,type,instance,data) VALUES('before','life','luna','{}');").unwrap();
        db.ensure_schema().unwrap();
        assert_eq!(db.conn().query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0)).unwrap(), crate::db::SCHEMA_VERSION);
        assert_eq!(db.kv_get("retained").unwrap().as_deref(), Some("yes"));
        assert_eq!(db.conn().query_row("SELECT session_id FROM instances WHERE name='luna'", [], |r| r.get::<_, String>(0)).unwrap(), "session");
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM events WHERE timestamp='before'", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM agent_label_sets", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
        assert!(!directory.path().join("archive").exists());
        db.conn().execute("INSERT INTO agent_label_sets(owner_creation,revision) VALUES('owner',1)", []).unwrap();
        db.conn().execute("INSERT INTO agent_labels VALUES('owner','project','atlas')", []).unwrap();
        assert!(db.conn().execute("INSERT INTO agent_labels VALUES('owner','project','duplicate')", []).is_err());
        assert!(db.conn().execute("INSERT INTO agent_labels VALUES('owner','empty','')", []).is_err());
        assert!(db.conn().execute("INSERT INTO agent_labels VALUES('missing','project','atlas')", []).is_err());
        db.conn().execute("DELETE FROM agent_creations WHERE id='owner'", []).unwrap();
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM agent_labels", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    }
    #[test]
    fn labels_migration_failure_rolls_back_tables_version_and_preserves_data() {
        let directory = tempfile::tempdir().unwrap();
        let mut db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
        downgrade(&db);
        db.conn().execute_batch("INSERT INTO instances(name,created_at) VALUES('luna',42.25); INSERT INTO kv VALUES('retained','yes'); CREATE VIEW agent_labels AS SELECT 1 AS blocked;").unwrap();
        let error = db.ensure_schema().unwrap_err();
        assert!(error.to_string().contains("existing database preserved"));
        assert_eq!(db.conn().query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0)).unwrap(), 27);
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='agent_label_sets'", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
        assert_eq!(db.kv_get("retained").unwrap().as_deref(), Some("yes"));
        assert!(db.get_instance_full("luna").unwrap().is_some());
        assert!(!directory.path().join("archive").exists());
    }
}
