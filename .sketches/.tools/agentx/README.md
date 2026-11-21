# agentx

agentx discovers directory-scoped Roles, Skills and bound agents, launches agents with
an explicit identity and bindings, and restores their working context after compaction.
It consumes resource state; it does not manage Task lifecycles. [Usage](USAGE.md) owns examples.

Run `.bin/agentx --help`, or `agentx --help` with the project's `.bin` on PATH. Requires
Python 3.11+, uv and the declared ruamel.yaml dependency. Live execution uses agentx/Codex.

## Responsibility and ownership

| Information | Authority |
| --- | --- |
| Role responsibilities, prompt and default Skills | Original `.agent-guides/roles` definition |
| Skill methods and resources | Original `SKILL.md` and its resources |
| Stable working identity, scope, resolved bindings, launch settings and native identity references | `<scope>/.agentx/sessions/<id>/session.yaml` |
| Task status, progress, blockers, next action and results | Original Task document, edited by its agent or owner |
| Agent runtime, admission and messaging | agentx |
| Conversation history and native continuation | Codex |

Each Session has one YAML file. agentx writes its own bindings and native identity references,
never the original Task, Role or Skill. Startup instructions are generated in memory.
There are no independent Run records, lock files, task-state replicas, saved launch reports,
task ownership claims or checkpoint commands. Task schema and lifecycle values belong to
the task owner; agentx reads its Markdown YAML frontmatter without enforcing a status machine.

## Commands

```text
agentx [--root DIR] [--format yaml|text]
  discover [--dir DIR] [--timeout SECONDS]
  launch [--dir DIR] [--role REF] [--skill REF ...] [--task FILE]
         [--env NAME=VALUE ...] [--timeout SECONDS] [-- CODEX_ARGS ...]
  resume [SESSION] [--dir DIR] [--timeout SECONDS]
  status [--session SESSION] [--dir DIR] [--timeout SECONDS]
  mycontext [--session SESSION] [--dir DIR] [--timeout SECONDS]
```

- `discover` lists effective Role and Skill definitions available from scope towards root,
  plus agents bound by agentx directly in the selected directory. Roles are reusable
  definitions; an entry in `agents` is a Session and its bound resources, not another Role.
  Nested Skills are exposed by their explicit bindable source path. agentx agents without
  agentx bindings remain inspectable through agentx itself.
- `launch` resolves bindings, creates a working Session and asks agentx to start a new agent.
  agentx allocates the native name; caller identity is inherited from native agentx environment.
- `resume` delegates continuation using a Session's native conversation ID or agentx name.
  If no native binding is known, it submits that Session's initial launch. It never invents
  a replacement conversation after native resume failure.
- `status` reads current bound Task metadata first, then Role/Skill source availability
  and supplementary agentx observation. It omits Role instructions and Task body.
- `mycontext` supplies Role instructions, Skill entry points, project guidance, the current
  Task metadata/body and feedback references for recovery. It does not write progress,
  restore unrecorded reasoning or duplicate the task's domain state.

Status and context remain usable when a bound task is missing or malformed: the resource
is marked unavailable with a diagnostic while other bindings remain visible. The retained
Role prompt is still recoverable if its original source has been removed. `available`
reports source-file availability, not task completion or runtime activity.

## Identity and resolution

Inside an agent, `agentx status` and `agentx mycontext` use injected `AGENTX_SESSION_DIR`.
No ID, directory or caller name needs repeating after a CWD change. An explicit `--session`
or resume operand selects an external target. Short IDs resolve only within `--dir`,
which defaults to `AGENTX_DIR` or CWD; the tool never guesses the latest Session.

Role/Skill lookup walks scope towards root, preferring `.agent-guides/local` over
`.agent-guides` at each level. Sibling modules are not searched. Explicit CLI paths resolve
from invocation CWD; explicit Role Skill paths resolve from the Role's directory.
Effective Skills are Role defaults plus launch additions, deduplicated by source path.
Different source files with the same native Skill name require disambiguation.

Role frontmatter uses `schema: role`, `role`, optional `description` and `skills`.
Markdown body supplies the prompt; standalone YAML can use `content`. The resolved Role
prompt is retained at creation; Skill bodies and Task data remain original live references.

| Input | Precedence, highest first |
| --- | --- |
| Root | `--root`, `AGENTX_ROOT`, tool's project |
| Directory | `--dir`, `AGENTX_DIR`, CWD |
| Role / Task | CLI option, `AGENTX_ROLE` / `AGENTX_TASK`, unset |
| Additional Skills | Repeated `--skill`, YAML list `AGENTX_SKILLS`, empty |
| Session | Explicit selector, `AGENTX_SESSION_DIR` |
| agentx caller | Native `AGENTX_INSTANCE_NAME`, otherwise agentx's own resolution |
| Timeout | `--timeout`, `AGENTX_TIMEOUT`, 45 seconds; positive |
| Extra environment | `--env` overrides `runtime.env`; `AGENTX_*` names are reserved |
| Codex arguments | Arguments after `--` replace `runtime.codex_args` |
| Format | `--format`, `AGENTX_FORMAT`, command default |

Optional `.agentx/config.yaml` maps merge from root towards scope; lists replace. It supplies
runtime defaults, not implicit Role or Task assignment. Global flags work before or after
commands; arguments after `--` retain native Codex meaning.

## Runtime and output

Native calls use argv without a shell. New Codex launches request headless execution;
resume omits that flag. agentx injects Session location, root and PATH through Codex's shell
configuration, supplies the bound Skill catalog and disables the automatic Skill instruction
block. Other native Skill entry points are not a strict allowlist.

Launch responses describe that command invocation and are returned without being persisted.
Outcomes are submitted, prepared (preview), failed or unknown. A timeout does not establish
that nothing started; inspect agentx before explicitly retrying an uncertain launch. agentx owns
runtime admission. agentx has no runtime state machine, exclusion or automatic retry loop.

Runtime observations are read from agentx with their query time, never from saved activity
state. Missing/mismatched instances or unavailable agentx leave observation null with a
diagnostic while local bound-resource information remains readable. Every read command is
read-only. No agentx command writes or claims ownership of the Task.

Output defaults to YAML, except mycontext defaults to Markdown. Its YAML output includes
full Task body; text limits long bodies to 12,000 characters and points to the original.
stdout carries results and stderr operating diagnostics. Exit codes: 0 for successful command
results, 1 for native launch failure, 2 for invalid inputs/operating errors, 3 for an unknown
launch outcome. Missing resources may be reported within an otherwise successful read.
Session writes use atomic replacement and assume one writer; there are no locking guarantees.

## Verification and limits

```sh
PYTHONDONTWRITEBYTECODE=1 uv run --with 'ruamel.yaml>=0.18.6,<0.19' python -m unittest discover -s .tools/agentx/tests -v
```

Tests use automatically cleaned temporary directories. They cover discovery, scope/shadowing,
bindings, task consumption without writes, unknown native task states, missing resources,
context recovery, launch delegation and native observation. No report files are retained.
Feedback notification routing, automatic compaction hooks and strict native Skill filtering
remain outside the implemented surface. Definitions/config may be versioned; local Session
bindings stay outside source control.
