---
schema: document
description: Current execution authority, professional writes, resources and status maintenance for observability implementation.
---

# Observability implementation team

The user has authorized the full [implementation Delivery](implementation.md) and explicitly requires efficient execution and clear work-item state. This entry supersedes the earlier design-only restrictions for Tasks bound under [current implementation assignments](controller.md#implementation-assignments). The [design Delivery](delivery.md#accepted-design-outcome) stays completed; its contracts and scenario evidence are the selected baseline, not executable acceptance.

## State and recovery

Each original `implementation-tasks/*.md` file owns its status, current work, blocker, evidence and next handoff. Delivery stages own milestone acceptance. Do not copy editable Task statuses into another ledger. Run `python3 .controller/observability/status.py` for a read-only view derived from original metadata; it reports stage prerequisites separately from Task progress. A Task can actively prepare while an upstream implementation is pending, but must name that condition and cannot claim callable behavior until actual imports and implementations exist.

Use `planned` for unstarted work, `active` for the assigned current work, `blocked` with a concrete `blocking_reason` only when the owner cannot progress, and `completed` only when the Task's result, context updates and release conditions hold. Remove stale blockers on resumption. Record meaningful changes in the original Task rather than sending routine chat updates. Controller maintains assignments/resource transfers and releases downstream guards on actual consumable handoffs. After compaction, restore root/team entry, original Task, current native declarations and the actual binding; old design-task completion does not authorize arbitrary writes.

## Professional ownership

| Original Task | Professional owner and writes | Consumable handoff |
| --- | --- | --- |
| native-contracts | Existing architecture owner: shared `observability/{mod,context}.rs`, native guide and one additive main module declaration | Release imports immediately, then main to composition. Domain native declarations remain their owners' responsibility. |
| runtime | Existing runtime owner: `observability/{runtime,registry}.rs` and scoped tests | Reader/recorder/reporter declarations and callable bounded state, exact hooks for composition. |
| configuration | Existing configuration owner: `observability/{configuration,configuration_store}.rs` and scoped tests | Exact refs/capture/lookup/compare/disclosure behavior and load-site insertion instructions. |
| events | Existing events owner: `observability/{events,journal}.rs` and scoped tests | Sink/read/cursor/loss behavior, safe payload mappings and postcommit insertion instructions. |
| composition | One dedicated source-composition owner | Sole writer of existing daemon/service/main/config/host/domain insertion sites and passive lane, with actual lifecycle/reset/report wiring. |
| diagnostics | One dedicated CLI consumer owner | New observe module and additive main/CLI routing under composition's explicit early transfer; native consumer implementation, grammar/output and authored cases for integration. |
| native-integration | Existing independent integration owner | Accepted backend receipts and completed fixture/source release; current executable ownership transfers to consumer-integration below. |
| local-adapter | Dedicated HTTP/session adapter owner after accepted backend | Native web module, finite browser bindings and additive launch routes; implementation/case authorship only, executable resources transfer separately. |
| consumer-integration | Same independent professional under its separate original consumer Task | Sole current executable resource, exclusive scoped lock resolution and independent API/browser fixture preparation; execute only coherent released source. |
| web | Dedicated management-product and frontend specialist | Native API consumer views/state/layout/CSS/cases/build source outside api/; actual rendered/interacted verification remains consumer-integration owned. |

Read applicable professional methods through root guidance. Existing domain models and typed design are sufficient to start implementation; refine a concrete structural defect at its owner rather than restarting discovery or adding a new review team. Structure first, then behavior and meaningful authored cases. Runtime/configuration/events may release public declarations early so their independent behavior can proceed without circular missing imports. Only the native owner edits the facade; publish needed export changes to that owner or obtain a bounded transfer. Guide/design promotion edits are limited to each owner's named sections; shared root navigation remains controller-owned.

## Shared-source and execution resources

- The shared checkout contains accepted prior implementation and unrelated preserved edits. Do not reset, stash, discard, clean, commit, push, change installed programs or touch default/live scopes. Current dirty status is not this Delivery's diff baseline. Preserve prior receipts/artifacts independently from later source changes.
- Composition alone changes existing daemon/service/configuration/business insertion files. Foundation owners send exact callable hooks and source regions, retaining original business policy and mutation authority. Native releases its main declaration region before composition writes it. Diagnostics later receives its named parser/router regions; the local adapter receives only its later additive executable route.
- Only the assigned independent executor runs required source checks, formatters, Cargo/tests/build/package, Node/browser tooling and fixtures: native integration for the completed backend; consumer integration for current adapter/browser work. Other owners author targeted tests and report them unexecuted. No repeated passing suite without relevant changed input; no compile of a knowingly incoherent moving tree. Integration uses a coherent fixed candidate and routes exact diagnostics to the responsible owner.
- Integration may freeze/preserve inputs and run fixtures only in disposable explicit HOME/AGENTX_DIR/XDG scopes. Keep one executor and exact process/root/endpoint cleanup. Existing user DBs/services, remote peers and vendor sessions remain outside this outcome.
- Product compatibility and receipt semantics are preserved. Observation must not change business results, acquire/start during diagnosis, read live SQLite on inspection workers, export mutation tokens or replay uncertain effects.

## Agent platform

Use [AgentX-native orchestration](../domain-daemon/orchestration.md) for implementation assignments and communication. Preserve `codex/gpt-6.1-sol` with `max` reasoning. Historical design-agent bindings do not reactivate assignments. No nested delegation. Send actionable native-contract releases and diagnostics to their assigned consumers through AgentX; keep provider-error recovery with the controller.

The selected outcome includes the native-to-Web investigation and required local package/consumer verification. Backend acceptance must precede actual adapter implementation, and callable authenticated adapter routes must precede Web behavior. Native platforms/toolchains/providers that cannot execute retain explicit support limits, not fabricated passes or new blanket acceptance gates.
