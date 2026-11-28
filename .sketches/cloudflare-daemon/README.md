# AgentX daemon 在 Cloudflare 上可以怎样运行

**决定路线的第一个问题是：谁保管已经答复成功的那条消息？** 接受自管持久主机时，Tunnel 可以直接复用原生 daemon；要求 Cloudflare 承担计算和状态时，Containers 与 Workers 都需要设计可恢复的提交边界。三条路线可以分别立项，没有依次迁移的前置关系。

| 要解决的问题 | 候选路线和状态归属 | 适用条件 | 先拿什么证据 |
| --- | --- | --- | --- |
| 让两台 NAT 后的主机经公网协作 | [Tunnel + Linux](tunnel.md)。Cloudflare 提供入口，SQLite 与唯一写进程在 Linux origin | 可以持续维护主机、磁盘、备份，只希望先打通公网 | 真实 CLI / Codex 在公开域名收发；连接器中断后按原操作恢复 |
| 云端运行现有 Rust 服务 | [Containers](containers.md)。容器复用 HTTP 实现，必须为成功写入安排跨实例的持久权威 | 保留 Linux 软件栈有实际价值，团队能承担持久后端适配和运维 | 销毁容器和原盘后成功回执仍可查，旧写者恢复也不能覆盖新状态 |
| 去掉自管主机，按 scope 托管通信状态 | [Workers + SQLite-backed Durable Objects](workers-do.md)。入口处理请求，按部署和 scope 划分状态所有者 | 愿意实现新的云端适配器，维持既有 HTTP 契约 | 在对象重启、响应丢失及轮换并发下重试；单个繁忙 scope 的延迟和费用符合目标 |

## 从哪里读起

- [空闲收件、在线租约与休眠方案](idle-delivery.md) 比较退避和按主机通知，列出费用、延迟与断线补领的验证方法。

- [5 美元套餐的对象划分与账单](paid-do-budget.md) 说明类、对象、请求、计费时长和存储分别消耗什么，以及多个项目如何共享额度。

- [Rust、Workers 与 Durable Objects 图解](runtime-and-objects.md) 从源码编译讲到对象身份、私有 SQLite、收件事务与设计模式，也可直接打开[离线图解](runtime-and-objects.html)。

- [体系与能力边界](system.md) 说明管理端、云服务和执行宿主分别掌握什么，区分已验收的云通信与已经过三机控制验收的联网 daemon。
- 三个路线专题分别给出拓扑、可复用部分、需要改造的部分、故障情形、官方资料及未来试验。分工和原始问题保存在 [Tunnel 任务](tasks/tunnel.task.md)、[Containers 任务](tasks/containers.task.md)、[Workers 任务](tasks/workers-do.task.md)。
- [共同验证与成本口径](validation.md) 给出同一批黑盒场景、各路线的首个否决条件，以及用真实宿主收件频率估算请求量的方法。

## 三个会改变选型的问题

**公网入口是否强制 Cloudflare Access？** 现有 CLI 能向公开 HTTPS 地址提交 AgentX Bearer。若同时启用 Access，机器客户端还要能独立提交 Access 服务凭据；管理页面打开后，浏览器 API 请求还要携带 Access cookie，而现有管理代码显式省略 cookie。把双层鉴权纳入要求后，Tunnel 路线也有客户端改造和回归成本。[Tunnel 鉴权与验证](tunnel.md)

**是否允许使用 Cloudflare 以外的事务数据库？** Containers 专题的主候选是 Rust 容器连接事务主库，由主库在同一事务内检查写者代次并提交业务记录。若所有持久服务都必须在 Cloudflare 内，应直接比较 DO 承载域事务和纯 Workers/DO 的工作量。容器快照有快照间隔、镜像版本与保留期边界；仅靠快照无法恢复快照之后已经答复成功的写入。[Containers 存储候选与故障判据](containers.md)

**空闲 Agent 是集中在少数 scope，还是分散在很多 scope？** Workers 方案中，同一个对象上重叠的 HTTP 等待共享活跃时长，不同对象分别累计。即使总 Agent 数和消息量相同，两种部署的费用也可能差很多。衡量方案时，应同时记录每个 scope 的在线分布、空 claim 数、等待时长和最热 scope 的延迟。[Workers 容量、费用与验证](workers-do.md)

## 以后如何做选择

如果近期目标是外网可达，可以先给同一套 Linux 服务加 Tunnel。这一步也能采到真实负载，为是否值得迁移存储提供依据。如果要求撤掉常驻主机，可以直接研究原生 Workers 后端；优先证明原客户端在凭据轮换、消息领取和实例重启后还能恢复。Containers 的价值取决于复用 Rust 服务能省下多少工作，以及为了跨实例持久化又要改掉多少代码。三条路线都将 Provider、凭据和执行证据留在执行主机，云上运行 Agent 本身属于另一项产品选择。[云端操作指南](../../.docs/architecture/cloud-scope.md) · [控制平面设计](../../.controller/cloud-daemon/AGENTS.md)

对现有部署迁移还要处理身份连续性。原生客户端按 scope URL 和主机名派生本地状态目录；管理变更重放会返回原始回执里的 URL。用新域名跑通一个新 scope，无法证明老 Agent 会自动接回原会话。若未来需要无感迁移，应单独验证保持原公开 URL、deployment ID、generation、凭据版本、原始操作与未确认 claim；若选择创建全新 scope，则用新注册和新会话验收。切换曾接收写入的新后端前，需要定义如何保护这些新增回执，简单改回 DNS 可能让客户端看到旧状态。[客户端状态路径](../../src/cloud/client.rs) · [管理回执与身份](../../src/cloud/catalog.rs)

本目录为方案调研。已运行的对照来自项目的 [Linux 验收](../../.controller/cloud-scope/delivery.md) 和 [三机实验](../../.controller/cloud-scope/three-host-lan.md)；Cloudflare 各路线的试验与选型尚待开展。
