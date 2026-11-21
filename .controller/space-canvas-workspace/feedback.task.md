---
schema: task
status: completed
title: Meaningful workspace motion and optional sound
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"laveha","incarnation_bits":4745316436585786525}}]
---

Read [controller design](controller.md), [Space brief](../../.ui-craft/surfaces/space.md), [UI Craft](../../.codex/skills/ui-craft/SKILL.md), Animate, Delight, motion, sound and reduced-motion guidance. Own `src/workspace/feedback.tsx` and `feedback.css` plus your Task. Compare which events merit feedback, then implement purposeful brief movement for workspace surfaces without animating long text/scroll, simulate nothing and avoid deps. Provide `useWorkspaceFeedback()` returning `{ enabled:boolean, toggle():void, play(event: "open" | "minimize" | "restore" | "close"):void }` with optional sound (default off, explicit opt-in, settings persisted defensively, no autoplay/arrival sound). Export `WorkspaceSoundToggle` if helpful with `enabled`/ `onToggle`. CSS can target .space-window-pane and .space-window-shelf; coordinate through Task and prefer reduced motion. No App, scene clock, styles from another lane or deployment edits. Record browser acceptance ideas and actual focused evidence.

## Design and integration contract

The visual cue follows the destination of a deliberate window action. The stylesheet lives beside the feedback hook and targets the pane's window actions or its exact shelf item; window state and exact IDs stay owned by the controller and layout.

| Event | Visual feedback | Sound after opt-in |
| --- | --- | --- |
| Open a new window | The window actions settle 4 px into place (200 ms). | A short rising tone. |
| Minimize a visible window | The matching shelf item settles down into the shelf (120 ms); the pane is removed immediately. | A short descending tone. |
| Restore a window from the shelf | The window actions lift 4 px into place (200 ms). | A softer rising tone. |
| Close a window | The pane and shelf item disappear immediately. Focus and the final layout provide the confirmation. | A quiet descending tone. |
| Activate an already visible pane, toggle focus, change channel, scroll, type, receive updates or restore a route | The relevant state changes immediately. Reading content, initial route restoration and frequent actions get no choreographed motion. | Silent. |

- Controller: render one `WorkspaceSoundToggle` in the existing workspace controls and call `play(event)` from the direct click/keyboard handler for a real transition. When the handler opens, minimizes or restores an exact window, forward a transient marker `{ id, event: "open" | "minimize" | "restore" }` to the layout, scoped to the destination render. Leave it absent for initial/deep-link restoration, scope changes and passive reader updates. Clear it for subsequent unrelated window changes so remounting another pane cannot replay it. This marker is needed because CSS alone cannot distinguish a user-requested mount from route or data restoration.
- Layout: on the matching visible `.space-window-pane` put `data-window-motion="open"` or `"restore"` and mark the nested window action group with `data-window-motion-header`, including the fallback placement. This keeps the existing conversation channel strip and record content still. Put `data-window-motion="minimize"` on the exact interactive shelf item inside `.space-window-shelf` when minimized. No wrapper, delay or hidden exiting pane is needed.
- Feedback: the hook owns only audio preference and Web Audio. It reads storage defensively, creates its context on the first opted-in window gesture, never plays on preference change or render, and discards interrupted callbacks. The toggle has a stable accessible name and pressed state; its glyph also shows muted versus enabled. Reduced motion renders the final state immediately and the sound preference remains independent. A fixed quiet gain and the explicit mute are enough for this bounded cue; no separate volume panel.

## Browser acceptance

In desktop view, exercise exact open → minimize → restore → close, with a second long reader scrolled and a focused control. Check that only the destination chrome animates, the second reader keeps its scroll and position, and no extra pane survives a fast reversal. Repeat at narrow width and with reduced motion. Activate the toggle with keyboard, verify default silence and no sound on toggle or initial/deep-link restoration, then exercise opted-in sounds, reload persistence, muting and blocked storage/audio. Inspect the Chrome Web Audio call path separately from a listening check; neither verifies a native rollout.

## Focused evidence

[Feedback hook and control](../../web/agent-space/src/workspace/feedback.tsx) and [feedback CSS](../../web/agent-space/src/workspace/feedback.css) pass the targeted TypeScript check and the Space runner's scoped typecheck. Chrome 147, using the actual sources in an isolated DOM harness, passed desktop cue placement and unchanged scrolled peer (same DOM node at `scrollTop=145`), 44 px minimum native button with keyboard opt-in, persisted opt-in with no cue on reload until a gesture, 390 × 844 touch with reduced motion and independent opt-in, and graceful recovery with inaccessible storage or Web Audio. Only the destination action group (200 ms) or shelf item (120 ms) fired an animation. All four native oscillator pitch pairs started in a running context after the relevant gestures, with one context per mounted page. Restricted storage left session opt-in usable; a forced audio failure left the window action usable. The two failure cases only inject environment failures. [Local receipt](/tmp/agentx-space-feedback.3HTifX/nested-receipt.json) and [runner](/tmp/agentx-space-feedback.3HTifX/nested.mjs) are ephemeral; the browser outcomes above are the durable evidence.

Chrome 147 also passed the actual Space App at an isolated `127.0.0.1:19368/?supply=demo` using the repository's [WASI-backed Space runner](../../web/agent-space/scripts/run.mjs). At 1440 × 1000, opening @luna was silent by default; after keyboard opt-in, @mora opened, minimized to its exact shelf button (receiving focus), restored with Enter and closed. The @luna reading node and scroll were preserved across @mora's minimize and restore. Open and restore animated only the marked action group inside the real conversation channel strip. The intermediate fallback header placement was also checked and moved only the marked group. Toggling focus mode in both directions added no cue, and the four sound actions used one native context. Reload restored the @luna route and the opt-in without any entrance cue or audio context. At 390 × 844 with touch and system reduced motion, the same window cycle completed with no cue animation and default silence. No page errors. The raw Vite CLI failed before listening on this host; using the documented runner selected WASI successfully. [App receipt](/tmp/agentx-space-feedback.3HTifX/app-nested-receipt.json), [fallback receipt](/tmp/agentx-space-feedback.3HTifX/app-fallback-receipt.json) and [runner](/tmp/agentx-space-feedback.3HTifX/app-nested.mjs) are ephemeral.

| Before | After | Why |
| --- | --- | --- |
| A window transition had no local directional cue. | Motion is bounded to the window actions or the exact shelf destination, with a static reduced-motion result. | Track the window without disturbing the reader. |
| Workspace window actions had no optional audio setting. | One accessible and persisted opt-in; quiet native cues only from deliberate transitions. | Shared or quiet workspaces remain silent until requested. |

No hover choreography, decorative delight, content movement or window exit copies were added: these would pull attention away from the reading task or delay its next action. The native call trace is not a subjective listening test, and the offline browser exercises do not establish live rollout.
