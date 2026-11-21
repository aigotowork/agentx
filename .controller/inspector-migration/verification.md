---
schema: task
status: completed
owner: inspector-check-veru
description: Independently execute the final Inspector mock migration build and browser acceptance.
---

# Inspector verification

Accepted by lube at the declared finite boundary: [completed result](verification-result.md), S01–S12 supported, substantive repairs verified, temporary resources cleaned up. See [Delivery](delivery.md#accepted-local-outcome) for composition acceptance and deferred real integration.

Read [Delivery](delivery.md), [frontend assignment](frontend.md), and the final module README/implementation inventory when available. The source original remains read-only. You own `web/inspector/tests/**` and `verification-result.md` beside this task. No implementation source edits, root manifest edits, native builds or services. No subagents. The implementation owner remains responsible for repairs.

## Outcome and start conditions

The purpose is to establish that the complete migrated Inspector can actually be used through its offline AgentX provider, including source UI, alternate skins and connected flows. You may prepare cases now from source and the agreed outcome; final build/browser execution starts only after beta hands off usable source and exact commands. Coordinate directly with beta for that handoff, source fixes and scoped continuation. Record the source basis so only changes affecting completed checks justify reruns.

Derive finite acceptance cases from the actual transplanted surfaces and controls; use one reusable browser script rather than manual repetitions. Required coverage: initial offline load; all eight main tabs with representative nonempty data; meaningful list→detail and chart/breakdown/comparison navigation; key supporting archive/execution/diagnostic/schedule/job surfaces; original/pixel and light/dark switching without resetting working context; component preview and alternate recovery if present in source; recovery or other simulated action with truthful local result; unknown/partial/unavailable state where supplied; desktop and narrow layout with reachable controls and no fatal page error; no Paseo package/SDK/runtime/business network dependency in the built application. Do not invent API support from mock success. Check package/import and browser-request evidence separately.

Run scoped typecheck/build from the final provided commands, then serve only your own disposable output on a free non-default loopback port. Use existing Chrome/Playwright where available (see `web/agent-space/tests/README.md`); dependency availability needs confirmation. Never stop or replace current user-owned services. Record all failed expectations, distinguish test assumptions from application defects and notify beta with concrete reproduction. Continue only affected cases after repair. Final result states checks run, limits, outstanding defects, evidence locations and owned process cleanup. Final acceptance is by lube, who will arrange an optional persistent preview separately.
