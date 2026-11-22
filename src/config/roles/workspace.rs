//! Filesystem workspace defaults; runtime membership and process cwd remain separate.
use super::{canonical_directory, canonical_path, local_namespace, role_directory};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Explicit workspace > target project > inherited membership > cwd project > personal anchor.
/// Bound callers replace the inherited value with native membership before calling.
pub(crate) fn select_workspace(cwd: &Path, env: &HashMap<String, String>, dir: Option<&str>, workspace: Option<&str>) -> Result<Option<PathBuf>> {
    if let Some(value) = workspace {
        return explicit(cwd, value);
    }
    let home = home(env)?;
    if let Some(dir) = dir
        && let Some(root) = discover(&canonical_directory(cwd, dir)?, &home)?
    {
        return Ok(Some(root));
    }
    if let Some(value) = env.get("AGENTX_WORKSPACE").filter(|value| !value.is_empty()) {
        if value == "global" {
            return Ok(None);
        }
        let path = cwd.join(value);
        let personal = lookup_root(&home.join(".agentx"))?;
        if crate::paths::resolve_deepest_existing(&path).as_ref() == Some(&personal) {
            return Ok(Some(personal));
        }
        return canonical_directory(cwd, value).map(Some);
    }
    if let Some(root) = discover(&canonical_directory(cwd, ".")?, &home)? {
        return Ok(Some(root));
    }
    lookup_root(&home.join(".agentx")).map(Some)
}

fn explicit(cwd: &Path, value: &str) -> Result<Option<PathBuf>> {
    if value == "global" { Ok(None) } else { canonical_directory(cwd, value).map(Some) }
}

fn home(env: &HashMap<String, String>) -> Result<PathBuf> {
    let path = env.get(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).filter(|value| !value.is_empty()).map(PathBuf::from).or_else(dirs::home_dir).context("Cannot resolve personal workspace: HOME/USERPROFILE is unavailable")?;
    if !path.is_absolute() {
        bail!("Personal workspace home must be an absolute path");
    }
    lookup_root(&path)
}

/// Passive lookups allow a personal anchor before first creation, without writing it.
pub(super) fn lookup_root(path: &Path) -> Result<PathBuf> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => canonical_path(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => crate::paths::resolve_deepest_existing(path).filter(|path| path.is_absolute()).with_context(|| format!("Resolving workspace {}", path.display())),
        Err(error) => Err(error).with_context(|| format!("Reading workspace {}", path.display())),
    }
}

fn git_marker(directory: &Path) -> Result<bool> {
    let marker = directory.join(".git");
    match std::fs::metadata(&marker) {
        Ok(metadata) if metadata.is_dir() => Ok(true),
        Ok(metadata) if metadata.is_file() => {
            let content = std::fs::read_to_string(&marker).with_context(|| format!("Reading Git worktree marker {}", marker.display()))?;
            Ok(content.lines().next().is_some_and(|line| line.strip_prefix("gitdir:").is_some_and(|path| !path.trim().is_empty())))
        }
        Ok(_) => bail!("Git marker is not a file or directory: {}", marker.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("Reading Git marker {}", marker.display())),
    }
}

fn agentx_marker(directory: &Path, home: &Path) -> Result<bool> {
    // ~/.agentx is the personal anchor, not a marker that turns all of HOME into one project.
    if directory != home && role_directory(&directory.join(".agentx"))?.is_some() {
        local_namespace(directory)?;
        return Ok(true);
    }
    Ok(role_directory(&directory.join(".agent-guides/roles"))?.is_some())
}

fn discover(directory: &Path, home: &Path) -> Result<Option<PathBuf>> {
    for ancestor in directory.ancestors() {
        if git_marker(ancestor)? {
            return Ok(Some(ancestor.to_path_buf()));
        }
    }
    for ancestor in directory.ancestors() {
        if agentx_marker(ancestor, home)? {
            return Ok(Some(ancestor.to_path_buf()));
        }
    }
    Ok(None)
}

pub(super) fn is_project_root(directory: &Path) -> Result<bool> {
    let home = dirs::home_dir().and_then(|path| path.canonicalize().ok()).unwrap_or_default();
    Ok(git_marker(directory)? || agentx_marker(directory, &home)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        root: tempfile::TempDir,
        cwd: PathBuf,
        home: PathBuf,
        env: HashMap<String, String>,
    }

    impl Fixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let cwd = root.path().join("caller/sub");
            let home = root.path().join("home");
            std::fs::create_dir_all(&cwd).unwrap();
            std::fs::create_dir_all(&home).unwrap();
            let key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
            let env = HashMap::from([(key.into(), home.display().to_string())]);
            Self { root, cwd, home, env }
        }

        fn directory(&self, name: &str) -> PathBuf {
            let path = self.root.path().join(name);
            std::fs::create_dir_all(&path).unwrap();
            path
        }

        fn select(&self, dir: Option<&Path>, workspace: Option<&str>) -> Option<PathBuf> {
            select_workspace(&self.cwd, &self.env, dir.map(|path| path.to_str().unwrap()), workspace).unwrap()
        }
    }

    #[test]
    fn explicit_workspace_then_target_git_then_inherited_then_cwd() {
        let mut f = Fixture::new();
        let inherited = f.directory("team-a");
        let target = f.directory("target/src");
        std::fs::create_dir_all(target.parent().unwrap().join(".git")).unwrap();
        f.env.insert("AGENTX_WORKSPACE".into(), inherited.display().to_string());
        assert_eq!(f.select(Some(&target), Some("global")), None);
        assert_eq!(f.select(Some(&target), Some(inherited.to_str().unwrap())), Some(inherited.clone()));
        assert_eq!(f.select(Some(&target), None), Some(target.parent().unwrap().to_path_buf()));
        assert_eq!(f.select(None, None), Some(inherited.clone()));
        let plain = f.directory("plain");
        assert_eq!(f.select(Some(&plain), None), Some(inherited));
        f.env.remove("AGENTX_WORKSPACE");
        std::fs::create_dir_all(f.cwd.parent().unwrap().join(".git")).unwrap();
        assert_eq!(f.select(Some(&plain), None), Some(f.cwd.parent().unwrap().to_path_buf()));
    }

    #[test]
    fn git_wins_over_nested_agentx_and_nearest_git_wins() {
        let f = Fixture::new();
        let root = f.cwd.parent().unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(f.cwd.join(".agentx")).unwrap();
        assert_eq!(f.select(None, None).as_deref(), Some(root));
        std::fs::create_dir_all(f.cwd.join(".git")).unwrap();
        assert_eq!(f.select(None, None), Some(f.cwd.clone()));
    }

    #[test]
    fn linked_worktree_file_keeps_its_own_workspace() {
        let f = Fixture::new();
        let worktree = f.directory("feature/src");
        let root = worktree.parent().unwrap();
        std::fs::write(root.join(".git"), "gitdir: /fixture/main/.git/worktrees/feature\n").unwrap();
        assert_eq!(f.select(Some(&worktree), None).as_deref(), Some(root));
    }

    #[test]
    fn non_git_agentx_and_guidance_markers_still_form_projects() {
        let f = Fixture::new();
        let root = f.cwd.parent().unwrap();
        std::fs::create_dir_all(root.join(".agentx")).unwrap();
        assert_eq!(f.select(None, None).as_deref(), Some(root));
        std::fs::create_dir_all(f.cwd.join(".agent-guides/roles")).unwrap();
        assert_eq!(f.select(None, None), Some(f.cwd.clone()));
    }

    #[test]
    fn personal_fallback_is_passive_inheritable_and_not_the_home_directory() {
        let mut f = Fixture::new();
        let personal = f.home.join(".agentx");
        assert_eq!(f.select(None, None), Some(personal.clone()));
        assert!(!personal.exists());
        f.env.insert("AGENTX_WORKSPACE".into(), personal.display().to_string());
        assert_eq!(f.select(None, None), Some(personal.clone()));
        assert!(super::super::list_roles(f.root.path(), Some(&personal), false).unwrap().is_empty());
        assert!(!personal.exists());
        f.env.remove("AGENTX_WORKSPACE");
        std::fs::create_dir_all(&personal).unwrap();
        f.cwd = f.directory("home/unconfigured/sub");
        assert_eq!(f.select(None, None), Some(personal));
    }

    #[test]
    fn inherited_global_ends_selection_but_explicit_target_can_override_it() {
        let mut f = Fixture::new();
        std::fs::create_dir_all(f.cwd.join(".git")).unwrap();
        f.env.insert("AGENTX_WORKSPACE".into(), "global".into());
        assert_eq!(f.select(None, None), None);
        assert_eq!(f.select(Some(&f.cwd), None), Some(f.cwd.clone()));
        f.env.insert("AGENTX_WORKSPACE".into(), String::new());
        assert_eq!(f.select(None, None), Some(f.cwd.clone()));
        assert!(select_workspace(&f.cwd, &f.env, Some("missing"), None).is_err());
    }
}
