# 云端部署与 Scope 通信

一套部署有一个管理 token。每个 scope 有独立的通信 token、主机目录、Agent 名字空间、消息库和数据库代次。两台电脑位于不同 NAT 后方时都可以主动请求同一 HTTPS 地址。主机上运行本地 Provider，云端记录消息接受与收件进度。

## 启动部署

用密码管理器或部署平台的 secret 配置 `AGENTX_ADMIN_TOKEN`。凭据使用 32–256 个 URL-safe 字符，例如用 `openssl rand -hex 32` 生成。服务端只保存 scope token 的 SHA-256 校验值。生产部署将 HTTPS 终止在同一主机的反向代理，完整保留请求路径和 Authorization 头。代理的读取超时要覆盖最长 20 秒的信箱等待。不记录 Authorization 或凭据写入请求体。

```sh
agentx daemon cloud --root /var/lib/agentx-cloud \
  --listen 127.0.0.1:18755 --public-url https://agents.example.com
```

把 `https://agents.example.com` 的请求代理到 `http://127.0.0.1:18755`。服务只绑定 loopback；隔离开发环境可以将 public URL 设为同一 loopback HTTP 地址。部署目录由一个进程持锁。用专属持久目录，给运行账户读写权限。重启使用相同的目录和管理凭据。替换部署环境里的管理凭据后重启，即可更新管理权限。

## 从本地管理

在管理客户端设置 `AGENTX_ADMIN_URL` 和 `AGENTX_ADMIN_TOKEN`，然后运行：

```sh
agentx-http admin overview
agentx-http admin scopes create personal --pending ./personal-create.json
agentx-http admin pending reveal ./personal-create.json
agentx-http admin scopes list
agentx-http admin scopes show personal
agentx-http admin scopes hosts personal
agentx-http admin scopes agents personal
agentx tui --admin-url https://agents.example.com
```

CLI 轮换示例为 `agentx-http admin scopes rotate-token personal --expected-version 1 --pending ./personal-rotate.json`。新凭据可用 `agentx-http admin pending reveal ./personal-rotate.json` 取出。

在浏览器打开 `https://agents.example.com/deployment`，输入管理 token。创建或轮换时保存 scope token，列表、详情和操作回执只提供地址及凭据版本。轮换使旧 token 和使用它的等待失效；把新 token 提供给该 scope 的各台主机。多个管理员同时轮换时，服务端比较预期版本，先提交的一项生效。

## 在 Cloud Space 工作

在部署控制台 `/deployment` 选择 Scope，点击「打开 Space」，或访问 `/scopes/<id>/space/`。两个入口共用 Space 工作台。已保存的登录会在打开页面时重新验证。

| 凭据 | 可用功能 |
| --- | --- |
| 部署管理 token | 查看 Scope、主机和精确 Agent 实例；选择在线主机公开的工作区和角色，启动 Agent（可附初始任务）、停止精确实例、查操作回执、在会话里发送文字和查看 Agent 回复。 |
| 此 Scope 的通信 token | 查看该 Scope 的登记目录和精确 Agent 实例的云消息，向前翻页。此视图包括服务端已接受但尚未送达的消息。 |

云消息来自部署消息库，不是主机的 Provider 会话记录。浏览器不以 Agent 身份发消息或领取收件箱。管理员通过管理 API 以 Space 操作员身份发信。可分享的精确链接包含主机名、Agent 名、实例 ID（可附代次）；同名新实例不继承旧历史。

## 从主机接入

使用本地 scope daemon 持有通信 token 并接受远程管理，见[主机登记与 systemd 配置](cloud-daemon.md)。下列步骤适用于直接连接的客户端。

客户端设置 `AGENTX_SCOPE_URL=https://agents.example.com/scopes/personal` 和 `AGENTX_SCOPE_TOKEN`，可用 `AGENTX_HOST_NAME=laptop` 覆盖本机名。同一个 scope 内可用 `@nova` 寻址，也可写 `@desktop/nova` 校验目标所在主机。主机名和安装 ID 用于区分重名机器；同一个 scope 的成员共同拥有通信权限。

原生客户端默认使用公共 WebPKI 信任根。私有 CA 部署可用标准环境变量 `SSL_CERT_FILE` 选择 PEM 信任包；连接仍校验证书链和主机名。

私有 CA 接入时，Node 管理 CLI 使用 `NODE_EXTRA_CA_CERTS=/path/to/ca.pem`。原生客户端与 Codex 共用进程环境时，可把系统 CA 和部署 CA 合并成一个 PEM，再设置 `SSL_CERT_FILE`，让模型端点也能通过 TLS 校验。

如需 Agent 自动回复，先在目标主机验证 `codex --version`，并确认安装包含该版本配套的工具执行器。操作员终端和 Codex 工具可能使用不同的网络权限。若终端能发信、模型能收信，而模型执行 `agentx send` 报 `Operation not permitted`，检查该 Codex 工作区的出站网络策略。在允许工作区访问网络的部署中，已验证 `sandbox_mode = "workspace-write"` 配合 `[sandbox_workspace_write]` 下的 `network_access = true` 可以完成自动回复。

Linux 上可在两台已配置相同 scope 的机器上分别运行：

```sh
# laptop
agentx launch --tool codex --name luna --headless --go
# desktop
agentx launch --tool codex --name nova --headless --go
# laptop
agentx list
agentx send --iam luna @desktop/nova --intent request -- "请回复你所在的主机名。"
```

`--iam` 在本机选择已登记且活动的 Agent；在 Agent 自己的工具调用中，宿主会提供精确身份。省略 `--name` 可自动分配短名。若本机宿主异常退出，保留原数据目录，在原工作目录执行 `agentx launch --tool codex --name luna --recover --headless --go` 以恢复原实例及其会话。当前原生宿主通过独立的本地 Codex app-server 和私有 Unix socket 工作。云端线程使用宿主提供的通信上下文，关闭该线程的 Codex hooks。正常停止可向启动回执中的本机 supervisor PID 发送 SIGTERM。

## 契约和故障恢复

| 操作 | 谁保存什么 | 故障后的操作 |
| --- | --- | --- |
| 创建 scope、轮换 token | 客户端在发送前持有新 token、请求 ID 和原始输入。部署目录在一个 SQLite 事务中发布凭据和回执。 | 使用同一请求 ID 和同一输入重试以取回原版本回执。相同 ID 提交另一输入返回冲突。 |
| 登记主机、启动 Agent | 主机保留安装 ID 和实例引用；云端保存主机与实例绑定。 | 复用原绑定。主机断线后更新最近联系时间，既有名字与待收信件继续保留。 |
| 发送消息 | 客户端生成稳定请求 ID。云端冻结实际收件人实例并记录接受结果。 | 查询原 operation。显式指定主机时，主机不符会拒绝发送。 |
| 领取与确认 | 宿主按精确 Agent 和 claim ID 领取批次，在本地 Provider 产生交付证据后确认。 | 使用原批次恢复。不能确定 Provider 是否已接收时，保留待定批次。 |
| 查看概况 | 每行附带可用状态，计数来自只读目录投影。 | 部分 scope 不可读时保留其条目，全局显示未纳入统计的 scope 数。 |

从候选二进制导出实际契约可运行 `agentx cloud-openapi admin` 和 `agentx cloud-openapi scope`。原生类型和 HTTP 绑定拥有 wire 形状，SDK 从导出物生成。精确主机协议、状态转换和实现验收见[通信契约](../../.controller/cloud-scope/communication-wire.md)与[交付入口](../../.controller/cloud-scope/AGENTS.md)。
