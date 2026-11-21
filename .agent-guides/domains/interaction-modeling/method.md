---
schema: scenario
description: Build a source-linked scenario, domain and UI slice with explicit object roles, state transitions and action bindings.
tags: [interaction-modeling, scenarios, state, ui, contracts]
---

# Connect a scenario, its domain and its interface

Use to answer: under these starting facts, can the user reach the promised result through the actual interface, with correct scope, preserved facts and a continuation after failure or uncertainty? The model connects authoritative sources. It does not replace their rules, fields, requirements or components.

## Establish the three axes

Select a canonical scenario criterion. Record actor, goal, observable success, starting facts, object/version/scope, permissions, input providers and decisive failure/unknown branches. Reference the criterion rather than maintaining its text again.

| Axis | Authoritative source | Connecting information |
| --- | --- | --- |
| Scenario | Existing product requirements and scenarios | Criterion/node reference, actor, prerequisites, normal and alternative outcomes |
| Domain | Native objects, relationships, state, methods and contracts | Object roles, identity/version, state/rule/operation references, input/result/effect sources |
| UI | Actual surfaces, controls, handlers and observed paths | Entry/target, action meaning, input binding, local state, feedback and return behavior |

Label `current` source facts, `target` selected design, `assumption` unconfirmed interpretation and `observed` executed behavior separately. A UI label cannot establish service support. Give unresolved references an owner; do not use them as authoritative check inputs.

Maintain this chain and its reverse links:

```text
scenario criterion → action → control/surface → call or local effect
                   → domain/UI result → retained context → next state
```

Reverse reading must answer both “which fact/provider does this criterion need?” and “which object, variant and criterion does this control serve?” Navigation and draft editing can have local effects without an RPC. One action can compose several calls; several controls can expose one action.

## Keep the connecting model small

Use these concepts where they help. Their names describe semantics, not a required tool schema.

| Element | Owns in this model | Leaves at the original owner |
| --- | --- | --- |
| Scenario reference/node | Criterion reference, actor, prerequisite and branch links | Requirements and research conclusions |
| Subject/relation binding | Native object/type, identity, scope, version, endpoint roles, keys and relation references | Object fields and domain rules |
| State slice/dimension | Relevant observations, ownership and version | Full business state and independent status replicas |
| Operation/call binding | Native variant, input source/transformation, order, results and effect boundary | Procedure registry and contract definitions |
| Action/control binding | Intent, surface, object facet, scope, feedback and recovery | Component implementation |
| Flow | User/system/wait/event steps, conditional edges and terminal/return links | A second scenario source or menu tree |
| Property | Expected guarantee, preconditions, finite domain and evidence needed | A guarantee inferred solely from implementation output |
| Trace/finding | Source basis, events/calls/results, changed/unchanged facts, violation and owner | Unsupported claims of correctness or user preference |

When useful, observe composite state as `X = (D, U, A, E)`: domain snapshot; UI selection/focus/draft/surface/comparison/return context; actor/permission/capability prerequisites; external requests/events/unknown outcomes. These are slices of different owners, not four state stores. A UI draft is not a committed domain fact.

If information changes a business rule or service promise, return it to that owner. If it only controls display, focus or navigation, keep it with the UI. Store the relationship between them here only when it helps check reachability, scope, continuity or coverage.

## Object cards and relationship graphs

When readers need object understanding, expose a compact source-backed card in the existing model/view. Separate a concept card from an instance view with selected identity, version and observation time.

| Concern | Explain or link |
| --- | --- |
| Meaning/responsibility | Purpose, owner, example and a nearby concept it must not be confused with |
| Identity/lifetime | Entity versus value, identity provider, scope, revision, creation/end conditions and invariants |
| Properties/provenance | Relevant property → read/declaration/derivation → meaning, unit and freshness; distinguish schema constraint, stored fact, computed projection and draft |
| Methods/outcomes | Native operation/variant, actor, guards, input sources, results/errors, effects, unchanged facts and continuation |
| Input/output roles | Which operation consumes, creates, returns, changes or references each role; binding keys and scope/version agreement |
| Relationships/dependencies | Endpoint roles, relation kind, conditional necessity, cardinality/optionality and evidence |
| Scenario uses | Canonical criterion, object role, read/mutation, facet/scope and expected result; label candidate uses as candidates |
| Source/support | Located original source, selected version, current/target/assumption status and evidence type |

Omit irrelevant concerns with a reason; keep unknowns visible. A type describes accepted shape, not who supplies a value, whether it is fresh or whether all rules are enforced. Trace consequential properties to their actual provider. Valid sample values do not close a supply gap.

Use one relationship definition for card and graph projections. Every consequential edge retains endpoint roles, kind/direction, condition, scope/version, cardinality, owner and evidence. Distinguish containment, reference, consumption/production, derivation and scenario use. Imports, shared names, nearby boxes and similarly shaped IDs do not establish a relationship.

Keep relation kind, dependency strength and certainty separate:

- A hard dependency means the named outcome cannot satisfy its promise without the provider under the stated condition. Name the missing-provider consequence and source rule. Define arrow convention, such as consumer → required provider. An alternative provider can invalidate an unconditional dependency claim.
- An association establishes linkage without proving necessity, order, ownership or authorization.
- An inferred or unknown relation stays visibly unverified, even if the proposed kind is a hard dependency. Exclude it from confirmed coverage paths. An incomplete graph also cannot prove absence of a relation.

Label edges and legends in text; color alone is insufficient. Unknown cardinality is not `1:1`. Explain dashed lines. Keep ownership, data flow, dependency and scenario-use views distinguishable. Reachability in a graph is not task completion; a cycle alone is not a defect. Disclose collapsed/omitted nodes and retain source links.

Support scenario → object → related role/method → located original source → return to the same selection. Use [source reading](sources-and-coverage.md#read-the-source-behind-a-claim) when assessing a claim.

## Model state where behavior depends on it

Use explicit transitions when meaning depends on state, history, time, asynchronous events, permissions, versions, retries or concurrency. An unconditional display/value transformation needs only its actual constraints.

```text
transition(state, event/input) → Accepted(next, effects)
                              | Rejected(reason, unchanged)
                              | Unknown(observation, recovery)
```

For each consequential transition, retain source state, trigger, guard, next state, changes, calls/effects, unchanged facts and rejection/duplicate/stale behavior. A command requests an effect; an event records an established fact. UI `allowed` reflects a snapshot; the service rechecks at its commit boundary.

Use hierarchy for shared behavior and independent dimensions for concurrently progressing facts. State their interaction constraints rather than enumerating a needless Cartesian product. Domain lifecycle, surface lifecycle, request observation and tutorial progress are separate. Closing a panel does not cancel remote work; stopping observation does not prove cancellation.

Cover applicable entry/completion, permission/refusal, missing input/decision, stale version/conflict, partial result, duplicate/late event, cancel-wait/cancel-request, unknown effect, return/close, invalid object and recovery transitions. Keep recovery grounded in [domain effect semantics](../../scenarios/data-state-modeling.md).

## Bind actions, controls and calls

| Binding | Resolve |
| --- | --- |
| Intent | Select, open, edit draft, submit, observe, navigate, close or another precise user operation |
| Control | Surface/region, discovery path and why it is visible, hidden, enabled, disabled or pending |
| Subject/facet | Stable identity, use site, version, candidate, selected set or comparison baseline |
| Input | Selection, draft, prior result, query or trusted context; transformation owner and any simulated provider |
| Call | Native procedure/variant, sequence and consistency boundary; no invented call for local behavior |
| Result | Accepted, ready, applied, rejected, partial, unknown, stale and error mapping to domain/UI states |
| Continuation | Visible feedback, next action, recovery and retain/invalidate rules for draft, selection, comparison, focus and work identity |

Disambiguate identical verbs by facet and scope: deleting one use of a shared asset differs from deleting its reusable definition; copying an object differs from copying its styling. Reference actual discriminants rather than counting endpoints as capabilities.

A multi-call path is not automatically atomic. Track each completed, absent and unknown effect plus actual rollback/compensation support. A created resource followed by a conflicting application is a partial result, not “nothing happened.” A computed preview differs from a user having reviewed it; acceptance differs from completion; canceling observation differs from canceling execution.

Intermediate calls may operate on other objects than the final user target. For example, create a reusable resource, then apply it to one use site. Check the roles, relationships, input provenance and final unchanged facts; do not require every call's facet to equal the final target. If a checker cannot express that distinction, retain an uncovered obligation rather than distorting the binding to pass.

## Leave a reviewable slice

Use prose, a table or a supported tool format in the existing owner record:

```text
Slice / canonical scenario criterion / source basis:
Initial D/U/A/E, identity, version, scope and input providers:
Observable goal and facts that must remain unchanged:
Ordered user/system/event path with conditional branches:
Action → control → call/local effect → result → next state:
Properties: legality, reachability, scope, feedback and continuity:
Applicable normal/refusal/conflict/partial/unknown/stale cases:
Evidence per conclusion and gaps with consequence, owner and next action:
```

Start with one small complete outcome before expanding. Use [checks and evidence](checks-and-evidence.md) to judge it, and [independent inventories](sources-and-coverage.md#build-three-independent-inventories) only when the question includes completeness beyond that slice.
