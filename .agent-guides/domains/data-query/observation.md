---
schema: scenario
description: Reconcile predicted and observed work using matching identities, units and coverage without double counting shared operations.
tags: [data, observation, performance]
---

# Observe and reconcile actual work

Use after a [work model](model-work.md) or when an actual run needs explanation. Choose the specific claim, population, lifetime and layers needed before adding instrumentation. Prefer the existing trace/counter/profile seam; observation overhead and changed timing must remain visible.

Record original run/process/operation identity, request and model-node links, source/model/data/configuration basis, units, start/end boundaries, attempts and outcomes. Attempts, unique physical operations, returned rows, distinct objects and accepted business results are different quantities. A historical receipt is not new execution.

## Preserve causality and accounting

Relate parent work to children through actual cause or supplied links; temporal proximity alone is not causality. Inclusive parent duration includes descendants, so summing it with every child double counts. For overlapping intervals, compute the union within the same clock/domain when meaningful; parallel elapsed time is not the sum of durations. Keep cross-process clock uncertainty explicit.

Deduplicate only records identifying the same physical operation with consistent counts. One operation may support several request or node links: show shared work at each node while counting it once in the system total. Conflicting counts for the same identity are an input problem; do not average them away or allocate arbitrary fractions. Distinct retries remain distinct attempted work.

Keep coverage explicit by source/layer/window. No event can mean uninstrumented, filtered, truncated, not executed or zero work; determine which is supported. Track observation failures separately from product failures. Use bounded fields and fixture data; retain exact raw evidence necessary for challenge before generating summaries.

## Compare only compatible bases

| Result | Interpretation |
| --- | --- |
| Equal | Matching units, scope and assumptions agree for this run; not universal truth |
| Contradicted | Compatible actual evidence refutes a prediction; identify the node/assumption |
| Compatible bound | Observation falls inside a declared interval; the interval is not thereby validated |
| Not comparable | Different source, revision, population, configuration, lifetime or ownership prevents a meaningful residual |
| Partial/unknown | Missing coverage or budget permits only the stated narrower conclusion |

Do not mutate an old run's identity or statistics key to force comparison. Preserve actual evidence even when it cannot be reconciled. A source digest can establish byte agreement, not that observed data or transaction conditions were identical.

Choose the next observation from the discrepancy: wrong loop count, correlated degrees, extra fetch, stale cache, repeated decoding, materialization or resource contention. Query counts may explain work shape but not elapsed time. Use query plans for access paths, causal tracing for repeated work and profiles for CPU/allocation where the question requires them.

Test the selected alternative with the same result semantics and relevant environment, including reset/cold/warm conditions and measurement window. Compare correctness, work, latency and observation cost as supported. Stop when the selected bottleneck/change has adequate evidence or an exact missing input; do not claim an entire system improved from a synthetic fixture. Keep old runs, a revised model and the new run linked under their own identities.
