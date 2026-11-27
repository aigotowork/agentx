//! `agentx run` command — execute scripts from embedded bundled scripts and ~/.agentx/scripts/.
//!
//! Bundled scripts are compiled into the binary via `scripts::SCRIPTS`.
//! User scripts in `~/.agentx/scripts/` still discovered from disk and shadow bundled.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::commands::help;
use crate::paths::scripts_dir;
use crate::workflows as scripts;

#[cfg(windows)]
fn resolve_git_bash() -> Result<String, String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    find_git_bash(&path, |bash| {
        // WSL's launcher can successfully run Bash once a distro is installed,
        // but that Bash cannot open the Windows paths passed to our scripts.
        Command::new(bash).args(["--noprofile", "--norc", "-c", "case \"$OSTYPE\" in msys*|cygwin*) exit 0 ;; *) exit 1 ;; esac"]).output().is_ok_and(|output| output.status.success())
    })
    .map(|bash| bash.to_string_lossy().into_owned())
    .ok_or_else(|| format!("{BASH_MISSING_MSG}\nThe Windows WSL launcher is not sufficient."))
}

#[cfg(windows)]
fn find_git_bash(path: &std::ffi::OsStr, is_git_bash: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    std::env::split_paths(path).map(|dir| dir.join("bash.exe")).find(|bash| bash.is_file() && is_git_bash(bash))
}
#[cfg(windows)]
const BASH_MISSING_MSG: &str = "Git Bash required to run shell (.sh) workflow scripts — install it and ensure `bash` is on PATH.";

fn python_command() -> Result<Command, String> {
    if let Ok(program) = std::env::var("PYTHON")
        && !program.trim().is_empty()
    {
        return Ok(Command::new(program));
    }

    #[cfg(windows)]
    {
        for (name, args) in [("python", &[][..]), ("py", &["-3"][..])] {
            let Some(program) = crate::runtime::terminal::which_bin(name) else {
                continue;
            };
            let mut probe = Command::new(&program);
            probe.args(args).arg("--version");
            if probe.output().is_ok_and(|output| output.status.success()) {
                let mut command = Command::new(program);
                command.args(args);
                return Ok(command);
            }
        }
        Err("Python 3 is required to run .py workflow scripts. Install Python, set `PYTHON`, \
             or ensure `python`/`py` is on PATH."
            .to_string())
    }
    #[cfg(not(windows))]
    {
        Ok(Command::new("python3"))
    }
}

#[derive(clap::Parser, Debug)]
#[command(name = "run", about = "Run a bundled or user workflow script", disable_help_flag = true)]
pub struct RunArgs {
    /// Script name plus forwarded args
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

/// Bundled script agent descriptions.
fn bundled_agent_desc(name: &str) -> &'static str {
    match name {
        "confess" => "3 agents",
        "debate" => "2+ agents",
        "fatcow" => "1 agent",
        _ => "",
    }
}

/// Script info.
enum ScriptSource {
    /// Embedded bundled script — content is in memory.
    Bundled { content: &'static str },
    /// User script on disk.
    User { path: PathBuf },
}

struct ScriptInfo {
    name: String,
    source: ScriptSource,
    description: String,
}

/// Extract first comment line as description from shell script content.
fn extract_description_from_content(content: &str) -> String {
    for line in content.lines() {
        let stripped = line.trim();
        if stripped.starts_with("#!") {
            continue;
        }
        if stripped.starts_with('#') {
            return stripped.strip_prefix('#').unwrap_or(stripped).trim().to_string();
        }
        if !stripped.is_empty() {
            break;
        }
    }
    String::new()
}

/// Extract first line of docstring (Python) or comment (shell) as description from a file.
fn extract_description(path: &Path) -> String {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "py" => {
            let mut in_docstring = false;
            for line in content.lines() {
                let stripped = line.trim();
                if stripped.starts_with("\"\"\"") || stripped.starts_with("'''") {
                    let quote = &stripped[..3];
                    if stripped.matches(quote).count() >= 2 {
                        return stripped.trim_matches(['"', '\'']).trim().to_string();
                    }
                    in_docstring = true;
                    let rest = stripped[3..].trim();
                    if !rest.is_empty() {
                        return rest.trim_end_matches(['"', '\'']).trim().to_string();
                    }
                } else if in_docstring {
                    if stripped.ends_with("\"\"\"") || stripped.ends_with("'''") {
                        return stripped.trim_end_matches(['"', '\'']).trim().to_string();
                    }
                    if !stripped.is_empty() {
                        return stripped.to_string();
                    }
                }
            }
            String::new()
        }
        "sh" => extract_description_from_content(&content),
        _ => String::new(),
    }
}

/// Discover all available scripts (user scripts shadow bundled).
fn discover_scripts() -> Vec<ScriptInfo> {
    let user_dir = scripts_dir();
    let mut scripts = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // User scripts first (they shadow bundled)
    if user_dir.exists() {
        for ext in &["py", "sh"] {
            if let Ok(rd) = std::fs::read_dir(&user_dir) {
                let mut entries: Vec<PathBuf> =
                    rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().and_then(|e| e.to_str()) == Some(ext) && !p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('_') || n.starts_with('.'))).collect();
                entries.sort();

                for path in entries {
                    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.is_empty() && !seen.contains(name) {
                        seen.insert(name.to_string());
                        let desc = extract_description(&path);
                        scripts.push(ScriptInfo { name: name.to_string(), source: ScriptSource::User { path }, description: desc });
                    }
                }
            }
        }
    }

    // Bundled scripts from embedded const (if not shadowed by user)
    for (name, content) in scripts::SCRIPTS {
        if !seen.contains(*name) {
            seen.insert(name.to_string());
            let desc = extract_description_from_content(content);
            scripts.push(ScriptInfo { name: name.to_string(), source: ScriptSource::Bundled { content }, description: desc });
        }
    }

    scripts
}

/// Find a script by name.
fn find_script(name: &str) -> Option<ScriptInfo> {
    discover_scripts().into_iter().find(|s| s.name == name)
}

/// List all available scripts.
fn list_scripts() -> i32 {
    let scripts = discover_scripts();

    let bundled: Vec<&ScriptInfo> = scripts.iter().filter(|s| matches!(s.source, ScriptSource::Bundled { .. })).collect();
    let user: Vec<&ScriptInfo> = scripts.iter().filter(|s| matches!(s.source, ScriptSource::User { .. })).collect();

    if scripts.is_empty() {
        println!("No scripts available");
        return 0;
    }

    if !bundled.is_empty() {
        println!("Bundled Scripts:");
        println!();
        for s in &bundled {
            let agents = bundled_agent_desc(&s.name);
            let agents_part = if agents.is_empty() { String::new() } else { format!("  ({agents})") };
            println!("  {}{agents_part}", s.name);
            println!("      {}", s.description);
        }
        println!();
    }

    if !user.is_empty() {
        println!("User Scripts:");
        println!();
        for s in &user {
            println!("  {}", s.name);
            println!("      {}", s.description);
        }
        println!();
    } else {
        println!("User Scripts:");
        println!();
        println!("  No custom scripts yet.");
        println!();
        println!("  Run 'agentx docs --scripts' to create a custom script:");
        println!("    - Multi-agent workflows");
        println!("    - Background watchers");
        println!("    - Task automation");
        println!("    - etc...");
        println!();
    }

    println!("Commands:");
    println!("  agentx run <script>           Run workflow script");
    println!("  agentx run <script> --source  View script source");
    println!("  agentx run <script> --help    Script help");
    println!("  agentx docs --scripts        Script creation guide");

    0
}

/// Write embedded script content to a temp file and return the path.
fn write_embedded_to_temp(name: &str, content: &str) -> std::io::Result<tempfile::NamedTempFile> {
    let mut tmp = tempfile::Builder::new().prefix(&format!("agentx-{name}-")).suffix(".sh").tempfile()?;
    tmp.write_all(content.as_bytes())?;
    tmp.flush()?;
    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = tmp.as_file().metadata()?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o755);
        tmp.as_file().set_permissions(perms)?;
    }
    Ok(tmp)
}

pub fn cmd_run(args: &RunArgs, explicit_name: Option<&str>) -> i32 {
    let mut argv = args.args.clone();
    if let Some(name) = explicit_name {
        argv = vec!["--iam".to_string(), name.to_string()].into_iter().chain(argv).collect();
    }

    if argv.is_empty() {
        return list_scripts();
    }

    // `run` disables clap's built-in help flag so script-specific `--help`
    // can be forwarded. Handle help here when there is no script name.
    let run_help_only = {
        let mut saw_help = false;
        let mut only_help = true;
        let mut i = 0;
        while i < argv.len() {
            if argv[i] == "--iam" && i + 1 < argv.len() {
                i += 2;
            } else if argv[i] == "-h" || argv[i] == "--help" {
                saw_help = true;
                i += 1;
            } else {
                only_help = false;
                break;
            }
        }
        saw_help && only_help
    };
    if run_help_only {
        println!("{}", help::get_command_help("run"));
        return 0;
    }

    // Handle --source flag
    let show_source = argv.iter().any(|a| a == "--source");
    let argv: Vec<String> = argv.into_iter().filter(|a| a != "--source").collect();

    // Find script name (skip --iam flag and value)
    let mut script_idx = None;
    let mut i = 0;
    while i < argv.len() {
        if argv[i] == "--iam" && i + 1 < argv.len() {
            i += 2;
        } else if argv[i].starts_with('-') {
            i += 1;
        } else {
            script_idx = Some(i);
            break;
        }
    }

    let script_idx = match script_idx {
        Some(idx) => idx,
        None => return list_scripts(),
    };

    let name = &argv[script_idx];
    let mut args: Vec<String> = argv[..script_idx].to_vec();
    args.extend(argv[script_idx + 1..].to_vec());

    // Find script
    let script = match find_script(name) {
        Some(s) => s,
        None => {
            println!("Unknown script: {name}");
            println!("Run 'agentx run' to list available scripts");
            return 1;
        }
    };

    // --source: print source and exit
    if show_source {
        match &script.source {
            ScriptSource::Bundled { content } => {
                print!("{content}");
                return 0;
            }
            ScriptSource::User { path } => match std::fs::read_to_string(path) {
                Ok(content) => {
                    print!("{content}");
                    return 0;
                }
                Err(e) => {
                    eprintln!("Error reading {}: {e}", path.display());
                    return 1;
                }
            },
        }
    }

    // Resolve an explicitly selected participant only for actual execution.
    // Discovery, source, docs and script help remain local operations.
    if let Some(name) = explicit_name
        && !args.iter().any(|arg| matches!(arg.as_str(), "--help" | "-h"))
    {
        let result = crate::service::InvocationContext::capture()
            .and_then(|mut invocation| {
                invocation.explicit_name = Some(name.to_string());
                let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
                crate::service::client::Client::connect(scope, invocation)
            })
            .and_then(|client| client.call(crate::service::Request::Participants(crate::agents::service::Request::Resolve)));
        match result {
            Ok(crate::service::Response::Participants(crate::agents::service::Response::Actor(actor))) => {
                if args.first().map(String::as_str) == Some("--iam") && args.len() >= 2 {
                    args[1] = actor.participant.canonical_name;
                }
            }
            Err(crate::service::ServiceError::NotFound(_)) => {}
            Err(error) => {
                eprintln!("Error: {error}");
                return 1;
            }
            _ => {
                eprintln!("Error: daemon returned a different participant response");
                return 1;
            }
        }
    }

    // Run the script
    match &script.source {
        ScriptSource::User { path } => {
            let mut cmd = if path.extension().and_then(|e| e.to_str()) == Some("py") {
                let mut c = match python_command() {
                    Ok(command) => command,
                    Err(err) => {
                        eprintln!("{err}");
                        return 1;
                    }
                };
                c.arg(path);
                c.args(&args);
                c
            } else {
                #[cfg(windows)]
                let bash = match resolve_git_bash() {
                    Ok(bash) => bash,
                    Err(err) => {
                        eprintln!("{err}");
                        return 1;
                    }
                };
                #[cfg(not(windows))]
                let bash = "bash";
                let mut c = Command::new(bash);
                c.arg(path);
                c.args(&args);
                c
            };

            match cmd.stdin(std::process::Stdio::inherit()).stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit()).status() {
                Ok(status) => status.code().unwrap_or(1),
                Err(e) => {
                    eprintln!("Error running {}: {e}", path.display());
                    1
                }
            }
        }
        ScriptSource::Bundled { content } => {
            // Write to temp file and execute
            let tmp = match write_embedded_to_temp(name, content) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Error creating temp file for {name}: {e}");
                    return 1;
                }
            };

            #[cfg(windows)]
            let bash = match resolve_git_bash() {
                Ok(bash) => bash,
                Err(err) => {
                    eprintln!("{err}");
                    return 1;
                }
            };
            #[cfg(not(windows))]
            let bash = "bash";
            let mut cmd = Command::new(bash);
            cmd.arg(tmp.path());
            cmd.args(&args);

            match cmd.stdin(std::process::Stdio::inherit()).stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit()).status() {
                Ok(status) => status.code().unwrap_or(1),
                Err(e) => {
                    eprintln!("Error running {name}: {e}");
                    1
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[cfg(windows)]
    #[test]
    fn git_bash_search_skips_wsl_and_checks_later_path_entries() {
        let dir = tempfile::tempdir().unwrap();
        let wsl = dir.path().join("system32");
        let git = dir.path().join("Git with spaces").join("bin");
        for candidate in [&wsl, &git] {
            std::fs::create_dir_all(candidate).unwrap();
            std::fs::write(candidate.join("bash.exe"), "").unwrap();
        }
        let path = std::env::join_paths([&wsl, &git]).unwrap();
        assert_eq!(find_git_bash(&path, |bash| bash == git.join("bash.exe")), Some(git.join("bash.exe")));
        assert!(find_git_bash(&path, |_| false).is_none());
    }

    #[test]
    fn run_args_capture_script_and_flags() {
        let args = RunArgs::try_parse_from(["run", "debate", "--topic", "hooks"]).unwrap();
        assert_eq!(args.args, vec!["debate", "--topic", "hooks"]);
    }

    #[test]
    fn test_extract_description_python() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.py");
        std::fs::write(&path, "\"\"\"My cool script.\"\"\"\nimport sys\n").unwrap();
        assert_eq!(extract_description(&path), "My cool script.");
    }

    #[test]
    fn test_extract_description_shell() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sh");
        std::fs::write(&path, "#!/bin/bash\n# A shell script\necho hi\n").unwrap();
        assert_eq!(extract_description(&path), "A shell script");
    }

    #[test]
    fn test_extract_description_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.py");
        std::fs::write(&path, "import sys\n").unwrap();
        assert_eq!(extract_description(&path), "");
    }

    #[test]
    fn test_bundled_agent_desc() {
        assert_eq!(bundled_agent_desc("confess"), "3 agents");
        assert_eq!(bundled_agent_desc("unknown"), "");
    }

    #[test]
    fn test_embedded_scripts_available() {
        assert_eq!(scripts::SCRIPTS.len(), 4);
        let names: Vec<&str> = scripts::SCRIPTS.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"confess"));
        assert!(names.contains(&"debate"));
        assert!(names.contains(&"fatcow"));
        assert!(names.contains(&"onidle"));
    }

    #[test]
    fn test_extract_description_from_embedded() {
        // Verify we can extract descriptions from embedded content
        for (name, content) in scripts::SCRIPTS {
            let desc = extract_description_from_content(content);
            assert!(!desc.is_empty(), "Bundled script '{name}' should have a description comment");
        }
    }
}
