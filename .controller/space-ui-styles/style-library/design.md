# Space style library

Allow users to compare and try contrasting art directions without changing the Space controller, SDK, conversation, scene geometry, bindings, or clock. Maintain one visible catalogue for all currently available styles, including older UI-only treatments. Compare a single roster, lighting interaction and exact conversation at common sizes. Keep topical style descriptions in the review catalogue rather than in product surfaces.

| ID | Style | Visual and material target |
| --- | --- | --- |
| citrus | 柚风小筑 | Fresh seaside daylight, mineral white, pale lemon and breezy aqua; airy light and botanical silhouettes |
| reverie | 星绒梦室 | Dreamlike indigo/lavender/peach, nacre and soft halos; celestial fabrics, floating constellations and whimsical graphic silhouettes |
| wash | 雨后水彩 | Painterly watercolor on warm cold-press paper; translucent pigments, broken edges, backruns, paper showing through |
| collage | 彩纸事务所 | Color-block editorial paper collage: bold primary planes, cut edges, stepped shadows and halftone detail |

Use one style variable persisted in Space. Common registry names and groups each style, identifies scene art on/off and existing components read semantic theme variables. Canvas palettes consolidate in one scene recipe keyed by world, with the same 23 sprite roles and two scene illustrations. Complete new world packs decode before the in-place change. Workbench story fixtures share the existing composition. Design Canvas gets a four-world group. A separately hosted comparison table represents the actual registry and links to each selected style; screenshots will show the four whole rooms and their shared reader. Keep historical styles, ports and production services.

Each artist owns one coherent world they can advance in parallel without source conflicts. Controller owns common catalogue, palette, UI tokens, source and preview integration, screenshot review and acceptance. Independent evidence should exercise desktop, one narrow case, one actual lantern action, and in-place continuity through the four new appearances.

## Composition and verification

[Appearance registry](../../../web/agent-space/uiux/systems/space/components/presentation.ts) owns the 13 style identities and catalogue copy. [Canvas recipes](../../../web/agent-space/src/scene/studio-appearance.ts) and [UI tokens](../../../web/agent-space/uiux/systems/space/styles/library-worlds.css) own presentation. Each of [Citrus](citrus.task.md), [Reverie](reverie.task.md), [Wash](wash.task.md) and [Collage](collage.task.md) supplies the same 25 editable SVG roles. The plane, animation clock, input hit targets, message controller and domain SDK remain shared. With no legacy `art=studio` hint, `style=classic` selects the original pixel renderer. `view=scene|chat` is independent.

[Conversation contract](conversation.task.md) records the live diagnosis and isolated native browser run. The style catalogue is a read-only review fixture, and the Design Canvas uses local story data. Their scene controls and exact drafts exercise production components while native writes are verified separately.

Reproduce the catalogue using [browser capture](catalogue.browser.mjs), [image packing](pack-captures.py) and [catalogue export](catalogue.mjs). The capture command takes product URL, Design Canvas URL and a fresh evidence directory, then opens the 13 registry styles and all four new Candidate cards. Pack that directory to WebP. Pass a built product's `review/` directory, Design Canvas URL and packed WebP directory to the exporter. [Native check](native-check.py) takes a frozen AgentX candidate, a new evidence directory and an unused loopback port.

## Delivered preview

- [Style catalogue](http://192.168.2.178:19013/review/) presents 13 styles with actual office and reader captures. [Space](http://192.168.2.178:19013/?style=classic&office=%2Fstudio%2Fcatalog) opens the original pixel appearance; use the same switcher for the other styles.
- [Design Canvas](http://192.168.2.178:19014/#candidates) includes the four new interactive Candidate stories in 插画风格目录.
- [Final browser evidence](../../../.artifacts/space-ui-styles/style-library/captures-final/report.json) verifies all 13 styles, the Citrus chat list and scene reader, the keyboard lamp changing actual Canvas pixels, the 390px appearance/draft continuity and all four Design Canvas stories. The optional tools backend metadata is absent on these static listeners. No business requests originate from the Design Canvas stories.
- Isolated preview artifacts are `/tmp/agentx-space-style-library-product-r2` and `/tmp/agentx-space-style-library-design-r2`, owned by the user units `agentx-space-style-library-product.service` and `agentx-space-style-library-design.service`. Earlier preview listeners and the selected live :38023 service remain independent. Source typecheck, scoped conversation/route/SDK/Studio tests, and UIUX catalogue and build checks passed.

## Archived r5 review

This section records the 13-style r5 review checkpoint. The dedicated listener was retired after [formal appearance consolidation](../formal-adoption.task.md#adopted-project-release). Use the project `/space/` selector for the supported 14 choices.

The restored catalogue showed the retained six-member Studio captures and opened the corresponding style in the isolated four-member r5 workspace fixture. Its Studio entry exercised the bundled artwork through the legacy art hint. The original standalone two-room SVG [Lighthouse](../../space-svg-redesign/delivery.delivery.md) used local simulated conversations.

The [live asset check](../../space-conversation-workbench/delivery.md#2026-11-22-live-status) matches r5 in both formal services; the [registry](../../../web/agent-space/uiux/systems/space/components/presentation.ts) owns the 13 names: three classic or reader treatments and ten illustrated scenes. The [restored browser report](../../../.artifacts/space-conversation-workbench/post-rollout-audit/styles.json) verifies all 13 on the same mounted Canvas and exact conversation with a retained draft and no extra latest reads, checks Studio as an independent art hint, and switches rooms and opens a letter in the original SVG prototype. The older dedicated ports :18985, :18989, :18992–:18993, :18996–:18997, :19006–:19007 and :19013–:19014 were offline at the check. Design Canvas stories remain in [the candidate catalogue](../../../web/agent-space/uiux/candidates.json).

The restored build directory is `.artifacts/space-conversation-workbench/review-r5`; it copies the frozen candidate's `space/` and synthetic fixture and adds only `space/review/`. Regenerate the gallery with [catalogue.mjs](catalogue.mjs), the retained `style-library/packed-final` WebP captures, `--office /projects/workspace`, and the relative reference `./lighthouse/index.html`. The prototype files come directly from [the original preview](../../space-svg-redesign/preview/). When the transient unit is absent, run `node .controller/space-conversation-workbench/preview.mjs restore .artifacts/space-conversation-workbench/review-r5`. Restoration checks that the selected app assets match the accepted hashes before it starts.
