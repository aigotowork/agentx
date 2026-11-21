---
schema: scenario
status: active
description: Publish selective task documentation, discoverable Skills and accurate MCP setup
tags: [agents, documentation, skills, mcp]
---

# Discovery and caller experience

Use when making existing capabilities discoverable or packaging operating guidance. Begin with the caller's task and supported interface; do not require a domain rewrite merely to improve discovery.

## Give each entry a distinct job

| Entry | Initial information | Deeper material loaded when |
| --- | --- | --- |
| Product README / landing page | Purpose, installation and ordinary first task | User chooses detailed operation or maintenance |
| `llms.txt` | Short capability summary and annotated task links | Agent selects a caller/task or exact contract |
| Agent Skill description | What the Skill does and when to activate it | Host activates the Skill body |
| Skill body | Prerequisites, short workflow, boundaries and reference routes | Selected task needs an exact recipe |
| Caller recipe | Exact runnable commands/code, inputs, results and recovery | Task requires that caller/tool |
| Machine contract | Exact operations, fields, errors and deliberate mappings | Consumer needs schema or programmatic integration |
| Repository `AGENTS.md` | Maintainer responsibilities and context routes | Agent changes the project |
| Configured MCP server | Protocol-discovered tools and their schemas/annotations | Client calls a selected operation |

These entries compose; they do not imply one another. A documentation URL is not Skill installation, Skill discovery is not MCP registration, and a tool annotation is not user authorization.

## Publish a short llms index

The [llms.txt proposal](https://llmstxt.org/) describes a Markdown documentation entry, commonly at the website root. Use its recognizable shape when adopting it: title, brief summary and grouped annotated links. State capabilities and important operating boundaries, then route by task/caller to complete recipes and exact contracts. Do not inline every guide or expose internal agent instructions merely to make a larger index.

Treat `llm.txt` as an explicit compatibility alias if requested, keeping one canonical content source. It is not the proposal's canonical filename. Neither filename guarantees automatic loading, installation, authentication or permission. Add HTML/HTTP discovery links only where supported and useful; do not imply they compel a client to fetch the entry. Add a full-text bundle only when a real consumer needs it and its maintenance/distribution is justified.

For HTTP publication, define a narrow explicit public-file mapping and correct content types. Check the built distribution, link resolution and an unlisted-path rejection; source-checkout existence alone does not prove a deployed URL works. Keep repository guidance, secrets and arbitrary filesystem paths outside that publication mapping.

## Write complete caller recipes

One recipe owns a coherent task for a supported caller. Include installation/runtime/version prerequisites, starting service/endpoint, target selection, literal invocation syntax, placeholder meanings, expected machine result, relevant effects, cleanup and a meaningful recovery path. Keep architecture rationale in design documents and exact field definitions in the native contract/reference.

Do not leave essential steps as “configure appropriately” or mix incompatible interfaces. Distinguish native RPC from ordinary HTTP JSON, protocol connection from browser connection, and a selected object's existing data from a new temporary context. If a dependency cannot consume a runtime result through static configuration, say how the user supplies it and when reconnection is required.

Verify tool syntax against the selected version's official help/source, then distinguish that inspection from actual execution. A help flag's existence does not prove attachment, identity, persistence or shutdown behavior. Name unsupported modes instead of extrapolating from a similar tool. Use the [caller recipe scaffold](templates.md#caller-recipe) when starting a guide.

## Package a real Agent Skill

Follow the [Agent Skills specification](https://agentskills.io/specification) and the chosen host's current discovery contract. Give the canonical `SKILL.md` native `name`/`description` frontmatter, a concrete activation description, short operating steps and explicit reference routes. Put selectively needed details in `references/`; use `scripts/` only for an actual reusable executable capability. Do not register a directory by merely naming it “skills”.

Keep one editable Skill source and use supported discovery links or packaging to reach it. For example, the Browser Environments example used repository `.agents/skills/` (Codex) and `.claude/skills/` (Claude Code), including symlinked Skill directories supported by the selected hosts at validation time. Verify host/version behavior before applying those paths elsewhere. User and organization scopes have their own documented locations; do not silently edit global settings.

Choose a distribution policy deliberately:

| Distribution | Requirement | Evidence |
| --- | --- | --- |
| In-repository canonical Skill | Required references resolve in this checkout; host can discover the registered path | Inspect real path/metadata and selected host discovery where available |
| Repository aliases | Relative links resolve to the same source; host supports following them | Resolve each alias, avoiding edited copies |
| Standalone installed Skill | All required references/scripts are carried with the package; unavoidable external prerequisites are explicit | Inspect/use a copy outside the author checkout |

A symlink from `references/` to project docs can avoid duplication in a checkout. It can break in an archive or external copy. Materialize references during packaging, such as `cp -RL SOURCE DESTINATION` on systems supporting that option, or use the project's packaging tool. Select a fresh destination rather than merging stale files. Verify outgoing references from the packaged location, including frontmatter metadata as well as body links. Keep its version/update source discoverable. A Skill is not the application binary; document how it locates the installed product.

Host references: [Codex Skills](https://developers.openai.com/codex/skills) and [Claude Code Skills](https://code.claude.com/docs/en/skills). Discovery behavior is host-specific and may require reload; do not promise universal dynamic rescanning.

## Separate MCP protocol from client setup

MCP defines messages, capabilities and supported transports. Client products own configuration schemas, paths and startup/reload behavior. Publish examples for explicitly supported clients, with absolute executable paths, endpoint/environment configuration, prerequisites and placeholder replacement instructions. Label illustrative configurations clearly; do not distribute a live configuration containing unusable paths.

For stdio, launch the actual server entry without shell/script banners on stdout. Put diagnostics on stderr and define process lifetime independently from domain resource lifetime. HTTP transports, when supported, need their actual URL/authentication contract. Do not invent a universal `/.well-known/mcp.json`, HTTP `/mcp` endpoint or automatic remote registration.

Describe what the configured server exposes: tools for operations, resources for URI-addressable context, and prompts only when those capabilities are actually implemented. A domain resource label does not establish MCP resource support. Let clients use protocol discovery; a capability index supplements it rather than becoming a new protocol.

For multiple MCP servers, explain the concrete handoff. Static startup arguments generally do not evaluate shell substitution or consume another server's tool result. If one operation returns an endpoint for another tool, define who supplies it, how to refresh it and what disconnect/close does. Reuse [contract policies](contracts.md) for result/error semantics.

Official references: [MCP transports](https://modelcontextprotocol.io/specification/latest/basic/transports), [MCP resources](https://modelcontextprotocol.io/specification/latest/server/resources), [Codex MCP](https://developers.openai.com/codex/mcp) and [Claude Code MCP](https://code.claude.com/docs/en/mcp).
