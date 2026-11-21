# Dialogue trim set

These seven original SVGs are decorative finishes for the two selectable Space conversation treatments. They are self-contained, flat vectors with no external references, animation, text, input handling or product facts. The two edge masters are the `conversation-trim` variants.

| File | Intrinsic size | Intended use | Anchor and scale | Contrast |
| --- | ---: | --- | --- | --- |
| `folio-page-edge.svg` | 160 × 12 | Repeating paper seam at the Folio history/composer join | top-left, 1×, repeat-x | warm green/brown on warm stock |
| `folio-seal.svg` | 36 × 36 | Optional correspondence mark in a clear Folio header corner | center, 1× | rust ring with green linework |
| `folio-corner.svg` | 28 × 28 | Sparse sheet corner impression | top-left, 1× | low contrast green/rust |
| `folio-ruled-stock.svg` | 160 × 80 | Optional replacement for the regular paper hairline pattern | top-left, 1×, repeat in both axes | sparse, low contrast fibers; existing record rules supply the ruling |
| `signal-panel-edge.svg` | 160 × 12 | Single inset instrument seam at the Signal history/composer join | top-left, 1× | blue-grey frame with one teal slot |
| `signal-corner.svg` | 24 × 24 | Optional beveled panel corner | top-left, 1× | blue-grey brackets with two muted teal ticks |
| `signal-indicator.svg` | 18 × 18 | Optional slotted fastener beside a shell edge | center, 1× | muted teal slot with blue-grey housing |

The join trims are the stable first integration points. Fill the rail with Folio's repeat; show Signal once and retain the existing border across the full rail, including under the unadorned part. At narrow widths clip the empty end of a seam rather than stretching it. Keep the artwork within the available 12 px at the top of the rail and clear of the focus ring; omit if no free band exists. The seal, corners and fastener are optional details and should be omitted when they compete with identity, controls or message content. For Scene Canvas reuse only at an existing decorative facade, threshold or frame anchor with enough clear space, at a 1× starting scale; the current sample needs no extra overlays on the room or rendered actors.

The vectors are decorative. They do not encode unread, availability, attention, role, identity, message provenance or command outcome. The teal slot and fastener remain static material details. Live labels, focus, status and error semantics stay in the existing DOM. SVG roots declare `aria-hidden`, `focusable=false` and `pointer-events=none`. Bind as a CSS background where possible; decorative `<img>` elements still need `alt=""` and `aria-hidden="true"`.

`proof.html` is a local composition showing the trims on representative Folio and Signal reader edges. It is a static art review artifact, not an application screenshot or an interaction claim.
