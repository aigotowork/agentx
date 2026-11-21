---
schema: document
status: active
owner: flow-atlas-experience-design
last_updated: 2026-11-11
description: Scenario-led experience proposal for the source-first domain canvas workbench.
links: [./workbench.md, ../domain/contracts.ts, ../prototypes/experience/README.md, ../../../.controller/domain-daemon/tasks/flow-atlas-experience-design.md]
---

# 领域画布：从对象走到证据

本轮“协作流程｜场景时序”方案已实现并完成独立走查：[打开联动工作区18961](http://192.168.2.178:18961/)。[父任务最终结果](../../../.controller/domain-daemon/tasks/flow-atlas-linked-workspace.md#accepted-linked-workspace-and-publication)记录准确支持、呈现修正、离线工件和证据；下文保留设计推导，实际用户体验反馈仍可继续修正方案。

## 多视图连续性：本轮用户反馈后的设计建议

用户实际查看原型后指出流程、时序等视图仍然割裂，希望统一表达、合并适合的内容，并可并排对比。以下建议已进入本轮有界源码实现，结构与实际交接见[联动工作区](#联动工作区本轮源码设计)；新构建、画面验收和发布由[父任务](../../../.controller/domain-daemon/tasks/flow-atlas-linked-workspace.md)拥有。已发布18960的能力与原走查证据保持原范围。

当前视图确实消费同一份工件，但语义层次不同：流程使用 `ProcessDefinition.steps/flows` 描述允许的分支；选定场景后的时序使用 `DomainScenario.visits` 和 `Checkpoint.calls` 描述该场景记录的调用；对象状态来自同一检查点。原生 app 也保留场景、选择与播放进度，且相机按视图保存。不过同一交互在不同视图中换了布局和符号，用户必须自己对照；单视图页签缺少同时可见的语义对应。

原基线存在一项具体表达缺陷：`workbench/views.js::sequence` 在无场景时把全部 operation steps 按数组顺序展开。订单示例包含 begin、reserve、confirm、reject，其中 confirm/reject 是不同结果分支；无条件排成一次顺序会造成错误理解。本轮源码改为无场景只显示选择提示，协作图继续显示条件设计；不能从 steps 数组顺序推断一条合法路径。这里的缺陷判断来自源码和既有JSON核对，实际运行验证由父任务负责。

### 统一语义主题与视图组合

展示应围绕现有的“对象之间的一次公开操作交互”组织：步骤引用调用方、拥有者、operation 与参数来源；场景 visit/call 再给出这次出现的输入、拥有者效果、调用方结果与完成方式。继续从现有契约派生投影引用，不新增一份人工维护的图模型或业务事实。定义引用保留 processId/stepId，场景引用进一步保留 scenarioId/visitId/callId，阶段来自当前 checkpoint。

| 用户问题 | 主表示及适合合并的信息 | 按需辅助表示 |
| --- | --- | --- |
| 有哪些对象、谁拥有哪些能力与状态？ | 对象画布内显示身份、核心字段、方法 Port 与紧邻规则；选中关系突出当前协作。 | 选中对象的生命周期或原生Schema。 |
| 一项用例由谁完成，按什么条件继续？ | 对象泳道的协作流程：对象是责任分区，操作是步骤，结果是分支；选中步骤再叠加参数来源与状态效果。 | 精确的调用／回复时序。 |
| 一个参数从哪里来？ | 在同一流程步骤上下文聚焦来源结果、目标参数与Schema，保留返回原步骤的位置。 | 完整数据依赖图。 |
| 该场景走了哪条路径、谁知道什么？ | 在同一协作流程高亮选定 visits，邻近对象展示已应用状态，回复和独立效果保持区别。 | 并排的该场景时序。 |

流程与时序可以统一其交互引用、参与者和场景路径，但完整分支图与一条具体路径具有不同含义。对象泳道协作图可作为解释用例的默认主图，精细时序仍作为专门观察方式。合并采用按任务披露的图层，避免把所有字段、分支、参数、状态机与消息永久叠满主图。

### 并排工作区与联动规则

建议先做桌面双视图预设“协作流程｜场景时序”，而非五张图同时铺开。主视图约占三分之二，辅助视图可调整、收起或换成对象状态；参数和原生Schema继续局部披露。窄屏使用主图与可展开辅助视图，保留同一语义焦点。

同一场景的双视图共享场景、已应用检查点、语义选择和唯一播放器；点流程中的 reserve，应同时定位对应的时序调用、参数和拥有者状态。相机和几何保持各自独立，联动步骤／调用身份，而非让不同图形强行使用相同坐标。

“同一场景的不同视图”与“不同场景的对比”使用不同同步方式。比较成功／拒绝／丢响应时，两边分别保留自己的场景事实和检查点，以步骤与阶段对齐；不能用相同检查点编号假定含义一致。缺失的回复／未走到的步骤／未记录的证据应分开显示，不补造事件。对齐游标只是展示状态，不能改写两边的事实。

首个完整验证路径：选Checkout和一个场景 → 左侧查看完整分支并高亮已选路径 → 右侧查看同一次调用的时序 → 选reserve，双侧及参数/状态保持对应 → 播放并暂停 → 比较成功与丢回复，看到后续confirm和回复的有无，以及库存效果保持。当前实现仅承诺同一场景的双窗同步；比较保持各场景最终快照，不宣称跨场景对齐播放。

## 联动工作区：本轮源码设计

主要问题是“一次调查怎样同时理解条件设计、所选路径和当前已应用事实”。主体沿用原生 process/step/flow、scenario/visit/call 和 checkpoint，不添加人工维护的图数据或执行模型。步骤定义的身份是 processId/stepId，精确访问再带 scenarioId/visitId/callId/phase；相机只拥有几何。用户从图中选中交互后，详情就地呈现参数来源、当前调用方所知、独立拥有者效果和当前拥有者状态，再回原窗继续播放或调查。

| 层 / 源 | 本轮责任与边界 |
| --- | --- |
| `domain/contracts.ts` / 导出 JSON | 唯一语义、场景路径、检查点、证据；只读。 |
| `workbench/app.js` | 唯一上下文、语义选择、原生 player 实例和主窗对象布局；选中不再隐式换图。 |
| `workbench/collaboration.js` | 责任泳道、操作卡、全部条件边、精确访问标记；图深度仅为几何，不构造执行。 |
| `workbench/views.js` | 原有投影和场景时序；只读 visits 决定顺序，call ID 决定重复访问的路径和动画。 |
| `workbench/linked.js` / `linked.css` | 单/双窗、辅助时序/状态、1000px及以下局部切窗、分离相机、语义定位与只读快照；不另建时钟或业务 reducer。 |
| `workbench/details.js` | 从已应用前缀查找所选访问的参数证据；输入来源和当前状态独立披露，不借用另一次同名步骤的记录。 |
| `prototypes/experience/{build,projection,enhancement}` | 使用上述同一实现；保留对象/参数投影和体验布局、轨道、返回历史及原生安全 DOM。 |

```text
上下文栏：用例 / 场景 / 终点比较
主工作区
├─ 图种条：显式选择主窗图种
├─ 联动条：单/双窗 · 辅助图种 · 窄屏切窗 · 所选交互 · 图例
├─ 主窗：协作泳道/步骤/完整分支 + 选定路径与已应用前缀
└─ 辅助窗：场景时序或对象状态，独立相机
局部覆盖：参数与调用/拥有者证据，关闭返回原引用
原有场景底栏：一条检查点轨道 + 调用方/效果 + 唯一播放器
```

桌面将可用空间约按1.35:1分给两图；主协作泳道每列218世界单位，时序只保留所选场景实际使用的角色实例，每列164单位。未调用的其他实例仍在资源目录/对象调查，不挤进时序。首入保留可读比例与参与者/责任头部，完整长图由共享 pan/zoom 调查。主窗可显式切回对象/参数/状态/时序；辅助窗不挤占为第三列。1000px及以下切换同一工作区域的主/辅助窗，场景、播放器、选择和两相机保留，不把两图变成长页面。检查器以有界覆盖层披露，避免同时缩小两张图。

主控第一轮图像揭示三项有界问题，原因与对应图片保留在[源码任务](../../../.controller/domain-daemon/tasks/flow-atlas-linked-ui.md#bounded-visual-findings-and-derivative)。后续源码将示意载荷从移动文字改为窗头“参数/数据”局部披露：运输点继续沿精确路径运动，完整安全文本在有界 popover 中按需读取，避免盖住操作名与分支。泳道头部使用按字形宽度预算的省略和独立 SVG 裁剪，完整名称/职责/状态保留在可访问名称、title和详情中。窗格重新可见时，即使语义焦点没有改变，也在原 scope/缩放中定位所选 occurrence；不重置相机比例、播放器或检查点。这些是针对已观察问题的源码修复，受影响复验仍由主控执行。

后续技术模型图像又确认终点标题与结果共享一行会合并为 `Layout may movecompleted`。终点卡现将标题和语义结果分成两行，复用正常字号、字形宽度预算与完整title，保持44px高度和既有图形几何；具体图片、源码范围及受影响验证归属见[最小终点修复](../../../.controller/domain-daemon/tasks/flow-atlas-linked-ui.md#terminal-label-derivative-source-release)。

声明边始终存在；`data-scenario-path` 表示明确场景路径，`data-applied` 表示当前检查点之前已应用。缺少可选 viaFlowId 时，只把实际相邻 visits 和上一次结果匹配到唯一已声明边；多条候选保留未定，不猜一条。终点边还需当前 terminal 检查点才能标为已应用。两者有不同线型与局部图例。时序的未来请求以“未应用”标记；效果、本地结果和实际回复只来自已应用 call，示意中的回复还必须有 reply 完成方式。未知不会变成成功，local-outcome不会出现返回箭头。

重复步骤的定义点击保留定义焦点，编号访问标记和访问列表才选择精确 occurrence；两窗定位相同 visit/call。跟随同一步骤的参数引用保留该 occurrence，来源步骤的引用重新按其真实身份调查。未来访问详情只展示明确标注的场景输入，当前证据和拥有者状态仍来自已应用前缀。布局/相机/切窗不 seek；暂停和选择不清空 elapsed。更换过程/场景取消旧示意并清除不再属于该上下文的选择。比较入口仍标为终点事实，未实施不同场景的同步播放。

固定 hooks、`state().linked` 分离快照和源码交接由[本轮源码 Task](../../../.controller/domain-daemon/tasks/flow-atlas-linked-ui.md#structural-contract-2026-11-11-source-author)拥有。上述尺寸、交互和边界是源码设计与作者用例，不是已渲染或通过验收的声明；父任务拥有独立L1–L9、截图、构建收据、新预览及实际支持。新工作保留18957–18960和全部旧输出，不用新源码倒写历史验收。

## 设计基点

用户反馈是“UI 风格、交互友好度不足”；并未否定已接受的领域模型。产品对象是需要理解、设计和评审领域协作的作者与协作者，主要工作是调查：谁拥有状态，谁提供方法，参数来自哪里，某条场景到底证明了什么。语义编辑仍由 TS 源拥有。原型不增加业务写入、执行或源码推断。

真实渲染基线由[父任务](../../../.controller/domain-daemon/tasks/flow-atlas-experience.md)拥有；主控指出：规则/语义值的展开目录挤走用例与场景、方法和字段缺少分组、外围绕线扩大空白、窄屏对象被离屏连线包围、参数来源与消费方分居视口两端。独立渲染与走查已完成，主控对所检查场景确认有实质改善；精确范围见文末支持与证据，用户判断仍开放。

采用对象中心的工作台，而非按技术图种组织的查看器。设计方向为浅色纸面、深墨文字、靛蓝交互和少量语义色：左侧领域目录，中央可移动对象画布，右侧就地检查器，底部场景证据轨道。视图选择位于画布顶部，过程与场景位于固定上下文栏。没有可调用的 Design Canvas 插件，使用原生 HTML/SVG、既有工作台逻辑和独立呈现适配器表达这一方向。

## 场景与信息需求（原型的唯一用例源）

| Key / 用户意图与触发 | 同时需要的资源与权威字段 | 行动 → 可观察反馈 → 返回 |
| --- | --- | --- |
| EX1 定位领域责任：首次进入或搜索对象 | `WorkbenchDefinition.contexts/modules/objects`；`DomainModule.contextId/dependencies`；`ObjectDefinition.moduleId/responsibility/consistencyRootId` | 领域目录选对象 → 画布定位并突出对象、右侧显示职责/所属模块/身份 → 关闭检查器回到所选对象；视口与对象位置保留。 |
| EX2 理解可调用能力：选择公开 Port | `OperationDefinition.ownerId/kind/inputSchemaId/outcomes/effects/ruleIds/binding/nativeMethodId`；`ObjectDefinition.stateSchemaId/fields` | 命名的公开方法区选择 Port → 输入、结果、状态效果和流程使用就地展开 → 从所属对象或返回按钮回到对象。公开方法不是图上的泛用连接点。 |
| EX3 追踪参数：从方法进入流程使用，再选参数 | `DomainStep.operationId/bindings`；`ParameterBinding.targetPath/source/valueId`；`ParameterProvenance.status/value/actual/sourceCallId` | 流程使用 → 步骤 → 参数来源 → 数据流画布与检查器展示来源/目标，结果来源可再定位其步骤和结果 Schema → 检查器返回保留原场景、检查点、布局。 |
| EX4 区分回复与效果：选择丢回复场景或拒绝场景 | `CompiledScenario.checkpoints`；`Checkpoint.calls/states/derived/differences/parameters`；`CallProjection.phase/completion/effectOutcomeId` | 选择场景 → 初态；点轨道检查点或示意播放 → 已应用事实与目标动画明确分开；调用方所知和拥有者效果并列 → 暂停、逐步、返回设计。未知不得显示成功回复；库存拒绝不得伪造预留。 |
| EX5 评审规则与覆盖：打开检查或对象规则 | `WorkbenchArtifact.diagnostics/coverage`；`DomainRule.basis/check/sourceIds/outcomeIds`；`Checkpoint.assertions/diagnostics`；`AssertionResult.basis/status` | 检查范围 → 真实诊断定位（若存在）或覆盖项 → 规则/方法来源 → 返回原对象；零诊断不显示“模型正确”。文字声明、源码对应与案例检查有不同标签。 |
| EX6 调整视野：拖动、缩放、布局或窄屏调查 | 布局/相机为 presentation；语义仍由以上数据拥有 | 共享相机平移/缩放与对象布局 → 位置变化、固定/撤销/导入导出；切视图/开详情后仍保持语义选择；390px 用抽屉调查，触摸可达，Escape/关闭回到原始触发点。 |

类型精确定义见[已实施合同](../domain/contracts.ts)，不在此复制 Schema。五种图是同一语义资源的投影：对象解释责任与可调用面，流程解释分支，参数图解释数据供应，状态图解释合法转换与已访问状态，时序图解释调用方所知和独立效果。

## 各模块的用户问题与责任边界

| 用户真正需要判定的事 | 对应的原生资源 | 主工作面 → 深入路径 |
| --- | --- | --- |
| “我在哪个术语/责任边界里？” | `Context.responsibility`；`DomainModule.contextId/responsibility/dependencies` | 目录的上下文→模块→对象；对象标题显示上下文/模块，详情可沿模块依赖调查。模块不是运行进程。 |
| “这是同一个实体，还是同类型的另一个实例？” | `ObjectDefinition.businessIdentity/consistencyRootId`；`ScenarioInstance.id/objectId/businessIdentity/consistencyRootInstanceId` | 定义时显示类型；场景时显示实例名与其字段，详情区分画板 ID、业务身份与一致性根。相同类型不合并。 |
| “这个数据由谁拥有，哪些是推导的？” | `DomainField.path/valueId/derived`；`ObjectDefinition.stateSchemaId`；`Checkpoint.states/derived/differences` | 对象状态分区→字段→Schema/语义值；`ƒ` 标记只读派生。变更高亮来自实际差异，不能在画布上直接写状态。 |
| “形状一样，业务含义是否也一样？” | `ValueDefinition.schemaId/meaning/unit`；`InputField.valueId`；`ValueSchema.jsonSchema` | 字段/参数→语义值→原生 Schema；缺少单位保留“未指定”，不按名称猜兼容性。 |
| “我可以请求谁做什么，会得到哪些结果？” | `OperationDefinition.ownerId/kind/inputSchemaId/outcomes/effects/ruleIds/binding/nativeMethodId/nativeOutputSchemaId`；`OutcomeDefinition.payloadSchemaId` | 命名 Port→输入/结果/效果/规则→流程使用/原生对应。命令和查询有文字种类；调用者空心起点不是另一套可调用 Port。 |
| “这一条规则有多强的依据？” | `DomainRule.ownerId/basis/check/sourceIds/outcomeIds`；`CoverageItem.status/detail`；`AssertionResult.basis/status` | 对象规则或检查范围→规则及源码→该场景断言；文字、源码对应、已检查案例各保留原依据。 |
| “协作关系和执行先后分别是什么？” | `DomainRelationship.kind/cardinality/fromObjectId/toObjectId`；`ProcessDefinition.roles/steps/flows/startId` | 对象关系/合作线→精确步骤→流程结果分支；参数图另解供应。关系不等于已调用，分支声明不等于已访问。 |
| “下一步的参数从哪个结果、哪一次调用来？” | `ParameterBinding.source/targetPath/valueId`；`ParameterProvenance.sourceVisitId/sourceCallId/status/value/actual` | 方法的流程使用→步骤参数→并列来源/目标；用例输入、身份、结果、常量有显式类型，缺证据时保持定义态。 |
| “哪些转换合法，当前实例走过哪一条？” | `ObjectDefinition.lifecycle`；`LifecycleTransition.operationId/outcomeId/from/to`；`StateDifference.transitionId/causeVisitId` | 状态页签→合法转换与当前/已访问标记→拥有者/方法；不以全部枚举组合构造虚假状态机。 |
| “实际记录了什么，调用者知道多少？” | `ScenarioVisit.completion/effect/evidence`；`CallProjection.phase/completion/effectOutcomeId`；`Checkpoint.states/calls` | 场景轨道与调用方/拥有者证据对→时序/调用详情；未知保持未知，效果单独可见，播放没有业务执行权。 |
| “是否符合我声明的预期，下一步查哪里？” | `ScenarioExpectation`；`Checkpoint.assertions/diagnostics`；`WorkbenchArtifact.coverage/diagnostics` | 预期与证据/比较/检查→真实定位链接→原资源；没有诊断不等于普遍正确，终点比较不是当前检查点。 |
| “如何回到调查现场？” | 原生选择、相机 scope 与布局文档（均为呈现态） | 引用返回、关闭、视图切换、暂停及布局撤销；保留真实过程/场景上下文，禁止回放动画被解释为业务补偿。 |

## 状态与反馈模型（复用原生所有者）

| 当前状态 / 触发 | 画面反馈与保持项 | 所有者 |
| --- | --- | --- |
| 定义态 / 未选择场景 | 对象与方法声明；没有播放器，也没有虚构字段实际值。 | 原生 app 的 scenarioId 为空。 |
| 场景初态 / 选择场景 | #0 的导出状态；轨道可跳转；证据依据随该场景显示。 | 原生 player.context。 |
| 示意中 / 播放 | 目标检查点虚线标记和原生运动；字段与调用证据仍是已应用检查点。 | 原生 player 的唯一 RAF 时钟。 |
| 暂停 / 用户暂停、布局或页面隐藏 | 已应用事实、elapsed 和选择保留；下一步/继续/切视图仍可用。 | 原生 player.pause；布局/页面调用原有暂停。 |
| 已应用 / 示意到边界或用户跳点 | 仅读取该检查点；轨道实选中、字段差异、调用所知与效果同步更新。 | 原生 player.seek 或边界应用。 |
| 资源调查 / 选择或返回引用 | 检查器局部变化，原生引用决定相关视图与定位；返回栈只保存引用触发点。 | 原生 app.select；增强层只重放原有 DOM 事件。 |
| 视野调整 / 相机、拖动、固定/撤销 | 相机或对象几何变化；语义 ID 与检查点保持；实际 Port 与线端一同更新。 | 共享 viewport 与原生 layout。 |

悬停/焦点只帮助发现可操作对象；选中由边框、局部底色和 ARIA 共同表达。正在示意与已应用使用不同边框/标签；未知与已提交有文字事实，不依赖单独颜色。没有另建“运行状态”或可写表单来伪装作者能力。

调用方证据中的“结果未知”与“收到回复”是两个维度，按原生 `CallProjection.phase` 与可选的 `completion` 分别呈现：`unknown + reply` 显示“已收到回复 · 结果未知”；`unknown + local-outcome` 显示“未知 · 无返回”；未记录 `completion` 则显示“未知 · 返回方式未记录”。不以未知结果或 payload 缺失推断没有回复。三种情况都不改变独立的拥有者效果或权威检查点；收到未知 envelope 也不能标成业务成功。这一边界来自主控独立惰性派生输入的真实走查发现，不构成新的业务执行证据。

## 信息架构与组合骨架

```text
ExperienceWorkspace
├─ ModelHeader: artifact identity · prototype scope · checks
├─ WorkContext: process · scenario · compare
├─ WorkspaceLayout
│  ├─ ResourceNavigator: model / use cases · search · scoped tree
│  ├─ CanvasWorkspace
│  │  ├─ ViewStrip: objects / process / parameters / state / sequence
│  │  ├─ Projection: object groups + selected collaboration, or canonical linked view
│  │  └─ ViewportControls + LayoutPopover
│  └─ SubjectInspector: back · current resource · contract/evidence · close
└─ ScenarioDock: checkpoint rail · applied facts · separate caller/effect · player
```

固定区只保留当前模型、过程、场景、五个关联视图与检查入口；侧栏规则和语义值默认折叠，用例/场景有独立目录页签。对象卡片区分标题/职责、状态字段、公开方法、规则；Port 有名称、操作种类和足够的选中/触摸区域。关系线优先强调当前选中对象或方法的相关协作；未选择时呈现过程调用，使用相邻列间通道直达真实 Port，避免用外围线路包住整幅画布。精确步骤和结果分支始终由流程/时序/参数视图及详情拥有。双列默认布局将首个操作的拥有者与其调用者/其他拥有者放在相邻列；重复实例保留独立身份并在同列排列，不因同类型而合并。

检查器共享原有引用选择行为，新增短返回栈仅记语义引用而非复制业务状态。次要来源、完整输出和扩展元数据用局部折叠；输入与结果、参数证据不藏入总览说明。窄屏保留模型/场景/视图和底部控制，资源与检查器使用互斥抽屉，不把同一任务拆成多页。

## 实现边界与早期交接

原生 `workbench/{dom,details,player,layout,app}.js`、`spatial/viewport.js`、`viewer/layout.js` 全部只读复用。原生 `views.js` 仍拥有完整流程、参数、状态、时序与动画；体验投影覆盖对象视图的几何、分组与聚焦关系，并为选中参数增加来源→消费方的局部镜头，均返回相同 projection 合同。原生参数全图由“关联全图”保留，可切回聚焦而不清空选择。既有对象投影的固定 300px 无分组卡片、全图外围绕线及参数两端过宽无法通过 CSS 解决，主控已授权小型用途明确的呈现适配器；不复制 app/player/reducer。

原型入口保留全部原生 DOM ID、`FlowAtlasWorkbench.state()` 和语义 `data-*`。新增 `data-testid="experience-view-{objects,process,dataflow,state,sequence}"`、`experience-nav-model`、`experience-nav-cases`、`experience-back`、`experience-binding-focus`、`experience-checkpoints`、`experience-checkpoint-{index}`、`experience-caller`、`experience-effect`、`experience-scope`。组合顺序：原生相机/布局/DOM/views/details/player/layout → 体验 projection → 原生 app → 体验 enhancement。构建器只读 JSON 与维护源码，生成独立 HTML 与收据，绝不读取或执行用户 TS。

样式分为 tokens、workspace layout、component treatment；无网络字体、图标包、框架、安装或依赖新增。静态线条 SVG 图标由原型拥有。构建、浏览器走查、截图、发布由主控独立执行；源作者不将源码检查冒充渲染证据。

## 关键取舍与待确认偏好

| 选择 | 理由 / 拒绝项 |
| --- | --- |
| 对象设计作为无场景默认入口 | 作者先理解定义；不默认选择“成功”并把见证误读为一般运行事实。 |
| 用例/场景独立目录页签，顶部仍有直接选择 | 与对象调查同一工作区；拒绝把所有分类永久展开或另建单独页面。 |
| 只在语义相关边上强调合作 | 降低全局绕线负担；精确每步仍有专用关联视图。拒绝只换配色而保留原有空白几何。 |
| 声明/源码绑定/案例证据在资源位置显示 | 拒绝统一绿色完成标记；“passed”仅属于对应断言，动画完成不产生新事实。 |
| 轻量原生源原型 | 当前没有可调用 Design Canvas；拒绝伪称使用插件或安装额外集成。 |
| 等待用户实际体验后确定风格 | 当前浅色靛蓝、对象卡片密度、聚焦边策略是设计选择；用户是否偏好始终显示全部连接、检查器默认宽度仍开放。 |

## 验证与后续归属

完整操作用例、稳定 hooks 和[已发布支持](../prototypes/experience/README.md#已发布预览与验证边界)由原型 README 路由。现在可打开[18960预览](http://192.168.2.178:18960/)、[订单离线页面](../examples/domain-workbench-experience.html)和[技术领域离线页面](../examples/domain-workbench-experience-technical.html)。[父任务](../../../.controller/domain-daemon/tasks/flow-atlas-experience.md)拥有主控实际执行、图像评估与最终闭合；服务器和输出精确身份按其链接记录读取，不在设计文档维护第二份资源清单。

已保留六组独立走查与源码/嵌入安全结果；最后文案修正后的两条受影响路径在 `walkthrough-final/result.json` 通过。实际观察包含1440×900对象层级/方法/来源镜头/未知效果与390×844初始/方法任务，检验原生引用返回、关联视图、时钟与检查点、相机/布局/真实 Port、窄屏触摸和技术模型复用。作者案例表仍不等于逐条执行证据，未记录 completion 的分支没有单列执行证据；完整边界、收据和发布/保留证据均见 README。此次文档接收不改变冻结的执行输入、生成文件或运行资源，不追加检查。

用户的风格、密度、参数默认聚焦与检查器宽度偏好仍开放；Agent 的有界走查不代替用户接受。当前设计为已有有限编译模型的体验提案，声明不是业务行为证明，authored/observed 是证据来源，expectations 是预期，动画是示意。来源 URI 仍按原生规则安全显示，不变为任意执行/文件链接。
