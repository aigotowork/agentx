---
schema: document
description: Cloudflare 领域使用、CLI 选择和官方技能快照维护
---

# Cloudflare 领域

[AGENTS.md](AGENTS.md) 是按工作情境选择技能的入口；[操作指引](operations.md) 说明项目工具准备、认证、开发、部署与诊断。当前收录官方 `cloudflare/skills` 的全部 14 项技能及其配套资源，具体来源由 [sources.json](sources.json) 固定，原文采用上游 [Apache-2.0 许可证](skills/LICENSE)。

使用时先描述目标，例如“部署带 D1 的 Worker”“修改 DNS”“接入 Turnstile”“迁移 Zero Trust”。Agent 通过领域入口读取所需技能；无需预先加载整个目录。构建 harness 指引后，这个目录随发布物安装到项目的 `.agent-guides/domains/cloudflare/`，通过 `.agent-guides/AGENTS.md` 路由发现。这里的下载和 harness 安装不等于向各 Agent 的原生 Skill 列表注册，也不执行技能内附带的脚本。

## 默认使用 CLI 和 API

常见开发任务使用项目内 Wrangler：本地运行是 `npx wrangler dev`，目标和构建就绪后的部署通常是 `npx wrangler deploy`。既有项目优先用自己的脚本、包管理器和固定版本。

Wrangler 并不覆盖整个 Cloudflare 管理面。DNS、WAF、Access 等操作可以使用官方 REST API 的单条 curl 命令，较复杂自动化用官方 SDK，持续的基础设施管理用项目已有的 Terraform/Pulumi。Tunnel 连接器由 cloudflared 负责。具体示例、认证方式和边界见[操作指引](operations.md)。

网页性能需要真实浏览器测量；Lighthouse CLI 能提供实验室报告，但不等于所有交互 trace 和真实用户指标。若明确需要 MCP 客户端接入，再读取[可选 Agent 集成](refs/agent-integrations.md)，日常开发部署不需要该步骤。

## 更新官方内容

以 `skills/` 为原样上游快照；本地路由和工具选择写在领域入口、operations 或 README。更新时：

1. 查询官方仓库新提交与全部 `SKILL.md`，下载到临时目录，保留每个 Skill 的完整目录和许可证。不要直接用全局技能安装命令覆盖本领域。
2. 对照 [sources.json](sources.json) 的固定来源检查差异，包括新增/移除技能、引用资源、脚本、依赖、版本分流和 MCP 假设。只在匹配的真实场景中执行上游脚本。
3. 用核对过的快照替换旧 `skills/`，同步路由和来源记录；保留上游版权及 NOTICE（若出现）。需要本地改写时明确标注修改，不能继续称为原样内容。
4. 按作者项目的 harness 发布流程构建、检查安装计划并安装；验证消费目录和相对引用。嵌套目录自动发现由各平台决定，不在本领域猜测注册路径。

已知上游问题：当前 Turnstile README 的 “Step 11 — Persist the skill” 链接锚点已失效；实际内容见其 [Conversation flow](skills/turnstile-spin/SKILL.md#conversation-flow) 第 11 步。保留原文，更新版本时复查。
