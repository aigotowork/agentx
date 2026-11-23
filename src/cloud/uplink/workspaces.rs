//! Remotely registered launch workspaces stay beneath a configured export.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::cloud::control::types::{MAX_WORKSPACES, valid_working_directory};
use crate::cloud::directory::WorkingDirectory;
use crate::cloud::validate_id;

use super::capabilities::Catalogue;
use super::journal::Failure;

pub(super) type Exports = BTreeMap<String, PathBuf>;
pub(super) type Registrations = BTreeMap<String, Registration>;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Registration {
    pub parent: String,
    pub parent_path: PathBuf,
    pub directory: String,
}

impl Registration {
    /// Check the request against static exports and the last accepted catalogue.
    /// No directory is created until the caller records the exact operation.
    pub(super) fn prepare(catalogue: &Catalogue, exports: &Exports, registrations: &Registrations, parent: String, directory: String, alias: &str) -> Result<Self, Failure> {
        if validate_id(alias).is_err() {
            return Err(Failure::WorkspaceAliasInvalid);
        }
        if directory.is_empty() || !valid_working_directory(&directory) {
            return Err(Failure::DirectoryInvalid);
        }
        if exports.contains_key(alias) || registrations.contains_key(alias) {
            return Err(Failure::WorkspaceAliasConflict);
        }
        if exports.len() + registrations.keys().filter(|alias| !exports.contains_key(*alias)).count() >= MAX_WORKSPACES {
            return Err(Failure::WorkspaceLimitReached);
        }
        let parent_path = catalogue.recheck_parent(exports, &parent).ok_or(Failure::CapabilityChanged)?;
        Ok(Self { parent, parent_path, directory })
    }

    pub(super) fn open(&self, create_directory: bool) -> Result<WorkingDirectory, Failure> {
        WorkingDirectory::open(&self.parent_path, &self.directory, create_directory).map_err(Failure::from)
    }

    pub(super) fn resolve(&self, exports: &Exports) -> Option<PathBuf> {
        if resolve_export(exports.get(&self.parent)?)? != self.parent_path {
            return None;
        }
        self.open(false).ok().map(|directory| directory.path().to_owned())
    }
}

pub(super) fn resolve_export(path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    (canonical.is_dir() && canonical.to_str().is_some()).then_some(canonical)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;

    use super::*;

    #[test]
    fn registration_accepts_an_existing_directory_and_refuses_alias_reuse_or_escape() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("export");
        let outside = temp.path().join("outside");
        fs::create_dir_all(root.join("existing")).unwrap();
        fs::create_dir(&outside).unwrap();
        symlink(&outside, root.join("escape")).unwrap();
        let exports = Exports::from([("default".into(), root.clone())]);
        let original = Catalogue::discover(temp.path(), &exports, &Registrations::new());
        let prepare = |directory: &str, alias, registrations: &Registrations| Registration::prepare(&original, &exports, registrations, "default".into(), directory.into(), alias);
        for directory in ["", "../outside", "/tmp", "below/../outside"] {
            assert_eq!(prepare(directory, "project", &Registrations::new()).err(), Some(Failure::DirectoryInvalid));
        }
        let existing = prepare("existing", "project", &Registrations::new()).unwrap();
        assert_eq!(existing.open(false).unwrap().path(), root.join("existing"));
        let registrations = Registrations::from([("project".into(), existing)]);
        assert_eq!(prepare("not-created", "default", &registrations).err(), Some(Failure::WorkspaceAliasConflict));
        assert_eq!(prepare("not-created", "project", &registrations).err(), Some(Failure::WorkspaceAliasConflict));
        assert_eq!(prepare("escape/leaf", "escape", &registrations).unwrap().open(true).err(), Some(Failure::DirectoryUnsafe));
        assert_eq!(prepare("missing/leaf", "missing", &registrations).unwrap().open(true).err(), Some(Failure::DirectoryMissing));
        assert!(!outside.join("leaf").exists());
        assert!(!root.join("missing").exists());
        assert!(!root.join("not-created").exists());

        fs::rename(root.join("existing"), root.join("moved")).unwrap();
        symlink(&outside, root.join("existing")).unwrap();
        let published = Catalogue::discover(temp.path(), &exports, &registrations);
        let project = published.workspaces.iter().find(|workspace| workspace.alias == "project").unwrap();
        assert_eq!(project.default_reason.as_deref(), Some("workspace_unavailable"));
        assert_eq!(published.managed_parents, ["default"]);
    }
}
