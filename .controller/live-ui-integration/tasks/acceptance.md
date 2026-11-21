---
schema: task
status: completed
description: Independently verify both actual UIs and native/API composition.
---

# Acceptance

Read [team entry](../AGENTS.md), [design](../design.md) and [Delivery](../delivery.md). Record scenario-to-native supply, unsupported/redundant concepts, selected design/skeleton, source changes and exact verification here. Implementation is authorized; no separate staged approval is required.


## Independent assignment

Own tests/live_ui_integration/** and evidence.md/this Task. Author finite scenarios from consumer tasks/native model before candidate handoff; no product edits, native build, managed DB clients, paid models, user-service changes or source reset. Reuse existing management fixture real hooks/transcripts and public HTTP. Cover both embedded UIs, default real supply/no mock fallback, exact identity/generation, receipt uncertainty and scoped directory actions; Inspector enable-time sampled collection/unknown usage/dedup, durable jobs/restart/CAS and explicitly queued continuation audit. Deterministic native episode/storage tests are native-owner-authored and integration-executed; actual browser acceptance independent. New immutable candidate only after source frozen.

## Case design and current state

Owner `live-check-timu` published the finite [case map](../../../tests/live_ui_integration/README.md) before implementation handoff, derived from consumer Tasks and native [live model](../../../.docs/inspector/live-model.md)/[contract](../../../src/inspection/contract.rs). [Evidence](../evidence.md) distinguishes authorship from execution. Method: scenario/state transitions and cross-consumer exact identity/receipt invariants; real browser journeys retain DOM, state, screenshot and corresponding public API/native effects. Existing management fixtures supply generated hooks/transcript/native replies; internal deterministic storage/episode cases remain native/integration-owned.

[Runner](../../../tests/live_ui_integration/acceptance.py), [private generated-hook fixture](../../../tests/live_ui_integration/codex_provider.py) and [actual two-app browser journeys](../../../tests/live_ui_integration/browser.mjs) follow actual native Request wrappers. Integration authorized the finite Codex terminal substitute (#24929), native owner supplied failed/completed hook recipe (#24937), and fixture counters use real native transcript parsing. No public internal seeding or successful-response mocks replace native state.

## Execution and resolved finding

Integration handed over immutable r1 path/SHA in #25264. Native integration ran 38 cases including nine inspection cases, all passing. Independent r1 API scenarios passed, including collection/restart/CAS/unknown+known usage/resumed-source dedup/pinned jobs and generated failed model Hook composition. After two corrected harness bindings (setup directory order and actual Space `@name` accessible labels), the actual composed browser journey passed Space launch/draft/send/reply/transcript and receipt uncertainty/reload/reconcile, plus Inspector exact live reads.

Actual r1 Inspector send was rejected HTTP 400 because `requestId()` emitted `inspector-hex`, while native management requires UUID. Integration confirmed #25346 and the Inspector owner corrected it. [Resolved F01 and exact evidence](../evidence.md#f01--inspector-uuid-incompatibility-resolved-on-r2) retain the original failure and subsequent actual r2 send/transcript/stop/resume receipts. Acceptance made no product source edit or substantive assertion change.

## Final independent outcome

Frozen final candidate `/home/long/works/longwork/agentx/.bin/agentx-live-ui-r2`, SHA-256 `52ccc7010c3845f345af855003c29d70e8a5d74487962badefb6cb8f1e93ae46`, was authorized in #25443. Both actual embedded `/space/` and `/inspector/` were independently exercised on this same artifact. Completed coherent r2 browser segments cover default real supply/no mock fallback, Space launch/draft/native reply/transcript, actual scene desktop/narrow/camera retention, per-consumer receipt uncertainty/reload/reconcile, inherited directory actions, collection/Token/report/queued continuation, each UI's deliberate stop/resume, narrow screens and real generation reset with delayed old reads. The final lifecycle/reset/no-fallback continuation passed. Earlier failed reports remain preserved, with specific harness corrections and passed checkpoints linked in the [complete evidence ledger](../evidence.md).

Unchanged native/API evidence stays explicitly r1; r2's sole product delta is the Inspector UUID helper. Internal storage/episode/crash/unknown guards and corrupt legacy/unavailable facts use the separately integration-executed native38 results; public clients never seeded managed storage. These layer boundaries were selected before execution. No substantive selected invariant remains without evidence at its assigned layer. No paid-model/production Provider interoperability, complete historical ledger, manual recovery race or continuation causation is claimed.

All seven attempt manifests record unchanged candidate hashes and zero cleanup errors. Independent fixture processes/descriptors were cleaned; acceptance did not target protected previews or user services. Integration accepted the result in #25570 and separately owns authorized live adoption/read-only LAN smoke. Evidence and test writes are now frozen; no further runtime checks are needed for this assignment.
