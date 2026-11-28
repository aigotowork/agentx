//! Resolve launch cwd on the native host, beneath one published workspace export.
use nix::{
    errno::Errno,
    fcntl::{OFlag, open, openat},
    sys::stat::{Mode, mkdirat},
};
use std::{
    os::fd::{AsRawFd, OwnedFd},
    path::{Component, Path, PathBuf},
};

use super::control::types::valid_working_directory;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DirectoryError {
    Invalid,
    Missing,
    Unsafe,
    Unavailable,
    CreateFailed,
}

/// The lexical name is for diagnostics and session metadata; only the opened directory can set cwd.
pub(super) struct WorkingDirectory {
    path: PathBuf,
    fd: OwnedFd,
}

impl WorkingDirectory {
    pub(super) fn open(workspace: &Path, relative: &str, create: bool) -> Result<Self, DirectoryError> {
        if !workspace.is_absolute() || !valid_working_directory(relative) || (create && relative.is_empty()) {
            return Err(DirectoryError::Invalid);
        }
        let flags = OFlag::O_PATH | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
        let mode = Mode::empty();
        let mut fd = open("/", flags, mode).map_err(directory_error)?;
        // Do not follow a replacement symlink even in a canonical export's own ancestors.
        for part in workspace.components() {
            match part {
                Component::RootDir => (),
                Component::Normal(part) => fd = openat(&fd, part, flags, mode).map_err(directory_error)?,
                _ => return Err(DirectoryError::Invalid),
            }
        }
        let mut parts = relative.split('/').filter(|s| !s.is_empty()).peekable();
        while let Some(part) = parts.next() {
            fd = match openat(&fd, part, flags, mode) {
                Err(Errno::ENOENT) if create && parts.peek().is_none() => {
                    match mkdirat(&fd, part, Mode::S_IRWXU) {
                        Ok(()) | Err(Errno::EEXIST) => (),
                        Err(_) => return Err(DirectoryError::CreateFailed),
                    }
                    openat(&fd, part, flags, mode).map_err(directory_error)?
                }
                result => result.map_err(directory_error)?,
            };
        }
        let path = workspace.join(relative);
        // Reject a descriptor that has already been moved away from the selected name.
        let pinned = Self { path, fd };
        if std::fs::read_link(format!("/proc/self/fd/{}", pinned.raw_fd())).ok().as_deref() != Some(pinned.path()) {
            return Err(DirectoryError::Unsafe);
        }
        Ok(pinned)
    }

    pub(super) fn path(&self) -> &Path { &self.path }

    pub(super) fn raw_fd(&self) -> std::os::fd::RawFd { self.fd.as_raw_fd() }
}

fn directory_error(error: Errno) -> DirectoryError {
    match error {
        Errno::ENOENT => DirectoryError::Missing,
        Errno::ENOTDIR | Errno::ELOOP => DirectoryError::Unsafe,
        _ => DirectoryError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn working_directory_stays_inside_export_and_creates_only_the_leaf() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        let outside = root.path().join("outside");
        std::fs::create_dir(&workspace).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::create_dir(workspace.join("existing")).unwrap();
        std::os::unix::fs::symlink(&outside, workspace.join("escape")).unwrap();

        let selected = WorkingDirectory::open(&workspace, "existing", false).unwrap();
        assert_eq!(std::fs::read_link(format!("/proc/self/fd/{}", selected.raw_fd())).unwrap(), workspace.join("existing"));
        assert_eq!(WorkingDirectory::open(&workspace, "new/leaf", true), Err(DirectoryError::Missing));
        let created = WorkingDirectory::open(&workspace, "new", true).unwrap();
        assert!(created.path().is_dir());
        assert_eq!(WorkingDirectory::open(&workspace, "../outside", false), Err(DirectoryError::Invalid));
        assert_eq!(WorkingDirectory::open(&workspace, "escape", false), Err(DirectoryError::Unsafe));
    }

    #[test]
    fn directory_paths_are_portable_and_bounded() {
        assert!(valid_working_directory(""));
        assert!(valid_working_directory("src/agentx"));
        for path in ["../outside", "/tmp", "a//b", "a/./b", "a\\b", "C:/tmp", "a/"] {
            assert!(!valid_working_directory(path), "{path}");
        }
    }
}
