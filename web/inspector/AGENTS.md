Current live HTTP boundary uses named public `@agentx/client` operations through `host/live/client.ts` and `host/live/inspection.ts`. `host/live/native.ts` derives live types from public `ManagementTypes`. Mock/demo supply remains explicit and independent. Long revisions/timestamps/counters stay bigint through pending journals and result views; Number conversion is confined to the representable Date display range. See [Web adoption Task](../../.controller/web-sdk-adoption/web.task.md), [package setup](../../packages/agentx-client/README.md#web-consumers) and [sole integration evidence](../../.controller/web-sdk-adoption/integration.task.md). Root, standalone SDK and Inspector locks/dependencies are required; build the public SDK before standalone typecheck/build. `?managementUrl=URL` explicitly selects a separate management endpoint; default is the embedded listener.

# Inspector module

For actual API integration and maintenance, read [current consumer Task](../../.controller/live-ui-integration/tasks/inspector.md), [team ownership](../../.controller/live-ui-integration/AGENTS.md), [README](README.md) and [native live model](../../.docs/inspector/live-model.md). The live delivery supersedes historical offline-only restrictions within its selected scope; historical migration evidence remains separate.

| Current responsibility | Owner/entry |
| --- | --- |
| Existing appearance shell and explicit demo isolation | `web/client/inspector.tsx`, `demo-surface.tsx`, `main.tsx` |
| Live native transport, exact reference/data keys and request journal | `host/live/client.ts`, `model.ts`, `operations.ts` |
| Inspector finite HTTP adapter and quantities | `host/live/inspection.ts`; authoritative [native declaration](../../src/inspection/contract.rs) |
| Generated Inspector types | `host/live/native.ts` aliases public SDK types |
| Generated management types | Public `web/management/api/native.ts` type-only imports; no private sibling implementation dependency |
| Eight live destinations and retained session/collection/analysis/recovery blocks | `plugin/client/live/`; layout in `live/layouts/` |
| Unknown mutation reconciliation | `plugin/client/live/use-mutations.ts`; native settings CAS/jobs/episode barriers own durable effects |
| Shared browser primitives without fixture initialization | `host/ui.tsx`; demo RPC/settings hooks remain in `web/client/host/adapter.tsx` |
| Fresh integration artifact | `build.mjs /absolute/nonexistent/output`, `/inspector/` base and license/notices; final build belongs to integration |

Default source is live, same-origin `/api/v1`; `source=demo` explicitly selects historical fixture supply. Never fall back to mock on API failures. The standalone old static server has no live API. Preserve running :18971, existing dist and all unrelated services. Consumer checks are `npm run typecheck` and `node tests/runtime-read-repair.mjs`; independent HTTP/browser/native acceptance belongs to the active Delivery. Do not repeat passed broad checks without affected changes.

Live facts retain scope/generation and exact AgentRef (current incarnation or stopped_event), native status, optional fields, sampled coverage, settings revision and receipts. No identity conversion from labels/session IDs; no guessed timestamps, health, turn success or zero-filled usage. Exact token counters are decimal strings. Missing latest samples never reuse old counters; repeated cumulative polls/shared sources are not summed. History starts at collection enablement, never backfilled. Analysis is explicitly deterministic evidence summary. Recovery queues a guarded same-episode continuation; accepted/queued never means model execution or recovered. Unknown mutation outcomes permit read reconciliation only.

The remainder describes the historical offline migration and its explicit demo consumer, not default live support.


Read [migration ownership](../../.controller/inspector-migration/frontend.md), [implementation state](../../.controller/inspector-migration/frontend-implementation.md), [README](README.md), and [current model](docs/migration.md). This module is isolated; do not modify adjacent Web apps or running user services. Original migration tests/evidence belong to the completed verification task. Current selected-scope checks and support are owned by [scope cleanup](../../.controller/inspector-migration/scope-trim-implementation.md); quota, money and scheduling are removed.

| Responsibility | Owner/entry |
| --- | --- |
| Browser composition and alternate recovery | `web/client/main.tsx`, `web/client/inspector.tsx`, `web/client/recovery/` |
| Domain presentation and eight tabs | `plugin/client/surfaces.tsx`, `plugin/client/dashboard.tsx` |
| Pure domain/schema/view calculations | `plugin/shared/` |
| Preserved server-independent algorithms | `plugin/core/`; [algorithm inventory](docs/migration.md#portable-core) |
| Finite typed provider and mock identity | `host/contracts.ts`, `host/provider.ts` |
| Memory effects, correlated datasets, acquisition replacement | `plugin/mock/operations.js`, `core-projections.ts`, `recovery-memory.ts` |
| Canonical design system and assets | [Inspector design](uiux/systems/inspector/inspector.design.md), [Inspector system](uiux/systems/inspector/README.md), [Recovery design](uiux/systems/recovery/recovery.design.md) |
| Copied pure token engine and resource declarations | `uiux/framework/`; preserve its adjacent license and notice |
| Original file provenance | `docs/source-inventory.json`; original source stays read-only |

Run `npm ci`, `npm run typecheck`, `npm run build`, then `npm start` from this directory. Default isolated port is 18971; `INSPECTOR_PORT` overrides it. `scripts/check-contracts.ts` is a finite development schema probe (`node_modules/.bin/tsx scripts/check-contracts.ts`). Final execution/evidence is owned by the linked verification Task. Do not rerun passed broad checks without an affected change or concrete uncertainty.

Keep original/pixel independent from light/dark; theme changes must preserve component identity, selection and drafts. Preserve source sequence ranges, coverage, metric units, unknown data and delivery-vs-execution distinctions. Every exported TSX declaration requires an adjacent `@aiPurpose`/`@aiUsage` usage contract. Follow root interface guidance for compact, resource-first layouts and contextual disclosure.

Mock data and effects must remain visibly simulated. Never add live storage, generic RPC transport, implicit fetch fallback, or real mutation to fixture paths. The native adapter must use actual AgentX references, dataset generations, source health and command outcomes from [data foundation](../../.docs/inspector/data-foundation.md), not map display names or old provider session IDs into incarnations. Live supply is owned by `host/live/` and `plugin/client/live/`; `host/provider.ts` owns explicit demo supply only. Do not conceal unsupported facts in schema defaults. Local identity metadata describes only this mock dataset.

Source `.docs` copies are historical research, not current API promises. UIUX `.scenes.tsx` and workbench adapters are preserved design-authoring resources; the maintained product and `?surface=components` run without the original workbench application. Standalone workbench authoring tooling is not installed or verified here.

Current immediate sending uses `messages.tsx`, `message-state.ts` and `shared/messages-rpc.ts`; no deferred execution queue remains. `archive-candidates.ts` owns read-session archive eligibility. `handover.ts` owns explicit Provider handover independently of removed scheduled continuation. Historical design scenes and source research may still describe discarded capabilities; do not reintroduce them from snapshots.
