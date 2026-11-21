# Use agentx

Use agentx to discover available Roles, Skills and agents, start a bound agent, or recover
its identity and task context. It reads task state; the agent edits the original task directly.

From the project root, add `.bin` to PATH or invoke `.bin/agentx` explicitly:

```sh
export PATH="$PWD/.bin:$PATH"
agentx --help
```

## Choose a command

| Goal | Command | Result |
| --- | --- | --- |
| Explore a directory | `agentx discover --dir <module>` | Available Roles/Skills and bound agents with current resource state |
| Start a new agent | `agentx launch --dir <module> --role <role> --skill <extra> --task <task.md>` | New working Session and agentx-allocated native agent |
| Read your bound resources | `agentx status` | Current task metadata, source availability and agentx observation |
| Recover your context | `agentx mycontext` | Role prompt, Skill references, guidance and original task content |
| Continue an existing agent | `agentx resume <session>` | Same working Session and native continuation through agentx |

Inside an agent, Session selection comes from injected `AGENTX_SESSION_DIR`; no arguments
are needed for status/mycontext. agentx supplies its own caller identity. Outside an agent,
use `status --session <ID>` or `mycontext --session <ID>`; `--dir` locates the ID if you are
not in the owning directory. Resume accepts a Session path/ID, or the injected identity.
There is no guessed latest Session and no caller-name argument.

## Launch with bindings

A Role uses YAML frontmatter and a Markdown prompt:

```markdown
---
schema: role
role: frontend
description: Maintain the selected module's UI.
skills: [shadcn, tsxctx]
---

Read the original task, implement within its scope, and maintain progress in that task.
```

Discover actual available sources before selecting names. Role defaults and repeated
launch `--skill` additions are combined. A discovered explicit source path can be bound
directly. Task binding is optional and points to an existing Markdown file with YAML
frontmatter; binding does not change it or claim its ownership.

```sh
agentx discover --dir apps/web
agentx launch --dir apps/web --role frontend --task .controller/settings.md
```

Repeat `--skill` or `--env NAME=VALUE` for more bindings/settings. Pass Codex arguments
after `--`, for example `-- --model <available-model>`. Role, Task and extra environment
are optional. agentx generates the new native name; launch returns the working Session path.

## Read resources and recover context

```sh
agentx status
agentx mycontext
agentx mycontext --format yaml
```

Status is a brief resource view. Its Task metadata comes from the original frontmatter,
including the owner's status, progress, blocker, next action and result. It reports missing
Role/Skill sources or unavailable Task files and includes agentx runtime state separately.

Mycontext includes the original Task body and retained Role instructions. Call it after
context compaction or CWD changes. Save important reasoning, progress and next steps by
editing the original Task and its domain documents directly; agentx does not provide a task
write, checkpoint, transition or assignment command. No Task state is copied into Session.

To inspect a particular agent from the project root:

```sh
agentx status --session s_012345abcdef --dir apps/web
agentx mycontext --session s_012345abcdef --dir apps/web
```

## Configuration and recovery

Optional `.agentx/config.yaml` supplies runtime defaults:

```yaml
runtime:
  codex_args: [--model, your-available-model]
  env: {EXAMPLE_MODE: "trial"}
```

Closer maps override ancestor values; lists replace. Explicit `--env` overrides configured
environment. The executable uses uv to prepare its isolated YAML dependency.

Each Session contains one `session.yaml`. Launch outcome/response is returned, not saved
as a report. Current runtime observation is read from agentx and may be unavailable without
preventing local task/context reads. No read command rewrites any bound source or Session.

An unknown launch result requires checking agentx before retrying, especially if the native
identity reply was lost. Use agentx to stop an agent, and `agentx resume <session>` to delegate
continuation. Native failure is returned directly; there is no replacement-conversation fallback.
