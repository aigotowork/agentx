---
name: "hcom"
description: "Use for hcom agent messages, runtime observation, and authorized launch or resume."
---

# hcom quick use

Use the identity supplied by the current session as `SELF` in every command; `PEER` is the verified recipient. Replace all uppercase placeholders before running examples. Current session instructions and installed `hcom COMMAND --help --name SELF` take precedence over examples. These commands require an available hcom runtime and authorization for the intended action.

The native HCOM runtime is separate from this Skill and from any project `hcom` wrapper. A wrapper needs an explicit native target (for example the absolute installed executable selected by `HCOM_REAL_BIN`); never resolve it back to the wrapper itself. The native executable need not be named `hcomx`. Provider CLIs, tmux and GUI terminals are required only for the selected launch/terminal route. Use the consumer's setup diagnostics and original HCOM installation instructions; copying this Skill does not install those programs.

## Common patterns

| Need | Command |
| --- | --- |
| Identify agents and inspect status | `hcom list -v --name SELF` |
| Request a scoped action            | `hcom send @PEER --intent request --name SELF -- 'Read TASK_ENTRY; report evidence in REPORT_PATH.'` |
| Acknowledge a received request     | `hcom send @PEER --intent ack --reply-to EVENT_ID --name SELF -- 'Accepted; starting the assigned scope.'` |
| Report completion or an actionable handoff | `hcom send @PEER --intent inform --name SELF -- 'Assigned outcome complete; evidence and limits are at REPORT_PATH.'` |
| Send exact multiline text          | `hcom send @PEER --intent request --name SELF --file PROMPT_FILE` |
| Inspect recent activity            | `hcom events --agent PEER --last 10 --name SELF` |
| Read recent conversation           | `hcom transcript PEER --last 10 --name SELF` |
| Subscribe to next idle event       | `hcom events sub --agent PEER --status listening --once --name SELF` |

Target the exact local name; `@PEER:DEVICE` addresses a remote instance. Omitting recipients broadcasts, so keep explicit targets for ordinary work. Put flags before `--`; everything after it is message text. Use `--file` or a quoted heredoc for code, backticks, or multiline prompts. Never impersonate `bigboss` with sender overrides.

## Basic scenarios

- **Receive work:** acknowledge a request using its received event ID, read the linked assignment, do the scoped work, and report evidence. Launch readiness is not a received request; never fabricate a reply ID for an initial claim. Follow current runtime reply rules; do not acknowledge acknowledgements or send filler.
- **Check a delayed reply:** inspect `list` and recent `events`; read `transcript` only when those leave a concrete question. If progress is ongoing, subscribe and end the turn for automatic delivery. For an expected hcom message, simply end the turn. Do not poll or sleep; `hcom listen` is not the wait mechanism for hcom messages or agent progress.
- **Launch or resume an authorized agent:** new Codex: `hcom 1 codex --headless --tag DOMAIN --dir REPO_PATH --name SELF`. Find a stopped instance: `hcom list --stopped PEER --name SELF`. Resume it: `hcom r PEER --go --name SELF`. Codex resume does not support `--headless`; do not reuse new-launch flags blindly. Confirm readiness and the returned identity before addressing the agent.

For unfamiliar flags, other runtimes, remote operations, fork, bundles, or terminal control, consult that command's installed help. A command example does not grant authority to stop, kill, or inject into another agent's terminal.
