---
schema: document
description: Use office membership, effective posts, route previews and Role/directory actions through the existing management service.
status: active
owner: agent-management
last_updated: 2026-11-15
links: [./README.md, ./offices.md, ../../.controller/office-management/delivery.md]
---

# 办公室、岗位与目录联系

这是现有 daemon 管理服务的扩展，已验收构建为 `.bin/agentx-office-r3`，启动方式沿用[管理台说明](README.md)。本次构建、候选和实际验证范围以 [Delivery](../../.controller/office-management/delivery.md) 为准。可视化办公室的独立离线页面不因这些 API 自动变成实时客户端。

## 管理台

在「办公室 / 岗位」查看已知目录办公室，选择办公室读取当前有效岗位。列表来自已保存的成员归属和项目内有限目录发现，因此空的 `.agentx/roles/` 办公室也可以出现。覆盖详情会显示扫描限制和错误；列表不是整台机器的全部项目。

成员列表支持按办公室、明确全局、旧记录和元信息不可用筛选。展开成员归属可以查看启动时采用的岗位及来源。工作目录、岗位定义的来源目录和成员归属各自独立；修改或删除岗位文件不会改写已有成员的岗位快照。停止后的同名历史仍是独立对象。

点击岗位的启动按钮会预填岗位名称和目标办公室。启动面板中的「启动目录」是进程工作目录，可与办公室不同；`global` 表示明确全局归属。采用岗位时由岗位选择 profile，不能同时指定工具或其他 profile。执行时读取当前定义，实际采用内容以启动后成员快照为准。

「预览路由」只读取目录的默认岗位、真正声明默认值的办公室，以及当前入口绑定或候选状态，不会采纳成员、创建进程或发送消息。「向目录发送」是独立操作，可能复用、采纳或启动入口成员。操作回执显示实际解析结果、准确收件人、消息 ID 和新建/复用结果。存在多个候选或入口异常时，原生规则决定拒绝原因；页面不擅自指定一个 controller。

通信、模型记录、停止和恢复继续使用原有管理台。结果未知或回执丢失时，先在「操作」查询原请求，保留原 ID；不要换一个 ID 重复执行。

## HTTP 读取

所有新增读取请求均携带 `/api/v1/management` 返回的 `generation`：

| 请求 | 用途 |
| --- | --- |
| `GET /api/v1/workspaces?generation=…` | 已知办公室及有限发现覆盖情况 |
| `GET /api/v1/roles?generation=…&workspace=…` | 指定绝对 workspace 或 `global` 的有效岗位 |
| `GET /api/v1/directory-entry?generation=…&directory=…` | 指定绝对目录的只读入口预览 |

路径参数需要 URL 编码。Role 的 `namespace` 是来源加原生目录，例如 `global:/scope/roles`；`path` 是定义文件路径。二者用于来源展示，启动选择器仍是 `reviewer`、`local:reviewer` 或 `global:reviewer`，不能把定义文件路径当作选择器。

现有 AgentView 增加 `collaboration`：`status=adopted` 时，workspace 为绝对归属路径或明确全局的 null；role 为已采用岗位或明确无岗位的 null。`legacy` 表示没有记录采用信息、沿用原生全局行为；`unavailable` 表示信息损坏/不支持，reason 给出原因。它们不能被统一解释为用户明确选择了全局和无岗位。

入口状态有 `bound`、`adoptable`、`absent`、`ambiguous`、`role_changed`、`stale` 和 `unknown`。`adoptable` 只是观察到唯一兼容候选；预览不会将它变成绑定。`bound` 不代表 Agent 一定健康或可以接收消息，实际发送仍执行原生可用性检查。 同一岗位文件的描述、提示词或 profile 更新不会自动使已有入口失效，也不会修改已有成员快照；默认入口改指其他规范岗位文件路径才属于岗位身份变更。

## HTTP 操作

向现有 `POST /api/v1/agents` 提交岗位启动。新操作使用新 UUID，重试原操作保留完全相同的 ID 和请求体：

```json
{
  "request_id": "1a8dc190-46e6-4a0b-a788-31b1d723bba7",
  "generation": "REPLACE_WITH_GENERATION",
  "launch": {
    "role": "global:reviewer",
    "workspace": "/projects/app/web",
    "directory": "/tmp/review-work",
    "tool": null,
    "profile": null,
    "prompt": "检查项目并给出审查结果",
    "title": "Web review"
  }
}
```

向 `POST /api/v1/directory-messages` 提交目录联系：

```json
{
  "request_id": "a37fb387-f52b-4b42-999d-bb4a930ec977",
  "generation": "REPLACE_WITH_GENERATION",
  "directory": "/projects/app/web",
  "message": {
    "text": "请处理这项任务并回复结果",
    "intent": "request",
    "reply_to": null,
    "thread": "web-task"
  }
}
```

两种请求均返回 202 操作回执，沿用 `GET /api/v1/operations/{request_id}?generation=…` 查询。目录联系结果为 `DirectorySent`，包含执行时的 entry、准确 recipient、sent 消息回执和 created。子目录继承父目录默认岗位时，入口成员属于实际声明默认岗位的父办公室；全局默认入口则采用全局归属，工作目录沿用请求目录。普通按岗位启动支持独立选择归属与工作目录，两种用法不要混淆。

可用岗位读取遵循原生继承和遮蔽顺序。配置、profile 或默认值冲突是明确错误，不会伪装成空岗位列表。旧回执不因岗位文件或目录随后变更而重新执行；一次真实执行读取当时选定的配置。继续已有会话请使用 resume，fresh launch 不等价于 resume 或 fork。
