//! Canonical data-scope ownership. The lease must outlive application/database
//! destruction; the lock file is retained across clean shutdown and reset.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[cfg(unix)]
use sha2::{Digest, Sha256};

use super::DaemonError;

/// Canonical singleton key resolved from a client's explicit invocation scope.
/// Preparing it creates only directories, never a database or application.
#[derive(Debug, Clone)]
pub struct DaemonScope {
    root: PathBuf,
    state: PathBuf,
}

impl DaemonScope {
    /// Resolve an already selected scope without creating or repairing it.
    /// The caller supplies its originating cwd rather than the daemon's cwd.
    pub(crate) fn existing(path: &Path, cwd: &Path) -> std::io::Result<Self> {
        let absolute = if path.is_absolute() { path.to_path_buf() } else { cwd.join(path) };
        if !absolute.is_absolute() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "scope and originating cwd must resolve to an absolute path"));
        }
        let root = absolute.canonicalize()?;
        validate_existing_directory(&root)?;
        Ok(Self { state: root.join(".daemon"), root })
    }

    pub fn prepare(path: impl AsRef<Path>) -> Result<Self, DaemonError> {
        let path = path.as_ref();
        let absolute = if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir()?.join(path) };
        fs::create_dir_all(&absolute)?;
        let root = absolute.canonicalize()?;
        private_directory(&root)?;
        let state = root.join(".daemon");
        private_directory(&state)?;
        Ok(Self { root, state })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn state(&self) -> &Path {
        &self.state
    }

    /// The same scope maps to the same short socket path regardless of cwd,
    /// home, TMPDIR or the environment of the process that first started it.
    #[cfg(unix)]
    pub(crate) fn socket_path(&self) -> Result<PathBuf, DaemonError> {
        self.unix_endpoint(".sock", true)
    }

    #[cfg(unix)]
    pub(crate) fn existing_socket_path(&self) -> Result<PathBuf, DaemonError> {
        self.unix_endpoint(".sock", false)
    }

    /// Observation discovery never repairs the per-user endpoint directory.
    #[cfg(unix)]
    pub(crate) fn observation_socket_path(&self, create: bool) -> Result<PathBuf, DaemonError> {
        self.unix_endpoint(".observe.sock", create)
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn cloud_socket_path(&self, create: bool) -> Result<PathBuf, DaemonError> {
        self.unix_endpoint(".cloud.sock", create)
    }

    #[cfg(unix)]
    fn unix_endpoint(&self, suffix: &str, create: bool) -> Result<PathBuf, DaemonError> {
        use std::os::unix::ffi::OsStrExt;
        // SAFETY: geteuid has no preconditions and returns this process's UID.
        let uid = unsafe { libc::geteuid() };
        let directory = PathBuf::from(format!("/tmp/agentx-{uid}"));
        if create {
            private_directory(&directory)?;
        } else {
            validate_existing_directory(&directory)?;
        }
        let digest = Sha256::digest(self.root.as_os_str().as_bytes());
        Ok(directory.join(format!("{}{suffix}", super::hex_digest(&digest))))
    }
}

/// Passive validation is deliberately separate from private_directory, whose
/// create/chmod/DACL repair effects are authorized only during acquisition.
fn validate_existing_directory(path: &Path) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotADirectory, "scope must be an existing directory"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // SAFETY: geteuid has no preconditions.
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "scope directory is not private to this user"));
        }
    }
    #[cfg(windows)]
    super::windows::validate_existing_directory(path)?;
    Ok(())
}

/// Published mirror of the database-owned durable generation. Application
/// construction and maintenance publish only after reading/committing the
/// native persistence key; no independent lifecycle file stores this value.
#[derive(Clone, Default)]
pub struct DatabaseGeneration {
    value: Arc<Mutex<String>>,
    observation: Arc<Mutex<Option<std::sync::Weak<super::observation::ObservationState>>>>,
}

impl std::fmt::Debug for DatabaseGeneration {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("DatabaseGeneration").field("current", &self.current()).finish_non_exhaustive()
    }
}

impl DatabaseGeneration {
    pub(crate) fn bind_observation(&self, observation: &Arc<super::observation::ObservationState>) {
        if let Ok(mut target) = self.observation.lock() {
            *target = Some(Arc::downgrade(observation));
        }
    }

    fn withdraw_observation(&self) -> Option<Arc<super::observation::ObservationState>> {
        let target = self.observation.lock().ok().and_then(|target| target.as_ref().and_then(std::sync::Weak::upgrade));
        if let Some(target) = &target {
            target.withdraw_epoch();
        }
        target
    }

    pub fn current(&self) -> String {
        self.value.lock().map(|value| value.clone()).unwrap_or_default()
    }

    /// Fail closed after a committed maintenance change cannot publish its
    /// generation. The dispatcher stops admission as soon as this is empty.
    pub fn invalidate(&self) {
        let observation = self.withdraw_observation();
        self.value.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clear();
        if let Some(observation) = observation {
            observation.generation_committed();
        }
    }

    /// Publish a generation read from durable state. Call before readiness and
    /// after a reset commits, on the same application dispatch thread.
    pub fn publish(&self, committed_generation: String) -> Result<(), DaemonError> {
        if uuid::Uuid::parse_str(&committed_generation).is_err() {
            self.invalidate();
            return Err(DaemonError::Protocol("database generation must be a committed UUID".into()));
        }
        // Application calls this only after durable read/commit, outside its
        // write scope. Withdraw volatile dataset availability before changing
        // the mirror; telemetry cannot invent or veto the committed generation.
        let observation = self.withdraw_observation();
        let mut value = self.value.lock().map_err(|poisoned| {
            poisoned.into_inner().clear();
            DaemonError::Unavailable("database generation state is poisoned".into())
        })?;
        *value = committed_generation;
        drop(value);
        if let Some(observation) = observation {
            observation.generation_committed();
        }
        Ok(())
    }
}

/// OS-held scope ownership. Construct the application after acquisition and
/// destroy it before dropping this value. Never delete `owner.lock` on exit.
pub struct ScopeLease {
    scope: DaemonScope,
    lock: File,
    instance: String,
    generation: DatabaseGeneration,
}

impl ScopeLease {
    pub fn acquire(scope: DaemonScope) -> Result<Self, DaemonError> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE};
            // Refuse unlink/rename while this lifetime lock is held.
            options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
        }
        let lock = options.open(scope.state.join("owner.lock"))?;
        if !lock.metadata()?.is_file() {
            return Err(DaemonError::Unavailable("scope lock is not a regular file".into()));
        }
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(DaemonError::AlreadyRunning),
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
        let lease = Self { scope, lock, instance: uuid::Uuid::new_v4().to_string(), generation: DatabaseGeneration::default() };
        lease.ensure_ownership()?;
        Ok(lease)
    }

    pub fn scope(&self) -> &DaemonScope {
        &self.scope
    }

    pub fn instance(&self) -> &str {
        &self.instance
    }

    pub fn database_generation(&self) -> DatabaseGeneration {
        self.generation.clone()
    }

    /// Detect accidental external replacement of the inode that carries the
    /// singleton lock; managed reset must preserve the ownership directory.
    pub(crate) fn ensure_ownership(&self) -> Result<(), DaemonError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let held = self.lock.metadata()?;
            let current = fs::symlink_metadata(self.scope.state.join("owner.lock"))?;
            if !current.is_file() || held.dev() != current.dev() || held.ino() != current.ino() {
                return Err(DaemonError::Unavailable("scope ownership file was replaced; daemon must stop".into()));
            }
        }
        Ok(())
    }
}

fn private_directory(path: &Path) -> Result<(), DaemonError> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(DaemonError::Unavailable(format!("{} is not a private directory", path.display())));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // SAFETY: geteuid has no preconditions.
        if metadata.uid() != unsafe { libc::geteuid() } {
            return Err(DaemonError::Unavailable(format!("{} belongs to another user", path.display())));
        }
    }
    #[cfg(windows)]
    super::windows::private_directory(path)?;
    crate::sys::fs::set_private_dir(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passive_missing_scope_does_not_create_directories() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("absent").join("scope");
        let error = DaemonScope::existing(&missing, directory.path()).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert!(!directory.path().join("absent").exists());
    }

    #[cfg(unix)]
    #[test]
    fn passive_aliases_share_endpoint_without_repairing_scope() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("scope");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let alias = directory.path().join("alias");
        symlink(&root, &alias).unwrap();
        let first = DaemonScope::existing(Path::new("scope"), directory.path()).unwrap();
        let second = DaemonScope::existing(&alias, directory.path()).unwrap();
        assert_eq!(first.root(), second.root());
        assert!(!root.join(".daemon").exists());
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(DaemonScope::existing(&root, directory.path()).unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(fs::metadata(&root).unwrap().permissions().mode() & 0o777, 0o755);
    }

    #[test]
    fn singleton_is_held_until_lease_drop() {
        let directory = tempfile::tempdir().unwrap();
        let scope = DaemonScope::prepare(directory.path().join("scope")).unwrap();
        let lease = ScopeLease::acquire(scope.clone()).unwrap();
        assert!(matches!(ScopeLease::acquire(scope.clone()), Err(DaemonError::AlreadyRunning)));
        drop(lease);
        assert!(ScopeLease::acquire(scope).is_ok());
    }

    #[test]
    fn invalid_publication_clears_previous_generation() {
        let generation = DatabaseGeneration::default();
        generation.publish(uuid::Uuid::new_v4().to_string()).unwrap();
        assert!(generation.publish("not-a-committed-uuid".into()).is_err());
        assert!(generation.current().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn replaced_lock_inode_stops_admission() {
        let directory = tempfile::tempdir().unwrap();
        let scope = DaemonScope::prepare(directory.path().join("scope")).unwrap();
        let lease = ScopeLease::acquire(scope.clone()).unwrap();
        fs::rename(scope.state.join("owner.lock"), scope.state.join("old.lock")).unwrap();
        fs::write(scope.state.join("owner.lock"), b"").unwrap();
        assert!(lease.ensure_ownership().is_err());
    }
}
