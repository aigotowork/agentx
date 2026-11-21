---
schema: scenario
description: Interpret explicit SQL lineage, conditional cardinality and plan evidence at a supported dialect and version boundary.
tags: [sql, data, query-analysis]
---

# Analyze SQL with explicit inputs and limits

Use when field origin, row expansion or index choice is the question. Supply original SQL, explicit dialect, actual bound parameters and catalog. Parameter values remain data; do not interpolate them into SQL. For ORM consumers, retain emitted SQL and bind values from the real invocation; entity declarations alone omit loading strategy and repeated calls.

Read the installed SQL addon's public input contract, parser version and resource limits. The current source profile uses Node 24, Python >=3.9 and SQLGlot 30.21.0; the selected release manifest owns the supported artifact/runtime pairing. Select an explicit prepared Python executable from the consumer environment. A missing parser is a capability diagnostic, not an empty successful report, and does not disable the pure work analyzer. Follow the installed setup recipe rather than an author's `.venv` path.

## Keep three questions separate

| Question | Inputs and output | Limit |
| --- | --- | --- |
| Where do fields come from? | SQL/catalog → output ordinals, expressions, column lineage, alias/CTE/subquery scopes | Value lineage differs from filtering dependencies; duplicate names need ordinals |
| How many rows result? | Selected population and supported relationship statistics → stage-by-stage expansion | Parser acceptance does not establish cardinality support for every SQL shape |
| Which access path is used? | Actual engine/schema/index version and retained EXPLAIN → selected path | Candidate index is not selected index; plan cost is not measured milliseconds |

An input key binds SQL/parameters/catalog according to the tool's canonical rules. Statistics and plans must retain the key from actual collection, plus data/basis identity. Never rewrite historical keys to match new input. A separate statistics digest distinguishes changed distributions even if SQL and prose basis stay unchanged.

No-filter table count can describe a root population where the contract permits it. With WHERE, use statistics for the selected population; full query result rows are not the root count. For a chain, each join's groups cover all current input rows. LEFT preserves unmatched inputs; a declared unique relationship can supply a bound only under the selected engine's actual nullability, affinity, collation and index conditions. A foreign key declaration alone does not establish enabled enforcement or a match.

Root-only deterministic filtering can sometimes be modeled before joins; relationship filtering needs its own downstream boundary. Missing final filter statistics stay bounded/unknown, not zero. CTEs, UNION, aggregates, windows and LIMIT may have useful lineage while expansion remains unsupported. Retain those independent coverage results.

## Acquire and interpret actual plans

Database access requires an explicitly supplied, authorized connection and scope. A catalog adapter should read only named tables, and an EXPLAIN adapter should not silently execute the target query or scan data for counts. The host owns transaction consistency and closure. Prefer a disposable representative fixture when production access is unnecessary.

Index-prefix heuristics can suggest simple equality/range candidates; expression, partial, collation and OR semantics often require actual plans. SQLite `SCAN USING INDEX` is still a scan; `SEARCH` describes a lookup/range path, not rows visited or proven speed. Plan text can change by engine version. An unsupported-version plan remains retained raw evidence with an interpretation gap.

Bound parser input/output, time and cancellation through the public contract; stop and await the owned child on timeout. Keep invalid, cancelled, partial and complete outcomes distinct. Parsing another dialect is not evidence of its real engine execution. Finish with a useful lineage/expansion/plan result and explicit unsupported coverage, then [reconcile](observation.md) actual work before claiming an optimization.
