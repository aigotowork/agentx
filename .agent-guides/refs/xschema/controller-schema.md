---
schema: role
role: controller
description: Planning and proposal structures for authors, plus work and coordination structures for controllers.
---

<!-- Generated from the declared xschema sources; see sources.json. Do not edit this copy. -->


# Controller structures

Use the shared Task structure to create Tasks. Add only the fields needed for the current operation; `...` is a value placeholder.

## Roadmap

Planning authors can use this structure without taking a Controller role. Keep capability value, entry conditions and priorities in the body; create only real items.

```yaml
schema: roadmap
description: ...
status: active # draft | active | paused | retired
items:
  revisit:
    description: ...
    status: selected # optional: candidate | selected | deferred | dropped
    horizon: now # optional text
    proposals: [./proposals/revisit.md] # optional
```

At least one item is required. Item keys are local field addresses (`part.field: [items, revisit]`), not Markdown anchors. Missing status is undeclared, and selected does not mean implemented.

## Proposal

Design authors use a Proposal for a selected change; coordination authority comes from the assignment, not this structure.

```yaml
schema: proposal
description: ...
status: accepted # draft | proposed | accepted | rejected | withdrawn | superseded
models: [../model.md#retained-results] # optional existing model/section
# Optional selected batches, relative to this Proposal:
deliveries: [../../.controller/cli/cli.delivery.md]
```

Keep current-to-target changes, retained guarantees, decisions, open parts and batch coverage in the body. Use Common links for background. Models may use real heading fragments; Delivery and Proposal paths cannot. A new domain can initially omit models until its design exists. An accepted Proposal or completed listed Deliveries never implies that all its targets are implemented. Update the current model when the owned behavior is established; retain the original decision basis here.

Forward ownership is Roadmap item → Proposal → Delivery → stage/Task; incoming relations are derived. Aggregation grants no Controller management or execution authority. Unknown status words remain author data. Query the declared relations with their original evidence and scope; an empty incoming result does not prove global absence.

## Decompose a Task — optional additions

```yaml
tasks: [./tasks/a.task.md, ./tasks/b.task.md]
dependencies: [{before: ./tasks/a.task.md, after: ./tasks/b.task.md, condition: ...}]
```

Dependency endpoints must be descendant Tasks; no cycles or parent/controller back-references.

## Delivery

```yaml
schema: delivery
description: ...
status: planned
stages:
  a:
    description: ...
    tasks: [./tasks/a.task.md]
  b:
    description: ...
    status: planned # optional
    tasks: [./tasks/b.task.md]
    depends_on: [{stage: a, condition: ...}]
```

Stage tasks/dependencies are optional. Stage keys are local; no cycles or multi-stage Task ownership. Keep detailed stage content in this Delivery's body.

## Controller

```yaml
schema: controller
scope: ../ # for a controller in <module>/.controller/; . means the project root
agent: ... # optional
# Optional direct relationships:
deliveries: [./plans/example.delivery.md]
tasks: [./tasks/a.task.md]
controllers: [../child/.controller/controller.md]
```

Each managed object has at most one direct Controller; no management cycles.

## Task Meta — adjacent tasks/.a.task-meta.md

When useful before dispatch, record the Agent's anticipated scenarios, guidance and skill needs, with reasons and gaps. Preserve this analysis as source evidence for later Agent Guides and skills; identifying a gap does not require fixing it now.

```yaml
schema: task-meta
task: ./a.task.md
state: draft # draft | reviewed | promoted | retired
# Optional groups:
scenarios: [{name: ..., description: ..., source: candidate, reason: ..., gap: ..., confidence: medium}]
roles: [{name: ..., description: ..., source: inferred, fit: primary}]
skills: [{name: ..., description: ..., source: map, ref: ../guides/skill.md, need: required}]
tools: [{name: ..., description: ..., source: inferred, need: useful}]
```

`map` requires `ref`; `candidate` requires `reason`. Optional values: confidence high/medium/low, fit primary/alternate, need required/useful. No Task status, actual agent or dependency fields.
