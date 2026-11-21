---
schema: document
status: active
owner: flow-atlas
description: Implemented design lint rules, invocation, policies, ownership and coverage limits.
links: [./README.md, ./compiler.ts, ./conformance.ts, ../../../tests/flow_atlas_authoring.mts]
---

# 设计 lint：检查什么、如何维护

已经实现的检查器在本目录内，公共入口为 [index.ts](index.ts)。`lint(model)` 返回诊断；`inspect(model)` 返回带诊断的可视化草稿；`compile(model)` 遇到任意 error 时返回 `ok:false` 且没有有效工件。三者使用同一个 [compiler.ts](compiler.ts)，没有为 UI 再维护一套检查逻辑。

诊断统一包含 `{severity,code,path,message}`。`path` 指向传入设计中的具体字段；UI 的“检查”面板消费相同工件中的 diagnostics。下面列出的是当前实现，不代表将来目标中的所有检查都已实现。

## 结构与依赖规则

| 范围 | 当前规则代码 | 触发与处理 |
| --- | --- | --- |
| 身份 | `INVALID_ID`、`INVALID_LABEL`、`DUPLICATE_ID`、`DUPLICATE_REF` | 标识／名称非法，集合标识或依赖重复。默认 error。 |
| 引用与归属 | `BROKEN_REF`、`FOREIGN_OPERATION`、`DUPLICATE_OPERATION_OWNERSHIP` | 引用目标不存在，未使用原生操作句柄，或同一方法被多个操作重复占有。当前每个原生方法需要一个规范操作身份。 |
| 模块边界 | `FORBIDDEN_MODULE_CALL` | 流程中的 caller 调用了未在其模块 dependencies 中声明的目标模块。声明了间接依赖不能自动放行直接调用。 |
| 环 | `DEPENDENCY_CYCLE` | 模块依赖环默认 warning，可配置；domain/module 的 parent 环始终 error。检测依赖环不表示流程中的业务循环非法。 |
| Schema | `DUPLICATE_SCHEMA_OWNERSHIP`、`SCHEMA_ID_MISMATCH`、`SCHEMA_CONVERSION_FAILED`、`NONPORTABLE_SCHEMA`、`UNSUPPORTED_SCHEMA` | 同名Schema源不一致、adapter更改标识、转换失败或不可安全导出。Zod adapter有明确的4.6.7版本及支持范围。 |
| 方法结果 | `MISSING_OUTCOMES`、`INVALID_OUTCOME_BINDING` | 没有声明结果，或失败／未知／无响应映射指向错误含义。 |
| 流程结构 | `INVALID_START`、`INVALID_START_FLOW`、`UNREACHABLE_STEP`、`DEAD_END_STEP`、`TERMINAL_HAS_FLOW` | 入口不唯一／被重新进入、步骤不可达、无任何到终点的路径、终点还有出边。不会证明业务循环一定终止。 |
| 分支 | `UNDECLARED_OUTCOME`、`UNEXPECTED_OUTCOME`、`UNHANDLED_OUTCOME`、`AMBIGUOUS_FLOW` | 未声明结果、非操作节点携带结果、缺少结果边，或同一结果选择多条边。 |
| 实现绑定 | `UNBOUND_STEP`、`WRONG_OPERATION_BINDING`、`DUPLICATE_BINDING`、`ORPHAN_BINDING` | 步骤缺失实现绑定、调用错误操作、重复绑定或引用已移除步骤。绑定使用相同句柄，而非另抄方法名字。 |
| 元信息 | `NONPORTABLE_METADATA`、`EXTENSION_NAMESPACE`、`INVALID_DOCUMENT_REF`、`UNSAFE_DOCUMENT_URI` | 非JSON／带访问器元数据、扩展缺少命名空间、文档标识重复、危险链接。这里只检查引用格式，不检查文件或网址是否存在。 |

模块依赖通过 `defineModule({..., dependencies:['inventory']})` 显式声明。`model.policy.dependencyCycles` 接受 `warning`（默认）、`error`、`allow`。不会根据文件import自行推断边，也不会自动判断架构风格是否允许循环。

## 轨迹一致性是另一种检查

[conformance.ts](conformance.ts) 的 `checkTrace(process, trace, {complete:true})` 检查已记录的串行调用，而不是静态扫描源码。`observeProcess` 只在消费者显式 invoke 时调用真实方法；静态lint、导出和查看不会执行它。

| 规则 | 含义 |
| --- | --- |
| `TRACE_INVALID_PROCESS`、`TRACE_SEQUENCE`、`TRACE_ORDER`、`TRACE_OPERATION` | 流程入口不可解析、序号不连续、顺序不符或方法不符。 |
| `TRACE_OUTCOME`、`TRACE_STATUS`、`TRACE_REASON`、`TRACE_BRANCH` | 结果未解析、未知被冒充成功、未知没有原因或结果不能唯一选择下一条边。 |
| `TRACE_PENDING`、`TRACE_INCOMPLETE`、`TRACE_TERMINAL_MEANING` | 真实调用仍挂起、要求完整却未到终点，或失败／未知／拒绝直接冒充成功终点。 |

`complete:true` 的报告表示这条观察路径有效且到达终点，拒绝或未知也可以是有效终点；业务是否成功要看终点语义。未使用 wrapper 的代码、隐藏副作用、未覆盖分支与并行调用不在证明范围内。

## 项目中使用

让消费者的可信 contract-only TS 入口组装模型并运行：

```ts
import { compile } from './tools/flow-atlas/authoring/index.ts';
import { createOrderExample } from './tools/flow-atlas/authoring/examples/order-inventory.ts';

const checked = compile(createOrderExample().model);
for (const item of checked.diagnostics) {
  console.error(`${item.severity} ${item.code} ${item.path.join('.')}: ${item.message}`);
}
if (!checked.ok) process.exitCode = 1;
```

它是仓库内的 TS 库，当前没有 ESLint插件、自动全项目扫描、oRPC importer 或通用lint CLI。项目CI可以调用自己的上述入口；是否把warning升级为失败由项目决定。运行TS入口属于受信代码执行，不能把应用启动模块当作无副作用契约入口。

## 维护路径

| 修改 | 修改与检查入口 |
| --- | --- |
| 增加结构／依赖规则 | [compiler.ts](compiler.ts)；保留稳定code和精确path；在[案例输入](examples/cases.ts)添加有意义的正常／失败模型。 |
| 修改状态／结果／顺序检查 | [conformance.ts](conformance.ts)；增加正常路径和偏离路径，不改变业务handler调度。 |
| 修改Zod支持范围 | [adapters/zod.ts](adapters/zod.ts)；依实际固定版本验证，保留不执行作者callback的边界。 |
| 验证公共结构 | `node_modules/.bin/tsc -p tools/flow-atlas/authoring/tsconfig.json`，覆盖正例和编译期反例。 |
| 验证运行语义 | `node --experimental-strip-types tests/flow_atlas_authoring.mts`，覆盖13个设计案例、13个轨迹案例、18次Schema转换、额外同名字段边界和5种真实调用结果。 |
| 验证展示 | [Inspector使用](../inspector/README.md)，消费同一份diagnostics，不重复实现规则。 |

这些案例已通过[独立验收](../../../.controller/domain-daemon/flow-atlas-implementation.md#accepted-local-slice)，但并非表中每个诊断码都有独立反例。新增规则时按实际风险补覆盖，不用一条通过记录推断所有设计语义都已证明。源码是规则权威；本表是维护索引，修改规则时同步更新相关行。
