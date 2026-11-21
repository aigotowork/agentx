---
schema: document
status: active
owner: flow-atlas
last_updated: 2026-11-10
description: Source-grounded technology decisions and remaining probes for the Flow Atlas target design.
links: [./README.md, ./api.ts]
---

# 技术选择与适用边界

这份记录区分候选选择、接入实现和实际验证。本轮只研究官方文档与npm元数据，未安装候选依赖、运行oRPC handler、改现有前端栈或服务。源页面和元数据留存在 `/tmp/flow-atlas-platform-design-kf3a92um/`，本地采集日期2026-11-10。

## TypeScript + 运行时 Schema

选择TS表达作者接口与模块契约；选择运行时Schema表达输入数据和portable定义，再推导TS导出。优先沿用项目已经使用的Zod体系，不新增一套Schema DSL。核心序列化产物是JSON Schema加引用；构建代码的函数、handler与Schema库对象不进入导出HTML。

[Standard Schema](https://standardschema.dev/)区分验证接口与Standard JSON Schema转换接口。二者不是等价能力：支持前者不保证能导出结构。我们自己的 `SchemaExporter` 报告不支持和不透明节点，禁止静默降级为全通过Schema。TS结构检查只解决作者形状，不替代运行时schema和跨引用编译。

在实施时以运行时Schema为唯一字段来源，`z.infer`等推导公开类型；本轮 `contracts.ts` 是设计期形状声明，切换到真实Schema后删除重复手写字段。JSON Schema目标版本和unsupported规则要固定在导出器测试中；不依赖版本随意猜测。

## oRPC：契约导入，不承担领域执行

[v1 contract-first文档](https://v1.orpc.dev/docs/contract-first/define-contract)明确输入、输出、错误与contract-only定义；[v1 metadata](https://v1.orpc.dev/docs/metadata)提供语义绑定位置。[npm @orpc/contract元数据](https://registry.npmjs.org/@orpc/contract/latest)本次读取的稳定版为1.16.2，许可证MIT。没有为本设计安装或固定项目依赖。

需要特别区分官网代际：[orpc.dev contract文档](https://orpc.dev/docs/contract-first/define-contract)与[Standard Schema集成](https://orpc.dev/docs/integrations/standard-schema)本次访问标明v2 beta；不能把v2 metadata插件或Standard JSON Schema fallback行为套到v1。v1版同一路径的Standard Schema页面返回404，记录为文档差异，不能声称v1支持该页面描述的全部自动转换。适配器首个实现固定并验证一个版本的真实公开导出，再通过 `ProcedureContractImporter` 隔离差异。

oRPC适合复用服务方法的输入／输出／错误；领域角色、状态机、process和场景证据仍要作者补充。稳定版本的Schema转换由显式 converter承担，不依赖某个版本的默认空Schema fallback。工具内部进程内调用不走RPC；未来有真实远程协作需求时再定义服务契约和冲突协议。

## React Flow：空间交互适配器

选择作为下一版交互画板的优先机制候选：[custom nodes](https://reactflow.dev/learn/customization/custom-nodes)、[handles](https://reactflow.dev/learn/customization/handles)提供节点定制和多个带唯一ID的入口，边通过 `sourceHandle/targetHandle` 连接具体入口。它能承接拖动、选择、平移缩放，核心业务数据仍由我们的模型拥有。

[npm @xyflow/react元数据](https://registry.npmjs.org/@xyflow/react/latest)本次为12.12.1、MIT、peer支持React/DOM>=17；项目已有React19，版本范围表面匹配。范围匹配不等于已验证兼容；实施时要做实际DOM/keyboard/touch检查。无需购买Pro才能定义自有节点与handles，但官方部分完整避障示例属于Pro；本设计不把付费示例当作已获得能力。

[官方布局说明](https://reactflow.dev/learn/layouting/layouting)区分自动布局与边路由。React Flow不是完整语义建模器，也不承诺默认边会绕开所有对象。不能换了节点库就宣称解决挡线；语义核心、路由provider和最终几何检查由本工具承担。

现有SVG查看器可先交付有界拖动切片；不是为了通用化立即重写所有视图。下一版画板通过 `Viewer`／`DiagramLayout`消费同一数据，sequence/state可继续用直接SVG。生成独立HTML时把所需资源打包进去，不引入CDN、必须联网的在线编辑器或业务后端。

## ELK：布局与路由优先探针

[ELK Layered](https://eclipse.dev/elk/reference/algorithms/org-eclipse-elk-layered.html)提供层次布局选项；[Port Constraints](https://eclipse.dev/elk/reference/options/org-eclipse-elk-portConstraints.html)包含 `FIXED_SIDE/FIXED_ORDER/FIXED_RATIO/FIXED_POS`；[Edge Routing](https://eclipse.dev/elk/reference/options/org-eclipse-elk-edgeRouting.html)区分POLYLINE、ORTHOGONAL和SPLINES。[npm elkjs元数据](https://registry.npmjs.org/elkjs/latest)本次为0.12.1，许可证表达式 `EPL-2.0 OR GPL-3.0-or-later`，不同于React Flow的MIT；真正分发需保留所选许可及声明。

选择ELK作为节点自动排列和固定Port正交布线的首个候选，而不是自己写一个通用图算法。**固定Port位置不等于任意固定节点布局的可靠纯路由。** `DiagramLayout.arrange` 与 `route` 因此分开；实施前的最小探针必须证明：已有坐标、障碍节点、四个同侧Port、双向调用、自调用、嵌套组，以及一个固定节点位置冲突。结果应检查路径线段与非端点节点的相交，不只看最终截图。

若固定节点情况下ELK能力不足，不偷偷移动用户pin或重复改算法参数。保留明确的 `unrouted` 诊断并选专门的routing provider，或先交付临时连接+用户显式重新排列；与全局自动布局分开报告能力。provider可替换不要求用户改领域模型。拖动中不每帧运行大型布局，释放后在worker中做有generation fencing的更新。

## Process／state：自有有限表达，复用外部模型按需

[XState官方说明](https://stately.ai/docs/state-machines-and-statecharts)包含层级、并行region等可运行statechart能力。我们当前表达的是可理解、可校验的定义与给定轨迹，不需要自动执行业务守卫、invoke服务或调度并行工作。因此不引入XState作为业务执行器，也不重新声称实现其全部能力。

自己的有限process图只拥有start/operation/event/decision/fork/join/end、明确条件和场景visit。自己的状态定义只有字段及可选合法transition。后续消费者已有XState/BPMN模型时，可以做有覆盖限制的导入；当前不建设一套兼容所有工作流方言的框架。

## 选择结果

| 责任 | 本轮决定 | 实施前必须解决的证据 |
| --- | --- | --- |
| 数据与作者接口 | TS + runtime Schema，类型从Schema推导 | 编译／引用／Schema反例和非JSON值规则。 |
| SDK契约复用 | 可选oRPC适配，固定首个真实版本 | contract-only导入，schema转换和错误映射，不调用handler。 |
| 画板 | React Flow优先，现有SVG可先做小切片 | 真实拖动／触控／键盘、稳定handles、离线产物。 |
| 空间计算 | ELK优先探针，route与arrange分开 | 固定位置、避障、取消、过时结果拒绝；未解决线路明确呈现。 |
| 行为意义 | 自有typed定义与显式trace | 分支与因果、state与调用结果不混淆，不执行表达式。 |

没有为比较选项而增加一层服务、插件市场、远程存储或微服务。只有上述消费者确实存在的模块边界才进入第一版实现。
