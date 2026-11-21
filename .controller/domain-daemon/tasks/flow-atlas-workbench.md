---
schema: task
status: completed
description: Reframe Flow Atlas as a domain-model design workbench with coherent contracts, processes, dataflow and scenario semantics.
links: [../flow-atlas-design.md, ../../../tools/flow-atlas/design/README.md, ./flow-atlas-workbench-semantics.md]
---

# Domain design workbench refinement

## User outcome

The user finds the current model coarse and requests a domain-model design workbench: objects, operations, parameters and rules composed with process/scenario dataflow. Deliver a coherent product and semantic design grounded in the existing tool, with a concrete complete example and implementable boundaries. Existing HTML demonstrations and authoring acceptance remain intact; this task does not claim a new editor/compiler exists.

## Model and ownership

Treat domain design as primary: domain meanings, identity/value semantics, state/invariants, relationships, ownership and public behaviors. Processes compose owned capabilities; scenarios bind instances, inputs and chosen outcomes; layouts and evidence are projections with separate authority. Identify missing control/data/state relationships, avoid duplicating schema or inventing arbitrary-source flow inference. DDD vocabulary must clarify actual consistency and responsibilities, not force every project into aggregates/event sourcing/workflow infrastructure.

Controller owns canonical design documents/contracts/examples, entry routing and bounded checks. A single modeling specialist owns only [semantic analysis](flow-atlas-workbench-semantics.md): isolated domain reasoning is justified because the prior active context centered on SVG/camera implementation. The specialist must not edit the controller's leaves or execute checks. No extra reviewer or implementation team is requested.

Actual specialist binding: `{serverId:srv_yDbm9zuqIMiM,agentId:8df02fe5-e121-4c64-85eb-89755bf6e8ee}`, workspace `wks_35140a9c49235329`; selected actual `default` profile materialized as `codex/gpt-6.1-sol`, `full-access`, `max`. It owns only its Task. Finish notification is the receiving mechanism; no polling.

## Work and completion

1. Derive gaps from existing design/authoring and decisive scenarios, including refusal and unknown external effects.
2. Establish the semantic relationships, authority, minimal typed contract refinements and an example whose data sources and state owners are explicit.
3. Define the workbench paths/views and staged usable outcomes, distinguishing current support from target.
4. Check affected design types/examples and link current docs. Preserve package/lock/models/generated HTML, user services and all product resources.

Unresolved questions: precise boundary between a domain and a bounded context; optional DDD semantics versus generic objects; parameter mapping without arbitrary expression evaluation; whether state rules are declared intent or executable source; coverage guarantees for source-bound processes. Resolve them in the canonical design, not in a second editable model.

## Working findings

The current TS target contains ObjectType, fields, ports and graph roles but lacks identity/value/aggregate semantics and process input bindings. The implemented Inspector is a deliberately smaller contract/module graph. `design/example.ts` additionally puts an unknown observation in order.status; the refined design instead separates domain status from caller knowledge. Native `prepareOrder` only validates/returns a request; the fixture has no complete order aggregate, order-confirm operation, durable transaction or unknown-result recovery. Those capabilities must not be inferred from its successful native test.

The canonical [workbench design](../../../tools/flow-atlas/design/workbench.md) now records the product purpose, domain model, three behavioral relationships, source authority, concrete success/rejection/lost-response cases, task-led views, new versus existing lint and first complete implementation slice. No target TS declaration is being rushed into a second speculative framework while semantic choices are still under discussion. Existing declarations remain the earlier skeleton; future native-contract changes are explicitly owned.

Verification is proportional to this design-only result: semantic walkthroughs, local context/link coverage and whitespace. No TS/runtime source changed, so unrelated passed tsc, lint and browser checks are not rerun. Existing executable acceptance remains historical with its exact evidence.

## Completed design and receiving

Specialist analysis completed and its claim was released. Integrated the decisive findings: Atlas reference IDs differ from business identity and call occurrence; value meaning is more than Schema shape; owned rules/state require an explicit authority; process arguments need result/branch-qualified sources; current ObservedStep lacks payload correspondence; actual stock can change while the caller remains unaware. No general epistemic engine is required for the first slice. Bounded contexts are meaningful optional boundaries, not another mandatory hierarchy.

The canonical example selects Order and StockPosition as separate owners, Checkout as use-case composition, and reserve data flowing from Order-owned preparation. Success/refusal/committed-effect-lost-response plus wrong receipt and wrong quantity are the deciding cases. Stock's derived available value is not stored as a conflicting third truth. Target intent/receipt fields and local commands are explicitly absent from the current native fixture; their contract/source implementation is an obligation rather than a fabricated reuse claim. The specialist's illustrative arithmetic remains its analysis; the product document owns the integrated example.

The [canonical workbench design](../../../tools/flow-atlas/design/workbench.md) now owns the complete proposed model, task-led UI, module responsibilities, authority and first usable slice. Existing design/tool entries, Proposal and Delivery route to it. Earlier TS design declarations deliberately remain the earlier skeleton: the refinement is a design decision for discussion and implementation, not a new callable DSL or runtime library. Native contract/compiler/case updates are the first implementation action and do not need an extra review/permission ritual.

No code/compiler/browser/package/generated HTML/server/product operation was performed. The one specialist owns no remaining write/resource claim. Completion means the requested design outcome, not implementation acceptance. Proportional local-link/whitespace and preservation results are recorded at `/tmp/flow-atlas-workbench-design/receipt.json`.
