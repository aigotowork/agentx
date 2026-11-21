---
schema: document
description: Explain data grain, relationship expansion, query strategies and actual work using explicit scope and evidence.
---

# Data and query work

Use for a slow data path, new access design, join/loop expansion, loading-strategy comparison or SQL analysis. Keep [domain/storage/transport meaning](../../scenarios/data-state-modeling.md#domain-storage-and-transport-representations) with its original owner. This domain models work and evidence; it does not choose the product's database or authorize queries.

| Need | Route and result |
| --- | --- |
| Connect general API/tool read-write fan-out across layers | [Workload modeling](../../skills/workload-modeling/SKILL.md): shared quantity, trigger and accounting semantics; query-specific meaning stays here |
| Explain or predict a query path | [Model work](model-work.md): required result, population/grain, loading strategy, flow and bounded work prediction |
| Compare prediction to a run | [Observe and reconcile](observation.md): identity, units, coverage, shared-operation accounting and corrected assumptions |
| Extract SQL lineage or inspect index/cardinality evidence | [SQL analysis](sql-analysis.md): explicit dialect/catalog/parameters, supported coverage and actual plan limits |

Use the installed query analyzer's public CLI/API, declarations and examples for exact fields. Core analysis accepts explicit models/runs/source files; the SQL addon has its own parser/runtime prerequisites. Neither a model reference nor an SQL string grants database access. Retain results under the consumer's output root and close only owned resources. Missing SQL support does not prevent using the pure core or reasoning from explicit data.

The standalone [data-query Skill](../../skills/data-query/SKILL.md) provides a compact entry. In a harness-kit release, select component `query-model` and its manifest documentation resource under `docs/tools/query-model`; `query-model --help` owns actual CLI syntax. Core and SQL capability readiness are separate.
