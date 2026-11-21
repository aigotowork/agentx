---
schema: delivery
status: completed
description: Deliver a consumable TypeScript domain model and the first independently exercised AgentX object SDK.
stages:
  first_slice:
    description: Recover the native model, publish object contracts, implement the cohesive consumer slice, and validate it against real isolated supply.
    tasks: ["./tasks/domain-sdk.task.md", "./tasks/integration.task.md"]
---

# TypeScript domain SDK: first complete slice

[Proposal](../../.docs/proposals/typescript-domain-sdk.md) owns the selected direction and retained layer boundaries. The expected consumer can select a scope, use Workspace / Role / Agent / Message / Operation meanings and perform the full exact send-and-recovery journey without assembling HTTP requests or importing Web code. Space and Management will later choose their own presentation objects; neither application is migrated in this Delivery.

| Responsibility | Owner and handoff |
| --- | --- |
| Product intent, boundary, integration sequencing and scope judgment | Controller kula, with user acceptance separate. A materially incompatible domain assumption or real missing native capability returns to this owner for an explicit decision. |
| Cohesive domain and SDK | A domain-contracts owner models native-to-TS concepts, chooses inheritance/composition and public surface, implements the selected slice through the current genuine HTTP SDK, writes consumer examples and fixes its defects. Supply an early coherent contract to the integration owner without closing the assignment. |
| Independent public-package/native verification | A separate integration owner derives observable cases from the model, drives the real public import through a disposable Daemon and inspects current/stopped identity, stale scope, bounded supply, and original recovery. They can prepare from a usable model/skeleton and need the working package for final verification. |

Known first decision: current public Agent list query ([generated shape](../../packages/agentx-client/src/generated/management/types.gen.ts)) has no server-side Workspace selector. Establish whether a bounded source-linked projection can meet the first semantic scenario or whether true membership paging is needed. Disclose partial results; do not drain an unbounded global collection or pretend its cursor is native membership paging. Have the domain owner present a concrete contract/supply gap before assigning any native work. Existing source, ongoing local changes, prior candidate artifacts, running daemons/PTYs, installed programs and services are preserved. No production rollout and no registry publish.

Acceptance is the maintained object model, stable names and imports, source-backed semantics, meaningful examples, diagnostic/recovery behavior and retained independent public-package/native evidence. Test same-name successor refusal, old generation and wrong endpoint/scope, partial/discovery coverage, and original request recovery after a response fault (with at most one effect and no hidden resubmit). Cover applicable existing package compatibility once. If host tooling or a missing native contract limits a promised branch, record the observed boundary and owning next step rather than declaring success from mocks. The finished model, declarations and implementation are linked below; the accepted evidence bounds the scope of the result.


## Active bindings and verification supply

AgentX launched `moku` (domain-contracts) on the [model and implementation Task](tasks/domain-sdk.task.md). The [model](../../.docs/model/typescript-domain-sdk.md) and [typed skeleton](../../packages/agentx-client/src/domain/contracts.ts) released the integration assignment; AgentX launched `rife` (coder) on [independent verification](tasks/integration.task.md). The owner selected an explicitly labeled projection of one global Agent page per Workspace member read, preserving continuation and unknown coverage; true server-side filtering stays a concrete follow-up unless evidence requires it for the first journey. Reuse the [retained package/HTTP/native test lane](../../tests/contract_service_projections/AGENTS.md), including the existing response-loss fixture where compatible. Existing package dependencies and native candidates are present; verify compatibility and exact candidate identity before selecting, and build a new isolated candidate only if needed. No test runs against an installed or live service.


## Accepted first-slice outcome

The [TypeScript object model](../../.docs/model/typescript-domain-sdk.md), [additive public import](../../packages/agentx-client/src/index.ts) and [independent caller example](../../packages/agentx-client/examples/scoped-send.mjs) are complete for the selected discovery → exact target → send → original receipt path. The author built the package and passed 15 cases. The [independent outcome](tasks/integration.task.md#independent-outcome) passed an external strict TypeScript/public package check and five real isolated native scenarios, plus Chromium cross-origin selection and existing HTTP CLI compatibility. The transport fault was a deliberately cut real HTTP 202 response: one accepted POST, one corresponding native message, read-only recovery by the original request ID, no resubmit. Same-name successor isolation, reset invalidation and partial workspace coverage were exercised. The exact candidate, package and cleanup are in the [evidence index](evidence/README.md).

Scope of acceptance: exported object source and tested local package against the recorded compatible native candidate; no published registry package, installed/live rollout, application migration, production Provider reply, or native membership-filtered paging. Next consumers can separately adopt a small Space and Management path; extend native membership paging if product requirements need proportional or complete workspace inventories. Keep visual/runtime models in their owners. This Delivery is closed at the requested first slice.
