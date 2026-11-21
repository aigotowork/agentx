---
schema: proposal
description: Turn AgentX's native coordination capabilities into a task-centered professional workbench with explicit results and acceptance.
status: accepted
models: [../model/README.md, ../collaboration-workbench/README.md]
deliveries: [../../.controller/collaboration-workbench/delivery.md]
---

# 以任务交付为中心的协作工作台

## 选择依据

现有 AgentX 已有参与者、消息/回执、Provider 激活、单二进制 daemon、原生观测和可直接访问的内嵌 LAN 调查台。用户认为管理体验仍粗糙，要求在既有管理体系中建立 Delivery 和原始 Tasks，并由专职 Agent 先从场景、结构和交互出发，用 SVG 呈现专业设计，再讨论优化和实现。

本提案选择[路线图](../roadmap.md)中“协作台”和“可重复任务交付”的首个完整切片：用户发起明确工作，安排真实 Agent，跟进问题和进展，查看产物与独立检查，明确验收。必要的日常入口、构建与所用 Provider 支持服务于该切片，不自动选择所有路线图方向。

## 已接受与未决定

| 决策 | 处置 |
| --- | --- |
| 以任务交付结果串联产品体验 | 已选择。页面、操作和工程任务必须能说明自己推进哪一步用户结果。 |
| 先专业设计，后实现 | 已选择。独立体验设计提供实际 SVG、信息层级与完整交互；用户讨论具体方案后进入实现。 |
| 主要任务集中在工作主屏，相关操作局部展开 | 已选择。具体工作对象、布局和组件由设计者从场景推导。 |
| React 与前端构建 | 允许但未强制。专业工程设计依据交互复杂度、维护和构建成本推荐；产物由 Rust 内嵌，无需用户传 `--assets`。 |
| 任务引用、执行、检查与验收的最小领域语义 | 设计 Task 拥有具体决策。优先复用已有能力和原始工作引用，不能把 Agent 空闲自动当作完成。 |
| 通用调度、组织权限、遥测平台、多作用域/多机器控制 | 本轮不选择；实际需求出现时另行评估。 |

## 保留的产品保证

保持 AgentX 自身的参与者、Provider 和消息语义，原生 Rust 契约与 daemon 存储所有权，以及已有只读观测的独立用途。开发协作使用 AgentX 自身的调度与通信能力。新的协作写操作需有自身明确、有限的授权和作用域边界，不能从当前无需凭据的只读 LAN 页面自动推导任意命令权限。

首个完整协作切片已由[完成的 Delivery](../../.controller/collaboration-workbench/delivery.md#已接受的本地实现结果)接受；当前语义与支持以[领域模型](../model/README.md)、[工作台入口](../collaboration-workbench/README.md)和[独立最终证据](../../.controller/collaboration-workbench/evidence/acceptance.md#final-artifact-evidence-and-cleanup-handoff)为准。方案接受状态仍表达设计选择，具体实现与 Linux 本地验收不扩展成其他平台/Provider 或整个路线图完成。既有[观测结果](../../.controller/observability/implementation.md#accepted-local-implementation-outcome)和[LAN 演示](../../.controller/observability/lan-usability.md#accepted-running-outcome)继续保持各自范围。

