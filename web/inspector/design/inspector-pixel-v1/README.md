# 像素案头 / Agent Inspector

这是 Agent Inspector 的**推荐视觉与体验候选**。用暖纸、墨绿、正交像素图标与紧凑工作区，连接“找到 Agent → 看原因与证据 → 合适的操作 → 理解结果 → 返回”。设计不是生产改版；没有改动 `plugin/`、自动策略、真实消息或业务数据。

[独立评审入口](index.html) · [原生 manifest](design.json) · [恢复调查 HTML](recovery-investigation.html) · [恢复调查 SVG](svg/recovery-investigation.svg)

原生 slug：`inspector-pixel-v1`。Workspace：`wks_951ba811dd97c593`。在该 Workspace 的 Design Canvas 中打开 **Agent Inspector · 像素案头 01**。所有 HTML 已内联 CSS、图标、样例与交互脚本，无外部字体、网络或服务依赖，直接打开文件即可操作。

## 为什么是这个方向

参考来自 `../paseo-bots` 的画布内部，不是 Design Canvas 应用外壳：

| 参考与实际观察 | 提炼 | Inspector 的转译 |
| --- | --- | --- |
| `design/bots-team-office-v2/design.json`、`.docs/design/scene-system.md`、`team-office/source-analysis.md`；实际查看 `office-desktop.png` 与固定 Lobby 的 `lobby-surface-light.png` | 暖灰/暖白底、苔绿/土黄点色、阶梯物件轮廓、格纹分区、物件邻近披露 | 暖纸底、语义状态点、16×16 物件符号、选中行邻接检查器、低频操作局部 Sheet |
| `design/bots-zh-v1` 的普通中文与暖纸界面、`bots-identity-v1` 的角色/分身区别 | 文字可读、身份可识别；美术不是事实来源 | Agent 是调查主体；项目与 Workspace 显示真实从属；中文不用像素字体 |
| `shared/pixel.ts` 的整格 run 与低饱和色板、Lobby `client/pixel/palette.ts` 的主题/状态槽 | 轮廓稳定、主题与状态有分工 | 原创填充 path 图标，共享语义 token；不复制人物/家具美术 |

参考办公室的留白、走路和空间导航服务于探索，不能证明后台密度足够。Inspector 的用户需要同时核对对象、状态、证据与动作，因此使用队列与检查器。画板 01 用同一对象比较重像素与轻像素：推荐后者，保留中文和核验信息的视觉优先级。

## 任务、对象与布局

事实依据来自项目现行[领域模型](../../.docs/domain-model.md)、[数据架构](../../.docs/data-architecture.md)、[kit 与浮层责任](../../.docs/design-system.md)。[工作台体验提案](../../.docs/proposals/workbench-experience.md)中的导航分组与连续性仅作为设计方向，不宣称已在生产落地。

Agent 是操作目标；Host / Project / Workspace 是范围；轮次终止事件、目录观察、投递事实和日志贡献各自有来源。目录正常不证明 provider 日志完整，更不产生恢复授权。用户先判断对象及当前事实，再选择操作。

桌面为“轻导航 / 范围与摘要 / 队列 + 邻接检查器”；列表和证据各自滚动，底部动作持续可达。窄屏聚焦同一对象，保留标题、所属范围、证据与底部动作；返回恢复原查询。需要支持判断的长说明在展开区或 Sheet 中，必要的禁止补发原因保留在操作附近。

| 画板 | 页面任务与结构 | 入口 / 可编辑 SVG |
| --- | --- | --- |
| 01 · 1440×1040 | 视觉方向、颜色分工、字阶、图标、真实控件状态与表达强度比较 | [HTML](visual-language.html) / [SVG](svg/visual-language.svg) |
| 02 · 1440×960 | 关注摘要 → 搜索/筛选队列 → 选中对象；邻接关注提示 | [HTML](work-overview.html) / [SVG](svg/work-overview.svg) |
| 03 · 1440×960 | 两个调查对象；停止证据、来源展开、时间线、二级操作与继续确认 | [HTML](recovery-investigation.html) / [SVG](svg/recovery-investigation.svg) |
| 04 · 1280×900 | 未决投递；原操作 ID、执行观察、原会话和禁用继续 | [HTML](delivery-unknown.html) / [SVG](svg/delivery-unknown.svg) |
| 05 · 1440×1040 | 保留窗口、每日输入/输出、精确值、独立采集边界、按 Agent 下钻 | [HTML](usage-coverage.html) / [SVG](svg/usage-coverage.svg) |
| 06 · 390×844 | 同一调查工作区；长内容内部滚动、返回队列、底部动作与导航 | [HTML](mobile-workspace.html) / [SVG](svg/mobile-workspace.svg) |

画板 06 也可在 320×568 操作。窄屏默认在调查对象内；左上返回进入队列，底部巡检/恢复/用量/语言切换工作区。320 长标题观察使用额外压力文案，见 evidence；没有将普通默认标题假装成长标题证据。

## 视觉与组件语言

颜色和数值的候选源是 [src/tokens.json](src/tokens.json)，深色角色也在同一文件。Web 组件消费具名变量；结构局部变量归 [styles.css](src/styles.css)。SVG 从实际 HTML 的计算布局与颜色导出，因此不需要再手维护一份页面画稿。

- 暖纸 `#F5F3EC` 与暖白提供层次；墨绿 `#273E35` 承担文字；动作绿 `#315E46`、苔绿选择与土黄关注各有分工。陶土表示中断，紫灰问号表示投递未知，蓝灰表示投递已接受。状态同时有文字与符号。
- 字阶以系统中文 14px、紧凑标签 12/13px、标题 26px 为基准；数据使用等宽数字。格纸只用于强调区，不在长文本或表格底下铺纹理。
- 1px 线、4px 轻角、局部 2px 台阶阴影，避免复古游戏窗口侵占信息。仅少量图标/物件放大表现个性。
- 按钮有主/次/弱、禁用与焦点；状态标签、搜索、页签、复选、表格、图表与展开各有语义。未知不能只通过换颜色表达。
- [29 个 SVG 图标](icons/)由 [src/icons.mjs](src/icons.mjs)定义：16×16 整数网格、正交填充轮廓。终端表示 Agent、文件表示证据、柱形表示用量、问号表示未知。图标原创，无第三方美术或位图嵌入。
- Tooltip 在共享顶层浮层测量并钳位，支持 hover / focus / 触摸长按；按钮内部没有 absolute 提示。Sheet 保留 dialog、焦点约束、Escape 和焦点返回。触摸主操作为 44px；浏览器触摸模拟不等于原生触控验收。

## 可操作路径与状态

全部动作只修改当前 HTML 文档内的状态；刷新文档恢复样例。确认里的“离线回执分支”是评审工具，不是拟加入产品的业务控件。

1. 打开 [巡检](work-overview.html)，搜索“支付”；选中“整理支付模块的异常边界”。搜索只覆盖 6 个已加载样例。
2. 在详情内滚动并展开“来源与核验细节”；对象、范围与底部操作保持不变。可用 Agent 右侧菜单或筛选图标打开局部 Sheet。
3. 点击“继续原 Agent”，编辑跟进草稿，选择“投递未知”，模拟发送。状态短暂显示提交中，再显示原操作和未知反馈。
4. “刷新观察”仍保持未知；继续按钮禁用。查看原会话样例后按 Escape 关闭，再按 Escape 返回队列，保留查询。
5. 再选中该 Agent，未知操作仍保留，不能因关面板再发送。样例回执按 Agent 保存；草稿保留在本次文档中。
6. 在新打开的调查 HTML 中选择“宿主已接受”分支，可看“已接受，继续观察”。这不是恢复完成，继续按钮同样不可重复提交。
7. 打开 [用量](usage-coverage.html)，选 11/12，显示精确值 **1,809,752 Token**；展开保留/质量，再下钻执行中 Agent 并返回，日期、展开和来源滚动保持。

键盘 `/` 聚焦搜索；页签支持左右箭头、Home/End；Tab 有可见焦点；Escape 关闭局部面板或返回队列。触摸长按图标显示名称并消费本次点击。筛选、复选、局部菜单、来源展开、额度摘要、深浅主题均可操作。未展开设计的执行管理/分析/历史入口明确显示当前候选范围，不伪造新增业务能力。

## 到现行 RN kit 的适配取舍

这是候选表达，不新增平行生产 kit。已只读核对 `client/tokens.ts`、`kit.tsx`、`overlay.tsx` 与项目记载的 Paseo 0.15.3 边界。

| 候选 | 可保留的产品价值 | 进入 RN 时的选择 |
| --- | --- | --- |
| 暖纸/墨绿、轻边框、选择与状态分开 | 视觉秩序、低噪声、状态可辨 | 仍由 `PluginTheme → palette → kit` 供色；不能将 Web 色值覆盖宿主主题。候选半径与密度需在 canonical tokens 内统一决定 |
| 原创像素 SVG 图标 | 独特物件语言、统一语义 | 现行 `Icon` 只接名称。先映射已有宿主图标保留语义；自定义像素矢量需要另行验证官方原生资源路径，不在本任务声称可直接传入 SVG |
| 队列+检查器、底部动作 | 高密度调查与上下文连续性 | 组合现行 `Toolbar / SplitView / BottomBar`；选择、返回、数据可见性留在领域页面，不能塞进万能 Page |
| 搜索/状态筛选/Sheet | 就地收敛对象与二级操作 | 复用 `SearchControls / ChoiceGroup / ChipBar / Sheet / ActionMenu`。搜索与联动仍受控，临时筛选不修改后台策略 |
| hover/focus/长按 Tooltip | 图标名称可达，避免逐字竖排 | 直接复用 `OverlayBoundary / useTooltip` 的 owner/revision、宿主 Modal 局部边界与生命周期；不能移植本离线 DOM 脚本或页面 z-index |
| 接受/未知/执行观察分开 | 用户理解实际效果，不误补发 | 由恢复领域 DTO 与原发送守卫驱动；样例的本地字典不是持久 journal 或发送账本 |
| 用量与三个边界 | 统计依据可查，未知不补零 | 保留日志贡献、目录健康、外部额度各自所有者；不把样例汇总冒充真实查询或账单 |
| 窄屏主体与固定动作 | 小屏仍围绕当前 Agent 工作 | 用 RN 可用高度、safe area、字体缩放与 ScrollView 适配；不原样搬运 CSS breakpoint/viewport 高度 |

全产品功能继续由现行领域拥有；左侧分组是导航候选，未替换现有 tabs、深链接和宿主入口。原生是否采用更小圆角、像素图标或新导航仍需主控/用户评审，不把视觉候选登记成当前 runtime 能力。

## 已观察的证据与限制

完整短走查记录：[evidence/walkthrough.json](evidence/walkthrough.json)。这份记录支持离线交互事实，不是测试数量或真实服务验收。

| 设计问题 | 实际查看的画面与操作 | 结论 |
| --- | --- | --- |
| 队列和动作能否在同一桌面工作区 | [巡检](evidence/work-overview.png)、[恢复](evidence/recovery-investigation.png)；实际选择与滚动 | 首轮发现内容撑高容器导致动作出画板，已改为视口内队列/证据滚动；当前动作可见，更多对象/来源可局部滚动到达 |
| 像素是否压过信息 | [视觉体系](evidence/visual-language.png)与实际 bots / Lobby 参考 | 暖底和轻像素支持辨识；普通中文、对齐列表和精确数字保持主导。重像素仅作对照 |
| 未知是否会被读成成功 | [未知结果](evidence/unknown-after-send.png)、[已接受](evidence/accepted-awaiting-observation.png)；操作确认→刷新→返回→重进 | 问号/原操作/禁止补发与接受后的执行观察分开。关闭详情不解除未知屏障 |
| 提示与 Sheet 是否局部且可读 | [共享 Tooltip](evidence/shared-tooltip.png)、[确认](evidence/continue-confirmation.png)、[筛选](evidence/filter-sheet.png)；focus、Escape、长按 | 提示不挤压列表；焦点可见；确认有目标、后果、草稿和分支，Sheet 可关闭返回 |
| 390/320 是否保持主体和动作 | [390](evidence/mobile-workspace.png)、[320 长标题](evidence/mobile-320-long-title.png)、[320 确认](evidence/mobile-320-long-title-confirmation.png) | 没有横向溢出；标题换行；主操作高 44px。390 按钮底边 759、导航顶边 786；320 为 483 / 510。320 证据须内部滚动，不能宣称首屏展示所有事实 |
| 用量能否查精确值并连续返回 | [用量](evidence/usage-coverage.png)、[覆盖展开](evidence/usage-coverage-expanded.png)；按日选择→Agent→返回 | 日期、展开与滚动保留；执行中对象显示观察而不是伪造停止证据。当前窗口精确求和 8,420,000 Token，费用未知另列 |
| SVG 是否仍可编辑 | [矢量渲染](evidence/svg-vector-review.png)与实际解析 | 页面由 rect/path/text/clipPath 构成，无 image、foreignObject 或截图；保留可编辑文字。与 HTML 的轻阴影/细格纹可能有细微差异 |

深色视觉体系已截图检查 [dark](evidence/visual-language-dark.png)；没有完成全页深色、200% 字体、真实 iOS/Android、读屏或真实 provider/投递闭环验收。用量下方表格与完整证据可滚动，SVG 保留的是指定视口的静态构图。自包含 HTML 已另在 `sandbox=allow-scripts`、CSP 禁止网络的 srcdoc iframe 中操作确认→unknown（[截图](evidence/sandbox-srcdoc-unknown.png)）；这仍不冒充原生读取。原生 Design Canvas 的最终读取与用户体验接收由主控负责。

## 维护与重新生成

源入口：`src/app.mjs`（页面/组件/样例），`src/runtime.js`（本地交互），`src/icons.mjs`（图标），`src/tokens.json`（主题与数值），`src/styles.css`（布局与组件样式）。

```sh
node design/inspector-pixel-v1/build.mjs
node design/inspector-pixel-v1/export-svg.mjs
node design/inspector-pixel-v1/observe.mjs
```

`build.mjs` 生成自包含 HTML、图标和 manifest；`export-svg.mjs` 读取相同 HTML 的浏览器计算布局导出矢量；`observe.mjs` 只执行本目录短设计走查，复用 `plugin/` 已有 Playwright，无新增包或业务 runtime。

生成器保存已有原生画板坐标；`.generated.json` 检测 HTML/图标的用户直接编辑，发现差异即停止覆写。必须先将改动合并进源，再更新相应基线。生成器不读写 `comments.json / tweaks.json / tweaks.css`，本轮未创建或清理它们；不要通过删除基线绕过用户编辑。无需长期预览服务，本轮临时预览已关闭，所有 Playwright 浏览器均已释放。
