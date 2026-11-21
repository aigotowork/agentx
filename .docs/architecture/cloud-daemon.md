# 本地 scope daemon 接入云端

一台主机上，每个本地数据 scope 使用原有的 daemon 单实例。启用云端配置后，daemon 负责出站连接和主机管理，云端按部署及 scope 记录请求与回执。Provider 在实际拥有工作目录的机器运行。三机后台启动、停止和断线重连均已验证，工具沙箱的限制见下文。[交付记录](../../.controller/cloud-daemon/delivery.md)

## 在主机上登记一个本地 scope

服务端部署、Admin token 和通信 token 的创建步骤见[云端操作指南](cloud-scope.md)。在目标机器上以实际运行 Codex 的账户操作。以下示例将本地数据和工作区分开放置。通信 token 文件只需包含 scope token 本身。

```sh
export AGENTX_DIR="$HOME/.local/share/agentx-personal"
agentx daemon connect --scope "$AGENTX_DIR" \
  --url https://agents.example.com/scopes/personal \
  --token-file "$HOME/.config/agentx/personal.token" \
  --workspace project="$HOME/works/project"

agentx daemon status --scope "$AGENTX_DIR"
```

省略 `--workspace` 时，当前目录成为 `default` 工作区。重复指定该选项可以开放多个目录。主机默认使用系统名，可通过 `--host` 指定便于辨认的小名。登记命令会启动或重启选定 scope 的 daemon，使新配置生效；轮换通信凭据后，用新的 token 文件再次登记。配置以当前账户私有的 `cloud.toml` 保存在本地数据 scope 中。命令输出与连接状态不会显示 token。检查状态时同时看 `Daemon` 和 `Cloud`：本地 daemon 运行后，还要成功连接并发布能力，云端才会接受该主机的管理请求。

## Linux Codex 0.171 的工具通信权限

后台在目标主机启动 Codex，以及宿主把来信交给 Codex，均可在工具禁止外网时工作。模型主动调用 `agentx send` 还需要工具进程能连接本地 daemon 的 Unix socket。实测 Codex 0.171.0 的 Linux restricted 配置拒绝 AF_UNIX connect；使用 managed network proxy 并只允许这个 socket 路径时，工具进程仍无法建立该连接。在这种配置下，管理后台看到消息已投递，也可能一直收不到模型的回复。主机终端可使用同一 scope 的 `agentx send --iam NAME`。具体系统调用观测见[隔离实验](../../.tmp/cloud-daemon/codex-unix-allow-probe/result.json)与[对应版本的上游测试](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/linux-sandbox/tests/suite/managed_proxy_unix_sockets_tests.rs)。

## 随用户服务启动

需要在首次运行 CLI 前就从后台创建 Agent 的 Linux 主机，可以让 systemd 用户服务在登录时启动同一个 daemon。将下列内容放在 `~/.config/systemd/user/agentx-personal.service`，并将程序目录和 Provider 的 PATH 改为实际安装位置。

```ini
[Unit]
Description=AgentX personal scope daemon

[Service]
Type=simple
Environment=AGENTX_DIR=%h/.local/share/agentx-personal
Environment=PATH=%h/.local/bin:/usr/local/bin:/usr/bin:/bin
ExecStart=%h/.local/bin/agentx daemon serve --scope %h/.local/share/agentx-personal
Restart=on-failure
RestartSec=5
KillMode=process
UMask=0077

[Install]
WantedBy=default.target
```

`KillMode=process` 让停止或重启 daemon 时已启动的独立 Agent 宿主可以继续运行。首次使用 systemd 托管时，给登记命令加上 `--configure-only`，保存配置后用 `systemctl --user enable --now agentx-personal.service` 启动。托管期间更改配置也使用 `--configure-only`，随后由 systemd 重启该 unit。已有手工启动的 daemon 时，执行 `systemctl --user daemon-reload` 和 `systemctl --user enable agentx-personal.service`，下次开机/登录由该服务接管。需要在机器启动后、用户登录前接单时，由管理员为该账户启用 systemd linger。使用私有 CA 时，将 `SSL_CERT_FILE` 配在该服务的环境中；使用版本管理器安装 Codex 时，将对应的 bin 目录加入服务 PATH。

[云端主机管理契约](../../.controller/cloud-daemon/wire.md)记录可用工作区、操作受理、主机领取、运行结果和失联恢复的语义。

## 变更接入方式

原有直连实例已持有自己的信箱领取记录。升级时记录原 scope URL、主机名、安装 ID 和实例身份；用同一个本地数据 scope 登记。保留旧实例并观察它退出，或在原工作目录按原方式恢复它。手工操作时按完整实例身份核对，不通过删除数据库、删除信箱文件或复用 Agent 名称来强制接管。发布验收需分别检查既有直连、daemon 管理及同机两类实例共存。

停止接单前，先在后台核对是否仍有运行中或结果待确认的操作，记录请求 ID。需要回到纯本地模式时，把该 scope 的 `cloud.toml` 移到私有备份位置，在管理该 daemon 的服务入口执行重启。云端保存原 scope、消息和操作回执；旧 Agent 按自己的宿主生命周期退出。恢复接入时使用原数据目录及相应通信 token。

## 从管理机发起操作

设置 `AGENTX_ADMIN_URL` 和 `AGENTX_ADMIN_TOKEN` 后，先读取目标主机的目录。查看返回的 `online`、`generation`、`installation_id`，选择一个可用工作区和可选角色。下面的命令假定已将返回的代次和安装 ID 赋给同名 shell 变量。

```sh
agentx-http admin hosts capabilities personal --host desktop
agentx-http admin operations launch personal --host desktop \
  --generation "$generation" --installation-id "$installation_id" \
  --workspace project --name nova --title "Project assistant" \
  --request-id project-nova-1
agentx-http admin operations show personal --request-id project-nova-1
```

请求已登记后可以随时凭同一 request ID 查询。看到成功回执后，使用返回的 Agent 名和 incarnation 停止那一个实例；重复提交时保留原请求 ID 和输入。工作区的默认 Codex 不可用时，可选择主机列出的可用角色。长时间失联或执行结果待核实时，保留原回执便于恢复。

```sh
agentx-http admin operations stop personal --host desktop \
  --generation "$generation" --installation-id "$installation_id" \
  --name nova --incarnation "$incarnation" --request-id stop-project-nova-1
```
