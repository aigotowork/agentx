//! Local role inspection and explicit native endpoint-binding management.
//! Only `reset-endpoint` acquires a daemon; discovery probes an existing daemon for adopted role data.
use crate::agents::launch_service;
use crate::config::roles::{self, RoleDefinition};
use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Parser)]
#[command(
    name = "agentx roles",
    about = "Inspect role definitions or reset a retired workspace endpoint",
    after_help = r#"Role files:
  Published: <workspace>/.agent-guides/roles/NAME.md or OWNER.HELPER.md
  Overrides: <workspace>/.agentx/roles/NAME.toml
  Global: $AGENTX_DIR/roles/NAME.toml (default: ~/.agentx/roles/NAME.toml)
  Markdown requires YAML frontmatter schema: role; optional role must match the filename.
  Only immediate role files are discovered; reference Markdown without schema: role is ignored.
  Short names resolve the selected workspace, parents, then global.
  Workspace: explicit --workspace > explicit --dir project > AGENTX_WORKSPACE > cwd project > ~/.agentx.
  Project discovery searches nearest Git root (.git directory or worktree file), then .agentx/,
  then the existing .agent-guides/roles boundary. A target without a root continues to inheritance.
  Bound Agents inherit persisted membership; environment values only carry the default to children.
  --dir sets cwd; omitted --dir preserves caller cwd, independently of workspace selection.
  Within one workspace, TOML overrides Markdown; a nearer Markdown beats a farther TOML.
  local:NAME selects only the current workspace; global:NAME selects global TOML definitions.
  OWNER.HELPER is auxiliary; default discovery shows public roles plus your adopted owner's helpers
  from the same logical workspace, including mixed Markdown/TOML sources. --all shows shadowed
  roles and all helpers. Explicit show/launch is unrestricted. Disabled names never fall through.
  Identity discovery probes an existing daemon only; unavailable implicit identity shows public roles.
Setup:
  Install role Markdown into .agent-guides/roles/; no wrapper is required for ordinary roles.
  agentx profile create team-codex --tool codex
  agentx profile default team-codex
  agentx launch --role coder
  Optional .agentx/roles/coder.toml: profile = "team-codex"
  Optional .agentx/roles/controller.toml: default = true
  Thin TOML inherits only the same-workspace Markdown prompt and description.
  Explicit prompt or prompt_file replaces the Markdown prompt; use at most one.
  Without matching Markdown, one prompt field is required. prompt_file is relative to TOML.
  File prompts retain their absolute source path and unchanged body, with parent-relative references.
  Omitted profile uses ordinary launch defaults; --profile overrides the role's explicit profile.
  Listing/show needs no default profile; role launches require a concrete named profile.
  default = true chooses a directory recipient, independently of the default launch profile.
  At most one default per namespace; auxiliaries cannot be default recipients.
  To suppress a role including ancestor/global fallback: NAME.toml containing only disabled = true.
Examples:
  agentx roles --dir /path/project
  agentx roles show controller --dir /path/project --json
  agentx launch --dir /path/project --role controller --headless --go
  agentx launch --workspace /path/project --role global:reviewer --headless --go
  agentx list --dir /path/project
  agentx list --all
  agentx send --dir /path/project --dry-run
  agentx send --dir /path/project --intent request -- "Handle this task"
Each launch creates a new member. Directory send reuses or starts one default recipient;
parent fallback uses the parent's endpoint and preserves the originally requested directory.
Role/profile/instruction snapshots survive resume and fork; editing files affects new agents.
Role/workspace launches are local; shared Codex app-server stdio is unsupported.
More reference: agentx docs --cli"#
)]
struct Arguments {
    #[arg(long, global = true, value_name = "PATH")]
    /// Discover the workspace containing this directory.
    dir: Option<String>,
    #[arg(long, global = true, value_name = "PATH|global")]
    /// Select an exact workspace, or the global context.
    workspace: Option<String>,
    #[arg(long, global = true)]
    /// Include auxiliary roles and shadowed ancestor/global definitions.
    all: bool,
    #[arg(long, global = true)]
    /// Emit workspace and role metadata as JSON.
    json: bool,
    #[arg(long = "iam", global = true, value_name = "NAME")]
    /// Inspect discovery as this existing Agent; normally identity is automatic.
    caller: Option<String>,
    #[command(subcommand)]
    command: Option<Operation>,
}

#[derive(Subcommand)]
enum Operation {
    /// Inspect one resolved role, including its literal prompt.
    Show { name: String },
    /// Clear the selected workspace's endpoint binding after exact retirement.
    #[command(
        long_about = "Clear the selected workspace's endpoint binding so its default role can be changed. The daemon rejects live or uncertain recipients. This operation connects to the daemon; show stays local and list only probes an existing daemon."
    )]
    ResetEndpoint,
}

pub(crate) fn help_command() -> clap::Command {
    Arguments::command()
}

pub(crate) fn help_text() -> String {
    Arguments::command().render_long_help().to_string()
}

pub(crate) fn run(argv: &[String]) -> Result<i32> {
    let args = match Arguments::try_parse_from(argv) {
        Ok(args) => args,
        Err(error) => {
            let code = error.exit_code();
            crate::cli::commands::help::print_clap_error(&error)?;
            return Ok(code);
        }
    };
    if args.all && matches!(args.command, Some(Operation::ResetEndpoint)) {
        bail!("reset-endpoint selects one workspace; --all is only for role inspection");
    }
    let mut invocation = crate::service::InvocationContext::capture()?;
    invocation.explicit_name = args.caller.clone();
    let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
    let actor = if args.command.is_none() && !args.all { discovery_actor(&scope, &invocation)? } else { None };
    let mut environment = invocation.environment.clone();
    if let Some(actor) = &actor {
        environment.insert("AGENTX_WORKSPACE".into(), crate::agents::workspaces::workspace(&actor.record).unwrap_or_else(|| "global".into()));
    }
    let workspace = roles::select_workspace(&invocation.cwd, &environment, args.dir.as_deref(), args.workspace.as_deref())?;
    match args.command {
        Some(Operation::ResetEndpoint) => {
            let client = crate::service::client::Client::connect(&scope, invocation)?;
            let selected = workspace.as_ref().map(|path| path.to_string_lossy().into_owned());
            let request = crate::service::Request::Participants(crate::agents::service::Request::Launch(launch_service::Request::EndpointReset { workspace: selected }));
            match client.call(request)? {
                crate::service::Response::Participants(crate::agents::service::Response::Launch(launch_service::Response::Changed)) => {}
                _ => bail!("Daemon returned a different endpoint reset response"),
            }
            if args.json {
                println!("{}", serde_json::json!({"workspace": workspace, "endpoint_reset": true}));
            } else {
                let workspace = workspace.as_ref().map(|path| path.display().to_string()).unwrap_or_else(|| "global".into());
                println!("Reset endpoint binding for workspace {workspace}.");
            }
        }
        Some(Operation::Show { name }) => {
            let role = roles::read_role(&scope, workspace.as_deref(), &name)?;
            if args.json {
                println!("{}", serde_json::json!({"workspace": workspace, "role": role}));
            } else {
                println!(
                    "Role: {}\nProfile: {}\nNamespace: {}\nPath: {}\nDefault: {}\nDescription: {}\nPrompt:\n{}",
                    role.name,
                    role.profile.as_deref().unwrap_or("(launch default)"),
                    role.namespace,
                    role.path.display(),
                    role.default,
                    role.description.as_deref().unwrap_or(""),
                    role.prompt
                );
            }
        }
        None => {
            let owner = actor.as_ref().and_then(|actor| crate::agents::workspaces::collaboration(&actor.record)).and_then(|snapshot| snapshot.role);
            let roles = views(roles::list_roles_for(&scope, workspace.as_deref(), args.all, owner.as_ref())?);
            if args.json {
                println!("{}", serde_json::json!({"workspace": workspace, "roles": roles}));
            } else if roles.is_empty() {
                println!("No roles. Define <workspace>/.agentx/roles/NAME.toml or {}/roles/NAME.toml.", scope.display());
            } else {
                for view in roles {
                    let role = view.role;
                    let default = if role.default { " [default]" } else { "" };
                    let shadowed = if view.shadowed { " [shadowed]" } else { "" };
                    let auxiliary = role.name.split_once('.').map(|(owner, _)| format!(" [helper of {owner}]")).unwrap_or_default();
                    println!("{}\t{}\t{}{}{}{}\t{}", role.name, role.profile.as_deref().unwrap_or("(launch default)"), role.namespace, default, shadowed, auxiliary, role.description.as_deref().unwrap_or(""));
                }
            }
        }
    }
    Ok(0)
}

/// Read adopted facts through the native resolver without acquiring a daemon or
/// binding sessions. Missing implicit identity degrades to public discovery.
pub(crate) fn discovery_actor(scope: &std::path::Path, invocation: &crate::service::InvocationContext) -> Result<Option<crate::agents::service::ResolvedActor>> {
    use crate::agents::service::{Request, Response};
    use crate::service::{Request as NativeRequest, Response as NativeResponse, ServiceError};
    let runtime = crate::runtime::context::AgentxContext::from_env(&invocation.environment, invocation.cwd.clone());
    let shared = crate::integrations::codex::roots::is_shared_context(&runtime);
    let has_claim = invocation.explicit_name.is_some() || invocation.process_binding_id.is_some() || invocation.environment.contains_key("AGENTX_CLAUDE_ACTOR") || shared;
    if !has_claim {
        return Ok(None);
    }
    let resolve = || -> crate::service::ServiceResult<_> {
        let mut invocation = invocation.clone();
        let client = crate::service::client::Client::connect_existing(scope, invocation.clone())?;
        if shared {
            let store = crate::agents::service::RemoteAttachments::new(client.clone());
            let attachment = crate::integrations::codex::roots::resolve_shared_attachment(&store, &runtime, runtime.codex_thread_id.as_deref().unwrap_or(""))
                .map_err(|error| ServiceError::IdentityRequired(error.to_string()))?
                .ok_or_else(|| ServiceError::IdentityRequired("No current attachment for this provider session.".into()))?;
            invocation.activation_target = Some(attachment.target);
        }
        match client.with_invocation(invocation).call(NativeRequest::Participants(Request::Resolve))? {
            NativeResponse::Participants(Response::Actor(actor)) => Ok(actor),
            _ => Err(ServiceError::Internal("daemon returned a different participant response".into())),
        }
    };
    match resolve() {
        Ok(actor) => Ok(Some(actor)),
        Err(error) if invocation.explicit_name.is_some() => Err(error.into()),
        Err(error) => {
            eprintln!("[agentx] Cannot resolve role discovery identity; showing public roles: {error}");
            Ok(None)
        }
    }
}

#[derive(Serialize)]
struct RoleView {
    #[serde(flatten)]
    role: RoleDefinition,
    shadowed: bool,
}

fn views(roles: Vec<RoleDefinition>) -> Vec<RoleView> {
    let mut seen = HashSet::new();
    roles.into_iter().map(|role| RoleView { shadowed: !seen.insert(role.name.clone()), role }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspection_options_work_before_and_after_show() {
        for argv in [vec!["roles", "--workspace", "global", "--json", "show", "global:reviewer"], vec!["roles", "show", "global:reviewer", "--workspace", "global", "--json"]] {
            let parsed = Arguments::try_parse_from(argv).unwrap();
            assert_eq!(parsed.workspace.as_deref(), Some("global"));
            assert!(parsed.json);
            assert!(matches!(parsed.command, Some(Operation::Show { name }) if name == "global:reviewer"));
        }
        assert!(Arguments::try_parse_from(["roles", "--all", "--dir", "../other"]).unwrap().all);
        assert_eq!(Arguments::try_parse_from(["roles", "--iam", "rosa"]).unwrap().caller.as_deref(), Some("rosa"));
        assert!(Arguments::try_parse_from(["roles", "show", "fronter.e2e"]).is_ok());
        assert!(Arguments::try_parse_from(["roles", "--unknown"]).is_err());
        assert!(Arguments::try_parse_from(["roles", "show"]).is_err());
        assert_eq!(Arguments::try_parse_from(["roles", "--help"]).err().unwrap().kind(), clap::error::ErrorKind::DisplayHelp);
    }

    #[test]
    fn all_json_rows_mark_farther_definitions_as_shadowed() {
        let local = RoleDefinition { name: "reviewer".into(), path: "/a/.agentx/roles/reviewer.toml".into(), namespace: "local:/a/.agentx/roles".into(), profile: Some("deep".into()), prompt: "local".into(), description: None, default: false };
        let global = RoleDefinition { path: "/scope/roles/reviewer.toml".into(), namespace: "global:/scope/roles".into(), prompt: "global".into(), ..local.clone() };
        let rows = serde_json::to_value(views(vec![local, global])).unwrap();
        assert_eq!(rows[0]["shadowed"], false);
        assert_eq!(rows[1]["shadowed"], true);
        assert_eq!(rows[1]["namespace"], "global:/scope/roles");
    }

    #[test]
    fn reset_endpoint_selects_one_workspace_and_rejects_all_before_acquisition() {
        for argv in [vec!["roles", "reset-endpoint", "--dir", "../other"], vec!["roles", "--workspace", "global", "reset-endpoint"]] {
            let parsed = Arguments::try_parse_from(argv).unwrap();
            assert!(matches!(parsed.command, Some(Operation::ResetEndpoint)));
            assert!(parsed.dir.is_some() || parsed.workspace.is_some());
        }
        assert!(Arguments::try_parse_from(["roles", "reset-endpoint", "reviewer"]).is_err());
        let args = ["roles", "reset-endpoint", "--all"].map(str::to_owned);
        assert!(run(&args).unwrap_err().to_string().contains("one workspace"));
        let help = Arguments::command().find_subcommand_mut("reset-endpoint").unwrap().render_long_help().to_string();
        assert!(help.contains("live or uncertain"));
    }
}
