---
scope: Browser investigation source scenarios and independent executor handoff
status: authored-with-bounded-independent-evidence
owner: web
related: [design.md, ../../.docs/observability/scenarios.md, ../../.controller/observability/implementation-tasks/consumer-integration.md]
last_updated: 2026-11-08
---

# Browser cases and build handoff

The [canonical diagnosis scenarios](../../.docs/observability/scenarios.md) own native meaning. These cases specify the actual console path and expected presentations. Only [consumer integration](../../.controller/observability/implementation-tasks/consumer-integration.md) executes them. No Web author command, browser, process, dependency install or bundle has run. Unit test injection is confined to `tests/state.test.mjs`; the product always connects to the actual adapter.

## Executor recipe and finite manifest

Use existing repository Node/TypeScript installation; no root manifest, lock or dependency change is required. From repository root, execute `node web/observability/build.mjs /ABS/NEW/disposable-browser-bundle`. The parent must exist, the output directory must not exist. The script invokes the existing `node_modules/typescript/bin/tsc` with the Web-local strict `tsconfig.json`, fails on type errors, emits ES2022 modules and copies real HTML/CSS. It does not start a server or run native code. Do not run against source or a previously populated artifact. Compiler failure retains the named output for executor diagnosis; cleanup belongs to that executor.

The emitted manifest lists `/`→`index.html`, `/entry.js`, `/bindings.js`, `/app.js`, `/state.js`, `/presentation.js`, `/elements.js`, `/subjects.js`, `/configuration.js`, `/events.js`, `/app.css`. MIME is exactly `text/html; charset=utf-8`, `text/javascript; charset=utf-8`, or `text/css; charset=utf-8`. The native-derived type closure is erased at runtime. Adapter `api/client.ts` is compiled and renamed to `bindings.js`; all emitted imports are rewritten from `./api/client.js` to `./bindings.js` because `/api/` static URLs are forbidden. No CDN, inline script/style, source maps, development runtime, fonts or raster assets are needed. Entry imports only the actual client and calls connect(); CSS and UI modules load after scope discovery. There is no preload/prefetch.

Run authored state scenarios against this exact bundle with `OBSERVABILITY_BUNDLE=/ABS/NEW/disposable-browser-bundle node --test web/observability/tests/state.test.mjs`. These are intentionally independent from real browser acceptance. They cover precision, same-clock duration, withheld/absent, daemon versus adapter loss, restart/rebind, reset, retained A/B, native comparison, fixed-window gaps, stream replacement, read denial and page closure. They do not assert rendered usability.

Launch the actual adapter with its selected explicit disposable scope and this asset directory; use its one-time handoff in page memory only. Independent integration chooses available existing browser driver; no Web-owned dependency is installed. Never retain the handoff fragment/bearer in screenshots, traces, console logs or fixture results. Browser refresh needs a newly started adapter URL.

## Semantic controls

All selectors below are `data-testid`, with text labels in Chinese. Repeated rows support a scoped locator; request rows additionally expose exact `data-request-id` decimal strings. The source never converts these IDs to Number.

| Region / path | Selectors |
| --- | --- |
| Persistent workspace | `investigation`, `scope-context`, `connection-status`, `refresh-observation`, `close-investigation` |
| Subjects and finite page | `subject-requests`, `subject-connections`, `subject-peers`, `subject-hosts`, `subject-reports`, `load-subjects`, `more-subjects` |
| Request selection | `request-search`, `active-only`, `request-row`, `related-request`, `report-row` |
| Local evidence tabs | `tab-overview`, `tab-configuration`, `tab-events`, `open-configuration`, `open-events`, `return-request` |
| Capture / comparison | `configuration-left`, `configuration-right`, `operand-a`, `operand-b`, `compare-configurations`, `comparison-result`, `configuration-head` |
| Actual publication fallback | `events-by-attempt`, `events-scope`, `event-configuration-left`, `event-configuration-right` |
| Event window / gaps | `events-open`, `events-next`, `events-continue-retained`, `event-records`, `event-gap`, `historical-event-records` |
| Era / narrow workspace | `context-rebind`, `narrow-list`, `narrow-detail`, `subject-pane`, `detail-pane` |

Use native `details > summary` names for supporting disclosure: coverage, request era/deadline, source steps, snapshot rules, comparison basis, event identity and loss counts. Toggle by keyboard and touch as well as pointer. The console uses ordinary pressed buttons for local tabs, not a partially implemented ARIA tab widget.

## Connected main case: real fixture path

The independently authored native fixture `actual_browser_investigates_real_native_request_configuration_and_events` supplies actual native requests, two configuration resolutions and publication refs. Its seed is external expected-reference evidence only; never inject it as an API response. The selected ordinary Watermark request may legitimately have `NotEvaluated`. Configuration requests can lose details under the native 2KiB bound, so a `Captured` row is not a mandatory fixture assumption.

1. Open the real handoff. Verify fragment removal, explicit scope and connected adapter state. Snapshot strip exposes lifecycle, queue/execution and connection facts; expand source stamps and coverage. No daemon acquisition or Application call is allowed.
2. Find the fixture request in `request-row` by exact observation ref or operation/request ID. Select it. Verify queue/execution/reply remain separate, exact IDs remain intact and capture loss is visible under coverage. Record identity to prove return.
3. Open configuration. If `Captured` uses exist, select A, visit another request, select B. If native detail was evicted or `NotEvaluated`, preserve that honest state and use the explicit alternative: open event tab → `events-scope` (or `events-by-attempt` when exact relevant attempt exists) → locate actual Configuration / SnapshotPublished events by exact seed snapshot sequence and attempt in their own event row → choose `event-configuration-left` and `event-configuration-right`. Use `tab-events` to return to the retained journal after selecting A; selecting a snapshot switches to configuration but does not erase that journal. If the fixed page is incomplete, continue `events-next` until matching publications appear or the frozen window completes.
4. Run `compare-configurations`. Confirm actual differing timeout values from the fixture, native value/provenance relations and purpose/time/era basis. Withheld fields, including a self-comparison if exercised, show unknown rather than equality. A missing operand retains the available side.
5. Return to event tab and use `events-open` for the selected request or `events-by-attempt` for a chosen client attempt. Inspect correlated facts, exact fixed frontier and source/collection clocks. Reopening archives prior windows under `此前窗口`. Domain commit refs are readable evidence with visibly unavailable original-domain detail.
6. `return-request` restores the original selected request overview. Reopen configuration to confirm A/B is retained; reopen events to confirm loaded window and older journal remain accessible. Switch related subjects and back; selection and loaded evidence remain.

## State/branch cases

| Case / canonical basis | Trigger | Expected evidence and action |
| --- | --- | --- |
| Bootstrap / consumer boundary | Invalid, replayed or refreshed fragment | Session failure with fresh-launch guidance; no raw error or secret reflection. UI/CSS modules do not load before successful exchange. |
| Adapter loss / OBS-01 | Stop only adapter, explicitly refresh | `adapter-lost`, loaded evidence historical, selected request/A/B/events readable. No claim that daemon stopped. |
| Daemon loss / OBS-01 | Adapter survives but native fixture daemon unavailable | `daemon-lost`, adapter reachable and exact native failure stage/kind; historical evidence retained. |
| Native busy / OBS-03 | Read returns native Busy | Explicit evidence-unavailable message; no empty success, daemon restart or automatic retry. |
| Partial evidence / OBS-04/10 | Detail dropped or a configuration capture skipped | DetailUnavailable/Pressure/NotEvaluated remains explicit; list/coverage and actual event publication provide independent evidence. |
| Config withheld / OBS-04 | Native snapshot/compare conceals a field | `已隐去` / `未知 · Withheld`; no hashes, inferred equality or current environment fallback. |
| Config retirement / OBS-04/10 | One retained operand expires | Native unavailable comparison preserves the found side; retirement frontier remains visible. |
| Pressure / OBS-10 | Projection/capture loss counters advance | Runtime drop/eviction and event capture/retention loss stay separate and disclose sampled times. |
| Fixed event pagination / OBS-05 | Append after frozen through | `events-next` uses exact returned window; new event appears only after explicit new head/window. |
| Event gap / OBS-05/10 | Native returns Gap | Missing interval visible, no automatic rebase. Explicit retained continuation keeps missing interval and original through. Fully expired window asks for a new window. |
| Restart / OBS-08 | Daemon ref changes | Fence new reads; retain old identities/evidence; `context-rebind` obtains new daemon explicitly. Old request stays historical without ref rewriting. |
| Reset / OBS-09 | Generation changes under same daemon | Fence configuration/event joins; preserve historical data. New context does not relabel old request or cursor. |
| Stream replacement / OBS-05 | Collector nonce changes | `StreamChanged` stops old window; explicit opening archives it. No fabricated continuity. |
| Return / consumer lifetime | Switch local tabs, subjects, disclosure and narrow panes | Selected request, A/B, loaded journal, disclosure, scroll and input focus survive rendering. |
| Closed investigation | Close this page investigation | No fresh reads, retained evidence historical; daemon remains alive and business work unchanged. |

## Representative wide/narrow inspection

Use real representative native data at a wide viewport (for example1440×900) and narrow viewport (for example390×844), with long scope/request/daemon values. Record actual screenshot paths and interaction evidence in the consumer evidence record; capture after direct scope discovery. At wide size, scope/era, runtime counts, list actions and selected request chronology should share one workspace. At narrow size, context plus local list/detail toggle stays available while the main pane scrolls; configuration and event supporting facts are reachable through local disclosure. Horizontal overflow belongs only to bounded comparison tables or subject navigation, never the document. Verify text zoom, keyboard focus, summary activation, searching multiple characters, exact selection, return, compare and gap action. Count meaningful facts/actions visible together and reachable secondary actions using the same fixture; do not infer density/usability from CSS source or absence of overflow alone.

No representative user has participated. Consumer Agent interaction evidence can establish bounded rendered/operational experience, not user research or unsupported platforms. Installed/default/live scopes remain outside this release.

## Executed subset and release

The [original Web Task closure](../../.controller/observability/implementation-tasks/web.md#actual-browser-outcome-and-final-release) records the independent candidate01 build, nine passing state cases, actual Chrome147/Playwright1.59.1 native-browser path and direct wide/narrow screenshot inspection. The [consumer evidence](../../.controller/observability/implementation-evidence/consumer.md) owns exact artifacts and final process/package/resource receipts. No source repair followed that passing candidate; later changes are documentation only. The real browser path covers native detail loss/NotEvaluated, actual publication-ref configuration17/29 comparison/withheld unknown, request/events/related-subject return, retained A/B,390px keyboard/search/geometry and adapter-loss historical evidence. The remaining authored negative branches are not blanket browser passes; state/native/API evidence has its own bounded scope. All Web writes/resources are released.
