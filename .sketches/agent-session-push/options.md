---
scope: AgentX 接入路线与取舍
status: 候选，未选定实现
last_updated: 2026-11-22
---

# 哪条路线适合我们

先决定**要托管新会话，还是必须保留已经打开的原生终端**。前者可以让 AgentX 成为协议客户端，拿到目标 ID 和轮次状态；后者只在宿主本身公开接收入口时能无按键接入。旁边新起一个 ACP 或 MCP 进程，不能取得旧进程里的会话控制权。

还有一个中间选项：**保留原生 TUI，但在启动时连接可控后端**。Codex 的 `--remote unix://…` 支持这条路，见项目已有 [0.170.0 的共享终端实测](../../.docs/codex-activation/acceptance.md#shared-terminals-by-default-and-native-recover)。这是新启动时选择的拓扑，不能接管任意已打开终端，也不需要 fork。

| 路线 | 对什么有效 | 接入成本与边界 | 建议 |
| --- | --- | --- | --- |
| 既有 hook / 兼容唤醒 | 当前以 PTY 接入且没有可用控制协议的工具 | 等 hook 或用既有唤醒策略；保留当前经过验证的兼容面 | 作为逐工具迁移的回退。现状以[投递修复记录](../../.controller/message-delivery/AGENTS.md)为准 |
| AgentX 信箱 → ACP | AgentX 托管或适配器明确支持恢复的主会话；同一客户端服务多工具 | 统一协议；每个工具仍需一条可启动的适配器配置和能力、权限验收。ACP v1 的忙时队列由 AgentX 拥有 | 新托管路径首选。先覆盖已经过隔离验证的 Codex 和 OpenCode |
| AgentX 信箱 → Codex App Server | AgentX 控制的 Codex 原生线程；需要精确的轮次、权限或历史关联 | `thread/start` / `thread/resume` / `turn/start` / `turn/steer`，接口更具体，Codex 专用。已有[原生接入证据](../../.docs/codex-activation/README.md)，不必另建桥 | 继续作为 Codex 路径；与 ACP 共用上层收件箱和附件语义 |
| 外部事件 → MCP Events → 宿主 → ACP/原生 | 外部系统独立发现、订阅、推送；接收端需可配置并决定执行 | Events 能统一外部事件传输，不会自动为所有 Agent 打开会话控制。只做 AgentX 内部通信时，这一跳不是必需 | 有独立 MCP 事件生产者时加。可以由 AgentX 订阅，不要求每个工具都实现 beta 扩展 |
| MCP sidecar → Claude Channels 等原生接收端 | 宿主支持向现有运行会话接入事件，且启动时已启用 | 体验接近用户设想；每家方法、许可、队列和回执不同，可能无法覆盖无人值守或历史会话 | 确实要保留原生终端时按工具评估 |
| 改 Codex 上游或维护 fork | 必须保留原生 Codex TUI，又没有可用的非交互输入入口 | 能添加事件到会话的最后一步；只解决 Codex。引入构建、发布、升级同步和跨平台验证 | 先验证原生/ACP；若终端缺口成为硬需求，优先小范围上游提案 |

## Codex 是开源的，改哪里才有用？

[OpenAI Codex CLI / App Server 源码](https://github.com/openai/codex/tree/rust-v0.171.0)采用 [Apache 2.0](https://github.com/openai/codex/blob/rust-v0.171.0/LICENSE)，可以修改自己运行的开源客户端或构建自己的发行版，分发时遵守许可证。这样不会改变 OpenAI 托管服务或模型权重，也不会让其他工具自动获得同样的功能。

只放开 `mcpServer/event/stream/start` 对 server 的限制仍不够。[当前源码](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/app-server/src/request_processors/mcp_event_stream.rs)会把事件转发给订阅连接；没有替它选择会话、插入消息或开始一轮。若真需要改，优先让自建事件通过**显式启用的本地宿主入口**提交给当前绑定的线程，重用现有 turn 调度。两种上游切入点：

1. **支持普通 MCP Events 并向宿主转发**：适用于 App Server 的消费端，宿主仍负责投递。若目标是通用 Events 生态，这个改动有独立价值；若只要唤醒自管 Codex，当前原生 turn API 已够。
2. **原生 TUI 提供会话输入入口**：适用于必须保留原生 TUI 的用户。启动时显式配置本地接收器，入口绑定确切线程和运行代次；收到后用现有消息与轮次流程，能给出“拒绝、排队、已写入”证据。来源许可、忙时策略和恢复需要随实际需求设计。

若选择 fork，先固定版本、记录上游差异，再用双线程、并发用户输入、审批、断线和升级回归证明它值得维护。本轮没有改、编译或安装 Codex 自定义分支。

## AgentX 的最小适配轮廓

- **启动**：给已验证工具添加可选 ACP 托管方式。AgentX 启动并拥有 stdio，协商能力，把 Provider、托管作用域、native/ACP session ID、绑定代次记录到既有参与者附件；选择保留原生 PTY 时，不创建一个同名但互不相干的 ACP 会话。
- **投递**：收件箱持久化先于唤醒。ACP v1 对每会话只发一条在途 prompt，忙时在宿主排队；原生 Codex 可按策略选等待或 `turn/steer`。保留来源和消息 ID，收到消息不能授予批准权限或修改 sandbox。
- **确认**：RPC 写出不是送达确认。v1 可以关联确切轮次和历史证据，失联后的投递状态未知时先核对；v2 的 `messageId` 是收录回执，幂等键在保留期间有价值。传输、持久信箱、模型完成和业务回复分别记账。恢复时检查代次，禁止把旧连接回调算给新会话。
- **接外部消息**：需要时加 MCP Events 订阅，把收到的 `eventId` 对应到既有消息 ID，再走同一队列和权限规则。没有外部生产者时，直接由已有 AgentX 入库事件通知宿主。

## 下一轮值得做的验证

隔离探针已消掉「ACP 能否跨两种工具在原会话启动下一轮」的不确定性。真正决定是否可合入的是：原生终端是否必须保留、真实身份和权限是否能无损绑定，以及在**请求写出后、回执返回前**断线时能否关联原始消息。建议优先做一条使用现有 AgentX 信箱的闭环：两会话不串信、等忙轮、原参与者重绑、审批不被绕过、丢失回执时不盲重发。MCP Events 的跨实现端到端订阅，以及原生 TUI 接收器，按需要另验。
