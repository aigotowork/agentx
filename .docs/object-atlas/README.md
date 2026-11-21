# AgentX 对象图谱

双击 [index.html](index.html) 可离线浏览；共享预览为 [LAN :18987](http://192.168.2.178:18987/)。切换核心对象、管理资源或协作与观测即可看对应关系；按 `/` 搜索对象、字段和方法。检查器里的关系行可跨视图跳转，**总览**会复位。窄屏可用分区按钮移动画布，用**检查**跳到当前对象。**回执路径**是供比较的固定情形，可直接点选步骤或原生状态，没有真实提交或轮询。

[静态图版](companion.png)与交互画布共用关系图和 [Signal Engraving](visual-philosophy.md) 的图形语言。PNG 使用本地 Canvas Design 字体 Instrument Sans 与 JetBrains Mono（均为 SIL OFL），图版无需字体文件。单文件 HTML 自带数据、代码、CSS 和 SVG；配套 PNG 独立于工具。

图谱是本仓库源的选读索引。检查器提供可复制的仓库路径和定位行，不把仓库暴露给 LAN 静态服务。TS 对象层的实际交付边界见[独立验证记录](../../.controller/typescript-domain-sdk/implementation.delivery.md#accepted-first-slice-outcome)。管理、Workbench 和 Observation 的调用入口独立；图谱不授予权限，不读取服务，不触发操作，也不提供原生 Workspace 筛选分页。可复现检查、范围和屏幕证据见 [Atlas Task](../../.controller/object-atlas/tasks/atlas.task.md)。
