# Illustrated game office

The previous three worlds proved that one live Space canvas can change cast and skin without replacing its controller. They share very flat room compositions, nearly static vector bodies, and sparse interaction feedback. This follow-up is a single playable office slice across three distinct art directions. Use the same user task and reference sizes, and compare the actual rendering.

| ID | Label | Illustration and material language |
| --- | --- | --- |
| inkpost | 烛影邮局 | Hand inked illustrated dark fable: rough cross-hatching, tapered contours, crooked wood and amber pools; distinct paper-carved couriers |
| windward | 风屿驿站 | Painterly adventure illustration: daylight, drifting cloud islands, weathered turquoise and ochre, woven cloth and observatory gear |
| jadecourt | 流光庭院 | Color-rich fantasy illustration: dusk, cyan porcelain and deep red metals, glasswork and layered botanical forms; jewel lit outlines |

Reference games only for visual vocabulary. Artwork, costumes, architecture and icons are original. Each art owner delivers a complete pack and room proof. The existing Studio IDs, dimensions, and feet stay fixed; two supplemental decorative images `scene-vista.svg` (800×450, scenic background, leave quiet center), `scene-lantern.svg` (64×62, foot at bottom center; tall readable switch/set piece) compose below/above the same render projection. No bundled script or external assets.

Interaction: framed backdrop, layered floor/light/contact shadows, breathing or working gestures and familiar motion in the room; use actual sampled pose and reduced motion preference. Touch, keyboard or pointer activate a local lantern and see the room lighting and switch state change. Lamp action is a page-local visual preference per room; no simulated Agent state, message writes or capabilities. Interact with an exact Agent to open the established reader, keep drafts across skins and scene/list, and identify hover/focus/selection with distinct cues. Product design remains visual and compact.

Owners: parallel world artists with independent production paths; Controller owns same renderer, presentation registry/CSS, local state, isolated preview, Design Canvas and screenshot comparison. Browser checks cover all three renders, real animated frames, pause/reduced motion, direct activation, and exact draft continuity. Prior previews and live hosts retain their builds.

## Delivered preview

Three complete art packs, 25 original SVGs per set: [Inkpost](../../../web/agent-space/src/scene/art/studio/worlds/inkpost/README.md), [Windward](windward.task.md#completed-art-and-evidence) and [Jade Court](../../../web/agent-space/src/scene/art/studio/worlds/jadecourt/README.md). Each artist recorded a full room and cast proof in the linked pack. The playable Space composes the same stable room geometry and actors with scenery depth, directional light, projected contact shadows, timed material accents, and foot-anchored pose motion. The lamp is exposed as a real keyboard/touch target and a visible room prop in the scene inspector. It changes local room lighting only; its state is kept in page-owned scene memory. Existing role, message and SDK contracts are unchanged.

[Inkpost](http://192.168.2.178:19006/?art=studio&style=inkpost&office=%2Fstudio%2Fcatalog) · [Windward](http://192.168.2.178:19006/?art=studio&style=windward&office=%2Fstudio%2Fcatalog) · [Jade Court](http://192.168.2.178:19006/?art=studio&style=jadecourt&office=%2Fstudio%2Fcatalog) · [Design Canvas](http://192.168.2.178:19007/#candidates).

[Actual browser comparison](http://192.168.2.178:19006/review/game-office-comparison.webp) and [Inkpost motion capture](http://192.168.2.178:19006/review/inkpost-motion.webp) were captured from the running r2 Space preview using [the capture script](review.browser.mjs) and [sheet generator](review-board.py). Final product and design paths: `/tmp/agentx-game-office-r2` and `/tmp/agentx-game-office-design-r2`. The product uses the public SDK against the read-only Studio HTTP fixture; Design Canvas uses local data and effects. Previous preview and live listeners were preserved.

The [final independent Chromium report](../../../.artifacts/space-ui-styles/game-office-zilima-r2/report.json) passes all six cases with exact r2 artifact/fixture replay; the Controller separately operated the running product, captured r2 and confirmed both listeners serve the selected previews. The report covers lamp pixels in paused frames, actual motion and reduced-motion stillness, same camera/identity/draft through appearance changes, the 390px flow, and real Design Canvas room instances. Typecheck, selected runtime/Studio/debug tests and UIUX catalogue check pass. The current six-member sample reuses Studio room/camera/navigation and does not establish a new animation rig or a generalized game editor. [Verification findings](verification.findings.md) preserve the remaining narrow status-filter and canvas-workbench observations.
