---
schema: document
description: Connect canonical scenarios, domain meaning and UI behavior through traceable bindings, independent inventories and bounded evidence.
---

# Interaction modeling

Use when a user goal, domain objects and operations, and actual UI paths must be connected to answer whether a task can be completed, with the right identity, scope, state and recovery. Workbenches, editors, asynchronous operations and complex forms often need this connection. A small visual change can reuse its existing slice and the [Web methods](../webdevelopment/AGENTS.md).

This domain owns the connecting references, coverage judgments and interpretation of interaction checks. [Product requirements](../product-design/requirements.md) own promises and scenarios; [data/state modeling](../../scenarios/data-state-modeling.md) owns domain meaning; native declarations own contracts; the UI owns components and local behavior. Do not create a second requirement catalog, operation registry, schema or business state store.

| Question | Read and produce |
| --- | --- |
| Can this goal be reached through the actual interface and correct calls? | [Three-axis method](method.md): a complete scenario–domain–UI slice |
| What does an object mean, and what do its relationships establish? | [Object cards and relationship graphs](method.md#object-cards-and-relationship-graphs): source-linked meaning, provenance, methods and conditional relationships |
| What original material supports this claim? | [Source reading](sources-and-coverage.md#read-the-source-behind-a-claim): located sources, readable context and freshness/access limits |
| Are scenarios and operations covered beyond the selected model? | [Independent inventories](sources-and-coverage.md#build-three-independent-inventories), then the case matrix and dynamic discovery limits |
| Are scope, asynchronous state, failure and return behavior sound? | [Checks and evidence](checks-and-evidence.md): properties, finite bounds, counterexamples and receiving owners |
| How do uncertain actors/needs become versioned flow and report handoffs? | [Product experience](../product-experience/AGENTS.md): progressive refinement and native-run projections; this domain retains binding and coverage ownership |
| How should a finding improve the product? | [Apply the inventory](sources-and-coverage.md#apply-the-inventory-to-product-work): distinguish tool, model and product corrections and verify the actual consumer |
| How can a prototype teach a workflow and recover progress? | [Guided demos](guided-demos.md): observed completion, stable targets and separate tutorial/domain recovery |
| Does the interface support understanding, precision and continued work? | [Tool-experience review](../product-design/scenarios/tool-experience-review.md): task, cognition, scope and continuity |

Start with one canonical criterion, its observable result and decisive failure/unknown branch. Use `scenario → action → control → call or local effect → result → next state`, retaining reverse navigation to original sources. Mark current facts, chosen targets, assumptions and executed observations separately. For completeness claims, obtain each axis's denominator independently before joining; a selected model cannot measure its own omissions.

The method works with tables and prose. The [worked slice](examples/scoped-update.md) demonstrates that route without an executor. If a checker, extractor, viewer or tutorial runtime is selected, use its installed README/help and public declarations for version, supported input, dependencies and commands. This guidance supplies none of those executors. Unsupported properties remain manual or uncovered; a conditional trace does not execute business operations.

Compose [contract modeling](../../scenarios/contract-modeling.md) when native caller/input/result semantics are unresolved, [Mock provisioning](../../scenarios/mock-provisioning.md) when simulated facts are needed, and [modeling adaptation](../modeling/adaptation.md) for finite executable projections. Before UI design or review, read [interface requirements](../../scenarios/interface-design.md). Read only the routes needed for the question and reuse context already loaded.

Keep outputs in the consumer's existing model, review or Task, linked to the original authorities. A finite check establishes only its declared domain; UI replay establishes only the visited environment and boundary; expert walkthrough and tutorial execution do not establish user research or production support. Stop when the selected question has a usable answer and remaining gaps have reasons, owners and next actions.
