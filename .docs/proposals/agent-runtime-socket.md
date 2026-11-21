---
schema: proposal
status: proposed
description: Give every launched Agent an exact host-owned local socket for direct diagnostics, maintenance and daemon communication
models: [../architecture/agent-runtime-socket.md, ../architecture/runtime-hosts.md, ../model/state-ownership.md]
links: [../../.controller/agent-debug/design.task.md, ../../.controller/agent-debug/contracts.rs, ../../.controller/message-delivery/repair.task.md]
---

# 每个启动 Agent 的直接运行端点

用户要求每个启动的 Agent 都有可直接寻址的 socket，用于内部状态、日志、监控与维护，并允许 daemon 通过同一 socket 与运行中的 Agent 通信。本提案建议由 AgentX 长驻运行宿主持有端点，每个准确 Agent 实例一个；Provider 和模型无需实现 socket。普通消息继续保存在 daemon 的唯一持久邮箱中。

## 建议及理由

| 选择 | 用户获得的能力 | 保留的边界 |
| --- | --- | --- |
| 宿主持有端点，CLI 与 daemon 访问同一地址 | 消息链路卡住时仍可直接查看故障阶段；daemon 可持续通知宿主 | shared app-server 不是 Agent 本体；关闭一个端点不影响其他会话 |
| 本地认证传输、完整实例与绑定校验 | 精确观察和维护目标，不误触同名后继 | Linux Unix socket 首发；其他平台必须单独证明 |
| 独立诊断处理 + 有界事件订阅 | Provider 卡住或 daemon 不可用时返回最后观察、时间与错误 | 宿主整体崩溃时无法应答；无“万能后门”承诺 |
| 维护仅提供重新扫描、Provider 只读探测、daemon 重连 | 有界结果关联、过期/未知与逐操作恢复边界的可控维护 | 不任意执行命令、不强制确认消息、不强制重启共享 Provider |
| daemon socket 通知 + 原生邮箱重读 | 持续通信更容易诊断；丢提示仍可恢复 | 通知成功、Provider 接受、消息消费、模型启动分别观察 |

[权威设计](../architecture/agent-runtime-socket.md)给出职责、实例生命周期、协议、信任、流控、断线恢复与首批实现模块。[Rust 契约草案](../../.controller/agent-debug/contracts.rs)是实现交接，不是当前生产 API。[中文交互全景](http://192.168.2.178:39731/)是主要讨论面；可离线打开[同一 HTML](../../.controller/agent-debug/preview/index.html)。讨论、浏览器观察与证据边界在 [Task](../../.controller/agent-debug/discussion.task.md)，原 Task 保留历史依据。

## 与当前消息修复的关系

Mara [诊断](../../.controller/message-delivery/diagnosis.md)已经观察到原始旧宿主与新 daemon 的协议不兼容，在读取邮箱前失败。持续 socket 不能替换协议升级或独立根因修复。本设计要求这种状态直接可见、不能伪装健康，也不能借“重连”丢弃旧 generation/claim。当前修复 r2 已完成并接受，controlled Linux Codex/Qoder 普通 send / restart / native recovery 已通过；[结果](../../.controller/message-delivery/result.md)拥有精确范围。它不是新端点实现，也没有自动升级原始宿主。本设计不再等待修复执行。

## 交付与取舍

与 nezi 讨论同意的建议顺序：S1 先交付真实 PTY 的 CLI → daemon → 同一宿主端点 → 既有 Provider adapter → 原生回执，含直接快照/安全日志/订阅、有限扫描、准确身份和故障恢复；S2 独立验证共享 Codex bridge sibling 隔离，补齐按 adapter 能力支持的只读探测与 daemon 重连；S3 完成其他受支持启动模式与选定平台，再声明“每个受支持启动 Agent”全覆盖。PTY-only 不是完整交付，默认不启用也不能充当最终覆盖策略。不能只交付 socket ping 或文档类型检查。场景 AS-01 至 AS-12 在权威设计中唯一维护，包含等待、活跃转空闲、丢响应、重启、版本变化、Provider 停滞、同名隔离及有界负载。

不新增通用控制框架、远程 bus、独立日志数据库或第二邮箱。未选 daemon 代理诊断（依赖故障路径）、Provider 本体持有 socket（身份和生命周期不匹配）、任意 shell/终端控制（效果无界）或直接削弱协议校验。旧进程不会因设计或新二进制自动具备端点。

独立探索已经与 nezi 讨论：保留准确宿主端点与唯一邮箱；精简原稿没有时钟供给的15分钟维护去重协议，保留期内复用结果，过期/缺失明确 NotRetained/Unknown，不承诺无限去重，不自动重发。扫描合并、只读探测和重连各有并发与重试边界。长连接是可重连会话建议，短调用同样可保证邮箱正确性；硬编码 heartbeat/负载常数仅为待测参数。用户仍可讨论会话偏好、同 UID 权限、首发平台和维护范围。

用户已授权独立设计与交互式表达；本 Proposal 仍为 proposed，未获得用户接受；生产实现及 live adoption 尚未授权。当前不修改生产代码、Cargo、运行 daemon、原始进程、邮箱或已接受候选。后续按权威设计的实施/回滚边界建立 Delivery；不从提案接受推断运行支持。
