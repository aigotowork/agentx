//! Project catalogue publication and launch-time revalidation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::super::client::check_headless;
use super::super::communication::store::opaque;
use super::super::control::types::{MAX_ROLES, RoleCapability, WorkspaceCapability};
use super::workspaces::{ProjectRoots, Projects, Registrations};

pub(super) struct Catalogue {
    pub workspaces: Vec<WorkspaceCapability>,
    pub roots: ProjectRoots,
    paths: BTreeMap<String, PathBuf>,
}

impl Catalogue {
    pub(super) fn discover(root: &Path, project_roots: &ProjectRoots, registrations: &Registrations) -> Self {
        let Projects { roots, entries, .. } = Projects::discover(project_roots, registrations);
        let mut paths = BTreeMap::new();
        let mut remaining_roles = MAX_ROLES;
        let workspaces = entries
            .into_iter()
            .map(|(workspace_id, project)| {
                let default_reason = unavailable(root, &project.path, None);
                let roles: Vec<_> = crate::config::roles::list_roles(root, Some(&project.path), false)
                    .map(|mut roles| {
                        roles.sort_by(|a, b| a.name.cmp(&b.name));
                        roles
                            .into_iter()
                            .filter(|role| opaque(&role.name).is_ok())
                            .take(remaining_roles)
                            .map(|role| {
                                let reason = unavailable(root, &project.path, Some(&role.name));
                                RoleCapability { name: role.name, available: reason.is_none(), reason }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                remaining_roles -= roles.len();
                paths.insert(workspace_id.clone(), project.path);
                WorkspaceCapability { workspace_id, name: project.name, default_available: default_reason.is_none(), default_reason, roles }
            })
            .collect();
        Self { workspaces, roots, paths }
    }

    pub(super) fn recheck(&self, root: &Path, project_roots: &ProjectRoots, registrations: &Registrations, workspace_id: &str, role: Option<&str>) -> Option<PathBuf> {
        let advertised = self.workspaces.iter().find(|workspace| workspace.workspace_id == workspace_id)?;
        let advertised_available = match role {
            Some(role) => advertised.roles.iter().any(|entry| entry.name == role && entry.available),
            None => advertised.default_available,
        };
        if !advertised_available {
            return None;
        }
        let current = Projects::discover(project_roots, registrations).entries.remove(workspace_id)?.path;
        if self.paths.get(workspace_id)? != &current {
            return None;
        }
        if let Some(role) = role
            && !crate::config::roles::list_roles(root, Some(&current), false).ok()?.iter().any(|entry| entry.name == role)
        {
            return None;
        }
        check_headless(root, &current, role).is_ok().then_some(current)
    }
}

fn unavailable(root: &Path, workspace: &Path, role: Option<&str>) -> Option<String> {
    check_headless(root, workspace, role).err().map(|_| "launch_unavailable".into())
}
