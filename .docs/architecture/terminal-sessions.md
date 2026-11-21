---
schema: document
status: active
owner: runtime
scope: Launch location, managed tmux hosting and terminal attachment
related: [../../src/runtime/AGENTS.md, ../../src/cli/AGENTS.md, ../../.controller/tmux-headless/task.md]
---

# 启动位置与后台终端

AgentX 按三种方式承载交互式 Agent。启动方式决定进程在哪里运行，终端配置决定用哪个宿主。[交付任务](../../.controller/tmux-headless/task.md)记录实际验证范围。

| 方式 | 使用方法 | 终端与生命周期 |
| --- | --- | --- |
| 当前终端 | `agentx launch --tool codex --run-here` | 复用当前终端；普通交互式 shell 中启动单个 Agent 时也默认如此。关闭终端可能结束 Agent。 |
| 新终端 | `agentx launch --tool codex --no-run-here --terminal iterm` | 在选定终端中启动现有 PTY runner；此处 iTerm 示例适用于 macOS。 |
| 前台 tmux | `agentx launch --tool codex --terminal tmux` | 创建受管 tmux 会话并立即进入；分离后 Agent 继续运行。 |
| 后台终端 | `agentx launch --tool codex --headless --terminal tmux` | 命令返回后由 tmux 保持终端；之后用 `agentx attach NAME` 回到同一个 Agent。 |

`agentx l` 在普通交互式 shell 中启动单个 Agent 时默认复用当前终端；`--run-here` 显式要求此行为。`--terminal tmux` 创建受管会话并进入，`--terminal tmux --headless` 与 `--terminal tmux --no-run-here` 均创建受管会话并立即返回。后两者的终端体验一致，但 `--headless` 还保留后台启动的状态与提示语义。

显式 `--terminal iterm` 等窗口预设直接选择新终端，无需再加 `--no-run-here`。`--run-here` 与 `--headless`、显式非 `here/default` 的终端均互斥。后台宿主只接受 `pty` 或 `tmux`。

自动进入 tmux 仅适用于本地、单个 Agent、普通交互式调用；`--json`、非交互标准输入/输出、批量启动及 AI 工具内部调用均不自动进入。需要时执行 `agentx attach NAME`。远端启动不会连接本机终端。如果当前位于另一个 tmux server，先 detach，再连接新 Agent；自动连接失败会保留已启动的 Agent 并显示重连命令。

受管 PTY（包括后台 PTY、tmux 内部 runner 和 Windows ConPTY）在启动子进程时会将缺失、空值或 `dumb` 的 `TERM` 自动设为 `xterm-256color`；有效的显式配置或继承值保持不变。该默认值仅作用于子进程，不修改用户 shell 或非 PTY 命令环境。

[`AGENTX_PTY_MODE`](../../src/runtime/launcher.rs#L718) 是 Claude Hook 的投递开关，告诉 Stop hook 由 PTY 负责输入。Codex 前台和后台都可运行在受管 PTY 中，启动器不会为它设置这个变量。判断具体 Agent 是否有受管终端，应查询原生进程绑定和宿主注册。

## 配置放在哪里

配置写在当前数据目录的 `$AGENTX_DIR/config.toml`。未指定 `AGENTX_DIR` 时通常是 `~/.agentx/config.toml`。

```toml
[terminal]
active = "iterm" # 新终端的默认宿主；此例用于 macOS。
headless = "tmux" # 后台默认宿主；可选 "pty" 或 "tmux"。

[terminal.tmux]
socket = ".tmp/tmux/socket" # 相对于 AGENTX_DIR，也接受绝对路径。
config = "tmux.conf" # 可选：原生 tmux 配置文件，相对路径规则相同。
```

窗口和后台分别配置，因此可以同时使用“新窗口默认 iTerm”和“后台默认 tmux”。未设置 `headless` 时保留原有 `pty` 默认值。上面的 socket/config 配置同时作用于前台与后台受管 tmux 会话。`tmux-split` 保留在当前 tmux server 分屏的原有预设行为。

| CLI 配置键 | TOML 路径 | 环境变量 | 默认值 |
| --- | --- | --- | --- |
| `terminal` | `terminal.active` | `AGENTX_TERMINAL` | 原有 `default` 终端选择 |
| `headless_terminal` | `terminal.headless` | `AGENTX_HEADLESS_TERMINAL` | `pty` |
| `tmux_socket` | `terminal.tmux.socket` | `AGENTX_TMUX_SOCKET` | `$AGENTX_DIR/.tmp/tmux/socket` |
| `tmux_config` | `terminal.tmux.config` | `AGENTX_TMUX_CONFIG` | `$AGENTX_DIR/.tmp/tmux/default.conf`，默认 10 万行历史、开启鼠标 |

宿主选择优先级为本次 `--terminal` → 对应方式的环境变量 → TOML → 默认值；路径配置使用环境变量 → TOML → 默认值。相对路径基于 `AGENTX_DIR`，路径中的 `~` 不会自动展开。

可以直接编辑文件，也可以用 CLI 写入同一份配置：

```bash
agentx config headless_terminal tmux
agentx config tmux_socket .tmp/tmux/socket
agentx config tmux_config tmux.conf

agentx launch --tool codex --headless                # 使用配置中的 tmux
agentx launch --tool codex --headless --terminal pty # 仅本次改用原生后台 PTY
```

## Socket 和 tmux.conf

默认 socket 通常是 `~/.agentx/.tmp/tmux/socket`。不同 AgentX 数据目录分别使用各自的 socket；不会连接用户的默认 tmux server。AgentX 会为新建的 socket 父目录设置私有权限，已有目录的权限由用户维护。每个 Agent 使用唯一的 `agentx-UUID` 会话，正常操作只处理目标会话，不执行整个 server 的关闭。

自定义 socket 可以让多个数据目录显式共享一个 tmux server。Unix socket 路径有长度限制，过长时请设置更短的绝对路径。宿主必须安装 tmux；选定 tmux 但找不到程序时会明确报错。

`tmux_config` 指向普通的 tmux 配置文件。例如，上面的配置需要先创建 `$AGENTX_DIR/tmux.conf`：

```tmux
set -g history-limit 100000
set -g mouse on
```

该设置可省略；省略时 AgentX 生成 `.tmp/tmux/default.conf`，默认 `history-limit 100000` 和 `mouse on`，不加载用户的 `~/.tmux.conf`。生成文件由 AgentX 管理，自定义设置应写入 `tmux_config` 指定的文件。明确指定的文件必须存在。Tmux 只在 server 首次启动时读取配置；修改文件不会自动重载已有 server。要主动重载，使用真实 socket 和配置文件路径执行：

```bash
tmux -S /absolute/path/to/socket source-file /absolute/path/to/tmux.conf
```

## 回到正在运行的 Agent

```bash
agentx attach NAME
agentx attach --dir .
# 分离：默认快捷键 Ctrl+B，然后按 D。
```

`attach` 查询当前本地 Agent 的实际终端元数据，并校验 tmux server 和 pane 后连接原会话。它不重新启动 Provider；`resume` 则是重新启动已有对话。修改当前 socket 配置不会把已有 Agent 的 attach 重定向到新 server。已退出、非 tmux 或尚无完整终端信息的目标会给出错误。

省略名字时，`attach` 在当前 workspace 和全局成员中打开选择器；`--all` 可浏览所有 workspace 的可连接终端。`--dir .` 从当前目录发现项目，`--workspace .` 精确选择当前目录。也可添加重复的 `--label` 条件。详情见[目录和团队筛选](../roles-workspaces.md#归属目录与发现)。

从同一 tmux server 内调用时切换当前 client；如果当前终端属于另一个 tmux server，需要先 detach 再 attach。自定义过 tmux prefix 的用户使用自己的分离快捷键。

Runtime 负责终端 I/O 和 tmux 进程；Agent 身份、消息投递及现有 PTY runner 保持原有职责。显式 Claude print 模式没有交互终端，拒绝显式选择 tmux。Tmux 宿主仅用于 Unix；Windows 保留原生 PTY。

## Codex Agent Command Center

Codex 的 **Agent Command Center**（`/agents` 或 `codex agents`）按 [当前 Codex app-server 加载的会话和近期历史](https://github.com/openai/codex/blob/rust-v0.170.0/codex-rs/tui/src/app/agents_overview_threads.rs#L363-L398) 建列表。两个进程可以使用同一个 `CODEX_HOME` 并读到对方的会话历史，实时运行状态仍由各自的后端报告。在另一个后端运行的前台或 headless Codex 会话可能显示为 [Inactive](https://github.com/openai/codex/blob/rust-v0.170.0/codex-rs/tui/src/app/agents_overview_view.rs#L78-L102)。查看 AgentX 管理的运行状态用 `agentx dashboard`，查看受管 PTY 的当前画面用 `agentx term NAME`，进入原 tmux 会话用 `agentx attach NAME`。

排查前台 Codex 分派的后台 Agent 时，分别比较实际可执行文件、`CODEX_HOME` 和 Codex 后端。[嵌套启动](../../src/runtime/launcher.rs#L506-L510) 会先用干净的登录 Shell 环境加上显式环境来源重建子环境，父进程临时导出的 `CODEX_HOME` 因此可能不同；AgentX 也会从发起启动的命令环境重新查找 Codex。[tmux](../../src/runtime/tmux.rs#L85-L92) 执行的是已经生成的 runner，runner 会还原选好的环境和可执行路径。

即使版本与 `CODEX_HOME` 相同，Codex 0.170.0 仍会因 AgentX 终端启动注入的 [`-c hooks.*`](../../src/integrations/hooks/codex.rs#L234-L240) 或 [`-c developer_instructions`](../../src/integrations/launch_args/codex.rs#L153-L159) [跳过自动复用共享后端](https://github.com/openai/codex/blob/rust-v0.170.0/codex-rs/tui/src/daemon_startup.rs#L62-L125)。这个分支同样适用于前台终端和原生后台 PTY；将 tmux 换成 PTY 不会移除这些启动参数。

见 [同版本、同 home 的直接终端与 tmux 对照](../codex-activation/acceptance.md#codex-0170-terminal-sharing-diagnosis)。

普通终端的 Codex 请求不经过 AgentX 协议桥。隔离验证中，AgentX daemon 持续离线时，已经启动的 headless Codex 仍能在同一 tmux 会话完成用户发起的模型轮次；AgentX 消息和状态同步依赖其服务。[测试边界和共享后端直连方案](../codex-activation/acceptance.md#ordinary-headless-during-agentx-daemon-outage)与[终端共享任务](../../.controller/codex-terminal-sharing.task.md)记录实测范围和待接入条件。

Linux 上新建的兼容 Codex 0.170.0 终端默认直连共享 app-server。父 Agent 可用 `agentx launch --tool codex --run-here --go`，再派生 `agentx launch --tool codex --headless --terminal tmux --go`。同一个 `CODEX_HOME` 和兼容的后端配置才会共享。无法共享时，自动模式在创建原生线程前回到原有 PTY 路径，保留用户原有的 Codex 启动参数。`AGENTX_CODEX_TERMINAL=pty` 强制原有路径，`AGENTX_CODEX_TERMINAL=shared` 要求共享。AgentX 不会重启已有的原生 Codex daemon；已有会话的 resume/fork 保持原路径。

共享模式下，收件和 `agentx recover` 都按 Codex 线程投递；在窗格切换会话不会改变目标。Codex 的 `/new` 仍创建普通 Codex 会话；用 `agentx launch` 创建新的受管 Agent。通过 `agentx attach NAME` 回到 tmux。

## 构建与验证

`just package .bin/agentx-tmux` 构建独立候选程序，保留当前服务使用的 `.bin/agentx`。普通 `just package` 仍输出 `.bin/agentx`。运行 `python3 tests/tmux_headless.py .bin/agentx-tmux`，用临时数据目录、真实 tmux 和本地 Provider fixture 验证前台与后台启动、attach/detach、PID 连续性、配置覆盖及会话隔离。执行结果与平台范围以[交付任务](../../.controller/tmux-headless/task.md)为准。

候选程序的验证使用隔离数据目录。现有数据目录的 daemon 与其客户端编译产物绑定；如需切换现有运行版本，按 [daemon 状态与恢复](daemon.md#operator-status-and-recovery)操作。本任务的构建与测试不代表现有服务已切换。
