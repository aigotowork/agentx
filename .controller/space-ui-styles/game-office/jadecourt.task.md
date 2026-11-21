---
schema: task
status: completed
owner: prototyper
links: [./design.md]
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"nafizo","incarnation_bits":4745315982210112235}}]
---

# jadecourt game illustration delivery

Jade Court, an original dusk fantasy mail conservatory. Reference polished fantasy concept art: sculpted architecture, jewel colors, lantern metalwork, glass, leaves and luminous silhouettes, with coherent reflected light.

Own the entire game-quality illustration pack; read [the integration model](design.md) and [Studio dimensions](../../../web/agent-space/src/scene/studio-art.ts). The user's core complaint was that previous rooms and sprites look flat and generic. Plan a scenic composition first, with foreground/middle/distance, an intentional lighting direction, large readable silhouettes and layers of small material details. Palette alone is insufficient. Build original SVGs for all 23 stable Studio IDs at their exact intrinsic sizes and footpoints, plus scene-vista.svg (800×450 camera background with quiet center) and scene-lantern.svg (64×62 foot bottom center). Use dense crafted detail on focal props and four characters at 64×72; favor atmosphere, consistency and visual hierarchy over blanket ornament. No copied characters, game textures, logos, fonts or external references. Create a thumbnail review proof using the actual assets and a full cast close-up at working scale. Consider three motion states visually, but no SVG-owned clocks. Record intended colors for room light and action highlights. Notify the Controller when the complete exact-dimension pack is available. Own only `web/agent-space/src/scene/art/studio/worlds/jadecourt/` and this Task. Application changes, preview, and the existing Studio sets are outside the assignment.

## Art direction

**Glazed Dusk.** Treat the conservatory as a cutaway above a flooded terrace: silhouette foliage and bronze ribs at the frame, layered violet roofs and still water in the distance, and an uncluttered middle for the office. Inside, a jade stone grid carries cool reflected light from the upper left; amber belongs to the switch lantern, the paper under its glow, and a few bevels that face it.

The studio is built from pierced bronze, celadon fired in pools of dark glaze, and red lacquer worn back to the metal on edges. The frame, mail slot and stained glass share a split-petal form; repeat it wherever the craft or function justifies it. Primary props get sculpted profiles and readable negative spaces. The tile, rug and wall keep their pattern contrast below the figures. A full-window ornamental treatment was considered and set aside because framed detail should recede when the room and cast are over it.

Give each worker a trade silhouette and a single personal treasure: Rivet's wide smith's apron and folding iron, Lin's tall glass cap and drafting lens, Sable's layered folio coat and key, and Kestrel's trailing scarf and dispatch satchel. The frozen proof will check those figures at 64 × 72 and as a group over the room. Studio owns all pose timing and runtime interaction.

## Delivered art and evidence

The [Jade Court pack](../../../web/agent-space/src/scene/art/studio/worlds/jadecourt/README.md) contains all 23 original Studio masters plus the vista and lantern. The [fixed art proof](../../../web/agent-space/src/scene/art/studio/worlds/jadecourt/proof.html) places every master. Review the [room plate](../../../web/agent-space/src/scene/art/studio/worlds/jadecourt/room-proof.png), [366 px thumbnail](../../../web/agent-space/src/scene/art/studio/worlds/jadecourt/thumbnail.png) and [native cast plate](../../../web/agent-space/src/scene/art/studio/worlds/jadecourt/cast-proof.png).

The dimensions and viewBoxes match Studio and both ambience sizes. XML inspection found 25 self-contained, static SVGs with unique local resource IDs. Chrome decoded and painted them all into Canvas, and rendered the standalone proof at 842 px and 390 px with a 64 × 72 cast at both widths. The layered framing reads at thumbnail size, the lantern remains the brightest floor object, and the four trade silhouettes and hand props are identifiable in the native lineup. Small metal and cloth marks recede in the thumbnail as intended.

Integration note: the runtime currently shares one halo hue for daylight and lantern light. The standalone art proof demonstrates the intended separate cool window tint and warm switch tint; the pack README records both. The controller owns the actual runtime appearance, sampled motion, and interaction evidence.
