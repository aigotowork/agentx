# Workbench frontend

Read the [Web Task](../../.controller/collaboration-workbench/tasks/workbench-web.md), [frontend model](../../.docs/collaboration-workbench/frontend.md) and [accepted interaction contracts](../../.docs/collaboration-workbench/interface-details.md). `api/native.ts` now aliases public SDK-derived `WorkbenchTypes`, and the page-lifetime adapter consumes public `@agentx/client`; see [current API ownership](api/AGENTS.md). Do not add fixtures to the production app.

For startup, operator use and observed support, follow the [current frontend guide](../../.docs/collaboration-workbench/frontend.md#startup-and-everyday-use) and its canonical evidence links. The finite Linux outcome, final r8 embedded-artifact/narrow-touch checks, repository-local promotion and cleanup are independently complete; formal Delivery acceptance remains controller/user-owned. Do not infer universal platform support. `IconAction` owns local label/reason disclosure; activation reveals it while available intent invokes once, and touch must not depend on hover/focus compatibility events.

Dependency direction: `entry` → `workbench`/Page → feature Blocks → semantic components/layout. `state.ts` owns pure presentation transitions; the Page owns native reads/commands. Drafts remain in memory. Only a bounded pending-command reference may survive in session storage; it is never permission or replay input.

The sole independent acceptance owner runs installs, lock resolution, formatters, compilers, tests, builds, browser and packaged executable checks. Source authors write meaningful cases and label them unexecuted. Preserve diagnostics and the running LAN demo. Do not write locks or run product processes from this Task.
