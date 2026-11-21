# 真实设计系统与 React 展示平台

本轮已将四张画板迁移为 Vite + React + TypeScript 工作台，入口 `http://192.168.2.178:4190/`；Canvas 使用 `react-{frames,tools,signals,investigation}.html`。实现与检查见 [README](README.md)。

采用关系：共享基础 token → 语义主题解析器 → 同一生产 kit / Web 适配 → React 画板。Vite 只拥有开发、打包与展示布局。完整 Inspector 与工作台消费相同 theme、组件和 Web 反馈资源；没有复制 demo 按钮、表单或图表。

- `plugin/client/design-system/` 拥有 foundation、color、theme、skin、Panel；`tokens.ts` 和 `skin.ts` 保留既有公共导出。
- `web/client/appearance/` 拥有 CSS、原生工具指针、共享工具矢量、React 试敲、声音、偏好及清理；无领域副作用。
- `studio/` 拥有画板组合、明确的本地样例、语义变量面板；保持组件身份与输入状态。Canvas Tweaks 通过适配接到同一解析器。
- 切换改变语义主题与资源角色；颜色／尺度／时长属于变量，SVG path／音色属于资源。权限、未知结果、busy／disabled 由原组件与领域契约决定。
- 原静态画板和旧证据保留为历史；完整 Inspector 的正式 kit 与显式恢复入口的既有 DOM 实现仍有平台边界，不声称全部样式已完全统一。

完成检查：共享主题与偏好定向检查、plugin 和 Web 类型／构建、工作台 Vite 构建、实际浏览器场景。实际执行结果与限制记录在 README 和 evidence，不进行插件 reload 或服务重启。
