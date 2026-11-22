//! Direct non-app-server stdio checks using an isolated fake Codex, never a model.
//! Shared app-server composition is covered by codex_shared_runtime.py.
#![cfg(unix)]

mod support;

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use support::Agentx;

struct FakeCodex {
    agentx: Agentx,
    bin_dir: std::path::PathBuf,
    capture: std::path::PathBuf,
}

impl FakeCodex {
    fn new() -> Self {
        let agentx = Agentx::new();
        let bin_dir = agentx.root_path().join("fake-bin");
        let capture = agentx.root_path().join("capture");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&capture).unwrap();
        let hooks: Vec<_> = ["sessionstart", "userpromptsubmit", "pretooluse", "posttooluse", "premodelrequest", "postmodelrequest", "stop", "interrupt"]
            .iter()
            .enumerate()
            .map(|(i, hook)| serde_json::json!({"key": format!("fake-hook-{i}"), "command": format!("agentx codex-{hook}"), "source": "sessionFlags", "currentHash": format!("fake-hash-{i}")}))
            .collect();
        fs::write(capture.join("hook-list.json"), serde_json::to_vec(&hooks).unwrap()).unwrap();
        let codex = bin_dir.join("codex");
        fs::write(
            &codex,
            r#"#!/bin/sh
set -eu
# The existing hook trust preflight has private pipes and starts with these
# overrides. Answer only its initialize/hooks-list exchange without a model.
if [ "${1-}" = -c ] && [ "${2-}" = features.hooks=true ]; then
    printf '%s\0' "$0" "$CODEX_HOME" "$PWD" > "$CAPTURE/preflight-env"
    IFS= read -r initialize
    printf '{"id":1,"result":{}}\n'
    IFS= read -r initialized
    IFS= read -r list
    if [ "${FAKE_BREAK_LAUNCH-}" = 1 ]; then
        printf '#!/nonexistent/agentx-test-interpreter\n' > "$0"
    fi
    printf '{"id":2,"result":{"data":[{"hooks":'
    cat "$CAPTURE/hook-list.json"
    printf '}]}}\n'
    exit 0
fi
printf '%s\0' "$@" > "$CAPTURE/args"
printf '%s\0' "$AMBIENT" "$OVERRIDE" "$CODEX_HOME" "${AGENTX_PROCESS_ID-}" "${AGENTX_INSTANCE_NAME-}" "${AGENTX_TOOL-}" "${AGENTX_LAUNCHED-}" "${CODEX_EXEC_SERVER_URL-unset}" "${AGENTX_PTY_MODE-unset}" "${AGENTX_BACKGROUND-unset}" "${INJECT_PORT-unset}" "${CODEX_THREAD_ID-unset}" > "$CAPTURE/env"
printf '%s\0' "${REMOVED-unset}" "${HCOM_LITERAL-unset}" "$PATH" "$PWD" "${AGENTX_DIR-}" > "$CAPTURE/project-env"
pwd > "$CAPTURE/cwd"
for fd in 0 1 2; do
    if [ -t "$fd" ]; then printf 'tty\n'; else printf 'pipe\n'; fi
done > "$CAPTURE/streams"
printf '%s' "$$" > "$CAPTURE/pid"
if [ "$#" -eq 1 ] && { [ "$1" = --version ] || [ "$1" = -V ]; }; then
    printf 'codex-cli 0.155.0\n'
    exit 0
fi
if [ "${FAKE_SIGNAL-}" = trap ]; then
    trap 'printf terminated > "$CAPTURE/terminated"; exit 42' TERM
    : > "$CAPTURE/ready"
    while :; do sleep 1; done
fi
if [ "${FAKE_SIGNAL-}" = self ]; then kill -TERM "$$"; fi
printf 'fake diagnostic\n' >&2
cat
exit "${FAKE_EXIT-0}"
"#,
        )
        .unwrap();
        fs::set_permissions(&codex, fs::Permissions::from_mode(0o755)).unwrap();
        Self { agentx, bin_dir, capture }
    }

    fn command(&self) -> Command {
        let mut cmd = self.agentx.cmd();
        let inherited_path = cmd.get_envs().find(|(key, _)| *key == "PATH").and_then(|(_, value)| value).unwrap().to_os_string();
        let path = std::env::join_paths(std::iter::once(self.bin_dir.clone()).chain(std::env::split_paths(&inherited_path))).unwrap();
        cmd.env("PATH", path).env("CAPTURE", &self.capture).env("AMBIENT", "literal \"quote\" \\path\n中文").env("OVERRIDE", "inherited");
        // Version probes intentionally have no agentx identity.
        cmd.env("AGENTX_PROCESS_ID", "parent-process").env("AGENTX_INSTANCE_NAME", "parent-name").env("AGENTX_TOOL", "claude").env("AGENTX_LAUNCHED", "1").env("CODEX_THREAD_ID", "parent-thread").env("CODEX_EXEC_SERVER_URL", "parent-socket");
        cmd
    }

    fn captured(&self, file: &str) -> Vec<String> {
        let bytes = fs::read(self.capture.join(file)).unwrap();
        bytes.strip_suffix(&[0]).unwrap_or(&bytes).split(|byte| *byte == 0).map(|part| String::from_utf8(part.to_vec()).unwrap()).collect()
    }

    fn assert_clean(&self) {
        let database = self.agentx.agentx_dir.join("agentx.db");
        // Passive probes and rejected launches need not acquire a daemon.
        if database.exists() {
            let db = rusqlite::Connection::open(database).unwrap();
            for table in ["instances", "process_bindings"] {
                let count: i64 = db.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)).unwrap();
                assert_eq!(count, 0, "leaked {table}");
            }
        }
        let launch_dir = self.agentx.agentx_dir.join(".tmp/launch");
        assert!(!launch_dir.exists() || fs::read_dir(launch_dir).unwrap().next().is_none(), "stdio created a terminal runner");
        let tracked = fs::read_to_string(self.agentx.agentx_dir.join(".tmp/launched_pids.json")).unwrap_or_else(|_| "{}".to_string());
        let tracked: serde_json::Value = serde_json::from_str(&tracked).unwrap();
        assert!(tracked.as_object().unwrap().is_empty(), "stdio retained a PTY orphan: {tracked}");
    }
}

fn bounded_output(mut child: Child) -> Output {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("stdio child did not exit within 10 seconds");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Disposable direnv stand-in: preserves the exec/authorization/env boundary
/// without depending on a host install or authorizing a developer's files.
fn project_env(fake: &FakeCodex, body: &str) {
    fs::write(fake.agentx.workspace.join(".envrc"), body).unwrap();
    let direnv = fake.bin_dir.join("direnv");
    fs::write(
        &direnv,
        r#"#!/bin/sh
set -eu
[ "$1" = exec ]
project=$2
shift 2
[ "$PWD" = "$project" ]
printf 'direnv fixture loading\n' >&2
printf 'load\n' >> "$CAPTURE/direnv-loads"
if [ "${FAKE_DIRENV_BLOCKED-}" = 1 ]; then
    printf 'direnv fixture blocked\n' >&2
    exit 1
fi
envdir=$project
while [ ! -e "$envdir/.envrc" ]; do envdir=$(dirname "$envdir"); done
export DIRENV_FILE="$envdir/.envrc"
export DIRENV_DIR="-$envdir"
. "$DIRENV_FILE"
exec "$@"
"#,
    )
    .unwrap();
    fs::set_permissions(direnv, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn stdio_project_env_precedes_config_identity_and_hooks_and_preserves_bytes() {
    let fake = FakeCodex::new();
    project_env(
        &fake,
        r#"export AGENTX_DIR="$PWD/project-state"
export CODEX_HOME="$PWD/project-codex"
export AGENTX_CODEX_ARGS='--model project-default'
export HCOM_LITERAL='legacy "quote" \literal'
unset REMOVED
export PATH="$PWD/native-bin:$PATH"
echo envrc-diagnostic >&2
"#,
    );
    let native_bin = fake.agentx.workspace.join("native-bin");
    fs::create_dir(&native_bin).unwrap();
    fs::copy(fake.bin_dir.join("codex"), native_bin.join("codex")).unwrap();
    // The initial PATH tool must not run: direnv's PATH takes precedence.
    fs::write(fake.bin_dir.join("codex"), "#!/bin/sh\nexit 99\n").unwrap();
    let native_args = ["exec", "literal \"quote\" \\ newline\n中文", "", "--dir", "native-only"];
    let mut child = fake
        .command()
        .current_dir(&fake.agentx.home)
        .env("PASEO_AGENT_CWD", &fake.agentx.workspace)
        .env("REMOVED", "parent")
        .args(["launch", "--tool", "codex", "--stdio", "--go", "--"])
        .args(native_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input: Vec<u8> = (0..300_000).map(|i| (i % 256) as u8).collect();
    let mut stdin = child.stdin.take().unwrap();
    let sent = input.clone();
    let writer = std::thread::spawn(move || stdin.write_all(&sent).unwrap());
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, input);
    assert_eq!(output.stderr, b"direnv fixture loading\nenvrc-diagnostic\nfake diagnostic\n");
    let args = fake.captured("args");
    assert!(args.windows(native_args.len()).any(|args| args == native_args), "{args:?}");
    assert!(args.windows(2).any(|args| args == ["--model", "project-default"]));
    assert!(!args.iter().any(|arg| arg.contains("__agentx-stdio-direnv")));
    let env = fake.captured("env");
    let state = fake.agentx.workspace.join("project-state");
    let home = fake.agentx.workspace.join("project-codex");
    assert_eq!(env[2], home.to_string_lossy());
    assert!(!env[3].is_empty() && env[3] != "parent-process");
    assert!(!env[4].is_empty() && env[4] != "parent-name");
    let project = fake.captured("project-env");
    assert_eq!(project[0], "unset");
    assert_eq!(project[1], "legacy \"quote\" \\literal");
    assert!(project[2].starts_with(&native_bin.to_string_lossy().to_string()));
    assert_eq!(project[3], fake.agentx.workspace.to_string_lossy());
    assert_eq!(project[4], state.to_string_lossy());
    assert!(state.join("agentx.db").exists());
    let preflight = fake.captured("preflight-env");
    assert_eq!(preflight, [native_bin.join("codex").to_string_lossy(), home.to_string_lossy(), fake.agentx.workspace.to_string_lossy()]);
    let db = rusqlite::Connection::open(state.join("agentx.db")).unwrap();
    let count: i64 = db.query_row("SELECT count(*) FROM instances", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn stdio_explicit_relative_directory_wins_and_relative_home_is_stable_on_reopen() {
    let fake = FakeCodex::new();
    project_env(&fake, "export CODEX_HOME=relative-codex\n");
    let name = fake.agentx.workspace.file_name().unwrap();
    for _ in 0..2 {
        let output = fake
            .command()
            .current_dir(fake.agentx.workspace.parent().unwrap())
            .env("PASEO_AGENT_CWD", fake.agentx.root_path().join("missing-project"))
            .args(["launch", "--tool", "codex", "--stdio", "--go", "--dir"])
            .arg(name)
            .args(["--", "exec"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(fake.captured("env")[2], fake.agentx.workspace.join("relative-codex").to_string_lossy());
        assert_eq!(fs::read_to_string(fake.capture.join("cwd")).unwrap().trim(), fake.agentx.workspace.to_string_lossy());
    }
    assert_eq!(fs::read_to_string(fake.capture.join("direnv-loads")).unwrap(), "load\nload\n");
    fake.assert_clean();
}

#[test]
fn stdio_paseo_directory_without_env_needs_no_direnv_and_preserves_explicit_home() {
    let fake = FakeCodex::new();
    let output = fake.command().current_dir(&fake.agentx.home).env("PASEO_AGENT_CWD", &fake.agentx.workspace).args(["launch", "--tool", "codex", "--stdio", "--go", "--", "exec"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stderr, b"fake diagnostic\n");
    assert_eq!(fake.captured("env")[2], fake.agentx.codex_home.to_string_lossy());
    assert_eq!(fs::read_to_string(fake.capture.join("cwd")).unwrap().trim(), fake.agentx.workspace.to_string_lossy());
    assert!(!fake.capture.join("direnv-loads").exists());
    fake.assert_clean();
}

#[test]
fn stdio_project_environment_failures_stop_before_config_or_codex() {
    for blocked in [true, false] {
        let fake = FakeCodex::new();
        project_env(&fake, "echo failing-envrc >&2\nexit 23\n");
        let state = fake.agentx.workspace.join("must-not-create");
        let output = fake.command().env("AGENTX_DIR", &state).env("FAKE_DIRENV_BLOCKED", if blocked { "1" } else { "0" }).args(["launch", "--tool", "codex", "--stdio", "--go", "--", "app-server"]).output().unwrap();
        assert!(!output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(if blocked { "blocked" } else { "failing-envrc" }));
        assert!(!state.exists());
        assert!(!fake.capture.join("pid").exists());
    }
}

#[test]
fn stdio_version_without_project_and_ordinary_launch_bypass_direnv() {
    let fake = FakeCodex::new();
    project_env(&fake, "exit 23\n");
    let output = fake.command().args(["launch", "--tool", "codex", "--stdio", "--go", "--", "--version"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"codex-cli 0.155.0\n");
    assert!(output.stderr.is_empty());
    assert!(!fake.capture.join("direnv-loads").exists());
    let output = fake.command().args(["launch", "--tool", "codex", "--", "app-server"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("[Preview Mode]"));
    assert!(!fake.capture.join("direnv-loads").exists());
}

#[test]
fn stdio_project_version_honors_environment_without_identity_or_hooks() {
    let fake = FakeCodex::new();
    project_env(&fake, "export AGENTX_DIR=version-state\nexport CODEX_HOME=version-codex\nexport AGENTX_CODEX_ARGS='--model ignored'\n");
    let output = fake.command().env("PASEO_AGENT_CWD", &fake.agentx.workspace).args(["launch", "--tool", "codex", "--stdio", "--go", "--", "-V"]).env_remove("AGENTX_PROCESS_ID").env_remove("AGENTX_INSTANCE_NAME").output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"codex-cli 0.155.0\n");
    assert_eq!(output.stderr, b"direnv fixture loading\n");
    assert_eq!(fake.captured("args"), ["-V"]);
    let env = fake.captured("env");
    assert_eq!(env[2], fake.agentx.workspace.join("version-codex").to_string_lossy());
    assert!(env[3].is_empty() && env[4].is_empty());
    assert!(!fake.agentx.workspace.join("version-state/agentx.db").exists());
}

#[test]
fn stdio_rejects_mismatched_internal_guard_and_self_recursion() {
    let fake = FakeCodex::new();
    project_env(&fake, "export CODEX_HOME=relative-codex\n");
    let state = fake.agentx.workspace.join("must-not-create");
    let output = fake.command().env("AGENTX_DIR", &state).arg("__agentx-stdio-direnv").arg(&fake.agentx.home).args(["launch", "--tool", "codex", "--stdio", "--go", "--", "app-server"]).output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("project relationship"));
    assert!(!state.exists());
    fs::remove_file(fake.agentx.workspace.join(".envrc")).unwrap();
    let binary = fake.command().get_program().to_os_string();
    for hardlink in [false, true] {
        fs::remove_file(fake.bin_dir.join("codex")).unwrap();
        if hardlink {
            fs::hard_link(&binary, fake.bin_dir.join("codex")).unwrap();
        } else {
            std::os::unix::fs::symlink(&binary, fake.bin_dir.join("codex")).unwrap();
        }
        let output = fake.command().env("AGENTX_DIR", &state).args(["launch", "--tool", "codex", "--stdio", "--go", "--", "app-server"]).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("refusing recursive launch"), "{output:?}");
        assert!(!state.exists());
    }
}

#[test]
fn stdio_preserves_bytes_arguments_effective_environment_and_exit() {
    let fake = FakeCodex::new();
    fs::write(fake.agentx.agentx_dir.join("env"), "OVERRIDE=from-env-file\n").unwrap();
    let native_args = ["exec", "--listen", "stdio://", "quote \" slash \\ newline\n中文", "", "--json", "--name", "literal-name"];
    let mut child = fake.command().args(["launch", "--tool", "codex", "--stdio", "--go", "--"]).args(native_args).env("FAKE_EXIT", "37").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let input: Vec<u8> = (0..300_000).map(|i| (i % 256) as u8).collect();
    let mut stdin = child.stdin.take().unwrap();
    let sent = input.clone();
    let writer = std::thread::spawn(move || stdin.write_all(&sent).unwrap());
    // wait_with_output drains both streams while the writer feeds long input.
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert_eq!(output.status.code(), Some(37), "{output:?}");
    assert_eq!(output.stdout, input);
    assert_eq!(output.stderr, b"fake diagnostic\n");
    let args = fake.captured("args");
    assert_eq!(&args[..native_args.len()], native_args);
    assert!(args.iter().any(|arg| arg.starts_with("hooks.SessionStart=")), "{args:?}");
    assert!(args.iter().any(|arg| arg.starts_with("developer_instructions=") && arg.contains("AGENTX") && arg.contains(&fake.captured("env")[4])), "{args:?}");
    assert!(args.iter().any(|arg| arg.starts_with("sandbox_workspace_write.writable_roots=")));
    let env = fake.captured("env");
    assert_eq!(env[0], "literal \"quote\" \\path\n中文");
    assert_eq!(env[1], "from-env-file");
    assert_eq!(env[2], fake.agentx.codex_home.to_string_lossy());
    assert!(!env[3].is_empty() && env[3] != "parent-process");
    assert!(!env[4].is_empty() && env[4] != "parent-name");
    assert_eq!(&env[5..], ["codex", "1", "unset", "unset", "unset", "unset", "unset"]);
    assert_eq!(fs::read_to_string(fake.capture.join("cwd")).unwrap().trim(), fake.agentx.workspace.to_string_lossy());
    assert_eq!(fs::read_to_string(fake.capture.join("streams")).unwrap(), "pipe\npipe\npipe\n");
    fake.assert_clean();
}

#[test]
fn stdio_success_keeps_configured_arguments_and_directory() {
    let fake = FakeCodex::new();
    fs::write(fake.agentx.agentx_dir.join("config.toml"), "[launch.codex]\nargs = '--model configured-default'\n").unwrap();
    let cwd = fake.agentx.root_path().join("launch directory");
    fs::create_dir(&cwd).unwrap();
    let output = fake.command().args(["launch", "--tool", "codex", "--stdio", "--go", "--dir"]).arg(&cwd).args(["--", "exec"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"fake diagnostic\n");
    let args = fake.captured("args");
    assert!(args.windows(2).any(|pair| pair == ["--model", "configured-default"]), "{args:?}");
    assert!(args.iter().any(|arg| arg == "exec"));
    assert_eq!(fs::read_to_string(fake.capture.join("cwd")).unwrap().trim(), cwd.to_string_lossy());
    fake.assert_clean();
}

#[test]
fn ordinary_codex_launch_keeps_preview_gate() {
    let fake = FakeCodex::new();
    let output = fake.command().args(["launch", "--tool", "codex", "--", "app-server"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("[Preview Mode]"), "{output:?}");
    assert!(!fake.capture.join("pid").exists());
}

#[test]
fn launch_profile_preview_and_stdio_preserve_overrides_and_saved_definition() {
    let fake = FakeCodex::new();
    fs::write(fake.agentx.agentx_dir.join("config.toml"), "[launch.codex]\nargs='--model legacy'\n").unwrap();
    let create = fake.command().args(["profile", "create", "deep", "--tool", "codex", "--", "--model", "saved-model", "--add-dir", "dir with spaces"]).output().unwrap();
    assert!(create.status.success(), "{create:?}");
    let saved = fs::read(fake.agentx.agentx_dir.join("config.toml")).unwrap();
    // The shared fixture's initial config probe may already acquire its daemon.
    // An unbound human preview in a fresh scope must remain runtime-free.
    let preview_scope = fake.agentx.root_path().join("profile-preview-state");
    fs::create_dir(&preview_scope).unwrap();
    fs::write(preview_scope.join("config.toml"), &saved).unwrap();
    let preview = fake
        .command()
        .env("AGENTX_DIR", &preview_scope)
        .env_remove("AGENTX_PROCESS_ID")
        .env_remove("AGENTX_INSTANCE_NAME")
        .env_remove("AGENTX_TOOL")
        .env_remove("AGENTX_LAUNCHED")
        .env_remove("CODEX_THREAD_ID")
        .env_remove("CODEX_EXEC_SERVER_URL")
        .args(["launch", "--profile", "deep", "--stdio", "--dry-run", "--json", "--", "--model", "temporary"])
        .output()
        .unwrap();
    assert!(preview.status.success(), "{preview:?}");
    let value: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(value["configuration"]["args"], serde_json::json!(["--add-dir", "dir with spaces", "--model", "temporary"]));
    assert!(!fake.capture.join("pid").exists());
    assert!(!preview_scope.join("agentx.db").exists());
    assert!(!fake.capture.join("preflight-env").exists());
    assert_eq!(fs::read(preview_scope.join("config.toml")).unwrap(), saved);
    let output = fake.command().args(["launch", "--profile", "deep", "--stdio", "--go", "--", "--model", "temporary"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let args = fake.captured("args");
    assert!(args.windows(2).any(|pair| pair == ["--model", "temporary"]));
    assert!(!args.iter().any(|arg| arg == "saved-model" || arg == "legacy"));
    assert_eq!(fs::read(fake.agentx.agentx_dir.join("config.toml")).unwrap(), saved);
    fake.assert_clean();
}

#[test]
fn stdio_profile_resolves_after_project_environment_selects_scope() {
    let fake = FakeCodex::new();
    project_env(&fake, "export AGENTX_DIR=\"$PWD/project-state\"\n");
    let scope = fake.agentx.workspace.join("project-state");
    fs::create_dir_all(&scope).unwrap();
    fs::write(scope.join("config.toml"), "[launch.profiles.deep]\ntool='codex'\nargs=['--model','project-profile']\n").unwrap();
    let output = fake.command().args(["launch", "--profile", "deep", "--stdio", "--go"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(fake.captured("args").windows(2).any(|pair| pair == ["--model", "project-profile"]));
    assert_eq!(fake.captured("project-env")[4], scope.to_string_lossy());
    assert!(!fake.agentx.agentx_dir.join("agentx.db").exists());
}

#[test]
fn stdio_version_probe_is_native_and_skips_conversation_defaults() {
    let fake = FakeCodex::new();
    fs::write(fake.agentx.agentx_dir.join("config.toml"), "[launch.codex]\nargs = '--model conversational-default'\n").unwrap();
    // The fake requires these vars even for its version path. Empty native
    // identity values let it capture exactly what the probe received.
    for version in ["--version", "-V"] {
        let output = fake.command().args(["launch", "--tool", "codex", "--stdio", "--go", "--", version]).env("AGENTX_PROCESS_ID", "").env("AGENTX_INSTANCE_NAME", "").env("AGENTX_TOOL", "").env("AGENTX_LAUNCHED", "").output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"codex-cli 0.155.0\n");
        assert!(output.stderr.is_empty(), "{output:?}");
        assert_eq!(fake.captured("args"), [version]);
        fake.assert_clean();
    }
}

#[test]
fn stdio_rejects_incompatible_modes_without_stdout() {
    let fake = FakeCodex::new();
    let cases: &[&[&str]] = &[
        &["launch", "--tool", "codex", "--count", "2", "--stdio"],
        &["launch", "--tool", "claude", "--stdio"],
        &["launch", "--tool", "codex", "--stdio", "--headless"],
        &["launch", "--tool", "codex", "--stdio", "--json"],
        &["launch", "--tool", "codex", "--stdio", "--terminal", "here"],
        &["launch", "--tool", "codex", "--stdio", "--no-run-here"],
        &["launch", "--tool", "codex", "--stdio", "--device", "remote"],
        &["launch", "--tool", "codex", "--stdio", "--workspace", "global", "--go", "--", "app-server"],
        &["launch", "--tool", "codex", "--stdio", "--", "app-server"], // preview gate without --go
        &["r", "missing", "--stdio"],
    ];
    for args in cases {
        let output = fake.command().args(*args).output().unwrap();
        assert!(!output.status.success(), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty(), "{args:?}: {output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("stdio"), "{args:?}: {output:?}");
    }
    assert!(!fake.capture.join("pid").exists());
    fake.assert_clean();
}

#[test]
fn stdio_unsupported_shared_version_rejects_without_registration_or_private_fallback() {
    let fake = FakeCodex::new();
    let output = fake.command().args(["launch", "--tool", "codex", "--stdio", "--go", "--", "app-server"]).output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty(), "acquisition failure polluted native stdout");
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("0.155.0") && error.contains("0.167.0"), "{error}");
    assert_eq!(fake.captured("args"), ["--version"], "unsupported version attempted to run a private app-server");
    assert!(!fake.capture.join("preflight-env").exists(), "unsupported service triggered hook preflight");
    fake.assert_clean();
}

#[test]
fn stdio_spawn_failure_cleans_registration() {
    let fake = FakeCodex::new();
    let output = fake.command().args(["launch", "--tool", "codex", "--stdio", "--go", "--", "exec"]).env("FAKE_BREAK_LAUNCH", "1").output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Failed to spawn Codex on stdio"), "{output:?}");
    fake.assert_clean();
}

#[test]
fn stdio_forwards_termination_and_cleans_registration() {
    let fake = FakeCodex::new();
    project_env(&fake, "export CODEX_HOME=relative-codex\n");
    let child = fake.command().args(["launch", "--tool", "codex", "--stdio", "--go", "--", "exec"]).env("FAKE_SIGNAL", "trap").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    fake.agentx.eventually("fake Codex signal handler ready", Duration::from_secs(5), || Ok(fake.capture.join("ready").exists().then_some(())));
    let pid: i64 = fs::read_to_string(fake.capture.join("pid")).unwrap().parse().unwrap();
    fake.agentx.track_cleanup_pid(pid);
    // Send only to the wrapper, as Paseo does, rather than a terminal group.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGTERM) }, 0);
    let output = bounded_output(child);
    assert_eq!(output.status.code(), Some(42), "{output:?}");
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read_to_string(fake.capture.join("terminated")).unwrap(), "terminated");
    fake.assert_clean();
}

#[test]
fn stdio_preserves_signal_exit_status() {
    let fake = FakeCodex::new();
    let output = fake.command().args(["launch", "--tool", "codex", "--stdio", "--go", "--", "exec"]).env("FAKE_SIGNAL", "self").output().unwrap();
    assert_eq!(output.status.code(), Some(128 + libc::SIGTERM), "{output:?}");
    assert!(output.stdout.is_empty());
    fake.assert_clean();
}
