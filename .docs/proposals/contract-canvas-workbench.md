---
schema: proposal
status: proposed
description: Evolve Object Atlas into a typed contract and port canvas with navigable method/type dependencies.
deliveries: ["../../.controller/object-atlas-v2/delivery.delivery.md", "../../.controller/object-atlas-v2/layers.delivery.md", "../../.controller/contract-canvas-tool/delivery.delivery.md"]
models: ["../model/typescript-domain-sdk.md"]
---

# Typed contract canvas workbench

The completed Object Atlas proved that a curated relationship graph, inspector, search and receipt lens are useful. The user identified its structural gap: returned types such as MemberPage were only prose in a method detail and could not be navigated as objects. This follow-up treats every public non-literal contract as a first-class visual object.

The canonical projection separates facts from layout. Facts include domains, classes/interfaces/enums/value objects, fields, method ports, parameter/return/error types, state/outcome branches, relations and source evidence. Layout includes scene, groups, coordinates, routing and camera defaults. A renderer consumes this projection and exposes overview, type detail, method trace and scenario sequence views. The same object identity remains selected across views.

This is a model and visualization workbench, not a compiler, source extractor, runtime inspector or code editor. The first model slice is the validated TypeScript domain SDK and its management contracts, including Workspace.members to MemberPage, PageRequest<MemberCursor>, MemberPage.members to Agent and AgentSnapshot, MemberPage.coverage.next to MemberCursor, and Agent.prepareMessage to PreparedMessage to Operation.

The workbench answers: What can this object do? What does this method accept? What does it return? Which object owns that value? What errors or state branches exist? What calls this port and what does this port call? Which declaration and source evidence support the edge? A type/class node is incomplete without navigable fields and ports. A port is incomplete without typed inputs, outputs and explicit error/outcome edges.

The lower domain model owns semantics; upper visual scenes may compose objects into an Office or other presentation without changing lower identities. Domain classes and contracts are reusable; renderable scene instances and interaction affordances are an upper-layer projection.


## User-selected layer extension

The user requests first-class application/UI domain models built over lower SDK contracts and a focused view at a chosen abstraction layer. Layer ownership is independent of business-domain grouping and temporary Canvas visibility. Cross-layer inheritance, interface implementation, composition/wrapping and method delegation retain their own meanings and navigable endpoints. SDK internals are locally discoverable dependencies instead of mandatory scene clutter. Application examples without production declarations remain visibly proposed. The [Layer Delivery](../../.controller/object-atlas-v2/layers.delivery.md) owns the bounded implementation and current evidence; the existing graph is not proof of a production application object hierarchy.

## Tool and consumer separation

The user requests an independently reusable tool under `tools/`. The tool owns the portable model contract, generic checks, diagram/layout and inspection behavior. A project owns its declaration data, source correspondence and the presentation choices for its specific reading paths. The existing AgentX application/SDK model is one consumer; a second unrelated example establishes reuse without requiring an AgentX checkout. The [extraction and Web design Delivery](../../.controller/contract-canvas-tool/delivery.delivery.md) owns the selected implementation and actual evidence.

The same modeling method applies to product UI, with different runtime ownership: the domain SDK owns exact AgentX handles and authoritative operations, each product owns its scenario objects and local state, and a scene owns drawable instances, geometry and interaction intent. The Contract Canvas reads declarations about these layers. A live Space scene does not import Contract Canvas to obtain identity, state or effects.
