# Canvas workspace redesign

Status: completed for source and isolated browser preview. Owner: vedime. Outcome: one operable workspace for navigating Agent conversations and creating Agents alongside the office. The user asked for dedicated UI Craft assignments across information architecture, density, workflows, visual style, motion and sound. Separate ownership is warranted by the independent exact-identity SDK lifecycle, conversation/command anatomy, workspace layout and feedback disciplines. The controller integrates these against the same browser scenario.

## Experience and structural contract

- Keep the scene mounted; the workspace overlays it. The roster, scene actor, exact URL, related Agent and context entry use the same exact conversation selection. The chat presentation remains a view mode, not a step in a task. The existing new Agent modal remains the real launch path and returns focus to the workspace.
- Conversation windows form a bounded, ordered set of exact IDs, not names. Desktop: two simultaneous live panes, with further open conversations retained in a shelf. A pane can focus (temporarily suppress its peers), restore the arrangement, minimize to the shelf, or close. Narrow: one visible pane and the same shelf; use a container-width rule, not another route. Drafts, attachment state and history positions are keyed by exact ID. Workspace/connection switches must not retarget a draft or request. The focused exact target remains the existing shareable route; auxiliary windows are page-local.
- Prefer tiled panes + a shelf plus an explicit focus mode. Floating overlapping windows were rejected for the first candidate because they obscure long text, demand manual positioning and collapse poorly on narrow viewports; tabs alone cannot compare two conversations. No drag/resizing framework is needed for this slice. Create is a focused modal; relations are a contextual disclosure next to the Agent identity, with explicit upstream/downstream meanings, not a standing thread section.
- Reuse canonical Space UIUX tokens, typography, icon language and existing public SDK with no new backend behavior. Aggressive compression applies to chrome and redundant metadata; message type, line-height, usable focus rings, actionable target size and readable Chinese remain. The visible feedback must work silently; sound is optional and off by default. Continuous scene activity and arrivals must not trigger sound.
- Verification: realistic long records; compare before/after same viewport where feasible; exercise two panes, focus/restore/minimize/close, launch in place, relation navigation, keyboard and narrow layout. Use isolated browser demo/fixture, then distinguish what the real SDK-backed checks establish. Keep selected live deployments intact.

## Ownership

| Task | Owns | Integration |
| --- | --- | --- |
| [Exact readers](readers.task.md) | `src/live/controller.ts`, `src/live/domain-office.ts`, targeted fixture | App drives visible exact IDs; preserve the existing singular entry for legacy checks |
| [Conversation and launch](surfaces.task.md) | Consumer/code for conversation, relation disclosure, composer, command and dedicated compact styles | Existing adapters; window slot contract below |
| [Workspace layouts and style directions](layout.task.md) | `src/workspace/WorkspaceWindows.tsx`, dedicated CSS and style exploration | Controller passes pure pane props; Canvas remains mounted outside this module |
| [Motion and sound](feedback.task.md) | `src/workspace/feedback.tsx`, `feedback.css`; no shared component edits | Controller consumes feedback hook; layout applies documented classes |
| Controller | `App.tsx`, `src/offline/controller.ts`, `src/workspace/windows.ts`, navigation wiring, integration and delivery | Production source and isolated evidence |

Shared contracts may be adjusted in this record once a professional owner identifies the need; otherwise the file ownership above is exclusive. Work in place in the existing dirty worktree with path-scoped changes; no resets/stash/commits.

## Active assignments and setup

UI Craft 1.0.21 is installed through its interactive “Install (this project)” path, Codex skill+commands only, under repository `.codex/skills/`. The installer reported completion; its result screen requires a separate keypress to quit. The first launch held the daemon briefly; subsequent launches were sequenced after readiness. Existing root design memory stays Management-owned.

| Owner | Assignment |
| --- | --- |
| nokoze | Exact readers |
| zidiro | Compact conversation and launch |
| ruvuma | Layout and style directions |
| laveha | Motion and sound |
| vedime | Window state, standard app integration, isolated preview and acceptance |

The operational workspace is the source default; `workspace=classic` selects the previous single-reader presentation for comparison and fallback. Only the focused conversation and canonical panel/scope/channel are shareable. A deliberate workspace change clears open windows (drafts remain exact); an explicit relation whose membership differs changes scope, except when started from the all-members view, where it can remain alongside the original. Each addressable surface retains the existing native membership/access checks.

## Integration findings

- The long-record browser scenario exposed duplicate title chrome and an initial window implementation that discarded paged history when shrinking from two windows. The channel strip now owns the window actions via a shared slot, and [reader leases](readers.task.md) park completed pages without polling. The selected pane keeps the original route-detail contract; other disclosures are local.
- Office resources exercise the same principle as launch: a local dialog retains the Canvas and original conversation context. Narrow reading discloses filters on demand, including the existing search shortcut, and leaves work style, keyboard inspection and opt-in audio under one header control.
- During integration the parallel Domain SDK expansion introduced a stale local distribution: TypeScript and the Vite bundle alone did not catch an old runtime `Agent` without `launchRelations`. Build the package's JavaScript and declarations before constructing the Space artifact. This is a build-input issue; the existing services do not need a restart for isolated testing.

## Delivered evidence

The source-default workspace is integrated. The frozen [Space build log](../../.artifacts/space-canvas-workspace/build-r5.log) and [Chrome report](../../.artifacts/space-canvas-workspace/browser-r6/report.json) cover independent live-shaped leases and paging, parked-reading restoration, exact relationship navigation, in-place Agent launch and resource dialogs, source-width compaction, and a single exact send in the 320–390 px narrow path. No browser page errors were reported. The same 1440 × 900 fixture measured one 328.5 px classic history viewport versus two 451 px workspace history viewports (each 450 px wide). [Classic](../../.artifacts/space-canvas-workspace/browser-r6/classic-desktop.png) · [Two panes](../../.artifacts/space-canvas-workspace/browser-r6/two-readers-desktop.png) · [Launch](../../.artifacts/space-canvas-workspace/browser-r6/launch-desktop.png) · [Short viewport](../../.artifacts/space-canvas-workspace/browser-r6/short-reader.png). The [field desk and reading room studies](layout.design.md#browser-evidence) compare the same data and scene; the shared final treatment combines clear boundaries over art with a quiet interior and existing theme roles.

The reader owner passed 20 SDK/legacy receipt and relation checks; four canonical route cases passed for the controller. Motion/sound's [separate Chrome evidence](feedback.task.md#focused-evidence) covers actual audio calls, deliberate window feedback and reduced motion. A full separate SDK package compilation exposed interfaces still under parallel development; after rebuilding the local runtime code, the scoped Space production build and end-to-end cases above passed. This is not package-wide SDK acceptance or a native service rollout.

The original frozen build used the local preview and is superseded by the [long-conversation LAN preview](experience-review.md#accepted-improvement-and-actual-lan-evidence). Both use the [synthetic fixture](../../web/agent-space/tests/workspace.fixture.mjs). Its SDK responses (including launch and send) are in-memory examples. Reproduce Chrome evidence with `node web/agent-space/tests/workspace.browser.mjs .artifacts/space-canvas-workspace/candidate-r5 /absolute/new/evidence`. No installed binary, native Agent, or existing listener was changed.
