---
scope: Studio scene/list, conversation and command dialog
status: experimental
owner: Space UIUX
related: [./conversation.design.md, ./space.web.ts, ../../../../../.controller/space-scene-art/delivery.delivery.md]
---

# Two Space directions

The [full-world follow-up](../../../../../.controller/space-ui-styles/worlds/design.md) adds Lunar, Terrarium and Newsprint. The two-reader review below records the earlier independent preview and its evidence.

Compare the same action in both treatments: find @mora in the workshop, read the R-14 reply, write a draft, switch to the list, and return to the room. Keep actor art and domain inputs fixed. Compare dialogue framing, hierarchy, reading rhythm and density. The review asks which treatment feels at home in the workshop and which makes a repeated conversation task easier.

**工坊信笺 · Folio.** An illustrated letter and its binding: warm stock, deep green ink, rust highlights and a small envelope mark. On desktop the letter sits inside the office as a generously inset sheet, with the workshop still visible around it. A clear identity line, rules between records, and a writing well define the rhythm.

**夜班联络台 · Signal.** A night instrument panel: midnight blue, legible blue-grey text, restrained teal signals and a tabular record rhythm. A right-side dock leaves the scene borders in view. Read a denser sequence of messages, with the same identity and the writing area always in reach.

Both give the subject and status the first glance. Provenance, details and launch relationships are local disclosures. At narrow widths the reader occupies the viewport with the same single contextual return and offers the same style switch. Required or destructive command confirmation remains a native modal; its visual framing, fields and footer follow the selected style. Preserve exact target, focus, keyboard handling, scroll, drafts and native availability. An explicit `style=classic|folio|signal` link takes priority on entry; without it, use the browser's saved preference, then Classic as the fallback.

| Layer | Owner and boundary |
| --- | --- |
| Theme | `SpaceTheme` scopes Folio and Signal semantic skins while classic keeps the original CSS. The public registry, parser and controlled switch live in the Space UIUX entry; product preference/history remains app-owned. |
| Surfaces | `ObjectWindow`, `ConversationBlock` and `ConversationLayout` own window, identity, channels and reader slots. `SurfaceHeader` and `CommandSurface` own heading, modal focus and dialog part arrangement. No transport imports. |
| Product | Existing Space controller, adapters and public SDK own workspace reads, subjects, drafts, permissions and native commands. Theme never changes what a character or command can do. |
| Comparison | UIUX candidate canvas registers both readers and both dialogs over the real React pieces with deterministic data. A separate actual Space preview runs the public SDK against the same read-only Studio HTTP fixture for both styles. |

No new message protocol, renderer, design framework or generic component registry. A screenshot review compares the two candidates at the same size with open reader and command, then checks keyboard/touch return. A preference is still a product decision; browser checks establish operated behavior only.

The three Studio appearances share the same [original scene finishes](../../../src/scene/art/studio/finish/README.md): oak floor, window, rug, drafting desk and cat mailbox. The finishes keep their existing IDs, dimensions and anchors, so room geometry and hit targets stay fixed. Character art and motion are unchanged. The [dialogue SVG set](../../../src/scene/art/studio/refined/README.md) supplies Folio paper fibers, seam and command seal, plus Signal's panel seam and command corner. The extra corner impression and fastener remain available as assets.

## Preview and evidence

[Folio](http://192.168.2.178:18992/?art=studio&style=folio&office=%2Fstudio%2Fcatalog), [Signal](http://192.168.2.178:18992/?art=studio&style=signal&office=%2Fstudio%2Fcatalog) and [Classic](http://192.168.2.178:18992/?art=studio&style=classic&office=%2Fstudio%2Fcatalog) use the actual Space app and public object SDK against the same Studio read-only fixture. [Design Canvas](http://192.168.2.178:18993/#candidates) contains the [four registered candidates](../../candidates.json) with local-only data and effects. The [review sheet](http://192.168.2.178:18992/review/style-comparison.png) uses screenshots from both preview origins; [the workshop](http://192.168.2.178:18992/review/workshop.png) shows the scene finishes at product scale.

The [current Chrome result](../../../../../.artifacts/space-ui-styles/refined-02/report.json) covers three in-place appearances, exact reader and draft continuity across office/list without extra exact reads, saved preference and explicit URL precedence, the read-only new-Agent dialog, 390px switching and keyboard/touch return, and local confirmation on Design Canvas. The compared builds are `/tmp/agentx-space-styles-refined-r1` and `/tmp/agentx-space-uiux-refined-r1`. The [earlier comparison](../../../../../.artifacts/space-ui-styles/compare-02/report.json) and [default Studio regression](../../../../../.artifacts/space-ui-styles/default-regression/report.json) describe the previous builds. Preview evidence does not establish production rollout or native writes.

Build Space against the [Studio fixture protocol](../../../../../.controller/space-scene-art/verification/README.md) or run it with `?supply=studio&style=folio` when disconnected. Run `uiux check web/agent-space` and use the candidate tab of the UIUX workbench. The private package marker locates the Space-owned candidate catalogue; the repository root still owns dependencies. The [browser runner](../../../../../.controller/space-ui-styles/candidates.browser.mjs) accepts the two preview origins and a new output directory. [The review sheet generator](../../../../../.controller/space-ui-styles/export-board.py) accepts that directory and an output PNG.
