# AgentX 命令交互导览

状态：已完成，已通过本地 Chrome 离线交互检查。所有演示使用本地教学数据，不连接 AgentX 服务。

直接用浏览器打开 [index.html](index.html)，无需安装依赖或构建。主入口提供命令关系、协作过程、入口模式、诊断过程及完整帮助。禁用 JavaScript 时可阅读页面内核心比较和 [帮助快照](help-source.txt)。

## 读者与结构

目标是回答「此时应使用哪个命令，它操作谁、意味着什么、接下来如何继续」，尤其区分任务事实、运行事实和浏览器访问模式。采用紧凑的导航＋SVG 关系图＋就地解释；详细规则与命令语法渐进展开。无后端、外部字体、CDN 或运行时数据访问。选择原生 HTML/CSS/JS 是因为固定教学模型不需要 React 或可视化平台。

| 文件 | 责任 |
| --- | --- |
| `index.html` | 可访问页面骨架、静态核心解释 |
| `style.css` | 布局、设计变量及窄屏适配 |
| `catalog.js` | 源码核对的解释、模式、场景与生成的帮助数据 |
| `app.js` | 导航、SVG 投影、筛选和确定性教学演示 |
| `help-source.txt` | 开发构建完整公共帮助快照 |
| `refresh-help.py` | 从显式指定的本地二进制刷新帮助数据；仅调用 `--help-all` |
| `verify.cjs` | 离线浏览器消费者检查；使用外部已有 Playwright 与 Chrome |

## 教学模型与验收

协作演示用同一任务串联保存、分派、提问、答复、结果、检查请求、检查报告和明确接受。失败分支保留旧结果并在退回后形成新结果；教学步骤不表示产品只有一个线性状态枚举。切换场景和重置恢复完全相同的数据。SVG 连线表示责任或请求路径，不表示实际网络部署拓扑。

模式对照可切换 `workbench` 只读／协作和 `observe`／`observe-web`；必须可观察「业务只读」与「本地是否获取服务」是两个轴。诊断步骤保留历史配置与当前重算的差别、缺口与时代变化，不暗示自动重放。关键验收：正常／失败协作路径、模式切换、命令筛选／深链接、键盘与 390px 窄屏、离线无网络请求。

2026-11-14 检查结果：JavaScript 语法通过；本机 Chrome 实测正常路径接受、失败路径退回／新结果／再检查／接受、重置、只读／协作／被动观察切换、历史／当前配置解释、搜索和刷新后的深链接、SVG Enter 选择通过。五个视图在 390×844 无页面级水平溢出；1440×1000 桌面及窄屏截图人工检查，无脚本错误或 HTTP(S) 请求。该检查验证教学页面，不验证 AgentX 业务执行、所有屏幕阅读器或全部浏览器。页面内命令没有被执行。

维护时使用 `python3 .sketches/cli-explorer/refresh-help.py /absolute/path/to/agentx` 更新帮助快照。浏览器检查使用 `PLAYWRIGHT_MODULE=/path/to/playwright node .sketches/cli-explorer/verify.cjs`；可用 `CHROME_PATH` 覆盖默认 `/usr/bin/google-chrome`。它不安装依赖；截图写入 `/tmp/agentx-cli-*.png`。

## 来源与已知差异

帮助快照于 2026-11-25 用当前开发构建的 `target/debug/agentx --help-all` 刷新，包含独立的 `docs` 和一次性提醒指引。实际命令来自 [路由](../../src/cli/router.rs)、[Work CLI](../../src/cli/commands/work.rs)、[工作领域契约](../../src/collaboration/service.rs)、[资格规则](../../src/collaboration/model.rs)、[Workbench 启动](../../src/collaboration/web/mod.rs)、[Observe 参数](../../src/cli/commands/observe/args.rs)；使用含义参考 [协作 Native](../../.docs/collaboration-workbench/native.md)、[诊断 CLI](../../.docs/observability/cli.md)、[Profile](../../.docs/launch-profiles.md)、[Daemon](../../.docs/architecture/daemon.md)。发布/跨平台支持由这些文件链接的验收记录拥有。

历史检查发现：同为 `0.7.31`，当时 `.bin/agentx` 的完整帮助没有 `profile`，开发构建有；该历史观察不代表后续重新打包的产物。不能仅靠版本号判断命令可用性，也不将 `.sketches/agentx-cli-panorama.md` 的目标描述作为执行证据。本导览不替换现有程序或运行服务。
