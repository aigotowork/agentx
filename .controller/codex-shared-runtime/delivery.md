---
schema: delivery
description: Build and verify shared native Codex hosting with independent AgentX activation and provider isolation.
status: completed
stages:
  contracts:
    description: Resolve owned interfaces, publish production declarations and a usable module skeleton.
    status: completed
    tasks: [./tasks/architecture.md]
  runtime:
    description: Implement service hosting, identity/message state and the native protocol bridge.
    status: completed
    tasks: [./tasks/native-service.md, ./tasks/identity-messaging.md, ./tasks/bridge-composition.md]
    depends_on: [{stage: contracts, condition: The relevant owned production contracts and file boundaries are usable; discovery can start earlier.}]
  acceptance:
    description: Independently verify the complete user outcome and preserved provider behavior.
    status: completed
    tasks: [./tasks/integration.md]
    depends_on: [{stage: contracts, condition: Scenario contracts are usable for test authorship; final acceptance additionally requires real runtime implementations.}]
---

# 目标与范围

实施 [已接受的变更](../../.docs/proposals/codex-shared-runtime.md)，把 [设计入口](../../.docs/codex-activation/README.md) 中 R1–R9、A1–A11 从设计目标落实为生产能力。由 zomi 负责跨领域推进，领域负责人负责专业设计、实现和所属模型，独立集成负责人负责实际验证。

完整用户结果是：受支持的上游 app-server 调用经 AgentX 接入，同一兼容作用域复用同一原生 Codex 后端；每个会话拥有正确身份与消息归属；父 Agent 空闲后收到子 Agent 回复能继续；原生请求、事件、审批、取消及连接退出语义正确；其他 Provider 的既有能力保持。

交付包含契约、协议与架构更新、生产代码、场景与故障验证、当前支持说明及本地构建产物。已安装程序替换、用户服务重启、远程部署和版本发布不在此批范围内。未验证的真实外层产品行为必须明确标识，不能把局部 fixture 结果升级为完整兼容声明。

用户后续要求普通 Codex 使用恢复原生入口：不保留或重新安装 `~/.local/bin/codex` 包装。AgentX 优化继续在项目源码、隔离测试和本地构建产物中推进，接入候选版本使用明确入口；普通 PATH 中的 Codex 不接管到候选协议桥。

# 工作流与结果契约

| 责任 | 消费方需要的结果 | 开始与完成条件 |
| --- | --- | --- |
| 架构与契约 | 明确分层、公共类型/签名、生命周期、错误、生产模块骨架及文件归属 | 立即开始；发布可供消费者实现的契约，解决阻塞性专业边界后完成 |
| 原生服务 | 可复用的真实后端连接、服务身份/代次与明确的兼容/退出语义 | 可先调研；生产行为依赖已发布接口，最终需真实隔离原生证据 |
| 身份与消息 | 线程附件和 claim 的事务边界、统一交付/恢复及不依赖具体 Provider 的调度 | 可先重建现状；接口可用后实现，不能让旧 hook/listen 绕过一致性边界 |
| 协议桥与组合 | 原生双向消息完整路由、正确 bootstrap/hooks 与选定启动接线 | 可先读取调用方；依赖相关服务和身份契约，最终必须接入真实参与能力 |
| 独立集成 | 可挑战 A1–A11 的测试/fixture、适用检查结果及诚实支持矩阵 | 可先设计用例；执行按实现就绪度安排，最终覆盖实际组合和其他 Provider 影响 |

阶段描述结果，不形成固定串行批次。可用契约可以先释放消费者；实现尚未完成时，其他领域继续不重叠的调研、场景和骨架工作。接口冲突由架构负责人协调相关领域决定，zomi 更新依赖与分工。

公共契约与骨架已由架构负责人发布到 [生产契约交接](./contracts-handoff.md)，对应源码是精确接口的来源。构建与验收继续按这些接口推进；该契约阶段完成不表示后端组合或其他 Provider 已通过验证。

# 完成条件

- 生产声明成为唯一精确契约来源，设计、协议与入口可以让新 Agent 独立恢复上下文；没有以 TODO、未接线代码或永远拒绝的空实现替代必需能力。
- A1–A11 的实际覆盖、故障边界和环境范围由集成负责人记录。首次接入、并发复用、逐会话身份、父子往返、用户控制、未知结果与代理退出隔离都有相应证据。
- 对共享身份、数据库、投递、hook、launcher 变更执行受影响 Provider 的回归。Codex 服务不可用不成为其他工具的启动依赖。
- 适用的格式、编译、静态检查、相关测试和本地打包通过；既有失败要用原基线的相应用例证明归属，不能直接忽略新失败。
- 生产支持说明准确区分实现、集成、验证及尚未验证的环境。临时服务、测试进程和写入归属在验收后清理；Delivery 的完成由本结果决定，不由已完成 Task 数决定。

调研旧任务保持已完成，作为输入证据；本 Delivery 独立管理构建工作。执行状态见各 Task，实时分配与下一动作见 [controller](./controller.md)。

# 交付验收

zomi 于 2026-11-07 接受本 Delivery 的实现、独立验证及本地构建结果。五个领域 Task 均已完成，生产声明和当前模型已归位；精确场景结果及覆盖边界由 [验收报告](../../.docs/codex-activation/acceptance.md) 统一维护。

验收范围为 Linux x86_64、官方 Codex `0.167.0` 共享服务档案及未修改的 Paseo `0.15.3` provider API。真实父线程启动 AgentX/native 子 Agent、空闲收回复后自主继续并发送业务回复的闭环已通过；并发复用、线程身份/环境/hooks、resume/fork、忙碌输入、审批/取消和代理退出隔离均有相应验证。受影响的身份、数据库、消息、启动及其他 Provider 边界回归通过。格式、全部目标严格 Clippy 和 `just package` 完成。

本地产物为 [`.bin/agentx`](../../.bin/agentx)，版本 `0.7.31`。控制方核对了 [构建回执](../../.agentx/.tmp/deliveries/codex-shared-runtime/package-receipt.json) 的产物哈希、238 项当前源码哈希及已安装 AgentX 哈希，均一致；没有替换安装版本、重启用户服务或创建全局 Codex 包装。17 次原生隔离尝试的清理回执均无错误、无剩余状态；领域写入及测试构建资源已释放。

验收不外推桌面/移动端渲染、用户现有主机、付费推理、其他系统/原生版本或真实 Claude 厂商进程。用户要求解决本地依赖后，zomi 通过既有 `just typecheck` 入口安装锁定开发依赖并完成类型及插件回归；随后完成 [开发依赖安全维护](../plugin-dependency-maintenance/task.md)，在保留 SDK 档案和插件源码的前提下更新安全覆盖与锁文件。固定 CI 工具链下的完整依赖审计为 0 项漏洞，8 项依赖兼容、类型检查、25 项插件投递和 71 项 Hermes 回归通过。具体回执见 [验收报告](../../.docs/codex-activation/acceptance.md#development-dependency-follow-up)。未知新增环境输入仍按兼容冲突处理。安装、发布与 live-host 接入属于后续单独操作，不由本次本地交付自动执行。
