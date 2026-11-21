---
schema: document
description: Maintain connected product design views, current implementation claims and planned changes.
---

# Design management

Use this method when a user needs to understand or steer an evolving product, when architecture/commands/scenarios change, or when fragmented documentation obscures current versus intended behavior. The method owns the connected reading and maintenance loop; existing declarations, designs, scenarios and Tasks retain their facts.

Read [workflow](workflow.md) for the current action and [views](views.md) for the relationships being represented. [Overview](README.md) explains the user outcome. Compose [product design](../../scenarios/product-design.md) for uncertain needs, [collaborative modeling](../../scenarios/collaborative-modeling.md) for semantic derivation, and [CLI design](../../scenarios/command-line-design.md) for command contracts. Do not restart those methods for an unchanged bounded edit.

Express simple content directly. For new models, complex relationships, interacting states or multiple dimensions, apply [visual model communication](../../scenarios/interactive-design-communication.md): compose a polished panoramic HTML view from professional visual forms and keep it connected to the affected model. Update both during discussion; the view does not replace native facts or current/target distinctions.

For a feature evolving across changes, use [its current design](workflow.md#maintain-a-features-current-design), [affected updates](workflow.md#determine-the-affected-updates) and, when useful, the [portable worked example](feature-example.md). Requirements own feature/scenario meaning; this method owns their reading and maintenance relationships.

- Maintain one discoverable model entry per meaningful product/module scope. Reuse its actual home; a new entry defaults to `.docs/model/README.md` under [placement guidance](../../documentation.md#artifact-placement-rules). Connect scenario, concept, architecture, interface and evidence views with stable references where needed.
- Treat the model as a distributed design: its entry explains scope, language and relationships and leads to owned topics, module designs, native contracts, protocols and technology decisions. Keep those sources with their owners; a model directory need not contain them all. Register real entry paths through the owner's [artifact catalog](../../documentation.md#module-artifacts).
- Keep current behavior, accepted target and unaccepted alternatives explicit. A proposed design is not an implementation fact.
- Record decision state separately from implementation, integration, verification and distribution evidence. Apply this to relevant relationships as well as nodes.
- Use [feature planning](../../scenarios/feature-planning.md#planning-levels) for Roadmap → Proposal → Delivery → Task and their states. Link the selected Proposal's decisions and affected model topics; keep execution state in its own record and return established meaning to this model.
- Update affected views with a design/behavior change; retain unresolved drift if current source has not been checked. Do not redraw unaffected views.
- Use readable domain documentation for meaning and native declarations for exact contracts that implementation can consume. Link those sources; do not establish a second editable API, command catalog, scenario promise or Task status.
- Scale to the module. A small library may need only an overview, contract relationships and one scenario. No mandatory diagram count, universal schema migration or release gate follows from this guide.
- Tools derive disposable views from the original materials; do not require authors to register the same facts again for a renderer. A renderer or structural check does not certify semantic correctness or actual runtime behavior.

For authoring changes to this method, use the enclosing harness maintenance and guidance-authoring routes. Consumer facts and evidence stay with their product; installed shared guidance is updated through its distribution owner.

