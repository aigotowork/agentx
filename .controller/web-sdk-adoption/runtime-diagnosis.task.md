---
schema: task
status: completed
description: Diagnose impactful launch-host failure, intermittent HTTP 503 and Inspector first-render latency after selected home rollout.
---

# Outcome and causal model

The human requested prioritizing consequential issues and establishing their causes after [home rollout](home-rollout.task.md). Tele investigates one connected runtime/consumer chain inline. Preserve user processes, original receipts, database and selected live service; no production launch/replay or stress test. Scoped read observations and disposable diagnostic subprocesses are permitted. Source repair follows discriminating evidence, not speculation or generic retries.

| Priority | Question | Discriminating observation |
| --- | --- | --- |
| High: launch cannot execute | ENOENT before worker claim: missing cwd, executable/deleted inode, loader or worker bootstrap? | Original journal worker/command timestamps, source supervision stages, retained process/build lifecycle; reproduce missing-executable behavior only in a disposable process. |
| High: data unavailable | Which gate emits503: HTTP eight-call budget, native admission, expensive catalog/listing or startup? | Capture exact error envelope and duration per normal-page request; correlate native/read fanout and endpoint timing. No load flood. |
| Medium/high: Inspector appears stalled | Slow native pages versus frontend serial pagination/all-or-nothing publication? | Count pages, per-request duration, first successful response and actual first rendered member time on existing data. |

Avoid expanding into optimized packaging/default adoption until actual runtime failures are understood. Reuse accepted SDK/browser evidence. Exact probe scripts/results remain under `.artifacts/home-sdk-rollout-20261118/diagnosis/`; sensitive commands/prompts must not be copied into this document.

## Prioritized diagnosis outcome

This completes the requested investigation and treatment assessment, not implementation or deployment of the remedies below. The selected daemon was not restarted, original operations were not replayed and no production Agent was launched during diagnosis.

| Priority | Established finding | Cause and treatment decision |
| --- | --- | --- |
| P1: launch execution and actionable errors | Both original launch operations retain raw ENOENT, Unknown and no worker claim; one used coder and the other selected a profile without a role. | `src/management/service.rs::start_worker` reexecutes `current_exe()` with the requested cwd; lookup/spawn errors lose their stage/path context and child stderr is discarded. Failure precedes role parsing. Fix supervision diagnostics and executable lifetime handling, with disposable replacement/cwd failure cases. Do not change role precedence on this evidence. |
| P1: shared read workload | Normal simultaneous Management, Space and Inspector navigation produced a 6.715s workspace request; Space workspace reads took 3.797–4.482s and Agent list reads roughly 1–2.9s. | `src/management/store.rs::live_view` calculates unread counts with a separate event/JSON scan per current Agent. `src/management/offices.rs::catalog` requests up to ten full 100-Agent pages merely to discover adopted workspace membership. The live database has 254754 events and 2054 stopped events. Reuse native batch unread calculation and design a bounded workspace projection retaining existing discovery/coverage semantics, instead of hydrating unrelated Agent fields/history on each catalog refresh. |
| P1 investigation: intermittent unavailable reads | Earlier rollout captured503 for roles/workspaces. A bounded normal three-page observation in this diagnosis returned200 throughout. | Both HTTP call-budget refusal (`HTTP calls busy`, eight permits) and native admission/unavailability can surface503. Heavy reads are confirmed, but their causal connection to the original503 is unproved. Capture the exact structured error, route and duration at the next ordinary occurrence; do not increase permits or add blanket retries as a supposed root-cause repair. |
| P2: Inspector first display and polling | Standalone navigation waited roughly4s; normal three-page navigation still showed zero members at7.8s and displayed2000 at10.3s. | `web/inspector/host/live/client.ts::directory` serially collects up to twenty100-record pages including stopped history and publishes only after completion. `plugin/client/live/dashboard.tsx` repeats the complete collection five seconds after completion. Publish the first page promptly, page history explicitly, and refresh the current working set without repeatedly walking unchanged historical pages. Preserve scope/generation, exact identities and honest partial-coverage presentation. |
| P1/P2: failure visibility | Space can show an unresolved-operation count while the detailed native error remains inside collapsed receipts. | `web/agent-space/src/live/Actions.tsx` renders `operation.error` in `<details>`; `controller.ts` preserves Unknown and avoids replay, but the launch error escalation checks Failed specifically. Surface the original operation's Unknown/error next to the launch action and offer original-receipt reconciliation. Never label an uncertain outcome as a definite non-execution or automatically launch again. |

### Historical ENOENT: supported boundary and unresolved mechanism

The stored absolute DancingAI directory and its coder prompt path are valid. A profile-only launch failed with the same preclaim signature, independently rejecting a coder-only explanation. The failed daemon instance was already replaced before the retained rollout process snapshot, so the executable/cwd state at the original spawn cannot be recovered from the current process.

A disposable process reproduction confirms the Linux executable-lifetime mechanism: unlinking a running executable makes `/proc/self/exe` resolve to a pathname ending in ` (deleted)`; attempting to spawn that pathname returns errno2. This makes atomic executable replacement a concrete candidate for the current `current_exe()` implementation, but does not establish that it caused the historical incident. Missing cwd and executable/loader failure remain alternatives. Current scoped R3 runs from its retained persistent artifact; deployment itself does not prove launch repair.

### Evidence and stopping condition

The bounded browser observations are [standalone read probe](../../.artifacts/home-sdk-rollout-20261118/diagnosis/read-probe.json) and [normal simultaneous pages](../../.artifacts/home-sdk-rollout-20261118/diagnosis/normal-three-pages.json), with their scripts in the same directory. The simultaneous observation opened only the three ordinary pages for approximately15s; it was not a saturation test. No503 reproduced, which does not dismiss the earlier captured failures. No further unchanged read probes are justified for that question.

The treatment order is launch supervision/visibility, native read amplification, then Inspector paging, while retaining exact503 diagnostics when it next occurs. Optimized packaging and account-wide default promotion are lower priority than these usability/reliability defects. Source fixes need focused disposable native failure/data scenarios and affected browser acceptance before any new scoped deployment. Existing SDK migration/browser/CORS evidence remains valid within its documented limits.
