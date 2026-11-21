---
schema: document
status: active
owner: flow-atlas
last_updated: 2026-11-11
description: Source-first domain-model workbench, contract inspector and finite trace viewer with portable offline artifacts.
links: [./domain/README.md, ./workbench/README.md, ./authoring/README.md, ./contracts.md, ../../.controller/domain-daemon/flow-atlas-workbench-implementation.md]
---

# Flow Atlas

Flow Atlas 用源码定义领域对象、方法契约、流程和场景，再生成可离线打开的工作台。同一份模型提供对象关系、流程、数据流、状态和时序视图；参数来源、拥有者状态、调用方所知和场景预期各有明确归属。

| 需要表达的内容 | 作者输入与语义入口 | 查看入口 |
| --- | --- | --- |
| 领域对象、身份、值、规则、方法、流程参数和具体场景 | [domain](domain/README.md)：TS 原生 Schema／方法引用与显式领域声明 | [领域工作台](workbench/README.md)：五视图、场景比较与播放 |
| 模块职责、方法契约、依赖和显式串行调用轨迹 | [authoring](authoring/README.md)：TS 契约及步骤绑定 | [Inspector](inspector/README.md)：结构与调用检查 |
| 已知行为的有限逐步讲解 | 本文的 JSON 轨迹：initial、patch、beats | 关系图、流程图与时序图 |

接入自己的 TS 项目，从[领域公共契约与原生示例](domain/README.md)开始。`domain/contracts.ts` 是领域工作台的已实现契约；[design](design/README.md)保留产品设计和更广目标。各入口共享适用的基础词汇、Schema 转换及画布能力，各自的输入格式和检查范围由其所属契约定义。

领域工作台已完成[本地实现、独立验收与发布](../../.controller/domain-daemon/flow-atlas-workbench-implementation.md#accepted-local-outcome)。[领域诊断说明](domain/lint.md)和[较早契约 lint 索引](authoring/lint.md)分别记录规则、实现位置、覆盖边界及维护入口。源码绑定不等于业务规则已被证明；oRPC 自动导入、任意源码流程推断、通用并发／循环和自动避障仍在当前范围之外。

## 打开领域工作台

最新的[联动双视图工作区](http://192.168.2.178:18961/)把协作流程和场景时序放在同一画布工作区。左侧合并对象责任、操作和条件分支；右侧显示同一场景记录的调用，也可换成对象状态。点击任一侧的步骤或访问标记，另一侧同步定位；两侧共享检查点和播放器，各自保留缩放与位置。

建议先选“成功”场景，点击 `Reserve inventory`，再以0.25×或0.5×播放；流动中的参数可从窗头“参数／数据”提示展开。换到“已提交，但回复丢失”场景，可以对照没有回复、调用方未知和拥有者效果已提交。手机用“协作／时序”切换同一个调查现场；“布局”可切单窗，辅助窗可切对象状态。不同场景的比较仍是最终快照比较。

[离线联动订单页面](examples/domain-workbench-linked.html)与[技术模型页面](examples/domain-workbench-linked-technical.html)可直接打开；[设计与模块责任](design/experience.md#联动工作区本轮源码设计)、[维护及重建](workbench/README.md)和[本轮执行证据](../../.controller/domain-daemon/tasks/flow-atlas-linked-workspace.md#accepted-linked-workspace-and-publication)说明实现与验证范围。下方旧演示继续保留原发布文件。

[局域网演示](http://192.168.2.178:18959/)展示订单／库存领域；同机也可用[本机地址](http://127.0.0.1:18959/)。[离线订单页面](examples/domain-workbench.html)和[技术领域页面](examples/domain-workbench-technical.html)可直接用浏览器打开。先点对象看身份、字段、方法与规则，再选择场景，切换数据流或时序视图观察参数和状态变化。

从已有 TS 契约生成自己的页面，使用支持 TypeScript stripping 的 Node（本次验证为22.23.3）、仓库已准备的锁定依赖和 Python3.10+，并选择尚不存在的 JSON 输出：

```sh
node --experimental-strip-types tools/flow-atlas/domain/examples/export.ts order-stock /tmp/my-domain.json
python3 tools/flow-atlas/workbench/build.py /tmp/my-domain.json --output /tmp/my-domain.html
```

把 `order-stock` 换成 `technical` 可生成第二个例子；自有模型通过[领域公共入口](domain/index.ts)编译，按相同格式交给构建器。导出与浏览器查看不调用业务方法。HTML包含模型、资源与已计算的检查点，可离线分享；布局JSON只保存画面位置。真实方法签名复用原生Schema，流程与参数来源显式设计，JSON／HTML可重新生成。当前是仓库内源码工具，尚未发布独立npm包。

本次独立验收覆盖22个语义案例、6种真实原生行为、4个错误成功反例、实际方法绑定和零业务执行的静态导出、离线安全及桌面／390px触控路径。实际能力与证据由[验收记录](../../.controller/domain-daemon/tasks/flow-atlas-domain-acceptance.md#accepted-final-derivatives-and-publication)拥有。

## 新的画布体验提案

[打开新原型](http://192.168.2.178:18960/)与[现有版本](http://192.168.2.178:18959/)可并排比较。新原型按用户任务组织领域／用例导航，分区显示对象字段、公开方法与规则；参数追踪可聚焦来源与消费方，场景轨道分别显示调用方所知和拥有者效果。[场景与设计映射](design/experience.md)说明每类任务需要哪些元模型事实、信息和动作。[原型源码与重建](prototypes/experience/README.md)及[离线页面](examples/domain-workbench-experience.html)可直接使用，另有[技术模型原型](examples/domain-workbench-experience-technical.html)。

原型复用现有真实模型、详情、播放器与相机，完成桌面／390px／键盘和关联任务的[独立走查](../../.controller/domain-daemon/tasks/flow-atlas-experience.md#published-prototype-and-walkthrough)。这是供用户体验和比较的设计方案；视觉偏好仍待实际反馈。当前无可调用的Design Canvas插件，采用独立HTML/SVG实现；现有语义和18959演示保持原样。

## 有限轨迹接口与使用

需要 Python 3.10+ 和现代浏览器，无第三方 Python 包、npm、Rust、服务器或网络依赖。在仓库根目录生成独立示例：

```bash
python3 tools/flow-atlas/build.py tools/flow-atlas/examples/request.json --output /tmp/catalog-request.html
```

也可从任何工作目录调用，输入和输出路径按调用者当前目录解释；页面资源始终按构建器自身位置查找：

```bash
python3 /home/long/works/longwork/agentx/tools/flow-atlas/build.py /home/long/works/longwork/agentx/tools/flow-atlas/examples/request.json --output /tmp/catalog-request.html
```

直接在浏览器打开 `/tmp/catalog-request.html`，即可比较关系、流程和时序视图，在同一轨迹上播放、暂停、逐步移动或查看对象、事实与来源。视图和速度属于展示状态，不改变检查点事实。生成的单个 HTML 包含模型、样式和脚本，可以离线分享。

相邻 `/tmp/catalog-request.receipt.json` 记录原始模型、工具输入和输出的 SHA-256、输出字节数与解释性范围。标准输出只有一行 JSON，包含输出、收据路径、摘要和字节数；诊断写入标准错误。成功退出 0，输入/构建失败退出 1，命令语法错误退出 2，中断退出 130。

模型、资源和三个模板槽位全部通过后才写文件，非法输入保留旧输出。目录按需创建，每个文件独立原子替换；HTML 与收据不是一个事务。若写入收据失败，命令会失败，修复目标目录权限后重试即可。构建器禁止把输出或收据指向输入模型或工具源码。

## 模型：一条有序轨迹

规范接口以 [model.py::validate](model.py) 和[对象端口契约](contracts.md)为准。对象、入站端口、通道和运行方式构成静态结构；场景从完整初态开始，逐个应用步骤补丁。每个场景都是一条作者已经选择的路径，不表达全部分支、并发调度、可执行工作流、自动发现架构或实时遥测。关系图、流程图和时序图是同一份数据的互补投影。

| 字段 | 含义与要求 |
| --- | --- |
| `schemaVersion`、`title` | 固定版本 1；非空标题。 |
| `factDefinitions[]` | `{id,label}`，事实维度由模型声明，可为空数组，不固定事实名称或数量。 |
| `defaults` | `{modeId,scenarioId}`，均存在且场景属于所选运行方式。 |
| `sources[]` | 可省略；条目 `{id,label,path,basis?}`。路径是展示/溯源文本，可是仓库路径、文档地址或其他来源；通用工具不读取、不检验文件存在。 |
| `nodes[]` | 非空；条目 `{id,label,kind,owner?,executable?,lifetime?,capabilities?,modules?,sourceIds?,stateFacts?}`。`kind` 为 `actor|service|process|process-group|store|logical`。`stateFacts` 是不重复的事实 ID，引出对象内的核心状态字段。 |
| `ports[]` | 可省略；对象拥有的入站操作端口，条目 `{id,nodeId,label,kind,input,outcomes,description?,sourceIds?}`。`kind` 为 `command|query|event`，所有端口 ID 在集合内唯一。事件端口表示接收另一个对象发出的事实。 |
| `modules[]` | 节点内的可选细分；条目 `{id,label,path?,description?}`，模块 ID 在整个模型中唯一。 |
| `edges[]` | 条目 `{id,from,to,label,transport?,purpose?,sourceIds?}`，端点必须存在；通道方向作为正向基准，不代表所有消息都同向。 |
| `modes[]` | 非空；条目 `{id,label,nodeIds,edgeIds,initial,command?,summary?,layout?}`。可见节点非空；可见通道的两端都必须可见。每种方式至少有一个场景。 |
| `scenarios[]` | 非空；条目 `{id,modeId,label,initial,steps,question?}`。每个初态都完整覆盖其方式的可见节点和全部事实维度。 |
| `steps[]` | 非空；条目 `{id,title,event,actor,handler,edgeIds,actions,beats,patch,interactions?,sourceIds?,certainty?,note?}`。动作列表和演示节拍非空；动作方、处理方和通道属于当前可见方式。 |

所有 ID 必须匹配 `[A-Za-z0-9][A-Za-z0-9_.:-]*`，并且不能是 `__proto__`、`constructor` 或 `prototype`。ID 在所属集合中唯一，步骤 ID 在场景内唯一；引用数组拒绝重复项。标签、说明、动作和事实文本不受这个 ID 字符限制。

状态为 `{nodes:{[nodeId]:{label,tone}},facts:{[factId]:string}}`。完整初态要求每个节点同时有非空 `label` 和 `tone`；色调为 `neutral|running|busy|waiting|stopped|unknown|stored`。事实值可为空字符串，工具保留其确切值。补丁可只写 `nodes` 或 `facts`，也可为 `{}`；节点补丁可只改标签或色调，但不能包含其他状态属性。补丁不允许未声明事实或不可见节点。

`node.stateFacts` 可省略并默认 `[]`。它把已有事实投影为对象内部的命名字段，例如 `"stateFacts": ["inbox"]`；取值始终读取同一检查点的 `facts.inbox`，不再保存一份值，也不声明原生存储归属。播放中保留旧值，应用后在字段内突出“旧值 → 新值”。不认识的事实 ID 和重复绑定会被拒绝。

节拍有两种形状：

```json
{"kind":"transfer","edgeId":"public-api","direction":"reverse","label":"返回响应"}
```

```json
{"kind":"observe","nodeId":"visitor","label":"观察结果"}
```

传输类是 `transfer|disconnect`，方向为 `forward|reverse`，相对通道的 `from → to`；必须使用当前步骤已声明的通道，不能再声明本地目标。原地类是 `local|observe|retain`，只能指向可见节点，不能携带通道或方向。步骤的 actor/handler 描述整件事，不限定下游节拍或反向响应。校验不会把这些解释性注解变成发送、收到、退出或继续运行的证据。断开、保留与未知有各自表达，最终事实始终由补丁决定。

端口 `input` 声明 `{name,type,required}` 数组：参数名按安全 ID 规则校验且在端口内唯一，类型是非空说明文本，`required` 是布尔值。`outcomes` 是非空 `{id,label,kind}` 数组，结果 ID 在端口内唯一，种类为 `accepted|returned|observed|rejected|unknown`。端口属于已有节点，但不等于操作系统端口，也不声称其标签是某个产品的原生方法名。

步骤中的 `interactions` 按声明顺序记录调用，目标由 `portId` 解析，不能另写 `to`。例如同一步内的第 0 个节拍调用、第 2 个节拍返回：

```json
{
  "id": "selection-accepted", "from": "visitor", "portId": "remember-selection",
  "requestBeat": 0, "responseBeat": 2,
  "input": {"recordId": "42"}, "result": {"outcomeId": "accepted"},
  "effects": [{"target": "node", "id": "catalog", "kind": "change"}]
}
```

调用 ID 在步骤内唯一。调用方及端口所属节点须可见；`requestBeat` 必须是从调用方实际传到端口所属节点的 `transfer`，也可为同一节点自调用的 `local`。断开与保留不能作为调用。可选 `responseBeat` 必须指向同一步内更晚、沿同一通道反向返回调用方的真实传输。缺失时不补造回复，未知结果禁止声明回复；跨步骤的节拍不能用这个索引关联。跨步查询可先声明已受理，再由后续入站事件记录观察到的数据，受理不等于查询完成。

调用 `input` 必须是对象，只能包含端口声明的参数名，必填参数不得缺失；可选参数允许省略。参数值是惰性的 JSON 数据，类型文本不触发求值、转换或类型校验。`result.outcomeId` 必须属于该端口，`summary` 可省略并默认使用结果标签。结果是应用该步骤后已知的作者声明；播放中的计划细节不能提前当成已返回事实。

可选 `effects` 中的 `{target,id,kind}` 引用本步骤补丁中真正变化的节点或事实；`target` 为 `node|fact`，`kind` 为 `change|observation`。节点只改色调也算真实变化。`change` 声明成功变更，`observation` 表示更新模型所知；拒绝和未知结果只能引用观察效果。它们不能证明远端完全没有副作用。补丁仍是唯一状态值来源，效果不再保存一份新值或执行另一套业务归约。

可选 `layout` 是展示提示：`{width,height,nodes:{[nodeId]:{x,y}},routes:{[edgeId]:{start,end,startRatio?,endRatio?,via:[[x,y]],label:[x,y]}}}`。尺寸须为正有限数，坐标须有限，端口为 `top|bottom|left|right`，比例在 `[0,1]`；只允许可见节点和通道。节点坐标和路由可以只提供一部分，`routes` 可省略。未提供的布局使用查看器的通用排列，布局不参与状态折叠。

`validate(model)` 返回独立的规范化 JSON 副本，不修改输入；错误为携带对象/字段位置的 `ValueError`。可选字符串默认 `""`，可选列表默认 `[]`，缺失的补丁 `nodes/facts` 默认 `{}`，布局缺失时仍保持缺失、已存在布局缺失的 `routes` 默认 `{}`。字符串默认覆盖根 `scope`、来源 `basis`、节点 `owner/executable/lifetime`、模块 `path/description`、端口 `description`、通道 `transport/purpose`、方式 `command/summary`、场景 `question`、步骤 `certainty/note`；列表默认覆盖根 `sources/ports`、节点 `capabilities/modules/sourceIds`、端口/通道/步骤的 `sourceIds`、步骤 `interactions` 和调用 `effects`。`result.summary` 默认使用结果标签，缺失的 `responseBeat` 保持缺失。旧模型不需要声明端口或调用。未知 JSON 元数据保留，已定义状态字段按上面的契约校验。

## 核心与嵌入接口

[core.js](core.js) 是普通 classic script 全局 `FlowAtlas`，不依赖 DOM、网络、时钟或应用标识。传入已校验模型：

```javascript
var selection = FlowAtlas.defaultSelection(model);
var initial = FlowAtlas.fold(model, selection.scenarioId, -1);
var afterFirst = FlowAtlas.fold(model, selection.scenarioId, 0);
var differences = FlowAtlas.changes(model, selection.scenarioId, 0);
var calls = FlowAtlas.calls(model, selection.scenarioId, 0);
```

`fold` 返回新的 `{nodes,facts}`；`-1` 表示初态，索引 i 包含第 i 步。返回值、模型及此前返回的快照彼此独立。`changes` 返回当前步骤真正改变的 `{id,kind:'node'|'fact',label,before,after}`，节点优先，随后事实，保留补丁的声明顺序；初态返回空数组，未变化的值不返回。`before/after` 是状态标签或事实原文；节点差异额外携带 `beforeTone/afterTone`，因此只改色调也有准确差异。未知场景或非整数/越界索引抛出 `RangeError`，不会默默改为其他检查点。默认选择来自模型；只在原始/空模型的 UI 恢复路径使用首个兼容对象或 null。

`calls` 只投影当前步骤的调用，初态或没有调用时返回 `[]`，索引错误与 `fold` 一致。每次返回独立数组和对象，包括解析出的节点、端口、嵌套参数及效果；调用方可修改这些返回值而不影响模型或另一次投影。返回形状为：

```text
{id,from:<node>,target:<node>,port:<port>,requestBeat,responseBeat?,input,
 result:{id,label,kind,summary},
 effects:[{target,id,kind,label,before,after,beforeTone?,afterTone?}]}
```

其中 `from/target/port` 为完整对象，节拍索引保留作者原值；未声明回复时省略 `responseBeat`。效果顺序与调用的声明一致，前后值来自原有 `changes` 投影，节点携带色调。结果和效果描述该步骤应用后的检查点，`calls` 本身不管理时钟或播放阶段，查看器负责在应用检查点前保持结果待定。

[build.py::render(model)](build.py) 返回完整 HTML 文本，不写文件。它先校验并规范化，再嵌入共享视口、纯核心和viewer脚本；确切顺序由 `build.py::VIEWER_INPUTS` 和 `_render` 拥有，收据记录每个输入摘要。模板只含三个唯一槽位 `__ATLAS_STYLES__`、`__ATLAS_MODEL__`、`__ATLAS_SCRIPT__`。模型中的 HTML 字符安全编码；槽位只替换一遍，模型文本中的相似字样不会作为模板执行。仓库消费者可以先做自己的来源检查，再调用它并保留自己的输出/收据格式。通过路径导入时无需更改 Python 模块搜索路径：

```python
import importlib.util
from pathlib import Path

tool = Path("/home/long/works/longwork/agentx/tools/flow-atlas/build.py")
spec = importlib.util.spec_from_file_location("flow_atlas_build", tool)
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)
html = builder.render(model)
```

## 为什么保留这个小工具

复用比较基于 2026-11-10 查阅的 [Mermaid 时序图文档](https://mermaid.js.org/syntax/sequenceDiagram.html)及其[源文档](https://github.com/mermaid-js/mermaid/blob/develop/packages/mermaid/src/docs/syntax/sequenceDiagram.md)，以及 [XState 状态机文档](https://stately.ai/docs/machines)。这里没有引入第三方依赖或宣称实现这些工具的能力。

| 方案 | 已提供的能力 | 与当前需求的关系 |
| --- | --- | --- |
| 现有 SVG 查看器 + Python/浏览器标准库 | 已有关系图、节拍播放、检查点与局部检查 | 保留实际页面，抽出有限模型、纯折叠和构建入口，直接支持所有视图共享状态。 |
| Mermaid 等图表库 | 用声明式语法表达图表；时序图还提供 loop、alt、parallel 等结构 | 适合静态说明，但模型事实、补丁、共享播放、来源检查仍需自行拥有。当前没有新增图表引擎的必要。 |
| XState 等状态机/工作流引擎 | 状态、事件、转换、动作、守卫和并行状态 | 适合需要运行和探索状态转移的系统；本需求只有一条作者选择的顺序轨迹，引入执行语义会扩大契约。 |

决定：复用已有渲染，自己拥有有限轨迹的语义，用标准库完成解析和打包。暂缓编辑器、执行引擎、插件注册、迁移框架、分支求解、并发调度及自动布局引擎；只有具体的新场景要求它们时才重新选择。

## 示例与证据边界

[request.json](examples/request.json) 是完全虚构的目录请求示例：3 个对象、2 条通道、5 个端口、3 个自定义事实，保留原 `lookup/read-record` 默认选择及前 4 步身份/补丁值，并在成功场景尾部增加一个记住选择的命令。查询、命令与事件都有明确参数和结果契约。三个场景分别展示查询后记住成功、条目不存在被拒绝、连接中断结果未知；成功和拒绝命令有真实反向回复节拍，未知场景没有回复或成功变更效果。示例省略来源、进程元数据和布局，仍使用通用排列，并保留只改色调的补丁。它描述作者声明的行为，不是查询执行、原生方法调用或存储持久化的实测证据。

对象端口扩展已完成[独立接收与实际验收](../../.controller/domain-daemon/tasks/runtime-atlas-ports.md#accepted-object-and-port-outcome)：模型反例、纯调用投影及隔离、离线构建和浏览器中的请求／返回／结果／补丁边界均通过。源码交接见[核心任务](../../.controller/domain-daemon/tasks/runtime-atlas-ports-core.md)。工具可用性与 AgentX 产品支持、原有来源证据是不同结论。

在[生成示例](examples/request.html)中，对象内部直接显示状态字段和命名 Port；边缘小点是方法入口，空心圆是调用方的发出点，连线落到实际目标 Port。查询入口为空心点，命令为实心点，事件入口为菱形，并有名称和类型文字。播放时参数框随请求移动，返回沿同一调用路径反向播放，未知结果不会补造返回。点击入口可看完整契约，点击当前调用可看完整参数和结果；`◇ 数量` 仍可打开方法列表。代码模块可在对象详情里查看，带方法的对象在画板上优先呈现对外接口。

选择“条目不存在”或“连接中断”场景，可比较同一个 Port 的拒绝和未知结果。关系图呈现对象、状态和接口连接，流程图呈现步骤与结果，时序图呈现调用和明确声明的反向回复；三个视图使用同一份数据与播放进度。实际画板验收由[同一结果任务](../../.controller/domain-daemon/tasks/runtime-atlas-ports.md#accepted-canvas-port-and-state-outcome)持有。

端口扩展之前，主控已完成[原版生成、模型/核心及浏览器验收](../../.controller/domain-daemon/tasks/runtime-atlas-multiview.md#accepted-pacing-views-and-reusable-outcome)，对应[生成示例](examples/request.html)。这项历史验收不覆盖新增端口能力；当前生成物与执行支持以当前主控任务为准。

## 可调整画板

两种查看器共用可自由平移、缩放的画布：SVG铺满容器，世界坐标不受内容边界限制。空白处拖动平移，滚轮以指针为中心缩放；鼠标中键或聚焦画布后按空格也可拖动。触屏支持空白处单指平移、双指缩放。左下角提供缩放、100%和适应全部；聚焦画布后方向键平移，`+/-`缩放，`0`恢复100%，`F`适应全部。

桌面首次适应内容；窄屏首次至少85%以保持文字可读，可平移查看其余部分，点“适应全部”看总览。切换回来和选择对象保留当前视角。相机仅在页面内存中按视图／模式／场景保存，与下述对象位置和布局JSON分开；缩放不会改写业务状态或布局文档。[共享视口契约](contracts.md#shared-spatial-viewport)和[实际结果](../../.controller/domain-daemon/tasks/flow-atlas-viewport.md)持有维护与验证入口。

关系视图可拖动对象标题；方向键移动10单位，Shift+方向键移动1单位，P固定／取消固定，Escape取消本次拖动。播放中实际开始移动会暂停并保留进度。布局菜单提供触控方向按钮、撤销、重置和JSON导入／导出；页面关闭前需导出，当前不自动持久保存。每次拖动只有一条撤销记录，Port和状态字段仍保留直接点击。

布局是按模型内容摘要及视图／运行方式／场景区分的独立文档，不改变initial／patch。移动后请求与返回使用更新的真实Port坐标。简单路线仍可能穿过其他对象，橙色诊断指出具体路线和对象；这不等于已经实现通用避障或自动排列。格式与精确API见[布局契约](contracts.md#independent-viewer-layout)。

## 从 TS 契约生成设计视图

```sh
node --experimental-strip-types tools/flow-atlas/authoring/examples/order-inventory.ts /tmp/order-design.json
python3 tools/flow-atlas/inspector/build.py /tmp/order-design.json --output /tmp/order-design.html
```

[订单／库存示例](authoring/examples/order-inventory.ts)直接导入[原生契约](authoring/examples/native-contracts.ts)，包含模块边界、方法归属、职责／设计文档、结果分支和实现绑定；[生成的离线HTML](examples/order-inventory.html)可直接打开。方法签名不重复维护，业务流程显式设计；实现通过同一操作／步骤句柄接入，检查实际记录的路径是否符合声明。完整的[设计与代码对应机制](design/source-binding.md)区分可检查的偏离与未覆盖源码。

本片通过既有TypeScript7.0.2、Node22.23.3和Zod4.6.7验证。authoring是private源码模块，未发布npm包；核心不依赖Zod，Zod适配器有明确可选peer。TS入口是受信构建代码，HTML仅消费导出JSON。完整运行证据与实际边界见[实施结果](../../.controller/domain-daemon/flow-atlas-implementation.md#accepted-local-slice)。
