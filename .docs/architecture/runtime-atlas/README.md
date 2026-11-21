---
schema: document
status: active
owner: agentx
last_updated: 2026-11-10
description: Source-grounded model and standalone interactive process, communication and lifecycle atlas.
links: [../../../.controller/domain-daemon/runtime-atlas.md, ../runtime-hosts.md, ../../codex-activation/acceptance.md]
---

# AgentX 运行全景图

看清进程边界、进程内模块、通信通道，以及事件发生时谁观察、谁处理、改变了哪些事实。图中状态是行为演示，不是实时监控，也不操作任何 Agent。

先选择运行方式和行为场景，再点下一步执行一个演示事件，或点播放连续演示。默认0.5×，可选0.25×、0.5×、1×、2×；关系、流程、时序三种视图共享场景、步骤、当前动作和暂停位置。沿通信线移动的标记表示当前传输，红色断点表示连接关闭，原地标记表示本地处理、观察或保留。事件处理完成后才应用状态变化，操作条显示前后值；可以暂停在过程中，或重播当前事件。点击进程、模块、通信线、状态项或“详情”，可就地查看完整内容并用 Escape/关闭返回。默认从“共享 App Server → 关闭前台连接”开始；也可以对比前台、Headless 和 tmux 的关闭/恢复路径。

## Model contract

`model.json` is canonical declarative data. The shared [Flow Atlas tool](../../../tools/flow-atlas/README.md) owns validation, pure fold/diff helpers and the single presentation source in `tools/flow-atlas/viewer/`. Local `build.py` retains AgentX source-path checks and calls that tool to generate portable `index.html`; there is no duplicate renderer. No CDN or server is needed to open the generated file locally.

| Collection | Meaning |
| --- | --- |
| `factDefinitions[]`, `defaults` | Data-owned fact IDs/labels and initial mode/scenario; no AgentX identifiers in the generic viewer. |
| `sources[]` | `{id,label,path,basis}`; repository-relative path, source or scoped verified basis. References do not imply new executed evidence. |
| `nodes[]` | `{id,label,kind,owner,executable,lifetime,capabilities[],modules[],sourceIds[],stateFacts?}`. Module `{id,label,path,description}`. Kind separates process, process group, store and logical object; optional stateFacts references existing canonical facts. |
| `edges[]` | `{id,from,to,label,transport,purpose,sourceIds[]}`; connection does not imply parent-child process ownership. |
| `ports[]` | Object-owned inbound `command|query|event`, input fields, allowed outcomes and source references; independent of the transport channel. |
| `modes[]` | `{id,label,command,summary,nodeIds[],edgeIds[],initial,layout?}`; optional topology positions/routes are view hints, never lifecycle facts. |
| `scenarios[]` | `{id,modeId,label,question,initial,steps[]}`. Step `{id,title,actor,handler,edgeIds[],actions[],event,patch,beats[],sourceIds[],certainty,note}`. |
| `step.interactions[]` | Caller, target Port, invocation/reply beat indices, inert input, declared outcome and references to actual patch effects; optional for legacy steps. |

State: `{nodes:{[id]:{label,tone}},facts:{connection,host,provider,turn,inbox}}`. Fact values are concise Chinese strings; tones `neutral|running|busy|waiting|stopped|unknown|stored` supplement labels. Reconstruct step index by folding scenario initial and prior patches; index=-1 precedes the first action. Every node/edge reference belongs to the mode. Runtime model defaults select appserver/disconnect; hash keys are `mode,scenario,step,node,view,speed`. Invalid presentation values fall back to relationship view and0.5×; switching modes resets incompatible playback/selection. Source paths remain inspectable/copyable. No control executes a product command.

`modules` means code executing inside that process, not files that merely describe or integrate with it. External Codex/client/tmux internals are not invented from AgentX adapters; stores and logical threads contain no executable modules. The terminal/Shell node explicitly groups separate environment processes as `process-group`. AgentX managed-service acquisition belongs to the bridge; SQLite access belongs to the daemon. Broader source basis remains attached through `sourceIds`.

`beats[]` owns the illustrative execution order within a step. A transport beat is `{kind:"transfer"|"disconnect",edgeId,direction:"forward"|"reverse",label}` and uses only an edge already declared by that step. A local beat is `{kind:"local"|"observe"|"retain",nodeId,label}`. These beats project the existing step/source meaning; they do not add runtime events, prove an actual payload, or specify measured timing. A returned native event can traverse an existing connection in reverse; a disconnected edge must show a break, and an abrupt local crash cannot invent an outgoing report. The step's final `patch` stays authoritative. UI phases keep the prior snapshot until application, then display actual before/after differences.

## 同一模型的三个观察角度

| 视图 | 读什么 | 交互 |
| --- | --- | --- |
| 关系 | 进程、内部模块、通道和当前状态 | 选择对象或连接看局部详情。 |
| 流程 | 当前场景从开始到结束的步骤链、动作方、处理方及变化数 | 点击步骤演示该事件；不凭空补充分支。 |
| 时序 | 参与方生命线，以及每个动作的传输方向、本地处理、断开或保留 | 当前动作随播放突出显示，长场景可平移、缩放，当前动作按需进入视野。 |

速度只影响演示时间。调速和切换视图不会提前应用状态；暂停位置保持。场景是人工定义的一条有序轨迹，不能据此推断真实耗时、并发关系或全部可能分支。通用契约、接入命令和另一套系统的示例在 [工具说明](../../../tools/flow-atlas/README.md)；AgentX 的源码依据与生命周期判断仍由本模型及下列原生资料负责。

## Source authority and limits

Use [runtime hosts](../runtime-hosts.md), [daemon lifecycle](../daemon.md), [Codex topology](../../codex-activation/topology.md), production leaves named by each entity, and [scoped shared-runtime evidence](../../codex-activation/acceptance.md#current-support-by-acceptance-boundary). Shared Codex hosting is verified only for documented Linux/Codex0.167.0. Ordinary PTY host exit does not prove termination of independent native CLI-internal backends. Backend survival does not prove continued AgentX delivery or uninterrupted in-flight work after all clients leave.

The earlier [visible execution outcome](../../../.controller/domain-daemon/tasks/runtime-atlas-model.md#accepted-visible-execution-outcome) is complete. Its actual normal-motion browser evidence covers moving packets, pause/resume, connection break, reverse receipt, delayed state application, compact layout and local disclosure. The previous [acceptance](../../../.controller/domain-daemon/tasks/runtime-atlas-model.md#accepted-artifact-and-handoff) remains historical. Neither extends the native product's verified platform scope.

## 打开全景图

局域网页面：http://192.168.2.178:18957/ 。也可直接打开 [index.html](index.html)，无需联网或启动 AgentX。共5种运行方式、14个行为场景，支持关系/流程/时序视图；对象、模块、通道及每步动作均来自独立 [model.json](model.json)。主图使用完整工作区宽度，底部保留紧凑操作条和五项状态；画布填满容器，空白处拖动可在任意方向平移，滚轮或触屏双指缩放，左下角可恢复100%／适应全部；窄屏首次保持可读比例，详情在局部抽屉中打开。地址中的模式、场景、步骤和节点可复制分享；重载恢复完成的检查点，不恢复瞬时动画进度。

实际浏览器验证覆盖切换、回放、断线状态、键盘、390px 触控、路径复制及离线打开。重新构建会在本地记录产物和输入的摘要。页面演示已有行为，不读取实时数据或控制 Agent。

## Build and inspect

```bash
python3 .docs/architecture/runtime-atlas/build.py
python3 .docs/architecture/runtime-atlas/serve.py --host 0.0.0.0 --port 18957
```

Open `index.html` directly for offline use, or the printed listener with this machine's numeric LAN address. The server serves only the generated HTML; it does not expose repository files, source directories or live AgentX data. Editing model or UI source requires regeneration; existing product `.bin` and services are unrelated. `build-receipt.json` records exact source input/artifact digests.

The controller's scoped checks include `python3 tests/flow_atlas_model.py`, `node tests/flow_atlas_core.mjs`, `tests/runtime_atlas_multiview_browser.mjs`, `python3 tests/runtime_atlas_model.py`, `tests/runtime_atlas_motion_browser.mjs` and the relevant cases in `tests/runtime_atlas_browser.mjs`. Browser scripts consume the existing Playwright/Chrome installation and explicit listener/evidence directory; they never start AgentX. The motion script samples real frames, pause, reverse receipts, disconnect, delayed fact application, cancellation and compact layout. The compatibility script waits for applied checkpoints; its optional `remaining` phase exercises narrow/copy/offline/keyboard/continuous playback without repeating the unchanged14-scenario walkthrough. Source-only/authored cases remain distinct from actual runs in the [model/outcome Task](../../../.controller/domain-daemon/tasks/runtime-atlas-model.md#visible-execution-follow-up).

## Pacing and reuse acceptance

The [accepted pacing/multiview outcome](../../../.controller/domain-daemon/tasks/runtime-atlas-multiview.md#accepted-pacing-views-and-reusable-outcome) owns exact source/artifact identity, speed-frame samples, view continuity,14 scenario projections, narrow flow and actual offline reuse. Tool source/research belongs to [core Task](../../../.controller/domain-daemon/tasks/runtime-atlas-tool.md). Prior frame evidence remains valid for its recorded artifact. The new [catalog example](../../../tools/flow-atlas/examples/request.html) is a generated standalone HTML using the same viewer; browser checks and all owned resources are closed except the intentionally retained atlas listener.

## 对象、Port 与调用

已完成[对象端口验收](../../../.controller/domain-daemon/tasks/runtime-atlas-ports.md#accepted-object-and-port-outcome)。对象拥有 Port，Port 定义输入与允许结果；场景步骤通过真实方向的节拍调用它，效果引用已有状态补丁。内部计算保持黑盒，端口也不等于网络端口。完整数据契约见 [contracts.md](../../../tools/flow-atlas/contracts.md)。

建议打开[消息投递时序](http://192.168.2.178:18957/#mode=appserver&scenario=appserver-mail&step=-1&view=sequence&speed=0.5)，逐步观察“消息持久接受 → 唤醒 → 准备批次 → 原生输入结果待确认 → 匹配记录 → 交付确认”。返回途中保持待确认，完成该步后才显示结果和状态变化；消息已保存、Provider 交付和业务完成是不同事实。关系图中点击 `◇ 数量` 可查看对象的方法及参数；播放时点击当前调用可检查具体输入与效果。

AgentX 模型新增7个 Port 和7次调用注解，覆盖消息与释放前台根会话的关键边界；原49个步骤的身份、初态及补丁保持不变。断线继续作为断线展示，不能冒充停止后端的方法。通用示例另外覆盖成功、拒绝和未知。实际验收包含桌面与390px触控、键盘契约选择、正常动画、暂停切视图／调速、离线示例与无提前成功结果；精确输出仍由当前 build-receipt.json 和上述 Task 持有。

## 画板中的对象接口与状态

画板已直接显示每个方法的命名 Port 小点，以及 daemon 的收件箱、原生 thread 的轮次等核心状态字段。调用线从调用方的空心发出点连接到具体方法入口，参数按字段随请求移动；显式返回沿同一条线反向播放。普通通信通道保留为浅色背景，断线仍属于该通道。处理完成时，对象内直接显示相关状态的“旧值 → 新值”。

可从[消息调用关系图](http://192.168.2.178:18957/#mode=appserver&scenario=appserver-mail&step=-1&view=relations&speed=0.5)开始，观察 `Messaging.Submit`、`Inbox.Prepare`、原生输入、匹配事件和 `Inbox.Confirm` 各自入口。点对象看对象详情，点 Port 看方法契约，点状态字段看前后变化。完整参数、结果和来源继续按需打开。带方法的对象优先在画板展示对外接口，代码模块保留在对象详情。

这些字段只引用已有事实，未新增原生能力或改变初态／补丁。[画板结果记录](../../../.controller/domain-daemon/tasks/runtime-atlas-ports.md#accepted-canvas-port-and-state-outcome)持有实际落点、动态参数、延迟状态变化、14个场景布局、离线复用和触控验证。

## 调整对象位置

关系画板现支持标题拖动、键盘移动、固定／取消固定、撤销／重置，以及布局JSON导入导出。布局菜单也有触控方向按钮。拖动开始时保留播放进度并暂停，连线跟随真实Port；遮挡或重叠显示局部诊断。当前不会自动避障，位置只保存在页面内存，需要导出才能留存。业务模型与49个原步骤保持不变。精确[当前实施结果](../../../.controller/domain-daemon/flow-atlas-implementation.md#accepted-local-slice)取代旧的固定布局能力说明；旧截图和证据仍属于其原产物。

与运行行为演示互补的[TS契约设计Inspector](../../../tools/flow-atlas/inspector/README.md)直接消费原生Schema／方法引用的设计投影，展示模块、方法与可能流程；它不表示这些流程已经在AgentX实际运行。

## 共享画布视口

运行图与契约设计Inspector现在共用[空间视口](../../../tools/flow-atlas/spatial/AGENTS.md)，取代原来的有边界SVG滚动页。画布容器决定屏幕尺寸，相机决定观察位置；切换回来保留视角。标题拖动仍是移动对象，空白拖动是移动视角，两者不会混淆。相机不写入模型或已有布局JSON。具体手势、键盘操作和限制见[工具使用](../../../tools/flow-atlas/README.md#可调整画板)，精确产物及实际浏览器证据见[本次结果](../../../.controller/domain-daemon/tasks/flow-atlas-viewport.md)。此前静态截图和滚动行为属于其原产物。
