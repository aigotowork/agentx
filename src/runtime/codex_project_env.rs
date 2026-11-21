//! Early project environment preparation for native Codex stdio only.
//! Must run before Config, paths, identity or hooks are initialized.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::cli::commands::launch::{stdio_project_request, with_stdio_project_dir};

// An argv marker survives envrc deletions. It is consumed here, never passed to
// Codex or exported, so nested agentx invocations prepare their own environment.
const REENTRY: &str = "__agentx-stdio-direnv";

/// Return dispatch argv with the internal re-entry marker removed, or replace
/// this process through direnv before any process-wide agentx state is read.
pub(crate) fn prepare(mut argv: Vec<String>) -> Result<Vec<String>> {
    let guard = if argv.first().is_some_and(|arg| arg == REENTRY) {
        if argv.len() < 3 {
            bail!("Invalid native stdio direnv re-entry");
        }
        let project = PathBuf::from(&argv[1]);
        argv.drain(..2);
        Some(project)
    } else {
        None
    };
    let Some(request) = stdio_project_request(&argv)? else {
        if guard.is_some() {
            bail!("Native stdio direnv re-entry requires a Codex stdio launch");
        }
        return Ok(argv);
    };

    let hint = std::env::var_os("PASEO_AGENT_CWD").filter(|value| !value.is_empty());
    let explicit_context = request.dir.is_some() || hint.is_some();
    let project = request.dir.map(PathBuf::from).or_else(|| hint.map(PathBuf::from)).unwrap_or(std::env::current_dir()?);
    if !project.is_dir() {
        bail!("Native stdio project is not a directory: {}", project.display());
    }
    let project = project.canonicalize().context("Cannot canonicalize native stdio project")?;
    let load_env = !request.version || explicit_context;
    let mut envrc = None;
    for dir in project.ancestors().filter(|_| load_env) {
        let file = dir.join(".envrc");
        match file.symlink_metadata() {
            Ok(_) => {
                envrc = Some(file);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Cannot inspect native stdio project .envrc"),
        }
    }

    if let Some(guard) = guard {
        let loaded = std::env::var_os("DIRENV_FILE").map(PathBuf::from);
        if guard != project || std::env::current_dir()?.canonicalize()? != project || !load_env || envrc.is_none() || loaded != envrc {
            bail!("Invalid native stdio direnv project relationship");
        }
    } else if load_env && envrc.is_some() {
        let executable = std::env::current_exe()?.canonicalize()?;
        let direnv = crate::runtime::terminal::which_bin("direnv").context("Native stdio project has .envrc but direnv is not installed or not in PATH")?;
        // Resolve before changing cwd: PATH entries may themselves be relative.
        let direnv = Path::new(&direnv).canonicalize().context("Cannot resolve direnv executable")?;
        let mut command = Command::new(direnv);
        command.arg("exec").arg(&project).arg(executable).arg(REENTRY).arg(&project).args(with_stdio_project_dir(argv, &project)).current_dir(&project).env("PWD", &project);
        return Err(crate::sys::process::exec_replace(command)).context("Cannot enter native stdio project environment");
    }

    std::env::set_current_dir(&project).context("Cannot enter native stdio project")?;
    // No threads exist yet. PWD must also match on the no-env path.
    unsafe { std::env::set_var("PWD", &project) };
    Ok(with_stdio_project_dir(argv, &project))
}
