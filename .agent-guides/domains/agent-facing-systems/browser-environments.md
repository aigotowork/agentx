---
schema: document
status: active
description: Source-linked application of the method to Browser Environments
links: ["model.md", "contracts.md", "discovery.md", "maintenance.md"]
---

# Worked example: Browser Environments

Use this optional example to see the generic method applied to the originating Browser Environments system. This page records architectural choices and bounded observations rather than maintaining another operation reference.

All application and evidence paths below are relative to a separate Browser Environments checkout, not to Harness or an installed guidance directory. They identify the authoritative sources inspected in the originating project. That application and its raw receipts are not distributed with this domain; consult its current contracts before reusing exact operations or provider versions. The method and templates remain usable without the example checkout.

## Model to caller outcome

The architecture (`docs/architecture.md`) keeps Environment as the sole primary managed object. Hosts/settings support that object; process and connection information are observations. The manager owns persistent data and verified browser lifecycle. External browser tools receive CDP and own page interaction or attachment. A named tool session does not create a new isolated Environment.

The complete task is: select/create an Environment, obtain a reachable CDP endpoint, attach the chosen browser tool to that exact browser, perform the task, detach, then stop/remove through the manager when intended. Quick start (`docs/agent-quickstart.md`) and workflows (`docs/agent-workflows.md`) own the operational paths and recovery rules.

## Authoritative chain and projections

| Responsibility | Actual source / consumer |
| --- | --- |
| Public fields, validation, creation defaults and patch semantics | models.ts (`src/contracts/models.ts`) |
| Transport-independent service and stable domain failures | service.ts (`src/contracts/service.ts`) |
| Native oRPC operations and resource/effect/CLI/MCP metadata | contract.ts (`src/contracts/contract.ts`), maintained through contract guidance (`src/contracts/AGENTS.md`) |
| Repeatable derivation of six outputs | generate.ts (`scripts/generate.ts`); commands in package.json (`package.json`) |
| Native client and provider binding | generated client (`src/generated/client.ts`), generated server (`src/generated/server.ts`), public client entry (`src/client/index.ts`) |
| CLI registration and authored presentation | generated CLI (`src/generated/cli.ts`), CLI adapter (`src/cli/main.ts`) |
| HTTP schema and operation/effect map | OpenAPI (`src/generated/openapi.json`), capabilities (`src/generated/capabilities.json`) |
| MCP descriptors and authored protocol policy | generated MCP (`src/generated/mcp.ts`), server (`src/mcp/server.ts`), result schema (`src/mcp/result.ts`), MCP entry (`src/mcp/main.ts`) |

The generator emits native client/server glue, CLI registration, MCP descriptors, a capability index and OpenAPI for eleven declared operations. Handwritten domain behavior, Commander parsing/output, server setup and MCP transport policy stay outside generated files. Resource metadata groups Environment and supporting settings; it does not introduce separately managed process/connection objects or MCP Resources.

The projection has deliberate differences: flat CLI names remain compatible, OpenAPI has resource tags/stable identifiers, MCP has names such as `environment_connect`, and native RPC uses oRPC serialization while `/api` accepts ordinary HTTP JSON. CLI parsing and MCP object wrappers preserve the native operation meaning through explicit adapters. Input patches retain omission/null/false distinctions rather than inheriting creation defaults.

## Provider choice and an actual boundary correction

The fixed stack is oRPC1.16.2 + Zod4.6.7. Research found `@orpc/mcp@2.0.0-beta.44` and `orpc-mcp@0.1.3` require the oRPC2 beta line. The project selected official `@modelcontextprotocol/sdk@1.32.2` for a narrow compatible stdio adapter, while deriving descriptors from native schemas/metadata. Task decisions (`.controller/agent-experience.md`) and package declarations (`package.json`) retain the version/compatibility basis. This is a scoped choice, not a recommendation to retain SDK1 in every future project.

An actual SDK client rejected structured tool errors against the original success-only `outputSchema`, even with `isError:true`. The result projection (`src/mcp/result.ts`) now derives a strict union of `{result: nativeOutput}` and `{error: {code, message}}`, using native domain error codes. Generation and runtime share that policy. This demonstrates why generated schema consistency and real-client compatibility need separate evidence; domain errors did not need to change to fit MCP.

## Progressive discovery and portable guidance

| Caller need | Maintained entry |
| --- | --- |
| Choose a task | llms.txt (`llms.txt`); llm.txt (`llm.txt`) is its requested alias |
| Discover/activate a reusable operating capability | Canonical Skill (`skills/browser-envs/SKILL.md`) |
| Register in supported hosts | .agents Skill alias (`.agents/skills/browser-envs/SKILL.md`), .claude Skill alias (`.claude/skills/browser-envs/SKILL.md`), explained in discovery (`docs/agent-discovery.md`) |
| Invoke a selected browser caller | Playwright CLI (`docs/agent-playwright-cli.md`), agent-browser (`docs/agent-browser.md`), SDK/HTTP (`docs/agent-sdk.md`), MCP (`docs/agent-mcp.md`) |
| Understand derivation and maintenance | Generation (`docs/generation.md`), system explanation (`docs/agent-system.md`), task state (`.controller/agent-experience.md`) |

Skill references point to the canonical caller docs in the checkout. The documented external installation uses a symlink-dereferencing copy so references are carried with the Skill. Supported host aliases share one source; installation still requires a built manager and its runtime prerequisites. Public documentation is published through a narrow server mapping (`src/server/agent-docs.ts`), with source/compiled documentation cases (`src/server/agent-docs.test.ts`).

MCP is a configured stdio server over the existing manager client. It does not claim an HTTP `/mcp`, a universal discovery endpoint or automatic handoff to Playwright MCP. The MCP recipe (`docs/agent-mcp.md`) explains how to pass the fresh CDP result to the separate browser-automation server.

## Evidence and its limits

Agent-tool integration (`tests/agent-tools.ts`) exercises compiled management MCP → HTTP manager → real Chrome → selected external tool. The retained receipts show the browser survives each tested attachment cleanup, followed by intentional manager stop/purge:

| Pinned caller / decisive case | Evidence |
| --- | --- |
| `@playwright/cli@0.1.24`: same Environment, then `detach` retains the browser | Playwright CLI receipt (`.artifacts/agent-tools/playwright-cli-attempt2/results.json`) |
| `agent-browser@0.42.0`: explicit CDP attachment, then `close` retains the browser | agent-browser receipt (`.artifacts/agent-tools/agent-browser/results.json`) |
| `@playwright/mcp@0.0.85`: client disconnect retains the browser | Playwright MCP receipt (`.artifacts/agent-tools/playwright-mcp/results.json`) |

The MCP cases (`src/mcp/server.test.ts`) cover descriptor/schema exposure, invalid-input non-dispatch, real SDK success/domain-error behavior through HTTP and offline help. The lead reported these cases, generation freshness, type/build and source/compiled public documentation checks passing; final commands/results belong to agent experience delivery (`.controller/agent-experience.md`). Existing broader lifecycle scenarios remain in integration guidance (`tests/README.md`).

These observations apply to the selected versions and exercised local task paths. They do not prove every upstream shutdown mode, client host's automatic discovery, remote GUI login or future dependency version. Future changes follow [maintenance](maintenance.md) from the affected owner and rerun only evidence invalidated by those changes.
