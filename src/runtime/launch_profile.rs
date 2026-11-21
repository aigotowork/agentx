//! Resolve one launch base before Provider-specific execution preparation.
use crate::config::profiles::LaunchProfile;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq, clap::Args)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct LaunchSelection {
    /// Saved launch profile; overrides AGENTX_PROFILE.
    #[arg(long, conflicts_with = "no_profile")]
    pub profile: Option<String>,
    /// Ignore profiles and configured tool args; requires --tool.
    #[arg(long, requires = "tool")]
    pub no_profile: bool,
    /// Replace all inherited native args, including with an empty list.
    #[arg(long)]
    pub replace_args: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LaunchProvenance {
    pub name: String,
    pub definition: LaunchProfile,
    pub overrides: Vec<String>,
    pub replace_args: bool,
    pub selected_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ResolvedLaunchConfiguration {
    pub tool: String,
    pub args: Vec<String>,
    pub selected_by: String,
    pub base_args: Vec<String>,
    pub overrides: Vec<String>,
    pub replace_args: bool,
    pub profile: Option<LaunchProvenance>,
}

/// Select exactly one base. Legacy defaults are lazy and never read for profiles.
pub(crate) fn resolve(
    catalog: &crate::config::profiles::ProfileCatalog,
    tool: Option<&str>,
    selection: &LaunchSelection,
    environment_profile: Option<&str>,
    extra: &[String],
    legacy: impl FnOnce(&str) -> anyhow::Result<Vec<String>>,
) -> anyhow::Result<ResolvedLaunchConfiguration> {
    use anyhow::{Context, bail};
    let tool = tool.map(crate::config::profiles::canonical_tool).transpose()?;
    if selection.no_profile && (selection.profile.is_some() || tool.is_none()) {
        bail!("--no-profile requires --tool and cannot be combined with --profile");
    }
    let (name, selected_by) = if selection.no_profile {
        (None, "no_profile")
    } else if let Some(name) = selection.profile.as_deref() {
        (Some(name), "explicit_profile")
    } else if let Some(tool) = tool.as_deref() {
        (catalog.tool_defaults.get(tool).map(String::as_str), "tool_default")
    } else if let Some(name) = environment_profile.filter(|value| !value.is_empty()) {
        (Some(name), "environment_profile")
    } else {
        (catalog.default_profile.as_deref(), "scope_default")
    };
    let (tool, base, profile) = if let Some(name) = name {
        let definition = catalog.get(name)?.clone();
        definition.validate()?;
        if tool.as_ref().is_some_and(|tool| *tool != definition.tool) {
            bail!("Profile '{name}' uses '{}', which does not match --tool", definition.tool);
        }
        let provenance = LaunchProvenance { name: name.into(), definition: definition.clone(), overrides: extra.to_vec(), replace_args: selection.replace_args, selected_by: selected_by.into() };
        (definition.tool, definition.args, Some(provenance))
    } else {
        let tool = tool.context("No default profile configured; use --tool TOOL, --profile NAME or agentx profile default NAME")?;
        let base = if selection.no_profile || selection.replace_args { Vec::new() } else { legacy(&tool)? };
        (tool, base, None)
    };
    let native_tool = tool.parse().map_err(|_| anyhow::anyhow!("Unknown tool '{tool}'"))?;
    let args = if selection.replace_args { extra.to_vec() } else { crate::integrations::launch_args::overrides::override_args(native_tool, &base, extra) };
    let launch_tool = crate::runtime::launcher::LaunchTool::from_str(&tool)?;
    let errors = crate::runtime::launcher::validate_tool_args(&launch_tool, &args);
    if !errors.is_empty() {
        bail!("Invalid launch args: {}", errors.join("; "));
    }
    let selected_by = if name.is_none() && !selection.no_profile { "legacy_tool_defaults" } else { selected_by };
    Ok(ResolvedLaunchConfiguration { tool, args, selected_by: selected_by.into(), base_args: base, overrides: extra.to_vec(), replace_args: selection.replace_args, profile })
}

pub(crate) fn prepare(scope: &std::path::Path, environment: &std::collections::HashMap<String, String>, tool: Option<&str>, selection: &LaunchSelection, args: &[String]) -> anyhow::Result<ResolvedLaunchConfiguration> {
    let catalog = crate::config::profiles::ProfileCatalog::load(scope)?;
    resolve(&catalog, tool, selection, environment.get("AGENTX_PROFILE").map(String::as_str), args, |tool| {
        let config = crate::config::AgentxConfig::load_for_scope(scope, environment);
        let launch_tool = crate::runtime::launcher::LaunchTool::from_str(tool)?;
        Ok(crate::runtime::launcher::merge_tool_args(&launch_tool, &[], &config))
    })
}

/// Compose filesystem role selection and the existing profile policy once for all fresh-launch consumers.
pub(crate) fn prepare_with_role(
    scope: &std::path::Path,
    cwd: &std::path::Path,
    environment: &std::collections::HashMap<String, String>,
    flags: &super::launch_options::AgentxLaunchFlags,
    tool: Option<&str>,
    selection: &LaunchSelection,
    args: &[String],
) -> anyhow::Result<(crate::agents::workspaces::LaunchCollaboration, ResolvedLaunchConfiguration)> {
    use anyhow::{Context, bail};
    let workspace = crate::config::roles::select_workspace(cwd, environment, flags.dir.as_deref(), flags.workspace.as_deref())?;
    let definition = flags.role.as_deref().map(|selector| crate::config::roles::read_role(scope, workspace.as_deref(), selector)).transpose()?;
    let mut selection = selection.clone();
    let role_selects_profile = selection.profile.is_none() && definition.as_ref().is_some_and(|role| role.profile.is_some());
    if let Some(role) = &definition {
        if selection.no_profile {
            bail!("--role cannot be combined with --no-profile");
        }
        if selection.profile.is_none() {
            selection.profile = role.profile.clone();
        }
    }
    let mut configuration = prepare(scope, environment, tool, &selection, args)?;
    if role_selects_profile {
        configuration.selected_by = "role_profile".into();
        if let Some(profile) = &mut configuration.profile {
            profile.selected_by = "role_profile".into();
        }
    }
    let role = definition
        .map(|role| {
            let profile = configuration.profile.as_ref().context("Role launch requires a named profile; use --profile NAME or agentx profile default NAME")?;
            Ok::<_, anyhow::Error>(role.adopt(profile.name.clone()))
        })
        .transpose()?;
    let collaboration = crate::agents::workspaces::LaunchCollaboration { workspace: workspace.map(|path| path.to_string_lossy().into_owned()), role, endpoint: false, instruction_additions: None };
    Ok((collaboration, configuration))
}

pub(crate) fn preview(resolved: &ResolvedLaunchConfiguration, count: usize, flags: &super::launch_options::AgentxLaunchFlags) -> serde_json::Value {
    serde_json::json!({
        "action": "launch_preview", "executed": false, "configuration": resolved,
        "count": count, "cwd": flags.dir, "headless": flags.headless,
        "terminal": flags.terminal, "stdio": flags.stdio,
        "execution_adjustments": "Provider protocol arguments and process facts are resolved during execution"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::profiles::ProfileCatalog;

    fn catalog() -> ProfileCatalog {
        ProfileCatalog::parse(
            r#"
            [launch]
            default_profile = "daily"
            [launch.codex]
            default_profile = "deep"
            [launch.profiles.daily]
            tool = "claude"
            args = ["--model", "daily"]
            [launch.profiles.deep]
            tool = "codex"
            args = ["--model", "deep", "--add-dir", "a"]
        "#,
        )
        .unwrap()
    }

    #[test]
    fn role_launch_selects_profile_once_then_freezes_actual_definition() {
        let root = tempfile::tempdir().unwrap();
        let scope = root.path().join("scope");
        let project = root.path().join("project");
        std::fs::create_dir_all(project.join(".agent-guides/roles")).unwrap();
        std::fs::create_dir_all(&scope).unwrap();
        std::fs::write(project.join(".agent-guides/roles/coder.md"), "---\nschema: role\n---\nOriginal").unwrap();
        std::fs::write(scope.join("config.toml"), "[launch.profiles.deep]\ntool='codex'\n[launch.profiles.daily]\ntool='claude'\n").unwrap();
        let flags = super::super::launch_options::AgentxLaunchFlags { role: Some("coder".into()), ..Default::default() };
        let prepare = |selection: &LaunchSelection, env: &std::collections::HashMap<String, String>| prepare_with_role(&scope, &project, env, &flags, None, selection, &[]);
        assert!(prepare(&Default::default(), &Default::default()).is_err());
        let selection = LaunchSelection { profile: Some("deep".into()), ..Default::default() };
        let (adopted, configuration) = prepare(&selection, &Default::default()).unwrap();
        assert_eq!(adopted.role.as_ref().unwrap().profile, "deep");
        assert_eq!(configuration.selected_by, "explicit_profile");
        let environment = std::collections::HashMap::from([("AGENTX_PROFILE".into(), "daily".into())]);
        assert_eq!(prepare(&Default::default(), &environment).unwrap().0.role.unwrap().profile, "daily");
        std::fs::create_dir_all(project.join(".agentx/roles")).unwrap();
        std::fs::write(project.join(".agentx/roles/coder.toml"), "profile='daily'").unwrap();
        assert_eq!(prepare(&Default::default(), &environment).unwrap().1.selected_by, "role_profile");
        assert_eq!(prepare(&selection, &environment).unwrap().0.role.unwrap().profile, "deep");
        std::fs::write(project.join(".agent-guides/roles/coder.md"), "---\nschema: role\n---\nEdited").unwrap();
        assert!(adopted.role.unwrap().prompt.ends_with("Original"));
        let bypass = LaunchSelection { no_profile: true, ..Default::default() };
        assert!(prepare(&bypass, &environment).is_err());
    }

    #[test]
    fn selection_precedence_and_conflicts() {
        let catalog = catalog();
        let no_legacy = |_: &str| -> anyhow::Result<Vec<String>> { panic!("profile must not read legacy defaults") };
        assert_eq!(resolve(&catalog, None, &Default::default(), None, &[], no_legacy).unwrap().tool, "claude");
        assert_eq!(resolve(&catalog, Some("codex"), &Default::default(), Some("missing"), &[], no_legacy).unwrap().profile.unwrap().name, "deep");
        assert_eq!(resolve(&catalog, None, &Default::default(), Some("deep"), &[], no_legacy).unwrap().tool, "codex");
        assert!(resolve(&catalog, None, &Default::default(), Some("missing"), &[], no_legacy).is_err());
        let selection = LaunchSelection { profile: Some("deep".into()), ..Default::default() };
        assert!(resolve(&catalog, Some("claude"), &selection, None, &[], no_legacy).is_err());
    }

    #[test]
    fn overrides_replace_and_bypass_preserve_definition() {
        let catalog = catalog();
        let before = catalog.profiles.clone();
        let selection = LaunchSelection { profile: Some("deep".into()), ..Default::default() };
        let extra: Vec<String> = ["-m", "other", "--add-dir", "b", "--future", ""].into_iter().map(String::from).collect();
        let result = resolve(&catalog, None, &selection, None, &extra, |_| panic!()).unwrap();
        assert_eq!(result.args, ["--add-dir", "a", "-m", "other", "--add-dir", "b", "--future", ""]);
        assert_eq!(catalog.profiles, before);
        let replace = LaunchSelection { replace_args: true, ..selection };
        assert!(resolve(&catalog, None, &replace, None, &[], |_| panic!()).unwrap().args.is_empty());
        let bypass = LaunchSelection { no_profile: true, ..Default::default() };
        assert!(resolve(&catalog, Some("codex"), &bypass, Some("missing"), &[], |_| panic!()).unwrap().args.is_empty());
    }

    #[test]
    fn missing_default_and_invalid_reference_fail_without_fallback() {
        assert!(resolve(&ProfileCatalog::default(), None, &Default::default(), None, &[], |_| panic!()).is_err());
        let mut catalog = catalog();
        catalog.tool_defaults.insert("codex".into(), "missing".into());
        assert!(resolve(&catalog, Some("codex"), &Default::default(), None, &[], |_| panic!()).is_err());
    }
}
