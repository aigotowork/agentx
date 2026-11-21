---
schema: delivery
description: Design AgentX observability data, domain contracts, state maintenance and engineering governance before Web implementation.
status: completed
stages:
  foundation:
    description: Source-grounded domain designs and coherent data/contract/governance model.
    status: completed
    tasks: [./tasks/architecture.md, ./tasks/runtime.md, ./tasks/configuration.md, ./tasks/events.md]
  handoff:
    description: Independently consume the design through diagnosis scenarios and produce a dependency-correct implementation task system.
    status: completed
    tasks: [./tasks/integration.md]
    depends_on: [{stage: foundation, condition: domain owners release mutually consumable typed contracts and state obligations}]
---

# Observability foundation design

## Accepted design outcome

The controller accepts this design Delivery as completed. All five original design Tasks are completed, all owned writes/resources are released, and the [independent handoff disposition](../../.docs/observability/acceptance.md#independent-handoff-disposition) reports no known remaining contract blocker. The [design entry](../../.docs/observability/README.md) is the durable route to the selected model, typed offerings, state maintenance, engineering/context ownership and consumer boundaries. The prior [daemon Delivery](../domain-daemon/delivery.md#accepted-local-outcome) retains its implemented support and evidence.

| Promised design outcome | Accepted authority and scope |
| --- | --- |
| Clear data/domain structures and native contracts | [Model](../../.docs/observability/model.md), [shared contracts](../../.docs/observability/contracts.md) and runtime/configuration/events offerings define ownership, concrete types, relationships, operations/results/errors, evidence and identity boundaries. They are planned native declarations, not compiled source. |
| Explicit state maintenance and engineering governance | Domain offerings own capture/update/retention/reset/pressure rules; the [engineering map](../../.docs/observability/engineering.md) owns native module/dependency direction, one composition writer, declaration promotion and context maintenance. |
| Complete consumer diagnosis and implementation handoff | [Ten investigation scenarios](../../.docs/observability/scenarios.md), [consumer boundary](../../.docs/observability/consumer-boundary.md), nine resolved contract findings and the [planned implementation Delivery](implementation.md) with ten original Tasks are independently consumed. |

The [acceptance receipt](../../.docs/observability/acceptance.md#machine-checks) records one bounded successful document check over 38 files, 529 local links and 152 heading links, plus metadata/dependency/whitespace checks. Professional scenario walkthroughs and source analysis establish design consistency; neither they nor those machine checks establish executable behavior, performance, native platforms or a rendered management experience. Final controller closure changes only completion/navigation records and does not alter the six hashed domain-design inputs.

No production code, dependencies, installed artifacts, live/default data scopes or services were changed by this design batch. No build, product test, runtime probe or deployment was selected. The implementation Delivery and all ten implementation Tasks remain planned; they own the later native/backend/CLI acceptance followed by local adapter/Web and independent consumer evidence. No further design/review/check or owner work is selected after this closure; a concrete later defect returns to its professional owner within the existing model.

## Selected outcome

Deliver a complete, implementable design for understanding AgentX runtime state from daemon and client perspectives. The user explicitly prioritizes data structure and domain model clarity, contracts, engineering architecture, context discoverability and state maintenance before Web pages. The selected [proposal](../../.docs/proposals/agentx-observability.md) owns this change; [team entry](AGENTS.md) owns professional routes and authority.

This design batch must establish what facts exist, their owner and identity, how they are observed/projected, how their freshness and lifetime work, and how consumers investigate a problem. The downstream implementation plan must assign meaningful outcomes and concrete dependencies, including backend/CLI adapters and eventual Web consumption. Design completion does not claim running instrumentation or a working management page.

## Foundation handoff

The four original domain Tasks have completed their assigned design and released owned writes/resources. The [shared model and contracts](../../.docs/observability/contracts.md), [runtime/client offering](../../.docs/observability/runtime.md), [configuration offering](../../.docs/observability/configuration.md), [event offering](../../.docs/observability/events.md) and [engineering map](../../.docs/observability/engineering.md) are the concrete inputs to independent design acceptance. The runtime final release includes the route-neutral changed-daemon result, exact recorder/reporter/reader contracts, scope-qualified inspection endpoint and independent request/reply evidence. No known owner-local contract blocker remains in those handoffs; integration still owns their cross-domain consumption, final findings and bounded document checks. Foundation completion alone does not complete this Delivery.

## Acceptance

1. A source-grounded context/domain map distinguishes implemented AgentX facts from the planned observation capability. Every selected data object has an owner, stable identity, field/type/unit/cardinality meaning, absence/unknown semantics and native contract destination.
2. Runtime/client, configuration/context and events/retention owners publish consumable commands/queries/results/errors, state transitions, cross-domain references and evidence boundaries. Shared contracts resolve incompatible assumptions rather than hiding them behind generic JSON.
3. State maintenance is explicit: construction, capture/update, observation freshness, concurrency, bounds, cleanup, pressure, restart/reset/generation, missing/partial evidence and durable-versus-volatile retention. Existing business authority and daemon-only live storage ownership remain intact.
4. An engineering map defines planned directories/modules, composition and dependency direction, native versus transport/storage representations, owned context entry points and change-impact/maintenance procedures. A fresh agent can navigate from root to the relevant model, contract, implementation work and evidence.
5. Independent design walkthroughs consume the exact selected contracts for connection failure, slow requests, delivery uncertainty, configuration differences, event gaps, restart/reset and pressure. Concrete findings are resolved with the professional owner. Evidence reports design-level checks and their limits.
6. A planned implementation Delivery and meaningful Tasks are ready to execute in dependency order. Web consumes callable observation contracts; layout/visual polish does not precede backend semantics. No live installation, package replacement or vendor/platform execution is claimed.

## Workflow

All four foundation owners may investigate and draft independently. Architecture releases the shared vocabulary early and merges only actual cross-domain agreements. Integration begins scenario derivation immediately, then closes contract-consumer acceptance when the required domain inputs become usable. Stage dependencies constrain acceptance, not independent preparation. One source owns each fact; the controller does not write specialist solutions.

The existing [accepted local daemon implementation](../domain-daemon/delivery.md#accepted-local-outcome) is the baseline. The [current design entry](../../.docs/observability/README.md) owns the established target decisions and links actual implementation limits.

## Implementation handoff

The [planned implementation Delivery](implementation.md) and its [implementation plan](../../.docs/observability/implementation-plan.md) are the concrete downstream task system required by this design outcome. Native foundation/composition and passive CLI acceptance precede the authenticated local adapter, Web work and independent consumer evidence. The implementation Delivery owns its stage/Task state and dependencies; this design Delivery accepts their adequacy without executing them.
