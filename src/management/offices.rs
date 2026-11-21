//! Host office projections and finite discovery. Client reads retain generation fencing;
//! filesystem/config work must run outside SQLite dispatch (HTTP uses blocking tasks).
use super::contract::*;
use crate::agents::workspaces::{self, CheckedCollaboration};
use crate::config::roles::{self as native_roles, ResolvedRole};
use crate::service::{Request, Response, ServiceError, ServiceResult, client::Client};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};

const MAX_WORKSPACES: usize = 100;
const MAX_DIRECTORIES: u32 = 512;
const MAX_ERRORS: usize = 16;
const MAX_SCAN_ENTRIES: u32 = 8192;

pub(crate) fn collaboration(context: Option<&str>) -> CollaborationView {
    project_collaboration(workspaces::checked_collaboration(context))
}

/// Stopped snapshot launch_context is historically either serialized JSON or an object.
pub(crate) fn collaboration_value(snapshot: &serde_json::Value) -> CollaborationView {
    match snapshot.get("launch_context") {
        None | Some(serde_json::Value::Null) => collaboration(None),
        Some(serde_json::Value::String(context)) => collaboration(Some(context)),
        Some(context) => project_collaboration(workspaces::checked_collaboration_value(context)),
    }
}

fn project_collaboration(checked: CheckedCollaboration) -> CollaborationView {
    let (status, workspace, role, reason) = match checked {
        CheckedCollaboration::Missing => ("legacy", None, None, None),
        CheckedCollaboration::Valid(snapshot) => ("adopted", snapshot.workspace, snapshot.role.map(role_view), None),
        CheckedCollaboration::Unavailable(reason) => ("unavailable", None, None, Some(reason)),
    };
    CollaborationView { status: status.into(), workspace, role, reason }
}

fn role_view(role: ResolvedRole) -> RoleView {
    RoleView { name: role.name, path: role.path.to_string_lossy().into_owned(), namespace: role.namespace, profile: role.profile, description: role.description, default: role.default }
}

pub(crate) fn catalog(client: &Client, project: &Path) -> ServiceResult<WorkspaceCatalog> {
    let mut rows = BTreeMap::new();
    let mut coverage = DiscoveryCoverage { agent_pages: 0, agent_limit_reached: false, directories_examined: 0, scan_limit_reached: false, errors: vec![] };
    let response = client.call(Request::Management(super::contract::Request::AdoptedWorkspaces))?;
    let Response::Management(super::contract::Response::AdoptedWorkspaces(adopted)) = response else { return Err(unexpected()) };
    coverage.agent_pages = adopted.agent_pages;
    coverage.agent_limit_reached = adopted.agent_limit_reached;
    for path in adopted.paths {
        add_workspace(&mut rows, path.clone(), "adopted", &mut coverage);
        if let Err(error) = existing_directory(&path) {
            record_error(&mut coverage, error);
        }
    }
    discover(project, &mut rows, &mut coverage);
    let workspaces = rows.into_iter().map(|(path, sources)| WorkspaceView { path, sources }).collect();
    Ok(WorkspaceCatalog { generation: client.info().database_generation.clone(), workspaces, coverage })
}

pub(crate) fn roles(client: &Client, workspace: &str) -> ServiceResult<RoleCatalog> {
    let selected = explicit_workspace(workspace)?;
    let roles = effective_roles(client.scope_root(), selected.as_deref())?;
    Ok(RoleCatalog { generation: client.info().database_generation.clone(), workspace: selected.map(path_string), roles })
}

fn effective_roles(scope: &Path, workspace: Option<&Path>) -> ServiceResult<Vec<RoleDefinitionView>> {
    // Native effective selection owns shadowing and selected-default validation.
    let roles = native_roles::list_roles(scope, workspace, false).map_err(config_error)?;
    if roles.len() > 100 {
        return Err(ServiceError::InvalidInput("effective role catalog exceeds 100 roles".into()));
    }
    Ok(roles.into_iter().map(|role| RoleDefinitionView { name: role.name, path: role.path.to_string_lossy().into_owned(), namespace: role.namespace, profile: role.profile, description: role.description, default: role.default }).collect())
}

pub(crate) fn entry(client: &Client, directory: &str) -> ServiceResult<DirectoryEntry> {
    let directory = existing_directory(directory)?;
    // Preview and execution must use the same originating cwd/environment defaults.
    let requested = native_roles::select_workspace(&client.invocation().cwd, &client.invocation().environment, Some(&path_string(directory.clone())), None).map_err(config_error)?;
    let route = native_roles::resolve_default_with_environment(client.scope_root(), requested.as_deref(), client.invocation().environment.get("AGENTX_PROFILE").map(String::as_str)).map_err(config_error)?;
    let response = client.call(Request::Participants(crate::agents::service::Request::Launch(crate::agents::launch_service::Request::EndpointInspect {
        workspace: route.workspace.as_ref().map(|path| path.to_string_lossy().into_owned()),
        role_path: route.role.path.to_string_lossy().into_owned(),
    })))?;
    let Response::Participants(crate::agents::service::Response::Launch(crate::agents::launch_service::Response::EndpointInspected(inspection))) = response else { return Err(unexpected()) };
    let status = match inspection.status {
        workspaces::EndpointStatus::Bound => "bound",
        workspaces::EndpointStatus::Adoptable => "adoptable",
        workspaces::EndpointStatus::Absent => "absent",
        workspaces::EndpointStatus::Ambiguous => "ambiguous",
        workspaces::EndpointStatus::RoleChanged => "role_changed",
        workspaces::EndpointStatus::Stale => "stale",
        workspaces::EndpointStatus::Unknown => "unknown",
    };
    let endpoint = EndpointView {
        status: status.into(),
        candidates: inspection.candidates,
        reason: inspection.reason,
        participant: inspection.participant.map(|participant| AgentRef { name: participant.canonical_name, incarnation: Some(participant.incarnation_bits.to_string()), stopped_event: None }),
    };
    Ok(DirectoryEntry {
        generation: client.info().database_generation.clone(),
        directory: path_string(directory),
        requested_workspace: requested.map(path_string),
        resolved_workspace: route.workspace.map(path_string),
        role: role_view(route.role),
        endpoint,
    })
}

fn explicit_workspace(workspace: &str) -> ServiceResult<Option<PathBuf>> {
    if workspace != "global" {
        existing_directory(workspace)?;
    }
    native_roles::select_workspace(Path::new("/"), &HashMap::new(), None, Some(workspace)).map_err(config_error)
}
fn existing_directory(value: &str) -> ServiceResult<PathBuf> {
    if !Path::new(value).is_absolute() {
        return Err(ServiceError::InvalidInput("directory/workspace must be absolute or workspace must be global".into()));
    }
    let canonical = std::fs::canonicalize(value).map_err(|error| ServiceError::InvalidInput(format!("reading directory {}: {error}", bounded(value))))?;
    if !canonical.is_dir() {
        return Err(ServiceError::InvalidInput("selected path is not a directory".into()));
    }
    Ok(canonical)
}
fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}
fn config_error(error: anyhow::Error) -> ServiceError {
    ServiceError::InvalidInput(bounded(&error.to_string()))
}
fn unexpected() -> ServiceError {
    ServiceError::Internal("unexpected office query response".into())
}
fn bounded(value: &str) -> String {
    value.chars().take(256).collect()
}
fn record_error(coverage: &mut DiscoveryCoverage, error: impl std::fmt::Display) {
    if coverage.errors.len() < MAX_ERRORS {
        coverage.errors.push(bounded(&error.to_string()));
    }
}
fn add_workspace(rows: &mut BTreeMap<String, Vec<String>>, path: String, source: &str, coverage: &mut DiscoveryCoverage) {
    if let Some(sources) = rows.get_mut(&path) {
        if !sources.iter().any(|existing| existing == source) {
            sources.push(source.into());
        }
    } else if rows.len() < MAX_WORKSPACES {
        rows.insert(path, vec![source.into()]);
    } else {
        coverage.scan_limit_reached = true;
    }
}

/// No symlink recursion, hidden home walk, or dependency/build/AgentX internals.
fn discover(project: &Path, rows: &mut BTreeMap<String, Vec<String>>, coverage: &mut DiscoveryCoverage) {
    let root = match existing_directory(&project.to_string_lossy()) {
        Ok(root) => root,
        Err(error) => {
            record_error(coverage, error);
            return;
        }
    };
    let mut queue = VecDeque::from([(root, 0u32)]);
    let mut admitted = 1u32;
    let mut entries_examined = 0u32;
    'scan: while let Some((directory, depth)) = queue.pop_front() {
        coverage.directories_examined += 1;
        match native_roles::is_workspace(&directory) {
            Ok(true) => add_workspace(rows, directory.to_string_lossy().into_owned(), "marker", coverage),
            Ok(false) => {}
            Err(error) => record_error(coverage, format!("{error:#}")),
        }
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                record_error(coverage, format!("{}: {error}", directory.display()));
                continue;
            }
        };
        for item in entries {
            if entries_examined >= MAX_SCAN_ENTRIES {
                coverage.scan_limit_reached = true;
                break 'scan;
            }
            entries_examined += 1;
            let item = match item {
                Ok(item) => item,
                Err(error) => {
                    record_error(coverage, error);
                    continue;
                }
            };
            let name = item.file_name();
            if matches!(name.to_str(), Some("node_modules" | ".git" | "target" | ".agentx")) {
                continue;
            }
            match item.file_type() {
                Ok(kind) if kind.is_dir() && !kind.is_symlink() => {
                    if depth >= 3 || admitted >= MAX_DIRECTORIES {
                        coverage.scan_limit_reached = true;
                        continue;
                    }
                    admitted += 1;
                    queue.push_back((item.path(), depth + 1));
                }
                Ok(_) => {}
                Err(error) => record_error(coverage, error),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn coverage() -> DiscoveryCoverage {
        DiscoveryCoverage { agent_pages: 0, agent_limit_reached: false, directories_examined: 0, scan_limit_reached: false, errors: vec![] }
    }
    fn write(path: &Path, value: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, value).unwrap();
    }
    fn adopted() -> serde_json::Value {
        json!({"workspace":"/member", "endpoint":false, "role":{
            "name":"controller", "path":"/deleted/role.toml", "namespace":"global:/deleted",
            "profile":"deep", "prompt":"SECRET ADOPTED PROMPT", "description":"controller role", "default":true
        }, "instruction_additions":"SECRET ADDITION"})
    }

    #[test]
    fn current_and_stopped_collaboration_preserve_provenance_and_withhold_instructions() {
        let context = json!({"collaboration":adopted(),"env":{"TOKEN":"SECRET ENV"}});
        let current = collaboration(Some(&context.to_string()));
        let object = collaboration_value(&json!({"launch_context":context}));
        let string = collaboration_value(&json!({"launch_context":context.to_string()}));
        let expected = serde_json::to_value(&current).unwrap();
        assert_eq!(serde_json::to_value(object).unwrap(), expected);
        assert_eq!(serde_json::to_value(string).unwrap(), expected);
        assert_eq!(current.status, "adopted");
        assert_eq!(current.workspace.as_deref(), Some("/member"));
        assert_eq!(current.role.unwrap().path, "/deleted/role.toml");
        assert!(!expected.to_string().contains("SECRET"));
    }

    #[test]
    fn absence_explicit_global_roleless_and_corrupt_are_distinct() {
        for snapshot in [json!({}), json!({"launch_context":null}), json!({"launch_context":"{}"})] {
            assert_eq!(collaboration_value(&snapshot).status, "legacy");
        }
        let global = collaboration(Some(&json!({"collaboration":{"workspace":null,"role":null,"endpoint":false}}).to_string()));
        assert_eq!(global.status, "adopted");
        assert!(global.workspace.is_none() && global.role.is_none());
        let local = collaboration(Some(&json!({"collaboration":{"workspace":"/explicit/no-marker","role":null,"endpoint":false}}).to_string()));
        assert_eq!(local.workspace.as_deref(), Some("/explicit/no-marker"));
        for context in [json!("broken"), json!([]), json!({"collaboration":null}), json!({"collaboration":{"workspace":null,"endpoint":false}})] {
            let unavailable = collaboration_value(&json!({"launch_context":context}));
            assert_eq!(unavailable.status, "unavailable");
            assert!(unavailable.workspace.is_none());
        }
        for (field, invalid) in [("workspace", json!("relative")), ("role", json!({"name":"x"}))] {
            let mut adoption = adopted();
            adoption[field] = invalid;
            assert_eq!(collaboration(Some(&json!({"collaboration":adoption}).to_string())).status, "unavailable");
        }
        for (field, invalid) in [("namespace", json!("global:relative")), ("path", json!("relative")), ("namespace", json!("future:/role"))] {
            let mut adoption = adopted();
            adoption["role"][field] = invalid;
            assert_eq!(collaboration(Some(&json!({"collaboration":adoption}).to_string())).status, "unavailable");
        }
    }

    #[test]
    fn discovery_includes_git_root_and_nested_config_without_scanning_internal_markers() {
        let root = tempfile::tempdir().unwrap();
        for path in ["empty/.agentx/roles", "nested/office/.agentx/roles", "node_modules/no/.agentx/roles", ".git/no/.agentx/roles", "target/no/.agentx/roles", ".agentx/no/.agentx/roles", "too/deep/four/office/.agentx/roles"] {
            std::fs::create_dir_all(root.path().join(path)).unwrap();
        }
        let mut rows = BTreeMap::new();
        let mut covered = coverage();
        discover(root.path(), &mut rows, &mut covered);
        assert_eq!(rows.len(), 3);
        assert!(rows.contains_key(&root.path().join("empty").to_string_lossy().into_owned()));
        assert!(rows.contains_key(&root.path().join("nested/office").to_string_lossy().into_owned()));
        assert!(rows.contains_key(&root.path().to_string_lossy().into_owned()), "the fixture root has its own .git and .agentx markers");
        assert!(covered.scan_limit_reached, "depth cap is incomplete coverage");
        assert!(covered.directories_examined <= MAX_DIRECTORIES);
    }

    #[cfg(unix)]
    #[test]
    fn discovery_never_recurses_through_directory_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(outside.path().join("office/.agentx/roles")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
        let mut rows = BTreeMap::new();
        let mut covered = coverage();
        discover(root.path(), &mut rows, &mut covered);
        assert!(rows.is_empty());
        assert_eq!(covered.directories_examined, 1);
        assert!(!covered.scan_limit_reached);
    }

    #[test]
    fn discovery_caps_directory_and_workspace_counts_and_reports_errors() {
        let root = tempfile::tempdir().unwrap();
        for index in 0..520 {
            std::fs::create_dir_all(root.path().join(format!("office-{index}/.agentx/roles"))).unwrap();
        }
        let mut rows = BTreeMap::new();
        let mut covered = coverage();
        discover(root.path(), &mut rows, &mut covered);
        assert_eq!(covered.directories_examined, 512);
        assert_eq!(rows.len(), 100);
        assert!(covered.scan_limit_reached);
        for _ in 0..30 {
            record_error(&mut covered, "e".repeat(500));
        }
        assert_eq!(covered.errors.len(), MAX_ERRORS);
        assert!(covered.errors.iter().all(|error| error.len() <= 256));
        let mut invalid = coverage();
        discover(&root.path().join("missing"), &mut BTreeMap::new(), &mut invalid);
        assert_eq!(invalid.errors.len(), 1);
    }

    #[test]
    fn flat_directory_entry_budget_is_finite_and_reported() {
        let root = tempfile::tempdir().unwrap();
        for index in 0..MAX_SCAN_ENTRIES + 1 {
            std::fs::write(root.path().join(format!("file-{index}")), "").unwrap();
        }
        let mut covered = coverage();
        discover(root.path(), &mut BTreeMap::new(), &mut covered);
        assert!(covered.scan_limit_reached);
        assert_eq!(covered.directories_examined, 1);
    }

    #[test]
    fn adopted_marker_sources_merge_and_deleted_membership_stays_known() {
        let mut rows = BTreeMap::new();
        let mut covered = coverage();
        for source in ["adopted", "marker", "adopted"] {
            add_workspace(&mut rows, "/deleted/office".into(), source, &mut covered);
        }
        assert_eq!(rows["/deleted/office"], vec!["adopted", "marker"]);
        assert!(existing_directory("relative").is_err());
        assert!(explicit_workspace("relative").is_err());
        assert!(explicit_workspace("global").unwrap().is_none());
    }

    #[test]
    fn effective_roles_use_native_shadowing_origin_and_default_resolution() {
        let root = tempfile::tempdir().unwrap();
        let scope = root.path().join("scope");
        let parent = root.path().join("project");
        let child = parent.join("child");
        std::fs::create_dir_all(&child).unwrap();
        write(&scope.join("config.toml"), "[launch.profiles.deep]\ntool = 'codex'\nargs = []\n");
        write(&scope.join("roles/controller.toml"), "BROKEN shadowed definition");
        write(&scope.join("roles/helper.toml"), "profile='deep'\nprompt='global secret'\n");
        write(&parent.join(".agentx/roles/controller.toml"), "profile='deep'\nprompt='parent secret'\ndefault=true\n");
        std::fs::create_dir_all(child.join(".agentx/roles")).unwrap();
        let effective = effective_roles(&scope, Some(&child)).unwrap();
        assert_eq!(effective.len(), 2);
        assert_eq!(effective[0].name, "controller");
        assert!(effective[0].namespace.starts_with("local:"));
        assert!(!serde_json::to_string(&effective).unwrap().contains("secret"));
        let selected = native_roles::select_workspace(&child, &HashMap::new(), Some("."), None).unwrap();
        assert_eq!(selected.as_deref(), Some(child.as_path()));
        let route = native_roles::resolve_default(&scope, selected.as_deref()).unwrap();
        assert_eq!(route.workspace.as_deref(), Some(parent.as_path()));
        // Unshadowed malformed definitions and conflicting defaults fail the whole query.
        write(&scope.join("roles/helper.toml"), "BROKEN");
        assert!(effective_roles(&scope, Some(&child)).is_err());
        write(&scope.join("roles/helper.toml"), "profile='deep'\nprompt='ok'\n");
        write(&parent.join(".agentx/roles/second.toml"), "profile='deep'\nprompt='ok'\ndefault=true\n");
        assert!(effective_roles(&scope, Some(&child)).is_err());
    }

    #[test]
    fn effective_roles_reject_overflow_but_defer_profile_selection() {
        let root = tempfile::tempdir().unwrap();
        write(&root.path().join("config.toml"), "[launch.profiles.deep]\ntool = 'codex'\nargs = []\n");
        for index in 0..101 {
            write(&root.path().join(format!("roles/role{index}.toml")), "profile='deep'\nprompt='ok'\n");
        }
        assert!(matches!(effective_roles(root.path(), None), Err(ServiceError::InvalidInput(reason)) if reason.contains("100")));
        write(&root.path().join("roles/role0.toml"), "profile='missing'\nprompt='ok'\n");
        std::fs::remove_file(root.path().join("roles/role100.toml")).unwrap();
        let catalog = effective_roles(root.path(), None).unwrap();
        assert_eq!(catalog.iter().find(|role| role.name == "role0").unwrap().profile.as_deref(), Some("missing"));
        assert!(native_roles::resolve_role(root.path(), None, "role0").is_err());
    }
}
