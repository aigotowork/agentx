---
scope: AgentX public command panorama after the agreed CLI changes
status: target-reference
owner: AgentX CLI
last_updated: 2026-11-14
related: ["../src/cli/router.rs", "../src/cli/commands/help.rs", "../src/runtime/launch_options.rs"]
---

# AgentX 命令全景图（改造后目标）

这份文档保存本轮讨论确认的目标命令体系，供之后查阅和实现时对照。以此前整理的完整命令全景图为基线，将新建 Agent 的入口统一为 `agentx launch`，通过工具、具名 profile 或默认 profile 选择配置，并用 `--` 严格隔离原生工具参数。其余命令沿用现有职责和参数；本文不代表实现、打包或验收已经完成。

## 已确认的语法

```text
agentx [通用选项] launch [--tool <工具>] [--profile <名称>] [--count <数量>] [启动选项] [-- 原生参数…]
agentx [通用选项] resume <目标> [启动选项] [-- 原生附加参数…]
agentx [通用选项] fork <目标> [启动选项] [-- 原生附加参数…]
```

| 项目 | 目标规则 |
| --- | --- |
| 新建 Agent | 唯一入口为 `launch`；工具由显式 `--tool` 或选中 profile 确定；省略时按 [Profile 选择规则](../.docs/launch-profiles.md)解析 |
| 实例数量 | `--count <正整数>`，默认 `1`；同时受各工具的启动数量上限约束 |
| 原生参数归属 | `--` 前只解析 AgentX 参数，`--` 后属于原生工具；没有原生参数时可省略分隔符 |
| 未知输入 | `--` 前的未知参数报错；原生参数的有效性由对应工具及其接入约束决定 |
| 旧启动入口 | 移除工具名直启和数字前缀直启；按未知命令报错，不保留兼容解析、兼容窗口或迁移提示 |
| 恢复与分叉 | 使用相同的原生参数边界；`r`、`f` 分别为 `resume`、`fork` 的简写 |
| 脚本入口 | `run` 继续执行工作流脚本 |
| 参数保真 | 原生参数保留 argv 的顺序、空字符串、空格和后续 `--`；不得因与 AgentX 选项同名而被截取 |
| 默认工具参数 | 未选中具名 profile 时沿用 `<tool>_args`；选中 profile 时只读取该 profile 的 args；均不重新解析为 AgentX 参数 |
| 远端启动 | 使用相同的 AgentX／原生参数边界，传输过程中保留参数归属 |

例如，启动两个后台 Codex Agent，并把模型参数传给 Codex：

```text
agentx launch --tool codex --count 2 --headless -- --model MODEL
       └────────────── AgentX ───────────────┘    └── Codex ──┘

agentx launch --tool claude --dir ./project -- --model sonnet
agentx resume luna --headless -- --model MODEL
agentx fork luna --dir ./experiment -- --model MODEL
agentx launch --tool codex --stdio --go -- app-server
```

帮助也按同一条边界区分：

```text
agentx launch --tool codex --help       → AgentX 的启动帮助
agentx launch --tool codex -- --help    → 将 --help 交给 Codex
```

## 完整命令树

### 完整帮助模式

已确认采用独立选项 `--help-all`：从当前命令开始递归展开完整命令树及每个节点的详细帮助。`agentx --help-all` 展示全局，`agentx events --help-all` 只展开事件命令，`agentx events sub --help-all` 只展开订阅命令。普通 `--help` 保留当前层帮助；业务选项 `--all` 按各命令自己的含义解析。

实现结构：`cli/commands/help.rs` 继续拥有帮助分区与普通帮助；其 `help/all.rs` 子模块负责命令树、范围选择和完整帮助输出。Clap 命令树来自对应命令的原生声明，手写命令树来自既有帮助分区。入口在 main 的配置、环境文件、Codex 启动和 daemon 初始化之前处理；`--` 后以及文本选项值中的 `--help-all` 属于输入数据。帮助仅描述 AgentX 公共接口，不运行用户脚本或原生工具来收集信息。缺少必填业务参数不妨碍查询帮助，未知命令路径返回非零退出码。

2026-11-14 紧凑排版反馈：用户指出完整帮助以及 `agentx launch -h` 等普通帮助有过多无意义换行和空行。`help.rs` 统一处理普通 `-h` / `--help`、Clap 帮助和 `--help-all` 的紧凑排版：参数与首行说明并排，较长的手写命令说明不再强制另起一行，删除帮助页内部空行；完整帮助仅保留命令页之间和全局分区之间的间隔。完整长说明、参数、示例和命令树仍保留。

本次紧凑排版验证：19 项帮助相关测试通过，`just package` 完成并更新 `.bin/agentx`；对照修改前输出的 26 个普通帮助调用和 6 个递归帮助范围，非空白内容、输出流和退出码全部一致。`agentx launch -h` 从 83 行缩至 33 行，空行从 28 行降至 0 行；`agentx --help-all` 从 1644 行缩至 1217 行，空行从 407 行降至 91 行。下方历史快照保留原实现时的输出。

2026-11-14 此前完整帮助实现检查：30 项帮助相关测试、6 项元数据相关测试通过；开发构建 `target/debug/agentx` 的 80 个公共命令路径逐一验证通过，另检查了 11 个别名、位置参数、未知路径和参数隔离场景。检查时使用不存在的 dotenv、无效环境覆盖设置和全新的数据路径，完整帮助仍正常输出且未创建数据目录。[完整帮助实测快照](agentx-help-all.txt)由该开发构建直接生成，共 1499 行。发行打包尝试因已有长时间发行编译占用共享构建锁而结束本次等待，未据此更新 `.bin/agentx`；开发构建验证与发行包状态分开记录。

### 命令与参数

`<参数>` 表示必填，`[参数]` 表示可选，`…` 表示可重复，`A | B` 表示替代选择。选项行列出可用选项，不表示必须全部填写；标明“必填”的选项除外。所有节点均接在 `agentx` 后；分类标题只用于阅读，不是命令。

```text
agentx
│
├─ 无参数                                    打开 TUI 仪表盘
├─ --new-terminal                            在新终端打开 TUI
├─ --help / -h                               顶层帮助
├─ --help-all                                递归展开完整命令树与全部详细帮助
├─ help [命令 [子命令]]                      命令帮助
├─ --version / -v                            AgentX 版本
│
├─ Agent 启动与生命周期
│  │
│  ├─ launch [--tool <工具>] [--profile <名称>] [启动选项] [-- 原生参数…]
│  │  │                                      新建 Agent
│  │  ├─ --profile <名称>                   选中保存的配置；决定工具
│  │  ├─ --no-profile                       必须带 --tool；忽略 profile 和旧工具默认参数
│  │  ├─ --replace-args                     本次原生参数完整替换继承参数，可清空
│  │  ├─ --dry-run                          仅查看解析结果，可组合 --json
│  │  ├─ --count <N>                         实例数，默认 1，必须为正整数
│  │  ├─ --tool 的可选值
│  │  │  claude · gemini · codex · opencode · kilo · pi · omp
│  │  │  hermes · antigravity · cursor · kimi · copilot · qoder · grok
│  │  │
│  │  └─ 启动选项
│  │     --dir <路径>                        启动目录
│  │     --terminal <预设>                   终端选择
│  │     --headless                          后台运行；具体方式由工具接入决定
│  │     --device <设备>                     通过 relay 在远端启动
│  │     --agentx-prompt <文本>              初始用户提示
│  │     --agentx-system-prompt <文本>       系统提示
│  │     --json                              输出单个 JSON 结果
│  │     --go                                跳过启动预览
│  │     --run-here / --no-run-here          在当前终端运行 / 不占用当前终端
│  │     --stdio                             Codex 专用：前台标准流模式，无 PTY
│  │
│  ├─ resume <目标> [启动选项] [-- 原生附加参数…]
│  │  │                                      恢复已停止的 Agent；简写 r
│  │  └─ 目标
│  │     Agent 名字 / 会话 UUID / ses_<id> / 线程名称
│  │     <目标>:<设备>                       在远端设备恢复
│  │
│  ├─ fork <目标> [启动选项] [-- 原生附加参数…]
│  │  │                                      从会话分叉一个新 Agent；简写 f
│  │  ├─ 支持：claude、codex、opencode、kilo、pi、omp、qoder、grok
│  │  └─ 远端分叉需要 --dir 明确目标目录
│  │
│  ├─ start                                  从任意 AI 工具内部接入 AgentX
│  │  --name <name-or-agent-id>              使用实例名或子 Agent ID 注册
│  │  --as <name>                            重新认领已有身份
│  │
│  ├─ stop [name… | all]        断开连接；无目标时断开自己
│  │
│  └─ kill <name… | all>        结束进程，并关闭可管理的终端 pane
│     --json                                 输出 JSON 结果
│
├─ 消息通信
│  │
│  ├─ send [@目标…] -- <消息文本>            发消息；无目标为广播
│  │  │
│  │  ├─ 消息输入方式
│  │  │  -- <文本…>                         分隔符后全部作为正文
│  │  │  --file <路径>                      从文件读取
│  │  │  --base64 <编码文本>                从 Base64 解码
│  │  │  无正文参数                         从 stdin 读取
│  │  │
│  │  ├─ 消息语义
│  │  │  --intent request|inform|ack        请求 / 通知 / 应答
│  │  │  --reply-to <事件ID>                关联原消息；ack 必须指定
│  │  │  --thread <名称>                    按线程路由
│  │  │
│  │  ├─ 身份
│  │  │  --iam <名字或UUID>                已接入 Agent 的身份
│  │  │  --from <名称>                      外部发送者身份
│  │  │  -b                                等同 --from bigboss
│  │  │
│  │  ├─ 输出
│  │  │  --json                             JSON 回执，保留未读邮件
│  │  │  --quiet                            不输出，保留未读邮件
│  │  │
│  │  └─ 内联上下文包
│  │     --title <标题> --description <说明> 创建上下文包；有标题时必须有说明
│  │     --events <事件ID或范围> --files <逗号分隔路径>
│  │     --transcript <范围:详细度,…> [--extends <父包ID>]
│  │
│  └─ listen [超时秒数]                      等待消息或匹配事件
│     --timeout <秒数>                       默认 86400 秒
│     --json                                 JSON 输出
│     --sql <表达式或预设>                   例如 stopped:name
│     [事件过滤选项]                         见共享事件过滤器
│
├─ Agent 与终端查看
│  │
│  ├─ list                                   查看存活 Agent、状态、未读数
│  │  -v                                    详细信息
│  │  --json | --names | --format <模板>    JSON / 仅名字 / 自定义格式
│  │
│  ├─ list <self|name> [字段]                查看单个 Agent 或某个字段
│  │  --json | --sh                         JSON / Shell 环境导出
│  │
│  ├─ list --stopped [name]                  查看已停止的 Agent
│  │  --all                                 全部；默认最近 20 个
│  │
│  ├─ term [name]                            查看 PTY 屏幕；默认全部
│  │  --json | --clean                      原始 JSON / 简洁文本
│  │
│  ├─ term inject <name> [文本]             向 PTY 输入内容
│  │  --enter                               追加回车提交，也可单独使用
│  │
│  └─ term debug on|off|logs                 开关 PTY 调试 / 列出日志
│
├─ 事件与订阅
│  │
│  ├─ events [事件过滤选项]                  查询事件，默认最近 20 条 JSON Lines
│  │  --last <N>                             条数
│  │  --all                                  包含归档会话
│  │  --full                                 完整事件 JSON
│  │  --sql <WHERE表达式>                    与过滤选项按 AND 组合
│  │  --wait [秒数]                          等待匹配，省略秒数默认 60
│  │  --remote-fetch --device <设备ID>       向远端获取事件
│  │
│  ├─ events --follow [事件过滤选项]         持续输出新事件
│  │  --last <N>                             先回放最近 N 条
│  │  --after-id <ID>                        从保存的事件游标继续
│  │  --full / --sql <表达式>
│  │
│  ├─ events launch [batch_id]               等待启动批次进入终态
│  │  --timeout <秒数>                       默认 30 秒
│  │
│  ├─ events sub list                        查看订阅
│  │  --device <设备ID>                      查看远端订阅
│  │
│  ├─ events sub [过滤选项 | SQL表达式]      创建订阅；命中后以消息通知
│  │  --once                                 命中一次后删除
│  │  --for <Agent>                          为指定 Agent 订阅
│  │  --on-hit <文本>                        命中时附加发送消息
│  │  --as <名称> / -b                      使用外部订阅者身份
│  │  --device <设备ID>                      在远端安装订阅
│  │
│  └─ events unsub <订阅ID>                  删除订阅
│     --device <设备ID>                      删除远端订阅
│
├─ 上下文包与会话记录
│  │
│  ├─ bundle [list]                          最近上下文包
│  │  --last <N> --json                      默认 20 个
│  │
│  ├─ bundle show <ID> [--json]              查看上下文包
│  ├─ bundle cat <ID>                        展开内容；文件只展示元数据
│  ├─ bundle chain <ID> [--json]             查看继承链
│  │
│  ├─ bundle prepare                         根据近期活动生成打包建议
│  │  --for <Agent>                          默认自己
│  │  --last-transcript <N>                  默认 40 轮
│  │  --last-events <N>                      每类默认扫描 10 条事件
│  │  --json --compact
│  │
│  ├─ bundle create [标题]                   创建上下文包
│  │  --title <标题>                         替代位置参数标题
│  │  --description <说明>
│  │  --events <ID或范围,…>
│  │  --files <路径,…>
│  │  --transcript <范围:normal|full|detailed,…>
│  │  --extends <父包ID>
│  │  --bundle <JSON> | --bundle-file <文件> 整体 JSON 输入方式
│  │  --json
│  │
│  ├─ transcript <name> [N | N-M | N-]       查看会话；默认最近 10 轮
│  │  --last <N> --full --detailed --json
│  │
│  ├─ transcript timeline                    按时间查看各 Agent 的用户提示
│  │  --last <N> --full --detailed --json
│  │
│  ├─ transcript search <正则表达式>         搜索会话
│  │  --live                                 仅存活 Agent
│  │  --all                                  搜索磁盘全部记录，需要 rg
│  │  --limit <N>                            默认 20 条匹配
│  │  --agent <工具类型>
│  │  --exclude-self --json
│  │
│  └─ archive [N | 名称] [agents]            列出归档或查询归档事件 / Agent
│     --here                                 仅当前目录
│     --sql <表达式> --last <N> --json
│
├─ 项目协作任务
│  │
│  ├─ work show                              查看任务、分工、简报、证据和验收资格
│  │  --work <ID> [--generation <代次>]      --work 必填
│  │
│  ├─ work list                              任务列表
│  │  --limit <N> [--after <游标>]           默认 32
│  │
│  ├─ work candidates                        当前项目可参与的 Agent
│  │  --limit <N> [--after <游标>]           默认 32
│  │
│  ├─ work command                           查询已提交命令的结果
│  │  --generation <代次> --command-id <ID>  均必填
│  │
│  ├─ work report                            报告进度 / 问题 / 结果 / 检查
│  │  --work <ID> --generation <代次> --revision <版本>
│  │  --assignment <分配ID> --brief <简报版本>
│  │  --command-id <ID> --payload <JSON>     以上 report 选项均必填
│  │
│  ├─ work apply                             提交操作员命令
│  │  --generation <代次> --command-id <ID> --payload <JSON>  必填
│  │  [--work <ID>] [--revision <版本>]
│  │
│  └─ 所有 work 子命令共有
│     --scope <目录>                         必填
│     --project <目录>                       必填
│     --iam <身份>                          可选
│     输出固定为 JSON，无需 --json
│
├─ 浏览器入口
│  │
│  ├─ workbench                              项目协作工作台
│  │  --scope <目录>                         默认 AGENTX_DIR 或用户默认目录
│  │  --project <目录>                       默认当前工作目录
│  │  --collaboration                        开放有限的操作员写操作
│  │  --assets <目录>                        指定页面资源
│  │  --host <地址>                          默认 127.0.0.1
│  │  --port <端口>                          默认 0，自动分配
│  │  --iam <名称>                          公共调用参数，不赋予浏览器 Agent 身份
│  │
│  └─ observe-web                            只读运行诊断页面
│     --scope <目录>                         必填
│     --assets <目录>
│     --host <数字IP>                        默认 127.0.0.1；0.0.0.0 可用于 LAN
│     --port <端口>                          默认 0；非零时须 ≥1024
│     --iam <名称>                          公共调用参数，不初始化身份
│
├─ 原生 daemon 与诊断
│  │
│  ├─ daemon status [--scope <目录>]         检查状态和兼容性，不启动 daemon
│  ├─ daemon restart [--scope <目录>]        显式重启，保留数据
│  │
│  ├─ status                                 安装状态与综合诊断
│  │  --logs --json
│  │
│  ├─ observe [inspect]                      查看已有 daemon 的运行 / 进程快照
│  ├─ observe request --ref-file <JSON文件>  查看精确请求观测记录
│  ├─ observe config --ref-file <JSON文件>   查看已保留配置快照
│  ├─ observe config-current                 重新计算本次调用的当前配置
│  ├─ observe compare                        比较两个已保留配置快照
│  │  --left-file <文件> --right-file <文件> 均必填
│  │  --field <字段>…                        可重复
│  ├─ observe events                         查看原生诊断事件窗口
│  │  --window-file <文件>                   可选；省略时读取当前保留范围的首页
│  │  --limit <N>                            1～256，默认 128
│  │  --follow                               持续读取，遇缺口或代次变化停止
│  │
│  └─ 所有 observe 子命令共有
│     --scope <目录>                         可选；默认按调用环境解析作用域
│     --json                                 可选；省略时输出文字
│
├─ 跨设备 relay
│  │
│  ├─ relay [status]                         同步状态
│  ├─ relay new                              创建同步组
│  │  --broker <URL> --password <密钥>      自定义 broker
│  ├─ relay token                            查看加入令牌
│  ├─ relay connect <token>                  加入同步组
│  │  --password <密钥>
│  ├─ relay on                               恢复同步
│  ├─ relay off [--all]                      停止同步；可同时通知所有已知设备
│  └─ relay daemon [start|stop|restart]      relay 后台进程；无子命令查看状态
│
├─ 配置与集成
│  │
│  ├─ profile list [--json]                 列出保存的 profile 和默认标记
│  ├─ profile show <名称> [--json]           查看保存的定义
│  ├─ profile create <名称> --tool <工具> [--description <说明>] [-- 原生参数…]
│  ├─ profile update <名称> [--description <说明>] [--replace-args -- 原生参数…]
│  ├─ profile delete <名称>                 删除未被默认引用的 profile
│  ├─ profile default [名称 | --unset] [--tool <工具>] [--json]
│  │                                        查询、设置或清除全局／工具默认
│  │
│  ├─ config                                 查看有效配置及来源
│  ├─ config <key>                            读取配置
│  ├─ config <key> <value>                    写入配置
│  ├─ config <key> --info                     查看某个配置项说明
│  │  --json / --edit / --reset             JSON / 编辑器 / 归档并恢复默认
│  │
│  ├─ config -i <name|self> [key] [value]    Agent 级配置
│  │  key：timeout、hints、subagent_timeout
│  │
│  ├─ config environment [--json]            环境文件来源与变量溯源，不展示值
│  ├─ config env_files <JSON字符串数组>      有序 dotenv 文件列表
│  ├─ config env_override true|false         是否覆盖继承的环境变量
│  ├─ config bootstrap show [part]           查看引导上下文各部分
│  ├─ config bootstrap render <name> <tool> [launched]
│  │                                         渲染完整引导上下文
│  │
│  ├─ hooks [status [tool]]                  查看 hooks 接入状态
│  ├─ hooks add [tool|all]                   安装持久 hooks
│  └─ hooks remove [tool|all]                移除 hooks 和历史安装
│
└─ 脚本与维护
   │
   ├─ run                                     列出可用脚本
   ├─ run <脚本> [参数…]                     执行工作流脚本
   ├─ run docs [--cli|--config|--scripts]     内置参考文档
   ├─ update [--check]                        更新；--check 仅检查
   ├─ reset                                   归档会话并清空数据库
   └─ reset all                               停止全部、清空数据库、移除 hooks、重置配置
```

## 通用选项与环境

这些选项受对应入口的规则约束；不要由“通用”推断每个独立入口都会解析所有选项。传递原生参数时，AgentX 选项都必须在分隔符前。

| 选项／环境变量 | 作用与边界 |
| --- | --- |
| `--help` / `-h` | 查询 AgentX 帮助；启动分隔符后的同名参数属于原生工具 |
| `--help-all` | 从当前命令递归展开全部公共帮助；可用于任意命令层级，分隔符后或作为选项值时属于数据 |
| `--iam <身份>` | 普通 Agent 命令的调用身份；新建时代表调用方，不是新 Agent 的名字；部分独立入口另有规则 |
| `--go` | 跳过启动预览等确认门槛 |
| `--env-file <路径>` | 可重复；按给定顺序加载 dotenv 文件，必须位于 `--` 前 |
| `--env-override` / `--no-env-override` | 文件值覆盖／保留调用方已有环境变量 |
| `AGENTX_PROFILE` | 新建启动的默认选择；显式 `--profile` 或 `--tool` 优先，恢复/分叉不读取 |
| `AGENTX_DIR` | 选择数据与配置作用域，默认 `~/.agentx`；拥有 `--scope` 的命令按自己的规则选定作用域 |
| `AGENTX_ENV_FILES` | 逗号分隔环境文件列表；显式 CLI 文件列表替代此列表 |
| `AGENTX_ENV_OVERRIDE` | 环境覆盖策略；未显式配置时默认不覆盖 |

`--json` 由各命令单独支持，`--scope`、`--project` 也是对应命令的专属参数。环境文件的来源优先级与路径解析详见[环境加载契约](../.docs/architecture/environment.md)。

## 共享事件过滤器

用于 `events`、`events sub` 和 `listen`。同一个过滤选项重复通常按 **OR** 合并，不同选项按 **AND** 合并；不能混合要求不同事件类型的过滤条件。

| 类别 | 参数 |
| --- | --- |
| Agent | `--agent <名称>` |
| 事件类型 | `--type message|status|life` |
| 状态 | `--status active|listening|blocked|inactive|launching|error` |
| 生命周期 | `--action created|started|ready|stopped|batch_launched|launch_failed|launch_blocked` |
| 工具与操作 | `--context <模式>`、`--cmd <模式>`、`--file <路径或模式>` |
| 消息关系 | `--from <发送者>`、`--participant <参与者>`、`--mention <被提及者>`、`--reply-to <事件ID>` |
| 消息语义 | `--intent request|inform|ack`、`--thread <名称>` |
| 时间 | `--after <时间>`、`--before <时间>` |
| 快捷条件 | `--idle <Agent>`、`--blocked <Agent>`、`--collision` |

`--idle <Agent>` 展开为该 Agent 的 `listening` 状态，`--blocked <Agent>` 展开为该 Agent 的 `blocked` 状态。消息目标支持 `@luna` 精确名字、`@luna:设备ID` 远端名字；本地模糊名字会被拒绝。

## 配置键与现有简写

| 类别 | 主要配置键 |
| --- | --- |
| 启动 | `terminal`、`<tool>_args`、`auto_trust_workspace` |
| 上下文 | `hints`、`notes`、`bootstrap_exclude`、`gemini_system_prompt`、`codex_system_prompt` |
| 行为 | `auto_approve`、`auto_subscribe`、`timeout`、`subagent_timeout`、`claude_delivery`、`copilot_delivery` |
| 显示与环境 | `name_export`、`title_mode`、`env_files`、`env_override` |

默认工具参数与命令行原生参数的合并沿用[工具参数覆盖规则](../src/integrations/launch_args/overrides.rs)：显式参数覆盖已知的同名单值默认选项，未知或可重复选项保留顺序；这些规则不改变 `--` 两侧的归属。

| 类别 | 简写或别名 | 规范形式 |
| --- | --- | --- |
| 恢复／分叉 | `r` / `f` | `resume` / `fork` |
| `--tool` 的值 | `agy` | `antigravity` |
| `--tool` 的值 | `cursor-agent` | `cursor` |
| `--tool` 的值 | `kilocode` | `kilo` |
| `--tool` 的值 | `pi-agent` | `pi` |
| `--tool` 的值 | `omp-agent` | `omp` |
| `--tool` 的值 | `qodercli` | `qoder` |
| `--tool` 的值 | `grok-build` | `grok` |
| 上下文包 | `bundle preview` | `bundle prepare` |
| Hook 管理 | `hooks install` / `hooks uninstall` | `hooks add` / `hooks remove` |
| Relay | `relay disconnect` | `relay off` |

工具值别名只用于 `launch --tool <值>` 等需要工具标识的位置，不恢复工具名直启语法。这里记录既有简写；本次不新增旧启动语法的兼容层。

## 使用边界与查阅备注

| 项目 | 边界 |
| --- | --- |
| 原生参数范围 | 模型、模型服务、审批策略等由各原生工具定义，无法在 AgentX 的固定参数表中穷尽；必须放在启动命令的 `--` 后 |
| `--stdio` | 仅支持一个本地 Codex 前台进程；不能组合多实例、`--headless`、`--json`、`--device`、`--terminal` 或 `--no-run-here` |
| `--json` 启动结果 | 不能同时要求 `--run-here` 或 `--terminal here`；结果输出不能被交互终端占用 |
| 恢复与分叉 | 工具从已有会话推导，不增加 `--tool` 或 `--count`；具体可用选项还受操作和工具能力约束 |
| `start` | 接入当前 AI 工具，与 `launch` 创建新 Agent 的职责不同 |
| `stop` / `kill` | 前者断开 AgentX 连接，后者结束进程 |
| 两种 daemon | `daemon` 管理原生 AgentX 服务，`relay daemon` 管理跨设备 relay 进程 |
| 独立入口 | `work`、`workbench`、`observe`、`observe-web`、`daemon` 有自己的作用域和身份规则 |
| 帮助发现性 | 此前核对发现 `help work show` 等返回父级帮助，事件帮助未列全状态与 `--reply-to`；查阅目标以本图为准，执行能力仍需以实际实现和验证为准 |
| 可选性核对 | `observe --scope` 与 `observe events --window-file` 均可省略，已按[原生参数声明](../src/cli/commands/observe/args.rs)补充默认行为，避免将示例中的显式选项误写为必填 |
| 内部入口 | `pty`、`relay-worker`、`daemon serve`、工具 hook 入口及内部 `--batch-id` 不作为日常公共命令推荐；此前源码中的 `relay push` 等隐藏入口不在本次公共命令改造范围内 |

## 来源与后续对照

用户已授权实施具名启动配置；当前选择、覆盖和历史规则见 [Profile 使用说明](../.docs/launch-profiles.md)，实现与证据见 [Delivery](../.controller/launch-profiles/delivery.md)。本文的 profile 语法已同步；初始想法保留在[讨论草案](agentx-launch-profiles.md)。

本图的目标变更来自本轮用户确认；未调整的命令依据此前现状全景图，并参照以下源码与使用契约。源码和工作区可能处于改造中，不能仅凭本文件或某一个帮助输出判断全部目标已经落地。

| 范围 | 来源 |
| --- | --- |
| 顶层路由与通用选项 | [CLI 路由](../src/cli/router.rs) |
| 命令帮助与各命令选项 | [帮助定义](../src/cli/commands/help.rs)、[命令目录](../src/cli/commands) |
| 新建语法与工具选择 | [启动命令](../src/cli/commands/launch.rs) |
| 启动、恢复、分叉的参数边界 | [共享启动参数](../src/runtime/launch_options.rs)、[CLI 模块契约](../src/cli/AGENTS.md) |
| 工具能力与名称 | [工具注册表](../src/integrations/registry.rs)、[工具标识](../src/integrations/tool.rs) |
| 协作任务参数 | [work 参数定义](../src/cli/commands/work.rs) |
| 事件过滤器 | [过滤器定义](../src/events/filters.rs) |
| 环境文件 | [环境加载契约](../.docs/architecture/environment.md) |

后续应按此图统一解析器、帮助、内置脚本、TUI 启动调用、远端启动和示例。此文件最初作为目标参考保存；用户随后单独授权实现 `--help-all`，其实现边界见上方完整帮助模式，不能据此推断启动语法改造等其他目标的验收状态。若后续明确调整命令设计，应同步更新此文件，避免它继续展示过时语法。
