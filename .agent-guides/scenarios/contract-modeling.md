---
schema: scenario
description: Connect canonical scenarios, native declarations, caller supply and owned contract projections
tags: [contracts, modeling, native-schemas, callers]
---

# Derive consumable native contracts

Use when deriving a domain boundary from scenarios, changing native schemas/procedures or checking whether callers can obtain the facts and outcomes they need. Reuse established declarations for a bounded change. Presentation-only changes retain their UI route.

Compose [engineering contracts](engineering-design.md#types-and-contracts-before-implementation), [object/state modeling](data-state-modeling.md#object-models-as-the-design-basis), [collaboration and fact supply](collaborative-modeling.md#trace-collaboration-and-data-supply) and [native projections](../domains/agent-facing-systems/contracts.md). Those owners retain general meaning; the methods below connect their decisions into a declarations-first consumer path.

| Question | Select the method | Useful result |
| --- | --- | --- |
| Which objects, responsibilities and promises must the boundary preserve? | [Model and native contracts](contract-modeling/model-and-contracts.md#derive-responsibilities-from-the-scenario) | Owned concepts, invariants, operation/results and enforcement obligations |
| How can existing native definitions supply schemas and clients without a second model? | [Source chain and representations](contract-modeling/model-and-contracts.md#keep-one-native-source-chain) | One editable source chain, supported inference/generation and explicit conversions |
| Can the complete caller path obtain inputs and continue after results? | [Caller flows and evidence](contract-modeling/caller-flows.md) | Source-linked supply and relevant success/refusal/partial/unknown branches |
| How should a frontend receive usable simulated supply through these contracts? | [Mock provisioning](mock-provisioning.md) | Default supply, owned behavior/host lifetime and actual consumer-gap evidence |
| What changes when the promise evolves? | [Consumer evolution](contract-modeling/model-and-contracts.md#evolve-the-promise-with-its-consumers) | Affected consumers, representations, in-flight facts and required updates |
| What distinguishes schema shape, runtime validation and an unknown effect? | [Fictional examples](contract-modeling/examples.md) | Small worked distinctions with explicit evidence limits |

Required context is the canonical scenario, native model/declaration locations, actual consumers, current versus proposed decisions and the task's execution boundary. Locate it through the consumer's own entries. Name missing context and continue independent reasoning; do not infer product behavior or authority from a prototype, schema, method or old task.

Keep confirmed meaning in the owning model and exact fields/methods in native declarations. Retain open supply, validation, adapter and implementation obligations in the existing contract/Task; the guide is not a second registry. Stop when the selected promise has a usable native declaration and consumer path at the assigned evidence boundary. Declaration, finite Mock, actual UI and real service results remain distinct.
