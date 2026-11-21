---
schema: proposal
status: accepted
description: Evolve the finite runtime atlas into a reusable typed domain and process visualization tool.
models: [../../tools/flow-atlas/contracts.md, ../../tools/flow-atlas/design/README.md]
deliveries: [../../.controller/domain-daemon/flow-atlas-design.md, ../../.controller/domain-daemon/flow-atlas-implementation.md, ../../.controller/domain-daemon/flow-atlas-workbench-implementation.md]
---

# Flow Atlas as a domain and process design tool

The user selected general-purpose design on2026-11-10: use a TypeScript contract to describe domain objects, methods/ports, module relationships, processes, concrete scenarios, explanations and observable state changes; visualize them through one reusable tool. The demonstrated layout gap is overlapping/occluded call lines and no ability to reposition objects. Existing[canvas support](../../.controller/domain-daemon/tasks/runtime-atlas-ports.md#accepted-canvas-port-and-state-outcome) remains available.

The[design entry](../../tools/flow-atlas/design/README.md) owns the proposed product model and public declarations. This Proposal records the selected problem/direction, not acceptance of every new detailed choice or an implementation claim. The first Delivery covers usable design, source-grounded technology choices and typechecked contract/example sketches. Subsequent implementation must use that contract and preserve the current viewer until its replacement supports the existing consumer.

The selected scope is a local modeling library and visualization workbench, with portable data/artifacts. Domain methods stay opaque, process choices explicit and layout separate from facts. oRPC is a candidate adapter for existing service contracts and a future real remote boundary; it is not a substitute for lifecycle/process semantics. No distributed platform, business execution engine, automatic source inference, collaboration backend or universal BPMN implementation is requested by this design slice.

## Selected implementation follow-up

The user authorized continuation on2026-11-10 and selected direct TS contract reuse, module responsibilities, metadata/documents, process correspondence and design lint. [Source-binding design](../../tools/flow-atlas/design/source-binding.md) owns the authority and coverage model; [implementation Delivery](../../.controller/domain-daemon/flow-atlas-implementation.md) owns actual usable results. Acceptance of the direction does not establish every proposed compiler/SDK capability.

## Domain workbench refinement

The user finds the model too coarse and selects domain-model design as the product perspective. The [refined design](../../tools/flow-atlas/design/workbench.md) connects context, identity/value semantics, invariants and state ownership with process control/data dependencies and scenario expectations. It deepens the same product rather than starting another graph tool. The design Delivery's [follow-up Task](../../.controller/domain-daemon/tasks/flow-atlas-workbench.md) owns this bounded design result; actual implementation still requires canonical contract/compiler/view changes and outcome evidence.

The user explicitly authorized implementation on2026-11-11. The [domain-workbench implementation](../../.controller/domain-daemon/flow-atlas-workbench-implementation.md) now owns actual core/compiler/native examples, offline workspace and independent complete-path acceptance. Full target design and supported implementation remain distinguishable.
