# Live integration checks

From `web/inspector`, run `node tests/runtime-read-repair.mjs` for the maintained SDK fixtures covering current and history paging, dataset fences, direct exact selection, and Space launch receipt reconciliation. `npm run typecheck` checks consumer compatibility with the generated SDK. These checks do not start a service or mutate user Agents. See the [independent verification record](../../../.controller/runtime-read-repair/verification.task.md) for browser and native evidence. `live-adapter.ts` is a historical pre-SDK fixture.

Default product source is live. Historical tests below target explicitly selected `source=demo` fixture supply and original static evidence; they are not acceptance criteria for native semantics.

# Current selected-scope checks

Run `node tests/scope-trim.mjs` against the existing built LAN artifact (`INSPECTOR_URL` defaults to port 18971). It owns affected cleanup cases: eight tabs, no removed operations/UI, Token-only detail and analysis export, immediate send safety, recovery audit, auxiliary routes and narrow/theme continuity. See [implementation record](../../../.controller/inspector-migration/scope-trim-implementation.md).

The following complete-source migration cases and evidence are historical. Do not run their removed quota/scheduling/execution expectations as current requirements.

# Inspector independent acceptance

The [verification task](../../../.controller/inspector-migration/verification.md) and [result](../../../.controller/inspector-migration/verification-result.md) own scope and executed evidence. `browser-acceptance.mjs` operates an already served disposable build and never manages services; `dependency-check.mjs` inspects production package/import/metafile inputs separately.

Use the existing Playwright `/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright/index.mjs` and `/usr/bin/google-chrome`. From `web/inspector`, set `INSPECTOR_URL` (default `http://127.0.0.1:18971`), `INSPECTOR_EVIDENCE` (output directory), and optionally `INSPECTOR_CASES` (case-ID regular expression), then run `node tests/browser-acceptance.mjs`. Run `node tests/dependency-check.mjs` against a completed `dist/metafile.json`. Do not rerun completed checks without an affected source change or explicit requirement.

The browser binding covers offline load, eight tabs, selected detail/digest/timeline, chart drilldown/comparison, supporting routes/report return, style/theme continuity, component preview/alternate recovery, accepted recovery and unresolved barriers, unknown/partial facts, compact reachability and runtime/network evidence. It retains DOM snapshots, observable provider call inputs, root/selection state, geometry, screenshots, errors and actual requests. It does not expose private React state or claim live AgentX API/native effects.

[evidence/2026-11-15](evidence/2026-11-15/) retains source hashes, surface inventory, dependency findings and distinct execution attempts. `final/` is the original run name, not an assertion of final acceptance; `continuation/` corrects executor assumptions, and `repair-continuation/` binds application repairs. Failed attempts remain immutable and the result document maps accepted evidence. Binary screenshot captures support the stated viewports; they do not establish physical mobile/assistive technology or all-browser support. The disposable `tests/.artifact` service input is removed at handoff.
