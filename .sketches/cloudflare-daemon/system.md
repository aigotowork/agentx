# daemon 上云时，哪些状态跟着走

这里的部署单位是一个 AgentX deployment。一个 deployment 管理多个 scope，每个 scope 拥有独立的身份代次、名称空间和消息库。原生 daemon 还有自己的本地数据范围、进程身份和调用者鉴权，两种 scope 通过明确的接口协作。[部署模型](../../.controller/cloud-scope/AGENTS.md) · [本地状态所有权](../../.docs/model/state-ownership.md)

| 谁负责 | 已交付的云通信基线 | 原生 daemon 联网能力 |
| --- | --- | --- |
| 管理员与管理端 | Web、TUI、HTTP CLI 用 deployment bearer 创建 scope、轮换凭据、看主机与 Agent，统计能表达部分不可用 | 读取一台主机发布的能力，对精确安装提交或查询有类型的 launch / stop 操作。[管理契约](../../src/cloud/admin.rs) · [控制契约](../../.controller/cloud-daemon/wire.md) |
| 云端 catalog | 保存 deployment ID、scope 名录、generation、凭据版本及管理变更回执。创建的中间状态可在重启后按原请求恢复 | 继续提供部署级鉴权和管理入口。[catalog](../../src/cloud/catalog.rs) |
| 云端 scope | 保存主机安装绑定、Agent 实例、原请求收件人、信箱领取批次与确认。主机失联可保留名字和队列 | 增加每台 daemon 的独立租约、工作区和角色能力、操作及执行结果。主机有心跳与具备远程管理能力分别判断。[通信协议](../../.controller/cloud-scope/communication-wire.md) · [控制协议](../../.controller/cloud-daemon/wire.md) |
| Linux 宿主 | 本地 supervisor 启动 Codex，保管凭据、目录和 Unix socket，核对精确输入后才向云确认。当前直连路径支持 launch、send、list、listen | 本地 daemon 主动 HTTPS 出站，工具走本地 IPC；操作绑定安装、租约和实例，落盘记录后执行，执行结果不明时保留 uncertain。[直连客户端](../../src/cloud/client.rs) · [宿主设计](../../.controller/cloud-daemon/host.task.md) |
| 本地资源与其他应用 | 工作区文件、PTY、Provider 会话、进程控制都由宿主管理；云管理页面已有独立的 HTTP 接口 | 工作区按别名发布。完整 transcript、任意终端接管、Space / Inspector 的数据接入需要各自的资源接口和权限设计。[本地域模型](../../.docs/model/README.md) · [联网工作范围](../../.controller/cloud-daemon/AGENTS.md) |

## 一条消息经过哪些提交点

1. 发件人以稳定的 request ID 提交。服务端在事务里冻结当时的目标实例和输入。同 ID、同内容重试返回原操作；换内容会冲突。
2. 收件宿主用 claim ID 领取。在本地调用 Provider 前记录领取状态。Provider 输入证据一旦可核对，宿主再确认消息；仅收到启动回合的响应仍可能无法判断输入是否已经交付。
3. 响应丢失时，发件方查原 operation，收件方查原 claim。即使目标短名已重新分配，原消息仍绑定原实例。token 轮换保留数据代次与已提交记录，同时旧 token 的在途等待要重新鉴权。[协议与状态转换](../../.controller/cloud-scope/communication-wire.md) · [宿主投递逻辑](../../src/cloud/client/host.rs) · [领取和确认实现](../../src/cloud/communication/store/inbox.rs)

控制操作另有状态机。按控制契约，租约在领取前失效，排队操作失败；领取之后失联，可能已经创建了进程，云端应记录 uncertain。后继宿主必须用同一安装上持久化的精确 claim 和本地证据去对账。重新发一次 launch 可能多启动一个进程。这也是未来云后端必须独立测试控制回执的原因。[控制协议](../../.controller/cloud-daemon/wire.md)

## 现有证据能回答什么

| 材料 | 观察范围 |
| --- | --- |
| [冻结候选的验收记录](../../.controller/cloud-scope/delivery.md) | Linux 候选的云管理、通信、原生 HTTPS 宿主、SDK/CLI、TUI、浏览器，以及单机隔离网络联通 |
| [三台物理机器的实验](../../.controller/cloud-scope/three-host-lan.md) | 专网里的 Caddy TLS 入口、两台真实 Codex 客户端、自动回复、token 轮换和原会话恢复 |
| [联网原生 daemon 验收任务](../../.controller/cloud-daemon/acceptance.task.md) | 三机验证了零 Agent 主机的远程启动、精确停止及重连。Linux Codex 0.171 受限工具环境的 Unix socket 接入仍有限制。[候选交付](../../.controller/cloud-daemon/delivery.md) |
| [公开客户端契约](../../packages/agentx-client/AGENTS.md) | 管理和通信 wire 是不同后端与现有消费者互通的依据；按目标版本从原生导出物检查新增路由、字段与错误 |

本研究沿用已交付的云通信基线，并单列控制链路的云后端验证项。三个专题分别引用 Cloudflare 产品文档。公开 Cloudflare 路由、账号级可用能力、容量、账单和故障恢复均需要将来在选定环境取证。
