//! Local profile management: explicit scoped files, no Agent identity or daemon.
use crate::config::profiles::{self, Edit, LaunchProfile, ProfileCatalog};
use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "agentx profile", about = "Manage reusable tool launch configurations")]
struct Arguments {
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    /// List saved profiles and default selections.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Show one saved definition (not a launch's effective configuration).
    Show {
        name: String,
        #[arg(long)]
        json: bool,
    },
    /// Create a profile; native args must follow --.
    Create {
        name: String,
        #[arg(long)]
        tool: String,
        #[arg(long, allow_hyphen_values = true)]
        description: Option<String>,
        #[arg(last = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Update metadata, or replace the entire saved argv with --replace-args.
    Update {
        name: String,
        #[arg(long, allow_hyphen_values = true)]
        description: Option<String>,
        #[arg(long)]
        replace_args: bool,
        #[arg(last = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Delete a profile that is not selected as a default.
    Delete { name: String },
    /// Get, set or clear the scope default (or --tool TOOL default).
    Default {
        #[arg(conflicts_with = "unset")]
        name: Option<String>,
        #[arg(long)]
        unset: bool,
        #[arg(long)]
        tool: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn help_command() -> clap::Command {
    Arguments::command()
}

pub(crate) fn help_text() -> String {
    Arguments::command().render_long_help().to_string()
}

/// Dispatch before ordinary router setup; parser owns child help and failures.
pub(crate) fn dispatch(argv: &[String]) -> Option<i32> {
    let (cleaned, _, _) = crate::cli::router::extract_global_flags_full(argv);
    if cleaned.first().map(String::as_str) != Some("profile") {
        return None;
    }
    // Keep native clap help instead of the outer scanner's stripped --help.
    let (cleaned, _) = match crate::runtime::environment::extract_options(argv) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Error: {error:#}");
            return Some(2);
        }
    };
    let start = cleaned.iter().position(|word| word == "profile").unwrap_or(0);
    let parsed = match Arguments::try_parse_from(&cleaned[start..]) {
        Ok(value) => value,
        Err(error) => {
            let code = error.exit_code();
            let _ = crate::cli::commands::help::print_clap_error(&error);
            return Some(code);
        }
    };
    Some(match run(parsed.command) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("Error: {error:#}");
            1
        }
    })
}

fn run(operation: Operation) -> Result<()> {
    let invocation = crate::service::InvocationContext::capture()?;
    let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
    match operation {
        Operation::List { json } => {
            let catalog = ProfileCatalog::load(&scope)?;
            if json {
                println!("{}", serde_json::to_string(&catalog)?);
            } else {
                for (name, profile) in &catalog.profiles {
                    let global = if catalog.default_profile.as_deref() == Some(name) { " [default]" } else { "" };
                    let tool_default = if catalog.tool_defaults.get(&profile.tool) == Some(name) { " [tool default]" } else { "" };
                    println!("{name}\t{}\t{}{}{}", profile.tool, profile.description.as_deref().unwrap_or(""), global, tool_default);
                }
                if catalog.profiles.is_empty() {
                    println!("No profiles. Use agentx profile create NAME --tool TOOL -- native-args...");
                }
            }
        }
        Operation::Show { name, json } => {
            let catalog = ProfileCatalog::load(&scope)?;
            let profile = catalog.get(&name)?;
            if json {
                println!("{}", serde_json::json!({"name": name, "definition": profile}));
            } else {
                println!("Profile: {name}\nTool: {}\nDescription: {}\nArgs: {}", profile.tool, profile.description.as_deref().unwrap_or(""), serde_json::to_string(&profile.args)?);
            }
        }
        Operation::Create { name, tool, description, args } => profiles::edit(&scope, Edit::Create { name, profile: LaunchProfile { tool, description: description.filter(|v| !v.is_empty()), args } })?,
        Operation::Update { name, description, replace_args, args } => {
            if !replace_args && !args.is_empty() {
                bail!("profile update native args require --replace-args");
            }
            profiles::edit(&scope, Edit::Update { name, description, args: replace_args.then_some(args) })?;
        }
        Operation::Delete { name } => profiles::edit(&scope, Edit::Delete { name })?,
        Operation::Default { name, unset, tool, json } => {
            let tool = tool.as_deref().map(profiles::canonical_tool).transpose()?;
            if name.is_some() || unset {
                profiles::edit(&scope, Edit::Default { tool: tool.clone(), name })?;
            }
            let catalog = ProfileCatalog::load(&scope)?;
            let name = tool.as_ref().map(|tool| catalog.tool_defaults.get(tool)).unwrap_or(catalog.default_profile.as_ref());
            if json {
                println!("{}", serde_json::json!({"tool": tool, "profile": name}));
            } else {
                println!("{}", name.map(String::as_str).unwrap_or("(unset)"));
            }
        }
    }
    Ok(())
}
