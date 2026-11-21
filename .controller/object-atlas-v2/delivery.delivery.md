---
schema: delivery
status: completed
description: Replace the relationship-only Object Atlas with a navigable typed contract and method-port Canvas workbench.
stages:
  model:
    description: Define the canonical contract graph and first source-grounded type slice.
    tasks: ["./tasks/model.task.md"]
  canvas:
    description: Render the contract graph as a polished multi-scene SVG/HTML workbench and verify navigation.
    tasks: ["./tasks/canvas.task.md"]
---

# AgentX contract Canvas

This delivery upgrades the completed relationship Atlas into a type, contract, class and method-port workbench. Native Rust, generated OpenAPI, public SDK and domain model remain authoritative. The portable visual artifact is a curated projection and makes support boundaries explicit.

## Runtime assignment

The model owner releases a stable source-grounded contract graph before the Canvas owner binds the full renderer. Independent validation remains Controller-owned for this design delivery; no live operation or production adoption is involved.


## Model and consumer composition

Model owner `labu` completed the [contract graph](../../.docs/object-atlas-v2/model/README.md): 139 nodes (119 types and 20 ports), 279 field anchors, 600 typed edges, 121 source anchors and four scenarios. Nine source-backed directed navigation routes pass structural checks. The model contains no presentation coordinates; its deterministic classic-script wrapper supports offline consumers.

Canvas owner `nuvi` bound that model in the [portable workbench](../../.docs/object-atlas-v2/README.md), served at http://192.168.2.178:18988/. Four shared-identity scenes cover overview, type detail, method ports and send/recovery. The [browser report](evidence/browser-report.json) passes desktop, narrow, keyboard, source disclosure, receipt branches, offline opening and origin-preserving return. Controller screenshot reading of MemberPage, Workspace.members and overview confirmed that the missing returned contract is now an independent navigable object, with structural fields and typed method edges rather than prose-only references. The author completed routing and focus corrections, retaining affected evidence without repeating the passed full walkthrough.


## Accepted scoped outcome

The Controller accepts the complete static contract Canvas and model projection for the selected source-reading scope. [Final artifact identity](evidence/artifact-manifest.json), [geometry checks](evidence/geometry-report.json) and [focus checks](evidence/focus-report.json) supplement the author browser report. Geometry checks cover the four desktop scenes plus narrow method/type/recovery, without unrelated-card intersections or document overflow. Focus evidence covers MemberPage entry and keyboard field-reference navigation. These are finite authored checks consumed by the Controller; no independent user research or runtime-effect claim is made.

The original gap is closed: a returned public type, its internal fields, generics, nullable cursor and input/output/error contracts are separately selectable and linked. The reusable canonical schema/facts remain separate from visual scenes, routing and camera. The [workbench handoff](../../.docs/object-atlas-v2/README.md) owns usage and maintenance; [the model boundary](../../.docs/object-atlas-v2/model/README.md) owns graph refresh and integrity commands. Preview remains at http://192.168.2.178:18988/; keep the directory intact for offline use. Existing :18987 and all live AgentX services remain unchanged. Broader source extraction, editing/code generation and live observation remain outside this delivery.


## Local reference inspection follow-up

The user identified disruptive scene changes when reading external references. The bounded [reference-preview Task](tasks/reference-preview.task.md) owns the subsequent local preview/comparison enhancement and affected evidence. The original accepted model and broad scene evidence remain historical; consult the follow-up for current reference activation semantics.
