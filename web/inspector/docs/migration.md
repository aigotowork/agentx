---
schema: model
status: implemented
owner: inspector-ui-beta
related: [../AGENTS.md, ../../../.controller/inspector-migration/frontend-implementation.md]
---

# Inspector migration model

The original finite transplant retained the complete source composition. The current product follows the selected [scope refinement](../../../.controller/inspector-migration/scope-trim.md); original migration evidence remains historical. [Source inventory](source-inventory.json) records original file hashes before import/copy adaptations. The original checkout is read-only; source `server/` gateway, SDK packages, credentials, logs, journals, generated bundles and installed dependencies are excluded.

## Object and dependency boundary

`web/client` composes `plugin/client` surfaces, which consume `plugin/shared` descriptors through `host/adapter` and canonical `uiux/systems` components. `host/provider.ts` selects a local memory implementation. `plugin/mock/operations.js` has a finite operation switch; operation inputs and outputs are parsed against the retained schemas. No generic transport, HTTP, WebSocket, old SDK runtime or native daemon client is available. Static serving rejects `/api/**`; the alternate recovery's path-shaped keys are local dispatch identifiers only.

Directory observations, metric contributions, timeline evidence, recovery incidents/cycles/attempts, read receipts and analysis reports retain their distinct types. Mock identity is `{dataset: agentx-inspector-demo, generation: demo-page-v1, source: simulation}`; those values and historical display IDs are not native AgentRef/incarnation guarantees. All state belongs to this page. Reload resets business state and unknown barriers together; only presentation preferences may persist in new browser keys.

## Portable core

| Source algorithm | Local consumer | Retained behavior / adaptation |
| --- | --- | --- |
| `server/stats-facts.ts` / `stats-types.ts` / `stats-snapshot.ts` | `plugin/mock/core-projections.ts::coreStats` | Retained point and interval facts, per-Agent interval union versus host busy union, additive activity, frozen attribution, current live context, range/timezone grouping, basis/coverage, streaks and comparisons. Browser UTF-8 byte sizing replaces Buffer. |
| `server/tokens/admission.ts` / `tokens/data.ts` / `tokens/snapshot.ts` | `core-projections.ts::coreTokens` | One canonical contribution per key; replacement/duplicate/conflict decisions, bounded valid counters, event-time projection, model/source/provider/project/session grouping, speed percentiles, range/retained scopes and independent directory health. SHA-256 uses the audited general-purpose `@noble/hashes` browser implementation; browser byte sizing replaces Buffer. Local directory DTO replaces native acquisition class. OS temporary-directory discovery is deferred; browser mock groups explicit POSIX temporary roots only. |
| `server/turn-content.ts` / `digest/{fold,commands,model,activity,aggregates,highlights,tokens,build}.ts` | `core-projections.ts::coreDigest` | Bounded content metrics, command classification without executing commands, compact incremental fold, replay protection, turn/tool/time attribution, checks/files/highlights, once-only token attribution and bounded digest payload. `path-browserify` preserves path operations; local structural timeline types replace SDK-only types. |
| `server/recovery.ts` / `backoff.ts` plus `shared/domain/recovery*` | `plugin/mock/recovery-memory.ts` | Original recovery eligibility, source baselines, final delivery guard, serialized reservations, exact operation dedupe, unknown blocking, policy scope and retry transitions. Browser random bytes replace Node UUID. Memory store commits before local delivery. No timer starts real execution; accepted delivery is never invented running/completed evidence. |
| `shared/model`, `attention*`, `read-state`, `dashboard-model`, `timeline`, `digest-view`, stats/token views, recovery rules and client stores | Original client surfaces | Preserved normalization, parent/child tree, attention and completion semantics, stale/read guards, stable sequence windows, filters, view calculations and comparison. Acquisition hooks bind to local observations. |

Default statistics, token totals and per-Agent digest use correlated finite synthetic facts and the original projectors. These are sample calculations, not native AgentX telemetry. Rich source stress datasets remain selectable (`metric-comparison`, `data-basis`, `token-ledger`, `digest-rich`); illustrative live-rate curves and analysis reports remain explicit synthetic supply. Per-Agent digest token attribution correctly leaves calls outside its timeline window unassigned. The historic source server acquisition, incremental file readers/parsers, checkpoint persistence, native event admission and scheduled/background workers remain deferred because their facts/lifetimes need AgentX-owned adapters; retaining their IO would violate this phase's offline contract.

## Presentation retention

All eight tab keys remain: `agents`, `board`, `tokens`, `analytics`, `compare`, `activity`, `insights`, `recovery`. Original `surfaces.tsx` also registers archives, agent/workspace detail, diagnostics, insight jobs, language and team observation. Canonical Inspector and Recovery systems retain components, overlays, tokens, themes, backgrounds/icons/cursor/sound, motion/preferences, resource/scene/design files and component preview. The pure UIUX token engine/resource declarations are snapshotted locally with MIT notices, avoiding a sibling checkout dependency. Source authoring adapters that use the separate UIUX workbench remain resources, not an installed authoring application.

The former external Bots resource UI remains as a simulated team/role/runtime observation view. Its internal contract and route names are retained for source correspondence; visible copy refers to team resources. It does not assert equivalence between former Bot definitions, conversations and AgentX native incarnations. That semantic/API mapping is deferred to [data foundation](../../../.docs/inspector/data-foundation.md).

## Finite scenarios

| Input | Expected local behavior |
| --- | --- |
| default | Correlated payment Agent tree, permission request, unread result, metrics, event data and recovery incidents; all actions stay in memory |
| `scenario=approvals`, `send-unknown`, `send-lost-reply`, `permission-stuck` | Approval/quick reply controls, lost receipt or uncertain permission outcomes; no native effect |
| `scenario=recovery-unknown`, `recovery-not-sent` | Existing unknown/not-sent attempts, final guard evidence and no replay of unresolved reservations |
| `scenario=unknown-origin`, `long-history` | Unconfirmed result origin, or large paginated timeline retaining source range/epoch semantics |
| `scenario=directory-unknown` / `directory-stale` / `directory-unavailable` | Directory health is independent of token-ledger coverage and cannot authorize effects |
| `scenario=data-basis&basisCase=unknown` | Metric basis remains unknown/partial rather than displaying false precision |
| `scenario=token-ledger&ledgerCase=pressure` | Capacity/paused ledger projection remains explicit |
| `scenario=insight-jobs&jobsCase=partial` | Representative queued/running/completed/failed/canceled/skipped analysis jobs and partial history |
| `botsCase=partial` / `denied` | Team resource relationships remain partially available or inaccessible |

Unsupported operation names reject. Local creation adds a resolvable simulated directory object and checks an explicit idempotency key when provided; it does not execute a model. Simulated sending can start a simulated active turn; no model output is fabricated as a result. Recovery receipt simulation intentionally stops at acknowledgement. Policy/settings edits change only local state. Analysis start/feedback/cancel changes local jobs, never starts a runner. Archive/restore affects the same local identity and preserves receipt origins.

## Evidence ownership

Implementer: source/type/import checks and the finite schema probe. Independent executor: production build/browser/network/navigation/theme/viewport/action evidence under `tests/**` and the linked verification result. A schema pass proves only the finite checked values, and a copied resource does not establish full design-authoring runtime support. See [implementation](../../../.controller/inspector-migration/frontend-implementation.md) for current state and [verification](../../../.controller/inspector-migration/verification-result.md) for executed evidence.

## Current selected scope and boundaries

Account quota/reset/forecast cards, monetary usage/analysis fields, schedule definitions/cadence, heartbeat wakeups, scheduled continuation and deferred execution/run history are removed from current executable product paths, provider cases and fixtures. Eight main tabs remain. Immediate message delivery is owned by `client/messages.tsx`, `client/message-state.ts`, `shared/messages-rpc.ts` and `mock/messages-fixture.js`; it retains confirmations, final guard checks, idempotent receipt lookup and unknown-outcome barriers, with no enqueue/cancel/resume operation or background flush timer. Ordinary Agent archive/restore, stop evidence, explicit Provider handover, timeline/tool evidence, recovery audit and analysis jobs remain distinct. Archive eligibility now checks directory completeness and live/unread descendants without scheduler reads.

`uiux/**` scene/design snapshots and source `.docs` are historical authoring/research resources, not current runtime promises. Generic domain inventories may retain historical vocabulary; production bindings do not register discarded operations. `source-inventory.json` retains original hashes and adds currentDisposition/currentSha256 for removed, renamed, adapted and historical records. Original hashes are never rewritten as current hashes.

Monitoring history starts at collection enablement; old backfill is out of scope. Missing Provider facts remain optional. The source recovery/backoff mock is retained, while native sustained-error duration (configurable; example/default proposal 10 minutes) remains a separately documented future rule in [data foundation](../../../.docs/inspector/data-foundation.md#可配置持续错误恢复方向尚未实现). It requires native episode/instance/action-phase rechecks and a supported runtime action; `error` never grants ordinary PTY injection. No native/API actions or real scheduling were added. Current execution evidence is in [scope cleanup](../../../.controller/inspector-migration/scope-trim-implementation.md).
