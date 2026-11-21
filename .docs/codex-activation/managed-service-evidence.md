---
schema: document
status: active
owner: sake
last_updated: 2026-11-07
description: Bounded real managed Codex 0.167.0 daemon, transport, generation and thread-hook evidence.
links: [./topology.md, ./session-environment.md, ../../.controller/codex-shared-runtime/tasks/native-service.md, ./.tools/probe_managed_service.py]
---

# 原生托管服务证据

本记录只说明真实原生运行时的已观察事实。生产连接器、协议桥、数据库事务和未修改 Paseo 的组合验收由 [独立集成](./acceptance.md) 负责。运行的是本机官方 Codex `0.167.0`，完整平台包位于 `/home/long/.nvm/versions/node/v24.14.1/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl`。官方 [App Server 文档](https://developers.openai.com/codex/app-server/) 已通过 `proxy curl` 实际获取，支持按安装版本生成 schema、逐连接 initialize、JSONL 与 WebSocket 的区分；本页 managed daemon 的细节依据安装版本的 CLI、声明和以下隔离实验。

## 资源与隔离证明

先执行只运行 `id` 的 bubblewrap 能力探针，成功后才执行 daemon management。所有实际管理命令运行于新的 mount、PID、network、IPC、UTS、user namespace；系统根目录只读，`/home`、`/run` 和 `/tmp` 被空的私有 tmpfs 覆盖。只有实验目录以可写方式绑定到 `/mnt`，完整平台包只读绑定到 `/opt`，原有协议探针工具只读绑定到 `/tmp/tools`。新的 network namespace 只能访问自身固定响应的 loopback fixture。

实验进程清空原始环境，再显式设置临时 HOME、CODEX_HOME、XDG_CONFIG_HOME、XDG_RUNTIME_DIR、XDG_CACHE_HOME、PATH 和一个不存在的 D-Bus socket。没有用户凭证，没有用户 daemon PID 或用户 service bus 可供操作。每次都在 `finally` 内执行私有 `daemon stop`；namespace 初始进程结束还会回收该 PID namespace 的残留进程。没有执行 bootstrap、update、restart、systemctl 或修改安装中的可执行文件。

将单独的 Codex 二进制挂载为 `/tmp/codex` 会使 `daemon start` 报错 `this CLI has no complete local package`。改为只读挂载包含 `codex-package.json`、`bin`、`codex-path` 与 `codex-resources` 的完整平台包后，原生 daemon 可以把自身完整包安装到临时 CODEX_HOME。这个安装是原生首次启动行为，不是替换用户安装。

## 结果与限制

| 问题 | 实际观察 | 证据 |
| --- | --- | --- |
| 并发首次启动 | 两个调用分别返回 `started` 和 `alreadyRunning`，都选择同一 `socketPath`、版本和 `pid` backend | `summary.json`、`connection-summary.json` |
| 实际控制端点 | `$CODEX_HOME/app-server-control/app-server-control.sock`；两个独立 Unix WebSocket 连接都 initialize 成功，返回同一 codexHome | `connection-summary.json` |
| `proxy` 的边界 | `proxy --sock` 收到裸 JSONL initialize 后未返回协议响应；直接 WebSocket handshake 后成功 | `jsonl-proxy-summary.json` 与成功连接回执；不把未响应升级为所有 proxy 输入都失败 |
| 后端运行代次 | `app-server-daemon/daemon.pid` 包含 pid、processStartTime、processIdentity.bootId/startTicks；关闭另一连接前后记录保持相同 | `connection-summary.json` |
| startup flags | `daemon start -c model="service-override"` 的后端 cmdline 不包含覆盖项，`config/read` 仍返回文件中的 `fixture-model` | `connection-summary.json` |
| 多客户端共享原生线程 | B 能读取 A 创建的线程；`includeTurns:false`、resume/fork 的 `excludeTurns:true` 避免本版本 paginated history 的 `list_turns is not supported yet` | 成功回执与保留失败尝试 |
| 连接退出隔离 | A 的空闲连接关闭后，B 自己线程上已发起的固定响应轮次完成；daemon generation 不变，B 随后又完成一轮 | `connection-summary.json`，两次 loopback 模型请求 |
| 创建者退出的自身线程 | 创建者关闭后，没有观察到其原活跃轮次的 `turn/completed`；另一个连接观察到 idle | `owner-disconnect-summary.json`；不能承诺原生服务长驻就意味着该线程的原轮次继续 |
| 无 process hook flags 的托管 daemon | thread/start.config 中的 `features.hooks`、hook 声明、`hooks.state.<key>.trusted_hash` 使 UserPromptSubmit 实际执行 | `thread-hook-summary.json`，一轮完成、一次 hook |
| start/resume/fork 的 thread hook overlays | 三个操作均传入相同 hook/trust config，三次固定响应轮次完成，hook session_id 分别匹配原根线程或新 fork | `thread-hook-lifecycle-summary.json`，三轮完成、三次 hook |
| 服务身份环境 | 原生管理命令收到的 AGENTX_CODEX_SHARED、服务 AGENTX_PROCESS_ID、CODEX_HOME 和 AGENTX_DIR 被真实 hook 保留；INSTANCE_NAME 为空 | 同上 `threadHooks[*].serviceEnv` |

Hook 的信任 key/currentHash 由同一隔离环境中短暂运行的原生 stdio preflight `hooks/list` 返回，来源是 `/<session-flags>/config.toml`。随后 preflight 退出，正式 managed daemon 没有任何 hook CLI flags，线程 config 单独提供相同声明和信任状态。实验使用固定的 `/usr/bin/python3 /mnt/thread-hook.py`，不是伪造 hash 或改写用户 trust 文件。仅验证了 UserPromptSubmit 这一 hook 事件；不能据此宣称所有 hook 类型、任意管理策略或其他原生版本均兼容。

## 真实原生 shell 继承与兼容故障

独立集成的真实父子用例在父线程正确执行工具后，子 Agent 被服务兼容回执拒绝，原始失败是 `production-mail/csr-m1t63jfz/child-trace/child.stderr`。此前的错误把环境、配置和代次合成同一段文字，不能单凭错误确定变化字段。随后独立的 `production-environment/csr-t6cmjnbd/` 固定 fixture 在真实原生工具中保留 `fixture-environment.json` 与 `shell-environment-{A1,B,A2}.json`，这些快照来自清空环境的临时 namespace，不包含用户环境或凭证。

与初始 fixture 相比，原生工具加入 CODEX 工具调用/版本/CI/补丁标记、pager/terminal/shell 信息和冗余 LC_CTYPE，并把当前托管包的 `codex-path` 精确前置于原有 PATH。A/B 之间的差异还包含该环境用例明确设置的 `ACCEPTANCE_ROOT`；原始父子用例未设置它，产品代码没有特判这个测试变量。服务修正据此只消除明确的原生来源信息，保留实际用户 PATH 和未知变量冲突，并通过配置依赖退出规则保护被 env_key/env_vars/env_http_headers/env 引用的名称。独立完整父子复验与生产支持状态以 [acceptance](./acceptance.md) 为准。

## 复现与保留

可复用的 [隔离探针](./.tools/probe_managed_service.py) 将同一 bubblewrap 准备合并为一个入口，接受完整平台包和全新输出目录。其代码基于已执行的原始探针整理；最终可复用入口本身的执行由集成负责人负责。原始执行的脚本、逐事件 trace、stderr 和完整 JSON 回执保留在 `.agentx/.tmp/deliveries/codex-shared-runtime/native-service-probe/`，native-service Task 链接本记录。失败尝试与成功尝试分开保留，不将失败的 raw-proxy 或创建者断连案例改写成成功。

```sh
python3 .docs/codex-activation/.tools/probe_managed_service.py --package /absolute/path/to/vendor/x86_64-unknown-linux-musl --out /absolute/path/to/a-new-disposable-directory
```

这个入口不会连接生产 AgentX 数据库，也不会构建或执行生产连接器。它为服务端点、受管理 lifecycle 和线程配置事实提供可重复依据；完整生产接受范围仍以 acceptance 记录为准。
