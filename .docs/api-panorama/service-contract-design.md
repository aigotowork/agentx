---
schema: document
status: accepted
owner: agentx-domain-contracts
last_updated: 2026-11-17
description: Consumer-service boundary, first-slice operation contracts and projection policies for the authorized SDK/CLI delivery.
links: [../proposals/contract-service-projections.md, ./README.md, ../../.controller/contract-service-projections/AGENTS.md]
---

# Service 契约与派生设计

这是实施目标设计；准确的可执行签名最终归属于实现中的 Rust 声明。当前源码、建议操作全景和实际交付状态分别见[全景](README.md)、[原生契约](../architecture/contracts.md)、[Delivery](../../.controller/contract-service-projections/implementation.delivery.md)。rovi 负责本次交接的契约方案，lezi 负责后续实现分派、工程决策承接与状态治理。实现者可在保留本页业务语义的前提下细化模块、类型和工具，不需要逐文件重新请示。

## 1. 边界与依赖

保留 Rust/serde 原生源、daemon 存储所有权、既有领域处理和 host/worker。公开 Service 是消费者能力，不是把内部 `Application::Request` 暴露给远程调用者。具名方法提供明确的输入—输出类型关联；每组共享真实领域责任，不强制资源→trait→crate 一一对应。

| 边界 | 消费者 | 拥有的契约 | 实现责任 |
| --- | --- | --- | --- |
| 领域操作与事实 | Application、原生服务 | 身份、状态、事务、授权、持久回执、允许的效果 | 复用原生 agents/messaging/collaboration/inspection 等拥有者 |
| 管理消费者 Service | HTTP、类型化原生消费示例 | 有界读模型、管理操作、worker 接纳与结果观察 | 绑定既有 Client、管理 journal 与独立 worker；不在 SQLite dispatch 做进程 I/O |
| Workbench 消费者 Service | workbench HTTP | 五项 Read、九项 OperatorCommand；显式拒绝 | 不授予 AgentReport 权限，project 固定于 host |
| Observation 消费者 Service | observation HTTP | 十三项 BrowserRead、固定 Scope；证据与失败 | 不获取／启动 daemon，不把失败转换成健康空值 |
| HTTP 投影 | handler、规范导出 | 路由、参数位置、wire schema、HTTP 错误、安全边界 | 与 Service 类型关联；只处理解析、编码和入站 context 适配 |
| SDK / HTTP CLI | 应用、自动化、用户 | 公开方法、命令、配置、结果观察、恢复 | 不实现第二套业务规则，不直接打开 AgentX 数据库 |

HTTP/SDK 文件不得依赖数据库实现来导出 schema。仓库当前只有 bin crate；可先在现有模块边界内提供独立可导出的契约闭包与 SDK 生成入口，不把拆公共 Rust crate 作为前置要求。Trait 只在注入／替换确有需要时使用；不能为原生 IPC 每个 enum variant 再包一层无意义转发。

## 2. 对象、输入与 authority

| 概念 | 第一版选择 | 执行保证 |
| --- | --- | --- |
| Generation | 不透明字符串；与服务描述符／回执绑定 | 旧代次拒绝；对未知提交不自动换代重试 |
| AgentRef | 当前管理 wire `{name,incarnation,stopped_event}`；后两项恰好一个 | incarnation/stopped_event 的实际验证留在原生拥有者；内部可用更强 enum 并显式转换 |
| 当前协作参与者 | 原生 ParticipantRef `{canonical_name,incarnation_bits}` | 仅当前精确参与者可转换；拒绝把历史 stop 引用转成当前实例 |
| RequestId / CommandId | 新意图生成一次，重试保持原始 ID、代次、命令 | 复用既有 scope/generation/原命令绑定，不创设跨服务全局幂等保证 |
| Revision | labels、Work、Brief、配置等各自拥有 | 不同 revision 不可互换；冲突不自动读新值覆盖 |
| Context | host 捕获配置／可信过程身份；领域核验 caller claims | 远程 body 不含 InvocationContext、env、PID、ticket 或任意 actor capability |
| Project / scope | 沿现有入口固定／选择策略 | 不因为 SDK 一个参数而赋予切换服务数据范围的权限 |
| 分页 | 每个资源使用其原生 page/cursor | 不统一 cursor 语义、不承诺未有的跨页原子快照；保留覆盖率与缺口 |

构造／解析分工：输入 schema 验证形状与局部约束；领域验证权限、现存对象、版本、跨请求一致性。SDK 可提前校验，但服务端必须重复执行拥有者负责的校验。不要通过 permissive schema 或强制 cast 掩盖未建模分支。

## 3. 首个五操作闭环：原生消费者签名

下列为消费者语义形状；精确可执行签名归属于 [`service.rs`](../../src/management/service.rs) 与 [`contract.rs`](../../src/management/contract.rs)，编译和实际集成状态由 [native Task](../../.controller/contract-service-projections/tasks/pilot-native.task.md) 记录。`ServiceResult<T>` 复用既有原生类型化失败；HTTP 边界通过穷尽映射保留既有公开错误码。

```text
describe(ctx: TrustedManagementContext)
  -> ServiceResult<Descriptor>

list_agents(ctx, input: ListAgentsInput {
  generation: Generation,
  after: Option<AgentCursor>, limit: Option<PageLimit>,
  current_only: bool = false, labels: Vec<LabelSelector> = []
}) -> ServiceResult<AgentPage>

get_agent(ctx, input: GetAgentInput {
  generation: Generation, agent: AgentRef
}) -> ServiceResult<AgentView>

submit_message(ctx, input: SubmitMessage {
  request_id: RequestId, generation: Generation,
  agent: AgentRef, message: MessageInput
}) -> ServiceResult<Operation>

get_operation(ctx, input: GetOperationInput {
  generation: Generation, request_id: RequestId
}) -> ServiceResult<Operation>
```

实际实现入口为 [`management::service::ManagementService`](../../src/management/service.rs)：`new(scope: PathBuf, project: PathBuf, invocation: InvocationContext, workers: Arc<Semaphore>)` 捕获可信 host context；五个同步方法省略逐次 ctx 参数，返回原生 `crate::service::ServiceResult<T>`。输入声明在 [`contract.rs`](../../src/management/contract.rs)，`SubmitMessage` 与现有 wire 复用；`labels` 使用原生 `Vec<String>` selector。原生 `ServiceError` 沿既有穷尽映射得到 v1 公开 code，不另建服务错误继承体系。该模块提取既有 host 编排供 HTTP 与原生消费者共用，不复制 journal／worker 业务规则；当前执行与接入证据仍以 [Task](../../.controller/contract-service-projections/tasks/pilot-native.task.md) 为准。

整批管理扩展沿同一个具体 Service 补齐有实际消费者的生命周期／labels／mailbox／历史、目录发现／发送、附件磁盘供给；[`service/resources.rs`](../../src/management/service/resources.rs) 保留 host I/O 与前后 generation 检查，HTTP 保留二进制 framing／响应头。Inspector 通过 [`service/inspection.rs`](../../src/management/service/inspection.rs) 绑定独立 Snapshot/AnalysisJob/RecoveryAttempt；[`inspection::contract::PublicMutation`](../../src/inspection/contract.rs) 与原生 Request 共享四项 payload，保留原有外部标签，排除 Collect/RunAnalysis。当前整批实现／集成状态由 [full-native Task](../../.controller/contract-service-projections/tasks/full-native.task.md) 拥有；不把源码供给当作集中验收已通过。Workbench／Observation 复用既有有限消费者边界，不为清单数量添加逐 variant 转发层。

TrustedManagementContext 是职责名称，实施可直接复用已有经过构造的 context，不要求创造新 runtime 身份系统。`limit=None` 在服务侧使用既有 50，范围 1..100；labels 筛选只允许 current_only=true，AND 语义和 selector 绑定游标沿原生。HTTP 层将兼容的 query 字段归一成这里的输入，不能做第二套选择规则。

| 方法 | 输入来源与前置条件 | 结果承诺 | 对应真实 HTTP |
| --- | --- | --- | --- |
| describe | 到达选定 listener；不传 generation | 固定 scope/project、当前 generation、tools/profiles/limits；不授予权限 | GET /api/v1/management |
| list_agents | generation 来自描述符；after 来自前页 | 有界当前／停止实例投影，保留 current_complete/next；列表读与维护分开 | GET /api/v1/agents |
| list_workspace_members | generation 来自描述符；workspace 来自已选已采纳地址；after 来自同一选择器的前页 | 原生先筛选已采纳成员，后分页；支持当前与停止实例 | GET /api/v1/workspaces/members |
| get_agent | AgentRef 来自列表或已保存精确对象 | 精确对象的当前投影；同名替换不得按名称回落 | GET /api/v1/agents/{name} |
| submit_message | request_id 与原始输入在 dispatch 前保留；can_send 与附件由原生再次验证 | 接纳后返回原 Operation，worker 执行既有原生发送；HTTP 202 不等于最终成功 | POST /api/v1/agents/{name}/messages |
| get_operation | 原 generation+request_id | 原回执及原生 reconciliation 的当前结果；NotFound 不证明没执行 | GET /api/v1/operations/{request_id} |

第一批消息仍使用原生 MessageInput 的完整字段：text、attachments、intent、reply_to、thread。复用 64 KiB 文本限制、intent request/inform/ack、现有元数据与附件约束；不在 SDK 猜测收件人或静默丢弃字段。第一批消费者可用无附件消息完成场景，但契约必须准确表示全部字段。

### Workspace membership and launch identity

`GET /api/v1/workspaces/members` 用已选代次和已采纳的地址读取实际成员。工作区键是原样保存的绝对路径；`global` 只匹配有效采纳记录中明确保存的 `workspace: null`。旧记录、格式不支持的记录和远端实例不会混入，进程工作目录和 CLI 可见性规则也不参与。原生先筛选后取页，当前名称升序，随后按停止事件 ID 降序；`current_only=true` 只读当前段，`current_complete` 仅描述当前匹配成员。游标绑定代次、工作区和该选项；分页观察的是变化中的数据，每页独立。描述符的 `supports_workspace_members` 缺失或为 false 表示当前端点不支持；新版管理端连接旧版 daemon 时返回 `Unsupported`，不会转发未知请求或退回全局页。

启动、恢复、分叉的 `Launched` 回执保留 `agents`、`failed`、`errors`，并由实际成功的原生预留提供可选的 `agent_refs`。引用按 `agents` 顺序保存，携带准确的 `incarnation`；它不是就绪信号或当前执行保证。操作须以原代次、原请求 ID 恢复；收到旧回执或混合版本回执时可能缺少 `agent_refs`。同名后续实例永远不能用于补全；原引用失效时应显示其不可用。源代码与当前验证范围见 [native handoff](../../.controller/typescript-domain-sdk/tasks/native-completeness.task.md)。

## 4. 结果、错误与客户端知识

| 类别 | 保留的表达 | 客户端动作 |
| --- | --- | --- |
| 有界读取 | Page/View/Descriptor，及原生可用性／覆盖信息 | 展示实际观察，不把未知补成默认值 |
| 管理操作 | Operation{request_id,generation,status,result,error} | Accepted/Running 继续观察；Succeeded 解读 result；Failed 保留原因；Unknown 进行原身份查询／人工决策 |
| 原子协作 | Applied(Change) 或 Rejected(Failure) | 处理 revision/eligibility；command_id 查询原变化；不包装成新的全局操作 journal |
| Inspector 分析 | AnalysisJob / RecoveryAttempt / Snapshot | 依原生语义观察；不将 queued 表达成已执行 |
| 提交前明确拒绝 | 公开稳定 code 与安全 detail | 修正输入或目标；不能和提交后未知混为一类 |
| 提交后 transport loss | 客户端 OutcomeUnknown{operation identity, cause} | 使用原 ID 查询；不会生成新 ID 自动重新执行 |

首批公开错误码复用管理现有 `InvalidInput / Forbidden / NotFound / Conflict / StaleReference / Unsupported / Unavailable / OutcomeUnknown / Internal`，禁止解析 message 字符串分支。结构化字段仅由拥有者能够明确提供时新增，如现有协作 RevisionConflict.current_revision；不能伪造历史管理 Operation.error 的分类。

为兼容既有 Web，v1 管理 wire 的 status/result/error 形状保持；错误文案不作为程序接口。新增精细错误或 tagged Operation 联合时优先增加内部／SDK 类型与显式映射；如改变公共序列化、允许值、原生 IPC 或存储，先记录影响并按现有版本/epoch/兼容规则实施，不能直接重写历史回执。

Operation 仍可能包含不同 CommandResult。SDK 提供 `send()` 返回 Operation 与可选 `waitForSent()` 类型化组合；不能 unchecked cast 所有成功 result 为 Sent。验证请求 ID、generation、状态与 result variant；遇到不匹配显式协议失败并保留回执。结果未知不等价于 Failed；取消等待不取消服务工作。幂等查重优先于当前 worker 容量／目标状态的顺序沿现有 journal，避免重放旧请求被新环境错误遮蔽。

## 5. 三个 HTTP 投影与数值策略

| 投影 | 规范与客户端边界 | 特殊要求 |
| --- | --- | --- |
| Management（含 Inspector） | 一个明确 management base URL；既有投影外加工作区成员读取 | 普通 JSON；关键身份字符串；保留 attachment 原始二进制、四个 Inspector 外部标签、stop 兼容别名 |
| Workbench | 单独 workbench base URL；connection/read/apply | 全部整数标记 codec；五项 Read／九项 OperatorCommand；200 Rejected 与 transport error 分离 |
| Observation | 单独 observation base URL；scope/read | 全部整数标记 codec；十三项 BrowserRead；LocalFailure、stale/gap/epoch、被动连接不丢失 |

OpenAPI 必须描述 wire。不能仅用 bigint TS 类型声称普通 JSON parser 已经无损读取 u64。首批工具试验必须同时验证管理普通 JSON 中大于 2^53−1 的 u64、工作台/诊断的整数标记、外部标签联合与 null/omission。

第一版管理选择保留现有 wire，通过无损 JSON 解码与基于契约字段的转换生成精确 SDK 数值；仅有 JS 安全范围且有明确上界的字段使用 number。不得全局把所有数字或数字字符串一起转换。若候选生成器缺少可插入的 codec，则选择有该能力的生成器或一个薄解码适配；不得未经协议决策把现有 v1 number 改为 string。实现者要在源声明／配置中记录每类整数映射，测试上下界和往返；此处选择无损目标，不预选尚未试验的依赖。

HTTP 元数据包含稳定唯一 operationId、参数 required/null、内容类型、成功与错误 status/schema、精确引用的跨字段约束、权限／Host／Origin 语义。不能表达的约束明确附注并由服务执行。三份 listener-qualified 规范合计 35 个 method/path；消费者逻辑全景是 59 行（含一个别名）。先做五项，再覆盖整个清单；不以导出五项就宣称全量结束。

## 6. 可重复派生与包边界

单一可编辑源链：领域声明 → Service 关联类型 → HTTP 表面元数据 → OpenAPI → wire client。不同表面只维护自己特有的命名和映射；不再手抄一套业务 DTO。生成配置和工具版本固定，现有脚本／just 接入可重复导出、校验与生成差异检查。native 声明不得导入 SDK 或 CLI。

SDK 公共入口组合三套 client 的独立配置；内部 generated 目录由工具拥有，手写 codec/领域具名封装/回执等待放在生成目录外。浏览器入口与 Node CLI 的文件读写分开，不能把 node:fs 或 server startup 拉进浏览器 SDK。每个已公开方法可追溯到 operationId 或某个封闭 union variant；辅助方法标明组合来源及效果。

CLI 使用[全景命令树与输入矩阵](README.md#4-cli-命令树与参数全景)作为语义基线。`agentx-api` 是工作命名；最终包名/bin 名称由 consumer Task 检查已有命名冲突后记录，不改变原生 `agentx` 语法。普通 help/version 离线、无副作用。JSON stdout、stderr、退出码、stdin、文件覆盖规则和 pending ID 保留必须由实际 CLI 验证。

## 7. 验收矩阵与演进边界

| 场景 ID | 行为 | 验证边界 |
| --- | --- | --- |
| CS-01 | 描述符→分页→精确 Agent→发送→原回执 | SDK 与 CLI 正式入口各走真实隔离 HTTP/native/worker；区分 Provider fixture 与实际 Provider |
| CS-02 | 同名新 Agent / 旧 generation | 原生拒绝，HTTP/SDK/CLI 保留错误，不替换目标重试 |
| CS-03 | 同 ID 同输入、不同输入、worker busy 时回查 | 原 journal 保证与 SDK 观察一致；不再执行外部效果 |
| CS-04 | 请求已提交、HTTP 回复丢失、reload/重新查询 | 保留 ID/代次/原 payload；Unknown/NotFound 不变成“没执行” |
| CS-05 | Operation 失败／未知／错误 result variant、等待取消 | 客户端不误报 Sent，不假称取消服务工作 |
| CS-06 | null/omission/false/empty、整数边界、外部标签、二进制 | schema、生成 SDK 与真实 wire 的正反向检查 |
| CS-07 | 标签 revision、生命周期能力、目录预览／发送 | 扩展管理组的真实业务场景；目录 inspect 不启动 Agent |
| CS-08 | Work 创建→分派→结果来源→检查→接受／退回 | operator 与 AgentReport 权限分开；精确 revision/结果/检查依据；报告通过既有授权原生路径 |
| CS-09 | Observation daemon 缺失／重启、事件缺口、不可比较配置 | 不启动 daemon、不隐式 rebase、不抹去失败或历史含义 |
| CS-10 | 从固定源重新导出、生成、安装/打包消费 | 规范覆盖与实际路由一致，无陈旧生成物；包消费者不依赖工作区私有导入 |

必要的原生／适配器检查可由实现者执行；整体真实调用链与包安装/CLI 验收由独立 acceptance owner 负责。检查既有受影响 Web 消费者保持兼容；不把离线 mock、类型通过、生成器成功当真实服务效果证据。最终平台范围、构建、安装与在线采用分别记录。

实施中的未定工程项只有：具体 OpenAPI/SDK 工具与固定版本、最终包/CLI 入口、无损 parser 与字段 codec 的适配机制。负责人必须用首批实际 schema/consumer 试验解决，记录选项、结果与选择；不能为了“先跑通”降低这里的精确性或改变权限边界。上线／替换已安装程序／重启既有用户服务须遵守原资源所有者与既有部署授权，本次研发推进不含任意 live transition。
