//! One deployment owner serializes scope provisioning, credential changes and replay receipts.
use super::*;
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub(super) fn hash(value: &str) -> [u8; 32] {
    Sha256::digest(value.as_bytes()).into()
}

pub(super) fn matches(actual: &[u8], expected: &[u8]) -> bool {
    bool::from(actual.ct_eq(expected))
}

fn private_dir(path: &Path) -> anyhow::Result<()> {
    let mut dir = std::fs::DirBuilder::new();
    dir.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        dir.mode(0o700);
    }
    dir.create(path)?;
    Ok(())
}

struct ScopeSignals {
    space: tokio::sync::watch::Sender<changes::SpaceChanges>,
    version: tokio::sync::watch::Sender<u32>,
    control: tokio::sync::watch::Sender<()>,
}

struct DeploymentLease(std::fs::File);

impl Drop for DeploymentLease {
    fn drop(&mut self) {
        // A concurrent fork can briefly retain the open file description after close.
        // Release ownership explicitly so a successor can open the catalog immediately.
        let _ = self.0.unlock();
    }
}

pub(super) struct Catalog {
    db: Connection,
    pub(super) deployment_id: String,
    _lease: DeploymentLease,
    root: PathBuf,
    url: String,
    signals: std::collections::HashMap<String, ScopeSignals>,
}

impl Catalog {
    pub(super) fn open(root: &Path, url: String) -> anyhow::Result<Self> {
        private_dir(root)?;
        let root = root.canonicalize()?;
        let lease = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(root.join("deployment.lock"))?;
        lease.try_lock().map_err(|_| anyhow::anyhow!("cloud deployment directory is already owned"))?;
        let lease = DeploymentLease(lease);
        private_dir(&root.join("scopes"))?;
        let db = Connection::open(root.join("deployment.sqlite3"))?;
        db.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS deployment (singleton INTEGER PRIMARY KEY CHECK(singleton=1), id TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS scopes (id TEXT PRIMARY KEY, generation TEXT NOT NULL, token_hash BLOB UNIQUE NOT NULL, version INTEGER NOT NULL, created_at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS changes (request_id TEXT PRIMARY KEY, scope TEXT NOT NULL REFERENCES scopes(id), kind TEXT NOT NULL, input_hash BLOB NOT NULL, expected INTEGER, result_version INTEGER NOT NULL, result_url TEXT NOT NULL);",
        )?;
        db.execute("INSERT OR IGNORE INTO deployment VALUES(1,?1)", [uuid::Uuid::new_v4().to_string()])?;
        let deployment_id = db.query_row("SELECT id FROM deployment WHERE singleton=1", [], |row| row.get(0))?;
        let catalog = Self { db, deployment_id, _lease: lease, root, url, signals: Default::default() };
        catalog.upgrade_scope_message_history()?;
        Ok(catalog)
    }

    fn upgrade_scope_message_history(&self) -> anyhow::Result<()> {
        let mut stmt = self.db.prepare("SELECT id,generation FROM scopes ORDER BY id")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
        for row in rows {
            let (id, generation) = row?;
            // A detached or unavailable scope must not block unrelated scopes.
            if communication::store::upgrade_message_history(&self.root_for(&id), &generation).is_err() {
                eprintln!("Cloud scope {id}: message history indexes unavailable");
            }
        }
        Ok(())
    }

    fn receipt(&self, id: &str, generation: String, version: u32) -> ScopeReceipt {
        ScopeReceipt { id: id.into(), scope_url: format!("{}/scopes/{id}", self.url), credential_version: version, generation }
    }

    pub(super) fn lookup(&self, id: &str) -> Result<ScopeReceipt, Failure> {
        let row = self.db.query_row("SELECT generation,version FROM scopes WHERE id=?1", [id], |r| Ok((r.get(0)?, r.get(1)?))).optional()?.ok_or_else(Failure::missing)?;
        Ok(self.receipt(id, row.0, row.1))
    }

    pub(super) fn check_admin_token(&self, digest: &[u8]) -> anyhow::Result<()> {
        let reused: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM changes WHERE input_hash=?1)", [digest], |row| row.get(0))?;
        anyhow::ensure!(!reused, "deployment credential was previously issued as a scope token");
        Ok(())
    }

    pub(super) fn root_for(&self, id: &str) -> PathBuf {
        self.root.join("scopes").join(id)
    }

    pub(super) fn authenticate(&mut self, id: &str, bearer: &str) -> Result<ScopeContext, Failure> {
        let row = self.db.query_row("SELECT generation,version,token_hash FROM scopes WHERE id=?1", [id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?, r.get::<_, Vec<u8>>(2)?))).optional()?;
        let (generation, credential_version, expected) = row.ok_or_else(Failure::unauthorized)?;
        if !matches(&hash(bearer), &expected) {
            return Err(Failure::unauthorized());
        }
        Ok(self.context(id, generation, credential_version))
    }

    pub(super) fn context(&mut self, id: &str, generation: String, credential_version: u32) -> ScopeContext {
        let root = self.root_for(id);
        let signals =
            self.signals.entry(id.into()).or_insert_with(|| ScopeSignals { space: tokio::sync::watch::channel(changes::SpaceChanges::new()).0, version: tokio::sync::watch::channel(credential_version).0, control: tokio::sync::watch::channel(()).0 });
        let (current, control_changed) = (signals.version.clone(), signals.control.clone());
        ScopeContext { id: id.into(), root, generation, credential_version, current, control_changed, space_changed: signals.space.clone() }
    }

    pub(super) fn list(&self, after: &str, limit: i64) -> Result<Vec<ScopeReceipt>, Failure> {
        let mut stmt = self.db.prepare("SELECT id,generation,version FROM scopes WHERE id>?1 ORDER BY id LIMIT ?2")?;
        Ok(stmt.query_map(params![after, limit], |r| Ok(self.receipt(&r.get::<_, String>(0)?, r.get(1)?, r.get(2)?)))?.collect::<rusqlite::Result<_>>()?)
    }

    pub(super) fn change(&mut self, id: &str, request: &str, token: &str, expected: Option<u32>) -> Result<ScopeReceipt, Failure> {
        validate_id(id)?;
        if request.is_empty() || request.len() > 128 || !request.bytes().all(|c| c.is_ascii_graphic()) {
            return Err(Failure::invalid("invalid request_id"));
        }
        validate_token(token)?;
        let digest = hash(token);
        let kind = if expected.is_some() { "rotate" } else { "create" };
        let root = self.root_for(id);
        let tx = self.db.transaction()?;
        let replay = tx
            .query_row("SELECT scope,kind,input_hash,expected,result_version,result_url FROM changes WHERE request_id=?1", [request], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Vec<u8>>(2)?, r.get::<_, Option<u32>>(3)?, r.get::<_, u32>(4)?, r.get::<_, String>(5)?))
            })
            .optional()?;
        if let Some((scope, action, input, previous, version, scope_url)) = replay {
            if scope != id || kind != action || previous != expected || !matches(&input, &digest) {
                return Err(Failure::conflict("request_id already identifies a different change"));
            }
            let generation = tx.query_row("SELECT generation FROM scopes WHERE id=?1", [id], |r| r.get(0))?;
            drop(tx);
            return Ok(ScopeReceipt { id: id.into(), scope_url, generation, credential_version: version });
        }
        let existing = tx.query_row("SELECT generation,version FROM scopes WHERE id=?1", [id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))).optional()?;
        if tx.query_row("SELECT EXISTS(SELECT 1 FROM scopes WHERE token_hash=?1)", [digest.as_slice()], |r| r.get::<_, bool>(0))? {
            return Err(Failure::conflict("scope token already in use"));
        }
        let (generation, version) = match (existing, expected) {
            (None, None) => {
                // A crash after directory creation resumes against the same unpublished generation.
                private_dir(&root).map_err(|_| Failure::internal())?;
                let gen_file = root.join("generation");
                let generation = match std::fs::read_to_string(&gen_file) {
                    Ok(s) => uuid::Uuid::parse_str(&s).map_err(|_| Failure::internal())?.to_string(),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        let s = uuid::Uuid::new_v4().to_string();
                        let mut file = tempfile::NamedTempFile::new_in(&root).map_err(|_| Failure::internal())?;
                        use std::io::Write;
                        file.write_all(s.as_bytes()).and_then(|()| file.as_file().sync_all()).map_err(|_| Failure::internal())?;
                        file.persist(&gen_file).map_err(|_| Failure::internal())?;
                        s
                    }
                    Err(_) => return Err(Failure::internal()),
                };
                communication::initialize(&root, &generation).map_err(|_| Failure::internal())?;
                tx.execute("INSERT INTO scopes VALUES (?1,?2,?3,1,unixepoch())", params![id, generation, digest.as_slice()])?;
                (generation, 1)
            }
            (Some((generation, v)), Some(e)) if e == v => {
                let version = v.checked_add(1).ok_or_else(Failure::internal)?;
                tx.execute("UPDATE scopes SET version=?2,token_hash=?3 WHERE id=?1", params![id, version, digest.as_slice()])?;
                (generation, version)
            }
            (None, Some(_)) => return Err(Failure::missing()),
            _ => return Err(Failure::conflict("scope already exists or credential version changed")),
        };
        tx.execute("INSERT INTO changes VALUES (?1,?2,?3,?4,?5,?6,?7)", params![request, id, kind, digest.as_slice(), expected, version, format!("{}/scopes/{id}", self.url)])?;
        tx.commit()?;
        if let Some(active) = self.signals.get(id) {
            active.version.send_replace(version);
        }
        Ok(self.receipt(id, generation, version))
    }
    pub(super) fn created_at(&self, id: &str) -> Result<i64, Failure> {
        Ok(self.db.query_row("SELECT created_at FROM scopes WHERE id=?1", [id], |r| r.get(0))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn fork_inherited_descriptor_does_not_extend_deployment_ownership() {
        let directory = tempfile::tempdir().unwrap();
        let open = || Catalog::open(directory.path(), "https://cloud.example".into());
        let owner = open().unwrap();
        // dup and fork share the same underlying open file description on Unix.
        let inherited = owner._lease.0.try_clone().unwrap();
        assert!(open().is_err());
        drop(owner);
        let successor = open().unwrap();
        drop(inherited);
        assert!(open().is_err());
        drop(successor);
    }

    #[test]
    fn interrupted_provisioning_and_origin_change_keep_one_scope_and_original_receipt() {
        let directory = tempfile::tempdir().unwrap();
        let mut db = Catalog::open(directory.path(), "https://first.example".into()).unwrap();
        let deployment_id = db.deployment_id.clone();
        let root = db.root_for("personal");
        // Force initialization to fail after the provisional generation has been written.
        std::fs::create_dir(&root).unwrap();
        let blocker = root.join("communication.sqlite3");
        std::fs::create_dir(&blocker).unwrap();
        let token = "A".repeat(32);
        assert!(db.change("personal", "first", &token, None).is_err());
        assert!(db.list("", 10).unwrap().is_empty());
        let provisional = std::fs::read_to_string(root.join("generation")).unwrap();
        std::fs::remove_dir(&blocker).unwrap();
        let committed = db.change("personal", "first", &token, None).unwrap();
        assert_eq!(committed.generation, provisional);
        let context = db.authenticate("personal", &token).unwrap();
        let successor = "B".repeat(32);
        assert_eq!(db.change("personal", "second", &successor, Some(1)).unwrap().credential_version, 2);
        assert!(!context.valid());
        assert!(db.authenticate("personal", &token).is_err());
        assert!(db.check_admin_token(&hash(&token)).is_err());
        assert!(Catalog::open(directory.path(), "https://second.example".into()).is_err());
        drop(db);
        let mut reopened = Catalog::open(directory.path(), "https://second.example".into()).unwrap();
        assert_eq!(deployment_id, reopened.deployment_id);
        let replay = reopened.change("personal", "first", &token, None).unwrap();
        assert_eq!(replay.scope_url, committed.scope_url);
        assert_eq!(replay.credential_version, 1);
        let current = reopened.lookup("personal").unwrap();
        assert_eq!(current.credential_version, 2);
        assert_eq!(current.scope_url, "https://second.example/scopes/personal");
    }
}
