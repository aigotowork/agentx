# 可复用 Agent 指引

[AGENTS.md](AGENTS.md) 按工作情境选择指引；[编写方法](guidance-authoring.md) 说明如何维护和组合内容。

根级 `scenarios/` 保留通用、跨领域及尚未归域的可用场景；`roles/` 保存职责；`domains/` 聚合专业方法；`skills/` 保持能力包的原生结构；`refs/` 携带支持资料。这些分类也可在领域或场景内部按需使用，少量独立主题可以平铺。

消费项目在 `.agent-guides/local/` 维护自己的补充规则、场景与资料，通过其 AGENTS.md 明确适用条件和引用关系。安装升级保留 local 的全部内容，不自动覆盖或合并同名指引。模块自有引导由模块入口链接。

新方法先判断归属和成熟度：项目特定方法写 local，通用成熟方法进入产品源码，不确定的推演留在 `.sketches/`。每份正文只有一个维护来源。

## 完整方法，按情境使用

正式方法包含 [产品设计](domains/product-design/AGENTS.md)、[管理型产品与工作台设计](domains/management-product-design/README.md)、[建模](domains/modeling/AGENTS.md)、[数据与查询](domains/data-query/AGENTS.md)、[Web](domains/webdevelopment/AGENTS.md)、[TUI](domains/tui/AGENTS.md)、[VSCode](domains/vscode/AGENTS.md)、[Cloudflare](domains/cloudflare/AGENTS.md) 和 [治理内容编写](domains/harness-authoring/AGENTS.md)。产品设计入口将十二个专业主题映射到各自正文；已有专业解释保持单一 owner，不要求每次读完全部领域。

跨领域使用 [功能场景](scenarios/feature-scenarios.md)、[可观察反馈](scenarios/observable-feedback.md)、[Skill 研究](scenarios/skill-research.md) 和 [发布与恢复](scenarios/release-and-recovery.md)。方法本身不要求作者研究仓库；业务场景、证据和 local 接法归消费者。安装了工具或读到一个例子，不等于已运行、已验证或获准执行外部效果。

[专业职责入口](AGENTS.md#professional-responsibilities)连接开发、契约、原型、设计系统、产品体验及角色与方法构建。[产品体验](domains/product-experience/AGENTS.md)补充渐进模型与原生执行/报告交接，[角色与方法构建](domains/role-methods/AGENTS.md)组合既有专业能力和内容编写方法；二者复用原需求、契约及工具 owner，不建立平行注册表或测试引擎。

工具的准确 CLI/API、依赖、平台和资源随对应发行组件维护。按发行清单找到它的使用说明与公开声明，再为实际场景提供显式输入、数据权限和输出根。缺少外部前提时保留方法与制品，报告受影响能力；不把完整携带误写成全部同时激活，也不把历史例子冒充新项目验收。

在 harness-kit 中，按固定组件身份查找 manifest 的 documentation resource：`model-check`、`source-basis`、`model-inspection`、`model-viewer`、`sdk-generator`、`contract-evolution`、`orpc-projection` 对应建模与合同；`query-model` 对应数据分析；`observable-evidence` 对应独立观察适配器。文档由各组件的 `docs/tools/<id>` 资源携带，SDK/method 组件不自动具有同名 CLI。纯 guidance 发行不包含这些运行程序；组合发行需逐项满足完整制品和依赖检查。
