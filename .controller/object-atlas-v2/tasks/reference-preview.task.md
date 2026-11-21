---
schema: task
status: completed
description: Inspect external contracts locally and pin related objects without losing the current Canvas context.
---

## Outcome and interaction model

The user needs to read referenced types and methods alongside the current contract instead of repeatedly navigating away. This is a bounded renderer enhancement, implemented inline; canonical model, SDK and runtime services remain unchanged. Follow the [interface requirements](../../../.agent-guides/scenarios/interface-design.md) and [Canvas ownership](../../../.docs/object-atlas-v2/AGENTS.md).

Use a local interactive reference preview for hover, focus and touch/click. Show typed structure and explicit actions for pinning or entering the Canvas. Preserve selection, history, camera and inspector position during preview/pinning. Keep up to three independently removable comparison cards in the existing right rail, with full fields/method contracts and nested reference previews. Directory and major Canvas cards remain intentional navigation. On narrow screens the comparison shelf uses the existing details drawer. Escape/outside dismissal and keyboard return must preserve context.

## Design decisions and uncertainty

An interactive popover is appropriate for reference inspection; a plain tooltip cannot contain accessible contract navigation and pin actions. Reuse the existing fixed rail rather than adding columns that resize the diagram whenever a contract is pinned. A bounded shelf prevents unlimited panels from crowding the workspace; reaching capacity must never silently evict a pinned contract. Read all properties locally in pinned cards and disclose source evidence on demand. Preview shows a short structural slice and an explicit remaining-field count.

Verify the complete edit batch with a focused desktop/narrow/offline browser walkthrough and inspect representative screenshots. Existing broad browser evidence remains historical for its original interaction semantics; do not rerun unrelated scenes or claim runtime support.


## Completed outcome and evidence

The portable [Canvas](../../../.docs/object-atlas-v2/README.md) now supports interactive reference previews from fields, inspector links, related type cards and method ports, plus three independently collapsible pinned contracts in the fixed right rail. Previewed method contracts expose owner, inputs, return and errors/outcomes. Additional preview fields/methods expand locally. Nested references have a local back action. Explicit Canvas entry remains available and preserves pinned comparisons. Reset clears the shelf. Main selection, history, camera and inspector scroll stay unchanged during peeking/pinning.

[Focused browser report](../evidence/reference-preview-report.json) and [reproduction](../evidence/reference-preview-check.cjs) pass at 1440×900 and 390×844, including hover/focus, keyboard return, nested preview/back, duplicate suppression, explicit capacity, removal, explicit navigation, method-return preview, narrow drawer return and direct offline method/source inspection; no page errors were observed. [Layout follow-up](../evidence/reference-layout-report.json) and [reproduction](../evidence/reference-layout-check.cjs) establish visible identities/actions for all three cards, bounded independently scrolling bodies and stationary neighboring titles at both viewports. Later edits only tighten comparison spacing/provenance and are covered by this layout follow-up.

Screenshot inspection found that one shared scrolling shelf hid earlier comparison identities. Cards now keep their own title/actions visible and scroll their own content. Dense provenance avoids repeating the already-visible target title. Captures: [desktop comparison](../evidence/reference-desktop.png), [desktop preview](../evidence/reference-desktop-peek.png), [narrow preview](../evidence/reference-narrow-peek.png) and [narrow comparison](../evidence/reference-narrow-comparison.png). Three open cards trade visible body area for stable simultaneous identities; each can be collapsed independently to give the others more room. This is finite author-operated static UI evidence, not human research or runtime verification. No canonical contract facts or live services were changed.
