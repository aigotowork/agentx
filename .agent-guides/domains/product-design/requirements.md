---
schema: scenario
description: Connect sourced needs to typed requirement relationships, complete slices and acceptance evidence.
tags: [product-design, requirements, acceptance]
---

# Map needs and choose a complete outcome

Use when a feature list does not explain what must work together or how success is judged. Begin with the known actor, situation, desired progress, obstacle, constraints and evidence. Keep one [scenario source](../../scenarios/product-design.md#maintain-one-scenario-source); use local identifiers only where something refers to a requirement or criterion.

A need describes an outcome; a capability groups behavior; a scenario supplies conditions and a path; an acceptance criterion states an observable promise. Pages, APIs and components are designs for meeting those promises. A requirement's selection, evidence support and verification result are separate facts.

## Identify the continuing feature

Treat a product feature as a continuing capability that offers related outcomes to identified users. It can span several scenarios, interfaces, modules and changes. At the product level, feature and capability need not become separate objects; an internal technical capability retains its module meaning. A Roadmap item selects work on capabilities, while a Proposal describes one change. Neither replaces the feature's current definition.

Choose a boundary that explains a useful purpose, responsibility and independent change. A broad capability group can organize several features; a button, entity, package or backlog card is not automatically a feature. Retain an existing feature's identity through refinements. A real split or replacement identifies the old scope, new owners and retained or removed promises.

Features and scenarios can relate many-to-many. Keep a cross-feature scenario with the owner of its complete user outcome; participating features reference its relevant criteria. Shared requirements and domain rules retain their original owners. A criterion states how a rule becomes observable under this scenario's conditions, rather than copying the rule into another editable specification. Use [feature design maintenance](../design-management/workflow.md#maintain-a-features-current-design) to organize those sources.

## Make relationships explicit

| Relation | Meaning and check |
| --- | --- |
| Basis | Evidence supports or challenges a need, hypothesis or choice; a link is not proof |
| Contribution | A requirement is expected to help an outcome; fulfillment does not guarantee business benefit |
| Refinement | Parent promise to more precise children; label all-of versus alternatives and check sufficiency |
| Dependency | A dependent promise needs a prerequisite; distinguish business precondition from implementation order |
| Constraint | A rule applies across named requirements/scenarios, such as permission, accessibility or uniqueness |
| Conflict | Two promises cannot hold in the same stated conditions; different solutions need not conflict |
| Satisfaction and verification | A capability claims to satisfy a requirement; a check and its actual evidence assess that claim |

For each selected requirement, record its meaning, source, scope, owner, important relationships and acceptance link. Split when success conditions, owners or release timing differ, preserving the conditions that must still hold together. A hard prerequisite cycle needs a combined slice or a changed boundary; backlog ordering cannot resolve it.

## Slice across a real task

Build an activity backbone from entry and preparation through the meaningful action, result confirmation, recovery and exit. It is user order, not engineering order. Place alternative ways, scale, channels and detail beneath the activities, then choose a horizontal slice that reaches the outcome. Include handovers between roles and real offline steps.

For a loan example, a complete slice may include identifying one item, authoritative checkout, confirming its return and resolving conflict or an unknown reply. Batch loans or future reservations can be later candidates. “Build database, then all pages” can be an engineering plan, but is not three independently usable product deliveries. A normal-path-only prototype can answer a limited learning question; it cannot establish a dependable borrowing service.

Check all prerequisites against this slice or demonstrably available existing capabilities. Unknown prerequisites remain unknown. Retain necessary failure and recovery; if capacity is limited, narrow the audience/operation or use a real, owned manual handover with adequate capacity. A simulated manual success does not establish operational viability.

## Define feature and interaction promises

Connect the capability map to the selected scenario and its meaningful interaction nodes. The map says which outcomes the product offers and to whom; the scenario says under which conditions a user reaches one outcome; each node says what the user can understand, decide and rely on. A feature list or collection of screens does not establish that the path is complete.

Keep these details in the same scenario source, using prose or a small table where useful:

| At an important node | Make explicit |
| --- | --- |
| Entry and preconditions | Actor, selected object/version, permission and state that make the action available; where the user enters or returns |
| Action and feedback | User intent, meaningful choices, immediate feedback and pending behavior; labels must describe the effect actually offered |
| Observable result | What counts as acceptance or completion, which original identity/result remains available, and which domain fact supports the display |
| Failure and continuation | Rejection, partial or unknown result where relevant; retained draft/context, next supported action, interruption and return behavior |
| Underlying guarantee | The owning domain rule or operation, and any quality constraint necessary to complete this task |

For example, returning to a background report means reopening the original run under current read authority and displaying its observed outcome. Failure to read means the outcome cannot currently be confirmed; it must not become fabricated completion or a new run. A control that only stops waiting should not promise cancellation of execution. Link the domain lifetime/recovery rule and let [cross-interface mapping](../../scenarios/collaborative-modeling.md#carry-a-promise-across-interfaces) identify the implementation owner when it is unclear.

Use the relevant [experience and interface methods](AGENTS.md) to refine navigation, components and presentation; do not freeze incidental layout as a domain rule. Keep adopted interaction semantics with the owning feature/scenario and native interfaces with their modules. Mark planned behavior and actual supported channels/environments separately. A prototype or a passed service check supplies only its own evidence, not a complete product promise.

## Prioritize feasible choices

First retain the conditions necessary for the selected outcome. Then compare complete candidates by contribution, impact and reach, evidence, dependencies, construction/operation cost and delay consequences. Keep reasons; an ordinal judgment is often more honest than invented precision.

Use RICE only when comparable inputs exist: reach in the same population/window, a consistent impact scale, an evidence-based confidence discount and effort in one unit. `Reach × Impact × Confidence / Effort` is an aid; confidence is not a probability of success. Check sensitivity to uncertain inputs and overlapping users. Never score dependent cards independently and remove an essential low-scoring prerequisite. Planning state remains with [feature planning](../../scenarios/feature-planning.md).

## Write promises that can be challenged

Use condition → action/event → observable result, adding failure residue and evidence owner. A decision table can express alternatives more clearly than a forced story template.

| Promise | Required distinction and evidence |
| --- | --- |
| Confirm an operation | Same target and actor, authoritative result and readable UI; a click is insufficient |
| Prevent duplicate allocation | Real concurrent contenders with at most one accepted effect; disabled buttons are insufficient |
| Recover from a lost reply | Preserve original identity; distinguish no effect from unknown effect and use only supported recovery |
| Support a keyboard task | Complete the selected path with visible focus, understandable result and recovery in the promised environment |
| Meet a performance target | Named device/network/load/data/window and start/end boundary, including failed requests and tail latency |

Link each chosen criterion to its check and retained evidence. Behavior correctness, human task usability and outcome improvement need different evidence: a service test, representative task observation and a defined baseline/population/window respectively. Do not turn one into all three.

Judge coverage for a stated actor, outcome, channel, version and environment. Identify which scenarios and prerequisites must hold together for that result, which alternatives can satisfy it and which remain targets. A verified CLI path does not establish a Web path; it can close a deliberately CLI-only result. Two completed features do not establish the handover between them. Keep evidence dimensions separate and explain the composed result instead of assigning a feature-wide percentage from Task or test counts.

On new information, locate which fact changed: problem, scope, service guarantee, comprehension, presentation or evidence. Update its owner and referenced consumers; preserve old runs under their original criteria. Stop when selected promises have sufficient prerequisites, coherent success/recovery and an evidence plan usable by the next owner. Sources: [Patton story mapping](https://jpattonassociates.com/the-new-backlog/), [Intercom RICE](https://www.intercom.com/blog/rice-simple-prioritization-for-product-managers/), [Gherkin reference](https://cucumber.io/docs/gherkin/reference/). Their existence does not require adopting those tools.
