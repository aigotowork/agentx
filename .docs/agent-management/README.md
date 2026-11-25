---
schema: document
description: Start and use the daemon-owned Agent management Web console and HTTP API.
status: active
owner: agent-management
last_updated: 2026-11-15
links: [./design.md, ../../.controller/agent-management/delivery.md]
---

# Agent Web 管理台

在浏览器中管理 Agent、发送消息、查看通信和模型对话记录，并停止或恢复运行。管理台由所选数据目录的 daemon 提供，关闭网页不影响 Agent 运行。它与终端管理台复用原生能力；页面显示结构化数据，不传输终端画面。

## 启动

```sh
agentx daemon web --project /absolute/project
```

命令会启用当前作用域的管理服务、启动缺失的 daemon，并打印浏览器地址。默认监听 `127.0.0.1`，使用自动分配的端口。选择独立数据目录和固定端口：

```sh
agentx daemon web --scope /absolute/agentx-data --project /absolute/project --port 18749
agentx daemon status --scope /absolute/agentx-data
```

需要从局域网访问时添加 `--host 0.0.0.0`，使用本机局域网 IP 和打印出的端口打开页面。该模式向能访问此端口的客户端提供管理能力；当前没有账户登录。默认本机模式不需要此参数。

设置保存在该数据目录的 `config.toml`，正在运行的新版 daemon 会加载变更：

```toml
[management]
enabled = true
host = "127.0.0.1"
port = 18749
project = "/absolute/project"
```

将 `enabled` 改为 `false` 可关闭 HTTP 入口，已启动的 Agents 保持运行。如果当前 daemon 是旧版，启用命令会报告入口未就绪；升级后使用现有的 `agentx daemon restart --scope /absolute/agentx-data` 显式切换。不会为启用页面而自动替换已有 daemon。

源码构建与当前验收范围见 [Delivery](../../.controller/agent-management/delivery.md)。已验收的基础管理台候选为 `.bin/agentx-management-r4`，已验收的完整办公室扩展构建为 `.bin/agentx-office-r3`（[验收范围](../../.controller/office-management/delivery.md#accepted-local-outcome)）；上面的 `agentx` 可替换为这个可执行文件的绝对路径。

## 工作流程

1. 点击「启动」，选择工具或保存的 profile，填写目录和初始任务。
2. 从列表选择运行实例；在「消息」中发送任务、回复消息并查看真实通信记录。
3. 在「模型记录」读取 Provider 的对话快照。它与 AgentX 通信消息分别展示，内容取决于 Provider 已保存的记录。
4. 在选中实例上停止或恢复。停止保留历史；恢复使用保存的会话与启动参数，取决于 Provider 的既有恢复能力。
5. 网络中断后在「操作」查询原操作。页面保存请求 ID，避免不确定时重复启动或发送。

“消息已接受”表示进入 AgentX 的消息系统；“启动成功”表示启动流程已返回，Agent 的实际就绪/退出状态继续从列表查看。“停止请求已发送”也与进程已经退出分开。结果未知的操作保留原回执，不自动重发。

默认工作目录来自 `--project`，单次启动可以选择其他目录。工具/profile、环境文件与 Provider 参数沿用现有 AgentX 配置；HTTP 不接受任意 shell 命令或环境映射。环境文件由独立执行 worker 按操作目录和选定 scope 解析，不修改 daemon 的全局环境。

## HTTP 接入

同源页面和非浏览器客户端使用同一套 JSON 接口。首先获取当前数据代次：

```sh
curl http://127.0.0.1:18749/api/v1/management
curl 'http://127.0.0.1:18749/api/v1/agents?generation=REPLACE_WITH_GENERATION'
```

启动请求示例，`request_id` 每次新操作使用新的 UUID，断线重试则保留原值和原请求体：

```json
{
  "request_id": "d9e6a524-e998-47ec-a474-19bde4fe0d86",
  "generation": "REPLACE_WITH_GENERATION",
  "launch": {
    "tool": "codex",
    "profile": null,
    "directory": "/absolute/project",
    "prompt": "检查项目并报告当前状态",
    "title": "Web task"
  }
}
```

向 `/api/v1/agents` 发送 `Content-Type: application/json` 的 POST 后，查询 `/api/v1/operations/{request_id}?generation=...` 获取最终结果。操作状态为 `Accepted`、`Running`、`Succeeded`、`Failed` 或 `Unknown`。相同请求 ID 和内容返回原回执；更换内容返回冲突。清空/重置数据会更换 generation，旧请求不能进入新数据集。

详情与消息读取，以及发送、停止、恢复和 transcript 请求都使用列表返回的完整 `agent` 引用：名称加 `incarnation` 或 `stopped_event`，两者只能提供一个。不要自行用名字代替完整引用。接口列表、分页/大小限制和类型来源见[原生设计](design.md#http-contract)，生成的 TypeScript 位于 [native.ts](../../web/management/api/native.ts)。

## 验证范围

Linux 本地候选已通过完整 HTTP 和 Chromium 浏览器验收，包括启动、消息往返、模型记录、停止/历史/恢复、同名后续实例隔离、断线回执恢复、daemon 重启和 HTTP 重新配置。手机尺寸覆盖 390px 浏览器视口。验收使用本地 Provider 替身和真实原生 hooks，不代表真实付费模型或各 Provider 的完整互操作验证；实际范围与证据见 [已接受交付](../../.controller/agent-management/delivery.md#accepted-local-outcome)。

## 开发维护

HTTP 请求与响应的权威类型位于 [contract.rs](../../src/management/contract.rs)。修改公开类型后，按[公共 SDK 生成流程](../../packages/agentx-client/README.md)从原生候选导出 OpenAPI，再重新生成 SDK 并构建内嵌 Web 资源。[独立验收入口](../../tests/agent_management/README.md)使用单独候选和临时作用域验证 HTTP、原生执行链路与实际浏览器。

## 办公室管理扩展

成员归属、岗位目录、只读入口预览、按岗位/独立 workspace 启动和目录发送已作为一个完整能力实现并通过本地验收。浏览器操作和 HTTP 示例见[办公室使用说明](offices-usage.md)，准确契约见[统一设计](offices.md)，源码与验收进度见[Delivery](../../.controller/office-management/delivery.md)。原有 r4 验收不自动证明这些扩展已可用。


## 两套既有 UI 的实时接入

同一管理监听器增加 `/space/`（办公室场景与 Agent 对话）和 `/inspector/`（会话、观测、Token、分析与持续错误处理）。两套页面默认使用同源 `/api/v1` 的真实数据；原管理台 `/` 保留。实际候选、验收与运行切换状态以[实时集成 Delivery](../../.controller/live-ui-integration/delivery.md)为准。

Inspector 采集默认关闭，开启后才记录有限采样历史，不回填过去数据。Token 是 Provider 已保存记录的累计检查点，未知保持未知；分析为确定性证据汇总，不调用模型。“请求继续”表示原生消息入队，后续错误清除是独立观测。准确语义、容量和覆盖边界见[实时数据模型](../inspector/live-model.md)。

### 个体职责标题与办公室动态

本地 `agentx launch --title '负责原生 API'` 可提供独立于 Role、名字和标签的职责标题。省略时为空；旧记录保持兼容。标题去除首尾空白，须为 1–160 个 Unicode 字符的单行文本，不含控制字符；它持久保存在 SQLite 启动上下文，停止快照、恢复和 fork 保留原值，恢复/fork 的显式 `--title` 可覆盖。标题不会注入模型提示词，也不会决定岗位或 controller 王冠。远程 Relay 新启动和 shared Codex app-server stdio 暂不承载标题，会明确拒绝，普通本地终端/后台启动使用原生能力探测防止旧守护进程静默丢失。

原生 HTTP `LaunchInput.title` 可省略或为 null；`AgentView.title` 在当前/停止读取中返回可选文本，Web 类型由 Rust 生成。`AgentView.pending_messages` 是接受消息中位于该实例原生投递游标之后的待接收数量，失败为 null；它不是模型或人类已读确认。`GET /api/v1/office-activity?generation=…` 首次返回事件水位，不回放历史；携带 `after` 可按有界页读取接受消息的冻结精确寻址与新实例事实。旧事件缺少精确身份时不生成飞行映射。人类 @bigboss 单独表示作用域共享通信端，信箱的“待处理”是明确寻址记录减去显式处理标记，与阅读和 Agent 投递游标分开；打开信箱不改变状态。页面动画只解释接受事实，原生状态和操作结果不依赖动画完成。

## 嵌入会话与图片/文件上下文

现有 `/space/` 会话新增精确实例查询、iframe/sidebar 布局和 `hello`/`ready`/`addContext` 父窗口协议。文本、图片和文件先进入待发送草稿，用户确认后通过原生消息发送；附件保存在所选 AgentX 主机，刷新后可从消息历史打开。上传路径不代表 Provider 已接收多模态输入。准确协议、容量、宿主文件系统限制、父页面示例和候选状态见[设计与使用](web-context.md)及[实现 Task](../../.controller/web-context/implementation.md)。

## Agent 启动关系

`GET /api/v1/agents/:name/launch-relations` 使用 descriptor 的 `generation` 及列表返回的完整 `incarnation` 或 `stopped_event`。`limit` 为 1..100，`after` 使用上一页 `next`；不接受其他创建记录的游标。返回 `origin`（`agent` / `external` / `unknown`）、`upstream`、`downstream`、`next`。每项包含原始精确 name/incarnation；`agent` 为当前原生 AgentView 或保留的停止会话，缺失时为 null 且 availability=unavailable。不可用项不得按同名后继跳转。`launch_failure` 可保留原生 `cancelled_before_ready` 事实，Space 在会话已清除时仍显示启动失败。

普通恢复产生新 incarnation，但沿用初次创建关系；fork 以实际发起者为启动来源，与 Provider fork source 分开。停止及 daemon 重启保留关系；历史事件清除后保留关系但会话可能不可用；generation reset 清除本代关系。旧数据不按启动者名字回填。原生 launch/resume 已接入；共享原生 root 无预留事实、Relay 与无法证明的来源显示未知。该关系不授予权限，也不触发级联停止。

Space 在会话下方显示来源及直接启动项，逐页读取原生状态，点击重新核对精确引用后进入实际办公室/会话；草稿继续由页面持有。实现与独立验收状态见 [Delivery](../../.controller/agent-launch-relations/delivery.delivery.md)，准确 R5 已在既有 :38023 服务采用；历史来源仍未知，安装版启动器未替换。Agent 内要记录新的精确来源，请保留注入环境并使用 `/home/long/works/longwork/agentx/.artifacts/agent-launch-relations/candidate-pivula-r5/agentx launch ...`，沿用正常启动参数。Web 操作者启动显示外部来源。


### @bigboss 人类信箱

`GET /api/v1/human-mailbox?generation=…&pending=true&limit=20` 返回按事件 ID 倒序的待处理页；`pending=false` 查看全部历史，`before=next_before` 读取更早信件。`unhandled_count` 覆盖整个作用域的历史，不是当前页长度。原生 `HumanMailbox` / `HumanMessage` 是声明源，响应带 `generation`、`next_before` 和每条 `handled_at`。

`POST /api/v1/human-mailbox/handle` 的 JSON 为 `{ "generation": "…", "ids": ["123"] }`，仅标记明确列出的 1–100 个已接受且寻址给 bigboss 的事件；全部验证后在同一事务中保存首次处理时间，重复请求幂等，成功 HTTP 200 返回信箱观察。错误代际、非人类收件事件及无效 ID 被拒绝，混合无效批次不会部分生效。它不删除消息、不确认阅读、不推进任何 Agent 投递游标。网络结果不明时读取原信箱核对，显式重试同一 ID 集合安全。处理状态随作用域数据库保存并按代际隔离。

Space 猫窝入口显示待处理总数，信箱提供“待处理 / 全部历史”、逐条标记、“处理本页”和更早／最新分页。处理本页只提交当时显示的未处理 ID，新到和未显示信件保留。历史文字与处理时间持续可查。新接口需要包含该功能的候选；历史运行服务是否采用见[所属 Task](../../.controller/office-details/bigboss-cat.task.md)。

人类信件的 `sender` 来自接收时固化的精确 Agent 身份；旧消息可能为 null。Space 按该身份分组并渲染安全 Markdown，点击发送者只导航到原实例，不会按同名新实例替换。组处理仅处理当前页该组显示的待处理 ID。

Agent 操作新增 `POST /api/v1/agents/:name/stop-participation` 和 `/fork`，使用既有 `{request_id,generation,agent}` 操作凭据和原生回执。前者停止 AgentX 参与，进程处置遵循原生生命周期；后者仅在工具及保存会话支持分叉时可用。`/kill` 与历史 `/stop` 都表示终止进程。能力字段 `can_stop_participation`、`can_fork` 与既有 `can_stop` 分开显示；界面打开菜单和确认框不执行操作。

## Agent labels

当前源码增加标量 labels；固定候选的独立执行证据见 [Delivery](../../.controller/agent-labels/implementation.delivery.md)与[验收记录](../../tests/agent_labels/README.md)，不代表已安装程序或运行服务已经采用。规则以[分类模型](../agent-classification.md)及 Rust 原生声明为准。

`GET /api/v1/management` 的 `supports_labels=true` 表示支持；旧服务缺少此字段时显示“不支持”。当前 AgentView 返回 labels 对象和不透明字符串 labels_revision；旧历史未记录时两者为 null。

- `GET /api/v1/agents/:name/labels?generation=…&incarnation=…` 返回 `{generation,agent,labels,revision}`，incarnation 使用列表中的十进制字符串。
- `POST /api/v1/agents/:name/labels` 接受 `{request_id,generation,agent,set,remove,if_revision}`；set 是字符串映射，remove 是 key 数组，if_revision 可省略或为 null。不允许上传整张 map 覆盖未提及的 key。HTTP 返回现有 Operation，成功结果为 `{LabelsPatched:{snapshot,changed}}`；revision 冲突返回 409。
- 写入响应丢失时保留原 request_id，通过 `GET /api/v1/operations/:id?generation=…` 查询，或以相同 ID、相同精确目标和完全相同内容重送；不要换 ID 盲目补写。原回执不是最新状态，需重新 GET labels。
- `GET /api/v1/agents?generation=…&current_only=true&label=project%3Datlas&label=%21review%2Fstate&limit=50` 在原生端按 AND 筛选当前本地 Agent。返回 next 绑定 selector 与 scope；修改条件后清空 after，不接受普通历史游标。省略 current_only 时不允许 label 参数；无过滤普通列表保持既有历史语义。页面分组数仅代表本页。

CLI 对应 `launch --label project=atlas --label feature=search`、`labels get NAME --json`、`labels set NAME review/state=pending --if-revision REV --request-id UUID`、`labels remove NAME review/state`、`labels receipt UUID --json` 和 `list --label project=atlas --label '!review/state' --json`。创建 labels 目前支持新建本地 terminal Agent；远端 Relay 和 stdio 创建明确拒绝非空初始 labels。普通 resume 保留原生创建的 labels，fork 默认空；用 labels set 显式修改恢复后的分类。
