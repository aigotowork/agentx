---
schema: task
status: completed
owner: prototyper
links: [./design.md]
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"dirivo","incarnation_bits":4745315996148441233}}]
---

# wash art pack

Authentic watercolor-on-paper character. Simulate visible translucent color overlaps, uneven pigment pooling, dry brush and cold-press paper with self-contained SVG geometry and only inexpensive selective filters, and preserve legibility of the cast at 64×72. Think rain clearing over an artist studio, muted ultramarine, sage and ochre. Soft surroundings, distinct focal details.

Own a complete editable original art pack. Read [the common design](design.md), [asset placement contract](../../../web/agent-space/src/scene/studio-art.ts), and [the illustrated office precedent](../game-office/design.md). Work from visual vocabulary rather than previous asset outlines. Compose a complete office, not a gallery of independent icons. This is a theme variable for one running product. Own only `web/agent-space/src/scene/art/studio/worlds/wash/` and this Task. Keep all 23 exact Studio image names, intrinsic SVG width/height/viewBox and anchors. Include both `scene-vista.svg` (800×450) and `scene-lantern.svg` (64×62 bottom center). No external dependencies, scripts, fonts or embedded raster. Add a small README with palette, scene and cast decisions, and browser proof of the whole office. Validate all 25 images render, and tell Controller once they are ready for use. Do not modify the renderer, CSS, SDK or running previews.

## Completed pack and rendered evidence

The original [Wash pack](../../../web/agent-space/src/scene/art/studio/worlds/wash/README.md) contains all 23 exact Studio IDs at the declared intrinsic sizes plus the 800×450 rainy-terrain vista and 64×62 foot-anchored lantern. The visual language is a quiet studio after rain: ultramarine window light and slate distance, ochre work planes, sage lounge forms, warm paper tooth and selective pooled pigment. The Maker, Designer, Keeper and Scout have independent headwear, face, clothing and object silhouettes; work, available, attention and selection retain the separate Studio glyph meanings. SVGs remain editable and self-contained without external references, fonts, or active content.

The fixed [browser proof](../../../web/agent-space/src/scene/art/studio/worlds/wash/proof.html) composes six illustrative participants with workbenches, archive, lounge, threshold, postbox, lantern, and a native-size cast row. I asked whether the materials remain visible without obscuring the room's work/social/entry grouping or the faces and status glyphs, and whether the cast stays recognizable at its declared portrait size. Chrome 147 rendered all 25 contract images, including repeated floor/wall tiling, with no external requests, page errors or horizontal overflow at 848×670 and 390×500; all four cast portraits remain 64×72 CSS px. Inspection found the softened floor grain and edge pooling visible while the central vista stays quiet, worktables read as a separate group from the sage settle, and each cast role retains a readable prop and facial silhouette. This is static art evidence only; it does not exercise the live Canvas, camera, light toggle, motion or conversation.

Captures: [desktop](../../../web/agent-space/src/scene/art/studio/worlds/wash/proof-desktop.png) and [narrow](../../../web/agent-space/src/scene/art/studio/worlds/wash/proof-narrow.png). The isolated rendering check was run with local Google Chrome and `puppeteer-core`; it validated the contract dimensions, all 25 SVG parses/decode/raster draws, local-only references, and proof image placement. No application/runtime files or previews were changed.
