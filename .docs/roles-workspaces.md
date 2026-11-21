---
scope: Directory roles, stable workspace teams and default-recipient messages
status: active
owner: agentx
last_updated: 2026-11-17
related: ["./launch-profiles.md", "./agent-context.md", "./proposals/roles-workspaces.md", "../.controller/roles-workspaces/delivery.md"]
---

# 角色与目录团队

Role 是固定职责指令和可选的 profile 引用。Workspace 是稳定的目录协作归属；同一 workspace 的运行实例自然组成团队。角色定义的来源、实例归属和进程工作目录分别记录，使用全局角色不会让实例变成全局成员。

PTY 终端标题和终端标签使用 `状态 名字 [role] · title` 的身份部分，例如 `● saki [coder] · 优化终端标题`。角色来自实例启动时采用的快照，title 来自 `--title`；缺少时省略对应括号或分隔符。Label 模式继续追加 `[工具]`，Combined 模式继续追加工具的实时标题，Passthrough 模式保持工具原标题。这里的角色与职责展示不改变 `@名字` 寻址。源码实现需由新启动或恢复的 PTY 宿主采用；替换程序不会更新已有宿主。

原有 TOML 角色实现已通过独立验收，其项目程序位于 `.bin/agentx`（`agentx 0.7.31`）；构建标识、逐项证据和支持范围见 [Delivery](../.controller/roles-workspaces/delivery.md#accepted-local-outcome) 及 [引导验证](../.controller/roles-workspaces/tasks/guidance.md)。本文中的 `agentx` 表示该程序。系统安装与正在运行的服务保持原状；采用新版本时需明确选择程序及其 daemon 作用域。

## 从配置到第一次目录消息

先确认使用当前仓库构建的程序；在仓库根目录可设置 `export PATH="$PWD/.bin:$PATH"`。如果选定作用域的 daemon 仍运行旧程序，先查看 `agentx daemon status`，再用 `agentx daemon restart` 明确更新它；仅替换客户端程序不会更新已经运行的 daemon。以下示例中的 `/projects/app` 替换成真实项目目录。

安装引导包后，`.agent-guides/roles/` 中的角色可以直接使用。只需先选好真实的默认 profile：

```sh
agentx profile create team-codex --tool codex
agentx profile default team-codex
agentx roles --dir /projects/app
agentx roles show coder --dir /projects/app --json
agentx launch --dir /projects/app --role coder --dry-run --json
agentx launch --dir /projects/app --role coder --headless --go
```

默认 profile 提供工具、模型和参数，角色提供职责指令。不需要为每个已发布角色创建 TOML 包装。如果还需要目录消息负责人，在已有 `controller.md` 的同一项目创建一个覆盖文件：

```toml
# /projects/app/.agentx/roles/controller.toml
default = true
```

然后运行 `agentx send --dir /projects/app --dry-run` 查看路由，或运行 `agentx send --dir /projects/app --intent request -- "请处理这项需求"` 提交消息。目录发送复用或创建该角色的一个实例；普通 `launch --role` 每次创建新实例。默认 profile 和默认收件人是两项独立配置。Profile 的模型和其他参数见 [profile 用法](launch-profiles.md)。

## 从哪里发现这些能力

| 入口 | 能看到什么 |
| --- | --- |
| 新 Agent 的内置启动指引、原生子 Agent 指引 | 角色发现、角色启动、团队列表、目录发送和预览的直接示例 |
| `agentx --help` | 常用目录协作命令，以及角色配置入口 |
| `agentx roles --help` | Markdown 发现、可选 TOML、profile 默认、禁用、命名空间和完整使用流程 |
| `agentx launch --help`、`send --help`、`list --help` | 参数、目录选择、默认负责人路由和相应用例 |
| `agentx run docs --cli`、`agentx --help-all` | 复用上述命令声明的完整参考，不需要访问仓库文件 |
| `agentx config bootstrap show capabilities` | 内置启动指引的源文本 |
| `agentx config bootstrap render NAME TOOL launched` | 当前程序对指定 Agent/Provider 的启动指引预览 |

更新程序后新创建的 Agent 会得到新指引；已有会话的固定指令不会因为更新程序、resume 或 compact 自动改写。让已有 Agent 运行 `agentx run docs --cli` 或 `agentx roles --help` 即可读取当前用法。显式排除了 `capabilities` 或自定义替换了内置指引的配置，继续尊重原有选择。引导内容的实际构建与输出检查见 [guidance Task](../.controller/roles-workspaces/tasks/guidance.md)。

## 定义与使用

| 来源 | 定义方式 | 作用 |
| --- | --- | --- |
| `<workspace>/.agent-guides/roles/NAME.md` | YAML 元数据声明 `schema: role` | 引导包发布的通用职责 |
| `<workspace>/.agentx/roles/NAME.toml` | 可选覆盖或完整自定义角色 | 项目参数、替换指令、默认负责人、禁用 |
| `$AGENTX_DIR/roles/NAME.toml` | 全局 TOML 定义 | 显式 `global:NAME` 或祖先之后的回退 |

只扫描角色目录的直接文件，支持 `NAME.md` 和 `OWNER.HELPER.md`，不递归扫描。没有 `schema: role` 的参考 Markdown 不作为角色；声明了 `role` 时必须与文件名一致。`description` 用于目录展示，其余元数据不作为启动参数。Markdown 示例：

```markdown
---
schema: role
role: coder
description: Implement changes through the project contracts.
---

# Coder

Read the project AGENTS.md before implementing the assigned change.
```

文件名就是角色名。同一个 workspace 内 TOML 优先于 Markdown，最近 workspace 的 Markdown 优先于祖先或全局 TOML。`local:NAME` 只选择当前 workspace，`global:NAME` 只选择全局 TOML。不跨 workspace 合并字段。

```toml
# .agentx/roles/coder.toml — only if this role needs a different profile
profile = "codex-deep"
```

省略 `prompt` 和 `prompt_file` 时，只继承同一 workspace 的同名 Markdown 指令及描述；显式 `description` 替换描述。显式 `prompt` 或 `prompt_file` 完整替换指令，不拼接；二者最多指定一个。没有同名 Markdown 时必须提供一个指令字段，兼容已有完整 TOML 角色。`prompt_file` 相对 TOML 文件解析。文件正文按字面读取，不执行模板或脚本；指令前附规范化绝对来源路径，正文中的相对引用以来源文件父目录为准。内联 `prompt` 保持原样。

省略 profile 时复用启动选择规则：显式 `--profile` > 角色显式 profile > 既有工具默认、`AGENTX_PROFILE`、作用域默认的选择规则；显式工具仍屏蔽环境 profile。`default` 不是特殊 profile 名。`roles` 和 `roles show` 不要求已配置有效默认 profile，JSON 中未指定的 profile 是 `null`；启动角色时必须解析到真实命名 profile，否则明确报错。已采用记录始终保存实际 profile 和指令快照。

```toml
# .agentx/roles/unwanted.toml
disabled = true
```

`disabled = true` 必须是文件中的唯一设置，屏蔽该名字及祖先、全局回退；删除覆盖文件会重新露出共享角色。选中的无效 TOML 或 Markdown 报错，不静默降级。`--all` 可显示普通遮蔽定义，但不会复活已禁用的名字。

最多一个公共角色 TOML 可以声明 `default = true`；不从 `controller.md` 文件名推导默认负责人。目录默认查找保留既有语义：使用声明默认的祖先 workspace 的确切定义；子目录普通同名角色不改变祖先默认路由，显式禁用会阻止该名字回退。默认入口已绑定运行实例时，保留其 TOML 来源路径；更换来源需走既有退休及 `roles reset-endpoint` 流程。

## 角色专属辅助角色

文件名 `OWNER.HELPER.md` 或 `OWNER.HELPER.toml` 表示属于某个 Role 的辅助角色，例如与 `fronter.toml` 放在同一个角色目录的 `fronter.e2e.toml`。只支持两级；两段分别沿用字母、数字、下划线和连字符规则，且以字母或数字开头。辅助角色有自己的指令和可选 profile，不继承父角色指令，也不能声明 `default = true`。Markdown 和 TOML 可以混用，只要属于同一逻辑 workspace。

```toml
# <workspace>/.agentx/roles/fronter.e2e.toml
profile = "team-codex"
description = "验证前端交付的端到端行为"
prompt = "Run end-to-end checks for the assigned frontend delivery and report evidence."
```

| 入口 | 发现规则 |
| --- | --- |
| 普通 shell 的 `agentx roles` | 当前目录作用域内的公共角色 |
| Agent 的 `agentx roles` | 公共角色，加上与其采用的 Role 同名归属、同来源命名空间的辅助角色 |
| `agentx roles --iam NAME` | 显式按已有 Agent 的采用记录发现；一般无需指定身份 |
| `agentx roles --all` | 当前目录搜索范围的全部角色，包括辅助角色和被遮蔽定义；不会扫描其他项目 |
| `agentx roles show fronter.e2e` | 所有人均可显式查看 |
| `agentx launch --role fronter.e2e` | 所有人均可显式启动，沿用已有目录查找和 local/global 选择器 |

发现过滤在现有同名遮蔽规则之后应用，保证列表中的短名与显式启动解析一致。被近处同名定义遮蔽的助手不会自动显示；可用 `--all` 检查，并通过 `global:fronter.e2e` 或明确来源 workspace 查看和启动。

这是发现隔离，不是权限或文件保密机制。项目 A 的 `fronter` 不会因为同名而自动发现项目 B 的助手；全局 `fronter` 只自动发现全局同来源的助手。采用 `fronter.e2e` 的 Agent 不自动获得父角色的其他助手。多个 `fronter` 实例共享同一发现规则，实际启动的上下游关系继续独立记录。

默认列表通过已有 daemon 的只读身份解析获取采用记录，并优先采用持久 workspace；显式 `--dir` / `--workspace` 仍优先。该查询不会启动 daemon。隐式身份不可用时向 stderr 说明原因并退回公共列表；显式 `--iam` 解析失败则报错。`show`、`--all` 和无身份查询只读取本地文件，按显式目录、环境或 cwd 确定搜索范围。隐藏助手的正文错误不会影响公共列表或目录默认入口；显式检查、所有者发现及 `--all` 会报告这些错误。管理页面的公共 Role 目录继续使用公共发现视图。

本次实现、构建和实际检查状态见 [辅助角色 Task](../.controller/roles-workspaces/auxiliary-roles.task.md)；既有已安装程序和运行服务的采用状态与本次源码变更分别记录。

## 归属、目录与发现

新 Agent 的启动指引只给出一套最常用的任务分派命令，直接使用已配置的默认值：

```sh
agentx launch --go --headless --title "任务标题" --bind ".controller/xx/xx.task.md" --agentx-prompt "任务要求；请读取绑定的 Task"
```

替换标题、Task 路径和任务要求即可执行。已知合适角色时添加 `--role NAME`；普通分派无需先查询 profile、角色详情或命令帮助。缺少默认配置时，先由启动命令报错，再按诊断修复；只有明确的配置任务或错误恢复才进入 [profile 配置](launch-profiles.md)。`default = true` 控制目录消息的负责人选择；普通 `launch` 不会因此自动采用这个角色。

Workspace 与 cwd 分别解析。[本次设计](proposals/workspace-discovery.md)已由用户选定；[实现 Task](../.controller/roles-workspaces/tasks/workspace-discovery.task.md)记录实际源码、构建与采用状态。

| 优先级 | 输入 | 规则 |
| --- | --- | --- |
| 1 | 显式 `--workspace PATH` 或 `global` | 使用明确归属；`global` 表示明确不加入目录 workspace |
| 2 | 显式 `--dir PATH` | 先沿祖先链找最近 Git 根；没有 Git 时找最近项目 `.agentx/` 或已有 `.agent-guides/roles/` 边界 |
| 3 | 隐式 `AGENTX_WORKSPACE` | 继承路径或 `global`；已绑定 Agent 先用持久化归属校准此值 |
| 4 | 调用者 cwd | 按同一规则找 Git / AgentX 项目根 |
| 5 | 都未命中 | 个人默认 workspace `~/.agentx` |

显式目录没有项目边界时继续向后查找；无效目录和无效显式 workspace 报错，不静默回退。Git 优先级覆盖整条祖先链：仓库内更深的 `.agentx/` 不会自动把团队拆开。`.git` 可以是目录，也可以是 linked worktree / submodule 的有效 `gitdir:` 文件；默认归属是当前 checkout/worktree 根，不是共享 Git 元数据目录。角色查找仍使用所选 workspace、祖先及全局来源的既有规则。

项目 `.agentx/` 标记其父目录为 workspace。个人 `~/.agentx` 是兜底锚点，其存在不会把整个 HOME 变成项目 workspace；`AGENTX_DIR` 继续独立选择 daemon/数据库，不替换个人默认 workspace。发现与只读角色查询不创建目录，个人兜底路径尚未存在也可被返回并传递。显式 `--workspace` 仍选择已有目录，不要求该目录包含角色定义。

`--dir` 指定启动工作目录；未指定时保留调用者 cwd。`--workspace` 仅选择协作归属与角色搜索上下文，不隐式改变 cwd。可以组合使用：

```sh
agentx launch --workspace /projects/app --dir /projects/app/packages/web --role fronter --headless --go
agentx list
agentx list --dir /projects/other
agentx list --workspace /projects/app --json
agentx --dir .
agentx attach --dir .
agentx list --all --json
```

已有 Agent 改变 cwd 不会换队；普通子启动继承团队，显式 `--dir` 命中另一个项目时，新 Agent 加入目标项目的 workspace。目标目录没有边界则继续继承原归属。每个新 Agent 环境都注入解析后的 `AGENTX_WORKSPACE=绝对路径` 或 `global`，并在原生启动记录中保存同一归属。已绑定 Agent 的后续命令以持久记录为准，环境变量仅传递默认值，不能证明身份。旧 Agent 的已有归属和 resume/fork 历史不因这次发现规则变化而重算。

裸 `agentx` 保留 TUI 总览；`agentx --dir .` 打开按团队筛选的 Agent 列表，时间线仍显示整个数据范围。TUI、`list` 和 `attach` 可组合使用目录、workspace 和当前 Agent 的 label 条件。`attach` 只显示可连接的本地 tmux Agent；TUI 启用 label 筛选时只显示有当前标签数据的 Agent。

默认列表包含当前团队和标明 global 的全局成员；不递归包含子团队。`--all` 查看当前 AgentX 作用域内的全部团队。明确查询实例名称仍可跨团队；目录选择与明确收件人组合时检查归属。未携带新元数据的历史实例视为全局。

## 给目录负责人发消息

```sh
agentx send --dir /projects/app --dry-run
agentx send --dir /projects/app --intent request -- "请处理这项需求"
agentx send --workspace /projects/app --json -- "补充验收条件"
```

每个角色目录最多一个 `default = true`。当前目录没有默认声明时，逐级查找父目录，最后查全局。找到父目录负责人时消息交给父目录的入口，保留原始目标目录作为独立消息上下文；不会为每个子目录创建父级 controller。没有任何默认声明则报错。

入口已有实例时复用；正在忙时沿用现有邮箱。没有入口时原子预留一个角色实例再启动，并发首次发送不会创建多个负责人。若有多个可兼容的既有实例且尚未建立入口绑定，报歧义并要求明确收件人。启动失败、模型错误和结果不明不会触发盲目替换或重复投递。

普通未绑定 shell 的目录消息默认署名 `bigboss`；已有 Agent 保留其真实身份。显式 `--from` 继续可用。普通 `send @NAME`、线程和不带目录的广播语义保持不变。

JSON 回执包含消息事件、实际收件人和 `directory_route`（请求目录、请求/实际 workspace、角色来源、是否创建）。成功表示消息已持久接收，不代表任务已完成。接收端能看到原始目标目录，存储的正文保持原样。

更换默认角色不会自动替换已有入口。先通过现有生命周期操作明确停止旧实例，再重置原入口所属 workspace：

```sh
agentx kill INSTANCE
agentx roles reset-endpoint --workspace /projects/app
```

daemon 只允许重置有明确停止证据的绑定；存活、失败但未退役、结果不明的实例会阻止重置。父目录回退时需选择实际负责的父 workspace。

## 验证边界

本轮首先实现本地 Linux 终端/后台路径。远程 fresh launch 的 `--role`/`--workspace` 及共享 Codex app-server stdio 的角色/目录工作区启动明确拒绝；不会静默忽略参数。新发现规则默认产生目录 workspace，因此共享 app-server 不会自动退回 global；该宿主的 workspace 支持仍未实现。原有已解析为 global 的 stdio 路径保留原语义。平台和 Provider 实际执行范围以 [验收记录](../.controller/roles-workspaces/tasks/acceptance.md) 与 [本次 Task](../.controller/roles-workspaces/tasks/workspace-discovery.task.md) 为准。

新客户端在启动角色或目录工作区前会确认 daemon 支持这些能力；旧 daemon 会被明确拒绝，避免忽略新字段。构建新程序不会自动替换正在运行的 daemon。采用新版本时，使用现有 `agentx daemon status` 和 `agentx daemon restart` 管理明确选定的作用域；也可以先设置独立 `AGENTX_DIR` 验证，并在该作用域创建所引用的 profiles。

## 约定角色实现与维护

本次约定发现、薄覆盖、禁用及默认 profile 组合的设计和实际证据见 [维护入口](../.controller/convention-roles/AGENTS.md) 与 [实现 Task](../.controller/convention-roles/implementation.md)。Config 的 RoleDefinition 表达当前可编辑定义，runtime 在实际启动时转为具备真实 profile 的 ResolvedRole。CLI 与管理端共用选择逻辑；Web 目录中的“默认启动配置”表示尚未冻结 profile，不替换已采用 Agent 的配置。更新指引不会重写运行 Agent 的固定指令。

当前项目已采用 `.bin/agentx` 的约定角色版本，八个共享角色可发现，`.agentx/roles/` 仅保留 controller 的默认收件人声明。准确构建、隔离 CLI/HTTP 检查和运行服务边界见 [完成记录](../.controller/convention-roles/implementation.md#completed-local-outcome)。
