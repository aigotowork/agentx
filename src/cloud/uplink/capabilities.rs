//! Translate explicit host exports and native public role discovery into the
//! path-free management catalogue. Launch also rechecks the current selection.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::super::client::check_headless;
use super::super::communication::store::opaque;
use super::super::control::types::{MAX_ROLES, MAX_WORKSPACES, RoleCapability, WorkspaceCapability};
use super::workspaces::{Exports, Registrations, resolve_export};

pub(super) struct Catalogue {
    pub workspaces: Vec<WorkspaceCapability>,
    pub managed_parents: Vec<String>,
    paths: BTreeMap<String, PathBuf>,
}

impl Catalogue {
    pub(super) fn discover(root: &Path, exports: &Exports, registrations: &Registrations) -> Self {
        let mut candidates: BTreeMap<_, _> = exports.iter().map(|(alias, path)| (alias, resolve_export(path))).collect();
        let managed_parents = candidates.iter().filter(|(_, path)| path.is_some()).map(|(&alias, _)| alias.clone()).collect();
        let slots = MAX_WORKSPACES.saturating_sub(candidates.len());
        candidates.extend(registrations.iter().filter(|(alias, _)| !exports.contains_key(*alias)).take(slots).map(|(alias, registration)| (alias, registration.resolve(exports))));
        let mut paths = BTreeMap::new();
        let mut remaining_roles = MAX_ROLES;
        let workspaces = candidates
            .into_iter()
            .map(|(alias, path)| {
                // A separate renew header carries parent eligibility so old servers
                // can still deserialize this original catalogue shape.
                let Some(path) = path else {
                    return WorkspaceCapability { alias: alias.clone(), default_available: false, default_reason: Some("workspace_unavailable".into()), roles: vec![], managed_parent: None };
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
                WorkspaceCapability { alias: alias.clone(), default_available: default_reason.is_none(), default_reason, roles, managed_parent: None }
            })
            .collect();
        Self { workspaces, managed_parents, paths }
    }

    pub(super) fn recheck_parent(&self, exports: &Exports, alias: &str) -> Option<PathBuf> {
        let path = resolve_export(exports.get(alias)?)?;
        (self.paths.get(alias)? == &path && self.managed_parents.iter().any(|parent| parent == alias)).then_some(path)
    }

    pub(super) fn recheck(&self, root: &Path, exports: &Exports, registrations: &Registrations, alias: &str, role: Option<&str>) -> Option<PathBuf> {
        let advertised = self.workspaces.iter().find(|workspace| workspace.alias == alias)?;
        let advertised_available = match role {
            Some(role) => advertised.roles.iter().any(|entry| entry.name == role && entry.available),
            None => advertised.default_available,
        };
        if !advertised_available {
            return None;
        }
        let canonical = match exports.get(alias) {
            Some(path) => resolve_export(path)?,
            None => registrations.get(alias)?.resolve(exports)?,
        };
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
        let registrations = Registrations::new();
        let mut published = Catalogue::discover(root.path(), &exports, &registrations);
        // The advertised choice was previously available. Local launch still
        // must refuse to follow an export alias that now names a new directory.
        published.workspaces[0].default_available = true;
        published.workspaces[0].default_reason = None;
        std::fs::remove_file(&export).unwrap();
        std::os::unix::fs::symlink(&after, &export).unwrap();
        assert!(published.recheck_parent(&exports, "example").is_none());
        assert!(published.recheck(root.path(), &exports, &registrations, "example", None).is_none());
    }
}
