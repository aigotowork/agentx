---
schema: delivery
description: Deliver a user-centered AgentX workbench that takes one real task from assignment through independent checking to explicit user acceptance.
status: completed
stages:
  design:
    description: Establish scenario-first SVG experience and a minimal native capability model before implementation.
    status: completed
    tasks: [./tasks/experience-design.md, ./tasks/capability-design.md]
  implementation:
    description: Implement the selected collaboration path through native use cases and an embedded professional Web workbench.
    status: completed
    tasks: [./tasks/collaboration-service.md, ./tasks/workbench-web.md]
    depends_on: [{stage: design, condition: user-selected resource-workspace with released detailed experience and capability semantics; native declarations precede dependent implementation behavior}]
  outcome:
    description: Independently demonstrate real assignment, collaboration, outputs, verification and user acceptance on a packaged standalone artifact.
    status: completed
    tasks: [./tasks/acceptance.md]
    depends_on: [{stage: implementation, condition: actual cooperating native and Web implementations with authored cases released}]
---

# AgentX 任务协作工作台

## 用户结果

用户在一个工作区发起有明确目标和验收条件的任务，安排真实 Agent 协作，知道当前进展和谁需要输入，找到产物及独立检查依据，并决定接受还是继续修改。主屏保留工作上下文，相关信息和操作通过合适的局部层级展开。每项工程工作必须服务于这条路径，不能把边缘小场景升级为交付主线。

本 Delivery 执行[已选择提案](../../.docs/proposals/collaboration-workbench.md)中的首个完整协作切片。它复用已接受的单二进制 daemon 和只读观测基础；不重新验收历史实现，也不把目标界面写成已有能力。

本轮已完成并由控制者正式接受，范围为文末[已接受的本地实现结果](#已接受的本地实现结果)。真实双 Agent 主链路、最终 r8 内嵌产物、窄屏触控与布局、精确资源清理及本地产物交接均有[唯一验收证据](evidence/acceptance.md#final-artifact-evidence-and-cleanup-handoff)。五项原始 Task 均已完成。此接受仅覆盖已记录的 Linux 本地范围，不表示其他平台、全部 Provider 或已安装程序已发布升级。

## 第一阶段：先把体验画清楚

专职设计 Agent 从用户场景、信息层级和操作关系出发，提供可编辑 SVG 主屏与关键交互状态，并说明一次完整路径。独立的领域设计 Agent 梳理原生能力复用、任务引用、实际执行、输出与验收的最小语义，提出前后端边界和技术建议。两者并行，各自拥有专业判断，不用现有诊断页限制初始设计。

控制者消费实际 SVG，检查“用户能否找到下一步、谁在等谁、产物和依据在哪里”，将具体预览交给用户讨论优化。设计示例数据必须明确标注；静态图不算浏览器或业务验证。React 可选，依据真实交互和维护成本决定；最终前端构建产物必须能由 Rust 内嵌，用户不需要提供 `--assets`。

## 任务与交付结果的关系

| 原始 Task | 它为用户补齐的结果 | 完成边界 |
| --- | --- | --- |
| [体验设计](tasks/experience-design.md) | 一眼理解工作、负责人、待处理事项和结果入口 | SVG、场景/交互依据及用户讨论材料可用；不代表已实现。 |
| [能力与结构设计](tasks/capability-design.md) | 界面表达有真实语义和原生能力承接 | 最小责任模型、能力映射、可消费契约方向、选型和实施边界可用。 |
| [协作服务](tasks/collaboration-service.md) | 用户操作实际驱动工作，状态、产物与检查记录有依据 | 原生用例及有限浏览器操作实现，保留明确作用域和结果语义。 |
| [Web 工作台](tasks/workbench-web.md) | 用户在连续上下文内完成发起、跟进和验收 | 已选择设计的真实界面、状态与打包接入完成，案例由独立执行方消费。 |
| [独立交付验收](tasks/acceptance.md) | 一次真实工作得到可复查、可接受的结果 | 实际 Agent、产物、独立检查、浏览器操作及单文件产物有对应证据。 |

## 开始条件和专业责任

当前只启动两项设计任务。协作服务与 Web 可在设计方向和契约可用后并行；Web 不必等待所有后端代码完成，但最终验收需要实际协作能力。独立执行方可以在收到稳定契约后准备真实场景，不在移动代码树上提前跑整套检查。

UI 设计拥有用户场景和交互；领域设计拥有执行语义与工程边界；实施者维护自己的原生声明、模块文档和案例；独立验收者维护可复现结果、产物与资源清理。控制者维护 Proposal/Delivery/Task 关系、分派、跨域问题和最终接受结果。实际绑定及写入权见[控制记录](controller.md)，不维护第二份 Task 状态清单。

## 验收依据

1. 用户能在主工作区发起或关联一项有目标与验收条件的工作，选择真实 Agent，看到请求接收、执行进展、待输入及失败的实际区别。
2. 一名实现者与独立检查者完成一个真实仓库中的小功能；用户可从任务上下文找到真实产物与检查结果，并明确接受或要求修改。Agent 空闲、消息回执或 HTTP 成功不能自动变成任务验收通过。
3. 用户切换相关对话、产物、检查和诊断后仍保留任务上下文；主要桌面操作集中在主屏，窄屏保留主要操作而非无限堆叠卡片。
4. 运行/重连和检查失败不会把历史产物丢失或虚构成成功。只覆盖本路径需要的中断情形，不建设通用恢复编排器。
5. 从打包的单个 AgentX 产物可使用内嵌工作台；界面、原生契约、来源和当前支持可由新的维护 Agent 从入口发现。实际平台、Provider 和证据缺口如实记录。

## 本轮不做

多租户/组织权限、跨机器全局调度、通用 DAG 编辑器、自动成本优化、完整项目管理平台、遥测仓库与所有 Provider/平台组合不进入本轮。必要的日常入口和真实流程支持归入对应实施 Task，不单独扩展服务编排项目。现有 LAN 演示及其数据继续保留。

## 进展与反馈

执行状态以页首阶段和原始 Task 为准。规划反馈：用户认为现有页面粗糙，要求先专业 UI/SVG 设计，不能围绕现有字段排版；本轮用独立体验设计责任和具体设计讨论承接。新增需求先说明它如何帮助本 Delivery 的用户结果，再由原责任人纳入或暂缓，不以“顺手完善”为理由扩张范围。

两项设计 Task 已完成各自交接：[体验/交互与四张 SVG](../../.docs/collaboration-workbench/README.md#actual-visual-offering)和[能力/工程设计](../../.docs/collaboration-workbench/capabilities.md)已相互对齐。控制者完成静态版面走查，当前进入用户讨论具体主屏与交互的环节；因此设计阶段与整体 Delivery 仍保持 active，三项实施/验收任务仍 planned。推荐 React + Vite 构建工作台并由 Rust 内嵌，原生协作采用最小工作记录和明确操作语义；这些设计选择尚不是实现或运行验收结果。

用户在实际预览后指出：首版解释性文字过多、有效信息与交互不足，没有把核心资源体系作为主屏结构。原体验 Task 已重新 active，原设计者按[本地设计与审查标准](../../.agent-guides/scenarios/interface-design.md)重新推导布局，可大幅重做。原能力设计保持已发布状态，只有真实语义缺口才返还领域责任人；不因此新建边缘任务或启动生产实现。原预览保留为比较依据，不能作为已选定的 UI 实施基线。

该次重做现已完成原始 Task 交接，当前[实际设计入口](../../.docs/collaboration-workbench/README.md#actual-visual-offering)已更新为资源工作区：来源分组、任务与角色/结果/检查/待办在主屏对齐，问题、分工和结果验收在下方局部处理。四个最终状态已渲染并按本地六项标准检查，[最终观察](controller.md#final-revised-design-observation)保留结果和静态限制。两项设计 Task 再次均为 completed，设计阶段继续 active 以承接用户讨论；三项实施/验收任务仍 planned，尚无生产实现或用户接受的声明。

用户随后认可整体方向基本可用，并明确要求继续补齐各资源、操作、重要状态样式、组件和布局结构。当前沿用资源工作区主结构，将原体验 Task 重新 active 做[详细设计](controller.md#resource-operation-and-component-detailing)；不重复征询整体方向，也不把四张总览图当作完整设计。详细设计和实际原生契约会逐步释放对应实现条件，所有细化仍归属于本 Delivery 的任务分派到验收链路，未选择的边缘功能不扩成主线。

该细化现已完成原始 Task 交接：[资源、操作与组件设计](../../.docs/collaboration-workbench/interface-details.md)和新增九张图补齐资源透视、创建/编辑/选人、消息回执、检查失败/历史/重查、冲突及共享状态。控制者核对了十三张最终源与渲染预览，[接收记录](controller.md#final-detail-receiving-outcome)保留具体纠正和静态证据限制。未齐备分工的示例改为无原生编号的本页草稿，不新增无角色持久任务能力。两项设计 Task 均已完成；设计阶段继续承接细化讨论，实施与独立验收仍 planned。原生声明、真实组件与运行证据由既有实施/验收 Task 承接，不因图稿完成而声称功能已实现。

## 当前实施授权与推进顺序

用户现已确认规划/设计可用并授权按契约推进实施：保持资源领域/模型、对象方法与 UI 合理、自洽，先实现组件，再组装 Blocks，最后实现页面状态与方法。设计阶段的交付条件已满足，阶段转为 completed；这表示可用设计交接完成，不表示运行效果已验收。实施阶段 active，原生与 Web 两项原始 Task 承接生产源码；独立验收 Task 同时准备关键场景与资源，outcome 阶段在真实协作结果前仍 planned。

原生负责人首先将模型/方法草图落为可消费的声明、边界和模块骨架，再填充规则与适配；Web 同时落实组件合同、布局与叶组件，用发布的原生声明组装 Blocks 和页面状态。实际接口有可用早期释放即可衔接，不要求等待所有后端完成，也不以小组件/边缘状态拆新 Task。独立执行者消费稳定源码与案例后统一验证，源作者处理有具体依据的缺陷。分工、写入和执行资源以[当前实施分派](controller.md#implementation-dispatch)为准。

复用当前设计与既有能力；只有威胁契约正确性、核心用户结果或实际可用性的缺口才阻塞相关工作。兼容的实现细节由原责任人直接解决，不为单文件、组件、样式微调重复确认，不新增审批阶段。最终接受依据仍是本 Delivery 的真实分派到结果、独立检查和显式接受链路。

## 首个源码候选进入独立验收

`native-workbench-r1` 与 `web-workbench-r1` 已完成源码、案例、上下文交接，双方运行源码写入权已释放。原生领域规则/方法/持久化/CLI/HTTP 与 Web 组件/Blocks/Page 已有实际实现；这仍是源码完成，不能写成已验证支持。独立验收者收到两份一致候选后开始冻结与必要检查，outcome 阶段转 active。

实施阶段暂保留 active，承接原生负责人同一 Task 的[有限存储规则与模型导航接收](controller.md#coherent-source-receiving-and-storage-policy-transfer)：仅登记 daemon 借用存储的既有路径，无新建连接或客户端回退。该输入可与不受影响的候选格式、类型和编译检查并行；策略/文档更新后派生确切候选，保留之前有效证据。后续具体缺陷仍回原责任 Task 修复，不新增审查流程。阶段关闭以实际接收完成为准，整体 Delivery 继续 active，真实协作与浏览器/单文件结果尚待证据。

上述有限接收补充现已释放，另有验收方指出的 `workbench --name` 兼容解析缺口也已由原生负责人修正并附案例，两项[确切补丁](controller.md#bounded-receiving-corrections-released)直接交独立执行方。原生 Task 再次关闭并释放所有写入；两项实施 Task 均完成源码/案例/上下文交接，实施阶段转 completed。该状态不代表构建、浏览器或真实协作通过；outcome 与整体 Delivery 仍 active，实际结果由独立证据决定。

独立检查随后通过了候选格式、五项 Web 状态用例、修正后的严格 TypeScript 与严格存储归属规则。实际构建在进入 Rust 前被 Vite 原生依赖的 SIGILL 阻塞；相同 Node/环境下的最小导入已复现，诊断页构建可用。原 Web Task 接回有限的兼容依赖选择与构建修正，实施阶段相应 active；验收保留已通过证据，收到解释清楚的确切修正后先做有区分力的验证，再恢复后续构建。[当前接收与因果依据](controller.md#fixed-candidate-checks-and-build-tooling-diagnosis)引用原始证据，不增加工具链平台或反复盲试。真实协作、浏览器与发布产物仍待执行。

同版本官方 WASI 绑定已在独立环境通过导入、全新前端构建和严格类型检查，Web 原任务据此关闭。随后原生编译发现的 SQLite 整数表示问题也已由原负责人在同一 Task 内修正并释放案例，两项实施 Task 再次完成源码交接，实施阶段恢复 completed。原生修正的编译与用例结果、浏览器和真实协作仍由 active 的独立验收阶段承接；不能将源码阶段关闭等同于整体交付完成。

## 已接受的本地实现结果

控制者于 2026-11-09 接受本 Delivery 的首个完整协作切片。依据是独立执行的真实结果和最终产物交接，源码完成或 Task 数量不单独构成验收。前文保留各阶段历史，当前状态以页首、原始 Task 和本节为准。

| 验收条件 | 已建立的结果与依据 |
| --- | --- |
| 发起、分工与待输入 | 真实浏览器创建/保存/派发，精确角色绑定，实现者实际提问、操作方答复；[真实问题与上下文](evidence/acceptance.md#real-question-unknown-outcome-and-retained-ui-context)区分消息、命令和工作效果。 |
| 真实产物、独立检查和明确决定 | 同一真实仓库中，基线检查失败→显式返工→不可变修复→独立检查通过→指定结果/check/b1 接受；[结果与接受证据](evidence/acceptance.md#exact-repaired-result-and-explicit-operator-acceptance)保留来源、检查和公开角色工具记录。 |
| 主屏上下文与窄屏交互 | 任务/角色/结果/检查关系及局部操作可达；最终 390px 长标识布局、触屏 Tooltip/Escape、实际 criterion 和完整 Git 产物引用通过[最终布局及产物验证](evidence/acceptance.md#final-narrow-evidence-layout-correction)。 |
| 失败、未知与历史保留 | 提交后丢失响应的原命令查询、字段冲突、失败刷新、历史失败、取消无副作用、过期确认拒绝和不可用精确角色均有对应独立记录，未把回执或空闲推导为验收。 |
| 独立二进制、维护入口与清理 | 最终 r8 在无源码/无外部 assets 的其他目录运行，双应用嵌入字节与新 bundle 一致；相同产物已交付仓库 `.bin/agentx`。临时角色、浏览器、host/daemon、端口及 fixture 根均清理；[最终交接](evidence/acceptance.md#final-artifact-evidence-and-cleanup-handoff)和当前[原生](../../.docs/collaboration-workbench/native.md)/[前端](../../.docs/collaboration-workbench/frontend.md)文档拥有确切依据。 |

实际支持为 Linux 本地、明确作用域/项目、数值 LAN 浏览器，以及隔离 full-access Codex 双角色的显式 staged CLI/native 路径。原生/真实角色结果在 r5 建立，r8 仅组件/CSS 变化，已通过最终内嵌浏览器检查；未重复不受影响的领域与角色流程。其他 OS/Provider、旧版配置 hook 的全面兼容、workspace-write、实体触屏、辅助技术及规模/缩放矩阵不在本轮结论内，具体边界见[原生支持](../../.docs/collaboration-workbench/native.md#independently-exercised-support-scope)与[前端支持](../../.docs/collaboration-workbench/frontend.md)。

旧本地产物和全部失败/通过记录已独立保留；已安装/PATH/default 程序及既有 LAN 演示保持原样。当前选定结果无已知实质缺陷或未释放执行资源。后续新能力回到原 Proposal/Roadmap 选择，不自动扩大本 Delivery。

