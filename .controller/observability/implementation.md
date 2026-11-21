---
schema: delivery
description: Implement bounded native AgentX observation, prove a CLI investigation, then enable a local browser consumer.
status: completed
stages:
  native:
    description: Promote the selected shared contracts and establish native module/context authority.
    status: completed
    tasks: [./implementation-tasks/native-contracts.md]
  foundations:
    description: Implement the runtime registry, immutable configuration evidence and volatile event journal.
    status: completed
    tasks: [./implementation-tasks/runtime.md, ./implementation-tasks/configuration.md, ./implementation-tasks/events.md]
    depends_on: [{stage: native, condition: shared native refs and module composition are released}]
  composition:
    description: Wire origin capture and daemon producers with a passive authenticated inspection lane.
    status: completed
    tasks: [./implementation-tasks/composition.md]
    depends_on: [{stage: foundations, condition: all three native offerings and producer handles are consumable}]
  diagnosis:
    description: Expose the complete non-Web investigation path through a passive CLI adapter.
    status: completed
    tasks: [./implementation-tasks/diagnostics.md]
    depends_on: [{stage: composition, condition: existing-scope inspection and real producer publication are callable}]
  backend_acceptance:
    description: Independently exercise the native and CLI investigation scenarios on a fixed local candidate.
    status: completed
    tasks: [./implementation-tasks/native-integration.md]
    depends_on: [{stage: diagnosis, condition: CLI and native authored cases and source releases are available}]
  adapter:
    description: Offer a finite authenticated local browser API over the accepted native capabilities.
    status: completed
    tasks: [./implementation-tasks/local-adapter.md]
    depends_on: [{stage: backend_acceptance, condition: bounded backend and CLI investigation have independent executable evidence}]
  web:
    description: Build the browser investigation path using callable local-adapter contracts.
    status: completed
    tasks: [./implementation-tasks/web.md]
    depends_on: [{stage: adapter, condition: actual typed API and bootstrap rejection/failure paths are released}]
  consumer_acceptance:
    description: Independently verify browser investigation and adapter resource/disclosure behavior.
    status: completed
    tasks: [./implementation-tasks/consumer-integration.md]
    depends_on: [{stage: web, condition: actual adapter and browser path are ready with authored cases}]
---

# Observation implementation Delivery

## Accepted local implementation outcome

The controller accepts the [independent backend handoff](implementation-evidence/native.md#final-independent-backend-handoff) and [independent consumer handoff](implementation-evidence/consumer.md#final-independent-consumer-handoff) for the selected Linux local native-to-browser investigation. All ten original Tasks and all Delivery stages are completed. Those evidence records own exact fixed inputs, artifact identities, executed cases, retained failures, cleanup and limitations; this closure does not relabel earlier failed attempts or add unexecuted coverage.

Repository-local `.bin/agentx` and `.bin/observability-assets` are the verified release executable and actual browser bundle. Start the read-only console with `.bin/agentx observe-web --scope /absolute/existing/scope --assets .bin/observability-assets`, then open its operator handoff URL locally. The adapter does not start a missing daemon. That URL contains a short-lived single-use bootstrap secret; do not retain it in logs or shared evidence. The [browser contract](../../.docs/observability/browser-api.md) and [Web maintenance guide](../../web/observability/AGENTS.md) own use and maintenance details.

All implementation and verification writes/resources are released; preserved evidence and prior artifacts remain intentional. Installed/PATH programs, default/live user scopes and services were not replaced. Evidence covers the documented Linux/Rust1.99/Node24/Chrome investigation; Rust1.89, Node22, other operating systems, live Providers and representative-user research remain unverified. Selective capture, explicit gaps/unknowns and non-atomic cross-owner reads remain product semantics, not missing business effects or replay authority. No further implementation, review or rerun is selected by this completed Delivery.

## Authorized outcome and entry authority

The user authorized execution of this task system after accepting the [design Delivery](delivery.md), requesting efficient progress and clear maintenance of pending, active, blocked and completed work. The accepted outcome above closes that authorization's selected implementation scope. The [implementation team contract](implementation-team.md) and [assignments](controller.md#implementation-assignments) retain actual ownership and resource boundaries. Authorization included implementation and isolated verification through the local adapter/Web outcome; it did not authorize installed/live cutover or reopen the [completed daemon baseline](../domain-daemon/delivery.md#accepted-local-outcome). The [Proposal](../../.docs/proposals/agentx-observability.md), [design entry](../../.docs/observability/README.md), [implementation plan](../../.docs/observability/implementation-plan.md) and [design acceptance](../../.docs/observability/acceptance.md) supply meaning and evidence limits.

The first useful result is a native/CLI investigation that can explain unreachable or congested service, differing captured configuration and discontinuous event evidence while preserving unknown business outcomes. The browser consumer becomes ready only after that result has executable evidence. Visual work never substitutes for backend state capture or contract correctness.

## Ownership and execution

Each original Task owns its execution status. Stage dependencies constrain release and acceptance, not read-only preparation. A named stable source API and explicit routing-region transfer can also enable downstream implementation before the upstream stage closes; full upstream releases still precede whole-candidate freeze and acceptance. [Diagnostics](implementation-tasks/diagnostics.md#callable-entry-and-implementation-assignment) uses composition's actual early API release on that basis. Foundation owners may work concurrently within distinct modules after native shared references exist. All changes to `daemon/{server,transport,wire,mod}.rs`, `service/{context,client,application}.rs` and executable composition have one writer: [composition](implementation-tasks/composition.md). Domain owners supply producer contracts and exact insertion instructions; they do not race on those files.

The native-contract Task releases `observability/mod.rs`, `context.rs` and its module guide before foundation implementation. Domain owners own their native public offerings and private stores. The diagnostics Task owns CLI routing after composition releases executable wiring. The local-adapter Task receives only its named additive executable route after diagnostics releases it. Shared dependency/manifest changes require a bounded transfer to the selected sole writer with the concrete dependency justification, then pass to the named integration owner; no task has an open permission to update unrelated packages.

Implementers author meaningful cases at their owning boundaries and label them unexecuted. The two integration Tasks are independent consumers of those implementations, own necessary test/build execution and record fixed inputs, outcomes and cleanup. Do not reuse the design-only semantic result as runtime evidence, or rerun previous daemon acceptance wholesale. Production source, installed support and platform execution become current only when their own evidence exists.

## Acceptance and exclusions

1. Shared and domain observation declarations are native, discoverable and canonical; design signature copies are replaced with source links on promotion.
2. Runtime/configuration/event producers retain exact scope, era, clock, ownership, unknown and disclosure meaning under failure, reset/restart and pressure.
3. The inspection lane authenticates the actual peer and remains bounded/useful while ordinary workers or the sole Application dispatcher are exhausted; it never reads live SQLite or acquires/starts a daemon.
4. The passive CLI supports the [selected investigations](../../.docs/observability/scenarios.md) without product identity, mailbox, cleanup or mutation side effects. Local attempted evidence remains usable when no server evidence exists.
5. Independent backend evidence establishes authored native and actual CLI cases, including resource cleanup, before adapter/browser work starts.
6. The local adapter and browser preserve safe disclosure, finite read authority, exact references, gaps and historical evidence. Actual rendered/interacted evidence is required for browser experience claims.

Cross-scope fleet discovery, durable telemetry/audit storage, arbitrary environment or process inspection, new business mutation controls, remote browser hosting, live Providers/services, installed cutover and unexecuted platforms remain separate work. None is required to pass this Delivery by implication.

## Handoff and evidence

### Accepted local backend outcome

The controller accepts the [independent backend handoff](implementation-evidence/native.md#final-independent-backend-handoff) for the selected Linux disposable native/CLI scope. It establishes 117 selected ordinary passing cases, separately delimited actual process/configuration evidence, required source checks, and six CLI families on the exact packaged release. The partially failed actual08 parent remains failed; its completed steps and the separate actual-config09 pass retain their original attribution. The inventory of 119 authored cases is not a claim of 119 passes.

That single evidence record owns artifact identities, receipts, cleanup and platform/coverage limits; they are not duplicated here. All backend writes/executable resources are released. This earlier acceptance enabled the local adapter Task; the later accepted implementation outcome above consumes its independent browser evidence without implying installed cutover, universal capture or unexecuted-platform support.

The controller owns Proposal/controller/catalog linkage. The [native evidence record](implementation-evidence/native.md) and [consumer evidence record](implementation-evidence/consumer.md) contain actual executed results, preserved inputs and baseline attribution under their original Task owners. Final closure above consumes their source contracts, scenario results, limitations and resource releases. Earlier preparation/pending statements remain historical within those execution records; the completed design receipt remains separate design evidence.
