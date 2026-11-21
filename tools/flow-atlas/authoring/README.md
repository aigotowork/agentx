---
schema: document
status: active
owner: flow-atlas
last_updated: 2026-11-10
description: Bounded native TypeScript contract authoring, portable inspection and explicit process conformance.
links: [./AGENTS.md, ./index.ts, ./contracts.ts, ../design/source-binding.md, ../../../.controller/domain-daemon/tasks/flow-atlas-ts-authoring.md]
---

# 原生契约与流程设计

在普通 TS 代码中导入已有 Schema 和业务方法，补上模块归属、职责、文档与允许的流程，再导出纯 JSON 给离线 inspector。业务输入／输出来自原 Schema；方法、流程和实现绑定共用相同的 operation／step 句柄。

当前源实现是一个有界 authoring 库，独立验收归[任务与案例矩阵](../../../.controller/domain-daemon/tasks/flow-atlas-ts-authoring.md)。这里没有完整 `AtlasDefinition`／`Scenario` 编译器、调度器、任意源码推断、oRPC importer、文件监听或实时遥测。更大的设计与已实现边界见[对应关系设计](../design/source-binding.md)。本目录不修改既有有限轨迹查看器的数据格式。

## 从原生契约到检查工件

| 事实 | 唯一来源 | 本库的作用 |
| --- | --- | --- |
| 参数和返回字段 | [native-contracts.ts](examples/native-contracts.ts) 中真实 Zod 对象 | `schema(native, adapter, identity)` 保留引用，adapter 导出 JSON Schema。 |
| 方法行为 | 原生 `prepareOrder`／`reserveInventory` | `defineOperation` 保存真实方法句柄；inspect 不执行它们。 |
| 模块与公开对象 | `defineModule`／`defineObject` | 精确引用归属，声明允许的直接依赖。 |
| 允许的步骤和结果 | `step`／`flow`／`defineProcess` | 显式流程图；业务含义不能从函数签名猜出。 |
| 实现实际经过的边界 | `bindStep`／`observeProcess` | 显式调用时记录实际 operation、结果与顺序。 |
| 文档与来源 | 共享 `Identity.metadata` | 惰性 summary、responsibility、tags、sourceLinks、documents、命名空间 extensions。 |

[order-inventory.ts](examples/order-inventory.ts) 给出完整组装：[订单准备] → [库存预留] → [已预留／拒绝／未知／失败]。无响应也有独立结果与边，不会自动补成成功。Schema 字段只在 native-contracts 中定义，authoring 示例只导入和引用。

```ts
import { schema, defineOperation, bindStep, step } from './index.ts';
import { zodAdapter } from './adapters/zod.ts';
import { ReserveRequest, ReserveResult, reserveInventory } from './examples/native-contracts.ts';

// owner is a declared object handle; the native method remains unchanged.
const reserve = defineOperation({
  id: 'inventory.reserve', label: 'reserveInventory', owner, kind: 'command',
  input: schema(ReserveRequest, zodAdapter, { id: 'request', label: 'Request' }),
  output: schema(ReserveResult, zodAdapter, { id: 'result', label: 'Result' }),
  method: reserveInventory, classify: result => result.kind,
  outcomes: [
    { id: 'reserved', label: 'Reserved', meaning: 'returned' },
    { id: 'rejected', label: 'Rejected', meaning: 'rejected' },
    { id: 'unknown', label: 'Unknown', meaning: 'unknown' },
  ],
});
const reserveStep = step({ id: 'reserve-stock', label: 'Reserve stock', kind: 'operation', caller: owner, operation: reserve });
const implementation = bindStep(reserveStep, reserve);
```

示例片段展示契约复用；完整可调用组装包含全部结果处理与流程边。方法如依赖 `this`，先在原生契约层建立一次稳定的 bound function，再把该函数同时用于实际代码和 authoring；不要在每次声明时重复 `.bind()` 来冒充同一方法身份。

## 纯检查与导出

| API | 返回 / 行为 |
| --- | --- |
| `inspect(model)` | 始终给出 `InspectorArtifact`，保留草稿诊断；不表示检查通过。 |
| `compile(model)` | 返回共享 `Checked<InspectorArtifact>`；任何 error 都没有 `value`。 |
| `lint(model)` | 与 inspect 相同的诊断，包含 severity、稳定 code、精确 path 和 message。 |

`flow-atlas/inspector-1` 的精确字段由[InspectorArtifact](contracts.ts)拥有：domains、modules、objects、schemas、operations、processes、diagnostics 和 coverage。流程只存 step／operation／flow 的稳定 ID；没有业务函数、Schema 实例或可执行 guard。它是设计投影，不是一条已经发生的轨迹，也不能当作 v1 viewer 输入。离线查看器由[Inspector](../inspector/README.md)消费同一工件，区分声明图与实际轨迹。

导出会调用显式 Schema adapter，但不会调用业务方法、结果 classifier 或 Schema 的 parse。返回数据是独立的 JSON 副本，不保留消费者元数据的可变引用。函数、循环对象、NaN／Infinity、BigInt、Date、访问器、自定义原型、稀疏数组等拒绝成为元数据。扩展键使用 `owner:name` 形式。文档仅允许相对路径或 HTTP(S) 引用，不读取文件、不抓网页、不保证目标存在；查看器仍须用安全 DOM 和链接策略显示。

模块依赖只表示作者明确允许的直接调用；不从仓库 import 或 `query` 标签推断依赖／纯度。跨模块调用必须出现在 caller 模块的 dependencies 中。模块依赖环默认 warning，`model.policy.dependencyCycles` 可选 `warning`、`error` 或 `allow`；模块／领域 parent 环仍是结构错误。重复原生方法归属、错误句柄、未绑定步骤、缺少结果边、不可达或无终点路径都会报告 error。

具体已实现规则、诊断码、CI调用方式、维护文件与覆盖边界见[lint规则与维护](lint.md)。它检查显式设计声明和已记录路径，当前不是扫描任意TS项目的ESLint插件。

本目录是仓库内的 private TS 源模块，不是已发布的 npm 包。Zod adapter 有单独的可选精确 peer `zod: 4.6.7`，不依赖根工程某个未声明的传递依赖；generic core 没有 Zod import。当前工作区已有该版本，无需安装或改根 manifest／lock。

Zod adapter 固定导出 Draft 2020-12，并显式拒绝 transform／pipe、custom refinement、Date、BigInt 及不可转换节点。为避免默认值／catch／lazy 在转换时执行作者回调，本片也拒绝 default、prefault、catch、lazy 和对象 shape 的作者访问器。普通对象、联合、字面量、枚举、数组、可选／nullable、内置长度／数值约束等走真实 `z.toJSONSchema`。预检查针对安装的 4.6.7 结构，升级须重新验收；JSON Schema 不宣称证明任意业务行为。

执行下面的代码需要受信任、无启动副作用的 contract-only 入口；运行 TS 入口本身不是沙箱。以下命令供控制者验收，此分工未执行：

```sh
node_modules/.bin/tsc -p tools/flow-atlas/authoring/tsconfig.json
node --experimental-strip-types tools/flow-atlas/authoring/examples/order-inventory.ts
node --experimental-strip-types tools/flow-atlas/authoring/examples/order-inventory.ts /tmp/order-inspector.json
```

Node 22.23.3 的 strip-types 与系统 Node 24 是本轮约定环境。省略路径时 stdout 只有 JSON；给定路径时使用 exclusive create，已有文件不会覆盖。输入／转换诊断写 stderr 并返回非零退出码；本示例不创建目录、不启动服务。

## 显式运行与轨迹检查

```ts
import { observeProcess } from './index.ts';
import { createOrderExample } from './examples/order-inventory.ts';

const example = createOrderExample();
const observed = observeProcess(example.process, example.model.bindings);
const request = await observed.invoke(example.prepareBinding, { orderId: '42', sku: 'book', quantity: 1 });
await observed.invoke(example.reserveBinding, request);
const trace = observed.trace();
const report = observed.check({ complete: true });
```

消费者依然亲自选择与 await 每一次调用。wrapper 校验输入／返回 Schema，调用一次原生方法，然后运行 classifier 并记录结果；抛错先记录 failed 再抛回消费者，没有自动重试。未声明的失败映射保留为 unresolved，不会变成成功。`record(binding, 'unknown' | 'no-response', reason)` 仅记录明确观察边界，不执行方法，也不宣称远端没有副作用。挂起调用的 `check` 会返回 `TRACE_PENDING`；不把尚未结算的 Promise 当成已返回。每个 session 只支持串行调用，重叠 invoke／record 会拒绝。

`checkTrace(process, trace, {complete})` 是纯函数。它检查顺序、operation、结果映射和准确的流程边；允许有效前缀，但要求 complete 时必须进入终点。结果的 `complete: true` 表示轨迹有效并已结束，拒绝／未知／失败终点也可以是一条完整轨迹；**不表示业务成功**，需读取 terminalStepId 或工件中的终点语义。有错误的轨迹始终 complete=false。本库不保存输入／输出 payload，避免把业务数据隐式复制到检查工件。

同一个 step 可重复访问形成循环，每次有单调 sequence；没有 fork／join 并行调度、事务／事件／状态编译。未使用 wrapper 的代码、没有走过的分支与隐藏副作用不在证明范围。修改普通函数内部行为或绕过 wrapper 的调用不会自动暴露；项目需要把结构检查与关键路径观察放入自己的 CI。[cases.ts](examples/cases.ts) 提供 authored case 输入；[任务矩阵](../../../.controller/domain-daemon/tasks/flow-atlas-ts-authoring.md)区分这些输入与控制者实际执行的证据。

独立执行已完成，真实结果和已修复的Zod导出元数据边界见[实施Delivery](../../../.controller/domain-daemon/flow-atlas-implementation.md#accepted-local-slice)。上文的分工／示例限制描述其作者阶段，不代表仍在等待执行。
