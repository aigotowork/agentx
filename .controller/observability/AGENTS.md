---
schema: document
description: Team entry and design-only authority for AgentX runtime observability and management foundations.
---

# Observability design team

Read the root [entry](../../AGENTS.md), the selected [proposal](../../.docs/proposals/agentx-observability.md), [Delivery](delivery.md), your original Task, and [controller bindings](controller.md). Restore these after compaction. The previous [daemon Delivery](../domain-daemon/delivery.md#accepted-local-outcome) is completed and supplies the current implementation baseline; this work does not reopen it.

## Current follow-up

For the authorized telemetry optimization, read [coverage and hot-path rules](../../.docs/observability/telemetry-coverage.md) and the [owned task](telemetry-optimization.task.md). Preserve live scopes and the already accepted observation behavior.

For managed Codex hook latency, follow the [source audit](../../.docs/observability/codex-hook-latency.md), [optimization proposal](../../.docs/proposals/codex-hook-critical-path.md) and [implementation task](codex-hook-performance.task.md). The task owns implemented scope, native evidence and remaining conditional work.

For daemon work before reply, follow the [post-commit proposal](../../.docs/proposals/daemon-postcommit-performance.md) and [implementation evidence](daemon-postcommit-performance.task.md#implemented-outcome-and-evidence). Keep ordinary Events wake hints separate from business subscriptions and mailbox delivery evidence. The indexed selector is enabled by default in source and ordinary asynchronous wake hints remain opt-in. Source defaults do not establish live adoption.


The user authorized [direct LAN access and standalone packaging](lan-usability.md), including real scoped daemon/agent activity. That original Task owns current state and supersedes historical bootstrap/loopback/external-assets restrictions.

## Current implementation authority

The [implementation outcome](implementation.md#accepted-local-implementation-outcome) is completed and accepted, with all ten original Tasks completed and all resources released. Native and consumer evidence linked there own actual executable support and limits. Historical design and implementation workflow below does not select new work.

The user has subsequently authorized execution of the implementation Delivery. For current product writes, resources, state maintenance and assignments, read [implementation team](implementation-team.md), [implementation Delivery](implementation.md) and [current assignments](controller.md#implementation-assignments). This route supersedes design-only restrictions below for explicitly assigned implementation Tasks. The completed design and its limits remain unchanged.

## Completed design baseline

The [design Delivery is completed and accepted](delivery.md#accepted-design-outcome). All five design Tasks and their assignments are closed, with no retained writes/resources. The [independent design receipt](../../.docs/observability/acceptance.md#independent-handoff-disposition) and [durable design entry](../../.docs/observability/README.md) are the receiving context. Implementation is now separately authorized; restore its current original Task and actual runtime binding through the implementation route above. The design-team boundaries below remain maintenance guidance and history, not active assignments or executable-support claims.

## Outcome and authority

The user requests a Delivery-led, professionally divided design of the complete observability/management foundation: data structures, domain models, native contracts, engineering structure, context governance and state maintenance before Web implementation. Produce implementation-ready design and a dependency-correct implementation task system. This batch establishes design support, not executable product support.

The [implementation Delivery](implementation.md) and [implementation plan](../../.docs/observability/implementation-plan.md) carry the downstream task system. Its entry conditions and original Tasks now govern the separately authorized execution; design acceptance itself remains historical evidence.

The investigation must explain daemon state from client and server viewpoints, effective configuration/environment, client/host relationships, request activity and a correlated event stream. Reuse native identities and existing domain authority. Observation must preserve unknown outcomes, evidence freshness and scope/incarnation/generation boundaries. Do not infer universal visibility from centralized database ownership.

## Professional split and write boundaries

| Responsibility | Owning design artifacts | Decision boundary |
| --- | --- | --- |
| Architecture and governance | `../../.docs/observability/{README,model,contracts,engineering}.md`, that directory's artifact catalog, focused route in `../../.docs/model/README.md` | Shared vocabulary, cross-domain invariants, offered/required contract integration, planned source structure and maintenance ownership. Domain owners retain their rules. |
| Runtime/client observation | `../../.docs/observability/runtime.md` | Daemon lifecycle, admission/dispatch/request/connection observation, originating-client evidence and bounded runtime state. |
| Configuration/context | `../../.docs/observability/configuration.md` | Scoped and per-invocation effective configuration, provenance, comparison, environment disclosure and state lifetimes. |
| Events/correlation/retention | `../../.docs/observability/events.md` | Observation stream, domain-event references, correlation, ordering, retention, gaps, subscription and reset/restart semantics. |
| Independent design integration | `../../.docs/observability/{scenarios,acceptance,implementation-plan,consumer-boundary}.md`, `implementation.md`, `implementation-tasks/` | Cross-domain scenario walkthroughs, contract consumption, bounded adapter/Web prerequisite design and executable implementation task decomposition. |
| Controller | `delivery.md`, `controller.md`, `feedback.md`, this entry; root route, roadmap and proposal linkage | Scope, assignments, dependencies, design acceptance and delivery closure. |

Each owner maintains their own Task. Shared edits require explicit, bounded transfer between actual owners. No participant modifies production Rust, JS, dependencies, current packaged artifacts or the completed Delivery. Do not create an unused source skeleton during this design batch. Define concrete typed contracts in the owning design, clearly label them as planned, and identify their intended native declaration home; link current native types rather than duplicating them. Implementation will promote these declarations into their native source authority.

## Methods and expected results

- Architecture: apply [engineering design](../../.agent-guides/scenarios/engineering-design.md), [data/state modeling](../../.agent-guides/scenarios/data-state-modeling.md), [source layout](../../.docs/source-layout/README.md) and [design management](../../.agent-guides/domains/design-management/AGENTS.md). Establish ownership and dependency direction before representations.
- Runtime/configuration/events: apply the relevant engineering and data/state methods; inspect actual native declarations and producers/consumers. Record source-grounded current facts separately from selected targets. Define identities, fields/types/units/cardinalities, invariants, commands/queries/results/errors, transition/lifetime rules, boundedness and meaningful failure cases.
- Integration: apply [feature scenarios](../../.agent-guides/scenarios/feature-scenarios.md), [collaborative modeling](../../.agent-guides/scenarios/collaborative-modeling.md) and [management product design](../../.agent-guides/domains/management-product-design/AGENTS.md). Start scenario derivation independently; close acceptance only after usable domain contracts exist. Follow one diagnosis through its actual owners and evidence. Produce concrete implementation work rather than a generic checklist.
- All owners retain consequential alternatives, assumptions and method progress in their design/Task. Record genuine friction in [feedback](feedback.md), through the controller for shared edits. No separate review team, repeated status round or speculative mechanism.

## Runtime and resources

Use [AgentX-native orchestration](../domain-daemon/orchestration.md). Actual participant assignments belong in the controller. Preserve `codex/gpt-6.1-sol` with `max` reasoning. No nested delegation.

Use AgentX messages for actionable dependency handoffs and corrections; normal progress belongs in the original Task. Use AgentX completion events instead of polling. Do not mutate live services or run package changes, Cargo/Node jobs or platform probes without a task requirement. Integration alone may perform the smallest document/contract checks required by this design. Read-only source inspection is available to all owners; release all writes at handoff.
