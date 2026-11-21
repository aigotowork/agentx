---
schema: document
status: active
owner: AgentX
scope: Inventory of prompt-like hints and guidance emitted by AgentX commands and message delivery
last_updated: 2026-11-17
related: [./README.md, ./providers.md, ../../src/cli/tips.rs, ../../src/messaging/tips.rs]
---

# 命令与 Hint 清单

这里的 Hint 指 CLI、Hook 或事件机制额外显示给 Agent 的操作提示。它们不是一个统一的 Prompt 字段：有的进入固定 Bootstrap，有的随消息进入模型，有的只是普通命令输出。替换 Bootstrap 只会影响第一类中的 builtin 部分。

## 生命周期与投递位置

| 形式 | 何时出现 | Agent 看到的位置 | 是否随 Bootstrap 替换消失 |
| --- | --- | --- | --- |
| Bootstrap 教学 | 新固定指引首次生成 | Provider 启动或上下文钩子 | 选择自定义 sources 且省略 builtin 时，是 |
| `[tip]` CLI tip | 条件命令成功后的首次提示 | 命令工具结果 | 否 |
| `hints` | 使用统一模型消息 formatter 的非空投递批次 | 消息正文尾部；Hook 通常以 `<agentx>` 块注入 | 否 |
| 接收 intent/thread tip | 首次收到某类消息/thread | 消息格式尾部 | 否 |
| 事件/launch 通知 | 匹配事件、启动完成或失败时 | 发件人为系统的 AgentX 消息 | 否 |
| 结果/诊断指导 | 命令输出、错误或 Hook 反馈时 | 普通工具结果；少数进入原生 context 字段 | 否 |
| Help/预览内容 | Agent 显式运行 help/config/preview 时 | 当前命令工具结果 | 否；它本来就不是自动 bootstrap |

上游是否把工具结果当成模型上下文，以及使用何种消息角色，由宿主工具定义。下表的“模型可见”仅表示 AgentX 把文字放进模型可消费的 Hook/输入或命令返回路径，不保证模型遵从。

## `[tip]` 命令提示

真实注册表在 [messaging/tips.rs](../../src/messaging/tips.rs)，首次使用存储和 launch 条件组合在 [cli/tips.rs](../../src/cli/tips.rs)。去重按 Agent instance 与 key 记录；正常 tip 在准确输出 flush 后标记为已看。Launch 候选在一次成功启动后合并输出。没有 Agent 身份的 caller 不保存 launch tip 的已看状态，因此下次调用仍可能显示。

| Key / 提示内容 | 触发条件 | 生效边界 |
| --- | --- | --- |
| `launch:notify`：AgentX 会在实例启动并 ready 后自动通知 | 启动成功；在 AI 工具里调用；launcher 已加入 AgentX | 一次；与实际启动提示同次输出 |
| `launch:start`：运行 `agentx start` 接收实例通知/消息 | 启动成功；在 AI 工具里调用；launcher 尚未加入 AgentX | 一次；不能把它误读成每个启动都需要手动 start |
| `launch:kill`：用 `agentx kill` 结束实例并关闭 pane | 启动成功；在 AI 工具里调用；当前 terminal preset 有 close 操作 | 一次 |
| `launch:term`：用 `agentx term` 查看 screen 或注入按键 | 启动成功；在 AI 工具里调用；非后台启动 | 一次 |
| `launch:sub-blocked`：订阅 Agent 被 approval 阻塞 | AI 工具中启动，且 terminal 是 tmux 或后台运行 | 一次；否则是下一项 |
| `launch:sub-idle`：订阅 Agent 变 idle | AI 工具中启动，且既非 tmux 也非后台 | 一次；与 `launch:sub-blocked` 互斥 |
| `list:status`：解释 active/listening/blocked/inactive/unknown 状态符号 | 被 launch tip 组合器选中时：启动成功且 caller 在 AI 工具里 | `agentx list` 本身不会必然打印；这个 key 在此调用点提供 |
| `launch:send`：如何给 Agent 发消息 | 启动成功；caller 在 AI 工具之外 | 一次 |
| `launch:list`：用 `agentx list` 看状态 | 启动成功；caller 在 AI 工具之外 | 一次 |
| `send:intent:request`：发送者期待回复，未回复时会有自动通知，可先继续别的工作 | 成功发送、由本地 Agent 身份发出、设置了 `intent=request` | 每实例/每 intent 一次；JSON 与 quiet 提前返回不显示 |
| `send:intent:inform`：不期待回复 | 同上，`intent=inform` | 每实例/每 intent 一次 |
| `send:intent:ack`：确认收到，对方不需要再回复 | 同上，`intent=ack` | 每实例/每 intent 一次 |
| `sub:created`：下一次匹配事件将以 AgentX 消息送达 | `agentx events sub` 新建成功；已有订阅路径不显示 | 每实例一次；订阅实际结果另有 stop/id 行 |

Launch 成功的同一输出还会显示 `[info] Terminal: MODE (managed/unmanaged[, auto-detected])`，它不是一次性 tip，每次该路径执行都可以输出。managed 判断依据 preset 是否有 close 操作，包含 kitty/wezterm/tmux/zellij/waveterm 与提供 close 的自定义 preset。

除了 `list:status`，registry 还定义了 `list:types` 和 `mention:matching`：前者解释自动投递工具标签、`*` 未绑定和 `[AD-HOC]`，后者说明本地/远端 `@name` 写法。但本次源码检索没有发现它们的生产 `maybe_show_tip` 调用点；目前不能描述为 Agent 执行 `list` 或 `send` 时一定会收到的 tip。接收消息的 `recv:*` 项在下一节单独按真实生产路径处理。

## 每批消息携带的提示

生产 Hook 路径经 [routing.rs](../../src/messaging/routing.rs) / [inbox_store.rs](../../src/messaging/inbox_store.rs) 格式化冻结消息。下面的配置 hints 与 recv tip 由 `format_hook_messages` / `format_messages_json` 加入；普通 `agentx listen` 和 adhoc CLI 使用 [cli/output.rs](../../src/cli/output.rs) 的另一套文本 formatter，不会在文本消息尾部额外拼接这些配置 hints/recv tip，但仍返回消息、目录来源和待收邮件提示。不能把所有命令输出都理解为统一 Hook 字符串。

| 消息附加内容 | 何时加入 | 规则 |
| --- | --- | --- |
| `AGENTX_HINTS` / `launch.hints` | 每个非空投递批次 | 作为 ` | [配置文字]` 附在该批末尾；可用于长期消息处理习惯，如“Always respond in JSON format” |
| 实例级 hints | 有针对接收实例的非空 hints 时 | 覆盖本次全局 hints；不会与全局文本叠加。清空实例值后回退到有效 scope 配置 |
| `recv:thread:<thread>` | 本批包含首次遇见的 thread 消息 | 显示加入 thread 以及 `agentx events unsub` 离开方法；优先于 intent tip，同批最多一条 tip |
| `recv:intent:request` | 本批含首次遇见的 request，且本批没有更早命中的 thread tip | 提示发送者期待回复 |
| `recv:intent:inform` | 首次收到 inform，且没有优先 thread tip | 发送者不期待回复 |
| `recv:intent:ack` | 首次收到 ack，且没有优先 thread tip | 发送者确认收到，不需要再回复 |
| `requested_directory` 元数据 | 目录路由消息 | 消息正文前加 `[Requested directory: PATH]`，不改用户正文 |
| 未读批次剩余数量 | 一次输出未覆盖所有未读消息 | CLI 输出 `[+N more unread — run: agentx listen --iam NAME]` |
| 输出期间又到达新消息 | 当前批次之后观察到新事件 | CLI 提示再次运行 listen；不是新的 message body |

实例级 `hints` 可通过 `agentx config -i NAME hints TEXT` 设置，清空用空字符串。全局值用 `agentx config hints TEXT` 或 `AGENTX_HINTS`。前者跟某个实例记录，后者按调用端 scope 配置。消息配置不冻结进已有 Agent 的固定 instructions，因此两者的生效时间不同。

单条 Hook 消息通常是 `intent/thread/event-id sender → recipient: body`；多条为 `[N new messages] | ...`。CLI listen 有自己的易读 formatter。`format_messages_json` 添加 `<agentx>...</agentx>` 包装；Claude 还可能把同一内容分为人可见 `systemMessage` 与模型 `additionalContext`。这不把邮件内容并进 system/bootstrap source 列表。

接收 tip 的“已显示”状态由已准备的消息批次快照维护，避免重放时提示状态与冻结消息内容漂移；它和 CLI 的“先 flush、再标记 tip”事务时序不同。相同 prepared batch 的重放不会按新配置重新拼一次 hints。

## 事件与生命周期通知

| 通知 / 文本 | 触发场景 | 进入哪条路径 |
| --- | --- | --- |
| `[agentx-launcher] @launcher All N instances ready: ... (batch: ID)` | 某启动批次全部进入 ready | 系统消息；可按通常 mailbox Hook 自动投递，或由 `listen` 取出 |
| `[agentx-launcher] @launcher Launch failed: NAME: ... (batch: ID)` | 启动阶段实例失败 | 系统消息；给启动者诊断具体实例与 batch |
| `[agentx-events] [sub:ID] #event ...` | 已订阅事件匹配 | 系统消息。正文依事件类型含 status、来源、消息摘要等 |
| `[sub:ID] #event NAME went idle/stopped without responding to your request #request` | request 的收件 Agent 变 idle 或退出且请求未完成 | request watch 通知；回复可关联/匹配后撤销观察 |
| `⚠️ COLLISION ... both edited PATH` | 文件碰撞订阅匹配 | 事件通知说明可能共同编辑同一文件 |
| `[Task tool completed - Message history during Task tool]` | Claude 原生 Task 子 Agent 完成且期间有消息 | Claude PostToolUse 的模型附加上下文；列出子 Agent 与消息，不是主邮箱未读确认 |
| `[agentx] new message(s) arrived — run: agentx listen --iam NAME` | 发送命令或 listen 输出完成后又发现新消息 | 普通 CLI 反馈，不属于订阅事件本身 |

`agentx events sub` 新建成功后还有本地 `sub:created` 一次性 tip。启动结果另外可能输出 batch ID 与 `agentx events launch ID` 等待建议。Launch 命令成功与子 Agent 成为 ready 是两个不同时间点；完整条件见 [launch_status.rs](../../src/runtime/launch_status.rs) 和 [db/events.rs](../../src/db/events.rs)。

## 预览、上下文与命令结果

以下是 Agent 执行命令后可能作为工具结果看到的操作引导。它们由命令参数、状态与结果决定，不经过 Prompt source 合成：

| 命令/场景 | 返回的说明或动作 | 触发/限制 |
| --- | --- | --- |
| `agentx launch` 预览 | `Add --go to proceed.`，显示 tool、count、模式、目录、terminal 和参数来源 | 预览而非实际启动；JSON 的 `hint` 字段同义 |
| `agentx send` 广播预览 | 提醒无 `@targets` 会广播、列 recipient 与影响，并要求明确 `--go` | AI 工具中，非 `--go`，较多 recipient 且仍是未解析的广播 |
| `agentx launch` 启动结果 | 建议用 `agentx events launch BATCH` 等待全部 ready/fail | 启动成功且当前路径未内联等待 |
| `agentx config bootstrap show/render` | 展示 builtin 模板条件分支或 builtin renderer 预览 | 不是该实例最终冻结内容；render 不含自定义 sources、Role、additions |
| `agentx transcript` | 按 flags 提示还有 `--full` / `--detailed` 中哪些能显示被省略内容 | 输出确有 transcript 内容且选择的格式有所省略；JSON 路径不附该文字 |
| `agentx bundle prepare` | 说明如何选 transcript/event/file、用完整细节并创建/发送 bundle | 非 compact 人类输出；Compact 隐藏说明区 |
| `agentx list` | 空列表但 scope 有归档时提示 `agentx archive` | 有归档目录中的 session snapshot 时 |
| `agentx stop` / `kill` | 提供 `agentx r NAME` 恢复命令 | 对应成功或已停止的输出路径 |
| workflow `run` | 工作流自带 usage、分阶段的任务 prompt 与 AgentX 命令 | 各脚本自有功能；`debate`、`confess`、`fatcow` 会构造独立 role/system/task 文本，不是公共 Bootstrap |
| `help` / `--help-all` | 对应命令精确参数与选项 | 明确查询时返回；Bootstrap 只让 Agent 在不确定/需恢复时按需查帮助 |

工具结果还可能承载真正的用户任务文本、transcript、事件记录、脚本输出或 bundle 文件，不应统称为 Hint。特别是绑定 Task 是用 `--bind` 关联文件，不等于该文件全文跟随每条命令结果自动送给模型；初始 prompt 一般会明确要求读取它。

## Provider 与身份交接指导

| 提示 | 出现场景 | 受什么控制 |
| --- | --- | --- |
| `When done, send your result back to @... via agentx.` | Agent 发起的普通初始任务，且允许 handoff | `bootstrap_exclude=handoff` 可去掉；它在任务尾部，未放进 source 列表 |
| Resume/Fork 身份重置 | 恢复原 session 或新身份继承历史 | 恢复入口按 Provider 构造；不由 instruction source 替换控制 |
| `Your agent ID` / `Your agentx parent` / 先运行 start | Claude 原生 SubagentStart | Claude 生命周期 admission 后发出，随后是专属 subagent bootstrap |
| `ANTIGRAVITY_DELIVERY_ACTION` | Antigravity 的 BeforeAgent 有待投递消息 | 随投递加入，强调 request 同轮完成；独立于 builtin 排除 |
| `SHARED_CODEX_BOOTSTRAP` | Codex shared service 配置 | 每线程身份前的共享说明；独立于固定 sources 的 builtin 选择 |
| Cursor/Copilot 裸 `<agentx>` wake 解释 | 相应工具的 builtin 自动投递引导 | 该段在 builtin 内，`messages` 排除会移除；实际 wake 与 mailbox 不因此停止 |

精确通道和原生限制见 [Provider 附录](providers.md)，排除项覆盖范围见[主文档](README.md#内置-bootstrap-的模块)。

## 错误诊断和恢复提示

错误消息是按失败状态生成的指导，不等同于固定 Prompt。下面列出本次检索到的代表性生产族：

| 失败条件 | 诊断提供的恢复方向 | 来源 |
| --- | --- | --- |
| 当前工具没有默认 launch profile，且无法从 tool 解析 | 直接提示 `--tool`、`--profile` 或配置 `agentx profile default`；正常 Bootstrap 不要求预查这些内容 | [launch_profile.rs](../../src/runtime/launch_profile.rs) |
| `agentx --iam NAME ...` 不匹配当前绑定，或 session 过期/无身份 | 提示正确绑定的 start/reconnect；native 身份以 session 为准 | [cli/client.rs](../../src/cli/client.rs)、[identity.rs](../../src/agents/identity.rs) |
| 把外部发送者当成本地 participant | 说明 cron/script/manual alert 应走 `agentx send --from NAME ...` | [router.rs](../../src/cli/router.rs)；当前有生产 inline 提示，另有 cfg(test) helper |
| 私有 relay 凭证被拒 | 按是否已配置密码提示 `agentx relay connect ... --password` | [relay.rs](../../src/cli/commands/relay.rs) |
| terminal/session 创建被工具环境限制 | 按失败对象提示 escalation/approval；tmux 情况会给出专用恢复步骤 | [terminal/mod.rs](../../src/runtime/terminal/mod.rs) |
| Claude managed setting 禁止 native `--plugin-dir`，或 native mod 停止响应 | 指向 `agentx config claude_delivery inject`，并说明重启/日志诊断入口 | [claude/native.rs](../../src/integrations/hooks/claude/native.rs) |
| launch 部分失败、blocked 或超时 | 返回 ready/failed/blocked 数与实例详情；超时提示日志目录或 `agentx list -v` | [launch_status.rs](../../src/runtime/launch_status.rs) |
| unknown subcommand/key、无效 mention、模糊 target 或少分隔符 | 在相应错误后给出近似值、`@` 用法或 direct/broadcast 语法 | CLI 各命令 parser 与 `shared/suggest.rs`；依错误条件出现 |

上表不是所有错误句子的逐字词典：parser validation 和 Provider 自身报错种类很多。其组织规则是针对具体错误给最近的修复入口，不能据此推导会在每次运行前多发一次 Help 命令。

## 不应误算为已投递的提示

- `messaging/tips.rs` 中存在文案不等于运行时一定调用。`list:types`、`mention:matching` 本次没有找到生产发射点；测试中的 `maybe_show_tip` 也不算生产路径。
- `agentx list` 的状态解释 tip 不是 list 命令尾部提示，而是 launch tip 候选中的一项。
- 工作流脚本里写给新 Agent 的 `--agentx-system-prompt` / `--agentx-prompt` 是工作流任务内容；它们不等于 AgentX 的默认引导。
- Hook 收件、PTY `<agentx>` wake、CLI 命令文本可能是不同通道。裸 `<agentx>` 在部分 Provider 只是唤醒标记，不是消息正文。
- 自动附带消息不会因为配置了 `AGENTX_INSTRUCTION_SOURCES` 而消失；也不能用 `bootstrap_exclude=all` 屏蔽 mailbox、Help、诊断和预览输出。

## 提示生产者索引

| 文本族 | 权威源码 / 调用方 |
| --- | --- |
| 首次命令 tip、launch 条件提示、状态图例 | [cli/tips.rs](../../src/cli/tips.rs)、[messaging/tips.rs](../../src/messaging/tips.rs)、[launch.rs](../../src/cli/commands/launch.rs) |
| send intent tip 与广播预览 | [send.rs](../../src/cli/commands/send.rs) |
| 创建订阅与 sub tip | [events_client.rs](../../src/cli/commands/events_client.rs) |
| 消息 hints、intent/thread tip 与冻结格式 | [routing.rs](../../src/messaging/routing.rs)、[inbox_store.rs](../../src/messaging/inbox_store.rs)、[service_store.rs](../../src/messaging/service_store.rs) |
| 目录来源、普通 CLI 收件与 backlog | [formatting.rs](../../src/messaging/formatting.rs)、[cli/output.rs](../../src/cli/output.rs)、[cli/client.rs](../../src/cli/client.rs)、[listen_client.rs](../../src/cli/commands/listen_client.rs) |
| Event/request/collision 通知 | [db/subscriptions.rs](../../src/db/subscriptions.rs) |
| Launch ready/failure 消息 | [db/events.rs](../../src/db/events.rs) |
| Claude Task 历史与 SubagentStart 指导 | [claude/render.rs](../../src/integrations/hooks/claude/render.rs)、[claude/client.rs](../../src/integrations/hooks/claude/client.rs) |
| Transcript 省略说明 | [transcript/shared.rs](../../src/transcript/shared.rs)、[transcript/client.rs](../../src/transcript/client.rs) |
| Bundle 操作说明与 CREATE 模板 | [bundle.rs](../../src/cli/commands/bundle.rs) |
| Workflow 专属任务内容 | [debate.sh](../../src/workflows/bundled/debate.sh)、[confess.sh](../../src/workflows/bundled/confess.sh)、[fatcow.sh](../../src/workflows/bundled/fatcow.sh)、[onidle.sh](../../src/workflows/bundled/onidle.sh) |
