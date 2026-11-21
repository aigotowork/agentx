---
scope: 隔离进程运行证据与局限
status: 已执行
last_updated: 2026-11-22
---

# 实验如何区分传输和唤醒

以下均在 Linux 临时目录运行，模型接口是仅监听环回地址并返回固定 SSE 的夹具。使用真实 Codex CLI **0.171.0**、`@agentclientprotocol/codex-acp@2.9.0`、`opencode-ai@1.19.6`，使用临时 HOME、配置、工作目录和数据路径，不读现有会话或凭据，不调用付费模型。没有部署到 AgentX。

## Codex stdio MCP

1. 启动一个原生 App Server。按 `thread/start` 创建 A、B，并各执行一次基线轮次。两个基线的模型请求都包含 `probe_echo` 工具，进程探针看到的是**同一个已初始化的 stdio MCP 子进程**。这个结果已经足以否定「每一个 session 都有专属 MCP 进程」的假设；不推出所有配置总会复用。
2. 通过该子进程独立的 Unix socket，依次送入 `notifications/tools/list_changed` 与带唯一文本的自定义通知。分别检查写入确认，并在每次写出后观察 0.8 秒：没有 `turn/started`，也没有模型请求。“写入确认”来自探针 server，不能证明 Codex 已处理通知。
3. 对 A 显式调用 `turn/start`，看到该线程开始并完成一轮。固定模型收到显式输入，未收到自定义通知的唯一文本。
4. 独立调用 `mcpServer/event/stream/start` 指向该自建 server，返回 `MCP event subscriptions are only supported for hosted apps`；与 [0.171.0 源码](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/app-server/src/request_processors/mcp_event_stream.rs)一致。

基线使用旧版 MCP 握手；普通通知的观察不代表协商了官方 Events 扩展，也不代表必然能送进 Claude/OpenCode 的其他原生会话。

## ACP v1

同一个实验客户端只换启动命令，向 Codex ACP 与 OpenCode ACP 都执行同一组方法。握手均返回协议版本 1，均声明 `loadSession`。

| 验证动作 | 两个工具都观察到 |
| --- | --- |
| 一条连接创建 A、B，分别提交带标记输入；A 空闲后再收到一轮 | A 的请求保留 A 的第一条历史，没有 B 的标记；B 不包含 A；两侧收到对应 session ID 的输出和 `end_turn` |
| 固定模型让 A 繁忙，客户端暂存下一条消息 | 暂存期间不发第二个 RPC；收到第一轮的响应后才发，A 正常完成。证明的是**客户端排队策略**，没有探测宿主会怎样处理并发 v1 prompt |
| 干净结束 ACP 进程，保留临时数据，重启并 `session/load` | 原 A 可继续；模型请求保留此前投递的标记 |
| 将同一业务事件文本显式提交两遍 | 两次都完成，模型历史中有两条输入。v1 JSON-RPC request ID 不是业务幂等键；此步骤不模拟半途中断 |

## ACP v2 Preview

仅在 Codex ACP 2.9.0 上执行。按 v2 用 `info` 和 `capabilities` 握手，建会话，提交含 `idempotencyKey` 的 prompt，并由夹具阻塞模型输出。**前台模型未完成就收到 `messageId`**；同一连接在忙时重试原键，返回同一 ID。等会话报告 `idle` 后干净退出，重启并 `session/resume`，再提交同一键仍返回同一 ID；短观察窗口没有新模型请求。未模拟崩溃、持久化前中断、键过期，也未测试 OpenCode 的 v2 Preview。

## 复跑

需 Linux、Python 3、Node/npm，以及本地 Codex CLI 0.171.0。下面的 npm 安装只写临时目录；OpenCode 实验用的是此处 npm pin，不用 Registry 中可能不同的发行版本。运行时夹具与终端复用仓库的[原生协议探针](../../.docs/codex-activation/.tools/probe_native_protocol.py)，不调用生产 AgentX。

```bash
lab_dir=$(mktemp -d /tmp/agent-session-lab.XXXXXX)
npm install --prefix "$lab_dir" --no-audit --no-fund --no-package-lock \
  @agentclientprotocol/codex-acp@2.9.0 opencode-ai@1.19.6
python3 .sketches/agent-session-push/probe.py \
  --codex "$(command -v codex)" \
  --codex-acp "$lab_dir/node_modules/.bin/codex-acp" \
  --opencode "$lab_dir/node_modules/.bin/opencode" --codex-acp-v2
```

每个通过的场景输出一行 JSON；失败会非零退出。首行 `artifacts=` 指向生成的 `trace.jsonl`、`mcp.jsonl` 和子进程日志，本轮 [结果摘录](results.json) 只保留版本和通过项。可以省略其他适配器参数来复跑单独的原生 MCP 探针，或用 `--only-v2 --codex-acp …` 只运行 v2。

打开 [index.html](index.html) 可离线使用图谱；本机开发预览可从仓库根目录运行 `python3 -m http.server 18996 --bind 127.0.0.1 --directory .sketches/agent-session-push`，访问 `http://127.0.0.1:18996/`。预览只发布研究目录；仓库里其他目录的相对引用用本地文件打开。

## 尚未验证

- 真实 AgentX 持久信箱接 ACP、丢失回执后的核对、Agent 代次替换时不串信，以及真实模型的权限/工具流程。
- 任何运行中但没有接入可控后端的旧 PTY 会话能否被其他工具可靠接管。
- MCP Events 官方流或 webhook 在不同产品之间的端到端组合，以及 Claude 专属 Channels 在本项目环境下的可用性。
- Codex fork、用户所用其他工具和线上服务均未改动或实测。
