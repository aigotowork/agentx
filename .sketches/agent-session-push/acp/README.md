---
scope: Agent Client Protocol 中文参考
status: 规范调研
last_updated: 2026-11-22
---

# ACP 协议参考

这份参考讲的是 **Agent Client Protocol**：编辑器、终端宿主或编排器如何启动、驱动和观察 Agent。以 **v1** 为主线，**v2 Preview** 单独标注。这里不做 AgentX 接入，也不假定各工具都实现规范的全部可选能力。

| 想查什么 | 从哪里开始 |
| --- | --- |
| 哪一方能发什么，须在何时发送？ | [接口与契约](interfaces.md) |
| 请求和更新有哪些字段，ID 与状态如何关联？ | [核心字段与数据模型](fields.md) |
| 一次读代码、改文件、审批、取消或续聊实际怎么走？ | [使用模式与案例](scenarios.md) |
| 现有客户端迁往 v2 时，哪几处行为会变？ | [v1 与 v2 Preview 对照](versions.md) |
| ACP 是什么、工具怎么发现？ | [已有 ACP 概览](../acp.md) |
| 哪些是真正跑过的？ | [Codex / OpenCode 隔离证据](../evidence.md) |

阅读顺序可按「进程连接 → 会话 → 输入 → 更新/反向请求 → 结束或重连」。字段表解释主干含义；完整类型、可空性和版本规则以官方 [v1 Schema](https://agentclientprotocol.com/protocol/v1/schema) 与 [v2 Schema](https://agentclientprotocol.com/protocol/v2/schema) 为准。Mermaid 图与报文是教学示例；已执行情况只在实验记录里标示。

本轮核对用的是 2026-11-22 的官方文档与 [ACP 仓库 Schema 快照](https://github.com/agentclientprotocol/agent-client-protocol/tree/c9fef684e0ed6686a7b4aac6b9edb89f5187eed9/schema)。快照分别包含 v1 / v2 的基线与 unstable 文件，便于将来对照变化。
