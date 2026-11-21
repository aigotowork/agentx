---
schema: scenario
description: Build logos through geometry, anatomy, lettering, illustration and material craft with optical correction
tags: [logo, geometry, typography, craft]
---

# Construction and craft

Use after an organizing concept exists, or when a construction experiment is the current source of ideas. Select the method that serves the intended form. Circle systems, manual curves, drawing, collage, typography, sculpting and image-assisted studies are complementary tools.

## Establish the grammar

Identify the main silhouette, interior hierarchy, dominant angles or curve tension, stroke behavior and intended detail scale. Keep one editable source for each asset and preserve exploratory alternatives when they explain a real choice. Do not make export files the only editable definition.

Before refining, ask what recognition cue must survive: animal pose, letter skeleton, a particular opening, an interlock, a framing shape or material behavior. Without this, polishing may accidentally replace the concept.

## Geometry, grids and Boolean shapes

“Logo grid,” “geometric construction” and “keylines” describe ways to organize geometry. Shape Builder and Pathfinder are tool names for selecting, combining and subtracting regions. No single technique implies professional quality.

1. Choose actual constraints: angle families, construction axes, radius relationships, band widths or module spacing. Use as few as explain the form; add a new radius when the shape needs it.
2. Draw primitives or skeletons at named centers and dimensions. Make contact, overlap and tangent relationships intentional.
3. Use union, intersection, subtraction or exclusion to select the necessary regions. Define holes as real negative space, not background-colored patches when transparency is required.
4. Inspect the resulting contour and joins at the intended scale. Remove redundant nodes, accidental notches and crossings.
5. Apply optical corrections and retain the difference between geometric derivation and the final outline when relevant.

State operands and order explicitly: subtract the opening from the body, rather than merely naming a Difference command. A compound path can preserve multiple subpaths without computing a union; a clipping mask limits visibility without removing hidden geometry. Region coloring, such as Live Paint, is another operation. Choose the representation needed for editable work and the actual deliverable; these operations are not interchangeable.

Positional continuity (G0) makes curves meet. Tangent continuity (G1) aligns their tangent directions. Curvature continuity (G2) also matches curvature. A smooth organic outline may need G1 or G2; a deliberate corner should remain a corner. A circle touching another shape does not automatically provide the desired boundary after subtraction.

Control handles are real construction inputs, not presentation ornaments. A reflected Bézier handle or an SVG smooth command has specific preceding-segment conditions; tangent alignment alone does not ensure equal curvature. Keep anchors and handles available for editing instead of approximating every organic contour with many tiny line segments. The [technical source basis](../research/sources.md) distinguishes this path behavior from aesthetic judgment.

An overlay should come from the geometry actually used. If a grid is reconstructed after the design for analysis, label it as explanatory reconstruction. Never invent unused golden-ratio circles to imply rigor. The golden ratio is one possible proportion, not a requirement or a substitute for judgment.

Select a real making capability for those operations: explicit SVG/path construction, a vector editor's node/Boolean tools, a lettering tool or an appropriate dimensional source. Preserve the actual editable result and any meaningful parameters; a rendered board alone is not the construction source. Use the [making boundary](../tooling.md#making-and-auxiliary-construction) when designing tool support. Researching a tool-use Skill does not mean the required application or connector is available.

## Optical correction

| Situation | Check and possible correction |
| --- | --- |
| Equal bands look unequal | Compare perceived weight across angle and curve; adjust widths locally |
| Mathematical centering looks wrong | Compare visible mass and surrounding space; offset the placement |
| Round form looks shorter than a flat form | Evaluate an optical overshoot at aligned extremes |
| Acute junction looks heavy | Open the join, alter overlap or redistribute the internal space |
| Small hole closes during reduction | Enlarge it in a compact variant or simplify adjacent detail |
| Curve changes tension abruptly | Adjust neighboring handles or replace joined arcs with a better continuous curve |
| Wordmark spacing looks irregular | Judge letter pairs and word rhythm, not equal numerical gaps |

Make corrections for the actual application. Do not convert these examples into fixed percentages or assume one geometry works at all sizes.

## Figurative and animal marks

Research the actual subject. Build its skeletal gesture and major volumes before decorative detail. Choose which features identify the species or object, then decide how far to exaggerate or abstract them. A head, profile, full body and action pose are different concept choices.

For rounded geometric animals, relate ears, cheeks, muzzle, body masses and negative space through a coherent curve family. Preserve purposeful changes in curvature and asymmetry. Circles can help organize volumes; anatomy determines whether the result reads as an animal. Check likeness without the explanatory caption when review is available.

For characters, express personality through posture, gaze, proportion and interaction. Props should support the role rather than become a list of features. Separate the full illustration from any compact identity derived from it; do not assume the head alone preserves the character.

## Lettering and wordmarks

Design with the real name and required writing systems. Decide whether letters stay readable, become a monogram or intentionally form a more abstract sign. Explore skeleton, width, contrast, terminals and a small number of distinctive relationships. Check ambiguous letter pairs and unintended readings.

When starting from a font, verify its license for the intended desktop, web, embedding, modification and delivery uses as applicable. A font license and trademark clearance are separate matters. Preserve editable type where useful and create outlined exports when required; document the font and source version. Do not claim an SVG with unresolved system-font text is a portable outlined wordmark.

Relate the symbol and type through optical weight, cap/x-height, counter rhythm, spacing and personality. Avoid matching every radius mechanically when it hurts the word rhythm. Test horizontal, stacked or compact lockups only for needed contexts.

For a reusable lettering system or font deliverable, keep actual glyph outlines, bearings/advance and pair positioning in an appropriate font source. Match imported outline coordinates to its em, baseline and scale. A fixed-name wordmark may instead need only vector outlines and optical pair adjustments; do not build a font merely because a font editor is available.

## Engraving, linework and texture

Start from contour and tonal hierarchy. Let hatching describe planes, curvature or shadow; control direction, density and spacing. Use cross-hatching deliberately. In an engraved emblem, type, ornament and central subject need a clear order of attention.

For relief or metal engraving, distinguish three outputs: line artwork, an editable depth/mask or 3D source when needed, and a rendered surface image. A bevel filter is an approximation; it is not a machining file or evidence of physical manufacturability.

Woodcut, brush and handmade effects need coherent tool behavior. Retain meaningful irregularity and separate texture layers when future adaptation needs a clean source. Random distress applied over weak drawing is not craft. Check printing or fabrication limits against the intended process rather than an invented universal minimum line width.

## Materials and dimensional work

Choose the desired surface, light direction, roughness, edge profile, environment and background. Consistent highlights should agree with the form. The [style atlas](style-atlas.md) explains material families.

For a substantial expression study, form a compact art brief from the current proposition, source version, recognition cues to preserve, surface/lighting/detail choices, intended application and actual output requirements. State unwanted changes and how fidelity will be checked. Reuse the current plan and asset records; this does not require a separate campaign document. The art-direction mechanism in the [source ledger](../research/sources.md#method-and-skill-references) supplies a useful separation of intent, look, execution and variants, but assumes an established identity and cannot replace our concept exploration. Reference counts, advertising formats and unsupported revision-cost claims are not universal logo rules.

SVG gradients and masks can create controllable stylized volume. A 3D tool can provide coherent camera, normals and lighting for complex objects. An image model can explore rich material or illustrative expression. Select by fidelity, editability, time and the requested deliverable; do not force every material into a few vector gradients.

A curve-based dimensional source can retain its path while adding extrusion, bevel/profile and local radius controls. Account for the resulting outline and volume changes; a larger bevel or a centered tube is not automatically a faithful solid version of the original filled mark. Keep that geometry relationship separate from material, lighting and camera choices. The [Blender curve documentation](../research/sources.md) supports this making route, not a claim that a working scene or integration exists here.

For relief or engraving, distinguish outer boundaries, through-holes and surface marks. Bump/normal shading can suggest incisions without changing geometry; displacement, modeled grooves and Boolean cuts have different geometric consequences. Select by the expression or manufacturing requirement. Preserve useful curves, modifiers and material nodes in the dimensional source instead of keeping only a flattened render.

Keep the identity source connected to the rendering. If a model changes the shape while adding metal, it produced a different candidate; do not label it as the unchanged approved mark. Preserve source and derivation records, and distinguish art-directed texture studies from finished identity assets.

## Collage and source assembly

Record each constituent's origin, permitted use and transformation. Unify scale, viewpoint, edge language, palette and hierarchy where the concept calls for coherence; deliberate clash can also be a documented visual language. Masking several assets together is not itself a concept.

Stock icons and generated motifs can support exploration. For a distinctive commissioned identity, evaluate whether generic parts should be redrawn or replaced. Do not imply that adding shapes or converting a raster to SVG makes third-party content exclusive or clears rights.

## Source hygiene

Use explicit viewBox and intended bounds, stable IDs for definitions, controlled embedded resources, real alpha where needed and editable structure appropriate to the medium. Detect collisions when several SVGs share a page. Preserve the actual brand spelling and avoid external resource dependencies in a claimed standalone export.

Complex SVG constructs, filters, text and gradients may differ across renderers. Test the chosen export/runtime path through [evaluation](../evaluation.md); source validity alone is insufficient. Automated simplification or tracing must preserve the agreed cue and topology, not just reduce node count.
