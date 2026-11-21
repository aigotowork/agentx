---
schema: document
description: Adapt semantic models, retain source basis, interpret finite checks and connect contract tools without overstating proof.
---

# Modeling and bounded checks

Use when a selected semantic model needs a finite executable representation, a source basis, a readable projection or a contract evolution check. [Collaborative modeling](../../scenarios/collaborative-modeling.md) owns derivation and implementation obligations; [data/state modeling](../../scenarios/data-state-modeling.md) owns domain and effect semantics. Do not restart them for a known presentation change.

| Question | Read and produce |
| --- | --- |
| What does a finite checker actually check? | [Adaptation](adaptation.md): properties, finite mapping, atomicity and manual/uncovered obligations |
| What source and observations support this result? | [Basis and results](basis-and-results.md): fixed inputs, report identity, limits and historical reading |
| How do generated contracts and compatibility tools fit? | [Contract tools](contract-tools.md): native contract, supported projection, behavior obligations and honest evolution evidence |
| How do scenarios, native operations and UI actions compose into a task? | [Interaction modeling](../interaction-modeling/AGENTS.md): source-linked bindings, independent coverage, finite path evidence and recoverable guidance |
| Why does a relationship/query expand work? | [Data and query domain](../data-query/AGENTS.md): grain, population, strategy, flow and actual reconciliation |

For executable use, read the installed component's README/help and public declarations: model-check, source-basis, model-inspection, ModelView renderer or the selected contract tool each has its own input and version. Their release inventory must name resources and dependencies; none is an arbitrary source-code extractor or a substitute for the consumer's model. The method can be applied in prose without those programs. Missing execution support stays a specific evidence gap.

For a standalone selected Skill, [model-analysis](../../skills/model-analysis/SKILL.md) supplies a compact derivation flow and [model-adaptation](../../skills/model-adaptation/SKILL.md) supplies cross-method adaptation. Finite checker adaptation remains [here](adaptation.md); the similarly named Skill does not claim to be a Model v1 compiler.
