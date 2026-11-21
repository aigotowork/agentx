//! Bounded `.controller` discovery and last-success/current-read observations.
#[cfg(test)]
use super::Assignment;
use super::{Document, parse, write::read_locked};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Instant,
};

const MAX_FILES: usize = 2048;
const MAX_ENTRIES: usize = 8192;
const MAX_DEPTH: usize = 8;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Query {
    pub project: PathBuf,
    #[serde(default)]
    pub paths: Vec<PathBuf>,
    pub refresh: bool,
    pub agent: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Observation {
    pub path: PathBuf,
    pub checked_at: f64,
    pub last_success_at: Option<f64>,
    /// If diagnostic is present this is retained prior data, not a current read.
    pub document: Option<Document>,
    pub diagnostic: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Projection {
    pub project: PathBuf,
    pub files: Vec<Observation>,
    pub diagnostics: Vec<String>,
}
#[derive(Default)]
pub(crate) struct Index {
    entries: BTreeMap<PathBuf, Observation>,
    projects: BTreeMap<PathBuf, Vec<String>>,
    explicit: BTreeMap<PathBuf, std::collections::BTreeSet<PathBuf>>,
    last_refresh: Option<Instant>,
}

/// Nearest existing `.controller` or VCS root; never scan ancestors or the disk.
pub(crate) fn project_root(cwd: &Path) -> PathBuf {
    cwd.ancestors().find(|path| path.join(".controller").is_dir() || path.join(".git").exists()).unwrap_or(cwd).to_path_buf()
}
impl Index {
    /// Discover a project for assignment enrichment without cloning its full view.
    pub(crate) fn observe_project(&mut self, project: &Path) -> Result<()> {
        self.update_project(project.canonicalize()?, false)
    }

    pub(crate) fn observations(&self) -> impl Iterator<Item = &Observation> {
        self.entries.values()
    }

    pub(crate) fn query(&mut self, query: &Query, cwd: &Path) -> Result<Projection> {
        if query.paths.len() > 64 {
            bail!("at most 64 explicit files per query");
        }
        let project = if query.project.is_absolute() { query.project.clone() } else { cwd.join(&query.project) }.canonicalize()?;
        self.update_project(project.clone(), query.refresh)?;

        for path in &query.paths {
            let path = if path.is_absolute() { path.clone() } else { cwd.join(path) };
            let path = path.canonicalize().unwrap_or(path);
            self.observe(&path);
            self.explicit.entry(project.clone()).or_default().insert(path);
        }
        let mut diagnostics = self.projects.get(&project).cloned().unwrap_or_default();
        if self.entries.len() >= MAX_ENTRIES {
            diagnostics.push("observation limit (8192) reached; use a smaller project".into());
        }
        let controller = project.join(".controller");
        let explicit = self.explicit.get(&project);
        let files = self.entries.values().filter(|entry| entry.path.starts_with(&controller) || explicit.is_some_and(|paths| paths.contains(&entry.path))).cloned().collect();
        Ok(Projection { project, files, diagnostics })
    }

    fn update_project(&mut self, project: PathBuf, refresh: bool) -> Result<()> {
        if !project.is_dir() {
            bail!("project must be a directory");
        }
        if refresh || !self.projects.contains_key(&project) {
            if self.projects.len() >= 64 && !self.projects.contains_key(&project) {
                bail!("project observation limit (64) reached");
            }
            let (paths, diagnostics) = discover(&project);
            for path in paths {
                self.observe(&path);
            }
            self.projects.insert(project.clone(), diagnostics);
            // Includes deleted files and explicitly added paths under this project.
            if refresh {
                let paths: Vec<_> = self.entries.keys().filter(|path| path.starts_with(&project) || self.explicit.get(&project).is_some_and(|paths| paths.contains(*path))).cloned().collect();
                for path in paths {
                    self.observe(&path);
                }
            }
        }

        Ok(())
    }
    fn observe(&mut self, path: &Path) {
        if self.entries.len() >= MAX_ENTRIES && !self.entries.contains_key(path) {
            return;
        }
        let checked_at = crate::shared::time::now_epoch_f64();
        let result = read_locked(path).and_then(|text| parse(&text));
        let previous = self.entries.get(path);
        let (document, last_success_at, diagnostic) = match result {
            Ok(document) => (Some(document), Some(checked_at), None),
            Err(error) => (previous.and_then(|old| old.document.clone()), previous.and_then(|old| old.last_success_at), Some(format!("{error:#}"))),
        };
        self.entries.insert(path.to_path_buf(), Observation { path: path.to_path_buf(), checked_at, last_success_at, document, diagnostic });
    }
    /// Refresh only known files; the caller controls when a refresh is needed.
    pub(crate) fn refresh_known(&mut self) {
        if self.last_refresh.is_some_and(|at| at.elapsed().as_secs() < 1) {
            return;
        }
        let paths: Vec<_> = self.entries.keys().cloned().collect();
        for path in paths {
            self.observe(&path);
        }
        self.last_refresh = Some(Instant::now());
    }
    #[cfg(test)]
    pub(crate) fn for_assignment(&self, assignment: &Assignment) -> Vec<Observation> {
        self.entries.values().filter(|entry| entry.document.as_ref().is_some_and(|doc| doc.assignments.contains(assignment))).cloned().collect()
    }
}

fn discover(project: &Path) -> (Vec<PathBuf>, Vec<String>) {
    let mut files = Vec::new();
    let mut diagnostics = Vec::new();
    let root = project.join(".controller");
    if !root.exists() {
        return (files, diagnostics);
    }
    if root.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()) {
        return (files, vec![".controller symlink is not traversed".into()]);
    }
    let mut stack = vec![(root, 0)];
    let mut visited = 0usize;
    let mut candidates = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                diagnostics.push(format!("{}: {error}", dir.display()));
                continue;
            }
        };
        for entry in entries {
            visited += 1;
            if visited > MAX_ENTRIES {
                diagnostics.push("discovery entry limit (8192) reached".into());
                return (files, diagnostics);
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    diagnostics.push(error.to_string());
                    continue;
                }
            };
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => {
                    if depth < MAX_DEPTH {
                        stack.push((path, depth + 1));
                    } else {
                        diagnostics.push(format!("depth limit reached: {}", path.display()));
                    }
                }
                Ok(kind) if kind.is_file() => {
                    // Markdown carriers only, but filename has no schema meaning.
                    if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")) {
                        candidates += 1;
                        if candidates > MAX_FILES {
                            diagnostics.push("discovery file limit (2048) reached".into());
                            return (files, diagnostics);
                        }
                        // Unsupported document schemas are irrelevant discovery input;
                        // explicit files and previously observed sources retain errors.
                        let relevant = match read_locked(&path).and_then(|text| parse(&text)) {
                            Ok(_) => true,
                            Err(error) => {
                                let message = error.to_string();
                                !message.starts_with("unsupported schema:") && message != "missing YAML frontmatter"
                            }
                        };
                        if relevant {
                            files.push(path);
                        }
                    }
                }
                Err(error) => diagnostics.push(format!("{}: {error}", path.display())),
                _ => {}
            }
        }
    }
    (files, diagnostics)
}
