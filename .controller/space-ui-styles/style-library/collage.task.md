---
schema: task
status: completed
owner: prototyper
links: [./design.md]
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"porazo","incarnation_bits":4745315996331600189}}]
---

# collage art pack

Editorial geometric paper collage, contrasting the soft options. Use punchy primaries, off-white/charcoal, cut edges, stepped drop shadows and deliberate irregular shape rhythm. Architecture and cast should feel built from folded paper without inheriting Newsprint outlines. Use shape and material contrasts across foreground and background.

Own a complete editable original art pack. Read [the common design](design.md), [asset placement contract](../../../web/agent-space/src/scene/studio-art.ts), and [the illustrated office precedent](../game-office/design.md). Work from visual vocabulary rather than previous asset outlines. Compose a complete office, not a gallery of independent icons. This is a theme variable for one running product. Own only `web/agent-space/src/scene/art/studio/worlds/collage/` and this Task. Keep all 23 exact Studio image names, intrinsic SVG width/height/viewBox and anchors. Include both `scene-vista.svg` (800×450) and `scene-lantern.svg` (64×62 bottom center). No external dependencies, scripts, fonts or embedded raster. Add a small README with palette, scene and cast decisions, and browser proof of the whole office. Validate all 25 images render, and tell Controller once they are ready for use. Do not modify the renderer, CSS, SDK or running previews.

## Completed art and evidence

The complete editable pack is ready under `web/agent-space/src/scene/art/studio/worlds/collage/`. It contains all 23 fixed Studio IDs plus `scene-vista.svg` (800 × 450), `scene-lantern.svg` (64 × 62), `README.md` and `proof.html`. The composition uses off-white stock, charcoal registration shadows, primary blue, signal red, warm yellow and teal. Hard stepped planes, offset paper shadows and compact geometric marks carry depth without Newsprint's outline and halftone vocabulary.

The proof composes a complete office and repeats the four actor masters at native 64 × 72 display size. A local Chrome capture at 1200 × 900 rendered the proof without errors. XML validation checked all 25 SVG roots, exact width/height and viewBox values, and rejected external hrefs, raster data, scripts, foreign objects and style blocks. The pack is ready for Controller integration; renderer, CSS, SDK and running previews were not modified.
