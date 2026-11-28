---
scope: Cloudflare Workers + SQLite-backed Durable Objects
status: research only
owner: bizera
related: [tasks/workers-do.task.md, ../../.controller/cloud-scope/AGENTS.md, ../../.controller/cloud-daemon/AGENTS.md]
last_updated: 2026-11-22
---

# AgentX 云端 Scope：Workers + Durable Objects 可行性

**建议进入小规模原型验证。** 首选一个入口 Worker、每个部署一个 Catalog DO、每个 scope 一个 SQLite DO。把 scope 的凭据版本、消息和收件箱放进同一个事务域，才能清楚地回答「轮换时这条消息究竟有没有写进去」。这个方案保留现有 HTTPS 契约，但要改写服务端的运行时、存储接入、等待和跨对象管理流程。适合多个独立的中小 scope；单个大 scope 的跨洲延迟、等待费用和长期存储如果越过预算，就不应直接采用。本文只查阅源码、既有验收和 Cloudflare 公开文档，未实现、部署、创建资源或发起运行探针。

## 已验证的 AgentX 基线

- [交付记录](../../.controller/cloud-scope/delivery.md)和[三机实验](../../.controller/cloud-scope/three-host-lan.md)分别给出原生 HTTPS、SDK/CLI、TUI/Web 和真实局域网通信及轮换的证据；[验收记录](../../.controller/cloud-scope/acceptance.task.md)还覆盖发信唤醒、轮换撤销等待和响应丢失后的重放。这些是 Rust 服务的证据。部署 bearer 管理 scope，scope bearer 允许通信；本机仍拥有 Provider、进程、工作目录和投递确认的证据。
- [Catalog 实现](../../src/cloud/catalog.rs)以文件锁和进程内互斥确保单一所有者。先初始化未发布的 scope 数据库，随后在 catalog 事务内发布 `generation`、凭据版本及不含 token 的回执。管理写入用部署内唯一的 `request_id` 重放，原请求在重启或后续轮换后仍返回原版本、原 URL；更改请求内容冲突。活动 scope token 摘要在部署内唯一，轮换对比 `expected_version`，不更改 scope `generation`。原生服务还拒绝 scope token 等于部署 bearer，并在启动时检查部署 bearer 没有历史 scope token 记录（[启动和管理入口](../../src/cloud/mod.rs)、[admin 写入](../../src/cloud/admin.rs)）。
- [通信契约](../../.controller/cloud-scope/communication-wire.md)、[事务实现](../../src/cloud/communication/store.rs)和[收件箱实现](../../src/cloud/communication/store/inbox.rs)规定：host 名绑定 installation，60 秒在线判定不回收名称；scope 内短名唯一，退役实例不能复活。发送事务冻结收发双方的精确实例；相同 `request_id` 和输入先重放原消息，再考虑当前名字。不同 `claim_id` 争用同一未结批次返回 `409 claim_conflict`；空批次不占位；`confirm` 和显式 `recover/release` 按原 claim 处理，没有超时自动重投。旧 token 的等待会被撤销；已在途的写可能已经提交，用新 bearer 查原 ID，而不是换 ID 再发。

## 候选拓扑与复用边界

| 组件 | 权威和流量 |
| --- | --- |
| 入口 Worker | 托管同源管理静态资源，保留 `/api/v1/admin` 和 `/scopes/{id}/api/v1`。识别两种 bearer，管理密钥用 [Workers Secrets](https://developers.cloudflare.com/workers/configuration/secrets/)，scope token 可在入口算 SHA-256 后仅在可信绑定上传摘要，最终由 Scope 比对。沿用 HTTPS、请求大小、`{code,message}`、`no-store` 和现有路由/DTO；业务响应和诊断不带 bearer 或摘要。 |
| `CatalogDO`：每个部署一个 | 权威保存稳定的部署 ID、已发布 scope → `(DO ID, generation)`、创建和轮换保留项、活动/保留摘要的全局唯一约束以及管理回执。只接受内部绑定、管理请求和路由查询。入口可缓存已发布的肯定映射；缓存只路由，绝不授权，也不缓存凭据版本。首次访问、冷入口或缓存失效才查 Catalog；未知 scope 返回与现有 scope 鉴权一致的 401，不实例化任意 Scope。Catalog 无法确认映射时关闭访问并返回可重试错误。此优化依赖当前没有删 scope 或复用 scope ID 的契约。 |
| `ScopeDO`：每个 scope/generation 一个 | 唯一权威保存**当前有效**的凭据版本/摘要、host/Agent 目录、消息和 claim；同一对象事务做鉴权与业务写。同步提交后才通知临时等待者。管理查询按 `generation` 读取短事务、有限分页和摘要，与 [Rust admin](../../src/cloud/admin.rs) 一致：无法读取的 scope 必须体现为不可用覆盖，不能用健康的零值掩盖。入口限制并发扇出；严格全局 overview 的工作量随 scope 数增长。 |

这是应用设计。Cloudflare 文档确认 [按名称取 stub](https://developers.cloudflare.com/durable-objects/api/namespace/#getbyname)、[对象生命周期](https://developers.cloudflare.com/durable-objects/concepts/durable-object-lifecycle/)和[单对象 SQLite 事务](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/#transactionsync)；SQLite 数据只能由所属对象的存储 API 访问。新类显式选择 SQLite（[配置](https://developers.cloudflare.com/workers/wrangler/configuration/#migrations)）。可复用 [admin OpenAPI](../../packages/agentx-client/cloud-admin.openapi.json)、[scope OpenAPI](../../packages/agentx-client/cloud-scope.openapi.json)、现有各客户端、Web 构建产物、测试用例和表/索引设计；以原生契约为基准，不另造一份对外协议。重写 Axum、文件锁、rusqlite/WAL、Tokio 等待和进程级限流，Worker 的 [虚拟文件系统](https://developers.cloudflare.com/workers/runtime-apis/nodejs/fs/)不能代替持久目录。DO SQL 用 `transactionSync`、同步消费游标，环境 I/O 不进入事务；保留 `i64` 消息 ID 的十进制字符串（例如 SQL `CAST(id AS TEXT)` 输出，应用校验范围后以文本参数转回 SQL INTEGER），别经 JS `number`。用原生向量核对输入正则、UTF-8 字节长度、字段规范化/指纹、nullable 字段和错误码。支持已有数据迁入若是要求，原生 DB 到 DO 的导入、验证和切换另行设计。

## 跨对象写入和失败语义

**创建。** Catalog 在短事务内先查回执/待办，再为 `(request_id, scope_id, generation, digest)` 原子保留名称和摘要。用记录中的确定对象名初始化 Scope，Scope 以同一保留项幂等建表、写入 generation 和 v1，并证明已持久化；此时公共入口仍无法路由未发布对象。Catalog 核对原保留项，把已发布映射和最终回执写入同一事务。三个阶段之间不跨网络持有事务。任一响应丢失均以原 ID 和内容续做；相同 ID 改内容冲突，初始化重复不得重置 scope。未知提交的保留项不因超时自动释放：先以原 ID 或 Scope 的持久阶段核对。最小版本由同一请求续跑；若要求无人重试也能收敛，再用 Catalog 中的持久待办和一个 [DO alarm](https://developers.cloudflare.com/durable-objects/api/alarms/) 扫到期项。alarm 至少执行一次、平台重试有限，处理器必须幂等并暴露卡住的待办；不能靠内存定时器做恢复。

**轮换。** Catalog 同样先重放原回执，再比对版本并保留新摘要；每 scope 只允许一个尚未收敛的管理写，竞争请求可收到暂不可用，确定版本过期后返回 409。旧摘要在完成前仍占用全局唯一索引，新旧都不能被其他 scope 抢占。Scope 单事务按原管理 ID 和 `expected_version` 做 CAS，持久替换摘要、递增凭据版本、存本地切换回执。这个提交是吊销点；无需 Catalog 回调，等待者在提交后收到唤醒。Catalog 收到对应结果才完成管理回执、释放旧摘要并报告成功。若 Scope 已切换而 Catalog 未确认，新 token 仍可通信，旧 token 永远不能依赖 Catalog 的旧记录进入；管理视图对此边界须按原 ID 对账，或明确返回暂不可用，不能把旧版本当作当前版本。两种摘要要与管理凭据区分；配置级部署 bearer 更换还需保留原生的历史摘要检查，和 scope 轮换分开验证。

**轮换与消息竞争。** 在同一个 Scope 的短事务内重新核对持久的 `(generation, credential_version)`，不靠多个 stub 的到达顺序（[stub 顺序边界](https://developers.cloudflare.com/durable-objects/api/stub/)）。旧 bearer 的事务若先于轮换提交，可以留下原消息或 claim；若轮换先提交，旧请求不能再写。尚未决定响应的旧长等再次读取持久版本后给 401，不给旧批次；已经决定的响应可能因网络在轮换成功之后才抵达。摘要异步计算放在事务前，唤醒副作用仅在确认写持久之后执行（[Storage gates 与 `sync()`](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/#sync)）。新 bearer 以原 `request_id` 查 operation 或以原 `claim_id` 恢复，`generation` 没有变化。

**重试。** [Cloudflare 自动重试](https://developers.cloudflare.com/durable-objects/best-practices/automatic-retries/)覆盖特定 stub 网络错误，默认首次加至多四次重试，重试起点有 10 秒窗口；请求不可重传、已执行后出错、HTTP 500 和 `.overloaded` 等不能靠它解决。`retryable()` 可允许不确定是否送达的调用再次运行，只用于已有精确持久去重语义的方法；不把「发起 Provider launch/stop」标成可盲重试。传输超时/响应丢失按同一键查询或重发，变更输入及明确 4xx 不自动重发；对 transient 使用新 stub、抖动和总预算，过载做背压，不立即重试（[错误处理](https://developers.cloudflare.com/durable-objects/best-practices/error-handling/)）。[现有原生客户端](../../src/cloud/client.rs)对 5xx/传输错误立即再试一次，应用预算须纳入这一层；不能用新 ID 避开已提交写的不确定性。

## Inbox 长等：把连接丢失与投递丢失分开

保持 [当前 HTTP 语义](../../src/cloud/communication.rs)：`wait_ms` 默认 0、上限 20 秒；订阅临时通知后立刻执行一轮持久 claim 检查，空且无未结 claim 才等待，不持 SQL 事务、不用 `blockConcurrencyWhile` 包住等待。发送、释放、退役或轮换提交后通知对应等待者；超时、唤醒都再检查持久权限与队列。用每 scope 的等待槽、有限注册及超时清理给普通写和轮换留出余量。保留单条正文 64 KiB、最多 100 条且正文合计 1 MiB 的批次约束。

普通 HTTP 等待期间对象没有资格休眠；[生命周期](https://developers.cloudflare.com/durable-objects/concepts/durable-object-lifecycle/#shutdown-behavior)和[已知问题](https://developers.cloudflare.com/durable-objects/platform/known-issues/#global-uniqueness)说明，代码/运行时更新和迁移仍可使对象替换，旧实例访问存储会报错，内存中的 waiter、定时器和 HTTP continuation 不可恢复。重连时以原 `claim_id` 走持久状态：未提交仍可重新检查，已提交返回原批次剩余项，旧 bearer 401；Provider 是否已接收由[原生本地日志及输入证据](../../src/cloud/client/mailbox.rs)判定，云端不能因断线、休眠或断电自动 release。若持续等待费用超预算，可另评估 [Hibernation WebSocket](https://developers.cloudflare.com/durable-objects/best-practices/websockets/#durable-objects-hibernation-websocket-api)只发「请重新 claim」提示；冷启恢复附件时重验版本，轮换关闭旧连接，断线后仍走 HTTP 恢复。这个可选客户端变更不能冒充现有 HTTP 长等的透明休眠。

## 容量、费用和决策条件

- [DO Limits](https://developers.cloudflare.com/durable-objects/platform/limits/) 单对象是单线程，约 1,000 请求/秒是软限，不是 AgentX 吞吐承诺；同时对外连接上限 6，SQLite 每语句最多 100 绑定参数，Workers 每 isolate 内存 128 MB（[Workers limits](https://developers.cloudflare.com/workers/platform/limits/)）。确认批次若改成 `IN (...)`，100 个 ID 再带 claim 参数就会越界；保留事务内分次写。一个热门 scope 的请求、索引更新、列表与唤醒都在同一协调边界，横向扩多 scope 不会让单 scope 变快。全球入口也不会复制它的写权威；[位置提示](https://developers.cloudflare.com/durable-objects/reference/data-location/#provide-a-location-hint)只是尽力而为，按两地实际 RTT 验证。大规模 overview 要验证整页扫描和子请求预算，必要时与产品方另定投影/聚合契约。
- **文档冲突与保留期。** Limits 总表列每对象 10 GB、Free 每账户合计 5 GB；同页 [`SQLITE_FULL` 说明](https://developers.cloudflare.com/durable-objects/platform/limits/#what-happens-when-a-durable-object-exceeds-its-storage-limit)及 [FAQ](https://developers.cloudflare.com/durable-objects/reference/faq/)仍说 Paid 10 GB、Free 每对象 1 GB。Free 按较低值规划，确认目标账户限制后再定；不能假设每个对象可用满 10 GB。消息正文、索引、历史回执、旧 incarnation 和关闭的 claims 都累积，现有语义没有自动过期。以 `databaseSize` 及增长率设水位，满库前限制新消息，给确认与凭据更新留空间；直接删旧记录会影响重放、引用和恢复。DO 的 [PITR](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/#pitr-point-in-time-recovery-api)是单对象 30 天恢复；单独回滚一个 Catalog 或 Scope 可能复活旧 token、丢 claim 或使目录与实际状态分离，生产前要验证一致恢复和凭据栅栏。
- **价格量纲。** [Workers Paid](https://developers.cloudflare.com/workers/platform/pricing/#workers) 每账户最低 $5/月，入口含 1,000 万请求、3,000 万 CPU ms；超额分别 $0.30/百万请求、$0.02/百万 CPU ms。[DO 计算](https://developers.cloudflare.com/durable-objects/platform/pricing/#compute-billing)另算：含 100 万请求、40 万 GB-s/月，超额 $0.15/百万请求、$12.50/百万 GB-s（文档对超额计费单位有向上取整说明）。同对象重叠等待共享墙钟时长；不同对象分别计，按每对象 128 MB 计。Catalog 路由未命中、管理扇出和 alarm 是额外 DO 调用。[SQLite 用量](https://developers.cloudflare.com/durable-objects/platform/pricing/#sqlite-storage-backend)还按读写行数和储量：Paid 含 250 亿读、5,000 万写、5 GB-month；超出为 $0.001/百万读、$1/百万写、$0.20/GB-month。用 `rowsRead/rowsWritten` 和 `databaseSize` 记录放大，尤其看空 poll、心跳、领取和确认、索引及 overview。
- **原生空闲基线（[退避改动](idle-delivery.md)前）。** [host](../../src/cloud/client/host.rs) 的空闲 Provider 每次 claim 长等 1.5 秒，[客户端](../../src/cloud/client.rs)随后间隔约 350 ms；同一运行循环约每 12 秒心跳。忽略 RTT、执行、限流和重试，30 天一个常驻空闲 Agent 约 140 万 claim 加 21.6 万心跳；100 个这样的 Agent 约 1.62 亿次入口/Scope 调用，尚未计别的流量。同在一个长期活跃 scope 的墙钟用量约 33.18 万 GB-s/月；分在 100 个各有一个 Agent 的 scope，仅长等就约 2,690 万 GB-s/月。两种布局的调用数相近，时长开销差距很大；这些数不代表单一 scope 已能容纳 100 个长等客户端。Free 每日 10 万 DO 请求和 1.3 万 GB-s 也可能在小规模触顶。外部 SDK 能用 20 秒长等，但现有 [原生 HTTP 传输](../../src/cloud/client/http.rs)配置 8 秒全局超时；直接拉长原生等待会触发重试，还要评估该循环的心跳与 Provider 响应。

进入部署路线的条件：公开 API 回归和下述竞态零违约；最热 scope 与两地客户在目标峰值仍留有延迟余量；按真实空闲比、路由命中率和 scope 分布折算的账单可接受；有数据保留和一致恢复方案。仍待确定的是目标 Agent 数和持续在线分布、峰值/延迟和预算、Free 实际单对象上限、托管环境的冷启/断线行为，以及已有数据是否要迁移。若要求直接运行现有 Rust 二进制，或单个 scope 已逼近对象限制且业务无法拆开，应选能保留原生权威的方案。

## 进行中的 daemon 控制面：兼容性备注

[入口](../../.controller/cloud-daemon/AGENTS.md)与[控制 wire](../../.controller/cloud-daemon/wire.md)仍在实施中，不计入既有验收。若未来纳入，租约、capability、typed launch/stop 的队列/claim/receipt 应同属 `ScopeDO`，复用 generation、installation 和事务域。旧 heartbeat 只证明通信在线，不授予管理权限；约 5 秒续租、30 秒 TTL、建议 5 秒 control poll（上限 10 秒）另计并发、读写和费用。已 claim 后失去租约转 `uncertain`，不能发给替换租约再执行；原安装凭本地持久 journal 对账，Provider 和工作目录仍归宿主。单列这条迁移的租约替换/丢 poll 响应验收。

## 最小后续验证（未执行）

先用 [Workers Vitest/Miniflare](https://developers.cloudflare.com/workers/testing/vitest-integration/) 验证事务和接口，再经授权在独立测试环境用不改的 SDK/原生客户端跑一遍；本地通过不能证明托管侧重启、计费或跨地延迟。

| 场景 | 通过判据 |
| --- | --- |
| 创建/轮换故障注入 | 分别在保留后、Scope 持久写后、Catalog 回执落盘后截断响应并重试。同一 ID 同一输入最终只有一个映射、同一 generation 和原版本/URL；改内容 409；并发两次同 `expected_version` 只升一次，并发两个 scope 不能拿同一活动摘要，未发布对象不能直接通信。 |
| 轮换与等待、写入交错 | 旧 bearer 恰处于等待时轮换，应结束为 401、从不携带消息；若恰逢对象重启允许可重试的传输失败，但旧 bearer 重连必为 401。用新 bearer 查原发送 ID，结果为原消息一次或明确未写；用原 claim 恢复，已提交归属不变；随后相同短名的新 incarnation 拿不到旧邮件。覆盖两种提交顺序。 |
| 对象替换与 HTTP 契约 | 在空等期间、claim 提交后但响应前各强制一次对象重启。新请求继续从原持久队列工作；同一 claim 恢复相同未确认项，重复 confirm 成功，其他 claim 409，服务不自动重投；超过 JS 安全整数的 ID、nullable 字段、body/批次上限和关联回复与原生契约一致。 |
| 容量与账单 | 提前记录预期峰值与预算；在两地以正常空等、心跳、突发发送、目录查询和槽满同时施压，对比集中在单 scope 与分布多 scope。记录 p95/p99、503、凭据失效和发送唤醒时延、每次 SQL 读写行、DB 增长、DO/Worker 请求及 GB-s（[官方指标](https://developers.cloudflare.com/durable-objects/observability/metrics-and-analytics/)）。建议原型门槛：在目标峰值 2 倍的负载下，发送及轮换 p95 均 ≤1 秒、无写入错误或无限积压；账单按实测分布折算不得超事先约定预算。 |

来源按主张就近链接。AgentX 的行为以 [cloud-scope 入口](../../.controller/cloud-scope/AGENTS.md)及其 wire、实现和验收为准；平台文档与假设以本次只读查阅日期为界。
