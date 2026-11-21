//! Shared host-side AgentX launch options; CLI presentation stays in cli.
use anyhow::{Result, bail};

/// Agentx-level flags extracted from launch argv.
#[derive(clap::Args, Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct AgentxLaunchFlags {
    /// Initial scalar label key=value; repeat for different keys (fresh launch only).
    #[arg(long = "label")]
    pub labels: Vec<String>,
    /// Bind a Task/Delivery file and include it in the initial message where supported; repeatable, relative to caller cwd.
    #[arg(long = "bind")]
    pub bindings: Vec<std::path::PathBuf>,
    /// Brief individual responsibility (1–160 characters, single line).
    #[arg(long, value_parser = parse_title)]
    pub title: Option<String>,
    /// Terminal host override; tmux creates a managed session and attaches unless detached.
    #[arg(long)]
    pub terminal: Option<String>,
    /// Launch on a remote relay device.
    #[arg(long)]
    pub device: Option<String>,
    /// Run in the configured background host (pty or tmux).
    #[arg(long)]
    pub headless: bool,
    /// Codex only: one local foreground connection on standard streams.
    #[arg(long)]
    pub stdio: bool,
    /// Append to the native system prompt.
    #[arg(long = "agentx-system-prompt", allow_hyphen_values = true)]
    pub system_prompt: Option<String>,
    /// Initial user prompt.
    #[arg(long = "agentx-prompt", allow_hyphen_values = true)]
    pub initial_prompt: Option<String>,
    #[arg(skip)]
    pub run_here: Option<bool>,
    /// Explicit launch batch identifier.
    #[arg(long)]
    pub batch_id: Option<String>,
    /// Working directory for the agent.
    #[arg(long)]
    pub dir: Option<String>,
    /// Adopt a reusable role; omitted role profile uses launch defaults.
    #[arg(long)]
    pub role: Option<String>,
    /// Stable workspace membership; `global` selects no workspace.
    #[arg(long)]
    pub workspace: Option<String>,
    /// `--json`: report the outcome as one JSON object on stdout.
    #[arg(long)]
    pub json: bool,
}

fn parse_title(value: &str) -> Result<String, String> {
    crate::agents::launch_title::normalize(Some(value)).map(|value| value.expect("supplied title"))
}

/// `--json` output is meant to be captured by a script, so the agent must not
/// take over the caller's terminal: open it elsewhere, and reject an explicit
/// request to run here.
pub(crate) fn json_run_here(flags: &AgentxLaunchFlags) -> Result<Option<bool>> {
    if !flags.json {
        return Ok(flags.run_here);
    }
    if flags.run_here == Some(true) || flags.terminal.as_deref() == Some("here") {
        bail!("--json cannot be combined with --run-here or --terminal here");
    }
    Ok(Some(false))
}

/// Shared strict CLI boundary. Native argv is accepted only after `--`.
#[derive(clap::Args, Debug)]
pub(crate) struct LaunchOptions {
    #[command(flatten)]
    pub flags: AgentxLaunchFlags,
    /// Run in the invoking terminal.
    #[arg(long, conflicts_with = "no_run_here")]
    run_here: bool,
    /// Open elsewhere; with tmux, create a detached session.
    #[arg(long)]
    no_run_here: bool,
    /// Existing caller identity (not the new agent's name).
    #[arg(long)]
    iam: Option<String>,
    /// Skip launch preview.
    #[arg(long)]
    go: bool,
    /// Arguments passed verbatim to the native tool, after --.
    #[arg(last = true, allow_hyphen_values = true)]
    pub native_args: Vec<String>,
}
impl LaunchOptions {
    pub fn into_parts(mut self) -> (AgentxLaunchFlags, Vec<String>) {
        self.flags.run_here = if self.run_here {
            Some(true)
        } else if self.no_run_here {
            Some(false)
        } else {
            None
        };
        (self.flags, self.native_args)
    }
}

pub(crate) fn extract_launch_flags(args: &[String]) -> Result<(AgentxLaunchFlags, Vec<String>)> {
    use clap::Parser;
    #[derive(Parser)]
    #[command(name = "agentx", after_help = "Native tool arguments must follow --.")]
    struct Arguments {
        #[command(flatten)]
        options: LaunchOptions,
    }
    let (cleaned, _) = super::environment::extract_options(args)?;
    let parsed = Arguments::try_parse_from(std::iter::once("agentx".to_owned()).chain(cleaned))?;
    Ok(parsed.options.into_parts())
}

#[cfg(test)]
mod title_tests {
    use super::*;
    #[test]
    fn title_is_an_agentx_option_before_native_separator() {
        let words = |values: &[&str]| values.iter().map(|value| (*value).to_owned()).collect::<Vec<_>>();
        let (flags, native) = extract_launch_flags(&words(&["--title", " API owner ", "--", "--title", "provider title"])).unwrap();
        assert_eq!(flags.title.as_deref(), Some("API owner"));
        assert_eq!(native, words(&["--title", "provider title"]));
        assert!(extract_launch_flags(&words(&["--title", " "])).is_err());
        assert_eq!(extract_launch_flags(&[]).unwrap().0.title, None);
    }
}
