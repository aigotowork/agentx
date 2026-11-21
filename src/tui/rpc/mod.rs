//! TUI RPC types and helpers.
//!
//! All TUI actions are dispatched via commands::run_native() (subprocess of
//! same binary). No daemon dependency.

pub mod client;

use serde::Deserialize;

use crate::integrations::tool::Tool;

// ── Response type ────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct Response {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Response {
    pub fn ok(&self) -> bool {
        self.exit_code == 0
    }

    pub fn combined_output_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self.stdout.lines().map(String::from).collect();
        if !self.stderr.is_empty() {
            lines.extend(self.stderr.lines().map(String::from));
        }
        lines
    }

    pub fn error_message(&self) -> String {
        if let Some(line) = first_non_empty_line(&self.stderr) {
            return line.to_string();
        }
        if let Some(line) = first_non_empty_line(&self.stdout) {
            return line.to_string();
        }
        format!("exit code {}", self.exit_code)
    }
}

fn first_non_empty_line(text: &str) -> Option<&str> {
    text.lines().find(|line| !line.trim().is_empty())
}

// ── Helpers ──────────────────────────────────────────────────────

/// Submit only explicit selectors. The native launcher resolves inherited Role/Profile configuration.
pub(super) fn launch_argv(settings: &crate::tui::model::LaunchState, prompt: &str) -> Vec<String> {
    let mut argv: Vec<String> = vec!["launch".into()];
    for (flag, value) in [("--tool", settings.tool.map(|tool| tool.as_str())), ("--role", settings.quick.role.as_deref()), ("--profile", settings.quick.profile.as_deref())] {
        if let Some(value) = value {
            argv.extend([flag.into(), value.into()]);
        }
    }
    argv.extend(["--count".into(), settings.count.to_string(), "--no-run-here".into(), "--go".into()]);
    let terminal = settings.terminal_override();
    if !terminal.is_empty() {
        argv.extend(["--terminal".into(), terminal.into()]);
    }
    if settings.headless {
        argv.push("--headless".into());
        // claude-only: headless defaults to the live PTY session (no extra flag);
        // when headless_pty is false, opt into `-p` print mode.
    }
    if !prompt.is_empty() {
        argv.extend(["--agentx-prompt".into(), prompt.into()]);
    }
    if settings.headless && settings.tool.or(settings.resolved_tool) == Some(Tool::Claude) && !settings.headless_pty {
        argv.extend(["--".into(), "-p".into()]);
    }
    argv
}

pub fn parse_command_argv(cmd: &str) -> Result<Vec<String>, String> {
    let argv = shell_words::split(cmd).map_err(|e| format!("parse command: {}", e))?;
    if argv.is_empty() {
        return Err("empty command".into());
    }
    Ok(argv)
}

#[cfg(test)]
mod tests {
    use super::{Response, launch_argv, parse_command_argv};
    fn build_launch_argv(tool: Tool, count: u8, headless: bool, headless_pty: bool, terminal: &str, prompt: &str) -> Vec<String> {
        let mut s = crate::tui::model::LaunchState::with_defaults(Default::default(), vec![terminal.into()]);
        s.configured_terminal = String::new();
        s.tool = Some(tool);
        s.count = count;
        s.headless = headless;
        s.headless_pty = headless_pty;
        s.terminal = 0;
        launch_argv(&s, prompt)
    }
    use crate::integrations::tool::Tool;

    #[test]
    fn response_error_prefers_stderr() {
        let resp = Response { exit_code: 2, stdout: "stdout line\n".into(), stderr: "stderr line\n".into() };
        assert_eq!(resp.error_message(), "stderr line");
    }

    #[test]
    fn response_error_falls_back_to_stdout_then_exit_code() {
        let with_stdout = Response { exit_code: 4, stdout: "only stdout\n".into(), stderr: "".into() };
        assert_eq!(with_stdout.error_message(), "only stdout");

        let empty = Response { exit_code: 7, stdout: "".into(), stderr: "".into() };
        assert_eq!(empty.error_message(), "exit code 7");
    }

    #[test]
    fn parse_command_argv_respects_quotes() {
        let argv = parse_command_argv(r#"send nova "hello world""#).unwrap();
        assert_eq!(argv, vec!["send", "nova", "hello world"]);
    }

    #[test]
    fn parse_command_argv_rejects_empty() {
        let err = parse_command_argv("   ").unwrap_err();
        assert!(err.contains("empty"));
    }

    #[test]
    fn launch_argv_uses_explicit_tool_and_count() {
        let argv = build_launch_argv(Tool::Claude, 2, true, false, "kitty", "hello");
        assert_eq!(&argv[..5], ["launch", "--tool", "claude", "--count", "2"]);
        assert!(argv.contains(&"--no-run-here".into()));
        assert!(argv.contains(&"--go".into()));
        assert!(argv.contains(&"--terminal".into()));
        assert!(argv.contains(&"kitty".into()));
        // Headless → --headless; prompt → generic --agentx-prompt.
        assert!(argv.contains(&"--headless".into()));
        assert!(argv.contains(&"--agentx-prompt".into()));
        assert!(argv.contains(&"hello".into()));
    }

    #[test]
    fn launch_argv_always_includes_no_run_here() {
        let argv = build_launch_argv(Tool::Gemini, 1, false, false, "default", "");
        assert_eq!(argv, vec!["launch", "--tool", "gemini", "--count", "1", "--no-run-here", "--go", "--terminal", "default"]);
    }

    #[test]
    fn launch_argv_prompt_is_tool_agnostic() {
        // Per-tool prompt shaping now lives in the launcher (InitialPromptShape);
        // the TUI always forwards the prompt via the generic --agentx-prompt flag,
        // regardless of tool.
        for tool in [Tool::Gemini, Tool::Pi, Tool::Omp, Tool::Antigravity, Tool::Codex, Tool::Cursor, Tool::Copilot, Tool::Qoder, Tool::OpenCode] {
            let argv = build_launch_argv(tool, 1, false, false, "kitty", "fix the bug");
            assert_eq!(
                argv,
                vec![
                    "launch".to_string(),
                    "--tool".to_string(),
                    tool.as_str().to_string(),
                    "--count".to_string(),
                    "1".to_string(),
                    "--no-run-here".to_string(),
                    "--go".to_string(),
                    "--terminal".to_string(),
                    "kitty".to_string(),
                    "--agentx-prompt".to_string(),
                    "fix the bug".to_string(),
                ],
                "{tool:?} should forward prompt via --agentx-prompt"
            );
        }
    }

    #[test]
    fn launch_argv_headless_for_non_claude_tool() {
        // Headless is now offered for every tool; non-claude routes through the
        // PTY headless wrapper via a plain --headless.
        let argv = build_launch_argv(Tool::Gemini, 1, true, false, "kitty", "do task");
        assert_eq!(argv, vec!["launch", "--tool", "gemini", "--count", "1", "--no-run-here", "--go", "--terminal", "kitty", "--headless", "--agentx-prompt", "do task"]);
    }

    #[test]
    fn launch_argv_claude_headless_pty_is_default_no_extra_flag() {
        // Claude's headless default is the live PTY session — just --headless, no
        // extra flag (headless_pty = true).
        let argv = build_launch_argv(Tool::Claude, 1, true, true, "kitty", "do task");
        assert_eq!(argv, vec!["launch", "--tool", "claude", "--count", "1", "--no-run-here", "--go", "--terminal", "kitty", "--headless", "--agentx-prompt", "do task"]);
    }

    #[test]
    fn launch_argv_claude_headless_print_adds_dash_p() {
        // Claude print mode (headless_pty = false) opts into `-p`.
        let argv = build_launch_argv(Tool::Claude, 1, true, false, "kitty", "do task");
        assert_eq!(argv, vec!["launch", "--tool", "claude", "--count", "1", "--no-run-here", "--go", "--terminal", "kitty", "--headless", "--agentx-prompt", "do task", "--", "-p"]);
    }
}
