//! Centralized path resolution and file utilities for agentx.
//!
//! Single source of truth for all agentx directory and file paths.
//! Respects AGENTX_DIR env var for worktrees/dev, falls back to ~/.agentx.
//! Also provides atomic file operations and flag counters.

use crate::config::Config;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const LOGS_DIR: &str = ".tmp/logs";
pub const LAUNCH_DIR: &str = ".tmp/launch";
pub const FLAGS_DIR: &str = ".tmp/flags";
#[cfg(test)]
pub const LAUNCHES_DIR: &str = "launches";
pub const ARCHIVE_DIR: &str = "archive";
pub const SCRIPTS_DIR: &str = "scripts";

/// Resolve AGENTX_DIR from an environment snapshot.
///
/// Returns the normalized path plus whether AGENTX_DIR was explicitly set.
/// Normalization behavior:
/// - `~` expands against HOME/USERPROFILE when available
/// - relative paths are resolved against the provided cwd
/// - otherwise falls back to `HOME/.agentx` or `.agentx`
pub fn resolve_agentx_dir_from_env(env: &HashMap<String, String>, cwd: &Path) -> (PathBuf, bool) {
    let home = env.get("HOME").or_else(|| env.get("USERPROFILE"));
    let agentx_dir = env.get("AGENTX_DIR").filter(|value| !value.is_empty());

    let resolved = if let Some(dir) = agentx_dir {
        let expanded = if dir.starts_with('~') { if let Some(home_dir) = home { dir.replacen('~', home_dir, 1) } else { dir.clone() } } else { dir.clone() };

        let path = PathBuf::from(expanded);
        if path.is_relative() { cwd.join(path) } else { path }
    } else {
        home.map(|home_dir| PathBuf::from(home_dir).join(".agentx")).unwrap_or_else(|| PathBuf::from(".agentx"))
    };

    (resolved, agentx_dir.is_some())
}

/// Canonicalize a path through its deepest existing ancestor.
///
/// The existing prefix is resolved with `canonicalize` (following symlinks);
/// the not-yet-existing suffix is appended verbatim. A `..` component in that
/// suffix cannot be resolved on the filesystem here, so it is rejected outright
/// rather than folded lexically — otherwise `<tmp>/nope/../../etc` would
/// spuriously appear to sit under the temp prefix.
pub(crate) fn resolve_deepest_existing(path: &Path) -> Option<PathBuf> {
    let mut current = path;
    loop {
        if let Ok(existing) = current.canonicalize() {
            let rest = path.strip_prefix(current).ok()?;
            if rest.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
                return None;
            }
            return Some(existing.join(rest));
        }
        current = current.parent()?;
    }
}

/// Whether a unit-test path resolves beneath the system temporary directory.
///
/// Both sides are canonicalized through their deepest existing ancestor, so the
/// decision is safe for paths whose final components do not exist yet and
/// rejects a lexical temp path that crosses a symlink to a non-temp target.
#[cfg(test)]
pub(crate) fn is_test_temp_path(path: &Path) -> bool {
    let Some(temp_dir) = resolve_deepest_existing(&std::env::temp_dir()) else {
        return false;
    };
    let Some(resolved) = resolve_deepest_existing(path) else {
        return false;
    };
    resolved.starts_with(temp_dir)
}

/// Registry of test roots a fixture has explicitly claimed as disposable.
///
/// Temp-directory *geography* is not ownership: a real agentx DB can legitimately
/// live under `$TMPDIR` (and `TMPDIR=/` would trust almost everything). So the
/// Config redirect (see `config`) trusts only roots a test fixture registered
/// here, never "it's under /tmp". `open_raw`'s tripwire additionally accepts the
/// temp tree as a disposable backstop for ad-hoc `tempfile` DBs, and registers
/// what it opens so a later Config lookup on the same root stays consistent.
#[cfg(test)]
pub(crate) mod test_roots {
    use super::{Path, PathBuf, resolve_deepest_existing};
    use std::sync::{Mutex, OnceLock};

    fn roots() -> &'static Mutex<Vec<PathBuf>> {
        static ROOTS: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();
        ROOTS.get_or_init(|| Mutex::new(Vec::new()))
    }

    /// Run one exact path-safety test with owned disjoint temp/outside roots.
    /// Only the child receives a different temp environment; its registry starts
    /// empty. Neither branch derives a write target from source or user scope.
    pub(crate) fn with_temp_boundary(test: &str, check: impl FnOnce(&Path, &Path)) {
        const CHILD: &str = "AGENTX_PATH_BOUNDARY_TEST";
        const ROOT: &str = "AGENTX_PATH_BOUNDARY_ROOT";
        if std::env::var(CHILD).as_deref() == Ok(test) {
            let root = PathBuf::from(std::env::var_os(ROOT).expect("owned boundary root"));
            let inside = root.join("temp");
            let outside = root.join("outside");
            assert_eq!(std::env::temp_dir().canonicalize().unwrap(), inside.canonicalize().unwrap());
            assert!(super::is_test_temp_path(&inside.join("probe.db")));
            assert!(!super::is_test_temp_path(&outside.join("probe.db")));
            assert!(!is_registered(&outside));
            check(&inside, &outside);
            std::fs::write(root.join("checked"), test).unwrap();
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let inside = root.path().join("temp");
        std::fs::create_dir(&inside).unwrap();
        std::fs::create_dir(root.path().join("outside")).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture", "--test-threads=1"])
            .env(CHILD, test)
            .env(ROOT, root.path())
            .env("TMPDIR", &inside)
            .env("TMP", &inside)
            .env("TEMP", &inside)
            .stdin(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "isolated boundary test {test} failed: {status}");
                assert_eq!(std::fs::read_to_string(root.path().join("checked")).unwrap(), test, "exact child case must have executed");
                break;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("isolated boundary test {test} exceeded ten seconds");
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// Claim `path` (canonicalized through its deepest existing ancestor) as a
    /// disposable test root. Idempotent.
    pub(crate) fn register(path: &Path) {
        if let Some(canonical) = resolve_deepest_existing(path) {
            let mut roots = roots().lock().unwrap();
            if !roots.contains(&canonical) {
                roots.push(canonical);
            }
        }
    }

    /// Whether `path` resolves at or beneath a registered disposable root.
    pub(crate) fn is_registered(path: &Path) -> bool {
        let Some(canonical) = resolve_deepest_existing(path) else {
            return false;
        };
        roots().lock().unwrap().iter().any(|root| canonical.starts_with(root))
    }
}

/// Directory components that some AI tools (codex, claude, gemini) treat as
/// protected metadata under any writable root. Placing AGENTX_DIR beneath one of
/// these means the parent tool's sandbox/permission layer will block writes to
/// the agentx DB, with no escalation path for codex apply_patch.
///
/// - `.git`: codex (apply_patch hard-deny via FileSystemSandboxPolicy), claude
///   (DANGEROUS_DIRECTORIES auto-edit gate), gemini (GOVERNANCE_FILES).
/// - `.codex`, `.agents`: codex protected metadata.
/// - `.claude`: claude DANGEROUS_DIRECTORIES.
const PROTECTED_AGENTX_DIR_COMPONENTS: &[&str] = &[".git", ".codex", ".claude", ".agents", ".omp"];

/// If `path` sits at or beneath a protected metadata component, return that
/// component name. Component-wise match — `.gitfoo` and `dot.git` do not trigger.
pub fn protected_agentx_dir_component(path: &Path) -> Option<&'static str> {
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            for protected in PROTECTED_AGENTX_DIR_COMPONENTS {
                if name == std::ffi::OsStr::new(*protected) {
                    return Some(*protected);
                }
            }
        }
    }
    None
}

/// Get the agentx base directory.
///
/// Uses centralized Config (AGENTX_DIR env var or ~/.agentx fallback).
pub fn agentx_dir() -> PathBuf {
    Config::get().agentx_dir
}

/// Build path under agentx directory, optionally ensuring parent exists.
pub fn agentx_path(parts: &[&str]) -> PathBuf {
    let mut path = agentx_dir();
    for part in parts {
        path = path.join(part);
    }
    path
}

/// Get the database path (agentx_dir/agentx.db)
pub fn db_path() -> PathBuf {
    agentx_dir().join("agentx.db")
}

/// Get the pidtrack file path (agentx_dir/.tmp/launched_pids.json)
pub fn pidtrack_path() -> PathBuf {
    agentx_dir().join(".tmp").join("launched_pids.json")
}

/// Get the config TOML path (agentx_dir/config.toml)
pub fn config_toml_path() -> PathBuf {
    agentx_dir().join("config.toml")
}

/// Get the scripts directory (agentx_dir/scripts/)
pub fn scripts_dir() -> PathBuf {
    agentx_dir().join(SCRIPTS_DIR)
}

/// Ensure all critical AGENTX directories exist. Idempotent, safe to call repeatedly.
/// Called at hook entry to support opt-in scenarios where hooks execute before CLI commands.
/// Returns true on success, false on failure.
#[cfg(test)]
pub fn ensure_agentx_directories() -> bool {
    ensure_agentx_directories_at(&agentx_dir())
}

/// Ensure directories under a given base (testable without global config).
#[cfg(test)]
pub fn ensure_agentx_directories_at(base: &Path) -> bool {
    if ensure_private_directory(base).is_err() {
        return false;
    }
    for dir_name in [LOGS_DIR, LAUNCH_DIR, FLAGS_DIR, LAUNCHES_DIR, ARCHIVE_DIR] {
        if fs::create_dir_all(base.join(dir_name)).is_err() {
            return false;
        }
    }
    true
}

/// Create an agentx-owned directory and keep it private on POSIX (`0o700`).
pub(crate) fn ensure_private_directory(path: &Path) -> std::io::Result<()> {
    fs::create_dir_all(path)?;
    crate::sys::fs::set_private_dir(path)
}

/// SQLite sidecar path (`-wal` / `-shm`), appending the suffix to the *full*
/// database filename so custom names like `state.sqlite` map to
/// `state.sqlite-wal`, not `state.db-wal`.
pub(crate) fn sidecar_path(db_path: &Path, suffix: &str) -> PathBuf {
    let mut os = db_path.as_os_str().to_os_string();
    os.push(suffix);
    os.into()
}

/// Keep an agentx SQLite database and any WAL/SHM sidecars owner-private on POSIX
/// (`0o600`). No-op on `:memory:` and on Windows.
///
/// This secures the *files* only; the containing directory's `0o700` mode is
/// owned by the caller that creates the agentx directory (`ensure_private_db` is
/// also handed arbitrary temp paths under a shared, sometimes un-chmoddable
/// parent, so it must not touch the parent's mode).
///
/// Newly created sidecars inherit `0o600` from the main file via SQLite's Unix
/// VFS, but a pre-existing broad `-wal`/`-shm` (legacy install) is not
/// re-chmodded by SQLite on reopen — so we repair existing ones explicitly.
pub(crate) fn ensure_private_db(db_path: &Path) -> std::io::Result<()> {
    if db_path == Path::new(":memory:") {
        return Ok(());
    }
    if let Some(parent) = db_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    // Create the main db private, or repair an existing broad mode.
    match crate::sys::fs::create_private_new(db_path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            crate::sys::fs::set_private(db_path)?;
        }
        Err(e) => return Err(e),
    }
    for suffix in ["-wal", "-shm"] {
        let sidecar = sidecar_path(db_path, suffix);
        if sidecar.exists() {
            crate::sys::fs::set_private(&sidecar)?;
        }
    }
    Ok(())
}

/// Write content to file atomically (temp file + rename).
/// Returns the underlying IO error on failure for callers that need error detail.
pub fn atomic_write_io(filepath: &Path, content: &str) -> std::io::Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = filepath.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write to temp file in the same directory (same filesystem for rename)
    let tmp = tempfile::NamedTempFile::new_in(filepath.parent().unwrap_or_else(|| Path::new(".")))?;

    // Write content and fsync before rename to ensure data is on disk
    std::io::Write::write_all(&mut &tmp, content.as_bytes())?;
    tmp.as_file().sync_all()?;

    // Persist atomically (temp file → target path via rename)
    persist_temp_file(tmp, filepath)?;
    Ok(())
}

#[cfg(not(windows))]
fn persist_temp_file(tmp: tempfile::NamedTempFile, filepath: &Path) -> std::io::Result<()> {
    tmp.persist(filepath).map(|_| ()).map_err(|e| e.error)
}

#[cfg(windows)]
fn persist_temp_file(mut tmp: tempfile::NamedTempFile, filepath: &Path) -> std::io::Result<()> {
    // MoveFileExW can transiently return ERROR_ACCESS_DENIED while antivirus,
    // indexing, or another reader briefly holds the destination. Preserve the
    // same temp file and retry the atomic replacement for a short bounded
    // window instead of failing a config update immediately.
    const MAX_ATTEMPTS: u64 = 6;
    for attempt in 1..=MAX_ATTEMPTS {
        match tmp.persist(filepath) {
            Ok(_) => return Ok(()),
            Err(err) if err.error.kind() == std::io::ErrorKind::PermissionDenied && attempt < MAX_ATTEMPTS => {
                tmp = err.file;
                std::thread::sleep(std::time::Duration::from_millis(10 * attempt));
            }
            Err(err) => return Err(err.error),
        }
    }
    unreachable!("persist loop returns on success or final error")
}

/// Write content to file atomically (temp file + rename).
/// Returns true on success, false on failure.
pub fn atomic_write(filepath: &Path, content: &str) -> bool {
    atomic_write_io(filepath, content).is_ok()
}

/// Increment a counter in .tmp/flags/{name} and return new value.
#[cfg(test)]
pub fn increment_flag_counter(name: &str) -> i32 {
    increment_flag_counter_at(&agentx_dir(), name)
}

/// Increment flag counter under a given base (testable).
#[cfg(test)]
pub fn increment_flag_counter_at(base: &Path, name: &str) -> i32 {
    let flag_file = base.join(FLAGS_DIR).join(name);
    let _ = fs::create_dir_all(flag_file.parent().unwrap());

    let count = read_flag_file(&flag_file) + 1;
    atomic_write(&flag_file, &count.to_string());
    count
}

#[cfg(test)]
fn read_flag_file(path: &Path) -> i32 {
    fs::read_to_string(path).ok().and_then(|s| s.trim().parse::<i32>().ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_test_temp_path_accepts_temp_child() {
        let tmp = TempDir::new().unwrap();
        assert!(is_test_temp_path(&tmp.path().join("nested").join("agentx.db")));
    }

    #[test]
    fn test_is_test_temp_path_rejects_non_temp() {
        test_roots::with_temp_boundary("paths::tests::test_is_test_temp_path_rejects_non_temp", |inside, outside| {
            assert!(is_test_temp_path(&inside.join("nested/agentx.db")));
            assert!(!is_test_temp_path(&outside.join("nested/agentx.db")));
        });
    }

    #[test]
    fn test_is_test_temp_path_rejects_parent_dir_escape() {
        // A `..` in the not-yet-existing suffix lexically starts_with the temp
        // dir but resolves outside it on the real filesystem. Must fail closed.
        let escape = std::env::temp_dir().join("nonexistent").join("..").join("..").join("etc").join(".agentx").join("agentx.db");
        assert!(!is_test_temp_path(&escape));
    }

    #[test]
    fn test_ensure_agentx_directories_at() {
        let tmp = TempDir::new().unwrap();
        assert!(ensure_agentx_directories_at(tmp.path()));

        // Verify all directories were created
        assert!(tmp.path().join(LOGS_DIR).is_dir());
        assert!(tmp.path().join(LAUNCH_DIR).is_dir());
        assert!(tmp.path().join(FLAGS_DIR).is_dir());
        assert!(tmp.path().join(LAUNCHES_DIR).is_dir());
        assert!(tmp.path().join(ARCHIVE_DIR).is_dir());

        // Idempotent — second call succeeds too
        assert!(ensure_agentx_directories_at(tmp.path()));
    }

    #[cfg(unix)]
    #[test]
    fn ensure_agentx_directories_creates_private_base_directory() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("state").join(".agentx");

        assert!(ensure_agentx_directories_at(&base));

        let mode = fs::metadata(&base).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[cfg(unix)]
    #[test]
    fn ensure_agentx_directories_restricts_existing_base_directory() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join(".agentx");
        fs::create_dir(&base).unwrap();
        fs::set_permissions(&base, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(ensure_agentx_directories_at(&base));

        let mode = fs::metadata(&base).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[test]
    fn test_atomic_write() {
        let tmp = TempDir::new().unwrap();
        let filepath = tmp.path().join("test.txt");

        assert!(atomic_write(&filepath, "hello world"));
        assert_eq!(fs::read_to_string(&filepath).unwrap(), "hello world");

        // Overwrite
        assert!(atomic_write(&filepath, "new content"));
        assert_eq!(fs::read_to_string(&filepath).unwrap(), "new content");
    }

    #[test]
    fn test_atomic_write_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let filepath = tmp.path().join("a").join("b").join("test.txt");

        assert!(atomic_write(&filepath, "nested"));
        assert_eq!(fs::read_to_string(&filepath).unwrap(), "nested");
    }

    #[test]
    fn test_flag_counters() {
        let tmp = TempDir::new().unwrap();

        // Counter starts at 0 (read raw flag file)
        assert_eq!(read_flag_file(&tmp.path().join(FLAGS_DIR).join("test_flag")), 0);

        assert_eq!(increment_flag_counter_at(tmp.path(), "test_flag"), 1);
        assert_eq!(read_flag_file(&tmp.path().join(FLAGS_DIR).join("test_flag")), 1);

        assert_eq!(increment_flag_counter_at(tmp.path(), "test_flag"), 2);
        assert_eq!(read_flag_file(&tmp.path().join(FLAGS_DIR).join("test_flag")), 2);

        // Different flag is independent
        assert_eq!(read_flag_file(&tmp.path().join(FLAGS_DIR).join("other_flag")), 0);
    }

    #[test]
    fn test_resolve_agentx_dir_default() {
        let env = HashMap::from([("HOME".to_string(), "/home/test".to_string())]);
        let (path, overridden) = resolve_agentx_dir_from_env(&env, Path::new("/worktree"));
        assert_eq!(path, PathBuf::from("/home/test/.agentx"));
        assert!(!overridden);
    }

    #[test]
    fn test_resolve_agentx_dir_expands_tilde() {
        let env = HashMap::from([("HOME".to_string(), "/home/test".to_string()), ("AGENTX_DIR".to_string(), "~/custom/.agentx".to_string())]);
        let (path, overridden) = resolve_agentx_dir_from_env(&env, Path::new("/worktree"));
        assert_eq!(path, PathBuf::from("/home/test/custom/.agentx"));
        assert!(overridden);
    }

    #[test]
    fn test_protected_agentx_dir_component() {
        assert_eq!(protected_agentx_dir_component(Path::new("/home/u/proj/.git/agentx")), Some(".git"));
        assert_eq!(protected_agentx_dir_component(Path::new("/home/u/.codex/agentx")), Some(".codex"));
        assert_eq!(protected_agentx_dir_component(Path::new("/home/u/.claude/.agentx")), Some(".claude"));
        assert_eq!(protected_agentx_dir_component(Path::new("/home/u/.agents/.agentx")), Some(".agents"));
        assert_eq!(protected_agentx_dir_component(Path::new("/home/u/.agentx")), None);
        // Component-wise match: '.gitfoo' must not trigger.
        assert_eq!(protected_agentx_dir_component(Path::new("/home/u/.gitfoo/.agentx")), None);
        assert_eq!(protected_agentx_dir_component(Path::new("/home/u/proj/.agentx/sub")), None);
    }

    #[test]
    fn test_resolve_agentx_dir_makes_relative_absolute() {
        let env = HashMap::from([("AGENTX_DIR".to_string(), "relative/.agentx".to_string())]);
        let (path, overridden) = resolve_agentx_dir_from_env(&env, Path::new("/worktree"));
        assert_eq!(path, PathBuf::from("/worktree").join("relative/.agentx"));
        assert!(overridden);
    }
}
