---
name: data-query
description: Explain or compare query workload and data-flow cost using explicit cardinalities, execution boundaries and independent observations. Use for query-plan analysis, pagination or hydration work, and result-count reconciliation.
---

# Analyze query work

Collect the exact query or flow, parameter domain, input population, uniqueness/relationship constraints and expected result. Identify whether the question concerns logical rows, physical database work, application hydration, network payload or elapsed time; these quantities are different. For general file/event work or cross-layer fan-out beyond the query path, combine an available `workload-modeling` Skill; this Skill retains query-specific interpretation.

1. Draw the actual stages from source selection through filters, joins, grouping, sorting, paging and hydration. State where execution crosses a SQL, ORM, service or application boundary. Do not infer an unobserved inner plan from its outer result count.
2. Assign counts and units to each stage. Mark each value as measured, exact from a constraint, bounded or assumed. Account for zero/one/many, correlation and conditional multiplicity; a product of independent averages does not establish correlated join cardinality.
3. Predict the requested result and intermediate work for concrete inputs. Preserve unsupported expressions and unknown counts as explicit gaps; never silently turn them into zero. A returned-page limit does not prove the database avoided earlier work.
4. Obtain independent observations from an authorized fixture, query plan, counters or run. Keep the input, query bytes, tool versions and source identity. Compare expected versus observed quantities at matching boundaries and explain discrepancies.
5. Recommend a change only from the relevant workload and behavior constraints. Verify result meaning and order as well as work; include sparse/dense inputs and page boundaries when they affect the claim.

Use the installed `query-model` public help and packaged contracts when available. Its core analysis, SQL parser and ORM example have different dependencies; missing Python or SQL tooling need not prevent manual/core analysis. Do not invent command flags or copy the tool's algorithms into this Skill. Inspect the actual SQL dialect and supported subset before relying on a projection.

Deliver the decision, assumptions, observed differences and remaining gaps in the existing analysis record. A synthetic fixture or symbolic count does not establish production latency or production authorization behavior.
