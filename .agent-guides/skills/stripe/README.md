# Stripe 官方 Skill 参考

这三个 Skill 来自 [stripe/ai](https://github.com/stripe/ai)，保留官方原文及引用文件。实际使用从 [共享 Stripe 指引](../../domains/stripe/AGENTS.md) 按场景选择；后续任务通过共享场景索引选择这些资源。

| Skill | 用途 |
| --- | --- |
| [stripe-best-practices](stripe-best-practices/SKILL.md) | 支付 API／组件选择、订阅、测试环境、支付方式、Webhooks 与安全；包含全部六个官方引用文件。 |
| [stripe-docs](stripe-docs/SKILL.md) | 使用 `stripe docs` 查文档、搜索和API参考，结合CLI help确认实际命令。 |
| [upgrade-stripe](upgrade-stripe/SKILL.md) | API、服务端SDK、Stripe.js与Webhook版本的核对和升级。 |

来源提交：[`15e3db59ba17ff4ff08d787333ea803b47f38674`](https://github.com/stripe/ai/tree/15e3db59ba17ff4ff08d787333ea803b47f38674/skills)，上游提交日期2026-10-12；本地取得日期2026-10-13。完整路径、Git blob和SHA256见 [sources.json](sources.json)，许可见 [LICENSE](LICENSE)。本地目录内容通过 Git blob 与固定上游逐文件核对。

官方持续更新入口：[Skills说明](https://docs.stripe.com/skills)、[机器可读目录](https://docs.stripe.com/.well-known/skills/index.json)、[官方插件](https://docs.stripe.com/agents/plugin)。这里是随 guidance 分发的固定参考，未安装全局插件或配置MCP，不会自动更新。Skill里的命令、版本和示例仍需按本地入口核对当前文档与SDK；当前已发现的版本差异也记录在该入口。

更新时将选定的新提交下载到临时目录，比较三个Skill及其引用文件，核对许可证与本地兼容说明，再更新原文和 `sources.json`，最后检查链接。不要在这些原文中直接修改项目约束，也不要用批量安装无关Skill代替选择。此集合未采用Stripe Directory、代付、云服务开通、Connect专用等其他Skill。
