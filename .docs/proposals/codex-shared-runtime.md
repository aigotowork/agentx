---
schema: proposal
description: Build the shared native Codex backend and provider-isolated AgentX activation adapters.
status: accepted
models: [../codex-activation/README.md, ../codex-activation/topology.md, ../codex-activation/protocol.md, ../codex-activation/session-environment.md]
deliveries: [../../.controller/codex-shared-runtime/delivery.md]
---

# 变更与决策

当前 app-server 托管只继承标准流，每次启动一个独立原生后端；它没有独立的空闲投递路径。用户已选择：同一兼容作用域复用官方 Codex daemon，AgentX 提供协议适配、线程身份、消息交付与自主唤醒，并保持多 Provider 分层。

已接受的目标与约束以 [R1–R9 和 A1–A11](../codex-activation/README.md) 为准。过去的隔离探针证明了关键原生能力，但没有证明生产桥、受管理 daemon 或未修改 Paseo 的完整闭环；已有设计与实验记录保留其原有范围。

| 决策 | 状态与归属 |
| --- | --- |
| 通用服务保持 Provider 无关；Codex 协议、环境、daemon 策略留在 Codex 适配层 | accepted；架构负责人维护可编译的生产契约和依赖方向 |
| 同一兼容服务作用域的受支持接入复用同一原生后端 | accepted；原生服务负责人落实发现、兼容性、代次及连接退出隔离 |
| 父 Agent 收到子 Agent 回复后能自主继续，不依赖 Paseo 调度 | accepted；身份/消息负责人和协议桥负责人组合实现 |
| 原生 daemon/proxy 的确切端点、启动配置作用域及故障恢复 | open implementation decisions；原生服务负责人提供真实、可重复的隔离证据，架构负责人协调消费方 |
| 生产附件/claim 事务与上游协议生命周期的精确接口 | open implementation decisions；领域负责人在行为实现前发布生产 Rust 契约 |

用户本次明确授权设计、构建、创建领域子 Agent 和集成验证。Delivery 包含生产代码、适用的检查与本地可审查构建产物；不包含替换当前正在使用的已安装 AgentX、重启用户现有 Codex/Paseo 服务、发布版本或远程部署。验证优先使用隔离原生进程和固定本地模型 fixture。需要用户实际运行环境才能观察的剩余条件必须明确记录，不能用模拟成功替代。
