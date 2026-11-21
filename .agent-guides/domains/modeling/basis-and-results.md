---
schema: scenario
description: Retain enough source and model identity to read a result accurately without transferring live authority.
tags: [modeling, provenance, evidence]
---

# Retain basis and read results

Choose the basis needed for the claim. A current link is useful for navigation; a fixed revision identifies a historical source; retained bytes permit offline inspection. The [shared source method](../../scenarios/collaborative-modeling.md#retain-the-needed-basis) owns this choice. Do not snapshot a whole workspace when a bounded source set suffices.

For an actual capture, use the source-basis component's public capture/verify contract. Supply explicit source root and selected files, output root and snapshot choice. Review its symlink, file-size and race assumptions. A recorded source root is historical location, not authority to reopen or write there. Per-file digests do not make a multi-file read atomic, trusted or signed. If coherence matters, freeze inputs or retain the actual consistency mechanism and capture interval.

Verify against an explicitly selected current root; distinguish matching bytes, changed bytes, missing inputs and unchecked freshness. Never rewrite the old basis to make a new source match. A digest change is drift, not automatic semantic failure. Identify affected promises before deciding to recompute or revise the model.

## Choose the reader for the question

| Reader | Input and result | Limit |
| --- | --- | --- |
| Model report inspection | Matched model and checker report → graph, findings and witnesses | Does not authenticate a report or recheck its source |
| Semantic ModelView | Explicit consumer-owned semantic projection → readable entities, operations, scenarios and relationships | Projection completeness and correct domain meaning remain consumer obligations |
| Source basis reader | Selected original bytes, metadata and digest results | Does not generate properties or establish implementation conformity |
| Offline exported HTML | Included data/assets → fixed portable reading | Opening a page does not establish current live state or acceptance |

Use each installed package's full resources and exact schema/version. A generic renderer can consume supplied data without the author's fixed Agents or oRPC builder; those builders remain examples. Do not import a consumer's private source or use its fixture as evidence for a new product.

Check report/model versions, model digest and relevant source association before interpreting results. Keep invalid input, violated property, incomplete exploration, missing evidence and internal tool failure separate. Save stdout/stderr or structured diagnostics with the actual command when they explain the claim. Complete means the selected supported work finished, not that all product requirements passed.

For a portable handoff, preserve original input, report, assumptions/mapping, producer/version, output assets, source selection and omissions. Open the offline artifact from the destination and inspect a normal result and the relevant gap/witness. Reader success proves reading within its contract; it never executes saved actions or restores resources. Close owned processes and keep consumer output outside installed resources.
