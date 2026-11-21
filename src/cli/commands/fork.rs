//! Forks an active or stopped instance by launching with --fork-session
//! (Claude), fork subcommand (Codex), --fork (OpenCode), or respective options for Kilo/Pi/OMP.

use anyhow::Result;

use crate::cli::router::GlobalFlags;

/// Run the fork command. `argv` is the full argv[1..].
pub fn run(argv: &[String], flags: &GlobalFlags) -> Result<i32> {
    let (name, extra_args) = match crate::cli::commands::resume::parse_resume_argv(argv, "f") {
        Ok(parsed) => parsed,
        Err(e) => return Ok(crate::cli::commands::help::usage_error("fork", &e.to_string())),
    };
    crate::cli::commands::resume::do_resume(&name, true, &extra_args, flags)
}
