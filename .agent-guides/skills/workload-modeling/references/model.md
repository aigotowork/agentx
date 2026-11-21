# Workload model

Given a scenario, compose input sizes, step executions and local work into quantities with an explicit basis. Capacity planning consumes these quantities. Cache, retry, transaction, parser and scheduler mechanisms stay with their owners; consume their constraints or observations rather than rebuilding them here.

## Four author-facing concepts

| Concept | Responsibility | Minimum content |
| --- | --- | --- |
| Scenario | The question and its bound conditions | Required result, completion boundary, scope, strategy, parameters and source/data/state basis |
| Population | Data consumed or produced | Name, key or relation/bag grain, scope, count/bytes or unknown; degrees conditional on the selected population |
| Operation definition | Reusable work for one execution | Input/output meaning, local work law, premises and source |
| Step | One use of an operation in a flow | Unique name, operation, input bindings, trigger, outputs available to later steps |

`loadProducts` can be an operation and `page.products` a step. One step can execute many times; two steps using the same operation do not imply shared execution. References, distinct keys and executions are different quantities: 1200 product references, 240 distinct keys and 5 batch calls can describe the same scenario.

Keep parameterized definitions separate from scenario bindings. Changed parameters, versions or state produce a different analysis input; use the host's existing identity/reference rather than requiring a global registry. Runtime executions/attempts have their own identities; a static step ID does not identify one physical execution. Compress homogeneous execution groups without creating one author object per runtime item.

## Quantities

Record `name, metric/object, unit, grain or denominator, scope, value/formula, basis`. Inherit common scope and basis once rather than repeating them.

```text
orders = 100 distinct-order-key / request       [scenario assumption]
lines = 1200 relation-row / request             [scenario assumption]
products = 240 distinct-product-key / request   [key union of selected lines]
query.localChecks = F row-visit / execution     [identified source law]
```

A value is known, bounded or a named unknown. Work is nonnegative; exact counts and count bounds are integers. Expectations may be real, but means and quantiles are not exact counts. Assumed, observed, declared and derived describe the basis; an exact assumed integer remains an assumption.

- Add only compatible metrics, measurement meanings and scopes. Orders and calls cannot be added merely because both are counts.
- Explain multiplication/division and resulting units, such as `execution/request × row-visit/execution`. A denominator must be valid and nonzero.
- Separate references, bag rows and distinct keys. Distinct counts require keys, constraints or statistics.
- Use conditional strata or itemwise sums for correlated sizes; do not assume `E[XY] = E[X]E[Y]`.
- Never replace unknown with zero. Exclude a path only when it is known not to execute; retain any work already performed outside it.

Use bound parameter references, finite sums, arithmetic and ceil. A nonlinear local law can use an explicit sourced mathematical formula. A tool evaluates only its declared expression subset; unsupported laws remain quantities with bounds or unknowns, not executable business code.

## Step triggers and input bindings

Bind inputs to scenario populations, visible predecessor `step.output`, or the current `item/batch`. State the scope; never silently substitute request-wide quantities for the current item.

Interpret a trigger relative to one execution of its parent. Nested inputs rebind on each parent execution. Place an owner outside a loop when its output is shared across iterations.

| Trigger | Execution/work rule | Premise |
| --- | --- | --- |
| once | One execution's local work plus actually triggered children | An empty result does not erase work already executed |
| each(P) | `Σ(x∈P) localWork(x)` | Name the iteration grain/key; use `n×w` only for homogeneous work, otherwise strata/bounds/unknown |
| batch(P,b) | `Σ_g ceil(U_g/b_g)` calls | Explicit compatible groups, declared distinct keys, full batches except the last, positive integer capacities; empty groups make no calls |
| choose(condition) | Work of the selected branch | If unresolved, retain alternatives or justified bounds; no expectation without probabilities |
| repeat(n) | Sum of n actual attempts/iterations | Known count or finite bound; varying work needs separate laws or bounds |

For batch, group by compatibility before deduplication; `U_g` is the distinct input count in group g. If the implementation batches bag rows instead, bind to that grain without inventing deduplication. Per-order batching and request-wide batching are different rules. “At most b items” alone does not establish full batches or the ceil formula; use observed counts or justified bounds.

Sequence and data dependencies constrain execution. Concurrency does not itself reduce work or determine time/peak occupancy. Pagination, subscriptions and retries obtain their counts or bounds from their owning module; these triggers compose their effects without implementing those mechanisms.

Express sharing with **one owning step and references to its output**. State reusable key/scope, version/state premises and lifetime basis. A reference does not execute its owner again; the consumer's lookup, validation or encoding remains its own local work. Do not merge steps because names or parameters look alike. Uncertain hits, concurrent misses or partial overlap require bounds/observations/unknowns.

## Local work and accounting

For each selected local item record `metric, per-execution law, measurement boundary, basis`. Choose only dimensions needed for the decision: calls, visits, comparisons, bytes parsed/encoded/read/written, returned rows or others with clear meaning. Logical API bytes do not establish physical device I/O.

For metric m:

```text
W_m = Σ nonduplicated execution groups Σ executions in group Σ local items w_m
```

1. Local items are exclusive by default: count the operation's own work, with children accounted separately. Measurement ownership must not overlap; time intervals may overlap.
2. A metric may instead stop at an inclusive black-box boundary. Included child details explain that quantity and are not added again. A refinement must justify disjoint ownership; unexplained residuals remain unknown.
3. Shared work has one owner. Consumer views may each include it, but system/group totals use the union of owned work items. Inclusive consumer totals are not additive.

An item's identity comes from step, execution scope and local item. Define it once and reference it elsewhere. Different loop executions remain distinct even when they reuse a static item name. Grouping may compress homogeneous executions while retaining their multiplicity and scope.

Unresolved overlapping boundaries leave a coverage gap; do not claim a complete total. Missing statistics, unsupported semantics and stale premises need named reasons. Contradictory definitions, invalid bindings and illegal arithmetic are input errors, not normal estimates.

## Result and projections

Retain the analysis input reference, quantities/derivations, work ownership, needed typed relationships, unknowns and coverage. Sources and assumptions may be inherited by reference. Observation links retain actual execution identity and matching measurement boundaries.

| Semantic operation | Input → output | Responsibility |
| --- | --- | --- |
| derive | Bound scenario + populations/operations/steps → work items, derivations, unknowns/input issues | Check scope, grain, triggers and ownership; compute only supported semantics |
| reconcile | Derivation + compatible observations → differences, contradictions or incomparability | Check scope, state, version and coverage; do not silently rewrite facts or business behavior |
| project | Same result + selected view → graph/table/API representation | Preserve identity, quantities, uncertainty and ownership; do not invent new cost formulas |

These are semantic responsibilities, not available machine APIs. Analysis completion, quantity knowledge and budget violation are independent. Interrupted analysis is partial; completed traversal does not establish complete system coverage.

A projection uses the same analysis input, distinguishes data/trigger/sharing/resource relationships and preserves totals and unknown coverage under grouping. A work graph needs no timeline. A timeline requires ordering/holding evidence; absent evidence means that view is unavailable.

## Optional resource questions

| Question | Additional evidence | Boundary |
| --- | --- | --- |
| Capacity | Calibrated per-scenario service demand in actual resource pools, arrival load, background demand | `D_r = Σ_s λ_s E[d_sr] + background_r`, with compatible units and valid calibration range; provides necessary constraints, not sufficient SLA proof |
| Memory/connections | Holding intervals or direct boundary observations | Peak depends on simultaneous occupancy, not summed cumulative work |
| Latency | Dependencies, service/waiting, scheduling/contention evidence | Calls/work alone do not imply elapsed time or p95 |
| Dynamic state/recovery | Owner-supplied branches, finite counts, state premises or observations | Consume them without reproducing a complete state machine |

Keep work metrics, their bindings to actual pools and pool capacities distinct; they need not become separate services or registries. Without calibration/resource evidence, workload analysis is still useful. Arbitrary recursion, arbitrary cache states, feedback queues and full simulation are outside this finite-flow method.
