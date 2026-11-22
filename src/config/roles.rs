//! Pure filesystem role discovery and workspace selection.
//! Contracts and selected semantics: `.controller/roles-workspaces/AGENTS.md`.
mod reader;
mod workspace;
use super::profiles::ProfileCatalog;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
pub(crate) use workspace::select_workspace;

/// Frozen role data, independent of participant identity and workspace membership.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ResolvedRole {
    pub name: String,
    pub path: PathBuf,
    /// Logical origin plus canonical role-directory path, e.g. `global:/scope/roles`.
    pub namespace: String,
    pub profile: String,
    pub prompt: String,
    pub description: Option<String>,
    pub default: bool,
}

/// Current filesystem definition. A missing profile is selected only at launch.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RoleDefinition {
    pub name: String,
    pub path: PathBuf,
    pub namespace: String,
    pub profile: Option<String>,
    pub prompt: String,
    pub description: Option<String>,
    pub default: bool,
}

impl RoleDefinition {
    pub(crate) fn adopt(self, profile: String) -> ResolvedRole {
        ResolvedRole { name: self.name, path: self.path, namespace: self.namespace, profile, prompt: self.prompt, description: self.description, default: self.default }
    }

    fn resolve(self, scope: &Path) -> Result<ResolvedRole> {
        self.resolve_with_environment(scope, None)
    }

    fn resolve_with_environment(self, scope: &Path, environment_profile: Option<&str>) -> Result<ResolvedRole> {
        let catalog = ProfileCatalog::load(scope)?;
        let profile =
            self.profile.as_deref().or(environment_profile.filter(|value| !value.is_empty())).or(catalog.default_profile.as_deref()).context("No default profile configured for role; use --profile NAME or agentx profile default NAME")?.to_owned();
        catalog.get(&profile)?.validate().with_context(|| format!("Invalid role {}", self.path.display()))?;
        Ok(self.adopt(profile))
    }
}

/// Exact default definition and its declaring workspace, not the requested child.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DefaultRoute {
    pub role: ResolvedRole,
    pub workspace: Option<PathBuf>,
}

/// Resolve a concrete role for default routes and legacy native callers.
pub(crate) fn resolve_role(scope: &Path, workspace: Option<&Path>, selector: &str) -> Result<ResolvedRole> {
    let role = read_role(scope, workspace, selector)?;
    let path = role.path.clone();
    role.resolve(scope).with_context(|| format!("Invalid role {}", path.display()))
}

/// Read the selected definition without resolving launch configuration.
pub(crate) fn read_role(scope: &Path, workspace: Option<&Path>, selector: &str) -> Result<RoleDefinition> {
    let (qualifier, name) = match selector.split_once(':') {
        Some((qualifier @ ("local" | "global"), name)) => (Some(qualifier), name),
        Some(_) => bail!("Invalid role selector '{selector}'; use NAME, local:NAME or global:NAME"),
        None => (None, selector),
    };
    validate_name(name)?;
    let namespaces = match qualifier {
        Some("global") => global_namespace(scope)?.into_iter().collect(),
        Some("local") => {
            let workspace = workspace::lookup_root(workspace.context("local: role selection requires a directory workspace")?)?;
            local_namespace(&workspace)?.into_iter().collect()
        }
        _ => namespaces(scope, workspace)?,
    };
    for namespace in namespaces {
        if let Some(path) = namespace.source(name)? {
            return reader::read(&namespace, name, &path)?.with_context(|| format!("Role '{selector}' is disabled by {}", path.display()));
        }
    }
    bail!("Unknown role '{selector}'; use agentx roles to inspect available definitions")
}

/// List in namespace precedence and then name order; `all` retains shadowed roles.
pub(crate) fn list_roles(scope: &Path, workspace: Option<&Path>, all: bool) -> Result<Vec<RoleDefinition>> {
    list_roles_for(scope, workspace, all, None)
}

/// Default discovery is public plus direct helpers from the adopted owner's namespace.
/// `all` is an inspection escape hatch, never an authorization decision.
pub(crate) fn list_roles_for(scope: &Path, workspace: Option<&Path>, all: bool, owner: Option<&ResolvedRole>) -> Result<Vec<RoleDefinition>> {
    let mut seen = HashSet::new();
    let mut disabled = HashSet::new();
    let mut roles = Vec::new();
    for namespace in namespaces(scope, workspace)? {
        let mut defaults = Vec::new();
        for (name, path) in namespace.files()? {
            // Preserve explicit lookup precedence before filtering discovery: a
            // listed short name must never launch a different hidden definition.
            if disabled.contains(&name) || (!seen.insert(name.clone()) && !all) {
                continue;
            }
            if !all && name.split_once('.').is_some_and(|(parent, _)| owner.is_none_or(|owner| owner.name != parent || owner.namespace != namespace.identity())) {
                continue;
            }
            let Some(role) = reader::read(&namespace, &name, &path)? else {
                disabled.insert(name);
                continue;
            };
            if role.default {
                defaults.push(role.name.clone());
            }
            roles.push(role);
        }
        ensure_single_default(&namespace, &defaults)?;
    }
    Ok(roles)
}

/// Default inheritance delegates to the declaring workspace's exact definition.
#[cfg(test)]
pub(crate) fn resolve_default(scope: &Path, workspace: Option<&Path>) -> Result<DefaultRoute> {
    resolve_default_with_environment(scope, workspace, None)
}

pub(crate) fn resolve_default_with_environment(scope: &Path, workspace: Option<&Path>, environment_profile: Option<&str>) -> Result<DefaultRoute> {
    let mut disabled = HashSet::new();
    for namespace in namespaces(scope, workspace)? {
        let mut defaults = Vec::new();
        for (name, path) in namespace.files()? {
            if disabled.contains(&name) || name.contains('.') {
                continue;
            }
            let Some(role) = reader::read(&namespace, &name, &path)? else {
                disabled.insert(name);
                continue;
            };
            if role.default {
                defaults.push(role);
            }
        }
        let names: Vec<_> = defaults.iter().map(|role| role.name.clone()).collect();
        ensure_single_default(&namespace, &names)?;
        if let Some(role) = defaults.pop() {
            return Ok(DefaultRoute { role: role.resolve_with_environment(scope, environment_profile)?, workspace: namespace.workspace });
        }
    }
    bail!("No default role for the selected workspace; declare default = true in one role file or name an explicit recipient")
}

fn canonical_directory(cwd: &Path, value: &str) -> Result<PathBuf> {
    if value.is_empty() {
        bail!("Workspace/directory path must not be empty");
    }
    let path = cwd.join(value);
    canonical_path(&path)
}

fn canonical_path(path: &Path) -> Result<PathBuf> {
    let canonical = std::fs::canonicalize(path).with_context(|| format!("Resolving directory {}", path.display()))?;
    if !canonical.is_dir() {
        bail!("Not a directory: {}", path.display());
    }
    Ok(canonical)
}

fn validate_name(name: &str) -> Result<()> {
    let segments: Vec<_> = name.split('.').collect();
    let valid_segment = |segment: &&str| !segment.is_empty() && segment.as_bytes()[0].is_ascii_alphanumeric() && segment.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
    if segments.len() > 2 || !segments.iter().all(valid_segment) {
        bail!("Invalid role name '{name}': use NAME or OWNER.HELPER; each segment uses letters, digits, _ or -, starting with a letter or digit");
    }
    Ok(())
}

struct Namespace {
    // Keep the existing logical key for adopted-owner compatibility, even for Markdown-only workspaces.
    directory: PathBuf,
    markdown: Option<PathBuf>,
    workspace: Option<PathBuf>,
}

impl Namespace {
    fn identity(&self) -> String {
        let origin = if self.workspace.is_some() { "local" } else { "global" };
        format!("{origin}:{}", self.directory.display())
    }

    fn source(&self, name: &str) -> Result<Option<PathBuf>> {
        for path in std::iter::once(self.directory.join(format!("{name}.toml"))).chain(self.markdown.iter().filter(|_| self.workspace.is_some()).map(|directory| directory.join(format!("{name}.md")))) {
            match std::fs::symlink_metadata(&path) {
                Ok(_) => {
                    if path.extension().is_some_and(|value| value == "md") && !reader::is_role(&path)? {
                        continue;
                    }
                    return Ok(Some(path));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).with_context(|| format!("Reading role {}", path.display())),
            }
        }
        Ok(None)
    }

    fn files(&self) -> Result<BTreeMap<String, PathBuf>> {
        let mut files = BTreeMap::new();
        for (directory, extension) in self.markdown.iter().filter(|_| self.workspace.is_some()).map(|p| (p, "md")).chain(self.directory.is_dir().then_some((&self.directory, "toml"))) {
            for entry in std::fs::read_dir(directory).with_context(|| format!("Reading roles in {}", directory.display()))? {
                let path = entry?.path();
                if path.extension().is_none_or(|value| value != extension) {
                    continue;
                }
                // Non-role reference Markdown is not a definition. Defer malformed metadata errors until selected,
                // so unrelated hidden helpers cannot break public discovery.
                if extension == "md" && reader::is_role(&path).is_ok_and(|role| !role) {
                    continue;
                }
                let name = path.file_stem().and_then(|value| value.to_str()).with_context(|| format!("Non-UTF-8 role filename: {}", path.display()))?;
                validate_name(name).with_context(|| format!("Invalid role file {}", path.display()))?;
                files.insert(name.to_owned(), path);
            }
        }
        Ok(files)
    }
}

/// Project roots can be known offices without declaring any roles.
pub(crate) fn is_workspace(directory: &Path) -> Result<bool> {
    workspace::is_project_root(directory)
}

fn local_namespace(workspace: &Path) -> Result<Option<Namespace>> {
    let directory = role_directory(&workspace.join(".agentx/roles"))?;
    let markdown = role_directory(&workspace.join(".agent-guides/roles"))?;
    if directory.is_none() && markdown.is_none() {
        return Ok(None);
    }
    Ok(Some(Namespace { directory: directory.unwrap_or_else(|| workspace.join(".agentx/roles")), markdown, workspace: Some(workspace.to_path_buf()) }))
}

fn global_namespace(scope: &Path) -> Result<Option<Namespace>> {
    let Some(directory) = role_directory(&scope.join("roles"))? else { return Ok(None) };
    // A project-local AGENTX_DIR can expose the same TOML through local and global selectors.
    // Prompt inheritance follows its actual workspace, without publishing Markdown as global roles.
    let markdown =
        if directory.parent().is_some_and(|parent| parent.file_name().is_some_and(|name| name == ".agentx")) { role_directory(&directory.parent().and_then(Path::parent).expect("workspace parent").join(".agent-guides/roles"))? } else { None };
    Ok(Some(Namespace { directory, markdown, workspace: None }))
}

fn role_directory(path: &Path) -> Result<Option<PathBuf>> {
    // Distinguish absence from an unreadable, broken or non-directory marker.
    match std::fs::symlink_metadata(path) {
        Ok(_) => canonical_path(path).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("Reading roles directory {}", path.display())),
    }
}

fn namespaces(scope: &Path, workspace: Option<&Path>) -> Result<Vec<Namespace>> {
    let mut namespaces = Vec::new();
    if let Some(workspace) = workspace {
        let workspace = workspace::lookup_root(workspace)?;
        for ancestor in workspace.ancestors() {
            if let Some(namespace) = local_namespace(ancestor)? {
                namespaces.push(namespace);
            }
        }
    }
    if let Some(namespace) = global_namespace(scope)? {
        namespaces.push(namespace);
    }
    Ok(namespaces)
}

fn ensure_single_default(namespace: &Namespace, names: &[String]) -> Result<()> {
    if names.len() > 1 {
        bail!("Multiple default roles in {}: {}; declare at most one default per namespace", namespace.directory.display(), names.join(", "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct Fixture {
        _root: tempfile::TempDir,
        scope: PathBuf,
        parent: PathBuf,
        child: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = crate::paths::test_roots::canonical_tempdir().unwrap();
            let scope = root.path().join("scope");
            let parent = root.path().join("project");
            let child = parent.join("child");
            for path in [&scope, &parent, &child] {
                std::fs::create_dir_all(path).unwrap();
            }
            write(&scope.join("config.toml"), "[launch.profiles.deep]\ntool = 'codex'\nargs = []\n");
            Self { _root: root, scope, parent, child }
        }

        fn role(&self, workspace: Option<&Path>, name: &str, body: &str) -> PathBuf {
            let directory = workspace.map(|path| path.join(".agentx/roles")).unwrap_or_else(|| self.scope.join("roles"));
            let path = directory.join(format!("{name}.toml"));
            write(&path, body);
            path
        }
    }

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    const BASIC: &str = "profile = 'deep'\nprompt = 'literal $HOME {{name}}'\n";

    fn markdown(f: &Fixture, workspace: &Path, name: &str) -> PathBuf {
        let path = workspace.join(format!(".agent-guides/roles/{name}.md"));
        write(&path, &format!("---\nschema: role\nrole: {name}\ndescription: Shared {name}\n---\nLiteral $HOME {{{{name}}}}\n"));
        assert!(workspace.starts_with(f._root.path()));
        path
    }

    #[test]
    fn markdown_catalog_needs_no_profile_and_marks_workspace() {
        let f = Fixture::new();
        let path = markdown(&f, &f.parent, "coder");
        write(&f.parent.join(".agent-guides/roles/README.md"), "# Reference only");
        assert_eq!(select_workspace(&f.child, &HashMap::new(), None, None).unwrap(), Some(f.parent.clone()));
        let roles = list_roles(&f.scope, Some(&f.parent), false).unwrap();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0].profile, None);
        assert_eq!(roles[0].path, path);
        assert!(roles[0].prompt.ends_with("Literal $HOME {{name}}\n"));
        assert!(read_role(&f.scope, Some(&f.parent), "coder").is_ok());
        assert!(resolve_role(&f.scope, Some(&f.parent), "coder").is_err());
        write(&f.scope.join("config.toml"), "[launch]\ndefault_profile='deep'\n[launch.profiles.deep]\ntool='codex'\n");
        assert_eq!(resolve_role(&f.scope, Some(&f.parent), "coder").unwrap().profile, "deep");
    }

    #[test]
    fn thin_override_and_mixed_source_helpers_share_namespace() {
        let f = Fixture::new();
        markdown(&f, &f.child, "coder");
        markdown(&f, &f.child, "coder.test");
        let source = f.role(Some(&f.child), "coder", "profile='deep'\n");
        let owner = resolve_role(&f.scope, Some(&f.child), "coder").unwrap();
        assert_eq!(owner.path, source);
        assert_eq!(owner.description.as_deref(), Some("Shared coder"));
        assert_eq!(list_roles_for(&f.scope, Some(&f.child), false, Some(&owner)).unwrap().len(), 2);
        f.role(Some(&f.child), "coder", "prompt='replacement'\n");
        assert_eq!(read_role(&f.scope, Some(&f.child), "coder").unwrap().prompt, "replacement");
    }

    #[test]
    fn markdown_precedence_disable_and_invalid_overrides_do_not_fall_through() {
        let f = Fixture::new();
        f.role(None, "coder", BASIC);
        f.role(Some(&f.parent), "coder", &(BASIC.to_owned() + "default=true\n"));
        let md = markdown(&f, &f.child, "coder");
        assert_eq!(read_role(&f.scope, Some(&f.child), "coder").unwrap().path, md);
        let override_path = f.role(Some(&f.child), "coder", "disabled=true\n");
        assert!(read_role(&f.scope, Some(&f.child), "coder").unwrap_err().to_string().contains("disabled"));
        assert!(list_roles(&f.scope, Some(&f.child), true).unwrap().is_empty());
        assert!(resolve_default(&f.scope, Some(&f.child)).is_err());
        write(&override_path, "unknown=true\n");
        assert!(read_role(&f.scope, Some(&f.child), "coder").is_err());
        std::fs::remove_file(override_path).unwrap();
        write(&md, "---\nschema: role\nrole: wrong\n---\n");
        assert!(read_role(&f.scope, Some(&f.child), "coder").is_err());
    }

    #[test]
    fn project_scope_alias_keeps_thin_prompt_inheritance_without_global_markdown_discovery() {
        let f = Fixture::new();
        markdown(&f, &f.child, "controller");
        markdown(&f, &f.child, "coder");
        f.role(Some(&f.child), "controller", "profile='deep'\ndefault=true\n");
        let scope = f.child.join(".agentx");
        write(&scope.join("config.toml"), "[launch.profiles.deep]\ntool='codex'\n");
        let local = read_role(&scope, Some(&f.child), "controller").unwrap();
        let global = read_role(&scope, Some(&f.child), "global:controller").unwrap();
        assert_eq!(local.prompt, global.prompt);
        assert_ne!(local.namespace, global.namespace);
        assert!(read_role(&scope, None, "global:coder").is_err());
        assert_eq!(list_roles(&scope, Some(&f.child), true).unwrap().len(), 3);
    }

    #[test]
    fn thin_default_inherits_only_same_workspace_markdown() {
        let f = Fixture::new();
        markdown(&f, &f.parent, "controller");
        f.role(Some(&f.child), "controller", "profile='deep'\ndefault=true\n");
        assert!(read_role(&f.scope, Some(&f.child), "controller").is_err());
        markdown(&f, &f.child, "controller");
        assert!(resolve_default(&f.scope, Some(&f.child)).is_ok());
        write(&f.scope.join("config.toml"), "[launch.profiles.deep]\ntool='codex'\n[launch.profiles.other]\ntool='claude'\n");
        assert_eq!(resolve_default_with_environment(&f.scope, Some(&f.child), Some("other")).unwrap().role.profile, "deep");
        f.role(Some(&f.child), "controller", "default=true\n");
        assert_eq!(resolve_default_with_environment(&f.scope, Some(&f.child), Some("other")).unwrap().role.profile, "other");
        assert!(resolve_default_with_environment(&f.scope, Some(&f.child), Some("missing")).is_err());
    }

    #[test]
    fn auxiliary_discovery_is_public_plus_exact_adopted_owner() {
        let f = Fixture::new();
        for name in ["fronter", "reviewer", "fronter.e2e", "fronter.visual", "reviewer.e2e"] {
            f.role(Some(&f.child), name, BASIC);
        }
        let owner = resolve_role(&f.scope, Some(&f.child), "fronter").unwrap();
        let names = |roles: Vec<RoleDefinition>| roles.into_iter().map(|role| role.name).collect::<Vec<_>>();
        assert_eq!(names(list_roles(&f.scope, Some(&f.child), false).unwrap()), ["fronter", "reviewer"]);
        assert_eq!(names(list_roles_for(&f.scope, Some(&f.child), false, Some(&owner)).unwrap()), ["fronter", "fronter.e2e", "fronter.visual", "reviewer"]);
        let helper = resolve_role(&f.scope, Some(&f.child), "fronter.e2e").unwrap();
        assert_eq!(names(list_roles_for(&f.scope, Some(&f.child), false, Some(&helper)).unwrap()), ["fronter", "reviewer"]);
        assert_eq!(list_roles(&f.scope, Some(&f.child), true).unwrap().len(), 5);
        assert!(resolve_role(&f.scope, Some(&f.child), "local:reviewer.e2e").is_ok());
    }

    #[test]
    fn auxiliary_owner_namespace_prevents_namesake_capture_and_shadowing() {
        let f = Fixture::new();
        f.role(None, "fronter", BASIC);
        f.role(None, "fronter.e2e", BASIC);
        f.role(Some(&f.child), "fronter", BASIC);
        f.role(Some(&f.child), "fronter.e2e", BASIC);
        let global = resolve_role(&f.scope, Some(&f.child), "global:fronter").unwrap();
        let local = resolve_role(&f.scope, Some(&f.child), "local:fronter").unwrap();
        let listed = list_roles_for(&f.scope, Some(&f.child), false, Some(&local)).unwrap();
        assert_eq!(listed.iter().find(|role| role.name == "fronter.e2e").unwrap().namespace, local.namespace);
        assert_eq!(listed.len(), 2);
        let shadowed = list_roles_for(&f.scope, Some(&f.child), false, Some(&global)).unwrap();
        assert!(shadowed.iter().all(|role| !role.name.contains('.')));
        assert_eq!(resolve_role(&f.scope, Some(&f.child), "global:fronter.e2e").unwrap().namespace, global.namespace);
        std::fs::remove_file(f.child.join(".agentx/roles/fronter.e2e.toml")).unwrap();
        let listed = list_roles_for(&f.scope, Some(&f.child), false, Some(&global)).unwrap();
        assert_eq!(listed.iter().find(|role| role.name == "fronter.e2e").unwrap().namespace, global.namespace);
        let elsewhere = list_roles_for(&f.scope, Some(&f.parent), false, Some(&local)).unwrap();
        assert!(elsewhere.iter().all(|role| !role.name.contains('.')));
    }

    #[test]
    fn hidden_auxiliary_errors_do_not_break_public_discovery_or_default_routes() {
        let f = Fixture::new();
        f.role(None, "fronter", &(BASIC.to_owned() + "default = true\n"));
        f.role(None, "fronter.e2e", "invalid = [");
        assert_eq!(list_roles(&f.scope, None, false).unwrap().len(), 1);
        assert_eq!(resolve_default(&f.scope, None).unwrap().role.name, "fronter");
        assert!(list_roles(&f.scope, None, true).is_err());
        let owner = resolve_role(&f.scope, None, "fronter").unwrap();
        assert!(list_roles_for(&f.scope, None, false, Some(&owner)).is_err());
        f.role(None, "fronter.e2e", &(BASIC.to_owned() + "default = true\n"));
        assert!(format!("{:#}", resolve_role(&f.scope, None, "fronter.e2e").unwrap_err()).contains("Auxiliary roles cannot"));
    }

    #[test]
    fn auxiliary_names_have_exactly_two_valid_segments() {
        for valid in ["fronter.e2e", "Front_1.visual-check", "fronter"] {
            assert!(validate_name(valid).is_ok(), "{valid}");
        }
        for invalid in ["fronter.", ".e2e", "fronter..e2e", "fronter.e2e.more", "fronter.-e2e", "fronter/e2e", "fronter.é"] {
            assert!(validate_name(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn workspace_precedence_and_empty_markers_are_independent_of_cwd() {
        let f = Fixture::new();
        std::fs::create_dir_all(f.parent.join(".agentx/roles")).unwrap();
        let mut env = HashMap::from([("AGENTX_WORKSPACE".into(), f.scope.display().to_string())]);
        assert_eq!(select_workspace(&f.child, &env, None, None).unwrap(), Some(f.scope.clone()));
        assert_eq!(select_workspace(&f.child, &env, Some("."), None).unwrap(), Some(f.parent.clone()));
        assert_eq!(select_workspace(&f.child, &env, Some("missing"), Some("global")).unwrap(), None);
        assert_eq!(select_workspace(&f.child, &env, Some("missing"), Some(".")).unwrap(), Some(f.child.clone()));
        env.clear();
        assert_eq!(select_workspace(&f.child, &env, None, None).unwrap(), Some(f.parent.clone()));
        std::fs::create_dir_all(f.child.join(".agentx/roles")).unwrap();
        assert_eq!(select_workspace(&f.child, &env, None, None).unwrap(), Some(f.child.clone()));
        env.insert("AGENTX_WORKSPACE".into(), "global".into());
        assert_eq!(select_workspace(&f.child, &env, None, None).unwrap(), None);
        assert_eq!(select_workspace(&f.child, &env, Some("."), None).unwrap(), Some(f.child.clone()));
    }

    #[test]
    fn explicit_workspace_needs_no_marker_and_literal_global_directory_is_supported() {
        let f = Fixture::new();
        let global = f.child.join("global");
        std::fs::create_dir_all(&global).unwrap();
        assert_eq!(select_workspace(&f.child, &HashMap::new(), None, Some("./global")).unwrap(), Some(global));
        assert_eq!(select_workspace(&f.child, &HashMap::new(), None, Some(".")).unwrap(), Some(f.child.clone()));
        assert!(select_workspace(&f.child, &HashMap::new(), Some("missing"), None).is_err());
        assert!(select_workspace(&f.child, &HashMap::new(), None, Some("")).is_err());
        assert!(select_workspace(&f.scope, &HashMap::new(), None, Some("config.toml")).is_err());
    }

    #[test]
    fn nearest_definition_shadows_the_whole_role_and_explicit_origins_are_exact() {
        let f = Fixture::new();
        f.role(None, "reviewer", "profile = 'deep'\nprompt = 'global'\ndefault = true\ndescription = 'farther'\n");
        let parent_path = f.role(Some(&f.parent), "reviewer", BASIC);
        let inherited = resolve_role(&f.scope, Some(&f.child), "reviewer").unwrap();
        assert_eq!(inherited.path, parent_path);
        assert_eq!(inherited.description, None);
        assert!(!inherited.default);
        assert_eq!(inherited.prompt, "literal $HOME {{name}}");
        assert!(resolve_role(&f.scope, Some(&f.child), "local:reviewer").is_err());
        f.role(Some(&f.child), "reviewer", "profile = 'deep'\nprompt = 'child'\n");
        assert_eq!(resolve_role(&f.scope, Some(&f.child), "local:reviewer").unwrap().prompt, "child");
        assert_eq!(resolve_role(&f.scope, Some(&f.child), "global:reviewer").unwrap().prompt, "global");
        let all = list_roles(&f.scope, Some(&f.child), true).unwrap();
        assert_eq!(all.iter().map(|role| role.prompt.as_str()).collect::<Vec<_>>(), ["child", "literal $HOME {{name}}", "global"]);
        assert_eq!(list_roles(&f.scope, Some(&f.child), false).unwrap().len(), 1);
    }

    #[test]
    fn selected_invalid_definition_or_profile_never_falls_back() {
        let f = Fixture::new();
        f.role(None, "reviewer", BASIC);
        let path = f.role(Some(&f.child), "reviewer", "profile = 'missing'\nprompt = 'child'\n");
        let error = resolve_role(&f.scope, Some(&f.child), "reviewer").unwrap_err();
        assert!(format!("{error:#}").contains("Unknown profile 'missing'"));
        assert!(format!("{error:#}").contains(path.to_str().unwrap()));
        assert!(resolve_role(&f.scope, Some(&f.child), "global:reviewer").is_ok());
        write(&path, "invalid = [");
        assert!(resolve_role(&f.scope, Some(&f.child), "reviewer").is_err());
    }

    #[test]
    fn effective_listing_does_not_read_shadowed_bad_definition() {
        let f = Fixture::new();
        f.role(None, "reviewer", "invalid = [");
        f.role(Some(&f.child), "reviewer", BASIC);
        assert_eq!(list_roles(&f.scope, Some(&f.child), false).unwrap().len(), 1);
        assert!(list_roles(&f.scope, Some(&f.child), true).is_err());
    }

    #[test]
    fn prompt_file_is_literal_utf8_relative_to_toml_and_exclusive_with_inline_prompt() {
        let f = Fixture::new();
        let path = f.role(Some(&f.child), "reviewer", "profile = 'deep'\nprompt_file = '../prompts/reviewer.md'\n");
        let prompt_path = f.child.join(".agentx/prompts/reviewer.md");
        write(&prompt_path, "你好\n$HOME {{literal}}\n");
        let prompt = resolve_role(&f.scope, Some(&f.child), "reviewer").unwrap().prompt;
        assert_eq!(prompt, format!("[Role prompt file: {}]\nResolve relative paths in this role prompt from this file's parent directory.\n\n你好\n$HOME {{{{literal}}}}\n", prompt_path.canonicalize().unwrap().display()));
        for body in ["profile = 'deep'\n", "profile = 'deep'\nprompt = 'x'\nprompt_file = 'x'\n", "profile = 'deep'\nprompt_file = 'missing'\n", "profile = 'deep'\nprompt = 'x'\nunknown = true\n"] {
            write(&path, body);
            assert!(resolve_role(&f.scope, Some(&f.child), "reviewer").is_err(), "{body}");
        }
        write(&path, "profile = 'deep'\nprompt_file = '../prompts/reviewer.md'\n");
        std::fs::write(prompt_path, [0xff]).unwrap();
        assert!(resolve_role(&f.scope, Some(&f.child), "reviewer").is_err());
    }

    #[test]
    fn default_route_uses_declaring_namespace_despite_child_shadowing() {
        let f = Fixture::new();
        let parent_path = f.role(Some(&f.parent), "controller", "profile = 'deep'\nprompt = 'parent'\ndefault = true\n");
        f.role(Some(&f.child), "controller", BASIC);
        f.role(None, "global", "profile = 'deep'\nprompt = 'global'\ndefault = true\n");
        let route = resolve_default(&f.scope, Some(&f.child)).unwrap();
        assert_eq!(route.workspace, Some(f.parent.clone()));
        assert_eq!(route.role.path, parent_path);
        assert_eq!(route.role.prompt, "parent");
        f.role(Some(&f.child), "child", "profile = 'deep'\nprompt = 'child'\ndefault = true\n");
        assert_eq!(resolve_default(&f.scope, Some(&f.child)).unwrap().workspace, Some(f.child.clone()));
        assert_eq!(resolve_default(&f.scope, None).unwrap().workspace, None);
    }

    #[test]
    fn duplicate_defaults_and_malformed_candidates_stop_default_routing() {
        let f = Fixture::new();
        f.role(None, "fallback", "profile = 'deep'\nprompt = 'global'\ndefault = true\n");
        for name in ["one", "two"] {
            f.role(Some(&f.child), name, "profile = 'deep'\nprompt = 'child'\ndefault = true\n");
        }
        let error = resolve_default(&f.scope, Some(&f.child)).unwrap_err().to_string();
        assert!(error.contains("Multiple default roles"));
        assert!(error.contains("one, two"));
        assert!(list_roles(&f.scope, Some(&f.child), false).is_err());
        f.role(Some(&f.child), "two", "bad = [");
        assert!(resolve_default(&f.scope, Some(&f.child)).is_err());
    }

    #[test]
    fn coincident_local_global_directories_retain_logical_origin() {
        let f = Fixture::new();
        f.role(Some(&f.child), "controller", "profile = 'deep'\nprompt = 'one file'\ndefault = true\n");
        let scope = f.child.join(".agentx");
        write(&scope.join("config.toml"), "[launch.profiles.deep]\ntool = 'codex'\n");
        let local = resolve_role(&scope, Some(&f.child), "controller").unwrap();
        let global = resolve_role(&scope, Some(&f.child), "global:controller").unwrap();
        assert_eq!(local.path, global.path);
        assert_ne!(local.namespace, global.namespace);
        assert!(local.namespace.starts_with("local:"));
        assert!(global.namespace.starts_with("global:"));
        assert_eq!(list_roles(&scope, Some(&f.child), true).unwrap().len(), 2);
        assert_eq!(resolve_default(&scope, Some(&f.child)).unwrap().workspace, Some(f.child.clone()));
    }

    #[test]
    fn invalid_names_and_selectors_are_rejected_without_filesystem_escape() {
        let f = Fixture::new();
        for name in ["", "../outside", ".hidden", "bad:name", "global:../outside", "local:one:two", "a b"] {
            assert!(resolve_role(&f.scope, Some(&f.child), name).is_err(), "{name}");
        }
        f.role(None, "Valid_1-name", BASIC);
        assert!(resolve_role(&f.scope, None, "global:Valid_1-name").is_ok());
    }

    #[test]
    fn unconfigured_scope_uses_personal_workspace_and_missing_default_is_actionable() {
        let f = Fixture::new();
        let home_key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        let env = HashMap::from([(home_key.into(), f.scope.display().to_string())]);
        assert_eq!(select_workspace(&f.child, &env, None, None).unwrap(), Some(f.scope.join(".agentx")));
        assert!(list_roles(&f.scope, None, false).unwrap().is_empty());
        assert!(resolve_default(&f.scope, None).unwrap_err().to_string().contains("No default role"));
        f.role(Some(&f.child), "reviewer", BASIC);
        assert!(resolve_default(&f.scope, Some(&f.child)).is_err());
        std::fs::remove_dir_all(f.child.join(".agentx/roles")).unwrap();
        write(&f.child.join(".agentx/roles"), "not a directory");
        assert!(select_workspace(&f.child, &HashMap::new(), None, None).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn canonical_workspace_aliases_preserve_exact_local_selection() {
        let f = Fixture::new();
        f.role(Some(&f.child), "reviewer", BASIC);
        let alias = f.scope.join("alias");
        std::os::unix::fs::symlink(&f.child, &alias).unwrap();
        assert_eq!(select_workspace(&f.scope, &HashMap::new(), None, Some("alias")).unwrap(), Some(f.child.clone()));
        assert!(resolve_role(&f.scope, Some(&alias), "local:reviewer").is_ok());
    }
}
