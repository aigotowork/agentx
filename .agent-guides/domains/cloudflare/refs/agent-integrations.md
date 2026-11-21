---
schema: document
description: Optional Cloudflare MCP connections for tasks that explicitly select them
---

# Optional Agent integrations

Read this only when the user requests MCP setup, an existing MCP connection is needed for a specific task, or a concrete capability gap makes that integration useful. The default [operations](../operations.md) workflow uses CLI/API and works without MCP registration.

Cloudflare's [main MCP](https://github.com/cloudflare/mcp) provides developer-documentation search, API-schema discovery and API execution. [Product-specific servers](https://github.com/cloudflare/mcp-server-cloudflare) offer narrower tools. These are hosted HTTP services: a client with remote MCP support connects to the URL without starting a local Cloudflare server. Client configuration, authentication and possible client reload remain necessary; clients requiring a local bridge have additional runtime needs.

For requested setup, consult the current [official instructions](https://developers.cloudflare.com/agent-setup/prompt.md) and the client's documentation. Select only the necessary server and preserve existing settings; do not install every advertised integration by default. Keep account authorization explicit. Wrangler authentication and an MCP connection have separate lifetimes.

Upstream Skills may mention MCP tools or setup steps. Select their CLI/API or direct-documentation path for ordinary tasks. For browser measurements, use a tool that actually captures the required evidence; an HTTP fetch cannot replace an interaction trace. Implementing an application's MCP server/client is a separate product feature covered by the [Agents SDK Skill](../skills/agents-sdk/SKILL.md).
