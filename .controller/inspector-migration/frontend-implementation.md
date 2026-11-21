---
schema: implementation
status: completed
owner: inspector-ui-beta
description: Complete source transplant, local host/provider boundary and implementation handoff.
---

# Frontend implementation

This implements [frontend](frontend.md) within [Delivery](delivery.md). Source `/home/long/works/longwork/paseo-retry` is read-only. Writes are confined to `web/inspector/**` and this record; `tests/**` and `verification-result.md` belong to inspector-check-veru. No running services or root dependencies are changed.

## Model and structure before implementation

The product's resources are directory Agent observations, execution/timeline evidence, metric projections, attention/read receipts, recovery incidents/cycles/attempts, analysis reports/jobs, and schedules/runs. Their presentation contracts remain separate: accepted delivery never means successful execution; unknown source/history never means zero or a completed task. AgentX native incarnations and the mock's historical display IDs must not be equated. Future mapping is owned by [data foundation](../../.docs/inspector/data-foundation.md).

| Source scope | Target ownership | Decision |
| --- | --- | --- |
| `plugin/client/**` | `web/inspector/plugin/client/**` | Preserve complete surfaces and reusable view/state modules, including eight tabs, detail/timeline, comparison, insights, execution, diagnostics, schedules, archives and team observation. |
| `plugin/shared/**` | `web/inspector/plugin/shared/**` | Preserve pure models, normalization, attention/read-state, timeline projection, query schemas, metric/comparison, error classification, recovery/backoff and policy helpers. Replace SDK contract declarations with local typed schema descriptors. |
| `uiux/systems/{inspector,recovery}/**` | `web/inspector/uiux/systems/**` | Preserve all tokens, skins, appearance, motion, assets, overlays, native/Web components, design/resource/scene files and component preview. Exclude generated evidence and installed packages. |
| `web/client/**`, `web/shared/contracts.ts` | `web/inspector/web/**` | Preserve full Web host composition and alternate recovery UX; replace acquisition, SDK hooks and session assumptions with local provider. |
| `plugin/preview/*fixture*`, fixture i18n | `web/inspector/plugin/mock/**` | Seed correlated, explicitly simulated datasets and finite behavior; preserve unknown/error/idempotence scenarios. Remove preview's duplicate widgets; canonical UIUX owns widgets. |
| Portable `plugin/server/` algorithms | `plugin/core/` | Preserve recovery/backoff, statistics facts/projectors, token admission/projection and digest fold/build; browser-safe primitives replace Node primitives. See [algorithm inventory](../../web/inspector/docs/migration.md#portable-core). |
| IO server modules, vendor SDK, gateways, journals | Not copied | Runtime/storage/transport acquisition intentionally deferred. No generic proxy and no actual native mutation. |
| `plugin/LICENSE`, `plugin/NOTICE`, relevant `.docs` design | Module root / `docs/source/` | Preserve licenses and factual provenance; copy source design as historical references, not current API promises. |

The directory skeleton is materialized before filling it. `host/contracts.ts` owns presentation types and schema-based finite operations; `host/provider.ts` composes local fixtures and immutable identity metadata. `web/client/host/adapter.tsx` translates hooks/widgets into this boundary. No dependency on another Web app, root package, old SDK or server. Use module-local React 19, react-native-web, TanStack Query, Lucide, Zod, esbuild, TypeScript/tsx and Node static serving; these are existing source technologies rather than a new UI stack.

## Scenarios and checks

The planned independent coverage comprised eight tabs, chart/list/detail navigation, selected-agent and metric comparison, timeline/event filters, analysis/jobs, recovery/settings, archive/restore, supporting routes, original/pixel × light/dark, keyboard overlays, narrow viewports, alternate recovery and component preview. Provider cases include unknown delivery (never replay), idempotent same-key actions, stale target guards, partial history, directory/source health, rejected/unsupported operations and local reset. The implementer owns type/schema/import checks and repairs; final build/browser execution belongs to veru after exact-command handoff.

## Evidence and unresolved points

- Codebase-memory has no matching index for source or target; reachability is inspected directly from entry imports. No graph completeness claim.
- Source calls its provider sessions “Agents” and has a separate Bots observation surface. Display copy will identify these as simulated AgentX observations/team resources; no native incarnation claim is inferred.
- Source standalone recovery fixture rejects ordinary continue despite the UI supporting it. The local provider must implement coherent simulated receipts and explicit unknown outcomes, rather than silently inherit that gap.
- Local browser preferences may persist under new Inspector-specific keys. Business state is per page and resets on reload; the provider never reads old host databases, credentials or journals.

## Usable-source handoff

The usable-source handoff was executed and independently verified; see [final evidence](verification-result.md). From `/home/long/works/longwork/agentx/web/inspector`: `npm ci`, `npm run typecheck`, `npm run build`, then `INSPECTOR_PORT=18971 npm start`. No service was started by the implementer. Veru owns final build/browser/network evidence and `tests/**`; beta owns repairs.

Implementer checks: TypeScript passed (exit 0); finite schema probe `node_modules/.bin/tsx scripts/check-contracts.ts` passed with `stats=full`, `tokens=full`, `digest=full`, `recovery=accepted`, 14 Agents. The probe validates actual output schemas and replay of the same recovery operation. These are bounded implementation checks, not browser acceptance. Later edits affected copy/provenance and loading labels only.

The default metrics and digest now use preserved portable algorithms over correlated sample facts. Browser dependencies `@noble/hashes` and `path-browserify` replace Node hashing/path primitives; `react-native` supplies original native declaration types only, while production bundling aliases it to React Native Web. Original design proposals and UIUX source are retained as historical resources; standalone authoring tools and native App runtime are unverified. Module-owned README/AGENTS/model route current implementation, historical design, licenses and future API seam. Inventory records original hashes, including core and the pure UIUX framework snapshot.

Known limits: no live AgentX API, daemon, durable business persistence, model execution, OS directory discovery, real scheduler, quota acquisition, AI recap runner or authoring workbench. Quota/rate/schedule/report fixture facts remain visibly simulated. Accepted recovery delivery stops at acknowledgement. Native mapping must follow the separately owned [data assessment](../../.docs/inspector/data-foundation.md), especially incarnation identity, richer transcript facts and missing recovery baselines.

### Verification repair: default report discovery and completed identity

Veru #22207 found an empty default report list/overview and a completed fixture report showing an unrelated queued analysis. The default now enables the preserved correlated 12-report overview/list dataset. Status fixtures are always seeded as well, so the separate jobs surface remains populated under the default rich supply. The completed status case now belongs to `jobs-completed`, separating it from `jobs-agent-a`'s queued attempt; the source panel correctly reports active work for an Agent, so a synthetic identity collision had made that unrelated queue appear above the selected completed report. No domain panel behavior was changed.

A targeted provider-data probe passed: 12 reports, resolvable overview→report ID, completed status retained, and no active attempt sharing the completed fixture identity. Changed files were three mock JS modules only; the affected rebuild/browser recheck passed under S02/S05 and report-return evidence in [final verification](verification-result.md).

### Verification repair: narrow-screen simulation identity

Veru #22315 observed both desktop title/host labels shrinking to zero width at 390px. Below 700px, the compact host now places `AgentX · 本地模拟` in its own unshrinking text row immediately above the existing toolbar. Desktop labels remain on their original row; compact actions keep their space. This directly keeps simulation identity visible without hiding it behind an icon. Only `web/client/inspector.tsx` changed; TypeScript passed after the change. The 390×844 geometry and action reachability recheck passed under S10 in [final verification](verification-result.md).

## Completed frontend handoff

Independent executor closed verification on 2026-11-15 ([result](verification-result.md), message #22484). All S01–S12 finite cases passed after the two assigned repairs; no outstanding substantive defect remains in that executed scope. Production build and browser evidence are owned by that result, including four appearance combinations, eight tabs, report/detail/return, recovery acknowledgement without invented execution, unknown/partial facts, 390px reachability and absence of old SDK/network runtime. The verifier cleaned its disposable browser/server; no preview remains running. Lube accepted the offline Delivery using independent evidence #22481 and closed frontend, verification and parent Delivery in #22522. Root README/AGENTS now route the runnable module and data assessment; all assignments are closed.

Delivered consumer: isolated runnable `web/inspector/` with preserved UI/design resources and portable core, local typed finite AgentX mock provider, licenses/provenance, module guide and future API seam. Final commands and limitations remain in the module README/model; live integration and broader unexecuted combinations are deliberately outside this handoff. No files outside assigned implementation scope were changed by beta, and no subagents were launched.
