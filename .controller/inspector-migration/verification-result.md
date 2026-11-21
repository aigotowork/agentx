---
schema: document
status: completed
owner: inspector-check-veru
description: Finite independent Inspector migration scenarios and execution evidence.
links: [./verification.md, ./delivery.md, ./frontend.md]
---

# Independent verification

Independent finite verification completed on 2026-11-15 by `inspector-check-veru`. The scoped offline migration checks passed after beta's required repairs. No outstanding substantive defect remains in the executed scope. Scope is [verification](verification.md); beta owns implementation and lube owns final Delivery acceptance.

## Source basis and boundaries

The read-only source is `/home/long/works/longwork/paseo-retry`. Inspected source entries: `web/client/inspector.tsx`, `plugin/client/surfaces.tsx`, and `plugin/client/dashboard.tsx`. The dashboard declares eight tabs: `agents`, `recovery`, `board`, `tokens`, `analytics`, `compare`, `activity`, and `insights`. Surface registration connects archives, execution, diagnostics, insight jobs, schedule runs, workspace/Agent detail, and language. The source Web entry also supplies independent appearance controls and `?surface=components`; its Web guide specifies separate `?experience=recovery`.

The graph catalogue contains only `/home/long/works/longwork/xpm`, so neither migration checkout has a matching index. Preparation uses direct source evidence; there is no graph completeness claim. Target `web/inspector` did not yet exist at the initial inventory. Beta subsequently confirmed planned `plugin/client`, `plugin/shared`, and `uiux/systems` preservation with a local provider, esbuild, and Node static serving. Final source and fixture expectations must be bound after the handoff, not inferred from these plans.

Installed Playwright `/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright/index.mjs` and Chrome `/usr/bin/google-chrome` are readable/executable respectively. Neither has been launched. Reuse the established recipe in [Agent Space tests](../../web/agent-space/tests/README.md).

No native build, user-owned service, source implementation, root manifest, or external runtime is in scope. Writes are limited to `web/inspector/tests/**` and this record. No subagents.

The completed [data-foundation assessment](../../.docs/inspector/data-foundation.md) supplies semantic criteria for S08–S09 and the future integration limit: Mock must remain explicit; accepted delivery is not execution/recovery; missing permission/active-turn facts cannot manufacture recovery eligibility; unknown/partial coverage cannot become healthy/zero. Its capability matrix is static assessment, not runtime evidence. This does not expand verification into native or live HTTP checks.

## Finite scenario coverage

The following table is the original coverage design; actual execution and evidence are mapped below. One [reusable browser script](../../web/inspector/tests/browser-acceptance.mjs) retains named case results, actual requests, console/page errors, DOM/state/geometry and screenshots. Appearance cases share a live selected/filter/detail context; independent surfaces start from fresh navigation. Tests bind the handed-off fixtures and accessible controls.

| Case | Starting state and meaningful action | Expected evidence and invariant |
| --- | --- | --- |
| S01 Offline entry | Open built default entry with no backend | Recognizable transplanted Inspector and representative sessions; local sample supply is explicit; no indefinite loading or fatal error. |
| S02 Eight main tabs | Navigate through all declared tabs using actual controls | Nonempty correlated sessions/recovery/dashboard/tokens/details/comparison/events/insights; tab identity and meaningful content recorded separately. A shell or heading alone does not pass. |
| S03 Session detail and return | Filter/select a supplied session; inspect evidence/detail and return | Correct object identity and related content; context preserved through disclosure/return. Exercise a no-match/reset branch if supplied. |
| S04 Charts and comparison | Dashboard chart/metric → breakdown → meaningful scope/detail; choose comparison inputs | Selected range/dimension/object and rendered data agree; comparison changes reflect selected inputs; chart is an actual navigation control where source promises it. |
| S05 Supporting work | Visit archive, execution, diagnostics, schedule runs, insight jobs and available report links | Representative content, correct identities, usable return paths; job/report navigation arrives at related analysis. Record unsupported/deferred controls honestly. |
| S06 Appearance continuity | With nondefault tab/filter/selection or draft, toggle original↔pixel and light↔dark | Root style/theme changes; working context survives. Cover all four combinations without multiplying unrelated flows. |
| S07 Preview and alternate recovery | Open component preview and separate recovery experience when retained | Shared components render; preview return works; alternate recovery is independently reachable and usable. Distinguish absent required source preservation from an optional unsupported control. |
| S08 Local action truth | Perform a recovery or other supplied simulated operation; inspect resulting record/state | Explicit local/simulated result correlated with target and action; no real send/mutation/network call. Duplicate behavior follows the provider's stated idempotence contract; receipt is not proof of execution. |
| S09 Incomplete facts | Select supplied unknown/unavailable health or partial-history fixture | UI retains uncertainty/coverage limits, does not imply zero/full observation or successful recovery; unsafe/unavailable actions remain correctly bounded. |
| S10 Responsive reachability | Desktop 1440×1000 and narrow 390×844; navigate tabs/detail/action/return and appearance | Critical controls actionable without force; selected resource and return remain reachable; geometry records clipping/scroll and meaningful collisions. Capture layout for user-intent assessment. |
| S11 Package/import separation | Inspect module manifest/lockfile, imports and built dependency metadata/artifact | No executable Paseo package/SDK/runtime dependency; distinguish provenance/license mentions from code. Source/asset coverage and actual scan scope are explicit. |
| S12 Browser request separation | Collect requests across all executed cases | Only disposable static origin/data/blob resources where applicable; no former runtime/API/business calls. Requests and failed responses inspected separately from package/import evidence; zero page-fatal errors. |

## Original execution and rerun policy

After usable-source handoff, read target AGENTS/README and implementation inventory, bind fixture identities/control names, record a source digest excluding tests/generated outputs/dependencies, and run provided scoped typecheck/build once. Serve only the executor's disposable output on a free non-default loopback port (beta proposed 18971; verify availability). Browser execution reuses existing Playwright/Chrome. At preparation time no install/build/browser command had run; executed outcomes are recorded below.

Retain failed expectations with reproduction and source basis; notify beta directly and classify application defects separately from executor assumptions. Resume only cases affected by repairs, and rebuild only when changed runtime source requires it. Report executed checks, unverified limits, outstanding defects, evidence paths, and cleanup of owned browser/server processes. Mock success establishes neither live HTTP integration nor native AgentX facts, durable persistence, or real recovery execution.

## Executed result and evidence

Beta handed off usable source in message #22012; final execution started on that source. `npm ci`, final `npm run typecheck`, and `npm run build` passed. The dependency scan passed for 2,544 production inputs, including 18 portable-core files, with no forbidden package/import or outside-module production input. Initial source hashes are retained under `web/inspector/tests/evidence/2026-11-15/source-before.json`.

The executor served a disposable `tests/.artifact` copy on `127.0.0.1:18971` using its own Python HTTP process. Browser contexts are closed, that process exited, the disposable copy is removed, and the port was confirmed closed. No user service was touched. Original install/typecheck/build results, repaired artifact hashes and cleanup are in [execution-log.json](../../web/inspector/tests/evidence/2026-11-15/execution-log.json).

Initial browser evidence is under `web/inspector/tests/evidence/2026-11-15/final/`; continuation evidence is under `continuation/`. S01, S03, S04, S06, S08 and S12 passed initially. S07 and S09 passed after correcting executor assumptions: component preview uses a different local-sample label, and the unknown-recovery fixture already has an unresolved attempt and must not expose a new continue action. Narrow readiness must inspect mounted content rather than require a header intentionally compressed by layout. Preserve initial failed attempts; they are not application failures.

Required application findings were sent directly to beta: default insights list/overview empty and completed-job navigation showing the same Agent's queued analysis, plus invisible narrow Mock indication. Beta repaired the three insights fixture files in #22267 and compact host composition in #22347. One repair build passed; only pending/affected paths continued. [Source hashes after repair](../../web/inspector/tests/evidence/2026-11-15/source-after-repair.json) and [four changed source paths](../../web/inspector/tests/evidence/2026-11-15/repair-diff-paths.json) delimit the rerun basis. No new dependency, core algorithm or unrelated business page changed, so the completed core/dependency and working-context checks were not repeated.

Evidence root: [`web/inspector/tests/evidence/2026-11-15/`](../../web/inspector/tests/evidence/2026-11-15/). In the table, paths are relative to that root. Run directory `final` is its original name, not a claim that every first attempt passed.

| Coverage | Disposition and actual evidence |
| --- | --- |
| S01 offline entry | Passed in `final/result.json`; initial desktop has 14 correlated Agents and visible local simulation. |
| S02 eight tabs and analysis | Seven tabs passed and were captured in `repair-continuation/S02-*`; the rich insights headline expectation was corrected and passed in `affected-continuation/result.json`. Default overview now shows 10 in-window reports; the report list has 12 retained reports. List → selected report → close/return passed in `insights-return/result.json`. |
| S03 session → digest/timeline → return | Passed in `final/result.json`; search filter survives detail close, no-match/reset exercised, preserved digest operation consumed and rendered. |
| S04 chart/breakdown/comparison | Passed in `final/result.json`; dashboard model chart action reaches model breakdown, comparison sides swap, choice changes and collection-basis overlay opens. |
| S05 supporting surfaces | Passed in `repair-continuation/result.json`; archives/detail, execution/detail, diagnostics, schedules/filter, jobs → completed report, team view and language read. Completed job no longer opens with unrelated active queue. |
| S06 original/pixel × light/dark | Passed in `final/result.json`; selected session, search text and timeline subtab survive all four combinations in one live context. |
| S07 preview and alternate recovery | Passed in `continuation/result.json`; preview renders original components and returns to Inspector; alternate recovery renders its separate workspace and its continue action opens local confirmation. Final delivery semantics are checked through the main recovery case, not inferred from the alternate confirmation. |
| S08 truthful local recovery | Passed in `final/result.json`; permission-blocked target disabled, eligible continue produces one local invocation and 核验中 1 / 已恢复 0, with no second continue action. Recovery success/execution is never inferred from accepted delivery. |
| S09 unknown/partial/unavailable | Passed in `continuation/result.json`; unresolved recovery has no continue/replay, unknown metric basis, unavailable directory, partial jobs and partial-origin detail retained. |
| S10 compact use | Passed in `final-continuation/result.json` at 390×844; visible Mock label, theme control, all eight main tabs, detail and return are reachable without forced clicks; no document overflow. Desktop basis is 1440×1000. |
| S11 old runtime dependency removal | [dependency-result.json](../../web/inspector/tests/evidence/2026-11-15/dependency-result.json) records manifest/lockfile, 2,544 production inputs and executable local imports: no forbidden old SDK/package/runtime or external-checkout input. Eighteen retained core files include recovery/backoff, statistics, token admission/projection and digest folding. |
| S12 request/runtime separation | Passed in every retained browser run, including repaired artifact continuations: zero page-fatal errors, no external origin or fetch/XHR/WebSocket/EventSource business requests, and no failed application asset response. Only optional `/favicon.ico` 404 is excluded explicitly. |

All first failures remain in the original run records. They include executor assumptions about readiness labels, fixture initial unknown state, rich insights wording, compact return control name, and report-list empty-selection wording. The actual empty default analysis and compact Mock-disclosure gaps were recorded in source inventory/screenshots, repaired by beta and verified separately. Passing subsequent checks does not rewrite those attempts.

## Visual assessment and limits

For the concrete task of inspecting one payment session while preserving its list context, the desktop digest screenshot shows the filtered selected row, resource identity, detail tabs, metric values, bounded-history disclosure and execution timeline together. The repaired compact screenshot shows a visible local simulation cue, return control, session identity, six detail tabs and latest reply, with the primary open-session action retained at the bottom. Those observations support the stated viewports and tasks; they do not establish every chart's visual treatment or every possible overlay combination.

No live HTTP/native/provider execution, durable business state, actual scheduler/AI runner, real recovery, physical-mobile keyboard, assistive-technology compatibility, cross-browser matrix or arbitrary viewport was tested. The component gallery was rendered and navigated; not every leaf variant was individually exercised. Appearance continuity verified selected/filter/subtab state, not every possible editor draft. Exact same-operation replay was covered by beta's earlier schema probe and preserved source, not independently rerun here; the browser independently checked unresolved recovery cannot issue another continue. Copying source and passing a finite browser campaign do not prove every retained algorithm or all historical acquisition semantics.

The reusable test entry and execution recipe are in [tests/README.md](../../web/inspector/tests/README.md). Verification writes are confined to that tests subtree and this record; no implementation, native service, root manifest or source-original change was made by this executor. The generated module build output remains available for lube's integration; no preview remains running.
