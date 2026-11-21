---
schema: task
status: completed
owner: prototyper
links: [./design.md]
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"kuzile","incarnation_bits":4745315996922544106}}]
---

# citrus art pack

Fresh seaside spring daylight. Mineral white, citrus yellow and breezy aqua; whitewashed arches, linen canopies, trailing herbs and subtle green shadows. Make the room feel luminous and airy, not like a nursery. Four fresh silhouettes by apparel, tools, and facial expression.

Own a complete editable original art pack. Read [the common design](design.md), [asset placement contract](../../../web/agent-space/src/scene/studio-art.ts), and [the illustrated office precedent](../game-office/design.md). Work from visual vocabulary rather than previous asset outlines. Compose a complete office, not a gallery of independent icons. This is a theme variable for one running product. Own only `web/agent-space/src/scene/art/studio/worlds/citrus/` and this Task. Keep all 23 exact Studio image names, intrinsic SVG width/height/viewBox and anchors. Include both `scene-vista.svg` (800×450) and `scene-lantern.svg` (64×62 bottom center). No external dependencies, scripts, fonts or embedded raster. Add a small README with palette, scene and cast decisions, and browser proof of the whole office. Validate all 25 images render, and tell Controller once they are ready for use. Do not modify the renderer, CSS, SDK or running previews.

## Completed art and evidence

The [Citrus pack](../../../web/agent-space/src/scene/art/studio/worlds/citrus/README.md) is ready for Controller integration. The original editable drawings share limewashed arches, linen, pale oak, sea-glass glaze and leaf green contact shadows. The maker, designer, keeper and scout have distinct garments, tools and expressions at the same foot anchor. The [whole-office proof](../../../web/agent-space/src/scene/art/studio/worlds/citrus/proof.html) composes all 25 files and repeats the four characters at native size. The final [Chrome capture](../../../web/agent-space/src/scene/art/studio/worlds/citrus/proof.png) shows an 800 × 450 room with clear foot contacts and quieter floor grain after the visual review.

Chrome 147.0.7727.101 decoded and drew every image to Canvas with nonempty pixels, matching the exact filenames and dimensions from [Studio](../../../web/agent-space/src/scene/studio-art.ts) and [ambience](../../../web/agent-space/src/scene/ambience.ts). The SVG parse check confirmed width, height, viewBox and self-contained static vectors. The proof loaded without browser or resource errors at a 920px viewport and fit a 390px viewport without horizontal overflow. An ephemeral loopback server resolved file URL restrictions on Canvas pixel readback. The older Studio asset guide lists `tea-table` as optional; the runtime table does not consume it, so this pack follows the 23 runtime IDs. Live Canvas, switching and lamp behavior remain with the Controller. Renderer, product CSS, SDK and running previews were left unchanged.
