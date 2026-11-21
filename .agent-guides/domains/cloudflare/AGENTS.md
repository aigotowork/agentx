---
schema: document
description: Route Cloudflare tasks to official skills and project CLI or API workflows
---

# Cloudflare

Use this domain for Cloudflare product selection, implementation, deployment, account operations and troubleshooting. Select all matching rows; read only the selected Skill and the references needed for the task. For ordinary UI work, use [web development](../webdevelopment/AGENTS.md); compose both domains when platform behavior is involved.

## Select by task

| Situation | Read/apply | Expected result |
| --- | --- | --- |
| Choose products for an app, API, storage, AI, networking or security requirement | [cloudflare](skills/cloudflare/SKILL.md) | Product choice and relevant product references; skip this survey when the product is already selected |
| Set up project tools, authenticate, or discover a command/API | [Operations](operations.md) | A supported execution path, project version and explicit resource target |
| Develop, configure, deploy, preview or roll back Workers/Pages; manage bindings, KV, R2, D1, Queues, Vectorize or Workflows | [wrangler](skills/wrangler/SKILL.md), plus [operations](operations.md) before remote commands | Correct project command, config, environment and verification |
| Write, review or debug Worker code and production configuration | [workers-best-practices](skills/workers-best-practices/SKILL.md) | Runtime, binding, error and lifecycle decisions grounded in the project's versions |
| Implement persistent coordination, SQLite state, alarms, RPC or WebSockets | [durable-objects](skills/durable-objects/SKILL.md) | Durable Object ownership, persistence and tests |
| Build stateful AI agents, scheduled work, streaming chat, or application tool integrations | [agents-sdk](skills/agents-sdk/SKILL.md) | Agents SDK implementation and application-owned tool connections |
| Start, migrate or deploy Next.js on Workers; choose vinext/OpenNext | [nextjs-on-cloudflare](skills/nextjs-on-cloudflare/SKILL.md) | Existing adapter preserved during unrelated work; retrieve matching vinext upstream guidance when needed |
| Maintain Sandbox on stable `@cloudflare/sandbox` | [sandbox-stable](skills/sandbox-stable/SKILL.md) | Stable API and image line retained |
| Build or maintain Sandbox on `@cloudflare/sandbox@next` | [sandbox-next](skills/sandbox-next/SKILL.md) | Preview API and image line selected explicitly |
| Port an existing Sandbox app from stable to preview | [sandbox-migrate-to-next](skills/sandbox-migrate-to-next/SKILL.md) | Deliberate migration and cutover; do not apply merely because preview is newer |
| Configure or debug Email Sending, Email Routing or email delivery | [cloudflare-email-service](skills/cloudflare-email-service/SKILL.md) | Domain, binding and delivery configuration; read its CLI/API references before account operations |
| Add, repair or migrate CAPTCHA to Turnstile | [turnstile-spin](skills/turnstile-spin/SKILL.md) | Widget and server-side Siteverify integrated into the existing flow; preserve its credential-handling prerequisites |
| Audit page loading, interaction performance or Core Web Vitals | [web-perf](skills/web-perf/SKILL.md), then [browser evidence](operations.md#browser-performance) | Measured findings with actual tool coverage stated |
| Design or troubleshoot Access, Gateway, WARP, Tunnel, DLP, CASB or Cloudflare WAN | [cloudflare-one](skills/cloudflare-one/SKILL.md) | Identity, network, policy and rollout decisions |
| Assess or migrate VPN, SWG or SASE from another vendor | [cloudflare-one-migrations](skills/cloudflare-one-migrations/SKILL.md), plus [cloudflare-one](skills/cloudflare-one/SKILL.md) for target design | Source policy mapping, parity gaps and staged migration |
| Manage DNS, CDN/cache, WAF, certificates, account settings or a product without a matching Skill | [operations](operations.md#account-and-zone-operations), then the [product directory](https://developers.cloudflare.com/directory/) or [API reference](https://developers.cloudflare.com/api/) | Exact product documentation and a supported CLI/API/IaC operation |
| Diagnose Worker logs, deployments, builds or analytics | [operations](operations.md#diagnostics-and-builds), plus the affected product Skill | Evidence from the correct environment and time window |

## Execution and source boundaries

- Use existing project CLI commands first, then REST or an official SDK for operations they do not cover. Read [operations](operations.md) when choosing or changing the execution path.
- Inspect package versions before selecting Sandbox stable/preview or framework guidance. For a new Sandbox project, make the package-line choice explicit before applying either Skill.
- Keep the official Skill bundles unchanged. [Source identity](sources.json) records their upstream revision; their references can still become outdated. Retrieve current docs and inspect installed types/help when implementing version-sensitive behavior.
- Downloading a Skill does not execute it, authenticate an account, authorize a deployment, or install its optional dependencies. Apply setup and persistence steps only to the actual task and supplied authority. Do not duplicate this managed bundle into another skill location merely because an upstream workflow offers persistence.
- For Turnstile credential-bearing helper commands, follow its canonical approved Wrangler executable/version requirements; the ordinary project-local Wrangler preference does not override those requirements.

If the user specifically wants an MCP client connection or an existing connection is needed for the task, read [optional Agent integrations](refs/agent-integrations.md). Ordinary development and deployment do not require that setup. Product work that implements an MCP server/client still belongs to the Agents SDK Skill.

For use and source maintenance, read the [README](README.md). The [resource catalog](.xmeta.yaml) identifies this domain's owned artifacts. Skills are reached through this entry; nested placement alone does not register them in an agent's native Skill catalog.
