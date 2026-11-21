# 鲜彩像素换肤 · SVG 视觉候选

本轮依据用户对上一版像素体验的反馈，先评审视觉，不实施产品。保持完整 Inspector 的现有结构与功能，把探索范围收窄到配色、图标、轮廓、转角、阴影及控件状态。当前 `plugin/` 与 `web/` 没有改动。

Paseo Design Canvas 名称：**Agent Inspector · 鲜彩像素换肤**；slug：`inspector-pixel-skin-v2`。[局域网浏览入口](http://192.168.2.178:4188/)与真实产品 `4187` 分开。所有画板使用固定设计样例，按钮为静态 SVG，没有业务操作。

| 画板 | 视觉稿 | 保留的原版结构 |
| --- | --- | --- |
| 01 恢复 · 明亮 | [SVG](svg/recovery-light.svg) / [画板](recovery-light.html) | 八页签、自动状态、计数及筛选、搜索、Project → Workspace → Agent、行内详情及原会话入口 |
| 02 恢复 · 深色 | [SVG](svg/recovery-dark.svg) / [画板](recovery-dark.html) | 与明亮版使用同一绘图函数、内容与坐标；只替换颜色角色 |
| 03 会话 | [SVG](svg/sessions-light.svg) / [画板](sessions-light.html) | 原状态筛选、360px 左侧列表、子 Agent 层级、右侧未选择态及搜索 |
| 04 看板 | [SVG](svg/dashboard-light.svg) / [画板](dashboard-light.html) | 时间窗、来源健康、摘要、八项指标、年度活跃与成长档案；仅描绘指定首屏 |
| 05 色板与控件 | [SVG](svg/visual-system.svg) / [画板](visual-system.html) | 主次操作、禁用、状态、像素图标、明暗配色与保留范围 |
| 06 手机恢复 | [SVG](svg/recovery-mobile.svg) / [画板](recovery-mobile.html) | 图标页签、原工具栏顺序、项目分组、Agent 行及底部状态；390×844 |

明亮版使用宝蓝、青绿、亮黄、珊瑚红和紫色，搭配白色阅读面及深蓝文字；深色版使用深蓝底与更明亮的强调色。饱和色集中在当前页签、主操作、状态和数据表达上。中文使用正常字形，像素特征来自 16×16 整格 SVG 图标、阶梯转角、控件轮廓和短硬阴影。

基准来自 2026-11-13 对 `http://192.168.2.178:4187/` 的实际只读浏览：会话、恢复、看板与手机恢复。另查看用户给出的 `?experience=recovery`，确认它是单独的恢复体验，结构与完整 Inspector 有差异。本稿以完整 Inspector 为结构基准；示例中的计数与对象仅用于构图，不随服务更新。

## 来源与评审边界

[build.mjs](build.mjs) 是本候选唯一绘图源，内含色彩角色、SVG 图元、像素图标与各页面函数。它生成 `svg/` 中的纯矢量文件及自包含 Canvas HTML；没有位图嵌入、`foreignObject`、运行时依赖或业务请求。SVG 文字与图形可编辑，Canvas 元素使用 `data-dc-id` 标记，HTML 最后加载 `tweaks.css`，供原生评审保留用户调整。

重新生成：`node design/inspector-pixel-skin-v2/build.mjs`。`.generated.json` 保护已生成文件中的直接编辑；检测到修改会停止覆盖。生成器只读现有 manifest 以保留画板位置，不写批注及 tweaks 文件。

已检查六张画板及独立 SVG 的实际渲染、颜色角色、无横向溢出和 390px 构图；明暗恢复使用同一结构。明亮版饱和填充按钮与深色版选中按钮使用各自的前景颜色，避免亮色底配白字。以上是视觉稿检查，不代表完整交互、所有页面、所有字体缩放或产品实施验收。恢复详情、设置和确认仍按原产品入口保留，本轮没有重新设计其业务流程。

先根据用户批注确定色彩与像素强度；实施方案另行讨论。上一版 [像素案头](../inspector-pixel-v1/README.md) 保留为历史探索，不作为本轮导航或布局依据。
