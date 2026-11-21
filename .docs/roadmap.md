---
schema: roadmap
description: Selected AgentX capability work.
status: active
items:
  typescript_domain_sdk:
    description: Make AgentX objects independently usable in TypeScript before formal Web integration.
    status: selected
    horizon: now
    proposals: ["./proposals/typescript-domain-sdk.md"]
  contract_service_projections:
    description: Derive maintained OpenAPI, a TypeScript SDK and HTTP CLI from native consumer-service contracts across existing public capabilities.
    status: selected
    horizon: now
    proposals: [./proposals/contract-service-projections.md]
  agent_launch_relations:
    description: Inspect direct AgentX creation upstream/downstream status and open exact related conversations.
    status: selected
    horizon: now
    proposals: [./proposals/agent-launch-relations.md]
  launch_profiles:
    description: Save and select reusable Provider launch configurations with explicit temporary overrides.
    status: selected
    horizon: now
    proposals: [./proposals/launch-profiles.md]
  codex_shared_runtime:
    description: Preserve independent multi-provider collaboration through a shared native Codex runtime.
    status: selected
    horizon: now
    proposals: [./proposals/codex-shared-runtime.md]
  domain_driven_agentx_daemon:
    description: Refactor AgentX around explicit domain ownership and native contracts, then centralize each data scope behind one single-binary AgentX daemon.
    status: selected
    horizon: now
    proposals: [./proposals/domain-driven-agentx-daemon.md]
  runtime_observability:
    description: Explain AgentX runtime, effective configuration and correlated events through native inspection, CLI and browser consumers.
    status: selected
    horizon: now
    proposals: [./proposals/agentx-observability.md]
  agent_management_http:
    description: Manage Agent lifecycle, communication and model transcripts through a daemon-owned HTTP API and embedded Web console.
    status: selected
    horizon: now
    proposals: [./proposals/agent-management-http.md]
  office_management:
    description: Connect native workspace membership, adopted/effective Roles, pure routing and Role/directory actions in one usable management flow.
    status: selected
    horizon: now
    proposals: [./proposals/office-management.md]
  live_ui_integration:
    description: Connect existing Inspector and office conversation UIs to actual AgentX APIs with honest collection and supported operations.
    status: selected
    horizon: now
    proposals: [./proposals/live-ui-integration.md]
  everyday_operation:
    description: Install, start, inspect and stop a standalone AgentX workspace through a short coherent daily-use path.
    status: candidate
    horizon: next
  environment_loading:
    description: Prepare explicit ordered dotenv sources consistently for local invocations and Agent launches.
    status: selected
    horizon: now
    proposals: [./proposals/environment-loading.md]
  collaboration_console:
    description: Organize real agents and inspect their conversations, work and results from a browser workspace.
    status: selected
    horizon: now
    proposals: [./proposals/collaboration-workbench.md]
  repeatable_task_delivery:
    description: Repeat a bounded implement-review-verify workflow with explicit task references, outputs and completion evidence.
    status: selected
    horizon: now
    proposals: [./proposals/collaboration-workbench.md]
  retained_diagnostic_evidence:
    description: Preserve and share selected safe investigation evidence after volatile runtime retention expires.
    status: candidate
    horizon: later
  verified_provider_paths:
    description: Establish support and recovery evidence for provider and platform combinations used in real collaboration workflows.
    status: candidate
    horizon: next
  domain_process_visualization:
    description: Reuse typed domain and SDK contracts to explain objects, processes and state through an interactive visual tool.
    status: selected
    horizon: now
    proposals: [./proposals/flow-atlas-platform.md]
---

# 当前选择

用户已授权实现现有 Agent 管理台的 Web 对应物，包括启动、消息、模型记录、停止及恢复。该入口由所选作用域的 daemon 提供，页面消费结构化 HTTP 数据。设计、原生契约、使用说明与实际验收状态分别见[提案](./proposals/agent-management-http.md)、[功能入口](./agent-management/README.md)和[已接受的本地交付](../.controller/agent-management/delivery.md#accepted-local-outcome)，不扩大到所有 CLI 命令或浏览器终端镜像。

用户已授权完成共享原生 Codex daemon 的架构、契约、协议、生产实现与集成验证，并要求使用 Delivery 和领域子 Agent 推进。此项复用 [既有领域设计](./codex-activation/README.md)，保持其他 Provider 的能力与接入机制。实施进度沿 Proposal → Delivery → Task 查询，不在路线图重复维护。

用户已选择 AgentX 基于领域职责与 Rust 原生契约演进，并以单一 `agentx` 可执行文件提供 client 和 daemon 入口。具体 IPC、自动启动、Provider host 所有权与迁移原则见[领域与 daemon 提案](./proposals/domain-driven-agentx-daemon.md)；实际实现、已取得的本地验收及平台和部署边界以[该 Delivery 的已接受结果](../.controller/domain-daemon/delivery.md#accepted-local-outcome)为准，不在路线图重复维护。

运行观测与管理体系已完成数据模型、原生契约、CLI、浏览器适配器及 Web 调查台，并通过所选 Linux 本地验收。见[观测体系提案](./proposals/agentx-observability.md)及[已接受实施结果](../.controller/observability/implementation.md#accepted-local-implementation-outcome)。用户后续选择的[局域网直接访问、内嵌网页和真实 Agent 演示](../.controller/observability/lan-usability.md#accepted-running-outcome)也已完成；它取代旧的一次性凭据与外部资源目录要求。实际运行的是修正后的本地开发构建，已安装程序、修正后优化发布包和未执行平台不在该结论内。

## 当前选择与后续候选：让协作产生可验收结果

本节来自用户在前一轮交付完成后的方向讨论。用户现已选择“协作台 + 可重复任务交付”的首个完整切片，并要求专业 UI Agent 先从场景、信息层级和交互出发提供 SVG，再讨论优化及实现。其[提案](proposals/collaboration-workbench.md)和[Delivery](../.controller/collaboration-workbench/delivery.md)承接当前工作，其他方向仍是候选；不改变既有交付的完成状态。现有架构已经支撑参与者、消息与回执、Provider 激活、工作流脚本和运行观测，新工作围绕真实路径组合这些能力。

| 顺序 / 候选 | 已有基础与实际缺口 | 有价值的首个结果 | 完成条件与边界 |
| --- | --- | --- | --- |
| P0 日常启动与交付 | 单文件内嵌页面已实现，但当前演示使用开发构建和分离进程；安装版本与源码版本分开，服务查看、停止和作用域选择仍需了解内部结构。 | 新用户安装一个产物，用短命令打开工作区与管理台，并能查看和结束自己启动的服务。 | 用修正后的优化产物从干净目录完成安装、启动、状态查询、停止和升级后的重新连接；统一受影响的全局 CLI 参数。保留显式作用域和原有进程所有权，先不引入复杂服务编排。 |
| P1 以 Agent 为中心的协作台 | CLI 已有 launch/send/resume/kill/transcript，当前 Web 只读，入口是请求、连接、进程、主机和客户端报告。 | 用户看到谁在做什么、谁等待输入，打开对话或产物，再从同一工作区发起明确的协作操作。 | 一个真实工作区内完成选择 Agent、查看上下文、发送工作、看到回执与结果、停止或恢复；沿用原生用例与身份边界。观测读取保持独立，不把 HTTP 请求成功当作任务成功。 |
| P2 可重复的任务交付 | 已有 `run`、`onidle`、消息、启动批次和回执，但脚本触发与 Agent 空闲不等于工作已经完成或通过验收。 | 先做一个“实现 → 独立检查 → 用户验收”的可复用流程，保留输入、负责人、输出位置和验证结果。 | 用真实仓库完成一次交付，并在中断后说明已完成与待做步骤；优先关联现有文件或外部原始 Task，复用批次和消息回执。先记录一次运行的结果，不预先建设通用 DAG 调度器或另一套项目管理系统。 |
| 按首个协作流程并行补齐 Provider 支持 | 多 Provider 接入已存在；本轮真实任务是 Codex，不能据此宣称其他 Provider 与其他系统都经过同样验证。 | 选两种实际使用的 Provider，证明互发消息、忙时接收、结果交接和恢复可用。 | 发布按 Provider、版本、平台、操作划分的支持依据；只覆盖首个选定流程所需组合。MSRV、Node 固定版本及 Windows/macOS 的缺口属于证据范围，不能直接写成源码能力不存在。 |
| 后续：保留调查证据 | 请求历史约五分钟，事件与配置快照约十五分钟，并有行数和字节上限；持久消息历史与短期观测不是同一存储。 | 一次故障能导出所选请求、配置与事件窗口，之后打开仍能理解当时依据。 | 先提供有范围的安全导出与离线读取，保留身份、采样时刻、未知与缺口；再由真实使用决定是否需要长期采集。暂不建设完整遥测平台。 |

当前先推进专业工作台设计与“协作台上的一次真实任务”；必要的日常入口、发布与实际 Provider 验证纳入该切片，不并行启动所有候选方向。评价价值应看首次得到结果需要多少步骤、用户是否清楚等待谁、能否找到产物和验证依据，以及中断后是否重复产生作用；不以新增页面、Agent 数或抽象层数衡量。

多作用域聚合、Relay 远程工作区管理、自动成本调度和大规模无人值守运行暂不优先。Relay 与多作用域基础已存在；只有本地工作路径稳定且出现明确跨机器或规模需求，再选择这些增量。任务/执行/交付验收的最小语义由当前设计 Task 细化，设计选择和真实实施支持保持区分。

候选依据：[领域边界](./model/README.md)、[原生应用操作](../src/service/mod.rs)、[已有工作流](../src/workflows/mod.rs)、[当前 Web 对象与交互](../web/observability/app.ts)、[浏览器只读契约](./observability/browser-api.md)、[局域网实际结果及留存修正](../.controller/observability/lan-usability.md)和[已取得的平台证据](./architecture/acceptance.md#current-local-outcome)。
