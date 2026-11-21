# 经典恢复 DOM 控件

保留既有独立恢复入口的 Button / Tabs / Badge 契约。按钮 busy 与 disabled 拒绝重复操作，Tabs 使用方向键；未知标记保持未知。运行时从 recovery.web.ts 消费，工作台场景使用相同真实组件。

工作台以 Shadow DOM 隔离经典主题，把原 CSS 的根作用域映射到预览宿主；这是一种展示适配，不是第二套控件实现。当前场景展示基础控件，不声称覆盖完整恢复业务或原生 App。领域查询、发送权限和恢复规则仍由产品拥有；不虚构为 Inspector RN kit 的已合并 API。
