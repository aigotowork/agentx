# agentx

[![CI](https://github.com/aigotowork/agentx/actions/workflows/ci.yml/badge.svg)](https://github.com/aigotowork/agentx/actions/workflows/ci.yml) [![Latest release](https://img.shields.io/github/v/release/aigotowork/agentx)](https://github.com/aigotowork/agentx/releases) [![License: PolyForm Noncommercial](https://img.shields.io/badge/license-PolyForm%20Noncommercial-blue)](LICENSE)

Connect independent coding agents across terminals and machines. They can message each other, monitor activity, react to events, and launch more agents. Start an agent with `agentx launch --tool <tool>`, then prompt normally.

Use it to coordinate multi-agent pipelines, run different AI CLIs as each other's subagents, or just to avoid copy-pasting.

Works with `claude` `codex` `opencode` `grok` `hermes` `pi` `omp` `agy` `copilot` `qoder` `cursor` `kimi` `kilo` `gemini`

https://github.com/user-attachments/assets/1ce23ed9-f529-4be0-8124-816aa4c2fd43

The daemon also provides an optional [Web management console and HTTP API](.docs/agent-management/README.md): `agentx daemon web --project /absolute/project`. It manages Agents, messages and model transcript snapshots.

[AgentX Inspector](web/inspector/README.md) preserves the full monitoring UI and portable recovery/statistics/Token/digest core as a standalone offline application. It currently uses explicit local mock data; the [data foundation assessment](.docs/inspector/data-foundation.md) records native support and remaining API/data gaps before live integration.

## Install

Download the archive for your machine from [GitHub Releases](https://github.com/aigotowork/agentx/releases/latest).

| Platform | Archive |
| --- | --- |
| macOS Apple Silicon | `agentx-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `agentx-x86_64-apple-darwin.tar.gz` |
| Linux AMD64 (glibc) | `agentx-x86_64-unknown-linux-gnu.tar.gz` |
| Windows AMD64 | `agentx-x86_64-pc-windows-msvc.zip` |

Download the matching `.sha256` file to the same directory. Verify it with `shasum -a 256 -c <file>.sha256` on macOS or `sha256sum -c <file>.sha256` on Linux. On Windows, compare `Get-FileHash <archive> -Algorithm SHA256` with the checksum file. Extract the archive and run `./agentx --version` (or `./agentx.exe --version` in PowerShell).

To install directly, use the release installer for macOS or Linux:

```bash
curl -fsSL https://github.com/aigotowork/agentx/releases/latest/download/agentx-installer.sh | sh
```

Windows:

```powershell
irm https://github.com/aigotowork/agentx/releases/latest/download/agentx-installer.ps1 | iex
```

For other platforms, see the [source build instructions](CONTRIBUTING.md#development). The cloud daemon's [Docker image](deploy/README.md) is distributed separately.

When upgrading a running daemon from v0.7.31–v0.7.33, run `agentx daemon restart --scope PATH` with the new executable. Omit `--scope` to use the normal environment selection. Reopen any Agent sessions still running the earlier host after this one-time upgrade. [Native upgrade details](.docs/architecture/daemon.md#operator-status-and-recovery).

## Quickstart

<table>
<tr>
<td width="50%" valign="top">

**Terminal 1:**

```bash
agentx launch --tool claude
```

</td>
<td width="50%" valign="top">

**Terminal 2:**

```bash
agentx launch --tool codex
```

</td>
</tr>
</table>

<b>Prompt:</b>

<details>
<summary><code>ask the other agent their favorite cake</code></summary>

- `review what claude did and send it fixes`

- `spawn 3x opencode, split work, collect results`

- `fork yourself to investigate the bug and report back`

- `when codex goes idle, send it the next task`

- `find the agent that worked on that feature, resume it and ask it why it's bad`

</details>

**Open the TUI dashboard:**

```bash
agentx
```

单击 Agent 名称移动焦点，点击行首的圆点切换多选。点击底栏的 `transcript` 或按 `t` 打开最近十条会话记录。滚轮浏览指针所在的区域，也可用方向键、Page Up/Down 翻阅。按 `r` 刷新，点 `events` 或按 `Esc` 返回。写消息时可用 `Ctrl+T` 看会话，点击草稿继续输入。

开发时运行 `just tui-replay mixed` 可重放点击、翻阅、编辑和发送，查看逐步屏幕快照与 PNG。依赖和场景见[回放指南](tests/tui_replay/README.md)。

## 调用者身份

AgentX 启动的 Agent 默认使用注入的进程或会话身份，普通命令无需重复声明自己是谁。需要显式选择已有调用者时使用 `--iam`；`start --name` 用于注册命名。

```bash
agentx send @mira --intent inform -- hello
agentx --iam fugu send @mira --intent inform -- hello
agentx start --name fugu
```

`--iam fugu` 表示调用者，`@mira` 表示接收者。`--name` 不再是调用者参数或兼容别名。需要身份的操作遇到缺失、失效或冲突的绑定时会报错，并提示恢复方式。

## 常用任务分派

分派子 Agent 时直接使用已配置的默认值，一条命令传入标题、Task 和初始要求：

```sh
agentx launch --go --headless --title "任务标题" --bind ".controller/xx/xx.task.md" --agentx-prompt "任务要求；请读取绑定的 Task"
```

替换示例中的标题、Task 路径和要求即可。需要专业角色时添加 `--role NAME`；普通分派无需先查看 profile 或命令帮助。缺少默认配置时，启动命令会报错并提示修复方式。首次配置、显式调整模型或其他高级设置见 [profile 用法](.docs/launch-profiles.md) 与 `agentx launch --help`。

当前开发版本支持 [角色与目录团队](.docs/roles-workspaces.md)：已发布的 `.agent-guides/roles/*.md` 提供职责，`.agentx/roles/*.toml` 可按需覆盖。在上述命令中添加 `--role controller` 选择角色，添加 `--dir PATH` 选择工作目录；用 `agentx list --dir PATH` 查看团队，用 `agentx send --dir PATH -- "任务"` 复用或创建默认负责人。全局 TOML 角色位于 `$AGENTX_DIR/roles/`；构建与验收范围见 [Delivery](.controller/roles-workspaces/delivery.md)。

Command shortcuts accept the same arguments as their full names: `l` = `launch`, `r` = `resume`, `f` = `fork`, `k` = `kill`. The delegation command above can use `agentx l` with the same flags; other examples are `agentx r luna`, `agentx f luna`, and `agentx k luna`.

## What agents can do

The current checkout also includes an embedded [collaboration workbench](.docs/collaboration-workbench/README.md): define work, bind implementer/checker roles, follow questions and inspect exact results/checks before explicitly accepting. See [startup and use](.docs/collaboration-workbench/frontend.md#startup-and-everyday-use) and the [accepted Linux local scope](.controller/collaboration-workbench/delivery.md#已接受的本地实现结果). The packaged binary includes the page resources.

- **Message** each other in real time. Messages arrive mid-turn (after tool calls) or wake idle agents immediately.

- **Observe** who's running and what they're doing: status, transcripts, tool results, file edits, commands, live terminal screens, context/token usage. Search past conversations.

- **Subscribe** to status changes, commands, file edits, collisions, and session events, then react automatically.

- **Control**: spawn, fork, resume, kill agents in any terminal emulator or headless.

## How it works

Hooks record activity to a local SQLite database and deliver messages from it.

```text
agent → hooks → db → hooks → other agent
```

Hooks activate only when an agent is launched through `agentx launch`. Normal usage is unaffected.

<details>
<summary>Any other AI tool without hooks can join by running <code>agentx start</code></summary>

<br>

In any CLI tool, prompt:

```text
> run: `agentx start --help` follow what it says and stay connected
```

</details>

<details>
<summary>Any process can wake agents with <code>agentx send</code></summary>

<br>

Send messages from any process/script:

```bash
agentx send -b @luna -- "wake up and do this task"
```

Chain with `agentx events`:

```bash
agentx events --idle luna --wait 600 && agentx send -b @nova -- "luna is done, review it"
```

</details>

## Terminals

Every agent runs in a real terminal you can see, scroll, and interrupt. Any emulator works for spawning (agentx auto-detects). **kitty**, **wezterm**, **tmux**, **zellij**, **waveterm**, **cmux**, **herdr** also support closing panes from `agentx kill`.

To configure a custom terminal open/close setup, tell an agent to run:

```bash
agentx config terminal --info
```

## Cross-device

For the cloud daemon container, Compose quick start and GitHub image releases, see [Docker deployment](deploy/README.md). For one HTTPS deployment shared across machines, see [cloud namespaces, administration, and client setup](.docs/architecture/cloud-scope.md).

Connect agents across machines via MQTT relay.

```bash
agentx relay new               # get token
agentx relay connect <token>   # on each device
```

Check with `agentx relay status`, toggle with `agentx relay off` / `on`.

> Treat the token like an API/SSH key. See [SECURITY.md](SECURITY.md)

---

## Reference

Full reference: `agentx docs`. Select sections with `--cli`, `--config` or `--scripts`.

<details>
<summary><strong>Tools</strong></summary>

### Supported tools

| Tool | Message delivery | Connect |
|---|---|---|
| Claude Code | automatic | `agentx launch --tool claude` |
| Gemini CLI | automatic | `agentx launch --tool gemini` |
| Codex CLI | automatic | `agentx launch --tool codex` |
| Antigravity CLI | automatic | `agentx launch --tool agy` |
| OpenCode | automatic | `agentx launch --tool opencode` |
| Kilo Code | automatic | `agentx launch --tool kilo` |
| Pi | automatic | `agentx launch --tool pi` |
| Oh My Pi | automatic | `agentx launch --tool omp` |
| Cursor CLI | automatic | `agentx launch --tool cursor-agent` |
| Kimi | automatic | `agentx launch --tool kimi` |
| Copilot CLI | automatic | `agentx launch --tool copilot` |
| Qoder CLI | automatic | `agentx launch --tool qoder` |
| Grok Build | automatic | `agentx launch --tool grok` |
| Hermes Agent | automatic | `agentx launch --tool hermes` |
| Anything else | manual via `agentx listen` | `agentx start` (run inside tool) |

```bash
agentx r <session_id>   # Resume a session started outside agentx
agentx f <session_id>   # Fork a session in agentx
```

#### Terminal modes

```bash
agentx launch --tool codex --run-here                      # Use this terminal
agentx launch --tool codex --no-run-here --terminal iterm # Open iTerm (macOS)
agentx launch --tool codex --terminal tmux                 # Managed tmux session; attach now
agentx launch --tool codex --headless --terminal tmux      # Same host; stay detached
agentx attach <name>                                       # Attach to the same running agent
```

For a reconnectable background terminal, install tmux and set `agentx config headless_terminal tmux`. Detach with tmux's default `Ctrl+B`, then `D`. Window defaults remain separate: `agentx config terminal iterm` selects iTerm for `--no-run-here` on macOS. [Terminal sessions](.docs/architecture/terminal-sessions.md) documents all three launch modes, per-command overrides, scoped sockets and native tmux configuration.

#### Claude Code headless and subagents

Detached background processes in print mode stay alive. Manage through the TUI.

```bash
agentx launch --tool claude -- -p 'say hi in agentx'   # print mode (separate Agent SDK credits)
agentx launch --tool claude --headless            # Run normal claude in background pty (works for any tool)
```

For subagents, run `agentx launch --tool claude`, then prompt:

> run 2x task tool and get them to talk to each other in agentx

#### Hermes chats

A Hermes gateway chat (Telegram, Discord, etc) can join agentx:

```bash
agentx hooks add hermes   # plugin into the active profile (HERMES_HOME picks another); a running gateway loads it
```

In the chat: `/agentx start` connects, `/agentx stop` disconnects.

`agentx hooks remove hermes` disconnects and removes its plugin.

</details>

<details>
<summary><strong>CLI</strong></summary>

### CLI commands

What you might type from a shell. Agents run their own commands that they learn from the agentx CLI primer (~700 tokens) at launch. `agentx <command> --help` for full flags.

#### Spawn

```bash
agentx launch --go --headless --title "Task title" --bind ".controller/xx/xx.task.md" --agentx-prompt "Assignment"
agentx r <name|session_id>     # resume agent
agentx f <name|session_id>     # fork session
agentx kill <name|all>   # kill + close terminal pane
agentx attach <name>          # reattach to an existing local tmux-backed agent
```

agentx launch flags:

| Flag | Purpose |
|---|---|
| `--go` | Execute the launch directly |
| `--title <text>` | Readable task title |
| `--bind <path>` | Associate a file-backed Task or Delivery |
| `--role <name>` | Optional professional role |
| `--tool <tool>` | Optional explicit CLI tool selection; ordinary launches use configured defaults |
| `--count <N>` | Number of agents; positive integer, default `1` |
| `--terminal <preset>` | Terminal override: `tmux` attaches (unless detached); `kitty`, `wezterm`, `cmux`, `iterm`, etc. open a terminal |
| `--dir <path>` | Directory where the agent launches |
| `--headless` | Run in background pty with no terminal window |
| `--stdio` | Codex only: run one local foreground process on inherited stdin/stdout/stderr, without a PTY |
| `--device <name>` | Spawn on a remote device (via relay) |
| `--agentx-prompt <text>` | Initial user prompt |
| `--agentx-system-prompt <text>` | Append to system prompt |
| `--json` | Print the outcome as one JSON object (also `agentx r`, `agentx f`, `agentx kill`); an error that stops the command before it acts goes to stderr with a non-zero exit and no JSON |

Native arguments are forwarded to the tool only after `--`: `--model sonnet`, `--yolo`, etc., after the `--` separator. Unknown options before `--` are errors.

For a client that speaks the Codex app-server protocol over pipes, use this argv prefix:

```bash
agentx launch --tool codex --stdio --go -- app-server
agentx launch --tool codex --stdio --go -- --version
```

Configure the prefix as `["agentx", "launch", "--tool", "codex", "--stdio", "--go", "--"]` when the client appends `app-server` or `--version`. Standard streams pass directly to Codex; agentx diagnostics use stderr and its exit code follows Codex. Normal launches still prepare identity, hooks, bootstrap and environment. Version probes skip session preparation and configured conversation arguments. This option cannot be combined with multiple agents, `--headless`, `--json`, `--device`, `--terminal` or `--no-run-here`.

Native stdio selects its project from `--dir`, then nonempty `PASEO_AGENT_CWD`, then the current directory. It enters the canonical project and loads its authorized direnv `.envrc` (including an ancestor's) before agentx configuration, default arguments, identity or hooks. Direnv must be in PATH when an `.envrc` exists; blocked, changed or failing files stop the launch on stderr. Agentx never authorizes them automatically. Environment values, deletions and project PATH are imported literally, followed by the usual agentx configuration overrides. Projects without `.envrc` need no direnv. Ordinary interactive launches use the explicit dotenv controls below; `.envrc` execution remains specific to this authorized stdio boundary.

`CODEX_HOME` remains explicit when supplied; relative values resolve against the canonical project. Reopening with the same environment selects the same absolute home and native thread storage. Changing the home later does not move old threads into the new home. A `--version` probe without `--dir` or `PASEO_AGENT_CWD` bypasses project direnv loading, so daemon discovery does not load project secrets; project-specific probes can use either input and retain native version stdout.

Without a PTY, idle message wake, screen viewing and terminal injection are unavailable. Hooks can deliver messages during turns; the client must start another turn or poll explicitly while idle. Linux process shutdown is covered by focused tests; other platforms retain their compilation paths but have not been exercised for native stdio. Maintainers: see the [native contract and focused cases](src/README.md#native-codex-stdio).

#### Other commands

```bash
agentx                           # TUI dashboard
agentx send -b @luna -- hey      # one-off message to an agent
agentx list                      # show all active agents
agentx term [name]               # view/inject into an agent's PTY screen
agentx events --wait <filters>   # Block until match for scripting
agentx events --follow           # Stream events as JSON Lines
agentx update                    # update agentx version
```

`agentx docs --cli` for all commands.

</details>

<details>
<summary><strong>Config</strong></summary>

#Future key/value Agent classification is recorded in [the classification direction](.docs/agent-classification.md); it is not implemented.

## Configuration

Config lives in `~/.agentx/config.toml`. Precedence: defaults < `config.toml` < env vars.

```bash
agentx config                           # show all values with sources
agentx config <key>                     # get
agentx config <key> <value>             # set
agentx config <key> --info              # detailed help for a key
agentx config -i <name> <key> <value>   # per-agent override at runtime
```

#### Keys

| Key | Purpose |
|---|---|
| `hints` | Text appended to every message the agent receives |
| `notes` | Text appended to the built-in guide at startup and supported compact refreshes |
| `context_sources` | Ordered built-in/text/file sources; [configuration and compact support](.docs/agent-context.md) |
| `bootstrap_exclude` | Bootstrap parts to omit: `identity`, `messages`, `capabilities` (including relay, headless, and command notices), `rules`, `handoff`, or `all` (default: `none`). Inspect with `agentx config bootstrap show [part]`. |
| `auto_approve` | Auto-approve safe agentx commands (send/list/events/…) |
| `auto_subscribe` | Event subscription presets: `collision`, `created`, `stopped`, `blocked` |
| `name_export` | Export instance name to a custom env var |
| `title_mode` | Terminal/tab title behavior: `combined` (default), `label`, or `off` |
| `terminal` | Where new agent windows open (`agentx config terminal --info`) |
| `timeout` | Idle timeout for headless Claude (seconds) |
| `subagent_timeout` | Keep-alive for Claude subagents (seconds) |
| `claude_args` / `gemini_args` / `codex_args` / `opencode_args` / `kilo_args` / `pi_args` / `omp_args` / `cursor_args` / `kimi_args` / `copilot_args` / `qoder_args` / `grok_args` / `hermes_args` | Default args passed to the tool |

#### Scope

```bash
agentx config -i luna hints "respond in JSON"   # per-agent
```

#### Per-project isolation

```bash
export AGENTX_DIR="$PWD/.agentx"   # isolate agentx state (db, logs) to this folder
rm -rf "$AGENTX_DIR"             # clean up
```

Run `agentx config <key> --info` or `agentx docs --config` for the full per-key reference.

Load shared/project/local dotenv data explicitly when the calling terminal or app has not prepared it:

```bash
agentx --env-file ~/.config/agentx/shared.env --env-file .env --env-file .env.local launch --tool codex
agentx --env-file .env --env-override launch --tool codex
agentx config env_files '[".env",".env.local"]'
agentx config env_override false
agentx config environment --json
```

Repeated CLI files replace `AGENTX_ENV_FILES` (a comma-separated list), which replaces `[environment].files` in the original scope's config. Later files win; inherited variables, including empties, win unless `--env-override`, `AGENTX_ENV_OVERRIDE=true` or `[environment].override=true` selects replacement. `--no-env-override` reverses it. `--env-file=` or an empty `AGENTX_ENV_FILES` disables lower file lists. Source inspection exposes paths and variable provenance with all values withheld.

Relative paths use the selected local project (`--dir` for launches, `--project` for workbench/work, otherwise cwd). External files can supply `AGENTX_DIR`; TOML-selected files cannot redirect their scope. Selectors/override are frozen before dotenv, so a file-selected new scope never recursively loads another list. Files are data; `.envrc` still requires the authorized direnv stdio path. See the [complete environment contract](.docs/architecture/environment.md) and [independent support/evidence](.controller/environment-loading/delivery.md).

The packaged Linux local candidate passed independent selection/override/empty/scope/diagnostic, finite launch/stdio/nested-shell, isolation and authorized direnv/legacy checks. Workbench coverage is a finite startup/HTTP check; browser rendering, other operating systems and universal Provider/terminal behavior are outside this acceptance. [Exact evidence](.controller/environment-loading/evidence.md) and [Delivery](.controller/environment-loading/delivery.md) distinguish the verified candidate from repository promotion and running services.

Edit `~/.agentx/env` for the older child-only passthrough. It retains its existing precedence: non-empty, non-`AGENTX_*` values override launched children after config; it is not an originating dotenv selector.

</details>

<details>
<summary><strong>Workflow Scripts</strong></summary>

### Multi-agent workflows

Bundled and user scripts (`~/.agentx/scripts/`) for multi-agent patterns:

```bash
agentx run                  # list available scripts
agentx run debate "topic"   # run one
agentx docs --scripts       # read the workflow authoring guide
```

#### Included scripts

Tell agent to run them:

- **`agentx run confess`** — An agent (or background clone) writes an honesty self-eval. A spawned calibrator reads the target's transcript independently. A judge compares both reports and sends back a verdict via agentx message.

- **`agentx run debate`** — A judge spawns and sets up a debate with existing agents. It coordinates rounds in a shared thread where all agents see each other's arguments, with shared context of workspace files and transcripts.

- **`agentx run fatcow`** — headless agent reads every file in a path, subscribes to file edit events to stay current, and answers other agents on demand.

- **`agentx run onidle`** — waits for an agent to go idle, then types text into another agent (`agentx run onidle luna nova 'luna is done, review it'`) or launches a new one with it as the prompt (`agentx run onidle luna codex 'review what luna just did'`).

Custom scripts: drop `*.sh` or `*.py` into `~/.agentx/scripts/` — auto-discovered, override bundled scripts of the same name. Ask an agent to author one; `agentx docs --scripts` is the authoring guide.

</details>

<details>
<summary><strong>Troubleshoot and Uninstall</strong></summary>

### Troubleshoot

```bash
agentx status   # diagnostics
```

```bash
agentx reset all   # clear and archive: database + hooks + config
```

### Uninstall

Safely remove all agentx hooks:

```bash
agentx hooks remove
```

Then remove binary:

```bash
brew uninstall agentx
# or: rm "$(which agentx)"
```

</details>

## Contributing

Issues and PRs welcome. Build from source and dev setup: [CONTRIBUTING.md](CONTRIBUTING.md)

Common project tasks are managed in [justfile](justfile) using [just](https://just.systems). Run `just --list` to discover them; on macOS/Linux, `just package` builds the release executable into `.bin/agentx` and checks its version.

The standalone [Flow Atlas domain-model design workbench](tools/flow-atlas/workbench/README.md) visualizes source-first TS objects, contracts, owned rules/state, process dataflow and scenarios. [Open the local LAN demo](http://192.168.2.178:18959/) or the [offline example](tools/flow-atlas/examples/domain-workbench.html). See [development usage](CONTRIBUTING.md#领域模型设计工作台) and [accepted local scope](.controller/domain-daemon/flow-atlas-workbench-implementation.md#accepted-local-outcome); it is separate from the AgentX runtime and collaboration workbench.

## License

AgentX project code is source available under the [PolyForm Noncommercial License 1.0.0](LICENSE). Its terms permit noncommercial use, modification, and redistribution. Commercial use outside those terms requires separate permission from the copyright holder.

Third-party material stays under its original terms. See the notices for [Agent Space](web/agent-space/NOTICE.md), [Inspector](web/inspector/NOTICE), and the [bundled skills](.agent-guides/skills/NOTICE.md). This license does not change rights already granted under earlier licenses.
