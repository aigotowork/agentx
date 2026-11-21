---
scope: Cloudflare Containers 承载 AgentX cloud-scope
status: 文档研究完成，候选方案待验证
owner: milezo
related: ["[研究任务](tasks/containers.task.md)", "[Cloud scope 交付](../../.controller/cloud-scope/delivery.md)"]
last_updated: 2026-11-22
---

# Cloudflare Containers：Rust 能运行，状态必须另有归属

**结论。** Cloudflare Containers 能承载现有 Linux Rust HTTP 服务，现有客户端、管理界面和两套 HTTP 契约可保留。原样打包只适合可丢弃的演示：容器即使使用同一个 Durable Object（DO）地址，停止后也会从新磁盘启动。若要求在容器被杀、休眠、换位置或换镜像后，已提交的回执和状态都不倒退，首选候选是 **固定 DO 路由 + Rust 容器 + 支持事务的外部主库**。若必须同时保留原 SQLite 文件实现和现有提交路径，本路线不满足条件。[Cloudflare 容器生命周期](https://developers.cloudflare.com/containers/concepts/architecture/#use-snapshots)

## 已验收的 AgentX 基线

现有 [Cloud scope 交付](../../.controller/cloud-scope/delivery.md) 验证了隔离网络之间的原生客户端、SDK、TUI 和 Web 通过 HTTPS 使用同一部署；证据来自 Linux 和隔离 Docker 网络，并不包含 Cloudflare 或公网上线。[`agentx daemon cloud`](../../src/cloud/mod.rs#L157) 只接受 loopback `--listen`（默认 `127.0.0.1:18755`）；`--public-url` 是生成 scope URL 和回执的外部 HTTPS origin，不是监听地址。Rust 同时提供管理 API、scope API 和内嵌管理页，并验证各自的 Bearer。[路由与鉴权](../../src/cloud/mod.rs#L107)；[通信 wire](../../.controller/cloud-scope/communication-wire.md)

| 权威状态 | 当前源码及必须保留的语义 |
| --- | --- |
| 整个部署 | `deployment.sqlite3` 保存部署 ID、scope generation、token 哈希及版本，以及创建/轮换的幂等回执。先初始化 scope 目录与库，后在目录库内一次提交 scope 记录和回执；同请求能重放原回执。目录库启用 WAL，没有在此路径显式设置 `synchronous`。[打开与锁](../../src/cloud/catalog.rs#L37)；[创建与轮换](../../src/cloud/catalog.rs#L98) |
| 每个 scope | `communication.sqlite3` 保存主机安装身份、Agent incarnation、消息和 claim。写连接显式使用 `synchronous=FULL`，库使用 WAL；状态变更先提交，再唤醒或返回。消息按原 `request_id` 和输入重放，收件方已固定为原 incarnation；相同 ID 改写输入冲突。未确认 claim 不因超时自动归队，confirm/recover 继续用原 claim，确认以接收端的 Provider 证据为准。[事务](../../src/cloud/communication/store.rs#L53)；[发送与操作查询](../../src/cloud/communication/store.rs#L301)；[邮箱状态](../../src/cloud/communication/store/inbox.rs) |
| 进程内与主机本地 | `deployment.lock` 约束共享真实目录的进程；目录锁、异步唤醒和并发限额都不会跨独立容器磁盘。scope generation 是数据身份，installation/incarnation 是参与者身份，都不是服务写者 epoch。宿主机上的 Provider、凭据、终端及交付日志仍由本地主机负责。[目录锁](../../src/cloud/catalog.rs#L37)；[异步等待](../../src/cloud/communication.rs#L234)；[本地 claim 日志](../../src/cloud/client/mailbox.rs) |

这里的 **已提交写入** 指服务的权威事务已提交，即使 HTTP 响应没有送达；**RPO=0** 指在明确选定的故障范围内，一个成功响应之后，该状态和回执必须仍可找回，已提交但丢响应的请求也只能重放为同一个结果。HTTP 超时、502 或轮换竞态中的 401 不能证明事务失败。发送已提交也不等于 Provider 已接收；这仍由原 claim 和主机证据判定。

## 入口、绑定与生命周期

`客户端 / 管理页 → 自定义域 HTTPS Worker → 固定 deployment DO → 私有容器端口 → 容器内代理 → loopback Rust`。用每个部署固定的 DO 名称，所有 scope、管理 API 和静态资源走同一身份，滚动版本不改这个名字。Worker 终止公网 TLS；DO 通过 `ctx.container.getTcpPort(port).fetch(...)` 转发，公网无需 Spectrum、额外 TCP 入口或映射容器端口。转发时保留 URI 路径、查询、方法、请求体、单一 Authorization 和源响应语义；内部换主机名不改公开 origin。API 全程绕过缓存，边缘不自动重发写请求，也不在日志记 Bearer 或消息体。[自定义域](https://developers.cloudflare.com/workers/configuration/routing/custom-domains/)；[官方 HTTP 转发示例](https://developers.cloudflare.com/containers/get-started/#worker-code)；[端口 API](https://developers.cloudflare.com/containers/api/durable-object-container/#gettcpport)

候选保留 Rust 的 `127.0.0.1:18755`，在同一 `linux/amd64` 容器让薄代理监听私有接口（候选 `0.0.0.0:8080`）再转 loopback。官方示例是独立端口的 HTTP 服务；尚无证据说明代理可直达 Rust 的容器内 loopback，所以不把仅改 Wrangler 端口当成可用方案。备选是让 Rust 新增明确用于容器私网的绑定模式，默认仍拒绝非 loopback。镜像入口管理两个进程和退出信号；将管理密钥作为运行时 secret 注入。仅在 Rust 与权威状态已就绪、当代写者已获准后接入流量；`running=true` 本身没有就绪含义，探测端点应仅供内部使用。[运行时及就绪](https://developers.cloudflare.com/containers/concepts/architecture/#container-instance-lifecycle)；[环境变量与 secrets](https://developers.cloudflare.com/containers/examples/env-vars-and-secrets/)

当前官方推荐选择 `durable_object` 调度（公开 beta），旧 `default` 已弃用；这个策略下镜像切换由 DO 明确停止并重新启动，不能假定 Worker 与容器同时升级。冷启动常见 1–3 秒是文档参考值，不含 AgentX 状态装载，也不是 SLA。空闲可回收，重启可换位置，通用生命周期文档提示 DO 与容器未必同地；地域文档对 `durable_object` 调度另给出按 DO jurisdiction 放置的说明。此处不承诺两者物理同地或数据库往返延迟。DO 重启后要重设 inactivity timeout；有序停机发 SIGTERM，最多等 15 分钟再 SIGKILL，突然退出必须视作随时可能发生。排空有助于缩短中断，不能作为持久化屏障。[调度策略](https://developers.cloudflare.com/containers/configuration/scheduling-policy/)；[部署与停机](https://developers.cloudflare.com/containers/concepts/architecture/)；[空闲计时器](https://developers.cloudflare.com/containers/api/durable-object-container/#setinactivitytimeout)

scope claim 协议允许最长 20 秒长轮询；进入等待前后不持数据库事务。当前原生直连路径是 [1.5 秒 claim、12 秒心跳](../../src/cloud/client/host.rs#L35)，[HTTP 总超时 8 秒](../../src/cloud/client/http.rs#L45)。Workers 对仍连接的 HTTP 请求没有固定墙钟上限，但客户端断开、冷启动和重启仍可截断请求。DO 不应把整个长轮询和所有发送串在一个互斥区里；连接断开后客户端以相同 ID 查询或恢复。当心跳或 claim 连续到达，容器可能整月近似常驻，不能按只在发送时计费。[Workers 时限](https://developers.cloudflare.com/workers/platform/limits/#duration)；[AgentX 等待实现](../../src/cloud/communication.rs#L234)

## 选项与提交边界

| 状态方案 | 可复用的部分、代价与结论 |
| --- | --- |
| 容器本地盘；加定期/停机快照 | Rust 几乎原样。进程仍在且磁盘未换时可正常写；跨容器重建会丢盘。快照是不可变的全文件系统时点副本，只能恢复到同一镜像版本；公开 beta，20 GB、30 天 TTL（恢复刷新）。距上次成功快照之后的写入可能回退，在线 SQLite 一致切点也需另证。快照可供备份或演示复位，不做提交点。[快照指南](https://developers.cloudflare.com/containers/guides/snapshots/)；[限制](https://developers.cloudflare.com/containers/platform/limits/#snapshot-limits) |
| R2 FUSE 直接挂载 SQLite | 不采用。官方例子默认只读，写入说明要求等待写入并干净卸载。挂载能读写文件不代表支持 SQLite WAL 所依赖的同机共享内存、锁和同步，也没有跨容器的 `deployment.lock` 保证。R2 的对象一致性不自动变成目录、数据库页和 WAL 的事务。[R2 FUSE 官方例子](https://developers.cloudflare.com/containers/examples/r2-fuse-mount/)；[SQLite WAL 限制](https://www.sqlite.org/wal.html) |
| 本地 SQLite + R2 不可变提交包 + DO 原子清单 | 可保留较多 Rust/SQLite SQL，适合小数据低写入量的备选。短事务要串行经过持久提交闸：生成包含目录库、相关 scope 库和 generation 的一致版本（用 SQLite 安全备份或冻结后检查点，不能复制活跃的主 DB 文件了事）；上传并核验不可变对象后，由 DO 在一个存储事务里检查写者 epoch、预期前一版本，再推进清单。提交闸须置于本地事务结束与任何可见副作用之间，不能只在 HTTP 代理处延迟 2xx；只有清单持久化后才能放行读结果、唤醒和写响应。上传或推进清单失败且结局未决时，冻结服务直至按权威清单对账；长轮询单次数据库操作离开闸门再等待。DO 清单是唯一恢复来源；上传成功但清单未推进的对象没有被提交，清单应答丢失则先读取权威版本和原回执。也可用 R2 单 key 原子条件写表达清单，但必须让 epoch 和版本共享同一 CAS、没有旁路；只靠 CAS 写备份对象无效。每次状态变更的打包、上传和恢复都增加延迟与写放大；未测吞吐及卷增长，优先级低于事务库。[R2 对象一致性与条件写](https://developers.cloudflare.com/r2/reference/consistency/#conditional-writes)；[DO 事务及输出闸](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/#transactionsync) |
| 外部事务主库，如 PostgreSQL | **主候选。** 保留 Rust 的路由、鉴权接口、DTO、消息与 claim 规则、原生客户端及页面；移植目录库、scope 库 SQL 和事务边界，并迁移历史。应用改动比打包大，但避免每次传整个数据库，提交证据直接来自主库。数据库托管、网络、同步副本、备份和故障转移是额外成本。不要假定 Wrangler 绑定会让裸 Rust 进程获得 PostgreSQL 套接字：需验证受限的 TLS 连接，或经容器 HTTP 出站拦截调用一个受控事务网关；后者增加一跳。文档只承诺禁用公网时拦截 HTTP(S)，不能这样透明转发 PostgreSQL TCP。[容器出站](https://developers.cloudflare.com/containers/configuration/outbound-traffic/)；[PostgreSQL WAL 同步确认](https://www.postgresql.org/docs/current/runtime-config-wal.html#GUC-SYNCHRONOUS-COMMIT) |
| DO SQLite / D1 作为远程权威库 | 云内替代：用容器出站 HTTP 经 Worker 绑定执行整个域操作。DO 可用单对象事务和输出闸，D1 支持原子 `batch()`；两者都不会替 Rust 暴露原 SQLite 文件或任意跨 HTTP 调用的事务。要重划数据与事务边界，尤其是 scope 初始化、目录可见性和凭据轮换；成本由 DO/数据库行操作与额外 RPC 决定。若接受大幅迁移域事务，应与纯 Workers/DO 方案比较，不默认保留容器。[容器访问绑定](https://developers.cloudflare.com/containers/configuration/workers-connections/)；[DO SQLite](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/)；[D1 batch](https://developers.cloudflare.com/d1/worker-api/d1-database/#batch) |

### 主候选如何防双写

DO 的稳定身份负责路由和启动。主库按部署持久保存一个递增 **writer epoch 和绑定实例**；只有控制方可以切换，新 Rust 进程持有与当代实例绑定的授权，旧实例读到 epoch 数字也不能换证。每次业务写入（包括心跳、scope 创建/轮换、claim、确认和释放）都先在同一主库事务内锁住并校验该行，再提交业务记录和对应回执。写者切换独占更新同一行，与已在事务中的旧写排序：旧写先完成并对新实例可见，或在切换后被拒绝。可用 PostgreSQL 行锁实现：业务事务 `FOR SHARE`，接管方更新同一行；禁止任何写入绕过校验。凭据校验需读权威版本，状态读取走同一权威主库，旧实例不得从本地缓存继续提供旧回执。仅依靠关闭旧路由、进程文件锁、租约 TTL 或检查 scope generation 都会留下旧进程迟到提交的窗口。[DO 唯一性及异步交错](https://developers.cloudflare.com/durable-objects/best-practices/rules-of-durable-objects/#understand-how-input-and-output-gates-work)；[PostgreSQL 行锁](https://www.postgresql.org/docs/current/explicit-locking.html#LOCKING-ROWS)

切换顺序是停止接纳旧实例请求、在主库推进 epoch、待新实例从同一状态就绪再转流量。通知只作加速：发送、释放与轮换后可以唤醒等待，但重启丢通知时要重新查权威状态。发送和管理重试使用原请求 ID 与输入；claim 遗失响应沿用原 claim，确认后不重交 Provider。迁移要保住部署 ID、scope generation、原 origin/历史回执、凭据版本、Agent incarnation、消息 ID 序列和未完成 claim；数据库不可用、历史缺失或无法确认 epoch 时返回暂不可用，不能以空库或旧备份接流量。数据库跨节点或跨区故障也纳入 RPO=0 时，必须证明确认的 WAL 已同步到允许接管的副本，同时 fencing 旧主库；默认或异步副本恢复不足以承诺这一点。回退镜像仍用最新权威历史、发新 epoch；存在不兼容 schema 时须先有可行的回退迁移。

并行开发的 [native host 控制面](../../.controller/cloud-daemon/AGENTS.md) 仍在实施。若以后纳入，按 [control wire](../../.controller/cloud-daemon/wire.md) 一起保留同 scope 的能力、lease、操作/claim/结果和原回执；旧直连心跳不授予管理权限。失租前未领任务与已领任务分别按原规则处理为失败和不确定；云端持久状态不能替代主机本地执行日志，也不能让补跑重复执行 launch/stop。该补充不表示已验收或授权上线。

## 成本、决策与最小后续验证

按 [官方容器价目](https://developers.cloudflare.com/containers/platform/pricing/) 粗算，假设一个 30 天、720 小时常驻实例且独享套餐额度：Workers Paid 底价 $5/月；`lite`（256 MiB、2 GB）再加约 $1.71/月内存和盘，CPU 按实际使用，满额再加约 $2.79；`basic`（1 GiB、4 GB）的对应开销约 $6.93 和最多约 $12.51。对比每月只运行 60 小时的 `lite`，即使运行期间吃满 vCPU，单容器各项仍在包含额度内。以上未计 Worker 请求/CPU、[DO 请求/持续时间/存储](https://developers.cloudflare.com/durable-objects/platform/pricing/)、日志、超额区域出网和数据库；R2 路线还按 [R2 容量及请求](https://developers.cloudflare.com/r2/pricing/) 计费，主要风险是频繁小写导致的全量数据上传和等待。首轮以实测启动、内存峰值、轮询下的常驻比例与写入延迟选规格；没有选定数据库和同步副本拓扑前，不宜承诺总价。

**决策条件。** 若业务要 Cloudflare 运行 Rust、接受改存储层和 beta 调度，优先验证外部主库路线；云内存储是硬条件时，再看接受域事务重写，还是小规模下接受 R2 提交闸的延迟。若当前目录及 SQLite 代码必须原样保留，应选有已验证持久 POSIX 存储与单写者的其他运行环境。待确认账号的 beta/地域能力、数据库连通性和故障域、规格与请求量；[容器地域配置](https://developers.cloudflare.com/containers/concepts/placement/)说明所选调度按 DO jurisdiction 约束容器；外部主库、镜像和快照的位置仍需分别核实。

**未来最小验证（另行授权，隔离数据）。** 建一个部署、一个 scope、两台测试 host；用原生客户端和 SDK 消费真实 HTTP，故障由测试控制面注入。基线用现有 [验收矩阵](../../tests/cloud_scope/README.md)。本次未创建任何资源或做运行态探测。

| 验证 | 可观察的通过条件 |
| --- | --- |
| HTTPS 与等待 | 冷启动和镜像切换后同一公开 URL 可达，管理页、带查询的 API、两类 Bearer 状态码都与基线一致；20 秒空 claim 能返回、等待中发送能唤醒，轮换让旧等待失效且新凭据可查原回执。另记冷启动耗时，核对原生 8 秒请求预算。 |
| 提交前/后断点 | 记录 scope 创建、token 轮换、send、claim、confirm 的结果；分别在事务未提交、提交后丢弃 HTTP 响应和客户端收到成功以后杀容器。让新磁盘/新镜像重启，查回同一部署 ID、generation、凭据版本和已提交回执；原请求同 ID 同输入重放，改输入 409；明确未提交的尝试在重试前不应出现，不能把尚未结算的超时直接当作未提交。已确认的消息不能回来，未完成 claim 仍归同一 incarnation。任何一条成功写消失或同 ID 产生两份状态即失败。 |
| 交接及分区 | 暂停旧实例在写入边界，令新实例接管；分别让旧写先完成和在 epoch 递增后恢复，检查主库审计里的提交顺序与拒绝结果。阻断存储或换到宣称同步的副本，核对没有空库成功响应、没有旧 epoch 在接管之后提交、没有已确认写回退；记录恢复用时和拒写窗口，再决定可接受的故障域。若选 R2 路线，额外在上传完成但清单未推进、清单推进但回包丢失两个边界杀实例。 |

资料核对日期：2026-11-22。平台事实来自上文逐项链接的 Cloudflare 公开文档；源码事实及既有验证范围以 [Cloud scope 入口](../../.controller/cloud-scope/AGENTS.md) 和交付记录为准。拓扑、提交闸、epoch 与容量估算属于候选设计。
