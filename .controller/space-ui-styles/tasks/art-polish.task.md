---
schema: task
status: completed
owner: prototyper
description: Create a coherent, fine-detail original SVG trim set that makes the selectable Space dialogue styles feel finished and native to their uses.
links: [../AGENTS.md, ../../space-scene-art/tasks/art.task.md, ../../../web/agent-space/uiux/systems/space/candidates.design.md]
writes: [../../../web/agent-space/src/scene/art/studio/refined/]
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"zirate","incarnation_bits":4745315959988960128}}]
---

# Space dialogue art polish

## Outcome

The user has two working Space dialogue styles but says the design materials still look rough. Supply a compact, refined set of original, editable SVG details that can be used by the existing canvas-office scene and Folio/Signal dialogue shells. This must visibly increase craft and coherence, not merely add decorative noise or recolor existing icons.

## Context

Read the linked style design and existing [Studio SVG contract](../../../web/agent-space/src/scene/art/studio/AGENTS.md). The accepted scene art consists of editable flat SVGs with authored outlines, anchors, sensible viewBoxes and no external assets. Current Folio uses warm workshop letter stock and green/rust ink; Signal uses a dark restrained operator panel with teal state light. Same asset may have distinct channel/trim variants only where semantic meaning stays identical.

## Authority and boundary

You own visual design and writing only beneath `web/agent-space/src/scene/art/studio/refined/`. You may add a local inventory/usage note and proof SVG/HTML in that directory. Do not modify production CSS, React, transport, existing master assets or user service. Coordinate a stable filename, intrinsic dimensions and anchor list in the Task so the Controller can bind assets. Do not add text paragraphs to product surfaces.

## Design assessment and delivery

Assess the current two styles and assets before drawing. Produce one coherent asset batch that addresses actual visible joins and surfaces in screenshots: letter/page edge, correspondence seal or folio mark, restrained frame/corner language, surface grain or ruled stock where useful, and a signal-panel edge/indicator language that remains legible. Do not duplicate existing actor, status, role or message facts into artwork. SVGs must remain decorative, low byte size, keyboard/pointer inert and screen-reader hidden when bound.

Include a proof composition of a Folio reader edge and Signal reader edge. The proof is a visual review artifact, not an application screenshot. Record the exact files, use, scaling, intended contrast and assets' semantic limits here. Keep the set self-contained and free of reference to existing pixel/game art.

Update this Task with current understanding at meaningful transitions, then finish with author screenshots or proof and exact handoff. Do not await Controller implementation to produce a complete authored batch.

## Screenshot assessment and art direction

Reviewed the published compare-02 Folio and Signal reader, office, 390 px reader and command close-up captures in `.artifacts/space-ui-styles/compare-02/`. The design question is whether the same transcript and draft can keep their reading priority while the seams make the two shells feel like different physical objects. The Folio reader has clear record rules and a warm stock/rail split, but evenly spaced hairlines read as a screen pattern. Its writing well meets the sheet at a plain border. Signal's dock and card rhythm read clearly, including the active channel and focused draft, but its case, shelf and cards use nearly identical plain lines. The mobile captures place back, portrait, identity and overflow in one row and focus rings close to the screen edges. Keep artwork out of those areas. The Studio room already has textured floor and clear character/prop outlines, so carry the material motifs across using the reader's own edges. Keep the room quiet and existing state markers authoritative.

The selected family is bound paper and an inset instrument shelf. Both use a shallow decorative seam at the history-to-composer join. Folio adds sparse fibers to replace the existing regular grain, a blind-pressed register mark, and an optional corner impression. Signal adds one inset machined keyway and an optional notched corner. Its short blue-green slot indicates the seam's origin and has no live meaning. The first binding is the optional UIUX `conversation-trim` role with two variant URLs. Both masters are 160 × 12, at 1 CSS px per unit, with a top-left anchor. `folio-page-edge.svg` repeats horizontally and `signal-panel-edge.svg` sits once at the leading edge. Align the top at the join, preserve the existing border fallback, and do not stretch. The separate finish details can be composed where the existing layout leaves room.

## Completed art batch and proof

The complete seven-SVG batch, inventory and runnable local composition are under [`refined/`](../../../web/agent-space/src/scene/art/studio/refined/README.md). Its exact files are `folio-page-edge.svg` (160×12), `folio-seal.svg` (36×36), `folio-corner.svg` (28×28), `folio-ruled-stock.svg` (160×80), `signal-panel-edge.svg` (160×12), `signal-corner.svg` (24×24), and `signal-indicator.svg` (18×18). Each uses its top-left intrinsic canvas; the seal and indicator are centered marks. Primary conversation binding: map Folio and Signal edge files to `conversation-trim`, each as a decorative optional URL. Folio repeats-x across the 12px top band of the history/composer boundary. Signal is one top-left 160px motif in the same band; leave the remaining boundary line intact. Keep the existing border fallback. At narrow sizes retain natural 160×12 dimensions, clip unused right space, and omit the motif if the clear band or focus-ring clearance is unavailable. These variants are purely material decoration and carry no live meaning.

Finish inventory: the Folio seal is a compact correspondence stamp for an available header corner; the Folio corner is a quiet paper registration impression; the Folio stock tile (160×80) supplies sparse fibers on two-axis repeat, with existing record rules retained. Signal corner (24×24) supplies a beveled corner bracket and two muted ticks; its 18×18 indicator is a slotted fastener, never an online/attention light. These details are optional and should be omitted when they approach identity, controls, message text or focus. Canvas use is only for existing decorative facade/threshold/frame anchors with clear room; do not overlay actors or status glyphs.

The browser-rendered art proof is [`proof.html`](../../../web/agent-space/src/scene/art/studio/refined/proof.html); captures are [`proof-desktop.png`](../../../web/agent-space/src/scene/art/studio/refined/proof-desktop.png) and [`proof-narrow.png`](../../../web/agent-space/src/scene/art/studio/refined/proof-narrow.png). The deciding question was whether material joins strengthen each reader without displacing transcript content or the draft. At 1100px and 390px the Folio repeating edge reads as a paper seam, the Signal slot stays within its blue-grey rail, both headers/readers remain distinct, and the focused composer is still the strongest edge cue. The first proof placed corner marks over live labels; those marks were removed from the composition and remain optional inventory only. This is an authored static HTML art proof, not an application integration screenshot. No production CSS or app files were changed.

Validation: Chrome decoded the proof's Folio seal and Signal fastener and captured both widths. XML parsing verified all seven SVGs have matching intrinsic dimensions/viewBoxes, only vector primitives, screen-reader-hidden/pointer-inert root semantics, and no external references or event handlers. The seven files total 4,010 bytes. UIUX owner Bavola was sent the exact two variant filenames, dimensions, anchor and repeat rule for the `conversation-trim` role. Controller may bind those URLs in UIUX; the art source remains authoritative here. Remaining evidence is the Controller's integrated app rendering and candidate acceptance.

Task status: completed. Art authoring and isolated proof are complete; integration and whole-page acceptance remain with the Controller/UIUX owner.
