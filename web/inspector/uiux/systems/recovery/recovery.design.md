# 独立恢复 DOM 系统

原 `?experience=recovery` 的 DOM 控件、语义 CSS tokens、原创图标与浮层物理迁入本目录；保持既有行为与接口，不冒充 Inspector RN kit 的同一实现。业务状态仍由 web/client/recovery 拥有。公开入口为 recovery.web.ts，CSS 由 foundation.tokens.css 与 kit.styles.css 提供。

工作台登记真实经典控件与图标，recovery.preview.tsx 将同一 CSS 的根作用域映射为 Shadow DOM 宿主，防止与 Inspector 和工具外壳互相影响。当前预览覆盖 Button／Tabs／Badge／Icon；完整恢复业务和原浮层验收仍由产品入口承担。
