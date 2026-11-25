//! SQLite format admission for deployment-owned databases. The local host journal is separate.

use anyhow::{Result, ensure};
use rusqlite::Connection;

pub(super) const DEPLOYMENT_FORMAT: i64 = 2;
pub(super) const NAMESPACE_FORMAT: i64 = 5;

pub(super) fn version(db: &Connection) -> rusqlite::Result<i64> {
    db.pragma_query_value(None, "user_version", |row| row.get(0))
}

/// Run within a provisioning transaction. Only an empty database can acquire a version.
pub(super) fn provision(db: &Connection, expected: i64, label: &str, create: impl FnOnce() -> Result<()>) -> Result<()> {
    let found = version(db)?;
    if found != expected {
        let has_schema: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_schema)", [], |row| row.get(0))?;
        ensure!(found == 0 && !has_schema, "unsupported cloud {label} database format {found}; use a new deployment volume");
        create()?;
        db.pragma_update(None, "user_version", expected)?;
    }
    Ok(())
}
