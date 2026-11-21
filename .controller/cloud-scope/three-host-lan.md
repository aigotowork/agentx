# 三台机器的 HTTPS 接入实验

服务端运行在 `192.168.10.3`，Caddy 在专网 IP 的 18756 端口终止 TLS，转发至 loopback 的 18755 端口。客户端分别是 `longser`（本机）和 `xser01`（192.168.10.100），都使用系统主机名。服务和客户端运行同一候选的去符号副本。SSH 用于部署与操作，两台客户端直接通过 HTTPS 访问服务端。证书由临时 CA 签发，包含服务端 IP SAN。

| 场景 | 实际观察 |
| --- | --- |
| 本机管理远端 | CLI 创建、列出和查看 `lan`，查看主机和 Agent。TUI 显示两台在线主机。Chrome 通过证书公钥固定访问管理台，创建 `web-managed`、完成 v1 → v2 轮换，CLI 随后读到 v2。见 [TUI 快照](../../.tmp/cloud-scope/three-host-lan/tui-overview.txt) 和 [浏览器结果](../../.tmp/cloud-scope/three-host-lan/browser-result.json)。 |
| 跨机双向通信 | 本机发给 `@xser01/nova` 的请求 #2 和远端终端发起的关联回执 #3 均到达 confirmed。故意写错宿主 `@longser/nova` 得到 404。 |
| 实际模型自动回信 | 本机 `@longser/luna` 发出 #4，远端 `@xser01/nori` 的真实 Codex 模型执行发送命令，回信 #5 带有 `reply_to=4`。请求与回信均到达 confirmed。两端未使用脚本模型。见 [请求](../../.tmp/cloud-scope/three-host-lan/lan-net.json) 和 [回信](../../.tmp/cloud-scope/three-host-lan/live-model-reply.json)。 |
| 通信凭据轮换 | CLI 将活动 scope 从 v1 轮换到 v2，旧 token 返回 401，新 token 返回 200。两机更新凭据后用 `--recover` 恢复原 Agent 实例和各自原 Codex 线程；[轮换后的消息 #6](../../.tmp/cloud-scope/three-host-lan/rotated-ok.json) 达到 confirmed。 |

首次接入暴露了三个操作问题。远端原有 Codex 启动返回 139，改用实验目录的 0.171.0 二进制后成功。单独复制 Codex 主程序还缺少同版本的 `codex-code-mode-host`，导致模型收到消息后无法执行回复命令。本机第一次使用继承的 Codex 配置时 `thread/name/set` 超时，使用独立配置、沿用本机模型服务后启动成功；具体超时原因未经隔离验证。

远端工具沙箱首次发送时报告 `Operation not permitted`。允许该实验工作区联网并启动新 Agent 后，模型完成了 #5 的自动回复。部署返回的 confirmed 表示输入确已投递给目标 Provider，模型是否成功执行回复可通过独立的回复 operation 验证。
