---
schema: document
status: active
owner: flow-atlas
description: Portable source-first domain workbench UI and safe offline embedding.
links: [./AGENTS.md, ../domain/contracts.ts, ../design/workbench.md, ../../../.controller/domain-daemon/tasks/flow-atlas-domain-ui.md]
---

# Flow Atlas 领域工作台

## 联动工作区源码

[打开已发布的联动工作区](http://192.168.2.178:18961/) · [离线订单页面](../examples/domain-workbench-linked.html) · [技术模型页面](../examples/domain-workbench-linked-technical.html)。本轮已完成独立构建、桌面／390px交互与实际图像走查；精确范围见[父任务最终结果](../../../.controller/domain-daemon/tasks/flow-atlas-linked-workspace.md#accepted-linked-workspace-and-publication)。源码 Task 中的作者用例与实际执行证据保持区分。

本轮维护源码默认提供“协作流程｜场景时序”双窗：左窗按对象责任泳道显示操作、完整条件分支与所选场景路径，右窗只展开该场景的实际 visits。未选场景时，右窗提示选择场景，不把互斥声明排成一次执行。单窗保留五种原视图；辅助窗可换对象状态。两窗共用用例、场景、已应用检查点、精确选择与唯一播放器，各自保留相机。桌面并排，1000px及以下在同一区域切窗；390px不纵向叠放第二幅图。检查器局部覆盖，关闭后回到原引用。

点步骤或时序消息，两边保持图种并定位同一访问；重复步骤通过编号访问标记或详情中的访问列表选择，`visitId/callId` 保持独立。参数详情只读取已应用前缀内该访问的权威参数记录，拥有者状态明确对应当前检查点。未来访问的输入标为“场景记录 · 未应用”，不补造未来效果或回复。分支图例区分声明、场景路径与已应用前缀；未知结果保持未知，只有 `completion:reply` 才画返回。场景比较仍是各自的终点快照，不是跨场景同步播放。

当前源码责任与固定 hooks 见[源码 Task](../../../.controller/domain-daemon/tasks/flow-atlas-linked-ui.md#structural-contract-2026-11-11-source-author)。`state().linked` 提供双窗图种、可见性、语义焦点、检查点与独立相机的分离副本；隐藏且尚未在当前上下文挂载的窗，其相机为 `null`，打开后恢复对应 scope。请求/回复/效果/本地结果用 `data-message-kind` 标记，精确 occurrence 还带 `data-step-id/data-visit-id/data-call-id`。分支和访问标记的 `data-scenario-path` 与 `data-applied` 分别表示完整场景路径与当前前缀。

示意请求/回复期间，移动运输点保留在图中；窗头的“⇢ 参数/⇠ 数据”披露当前示意载荷，完整文本在有界浮层中按需查看，避免覆盖操作和分支。`linked-payload-cue` 带精确 call/visit/phase，原 `flow-payload` hook 现在位于浮层的只读 `<pre>`，不再是随点移动的SVG文字；Escape关闭该披露。切回刚显示的窗格会在独立相机原缩放中定位当前访问。泳道头部的长职责和状态按字形预算省略并裁剪在各自泳道内，完整内容仍可通过可访问名称/title/检查器读取。

独立构建、浏览器图像、实际支持与新预览由[本轮父任务](../../../.controller/domain-daemon/tasks/flow-atlas-linked-workspace.md#accepted-linked-workspace-and-publication)拥有；源作者没有执行这些检查。全部10组独立交互路径已有通过结果，呈现修正按受影响路径复验，旧尝试和截图均保留。下方18959及原验收证据属于此前固定版本，不会随源码自动更新。构建方式仍为本页原命令，输出请选择新路径。

## 原有发布与通用能力

工作台从一份 `flow-atlas/workbench-1` JSON 读取对象、原生 Schema、公开方法、规则、用例、参数来源和场景检查点。TS 定义是作者源；浏览器提供查看、追溯、比较、示意播放与布局操作。它不会调用业务方法。

当前 Linux 本地/静态浏览器范围已[独立验收](../../../.controller/domain-daemon/tasks/flow-atlas-domain-acceptance.md#accepted-final-derivatives-and-publication)。打开[局域网演示](http://192.168.2.178:18959/)或[离线订单示例](../examples/domain-workbench.html)即可使用；另有[技术领域示例](../examples/domain-workbench-technical.html)。实际执行结果、截图检查和精确输入/输出摘要由验收记录及[最终交接](/tmp/flow-atlas-domain-acceptance-h9uepct1/final-handoff.json)拥有，下面的维护用例表仍是作者用例，不代替执行证据。

```text
领域 / 模块 → 对象 → 字段 / 方法 / 规则 → Schema / 来源
                         ↓
用例 → 参数来源 → 场景 → 调用方所知 + 拥有者状态 → 预期检查
```

用[领域编译器](../domain/README.md)导出 JSON 后构建单文件页面：

```sh
python3 tools/flow-atlas/workbench/build.py /tmp/domain-model.json --output /tmp/domain-workbench.html
```

直接打开生成 HTML 即可离线使用；不需要网络、框架安装或服务端 API。构建器的 `render(model)` 只嵌入 JSON 与工具资源。相邻 `.receipt.json` 保留模型原始字节、所有嵌入脚本/样式/模板/构建器的 SHA-256 和输出摘要。模板槽位仅替换一次，模型转义 HTML 控制字符；元数据通过 `textContent` 展示，只有显式 http(s) URL 可点击。输入或受保护源码不能用作输出目标；非法输入在替换前失败。HTML 与收据逐个原子替换，不是跨文件事务。

此前发布版本默认显示对象设计；本轮源码的默认双窗见上文。无场景时没有播放栏。选择场景后，五种视图共享其语义 ID、当前检查点与选择上下文：

| 视图 | 用途 |
| --- | --- |
| 对象与关系 | 业务身份、职责、所属模块、拥有字段、命名方法 Port、规则及当前差异。 |
| 流程 | 公开调用步骤、合法结果分支和终点。 |
| 数据流 | 明确连接用例输入、身份、常量或指定结果字段到目标参数；点击看当前调用的解析值与实传值。 |
| 状态 | 生命期合法转换；已应用检查点中的实际变化另有“已访问”标记。 |
| 时序 | 同一组实例与调用；独立显示拥有者已提交效果、调用方结果和无返回的本地结果。 |

上一/下一步选择准确检查点，播放则先演示再应用。默认速度0.5×，可选0.25/0.5/1/2。请求与实际回复显示数据标记；效果只在拥有者处演示处理；本地未知与终点不生成返回。播放期间字段保持前一检查点，只有应用后才出现新的状态差异。暂停、详情和视图切换保留播放位置；更换场景/用例和重新播放取消旧段。关闭页面不保存播放或布局。

画布复用[共享相机](../spatial/AGENTS.md)：空白拖动、滚轮或双指缩放，方向键平移，`+/-` 缩放，`F` 适应全部，`0` 原始比例。对象视图可拖动标题；聚焦标题后方向键移动10世界单位，Shift+方向键移动1单位，`P` 固定，Escape取消当前拖动。布局菜单还有触控方向、撤销、重置、JSON导出/导入。布局只保存对象位置与固定状态，严格绑定模型内容摘要、用例、场景；相机独立保存在内存。路线从真实方法 Port 重新取点；简单路线仍可能被移后的对象遮挡，布局提示列出对应路线和对象。没有通用避障路由承诺。

首次进入每个视图/用例/场景时，如果整图适应会把缩放降到85%以下，工作台通过共享相机的既有100%操作恢复可读比例并定位当前对象或步骤。显式“适应全部”仍显示整图；之后返回该上下文保留用户相机，不自动放大。方法详情同时展示实际导出的 `nativeMethodId` 和原生完整输出 Schema；没有真实方法句柄的定义保持已声明，不补造源码绑定。

资源搜索覆盖对象、方法、字段、规则、语义值、用例和场景。字段、参数与步骤引用保留拥有者范围，允许不同对象/步骤复用局部 ID。详情可关闭，Escape依次关闭布局、详情、资源；窄屏用局部抽屉保留同一画布，参数均为只读。检查范围原样展示 Core 的声明、源码绑定、声明案例检查、观察记录检查与不支持状态；文字规则不自动变成已测试证明。

已完成的源码交接与独立执行结果分别由 [UI Task](../../../.controller/domain-daemon/tasks/flow-atlas-domain-ui.md#accepted-ui-outcome-and-closure) 和[验收 Task](../../../.controller/domain-daemon/tasks/flow-atlas-domain-acceptance.md#accepted-final-derivatives-and-publication)拥有。验收确认桌面场景/数据流100%可读、390px操作，以及播放/布局/来源/安全路径；实际范围与保留限制以原始证据为准。没有任意 TS 推断、图形语义回写、原生方法执行或实时运行状态。

## Maintenance and authored UI cases

Read [AGENTS.md](AGENTS.md) for module ownership. `app.js` composes safe DOM, views, details, one presentation clock and layout; it contains no domain reducer. The canonical contract is [domain/contracts.ts](../domain/contracts.ts). `FlowAtlasWorkbench.state()` returns a detached read-only inspection snapshot with selected semantic reference, applied checkpoint, presentation progress, camera and layout. It has no mutation API.

Stable controls are declared in [the template](index.template.html). SVG refs use `data-ref-kind`, `data-ref-id` and optional `data-ref-scope`; local fields use object ID, outcomes/operation parameters use operation ID, steps use process ID, bindings use `processId/stepId`, instances/visits/expectations use scenario ID. `data-node-id`, `data-port-id`, `data-step-id`, `data-binding-id`, `data-target-path`, `data-source-step-id`, `data-source-outcome-id` and `data-source-path` retain exact projection identities. Applied fields carry `data-value`, and differences add `data-before`/`data-after`. Player state is on `scenario-player` data attributes; the canvas carries view/scenario/applied checkpoint. Motion hooks are `flow-marker`, `flow-payload` and `processing-marker`.

The independent acceptance owner should exercise these authored consumer cases alongside its own scenario cases:

| Starting condition / action | Expected observable result |
| --- | --- |
| Open exported example in design mode; search method/field; inspect rule/source and navigate its step/binding. | No player initially; original IDs/scopes retained; current process/scenario only changes for an explicit cross-context reference; source/Schema are inert. |
| Select success, play at0.5, pause during a request, switch object→dataflow→sequence, resume. | Elapsed fraction/applied index stable across pause and view changes; old values persist until application; marker follows actual path and return only when completion is `reply`. |
| Step through committed/lost scenario effect→outcome→terminal. | Reservation/derived availability remain committed; order stays awaiting and caller unknown; no fabricated reply or confirmation. |
| Compare final expectations, then select a diagnostic on a scoped binding/field. | Recorded facts remain unchanged; assertion basis/status preserved; exact parent scope resolves even when local IDs repeat. |
| Zoom to nonunit scale, drag object header, cancel/commit/undo/pin; export, alter, then import. | World displacement matches screen transform; endpoints remain actual method ports; one undo per gesture; pinned nodes stay still; invalid digest/scope input is atomic. |
| At390px open/search resources, inspect method/parameter, close with Escape, use player/layout touch controls. | Body does not scroll/overflow; graph remains container-sized; drawers preserve subject; focus returns to the originating semantic ref or canvas. |
| Build hostile metadata containing closing tags, slot-looking text and javascript/file/data URLs. | Text remains inert, only http(s) anchors activate, template has one model/style/script embedding; no native code executes. |

These are authored cases, not execution claims. Source author execution is intentionally delegated to the independent acceptance owner for this Delivery.
