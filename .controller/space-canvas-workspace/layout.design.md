---
scope: Space pane shell, responsive layout and visual comparison
status: active
owner: ruvuma
related: [./layout.task.md, ./controller.md, ./feedback.task.md, ../../.ui-craft/surfaces/space.md]
---

# Canvas conversation windows

Directors need to compare Agent records, act on the right conversation and return to the room. The first view prioritizes the active conversation, a second comparison when there is room, and a compact shelf with every open conversation. The shelf restores windows in one action, including windows hidden by width or focus. A successful comparison leaves the route and the working draft tied to the exact target. See the [controller design](controller.md) for the parent state and SDK boundary.

```text
Scene · wide                             Chat · wide
┌── working scene ───────────────────┐   ┌── roster ──┬── workspace ─────────────────┐
│ visible floor     [A] [B] [C] [D] │   │            │             [A] [B] [C] [D] │
│ + scene controls  ┌─ A ──┬─ B ──┐ │   │            │     ┌──── A ────┬──── B ────┐ │
│                   │reader│reader│ │   │            │     │   reader  │   reader  │ │
│                   └──────┴──────┘ │   │            │     └───────────┴───────────┘ │
└────────────────────────────────────┘   └────────────┴──────────────────────────────┘

Narrow working region (same component)
┌─────────────────────────┐
│ [A] [B] [C] [D] →       │   Scroll only the shelf if its entries overflow.
│ exact reader identity   │
│ channels   focus min ×  │
│ records + composer      │   Same reader as the wider view.
└─────────────────────────┘
```

| State | Structure and route to the next action |
| --- | --- |
| Empty, zero open IDs | No overlay; normal chat empty or roster return is composed by the caller. |
| Tiled | Parent matches the visible IDs to its reader leases using available working-region width (`800px` scene, `1000px` chat including roster). The shell uses two equal columns only when supplied two visible panes; otherwise the active reader remains keyed and mounted in the single slot. |
| Focused | Parent marks only the active pane visible. Shelf remains in place and activation does not change focus mode; restoring the tile arrangement is a separate action. |
| Minimized, overflow, or narrow peer | All still have a shelf button. If all are minimized, the shelf still works and the scene receives pointer input elsewhere. |
| Loading, empty channel, failure, partial, conflict or offline | Reader and controller own status, recovery and send results. A supplied reader uses the same bounded slot in every window state. |

`WorkspaceWindows` uses exact IDs as React keys and exposes `.space-window-pane`, `.space-window-shelf`, `data-pane-id`, `data-shelf-id`, `data-active`, `data-minimized`, `data-focused` and `data-window-action="focus|minimize|close"`. Pass optional `motion={{ id, event }}` using [the companion Task](feedback.task.md): matching `open` and `restore` mark the pane, `minimize` marks the shelf button, and the nested `data-window-motion-header` marks the control group for the cue. Control order is focus, minimize, close. Entering an inactive reader with keyboard or pointer focus activates it; its window buttons directly address the pane without changing the active reader as a side effect. Activation uses `aria-current` on the shelf; native buttons include the supplied Agent title in their accessible names. Minimize returns keyboard focus to the pane's shelf button; close uses an adjacent shelf button when one remains. App/ChatLayout owns return when the last window closes and must keep the shelf reachable in narrow chat even when all windows are minimized. Pane `title` is caller-supplied; add a readable instance discriminator when different exact IDs share a display name.

The overlay uses a positioned working region as its containing block and allows scene hits outside the panes and shelf. Preserve the bottom camera and roster controls, even when a narrow reader fills the width. Chat uses the same slot as a flex- or grid-sized child of the existing reader region. Keep the native Canvas outside this module. The initial integrated render showed the separate pane caption duplicated the reader identity and cost a 34px row. The shell provides its window actions through `reader: (windowControls) => ReactNode`; the production reader slots them beside the existing channel controls. Plain `ReactNode` readers use the smaller caption and control fallback for independent consumers. The reader owns exact identity, history and composer. Motion is owned by the companion; the pane shell is usable with motion and sound off.

| Visual direction | Same scene, records, controls and density |
| --- | --- |
| Field desk | Warm trim, a firmer case edge and clearer shadow over the scene. A stronger selected shelf makes switching easier, but two framed windows can pull attention from the records. |
| Reading room | Cooler neutral shell, fine dividers and less elevation. Long writing feels quieter, but subtle frame edges can merge into busy scene art; high-contrast dark skins need the shell roles to resolve from their own palette. |
| Shared treatment | Keep the field desk's clear outer silhouette only over the scene and the reading room's light internal rules and plain label typography. Use the skin's existing surface, ink and focus colors; no additional Space skin, font or message padding. An underline cues the current shelf item and remains legible even when the theme's fill is subtle. |

## Browser evidence

At a 1440 × 900 viewport, compare [field desk](layout-evidence/field-desk.jpg), [reading room](layout-evidence/reading-room.jpg) and [the shared result](layout-evidence/shared.jpg). These show the same paused Studio scene, the same `@mora` and `@luna` panes, and the same synthetic long records added to the existing Studio replies in the browser fixture. The field desk's tinted action group is easy to locate but adds a second focal point in each pane. The reading room's low contrast edge is quiet, but the outline fades against the floor. The shared variant keeps the office-derived boundary and leaves the existing channel strip plain. A flexible gutter leaves the mailbox and floor visible at this width; narrower working regions give more of their width to the reader.

The [browser runner](layout.browser.mjs) passed in local Chrome against Space Vite on port `19147`. It exercised two equal columns, third-window shelving, reader focus activating the exact Agent without reordering the pair, named actions, keyboard focus/restore/minimize/close, motion markers with reduced motion, reader DOM and scroll continuity at 390px, working camera roster controls and a 44px window action at 320px with touch emulation, the ordinary chat reader region with desktop roster, and Signal palette inheritance. Space TypeScript passed with `node node_modules/typescript/bin/tsc -p web/agent-space/tsconfig.json --noEmit`. The preview used the existing [Rolldown WASI fallback](../../.docs/collaboration-workbench/frontend.md) on the local CPU; the runner accepts `AGENT_SPACE_LAYOUT_URL`, `AGENTX_PLAYWRIGHT_PACKAGE` and an output directory. This is isolated browser evidence for the layout and presentation, separate from native reads, send, audio playback and service adoption.
