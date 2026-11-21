---
schema: task
status: completed
owner: prototyper
links: [./design.md, ../../../web/agent-space/src/scene/art/studio/AGENTS.md]
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"zaluta","incarnation_bits":4745315969148280466}}]
---

# Space lunar world art

## Completed outcome and evidence

Authored the 23 original SVG masters in [the Lunar world pack](../../../web/agent-space/src/scene/art/studio/worlds/lunar/README.md), with a [standalone room and cast proof](../../../web/agent-space/src/scene/art/studio/worlds/lunar/proof.html). The cobalt pressure station has a lunar horizon in its observation windows, a striped orange airlock, telemetry consoles, a lounge and a courier cat. The Maker uses a heavy clamp and paired boots, the Designer a dish and long legs, the Keeper a wide scanner and treads, and the Scout a round optical dome and sprung feet. The four face and contact treatments remain legible at native 64 × 72. The four glyphs retain their Studio meanings.

Sent the core art paths early to @kula (delivery 59655). Final validation matched all files to the Studio IDs, intrinsic dimensions and viewBoxes, checked self-contained XML, and decoded and painted every master into a browser Canvas. Chrome rendered the complete proof at 1024 × 780 and 390 × 740, loading all 23 assets at both widths and displaying the individual cast at native size. The first narrow proof clipped the composition into a horizontal viewport and its keyboard scroll failed; the final proof fits the entire composition and keeps full-size portraits in a separate strip. Captures are at `/tmp/agentx-lunar-art-review/{desktop,narrow}.png`. This evidence covers asset and proof rendering; the controller owns application and live-service evidence.

## Original assignment

Own a coherent, original, design-ready world pack for the lunar direction in [the common model](design.md). The user found the existing people and scene too ordinary and wants much bolder visual alternatives. Start with a design composition, then deliver editable SVG masters used directly by the running Space canvas and portrait UI. Plan material, silhouette, depth and lighting together. Include an atmospheric room, editorial-quality prop detail, and four distinct character silhouettes readable at actual 64 × 72 display size. Keep strong contrast at face and foot. Give each character unique shape and equipment or costume. Use no existing game assets. Existing Studio is a geometry and naming contract only.

Write exclusively under `web/agent-space/src/scene/art/studio/worlds/lunar/` plus this Task. Use the same intrinsic dimensions/viewBoxes and anchors for the art IDs in [Studio placements](../../../web/agent-space/src/scene/studio-art.ts). At minimum author all four actors, floor-oak, wall-plaster, wall-window, wall-print, door-threshold, rug, desk, stool, coffee, sofa, bookshelf, cabinet, plant, human-mailbox, envelope and the four glyphs. No dependencies between image files, external fonts, scripting, or text. Shared glyph shapes may be redrawn in your visual language and preserve state meaning (work, available, attention, selected). Write a compact README with palette, composition decisions, and screenshot or runnable standalone proof. Please publish core SVGs early so integration can proceed, then finish the whole pack. Do not edit CSS/React/renderer, build the app, or modify any preview service. Update this Task and let the controller know when your whole pack is ready.
