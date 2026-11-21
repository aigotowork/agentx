---
schema: scenario
description: Derive work from required result grain, selected populations, relationship degrees and explicit execution flow.
tags: [data, query, performance, modeling]
---

# Model the work before estimating time

Start either from a real slow scenario with source and observed inputs, or from a new design with explicit assumptions. State what the user needs, the selected scope, source revision, data version and resource lifetime. Do not infer the cost from an entity diagram alone.

For composed work formulas, use the [workload core](../../skills/workload-modeling/references/model.md), which owns quantity meaning, input scope, triggers, batch premises and accounting. The sections below specialize it for data/query paths.

## Separate the views

| View | Meaning |
| --- | --- |
| Domain | Identity, relationships and invariants |
| Necessary result | Fields, subset, ordering, completeness, result grain and missing-value meaning required by the task |
| Execution trigger | Which action or demand causes work, including repetition and lifetime |
| Physical access | Carrier/table, query, load/join strategy, index, materialization and cache behavior |
| Resource/deployment lifetime | Connection/process/cache scope, sharing, concurrency and cleanup |

Map domain instances to physical carriers explicitly: one entity/table is a starting shape, not a universal rule. A repeated SQL row is not a distinct domain object; a JSON field, join table or read model may hold different grains. An optimization may omit a read only when the original required result and consistency promise permit it.

## Establish population and expansion

Bind query populations to the selected filters and version. Table-wide averages may no longer apply after filtering.

For each relationship, state join keys, optionality and conditional degrees. Multiple child joins multiply row bags for each root, not necessarily the marginal averages. Correlated groups need joint strata or another defensible bound. A LEFT join contributes at least one row per input even when no child matches; an INNER join can remove it. Successive distributions cover the previous step's output grain.

Illustrative only: two roots have child degrees `(1,1)` and `(9,9)`. Joining both child collections yields `1×1 + 9×9 = 82` rows, while `2×average(1,9)² = 50` is wrong. Summing a parent or one child value after such multiplication may also produce an incorrect total. Check correctness and cost together.

## Model calls separately from rows

Describe the strategy: per-owner load, batch, join, aggregate-only, cache/provided data or an explicit opaque boundary. Bind the workload core's triggers to actual query outputs and keys. A batch-size limit needs the core's compatibility and filling premises before it determines a query count.

A join may reduce round trips while increasing returned rows, bytes and materialization. Compare a work vector: calls, scanned/returned rows where known, bytes, objects/materialization, peak retained memory, writes and finite resources. Keep unavailable dimensions absent/unknown. Predict work before latency: CPU/I/O, cache, queueing, concurrency, access plan and hardware determine how work translates to elapsed time.

For each alternative, preserve the same selected objects, complete child subset, ordering, result grain and missing semantics. Record an equivalence basis. A faster query returning a different result is a scope change, not an equivalent optimization.

## Use a bounded analyzer honestly

Supply explicit source identity, model revision, scene, population/configuration and lifetime, entities/relations/keys, selected population/degree evidence, strategy and flow. Use the installed public contract for finite shapes, budgets and overflow behavior. Unsupported recursion, parallelism, writes, cache or arbitrary SQL may be opaque; do not invent internal costs.

Interpret known, bounded and unknown independently from complete/partial/invalid/cancelled analysis. Exact assumed integers remain assumptions. Review findings even after complete analysis. Keep both machine JSON and human report derived from the same report object where supported. Continue with [actual reconciliation](observation.md) to challenge the prediction; a plausible model alone does not establish a bottleneck.
