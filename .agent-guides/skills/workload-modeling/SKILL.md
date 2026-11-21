---
name: workload-modeling
description: Model how an API, tool or workflow expands into downstream calls, reads, writes and processing. Use to explain fan-out, compare execution strategies or establish workload inputs for capacity estimates beyond Big-O.
---

# Model workload propagation

Read [the core model](references/model.md) before deriving quantities. It owns population, trigger, sharing and accounting semantics. This Skill needs no analyzer or database connection. Use source, contracts and supplied facts; keep unsupported boundaries named and unknown.

Use these decisions to build a bounded analysis:

1. **Select the scenario.** State the required result, completion boundary, scope, strategy and source/state basis. Analyze only the path needed for the question.
2. **Name populations and parameters.** Identify keys, relation/bag grain, counts, bytes and conditional degrees. Reuse native declarations by reference; retain symbolic values when statistics are unavailable.
3. **Connect operations and steps.** Record each operation's local work law. Bind each step's inputs and trigger to the correct request, item or batch. Represent shared work with one owner and output references.
4. **Compose work.** Derive per-metric formulas from the bindings and local laws. Preserve grouping/batch premises, exclusive accounting, bounds and unknowns. A result limit does not bound preceding work without evidence.
5. **Challenge the conclusion.** Trace decisive formulas to actual source or explicit assumptions. Choose a small observation that could refute the uncertain relationship when needed and authorized. Add calibration, resource pools or timing inputs only if the question requires them.

Keep the result in the selected module's existing analysis/design record. A local question can fit on one page:

```text
Scenario: required result / completion boundary / scope / strategy and basis
Populations: name / key or grain / quantity or constraint / source
Operations: input-output meaning / local work per execution / premises and source
Steps: operation / input bindings / trigger / output references
Result: formulas and units / ownership and sharing / unknowns and coverage
Check: decisive source or observation / what would refute the conclusion
```

Before delivering, check whether calls, references, distinct keys and returned quantities were mixed; every multiplication has a binding and a local law; parent/child or shared work was counted twice; or assumptions/work totals became unsupported timing or capacity promises. Stop when the question has a supported answer or an exact missing premise. Do not reconstruct unrelated modules or enumerate every failure state.

Use an available tool only after checking its public contract against the selected slice. `query-model` can automate its declared finite query/row analysis; it does not automatically support arbitrary file bytes, metadata visits or log work. Keep its physical observation `Operation` distinct from this model's reusable operation definition. Inspect installed help/contracts for actual syntax; manual modeling remains useful when a tool cannot represent the metric.

For visual output, derive a focused graph or table from the same analysis result using the core projection rules. This Skill provides a method, not an automatic source extractor, expression evaluator or renderer. Research examples and project planning are not prerequisites.
