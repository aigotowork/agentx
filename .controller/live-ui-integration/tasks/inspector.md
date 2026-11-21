---
schema: task
status: completed
owner: live-inspector-zolu
description: Integrate Inspector with real facts, collection and supported analysis/recovery.
---

# Inspector

[Team](../AGENTS.md), [design](../design.md), [Delivery](../delivery.md), [selected scope](../../inspector-migration/scope-trim.md) govern this assignment. Writes are limited to `web/inspector/**` and this Task; integration gabi owns native/generated contracts, hosting and final artifact. No delegation, Cargo/install/native/server writes or changes to running :18971.

## Current source supply and scenario matrix

The [older assessment](../../../.docs/inspector/data-foundation.md) predates office management. Current [native management declarations](../../../src/management/contract.rs), [HTTP dispatch](../../../src/management/web.rs), [office projection](../../../src/management/offices.rs), [worker](../../../src/management/worker.rs) and [generated bindings](../../../web/management/api/native.ts) are authoritative. Codebase-memory list_projects contains only the other xpm checkout, so this investigation uses targeted current source rather than unrelated graph claims.

| Retained scenario | Actual current supply | Classification / selected UI |
| --- | --- | --- |
| Locate current/stopped sessions and inspect identity/status | `GET /api/v1/management`, `/agents`, `/agents/:name` with generation and exact AgentRef | Existing exposed fact. Preserve current incarnation and stopped_event separately; encode full scope/generation/ref for browser keys. Native status stays literal; listening does not imply completed. |
| Group by workspace/Role; inspect office provenance | AgentView.collaboration and `/workspaces`, `/roles`, `/directory-entry` | Existing exposed fact; older E classification is stale. Use adopted/legacy/unavailable directly. Remove Bots catalog/Conversation/Skill authority from live UI. |
| Immediate message, stop, resume, recent transcript | Native POST `/agents/:name/{messages,stop,resume,transcript}` and GET `/operations/:id`; MessagePage | Existing operation. Preserve request ID, generation and exact target through receipt lookup; accepted/running/unknown are not success. No automatic resend after transport uncertainty. |
| Read model/tool/file evidence and usage | TranscriptPage currently has bounded exchanges/text/files; native [Transcript](../../../src/transcript/shared.rs) has tools/errors/usage | Existing facts needing safe exposure. Render native transcript as bounded recent sample; unknown tokens stay unknown, no fake sequence/timing/full-history guarantees. |
| Current counts, model comparisons | AgentPage and optional native usage | Derived current loaded-directory facts only. Merge redundant statistics into compact snapshot tables; do not invent success rates, duration or throughput. |
| Event/history/timeline and Token observations from collection start | [Events](../../../src/events/service.rs), [model request](../../../src/agents/model_request.rs), [request writer](../../../src/agents/model_request_store.rs) have native inputs | New durable Inspector collection ledger/coverage needed; history starts explicit enablement, no backfill. Expose source/observed time/gaps and bounded pages; event facts must not imply turns. |
| Analysis jobs/report evidence | No equivalent in current management Operation | New native persistent analysis execution/job/report; deterministic evidence summary acceptable if clearly labelled; not an invented AI result. Job needs generation, exact inputs, status, report and failure. |
| Configurable sustained-error recovery and audit | Native RequestError.error_since and ModelRequestState; ordinary send is insufficient | New policy/episode guard/reservation/audit executor. Must recheck current same-instance failed episode at action time, one outstanding attempt, unknown never repeats. Keep ten minutes configurable; no legacy backoff. |
| Completion read/permissions/archive/handover | No equivalent public native semantics | Unsupported/redundant live concepts removed. Stopped history/resume remain; no dataset reset, approval emulation, workspace archive or silent completion inference. |
| Appearance/language/device view state | Existing Inspector UIUX and browser-owned preferences | Retained local presentation; no claim of daemon persistence. Explicit `source=demo` retains historical fixture scenarios, never fallback from API failures. |

## Adapter design and skeleton

Dependency: native/generated TS → finite `host/live/client.ts` transport → `host/live/model.ts` exact identity/current-count projections → existing Inspector shell + native-backed business blocks in `plugin/client/live/`. Historical fixture `host/provider.ts` remains isolated behind explicit demo mode. Existing UIUX Inspector controls, overlays, themes and pixel/original resources are reused; no third application/page or generic RPC facade.

Live dashboard retains session, board, tokens, details, compare, events, analysis and recovery destinations where supply exists, consolidating duplicate metrics into shared current-snapshot views and showing unavailable fields as unknown. Session selection owns exact native reference; detail shows adopted workspace/Role, capability-gated ordinary commands, messages, bounded transcript and separate operation receipts. Collection/analysis/recovery blocks consume integration-owned generated contracts once signatures are agreed, without local public schema inventions.

Async model: descriptor scope/generation fences every page and operation; generation changes clear selected/draft/pending reads before new facts become actionable. Late reads are rejected by request epoch. Selection keys never use bare display name. Command submit stores request ID before fetch; lost submit response permits receipt GET only. A persisted uncertain command remains blocked until its receipt is resolved; unknown outcome cannot acquire a new request ID through retry. Native capabilities govern controls; transport failures never switch to fixtures.

Initial skeleton: `host/live/{client,model}.ts`, `plugin/client/live/{dashboard,session,collection,analysis,recovery}.tsx`, `plugin/client/live/layouts/workspace.tsx` composed inside existing `web/client/inspector.tsx`. Final boundaries may collapse tiny leaves; no empty abstraction layers.

## Contract requirements handed to integration

Need authoritative generated TS public entry and native route signatures before consuming new Inspector supply. Finite requirements: collection state/read/enable-disable + collected_from/observed_at/coverage; paged exact-ref observations/events and optional usage with units/source; analysis submit/job/report/reconciliation; recovery policy read/update and episode/attempt audit plus guarded recovery action if separate from daemon auto executor. All mutations generation/request-id fenced; policy must distinguish disabled/unavailable, recovery phase and unknown outcome. Report/usage evidence must preserve missing Provider support. Owner contacted early; delivery pending owner response.

## Verification plan

Scoped typecheck plus a focused adapter test covers namesakes/stopped refs, generation reset/late results, paging coverage, unknown usage, operation uncertainty and no fixture fallback. Build only to isolated output (existing dist is served by protected :18971). Native/runtime/browser acceptance remains integration/independent acceptance ownership. No live mutation or collection enablement on user scopes from this worker.

## Contract agreement and materialized structure

Native owner `live-native-sumo` published [authoritative skeleton](../../../src/inspection/contract.rs) and [live model](../../../.docs/inspector/live-model.md). Selected supply is a bounded Snapshot (settings/coverage/observations/jobs/attempts), Configure CAS, pinned deterministic analysis, and exact-episode continuation. Gabi owns public HTTP wrappers/generated `host/native.ts`; consumer has confirmed the skeleton. Sampled observations are labelled as such, not a complete native event journal. Recovery means an atomic durable native continuation mailbox acceptance, not restarting a process or recovering a Provider; UI must retain `queued` and successful-model-observation distinctions.

Materialized `host/live/client.ts` (finite management requests), `model.ts` (exact keys/current counts), `operations.ts` (persist-before-POST + receipt reconciliation), `plugin/client/live/session.tsx` and `layouts/workspace.tsx`. Live session uses authoritative management generated types, never native identity conversions. `build.mjs` accepts a nonexistent absolute output directory with `/inspector/` asset paths and copied license/notices; no build run by consumer per integration #24603. Existing scripts/build.mjs and dist/dev service remain untouched.

## Current implementation and resolved contract feedback

Live shell now composes native-backed sessions, current snapshot counts, coverage/configuration, decimal-string Token checkpoints, sampled events, pinned deterministic analysis and sustained-error continuation policy/audit. Existing Inspector tabs, UIUX controls/overlay/theme/pixel resources are retained; unsupported historical source-product features exist only behind explicit `source=demo`. Shared UI primitives were separated from fixture hooks so design-system imports do not initialize fixture state. `experience=recovery` selects the native recovery tab unless explicitly in demo mode.

Shared HTTP signature agreement is AgentX #24710: GET `/api/v1/inspection?generation=...` returns Snapshot; POST configure/analyze/cancel-analysis/continuation accepts the corresponding native tagged Request variant and returns its payload. Internal Collect/RunAnalysis are unavailable to browser callers. Exact quantities are generated decimal strings; no Number conversion. Consumer found the shared generator omitted `RequestPhase` serde snake_case (#248xx); gabi corrected the canonical generator and regenerated bindings. Unix-second timestamps, settings CAS, frozen input coverage and analysis cap 16 come from native ownership.

Initial `npm run typecheck` passed after coherent shell/session/collection/analysis/recovery implementation. Subsequent affected fixes and focused scenario tests remain below. No final build or native runtime mutation was executed.

## Scoped verification and remaining acceptance

`node_modules/.bin/tsx tests/live-adapter.ts` passed: exact large-string current/stopped/namesake refs; dataset scope/generation isolation; multi-page reads, repeated cursor and finite partial paging; wrong-generation directory/messages/snapshot/receipt rejection; persistence before POST and no POST on storage/fence failure; lost command response/reload barrier and GET-only Unknown reconciliation; latest missing usage does not reuse earlier known counters; u64 decimal display; agreed public path/tag calls; network failure without fallback. These are test-only HTTP fixtures, not actual daemon/browser evidence.

Typecheck passed initially. An affected follow-up check includes lazy demo-only recovery loading, generation validation and frozen report input coverage. Subsequent final code inspection corrected policy form revision after a successful save and kept refresh failures separate from already-known mutation receipts; final affected typecheck follows. An initial test command used a module cwd with root-relative paths and failed before writing/running cases; corrected invocation produced the passing result above.

Remaining acceptance belongs to gabi / the independent acceptance owner: fresh build through `build.mjs`, embedded `/inspector/` same-origin HTTP, actual scope read smoke, disposable collection/analysis/configuration/continuation and lost-response/reload/reset browser journeys, desktop/narrow and original/pixel continuity. No final build, Cargo, installed binary, server launch/restart, :18971 dist refresh or user Agent/collection mutation was performed by this consumer owner.

Final inspection also separated explicit HTTP 4xx refusal from lost/unknown command results. Refusals remain browser transport state rather than invented native Operations and allow a corrected deliberate submission; uncertain effects still permit original-receipt GET only. The affected adapter scenarios passed again with this case. Historical source SHA values remain intact; modified copied host/bootstrap entries carry updated adaptation hashes in the source inventory. No source write outside assigned consumer/Task ownership occurred (generated host/native.ts was written by integration).


## Consumer handoff / source freeze

Inspector consumer contribution is complete and released to gabi for integrated artifact and independent acceptance. Final `npm run typecheck` exited 0 after all behavior edits; final `node_modules/.bin/tsx tests/live-adapter.ts` exited 0. Both checks are local consumer evidence, not live deployment acceptance. No unresolved contract blocker remains.

Callable integration entry: `node web/inspector/build.mjs /absolute/nonexistent/output`; the output owns `/inspector/` paths plus LICENSE/NOTICE. Current product entry is the original Inspector shell; `source=demo` is the only fixture selector. Authoritative new native signatures are consumed from integration-generated `host/native.ts`, management signatures from the existing public generated file. Native/shared/Cargo/build/server and final HTTP/browser evidence remain with integration and the independent acceptance Task. No temporary server/artifact resources were created by this worker.

## Acceptance repair: management UUID admission

Gabi reported real browser 400s for all common management commands in AgentX #25347, with evidence under `verification/live-r1-browser3/browser`. The consumer helper emitted prefixed hex, while [native admission](../../../src/management/store.rs) requires `uuid::Uuid::parse_str`. Corrected `host/live/operations.ts::requestId` to use `crypto.randomUUID()` where available and a cryptographically random RFC UUID v4 fallback with explicit version/variant bits for plain LAN HTTP. The same helper continues to serve management and analysis callers; request journaling and receipt identity remain unchanged.

Extended the existing `tests/live-adapter.ts` with deterministic fallback byte/variant coverage and actual submitted bodies for messages, stop, resume and transcript. Each body must carry a valid UUID v4 equal to the identity journaled before POST; test HTTP returns Accepted only after these contract assertions. Ran `node_modules/.bin/tsx tests/live-adapter.ts` once after the repair: exit 0, `live adapter scenarios passed`. No broad check repetition, Cargo/build/runtime or native changes. Source frozen again; integrated browser rerun belongs to gabi/acceptance.


## Integrated acceptance closure

Contribution accepted in the [completed Delivery](../delivery.md#accepted-local-outcome-and-running-lan-service). That entry links exact native, independent API/browser and final LAN adoption evidence and their separate limits. Earlier source-only handoff statements remain historical; maintenance follows the native declarations and module guides, without reviving this delivery team.
