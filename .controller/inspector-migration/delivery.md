---
schema: delivery
status: completed
owner: lube
description: Preserve the complete Agent Inspector experience in AgentX with a runnable offline provider and evidence-based future data mapping.
---

# Agent Inspector migration

Current scope has since been reduced by explicit user decision: [completed scope refinement](scope-trim.md) removes quota/reset, monetary estimation and scheduling/run-history capabilities. The acceptance below records the original transplant; the follow-up owns current executable scope and affected checks. Existing LAN preview serves the refined build.

The user requests bringing the core system and entire UI from `/home/long/works/longwork/paseo-retry` into this repository, removing Paseo data calls and replacing product concepts with AgentX capabilities. Missing HTTP APIs do not block this phase: use explicit mock supply, assess whether underlying facts exist, and leave real API wiring to a later change. The source currently includes sessions, recovery, dashboard, tokens, details, comparison, event stream, insights, supporting execution/diagnostic/schedule/archive/job surfaces, shared UI resources, original/pixel skins and light/dark themes.

## Responsibility and acceptance

| Contribution | Owner/task | Consumer and completion boundary |
| --- | --- | --- |
| Complete source/UI transplant, local contracts and mock behavior | Fresh coder, [frontend](frontend.md) | Runnable isolated `web/inspector/`; preserve source experience, no Paseo runtime/data dependency; design inventory, usage and extension boundary included |
| AgentX foundation and missing-data/API assessment | Fresh domain-contracts specialist, [data foundation](data-foundation.md) | Frontend/future API implementers can distinguish present facts, derivable projections, missing persistence and deliberate simulation using source links |
| Finite final build/browser checks | Fresh coder, [verification](verification.md) | Verify actual retained navigation, themes, representative drills/actions, mock truth, narrow layout and absence of old runtime calls |
| Integration and discoverability | lube | Root/module entries point to implementation, capability assessment and executed evidence; retain honest deferred API boundary |

A dedicated UI migration context and an independent native-data assessment are justified because each requires a different domain model and can advance concurrently. Keep one cohesive frontend owner, including ordinary repairs; do not split screens among agents. Existing HTTP implementation and `web/agent-space`, `web/management`, `web/workbench`, `web/observability` remain untouched. Root package changes require coordination and should be avoided through a module-local package. Do not copy node_modules, credentials, databases, build outputs, vendor runtime SDKs or old server integration. Required licenses/provenance remain.

## Closed bindings

Frontend: `inspector-ui-beta` / base `beta`, coder, batch `967690d1`; source and ordinary repairs accepted, [implementation](frontend-implementation.md). Data assessment: `inspector-data-fine` / base `fine`, domain-contracts, batch `06ee05b7`; read-only assessment accepted, [record](data-implementation.md). Verification: `inspector-check-veru` / base `veru`, coder, batch `eb867774`; final execution/cleanup accepted in message #22481, [verification result](verification-result.md). All three assignments are closed at their declared boundaries. They used `gpt-6.1-sol` with `max` reasoning and scoped writes; no existing agent or delegated subteam was assigned work.

## Accepted local outcome

Lube accepts the full offline source/UI and portable-core transplant into [web/inspector](../../web/inspector/README.md), the [data foundation assessment](../../.docs/inspector/data-foundation.md), and finite independent Linux Chrome execution. The module retains eight main tabs and supporting routes, shared components/resources, original/pixel skins, light/dark themes and the separate recovery/component-preview experiences. Original recovery/backoff, statistics, token admission/projection and digest-fold logic run against correlated memory facts. The local provider is the future integration seam, not a public native API contract.

Scoped install/typecheck/build passed. S01–S12 are supported by the retained browser runs and dependency evidence: 2,544 production inputs including 18 portable-core files; no old SDK/runtime package/import or outside-checkout production dependency; no page-fatal errors or business/external requests. Default analysis/report/job correlation and narrow simulation-label defects were repaired and affected cases passed. [Verification](verification-result.md) owns exact evidence, initial failed attempts, source hashes and limitations; this acceptance consumes that evidence without repeating checks.

No substantive defect remains in the executed offline scope. Real AgentX API integration, durable business state, real recovery/AI/scheduling/quota supply and independent UIUX authoring runtime remain deferred. Native facts, projection gaps and missing workflows are separated in the data assessment. Mock display IDs are not native incarnations; accepted local recovery is not execution success. Original source, adjacent apps, installed programs and live user services were preserved. Executor browser/server processes exited, port 18971 was confirmed closed and disposable artifact copy removed; no persistent preview is left running. Start the built module with `cd web/inspector && npm start` when needed.

## Known risks and dispositions

## User-requested LAN preview

After offline acceptance, the user explicitly requested starting a LAN preview. Lube started the existing built module with `node scripts/serve.mjs` from `web/inspector`, exec session `78507`, bound to `0.0.0.0:18971`. The numeric LAN URL `http://192.168.2.178:18971/` returned HTTP 200. This is a user-owned persistent preview; preserve it. It serves the explicit local mock application and does not connect to a native API or daemon. The earlier executor cleanup remains historical evidence; this separately requested preview is intentionally left running.

Usable source was handed off in message #22012. Module [README](../../web/inspector/README.md) and [AGENTS](../../web/inspector/AGENTS.md) route the complete transplant, portable algorithms, local provider, source/license inventory and future API boundary. Independent final acceptance is recorded above; earlier static checks remain distinct evidence.

The old product's word “Agent” includes provider sessions and external Bots resources. The data specialist owns identity/state mapping; a name or conversation ID must not silently become an AgentX incarnation. Token/cost, quota, persisted attention, scheduled execution and recovery policy may require derived or new facts even if an Agent list exists. The UI must preserve unknown/unsupported distinctions and label simulated data/actions locally.
