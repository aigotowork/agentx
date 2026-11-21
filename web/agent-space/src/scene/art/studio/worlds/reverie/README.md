# Reverie · 星绒梦室

An original twilight studio for the shared Space scene: pearlescent violet plaster, satin blue oak, peach nacre and small mint glints under a gentle crescent moon. The office is a celestial atelier rather than a purple wash: warm peach furniture establishes the working foreground, cool indigo supports depth, and pale moonlight is reserved for edges, faces and interaction marks.

The room reads in three depths at game scale. The 800×450 `scene-vista.svg` holds the quiet sky, cloud islands and two distant ribbon currents. `wall-window`, `wall-print` and `wall-plaster` establish the rear wall. Desks, archive furniture, sofa, rug and the cat-like human mailbox build the mid-ground. The four 64×72 cast silhouettes own the front contact line: The Maker has a broad cloud cap and raised tool, the Designer wears a crescent brim and trailing ribbon, the Keeper holds an open ledger under a wide velvet mantle, and the Scout pairs hare ears with a peach pennant scarf. Their outlines and held shapes remain distinct when rendered near 40–72 screen pixels.

The stable pack contains all 23 Studio IDs at the existing intrinsic dimensions and anchors, plus `scene-vista.svg` (800×450) and `scene-lantern.svg` (64×62, bottom-center foot). Every SVG is static, self-contained and editable: no external references, fonts, scripts or raster images. Motion, facing, state labels and lighting remain owned by the Space runtime.

Palette: deep ink `#292148`, twilight `#40376b`, periwinkle `#8b78b5`, pearl `#f7d8b3`, peach `#f2ad8e`, moon `#f7dda5`, and mint `#9ce1d0`. Intended room light is warm peach `#ffd2a9`; cool grade is `#242052`; action highlights are moon `#ffe5ae` and mint `#9ce1d0`. The lamp silhouette stays readable when dimmed.

Open [proof.html](proof.html) directly for a responsive whole-office board and a cast strip at 64×72. The proof places six couriers across work, lounge and archive areas using a static layout. From this directory, run `node verify.mjs` with Puppeteer and local Chrome (`CHROME_PATH` overrides the default) to validate the pack and generate local captures. The product's camera, motion, lighting switch and interactions are outside this proof.
