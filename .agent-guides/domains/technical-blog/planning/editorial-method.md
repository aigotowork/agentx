---
schema: document
description: Develop topics and series from reader decisions, defensible claims and useful explanatory results
---

# Develop the editorial argument

Use for topic selection, series design and substantial reframing. Start with the reader's situation. An explanation should help someone understand a mechanism, choose an approach or diagnose a class of failures. Exact product setup and interface reference retain their existing documentation owner.

## Define the reader's gain

Choose one primary reader and intended outcome. Supply small prerequisites for adjacent readers; avoid promising to teach every experience level everything in one piece.

| Question | Useful answer | Insufficient substitute |
| --- | --- | --- |
| Who faces which decision? | A maintainer choosing where to enforce a shared capacity limit | Developers interested in architecture |
| What can go wrong? | Two requests both trust the last free slot they read earlier | The code becomes inelegant |
| What must the reader know? | Reads and commits happen at different times; each request can retain an old observation | Familiarity with the entire stack |
| What can they do afterward? | Identify the invariant and construct a distinguishing event schedule | Appreciate the product's design |
| What remains outside scope? | Retry identity, billing policy and database-specific isolation details | Everything outside the title |

Name the reader's expected output: a decision rule, state model, diagnostic procedure, operation or reproducible experiment. The useful conclusion should survive a change of product name or directory layout.

## Select a problem with a defensible path

A technology name supplies context. Select an architectural choice, reproducible failure, surprising constraint, disputed trade-off or migration problem. For example, “Two requests compete for one remaining slot: where must the capacity condition live?” identifies an invariant, competing operations and an observable failure.

An adequate topic has a consequential question, plausible alternatives where relevant, a bounded mechanism and a source or experiment path capable of supporting the claim. Check its difference from existing material. Reframe if it merely repeats Docs, depends on invented results or needs substantial production whose reader value cannot be stated. A missing fact can become a research question; it cannot become a guarantee.

## Connect a series through questions

State the series' central question and how each piece contributes. Link prerequisites, deeper consequences and changes of scale. A progression might ask where responsibility belongs, what one operation does, what survives failure, which observation supports the guarantee, and what may change safely. Technology tags alone do not establish that progression.

Each piece should still deliver a bounded result on its own. Link shared explanations rather than reteaching them or requiring completion of a numbered course. The publication's actual themes, schedule and audience strategy stay with its owner.

## Use intermediate results that change a decision

| Work | Useful result | Reconsider when |
| --- | --- | --- |
| Frame the situation | Reader, trigger, decision and provisional claim in the [brief](article-brief.md) | The title fits almost any engineering topic |
| Investigate and model | Claim/evidence map; objects, boundaries, state and invariants needed for the argument | Sources disagree or describe only a target design |
| Challenge the argument | A counterexample or comparison that can change the conclusion | Evidence covers only the preferred happy path |
| Design expression | Primary form, intended observation, action sequence and material roles | The decisive relation remains hidden |
| Produce | The selected artifact with context, evidence and limits | The artifact promises more than its evidence supports |
| Review and deliver | An explanation that meets the reader promise and its actual host checks | The reader cannot reconstruct the reasoning or use the result |

These are dependencies in reasoning, not a required file sequence. Early sketches can guide research; consequential claims still need their basis before publication. Select the primary form through [expression design](expression-design.md) before a prose outline. Keep decisions in the existing Task rather than creating one record for each stage.

## Make each part advance the model

Choose expression for the reader's difficulty. Use boundary views for distinct responsibilities, sequences for temporal behavior, aligned cases for competing designs, defined quantities for thresholds, and compatibility stages for evolution. Each section or scene should add a relationship, observation, alternative, deduction or usable action.

Explain concrete consequences: which fact survives, which request is rejected, what cost increases and where the rule stops applying. Repeated tool lists, claims of importance and detached product praise do not provide that increment. Finish with a judgment the reader can use in a changed system.

Planning ends at a reviewable brief with a supported direction and explicit unknowns. Production additionally requires the selected artifact; publication additionally requires [actual renderer and release checks](../publication/renderer-and-release.md).
