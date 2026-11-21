# Catalog Studio art

[World art packs](../../../../../../.controller/space-ui-styles/worlds/design.md#authored-resources-and-preview) provide complete alternative drawings for the same IDs, dimensions and anchors. The loader validates the resolved set before changing any Canvas images. These packs are also the source of contact portraits.

Original editable vectors commissioned for the [Space scene art delivery](../../../../../../.controller/space-scene-art/delivery.delivery.md). Start with the [art proof](../../../../../../.controller/space-scene-art/art-evidence/proof.html) for composition, then use the geometry and placements from the actual scene. These masters do not contain native facts, interaction, collision or a room plan. No existing sprite, map, image-generation output, template or third-party art was used.

Each filename stem is a stable visual ID. Every SVG embeds its assets and has intrinsic width/height identical to its viewBox extent. Import individual files as Vite URLs; renderer owns loading, rasterization, and scaling. Draw dimensions below are in world units at 1×. Coordinates/anchors are normalized from top left once the drawing is scaled. The same actor retains its costume while its pose/state changes.

[Scene finishes](finish/README.md) replace selected visual IDs at the same geometry through [the Studio asset loader](../../studio-assets.ts). [Dialogue finishes](refined/README.md) are a separate decorative SVG set used by the Space design system and reviewed in its own proof.

| Layer | SVG IDs | Size, anchor and use |
| --- | --- | --- |
| Ground | `floor-oak` | 128 × 128, top left; seamless two-axis tile clipped to the actual room. Do not stretch the grain. |
| Rear facade | `wall-plaster` | 160 × 64, top left; horizontal repeat, clip to actual rear wall. Facade occupies the upper 48; lower apron is translucent. |
| Rear decoration | `wall-window`, `wall-print` | 120 × 48 and 54 × 48, top left. Optional, single placements within free rear facade width; omit on conflict. |
| Foreground entry | `door-threshold` | 64 × 32, bottom center on actual doorway. The object is flat and can sit under walking feet. Canvas owns the edges and door opening. |
| Social ground | `rug` | 192 × 112, top left; optional low contrast motif in a free lounge area. Skip or scale down if the zone is narrow. |
| Work/social/archive | `desk`, `coffee`, `sofa`, `bookshelf`, `cabinet`, `tea-table`, `plant` | 86×68, 54×62, 108×60, 56×64, 68×64, 70×48, 32×46 respectively. All bottom-center (0.5, 1). Normal desk visual foot is ~10 world units above actor foot; actual renderer and movement projection decide ordering and offset. Plant and tea table are optional; place only on clear ground. |
| Optional supporting props | `stool`, `envelope` | 38×36, bottom center; 24×16, center for the moving letter. One vector per object, no motion/placement assumptions. |
| Human endpoint cue | `human-mailbox` | 62 × 62, bottom center. Envelope on a post with the house cat as a single form; Canvas/DOM own count and action. |
| Cast | `actor-maker`, `actor-designer`, `actor-keeper`, `actor-scout` | 64 × 72, foot point (0.5, 1). Distinct caps/hair, body outlines and costume marks; stable presentation choices, never authority or status. Mirror only for actual motion/facing; a standalone front stance remains legible. |
| State badges | `glyph-work`, `glyph-available`, `glyph-attention` | 24 × 24, center. Etching tool = working, open conversational bubble = approachable, notched flag with exclamation = attention. Pair symbolic shape with authoritative DOM name/status. Attention covers the relevant supplied signal; it does not mean unread unless the UI can establish that fact. |
| Ground selection | `glyph-select` | 64 × 24, center on actor foot. Use beneath the actor for focus/selection; local state only. |

The back wall and ground are a consistent architectural shell. Furniture and characters belong to the same contact-depth order; shadow and rear texture should never compete with faces or actionable feedback. Work reads through station and focused stance; approachability through a clear stance away from desks and the open bubble; attention through the flag outline. A still review and reduced-motion mode keep these same cues. Runtime motion owns transition/facing; do not infer a service state from a costume, location, animation or color alone. The SVG files contain no animation or external references.
