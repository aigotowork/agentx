---
schema: task
status: completed
description: Make Runtime Atlas playback visibly show causal event flow and compact local disclosure.
links: [../runtime-atlas.md, ./runtime-atlas-model.md, ../../../.docs/architecture/runtime-atlas/README.md]
---

# Runtime atlas visible execution UI

This is a bounded follow-up to the completed Runtime Atlas artifact. The user reports that playback only changes text/highlights, long lower content hides the main graph, and a demonstration must show the operation, data flow and state changes. The artifact remains an explanatory simulation; it must never call AgentX, a Provider, a product API or a live process.

## Ownership

Own only `.docs/architecture/runtime-atlas/atlas.js`, `.docs/architecture/runtime-atlas/atlas.css`, `.docs/architecture/runtime-atlas/layouts/workspace.css` and this Task. Do not edit `model.json`, the template, builder, generated HTML, browser cases, native/product source, server or user services. Do not install dependencies or run build/browser/Cargo/formatter/product commands. Controller will build and execute the affected acceptance.

## Frozen semantic contract

The canonical `model.json` has16 nodes,23 edges,5 modes,14 scenarios and49 steps. A step has actor, handler, declared `edgeIds`, actions, event, patch, note and authored `beats`. The released128 beats use `{kind:"transfer"|"disconnect",edgeId,direction:"forward"|"reverse",label}` or `{kind:"local"|"observe"|"retain",nodeId,label}`. Consume this actual model; do not invent runtime events or timing claims. Preserve disconnect semantics: a broken connection cannot render a successful delivery; retained execution and unknown facts remain distinct.

## Required user-facing behavior

- Playback must visibly stage one selected step: identify source/handler, animate or otherwise visibly move an event marker along each declared edge in order, show connection break for disconnects, and then apply the node/fact changes.
- Display a compact before→after change strip for changed nodes/facts. Keep the graph as the main visual resource; move long action lists, notes and source detail into a local disclosure/drawer/popover.
- Play, pause, next, previous, reset and mode/scenario changes must cancel obsolete animation safely. Reduced motion preserves the stages and text changes without requiring movement.
- Keep current stable test IDs and keyboard/touch selection. Preserve hash navigation and offline/no-network behavior.
- Do not add standing explanatory paragraphs. Use concise labels, icons, status, and contextual disclosure.

## Completion evidence

Controller will run the existing semantic check, rebuild, and affected browser/design checks at1440×900,1366×768 and390×844. The result must show a visible event/data-flow transition, visible staged state change, pause/cancel behavior, readable graph, and no document-wide narrow overflow. Preserve prior acceptance evidence and disclose that this remains a behavior simulation.

Before coding, inspect the local interface standard and current atlas source. Release one coherent UI correction with a concise handoff and no extra review cycle.

## Selected presentation structure

The existing workspace reserves a permanent inspector column and a264px lower prose band; interval playback replaces snapshots without depicting causality. This is the owning structural defect behind the feedback. Keep the map full-width as the primary region, replace lower prose with a compact event dock and five fact buttons, and reuse the existing inspector as a dismissible local overlay for objects, full event actions/notes/sources and full before/after values. Preserve existing IDs and source-path copy behavior.

Controller clarification supersedes the original derivation-only contract: consume additive canonical `step.beats` (`transfer|disconnect|local|observe|retain`, optional `edgeId/direction/nodeId`, required concise `label`). Direction belongs to the authored beat; never infer successful transfer from a disconnect. Fallback without beats is a local event emphasis at the actor and makes no transport claim.

One presentation-owned transition runs `ready → flow (one beat at a time) → handling → applied`. `foldedState(index)` stays deterministic; a separate visible snapshot uses the preceding step until application. Next/replay starts immediately, play resumes a paused transition or immediately starts the next step; pause freezes elapsed progress. Previous/reset/mode/scenario/hash cancel the animation frame and obsolete transition. Reduced motion preserves phase duration, static break/local cues and state changes without moving packets. Timings are illustrative UI pacing, never runtime measurements.

Stable acceptance hooks: `event-phase[data-phase=ready|flow|handling|applied]`, moving SVG `flow-marker`, `state-diff`, `event-replay`, `detail-toggle`; existing node/module/edge/fact/control IDs remain. Controller owns all execution and rendered evidence.

## Source release and controller handoff

Released one coherent correction in the three owned presentation leaves. No other source, template, model, builder, generated output, tests, native/product files or service was changed by this Task. The controller's canonical beats are consumed directly; no delivery, direction, process continuity or runtime timing is inferred from labels. There are no new network calls or external assets.

The full-width map replaces the permanent inspector column. Desktop lower workspace is156px total, containing the current/next event, source→handler, phase, replay/detail controls, horizontally contained before→after strip and five fact buttons. Full actions, notes, source paths and unabridged state values use the existing inspector as a closeable overlay; on390px it becomes a contained bottom drawer. Escape/close return focus; keyboard module selection, touch controls and copy fallback remain. Actor/handler labels and distinct outlines provide meaning beyond color; final changed node states receive their own outline.

Playback starts immediately from Play, Next or current-event replay. One owned RAF advances authored beats then handling then application; facts and node states remain at the preceding snapshot until applied. Transfer markers follow actual SVG paths and authored reverse direction; `disconnect` draws a red cut and dashed connection, while local/observe/retain show stationary labeled cues. Pause keeps elapsed time and packet position; object/module/edge selection rerenders at the retained progress without restarting the event. Previous/reset/mode/scenario/hash discard obsolete frames and transitions. Reduced motion uses static packet/cue positions with the same phase labels and staged application. Backgrounding the page pauses playback.

Acceptance hooks additionally expose `event-phase[data-beat-kind]`, `flow-marker[data-direction][data-edge-id]`, `flow-break`, `flow-cue`, `state-diff[data-applied]`, `fact-{connection,host,provider,turn,inbox}`, `event-details` and `detail-close`. Existing model/node/module/edge/control test IDs are preserved. Deep links reconstruct deterministic completed checkpoints; transient playback timing is intentionally not serialized.

Authoring/static source inspection only: no build, test, browser, formatter, install, package or product command was executed, and no rendered-quality or passing-check claim is made. Controller owns generation and the actual normal-motion/reduced-motion, pause/resume/cancel, reverse/disconnect,1440×900/1366×768/390×844, keyboard/source-copy/hash/offline evidence in the [model/outcome Task](runtime-atlas-model.md#visible-execution-follow-up). Old browser assertions that immediately inspect state after Next must wait for `data-phase=applied`; long `.action-list` content is intentionally local to the event disclosure. The updated interaction invalidates the earlier immediate-snapshot/layout assumptions, not the unchanged native lifecycle facts.

All presentation and Task writes are released to the controller. Completion here means source handoff, with independent build/browser/design acceptance still pending under the owning outcome Task.
