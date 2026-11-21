# Inkpost · 烛影邮局

Original editable SVGs for the [illustrated office](../../../../../../../../.controller/space-ui-styles/game-office/design.md). The [Studio placement contract](../../../../studio-art.ts) owns the stable image IDs, intrinsic dimensions and anchors.

The material direction is *Cinder Etching*: a night postal station carved out of soot, ink and old paper. Uneven cuts in the silhouette catch a narrow rim of light. The lamp side is warm; cool reflected ink on the far side keeps shadows from turning into vacant black.

Distance is a foggy river valley visible between dark architectural wings. Almost all of its sharp detail stays in the upper and outer thirds. The center belongs to the playable room; floor grain and wall finish fall behind the cast and the stamped state marks.

The midground has two large masses: close, warm work surfaces and a darker, softer lounge. Papers, bottles and rubbed drawer pulls group at use points. Short cracks follow wood grain, straight cuts follow metal, and small stitch marks follow the shapes of cloth. Broad value changes carry form before any of those fine details.

In the foreground, the low stone threshold leads the eye toward the brass signal lamp and the red, cat-guarded postbox. Against those horizontal edges the cast reads vertically: broad pale apron, swept rust collar, tall cap over a long cyan coat, and pointed saffron hood.

The bright colors are reserved for agency: warm cream on the working plane, a cool mint conversational cue, and vermilion for the attention flag. The selection shape remains a low bracket beneath the feet. Each cue can be distinguished by shape in a still frame.

Open [proof.html](proof.html) directly for the original SVGs in a static 800×450 composition, a 240×135 thumbnail, and a four person cast close-up at native 64×72. [proof.png](proof.png) records the browser render at 1:1. The proof uses a curated placement and synthetic light pools; the product retains the real scene and camera.

| Integration surface | Intent |
|---|---|
| Distance | `scene-vista.svg`, 800×450, draw before the room in camera space. The middle and lower river are quiet; dusk structures stay near the opening edges. |
| Lantern | `scene-lantern.svg`, 64×62, bottom center at (32,62). The red toggle remains legible next to the stem. Keep the bright glass for on; for off, visibly dim the sprite without moving its base and remove the external pool. The small baked aureole is confined to the sprite. |
| Room light | Courtyard `#211827`, shell `#3d2c36`, lit edge `#83604d`, shadow `#181420bb`. Use amber `#eeb46d` with a soft rim `#d88759` at low opacity, and a dim `#9696aa` wash at the far edge. |
| Action highlights | Focus `#ffdf9a`, retained selection `#f2be76`, speaking accent `#b5cfad`, attention `#d68e6d`. Preserve the selection bracket, bubble and flag silhouettes when changing colors. |
| Three motion readings | Working: bias the body down toward the packet or drawing implement, feet fixed. Moving: mirror in the direction of travel with a shallow step lift; keep the hands, satchel and hem legible. Resting: return to the neutral silhouette or a settled seated compression. Under reduced motion use the settled pose and the same static state marks. Art owns no clock or runtime state. |
