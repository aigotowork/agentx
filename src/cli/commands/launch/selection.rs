//! Fresh-launch selection policy; native catalogs retain discovery and configuration rules.

use crate::config::{profiles::ProfileCatalog, roles};
use crate::runtime::{launch_options::AgentxLaunchFlags, launch_profile::LaunchSelection};
use crate::service::InvocationContext;
use crate::tui::select::{self, Choice};
use anyhow::{Result, bail};
use std::io::{IsTerminal, stderr, stdin};

const OPTIONS: &[&str] = &["--role", "--profile", "--tool"];

/// Protect omitted selectable values from early opaque-value scanners.
pub(crate) fn normalize_args(argv: &[String]) -> Vec<String> {
    let mut command = 0;
    while command < argv.len() && argv[command].starts_with('-') && argv[command] != "--" {
        command += if crate::runtime::environment::takes_opaque_value(&argv[command]) { 2 } else { 1 };
    }
    if argv.get(command).is_none_or(|word| crate::cli::router::canonical_command(word) != "launch") {
        return argv.to_vec();
    }
    let mut result = argv[..=command].to_vec();
    let mut index = command + 1;
    while index < argv.len() {
        let word = argv[index].as_str();
        if word == "--" {
            result.extend_from_slice(&argv[index..]);
            break;
        }
        if OPTIONS.contains(&word) && argv.get(index + 1).is_none_or(|next| next.starts_with('-')) {
            result.push(format!("{word}="));
            index += 1;
            continue;
        }
        result.push(argv[index].clone());
        index += 1;
        if (OPTIONS.contains(&word) || crate::runtime::environment::takes_opaque_value(word)) && index < argv.len() {
            result.push(argv[index].clone());
            index += 1;
        }
    }
    result
}

/// Resolve only explicitly requested choices before acquiring a launch session.
pub(super) fn resolve(tool: &mut Option<String>, selection: &mut LaunchSelection, flags: &mut AgentxLaunchFlags, invocation: &InvocationContext) -> Result<bool> {
    let requested = [("--role", &flags.role), ("--profile", &selection.profile), ("--tool", &*tool)].into_iter().filter(|(_, value)| value.as_deref() == Some("")).map(|(option, _)| option).collect::<Vec<_>>();
    if requested.is_empty() {
        return Ok(true);
    }
    let options = requested.join(", ");
    if flags.json || flags.stdio || flags.device.is_some() || !stdin().is_terminal() || !stderr().is_terminal() {
        bail!("{options} requires an explicit value in noninteractive, --json, --stdio or remote launches; use a local terminal to select");
    }
    if flags.role.is_some() && selection.no_profile {
        bail!("--role cannot be combined with --no-profile");
    }
    let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
    let catalog = ProfileCatalog::load(&scope)?;
    // Check supplied names before opening a picker; explicit mistakes stay errors.
    if let Some(name) = selection.profile.as_deref().filter(|name| !name.is_empty()) {
        let profile = catalog.get(name)?;
        profile.validate()?;
        if tool.as_deref().filter(|tool| !tool.is_empty()).is_some_and(|tool| tool != profile.tool) {
            bail!("Profile '{name}' uses '{}', which does not match --tool", profile.tool);
        }
    }
    let actor = if flags.role.is_some() { crate::cli::commands::roles::discovery_actor(&scope, invocation)? } else { None };
    let collaboration = actor.as_ref().and_then(|actor| crate::agents::workspaces::collaboration(&actor.record));
    let owner = collaboration.as_ref().and_then(|collaboration| collaboration.role.as_ref());
    let mut environment = invocation.environment.clone();
    if let Some(actor) = &actor {
        environment.insert("AGENTX_WORKSPACE".into(), crate::agents::workspaces::workspace(&actor.record).unwrap_or_else(|| "global".into()));
    }
    let workspace = roles::select_workspace(&invocation.cwd, &environment, flags.dir.as_deref(), flags.workspace.as_deref())?;
    let mut role = flags.role.as_deref().filter(|name| !name.is_empty()).map(|name| roles::read_role(&scope, workspace.as_deref(), name)).transpose()?;
    if flags.role.as_deref() == Some("") {
        let choices = roles::list_roles_for(&scope, workspace.as_deref(), false, owner)?
            .into_iter()
            .map(|role| Choice { value: role.name, label: None, description: format!("{}{}", role.description.as_deref().unwrap_or(""), if role.default { " [directory default]" } else { "" }) })
            .collect::<Vec<_>>();
        if choices.is_empty() {
            bail!("No roles available in this workspace; use agentx roles --help to configure roles");
        }
        let Some(name) = select::select("Select role", &choices)? else { return Ok(false) };
        role = Some(roles::read_role(&scope, workspace.as_deref(), &name)?);
        flags.role = Some(name);
    }
    if selection.profile.as_deref() == Some("") {
        let choices = profile_choices(&catalog, tool.as_deref().filter(|tool| !tool.is_empty()));
        if choices.is_empty() {
            bail!("No matching profiles; use agentx profile create NAME --tool TOOL to configure one");
        }
        let Some(name) = select::select("Select profile", &choices)? else { return Ok(false) };
        selection.profile = Some(name);
    }
    if tool.as_deref() == Some("") {
        let profile = selection.profile.as_deref().or_else(|| role.as_ref().and_then(|role| role.profile.as_deref()));
        let required_tool = profile.map(|name| catalog.get(name).map(|profile| profile.tool.as_str())).transpose()?;
        let choices = tool_choices(required_tool);
        let Some(name) = select::select("Select tool", &choices)? else { return Ok(false) };
        *tool = Some(name);
    }
    Ok(true)
}

fn profile_choices(catalog: &ProfileCatalog, tool: Option<&str>) -> Vec<Choice> {
    catalog
        .profiles
        .iter()
        .filter(|(_, profile)| tool.is_none_or(|tool| profile.tool == tool))
        .map(|(name, profile)| Choice {
            value: name.clone(), label: None, description: format!("{} · {}{}", profile.tool, profile.description.as_deref().unwrap_or(""), if catalog.default_profile.as_ref() == Some(name) { " [default]" } else { "" })
        })
        .collect()
}

fn tool_choices(required: Option<&str>) -> Vec<Choice> {
    crate::integrations::registry::ALL
        .iter()
        .filter(|spec| spec.released && required.is_none_or(|tool| spec.name == tool))
        .map(|spec| Choice { value: spec.name.into(), label: None, description: format!("{} · {}", spec.label, if crate::runtime::terminal::which_bin(spec.cli_binary).is_some() { "installed" } else { "not found in PATH" }) })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn selection_normalization_preserves_controls_and_opaque_values() {
        let input = words(&["--iam", "caller", "l", "--role", "--profile", "--tool", "--dry-run", "--json", "--env-file", ".env", "--", "--role", "--help"]);
        let expected = words(&["--iam", "caller", "l", "--role=", "--profile=", "--tool=", "--dry-run", "--json", "--env-file", ".env", "--", "--role", "--help"]);
        assert_eq!(normalize_args(&input), expected);
        for input in [
            words(&["launch", "--role", "coder", "--profile", "fast", "--tool", "codex"]),
            words(&["launch", "--agentx-prompt", "--role", "--agentx-system-prompt", "--tool"]),
            words(&["resume", "luna", "--role"]),
            words(&["run", "docs", "--role"]),
            words(&["launch", "--", "--profile"]),
        ] {
            assert_eq!(normalize_args(&input), input);
        }
        assert_eq!(normalize_args(&words(&["launch", "--role", "--help"])), words(&["launch", "--role=", "--help"]));
        assert_eq!(normalize_args(&words(&["launch", "--tool"])), words(&["launch", "--tool="]));
    }

    #[test]
    fn selection_profile_choices_follow_explicit_tool() {
        let catalog = ProfileCatalog::parse("[launch]\ndefault_profile = 'fast'\n[launch.profiles.fast]\ntool = 'codex'\ndescription = 'Quick work'\n[launch.profiles.other]\ntool = 'claude'\n").unwrap();
        let choices = profile_choices(&catalog, Some("codex"));
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].value, "fast");
        assert!(choices[0].description.contains("[default]"));
        assert!(profile_choices(&catalog, Some("gemini")).is_empty());
    }
}
