---
schema: scenario
status: active
description: Deliver a complete agent task through a proportionate set of public surfaces
tags: [agents, contracts, delivery]
---

# Deliver an agent-facing capability

Use this workflow for a new public capability or a substantial integration. Reuse the current model and skip unchanged work for bounded maintenance. Record results in the project's existing design/task, using [templates](templates.md) only where no adequate form exists.

| Stage | Input and decision | Concrete result / next dependency |
| --- | --- | --- |
| 1. Frame the caller task | Who invokes what, from which runtime, with which identity/data, and what counts as completion? | One normal task, its important failure/recovery path and the minimum useful surface |
| 2. Model meaning and ownership | Identify owned objects, observations, lifecycle, invariants and external capabilities; distinguish reads from effectful actions | Source-linked model, public operation boundaries and known assumptions |
| 3. Declare the native contract | Define inputs/outputs, absence/defaults, stable failures, completion meaning and relevant effects | Native schemas/service signatures and owned projection metadata before adapter behavior |
| 4. Select and build projections | Check compatible provider versions, map transport grammar/envelopes, expose the skeleton and then bind domain behavior | Actual client/provider entry points, minimal adapter policy and repeatable derivation where needed |
| 5. Make the task discoverable | Write the selected caller recipe; add a short entry, Skill or MCP setup only for supported consumers | Exact task path reachable from the real discovery/distribution layout |
| 6. Verify and leave a maintainable result | Check changed declarations/projections and exercise the decisive consumer success/failure/effect path | Bounded evidence, explicit limits, change routes and cleanup/recovery instructions |

These stages are reasoning dependencies within a coherent capability batch, not separate Agent assignments or per-operation acceptance gates. Author the related native definitions, selected projections, bindings and discovery entries together under [whole-domain authoring](../../principles.md#whole-domain-authoring); use actual generator/provider probes only for consequential uncertainties, then concentrate consumer verification.

Before stage 3, resolve the consequential questions: object identity, ownership of effects, omitted versus clear inputs, completion versus acceptance, and behavior after interruption. A missing answer is a modeling question; generating more bindings does not resolve it. Apply [contracts](contracts.md) as declarations and adapters are created.

For a non-trivial new boundary, first materialize the intended module entry, signatures and composition points. Keep parsing, protocol handling and external library setup outside domain policy. Then implement within that structure. A working native declaration can serve as the skeleton; no parallel signature document is needed.

## Keep the caller path complete

Follow the task from the context an external agent actually receives:

1. Choose the capability from the published entry, Skill description or configured tool list.
2. Reach prerequisites and identify the target without guessing paths, credentials or names.
3. Invoke a concrete operation, interpret its machine result and determine the next step.
4. Observe the promised effect through the relevant boundary, rather than inferring it from a successful exit.
5. Recover from the meaningful failure or unknown outcome, then release only resources owned by the caller.

If a task crosses products, describe the handoff explicitly. For example, a manager can return an endpoint while a separate automation tool attaches to it. State who owns persistent state, who owns the attachment, how a refreshed endpoint is obtained and which close action stops which resource. Do not imply that two statically configured MCP servers exchange tool results automatically.

## Stop at the demonstrated boundary

When an input or prerequisite is unavailable, continue independent work and state the missing fact in the existing task. Do not invent support or add confirmation steps that existing user authority does not require. When actual integration exposes a mismatch, correct the owning model, contract or projection and rerun only affected evidence.

Use [maintenance and evidence](maintenance.md) to decide completion. A useful first delivery may support one caller well and leave other integrations explicitly unsupported. An unexecuted recipe or a generated descriptor must remain labelled with that evidence limit.
