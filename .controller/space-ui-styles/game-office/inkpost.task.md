---
schema: task
status: completed
owner: prototyper
links: [./design.md]
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"zurimo","incarnation_bits":4745315981305534874}}]
---

# inkpost game illustration delivery

Inkpost, an original hand-inked dark fable post office. Reference hand-inked gothic storybook game language: irregular contours, hatching, skewed silhouettes, aged paper, amber lighting. Make it feel like an illustrated world at dusk.

Own the entire game-quality illustration pack; read [the integration model](design.md) and [Studio dimensions](../../../web/agent-space/src/scene/studio-art.ts). The user's core complaint was that previous rooms and sprites look flat and generic. Plan a scenic composition first, with foreground/middle/distance, an intentional lighting direction, large readable silhouettes and layers of small material details. Palette alone is insufficient. Build original SVGs for all 23 stable Studio IDs at their exact intrinsic sizes and footpoints, plus scene-vista.svg (800×450 camera background with quiet center) and scene-lantern.svg (64×62 foot bottom center). Use dense crafted detail on focal props and four characters at 64×72; favor atmosphere, consistency and visual hierarchy over blanket ornament. No copied characters, game textures, logos, fonts or external references. Create a thumbnail review proof using the actual assets and a full cast close-up at working scale. Consider three motion states visually, but no SVG-owned clocks. Record intended colors for room light and action highlights. Notify the Controller when the complete exact-dimension pack is available. Own only `web/agent-space/src/scene/art/studio/worlds/inkpost/` and this Task. Application changes, preview, and the existing Studio sets are outside the assignment.

## Composition decision

Use a dark cutaway silhouette around a low-contrast river-valley vista. Keep illuminated paper and brass concentrated around the left work stations, dull the red lounge weave, and use near-level stone and contact shadows to seat the foreground props. The cast uses distinct height, headwear and cloth masses before facial or costume marks. An evenly lit parchment room was considered; it would leave fewer useful value differences between action targets and their surroundings. The static proof will inspect the assembled room and each actor at native size, with the runtime composition and motion left to the Controller.

## Completed art delivery

[Inkpost art pack and integration notes](../../../web/agent-space/src/scene/art/studio/worlds/inkpost/README.md) contains 23 original Studio SVGs and both supplements, [the dusk vista](../../../web/agent-space/src/scene/art/studio/worlds/inkpost/scene-vista.svg) and [the brass lantern](../../../web/agent-space/src/scene/art/studio/worlds/inkpost/scene-lantern.svg). Static XML checks matched the exact 23 IDs and dimensions from `STUDIO_IMAGE`, plus both supplemental dimensions; every viewBox covers its full intrinsic size, references are local and the files have no scripts or animation. The actors and grounded props use the declared bottom center anchors; the lantern focuses on the stem at x32 and grounds at y62. The vista is camera space, not a world obstacle.

[Browser art plate](../../../web/agent-space/src/scene/art/studio/worlds/inkpost/proof.html) and [recorded proof](../../../web/agent-space/src/scene/art/studio/worlds/inkpost/proof.png) show the composed 800×450 frame at 1×, a 240×135 thumbnail, and all four native 64×72 sprites on the same ground. Chrome decoded and drew all 25 files, confirmed the displayed dimensions and fit at 390px, and the visual review confirmed the four hat and coat silhouettes separate at native size. The tighter moon and roof silhouettes are visible above the room after the single composition correction. Work surfaces stay warmer than the lounge; the green bubble, stamped flag and low brass selection land on separate shapes. This is a static art fixture; the Controller owns executable preview and motion/lighting evidence.
