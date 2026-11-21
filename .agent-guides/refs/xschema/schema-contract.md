<!-- Generated from the declared xschema sources; see sources.json. Do not edit this copy. -->

# 字段合同：Agent 记录与目录

以下章节来自 xschema 的规范说明；查询执行章节由工具自己的使用文档维护。

## Common

- `schema` 必填，标识类型；实例由文件或目录的项目根相对路径定位，不增加实例 ID。
- 数据用 YAML。Markdown 的 YAML front matter 保存字段，正文就是 `content`，front matter 不再写 `content`。独立 YAML 可写短 `content`。
- 目录载体只选一个：短内容 `.xmeta.yaml`，长内容 `.xmeta.md`。载体所在目录是主体。
- 结构化引用优先使用 `./` 或 `../` 开头的文档相对路径，以承载元数据的文件所在目录为基准；`.xmeta.yaml` / `.xmeta.md` 因而以所描述的目录为基准。无此前缀的已有路径仍相对工作区根，不按文件存在与否猜测基准。此规则适用于 links、artifacts.path/guidance、scope、任务/管理关系、依赖两端和 Meta 引用。
- 引用用 `/`；主体关系不带锚点，材料引用可带 `#heading-anchor`。允许开头的 `./` 或连续 `../`，不允许中途出现 `.` / `..` 段；`./`、`../` 可单独引用目录。拒绝绝对路径、URL、反斜杠、空白路径、重复分隔符及具名目录的尾部 `/`。结构化路径不做 URI 解码；归一化后不得越出工作区，越界属于需要来源位置的语义检查，不能只靠形状校验判断。
- Controller 的 `scope: .` 仍表示项目根，`scope: ./` 表示载体所在目录。查询请求、返回的资源身份和证据地址仍用规范的根相对路径；路径字段与 evidence.raw 保留源文件写法；schema 输入别名的规范投影与原值读取见 [Module 兼容边界](#module)。新旧写法归一化到同一目标后仍受重复关系与图约束检查。
- 省略表示未声明。未知字段保留；未知 schema 只校验类型 ID 并标记不支持，不套用已有类型的业务解释。

| 字段 | 类型 / 必填 | 含义 |
| --- | --- | --- |
| `description` | 非空字符串；通常可选 | 精要描述，统一此名，不使用 desc 别名；Roadmap、能力项、Proposal、Delivery 与阶段必填 |
| `content` | 可选字符串 | 详细说明；Markdown 从正文提供，YAML 可直接填写；同一内容只维护一份 |
| `status` | 开放字符串；Task / Delivery / Roadmap / Proposal 必填 | 各主体自己的状态；Task / Delivery 约定 planned / active / blocked / completed / cancelled，规划与提案词汇见各节；未知值不推断行为；task-meta 不使用它 |
| `tags` | 字符串数组，元素非空且不重复；通常可选，Scenario 必填且至少一项 | 用于分类、筛选与聚合；尽量复用一致的标签词汇 |
| `links` | 可选去重材料引用数组 | 相关资料，不产生管理或依赖关系 |


## Roadmap

`schema: roadmap` 维护能力方向与规划选择；必需 `description`、`status` 和非空 `items`。整体 status 的常用词为 draft / active / paused / retired；状态开放，不代表能力已经交付。

`items` 是按局部 schemaId 键组织的映射，不是独立文件或全局实例 ID。查询地址为 `{path: "roadmap.md", part: {field: ["items", "revisit"]}}`；键不是 Markdown 标题，不创建 `#items.revisit` 伪锚点。YAML 与 Markdown 载体使用同一规则。

| 能力项字段 | 必填 / 形状 | 语义 |
| --- | --- | --- |
| `description` | 必填非空字符串 | 要获得的能力与用户结果；价值、进入条件和取舍在正文展开 |
| `status` | 可选非空开放字符串 | 规划选择，常用 candidate / selected / deferred / dropped；缺省表示未声明 |
| `horizon` | 可选非空字符串 | 如 now / later 的规划提示，不是时间调度表达式 |
| `proposals` | 可选去重 Proposal 文件路径数组 | 本项对应的设计变化；不能带标题片段；尚无提案可省略或为空 |

一份 Proposal 可以被多个能力项引用；不复制 Proposal 或维护反向 roadmap 字段。selected 表示已选方向，不推断实现状态。能力项和 Roadmap 的未知扩展保留，工具不把扩展变成额外关系。


## Proposal

`schema: proposal` 记录一次可决定的设计变化；必需 `description` 与 `status`。常用状态为 draft / proposed / accepted / rejected / withdrawn / superseded，保持开放。accepted 表示明确范围内的设计已采用，不意味着执行授权或全部实现；原有授权继续有效。

| 字段 | 必填 / 形状 | 语义 |
| --- | --- | --- |
| `models` | 可选去重材料引用数组 | 受影响的原模型或细化设计，可引用真实 Markdown 标题片段；不强制目标拥有新的 model schema |
| `deliveries` | 可选去重 Delivery 文件路径数组 | 本方案聚合的交付，不能带标题片段；尚未选择交付可省略或为空 |
| `links` | 沿用 Common | 其他依据与背景材料，不自动解释为受影响模型 |

正文维护当前到目标、保留项、内部决定和未决问题、分批边界及细化去处。首批没有标准 `changes` 登记对象。尚不存在的模型目标在正文说明，不能把缺失文件视作已可读取的模型。实现后把成立的规则归入所属模型，Proposal 保留原决定依据与未交付目标，不重复维护现行规则。

一份 Delivery 可由多个 Proposal 采用，其正文说明覆盖的部分；这与 Controller 的直接管理不同，不新增单 Proposal 父项约束。所有已列 Delivery 完成仍不足以证明 Proposal 全部目标已交付；工具不自动传播状态或计算能力完成。

默认关系由同一 project provider 注册，正向关系只在拥有它的字段维护，反向查询从这些事实派生：

| 关系名 | 来源 → 目标 | 来源字段 |
| --- | --- | --- |
| `roadmap.item` | Roadmap → 本文档能力项字段地址 | `items.<key>` |
| `item.proposes` | 能力项字段地址 → Proposal | `items.<key>.proposals[]` |
| `proposal.models` | Proposal → 模型/设计材料及其真实标题 | `models[]` |
| `proposal.delivers` | Proposal → Delivery | `deliveries[]` |

沿用 Common 相对路径、原始来源/指纹、目标类型和扫描覆盖合同；归一化后的重复关系报冲突而不静默去重。缺失标题与未扫描目标分别报告，反查为空只说明当前扫描范围。原 `delivery.stage`、`stage.includes`、Task 分解及 Controller 管理保持各自含义。普通 find/read/select/SQL/relations 即可消费，不新增规划查询引擎或专属命令。

默认实体的顶层保留字段中，`items` 仅归 Roadmap，`models` 仅归 Proposal，`deliveries` 仅归 Controller / Proposal；`scope`、`controllers` 仍仅归 Controller。其余未知扩展及未知 schema 的原有行为保留；局部字段 `horizon` / `proposals` 不成为全类型通用保留词。关闭默认 definitions 后，新类型和关系也不隐式启用。


## Task

`schema: task` 表示可执行、可验收的工作。目标、范围与证据写在正文。

| 字段 | 必填 / 形状 | 语义 |
| --- | --- | --- |
| `status` | 必填；值见 Common | 当前工作状态 |
| `blocking_reason` | 仅 blocked 时必填的非空字符串 | 等待什么及继续条件；离开 blocked 时删除，不增加等待实体或计时模型 |
| `agent` | 可选字符串 | 当前已知执行地址，不是运行状态 |
| `tasks` | 可选 Task 路径数组 | 直接分解的子任务；每个子任务最多一个分解父项，关系无环 |
| `dependencies` | 可选 `{before, after, condition?}` 数组 | 内部下游任务间的依赖；两端为递归可达 Task，最近共同分解父 Task 拥有该边，condition 为人读文本 |

不维护 parent / delivery / controller / depends_on 反向字段。并行执行者分成不同 Task；回复对象与权限由分派上下文给出。


## Delivery

`schema: delivery` 是一份整体规划；必填 `description`、`status` 和非空 `stages`。不再维护顶层 plan / tasks / dependencies。

`stages` 是以局部键组织的映射，如 initialize、capture。键只在当前 Delivery 内定位阶段，不是实体 ID。每个阶段同时说明目的、任务与依赖；较长的输入、产出、边界和验收写在同一个 Delivery Markdown 的对应章节。

| 阶段字段 | 必填 / 形状 | 语义 |
| --- | --- | --- |
| `description` | 必填非空字符串 | 做什么及预期产出，不能只有模糊标题 |
| `content` | 可选字符串 | 细节；与正文对应章节二选一维护 |
| `status` | 可选开放字符串 | 阶段业务状态，已约定值见 Common |
| `tasks` | 可选 Task 路径数组 | 阶段的聚合入口；后代从 Task.tasks 展开 |
| `depends_on` | 可选 `{stage, condition?}` 数组 | 同一 Delivery 的其他阶段；condition 可说明开始条件或最终验收条件，不是自动调度表达式 |

阶段依赖必须存在、无重复、无自依赖且无环；只在所属计划内解释。一个 Task 在同一 Delivery 中只归属一个阶段，包括后代展开后的归属；跨 Delivery 可复用。展开任务集由读取器计算，不写第二份清单。整体完成由成果验收，不从阶段或任务计数推导。纯规划阶段可以没有 Task。


## Controller

`schema: controller` 表达管理职责。

| 字段 | 必填 / 形状 | 语义 |
| --- | --- | --- |
| `scope` | 必填目录范围路径 | 管理边界；项目根为 `.`，局部范围用 `./` 或 `../`，不授予写权限 |
| `agent` | 可选字符串 | 当前主控地址；交接不自动改变下游状态或执行者 |
| `deliveries` | 可选 Delivery 路径数组 | 直接管理的交付 |
| `tasks` | 可选 Task 路径数组 | 直接管理的任务，与阶段聚合和任务分解是不同关系 |
| `controllers` | 可选 Controller 路径数组 | 子主控；最多一个直接上级，管理树无环 |

每个 Task / Delivery 最多一个直接 Controller。没有声明的管理关系保持未声明；不从目录、分解父任务执行者或运行父级猜测消息收件人。


## Task Meta

`schema: task-meta` 存在 Task 旁的 `.<task-stem>.task-meta.md`（例如 `a.task.md` 对应 `.a.task-meta.md`）；必填 `task`（目标 Task 路径）和 `state`（开放字符串，已约定 draft / reviewed / promoted / retired）。不拥有业务 status、实际 agent 或依赖。

四个可选建议数组：`scenarios`＝工作情境，`roles`＝职责边界，`skills`＝可复用方法，`tools`＝调用入口。它们不要求一一对应。

| 建议字段 | 必填 / 值 | 语义 |
| --- | --- | --- |
| `name` | 必填非空字符串 | 人可读名称，属于建议值，不是新实体身份 |
| `description` | 必填非空字符串 | 适用问题、能力或职责边界 |
| `source` | 必填开放字符串 | map＝已有材料；candidate＝候选建设项；inferred＝上下文推断 |
| `ref` | map 时必填材料引用 | 已有 guide / skill 路径，可带锚点；不以路径替代名称 |
| `reason` | candidate 时必填；其他可选 | 当前 Task 的具体触发依据 |
| `gap` | 可选字符串 | 现有引导未覆盖的内容；有 ref 也可能有缺口 |
| `confidence` | 可选；已约定 high / medium / low | 判断把握程度 |
| `fit` | roles 可选；已约定 primary / alternate | 职责适合程度，不是实际绑定 |
| `need` | skills / tools 可选；已约定 required / useful | 需求程度，不授予权限 |

这些已约定值是开放词汇，未知值保留但不套用已知行为。正文只需说明当前分工、值得提炼的候选及提升条件。提炼时比较语义、任务证据与现有引导，决定复用、补充、合并或新建；不按条目数量创建 Agent、Skill 或文件。采纳后记录实际目的地，再 promoted / retired。


## Package

`schema: package` 描述一个有独立上下文的目录：它负责什么，以及相关资料在哪里、各自用于什么。项目根、产品、模块、功能、组件、私有源码、测试或文档目录均可采用；它不意味着 npm 包、公开 API 或独立架构边界，无需因此建立代码包、Controller 或 Task。新目录使用 package，已有 module 保持兼容。目录本身的项目根相对路径定位 Package，唯一 `.xmeta.yaml` 或 `.xmeta.md` 承载其 Common 字段与可选 `artifacts`。不额外声明 scope 或实例 ID。

`artifacts` 是 Package 内嵌的具体资料清单：这个目录实际有哪些文件、子目录或章节，及其具体用途。实际目录也可作为新增资料的具体写入位置。不新增 Artifact 实体或同范围的独立索引文件；没有清单需求时可以省略或使用空数组，已有的最小目录元信息仍然有效。

| 条目字段 | 必填 / 形状 | 含义 |
| --- | --- | --- |
| `kind` | 必填非空字符串 | 资料用途分类，如 design、contract、task、package、module；开放词汇，不声明目标的 schema 类型 |
| `description` | 必填非空字符串 | 简洁说明资料的实际用途或目标场景 |
| `path` | 必填材料引用 | 本 Package 范围内的具体文件、目录或章节；优先用载体相对路径，如 `./README.md`、`./.docs`；语法见 Common，不接受占位符或通配符 |
| `guidance` | 可选材料引用 | 维护此类资料时适用的规范或指南，可带章节锚点 |

条目允许自定义附加属性（`additionalProperties: true`）。`read_when`、`write_when` 不属于标准字段；仅在有明确本地需求时自定义，xschema 不规定其类型或语义。

每个 Package 只维护自己范围内有用的导航，父 Package 可以用 `kind: package`（已有 `kind: module` 同样有效）的目录条目引导到子 Package，由后者维护细节；读取父项不要求读取全部子项。条目不复制目标的正文、决策或实时状态，不表示 Controller 管理、Task 归属或运行依赖。相关但不属于此目录的材料使用 Common.links 或条目的 guidance；相同用途已由 artifacts 表达时，不再抄入 links 或 README 清单。AGENTS.md 保留行为规则与按需读取 Package 的入口。

同一个具体目的地只维护一条记录，需要多个用途时在 description 中简洁说明。`uniqueItems` 只阻止完整重复条目；同目标不同描述、所有权越界等仍需语义检查。`kind` 不推导目标类型，具体路径的存在性与锚点仍按扫描覆盖报告，未扫描不能视为缺失。清单不证明运行时存在性，也不授予写入权限。

运行时先按 Common 的载体规则归一化，再按路径段判断 `path` 是否为 Package 目录自身或后代。`./` 可以声明目录自身；嵌套 Package 的旧根相对目录名也可指向自身。工作区根 Package 的返回地址为 `.`，可拥有整个工作区内的目的地；根载体的 `../` 越出工作区，仍为无效引用。裸 `.` 是查询地址和 Controller scope 的专用写法，不属于材料引用语法，声明目录自身应使用 `./`。`mod-other` 不属于 `mod` 的后代。

重复检查只比较同一 Package 不同记录的 `path`，目的地包括规范资源路径和可选章节片段；整份文件与其不同章节是不同目的地。`./local.md` 和等价的旧根相对路径会冲突；同一记录的 `path` / `guidance` 同目标、多个 `guidance` 同目标及不同 Package 各自合法的导航均保留。归属或重复冲突在 relations / `package` view 中报告 `graph_conflict` diagnostics 与 omissions，`coverage.complete=false`，所有原始字段和指纹证据仍在。可解析边标为 `conflict`；目标已是 `missing`、`unscanned` 等时保留其 resolution 并附加冲突诊断，不能据此推断未观察的存在性。


## Module

`schema: package` 是 [Package](#package) 的输入别名，不是第二个模型。唯一 [package.schema.yaml](entities/package.schema.yaml) 拥有目录字段与 artifact 约束；旧 [module.schema.yaml](entities/module.schema.yaml) 和其 `$defs/artifact` 只转发到它。默认集合仅注册 `name: package, aliases: [module]`，旧 URI、`urn:xschema:entity:module` 与聚合引用仍转发到唯一 Package 声明。

| 边界 | 规范行为与旧输入兼容 |
| --- | --- |
| 查询元数据 | 新旧文档投影的 `metadata.schema` 都是 `package`；find 的 schema 等值条件、select 的实体源/原始 metadata.schema 条件及实体索引接受 `module` 别名 |
| SQL | 唯一 `package` 实体表包含新旧文档；`module` 是指向同一公开投影的兼容视图，没有第二份实体存储。SQL 文本、参数与任意字面值不改写，`WHERE schema='module'` 应改为 `WHERE schema='package'` |
| 目录视图 | 规范入口 `view: package` / `package-reading`；旧 `view: module` / `module-reading` 仅映射到同一实现。响应使用 `summary.view: package`，资料原因使用 `package-artifact` |
| 关系 | 规范名称 `package.artifact`；旧请求 `module.artifact` 映射到同一关系，结果只返回规范名。profile 的 source/target schema 同样按注册 alias 归一 |
| 原始证据 | 文件原文、载体名、指纹、字段路径及 evidence.raw 不变；字段 read 返回原始 schema，所以旧文档的 `read [schema]` 仍是 `module`。提供者的 rawMetadata 仅在投影不同于源字段时存在 |
| 类型引用 | 新类型 `PackageItem` / `PackageSummary` 拥有结果合同；旧 `ModuleItem` / `ModuleSummary` 是直接类型别名，wire 值仍使用 package |

实体与命名视图的可选 `aliases` 是当前定义集合内的输入名称，不会产生第二条目录注册或业务实现。名称/alias 冲突明确拒绝，关闭默认集合后，自定义 `module` 不会被全局改写。任意 SQL、来源视图输出以及 expand/project/group 后派生列上的值比较仍是字面语义，不根据字符串内容猜测类型；需要原值时读取源字段。历史快照保留，回放比较只适配上述明确改名的输出字段。


## Other Types

document（可发现文档）、example（独立示例）、skill（复用能力入口）使用 Common 字段，没有额外必填字段。保留原文件协议的其他字段；只在实际有管理价值时建模。

Role（`schema: role`）和 Scenario（`schema: scenario`）同样保留 Common 的 description、tags、links 等字段，并明确以下发现与分类信息：

| 主体 / 字段 | 必填 / 形状 | 含义 |
| --- | --- | --- |
| Role.role | 必填非空字符串 | 角色类别，如 controller、designer、implementation；开放词汇，使用一致命名，不是 Agent 地址或实例 ID |
| Scenario.tags | 必填去重字符串数组，至少一个非空标签 | 场景的领域、活动或主题分类，如 frontend、design；支持组合筛选，不表达阅读顺序或执行条件 |

`schema` 区分主体类型，`role` 区分具体角色类别，路径继续定位实例。三类指引都建议提供 description；Role / Skill 的 tags 以及三类主体的 links 按需填写。links 是资源级相关材料，正文中的条件链接不必再抄一份。标签复用已有词汇，不预设封闭枚举或新的分类层级；查询前端设计场景可对 tags 分别执行 contains frontend 与 contains design，按 AND 组合。也可用 SQL 查询和聚合（查询用法见 xschema 工具自身的使用说明），不新增路由字段。

三类指引在正文中渐进披露适用条件、方法和阅读路径；Role 保持精简并路由到场景，Scenario 可组合体系内材料，Skill 尽量自包含、可移植。多个场景的聚合也使用 scenario。Task Meta 的 scenarios / roles / skills / tools 仍是任务级分析建议，不作为这些指引主体的路由字段。


## Consumption Boundary

- 解析使用 YAML 1.2 普通数据，拒绝重复键、不支持的标签与循环结构。已知核心字段按主体解释；开放扩展不允许把 agent 放进 task-meta。
- 结构声明只校验形状。读取器另检查载体/content 唯一、实际路径边界/符号链接、目标类型和锚点、图约束、阶段归属、Meta 相邻及建议字段适用组。uniqueItems 比较完整元素，不保证关系键唯一；冲突不得静默去重。
- 扫描明确根与排除项，并携带覆盖范围；区分 missing / unscanned / unmodeled / invalid / conflict / unsupported。生产查询排除本模块 examples；部分扫描不能证明全局缺失。
- listening + active Task 只产生核对提示；状态或运行绑定不明时保持 unknown。元数据不触发消息、恢复或写入。未来编辑器须检查并发变化、保留正文与未知字段，并说明多文件部分失败；这些运行能力尚未实现。


## Canonical Task and Delivery standard

The canonical default collection declares authoritative Task and Delivery source bindings with only `.task.md` and `.delivery.md`, respectively. Both use `assertModel: true`. There is no conventional old-name, YAML or arbitrary-front-matter discovery fallback for these supplied models. The generic SourceBinding compiler owns candidate domains and excludes unrelated filenames before metadata/body acquisition; the scanner has no Task/Delivery switch. A directory traversal is still needed wherever matching descendants may exist, and current policy/no-follow admission remains mandatory.

`assertModel: true` requires a model target and file filename locators. A known matching candidate with absent/different content schema receives `source_model_mismatch`. Content retains identity and field validation; a wrong suffix cannot manufacture the expected model row. A known declaration outside its authoritative model binding receives `source_model_outside_binding`; metadata remains available for explicit source inspection, but it cannot populate model tables, relation ownership or model views. Unknown/unreadable candidates retain partial/unknown evidence. Raw caches remain definition-independent: the assertion and source projection gate run after raw-format acquisition, including reuse. Binding revisions remain part of the existing definition semantics.

