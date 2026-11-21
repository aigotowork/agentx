---
schema: document
status: active
owner: flow-atlas-experience-design
last_updated: 2026-11-11
description: Reproducible source-native offline experience proposal and authored desktop/narrow/keyboard cases.
links: [./AGENTS.md, ../../design/experience.md, ../../domain/contracts.ts, ../../../../.controller/domain-daemon/tasks/flow-atlas-experience.md]
---

# Flow Atlas · 领域画布体验提案

## 本轮联动源码与历史预览

[最新联动预览18961](http://192.168.2.178:18961/)已通过本轮独立走查并发布；可直接打开[订单离线页面](../../examples/domain-workbench-linked.html)与[技术模型页面](../../examples/domain-workbench-linked-technical.html)。[最终执行与维护范围](../../../../.controller/domain-daemon/tasks/flow-atlas-linked-workspace.md#accepted-linked-workspace-and-publication)拥有精确证据。下方18960仍是此前体验提案的固定版本。

新的[联动工作区任务](../../../../.controller/domain-daemon/tasks/flow-atlas-linked-workspace.md)在同一体验外壳中组合“协作流程｜场景时序”，复用原生 app、精确语义选择、唯一播放器与独立 pane 相机。源码模块、交互和 hooks 的单一入口是[工作台用法](../../workbench/README.md#联动工作区源码)与[固定源码契约](../../../../.controller/domain-daemon/tasks/flow-atlas-linked-ui.md#structural-contract-2026-11-11-source-author)。`build.py` 已注册新协作/组合模块与样式；`enhancement.js` 的返回历史保留访问身份，参数 lens 从共享的已应用 occurrence lookup 读取证据。源码作者未运行构建/浏览器/测试，实际执行与新发布由父任务独立记录。

下列18960及原收据/图像属于此前固定字节，继续保留。其“对象优先”和“原生源码只读”是当时范围；新的授权源码默认双窗，选资源会保留当前 pane 图种，单窗和显式图种切换仍可调查对象、参数、状态与时序。原案例表是历史作者用例，新行为的可操作验收以本轮父任务L1–L9为准。重建当前源码应输出到新路径，不能把旧收据当作新实现证据。

一套围绕“对象 → 公开方法 → 参数来源 → 场景证据”的离线交互原型。领域目录和用例目录分开，画布上明确分区状态、公开方法与规则；右侧检查器有局部返回；底部检查点轨道并排显示调用方所知与拥有者效果。

使用[真实订单模型](../../examples/domain-workbench.json)，也接受[独立技术模型](../../examples/domain-workbench-technical.json)。业务数据与检查点不复制成另一份手写场景；浏览器仍通过现有 `FlowAtlasWorkbench.state()` 读出同一导出事实。没有可调用的 Design Canvas 插件；这是原生 HTML/SVG 的体验设计提案。

## 已发布预览与验证边界

打开[订单离线页面](../../examples/domain-workbench-experience.html)或[技术领域离线页面](../../examples/domain-workbench-experience-technical.html)即可查看示例。[父任务](../../../../.controller/domain-daemon/tasks/flow-atlas-experience.md)记录交付状态。重新生成示例时，构建工具会把输入和输出摘要写入本地收据。

| 已执行范围 | 当前证据与边界 |
| --- | --- |
| Linux 本地静态浏览器，1440×900桌面与390×844触摸 | 原六组走查覆盖定位/Port/返回/键盘选择、参数来源与关联视图、场景慢放/暂停/已应用事实、相机/拖动/真实 Port 端点/撤销、窄屏触摸调查和独立技术模型。结果与实际图像观察见父任务；不把本页全部作者用例视为逐项执行过。 |
| 最后文案修正的受影响路径 | [最终结果](/tmp/flow-atlas-experience-du3ulrlu/walkthrough-final/result.json)记录 `scenario-clock-evidence` 与 `unknown-envelope` 两条通过：本地未知/无返回与实际回复/结果未知分别正确显示，效果和检查点保留。未记录 completion 的文案边界已写入来源与作者用例，此结果没有单列其执行证据。 |
| 构建、安全嵌入与原始输入 | 保留的 `builder-checks.json` 覆盖原始 JSON/检查点一致、资源收据、恶意模型文本惰性嵌入、非法输出保留与拒绝覆盖已接受示例。文案修正不改变这些机制，未重复业务/原生/编译器测试。 |
| 独立发布和旧资源保留 | [发布证据](/tmp/flow-atlas-experience-du3ulrlu/publication.json)记录实际文件摘要、本机/LAN响应字节与有限路由、34个原始文件和18957/18958/18959三个旧服务身份保留。18960只提供这份独立静态提案。 |

主控实际阅读了总览、方法、参数镜头、已提交/未知与窄屏图像，确认所检查任务的信息层级和连续性较基线有实质改善。这是有明确范围的 Agent 走查；用户的视觉偏好与验收仍开放。所有最终执行输入与发布字节已冻结，本次接收只更新文档，不触发重建或新一轮检查。

## 重建与来源

以下为已交付构建器的重建方式，从仓库根目录输出到新的独立路径；当前接收阶段不重新执行：

```sh
python3 tools/flow-atlas/prototypes/experience/build.py tools/flow-atlas/examples/domain-workbench.json --output /tmp/flow-atlas-experience.html
python3 tools/flow-atlas/prototypes/experience/build.py tools/flow-atlas/examples/domain-workbench-technical.json --output /tmp/flow-atlas-experience-technical.html
```

直接打开生成的 HTML 即可使用，无网络依赖、框架或安装。相邻 `.receipt.json` 记录原始输入字节摘要、所有嵌入资源和输出摘要。构建器只加载 JSON 与维护源码，拒绝覆盖输入、现存示例和维护的工具源；HTML 和收据分别原子替换。输出应选择独立路径。构建、桌面/窄屏走查、截图与18960发布均由主控完成，源作者仅提交来源与接收文档；真实结果按上面的证据边界解读。

| 来源与职责 | 处理 |
| --- | --- |
| `domain/contracts.ts` 与输入 JSON | 只读语义与权威检查点。 |
| 原生 `workbench/dom.js` / `details.js` / `player.js` / `layout.js` / `app.js` | 原样嵌入；引用选择、详情、唯一时钟、布局历史和上下文组合继续由它们拥有。 |
| 原生 `spatial/viewport.js` / `viewer/layout.js` | 原样嵌入；相机、世界坐标和布局文档继续共享。 |
| 原生 `workbench/views.js` | 原样嵌入；流程、完整参数图、状态、时序及动画继续消费其实现。 |
| `projection.js` | 对象视图的分区、双列默认几何、真实 Port 与相邻列连线；选中参数的来源/消费方镜头，可返回原生全图。共享同一 projection 的 Maps、boxes、routes 与 callPaths，不建 reducer 或第二播放器。 |
| `enhancement.js` | 组合现有 DOM 控件和引用事件；资源页签、检查器引用返回、局部折叠、检查点按钮、已应用事实、触摸/键盘披露。无自己的计时器。 |
| 模板、tokens / layouts / components | 布局骨架、可访问标签与浅色墨/靛蓝的局部视觉系统。 |

对象呈现适配器的必要 seam 是原生固定宽度无分区卡片与全局外围连线；参数 lens 的必要 seam 是两端在340px检查器打开时无法同时看清。两个适配都只消费导出事实和原生选择回调；原生 app、播放器、检查点与布局引擎未分叉。它不是一般避障路由器：任意拖放仍可能引起重叠/线路穿越，由既有布局诊断展示。

## 操作路径与验证案例

下表是作者用例，不是执行结果。场景需求和字段映射的唯一来源为[EX1–EX6](../../design/experience.md#场景与信息需求原型的唯一用例源)；独立走查须检查实际画面、动作和返回，而非只检查节点存在。

| Case / 范围 | 操作与需要观察的结果 |
| --- | --- |
| EX1 / desktop1440×900 | 首次打开为无场景定义；完整视图条、过程/场景、领域层级可见。规则和语义值默认折叠。Order、StockPosition、Checkout 的责任/字段/命名 Port 可辨；没有全局外围线路造成的空画布。选 Order → 画布定位、就地详情 → 关闭后选择与布局保留。 |
| EX2 / Port contract | 从画布 confirm Port 进入，看到所属 Order、输入 Schema 的字段与必填、结果分支、允许效果、规则、流程使用和原生方法对应。Schema 字段与参数可继续追溯；次要来源/完整输出折叠而可达。返回逐层回到原对象；不会出现无实现的 Save/Run。 |
| EX3 / selected parameter | Order → confirm → 流程使用 confirm → binding.confirm。参数镜头同时展示 reserve / reserved 来源与 confirm 目标，路径为 `$`；点击关联全图、再聚焦时选择不变。其他四视图仍共享同一 binding ID；返回回到原步骤。当前检查点无参数证据时只显示定义，不伪造实际值。 |
| EX4 / scene and pause | 通过“用例与场景”选择“已提交，但回复丢失”。初态显示#0；0.25×播放到请求中途，检查点仍#0。暂停、切时序再切回时 elapsed 与已应用检查点保留。跳到终点：调用方未知/无返回，拥有者已提交 reserved；当前权威状态仍有预留。无回复路径不出现成功返回。 |
| EX4 / unknown completion boundary | 对比原有 `phase:unknown, completion:local-outcome` 与主控惰性派生输入的 `phase:unknown, completion:reply`：前者显示“未知 · 无返回”，后者显示“已收到回复 · 结果未知”。两者拥有者效果仍为其原始 `effectOutcomeId`，检查点与 payload 原样保留；缺少可选 `completion` 时只显示“未知 · 返回方式未记录”，不能从 unknown 或缺 payload 推断无回复。派生输入仅验证呈现边界，不是新的业务见证。 |
| EX4 / refusal and mismatch | 场景选择库存不足并跳到终点：预留未增加。选择其他订单凭据/旧意图/数量不匹配：原始导出检查点、参数来源和结果保持不变。预期/比较读取现有终点事实；断言通过不代表任意业务执行正确。 |
| EX5 / rules and diagnostics | 检查入口即使零诊断仍可打开覆盖范围；声明、源码对应、实际断言有不同依据。规则 → 来源/相关方法 → 流程 → 返回。若输入有真实诊断，按原生定位链接到其资源；不要为演示生成假诊断。 |
| EX6 / camera and layout | 背景平移、光标处缩放/适合全部/100%；拖 StockPosition 的标题而不是字段，Port 路径端点仍等于真实 circle 中心。打开布局 → 固定/撤销/键盘方向移动/导出/导入；只改变布局，语义与检查点不变。 |
| EX1–EX6 /390×844 touch | 资源按钮打开抽屉，搜索 confirm → 选择后资源抽屉关闭，检查器打开；底部留在场景上下文。关闭/Escape回到画布。视图、场景、检查点与播放按钮都可达；对象初始按共享可读相机进入，可触摸平移查邻居，不缩成字点。 |
| EX1–EX6 / keyboard | `/` 聚焦搜索；Enter/Space选择资源、字段、Port、参数和检查点；Tab可达所有视图/披露；窄屏抽屉在首尾循环焦点；Escape关闭局部面板并按原生逻辑返回触发资源。图标名称可由 focus/touch Tooltip读取。 |
| Generic artifact | 用技术领域 JSON 重建：Playback、Pointer interaction、Pause before drag 和实际独立实例仍可见，场景轨道/五视图可用。不得出现订单专用 ID 判断或编造库存字段。 |

## 稳定检查 hooks

保留所有原生 hooks，包括 `workbench-canvas`、`subject-details`、`resource-navigation`、`view-select`、`process-select`、`scenario-select`、`player-*`、`viewport-*`、`layout-*` 与语义 `data-ref-kind/id/scope`、`data-node-id`、`data-port-id`、`data-binding-id`、`data-step-id`、`data-target-port/node`。

新增：`experience-view-{objects,process,dataflow,state,sequence}`、`experience-nav-{model,cases}`、`experience-back`、`experience-binding-focus`、`experience-checkpoints`、`experience-checkpoint-{index}`、`experience-caller`、`experience-effect`、`experience-scope`。检查点按钮有 `aria-current="step"` 与 `data-applied`；下一示意目标有 `data-target`，不提前更新检查点。对象卡片继续支持原生标题手柄、世界坐标拖动与动画。

## 提案范围

已声明定义、源码绑定、authored/observed见证、当前已应用检查点、预期和示意动画保持不同意义。浏览器不会执行原生方法、修改 TS、保存业务状态或推断未记录的效果。作者源中的过程仍为已有有限模型；原型不承诺并发、循环或一般障碍路由。

待用户实际体验后决定：浅色靛蓝风格与密度是否合适、是否喜欢参数默认聚焦、检查器的默认宽度与始终全显关系的取舍。Agent 的独立走查只能建立其记录范围的可操作性，不能代替用户接受。现有18959及所有旧viewer/demo/runtimes仍由原任务拥有并保留。
