---
schema: task
status: completed
owner: prototyper
links: [./design.md, ../../../web/agent-space/src/scene/art/studio/AGENTS.md, ../../../web/agent-space/src/scene/art/studio/worlds/terrarium/README.md]
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"ludiza","incarnation_bits":4745315969317740348}}]
---

# Space terrarium world art

Own a coherent, original, design-ready world pack for the terrarium direction in [the common model](design.md). The user found the existing people and scene too ordinary and wants much bolder visual alternatives. Start with a design composition, then deliver editable SVG masters used directly by the running Space canvas and portrait UI. Plan material, silhouette, depth and lighting together. Include an atmospheric room, editorial-quality prop detail, and four distinct character silhouettes readable at actual 64 × 72 display size. Keep strong contrast at face and foot. Give each character unique shape and equipment or costume. Use no existing game assets. Existing Studio is a geometry and naming contract only.

Write exclusively under `web/agent-space/src/scene/art/studio/worlds/terrarium/` plus this Task. Use the same intrinsic dimensions/viewBoxes and anchors for the art IDs in [Studio placements](../../../web/agent-space/src/scene/studio-art.ts). At minimum author all four actors, floor-oak, wall-plaster, wall-window, wall-print, door-threshold, rug, desk, stool, coffee, sofa, bookshelf, cabinet, plant, human-mailbox, envelope and the four glyphs. No dependencies between image files, external fonts, scripting, or text. Shared glyph shapes may be redrawn in your visual language and preserve state meaning (work, available, attention, selected). Write a compact README with palette, composition decisions, and screenshot or runnable standalone proof. Please publish core SVGs early so integration can proceed, then finish the whole pack. Do not edit CSS/React/renderer, build the app, or modify any preview service. Update this Task and let the controller know when your whole pack is ready.

The [composition](../../../web/agent-space/src/scene/art/studio/worlds/terrarium/README.md) sets warm light against a dark greenhouse canopy and assigns each botanical character an independent silhouette and tool. The review question is whether the greenhouse reads with the unmodified Studio placements and each cast member stays recognizable with the room, status badges and selection underneath.

Delivered the [editable masters and palette](../../../web/agent-space/src/scene/art/studio/worlds/terrarium/README.md), [standalone composition](../../../web/agent-space/src/scene/art/studio/worlds/terrarium/proof.html) and [desktop capture](../../../web/agent-space/src/scene/art/studio/worlds/terrarium/proof.png). The first nine masters were shared early so integration could begin. The complete world covers all 23 Studio IDs. The checker matched each filename, intrinsic size and viewBox to `STUDIO_IMAGE` and resolved the internal SVG references.

Chrome decoded all 23 images in the standalone proof at 1040 × 800 and 390 × 650. The arched glass facade and warm floor establish the greenhouse. The radish, orchid, acorn and ginkgo silhouettes and their face and foot accents remain distinct in the 64 × 72 cast lineup. The selected ring reads beneath the orchid, and the status badges stay clear of faces. Fine prop details recede when the complete room scales to 390 px. The composition uses illustrative room coordinates and includes a daylight wash that the proof draws separately. Canvas and actual application portraits are the controller's integration evidence.
