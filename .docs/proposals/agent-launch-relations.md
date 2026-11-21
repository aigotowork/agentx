---
schema: proposal
status: accepted
description: Expose durable AgentX launch provenance and direct related-Agent navigation in Space.
models: [../model/README.md, ../agent-management/README.md]
deliveries: [../../.controller/agent-launch-relations/delivery.delivery.md]
---

# Agent 启动关系

用户于 2026-11-15 授权设计、规划并分派实现：在 Agent 对话下方查看谁启动了它、它启动了哪些 Agent，显示状态并点击进入对应对话，可逐级继续查询。暂不需要全局关系图。

已有 `created`/`batch_launched` 生命周期事件保留启动者名字，但缺少启动者精确实例身份，HTTP/Space 尚未提供完整能力。沿用 [AgentX 领域边界](../model/README.md) 和 [管理入口](../agent-management/README.md)，补齐来源事实、查询与既有 Space 消费，不另建管理页面。

选定范围与验收见 [Delivery](../../.controller/agent-launch-relations/delivery.delivery.md)；[设计](../../.controller/agent-launch-relations/design.md)明确事实/状态分离、历史与边界，专业实现 owner 负责推导精确原生契约。启动关系不自动代表后续任务分派、权限、终止传播或 Provider 会话亲缘。保留 Claude 原生子 Agent 现状；相关更广反馈另行处理。实施授权不包含替换当前运行服务。
