---
schema: delivery
description: Deliver a daemon-owned HTTP management API and embedded Web console for local Agent lifecycle and conversation.
status: completed
stages:
  design: {description: Define native contracts and scenario-driven interface, tasks: [./tasks/integration.md, ./tasks/web.md]}
  implementation: {description: Compose existing native capabilities into HTTP and Web management, tasks: [./tasks/runtime.md], depends_on: [{stage: design}]}
  acceptance: {description: Independently exercise the complete API and browser path in disposable scopes, tasks: [./tasks/acceptance.md], depends_on: [{stage: implementation}]}
---

# Agent management delivery

User authorized the [proposal](../../.docs/proposals/agent-management-http.md) and implementation. The scoped outcome is one local AgentX daemon serving an embedded management page and typed HTTP API: discover/list, launch, message, communication history, transcript, stop and resume. Browser lifetime does not own Agent lifetime. No PTY mirror, arbitrary command endpoint, new Provider protocol or multi-tenant service.

Native source is canonical. JSON uses string IDs/cursors and explicit objects; operation results are durable under the same database generation. Long host effects execute outside SQLite dispatch with per-operation process/environment isolation. HTTP configuration is opt-in under the selected scope. The default bind is loopback; explicit non-loopback configuration grants management access to clients reaching that listener, matching the existing direct LAN product model.

Acceptance requires real HTTP requests against the candidate daemon and an isolated Provider fixture, including request replay/conflict, lost reply lookup, stale Agent references, finite bounds, stop/history/resume, and browser use. A recorded fixture is not paid/native Provider coverage. Candidate packaging must embed fresh resources. Existing live resources are preserved.

## Accepted local outcome

The requested management API and embedded Web console are implemented and accepted for Linux local operation. The immutable candidate is `.bin/agentx-management-r4`, SHA-256 `1a928f91f4dde4f19eebcf6ec64f75419688560948f6689de907b398be2b1d1e`; [packaging](verification/package-r4-receipt.json) embeds fresh assets using the documented local functional profile (release LTO disabled, 256 codegen units, optimization level 0). This is functional evidence, not optimized release-performance evidence. No installed/default executable or existing user daemon was replaced.

[Independent acceptance](evidence.md) establishes the complete launch → select → send/reply → transcript → stop/history → resume path, exact old/successor message isolation, duplicate/conflicting requests, admitted lost reply recovery, scope/reset fences, restart retention and bounded inputs/pages. All native public cases M01–M11 passed in [pula-06](verification/pula-06/cases.json). That run's overall failure preserves a browser-driver label locator defect; the corrected ARIA role/name driver passed all actual Chromium checkpoints B00–B07 on the same unchanged candidate in [pula-07](verification/pula-07/browser/report.json). Desktop and 390px browser cases include draft/selection continuity, reload, an admitted lost response with no duplicate effect and the complete lifecycle. Both manifests report successful owned cleanup and an unchanged candidate.

Native checks include the journal invariants and local-origin pagination regression, CLI/browser boundary cases, and [26 retirement/binding/operator regression cases](verification/native-retirement-r4-receipt.json). The substantive acceptance defects and causal repairs remain recorded in the Task and independent evidence; no fixture expectation was relaxed to bypass them.

Acceptance uses real HTTP, same-binary workers and candidate-generated hooks with a local Qoder substitute and fixture transcript. Actual paid Provider interoperability, direct LAN execution, physical mobile devices, forced-worker interruption/Unknown execution and other operating systems are not established by these runs. The opt-in LAN configuration and durable Unknown behavior remain bounded by their native contracts and existing evidence, not inferred from the local browser run. Installed/live rollout is separate from this completed source/candidate delivery.

Use [startup and HTTP documentation](../../.docs/agent-management/README.md). All original Tasks are complete; future maintenance starts from the module guides and affected cases rather than reviving the delivery team.
