# 5 美元的 Workers Paid 可以养多少个 Durable Objects？

按 2026-11-22 的公开标准价，假设订阅项目是 **Workers Paid**。每月最低 5 美元覆盖整个账号，多个项目共享内含用量。新建项目按 SQLite-backed Durable Objects 估算。仅给 Worker 配一个 DO binding（指向对象命名空间的入口）或为实体分配名字，没有按绑定或实例个数收取的固定月租。[Workers 套餐](https://developers.cloudflare.com/workers/platform/pricing/) · [DO 价目](https://developers.cloudflare.com/durable-objects/platform/pricing/)

| 在哪一层计算 | 额度或限制 | 对设计的影响 |
| --- | --- | --- |
| 账号 | 最多 500 个 DO 类；对象实例数不限；Paid 的 SQLite 总存储无固定容量上限 | 用同一类服务多个业务实体。各项目共用账单额度，数据持续保存产生 GB-month。 |
| 单个对象 | SQLite 上限 10 GB；约 1,000 请求/秒为软限，实际由工作量决定，积压后可能返回 overloaded | 观察单个最热对象的延迟和存储增长。一个账户有余量也无法让已满的对象继续增长。 |
| 单次调用 | 默认 30 秒活跃 CPU，可配置到 5 分钟 | 和等待网络消耗的墙钟时长分别计算。 |

[Cloudflare DO 限制与吞吐说明](https://developers.cloudflare.com/durable-objects/platform/limits/)

| 账号每月计费维度 | 内含用量 | 超量单价（美元） |
| --- | ---: | ---: |
| DO 请求 | 100 万次 | 0.15 / 百万次 |
| DO 活跃时长 | 40 万 GB-s | 12.50 / 百万 GB-s |
| SQLite 读行 | 250 亿行 | 0.001 / 百万行 |
| SQLite 写行 | 5,000 万行 | 1.00 / 百万行 |
| SQLite 存储 | 5 GB-month | 0.20 / GB-month |
| 入口 Worker 请求与 CPU | 1,000 万次；3,000 万 CPU-ms | 0.30 / 百万次；0.02 / 百万 CPU-ms |

普通 HTTP 经过动态 Worker 再调用一个 DO，两层分别计量；访问多个对象或多次调用 stub 会增加 DO 用量。SQL 按实际读写行计量，全表扫描会放大读取，维护索引会增加写入。设置 alarm 计一次行写入，alarm 触发另计 DO 请求。WebSocket 建连计请求，入站应用消息按 20 条折算 1 个请求，出站消息免费。普通 WebSocket 会保持时长计费，休眠 API 在符合条件的空闲阶段可免时长。[DO 价目与取整示例](https://developers.cloudflare.com/durable-objects/platform/pricing/) · [入口 Worker 价目](https://developers.cloudflare.com/workers/platform/pricing/)

## 同样是多个项目，账单会怎样变化？

下表按 30 天、独享账号内含量、美元估算；仅明确列出的维度参与比较，其他维度假设在额度内。官方示例按每个计费活跃对象 0.128 GB 计算；同一对象的并发等待按时间并集累计，不同对象各自累计。每个维度扣除内含量后按计费单位向上取整。

| 工作负载假设 | 月用量与推算 | 含订阅估算 |
| --- | --- | ---: |
| 100 个轻量项目，各有 1 个对象，各保存 1 MB，每天各处理 100 个短请求 | 合计约 0.1 GB、30 万次调用。若每次计费活跃时间为 100 ms，时长至多 3,840 GB-s | $5 |
| 1 个对象整月持续处于计费活跃状态 | 30 × 86,400 × 0.128 = 331,776 GB-s | $5 |
| 2 个对象都整月持续处于计费活跃状态 | 663,552 GB-s；超出 263,552 GB-s，按官方价目取整到 100 万 GB-s，增加 $12.50 | $17.50 |

作为[空收件退避](idle-delivery.md)改动前的比较基线，旧版直连客户端有一项固定流量：空闲 Agent 领取消息最多等待 1.5 秒，约 350 ms 后再查，心跳约每 12 秒一次。忽略网络和调度开销，单个 Agent 连续空闲 30 天约有 140 万次领取和 21.6 万次心跳。如果每次只调用一次 Scope DO，光这两项就约 162 万次 DO 请求；独享内含量时，该维度约加 $0.15。其他项目流量、目录查询、SQLite、入口 CPU 及控制租约需另计。这里是对旧直连负载的估算，未来联网 daemon 应用实际请求频率代入。[宿主实现](../../src/cloud/client/host.rs) · [完整负载假设](workers-do.md)

## 按什么维度创建？

用三个问题选边界。哪些操作争用同一项资源并且必须一起提交？哪些实体能独立休眠？哪一组数据最可能先碰到单对象 10 GB 或持续过载？协作文档通常按文档、聊天室按房间、购物车按用户或购物车 ID。可复用同一类，通过名称区分项目和实体。项目内有许多互不相关的房间时，每个房间一个对象能分摊热点。需要一起确认或排序的事实放进同一对象，可以减少跨对象往返。[对象架构指导](https://developers.cloudflare.com/durable-objects/best-practices/architecture-guidance/)

AgentX 可先用一个 Catalog 类加一个 Scope 类。每个 deployment 有一个目录对象，每个独立 scope 有一个业务对象，scope 内的 Agent、消息、claim 和回执用表行表示。100 个项目如果恰好有 100 个独立 scope，单部署只需 2 个类、101 个实例。若多个项目共享同一个通信 scope，则共享该 scope 对象。达到持续热点或存储水位时，再决定拆哪个职责，并检查要跨对象维护的唯一性、顺序和确认规则。[AgentX 候选架构](workers-do.md)

## 如何看住多项目预算

从账号的实际用量看 DO 请求、duration、读写行、GB-month，并定位流量最大的命名空间；对共享同一命名空间的项目，可按项目或 scope 记录聚合计数。常见的优先检查项是没有消息时仍在轮询、普通 WebSocket 长连、SQL 全表扫描和长期增长的历史表。可设置账号级 DO 支出限制；生效时新请求和 SQL 可能被拒绝，影响同账号其他项目。已有存储、部分在途时长和已建立的连接仍可能产生费用，订阅和入口 Worker 也有独立计费项。实际账单须结合账号其他项目用量。[DO 支出限制](https://developers.cloudflare.com/durable-objects/platform/spend-management/)
