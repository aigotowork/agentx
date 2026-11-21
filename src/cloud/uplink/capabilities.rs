//! Translate explicit host exports and native public role discovery into the
//! path-free management catalogue. Launch also rechecks the current selection.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::super::client::check_headless;
use super::super::communication::store::opaque;
use super::super::control::types::{MAX_ROLES, RoleCapability, WorkspaceCapability};

pub(super) struct Catalogue {
    pub workspaces: Vec<WorkspaceCapability>,
    paths: BTreeMap<String, PathBuf>,
}

impl Catalogue {
    pub(super) fn discover(root: &Path, exports: &BTreeMap<String, PathBuf>) -> Self {
        let mut paths = BTreeMap::new();
        let mut remaining_roles = MAX_ROLES;
        let workspaces = exports
            .iter()
            .map(|(alias, export)| {
                let Some(path) = resolve(export) else {
                    return WorkspaceCapability { alias: alias.clone(), default_available: false, default_reason: Some("workspace_unavailable".into()), roles: vec![] };
                };
                let default_reason = unavailable(root, &path, None);
                let roles: Vec<_> = crate::config::roles::list_roles(root, Some(&path), false)
                    .map(|mut roles| {
                        roles.sort_by(|a, b| a.name.cmp(&b.name));
                        roles
                            .into_iter()
                            .filter(|role| opaque(&role.name).is_ok())
                            .take(remaining_roles)
                            .map(|role| {
                                let reason = unavailable(root, &path, Some(&role.name));
                                RoleCapability { name: role.name, available: reason.is_none(), reason }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                remaining_roles -= roles.len();
                paths.insert(alias.clone(), path);
                WorkspaceCapability { alias: alias.clone(), default_available: default_reason.is_none(), default_reason, roles }
            })
            .collect();
        Self { workspaces, paths }
    }

    pub(super) fn recheck(&self, root: &Path, configured: &BTreeMap<String, PathBuf>, alias: &str, role: Option<&str>) -> Option<PathBuf> {
        let advertised = self.workspaces.iter().find(|workspace| workspace.alias == alias)?;
        let advertised_available = match role {
            Some(role) => advertised.roles.iter().any(|entry| entry.name == role && entry.available),
            None => advertised.default_available,
        };
        if !advertised_available {
            return None;
        }
        let canonical = resolve(configured.get(alias)?)?;
        if self.paths.get(alias)? != &canonical {
            return None;
        }
        if let Some(role) = role
            && !crate::config::roles::list_roles(root, Some(&canonical), false).ok()?.iter().any(|entry| entry.name == role)
        {
            return None;
        }
        check_headless(root, &canonical, role).is_ok().then_some(canonical)
    }
}

fn resolve(path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    (canonical.is_dir() && canonical.to_str().is_some()).then_some(canonical)
}

fn unavailable(root: &Path, cwd: &Path, role: Option<&str>) -> Option<String> {
    check_headless(root, cwd, role).err().map(|_| "launch_unavailable".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_catalogue_refuses_an_export_alias_retargeted_after_publication() {
        let root = tempfile::tempdir().unwrap();
        let before = root.path().join("before");
        let after = root.path().join("after");
        let export = root.path().join("configured");
        std::fs::create_dir(&before).unwrap();
        std::fs::create_dir(&after).unwrap();
        std::os::unix::fs::symlink(&before, &export).unwrap();
        let exports = BTreeMap::from([("example".into(), export.clone())]);
        let mut published = Catalogue::discover(root.path(), &exports);
        // The advertised choice was previously available. Local launch still
        // must refuse to follow an export alias that now names a new directory.
        published.workspaces[0].default_available = true;
        published.workspaces[0].default_reason = None;
        std::fs::remove_file(&export).unwrap();
        std::os::unix::fs::symlink(&after, &export).unwrap();
        assert!(published.recheck(root.path(), &exports, "example", None).is_none());
    }
}
