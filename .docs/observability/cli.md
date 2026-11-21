---
schema: document
status: active
owner: diagnostics
last_updated: 2026-11-08
description: Passive observation CLI usage, native operands, output and recovery.
---

# AgentX 只读诊断

`agentx observe` 用来查看已有范围中的守护进程、请求、配置证据与观测事件。源码和用例已编写；可执行支持以[独立集成记录](../../.controller/observability/implementation-evidence/native.md)为准，[原始任务](../../.controller/observability/implementation-tasks/diagnostics.md)记录交接状态。命令不会启动守护进程，也不会自动重试原来的业务操作。

## 选择入口

把 `<SCOPE>` 替换成要调查的现有 AgentX 数据目录；引用文件保存的是命令返回的完整原生 JSON 对象。

| 你要查什么 | 命令模板 | 输入与结果 |
| --- | --- | --- |
| 连不上、响应慢，先看进程与分派器 | `agentx observe --scope <SCOPE> --json` | 等同于 `observe inspect`；输出快照或本地失败证据。 |
| 找慢 Hook 或把单次 Hook 与 daemon 请求对上 | `agentx observe trace --scope <SCOPE>`，再运行 `agentx observe trace <TRACE_ID> --scope <SCOPE>` | 列表只读文件；按 ID 查看时尝试一次被动 daemon 查询。加 `--local` 只读文件，`--json` 包含客户端 IPC 子阶段、原生结果和缺失证据。 |

请求详情与 Trace 的原生 JSON 可包含 `post_commit_work`：SQLite 提交、观察者资格／提示、订阅候选和求值、request-watch 的互斥微秒计数，以及候选、谓词、提示入队／合并／丢弃次数。它涵盖被打点的应用阶段，不等于应用总耗时或端到端吞吐。提示入队不表示已发送或已交付。字段缺失表示当前请求未保留这段信息。
| 沿一个已返回的请求引用往下查 | `agentx observe request --ref-file <REQUEST.json> --scope <SCOPE> --json` | 文件只放完整 `RequestObservationRef`，不接受名称、PID 或单个编号。 |
| 看当时保留的配置 | `agentx observe config --ref-file <CONFIG.json> --scope <SCOPE> --json` | 文件只放完整 `ConfigSnapshotRef`；不会重读当前配置文件。 |
| 明确检查这次调用现在会怎样解析设置 | `agentx observe config-current --scope <SCOPE> --json` | 本地新读，标为 `DiagnosticReevaluation`；不需要联系守护进程。 |
| 比较两个历史配置引用 | `agentx observe compare --left-file <LEFT.json> --right-file <RIGHT.json> --field timeout --scope <SCOPE> --json` | `--field` 可重复；省略时由原生规则选择全部字段。 |
| 看当前保留的事件窗口第一页 | `agentx observe events --limit 128 --scope <SCOPE> --json` | 先描述范围，再从保留最早位置读取有界页面。 |
| 续读已返回的固定窗口 | `agentx observe events --window-file <WINDOW.json> --limit 128 --scope <SCOPE> --json` | 文件只放完整 `ObservationWindow`，包括 `after`、`through` 和 `filter`。 |
| 持续观察 | `agentx observe events --follow --scope <SCOPE> --json` | 每行一个完整结果；缺口、重启、重置或流替换会先显示再结束。 |
| 看帮助 | `agentx observe --help` 或 `agentx observe events --help` | 解析器生成的帮助；不读取配置或获取业务身份。 |

`--scope`、`--json` 可以放在 `observe` 后、子命令前或子命令后。省略 `--json` 使用文字输出。完整语法、字段前缀与验证入口见[原生 CLI 声明](../../src/cli/commands/observe/args.rs)；本页的例子使用同一入口。

## 输入与范围

| 输入 | 类型和默认值 | 来源与约束 |
| --- | --- | --- |
| `--scope PATH` | 现有目录；省略时走已有调用环境/cwd 解析器 | 显式 CLI 优先；否则按已有 `AGENTX_DIR`、`HOME`/`USERPROFILE` 回退规则选择。不添加配置文件键。相对路径按发起进程 cwd 解析。 |
| `--json` | 布尔；默认关闭 | 仅本次命令；没有新增 ENV 或配置键。 |
| `--ref-file`、`--left-file`、`--right-file`、`--window-file` | 显式本地普通文件 | 至多 32 KiB、UTF-8 JSON；不从 stdin 读取，不自动写回，不接受未知字段。 |
| `--field NAME` | 原生设置名、`context.FIELD` 或 `participant.FIELD` | 仅用于 compare；原生闭合目录和选择器验证拒绝未知或重复字段。 |
| `--limit N` | 每页记录数；默认 128，允许 1..256 | 仅用于 events；无 ENV 或配置键。 |
| `--follow` | 布尔；默认关闭 | 仅用于 events；每次读取间隔约一秒，没有后台订阅。 |

文件引用必须属于明确选择的范围。符号链接或相对路径别名通过原生现有范围解析器归一化；其他范围中的相同名称、PID、序列号不构成相同对象。参数、文件和选择器拒绝使用退出码 2；范围不存在、不可读或不是目录会输出结构化本地结果并退出 1。事件的时代、nonce 和数值窗口判断由原生事件所有者执行，保留其“先判断时代，再判断边界”的顺序。

## 保存引用和续读

JSON 输出的顶层字段是 `command`、`requested_scope`、`scope`、`local`、`result`。[输出类型](../../src/cli/commands/observe/output.rs)直接嵌入原生类型，未创建第二套引用或游标。

| 你已有的结果 | 要保存的完整对象 |
| --- | --- |
| 正在运行的请求快照 | `result.Inspection.response.Runtime.Ok.Snapshot.activity.Running`，仅当 activity 确实是 Running 时存在。 |
| 请求详情 | `result.Inspection.response.Runtime.Ok.RequestDetail.reference`。配置引用在该详情原生 `configuration` 中的 Recorded 使用记录里。 |
| 已找到的配置 | `result.Inspection.response.Configuration.Ok.Snapshot.result.Found.reference`。 |
| 事件页面 | `result.Inspection.response.Events.Ok.Read.Page.next`，整个对象保存到窗口文件。 |

把上述完整对象作为 JSON 文件保存，然后传给相应 `--ref-file` 或 `--window-file`。不要只取其中的数字，也不要将引用改成当前时代。序列和编号是原生 `u64`；使用保留整数精度的 JSON 工具，避免先转换成浮点数。命令一次只返回一个有界页面，接下来的窗口由返回的 `next` 决定。

窗口未读完时，保持返回的过滤器和 `through` 原样续读。显式 `--follow` 才会在读完后打开同一流的下一段；`Gap`、`EpochChanged`、`StreamChanged` 或 `DaemonChanged` 不会被自动抹平。出现这些结果时先保留它们，再明确决定是否重新运行不带旧窗口的 events。新窗口不证明旧缺口已恢复。

## 如何理解结果

`local.inputs` 是发起调用环境的安全输入投影；显式 `--scope` 的选择另保存在 `requested_scope`，因此二者可能不同。`local.attempt` 保留本次被动探查的原始范围、时间和已知失败。不会导出原始环境、身份凭证或业务请求体。时钟不可用时，已知失败的 `at` 可以是 null；它仍有已知阶段和原因，但无法据此推断顺序或年龄。

`result.Inspection` 包含原生 `ready`、`response` 和发起端的 `received_at`。Ready/快照/配置/事件各自保留原始时钟、采样时间或保留时间；不能跨时钟相减，也不能把历史捕获当成当前配置。文字输出显示这些边界及原生安全结果；JSON 输出保留原生枚举标签、整数字段、未知、脱敏和丢失状态。

| 退出码 | 含义 |
| --- | --- |
| `0` | 本次只读诊断完成，包括明确的配置缺失、不可比较或单次事件缺口；不表示原来的业务成功。已检测到 stdout 关闭时也停止观察并返回 0。 |
| `1` | 本地捕获/范围/传输失败、原生查询拒绝，或 follow 遇到已显示的缺口/时代边界；结果中保留具体原因。 |
| `2` | 参数或显式文件输入无效；stdout 为空，stderr 说明错误。 |
| `130` | SIGINT 结束本次观察。原生在途读取有自己的有界期限，结束后释放调用端资源。 |

单次 JSON 是一个完整 UTF-8 值加换行；follow 是相同结构的 JSON Lines。stderr 用于语法错误、输出失败和跟随结束提示，不混入 JSON。关闭输出或按 Ctrl-C 只停止观察，不取消原来的请求、Provider 工作或参与者。

## 丢失回复后的恢复边界

请求详情中的 `ReturnedOk`、回复超时和写出失败是不同维度。事件中的 `MessageAccepted` 是带作用域和代际的提交后观察；事件缺失也可能来自采集丢失。它们都不能单独授权重新提交、确认收件或释放不确定的 claim。

如果已有原始逻辑 `RequestId`、参与者/目标身份和数据库代际，恢复应交给原有领域允许的只读接口。例如 Messages 的 `Acceptance { request_id }`，或 Inbox 的 `Outstanding { target }`，必须保留原来的身份、范围和代际前提。具体入口和结果含义见[OBS-03 原生恢复路径](scenarios.md#obs-03-loss-of-reply-and-durable-evidence)。本命令不自动调用这些业务接口，也不创建通用凭证或历史收据查询。原来的分派器不可用时，诊断可以明确结束为“业务效果未知”。

源码交接、作者用例与可执行验证分别记录在[诊断任务](../../.controller/observability/implementation-tasks/diagnostics.md)、[CLI 用例](../../src/cli/commands/observe/tests.rs)和[独立证据](../../.controller/observability/implementation-evidence/native.md)。此入口不声明已安装替换或其他平台支持。
