---
schema: task
status: completed
agent: doma
description: Build the first AgentX Role backed by a provider-native browser automation MCP.
---

The user requested a concrete Role for browser exploration, page-state inspection and automation-script generation using the AgentX Profile -> Provider configuration model. The selected MCP is Microsoft Playwright MCP, pinned to `@playwright/mcp@0.0.85`; Chrome DevTools MCP remains a future optional diagnostic companion, and Browser Use is not selected for this local Role.

The [portable bundle](../.agent-guides/local/browser-explorer/README.md) owns the complete Role and AgentX Profile. It lives in a versionable location because `.agentx/`, `.codex/` and the installed shared role directory are ignored. The project adopted the bundle under its local `.agentx/` scope, preserving the selected default profile. The current launch arguments configure Playwright MCP directly, retain the original model and sandbox selection, and require no named Codex profile. The earlier local native profile file remains available for launch history that still uses it; it is not part of the bundle.

Verification: local adoption matched both bundle definitions. With only those two files in a temporary project, AgentX dry-run resolved the complete role and exact stored argv. Codex 0.170.0 accepted that argv and listed both the configured Playwright MCP and a pre-existing base MCP, with no named Codex profile file in the temporary home. This verifies configuration transport, not a browser run or authenticated workflow.
