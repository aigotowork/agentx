---
schema: task
status: completed
owner: prototyper
links: [./design.md, ../../../web/agent-space/src/scene/art/studio/AGENTS.md]
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"varika","incarnation_bits":4745315969496611440}}]
---

# Space newsprint world art

Own a coherent, original, design-ready world pack for the newsprint direction in [the common model](design.md). The user found the existing people and scene too ordinary and wants much bolder visual alternatives. Start with a design composition, then deliver editable SVG masters used directly by the running Space canvas and portrait UI. Plan material, silhouette, depth and lighting together. Include an atmospheric room, editorial-quality prop detail, and four distinct character silhouettes readable at actual 64 × 72 display size. Keep strong contrast at face and foot. Give each character unique shape and equipment or costume. Use no existing game assets. Existing Studio is a geometry and naming contract only.

Write exclusively under `web/agent-space/src/scene/art/studio/worlds/newsprint/` plus this Task. Use the same intrinsic dimensions/viewBoxes and anchors for the art IDs in [Studio placements](../../../web/agent-space/src/scene/studio-art.ts). At minimum author all four actors, floor-oak, wall-plaster, wall-window, wall-print, door-threshold, rug, desk, stool, coffee, sofa, bookshelf, cabinet, plant, human-mailbox, envelope and the four glyphs. No dependencies between image files, external fonts, scripting, or text. Shared glyph shapes may be redrawn in your visual language and preserve state meaning (work, available, attention, selected). Write a compact README with palette, composition decisions, and screenshot or runnable standalone proof. Please publish core SVGs early so integration can proceed, then finish the whole pack. Do not edit CSS/React/renderer, build the app, or modify any preview service. Update this Task and let the controller know when your whole pack is ready.

The selected composition is a night shift at a two-ink comics printer. Cobalt night panes, a pinned proof and a bright paper floor anchor the available scene geometry. The characters read first by silhouette: a wide maker with a brayer, a sharp designer holding a proof, a tall keeper with a folio, and a scarfed scout with a courier bag. The core floor, wall, window, desk and four actor masters are available in the final directory while the remaining art and render proof are authored.

The [complete pack and palette](../../../web/agent-space/src/scene/art/studio/worlds/newsprint/README.md) are ready for integration. XML inspection matched all 23 IDs in Studio placements and verified dimensions, viewBoxes and self-contained markup. The pack also contains an optional tea table. Chromium rendered [the standalone composition](../../../web/agent-space/src/scene/art/studio/worlds/newsprint/proof.html); [the capture](../../../web/agent-space/src/scene/art/studio/worlds/newsprint/proof.png) includes the complete room and all four portraits at 64 × 72 CSS pixels. The pale floor and larger rear dot screen separate the backdrop from the cast. The foreground trim was moved below furniture contacts and opened at the threshold after the first visual review.

The proof uses fixed art placements. Space loading, atomic Canvas swaps, portrait consumption and selected-agent continuity remain part of the world integration.
