---
schema: delivery
status: completed
description: Add explicit model layers and contextual cross-layer inspection to the typed Contract Canvas.
stages:
  model:
    description: Define layer ownership, typed cross-layer relationships and a truthful illustrative application domain.
    tasks: ["./tasks/layer-model.task.md"]
  canvas:
    description: Consume the layer contract in a focused, navigable Canvas with local dependency disclosure.
    tasks: ["./tasks/layer-canvas.task.md"]
---

# Layered modeling workbench

## User outcome

Read the objects, attributes and method contracts of one chosen abstraction layer without lower-level implementation clutter. Application/UI domain models are first-class models with their own semantics; they may inherit, compose or delegate to SDK objects. The reader can deliberately inspect those links and return to the same layer/context. The current [reference inspection](tasks/reference-preview.task.md) remains part of that path.

## Collaboration and boundaries

This extends the canonical graph and its consumer, so model semantics and interactive presentation are separate substantial professional responsibilities. Spawn one domain-contracts owner for layer/relationship contracts and source/proposal truth, and one prototyper owner for complete consumer integration. This split permits model design/source classification in parallel with the renderer's interaction/layout design; no child Controllers or separate review team. Kula owns their composition, resource boundaries and final bounded acceptance. Owners assess assumptions before implementation and communicate directly about the shared contract.

| Contribution | Write ownership | Consumer / completion |
| --- | --- | --- |
| Layer model · `rabeza` / `domain-contracts` | `model/` under `.docs/object-atlas-v2/`, a linked `layers.md` concept design, model Task | Canonical model/schema/wrapper consumed by the existing renderer; source-backed classifications and clearly labeled proposed application example |
| Layer Canvas · `nugaze` / `prototyper` | Parent renderer files under `.docs/object-atlas-v2/` (not `model/` or `layers.md`), browser evidence and Canvas Task | Working layer selection, current-layer structure, explicit cross-layer relationships/drill/return, preserved reference peek/pinning and offline/narrow support |
| Controller | This Delivery, links and integration decisions | Coherent complete path and evidence limits; no source/runtime SDK changes |

Canonical model IDs remain stable. Layer is independent of domain grouping and view filtering. Facts about inheritance, implementation, wrapping and method delegation must be distinguishable and supported; a source relation and an illustrative design relation cannot silently merge. Existing SDK `Agent` is a union, not an inheritable base class. The application example is a proposed domain model, not evidence that production Space implements those classes. Models/layouts/instances remain distinct.

## Resources and verification

Use the existing static preview at http://192.168.2.178:18988/ and existing local browser tooling; no new server, dependencies or runtime changes unless genuinely needed and coordinated. Preserve all unrelated source, live Agent services and other previews. Model owner checks schema/reference/source integrity once per coherent change batch; Canvas owner owns browser composition paths and screenshot reading once the integrated batch is ready. Controller consumes their evidence and investigates only substantive unresolved gaps.

## Accepted outcome

The Controller accepts the layered static workbench and the proposed Space conversation slice. Both direct assignments are complete; no integration or resource cleanup remains. The [model owner’s receipt](tasks/layer-model.task.md#delivered-follow-up) covers graph integrity, public type closure and strict TypeScript caller composition. The [Canvas owner’s receipt](tasks/layer-canvas.task.md#completed-consumer-and-evidence) covers the actual model binding and focused desktop, narrow touch emulation and direct-file paths against the final model; it records the renderer hashes and no page errors. The Controller reviewed the desktop application, desktop draft and narrow draft captures: application fields and ports lead, the draft and its revision are visible through the same reference and comparison controls, and the narrow walkthrough exposes all five checkpoints and the selected checkpoint’s contracts without requiring SDK drill-down.

Open [the existing preview](http://192.168.2.178:18988/) and select **应用模型 → 会话与草稿**. The five steps expose construction, exact actor selection, conversation opening, draft edit/reopen and separate advisory send availability. Public working ports use application concepts; wrapped receivers and state internals appear through disclosure. Genuine lower-layer references can be previewed, pinned or entered and returned from without losing the origin context. The preview reads declarations; no proposed app class, draft mutation, authorization or send was executed. Production Space, SDK and live services were outside the change.

## Application-domain composition acceptance

The user directed the team to assess whether an application can meaningfully express and construct its own model. The [Space design](../../.docs/object-atlas-v2/layers.md#proposed-space-session-exact-actor-conversation-draft) covers one complete slice: choose an exact office actor, open a conversation, edit a page-local draft, close/reopen without retargeting, and inspect send availability separately. The proposal remains distinct from production source and SDK support.

The source baseline already separates [`SpaceController` selection and revisioned drafts](../../web/agent-space/src/offline/controller.ts), [`Office`, `Post`, `AdoptedRole` and exact member facts](../../web/agent-space/src/offline/model.ts), [`HallActor` scene placement](../../web/agent-space/src/scene/model.ts) and [live read/send fences](../../web/agent-space/src/live/controller.ts). These are current responsibilities, not a production class hierarchy. Check that a scene actor is not a Role definition and that a closed/reopened conversation uses an exact Agent identity, not a display name or a sprite key. A stopped record may be read; selection or local draft methods grant no send capability. Check that the current-layer scene can expose the application contract and local effects before any SDK inspection, and that real lower-layer calls or missing supply remain explicit. A full production Space rewrite or live sends are outside this outcome.

The Canvas is a contract walkthrough: scenario steps navigate canonical declarations and mapping edges, without a simulated editor or live operation. The compile-only caller check supplies a matching bounded `ActorPage` and `ConversationSource` to `Office.openSession`, then checks the exact selection, read, draft and availability contracts. A private SDK receiver can belong to the application declaration layer while remaining an implementation detail; genuinely public SDK values retain their typed links. A later implementation would need an actual adapter and session methods, plus runtime checks for draft isolation, late results and current/stopped boundaries. Those are separate from this accepted design slice.
