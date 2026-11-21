---
schema: delivery
description: Deliver native consumer services, reproducible OpenAPI, a TypeScript SDK and HTTP CLI for all selected existing public HTTP capabilities.
status: completed
stages:
  pilot:
    description: Prove one exact-Agent message/receipt flow through Service, real HTTP, generated SDK and CLI.
    tasks: [./tasks/pilot-native.task.md, ./tasks/pilot-http.task.md, ./tasks/pilot-consumers.task.md, ./tasks/pilot-acceptance.task.md]
  complete:
    description: Extend the proven chain to all selected management, workbench and observation capabilities and close integrated acceptance.
    tasks: [./tasks/full-native.task.md, ./tasks/full-http.task.md, ./tasks/full-consumers.task.md, ./tasks/full-acceptance.task.md]
    depends_on: [{stage: pilot, condition: Reusable source contracts and derivation are usable and affected source/build resources are released; pilot diagnostic acceptance is not a blanket implementation gate.}]
---

# Service → OpenAPI → SDK → CLI delivery

User authorization: 2026-11-17, complete the scheme/contracts first, then hand control to lezi to assign and advance implementation. [Proposal](../../.docs/proposals/contract-service-projections.md) owns direction; [contract design](../../.docs/api-panorama/service-contract-design.md) owns retained guarantees; [panorama](../../.docs/api-panorama/README.md) owns source-grounded coverage. The current 35 listener-qualified method/path combinations and 59 logical rows are a dated baseline, not a runtime completeness assertion. Reconcile authorized concurrent source changes with the current inventory before claiming closure.

## Required user outcome

A developer can consume an exported TypeScript SDK package or invoke an HTTP CLI to operate selected AgentX resources through maintained native-derived HTTP contracts. Identity, actor authority, revisions, errors, numerical precision, asynchronous acceptance/completion and unknown outcomes retain their native meaning. The scheme can be rerun after a source change without maintaining separate DTO/API/client truth. Existing Web consumers and native CLI remain usable within the affected scope.

The pilot is five operations: descriptor, Agent list, exact Agent read, message submit and original Operation lookup. Full delivery adds the remaining existing public management/Inspector, Workbench operator and Observation reads. No generic SQL/argv/native-request endpoint, arbitrary Agent impersonation, PTY mirror, new provider execution platform, compulsory listener consolidation or internet auth platform is included.

## Schedule by usable contracts

| Task | Start condition | Result consumed by |
| --- | --- | --- |
| pilot-native | Proposal/design and source ownership recovered | pilot-http and consumers use compilable types/semantics; pilot-acceptance uses native/provider path |
| pilot-http | Five-operation shape usable; provider implementation need not be finished to trial schema generation | pilot-consumers uses real emitted spec and precision/codec policy |
| pilot-consumers | Actual first spec and generator probe usable | pilot-acceptance uses public package and CLI artifact |
| pilot-acceptance | Scenario planning can start immediately; execution waits for composed candidate | Full-stage release and pilot closure |
| full-native | Full domain inventory/contracts usable and source resources released | full-http and full-consumers, released per cohesive domain |
| full-http | Corresponding domain types and surface policy usable | full-consumers and full-acceptance |
| full-consumers | Domain-specific emitted contracts usable | full-acceptance |
| full-acceptance | Cases can be prepared at full-stage start; execution follows ready domain candidates | Final support statement, docs and closure |

Human workflow correction: implement complete cohesive domain/consumer batches before concentrated system verification and repairs. Reuse one integration workspace/scope; no per-Task environment or package cycle. The pilot is an early derivation/risk probe, not a mandatory stop before full implementation. Preserve its useful evidence and active build.

No Task's start condition is an assertion that it has an assigned worker or may mutate a contested source lane. Lezi owns actual bindings and resource scheduling. Full stages may be refined into smaller genuine outcomes by lezi without copying semantic facts or declaring pilot support as full completion.

## Acceptance and evidence

Use CS-01..CS-10 in [contract design](../../.docs/api-panorama/service-contract-design.md#7-验收矩阵与演进边界), selecting cases relevant to each stage. Native authors keep focused relevant cases; the acceptance owner supplies independent consumer and actual HTTP/native/worker evidence. A local Provider fixture may establish bounded worker admission/dedup behavior but does not prove actual model delivery or cross-platform support. Message acceptance and reply/task completion stay distinct.

Final closure requires reproducible native/HTTP schema emission and SDK regeneration, actual installed/disposable package consumption, complete method/union/CLI coverage, typed error/precision/recovery branches, affected existing-client checks, documented commands/entries and honest platform/Provider limits. Record immutable candidate/build/source information and how to run/reproduce it. Installed/default executables and live services are a separate adoption outcome; do not silently replace them.

## Accepted local implementation outcome

Controller lezi accepts the complete selected native Service → actual HTTP/OpenAPI → generated TypeScript SDK → separate HTTP CLI implementation for the documented Linux local scope. [Independent final evidence](tasks/full-acceptance.task.md#final-independent-integration-evidence--ready-for-controller-disposition) owns exact candidate/package identities, executed scenarios, original failures and bounded claims. [Package usage](../../packages/agentx-client/README.md) owns developer commands and [OpenAPI entry](../../.docs/api-panorama/openapi/README.md) owns three-document derivation/codec policy. The baseline 35 listener-qualified routes and selected closed unions have actual source/spec/consumer coverage; representative connected flows, rather than every route individually, were executed.

Actual installed SDK/CLI passed exact message submission and original-ID recovery after admitted-response loss, replay/conflict/stale identity/generation, management labels/bytes/mailbox/Inspector, genuine finite native implementer Result plus distinct checker Check through operator Accept, and passive Observation daemon/epoch/configuration branches. Precision/failure/Gap bodies have distinctly labeled finite HTTP evidence. All 57 generated/spec files reproduced identically; installed specs match the candidate. Affected native/schema checks and existing legacy Web build/typechecks passed, as detailed in the independent Task. No unresolved production behavioral defect was established by integration.

### Evidence limits and resource disposition

Actual paid Provider/model delivery or judgment, automatic native Inspector recovery-success, native retention-gap transition, every route individually, non-Linux platforms and source-Web interactive browser acceptance remain unverified. These are explicit support limits, not hidden passing claims or a new implementation backlog. No registry publication, installed replacement or live adoption is included.

The [final resource receipt](../../tests/contract_service_projections/evidence/full-system/final-resource-check.json) records unchanged protected programs/candidate, matching installed specs and zero owned processes. Candidate and source/package/failed evidence remain retained; no source/build/runtime lane is held. Professional source contributions and independent evidence are accepted. Documentation-only current-status updates completed (#49159), and existing affected legacy Web build/typecheck evidence was reconciled (#49106). Aggregate Delivery is completed at the accepted Linux local source/package/integration boundary. No further implementation, verification or documentation obligation remains in this Delivery; no new tests, environments or package cycles were introduced for closure.
