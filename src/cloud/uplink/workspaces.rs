//! Discovery and explicit registration under the local Scope's opened project roots.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

use crate::cloud::communication::types::AgentWorkspace;
use crate::cloud::control::types::{MAX_WORKSPACES, valid_working_directory};
use crate::cloud::directory::WorkingDirectory;

use super::journal::Failure;

pub(in crate::cloud) type ProjectRoots = BTreeMap<String, PathBuf>;
pub(super) type Registrations = BTreeMap<PathBuf, Registration>;

pub(super) struct Project {
    pub path: PathBuf,
    pub name: String,
}

pub(super) struct Projects {
    pub roots: ProjectRoots,
    pub entries: BTreeMap<String, Project>,
    collisions: BTreeSet<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Registration {
    pub parent_path: PathBuf,
    pub path: PathBuf,
    pub name: String,
}

impl Projects {
    pub(super) fn discover(roots: &ProjectRoots, registrations: &Registrations) -> Self {
        let roots = roots.iter().filter_map(|(alias, path)| Some((alias.clone(), opened_root(path)?))).collect();
        let mut entries = BTreeMap::new();
        let mut collisions = BTreeSet::new();
        for project in registrations.values().filter_map(|registration| registration.resolve(&roots)) {
            insert_project(&mut entries, &mut collisions, workspace_id(&project.path), project);
        }
        let slots = MAX_WORKSPACES.saturating_sub(entries.len());
        let mut automatic = BTreeMap::new();
        if slots > 0 {
            for root in roots.values() {
                let Some(opened) = WorkingDirectory::open(root, "", false).ok() else { continue };
                let mut consider = |path: &Path| {
                    let id = workspace_id(path);
                    if recognized(root, path) {
                        let name = path.file_name().and_then(|name| name.to_str()).map(str::trim).filter(|name| valid_name(name)).unwrap_or("Project").to_owned();
                        if let Some(explicit) = entries.get(&id) {
                            if explicit.path != path {
                                collisions.insert(id.clone());
                                entries.remove(&id);
                            }
                            return;
                        }
                        insert_project(&mut automatic, &mut collisions, id, Project { path: path.to_owned(), name });
                        if automatic.len() > slots {
                            automatic.pop_last();
                        }
                    }
                };
                consider(root);
                // Enumerate only this opened root. A child must also open without using any symlink.
                if let Ok(children) = std::fs::read_dir(format!("/proc/self/fd/{}", opened.raw_fd())) {
                    for child in children.flatten() {
                        if !child.file_type().is_ok_and(|kind| kind.is_dir()) {
                            continue;
                        }
                        let filename = child.file_name();
                        if filename.to_str().is_some()
                            && let Ok(directory) = WorkingDirectory::open(&root.join(filename), "", false)
                        {
                            consider(directory.path());
                        }
                    }
                }
            }
        }
        entries.extend(automatic);
        Self { roots, entries, collisions }
    }
}

impl Registration {
    /// Check the request against the last accepted catalogue; filesystem effects
    /// happen only after the operation has been committed to the journal.
    pub(super) fn prepare(roots: &ProjectRoots, published: &ProjectRoots, registrations: &Registrations, parent: String, directory: String, name: String) -> Result<Self, Failure> {
        if !valid_name(&name) {
            return Err(Failure::WorkspaceNameInvalid);
        }
        if directory.is_empty() || !valid_working_directory(&directory) {
            return Err(Failure::DirectoryInvalid);
        }
        let parent_path = opened_root(roots.get(&parent).ok_or(Failure::CapabilityChanged)?).ok_or(Failure::CapabilityChanged)?;
        if published.get(&parent) != Some(&parent_path) {
            return Err(Failure::CapabilityChanged);
        }
        let path = parent_path.join(directory);
        let registration = Self { parent_path, path, name };
        if registrations.len() >= MAX_WORKSPACES && !registrations.contains_key(&registration.path) {
            return Err(Failure::WorkspaceLimitReached);
        }
        let projects = Projects::discover(roots, registrations);
        let id = registration.id();
        if projects.collisions.contains(&id) || projects.entries.get(&id).is_some_and(|project| project.path != registration.path) {
            return Err(Failure::WorkspaceIdCollision);
        }
        Ok(registration)
    }

    pub(super) fn id(&self) -> String {
        workspace_id(&self.path)
    }

    pub(super) fn open(&self, create_directory: bool) -> Result<WorkingDirectory, Failure> {
        let relative = self.path.strip_prefix(&self.parent_path).ok().and_then(Path::to_str).filter(|path| !path.is_empty()).ok_or(Failure::DirectoryInvalid)?;
        let directory = WorkingDirectory::open(&self.parent_path, relative, create_directory)?;
        if directory.path() != self.path {
            return Err(Failure::DirectoryUnsafe);
        }
        Ok(directory)
    }

    fn resolve(&self, roots: &ProjectRoots) -> Option<Project> {
        if !roots.values().any(|root| root == &self.parent_path) {
            return None;
        }
        let path = self.open(false).ok()?.path().to_owned();
        Some(Project { path, name: self.name.clone() })
    }
}

/// The selected native workspace is already canonical. The lookup does not inspect cwd
/// or walk to another project; a selection outside the published catalogue is unexposed.
pub(in crate::cloud) fn membership(state: &Path, namespace_url: &str, generation: &str, installation: &str, host: &str, roots: &ProjectRoots, workspace: Option<&Path>) -> anyhow::Result<AgentWorkspace> {
    let Some(path) = workspace else { return Ok(AgentWorkspace::Global) };
    let namespace = journal_namespace(namespace_url, generation, installation, host)?;
    let registrations = super::journal::Journal::read_registrations(state, &namespace)?;
    let id = workspace_id(path);
    let projects = Projects::discover(roots, &registrations);
    Ok(match projects.entries.get(&id) {
        Some(project) if project.path == path => AgentWorkspace::Project { workspace_id: id },
        _ => AgentWorkspace::Unexposed,
    })
}

pub(super) fn journal_namespace(url: &str, generation: &str, installation: &str, host: &str) -> anyhow::Result<String> {
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(serde_json::to_vec(&(url, generation, installation, host))?)))
}

fn opened_root(path: &Path) -> Option<PathBuf> {
    // Enrollment stores canonical roots. Refuse a replacement symlink in any component.
    path.to_str()?;
    WorkingDirectory::open(path, "", false).ok().map(|directory| directory.path().to_owned())
}

fn recognized(root: &Path, project: &Path) -> bool {
    // The native recognizer accepts linked-worktree marker files and configured role
    // directories. Do not let a symlink marker inspect a directory outside the opened root.
    [".git", ".agentx", ".agentx/roles", ".agent-guides/roles"].iter().all(|marker| match project.join(marker).canonicalize() {
        Ok(marker) => marker.starts_with(root),
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    }) && crate::config::roles::is_workspace(project).unwrap_or(false)
}

fn insert_project(entries: &mut BTreeMap<String, Project>, collisions: &mut BTreeSet<String>, id: String, project: Project) {
    if collisions.contains(&id) {
        return;
    }
    match entries.entry(id) {
        std::collections::btree_map::Entry::Occupied(entry) if entry.get().path != project.path => {
            collisions.insert(entry.key().clone());
            entry.remove();
        }
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(project);
        }
        _ => {}
    }
}

fn workspace_id(path: &Path) -> String {
    crate::cloud::identifiers::workspace_from_digest(&workspace_digest(path))
}

fn workspace_digest(path: &Path) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"agentx:cloud-project-workspace:v1\0");
    hasher.update(path.as_os_str().as_encoded_bytes());
    hasher.finalize().into()
}

fn valid_name(name: &str) -> bool {
    name == name.trim() && !name.is_empty() && name.len() <= 128 && !name.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::uplink::capabilities::Catalogue;
    use crate::cloud::uplink::journal::{Journal, Recovery};
    use std::fs;
    use std::os::unix::fs::symlink;

    #[test]
    fn discovery_uses_native_project_markers_at_one_level_and_keeps_same_names_distinct() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("one");
        let second = temp.path().join("two");
        let outside = temp.path().join("outside");
        fs::create_dir_all(first.join("team/src")).unwrap();
        fs::write(first.join("team/.git"), "gitdir: /fixture/linked-worktree\n").unwrap();
        fs::create_dir_all(second.join("team/.agentx/roles")).unwrap();
        fs::create_dir_all(second.join("other/deep/.git")).unwrap();
        fs::create_dir_all(outside.join(".git")).unwrap();
        fs::create_dir(first.join("bad-marker")).unwrap();
        symlink(outside.join(".git"), first.join("bad-marker/.git")).unwrap();
        symlink(&outside, first.join("escape")).unwrap();
        fs::create_dir_all(first.join(".git")).unwrap();
        fs::create_dir_all(first.join("team/.agent-guides/roles")).unwrap();
        fs::write(first.join("team/.agent-guides/roles/alpha.md"), "---\nschema: role\nrole: alpha\n---\nAlpha instructions\n").unwrap();
        fs::write(second.join("team/.agentx/roles/beta.toml"), "profile = 'deep'\nprompt = 'Beta instructions'\n").unwrap();
        let roots = ProjectRoots::from([("default".into(), first.clone()), ("duplicate".into(), first.clone()), ("additional".into(), second.clone())]);
        let catalogue = Catalogue::discover(temp.path(), &roots, &Registrations::new());
        let first_id = workspace_id(&first.join("team"));
        let second_id = workspace_id(&second.join("team"));
        assert_ne!(first_id, second_id);
        assert_eq!(first_id.len(), 12);
        assert!(first_id.starts_with("WS") && first_id.bytes().all(|b| b.is_ascii_alphanumeric()));
        assert_eq!(catalogue.workspaces.len(), 3);
        assert_eq!(catalogue.roots.len(), 3);
        for (id, role) in [(first_id, "alpha"), (second_id, "beta")] {
            let project = catalogue.workspaces.iter().find(|project| project.workspace_id == id).unwrap();
            assert_eq!(project.name, "team");
            assert_eq!(project.roles.iter().map(|entry| entry.name.as_str()).collect::<Vec<_>>(), [role]);
        }

        fs::rename(&first, temp.path().join("backup")).unwrap();
        symlink(&outside, &first).unwrap();
        let current = Projects::discover(&roots, &Registrations::new());
        assert_eq!(current.roots.keys().collect::<Vec<_>>(), ["additional"]);
        assert_eq!(current.entries.len(), 1);
    }

    #[test]
    fn registration_rejects_escapes_and_classifies_only_the_prepared_workspace() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("opened");
        let outside = temp.path().join("outside");
        fs::create_dir_all(root.join("parent/deep/src")).unwrap();
        fs::create_dir(&outside).unwrap();
        symlink(&outside, root.join("escape")).unwrap();
        let roots = ProjectRoots::from([("default".into(), root.clone())]);
        let projects = Projects::discover(&roots, &Registrations::new());
        let ns = journal_namespace("https://fixture/namespaces/team", "generation", "installation", "host").unwrap();
        let classify = |workspace: Option<&Path>| membership(temp.path(), "https://fixture/namespaces/team", "generation", "installation", "host", &roots, workspace).unwrap();
        let prepare = |relative: &str| Registration::prepare(&roots, &projects.roots, &Registrations::new(), "default".into(), relative.into(), "My project".into());
        assert_eq!(prepare("../outside").err(), Some(Failure::DirectoryInvalid));
        assert_eq!(prepare("escape/new").unwrap().open(true).err(), Some(Failure::DirectoryUnsafe));
        assert_eq!(prepare("missing/leaf").unwrap().open(true).err(), Some(Failure::DirectoryMissing));
        assert!(!outside.join("new").exists());

        let project = root.join("parent/deep");
        assert_eq!(classify(Some(&project)), AgentWorkspace::Unexposed);
        let journal = Journal::open(temp.path()).unwrap();
        let registration = prepare("parent/deep").unwrap();
        registration.open(false).unwrap();
        journal.start(&ns, "op", "hash", "claim", "native", &Recovery::NoEffect).unwrap();
        journal.register(&ns, "op", &registration).unwrap();
        drop(journal);
        let renamed_roots = ProjectRoots::from([("renamed".into(), root.clone())]);
        let registrations = Journal::read_registrations(temp.path(), &ns).unwrap();
        assert_eq!(registrations.keys().collect::<Vec<_>>(), [&project]);
        let renamed = Projects::discover(&renamed_roots, &registrations);
        assert_eq!(renamed.entries[&registration.id()].name, "My project");

        let journal = Journal::open(temp.path()).unwrap();
        let renamed_registration = Registration::prepare(&renamed_roots, &renamed.roots, &registrations, "renamed".into(), "parent/deep".into(), "Renamed project".into()).unwrap();
        journal.start(&ns, "rename", "hash", "claim-two", "native", &Recovery::NoEffect).unwrap();
        journal.register(&ns, "rename", &renamed_registration).unwrap();
        assert_eq!(journal.registrations(&ns).unwrap().len(), 1);
        assert_eq!(journal.registrations(&ns).unwrap()[&project].name, "Renamed project");
        drop(journal);
        assert_eq!(classify(Some(&project)), AgentWorkspace::Project { workspace_id: registration.id() });
        assert_eq!(classify(Some(&project.join("src"))), AgentWorkspace::Unexposed);
        assert_eq!(classify(None), AgentWorkspace::Global);

        fs::rename(&project, root.join("backup")).unwrap();
        symlink(&outside, &project).unwrap();
        assert_eq!(classify(Some(&project)), AgentWorkspace::Unexposed);
    }
}
