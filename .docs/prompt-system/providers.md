---
schema: document
status: active
owner: AgentX
scope: Source-level map of Provider prompt channels used by AgentX
last_updated: 2026-11-17
related: [./README.md, ./hints.md, ../agent-context.md, ../../src/integrations/registry.rs]
---

# Provider Prompt 投递通道

AgentX 合成 Prompt 后，不会要求所有工具用同一种 system-prompt API。适配器按上游原生契约，把固定指引、动态快照、首条业务请求、消息邮件放进不同字段或时点。下表是源码集成地图；生命周期 Delivery 记录的测试范围才是执行证据，不能仅凭存在适配器代码推断特定已安装版本或模型实际遵从。

## 通道矩阵

| Provider / 接入 | 固定 AgentX 指引 | 动态上下文 | 主要限制与边界 |
| --- | --- | --- | --- |
| Codex 普通启动 | 启动 argv 的 `developer_instructions`；AgentX 内容在用户已有 developer instructions 前，沿用 Codex 基础 system instructions | SessionStart / 首次模型边界与 compact SessionStart 的独立 `additionalContext` | 动态内容不混入任务用户消息；保留 `CODEX_HOME/config.toml` 已发现的设置与显式 argv，不等于完整发现系统/项目配置 |
| Codex shared app-server | 先在 host config 追加 `SHARED_CODEX_BOOTSTRAP`；线程第一次输入 Hook 再带该线程具体冻结指引 | 精确线程附件及首次输入/compact Hook | 服务进程没有单一 Agent 身份；每个线程绑定自己的 Agent。必须走原生 turn 协议，不能用普通 `--agentx-prompt` shared stdio |
| Claude Hook | Hook `additionalContext` / Stop reason 等宿主定义的可见输出 | SessionStart initial/resume/compact 与后续支持的 model/tool boundary | Task 子 Agent 使用独立 `SUBAGENT_BOOTSTRAP`；邮箱的模型 context 和 UI 展示可以分开，PostToolUse 会显示 Task 子 Agent 消息历史 |
| Qoder | `additionalContext` | SessionStart（包括 compact/clear）和 UserPromptSubmit 边界 | compact/clear 语义按 Hook source 判断；实例权限、消息确认仍单独处理 |
| Gemini CLI | `additionalContext` Hook 文本 | 初始支持边界和后续 BeforeAgent 边界 | 不据此声称有独立的原生 post-compact 事件 |
| Antigravity | 与 Gemini Hook 共用 builtin 固定文本 | pending message / BeforeAgent 边界 | 每次投递还可先加 `ANTIGRAVITY_DELIVERY_ACTION`：request 必须同轮处理并回信；ack 本身不算完成 |
| Cursor | SessionStart additional context | BeforeSubmitPrompt 刷新 | 无独立 compact Hook 的源码声明；消息投递规则额外解释 wake trigger |
| Copilot | SessionStart `additionalContext` | UserPromptSubmit 刷新 | Hook 生命周期由 Copilot 适配器控制；`systemMessage` 不应自动当作高优先级 system role |
| Kimi | 不能在启动时传入 AgentX 初始 prompt；通过 UserPromptSubmit additional context 恢复固定文字 | UserPromptSubmit 刷新 | SessionStart Hook 输出本身不会进入模型上下文；启动业务任务要在 launch 后用 `agentx send` |
| Pi plugin | 原生 append-system slot；若 host 强制 system prompt，则回退 hidden/model context | request-local hidden context；session_compact 重新取快照 | append 保留原 system prompt；compact 后插件恢复固定文字与新快照，不声称 hidden text 具有 system 优先级 |
| OMP plugin | hidden custom model context | 主 Agent 的 request-local context 与 session_compact refresh | 不替换 host system prompt；仅主 Agent 上下文，不把其他 worker 强行归为 AgentX participant |
| OpenCode / Kilo plugin | v1 `experimental.chat.system.transform` 追加 system array；v2 system-array text | v1 synthetic context part / compaction；v2 request/compaction context | 插件 version path 不同；synthetic/hidden 内容不是 developer role 保证 |
| Hermes Hook | `pre_llm_call` context | `pre_llm_call` 每次模型调用刷新，包括压缩后的下一调用 | 由 Hermes Hook 的 input/output 契约决定合成位置 |
| Grok launch | 固定指引通过 `--rules` 注入 | 不支持独立动态通道 | 若解析出的动态 sources 非空，launcher 显式拒绝；将长期固定内容放进 instructions sources |
| adhoc participant | 工具结果中显示 `<agentx_system_context>` 固定文本 | adhoc 本身没有默认 native model context | 使用者手动启动 AgentX 后，从命令输出/listen 收到内容；完整读取命令结果很重要 |

来源导航：通用生命周期在 [hooks/client.rs](../../src/integrations/hooks/client.rs)、[hooks/output.rs](../../src/integrations/hooks/output.rs)、[hooks/common.rs](../../src/integrations/hooks/common.rs)；Claude、Gemini、Qoder、Cursor、Copilot、Kimi、Hermes 的 Hook 分支分别见 [claude/client.rs](../../src/integrations/hooks/claude/client.rs)、[gemini.rs](../../src/integrations/hooks/gemini.rs)、[qoder.rs](../../src/integrations/hooks/qoder.rs)、[cursor.rs](../../src/integrations/hooks/cursor.rs)、[copilot/client.rs](../../src/integrations/hooks/copilot/client.rs)、[kimi/handlers.rs](../../src/integrations/hooks/kimi/handlers.rs)、[hermes/registry_client.rs](../../src/integrations/hooks/hermes/registry_client.rs)；Codex native Hook 在 [hooks/codex/client.rs](../../src/integrations/hooks/codex/client.rs)，插件实现分别见 [Pi](../../src/integrations/plugins/pi/agentx.ts)、[OMP](../../src/integrations/plugins/omp/agentx.ts)、[OpenCode](../../src/integrations/plugins/opencode/agentx.ts)，初始 prompt 入口形状在 [registry.rs](../../src/integrations/registry.rs)。项目现有使用表及其已知限制见[Agent context 使用说明](../agent-context.md#native-delivery-and-supported-boundaries)。

## 固定文本与快照如何进入 Hook

通常 Hook client 先通过生命周期凭据确认当前 participant/session，再读取冻结 instructions 或请求 initial/compact context。Codex 是明确拆开的例子：普通 SessionStart 先准备 initial dynamic context；compact SessionStart 准备新快照；shared Codex 会合并固定文字与动态部分放进精确 thread context。其他 Hook 若只提供单个 additional-context 通道，compatibility helper 可将固定与动态内容以空行合并，固定文本仍来自已保存快照。

这与 Mailbox 不同。收件箱由独立的 prepare/freeze/flush/confirm 流程提供；context refresh 不应顺便 ack 邮件。适配器可能在一次 Hook 输出中同时给出 bootstrap 和消息，但二者由不同 receipt 管理。Claude 的模型消息常包 `<agentx>`，可见 UI 的 `systemMessage` 可能是另一个字段；两个字段的出现不代表相同展示/优先级。

Provider 自己还可能加载 base system prompt、用户设置、项目文件、commands/skills 或 native agents。AgentX 的固定 source 选择仅管 AgentX 这一份已命名贡献，不能取代这些 Provider 拥有的上下文。

## 两个 Codex 启动入口

普通 CLI 启动在 [launch_args/codex.rs](../../src/integrations/launch_args/codex.rs) 合成 Codex `developer_instructions`：AgentX 固定指引在前，显式或既有用户 developer instruction 在后，中间分隔。值被编码成合法 TOML 字符串。AgentX 不替换 Codex 的基础 system prompt。普通 Hook 的动态快照经 Codex `additionalContext` 另行注入；`systemMessage` 是 hook output 的 presentation 字段之一，不是固定指引的存储点。

Shared Codex app-server 没有可用于多个并发 thread 的单一进程身份。它在 [codex/configuration.rs](../../src/integrations/codex/configuration.rs) 保留现有 developer instructions 并加入简短共享说明，再由 thread-specific Hook 获取真实冻结文本和快照。shared service 的说明不会把所有 thread 身份混成一个身份。`--agentx-system-prompt` 在这个独立入口转成显式 developer instruction，也不改变普通 `AGENTX_INSTRUCTION_SOURCES` 的职责。

## User task 与工具参数

[`InitialPromptShape`](../../src/integrations/registry.rs) 将同一个 AgentX 初始任务映射成 Provider 所需的位置参数、flag 或 unsupported diagnostic。Provider 的原生 shell args 不是共享固定指引合成的一部分；已有系统指令会通过该 Provider 的合并/保留策略继续存在。某些工具在启动参数中承载 prompt，不等于它们将文本当作 system message。

`--agentx-prompt` 是首次业务请求；`--agentx-system-prompt` 在普通 fresh launch 被保留为 AgentX 固定指令追加项。Resume/Fork 的临时身份重置、工具已有历史、原生系统提示与用户任务可能落在不同字段。分析 transcript 时需要连同事件类型和消息来源判断，不能仅按 CLI 选项名字归类。

## Provider 专属文本

- Builtin 自动邮件规则中包含 Cursor 与 Copilot 对 wake trigger 的专属解读；对应邮件内容仍由 `hints` 与 intent/thread tip 组成，见 [Hint 附录](hints.md#每批消息携带的提示)。
- Antigravity 的 `ANTIGRAVITY_DELIVERY_ACTION` 在 pending message 前加入，尤其说明 request 必须在本轮完成或明确阻塞；它不是 `bootstrap_exclude` 中的独立部分，也不由替换 builtin sources 移除。
- Claude 原生 Task 子 Agent 的加入时会先收到 `Your agent ID` / parent 与 `agentx start --name ...` 提示，再经专属 subagent bootstrap 获取交互规则；这条入口不调用通用 `get_instructions_client`。
- 对没有独立 runtime-context channel 的 Grok，固定 sources 仍可进入 rules，但选择动态 sources 会失败。不要把“合成成功”误读为“各 Provider 都接受相同双通道”。

## 配置变量的证据边界

通用固定 source 替换配置只有 `AGENTX_INSTRUCTION_SOURCES`；它不改上游基础指令，也不接管任务、Role、动态 snapshots 或消息邮件。当前没有通用 `AGENTX_SYSTEM_PROMPT` 配置。

配置 schema 登记了 `AGENTX_CODEX_SYSTEM_PROMPT` 与 `AGENTX_GEMINI_SYSTEM_PROMPT` 字段，但本次对 Rust 生产源码的检索只找到字段定义、默认值、序列化/help 等引用，没有确认到读取它们并注入启动 Prompt 的生产调用点。因此这份文档不把它们描述为可用的 Bootstrap 控制项；使用前应先由负责该功能的代码与执行证据确认其现状。

当前 Provider 级支持/未知项和生命周期测试状态仍以[agent-context Delivery](../../.controller/agent-context-lifecycle/delivery.md)与[agent-context 使用表](../agent-context.md)为准。本附录建立的是源码导航和边界说明。
