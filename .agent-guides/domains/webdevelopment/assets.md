---
schema: scenario
description: Discover, specify, source and integrate real visual assets and product content
tags: [web, assets, visual-design]
---

# Assets and Content Resources

Use this guide when a scene needs images, illustrations, icons, typography, textures, video, animated artwork or representative content. Resource needs belong in design, before final layout and implementation decisions depend on missing material.

## Discover resource needs

Inspect the intended scenes and [component contracts](./component-design.md#the-complete-component-contract). Decide what each resource must explain, demonstrate or express. An asset can support product understanding, domain content, orientation or visual identity; it is not automatically decorative.

| Resource | Design considerations |
| --- | --- |
| Product imagery, screenshots or photography | Authenticity, subject, crop, focal point, legibility and placement |
| Illustration, 3D artwork or texture | Style family, composition, palette, negative space and background treatment |
| Icons and symbols | Meaning, family consistency, sizing, stroke/fill treatment and accessible labeling |
| Fonts and type assets | Brand voice, required characters, weights, fallback metrics, loading and permitted use |
| Video or animated artwork | Purpose, poster/static alternative, playback, controls and the [motion contract](./motion.md#motion-contract) |
| Copy, sample records or chart data | User terminology, realistic shape, authoritative versus illustrative status and localization |

An interface can deliberately use no custom artwork. Do not invent a resource backlog merely to populate every category. When a needed asset is absent, identify it explicitly rather than silently replacing it with an emoji, arbitrary gradient or unrelated stock image.

## Resource brief

Make requirements sufficient for the person, agent or tool preparing the resource. Record them with the existing consuming design/task; a separate manifest is useful only for a real asset set.

| Field | Useful content |
| --- | --- |
| Consumer and purpose | Page/Screen/Component, visible state and what the asset must communicate |
| Art direction | Subject, style family, references, palette, composition and exclusions |
| Placement | Container shape, aspect ratio, required size, focal point, crop and safe regions for adjacent text |
| Variants | Applicable themes, viewports, density, language and static/reduced-motion versions |
| Technical delivery | Format, transparency, source/editable file when needed, runtime compatibility and agreed size/loading constraints |
| Semantics | Informative or decorative role, text alternative/caption needs, text that should remain live UI |
| Provenance | Source or generation record, permitted use, attribution and modification constraints |
| Integration and acceptance | Destination, consuming contract, fallback, preview scene and who verifies the composed result |

Describe the result, not only a prompt such as “make a beautiful hero.” For example, an illustration behind a headline needs an agreed focal area and space for live text; whether it succeeds depends on the actual crop and theme.

## Source, adapt or create

Prefer suitable project-owned resources and an established icon/visual family. Use real product content where the design depends on product evidence. Investigate external sources or generation when an actual gap remains.

Verify current source terms, applicable license and attribution before adopting third-party material; “free,” a search result or an old catalogue entry is not sufficient evidence. Record useful provenance with the delivered resource. Do not present fabricated logos, testimonials, metrics or product screenshots as authentic.

Use the capability suited to the output: an existing SVG/icon system for consistent vector assets, code-native layout or effects where appropriate, and an available image-generation Skill for new raster artwork when it fits the task. Use the Skill's own guidance and actual tool availability. A mockup does not substitute for the required production asset.

When specialist collaboration is authorized, send the brief and consuming context through the [workflow handoff](./workflow.md#coordinate-by-contract-when-authorized). If producing resources inline, use the same contract. Artifact preparation and consumer integration remain distinct completion responsibilities.

## Integrate as product resources

Place assets with the appropriate owner using the project's existing import/public-resource conventions. Use meaningful stable references and avoid incidental hotlinks or temporary tool paths as production dependencies.

Place reusable visual families in the shared [design-system resource home](./design-system.md#design-system-artifacts-and-structure), and purpose-built local artwork with its consuming subject's design resources. Keep a resource family's brief, useful editable originals, provenance and exports discoverable together. Follow the [source-to-consumer path](./design-system.md#source-to-consumer-ownership) when build/public locations differ. Product/user content remains with its domain or content source; not every image is a design-system asset.

Render the real resource early enough to expose crop, contrast, density or layout problems. Provide suitable image sizing/cropping and reserve its intended space. Keep text that must adapt, localize or remain accessible in the UI unless the actual content requires another form.

Preserve useful originals when editing/exporting is part of the maintenance workflow; avoid checking in redundant variants without consumers. Optimize delivery for the actual display and runtime constraints, including fonts, large media and loading priority. Use [implementation guidance](./implementation.md#runtime-and-delivery-constraints) for the surrounding runtime.

Placeholders are acceptable in explicit prototypes or while a named dependency is pending. Mark their status in the owning task/design, keep their dimensions representative, and replace them before claiming completion when final assets are part of the requested outcome.

## Accept in context

Inspect the resource in its actual scene, important themes and narrow/wide layouts. Check subject relevance, crop, adjacent text, family consistency, sharpness, loading, accessible meaning and fallback. For animation, also inspect playback and reduced motion.

Record unmet requirements and the next owner in the existing artifact. Receiving an image file is not evidence that it fits the component; a component rendered with a placeholder is not evidence that its final asset is ready.
