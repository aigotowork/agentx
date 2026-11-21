---
schema: scenario
description: Derive bounded interaction checks, retain counterexamples and evidence limits, and select affected regression paths.
tags: [interaction-modeling, verification, evidence, recovery]
---

# Check an interaction slice and interpret its evidence

Derive properties from canonical success criteria and domain invariants. State each property's slice, prerequisites, expected guarantee, check method and required evidence. Use [modeling adaptation](../modeling/adaptation.md) for executable finite representations and the selected tool's actual contract. Tables and manual traces remain usable when no executor is installed; unexecuted or unsupported checks stay explicit.

## Select properties that answer the question

| Property | Question | Useful result distinctions |
| --- | --- | --- |
| Reachability | Can the declared starting conditions and UI path reach success? | Reachable, missing entry, missing supply, uncovered |
| Legality | Do object, relationship, version, authority, input and state permit the action? | Allowed, blocked, unsupported, unknown |
| Scope/frame | Does the call affect the intended facet/set/use site/version and preserve unrelated facts? | Scope mismatch, frame violation, preserved |
| Call binding | Is the correct operation/variant invoked with sourced input and proper result mapping? | Unbound operation, wrong binding, missing input |
| State | Are failure, duplicate/late event, cancellation, uncertainty and return transitions defined? | Illegal transition, dead end, stale/unknown gap |
| Feedback/continuity | Can the user see progress, outcome, next action and recovery? | Missing feedback, lost context, unsupported recovery |
| Branch coverage | Are decisive refusal/conflict/partial/unknown/decision branches represented and examined? | Uncovered branch, observed failure |

Keep `unsupported` (outside contract/scope), `blocked` (conditions currently unmet), `unknown` (missing facts or uncertain effect), `uncovered` (not explored/observed) and `observed-failure` (executed and failed) distinct. A disabled control must not conceal absent capability as a temporary block. An unvisited branch does not pass.

Model-internal coverage answers only the model's question. Before a product completeness claim, use [independent inventories](sources-and-coverage.md#build-three-independent-inventories), case challenges and predicate-specific denominators.

## Bound exploration and justify pruning

Declare finite object/version counts, set sizes, permission/capability values, input boundaries, clocks/identities, external events, initial states, maximum path length, state/trace budget and atomicity assumptions. Choose consequential boundaries and refusal values instead of expanding unrelated field products.

Whether using breadth-first search, depth-first search or explicit traces, retain source/model/rule versions, input identity, initial states, discovered/processed/pruned counts, remaining frontier, cutoff reason and property coverage. A budget cutoff means incomplete exploration. “No violation before cutoff” is not exhaustive satisfaction. A found witness survives a cutoff, but does not settle unexplored properties.

Success reachability proves existence of a path, not inevitable completion or fairness. Waiting is a deadlock only under declared external-event and progress assumptions; absence of a modeled completion event cannot silently become proof of failure.

Prune only with a property-preserving argument:

- Remove redundant illegal inputs only when refusal properties are unaffected; retain a refusal witness.
- Merge equivalent states only if future legal actions, properties, result branches and observable identity remain equivalent.
- Omit fields only if frame conditions and result mapping do not depend on them.
- Normalize symmetric objects only under declared symmetry; differing identity, history, permission, scope or error meaning can break it.
- Retain decisive duplicate, late, concurrent and unknown events. Their removal often deletes the very counterexample the model should find.

For each omitted combination, explain why it cannot change the selected property. “Unlikely,” random sampling and lack of a UI entry are not safe-pruning arguments. A few scripts or random traces are selected evidence. Do not strengthen guards during adaptation to repair the modeled defect.

## Counterexamples and gap owners

Prefer the shortest reproducible counterexample: fixed source basis → initial `D/U/A/E` → user action/external event → call/result → changes and unchanged facts → violated property → owner. Merge symptoms sharing a cause while retaining triggers that distinguish responsibilities.

| Category | Meaning | Receiving owner/action |
| --- | --- | --- |
| `scenario-gap` | Goal, success, scope or user decision is undefined | Product/scenario decision |
| `domain-gap` | Object, relationship, state, rule or recovery semantics are absent/conflicting | Domain model correction |
| `contract-gap` | Native operation/schema/result cannot uphold the promise, or projection loses meaning | Native contract owner |
| `supply-gap` | Necessary identity, version, permission, capability, input or later fact has no traceable provider | Data/contract/service owner |
| `ui-gap` | Supported capability lacks entry, clear scope, feedback, return or recovery | UI/product owner |
| `binding-gap` | Wrong control/action/procedure/variant/parameter/result mapping | UI/adapter owner |
| `state-gap` | Missing/illegal transition, late/duplicate/unknown/concurrent behavior or retention rule | Relevant domain/UI state owner |
| `evidence-gap` | Static/model support is insufficient for the claimed execution/user/production boundary | Owner of the required observation |
| `tool-gap` | Selected tool cannot express, discover or check the necessary question | Tool owner; retain manual/uncovered obligation |

A finding retains ID, property, trigger, minimal trace, actual call/return if observed, changed/unchanged facts, source locators, owner, classification, evidence type, reproduction path and next action. Link model and UI findings without merging their evidence meaning. Useful dispositions include fix-model, fix-binding, extend-contract, add-evidence, defer and out-of-scope, with reasons.

An undecided promise is not an implementation bug. Adding a button cannot supply a missing contract; a successful fixture cannot close a real-service evidence gap. Label injected negative cases `synthetic-negative`, with scenario/path scope. They can demonstrate a checker's sensitivity, not current product defects. Bound branch labels, checked condition worlds and executed business branches are different facts.

## Keep evidence types separate

Mark evidence beside each conclusion; a single “verified” label or color hides material differences.

| Type | Establishes within scope | Does not establish |
| --- | --- | --- |
| `model` | Declared relationships, state/transitions, owners, assumptions and properties | Implementation agreement, discovery or actual effects |
| `check` | Result under fixed model, finite domain, pruning and budget | Infinite state, production concurrency/authorization or whole-product correctness |
| `ui-replay` | Actual actions, calls, views and feedback on the selected environment/data/path | Unvisited states, independent users, long-term reliability or production service |
| `user-research` | Observations/reports for named participants, task, environment and assistance | All users, production success rate or contract correctness |
| `production` | Observed production behavior for a named version, tenant/data, authority, path and time | Other paths/versions, universal UX or exhaustive model correctness |

Also label supporting `static-source`, `schema`, `fixture/mock` and `compile` evidence. They do not automatically become stronger evidence types. Mock-backed UI is `ui-replay` with an explicit simulated boundary. Real HTTP or a local/test service establishes only the executed transport/service boundary; it is not production evidence. Expert walkthrough and tutorial execution are not user research.

## Make results readable without another fact source

A console/report is a projection and investigation entry, not a business authority. Support a reading path through scenario/criterion → object/relationship → state/path/call/result → finding/counterexample → original evidence and owner. Show source/time/environment, cutoff/frontier, unresolved discovery, unmatched inventory rows and simulated supply where they affect the conclusion. Preserve return context when opening original sources.

Use the [card and edge semantics](method.md#object-cards-and-relationship-graphs) and [source restrictions](sources-and-coverage.md#read-the-source-behind-a-claim). Graph, matrix and trace views should derive from the same records. Source access, classification, binding and behavior each need their own status.

Tutorial links should identify criterion, initial identity/version/scope, action binding, expected visible result, decisive failures and continuation/evidence. A tutorial's completion claim requires a result assertion. Click or request acceptance suffices only for a step explicitly promising that presentation/request result. Use [guided demos](guided-demos.md) for execution and recovery.

## Select regression by changed meaning

Follow references from changed criterion, object/relation/state/rule, native contract/result, action/control/surface, return promise, property or tool projection. Find affected scenarios, state, calls, tutorials, reports, UI observations and owners rather than guessing from filenames.

| Change | Relevant response |
| --- | --- |
| Semantic: goal, identity, scope, state, authority, result, effect or recovery | Compare old/new promises; check affected normal and decisive refusal/conflict/unknown paths |
| Binding: entry, input, variant, surface or result mapping | Exercise affected action and a decisive scope counterexample |
| Presentation: label, position, layout or visible state without semantic change | Review affected discovery, focus, feedback and return through [Web review](../webdevelopment/review.md); reuse the domain model |
| Evidence: data, environment, tool/model version or run boundary | Preserve prior report, mark drift and refresh affected evidence without rewriting history |

Choose the smallest sufficient set that can change the conclusion: relevant properties, normal path, decisive adverse branch, required continuity and affected tutorial assertions. Add complete input-provider/recovery checks when a changed contract crosses that boundary. Reuse unaffected evidence only while its sources and assumptions remain valid. Do not repeat passing checks without a relevant change.

## Stop at the established boundary

Close the selected question when authoritative axes and bindings are traceable; current/target/assumption are distinct; applicable properties/branches have results or explicit coverage limits; and findings have counterexamples, owners and next actions. Finite runs retain domain, pruning, budget and cutoff. Required source/tutorial navigation returns to original authorities. Completeness work also meets the [inventory exit conditions](sources-and-coverage.md#report-coverage-with-owners-and-dispositions).

Missing tool, service, participant or production conditions limit the available evidence. Report the specific gap and receiving owner rather than widening assumptions, fabricating results or creating production access. Closing an inventory/model review leaves implementation and delivery acceptance separate.
