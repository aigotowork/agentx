# Jade Court · 流光庭院

An original fantasy mail conservatory at dusk, built over flooded terraces. Cropped iron ribs and split leaves sit in front of the sky and roof silhouettes. In the office, pale glints along glazed edges and warm paper bring the benches, route archive, postbox and switch lantern forward. The center of the vista is subdued so it can sit behind the room at different camera proportions.

Open [the standalone proof](proof.html) directly in Chrome. It uses each master in a representative cutaway and shows the four actors at 64 × 72 CSS pixels. The placements and light pools are fixed for art review.

| Asset group | Drawing and placement |
| --- | --- |
| Scene frame | `scene-vista`, 800 × 450, behind the camera's playable area. `scene-lantern`, 64 × 62, foot at (32, 62). |
| Architecture | `floor-oak` repeats on both axes, `wall-plaster` repeats horizontally with a translucent apron below y=48, and `wall-window`, `wall-print`, `door-threshold` and `rug` dress those planes. |
| Work and lounge | `desk`, `stool`, `coffee`, `sofa`, `bookshelf`, `cabinet`, `plant` and `human-mailbox` all place by bottom center; `envelope` places by its center. |
| Cast | Rivet (`actor-maker`), a seal smith with a wide cap and folding iron; Lin (`actor-designer`), a narrow glasswright with a sharp hat and lens; Sable (`actor-keeper`), a folio keeper with coiled hair and a hanging key; and Kestrel (`actor-scout`), a scarfed runner with a tilted profile and dispatch bag. Each places by (32, 72). |
| State cues | `glyph-work` is an engraving tool, `glyph-available` an open voice bubble, and `glyph-attention` a notched pennant. `glyph-select` is a mint ring with three points that sits underneath the actor's feet. |

The [Studio placement table](../../../../studio-art.ts) owns the 23 stable IDs, exact sizes and anchors. The masters use original, editable SVG paths and gradients. All local fills and details travel with each SVG; there are no linked images, fonts, event handlers or timelines.

Use upper-left window bounce **#81F6DC** at low strength and blue-violet shade **#121628**. For the switch lantern, use a warm floor pool **#FBC182** and flame highlight **#FFF7C3**; **#F7CF9E** suits the small action glints. Keep the floor pool localized so the actor badges and persistent mint selection ring retain their silhouettes. When dimmed, keep the lamp's bronze and paper silhouette, lower its brightness and remove the emitted pool.

| Sampled motion | Drawing intent |
| --- | --- |
| Idle / talking | Stable foot contact with a small rise or sway; cap, lens, folio and scarf remain readable in a still pose. |
| Seated / working | Compress around the foot contact. Preserve face and hand tools above the desk line; the engraving badge carries the meaning in a still frame. |
| Walking / carrying | Mirror the whole costume with the supplied facing and use a small rise; the runner's scarf and diagonal satchel read in either direction. A still frame retains the directional silhouette and any separately drawn dispatch. |

The renderer owns the sampled clock, reduced-motion choice, light toggle, input and authoritative status. The fixed proof does not run any of those behaviors.
