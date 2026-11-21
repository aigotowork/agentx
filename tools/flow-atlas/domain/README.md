# 领域模型语义核心

本模块把原生 Schema、对象责任、公开操作、流程参数和场景记录编译成同一份 `flow-atlas/workbench-1` JSON。对象图、数据流、状态和时序视图消费相同的检查点。原生调用和默认设计导出分开：查看设计不会下单、预留库存或运行规则谓词。

本片已通过 Linux 本地静态工作台的独立验收并发布。当前支持、准确检查范围、最终工件与清理由[最终验收结果](../../../.controller/domain-daemon/tasks/flow-atlas-domain-acceptance.md#accepted-final-derivatives-and-publication)和[最终交接记录](/tmp/flow-atlas-domain-acceptance-h9uepct1/final-handoff.json)拥有；源码决策与修复历史见[核心 Task](../../../.controller/domain-daemon/tasks/flow-atlas-domain-core.md)。产品目标见[工作台设计](../design/workbench.md)，界面由[workbench](../workbench/AGENTS.md)拥有。验收覆盖下文的有限实现，不扩大为任意源码、业务逻辑或 Schema 的完整证明。

可直接打开[已发布的离线工作台](../examples/domain-workbench.html)及[技术领域示例](../examples/domain-workbench-technical.html)，或访问[局域网演示](http://192.168.2.178:18959/)。对应 JSON／receipt、相同字节验证和演示进程身份统一见最终验收记录；页面只展示导出的设计，不连接业务运行时。

## 模块与使用

| 入口 | 职责 |
| --- | --- |
| [index.ts](index.ts) | 公共类型与 `bindNativeMethod`、`inspect`、`compile`、`checkpoint`、`snapshot`、`changes`。 |
| [contracts.ts](contracts.ts) | 已实现的领域、流程、场景和 JSON 工件契约。 |
| [compiler.ts](compiler.ts) / [schema.ts](schema.ts) | 原生 Schema 安全转换、引用、所有权、顺序数据流和有限 Schema 检查。 |
| [binding.ts](binding.ts) | 把真实原生方法与输入／输出 Schema 强类型绑定；函数仅存在 WeakMap，不进入 JSON。 |
| [checkpoints.ts](checkpoints.ts) | 唯一场景归约；实例状态、派生值、调用知识、参数来源、差异和断言。 |
| [examples/native.ts](examples/native.ts) | 真实内存 Order／StockPosition／Checkout；必须显式调用。 |
| [examples/order-stock.ts](examples/order-stock.ts) | 六种纯作者场景及显式原生记录适配。 |
| [examples/technical.ts](examples/technical.ts) | 不需要实体、聚合或产品 ID 的技术领域复用。 |
| [lint.md](lint.md) | 规则、诊断和覆盖限制。 |

```ts
import { compile, inspect, snapshot } from './index.ts';
import { createOrderModel } from './examples/order-stock.ts';

const model = createOrderModel();
const draft = inspect(model); // 保留诊断；不调用原生 parse、handler 或业务谓词。
const accepted = compile(model);
if (accepted.ok) {
  const scenario = accepted.value.scenarios[0]!;
  const last = snapshot(accepted.value, scenario.id, scenario.checkpoints.length - 1);
  console.log(last['order-42-view']);
}
```

`inspect` 返回可定位的草稿，`compile` 只在没有 error 时返回 value。warning 和 unsupported coverage 始终保留，不能当作完整证明。Schema adapter 是显式可信转换边界；默认例子复用[已有安全 Zod adapter](../authoring/adapters/zod.ts)，不调用 parse。工件经过无损 JSON 复制和递归冻结；公共快照／差异／检查点访问返回可独立修改的副本。

可信 TS 示例导出命令是 `node --experimental-strip-types tools/flow-atlas/domain/examples/export.ts order-stock /tmp/domain-model.json`；第二个例子选择 `technical`。Node 必须支持 TypeScript stripping；也可以由既有 TS runner 导入 `exportArtifact()`。输出是新 `.json`，不覆盖既有文件，不写源码目录，`-` 输出到 stdout。命令本身不运行原生场景，默认没有服务、网络连接或业务 handler 调用。本次已验收工件由独立验收者执行导出和验证，准确输入与结果见最终验收记录。

## 一次调用的事实与所知

调用先产生 `call` 检查点。拥有者的明确状态变化产生 `effect`，实际收到回复产生 `reply`；本地未知、无响应或失败产生 `outcome`。最后的 `terminal` 比较独立预期。`ScenarioVisit.completion` 明确声明 `reply | local-outcome`，不能从 payload 缺失猜测传输历史。

提交后丢失回复时，`CallProjection.effectOutcomeId` 记录库存权威提交，`phase: unknown` 记录调用方所知，`completion: local-outcome` 保证没有虚构的返回。库存仍有预留，Order 保持 awaiting。真正返回“结果仍未知”的 envelope 则保留 `completion: reply` 和 envelope；它同样不撤销远端事实。

每个检查点包含完整 `states`、`calls`、当前 visit 的 `parameters`、该点刚应用的 `differences`、局部 `assertions` 与诊断。`derived[instanceId][fieldId]` 是编译器的只读派生投影；目前只有显式 `subtract-sum`（数值总量减集合数值之和）。`available` 不写入库存原生状态，也没有自己的 patch。没有投影的派生说明保持声明状态。UI 不再计算业务公式或归约 writes。

## 身份、归属与来源

对象类型 ID、场景实例 ID、业务身份、visit ID、call ID 分别拥有语义。`businessIdentity` 可以是按照定义字段路径投影的对象；单字段身份也可直接保存该标量。同一场景中两个实体实例不能重复同一业务身份。复合身份使用精确对象投影。`DomainField` ID 在对象内、`InputField` 在各 input/outcome 内、`ParameterBinding` 在步骤内唯一，消费者必须保留父对象／方法／步骤上下文。

参数只能来自 Process 输入、公开业务身份、已经收到的指定步骤／结果分支或显式常量。规则和效果由公开方法的拥有者负责；流程不能直接写另一实例。可选一致性根按实例引用绑定，不要求技术模型虚构聚合。效果只是可能写入的范围；真正值来自明确的场景见证，不从方法名或结果名称推断。

`Evidence.kind` 保留 authored／observed。方法的 `source-bound` 需要实际的 `bindNativeMethod` 句柄，注册原生函数或 prototype 方法、同一输入 SchemaHandle 和原生完整输出 Schema；类型检查会发现方法被移除或签名与 Schema 不符。编译器检查注册身份、拥有者及输入引用，JSON 只保留 `nativeMethodId` 与 `nativeOutputSchemaId`。方法没有被调用，结果分支投影、源码内部逻辑和业务符合性仍是显式声明。没有真实句柄的方法只保留 declared，源码链接不会升级为方法绑定。规则的源码对应与场景检查分开；比较规则仅执行不可调用的 `eq/ne/gte/lte` 声明，断言带 authored／observed 依据。没有有限检查的文字或源码规则保持 declared，不显示为 passed。

```ts
import { bindNativeMethod } from './index.ts';
// input/output 是原生 SchemaHandle；Order 的原生方法无需构造实例即可建立类型对应。
const method = bindNativeMethod({
  id: 'native.order.confirm', label: 'Order.confirm', ownerId: 'object.order',
  input: receiptSchemaHandle, output: confirmResultSchemaHandle,
  method: Order.prototype.confirm,
});
// 在 DomainOperation 中设置 native: method，并复用 method.input。
```

## 原生场景与反例

```ts
import { runCheckout } from './examples/native.ts';
import { createOrderModel, scenarioFromNativeRun } from './examples/order-stock.ts';
import { compile } from './index.ts';

const run = runCheckout('lost-response'); // 显式创建内存实例，真实提交，再丢弃回复。
const model = createOrderModel();
const observed = scenarioFromNativeRun(run); // 显式转换实际记录，预期来自独立作者案例。
const checked = compile({ ...model, scenarios: [observed] });
```

`runCheckout` 支持 `success`、`refusal`、`lost-response`、`wrong-receipt`、`stale-receipt`、`quantity-mismatch`。后三者显式注入传输凭据错误；库存拥有者提交的原始凭据不变，Order 自己拒绝不匹配的确认。这些是合法的拒绝场景；[cases.ts](examples/cases.ts) 另提供把错误凭据／数量／未知强行当成功的无效输入，供消费者检查诊断。

原生 `Order.beginReservation` 从自身行构造请求，`confirm` 校验订单／SKU／数量／意图及 awaiting 状态，`reject` 只消费匹配拒绝。`StockPosition.reserve` 维护 onHand 与 reservation records，重复意图明确拒绝，不承诺自动重试或恢复查询。所有状态和返回值复制隔离；没有全局业务实例、服务或用户数据写入。

## Canonical contract map

| 较早目标／已有能力 | 本片权威关系 |
| --- | --- |
| `design/contracts.ts` 的 Identity／Id／Json／Source／Evidence／Diagnostic／ValueSchema／OutcomeMeaning | 原样 import/re-export，单一原始拥有者。 |
| `authoring/contracts.ts` 的 SchemaHandle，`handles.ts::schema` 与安全 adapter | 原样复用原生身份和转换；按 native handle + io 去重，不手抄原生字段。领域原生方法绑定沿用现有 WeakMap 隔离模式，增加原生完整输出与领域 owner 的强类型对应；不创建另一套 Schema 或 runtime。 |
| 较早 Domain／Module／ObjectType／Port | 本片领域工作台使用 `Context`／`DomainModule`／`DomainObject`／`DomainOperation` 的显式细化，`contracts.ts` 单独拥有已实现语义；旧目标不被隐式当作可编译输入。 |
| 较早 ProcessDefinition／Scenario／CompiledAtlas | 本片顺序无环子集由 `DomainProcess`／`DomainScenario`／`Checkpoint` 取代；旧目标的 event/fork/join/loop 不冒充已支持。 |
| 较早 Layout／路由／SDK／oRPC | 不属于语义工件；既有 spatial/layout 独立消费，未实现目标不从声明获得支持。 |

当前不支持任意 TS 推断、图形回写任意代码、事件总线、循环／并行数据流、任意业务谓词、通用公式语言、通用规则证明、通用工作流调度或多观察者知识引擎。复杂 JSON Schema 导出可保留，但静态兼容与运行值检查只覆盖明确子集，诊断和 coverage 指出未证明部分。
